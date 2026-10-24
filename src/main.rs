use anyhow::{bail, Context, Result};
use rawler::imgop::develop::RawDevelop;
use std::path::{Path, PathBuf};
use std::process::Command;

mod cut;
mod dino;
mod music;
mod realm;
mod review;
mod verticals;
mod video;

// Real binary lives in HuginStitchProject.app; the tools_mac/enfuse symlink is dangling.
const SENSOR_WIDTH_MM: f32 = 23.5; // X-H2 APS-C
const ENFUSE: &str = "/Applications/Hugin/HuginStitchProject.app/Contents/MacOS/enfuse";

fn main() -> Result<()> {
    // positionals first, `--flags` anywhere after the subcommand
    let all: Vec<String> = std::env::args().skip(1).collect();
    let flag = |f: &str| all.iter().any(|a| a == f);
    let args: Vec<String> = all.iter().filter(|a| !a.starts_with("--")).cloned().collect();
    match args.first().map(String::as_str) {
        Some("fuse") if args.len() >= 3 => fuse(Path::new(&args[1]), &args[2..]),
        Some("tone") if args.len() >= 3 => tone(Path::new(&args[1]), Path::new(&args[2])).map(|_| ()),
        Some("blend") if args.len() >= 4 => blend(
            Path::new(&args[1]),
            Path::new(&args[2]),
            Path::new(&args[3]),
            args.get(4).map(|s| s.parse()).transpose()?.unwrap_or(0.5),
        )
        .map(|_| ()),
        Some("dev") if args.len() >= 3 => {
            let out_dir = Path::new(&args[1]);
            std::fs::create_dir_all(out_dir)?;
            for raf in &args[2..] {
                let raf = Path::new(raf);
                let stem = raf.file_stem().context("bad path")?.to_string_lossy();
                let tif = out_dir.join(format!("dev_{stem}.tif"));
                eprintln!("developing {}", raf.display());
                develop_to_tiff(raf, &tif)?;
                println!("{}", tif.display());
            }
            Ok(())
        }
        Some("level") if args.len() >= 3 => level(Path::new(&args[1]), Path::new(&args[2]), args.get(3).map(Path::new)).map(|_| ()),
        Some("card") if args.len() >= 2 => card_report(Path::new(&args[1]), true).map(|_| ()),
        Some("sharp") if args.len() >= 2 => sharp_report(Path::new(&args[1])),
        Some("run") if args.len() >= 3 => run_card(Path::new(&args[1]), Path::new(&args[2])),
        Some("deliver") if args.len() >= 4 => deliver(Path::new(&args[1]), Path::new(&args[2]), Path::new(&args[3])),
        Some("music") if args.len() >= 3 && args[1] == "stamp" => music::stamp(Path::new(&args[2])),
        Some("music") if args.len() >= 3 && args[1] == "check" => music::check(Path::new(&args[2])),
        Some("clips") if args.len() >= 3 => video::clips(Path::new(&args[1]), Path::new(&args[2])),
        Some("gate") if args.len() >= 3 => video::gate_cmd(Path::new(&args[1]), Path::new(&args[2])),
        Some("plan") if args.len() >= 3 => cut::plan(Path::new(&args[1]), Path::new(&args[2]), flag("--replan")),
        Some("review") if args.len() >= 3 => review::review(Path::new(&args[1]), Path::new(&args[2])),
        Some("resolve") if all.len() >= 3 => review::resolve(Path::new(&all[1]), Path::new(&all[2]), &all[3..]),
        Some("cut") if args.len() >= 3 => cut::cut(Path::new(&args[1]), Path::new(&args[2]), args.get(3).map(Path::new), flag("--unapproved")),
        Some("realm") if args.len() >= 3 => match (args[1].as_str(), args.len()) {
            ("build", 3) => realm::build(Path::new(&args[2]), Path::new("realm.json")),
            ("build", _) => realm::build(Path::new(&args[2]), Path::new(&args[3])),
            ("score", 3) => realm::score(Path::new("realm.json"), Path::new(&args[2])),
            ("score", _) => realm::score(Path::new(&args[2]), Path::new(&args[3])),
            _ => bail!("usage: retool realm build <refs_dir> [realm.json]\n       retool realm score [realm.json] <img|dir>"),
        },
        _ => bail!(
            "usage: retool card <raw_dir>                 (compliance report; video gate when ../video exists)\n       retool gate <video_dir> <out_dir>           (the video gate alone, stations from out/.meta)\n       retool run <raw_dir> <out_dir>               (finals + scorecard.md; realm.json from cwd)\n       retool deliver <realm.json> <out_dir> <delivery_dir>\n       retool realm build <refs_dir> [realm.json]\n       retool realm score [realm.json] <img|dir>\n       retool music stamp <dir|track>            (grid + verdict per track, MUSIC.md essentials)\n       retool music check <dir|track>            (is the stamped grid on the beat? click-track render to music-check/)\n       retool clips <video_dir> <out_dir>           (ingest + type the card's video -> out/video/clips.tsv)\n       retool plan <job_dir> <out_dir> [--replan]   (lay clips.tsv on the music grid -> out/video/cutlist.md + review.html; --replan discards an approval)\n       retool review <job_dir> <out_dir>            (serve the review page: swap/slip/reorder/drop/revive, Save, Approve -> the Resolve build)\n       retool resolve <job_dir> <out_dir> [--render none|check|all] [--template <drp>] [--replace] [--unapproved]  (the Resolve build from the shell, gate checked)\n       retool cut <job_dir> <out_dir> [cutlist.md] [--unapproved]  (ffmpeg render -> out/video/mls_branded.mp4 + mls_unbranded.mp4; needs approved:)\n       retool fuse <out_dir> <bracket.RAF>...\n       retool dev <out_dir> <frame.RAF>...\n       retool level <in.tif> <out.tif> [ref.RAF]     (lens data from the RAF)\n       retool tone <in.tif> <out.jpg>\n       retool sharp <img|dir>                       (blur measure per image; experimental, not a flag yet)\n       retool blend <flash.tif> <ambient.tif> <out.tif> [ambient_luma_weight]"
        ),
    }
}

struct Frame {
    path: std::path::PathBuf,
    t: chrono::NaiveDateTime,
    shutter: f64,
    iso: u64,
    flash_fired: bool,
    bracket: bool,
    focal_mm: f32,
}

/// A non-bracket frame longer than this (and longer than its station's
/// brackets) is a detail shot on the 18-50, not a pop.
const DETAIL_MIN_FOCAL_MM: f32 = 18.0;

/// Read EXIF for every RAF on the card via exiftool -json.
fn read_card(raw_dir: &Path) -> Result<Vec<Frame>> {
    let out = Command::new("exiftool")
        .args(["-json", "-q", "-DateTimeOriginal", "-ExposureTime", "-ISO", "-Flash", "-FocalLength", "-AutoBracketing", "-SequenceNumber", "-ext", "RAF"])
        .arg(raw_dir)
        .output()
        .context("run exiftool (brew install exiftool)")?;
    let rows: Vec<serde_json::Value> = serde_json::from_slice(&out.stdout).context("parse exiftool json")?;

    let parse_shutter = |v: &serde_json::Value| -> Option<f64> {
        if let Some(n) = v.as_f64() {
            return Some(n);
        }
        let s = v.as_str()?;
        match s.split_once('/') {
            Some((num, den)) => Some(num.trim().parse::<f64>().ok()? / den.trim().parse::<f64>().ok()?),
            None => s.trim().parse().ok(),
        }
    };

    let mut frames = Vec::new();
    for r in &rows {
        let get = |k: &str| r.get(k).cloned().unwrap_or(serde_json::Value::Null);
        frames.push(Frame {
            path: get("SourceFile").as_str().context("no SourceFile")?.into(),
            t: chrono::NaiveDateTime::parse_from_str(get("DateTimeOriginal").as_str().unwrap_or_default(), "%Y:%m:%d %H:%M:%S")
                .with_context(|| format!("bad DateTimeOriginal in {:?}", get("SourceFile")))?,
            shutter: parse_shutter(&get("ExposureTime")).context("bad ExposureTime")?,
            iso: get("ISO").as_u64().unwrap_or(0),
            flash_fired: get("Flash").as_str().unwrap_or_default().to_lowercase().contains("fired"),
            bracket: get("AutoBracketing").as_str().map(|s| s != "Off").unwrap_or(false)
                || get("SequenceNumber").as_u64().unwrap_or(0) > 0,
            focal_mm: get("FocalLength").as_str().and_then(|s| s.trim_end_matches("mm").trim().parse().ok()).unwrap_or(0.0),
        });
    }
    frames.sort_by(|a, b| (a.t, a.path.clone()).cmp(&(b.t, b.path.clone())));
    Ok(frames)
}

/// Cluster frames into stations: bursts split at >4s gaps, then a bracket
/// burst absorbs following non-bracket bursts (pops, a misfire then a pop,
/// several pops seconds apart) within 45s, and long-lens detail frames within 4 minutes.
/// A pop-less bracket followed by another bracket within 60s was a re-meter: only the last counts.
fn group_stations(frames: Vec<Frame>) -> Vec<Vec<Frame>> {
    let mut bursts: Vec<Vec<Frame>> = Vec::new();
    for f in frames {
        match bursts.last_mut() {
            Some(b) if (f.t - b.last().unwrap().t).num_seconds() <= 4 => b.push(f),
            _ => bursts.push(vec![f]),
        }
    }
    let mut stations: Vec<Vec<Frame>> = Vec::new();
    for b in bursts {
        let b_has_bracket = b.iter().any(|f| f.bracket);
        let absorb = !b_has_bracket
            && stations.last().is_some_and(|s| {
                let gap = (b[0].t - s.last().unwrap().t).num_seconds();
                let bracket_focal = s.iter().filter(|f| f.bracket).map(|f| f.focal_mm).fold(0.0f32, f32::max);
                let is_detail = b.iter().all(|f| f.focal_mm > bracket_focal.max(DETAIL_MIN_FOCAL_MM));
                s.iter().any(|f| f.bracket) && (gap <= 45 || (is_detail && gap <= 240))
            });
        if absorb {
            stations.last_mut().unwrap().extend(b);
        } else {
            // a bracket burst re-metered within 60s replaces the pop-less one before it
            let remeter = b_has_bracket
                && stations.last().is_some_and(|s| {
                    s.iter().all(|f| f.bracket && !f.flash_fired) && (b[0].t - s.last().unwrap().t).num_seconds() <= 60
                });
            if remeter {
                let dropped = stations.pop().unwrap();
                eprintln!(
                    "re-metered: dropping bracket {}..{}",
                    dropped[0].path.file_name().unwrap_or_default().to_string_lossy(),
                    dropped.last().unwrap().path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            stations.push(b);
        }
    }
    stations
}

/// Recipe-compliance notes for one station (empty = fully compliant).
fn compliance_notes(brackets: &[&Frame], pops: &[&Frame]) -> Vec<String> {
    let mut notes = Vec::new();
    if brackets.len() == 5 {
        let mut shutters: Vec<f64> = brackets.iter().map(|f| f.shutter).collect();
        shutters.sort_by(|a, b| a.total_cmp(b));
        let mid = shutters[2];
        let ladder: Vec<i32> = shutters.iter().map(|s| (s / mid).log2().round() as i32).collect();
        if ladder != [-2, -1, 0, 1, 2] {
            notes.push(format!("ladder {ladder:?} not [-2..+2]"));
        }
        // recipe: metered frame 1/30s or SLOWER (longer duration)
        if mid < 1.0 / 30.0 - 1e-9 {
            notes.push(format!("metered frame 1/{:.0}s faster than 1/30", 1.0 / mid));
        }
        let isos: Vec<u64> = brackets.iter().map(|f| f.iso).collect();
        if isos.iter().any(|&i| i != 125) {
            notes.push(format!("bracket ISO {:?} (recipe: 125)", isos[0]));
        }
    }
    for p in pops {
        if p.iso != 320 {
            notes.push(format!("pop ISO {} (recipe: 320)", p.iso));
        }
        if (p.shutter - 1.0 / 125.0).abs() > 1e-9 {
            notes.push(format!("pop 1/{:.0}s (recipe doc: 1/125)", 1.0 / p.shutter));
        }
    }
    notes
}

struct Station {
    n: usize,
    brackets: Vec<Frame>,
    pops: Vec<Frame>,
    details: Vec<Frame>,
    complete: bool,
    notes: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DetailMeta {
    file: String,
    frame: String,
    verticals: String,
}

/// What the machine measured while making one final; written next to it
/// (out/.meta/stNN.json) and read back for the scorecard.
#[derive(serde::Serialize, serde::Deserialize)]
struct Meta {
    station: String,
    brackets: Vec<String>,
    pop: String,
    recipe: Vec<String>,
    match_gain: f32,
    verticals: String,
    rotated_deg: Option<f32>,
    crop_pct: f32,
    tone_gain: f32,
    #[serde(default)]
    details: Vec<DetailMeta>,
    /// First bracket and pop, camera clock (exiftool format); the video ingest places clips by them.
    #[serde(default)]
    first_at: Option<String>,
    #[serde(default)]
    pop_at: Option<String>,
}

/// The card gate: group, print the per-station compliance report and a
/// verdict line. Runs in seconds (EXIF only), so it can be run at the door.
fn card_report(raw_dir: &Path, with_video: bool) -> Result<Vec<Station>> {
    let frames = read_card(raw_dir)?;
    let n_frames = frames.len();
    let grouped = group_stations(frames);
    println!("{} frames -> {} stations", n_frames, grouped.len());

    let mut stations = Vec::new();
    let mut deviations = 0;
    for (i, st) in grouped.into_iter().enumerate() {
        let (brackets, others): (Vec<Frame>, Vec<Frame>) = st.into_iter().partition(|f| f.bracket && !f.flash_fired);
        let bracket_focal = brackets.iter().map(|f| f.focal_mm).fold(0.0f32, f32::max);
        let (details, others): (Vec<Frame>, Vec<Frame>) =
            others.into_iter().partition(|f| f.focal_mm > bracket_focal.max(DETAIL_MIN_FOCAL_MM));
        let pops: Vec<Frame> = others.into_iter().filter(|f| f.flash_fired).collect();
        let notes = compliance_notes(&brackets.iter().collect::<Vec<_>>(), &pops.iter().collect::<Vec<_>>());
        let complete = brackets.len() == 5 && !pops.is_empty();
        let kind = if complete { "OK  " } else { "PART" };
        println!("st{:02} [{kind}] {} brk {} pop {} det  {}", i + 1, brackets.len(), pops.len(), details.len(), notes.join("; "));
        if complete && !notes.is_empty() {
            deviations += 1;
        }
        stations.push(Station { n: i + 1, brackets, pops, details, complete, notes });
    }
    let ok = stations.iter().filter(|s| s.complete).count();
    let verdict = if deviations == 0 { "CLEAN" } else { "REVIEW" };
    println!(
        "CARD: {ok} complete stations, {} partial, {deviations} with recipe deviations -> {verdict}",
        stations.len() - ok
    );
    // the video gate, when the job folder has a video/ next to the card
    let video_dir = raw_dir.parent().unwrap_or(Path::new(".")).join("video");
    if with_video && video_dir.is_dir() {
        let infos: Vec<video::StationInfo> = stations
            .iter()
            .filter_map(|s| {
                let first_at = s.brackets.iter().chain(&s.pops).chain(&s.details).map(|f| f.t).min()?;
                Some(video::StationInfo { n: s.n, final_jpg: PathBuf::new(), first_at, room: "unknown".into(), fit: 0.0 })
            })
            .collect();
        video::gate(&video_dir, &infos)?;
    }
    Ok(stations)
}

/// Full card pipeline: gate, then process every complete station
/// (fuse + pop + blend + level + tone) to out/stNN.jpg, then the scorecard.
fn run_card(raw_dir: &Path, out_dir: &Path) -> Result<()> {
    let stations = card_report(raw_dir, false)?;
    let meta_dir = out_dir.join(".meta");
    std::fs::create_dir_all(&meta_dir)?;
    let mut complete = 0;
    for st in stations.iter().filter(|s| s.complete) {
        let (n, brackets, pops) = (st.n, &st.brackets, &st.pops);
        complete += 1;
        let name = format!("st{n:02}");
        let final_jpg = out_dir.join(format!("{name}.jpg"));
        if final_jpg.exists() {
            println!("{name}: exists, skipping");
            continue;
        }
        let work = out_dir.join(".work");
        std::fs::create_dir_all(&work)?;

        let result = (|| -> Result<Meta> {
            let bracket_paths: Vec<String> = brackets.iter().map(|f| f.path.display().to_string()).collect();
            fuse(&work, &bracket_paths)?;
            // every pop at the station, developed and composited by luma: the
            // flambient light-painting of a big room, one flash aimed several ways
            let mut pop_tifs = Vec::new();
            for p in pops {
                let tif = work.join(format!("dev_{}.tif", p.path.file_stem().context("bad pop path")?.to_string_lossy()));
                develop_to_tiff(&p.path, &tif)?;
                pop_tifs.push(tif);
            }
            let pop_tif = if pop_tifs.len() == 1 {
                pop_tifs[0].clone()
            } else {
                let out = work.join("pops.tif");
                composite_pops(&pop_tifs, &out)?;
                out
            };
            let match_gain = blend(&pop_tif, &work.join("fused.tif"), &work.join("blended.tif"), 0.35)?;
            let geometry = level(&work.join("blended.tif"), &work.join("leveled.tif"), Some(&brackets[0].path))?;
            let tone_gain = tone(&work.join("leveled.tif"), &final_jpg)?;
            let file_name = |p: &Path| p.file_name().unwrap_or_default().to_string_lossy().into_owned();
            // details: one frame each, no fusion, same geometry and tone
            let mut details = Vec::new();
            for (i, d) in st.details.iter().enumerate() {
                let file = format!("{name}_d{}.jpg", i + 1);
                let tif = work.join("detail.tif");
                develop_to_tiff(&d.path, &tif)?;
                let geometry = level(&tif, &work.join("detail_leveled.tif"), Some(&d.path))?;
                tone(&work.join("detail_leveled.tif"), &out_dir.join(&file))?;
                details.push(DetailMeta { file, frame: file_name(&d.path), verticals: geometry.note });
            }
            Ok(Meta {
                station: name.clone(),
                brackets: brackets.iter().map(|f| file_name(&f.path)).collect(),
                pop: pops.iter().map(|p| file_name(&p.path)).collect::<Vec<_>>().join("+"),
                recipe: st.notes.clone(),
                match_gain,
                verticals: geometry.note,
                rotated_deg: geometry.rotated_deg,
                crop_pct: geometry.crop_pct,
                tone_gain,
                details,
                first_at: Some(brackets[0].t.format("%Y:%m:%d %H:%M:%S").to_string()),
                pop_at: Some(pops[0].t.format("%Y:%m:%d %H:%M:%S").to_string()),
            })
        })();
        std::fs::remove_dir_all(&work).ok();
        match result {
            Ok(meta) => {
                std::fs::write(meta_dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&meta)?)?;
                println!("{name}: done -> {}", final_jpg.display());
            }
            Err(e) => eprintln!("{name}: FAILED: {e:#}"),
        }
    }
    println!("{complete} complete stations processed into {}", out_dir.display());
    scorecard(out_dir)?;
    // the video, when the job folder has a video/ next to the card: clips → plan → cut
    let job_dir = raw_dir.parent().unwrap_or(Path::new("."));
    let video_dir = job_dir.join("video");
    if video_dir.is_dir() {
        video::clips(&video_dir, out_dir)?;
        let (j, o) = (job_dir.display(), out_dir.display());
        match cut::approved(out_dir) {
            Some(d) => println!("video: the plan is approved ({d}); `retool plan {j} {o} --replan` plans again"),
            None => cut::plan(job_dir, out_dir, false)?,
        }
        println!("video: the plan is this step's deliverable. Review and approve it, then build:\n  retool review {j} {o}      (Approve builds the Resolve project)\n  retool resolve {j} {o}     (the same build from the shell; --render all skips the QC pass)\n  retool cut {j} {o}         (the ffmpeg fallback, no Resolve)");
    }
    Ok(())
}

fn stem_of(p: &Path) -> String {
    p.file_stem().unwrap_or_default().to_string_lossy().into_owned()
}

/// The finals in an out dir, in station order: (wides stNN.jpg, details stNN_dK.jpg).
fn finals_in(out_dir: &Path) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut finals: Vec<PathBuf> = std::fs::read_dir(out_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jpg") && stem_of(p).starts_with("st"))
        .collect();
    finals.sort();
    Ok(finals.into_iter().partition(|p| !stem_of(p).contains("_d")))
}

/// The detail files belonging to a wide, in order.
fn details_of<'a>(wide: &Path, details: &'a [PathBuf]) -> Vec<&'a PathBuf> {
    let prefix = format!("{}_d", stem_of(wide));
    details.iter().filter(|d| stem_of(d).starts_with(&prefix)).collect()
}

// Room types a gallery must contain; each entry is satisfied by any of its
// labels (an exterior may type as porch).
const REQUIRED: [&[&str]; 5] = [&["house_view", "porch"], &["living_room"], &["kitchen"], &["bedroom"], &["bathroom"]];

fn coverage_line(labels: &[&str]) -> String {
    let missing: Vec<String> = REQUIRED
        .iter()
        .filter(|group| !labels.iter().any(|l| group.contains(l)))
        .map(|group| group.join("/"))
        .collect();
    if missing.is_empty() {
        "coverage: every required room type present".into()
    } else {
        format!("coverage: MISSING {}", missing.join(", "))
    }
}

/// out/scorecard.md: a card per final — realm type + fit, verticals, blend
/// and tone gains, recipe notes, flags — and the room coverage. Fit scores
/// need realm.json in the working directory.
fn scorecard(out_dir: &Path) -> Result<()> {
    let (finals, detail_files) = finals_in(out_dir)?;
    if finals.is_empty() {
        return Ok(());
    }
    let realm_path = Path::new("realm.json");
    if !realm_path.exists() {
        eprintln!("no realm.json in the working directory; scorecard skipped");
        return Ok(());
    }
    let realm = realm::load(realm_path)?;
    let embedder = realm::Embedder::new()?;
    let scored = realm::score_files(&realm, &embedder, &finals)?;
    let mut table = format!("# Scorecard — {}\n\n", out_dir.display());
    table += "fit = this image sits closer to the reference corpus than fit% of the references sit to each other. \
Low fit, in order of likelihood: technical defect (ghosting, darkness), room type missing from the corpus, \
clutter/staging, then composition.\n\n";
    table += "| image | type | fit | verticals | recipe | flags |\n|---|---|---|---|---|---|\n";
    let mut cards = String::new();
    for s in &scored {
        let stem = s.path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let meta: Option<Meta> = std::fs::read(out_dir.join(".meta").join(format!("{stem}.json")))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let mut flags: Vec<&str> = Vec::new();
        if s.no_ref {
            flags.push("no similar reference");
        }
        if s.fit < 10.0 {
            flags.push("low fit — review");
        }
        if meta.as_ref().is_some_and(|m| m.rotated_deg.is_none()) {
            flags.push("verticals left as shot");
        }
        let verticals = match meta.as_ref() {
            Some(Meta { rotated_deg: Some(d), crop_pct, .. }) => format!("{d:.2}° · crop {crop_pct:.1}%"),
            Some(_) => "as shot".into(),
            None => String::new(),
        };
        let recipe = meta.as_ref().map(|m| m.recipe.join("; ")).unwrap_or_default();
        table += &format!("| {stem}.jpg | {} | {:.0} | {verticals} | {recipe} | {} |\n", s.label, s.fit, flags.join(", "));
        for d in details_of(&s.path, &detail_files) {
            table += &format!("| {}.jpg | detail of {stem} | – | | | |\n", stem_of(d));
        }

        let headline_flags = if flags.is_empty() { String::new() } else { format!(" — {}", flags.join(", ")) };
        cards += &format!("\n## {stem}.jpg — {}, fit {:.0}{headline_flags}\n", s.label, s.fit);
        if let Some(m) = &meta {
            cards += &format!("- frames: {} + pop {}\n", m.brackets.join(", "), m.pop);
            cards += &format!("- recipe: {}\n", if m.recipe.is_empty() { "compliant".to_string() } else { m.recipe.join("; ") });
            cards += &format!("- verticals: {}\n", m.verticals);
            cards += &format!("- blend: ambient matched {:.2}x, luma weight 0.35 · tone gain {:.2}x\n", m.match_gain, m.tone_gain);
            for d in &m.details {
                cards += &format!("- detail {}: from {} alone · {}\n", d.file, d.frame, d.verticals);
            }
        }
        cards += &format!("- nearest reference: {} (d={:.3})\n", s.nearest, s.dist);
    }
    let labels: Vec<&str> = scored.iter().map(|s| s.label.as_str()).collect();
    let md = format!("{table}\n{}\n{cards}", coverage_line(&labels));
    let path = out_dir.join("scorecard.md");
    std::fs::write(&path, md)?;
    println!("{}", path.display());
    Ok(())
}

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) }
}

fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 { x * 12.92 } else { 1.055 * x.powf(1.0 / 2.4) - 0.055 }
}

/// Bright/airy global tone: exposure gain to a target median in linear light,
/// with a soft shoulder so highlights (windows) roll off instead of clipping.
fn tone(input: &Path, output: &Path) -> Result<f32> {
    const TARGET_MEDIAN: f32 = 0.38; // linear-light luma target (~0.65 in sRGB); 0.30 read dark on the first video card, 2026-09-03
    const MAX_GAIN: f32 = 8.0;

    let mut reader = image::ImageReader::open(input)?;
    reader.no_limits();
    let img = reader.decode()?.to_rgb32f();
    let (w, h) = (img.width(), img.height());

    let mut linear: Vec<f32> = img.into_raw();
    for v in linear.iter_mut() {
        *v = srgb_to_linear(*v);
    }

    // median of linear luma, sampled on a grid for speed
    let mut lumas: Vec<f32> = linear
        .chunks_exact(3)
        .step_by(101)
        .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
        .collect();
    lumas.sort_by(|a, b| a.total_cmp(b));
    let median = lumas[lumas.len() / 2].max(1e-6);
    let gain = (TARGET_MEDIAN / median).clamp(1.0 / MAX_GAIN, MAX_GAIN);
    eprintln!("median luma {median:.4} -> gain {gain:.2}x");

    let out: Vec<u8> = linear
        .iter()
        .map(|&v| {
            let g = v * gain;
            // Reinhard-style shoulder above 0.7 linear keeps window detail
            let toned = if g <= 0.7 { g } else { 0.7 + 0.3 * (1.0 - (-(g - 0.7) / 0.3).exp()) };
            (linear_to_srgb(toned.clamp(0.0, 1.0)) * 255.0 + 0.5) as u8
        })
        .collect();

    let buf: image::RgbImage = image::ImageBuffer::from_raw(w, h, out).context("buffer size mismatch")?;
    buf.save(output)?;
    println!("{}", output.display());
    Ok(gain)
}

/// Decode a RAF and develop it (demosaic, WB, calibrate, sRGB) to a 16-bit TIFF.
/// X-Trans goes through rawler's Markesteijn demosaic instead of the bilinear
/// one its developer picks by itself, which read soft at 100 percent on the
/// first card (2026-09-03): the developer rescales the sensor plane, we
/// demosaic it, and hand the three channels back as a linear raw for white
/// balance, calibration, crop and gamma.
fn develop_to_tiff(raf: &Path, out: &Path) -> Result<()> {
    use rawler::imgop::develop::{Intermediate, ProcessingStep};
    use rawler::imgop::sensor::{xtrans::markesteijn::XTransMarkesteijnDemosaic, Demosaic, SensorType};
    use rawler::rawimage::RawPhotometricInterpretation;
    let raw = rawler::decode_file(raf).with_context(|| format!("decode {}", raf.display()))?;
    let xtrans = match &raw.photometric {
        RawPhotometricInterpretation::Cfa(c) => c.sensor == SensorType::Xtrans && raw.cpp == 1,
        _ => false,
    };
    let developed = if xtrans {
        let plane = match RawDevelop::new_with(&[ProcessingStep::Rescale]).develop_intermediate(&raw)? {
            Intermediate::Monochrome(p) => p,
            _ => bail!("{}: unexpected sensor plane", raf.display()),
        };
        let RawPhotometricInterpretation::Cfa(config) = &raw.photometric else { unreachable!() };
        let roi = plane.rect();
        let rgb = XTransMarkesteijnDemosaic::new_pass_1().demosaic(&plane, &config.cfa, &config.colors, roi);
        let mut three = raw.clone();
        three.cpp = 3;
        three.width = rgb.width;
        three.height = rgb.height;
        three.data = rawler::RawImageData::Float(rgb.into_flatten());
        RawDevelop::new_with(&[ProcessingStep::WhiteBalance, ProcessingStep::Calibrate, ProcessingStep::CropDefault, ProcessingStep::SRgb]).develop_intermediate(&three)?
    } else {
        RawDevelop::default().develop_intermediate(&raw)?
    };
    let img = developed.to_dynamic_image().context("develop produced no image")?;
    img.to_rgb16().save(out)?;
    Ok(())
}

/// Develop each bracket frame, then exposure-fuse them with enfuse.
fn fuse(out_dir: &Path, rafs: &[String]) -> Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let mut tiffs = Vec::new();
    for raf in rafs {
        let raf = Path::new(raf);
        let stem = raf.file_stem().context("bad path")?.to_string_lossy();
        let tif = out_dir.join(format!("dev_{stem}.tif"));
        if !tif.exists() {
            eprintln!("developing {}", raf.display());
            develop_to_tiff(raf, &tif)?;
        }
        tiffs.push(tif);
    }

    let fused = out_dir.join("fused.tif");
    if fused.exists() {
        eprintln!("fused.tif exists, skipping enfuse");
        return preview_jpeg(out_dir, &fused);
    }
    eprintln!("enfusing {} frames", tiffs.len());
    let status = Command::new(ENFUSE)
        .args(["--depth", "16", "--exposure-weight", "1", "--saturation-weight", "0.2", "--contrast-weight", "0"])
        .arg("-o")
        .arg(&fused)
        .args(&tiffs)
        .status()
        .context("run enfuse (is Hugin installed?)")?;
    if !status.success() {
        bail!("enfuse failed: {status}");
    }
    preview_jpeg(out_dir, &fused)
}

fn load_linear_rgb(path: &Path) -> Result<(u32, u32, Vec<f32>)> {
    let mut reader = image::ImageReader::open(path)?;
    reader.no_limits();
    let img = reader.decode()?.to_rgb32f();
    let (w, h) = (img.width(), img.height());
    let mut data = img.into_raw();
    for v in data.iter_mut() {
        *v = srgb_to_linear(*v);
    }
    Ok((w, h, data))
}

fn luma(p: &[f32]) -> f32 {
    0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]
}

fn median_luma(data: &[f32]) -> f32 {
    let mut lumas: Vec<f32> = data.chunks_exact(3).step_by(101).map(luma).collect();
    lumas.sort_by(|a, b| a.total_cmp(b));
    lumas[lumas.len() / 2].max(1e-6)
}

/// The flambient blend (Cool stack, luminosity mode): chroma comes from the
/// flash frame, luminance is a mix of flash and exposure-matched ambient.
fn blend(flash: &Path, ambient: &Path, out: &Path, alpha: f32) -> Result<f32> {
    let (w, h, flash_px) = load_linear_rgb(flash)?;
    let (aw, ah, mut amb_px) = load_linear_rgb(ambient)?;
    if (w, h) != (aw, ah) {
        bail!("dimension mismatch: flash {w}x{h} vs ambient {aw}x{ah}");
    }

    // exposure-match ambient to the flash frame before mixing luminance,
    // like an editor equalizing layers first
    let match_gain = (median_luma(&flash_px) / median_luma(&amb_px)).clamp(0.1, 16.0);
    eprintln!("ambient exposure-match gain {match_gain:.2}x, luma weight {alpha}");
    for v in amb_px.iter_mut() {
        *v *= match_gain;
    }

    let blended: Vec<u16> = flash_px
        .chunks_exact(3)
        .zip(amb_px.chunks_exact(3))
        .flat_map(|(f, a)| {
            let lf = luma(f).max(1e-6);
            let target = (1.0 - alpha) * lf + alpha * luma(a);
            let ratio = (target / lf).clamp(0.0, 4.0);
            [f[0], f[1], f[2]].map(|c| (linear_to_srgb((c * ratio).clamp(0.0, 1.0)) * 65535.0 + 0.5) as u16)
        })
        .collect();

    let buf: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> =
        image::ImageBuffer::from_raw(w, h, blended).context("buffer size mismatch")?;
    buf.save(out)?;
    println!("{}", out.display());
    Ok(match_gain)
}

/// Focal length and the camera's own distortion table, from the RAF's EXIF.
fn lens_from_raf(raf: &Path, width: u32) -> Result<verticals::Lens> {
    let out = Command::new("exiftool")
        .args(["-n", "-s3", "-FocalLength", "-GeometricDistortionParams"])
        .arg(raf)
        .output()
        .context("run exiftool")?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut lines = text.lines();
    let focal_mm: f32 = lines.next().and_then(|l| l.trim().parse().ok()).context("no FocalLength in RAF")?;
    let vals: Vec<f32> = lines.next().unwrap_or_default().split_whitespace().filter_map(|v| v.parse().ok()).collect();
    // first value = half-diagonal / 9; then 9 radii (fraction of half-diagonal), 9 distortion %
    let distortion = if vals.len() == 19 { vals[1..10].iter().copied().zip(vals[10..19].iter().copied()).collect() } else { Vec::new() };
    Ok(verticals::Lens { focal_px: focal_mm / SENSOR_WIDTH_MM * width as f32, distortion })
}

/// Geometry once, on the flattened blend, before tone: undistort with the
/// lens table from the RAF, then rotate the camera so world verticals are
/// image verticals (undoes residual roll + pitch), crop to native aspect.
fn level(input: &Path, output: &Path, raf: Option<&Path>) -> Result<verticals::Report> {
    let (w, h, px) = load_linear_rgb(input)?;
    let lens = match raf {
        Some(raf) => lens_from_raf(raf, w)?,
        None => verticals::Lens { focal_px: 11.0 / SENSOR_WIDTH_MM * w as f32, distortion: Vec::new() },
    };
    let (out, report) = verticals::rectify(w, h, &px, &lens);
    eprintln!("verticals: {}", report.note);
    let encoded: Vec<u16> = out.iter().map(|&v| (linear_to_srgb(v.clamp(0.0, 1.0)) * 65535.0 + 0.5) as u16).collect();
    let buf: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> =
        image::ImageBuffer::from_raw(w, h, encoded).context("buffer size mismatch")?;
    buf.save(output)?;
    println!("{}", output.display());
    Ok(report)
}

/// Delivery gallery: type each final via the realm, order it the way an
/// MLS gallery reads, export 2048px sRGB JPEGs, report room coverage.
fn deliver(realm_path: &Path, out_dir: &Path, dest: &Path) -> Result<()> {
    const ORDER: [&str; 10] = [
        "house_view", "porch", "living_room", "dining_room", "kitchen", "bedroom", "bathroom", "patio", "yard", "aerial_view",
    ];
    const LONG_EDGE: u32 = 2048;

    let (finals, detail_files) = finals_in(out_dir)?;
    if finals.is_empty() {
        bail!("no st*.jpg under {}", out_dir.display());
    }
    let realm = realm::load(realm_path)?;
    let embedder = realm::Embedder::new()?;
    let mut scored = realm::score_files(&realm, &embedder, &finals)?;
    let rank = |label: &str| ORDER.iter().position(|o| *o == label).unwrap_or(ORDER.len());
    scored.sort_by(|a, b| (rank(&a.label), &a.path).cmp(&(rank(&b.label), &b.path)));

    std::fs::create_dir_all(dest)?;
    let export = |src: &Path, out: &Path| -> Result<()> {
        let img = image::ImageReader::open(src)?.decode()?.to_rgb8();
        let (w, h) = (img.width(), img.height());
        let scale = LONG_EDGE as f32 / w.max(h) as f32;
        let small = image::imageops::resize(&img, (w as f32 * scale) as u32, (h as f32 * scale) as u32, image::imageops::FilterType::Lanczos3);
        let mut file = std::io::BufWriter::new(std::fs::File::create(out)?);
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 92).encode_image(&small)?;
        Ok(())
    };
    println!("{:>3}  {:<14} {:>4}  station", "#", "type", "fit");
    let mut n = 0;
    for s in &scored {
        n += 1;
        export(&s.path, &dest.join(format!("{n:02}_{}.jpg", s.label)))?;
        let flag = if s.no_ref { "  [no similar reference — review]" } else if s.fit < 10.0 { "  [low fit — review]" } else { "" };
        println!("{n:>3}  {:<14} {:>4.0}  {}{flag}", s.label, s.fit, stem_of(&s.path));
        for d in details_of(&s.path, &detail_files) {
            n += 1;
            export(d, &dest.join(format!("{n:02}_{}_detail.jpg", s.label)))?;
            println!("{n:>3}  {:<14} {:>4}  {}", format!("{} detail", s.label), "–", stem_of(d));
        }
    }
    let labels: Vec<&str> = scored.iter().map(|s| s.label.as_str()).collect();
    println!("{}", coverage_line(&labels));
    println!("{n} images -> {}", dest.display());
    // the video package, when it has been cut
    let mut videos = 0;
    for f in ["mls_branded.mp4", "mls_unbranded.mp4", "reel_01.mp4", "reel_02.mp4", "reel_03.mp4"] {
        let src = out_dir.join("video").join(f);
        if src.exists() {
            std::fs::copy(&src, dest.join(f))?;
            videos += 1;
            println!("video  {f}");
        }
    }
    if videos > 0 {
        println!("{videos} videos -> {}", dest.display());
    }
    Ok(())
}

fn preview_jpeg(out_dir: &Path, fused: &Path) -> Result<()> {
    let preview = out_dir.join("fused.jpg");
    let mut reader = image::ImageReader::open(fused)?;
    reader.no_limits(); // our own 40MP 16-bit TIFF exceeds the decoder's default cap
    reader.decode()?.to_rgb8().save(&preview)?;
    println!("{}", preview.display());
    Ok(())
}

/// Several pops of one station into one flash frame: per pixel, the whole
/// pixel from the pop that lit it most, so bounces of different colour do
/// not fringe where they overlap. Same station, same framing, same bank.
fn composite_pops(tifs: &[PathBuf], out: &Path) -> Result<()> {
    let (w, h, mut best) = load_linear_rgb(&tifs[0])?;
    for tif in &tifs[1..] {
        let (pw, ph, px) = load_linear_rgb(tif)?;
        if (pw, ph) != (w, h) {
            bail!("pop size mismatch: {}x{} vs {w}x{h}", pw, ph);
        }
        for (b, p) in best.chunks_exact_mut(3).zip(px.chunks_exact(3)) {
            if luma(p) > luma(b) {
                b.copy_from_slice(p);
            }
        }
    }
    let encoded: Vec<u16> = best.iter().map(|&c| (linear_to_srgb(c.clamp(0.0, 1.0)) * 65535.0 + 0.5) as u16).collect();
    let buf: image::ImageBuffer<image::Rgb<u16>, Vec<u16>> = image::ImageBuffer::from_raw(w, h, encoded).context("buffer size mismatch")?;
    buf.save(out)?;
    eprintln!("{} pops composited by luma", tifs.len());
    Ok(())
}

/// Blur measure: mean |Laplacian| over mean |gradient| on a 1600 px grey
/// copy. Blur shrinks second differences faster than first ones, so a soft
/// frame scores low whatever the room holds.
fn sharpness(path: &Path) -> Result<f32> {
    let mut reader = image::ImageReader::open(path)?;
    reader.no_limits();
    let img = reader.decode()?.to_luma32f();
    let scale = 1600.0 / img.width().max(1) as f32;
    let (nw, nh) = ((img.width() as f32 * scale).max(3.0) as u32, (img.height() as f32 * scale).max(3.0) as u32);
    let g = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    let px = |x: u32, y: u32| g.get_pixel(x, y).0[0];
    let (mut lap, mut grad) = (0f64, 0f64);
    for y in 1..nh - 1 {
        for x in 1..nw - 1 {
            let c = px(x, y);
            let l = 4.0 * c - px(x - 1, y) - px(x + 1, y) - px(x, y - 1) - px(x, y + 1);
            let (gx, gy) = (px(x + 1, y) - px(x - 1, y), px(x, y + 1) - px(x, y - 1));
            lap += l.abs() as f64;
            grad += (gx * gx + gy * gy).sqrt() as f64;
        }
    }
    Ok((lap / grad.max(1e-9)) as f32)
}

/// `retool sharp <img|dir>`: the blur measure per image. Not yet a scorecard
/// flag: on the first card it ranked the operator's four soft frames wrong
/// (exteriors full of foliage score high whatever their focus).
fn sharp_report(target: &Path) -> Result<()> {
    let mut files: Vec<PathBuf> = if target.is_dir() {
        std::fs::read_dir(target)?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| ["jpg", "jpeg", "png", "tif", "tiff"].contains(&e.to_string_lossy().to_lowercase().as_str()))).collect()
    } else {
        vec![target.to_path_buf()]
    };
    files.sort();
    for f in files {
        println!("{:.3}  {}", sharpness(&f)?, f.display());
    }
    Ok(())
}
