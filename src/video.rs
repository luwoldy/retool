//! `retool clips`: ingest the card's video (camera MOVs in `video/`, phone
//! verticals in `phone/`), place every clip in the shoot (station by
//! timestamp, gimbal pass after the last station), segment its motion
//! (still / pan / push / pull) and type it (room from the scorecard for
//! station clips, by picture for phone and gimbal clips). Writes
//! `out/video/clips.tsv`, the planner's input.
//!
//! Media work is ffmpeg + exiftool shell-outs, like `fuse` shells out to
//! enfuse. The motion estimate is our own: vid.stab's detect file is binary
//! and its transform filter cannot read it back on this build, and all the
//! planner needs is where a clip holds still and whether a move is lateral
//! or a push.

use anyhow::{bail, Context, Result};
use chrono::NaiveDateTime;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::realm::Embedder;

/// Fuji's own F-Log2 → Rec.709 cube, unzipped from the download (README setup).
pub const WDR_CUBE: &str = "luts/XH2_FLog2_FGamut_to_WDR_BT.709_33grid_V.1.00.cube";
/// Apple's Log → Rec.709 cube when the operator has put it on disk; else our approximation from Apple's published curve.
pub const APPLE_CUBE: &str = "luts/AppleLogToRec709.cube";
pub const APPLE_CUBE_APPROX: &str = "luts/AppleLog_to_Rec709_approx.cube";
pub fn phone_cube() -> Option<&'static Path> {
    [APPLE_CUBE, APPLE_CUBE_APPROX].into_iter().map(Path::new).find(|p| p.exists())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Fuji,
    Phone,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Motion {
    Still,
    Pan,
    Push,
    Pull,
}

impl Motion {
    fn name(self) -> &'static str {
        match self {
            Motion::Still => "still",
            Motion::Pan => "pan",
            Motion::Push => "push",
            Motion::Pull => "pull",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub motion: Motion,
    pub start: f64,
    pub end: f64,
}

#[derive(Debug)]
pub struct Clip {
    pub file: PathBuf,
    pub source: Source,
    pub t: NaiveDateTime,
    pub duration: f64,
    /// As displayed: a rotation tag of 90/270 swaps the stored dimensions.
    pub width: u32,
    pub height: u32,
    /// 1-based station; None for the gimbal pass and strays.
    pub station: Option<usize>,
    /// station · detail · vertical · walkup · entry · hallway · lookback · grounds · close · stray
    pub kind: String,
    pub segments: Vec<Segment>,
    pub origin: Option<usize>,
    pub dest: Option<usize>,
    /// Room types where a gimbal clip starts and lands (a station's room, or the realm's nearest type at the gate).
    pub origin_room: String,
    pub dest_room: String,
    pub confidence: String,
}

/// A station as the planner sees it: when it was shot, its final, its room.
pub struct StationInfo {
    pub n: usize,
    pub final_jpg: PathBuf,
    pub first_at: NaiveDateTime,
    pub room: String,
    pub fit: f32,
}

pub const EXTERIOR: [&str; 3] = ["house_view", "porch", "aerial_view"];
pub const GROUNDS: [&str; 2] = ["patio", "yard"];

pub fn clips(video_dir: &Path, out_dir: &Path) -> Result<()> {
    let t0 = std::time::Instant::now();
    // a card with stills: its stations are already measured, and a missing
    // one should fail here rather than after minutes of motion analysis
    let from_stills = out_dir.join(".meta").is_dir().then(|| read_stations(out_dir)).transpose()?;
    let mut clips = probe(video_dir, Source::Fuji)?;
    let phone_dir = video_dir.parent().unwrap_or(Path::new(".")).join("phone");
    if phone_dir.is_dir() {
        clips.extend(probe(&phone_dir, Source::Phone)?);
    }
    if clips.is_empty() {
        bail!("no clips in {}", video_dir.display());
    }
    clips.sort_by(|a, b| (a.source as u8, a.t, a.file.clone()).cmp(&(b.source as u8, b.t, b.file.clone())));
    let phones = clips.iter().filter(|c| c.source == Source::Phone).count();
    eprintln!("{} clips ({} camera, {phones} phone)", clips.len(), clips.len() - phones);

    analyse_motion(&mut clips)?;
    let stations = match from_stills {
        Some(s) => s,
        None => derive_stations(&clips, out_dir)?,
    };
    eprintln!("{} stations", stations.len());
    place(&mut clips, &stations);
    type_by_picture(&mut clips, &stations, out_dir)?;
    gimbal_roles(&mut clips);

    let video_out = out_dir.join("video");
    std::fs::create_dir_all(&video_out)?;
    let tsv = table(&clips);
    std::fs::write(video_out.join("clips.tsv"), &tsv)?;
    print!("{tsv}");
    eprintln!("clips: {:.1} s → {}", t0.elapsed().as_secs_f64(), video_out.join("clips.tsv").display());
    Ok(())
}

fn table(clips: &[Clip]) -> String {
    let st = |n: Option<usize>| n.map(|n| format!("st{n:02}")).unwrap_or_default();
    let mut tsv = String::from("file\tsource\tstation\tkind\torientation\tsegments\torigin\tdestination\tconfidence\n");
    for c in clips {
        tsv += &format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            c.file.file_name().unwrap_or_default().to_string_lossy(),
            if c.source == Source::Fuji { "fuji" } else { "phone" },
            st(c.station),
            c.kind,
            if c.height > c.width { "portrait" } else { "landscape" },
            segments_str(&c.segments),
            st(c.origin),
            st(c.dest),
            c.confidence
        );
    }
    tsv
}

pub fn segments_str(segments: &[Segment]) -> String {
    segments.iter().map(|s| format!("{}:{:.1}-{:.1}", s.motion.name(), s.start, s.end)).collect::<Vec<_>>().join(",")
}

/// A video-only card has no RAFs, so no `retool run`, no `out/.meta` and no
/// finals — but the clips themselves carry everything the rest of the chain
/// asks of a station. Every tripod clip that pans is a station in shoot
/// order (an all-still clip after one is that station's detail, which
/// `place` already sorts out by time); its mid-hold frame, graded through
/// the WDR LUT, stands in for the station final; the realm types it. Writes
/// the `out/.meta/stNN.json` and `out/scorecard.md` that `read_stations`,
/// the planner and the review page read back, then returns the same list.
fn derive_stations(clips: &[Clip], out_dir: &Path) -> Result<Vec<StationInfo>> {
    let station_clips: Vec<&Clip> = clips
        .iter()
        .filter(|c| c.source == Source::Fuji && !has_move(c) && c.segments.iter().any(|s| s.motion == Motion::Pan))
        .collect();
    if station_clips.is_empty() {
        bail!("no stills (out/.meta) and no tripod pan in the video: nothing to derive stations from");
    }
    eprintln!("no out/.meta: deriving {} stations from the video", station_clips.len());
    let realm_path = Path::new("realm.json");
    if !realm_path.exists() {
        bail!("no realm.json in the working directory: a video-only card needs it to type its rooms");
    }
    std::fs::create_dir_all(out_dir.join(".meta"))?;
    let lut = Path::new(WDR_CUBE).exists().then_some(Path::new(WDR_CUBE));
    let mut finals = Vec::new();
    for (i, c) in station_clips.iter().enumerate() {
        let jpg = out_dir.join(format!("st{:02}.jpg", i + 1));
        extract_frame(&c.file, hold_time(c), lut, &jpg)?;
        finals.push(jpg);
    }
    let realm = crate::realm::load(realm_path)?;
    let scored = crate::realm::score_files(&realm, &Embedder::new()?, &finals)?;

    let mut table = format!("# Scorecard — {}\n\n", out_dir.display());
    table += "Stations derived from video (no stills): each row is the mid-hold frame of a tripod clip, \
graded through the WDR LUT. Type and fit are the realm's — correct a row by hand and the planner follows it.\n\n";
    table += "| image | type | fit | verticals | recipe | flags |\n|---|---|---|---|---|---|\n";
    let mut list = Vec::new();
    for (i, (c, s)) in station_clips.iter().zip(&scored).enumerate() {
        let n = i + 1;
        let name = format!("st{n:02}");
        let flags = if s.no_ref { "no similar reference" } else { "" };
        let clip = c.file.file_name().unwrap_or_default().to_string_lossy();
        table += &format!("| {name}.jpg | {} | {:.0} | | from video: {clip} @ {:.1} s | {flags} |\n", s.label, s.fit, hold_time(c));
        let meta = serde_json::json!({ "station": name, "from_clip": clip, "first_at": c.t.format("%Y:%m:%d %H:%M:%S").to_string() });
        std::fs::write(out_dir.join(".meta").join(format!("{name}.json")), serde_json::to_vec_pretty(&meta)?)?;
        list.push(StationInfo { n, final_jpg: finals[i].clone(), first_at: c.t, room: s.label.clone(), fit: s.fit });
    }
    std::fs::write(out_dir.join("scorecard.md"), table)?;
    eprintln!("wrote {} and {} station meta", out_dir.join("scorecard.md").display(), list.len());
    Ok(list)
}

/// The still the station is framed on: the longest hold, else mid-pan.
fn hold_time(c: &Clip) -> f64 {
    let longest = c.segments.iter().filter(|s| s.motion == Motion::Still).max_by(|a, b| (a.end - a.start).total_cmp(&(b.end - b.start)));
    match longest.or_else(|| c.segments.iter().find(|s| s.motion == Motion::Pan)) {
        Some(s) => (s.start + s.end) / 2.0,
        None => c.duration / 2.0,
    }
}

/// Stations from `out/.meta/*.json` (timestamps) and `out/scorecard.md` (room, fit).
pub fn read_stations(out_dir: &Path) -> Result<Vec<StationInfo>> {
    let rooms = scorecard_rooms(out_dir)?;
    let meta_dir = out_dir.join(".meta");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&meta_dir)
        .with_context(|| format!("no {} — run `retool run` first", meta_dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut list = Vec::new();
    for path in paths {
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?).with_context(|| format!("parse {}", path.display()))?;
        let name = v["station"].as_str().with_context(|| format!("{}: no station", path.display()))?.to_string();
        let n: usize = name.trim_start_matches("st").parse().with_context(|| format!("{}: station {name}", path.display()))?;
        let at = |k: &str| -> Result<NaiveDateTime> {
            let s = v[k]
                .as_str()
                .with_context(|| format!("{}: no {k} (retool run writes it since 2026-09-02; re-run on the card or add it by hand)", path.display()))?;
            NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S").with_context(|| format!("{}: bad {k}", path.display()))
        };
        let (room, fit) = rooms.get(&name).cloned().unwrap_or_else(|| ("unknown".into(), 0.0));
        list.push(StationInfo { n, final_jpg: out_dir.join(format!("{name}.jpg")), first_at: at("first_at")?, room, fit });
    }
    list.sort_by_key(|s| s.n);
    if list.is_empty() {
        bail!("no station meta in {}", meta_dir.display());
    }
    Ok(list)
}

/// `| stNN.jpg | type | fit | …` rows of the scorecard table (detail rows skipped).
fn scorecard_rooms(out_dir: &Path) -> Result<HashMap<String, (String, f32)>> {
    let path = out_dir.join("scorecard.md");
    let text = std::fs::read_to_string(&path).with_context(|| format!("read {} (retool run writes it)", path.display()))?;
    let mut map = HashMap::new();
    for line in text.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 4 || !cells[1].starts_with("st") || !cells[1].ends_with(".jpg") || cells[2].starts_with("detail") {
            continue;
        }
        map.insert(cells[1].trim_end_matches(".jpg").to_string(), (cells[2].to_string(), cells[3].parse().unwrap_or(0.0)));
    }
    Ok(map)
}

/// exiftool over one folder of clips. The recipe is 60p everywhere, so
/// anything else is a shooting error the tool refuses to paper over.
pub fn probe(dir: &Path, source: Source) -> Result<Vec<Clip>> {
    let out = Command::new("exiftool")
        .args(["-json", "-q", "-n", "-CreateDate", "-Duration", "-VideoFrameRate", "-ImageWidth", "-ImageHeight", "-Rotation", "-ext", "mov", "-ext", "mp4"])
        .arg(dir)
        .output()
        .context("run exiftool (brew install exiftool)")?;
    if out.stdout.trim_ascii().is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).context("parse exiftool json")?;
    let mut clips = Vec::new();
    for r in rows {
        let file = PathBuf::from(r["SourceFile"].as_str().context("no SourceFile")?);
        let fps = r["VideoFrameRate"].as_f64().unwrap_or(0.0);
        if (fps - 60.0).abs() > 1.0 {
            bail!("{}: {fps:.2} fps — the recipe is 4K 60p, every clip is conformed 2×", file.display());
        }
        let rotation = r["Rotation"].as_f64().unwrap_or(0.0) as i64;
        let (mut width, mut height) = (r["ImageWidth"].as_u64().unwrap_or(0) as u32, r["ImageHeight"].as_u64().unwrap_or(0) as u32);
        if rotation % 180 != 0 {
            std::mem::swap(&mut width, &mut height);
        }
        let t = NaiveDateTime::parse_from_str(r["CreateDate"].as_str().unwrap_or_default(), "%Y:%m:%d %H:%M:%S")
            .with_context(|| format!("{}: bad CreateDate", file.display()))?;
        let duration = r["Duration"].as_f64().with_context(|| format!("{}: no Duration", file.display()))?;
        clips.push(Clip {
            file,
            source,
            t,
            duration,
            width,
            height,
            station: None,
            kind: String::new(),
            segments: Vec::new(),
            origin: None,
            dest: None,
            origin_room: String::new(),
            dest_room: String::new(),
            confidence: String::new(),
        });
    }
    Ok(clips)
}

// ---- motion -------------------------------------------------------------

/// Analysis frame width; 10 samples a second; block-search radius per step.
const AW: usize = 320;
const STEP: f64 = 0.1;
const RADIUS: i32 = 12;
/// Under this many pixels per step (at 320 wide) a clip is holding still.
const STILL_PX: f64 = 0.6;

struct Step {
    tx: f64,
    ty: f64,
    /// Expansion per step: differential displacement of opposite quadrants over their separation.
    zoom: f64,
}

/// Motion for every clip, four ffmpeg decodes at a time.
fn analyse_motion(clips: &mut [Clip]) -> Result<()> {
    let workers = 4.min(clips.len().max(1));
    let results: Vec<(usize, Result<Vec<Segment>>)> = std::thread::scope(|s| {
        let shared = &*clips;
        let handles: Vec<_> = (0..workers)
            .map(|w| {
                s.spawn(move || {
                    shared
                        .iter()
                        .enumerate()
                        .skip(w)
                        .step_by(workers)
                        .map(|(i, c)| {
                            let r = motion(c);
                            if let Ok(segs) = &r {
                                eprintln!("{}: {}", c.file.file_name().unwrap_or_default().to_string_lossy(), segments_str(segs));
                            }
                            (i, r)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("motion worker")).collect()
    });
    for (i, r) in results {
        clips[i].segments = r.with_context(|| format!("motion analysis of {}", clips[i].file.display()))?;
    }
    Ok(())
}

/// Grey frames at 10 fps, 320 wide, straight from ffmpeg's stdout.
fn grey_frames(clip: &Clip) -> Result<(usize, usize, Vec<u8>)> {
    let h = ((AW as f64 * clip.height as f64 / clip.width.max(1) as f64 / 2.0).round() as usize * 2).max(2 * RADIUS as usize * 2 + 10);
    let out = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-hwaccel", "videotoolbox", "-i"])
        .arg(&clip.file)
        .args(["-vf", &format!("fps={},scale={AW}:{h}", 1.0 / STEP), "-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .output()
        .context("run ffmpeg")?;
    if !out.status.success() {
        bail!("ffmpeg: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok((AW, h, out.stdout))
}

fn motion(clip: &Clip) -> Result<Vec<Segment>> {
    let (w, h, data) = grey_frames(clip)?;
    let n = data.len() / (w * h);
    if n < 2 {
        bail!("under two analysis frames");
    }
    let frames: Vec<&[u8]> = (0..n).map(|i| &data[i * w * h..(i + 1) * w * h]).collect();
    let steps: Vec<Step> = (1..n).map(|i| estimate(frames[i - 1], frames[i], w, h)).collect();
    // smooth over three steps, then label
    let mut labels: Vec<Motion> = (0..steps.len())
        .map(|i| {
            let (lo, hi) = (i.saturating_sub(1), (i + 1).min(steps.len() - 1));
            let k = (hi - lo + 1) as f64;
            let s = Step {
                tx: steps[lo..=hi].iter().map(|s| s.tx).sum::<f64>() / k,
                ty: steps[lo..=hi].iter().map(|s| s.ty).sum::<f64>() / k,
                zoom: steps[lo..=hi].iter().map(|s| s.zoom).sum::<f64>() / k,
            };
            label(&s, w)
        })
        .collect();
    // runs shorter than 0.3 s are absorbed by the run before them (after, at the start)
    loop {
        let r = runs(&labels);
        let Some(pos) = r.iter().position(|&(_, a, b)| b - a < 3) else { break };
        if r.len() < 2 {
            break;
        }
        let (_, a, b) = r[pos];
        let fill = if pos > 0 { r[pos - 1].0 } else { r[pos + 1].0 };
        labels[a..b].iter_mut().for_each(|l| *l = fill);
    }
    let mut segments: Vec<Segment> = runs(&labels)
        .into_iter()
        .map(|(m, a, b)| Segment { motion: m, start: a as f64 * STEP, end: b as f64 * STEP })
        .collect();
    if let Some(last) = segments.last_mut() {
        last.end = clip.duration;
    }
    Ok(segments)
}

/// (label, first step, one past last step) for each run of equal labels.
fn runs(labels: &[Motion]) -> Vec<(Motion, usize, usize)> {
    let mut out: Vec<(Motion, usize, usize)> = Vec::new();
    for (i, &l) in labels.iter().enumerate() {
        match out.last_mut() {
            Some((m, _, b)) if *m == l => *b = i + 1,
            _ => out.push((l, i, i + 1)),
        }
    }
    out
}

fn label(s: &Step, w: usize) -> Motion {
    let t = s.tx.hypot(s.ty);
    let z = s.zoom.abs() * (w as f64 / 2.0);
    if t < STILL_PX && z < STILL_PX {
        Motion::Still
    } else if z > t {
        if s.zoom > 0.0 { Motion::Push } else { Motion::Pull }
    } else {
        Motion::Pan
    }
}

/// Global motion between two frames from four quadrant block matches: the
/// mean shift is the translation; opposite quadrants moving apart is a push.
fn estimate(a: &[u8], b: &[u8], w: usize, h: usize) -> Step {
    let r = RADIUS as usize;
    let (bw, bh) = (w / 2 - 2 * r - 4, h / 2 - 2 * r - 4);
    let mut d = [[(0.0f64, 0.0f64); 2]; 2]; // [row][col]
    for (qy, row) in d.iter_mut().enumerate() {
        for (qx, cell) in row.iter_mut().enumerate() {
            *cell = block_shift(a, b, w, qx * (w / 2) + r + 2, qy * (h / 2) + r + 2, bw, bh);
        }
    }
    let tx = (d[0][0].0 + d[0][1].0 + d[1][0].0 + d[1][1].0) / 4.0;
    let ty = (d[0][0].1 + d[0][1].1 + d[1][0].1 + d[1][1].1) / 4.0;
    let zx = ((d[0][1].0 - d[0][0].0) + (d[1][1].0 - d[1][0].0)) / 2.0 / (w as f64 / 2.0);
    let zy = ((d[1][0].1 - d[0][0].1) + (d[1][1].1 - d[0][1].1)) / 2.0 / (h as f64 / 2.0);
    Step { tx, ty, zoom: (zx + zy) / 2.0 }
}

/// The shift (dx, dy) that best lays a's block over b, full search within
/// RADIUS, refined to sub-pixel by a parabola through the three SADs on each axis.
fn block_shift(a: &[u8], b: &[u8], w: usize, x0: usize, y0: usize, bw: usize, bh: usize) -> (f64, f64) {
    let sad = |dx: i32, dy: i32| -> u64 {
        let mut s = 0u64;
        for y in 0..bh {
            let ra = &a[(y0 + y) * w + x0..][..bw];
            let rb = &b[((y0 + y) as i32 + dy) as usize * w + (x0 as i32 + dx) as usize..][..bw];
            s += ra.iter().zip(rb).map(|(p, q)| (*p as i32 - *q as i32).unsigned_abs() as u64).sum::<u64>();
        }
        s
    };
    let mut best = (0i32, 0i32, u64::MAX);
    for dy in -RADIUS..=RADIUS {
        for dx in -RADIUS..=RADIUS {
            let s = sad(dx, dy);
            if s < best.2 {
                best = (dx, dy, s);
            }
        }
    }
    let refine = |l: u64, m: u64, r: u64| -> f64 {
        let den = l as f64 + r as f64 - 2.0 * m as f64;
        if den <= 0.0 { 0.0 } else { ((l as f64 - r as f64) / (2.0 * den)).clamp(-0.5, 0.5) }
    };
    let (bx, by, bs) = best;
    let fx = if bx.abs() < RADIUS { refine(sad(bx - 1, by), bs, sad(bx + 1, by)) } else { 0.0 };
    let fy = if by.abs() < RADIUS { refine(sad(bx, by - 1), bs, sad(bx, by + 1)) } else { 0.0 };
    (bx as f64 + fx, by as f64 + fy)
}

// ---- placement and typing ----------------------------------------------

fn has_move(c: &Clip) -> bool {
    c.segments.iter().any(|s| matches!(s.motion, Motion::Push | Motion::Pull))
}

/// A camera clip that pushes or pulls is a gimbal clip wherever it falls in
/// the shoot (a tripod pan never does); the pass may come first, in daylight,
/// or last. The rest belong to the last station whose brackets started
/// before them.
pub fn place(clips: &mut [Clip], stations: &[StationInfo]) {
    for c in clips.iter_mut().filter(|c| c.source == Source::Fuji) {
        if has_move(c) {
            c.kind = "gimbal".into();
            c.confidence = "by motion".into();
            continue;
        }
        let Some(st) = stations.iter().rev().find(|s| s.first_at <= c.t) else {
            c.kind = "stray".into();
            c.confidence = "before the first station".into();
            continue;
        };
        c.station = Some(st.n);
        // the station clip is the one that pans; an all-still clip is the 18-50 detail static
        c.kind = if c.segments.iter().any(|s| s.motion == Motion::Pan) { "station" } else { "detail" }.into();
        c.confidence = "by time".into();
    }
}

/// Phone clips: the middle frame's nearest station final is the room (the
/// phone's clock is not the camera's). Gimbal clips: first and last frame →
/// origin and destination. Frames are kept under out/video/frames so a
/// doubtful row can be checked by eye.
fn type_by_picture(clips: &mut [Clip], stations: &[StationInfo], out_dir: &Path) -> Result<()> {
    let need: Vec<usize> = clips.iter().enumerate().filter(|(_, c)| c.source == Source::Phone || c.kind == "gimbal").map(|(i, _)| i).collect();
    if need.is_empty() {
        return Ok(());
    }
    let frames_dir = out_dir.join("video").join("frames");
    std::fs::create_dir_all(&frames_dir)?;
    let embedder = Embedder::new()?;
    let finals: Vec<Vec<f32>> = stations.iter().map(|s| embedder.embed(&s.final_jpg)).collect::<Result<_>>()?;
    let lut = Path::new(WDR_CUBE).exists().then_some(Path::new(WDR_CUBE));
    let st = |i: usize| stations[i].n;
    // phone clips: the picture first; the clock for the doubtful ones. The phone's clock
    // runs on its own offset from the camera's, constant through a shoot, so the
    // confident picture matches calibrate it and the weak ones fall back to it.
    let mut picks: Vec<(usize, Vec<(usize, f32)>)> = Vec::new();
    for &i in need.iter().filter(|&&i| clips[i].source == Source::Phone) {
        let stem = clips[i].file.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let jpg = frames_dir.join(format!("{stem}_mid.jpg"));
        extract_frame(&clips[i].file, clips[i].duration / 2.0, phone_cube(), &jpg)?;
        picks.push((i, rank(&embedder.embed(&jpg)?, &finals)));
    }
    let confident = |r: &[(usize, f32)]| r[0].1 <= 0.4 && r.get(1).is_none_or(|n| n.1 - r[0].1 >= 0.05);
    let mut offsets: Vec<i64> = picks.iter().filter(|(_, r)| confident(r)).map(|(i, r)| (clips[*i].t - stations[r[0].0].first_at).num_seconds()).collect();
    offsets.sort();
    let offset = (!offsets.is_empty()).then(|| chrono::Duration::seconds(offsets[offsets.len() / 2]));
    if let Some(o) = offset {
        eprintln!("phone clock runs {} min ahead of the camera ({} confident matches)", o.num_minutes(), offsets.len());
    }
    // station windows; the first opens an hour early, since the facade is shot on arrival
    let windows: Vec<(NaiveDateTime, Option<NaiveDateTime>)> = stations
        .iter()
        .enumerate()
        .map(|(k, s)| (if k == 0 { s.first_at - chrono::Duration::minutes(60) } else { s.first_at }, stations.get(k + 1).map(|n| n.first_at)))
        .collect();
    for (i, ranked) in picks {
        let (pick, d1) = ranked[0];
        let next = ranked.get(1).copied();
        let mut station = Some(st(pick));
        let mut note = String::new();
        if !confident(&ranked) {
            if let Some(o) = offset {
                let t_cam = clips[i].t - o;
                match windows.iter().position(|(a, b)| *a <= t_cam && b.is_none_or(|b| t_cam < b)) {
                    Some(k) => {
                        if st(k) != st(pick) {
                            note = format!(" → st{:02} by clock", st(k));
                        }
                        station = Some(st(k));
                    }
                    None => {
                        station = None;
                        note = " (outside the shoot by clock)".into();
                    }
                }
            }
        }
        clips[i].station = station;
        clips[i].kind = if station.is_some() { "vertical" } else { "stray" }.into();
        clips[i].confidence = match next {
            Some((n2, d2)) => format!("d {d1:.2}, next st{:02} {d2:.2}{note}", st(n2)),
            None => format!("d {d1:.2}{note}"),
        };
    }
    let others: Vec<usize> = need.iter().copied().filter(|&i| clips[i].source != Source::Phone).collect();
    for i in others {
        let stem = clips[i].file.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        {
            let (first, last) = (frames_dir.join(format!("{stem}_first.jpg")), frames_dir.join(format!("{stem}_last.jpg")));
            extract_frame(&clips[i].file, 0.5, lut, &first)?;
            extract_frame(&clips[i].file, (clips[i].duration - 0.5).max(0.0), lut, &last)?;
            let ro = rank(&embedder.embed(&first)?, &finals);
            let rd = rank(&embedder.embed(&last)?, &finals);
            clips[i].origin = Some(st(ro[0].0));
            clips[i].origin_room = stations[ro[0].0].room.clone();
            clips[i].dest = Some(st(rd[0].0));
            clips[i].dest_room = stations[rd[0].0].room.clone();
            clips[i].confidence = match rd.get(1) {
                Some(&(n2, d2)) => format!("dest d {:.2}, next st{:02} {d2:.2}", rd[0].1, st(n2)),
                None => format!("dest d {:.2}", rd[0].1),
            };
        }
    }
    Ok(())
}

/// Station finals by cosine distance to an embedding, nearest first.
fn rank(emb: &[f32], finals: &[Vec<f32>]) -> Vec<(usize, f32)> {
    let mut v: Vec<(usize, f32)> = finals.iter().enumerate().map(|(i, f)| (i, 1.0 - emb.iter().zip(f).map(|(a, b)| a * b).sum::<f32>())).collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v
}

/// One frame at `t`, graded through `lut` when given (log clips embed like the sRGB finals).
pub fn extract_frame(clip: &Path, t: f64, lut: Option<&Path>, out: &Path) -> Result<()> {
    let vf = match lut {
        Some(l) => format!("lut3d=file={},scale=1036:-2", l.display()),
        None => "scale=1036:-2".into(),
    };
    let status = Command::new("ffmpeg")
        .args(["-y", "-loglevel", "error", "-ss", &format!("{t:.2}"), "-i"])
        .arg(clip)
        .args(["-frames:v", "1", "-vf", &vf, "-q:v", "3"])
        .arg(out)
        .status()
        .context("run ffmpeg")?;
    if !status.success() {
        bail!("frame extraction failed for {}", clip.display());
    }
    Ok(())
}

/// The gimbal pass in shoot order: an exterior destination is the walk-up
/// first and the close last; grounds by room; the first interior is the
/// entry; a move without a push is a look-back; the rest are hallways.
fn gimbal_roles(clips: &mut [Clip]) {
    let idx: Vec<usize> = clips.iter().enumerate().filter(|(_, c)| c.kind == "gimbal").map(|(i, _)| i).collect();
    let n = idx.len();
    let mut seen_interior = false;
    for (k, &i) in idx.iter().enumerate() {
        let room = clips[i].dest_room.clone();
        let from_outside = EXTERIOR.contains(&clips[i].origin_room.as_str());
        // the first clip of the pass that starts outside is the walk-up, even when its landing is too dark to match
        let role = if k == 0 && (from_outside || EXTERIOR.contains(&room.as_str())) {
            "walkup"
        } else if EXTERIOR.contains(&room.as_str()) {
            if k == n - 1 { "close" } else { "grounds" }
        } else if GROUNDS.contains(&room.as_str()) {
            "grounds"
        } else if !seen_interior {
            seen_interior = true;
            "entry"
        } else if !has_move(&clips[i]) {
            "lookback"
        } else {
            "hallway"
        };
        clips[i].kind = role.into();
        clips[i].confidence += "; role by order";
    }
}

// ---- clips.tsv, read back for the planner and the gate ---------------------

/// A clip as `clips.tsv` describes it; the last segment ends at the clip's end.
#[derive(Clone, Debug)]
pub struct ClipInfo {
    pub file: String,
    pub source: Source,
    pub station: Option<usize>,
    pub kind: String,
    pub portrait: bool,
    pub segments: Vec<Segment>,
    pub dest: Option<usize>,
}

impl ClipInfo {
    pub fn duration(&self) -> f64 {
        self.segments.last().map(|s| s.end).unwrap_or(0.0)
    }
    /// The opening hold, if the clip starts still.
    pub fn still_a(&self) -> Option<&Segment> {
        self.segments.first().filter(|s| s.motion == Motion::Still)
    }
    pub fn pan(&self) -> Option<&Segment> {
        self.segments.iter().find(|s| s.motion == Motion::Pan)
    }
    /// The whole moving span of a gimbal clip: from the first motion to the last, bookend stills excluded.
    pub fn moving(&self) -> Option<Segment> {
        let first = self.segments.iter().find(|s| s.motion != Motion::Still)?;
        let last = self.segments.iter().rev().find(|s| s.motion != Motion::Still)?;
        Some(Segment { motion: first.motion, start: first.start, end: last.end })
    }
}

pub fn read_clips_tsv(out_dir: &Path) -> Result<Vec<ClipInfo>> {
    let path = out_dir.join("video").join("clips.tsv");
    let text = std::fs::read_to_string(&path).with_context(|| format!("read {} (retool clips writes it)", path.display()))?;
    let st = |s: &str| s.strip_prefix("st").and_then(|n| n.parse().ok());
    let mut clips = Vec::new();
    for line in text.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if c.len() < 8 {
            continue;
        }
        let mut segments = Vec::new();
        for seg in c[5].split(',').filter(|s| !s.is_empty()) {
            let (kind, span) = seg.split_once(':').with_context(|| format!("bad segment {seg}"))?;
            let (a, b) = span.split_once('-').with_context(|| format!("bad segment {seg}"))?;
            let motion = match kind {
                "still" => Motion::Still,
                "pan" => Motion::Pan,
                "push" => Motion::Push,
                "pull" => Motion::Pull,
                _ => bail!("bad segment {seg}"),
            };
            segments.push(Segment { motion, start: a.parse()?, end: b.parse()? });
        }
        clips.push(ClipInfo {
            file: c[0].to_string(),
            source: if c[1] == "phone" { Source::Phone } else { Source::Fuji },
            station: st(c[2]),
            kind: c[3].to_string(),
            portrait: c[4] == "portrait",
            segments,
            dest: st(c[7]),
        });
    }
    Ok(clips)
}

// ---- the gate --------------------------------------------------------------

/// `retool gate <video_dir> <out_dir>`: the video gate on its own, stations from `out/.meta`.
pub fn gate_cmd(video_dir: &Path, out_dir: &Path) -> Result<()> {
    let stations = read_stations(out_dir)?;
    gate(video_dir, &stations).map(|_| ())
}

/// The door check for video: every station has its clip, and the gimbal
/// pass has its set. No finals exist yet at the door, so a move's
/// destination is typed by the realm's nearest reference type. Prints the
/// report; true when nothing needs review.
pub fn gate(video_dir: &Path, stations: &[StationInfo]) -> Result<bool> {
    let mut clips = probe(video_dir, Source::Fuji)?;
    let phone_dir = video_dir.parent().unwrap_or(Path::new(".")).join("phone");
    let phones = if phone_dir.is_dir() { probe(&phone_dir, Source::Phone)?.len() } else { 0 };
    if clips.is_empty() {
        println!("VIDEO: no clips in {}", video_dir.display());
        return Ok(false);
    }
    clips.sort_by(|a, b| (a.t, a.file.clone()).cmp(&(b.t, b.file.clone())));
    analyse_motion(&mut clips)?;
    place(&mut clips, stations);

    let gimbal: Vec<usize> = clips.iter().enumerate().filter(|(_, c)| c.kind == "gimbal").map(|(i, _)| i).collect();
    let realm_path = Path::new("realm.json");
    if !gimbal.is_empty() && realm_path.exists() {
        let realm = crate::realm::load(realm_path)?;
        let embedder = Embedder::new()?;
        let lut = Path::new(WDR_CUBE).exists().then_some(Path::new(WDR_CUBE));
        let dir = std::env::temp_dir().join("retool-gate");
        std::fs::create_dir_all(&dir)?;
        let mut frames = Vec::new();
        for &i in &gimbal {
            let stem = clips[i].file.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            let (first, last) = (dir.join(format!("{stem}_first.jpg")), dir.join(format!("{stem}_last.jpg")));
            extract_frame(&clips[i].file, 0.5, lut, &first)?;
            extract_frame(&clips[i].file, (clips[i].duration - 0.5).max(0.0), lut, &last)?;
            frames.push(first);
            frames.push(last);
        }
        for (k, s) in crate::realm::score_files(&realm, &embedder, &frames)?.into_iter().enumerate() {
            let c = &mut clips[gimbal[k / 2]];
            if k % 2 == 0 { c.origin_room = s.label } else { c.dest_room = s.label }
        }
    } else if !gimbal.is_empty() {
        eprintln!("no realm.json in the working directory: gimbal roles by order only");
    }
    gimbal_roles(&mut clips);

    let mut part = 0;
    for s in stations {
        let station: Vec<&Clip> = clips.iter().filter(|c| c.station == Some(s.n)).collect();
        let main = station.iter().rev().find(|c| c.kind == "station");
        let details = station.iter().filter(|c| c.kind == "detail").count();
        match main {
            Some(c) => println!("st{:02} video: {} {:.0} s {}{}", s.n, c.file.file_name().unwrap_or_default().to_string_lossy(), c.duration, segments_str(&c.segments), if details > 0 { format!(" + {details} detail") } else { String::new() }),
            None => {
                part += 1;
                println!("st{:02} video: NO CLIP -> PART", s.n);
            }
        }
    }
    let roles: Vec<String> = gimbal.iter().map(|&i| format!("{} ({})", clips[i].kind, clips[i].dest_room)).collect();
    println!("gimbal pass: {} clips — {}", gimbal.len(), if roles.is_empty() { "none".to_string() } else { roles.join(" · ") });
    let have = |r: &str| gimbal.iter().any(|&i| clips[i].kind == r);
    let mut review = Vec::new();
    for r in ["walkup", "close"] {
        if !have(r) {
            review.push(format!("no {r}"));
        }
    }
    let wanted: Vec<&str> = ["entry", "hallway", "lookback", "grounds"].into_iter().filter(|r| !have(r)).collect();
    if !wanted.is_empty() {
        println!("gimbal set missing: {}", wanted.join(", "));
    }
    for r in &review {
        println!("REVIEW: {r}");
    }
    let strays = clips.iter().filter(|c| c.kind == "stray").count();
    if strays > 0 {
        println!("{strays} clip(s) before the first station, unplaced");
    }
    let clean = review.is_empty() && part == 0;
    println!(
        "VIDEO: {}/{} stations with a clip, {phones} phone verticals, {} gimbal clips -> {}",
        stations.len() - part,
        stations.len(),
        gimbal.len(),
        if clean { "CLEAN" } else { "REVIEW" }
    );
    Ok(clean)
}
