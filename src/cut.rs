//! The cut. `retool plan` lays the card's clips on the music's grid and
//! writes `out/video/cutlist.md`, the human's control surface; `retool cut`
//! renders a cutlist (PLAN-video-cut.md §3, steps 2–3).
//!
//! Film time starts where the track starts, `accent_s − shelf`. The
//! establish rides the shelf; the cut into the first interior lands on the
//! accent; rooms follow in whole phrases of four bars; at the bar line after
//! phrase N the audio splices to the composed ending and the close plays
//! over it, ending at `end_s`. Every row is 2× (60p → 30p), so a row's
//! source length is half its length on screen.

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::video::{self, ClipInfo, Motion, Source, StationInfo, EXTERIOR, GROUNDS};

const ROTATION: &str = "Documents/ShowReady/Music/ROTATION"; // under $HOME
const SHELF_S: f64 = 12.0;
const RUNTIME_MAX: f64 = 110.0;
const FIT_HERO: f32 = 60.0;
/// A support room under this fit is not shown in the master: build the intrigue (RESEARCH-video-edit.md §9).
const FIT_FLOOR: f32 = 30.0;
/// The grammar (MUSIC.md): a bar is four beats, a phrase four bars. A shot is
/// one bar (details, mediums, support statics) or two (wides, pans, gimbal
/// moves). A phrase is 8-4-4, a wide and its two mediums, or 4-4-8 when a
/// move arrives in the next room; reels open on 4-4-4-4 hook phrases.
/// Exposure nudge before the LUT: F-Log2 mid grey sits at 0.391 and a white-walled
/// room a little above it; one stop moves the log signal by 0.0738.
const LOG_TARGET: f64 = 0.44;
const LOG_STOP: f64 = 0.0738;
const GAIN_RANGE: (f64, f64) = (-1.0, 2.0);
const OUT_FPS: f64 = 30.0;

#[derive(Clone)]
pub struct Track {
    pub file: String,
    pub path: PathBuf,
    /// The catalog id (MUSIC.md `track_id`); empty for a stamp without one.
    pub id: String,
    pub band: String,
    pub bpm: f64,
    pub accent_s: f64,
    /// Where the full arrangement ends after the accent, when stamped (MUSIC.md `shelf_end_s`).
    pub shelf_end_s: Option<f64>,
    pub phrases_after_accent: u32,
    pub ending_start_s: f64,
    pub end_s: f64,
}

impl Track {
    pub fn bar(&self) -> f64 {
        240.0 / self.bpm
    }
    /// The catalog lines a cutlist header carries when the stamp has them: `track:`, `band:`.
    fn catalog_lines(&self) -> Vec<(String, String)> {
        [("track", &self.id), ("band", &self.band)].iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }
    pub fn ending(&self) -> f64 {
        self.end_s - self.ending_start_s
    }
}

pub struct Job {
    pub address: String,
    pub agent: String,
    pub brokerage: String,
    pub phone: String,
    /// `Feature:` in job.md, the selling point the feature reel is built on: a room type or `stNN`.
    pub feature: String,
    pub number: u32,
    pub note: String,
}

/// One row of the cutlist. `in`/`out` are source seconds on the 60p clip;
/// `at` is film time where the row starts; on screen the row runs 2×.
#[derive(Clone, Debug)]
pub struct Row {
    pub at: f64,
    pub role: String,
    pub clip: String,
    pub in_s: f64,
    pub out_s: f64,
    pub room: String,
    pub why: String,
    /// Stops of exposure added before the LUT (camera rows; 0 for the phone).
    pub gain: f64,
}

/// The cutlist as written and read back: header values, rows, skipped rows.
pub struct Cutlist {
    pub header: Vec<(String, String)>,
    pub rows: Vec<Row>,
    pub skipped: Vec<(String, String, String)>, // clip, room, why
}

impl Cutlist {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.header.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    pub fn num(&self, key: &str) -> Result<f64> {
        self.get(key).with_context(|| format!("cutlist header has no {key}:"))?.trim().parse().with_context(|| format!("bad {key}: in cutlist header"))
    }
    /// The `approved: <date>` line, the gate (PLAN-review.md V3).
    pub fn approved(&self) -> Option<&str> {
        self.get("approved").map(str::trim).filter(|d| !d.is_empty())
    }
}

/// The master cutlist's approval date, when `out/video/cutlist.md` carries one.
pub fn approved(out_dir: &Path) -> Option<String> {
    read_cutlist(&out_dir.join("video").join("cutlist.md")).ok().and_then(|c| c.approved().map(String::from))
}

/// Nothing is built from a plan nobody signed off: `retool cut`, `retool resolve`
/// and `tools/resolve_build.py` refuse a cutlist without `approved:`.
pub fn require_approved(cl: &Cutlist, path: &Path) -> Result<()> {
    if cl.approved().is_none() {
        bail!("{} is not approved: `retool review <job_dir> <out_dir>` to review and approve, or --unapproved to build anyway", path.display());
    }
    Ok(())
}

/// Lay the card on the grid and write the cutlists, then the review page.
/// An approved cutlist is not overwritten unless `replan`, which discards
/// the approval; header overrides (`music:`, `shelf:`) survive either way.
pub fn plan(job_dir: &Path, out_dir: &Path, replan: bool) -> Result<()> {
    let stations = video::read_stations(out_dir)?;
    let clips = video::read_clips_tsv(out_dir)?;
    let mut job = read_job(job_dir)?;
    let path = out_dir.join("video").join("cutlist.md");
    let overrides: HashMap<String, String> = read_cutlist(&path).map(|c| c.header.into_iter().collect()).unwrap_or_default();
    if let Some(d) = overrides.get("approved").map(|d| d.trim()).filter(|d| !d.is_empty()) {
        if !replan {
            bail!("{} is approved ({d}); `--replan` discards the approval and plans again", path.display());
        }
        eprintln!("{}: approved on {d}; --replan discards the approval", path.display());
    }
    let mut track = choose_track(overrides.get("music").map(String::as_str), job.number)?;
    // a stamp past the file's end would leave the picture without music at the close
    if let Some(d) = file_duration(&track.path) {
        if track.end_s > d {
            let clamped = (d * 10.0).floor() / 10.0;
            job.note = format!("{}{}: end_s {:.1} is past the file's end ({d:.2} s), clamped to {clamped:.1}; fix the stamp in MUSIC.md", if job.note.is_empty() { String::new() } else { format!("{}; ", job.note) }, track.file, track.end_s);
            track.end_s = clamped;
        }
    }
    if let Some(se) = track.shelf_end_s {
        let implied = ((se - track.accent_s) / (4.0 * track.bar())).floor().max(0.0) as u32;
        if implied != track.phrases_after_accent {
            job.note = format!("{}{}: shelf_end_s {se:.1} implies {implied} full phrases after the accent, the stamp says {}", if job.note.is_empty() { String::new() } else { format!("{}; ", job.note) }, track.file, track.phrases_after_accent);
        }
    }
    let shelf = overrides.get("shelf").and_then(|s| s.trim().parse::<f64>().ok()).unwrap_or(SHELF_S).min(track.accent_s);
    let mut cutlist = build(&clips, &stations, &track, shelf, &job)?;
    measure_gains(&mut cutlist.rows, &clips, job_dir);
    let md = write_cutlist(&cutlist, &job);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, &md)?;
    print!("{md}");
    eprintln!("→ {}", path.display());
    // the reels ride the master's track from the accent: one song per job (2026-09-05)
    for k in 1..=3 {
        let _ = std::fs::remove_file(path.with_file_name(format!("reel_{k:02}.md")));
    }
    for (k, reel) in build_reels(&clips, &stations, &track, &job).iter_mut().enumerate() {
        measure_gains(&mut reel.rows, &clips, job_dir);
        let rp = path.with_file_name(format!("reel_{:02}.md", k + 1));
        std::fs::write(&rp, write_cutlist(reel, &job))?;
        eprintln!("→ {} ({}, {} rows, {})", rp.display(), reel.get("lead").unwrap_or(""), reel.rows.len(), reel.get("native").unwrap_or(""));
    }
    crate::review::refresh(job_dir, out_dir)?;
    eprintln!("review the plan: retool review {} {}", job_dir.display(), out_dir.display());
    Ok(())
}

// ---- job, music -----------------------------------------------------------

/// `job.md`: title `# <address> — <agent>, <brokerage>`, a `Phone:` line;
/// the job number is the ledger row (jobs.tsv) with the same address.
pub fn read_job(job_dir: &Path) -> Result<Job> {
    let path = job_dir.join("job.md");
    let text = std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let title = text.lines().find(|l| l.starts_with("# ")).context("job.md has no `# <address> — <agent>, <brokerage>` title")?[2..].trim();
    let (address, rest) = title.split_once(" — ").unwrap_or((title, ""));
    let (agent, brokerage) = rest.split_once(", ").unwrap_or((rest, ""));
    let phone = text.lines().find_map(|l| l.strip_prefix("Phone:")).unwrap_or("").trim().to_string();
    let feature = text.lines().find_map(|l| l.strip_prefix("Feature:")).unwrap_or("").trim().trim_start_matches('<').trim_end_matches('>').to_string();
    let feature = if feature.contains("selling point") { String::new() } else { feature };
    let (number, note) = ledger_number(address);
    Ok(Job { address: address.to_string(), agent: agent.trim().to_string(), brokerage: brokerage.trim().to_string(), phone, feature, number, note })
}

fn ledger_number(address: &str) -> (u32, String) {
    let Ok(text) = std::fs::read_to_string("jobs.tsv") else { return (0, "no jobs.tsv in the working directory".into()) };
    let mut lines = text.lines();
    let header: Vec<&str> = lines.next().unwrap_or("").split('\t').collect();
    let (Some(ji), Some(ai)) = (header.iter().position(|h| *h == "job"), header.iter().position(|h| *h == "address")) else {
        return (0, "jobs.tsv has no job/address columns".into());
    };
    for line in lines {
        let c: Vec<&str> = line.split('\t').collect();
        if c.get(ai).is_some_and(|a| a.trim().eq_ignore_ascii_case(address.trim())) {
            return (c.get(ji).and_then(|j| j.trim().parse().ok()).unwrap_or(0), String::new());
        }
    }
    (0, format!("`{address}` is not in jobs.tsv; job 0"))
}

/// The stamp table in MUSIC.md, read by column name: file, bpm, accent_s,
/// phrases_after_accent, ending_start_s and end_s make a stamp; track_id, band
/// and shelf_end_s are read when the column is there.
pub(crate) fn read_stamps() -> Result<Vec<Track>> {
    let text = std::fs::read_to_string("MUSIC.md").context("read MUSIC.md from the working directory")?;
    let dir = std::env::var("HOME").map(|h| Path::new(&h).join(ROTATION)).unwrap_or_default();
    let mut cols: HashMap<String, usize> = HashMap::new();
    let mut tracks = Vec::new();
    for line in text.lines() {
        if !line.starts_with('|') {
            continue;
        }
        let c: Vec<&str> = line.split('|').map(str::trim).collect();
        if c.contains(&"file") && c.contains(&"bpm") && c.contains(&"accent_s") {
            cols = c.iter().enumerate().map(|(i, x)| (x.to_string(), i)).collect();
            continue;
        }
        if cols.is_empty() || c.len() < 4 || c[1].starts_with("---") {
            continue;
        }
        let cell = |name: &str| cols.get(name).and_then(|&i| c.get(i)).map(|x| x.to_string()).unwrap_or_default();
        let num = |name: &str| cell(name).parse::<f64>().ok();
        let file = cell("file");
        if !file.contains('.') {
            continue;
        }
        match (num("bpm"), num("accent_s"), num("phrases_after_accent"), num("ending_start_s"), num("end_s")) {
            (Some(bpm), Some(accent_s), Some(phr), Some(ending_start_s), Some(end_s)) => tracks.push(Track {
                path: dir.join(&file),
                file,
                id: cell("track_id"),
                band: cell("band"),
                bpm,
                accent_s,
                shelf_end_s: num("shelf_end_s"),
                phrases_after_accent: phr as u32,
                ending_start_s,
                end_s,
            }),
            _ => eprintln!("MUSIC.md: {file} has no complete stamp, skipped"),
        }
    }
    Ok(tracks)
}

/// The rotation folder's stamped tracks in order, starting at `job mod count`;
/// a track without a stamp is passed over.
pub(crate) fn rotation_tracks(job: u32) -> Result<Vec<Track>> {
    let stamps = read_stamps()?;
    let dir = std::env::var("HOME").map(|h| Path::new(&h).join(ROTATION)).unwrap_or_default();
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .with_context(|| format!("music folder {}", dir.display()))?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|f| !f.starts_with('.'))
        .collect();
    files.sort();
    if files.is_empty() {
        bail!("no tracks in {}", dir.display());
    }
    let start = job as usize % files.len();
    let mut out = Vec::new();
    for k in 0..files.len() {
        let f = &files[(start + k) % files.len()];
        match stamps.iter().find(|t| &t.file == f) {
            Some(t) => {
                let mut t = t.clone();
                t.path = dir.join(f);
                out.push(t)
            }
            None => eprintln!("{f}: no stamp in MUSIC.md, passing over"),
        }
    }
    if out.is_empty() {
        bail!("no track in {} has a stamp in MUSIC.md", dir.display());
    }
    Ok(out)
}

/// The master's track: the cutlist header's `music:` if set, else the first of the rotation.
fn choose_track(wanted: Option<&str>, job: u32) -> Result<Track> {
    if let Some(w) = wanted {
        let w = w.trim();
        let name = Path::new(w).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        let mut t = read_stamps()?.into_iter().find(|t| t.file == name).with_context(|| format!("music: {w} has no stamp in MUSIC.md"))?;
        if Path::new(w).exists() {
            t.path = PathBuf::from(w);
        }
        return Ok(t);
    }
    Ok(rotation_tracks(job)?.remove(0))
}

/// Seconds of audio in a file, by ffprobe.
fn file_duration(path: &Path) -> Option<f64> {
    let o = std::process::Command::new("ffprobe").args(["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0"]).arg(path).output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().parse().ok()
}

// ---- the planner ----------------------------------------------------------

/// How a row picks its window inside the clip once its length is known.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Anchor {
    /// The window ends here (a hold before the pan, the tail still of a move).
    End(f64),
    /// The window starts here (the pan, contiguous with its wide).
    From(f64),
    /// The window is centred here (a move, a detail).
    Centre(f64),
}

/// A shot on the grid, in beats: twos on the reels, fours and eights on the master.
#[derive(Clone, Debug)]
struct Item {
    role: &'static str,
    clip: usize,
    room: String,
    why: String,
    beats: u32,
    /// Fixed length on screen (establish and close), else `bars × bar`.
    screen_s: Option<f64>,
    anchor: Anchor,
}

/// A phrase of the grid, sixteen beats, and the pattern its items make.
struct Phrase {
    items: Vec<Item>,
    label: String,
}

/// A hero room is an eight with its fours and a reserve; a support room over
/// the floor a glimpse (a four that only a montage shows); a move an eight
/// that closes the arrival phrase before its room.
enum Unit {
    Eight { eight: Item, fours: Vec<Item>, reserve: Vec<Item> },
    Move(Item),
    Glimpse(Item),
}

fn build(clips: &[ClipInfo], stations: &[StationInfo], track: &Track, shelf: f64, job: &Job) -> Result<Cutlist> {
    let bar = track.bar();
    let mut skipped: Vec<(String, String, String)> = Vec::new();
    // the first clip of a kind is the station's (wide first, then tighter angles); a second panning clip is a second angle
    let by_station = |n: usize, kind: &str| clips.iter().position(|c| c.station == Some(n) && c.kind == kind);
    let second_clip = |n: usize| clips.iter().enumerate().filter(|(_, c)| c.station == Some(n) && c.kind == "station").nth(1).map(|(i, _)| i);
    let room_of = |n: usize| stations.iter().find(|s| s.n == n).map(|s| s.room.clone()).unwrap_or_default();
    let is_exterior = |s: &StationInfo| EXTERIOR.contains(&s.room.as_str());
    let is_grounds = |s: &StationInfo| GROUNDS.contains(&s.room.as_str());

    // the room sequence: interiors in station order, then grounds
    let mut seq: Vec<&StationInfo> = stations.iter().filter(|s| !is_exterior(s) && !is_grounds(s)).collect();
    seq.extend(stations.iter().filter(|s| is_grounds(s)));
    // consecutive same-type stations beyond two collapse to the best two by fit
    let mut kept: Vec<&StationInfo> = Vec::new();
    let mut i = 0;
    while i < seq.len() {
        let mut j = i;
        while j < seq.len() && seq[j].room == seq[i].room {
            j += 1;
        }
        let run = &seq[i..j];
        if run.len() > 2 {
            let mut best: Vec<&StationInfo> = run.to_vec();
            best.sort_by(|a, b| b.fit.total_cmp(&a.fit));
            let keep: Vec<usize> = best[..2].iter().map(|s| s.n).collect();
            for s in run {
                if keep.contains(&s.n) {
                    kept.push(s);
                } else {
                    skipped.push((format!("st{:02}", s.n), s.room.clone(), format!("duplicate {} (fit {:.0}), best two kept", s.room, s.fit)));
                }
            }
        } else {
            kept.extend(run);
        }
        i = j;
    }

    // hero: living, kitchen, the first bedroom and bathroom, any fit ≥ 60
    let first_of = |room: &str| kept.iter().find(|s| s.room == room).map(|s| s.n);
    let (first_bed, first_bath) = (first_of("bedroom"), first_of("bathroom"));
    let hero_why = |s: &StationInfo| -> Option<String> {
        if s.room == "living_room" || s.room == "kitchen" {
            Some(format!("hero {}, fit {:.0}", s.room, s.fit))
        } else if Some(s.n) == first_bed {
            Some(format!("hero primary bedroom (first bedroom), fit {:.0}", s.fit))
        } else if Some(s.n) == first_bath {
            Some(format!("hero primary bath (first bathroom), fit {:.0}", s.fit))
        } else if s.fit >= FIT_HERO {
            Some(format!("hero by fit {:.0}", s.fit))
        } else {
            None
        }
    };

    // moves by destination: eight beats, the motion centred, or a long walk's landing (in mid-walk, out on the tail still)
    let gimbal: Vec<usize> = clips.iter().enumerate().filter(|(_, c)| ["entry", "hallway", "grounds", "lookback"].contains(&c.kind.as_str())).map(|(i, _)| i).collect();
    let move_item = |ci: usize, why: String| -> Item {
        let c = &clips[ci];
        let (anchor, walk) = move_anchor(c, bar);
        let why = if walk > bar { format!("{why} (8; walk {walk:.1} s, its end)") } else { format!("{why} (8)") };
        Item { role: "move", clip: ci, room: c.dest.map(room_of).unwrap_or_default(), why, beats: 8, screen_s: None, anchor }
    };

    // a move lands on the station that stands for its destination: the same-type
    // neighbour that kept it as a second angle, or that survived the duplicate collapse
    let stands_for = |dest: usize| -> Option<usize> {
        if let Some(pos) = kept.iter().position(|s| s.n == dest) {
            let s = kept[pos];
            return Some(if pos > 0 && kept[pos - 1].room == s.room && hero_why(kept[pos - 1]).is_some() { kept[pos - 1].n } else { s.n });
        }
        let room = room_of(dest);
        seq.iter().filter(|s| s.room == room).min_by_key(|s| (s.n as i64 - dest as i64).abs()).and_then(|s| kept.iter().find(|k| k.room == room && k.n <= s.n.max(1)).map(|k| k.n)).or_else(|| kept.iter().find(|k| k.room == room).map(|k| k.n))
    };
    let four = |role: &'static str, ci: usize, room: &str, why: String, anchor: Anchor| Item { role, clip: ci, room: room.to_string(), why, beats: 4, screen_s: None, anchor };
    let centre = |ci: usize| Anchor::Centre(clips[ci].duration() / 2.0);
    let phrase = 4.0 * bar;

    // the cold open: tease · tease · walk-up landing on the accent, when the intro has a phrase in it
    // and the card two teases: the best details by hero fit, from different rooms when it can
    let walk = clips.iter().rposition(|c| c.kind == "walkup");
    let front = stations.iter().find(|s| is_exterior(s)).and_then(|s| by_station(s.n, "station").map(|ci| (s, ci)));
    let mut teases: Vec<usize> = Vec::new();
    if track.accent_s >= phrase - 1e-6 && (walk.is_some() || front.is_some()) {
        let mut cands: Vec<(f32, usize, String)> = Vec::new();
        for s in kept.iter().filter(|s| hero_why(s).is_some()) {
            let has_station_clip = by_station(s.n, "station").is_some();
            for (i, c) in clips.iter().enumerate() {
                if c.station == Some(s.n) && (c.kind == "vertical" || (c.kind == "detail" && has_station_clip)) {
                    cands.push((s.fit, i, s.room.clone()));
                }
            }
        }
        cands.sort_by(|a, b| b.0.total_cmp(&a.0));
        let spare_room = |room: &str| cands.iter().filter(|(_, _, r)| r == room).count() >= 2;
        for pass in 0..3 {
            for (_, ci, room) in &cands {
                let fresh = match pass {
                    0 => spare_room(room) && !cands.iter().any(|(_, t, r)| teases.contains(t) && r == room),
                    1 => spare_room(room) && !teases.contains(ci),
                    _ => !teases.contains(ci),
                };
                if teases.len() < 2 && fresh {
                    teases.push(*ci);
                }
            }
        }
        if teases.len() < 2 {
            teases.clear();
        }
    }
    let cold_open = teases.len() == 2;

    let mut units: Vec<Unit> = Vec::new();
    let mut k = 0;
    let mut grounds_started = false;
    while k < kept.len() {
        let s = kept[k];
        let next_same = kept.get(k + 1).filter(|n| n.room == s.room).copied();
        // the clip that pans; failing that, a static (the detail clip) stands in for the wide
        let Some(ci) = by_station(s.n, "station").or_else(|| by_station(s.n, "detail")) else {
            skipped.push((format!("st{:02}", s.n), s.room.clone(), "no station clip".into()));
            k += 1;
            continue;
        };
        let c = &clips[ci];
        let no_pan = if c.kind == "detail" { " (static: no pan shot)" } else { "" };
        let hold = hold_anchor(c, bar);
        let hero = hero_why(s);
        // moves close the arrival phrase before a hero room's, once; the grounds move before the first grounds station
        for &gi in &gimbal {
            let g = &clips[gi];
            let lands_here = g.dest.and_then(stands_for) == Some(s.n) || (g.kind == "grounds" && is_grounds(s) && !grounds_started);
            let placed = units.iter().any(|u| matches!(u, Unit::Move(m) if m.clip == gi));
            if lands_here && hero.is_some() && g.kind != "lookback" && !placed {
                units.push(Unit::Move(move_item(gi, format!("{} move → {}", g.kind, g.dest.map(room_of).unwrap_or_default()))));
            }
        }
        if is_grounds(s) {
            grounds_started = true;
        }
        if let Some(hw) = hero {
            // the eight: the pan when the clip has one (a bar of source shows two bars on screen), else the hold
            let eight = match c.pan() {
                Some(p) => {
                    let short = (p.end - p.start) - bar;
                    let why = if short > 0.05 { format!("{hw}: pan, cut {short:.1} s short (8)") } else { format!("{hw}: whole pan + slack (8)") };
                    Item { role: "pan", clip: ci, room: s.room.clone(), why, beats: 8, screen_s: None, anchor: Anchor::From(p.start) }
                }
                None => Item { role: "wide", clip: ci, room: s.room.clone(), why: format!("{hw}{no_pan}: wide (8)"), beats: 8, screen_s: None, anchor: hold },
            };
            // the fours: a medium (the next same-type station's hold, else the station's second clip), then a detail (18-50 static, else a vertical)
            let mut fours: Vec<Item> = Vec::new();
            if let Some(n) = next_same {
                match by_station(n.n, "station") {
                    Some(ai) => fours.push(four("angle", ai, &n.room, format!("second angle st{:02}, fit {:.0} (4)", n.n, n.fit), hold_anchor(&clips[ai], bar))),
                    None => skipped.push((format!("st{:02}", n.n), n.room.clone(), "no station clip".into())),
                }
                k += 1; // the second angle stands for that station
            } else if let Some(ai) = second_clip(s.n) {
                fours.push(four("angle", ai, &s.room, "second angle, the station's second clip (4)".into(), hold_anchor(&clips[ai], bar)));
            }
            // the unit's verticals: this station's and the second angle's, less the cold open's teases
            let unit = [Some(s.n), next_same.map(|n| n.n)];
            let mut verticals: Vec<usize> = clips.iter().enumerate().filter(|(i, v)| v.kind == "vertical" && unit.contains(&v.station) && !teases.contains(i)).map(|(i, _)| i).collect();
            if let Some(di) = by_station(s.n, "detail").filter(|&di| di != ci && !teases.contains(&di)) {
                fours.push(four("detail", di, &s.room, "detail static, 18-50 (4)".into(), centre(di)));
            } else if !verticals.is_empty() {
                let vi = verticals.remove(0);
                fours.push(four("detail", vi, &s.room, "phone vertical, centre-cropped to 16:9 (4)".into(), centre(vi)));
            }
            // the reserve completes the phrase, carries the arrival that follows, or feeds a montage: the room's other
            // verticals, then the clip's other hold (which completes a phrase but never goes into a montage)
            let mut reserve: Vec<Item> = verticals.iter().map(|&vi| four("detail", vi, &s.room, "phone vertical, centre-cropped to 16:9 (4)".into(), centre(vi))).collect();
            let other_hold = c.segments.iter().rev().find(|x| x.motion == Motion::Still).filter(|_| c.pan().is_some()).map(|x| Anchor::From(x.start)).unwrap_or(hold);
            reserve.push(four("wide", ci, &s.room, "the wide's own hold (4; nothing else on the card)".into(), other_hold));
            units.push(Unit::Eight { eight, fours, reserve });
            for &gi in &gimbal {
                if clips[gi].kind == "lookback" && clips[gi].dest.and_then(stands_for) == Some(s.n) {
                    units.push(Unit::Move(move_item(gi, format!("look-back in {}", s.room))));
                }
            }
        } else if s.fit >= FIT_FLOOR {
            let why = format!("{}{no_pan}: glimpse (4)", if is_grounds(s) { "grounds".to_string() } else { format!("support, fit {:.0}", s.fit) });
            units.push(Unit::Glimpse(four("static", ci, &s.room, why, hold)));
        } else {
            skipped.push((format!("st{:02}", s.n), s.room.clone(), format!("not shown: build the intrigue (fit {:.0}, floor {FIT_FLOOR:.0})", s.fit)));
        }
        k += 1;
    }
    for &gi in &gimbal {
        if !units.iter().any(|u| matches!(u, Unit::Move(m) if m.clip == gi)) {
            let g = &clips[gi];
            skipped.push((g.file.clone(), g.dest.map(room_of).unwrap_or_default(), format!("{} move: its room is not shown", g.kind)));
        }
    }

    // tile into phrases: a hero opens 8-4-4 on its own material; a move closes 4-4-8 on the last hero's
    // spare teases; after every second hero, four teases (spare verticals and details, the glimpses of
    // support rooms over the floor) make a 4-4-4-4 montage; grounds glimpses land in the last one
    let mut phrases: Vec<Phrase> = Vec::new();
    let mut spare: Vec<Item> = Vec::new(); // the last hero's unused reserve
    let mut montage: Vec<Item> = Vec::new(); // teases waiting for a montage, in order
    let mut heroes_since = 0;
    let montage_label = |items: &[Item]| {
        let mut rooms: Vec<&str> = Vec::new();
        for i in items {
            if !rooms.contains(&i.room.as_str()) {
                rooms.push(&i.room);
            }
        }
        format!("4-4-4-4 montage: {}", rooms.join(", "))
    };
    for u in units {
        match u {
            Unit::Glimpse(g) => montage.push(g),
            Unit::Eight { eight, mut fours, mut reserve } => {
                montage.extend(spare.drain(..).filter(|i| i.role != "wide"));
                while fours.len() < 2 && !reserve.is_empty() {
                    fours.push(reserve.remove(0));
                }
                while fours.len() < 2 {
                    let dur = clips[eight.clip].duration();
                    fours.push(Item { role: "wide", clip: eight.clip, room: eight.room.clone(), why: "the hold again (4; nothing else on the card)".into(), beats: 4, screen_s: None, anchor: Anchor::Centre(dur * if fours.is_empty() { 0.25 } else { 0.75 }) });
                }
                fours.truncate(2);
                let label = format!("8-4-4 {}", eight.room);
                let mut items = vec![eight];
                items.extend(fours);
                phrases.push(Phrase { items, label });
                spare = reserve;
                heroes_since += 1;
                if heroes_since >= 2 && montage.len() >= 4 {
                    let items: Vec<Item> = montage.drain(..4).collect();
                    phrases.push(Phrase { label: montage_label(&items), items });
                    heroes_since = 0;
                }
            }
            Unit::Move(mv) => {
                if spare.len() >= 2 {
                    let mut items: Vec<Item> = spare.drain(..2).collect();
                    let label = format!("4-4-8 {}", mv.why.split(" (").next().unwrap_or(""));
                    items.push(mv);
                    phrases.push(Phrase { items, label });
                } else {
                    skipped.push((clips[mv.clip].file.clone(), mv.room, "move: no two teases to arrive on".into()));
                }
            }
        }
    }
    montage.extend(spare.drain(..).filter(|i| i.role != "wide"));
    while montage.len() >= 4 {
        let items: Vec<Item> = montage.drain(..4).collect();
        phrases.push(Phrase { label: montage_label(&items), items });
    }
    for g in montage.drain(..) {
        skipped.push((clips[g.clip].file.clone(), g.room, format!("{}: no montage slot (fewer than four teases left)", g.role)));
    }

    // the shelf: the cold open is a phrase; else the walk-up held on the old 12 s shelf
    let mut establish: Vec<Item> = Vec::new();
    let mut shelf = shelf;
    let mut shelf_label = String::new();
    if cold_open {
        shelf = phrase;
        for &ti in &teases {
            let c = &clips[ti];
            establish.push(Item { role: "tease", clip: ti, room: c.station.map(room_of).unwrap_or_default(), why: "cold open: a tease before the house (4)".into(), beats: 4, screen_s: None, anchor: centre(ti) });
        }
        if let Some(wi) = walk {
            let mut e = move_item(wi, "walk-up, the arrival".into());
            e.role = "establish";
            e.why += "; address card";
            establish.push(e);
        } else if let Some((s, ci)) = front {
            establish.push(Item { role: "establish", clip: ci, room: s.room.clone(), why: "front exterior held, the arrival (8); address card".into(), beats: 8, screen_s: None, anchor: Anchor::From(0.0) });
        }
        shelf_label = "S 4-4-8 cold open · ".into();
    } else if let Some(wi) = walk {
        let w = &clips[wi];
        let on_screen = 2.0 * w.duration();
        if on_screen < shelf {
            match front {
                Some((s, ci)) => establish.push(Item { role: "establish", clip: ci, room: s.room.clone(), why: "front exterior, fills the shelf before the walk-up".into(), beats: 0, screen_s: Some(shelf - on_screen), anchor: Anchor::From(0.0) }),
                None => shelf = on_screen,
            }
        }
        let m = w.moving().map(|m| (m.start + m.end) / 2.0).unwrap_or(w.duration() / 2.0);
        establish.push(Item { role: "establish", clip: wi, room: w.dest.map(room_of).unwrap_or_else(|| "exterior".into()), why: "walk-up; address card".into(), beats: 0, screen_s: Some(shelf.min(on_screen)), anchor: Anchor::Centre(m) });
    } else if let Some((s, ci)) = front {
        let on_screen = 2.0 * clips[ci].duration();
        shelf = shelf.min(on_screen);
        establish.push(Item { role: "establish", clip: ci, room: s.room.clone(), why: "front exterior held (no walk-up); address card".into(), beats: 0, screen_s: Some(shelf), anchor: Anchor::From(0.0) });
    } else if let Some(first) = phrases.first().and_then(|p| p.items.first()) {
        let c = &clips[first.clip];
        shelf = shelf.min(2.0 * c.duration());
        establish.push(Item { role: "establish", clip: first.clip, room: first.room.clone(), why: "no exterior on the card: first room held; address card".into(), beats: 0, screen_s: Some(shelf), anchor: Anchor::From(0.0) });
    } else {
        bail!("nothing to cut: no station clips on the card");
    }

    // the runtime cap and the track's usable phrases: a montage goes first (from the end), then an arrival, then the last phrase; never the first
    let mut note = String::new();
    loop {
        let n = phrases.len();
        let runtime = shelf + n as f64 * phrase + track.ending();
        if n <= track.phrases_after_accent as usize && runtime <= RUNTIME_MAX {
            break;
        }
        let reason = if n > track.phrases_after_accent as usize { "track has fewer phrases" } else { "runtime over 110 s" };
        let pos = phrases
            .iter()
            .rposition(|p| p.label.starts_with("4-4-4-4"))
            .or_else(|| phrases.iter().rposition(|p| p.label.starts_with("4-4-8")).filter(|&p| p > 0))
            .or(if n > 1 { Some(n - 1) } else { None });
        let Some(pos) = pos else {
            note = format!("could not shorten further ({reason})");
            break;
        };
        for x in phrases.remove(pos).items {
            skipped.push((clips[x.clip].file.clone(), x.room, format!("{}: {reason}", x.role)));
        }
    }
    let n = phrases.len();
    let patterns = shelf_label + &phrases.iter().enumerate().map(|(i, p)| format!("P{} {}", i + 1, p.label)).collect::<Vec<_>>().join(" · ");
    let items: Vec<Item> = phrases.into_iter().flat_map(|p| p.items).collect();
    // a shrunk shelf moves the whole grid earlier: the track simply starts later
    let t_splice = shelf + n as f64 * phrase;
    let t_end = t_splice + track.ending();

    // close: the close clip fitted to the ending, keeping its tail still; else the last exterior held
    let mut close: Vec<Item> = Vec::new();
    let last_outside = stations.iter().rev().find(|s| is_exterior(s) || is_grounds(s)).and_then(|s| by_station(s.n, "station").map(|ci| (s, ci)));
    if let Some(ci) = clips.iter().rposition(|c| c.kind == "close") {
        let c = &clips[ci];
        let on_screen = 2.0 * c.duration();
        if on_screen < track.ending() {
            let fill = last_outside.map(|(s, fi)| (fi, s.room.clone())).or_else(|| items.last().map(|i| (i.clip, i.room.clone())));
            if let Some((fi, room)) = fill {
                close.push(Item { role: "close", clip: fi, room, why: "held before the pull-back to fill the ending".into(), beats: 0, screen_s: Some(track.ending() - on_screen), anchor: Anchor::From(0.0) });
            }
        }
        close.push(Item { role: "close", clip: ci, room: c.dest.map(room_of).unwrap_or_else(|| "exterior".into()), why: "close; end card".into(), beats: 0, screen_s: Some(track.ending().min(on_screen)), anchor: Anchor::End(c.duration()) });
    } else if let Some((s, ci)) = last_outside {
        close.push(Item { role: "close", clip: ci, room: s.room.clone(), why: "last exterior held (no close move); end card".into(), beats: 0, screen_s: Some(track.ending()), anchor: Anchor::From(0.0) });
    } else if let Some(last) = items.last().cloned() {
        close.push(Item { role: "close", clip: last.clip, room: last.room, why: "last room held (no exterior, no close move); end card".into(), beats: 0, screen_s: Some(track.ending()), anchor: Anchor::From(0.0) });
    }

    // lay everything on the timeline with frame-exact windows
    let mut rows = Vec::new();
    let mut t = 0.0;
    for list in [&establish, &items, &close] {
        lay(list, clips, bar, &mut t, &mut rows);
    }
    let runtime = t;
    let mut header = vec![
        ("music".to_string(), track.file.clone()),
        ("bpm".into(), format!("{:.1}", track.bpm)),
        ("accent_s".into(), format!("{:.1}", track.accent_s)),
        ("phrases_after_accent".into(), track.phrases_after_accent.to_string()),
        ("ending_start_s".into(), format!("{:.1}", track.ending_start_s)),
        ("end_s".into(), format!("{:.1}", track.end_s)),
        ("shelf".into(), format!("{shelf:.2}")),
        ("phrases".into(), n.to_string()),
        ("floor".into(), format!("{FIT_FLOOR:.0}")),
        ("patterns".into(), patterns),
        ("splice".into(), format!("{t_splice:.3}")),
        ("end".into(), format!("{t_end:.3}")),
        ("job".into(), job.number.to_string()),
        ("stabilise".into(), "on".into()),
        ("runtime".into(), format!("{runtime:.1} s = shelf {shelf:.1} + {n} phrases × {phrase:.1} + ending {:.1}", track.ending())),
    ];
    header.splice(1..1, track.catalog_lines());
    let mut notes: Vec<String> = Vec::new();
    if !job.note.is_empty() {
        notes.push(job.note.clone());
    }
    if !note.is_empty() {
        notes.push(note);
    }
    if runtime < 60.0 {
        notes.push(format!("runtime {runtime:.0} s is under the 60 s floor: a small card"));
    }
    if !notes.is_empty() {
        header.push(("notes".into(), notes.join("; ")));
    }
    Ok(Cutlist { header, rows, skipped })
}

/// The exposure nudge per camera row: the log frame at the row's centre is
/// measured and lifted (or lowered) so its mean sits at the bright/airy target.
pub(crate) fn measure_gains(rows: &mut [Row], clips: &[ClipInfo], job_dir: &Path) {
    for r in rows.iter_mut() {
        let camera = clips.iter().find(|c| c.file == r.clip).is_some_and(|c| c.source == Source::Fuji);
        if !camera {
            continue;
        }
        if let Ok(path) = locate(job_dir, &r.clip) {
            if let Some(g) = exposure_gain(&path, (r.in_s + r.out_s) / 2.0) {
                r.gain = g;
            }
        }
    }
}

/// Stops to add so the log frame's mean luma lands on LOG_TARGET, within GAIN_RANGE.
/// YAVG is on the pixel format's own scale, so the frame's pix_fmt is read with it
/// and the legal range scaled by bit depth (10-bit H.265 reads 4x an 8-bit H.264).
fn exposure_gain(path: &Path, t: f64) -> Option<f64> {
    let src = format!("movie={}\\:seek_point={t:.2},signalstats", path.display().to_string().replace(':', "\\:"));
    let o = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-f", "lavfi", "-i", &src, "-show_entries", "frame=pix_fmt:frame_tags=lavfi.signalstats.YAVG", "-of", "csv=p=0", "-read_intervals", "%+#1"])
        .output()
        .ok()?;
    let line = String::from_utf8_lossy(&o.stdout);
    let (pix_fmt, yavg) = line.lines().next()?.trim().split_once(',')?;
    let y: f64 = yavg.parse().ok()?;
    let scale = if pix_fmt.contains("p12") { 16.0 } else if pix_fmt.contains("p10") { 4.0 } else { 1.0 };
    let m = ((y - 16.0 * scale) / (219.0 * scale)).clamp(0.0, 1.0);
    let stops = ((LOG_TARGET - m) / LOG_STOP * 10.0).round() / 10.0;
    Some(stops.clamp(GAIN_RANGE.0, GAIN_RANGE.1))
}

/// A move's window and the walk's length: a long walk lands (its end plus
/// the tail still), a short one is centred.
pub(crate) fn move_anchor(c: &ClipInfo, bar: f64) -> (Anchor, f64) {
    let m = c.moving().unwrap_or(video::Segment { motion: Motion::Push, start: 0.0, end: c.duration() });
    let walk = m.end - m.start;
    if walk > bar {
        (Anchor::End(m.end + (c.duration() - m.end).min(0.5)), walk)
    } else {
        (Anchor::Centre((m.start + m.end) / 2.0), walk)
    }
}

/// The planner's anchor for a role, for a clip the operator swaps into a slot.
pub(crate) fn anchor_for(role: &str, c: &ClipInfo, bar: f64) -> Anchor {
    match role {
        "pan" => c.pan().map(|p| Anchor::From(p.start)).unwrap_or_else(|| hold_anchor(c, bar)),
        "wide" | "angle" | "static" => hold_anchor(c, bar),
        "move" => move_anchor(c, bar).0,
        "close" if c.kind != "station" => Anchor::End(c.duration()),
        "establish" if c.kind != "station" => move_anchor(c, bar).0,
        "establish" | "close" => Anchor::From(0.0),
        _ => Anchor::Centre(c.duration() / 2.0),
    }
}

/// The window inside a clip of `dur` seconds that shows `src` source seconds
/// from `anchor`, snapped to the 60p frame grid; `out` is clamped to the
/// clip (the renderer holds the last frame).
pub(crate) fn window(anchor: Anchor, dur: f64, src: f64) -> (f64, f64) {
    let mut in_s = match anchor {
        Anchor::End(e) => e.min(dur) - src,
        Anchor::From(s) => s,
        Anchor::Centre(m) => m - src / 2.0,
    };
    in_s = in_s.clamp(0.0, (dur - src).max(0.0));
    in_s = (in_s * 2.0 * OUT_FPS).round() / (2.0 * OUT_FPS);
    (in_s, (in_s + src).min(dur))
}

/// Where a clip holds still for a wide or a static: the opening hold, ending
/// where the pan starts; else the closing hold; else the clip's last bar.
pub(crate) fn hold_anchor(c: &ClipInfo, bar: f64) -> Anchor {
    if let Some(a) = c.still_a() {
        Anchor::End(a.end)
    } else if let Some(b) = c.segments.iter().rev().find(|s| s.motion == Motion::Still) {
        Anchor::From(b.start)
    } else {
        Anchor::From((c.duration() - bar).max(0.0))
    }
}

/// Lay items on the timeline from `t`: each slot is a whole number of
/// output frames, the source window is anchored inside the clip and
/// snapped to the 60p frame grid.
fn lay(items: &[Item], clips: &[ClipInfo], bar: f64, t: &mut f64, rows: &mut Vec<Row>) {
    for it in items {
        let len = it.screen_s.unwrap_or(it.beats as f64 * bar / 4.0);
        let t_end = *t + len;
        let frames = (t_end * OUT_FPS).round() - (*t * OUT_FPS).round();
        let src = frames / (2.0 * OUT_FPS);
        let c = &clips[it.clip];
        let dur = c.duration();
        let (in_s, out_s) = window(it.anchor, dur, src);
        let mut why = it.why.clone();
        if in_s + src > dur + 1e-6 {
            why += &format!(" (clip {:.1} s short, last frame held)", in_s + src - dur);
        }
        rows.push(Row { at: *t, role: it.role.into(), clip: c.file.clone(), in_s, out_s, room: it.room.clone(), why, gain: 0.0 });
        *t = t_end;
    }
}

/// Three 9:16 reels on the master's track, the accent at frame zero, two
/// phrases each in the social blocks (MUSIC.md): the hook phrase
/// 2-2-2-2 | 2-2-4, then 4-2-2 | 8 with the pan, centre-cropped, as the
/// payoff, or 4-2-2 | 4-2-2 when the reel has no pan. Each reel has a
/// purpose: `mls` walks the listing in station order from the facade,
/// `details` is verticals and 18-50 statics only, hero rooms first, and
/// `feature` is one station's material, `Feature:` in job.md or the hero
/// with the best fit. A thin pool serves second helpings, another window
/// of a clip already used, and says so.
fn build_reels(clips: &[ClipInfo], stations: &[StationInfo], track: &Track, job: &Job) -> Vec<Cutlist> {
    let bar = track.bar();
    let phrases = track.phrases_after_accent.clamp(1, 2) as usize;
    let room_of = |n: Option<usize>| n.and_then(|n| stations.iter().find(|s| s.n == n)).map(|s| s.room.clone()).unwrap_or_default();
    let first_of = |room: &str| stations.iter().find(|s| s.room == room).map(|s| s.n);
    let (first_bed, first_bath) = (first_of("bedroom"), first_of("bathroom"));
    let is_hero = |s: &StationInfo| s.room == "living_room" || s.room == "kitchen" || Some(s.n) == first_bed || Some(s.n) == first_bath || s.fit >= FIT_HERO;
    let is_exterior = |s: &StationInfo| EXTERIOR.contains(&s.room.as_str());
    let station_clip = |n: usize| clips.iter().position(|c| c.station == Some(n) && c.kind == "station");
    let of_kind = |n: usize, kind: &str| -> Vec<usize> { clips.iter().enumerate().filter(|(_, c)| c.station == Some(n) && c.kind == kind).map(|(i, _)| i).collect() };
    let facade = stations.iter().filter(|s| is_exterior(s)).find_map(|s| of_kind(s.n, "vertical").first().copied());
    let pan_of = |n: usize| station_clip(n).filter(|&ci| clips[ci].pan().is_some());
    let arriving = |n: usize| clips.iter().position(|c| ["entry", "hallway", "lookback", "grounds"].contains(&c.kind.as_str()) && c.dest == Some(n) && c.moving().is_some());

    /// What a reel can serve; a second helping is another window of the same clip.
    #[derive(Clone)]
    struct Shot {
        ci: usize,
        role: &'static str,
        why: String,
    }
    let vert = |ci: usize, why: &str| Shot { ci, role: "detail", why: why.into() };
    let hold = |ci: usize, why: &str| Shot { ci, role: "wide", why: format!("{why}, centre crop") };
    let interiors: Vec<&StationInfo> = stations.iter().filter(|s| !is_exterior(s) && !GROUNDS.contains(&s.room.as_str())).collect();
    let mut heroes: Vec<&StationInfo> = interiors.iter().copied().filter(|s| is_hero(s)).collect();
    heroes.sort_by(|a, b| b.fit.total_cmp(&a.fit));

    // the feature station: `Feature:` in job.md (stNN or a room), else the hero with the best fit
    let feature: Option<&StationInfo> = {
        let f = job.feature.trim();
        let by_number = f.strip_prefix("st").and_then(|n| n.parse::<usize>().ok()).and_then(|n| stations.iter().find(|s| s.n == n));
        let by_room = || {
            let mut r: Vec<&StationInfo> = stations.iter().filter(|s| s.room == f).collect();
            r.sort_by(|a, b| b.fit.total_cmp(&a.fit));
            r.first().copied()
        };
        by_number.or_else(|| if f.is_empty() { None } else { by_room() }).or_else(|| heroes.first().copied())
    };
    let feature_why = if job.feature.trim().is_empty() { "the hero with the best fit; `Feature:` in job.md chooses" } else { "job.md `Feature:`" };

    // the pools, in serving order, and the pan that closes the second phrase
    let mut mls: Vec<Shot> = Vec::new();
    if let Some(f) = facade {
        mls.push(vert(f, "facade vertical opens"));
    }
    let ordered: Vec<&StationInfo> = interiors.iter().copied().chain(stations.iter().filter(|s| GROUNDS.contains(&s.room.as_str()))).collect();
    for s in &ordered {
        if let Some(&v) = of_kind(s.n, "vertical").first() {
            mls.push(vert(v, "listing order"));
        } else if let Some(ci) = station_clip(s.n) {
            mls.push(hold(ci, "listing order, the hold"));
        }
    }
    for s in &ordered {
        for &v in of_kind(s.n, "vertical").iter().skip(1) {
            mls.push(vert(v, "listing order, another vertical"));
        }
    }
    let mls_pan = heroes.iter().find_map(|h| pan_of(h.n));

    let mut details: Vec<Shot> = Vec::new();
    let mut by_priority: Vec<&StationInfo> = heroes.clone();
    by_priority.extend(interiors.iter().copied().filter(|s| !is_hero(s)));
    for s in &by_priority {
        for v in of_kind(s.n, "vertical") {
            details.push(vert(v, "detail"));
        }
        for d in of_kind(s.n, "detail") {
            details.push(Shot { ci: d, role: "detail", why: "18-50 detail, centre crop".into() });
        }
    }

    let mut feat: Vec<Shot> = Vec::new();
    let mut feat_pan = None;
    if let Some(f) = feature {
        for v in of_kind(f.n, "vertical") {
            feat.push(vert(v, "feature vertical"));
        }
        for d in of_kind(f.n, "detail") {
            feat.push(Shot { ci: d, role: "detail", why: "feature 18-50 detail, centre crop".into() });
        }
        if let Some(ci) = station_clip(f.n) {
            feat.push(hold(ci, "feature room, the hold"));
        }
        // the same-type neighbour the master shows as the second angle
        if let Some(n) = stations.iter().find(|s| s.n > f.n && s.room == f.room) {
            for v in of_kind(n.n, "vertical") {
                feat.push(vert(v, "feature, the second angle's vertical"));
            }
            if let Some(ci) = station_clip(n.n) {
                feat.push(hold(ci, "feature, the second angle"));
            }
        }
        if let Some(mi) = arriving(f.n) {
            feat.push(Shot { ci: mi, role: "move", why: format!("{} move into the feature, centre crop", clips[mi].kind) });
        }
        feat_pan = pan_of(f.n);
    }
    let feature_label = feature.map(|f| format!("st{:02} {} ({feature_why})", f.n, f.room)).unwrap_or_else(|| "no station".into());

    let purposes: [(&str, String, Vec<Shot>, Option<usize>); 3] = [
        ("mls", "the listing in station order, from the facade".into(), mls, mls_pan),
        ("details", "details only, hero rooms first".into(), details, None),
        ("feature", feature_label, feat, feat_pan),
    ];
    const HOOK: [u32; 7] = [2, 2, 2, 2, 2, 2, 4];
    const PAYOFF: [u32; 3] = [4, 2, 2]; // then the eight
    const NO_PAN: [u32; 6] = [4, 2, 2, 4, 2, 2];

    let mut reels = Vec::new();
    for (purpose, label, pool, pan) in purposes {
        let mut items: Vec<Item> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        let (mut idx, mut helping, mut helpings_used) = (0usize, 0u32, 0u32);
        // the next shot as a window of `beats`; a pool that runs out serves second and third helpings
        let mut next = |beats: u32| -> Option<Item> {
            if pool.is_empty() {
                return None;
            }
            if idx >= pool.len() {
                idx = 0;
                helping += 1;
            }
            if helping > 2 {
                return None;
            }
            let sh = &pool[idx];
            idx += 1;
            let c = &clips[sh.ci];
            let dur = c.duration();
            let anchor = match (sh.role, helping) {
                ("move", _) => c.moving().map(|m| Anchor::Centre((m.start + m.end) / 2.0)).unwrap_or(Anchor::Centre(dur / 2.0)),
                ("wide", 0) => hold_anchor(c, bar),
                (_, 0) => Anchor::Centre(dur * 0.5),
                (_, 1) => Anchor::Centre(dur * 0.2),
                _ => Anchor::Centre(dur * 0.8),
            };
            let why = match helping {
                0 => format!("{} ({beats})", sh.why),
                1 => {
                    helpings_used += 1;
                    format!("{} ({beats}; second helping)", sh.why)
                }
                _ => {
                    helpings_used += 1;
                    format!("{} ({beats}; third helping)", sh.why)
                }
            };
            Some(Item { role: sh.role, clip: sh.ci, room: room_of(if sh.role == "move" { c.dest } else { c.station }), why, beats, screen_s: None, anchor })
        };
        'phrases: for p in 0..phrases {
            let mut ph: Vec<Item> = Vec::new();
            let label = if p == 0 {
                for &b in &HOOK {
                    match next(b) {
                        Some(it) => ph.push(it),
                        None => break 'phrases,
                    }
                }
                "2-2-2-2 | 2-2-4 hook".to_string()
            } else if let Some(pc) = pan {
                for &b in &PAYOFF {
                    match next(b) {
                        Some(it) => ph.push(it),
                        None => break 'phrases,
                    }
                }
                let c = &clips[pc];
                let start = c.pan().map(|x| x.start).unwrap_or(0.0);
                ph.push(Item { role: "pan", clip: pc, room: room_of(c.station), why: "the pan, centre crop: the payoff (8)".into(), beats: 8, screen_s: None, anchor: Anchor::From(start) });
                format!("4-2-2 | 8 {} pan", room_of(c.station))
            } else {
                for &b in &NO_PAN {
                    match next(b) {
                        Some(it) => ph.push(it),
                        None => break 'phrases,
                    }
                }
                "4-2-2 | 4-2-2".to_string()
            };
            items.extend(ph);
            labels.push(label);
        }
        let beats: u32 = items.iter().map(|i| i.beats).sum();
        let native: u32 = items.iter().filter(|i| clips[i.clip].source == Source::Phone).map(|i| i.beats).sum();
        let native_pct = if beats > 0 { 100 * native / beats } else { 0 };
        let mut rows = Vec::new();
        let mut t = 0.0;
        lay(&items, clips, bar, &mut t, &mut rows);
        let mut header = vec![
            ("music".to_string(), track.file.clone()),
            ("bpm".into(), format!("{:.1}", track.bpm)),
            ("accent_s".into(), format!("{:.1}", track.accent_s)),
            ("purpose".into(), purpose.to_string()),
            ("lead".into(), label),
            ("bars".into(), (beats / 4).to_string()),
            ("cuts".into(), items.len().to_string()),
            ("patterns".into(), labels.iter().enumerate().map(|(i, l)| format!("P{} {l}", i + 1)).collect::<Vec<_>>().join(" · ")),
            ("end".into(), format!("{t:.3}")),
            ("native".into(), format!("{native_pct}% of the reel from vertical-native clips")),
        ];
        header.splice(1..1, track.catalog_lines());
        let mut notes: Vec<String> = Vec::new();
        if track.phrases_after_accent < 2 {
            notes.push(format!("track has {} full phrases after the accent; a reel wants 2", track.phrases_after_accent));
        }
        if helpings_used > 0 {
            notes.push(format!("thin pool: {helpings_used} shots are second or third helpings of a clip already used"));
        }
        if native_pct < 40 {
            notes.push("under 40% vertical-native: shoot more phone verticals".into());
        }
        if beats < 32 {
            notes.push("under two phrases of material".into());
        }
        if !notes.is_empty() {
            header.push(("notes".into(), notes.join("; ")));
        }
        reels.push(Cutlist { header, rows, skipped: Vec::new() });
    }
    reels
}

// ---- cutlist.md ----------------------------------------------------------

pub(crate) fn write_cutlist(c: &Cutlist, job: &Job) -> String {
    let mut md = format!("# Cutlist — {}\n\n", job.address);
    md += "Edit and re-render with `retool cut`. `at` is film time (the grid; the row fills its slot, held or trimmed to fit), `in`/`out` are source seconds on the 60p clip, everything runs 2×. Rows in `skipped` can be revived by moving them up.\n\n";
    for (k, v) in &c.header {
        md += &format!("{k}: {v}\n");
    }
    md += "\n| # | at | role | clip | in | out | speed | gain | room | why |\n|---|---|---|---|---|---|---|---|---|---|\n";
    for (i, r) in c.rows.iter().enumerate() {
        md += &format!("| {} | {:.2} | {} | {} | {:.3} | {:.3} | 2x | {:+.1} | {} | {} |\n", i + 1, r.at, r.role, r.clip, r.in_s, r.out_s, r.gain, r.room, r.why);
    }
    if !c.skipped.is_empty() {
        md += "\n## skipped\n\n| clip | room | why |\n|---|---|---|\n";
        for (clip, room, why) in &c.skipped {
            md += &format!("| {clip} | {room} | {why} |\n");
        }
    }
    md
}

/// Read a cutlist back: `key: value` header lines, the rows table, the skipped table.
pub fn read_cutlist(path: &Path) -> Result<Cutlist> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut c = Cutlist { header: Vec::new(), rows: Vec::new(), skipped: Vec::new() };
    let mut in_skipped = false;
    for line in text.lines() {
        if line.starts_with("## skipped") {
            in_skipped = true;
            continue;
        }
        if line.starts_with('|') {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if cells.len() < 4 || cells[1] == "#" || cells[1] == "clip" || cells[1].starts_with("---") {
                continue;
            }
            if in_skipped {
                c.skipped.push((cells[1].to_string(), cells[2].to_string(), cells[3].to_string()));
            } else if cells.len() >= 10 {
                let num = |i: usize| cells[i].parse::<f64>().with_context(|| format!("cutlist row {}: bad number `{}`", cells[1], cells[i]));
                // with or without the gain column (cutlists written before 2026-09-02 evening)
                let (gain, room, why) = if cells.len() >= 11 { (num(8)?, cells[9], cells[10]) } else { (0.0, cells[8], cells[9]) };
                c.rows.push(Row { at: num(2)?, role: cells[3].to_string(), clip: cells[4].to_string(), in_s: num(5)?, out_s: num(6)?, room: room.to_string(), why: why.to_string(), gain });
            }
            continue;
        }
        if let Some((k, v)) = line.split_once(": ") {
            if !k.contains(' ') && !k.starts_with('#') {
                c.header.push((k.to_string(), v.to_string()));
            }
        }
    }
    Ok(c)
}

/// Where a clip named in the cutlist lives: the job's video/ or phone/ folder.
pub fn locate(job_dir: &Path, clip: &str) -> Result<PathBuf> {
    for sub in ["video", "phone"] {
        let p = job_dir.join(sub).join(clip);
        if p.exists() {
            return Ok(p);
        }
    }
    bail!("{clip} is in neither {}/video nor {}/phone", job_dir.display(), job_dir.display())
}


// ---- the renderer ---------------------------------------------------------

const X264: [&str; 12] = ["-c:v", "libx264", "-preset", "medium", "-crf", "18", "-maxrate", "16M", "-bufsize", "32M", "-pix_fmt", "yuv420p"];
const GIMBAL_KINDS: [&str; 6] = ["walkup", "entry", "hallway", "lookback", "grounds", "close"];

/// A title or end card: lines with their sizes, drawn lower-left with a fade.
struct Card {
    lines: Vec<(String, u32)>,
}

impl Card {
    fn address(job: &Job) -> Card {
        let (street, city) = job.address.rsplit_once(", ").unwrap_or((job.address.as_str(), ""));
        Card { lines: vec![(street.to_string(), 64), (city.to_string(), 40)] }
    }
    fn end(job: &Job, branded: bool) -> Card {
        if branded {
            Card { lines: vec![(job.agent.clone(), 56), (job.brokerage.clone(), 40), (job.phone.clone(), 40)] }
        } else {
            Card::address(job)
        }
    }
    /// drawtext filters for this card; `alpha` is an expression in segment time `t`.
    fn filters(&self, dir: &Path, tag: &str, alpha: &str, height: u32) -> Result<String> {
        let mut y = height as i32 - 80;
        let mut out = Vec::new();
        for (i, (text, size)) in self.lines.iter().enumerate().rev() {
            if text.is_empty() {
                continue;
            }
            let file = dir.join(format!("card_{tag}_{i}.txt"));
            std::fs::write(&file, text)?;
            y -= *size as i32 + 16;
            out.push(format!(
                "drawtext=font=Helvetica:textfile={}:fontsize={size}:fontcolor=white:x=96:y={y}:shadowcolor=black@0.7:shadowx=2:shadowy=2:box=1:boxcolor=black@0.35:boxborderw=14:alpha='{alpha}'",
                file.display()
            ));
        }
        out.reverse();
        Ok(out.join(","))
    }
}

struct SegJob {
    out: PathBuf,
    args: Vec<String>,
}

/// What the music does under a list: the master splices to the composed
/// ending; a reel runs from the accent and stops on the phrase boundary.
enum Score {
    Master { start_s: f64, splice: f64, ending_start_s: f64, end_s: f64 },
    Reel { start_s: f64, len: f64 },
}

/// Render the cutlist (or the one given) to the two MLS files, then every
/// planned reel: one 1080p30 segment per row, held or trimmed to its slot
/// on the grid, the close twice (branded, unbranded), concat, music laid on
/// the grid, two-pass loudness to −14 LUFS.
pub fn cut(job_dir: &Path, out_dir: &Path, from: Option<&Path>, unapproved: bool) -> Result<()> {
    let t0 = std::time::Instant::now();
    let video_dir = out_dir.join("video");
    let job = read_job(job_dir)?;
    let clips = video::read_clips_tsv(out_dir).unwrap_or_default();
    let seg_dir = video_dir.join(".seg");
    std::fs::create_dir_all(&seg_dir)?;
    let mut log = format!("# render {} — {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M"), job.address);
    let mut outputs = Vec::new();

    let cl_path = from.map(Path::to_path_buf).unwrap_or_else(|| video_dir.join("cutlist.md"));
    let cl = read_cutlist(&cl_path)?;
    if cl.rows.is_empty() {
        bail!("{} has no rows (retool plan writes it)", cl_path.display());
    }
    if !unapproved {
        require_approved(&cl, &cl_path)?;
    }
    outputs.extend(render(job_dir, &video_dir, &seg_dir, &cl, &cl_path, &job, &clips, false, &mut log)?);
    if from.is_none() {
        for k in 1..=3 {
            let rp = video_dir.join(format!("reel_{k:02}.md"));
            if rp.exists() {
                let reel = read_cutlist(&rp)?;
                if !reel.rows.is_empty() {
                    outputs.extend(render(job_dir, &video_dir, &seg_dir, &reel, &rp, &job, &clips, true, &mut log)?);
                }
            }
        }
    }
    let log_path = video_dir.join("render.log");
    std::fs::write(&log_path, &log)?;
    scorecard_video(out_dir)?;
    for out in &outputs {
        println!("{}: {}", out.display(), probe_summary(out));
    }
    eprintln!("render: {:.0} s → {}", t0.elapsed().as_secs_f64(), log_path.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn render(job_dir: &Path, video_dir: &Path, seg_dir: &Path, cl: &Cutlist, cl_path: &Path, job: &Job, clips: &[ClipInfo], reel: bool, log: &mut String) -> Result<Vec<PathBuf>> {
    let end = cl.num("end")?;
    let music = cl.get("music").context("cutlist header has no music:")?.trim();
    let track = if Path::new(music).exists() {
        PathBuf::from(music)
    } else {
        std::env::var("HOME").map(|h| Path::new(&h).join(ROTATION).join(music)).unwrap_or_default()
    };
    if !track.exists() {
        bail!("music {} not found", track.display());
    }
    let tag = cl_path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    let stabilise = cl.get("stabilise").is_some_and(|v| v.trim() == "on");
    let (w, h) = if reel { (1080u32, 1920u32) } else { (1920, 1080) };
    *log += &format!("\n## {tag}\nmusic: {}\ncutlist: {}\n", track.display(), cl_path.display());

    let n = cl.rows.len();
    let first_establish = cl.rows.iter().position(|r| r.role == "establish").filter(|_| !reel);
    let last_establish = cl.rows.iter().rposition(|r| r.role == "establish").filter(|_| !reel);
    let last_close = cl.rows.iter().rposition(|r| r.role == "close").filter(|_| !reel);
    let mut jobs: Vec<SegJob> = Vec::new();
    let mut lists: [Vec<PathBuf>; 2] = [Vec::new(), Vec::new()]; // branded, unbranded (a reel uses the first)
    for (i, r) in cl.rows.iter().enumerate() {
        let next_at = cl.rows.get(i + 1).map(|x| x.at).unwrap_or(end);
        let frames = ((next_at * OUT_FPS).round() - (r.at * OUT_FPS).round()).max(1.0) as u32;
        let src = locate(job_dir, &r.clip)?;
        let info = clips.iter().find(|c| c.file == r.clip);
        let phone = info.map(|c| c.source == Source::Phone).unwrap_or(false);
        let portrait = info.map(|c| c.portrait).unwrap_or(false);
        let in_f = (r.in_s * 2.0 * OUT_FPS).round();
        let out_f = (r.out_s * 2.0 * OUT_FPS).round().max(in_f + 1.0);
        let ss = format!("{:.5}", (in_f - 0.5) / (2.0 * OUT_FPS));
        let t = format!("{:.5}", (out_f - in_f) / (2.0 * OUT_FPS));
        let seg_len = frames as f64 / OUT_FPS;

        let mut chain: Vec<String> = vec!["setpts=2*PTS".into(), "fps=30".into()];
        if !phone && r.gain.abs() >= 0.05 {
            // the exposure nudge, in log code values while the frame is still YUV
            chain.push(format!("lutyuv=y='clip(val+({:.1})*maxval/255,0,maxval)'", r.gain * LOG_STOP * 219.0));
        }
        match (reel, portrait) {
            (true, true) => {}
            (true, false) => chain.push("crop=ih*9/16:ih".into()),
            (false, true) => chain.push("crop=iw:iw*9/16".into()),
            (false, false) => {}
        }
        chain.push(format!("scale={w}:{h}:flags=lanczos"));
        if stabilise && info.is_some_and(|c| GIMBAL_KINDS.contains(&c.kind.as_str())) {
            chain.push("deshake=rx=32:ry=32:edge=mirror:blocksize=8:contrast=64".into());
        }
        let lut = if phone { video::phone_cube().map(|p| p.display().to_string()).unwrap_or_else(|| video::APPLE_CUBE.into()) } else { video::WDR_CUBE.to_string() };
        if Path::new(&lut).exists() {
            chain.push(format!("lut3d=file={lut}"));
            if !phone {
                chain.push("eq=saturation=1.08".into());
            }
        } else if !phone {
            eprintln!("{}: no {lut}, rendering log as is", r.clip);
        }
        chain.push("tpad=stop_mode=clone:stop=900".into());
        if i == 0 && !reel {
            chain.push("fade=t=in:st=0:d=0.4".into());
        }
        // the address card over the establish, in after a second, out before the accent; the end card over the close
        let mut variants: Vec<(String, Option<String>)> = Vec::new();
        if r.role == "establish" && !reel {
            let fade_in = if Some(i) == first_establish { "clip((t-1)/0.5,0,1)" } else { "1" };
            let fade_out = if Some(i) == last_establish { format!("clip(({seg_len:.3}-0.5-t)/0.5,0,1)") } else { "1".into() };
            variants.push((String::new(), Some(Card::address(job).filters(seg_dir, "open", &format!("{fade_in}*{fade_out}"), h)?)));
        } else if Some(i) == last_close {
            for (suffix, branded) in [("_b", true), ("_u", false)] {
                variants.push((suffix.into(), Some(Card::end(job, branded).filters(seg_dir, if branded { "end_b" } else { "end_u" }, "clip((t-0.5)/0.5,0,1)", h)?)));
            }
        } else {
            variants.push((String::new(), None));
        }
        for (suffix, card) in variants {
            let mut vf = chain.clone();
            if let Some(c) = card {
                vf.push(c);
            }
            vf.push("format=yuv420p".into());
            let out = seg_dir.join(format!("{tag}_{:02}{suffix}.mp4", i + 1));
            let mut args: Vec<String> = ["-y", "-loglevel", "error", "-hwaccel", "videotoolbox", "-ss", &ss, "-t", &t, "-i"].iter().map(|s| s.to_string()).collect();
            args.push(src.display().to_string());
            args.extend(["-frames:v", &frames.to_string(), "-vf", &vf.join(","), "-an", "-r", "30"].iter().map(|s| s.to_string()));
            args.extend(X264.iter().map(|s| s.to_string()));
            args.push(out.display().to_string());
            match suffix.as_str() {
                "_b" => lists[0].push(out.clone()),
                "_u" => lists[1].push(out.clone()),
                _ => {
                    lists[0].push(out.clone());
                    lists[1].push(out.clone());
                }
            }
            jobs.push(SegJob { out, args });
        }
    }
    eprintln!("{tag}: {} segments for {n} rows", jobs.len());
    for j in &jobs {
        *log += &format!("ffmpeg {}\n", j.args.join(" "));
    }
    run_parallel(&jobs, 3)?;

    // concat, then music
    let names: Vec<String> = if reel { vec![tag.clone()] } else { vec!["mls_branded".into(), "mls_unbranded".into()] };
    let score = if reel {
        Score::Reel { start_s: cl.num("accent_s")?, len: end }
    } else {
        Score::Master { start_s: cl.num("accent_s")? - cl.num("shelf")?, splice: cl.num("splice")?, ending_start_s: cl.num("ending_start_s")?, end_s: cl.num("end_s")? }
    };
    let mut outputs = Vec::new();
    for (k, name) in names.iter().enumerate() {
        let list = seg_dir.join(format!("{name}.txt"));
        std::fs::write(&list, lists[k].iter().map(|p| format!("file '{}'\n", p.canonicalize().unwrap_or(p.clone()).display())).collect::<String>())?;
        let silent = seg_dir.join(format!("{name}_silent.mp4"));
        let args = ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", &list.display().to_string(), "-c", "copy", &silent.display().to_string()].map(String::from);
        *log += &format!("ffmpeg {}\n", args.join(" "));
        run("ffmpeg", &args)?;
        let out = video_dir.join(format!("{name}.mp4"));
        *log += &mux_music(&silent, &track, &score, &out)?;
        outputs.push(out);
    }
    Ok(outputs)
}

fn run(bin: &str, args: &[String]) -> Result<()> {
    let out = std::process::Command::new(bin).args(args).output().with_context(|| format!("run {bin}"))?;
    if !out.status.success() {
        bail!("{bin} failed: {}\n  args: {}", String::from_utf8_lossy(&out.stderr).trim(), args.join(" "));
    }
    Ok(())
}

fn run_parallel(jobs: &[SegJob], workers: usize) -> Result<()> {
    let results: Vec<Result<()>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..workers.min(jobs.len().max(1)))
            .map(|w| {
                s.spawn(move || {
                    jobs.iter()
                        .skip(w)
                        .step_by(workers)
                        .map(|j| {
                            let r = run("ffmpeg", &j.args);
                            eprintln!("{} {}", if r.is_ok() { "ok " } else { "ERR" }, j.out.file_name().unwrap_or_default().to_string_lossy());
                            r
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("render worker")).collect()
    });
    results.into_iter().collect()
}

/// The music under the picture, loudness normalised in two passes to
/// −14 LUFS / −1.5 dBTP, padded to the picture, muxed as AAC.
fn mux_music(silent: &Path, track: &Path, score: &Score, out: &Path) -> Result<String> {
    let total = match score {
        Score::Master { splice, ending_start_s, end_s, .. } => splice + (end_s - ending_start_s),
        Score::Reel { len, .. } => *len,
    };
    let graph = |src: &str, measured: &str, tail: &str| match score {
        Score::Master { start_s, splice, ending_start_s, end_s } => format!(
            "[{src}]asplit[m1][m2];[m1]atrim=start={start_s:.3}:end={:.3},asetpts=PTS-STARTPTS[a1];[m2]atrim=start={ending_start_s:.3}:end={end_s:.3},asetpts=PTS-STARTPTS[a2];[a1][a2]acrossfade=d=0.1:c1=tri:c2=tri[am];[am]loudnorm=I=-14:TP=-1.5:LRA=11{measured},aresample=48000{tail}[a]",
            start_s + splice + 0.1
        ),
        Score::Reel { start_s, len } => format!(
            "[{src}]atrim=start={start_s:.3}:end={:.3},asetpts=PTS-STARTPTS,afade=t=out:st={:.3}:d=0.1[am];[am]loudnorm=I=-14:TP=-1.5:LRA=11{measured},aresample=48000{tail}[a]",
            start_s + len,
            len - 0.1
        ),
    };
    let mut log = String::new();
    // pass 1: measure
    let args1 = ["-y", "-loglevel", "info", "-i", &track.display().to_string(), "-filter_complex", &graph("0:a", ":print_format=json", ""), "-map", "[a]", "-f", "null", "-"].map(String::from);
    log += &format!("ffmpeg {}\n", args1.join(" "));
    let o = std::process::Command::new("ffmpeg").args(&args1).output().context("run ffmpeg")?;
    let err = String::from_utf8_lossy(&o.stderr);
    let json_start = err.rfind('{').context("loudnorm measurement missing")?;
    let json_end = json_start + err[json_start..].find('}').context("loudnorm measurement unterminated")?;
    let m: serde_json::Value = serde_json::from_str(&err[json_start..=json_end]).context("parse loudnorm json")?;
    let g = |k: &str| m[k].as_str().unwrap_or("0").to_string();
    let measured = format!(
        ":measured_I={}:measured_TP={}:measured_LRA={}:measured_thresh={}:offset={}:linear=true",
        g("input_i"),
        g("input_tp"),
        g("input_lra"),
        g("input_thresh"),
        g("target_offset")
    );
    // pass 2: apply, pad to the picture, mux
    let args2 = [
        "-y", "-loglevel", "error", "-i", &silent.display().to_string(), "-i", &track.display().to_string(),
        "-filter_complex", &graph("1:a", &measured, &format!(",apad=whole_dur={total:.3}")), "-map", "0:v", "-map", "[a]", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
        "-shortest", "-movflags", "+faststart", &out.display().to_string(),
    ]
    .map(String::from);
    log += &format!("ffmpeg {}\n", args2.join(" "));
    run("ffmpeg", &args2)?;
    Ok(log)
}

fn probe_summary(path: &Path) -> String {
    let o = std::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "stream=codec_type,width,height,r_frame_rate:format=duration", "-of", "compact=p=0:nk=1", path.display().to_string().as_str()])
        .output();
    match o {
        Ok(o) => String::from_utf8_lossy(&o.stdout).split_whitespace().collect::<Vec<_>>().join(" "),
        Err(_) => "ffprobe missing".into(),
    }
}

// ---- the scorecard's video section -----------------------------------------

/// Append (or refresh) a `## video` section on `out/scorecard.md`: runtime
/// and track, rows used and skipped, the gimbal set, the reels, the files.
pub fn scorecard_video(out_dir: &Path) -> Result<()> {
    let video_dir = out_dir.join("video");
    let sc_path = out_dir.join("scorecard.md");
    let Ok(sc) = std::fs::read_to_string(&sc_path) else {
        eprintln!("no {}: video section not written", sc_path.display());
        return Ok(());
    };
    let cl = read_cutlist(&video_dir.join("cutlist.md"))?;
    let mut lines = vec![format!("- runtime: {} ({})", cl.get("runtime").unwrap_or("?"), cl.get("music").unwrap_or("?"))];
    let mut skipped: Vec<String> = cl.skipped.iter().map(|(clip, room, why)| format!("{clip} {room}: {why}")).collect();
    if skipped.len() > 4 {
        let more = skipped.len() - 3;
        skipped.truncate(3);
        skipped.push(format!("{more} more"));
    }
    lines.push(format!("- clips: {} rows used, {} skipped{}", cl.rows.len(), cl.skipped.len(), if skipped.is_empty() { String::new() } else { format!(" ({})", skipped.join("; ")) }));
    if let Some(n) = cl.get("notes") {
        lines.push(format!("- notes: {n}"));
    }
    if let Ok(clips) = video::read_clips_tsv(out_dir) {
        let have: Vec<&str> = ["walkup", "entry", "hallway", "lookback", "grounds", "close"].into_iter().filter(|r| clips.iter().any(|c| c.kind == *r)).collect();
        let missing: Vec<&str> = ["walkup", "entry", "hallway", "lookback", "grounds", "close"].into_iter().filter(|r| !have.contains(r)).collect();
        lines.push(format!("- gimbal set: {}{}", if have.is_empty() { "none".to_string() } else { have.join(", ") }, if missing.is_empty() { " — complete".to_string() } else { format!(" — missing {}", missing.join(", ")) }));
    }
    let reels: Vec<String> = (1..=3)
        .filter_map(|k| read_cutlist(&video_dir.join(format!("reel_{k:02}.md"))).ok())
        .map(|r| format!("{} s, {}", r.get("end").and_then(|e| e.parse::<f64>().ok()).map(|e| format!("{e:.1}")).unwrap_or_default(), r.get("native").unwrap_or("").split(' ').next().unwrap_or("")))
        .collect();
    if !reels.is_empty() {
        lines.push(format!("- reels: {} ({} vertical-native)", reels.len(), reels.join(" · ")));
    }
    let files: Vec<&str> = ["mls_branded.mp4", "mls_unbranded.mp4", "reel_01.mp4", "reel_02.mp4", "reel_03.mp4"].into_iter().filter(|f| video_dir.join(f).exists()).collect();
    lines.push(format!("- files: {}", files.join(", ")));
    let body = sc.split("\n## video\n").next().unwrap_or(&sc).trim_end();
    std::fs::write(&sc_path, format!("{body}\n\n## video\n\n{}\n", lines.join("\n")))?;
    Ok(())
}
