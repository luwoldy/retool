//! `retool review`: see, tweak and approve the plan before it is built
//! (PLAN-review.md). `plan` ends by writing `out/video/review.html`, a
//! read-only view of the cutlists with thumbnails; `retool review` serves
//! the same page on localhost and takes the operator's slot-preserving
//! edits (swap, slip, reorder, drop a phrase, revive) back into the
//! cutlists through the planner's own writer; Approve writes `approved:`
//! and builds the Resolve project. `retool resolve` is that build from the
//! shell, gate checked.
//!
//! The cutlists stay the plan; the page is a view of them, never a second
//! record. Thumbnails go through the fallback cubes so log footage reads as
//! a picture: recognition, not colour.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use crate::cut::{self, Cutlist, Row};
use crate::video::{self, ClipInfo, Source};

const PORT: u16 = 7877;
const LISTS: [&str; 4] = ["cutlist.md", "reel_01.md", "reel_02.md", "reel_03.md"];
const THUMB_W: u32 = 320;
const OUT_FPS: f64 = 30.0;
const BUILD: &str = "tools/resolve_build.py";
const PAGE: &str = include_str!("review.html");

// ---- the grid ---------------------------------------------------------------

/// A cutlist's rows on their grid: the band each slot sits in (the shelf, a
/// phrase, the close), its beats, and the source seconds it shows.
struct Grid {
    bar: f64,
    shelf: f64,
    splice: f64,
    end: f64,
    bands: Vec<Band>,
    slots: Vec<Slot>,
}

struct Band {
    label: String,
    /// `shelf`, `phrase` or `close`.
    kind: &'static str,
    /// Slots of equal beats can trade places: every phrase, and the cold open.
    on_grid: bool,
}

#[derive(Clone, Copy)]
struct Slot {
    band: usize,
    beats: u32,
    /// Source seconds the slot shows: its output frames, at 60p.
    src: f64,
}

impl Grid {
    fn phrase(&self) -> f64 {
        4.0 * self.bar
    }

    fn new(cl: &Cutlist, master: bool) -> Result<Grid> {
        let bar = 240.0 / cl.num("bpm")?;
        let phrase = 4.0 * bar;
        let end = cl.num("end")?;
        let labels: Vec<String> = cl.get("patterns").unwrap_or("").split(" · ").filter(|s| !s.is_empty()).map(String::from).collect();
        let mut bands: Vec<Band> = Vec::new();
        let (shelf, splice) = if master {
            let n = cl.num("phrases")? as usize;
            let splice = cl.num("splice")?;
            let cold_open = labels.first().is_some_and(|l| l.starts_with("S "));
            bands.push(Band { label: if cold_open { labels[0].clone() } else { "S establish".into() }, kind: "shelf", on_grid: cold_open });
            bands.extend(labels.iter().skip(cold_open as usize).map(|l| Band { label: l.clone(), kind: "phrase", on_grid: true }));
            while bands.len() < n + 1 {
                bands.push(Band { label: format!("P{}", bands.len()), kind: "phrase", on_grid: true });
            }
            bands.push(Band { label: "close".into(), kind: "close", on_grid: false });
            // the header's shelf is rounded to 0.01; splice carries three decimals
            (splice - n as f64 * phrase, splice)
        } else {
            bands.extend(labels.iter().map(|l| Band { label: l.clone(), kind: "phrase", on_grid: true }));
            (0.0, end)
        };
        let beat = bar / 4.0;
        let mut slots = Vec::new();
        // the file's `at` is rounded to 0.01 s; a slack under a frame keeps a row in its band
        let eps = 0.02;
        for (i, r) in cl.rows.iter().enumerate() {
            let next = cl.rows.get(i + 1).map(|x| x.at).unwrap_or(end);
            let band = if master && r.at < shelf - eps {
                0
            } else if master && r.at >= splice - eps {
                bands.len() - 1
            } else {
                ((r.at - shelf + eps) / phrase).floor().max(0.0) as usize + master as usize
            };
            while band >= bands.len() {
                bands.push(Band { label: format!("P{}", bands.len() + 1 - master as usize), kind: "phrase", on_grid: true });
            }
            let beats = ((next - r.at) / beat).round().max(1.0) as u32;
            let src = ((next * OUT_FPS).round() - (r.at * OUT_FPS).round()).max(1.0) / (2.0 * OUT_FPS);
            slots.push(Slot { band, beats, src });
        }
        Ok(Grid { bar, shelf, splice, end, bands, slots })
    }
}

// ---- thumbnails and the page -------------------------------------------------

/// The thumbnail cache: `out/video/thumbs/<stem>_<t×100>.jpg` per clip and
/// time, `stNN.jpg` per station still; what is missing is queued and
/// extracted in parallel at the end.
struct Thumbs {
    dir: PathBuf,
    jobs: Vec<(PathBuf, Vec<String>)>,
}

impl Thumbs {
    /// The frame of `file` at `t` through its source's fallback cube; the
    /// path the page uses, extraction queued when the cache lacks it.
    fn frame(&mut self, job_dir: &Path, c: Option<&ClipInfo>, file: &str, t: f64) -> String {
        let stem = Path::new(file).file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let name = format!("{stem}_{:05}.jpg", (t.max(0.0) * 100.0).round() as u64);
        let out = self.dir.join(&name);
        if !out.exists() && !self.jobs.iter().any(|(o, _)| *o == out) {
            match cut::locate(job_dir, file) {
                Ok(src) => {
                    let phone = c.is_some_and(|c| c.source == Source::Phone);
                    let lut = if phone { video::phone_cube().map(Path::to_path_buf) } else { Some(PathBuf::from(video::WDR_CUBE)).filter(|p| p.exists()) };
                    let vf = match lut {
                        Some(l) => format!("lut3d=file={},scale={THUMB_W}:-2", l.display()),
                        None => format!("scale={THUMB_W}:-2"),
                    };
                    let ss = format!("{t:.2}");
                    self.jobs.push((out.clone(), argv(&["-y", "-loglevel", "error", "-hwaccel", "videotoolbox", "-ss", &ss, "-i", &src.display().to_string(), "-frames:v", "1", "-vf", &vf, "-q:v", "4", &out.display().to_string()])));
                }
                Err(e) => eprintln!("review: {e}; no thumbnail"),
            }
        }
        format!("thumbs/{name}")
    }

    /// The station's final, `out/stNN.jpg`, at thumbnail size.
    fn still(&mut self, out_dir: &Path, n: usize) -> String {
        let name = format!("st{n:02}.jpg");
        let out = self.dir.join(&name);
        if !out.exists() && !self.jobs.iter().any(|(o, _)| *o == out) {
            let vf = format!("scale={THUMB_W}:-2");
            self.jobs.push((out.clone(), argv(&["-y", "-loglevel", "error", "-i", &out_dir.join(&name).display().to_string(), "-vf", &vf, "-q:v", "4", &out.display().to_string()])));
        }
        format!("thumbs/{name}")
    }

    fn run(&self, workers: usize) {
        let jobs = &self.jobs;
        std::thread::scope(|s| {
            for w in 0..workers.min(jobs.len()) {
                s.spawn(move || {
                    for (out, args) in jobs.iter().skip(w).step_by(workers) {
                        match Command::new("ffmpeg").args(args).output() {
                            Ok(o) if o.status.success() => {}
                            Ok(o) => eprintln!("review: {} failed: {}", out.display(), String::from_utf8_lossy(&o.stderr).trim()),
                            Err(e) => eprintln!("review: ffmpeg: {e}"),
                        }
                    }
                });
            }
        });
    }
}

fn argv(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// Rebuild the page from the cutlists on disk: thumbnails for every row's in
/// and out frame, every pool clip's mid frame and every hero still (cached),
/// then `review.html`. Returns the page's path.
pub fn refresh(job_dir: &Path, out_dir: &Path) -> Result<PathBuf> {
    let t0 = std::time::Instant::now();
    let video_dir = out_dir.join("video");
    let thumbs_dir = video_dir.join("thumbs");
    std::fs::create_dir_all(&thumbs_dir)?;
    let job = cut::read_job(job_dir)?;
    let clips = video::read_clips_tsv(out_dir)?;
    let stations = video::read_stations(out_dir)?;
    let station_of = |n: usize| stations.iter().find(|s| s.n == n);
    let room_of = |c: &ClipInfo| c.station.or(c.dest).and_then(station_of).map(|s| s.room.clone()).unwrap_or_default();
    let clip = |file: &str| clips.iter().find(|c| c.file == file);

    let mut lists: Vec<(&str, Cutlist, Grid)> = Vec::new();
    for name in LISTS {
        let path = video_dir.join(name);
        if !path.exists() {
            continue;
        }
        let cl = cut::read_cutlist(&path)?;
        if !cl.rows.is_empty() {
            let grid = Grid::new(&cl, name == "cutlist.md").with_context(|| format!("{}: not on a grid", path.display()))?;
            lists.push((name, cl, grid));
        }
    }
    if lists.first().is_none_or(|l| l.0 != "cutlist.md") {
        bail!("no {} (retool plan writes it)", video_dir.join("cutlist.md").display());
    }
    let mut used: HashMap<&str, Vec<String>> = HashMap::new();
    for (name, cl, _) in &lists {
        for (i, r) in cl.rows.iter().enumerate() {
            used.entry(r.clip.as_str()).or_default().push(format!("{} {} {}", if *name == "cutlist.md" { "master" } else { name.trim_end_matches(".md") }, i + 1, r.role));
        }
    }

    let mut thumbs = Thumbs { dir: thumbs_dir, jobs: Vec::new() };
    let mut lists_json = Vec::new();
    for (name, cl, grid) in &lists {
        let mut rows = Vec::new();
        for (i, r) in cl.rows.iter().enumerate() {
            let c = clip(&r.clip);
            let slot = grid.slots[i];
            let t_out = (r.out_s - 1.0 / (2.0 * OUT_FPS)).max(r.in_s);
            rows.push(json!({
                "n": i + 1, "at": r.at, "band": slot.band, "beats": slot.beats, "src": slot.src,
                "role": r.role, "clip": r.clip, "in": r.in_s, "out": r.out_s, "gain": r.gain,
                "camera": c.is_some_and(|c| c.source == Source::Fuji), "room": r.room, "why": r.why,
                "dur": c.map(|c| c.duration()).unwrap_or(0.0),
                "thumb_in": thumbs.frame(job_dir, c, &r.clip, r.in_s),
                "thumb_out": thumbs.frame(job_dir, c, &r.clip, t_out),
            }));
        }
        let mut bands = Vec::new();
        for (bi, b) in grid.bands.iter().enumerate() {
            // a hero phrase carries its station's still: the eight's clip names the station
            let hero = b.kind == "phrase" && b.label.contains("8-4-4");
            let st = if hero { grid.slots.iter().position(|s| s.band == bi).and_then(|k| clip(&cl.rows[k].clip)).and_then(|c| c.station).and_then(station_of) } else { None };
            let st = st.filter(|s| s.final_jpg.exists());
            bands.push(json!({
                "label": b.label, "kind": b.kind, "on_grid": b.on_grid,
                "still": st.map(|s| thumbs.still(out_dir, s.n)),
                "station": st.map(|s| format!("st{:02} {} · fit {:.0}", s.n, s.room, s.fit)),
            }));
        }
        let mut skipped = Vec::new();
        for (clip_name, room, why) in &cl.skipped {
            let (files, thumb): (Vec<String>, Option<String>) = match clip_name.strip_prefix("st").and_then(|n| n.parse::<usize>().ok()) {
                Some(n) => {
                    let mut fs: Vec<&ClipInfo> = clips.iter().filter(|c| c.station == Some(n)).collect();
                    fs.sort_by_key(|c| c.kind != "station");
                    let thumb = if station_of(n).is_some_and(|s| s.final_jpg.exists()) { Some(thumbs.still(out_dir, n)) } else { fs.first().map(|c| thumbs.frame(job_dir, Some(c), &c.file, c.duration() / 2.0)) };
                    (fs.iter().map(|c| c.file.clone()).collect(), thumb)
                }
                None => (vec![clip_name.clone()], clip(clip_name).map(|c| thumbs.frame(job_dir, Some(c), &c.file, c.duration() / 2.0))),
            };
            skipped.push(json!({ "clip": clip_name, "room": room, "why": why, "clips": files, "thumb": thumb }));
        }
        let header: serde_json::Map<String, Value> = cl.header.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect();
        lists_json.push(json!({ "file": name, "master": *name == "cutlist.md", "header": header, "bands": bands, "rows": rows, "skipped": skipped }));
    }
    let pool: Vec<Value> = clips
        .iter()
        .map(|c| {
            json!({
                "file": c.file, "source": if c.source == Source::Fuji { "fuji" } else { "phone" },
                "station": c.station.map(|n| format!("st{n:02}")).unwrap_or_default(), "room": room_of(c), "kind": c.kind, "portrait": c.portrait,
                "segments": video::segments_str(&c.segments), "duration": c.duration(),
                "thumb": thumbs.frame(job_dir, Some(c), &c.file, c.duration() / 2.0),
                "used": used.get(c.file.as_str()).cloned().unwrap_or_default(),
            })
        })
        .collect();
    let master = &lists[0].1;
    let data = json!({
        "address": job.address, "job_dir": job_dir.display().to_string(), "out_dir": out_dir.display().to_string(),
        "approved": master.approved(),
        "track": { "file": master.get("music").unwrap_or(""), "bpm": master.get("bpm").unwrap_or(""), "accent_s": master.get("accent_s").unwrap_or("") },
        "runtime": master.get("runtime").unwrap_or(""), "patterns": master.get("patterns").unwrap_or(""), "notes": master.get("notes"),
        "lists": lists_json, "pool": pool,
    });
    let n = thumbs.jobs.len();
    thumbs.run(6);
    let page = video_dir.join("review.html");
    std::fs::write(&page, PAGE.replace("/*DATA*/", &data.to_string().replace("</", "<\\/")))?;
    eprintln!("review: {n} thumbnails extracted, {:.1} s → {}", t0.elapsed().as_secs_f64(), page.display());
    Ok(page)
}

// ---- the writer: `retool review` -----------------------------------------------

/// What the page posts to `/save`: one cutlist, its rows in slot order by
/// original number (a swap is a new `clip`, a slip a new `in`/`out`, a
/// reorder a permutation), phrases to drop by label, header lines to set.
#[derive(Deserialize)]
struct Edit {
    cutlist: String,
    #[serde(default)]
    rows: Vec<EditRow>,
    #[serde(default)]
    drop: Vec<String>,
    #[serde(default)]
    header: HashMap<String, String>,
}

#[derive(Deserialize)]
struct EditRow {
    n: usize,
    clip: String,
    #[serde(rename = "in")]
    in_s: f64,
    #[serde(rename = "out")]
    out_s: f64,
}

/// Serve the page on localhost; edits and the approval are written into the cutlists.
pub fn review(job_dir: &Path, out_dir: &Path) -> Result<()> {
    let page = refresh(job_dir, out_dir)?;
    let listener = match TcpListener::bind(("127.0.0.1", PORT)) {
        Ok(l) => l,
        Err(e) => bail!("port {PORT} is in use ({e}); the page is read-only at {}", page.display()),
    };
    let url = format!("http://127.0.0.1:{PORT}/");
    eprintln!("review: {url} — edits go to {}; Ctrl-C to stop", out_dir.join("video").display());
    if Command::new("open").arg(&url).status().is_err() {
        eprintln!("review: could not open a browser; open {url} yourself");
    }
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("review: {e}");
                continue;
            }
        };
        if let Err(e) = handle(&mut stream, job_dir, out_dir) {
            eprintln!("review: {e:#}");
            let _ = respond(&mut stream, "500 Internal Server Error", "text/plain; charset=utf-8", format!("{e:#}").as_bytes());
        }
    }
    Ok(())
}

fn handle(stream: &mut TcpStream, job_dir: &Path, out_dir: &Path) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let (method, target) = (parts.next().unwrap_or("").to_string(), parts.next().unwrap_or("/").to_string());
    let mut len = 0usize;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0; len];
    reader.read_exact(&mut body)?;
    match (method.as_str(), target.as_str()) {
        ("GET", "/") => match refresh(job_dir, out_dir) {
            Ok(p) => respond(stream, "200 OK", "text/html; charset=utf-8", &std::fs::read(p)?),
            Err(e) => respond(stream, "500 Internal Server Error", "text/plain; charset=utf-8", format!("{e:#}").as_bytes()),
        },
        ("GET", t) if t.starts_with("/thumbs/") => {
            let name = &t["/thumbs/".len()..];
            match std::fs::read(out_dir.join("video").join("thumbs").join(name)).ok().filter(|_| !name.contains("..") && !name.contains('/')) {
                Some(b) => respond(stream, "200 OK", "image/jpeg", &b),
                None => respond(stream, "404 Not Found", "text/plain", b"no such thumbnail"),
            }
        }
        ("POST", "/save") => {
            let edit: Edit = serde_json::from_slice(&body).context("bad edit json")?;
            match apply(job_dir, out_dir, &edit) {
                Ok(msg) => {
                    eprintln!("save: {msg}");
                    respond(stream, "200 OK", "text/plain; charset=utf-8", msg.as_bytes())
                }
                Err(e) => {
                    eprintln!("save refused: {e:#}");
                    respond(stream, "400 Bad Request", "text/plain; charset=utf-8", format!("{e:#}").as_bytes())
                }
            }
        }
        ("POST", "/approve") => approve(job_dir, out_dir, stream),
        _ => respond(stream, "404 Not Found", "text/plain", b"no such page"),
    }
}

fn respond(stream: &mut TcpStream, status: &str, ctype: &str, body: &[u8]) -> Result<()> {
    write!(stream, "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len())?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

/// Validate an edit against the grid and write it: every slot keeps its
/// `at` and beats; a swapped clip's window is re-laid by the planner's
/// anchor for the role; a slipped window stays inside the clip; a dropped
/// phrase moves what follows up and the header with it; changed camera rows
/// get their gain measured again.
fn apply(job_dir: &Path, out_dir: &Path, edit: &Edit) -> Result<String> {
    if !LISTS.contains(&edit.cutlist.as_str()) {
        bail!("{} is not one of the job's cutlists", edit.cutlist);
    }
    if let Some(d) = cut::approved(out_dir) {
        bail!("the plan was approved on {d}; remove the `approved:` line from cutlist.md to edit it");
    }
    let master = edit.cutlist == "cutlist.md";
    let path = out_dir.join("video").join(&edit.cutlist);
    let mut cl = cut::read_cutlist(&path)?;
    let grid = Grid::new(&cl, master)?;
    let clips = video::read_clips_tsv(out_dir)?;
    let stations = video::read_stations(out_dir)?;
    let job = cut::read_job(job_dir)?;
    if edit.rows.len() != cl.rows.len() {
        bail!("{} rows sent, the cutlist has {}: reload the page", edit.rows.len(), cl.rows.len());
    }
    let snap = |t: f64| (t * 2.0 * OUT_FPS).round() / (2.0 * OUT_FPS);
    let mut seen = vec![false; cl.rows.len()];
    let mut rows: Vec<Row> = Vec::new();
    let mut changed: Vec<usize> = Vec::new();
    for (k, e) in edit.rows.iter().enumerate() {
        let n = e.n.checked_sub(1).filter(|&n| n < cl.rows.len()).with_context(|| format!("row {}: no such row", e.n))?;
        if std::mem::replace(&mut seen[n], true) {
            bail!("row {} sent twice", e.n);
        }
        let (slot, was) = (grid.slots[k], grid.slots[n]);
        if slot.band != was.band || slot.beats != was.beats {
            bail!("row {}: {} beats in {} cannot take slot {} ({} beats in {})", e.n, was.beats, grid.bands[was.band].label, k + 1, slot.beats, grid.bands[slot.band].label);
        }
        let mut r = cl.rows[n].clone();
        r.at = cl.rows[k].at; // the slot's time, unchanged to the digit
        let c = clips.iter().find(|c| c.file == e.clip);
        if e.clip != r.clip {
            let c = c.with_context(|| format!("row {}: {} is not in clips.tsv", e.n, e.clip))?;
            let (in_s, out_s) = cut::window(cut::anchor_for(&r.role, c, grid.bar), c.duration(), slot.src);
            r.why = format!("{}: {} swapped in by the operator (was {})", r.role, c.kind, r.clip);
            r.clip = e.clip.clone();
            r.in_s = in_s;
            r.out_s = out_s;
            if let Some(s) = c.station.or(c.dest).and_then(|n| stations.iter().find(|s| s.n == n)) {
                r.room = s.room.clone();
            }
            r.gain = 0.0;
            changed.push(k);
        } else if (e.in_s - r.in_s).abs() > 1e-4 || (e.out_s - r.out_s).abs() > 1e-4 {
            let dur = c.map(|c| c.duration()).unwrap_or(f64::MAX);
            if e.in_s < -1e-6 || e.out_s > dur + 1e-3 || e.out_s <= e.in_s {
                bail!("row {}: window {:.3}–{:.3} s is outside {} (0–{:.3} s)", e.n, e.in_s, e.out_s, r.clip, dur);
            }
            r.in_s = snap(e.in_s);
            r.out_s = snap(e.out_s).min(dur);
            changed.push(k);
        }
        rows.push(r);
    }

    let mut dropped: Vec<usize> = Vec::new();
    for label in &edit.drop {
        let b = grid.bands.iter().position(|b| &b.label == label).with_context(|| format!("no phrase `{label}` in {}", edit.cutlist))?;
        if grid.bands[b].kind != "phrase" {
            bail!("`{label}` is not a phrase; only phrases can be dropped");
        }
        dropped.push(b);
    }
    if !dropped.is_empty() {
        let (phrase, beat) = (grid.phrase(), grid.bar / 4.0);
        let mut kept = Vec::new();
        for (k, mut r) in rows.into_iter().enumerate() {
            let b = grid.slots[k].band;
            if dropped.contains(&b) {
                cl.skipped.push((r.clip.clone(), r.room.clone(), format!("{}: dropped by the operator ({})", r.role, grid.bands[b].label)));
                continue;
            }
            let before = dropped.iter().filter(|&&d| d < b).count();
            if before > 0 {
                r.at -= before as f64 * phrase;
                // back onto the beat: the file's `at` is rounded, the grid is not
                let on_grid = grid.shelf + ((r.at - grid.shelf) / beat).round() * beat;
                if (on_grid - r.at).abs() < 0.006 {
                    r.at = on_grid;
                }
            }
            changed.retain(|&c| c != k);
            kept.push(r);
        }
        rows = kept;
        let n_drop = dropped.len() as f64;
        let mut p = 0;
        let patterns: Vec<String> = cl
            .get("patterns")
            .unwrap_or("")
            .split(" · ")
            .filter(|l| !l.is_empty() && !edit.drop.iter().any(|d| d == l))
            .map(|l| match l.strip_prefix('P').and_then(|r| r.split_once(' ')).filter(|(num, _)| !num.is_empty() && num.chars().all(|c| c.is_ascii_digit())) {
                Some((_, rest)) => {
                    p += 1;
                    format!("P{p} {rest}")
                }
                None => l.to_string(),
            })
            .collect();
        set(&mut cl.header, "patterns", patterns.join(" · "));
        let end = grid.end - n_drop * phrase;
        set(&mut cl.header, "end", format!("{end:.3}"));
        if master {
            let n = cl.num("phrases")? as usize - dropped.len();
            set(&mut cl.header, "phrases", n.to_string());
            set(&mut cl.header, "splice", format!("{:.3}", grid.splice - n_drop * phrase));
            set(&mut cl.header, "runtime", format!("{end:.1} s = shelf {:.1} + {n} phrases × {phrase:.1} + ending {:.1}", grid.shelf, grid.end - grid.splice));
        } else {
            if let Some(b) = cl.get("bars").and_then(|b| b.trim().parse::<f64>().ok()) {
                set(&mut cl.header, "bars", format!("{}", b - 4.0 * n_drop));
            }
            set(&mut cl.header, "cuts", rows.len().to_string());
        }
    }
    for (k, v) in &edit.header {
        set(&mut cl.header, k, v.clone());
    }
    // a revived shot leaves the skipped table
    let used_stations: Vec<usize> = rows.iter().filter_map(|r| clips.iter().find(|c| c.file == r.clip)).filter_map(|c| c.station).collect();
    cl.skipped.retain(|(clip, _, _)| !(rows.iter().any(|r| &r.clip == clip) || clip.strip_prefix("st").and_then(|n| n.parse::<usize>().ok()).is_some_and(|n| used_stations.contains(&n))));

    let mut measured: Vec<Row> = changed.iter().map(|&k| rows[k].clone()).collect();
    cut::measure_gains(&mut measured, &clips, job_dir);
    for (r, &k) in measured.into_iter().zip(&changed) {
        rows[k].gain = r.gain;
    }
    cl.rows = rows;
    std::fs::write(&path, cut::write_cutlist(&cl, &job))?;
    refresh(job_dir, out_dir)?;
    Ok(format!("{}: {} rows written, {} changed, {} phrase{} dropped", edit.cutlist, cl.rows.len(), changed.len(), dropped.len(), if dropped.len() == 1 { "" } else { "s" }))
}

fn set(header: &mut Vec<(String, String)>, key: &str, value: String) {
    match header.iter_mut().find(|(k, _)| k == key) {
        Some(kv) => kv.1 = value,
        None => header.push((key.to_string(), value)),
    }
}

/// Write `approved: <date>` into the master cutlist (once), lock the page,
/// then stream the Resolve build's log to the browser as it runs.
fn approve(job_dir: &Path, out_dir: &Path, stream: &mut TcpStream) -> Result<()> {
    let path = out_dir.join("video").join("cutlist.md");
    let mut cl = cut::read_cutlist(&path)?;
    let job = cut::read_job(job_dir)?;
    let date = match cl.approved() {
        Some(d) => d.to_string(),
        None => {
            let d = chrono::Local::now().format("%Y-%m-%d").to_string();
            cl.header.push(("approved".into(), d.clone()));
            std::fs::write(&path, cut::write_cutlist(&cl, &job))?;
            d
        }
    };
    refresh(job_dir, out_dir)?;
    eprintln!("approved: {date} → {}", path.display());
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nTransfer-Encoding: chunked\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n")?;
    let mut chunk = |s: &str| -> Result<()> {
        if !s.is_empty() {
            write!(stream, "{:x}\r\n{s}\r\n", s.len())?;
            stream.flush()?;
        }
        Ok(())
    };
    chunk(&format!("approved: {date} → {}\n\n", path.display()))?;
    let status = build(job_dir, out_dir, &["--replace".to_string()], &mut chunk)?;
    if status.success() {
        chunk(&format!("\nDONE: project `{}` is in Resolve (folder re): the timelines built, render jobs queued, nothing rendered. Open it, QC, Render All.\n", job_dir.file_name().unwrap_or_default().to_string_lossy()))?;
    } else {
        chunk(&format!("\nBUILD FAILED ({status}). The approval stands; build when Resolve is up:\n  retool resolve {} {}\n", job_dir.display(), out_dir.display()))?;
    }
    write!(stream, "0\r\n\r\n")?;
    stream.flush()?;
    Ok(())
}

/// Run `tools/resolve_build.py` from the working directory, its output line
/// by line into `sink`; the exit status says whether Resolve took the project.
fn build(job_dir: &Path, out_dir: &Path, flags: &[String], sink: &mut dyn FnMut(&str) -> Result<()>) -> Result<ExitStatus> {
    if !Path::new(BUILD).exists() {
        bail!("{BUILD} not found: run from the repo root");
    }
    sink(&format!("$ python3 -u {BUILD} {} {} {}\n", job_dir.display(), out_dir.display(), flags.join(" ")))?;
    let mut child = Command::new("sh")
        .args(["-c", "exec python3 -u \"$0\" \"$@\" 2>&1", BUILD])
        .arg(job_dir)
        .arg(out_dir)
        .args(flags)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .context("run python3")?;
    let reader = BufReader::new(child.stdout.take().context("no stdout")?);
    for line in reader.lines() {
        sink(&format!("{}\n", line?))?;
    }
    Ok(child.wait()?)
}

/// `retool resolve <job_dir> <out_dir> [--render none|check|all] [--template <drp>] [--replace] [--unapproved]`:
/// the Resolve build from the shell, gate checked; the flags pass through.
pub fn resolve(job_dir: &Path, out_dir: &Path, flags: &[String]) -> Result<()> {
    let path = out_dir.join("video").join("cutlist.md");
    if !flags.iter().any(|f| f == "--unapproved") {
        cut::require_approved(&cut::read_cutlist(&path)?, &path)?;
    }
    let status = build(job_dir, out_dir, flags, &mut |s| {
        print!("{s}");
        std::io::stdout().flush()?;
        Ok(())
    })?;
    if !status.success() {
        bail!("{BUILD} failed ({status})");
    }
    Ok(())
}
