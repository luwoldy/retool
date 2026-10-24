//! Geometry stage: lens undistortion (from the camera's own radial table)
//! plus verticals correction — find the vertical vanishing point from
//! near-vertical edges, rotate the camera so that point sits at infinity
//! (undoing residual roll + pitch), crop back to the native aspect. One
//! resampling for both.

const WORK_W: u32 = 1200; // edge detection resolution
const MAX_TILT_DEG: f32 = 12.0; // beyond this the shot needs a retake, not a warp
const MIN_LINES: usize = 300;
const MIN_INLIER_FRAC: f32 = 0.25;
const INLIER_SIN: f32 = 0.035; // ~2° angular tolerance (uncorrected barrel distortion bows verticals)
const RANSAC_ITERS: usize = 500;

pub struct Lens {
    pub focal_px: f32,
    /// Radial distortion knots (radius as a fraction of the half-diagonal,
    /// distortion in %; negative = barrel), as Fuji writes them in the RAF.
    pub distortion: Vec<(f32, f32)>,
}

impl Lens {
    fn pct(&self, r: f32) -> f32 {
        let k = &self.distortion;
        if r <= k[0].0 {
            return k[0].1 * r / k[0].0;
        }
        for w in k.windows(2) {
            if r <= w[1].0 {
                return w[0].1 + (w[1].1 - w[0].1) * (r - w[0].0) / (w[1].0 - w[0].0);
            }
        }
        let (a, b) = (k[k.len() - 2], k[k.len() - 1]);
        b.1 + (b.1 - a.1) * (r - b.0) / (b.0 - a.0)
    }

    /// As-shot radius for an ideal (undistorted) radius, both as fractions of the half-diagonal.
    fn real_radius(&self, r_ideal: f32) -> f32 {
        if self.distortion.is_empty() {
            return r_ideal;
        }
        let mut r = r_ideal;
        for _ in 0..4 {
            r = r_ideal * (1.0 + self.pct(r) / 100.0);
        }
        r
    }

    /// Map an ideal point (centered coords) to its as-shot position.
    fn distort(&self, x: f32, y: f32, half_diag: f32) -> (f32, f32) {
        let r = (x * x + y * y).sqrt() / half_diag;
        if r < 1e-6 {
            return (x, y);
        }
        let k = self.real_radius(r) / r;
        (x * k, y * k)
    }
}

pub struct Report {
    pub note: String,
    /// None = verticals left as shot (undistortion still applied)
    pub rotated_deg: Option<f32>,
    pub crop_pct: f32,
}

/// Undistort + rectify a linear-light RGB f32 image. Returns pixels at the
/// same dimensions plus a report on the verticals decision.
pub fn rectify(w: u32, h: u32, px: &[f32], lens: &Lens) -> (Vec<f32>, Report) {
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let skip = |note: String| {
        (warp(w, h, px, lens, identity).0, Report { note: format!("undistorted; {note}"), rotated_deg: None, crop_pct: 0.0 })
    };
    let focal_px = lens.focal_px;

    let (small, s) = small_luma(w, h, px);
    let small = undistort_small(&small, lens, s, w, h);
    let lines = vertical_edges(&small, s);
    if lines.len() < MIN_LINES {
        return skip(format!("left as shot: only {} vertical edges", lines.len()));
    }
    let Some((vp, inliers)) = find_vp(&lines, w as f32) else {
        return skip("left as shot: no vanishing point".into());
    };
    let frac = inliers as f32 / lines.len() as f32;
    if let Ok(dir) = std::env::var("RETOOL_DEBUG") {
        debug_dump(&dir, &small, s, &lines, &vp, focal_px);
    }
    if frac < MIN_INLIER_FRAC {
        return skip(format!("left as shot: weak consensus ({inliers}/{} edges)", lines.len()));
    }

    // camera-frame direction of the world vertical, and the rotation that
    // brings it onto the image y axis (minimal rotation => no yaw)
    let a = normalize([vp[0] / focal_px, vp[1] / focal_px, vp[2]]);
    let b = [0.0, if a[1] >= 0.0 { 1.0 } else { -1.0 }, 0.0];
    let angle = dot(a, b).clamp(-1.0, 1.0).acos();
    let angle_deg = angle.to_degrees();
    if angle_deg > MAX_TILT_DEG {
        return skip(format!("left as shot: tilt {angle_deg:.1}° over {MAX_TILT_DEG}° cap"));
    }
    if angle_deg < 0.05 {
        return skip("already plumb".into());
    }
    let r = rodrigues(normalize(cross(a, b)), angle);
    let (out, crop) = warp(w, h, px, lens, r);
    let crop_pct = 100.0 * (1.0 - crop);
    let note = format!("undistorted; rotated {angle_deg:.2}° ({inliers}/{} edges agree), crop {crop_pct:.1}%", lines.len());
    (out, Report { note, rotated_deg: Some(angle_deg), crop_pct })
}

type Luma = image::ImageBuffer<image::Luma<f32>, Vec<f32>>;

/// Gamma-encoded luma, downscaled to the edge-detection width. Returns (image, scale).
fn small_luma(w: u32, h: u32, px: &[f32]) -> (Luma, f32) {
    let s = WORK_W as f32 / w as f32;
    let sh = ((h as f32 * s).round() as u32).max(3);
    let luma: Luma = image::ImageBuffer::from_fn(w, h, |x, y| {
        let p = &px[3 * (y * w + x) as usize..][..3];
        let l = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
        image::Luma([l.max(0.0).powf(1.0 / 2.2)])
    });
    (image::imageops::resize(&luma, WORK_W, sh, image::imageops::FilterType::Triangle), s)
}

fn undistort_small(small: &Luma, lens: &Lens, s: f32, w: u32, h: u32) -> Luma {
    if lens.distortion.is_empty() {
        return small.clone();
    }
    let (sw, sh) = small.dimensions();
    let half_diag = ((w * w + h * h) as f32).sqrt() / 2.0;
    let (cx, cy) = (sw as f32 / 2.0, sh as f32 / 2.0);
    image::ImageBuffer::from_fn(sw, sh, |x, y| {
        let (ix, iy) = ((x as f32 + 0.5 - cx) / s, (y as f32 + 0.5 - cy) / s);
        let (rx, ry) = lens.distort(ix, iy, half_diag);
        let (fx, fy) = ((rx * s + cx - 0.5).clamp(0.0, sw as f32 - 1.0), (ry * s + cy - 0.5).clamp(0.0, sh as f32 - 1.0));
        let (x0, y0) = (fx.floor() as u32, fy.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(sw - 1), (y0 + 1).min(sh - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let g = |xx, yy| small.get_pixel(xx, yy)[0];
        image::Luma([(g(x0, y0) * (1.0 - tx) + g(x1, y0) * tx) * (1.0 - ty) + (g(x0, y1) * (1.0 - tx) + g(x1, y1) * tx) * ty])
    })
}

/// A near-vertical edge pixel as a line: point (centered full-res coords) + unit direction.
struct Line {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

fn vertical_edges(small: &Luma, s: f32) -> Vec<Line> {
    let (sw, sh) = small.dimensions();
    let g = |x: u32, y: u32| small.get_pixel(x, y)[0];

    let (mut gx, mut gy, mut mag) = (vec![0f32; (sw * sh) as usize], vec![0f32; (sw * sh) as usize], vec![0f32; (sw * sh) as usize]);
    for y in 1..sh - 1 {
        for x in 1..sw - 1 {
            let dx = (g(x + 1, y - 1) + 2.0 * g(x + 1, y) + g(x + 1, y + 1)) - (g(x - 1, y - 1) + 2.0 * g(x - 1, y) + g(x - 1, y + 1));
            let dy = (g(x - 1, y + 1) + 2.0 * g(x, y + 1) + g(x + 1, y + 1)) - (g(x - 1, y - 1) + 2.0 * g(x, y - 1) + g(x + 1, y - 1));
            let i = (y * sw + x) as usize;
            gx[i] = dx / 4.0;
            gy[i] = dy / 4.0;
            mag[i] = (dx * dx + dy * dy).sqrt() / 4.0;
        }
    }

    // near-vertical edges (gradient mostly horizontal), thinned across x
    let (cx, cy) = (sw as f32 / 2.0, sh as f32 / 2.0);
    let mut cands: Vec<(f32, Line)> = Vec::new();
    for y in 1..sh - 1 {
        for x in 1..sw - 1 {
            let i = (y * sw + x) as usize;
            let m = mag[i];
            if m < 0.03 || gy[i].abs() > 0.47 * gx[i].abs() || m < mag[i - 1] || m < mag[i + 1] {
                continue;
            }
            let n = (gx[i] * gx[i] + gy[i] * gy[i]).sqrt();
            cands.push((
                m,
                Line { x: (x as f32 - cx) / s, y: (y as f32 - cy) / s, dx: -gy[i] / n, dy: gx[i] / n },
            ));
        }
    }
    cands.sort_by(|a, b| b.0.total_cmp(&a.0));
    cands.truncate(40_000);
    cands.into_iter().map(|c| c.1).collect()
}

/// Homogeneous line through the edge point along its direction, normalized so (a,b) is unit.
fn homog(l: &Line) -> [f32; 3] {
    [-l.dy, l.dx, l.x * l.dy - l.y * l.dx]
}

fn is_inlier(l: &Line, v: &[f32; 3]) -> bool {
    let (tx, ty) = (v[0] - v[2] * l.x, v[1] - v[2] * l.y);
    let n = (tx * tx + ty * ty).sqrt();
    n > 1e-6 && ((l.dx * ty - l.dy * tx) / n).abs() < INLIER_SIN
}

/// RANSAC over line pairs, then least-squares refit on the inliers.
fn find_vp(lines: &[Line], w: f32) -> Option<([f32; 3], usize)> {
    let mut rng = 0x9E3779B97F4A7C15u64;
    let mut next = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng % lines.len() as u64) as usize
    };
    // count on a subsample for speed, refit on everything
    let step = (lines.len() / 4000).max(1);
    let sample: Vec<&Line> = lines.iter().step_by(step).collect();

    let mut best: Option<([f32; 3], usize)> = None;
    for _ in 0..RANSAC_ITERS {
        let (i, j) = (next(), next());
        if (lines[i].x - lines[j].x).abs() < 0.2 * w {
            continue;
        }
        let v = cross(homog(&lines[i]), homog(&lines[j]));
        if v.iter().all(|c| c.abs() < 1e-9) {
            continue;
        }
        let n = sample.iter().filter(|l| is_inlier(l, &v)).count();
        if best.is_none_or(|(_, bn)| n > bn) {
            best = Some((v, n));
        }
    }
    let (mut v, _) = best?;
    for _ in 0..3 {
        let mut m = [[0f32; 3]; 3];
        for l in lines.iter().filter(|l| is_inlier(l, &v)) {
            let h = homog(l);
            for r in 0..3 {
                for c in 0..3 {
                    m[r][c] += h[r] * h[c];
                }
            }
        }
        v = min_eigvec(m);
    }
    let inliers = lines.iter().filter(|l| is_inlier(l, &v)).count();
    Some((v, inliers))
}

/// Eigenvector of the smallest eigenvalue of a symmetric 3x3 (cyclic Jacobi).
fn min_eigvec(mut a: [[f32; 3]; 3]) -> [f32; 3] {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..30 {
        for p in 0..3 {
            for q in p + 1..3 {
                if a[p][q].abs() < 1e-12 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..3 {
                    let (akp, akq) = (a[k][p], a[k][q]);
                    a[k][p] = c * akp - s * akq;
                    a[k][q] = s * akp + c * akq;
                }
                for k in 0..3 {
                    let (apk, aqk) = (a[p][k], a[q][k]);
                    a[p][k] = c * apk - s * aqk;
                    a[q][k] = s * apk + c * aqk;
                }
                for k in 0..3 {
                    let (vkp, vkq) = (v[k][p], v[k][q]);
                    v[k][p] = c * vkp - s * vkq;
                    v[k][q] = s * vkp + c * vkq;
                }
            }
        }
    }
    let i = (0..3).min_by(|&i, &j| a[i][i].total_cmp(&a[j][j])).unwrap();
    normalize([v[0][i], v[1][i], v[2][i]])
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let n = dot(a, a).sqrt().max(1e-12);
    [a[0] / n, a[1] / n, a[2] / n]
}

fn rodrigues(k: [f32; 3], angle: f32) -> [[f32; 3]; 3] {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    [
        [t * k[0] * k[0] + c, t * k[0] * k[1] - s * k[2], t * k[0] * k[2] + s * k[1]],
        [t * k[0] * k[1] + s * k[2], t * k[1] * k[1] + c, t * k[1] * k[2] - s * k[0]],
        [t * k[0] * k[2] - s * k[1], t * k[1] * k[2] + s * k[0], t * k[2] * k[2] + c],
    ]
}

fn apply(m: &[[f32; 3]; 3], p: [f32; 3]) -> [f32; 3] {
    [dot(m[0], p), dot(m[1], p), dot(m[2], p)]
}

/// H = K R K^-1 in centered coordinates on the undistorted image; output
/// canvas is the largest native-aspect rectangle inside the rotated frame.
/// Returns pixels + crop scale.
fn warp(w: u32, h: u32, px: &[f32], lens: &Lens, r: [[f32; 3]; 3]) -> (Vec<f32>, f32) {
    let f = lens.focal_px;
    let half_diag = ((w * w + h * h) as f32).sqrt() / 2.0;
    let fwd = |p: [f32; 3]| -> [f32; 2] {
        let q = apply(&r, [p[0] / f, p[1] / f, p[2]]);
        [q[0] / q[2] * f, q[1] / q[2] * f]
    };
    let (hw, hh) = (w as f32 / 2.0, h as f32 / 2.0);
    let tl = fwd([-hw, -hh, 1.0]);
    let tr = fwd([hw, -hh, 1.0]);
    let bl = fwd([-hw, hh, 1.0]);
    let br = fwd([hw, hh, 1.0]);
    let (x0, x1) = (tl[0].max(bl[0]), tr[0].min(br[0]));
    let (y0, y1) = (tl[1].max(tr[1]), bl[1].min(br[1]));
    let (mut cw, mut ch) = (x1 - x0, y1 - y0);
    if cw / ch > w as f32 / h as f32 {
        cw = ch * w as f32 / h as f32;
    } else {
        ch = cw * h as f32 / w as f32;
    }
    let (ox, oy) = ((x0 + x1) / 2.0 - cw / 2.0, (y0 + y1) / 2.0 - ch / 2.0);

    // inverse map: output -> rectified plane -> source, R^T = R^-1
    let rt = [[r[0][0], r[1][0], r[2][0]], [r[0][1], r[1][1], r[2][1]], [r[0][2], r[1][2], r[2][2]]];
    // bicubic (Catmull-Rom) sample; the bilinear one read soft at 100 percent on 40 MP (2026-09-03)
    let cubic = |t: f32| -> [f32; 4] {
        let (t2, t3) = (t * t, t * t * t);
        [-0.5 * t3 + t2 - 0.5 * t, 1.5 * t3 - 2.5 * t2 + 1.0, -1.5 * t3 + 2.0 * t2 + 0.5 * t, 0.5 * t3 - 0.5 * t2]
    };
    let sample = |x: f32, y: f32| -> [f32; 3] {
        let x = x.clamp(0.0, w as f32 - 1.0);
        let y = y.clamp(0.0, h as f32 - 1.0);
        let (x0, y0) = (x.floor() as i64, y.floor() as i64);
        let (wx, wy) = (cubic(x - x0 as f32), cubic(y - y0 as f32));
        let at = |xx: i64, yy: i64| {
            let (xx, yy) = (xx.clamp(0, w as i64 - 1) as usize, yy.clamp(0, h as i64 - 1) as usize);
            &px[3 * (yy * w as usize + xx)..][..3]
        };
        let mut o = [0f32; 3];
        for (j, wj) in wy.iter().enumerate() {
            for (i, wi) in wx.iter().enumerate() {
                let p = at(x0 + i as i64 - 1, y0 + j as i64 - 1);
                let wgt = wi * wj;
                for c in 0..3 {
                    o[c] += p[c] * wgt;
                }
            }
        }
        o.map(|v| v.max(0.0)) // Catmull-Rom undershoots at hard edges; the tone step handles the top
    };
    let mut out = vec![0f32; px.len()];
    for v in 0..h {
        let ry = oy + (v as f32 + 0.5) / h as f32 * ch;
        for u in 0..w {
            let rx = ox + (u as f32 + 0.5) / w as f32 * cw;
            let q = apply(&rt, [rx / f, ry / f, 1.0]);
            let (ix, iy) = lens.distort(q[0] / q[2] * f, q[1] / q[2] * f, half_diag);
            out[3 * (v * w + u) as usize..][..3].copy_from_slice(&sample(ix + hw - 0.5, iy + hh - 0.5));
        }
    }
    (out, cw / w as f32)
}

/// RETOOL_DEBUG=<dir>: edge map (inliers green, rest red) + the roll/pitch read of the VP.
fn debug_dump(dir: &str, small: &Luma, s: f32, lines: &[Line], vp: &[f32; 3], f: f32) {
    let (sw, sh) = small.dimensions();
    let mut img = image::RgbImage::from_fn(sw, sh, |x, y| {
        let v = (small.get_pixel(x, y)[0] * 255.0) as u8;
        image::Rgb([v, v, v])
    });
    for l in lines {
        let (x, y) = ((l.x * s + sw as f32 / 2.0) as u32, (l.y * s + sh as f32 / 2.0) as u32);
        if x < sw && y < sh {
            img.put_pixel(x, y, if is_inlier(l, vp) { image::Rgb([0, 255, 0]) } else { image::Rgb([255, 0, 0]) });
        }
    }
    let path = std::path::Path::new(dir).join("verticals_edges.png");
    img.save(&path).ok();
    let a = normalize([vp[0] / f, vp[1] / f, vp[2]]);
    let a = if a[1] < 0.0 { [-a[0], -a[1], -a[2]] } else { a };
    eprintln!(
        "verticals debug: vp=({:.0},{:.0}) px from centre, roll {:.2}°, pitch {:.2}° -> {}",
        vp[0] / vp[2], vp[1] / vp[2], a[0].atan2(a[1]).to_degrees(), a[2].atan2(a[1]).to_degrees(), path.display()
    );
}
