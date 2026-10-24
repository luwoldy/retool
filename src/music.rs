//! Music screening: stamp a track's grid (bpm, accent, phrases, ending)
//! from its audio and say whether it passes the operator's six essentials
//! (MUSIC.md). ffmpeg decodes; the analysis is a plain onset envelope, an
//! autocorrelation for the tempo, and RMS for the hit and the ending.
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

const SR: usize = 22050;
const HOP: usize = 256; // ~11.6 ms per hop
const BPM_MIN: f32 = 90.0;
const BPM_MAX: f32 = 115.0;
const ACCENT_SEARCH_S: (f32, f32) = (8.0, 25.0); // the master starts at accent − shelf, so anywhere here works
const MIN_PHRASES: u32 = 6; // ~58 s of full arrangement at 100 BPM
const END_DBFS: f32 = -35.0; // below this the track is over
const FADE_DB_PER_S: f32 = -1.0; // level falling this fast while the beat continues = fade-out

pub struct Stamp {
    pub duration_s: f32,
    pub bpm: f32,
    pub bpm_confidence: f32,
    pub accent_s: Option<f32>,
    pub phrases_after_accent: u32,
    pub ending_start_s: f32,
    pub end_s: f32,
    /// failures against the essentials; empty = PASS
    pub notes: Vec<String>,
    pub ring_s: f32,
}

/// Stamp one track or every track in a folder; prints a markdown table whose
/// columns match the stamp table in MUSIC.md, plus a verdict.
pub fn stamp(target: &Path) -> Result<()> {
    let files: Vec<PathBuf> = if target.is_dir() {
        let mut v: Vec<PathBuf> = std::fs::read_dir(target)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                ["wav", "mp3", "m4a", "aac", "flac", "ogg", "aif", "aiff"].contains(&ext.as_str())
            })
            .collect();
        v.sort();
        v
    } else {
        vec![target.to_path_buf()]
    };
    if files.is_empty() {
        bail!("no audio files at {}", target.display());
    }
    println!("| file | length | bpm | accent_s | phrases_after_accent | ending_start_s | end_s | verdict |");
    println!("|---|---|---|---|---|---|---|---|");
    for f in &files {
        let name = f.file_name().unwrap_or_default().to_string_lossy();
        match analyse(f) {
            Ok(s) => {
                let mut verdict = if s.notes.is_empty() { "PASS".to_string() } else { s.notes.join("; ") };
                if s.ring_s > 6.0 {
                    verdict += &format!(" ({:.0} s ring-out after the last bar)", s.ring_s);
                }
                println!(
                    "| {name} | {}:{:02} | {:.1} ({:.0}%) | {} | {} | {:.1} | {:.1} | {verdict} |",
                    (s.duration_s / 60.0) as u32,
                    (s.duration_s % 60.0) as u32,
                    s.bpm,
                    100.0 * s.bpm_confidence,
                    s.accent_s.map(|a| format!("{a:.1}")).unwrap_or_else(|| "none".into()),
                    s.phrases_after_accent,
                    s.ending_start_s,
                    s.end_s,
                );
            }
            Err(e) => println!("| {name} | | | | | | | ERROR {e:#} |"),
        }
    }
    Ok(())
}

/// Mono float samples at SR via ffmpeg.
fn decode(path: &Path) -> Result<Vec<f32>> {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-ac", "1", "-ar", &SR.to_string(), "-f", "f32le", "-"])
        .output()
        .context("run ffmpeg")?;
    if !out.status.success() {
        bail!("ffmpeg could not decode {}: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
}

fn analyse(path: &Path) -> Result<Stamp> {
    let x = decode(path)?;
    let hop_s = HOP as f32 / SR as f32;
    let n = x.len() / HOP;
    if n < (20.0 / hop_s) as usize {
        bail!("shorter than 20 s");
    }

    let (onset, rms_s) = envelopes(&x, n);

    // tempo: autocorrelation of the onset envelope, refined over multiples
    let (bpm, bpm_confidence) = tempo(&onset, hop_s);
    let bar = 240.0 / bpm;
    let phrase = 4.0 * bar;

    // accent: the strongest onset in the search window whose level jumps
    let accent_s = accent(&onset, &rms_s, hop_s);

    // the last real bar ends after the last strong beat, not where the
    // ring-out reaches silence; snapped to the bar grid from the accent
    let mut sorted = onset.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let strong_level = 0.5 * sorted[n * 9 / 10];
    let strong: Vec<bool> = onset.iter().map(|&o| o > strong_level).collect();
    let last_beat = (0..n).rev().find(|&i| strong[i]).unwrap_or(n - 1);
    let end_raw = last_beat as f32 * hop_s;
    let floor = 10f32.powf(END_DBFS / 20.0);
    let silent = (0..n).rev().find(|&i| rms_s[i] > floor).unwrap_or(n - 1);
    let ring_s = (silent as f32 * hop_s - end_raw).max(0.0);
    let origin = accent_s.unwrap_or(0.0);
    let end_s = origin + ((end_raw - origin) / bar).ceil() * bar;
    let ending_start_s = (end_s - phrase).max(origin);

    // fade: the level falls steadily over the last 12 s of beats while the
    // beats keep coming, and never comes back up. A final hit or swell that
    // rings out is a composed ending, however long the ring.
    let fade = {
        let db = |i: usize| 20.0 * (rms_s[i] + 1e-6).log10();
        let a = ((end_raw - 12.0) / hop_s).max(0.0) as usize;
        let b = last_beat.max(a + 1);
        let pts: Vec<(f32, f32)> = (a..b).step_by(4).map(|i| (i as f32 * hop_s, db(i))).collect();
        let o = (origin / hop_s) as usize;
        let density = |lo: usize, hi: usize| strong[lo..hi.max(lo + 1)].iter().filter(|&&s| s).count() as f32 / (hi - lo).max(1) as f32;
        let level_start = (a..(a + (2.0 / hop_s) as usize).min(b)).map(db).fold(f32::MIN, f32::max);
        let level_end = (b.saturating_sub((4.0 / hop_s) as usize)..b).map(db).fold(f32::MIN, f32::max);
        slope(&pts) < FADE_DB_PER_S && density(a, b) >= 0.5 * density(o, last_beat.max(o + 1)) && level_start - level_end > 6.0
    };

    // phrases of full arrangement after the accent
    let phrases_after_accent = match accent_s {
        Some(acc) => {
            let mean_over = |t0: f32, t1: f32| -> f32 {
                let (i0, i1) = ((t0 / hop_s) as usize, ((t1 / hop_s) as usize).min(n));
                if i1 <= i0 { 0.0 } else { rms_s[i0..i1].iter().sum::<f32>() / (i1 - i0) as f32 }
            };
            // a phrase is full when at least three of its four bars hold the
            // level of the first phrase after the hit (one-bar breaks are arrangement, not thinning)
            let baseline = mean_over(acc, acc + phrase);
            let mut k = 0;
            while acc + (k + 1) as f32 * phrase <= ending_start_s + 0.01 {
                let t0 = acc + k as f32 * phrase;
                let full_bars = (0..4).filter(|&j| mean_over(t0 + j as f32 * bar, t0 + (j + 1) as f32 * bar) >= 0.6 * baseline).count();
                if full_bars < 3 {
                    break;
                }
                k += 1;
            }
            k
        }
        None => 0,
    };

    let mut notes = Vec::new();
    if !(BPM_MIN..=BPM_MAX).contains(&bpm) {
        notes.push(format!("bpm {bpm:.0} outside {BPM_MIN:.0}–{BPM_MAX:.0}"));
    }
    if accent_s.is_none() {
        notes.push("no accent hit in 8–25 s".into());
    }
    if phrases_after_accent < MIN_PHRASES {
        notes.push(format!("{phrases_after_accent} full phrases after the accent, need {MIN_PHRASES}"));
    }
    if fade {
        notes.push("fades out, no composed ending".into());
    }

    Ok(Stamp {
        duration_s: x.len() as f32 / SR as f32,
        bpm,
        bpm_confidence,
        accent_s,
        phrases_after_accent,
        ending_start_s,
        end_s,
        notes,
        ring_s,
    })
}

fn smooth(v: &[f32], w: usize) -> Vec<f32> {
    let h = w / 2;
    (0..v.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(h), (i + h + 1).min(v.len()));
            v[a..b].iter().sum::<f32>() / (b - a) as f32
        })
        .collect()
}

fn slope(pts: &[(f32, f32)]) -> f32 {
    let n = pts.len() as f32;
    if n < 2.0 {
        return 0.0;
    }
    let (mx, my) = (pts.iter().map(|p| p.0).sum::<f32>() / n, pts.iter().map(|p| p.1).sum::<f32>() / n);
    let num: f32 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let den: f32 = pts.iter().map(|p| (p.0 - mx) * (p.0 - mx)).sum();
    if den == 0.0 { 0.0 } else { num / den }
}

fn autocorr(x: &[f32], lag: usize) -> f32 {
    let n = x.len().saturating_sub(lag);
    if n == 0 {
        return 0.0;
    }
    x[..n].iter().zip(&x[lag..]).map(|(a, b)| a * b).sum::<f32>() / n as f32
}

/// Beat period from the onset autocorrelation: best integer lag in the
/// 60–180 BPM range, refined to a fraction of a hop over its multiples,
/// then folded (×2, ÷2) into the recipe's range when that period also rings.
fn tempo(onset: &[f32], hop_s: f32) -> (f32, f32) {
    let lag_of = |bpm: f32| 60.0 / bpm / hop_s;
    let (lo, hi) = (lag_of(180.0) as usize, lag_of(60.0) as usize + 1);
    let ac0 = autocorr(onset, 0).max(1e-9);
    let ac: Vec<f32> = (0..=hi * 8 + 4).map(|l| autocorr(onset, l) / ac0).collect();
    let best = (lo..=hi).max_by(|&a, &b| ac[a].total_cmp(&ac[b])).unwrap();
    let mean_ac = ac[lo..=hi].iter().sum::<f32>() / (hi - lo + 1) as f32;
    let confidence = ((ac[best] - mean_ac) / (ac[best] + 1e-9)).clamp(0.0, 1.0);

    // refine: parabolic peaks around m × best, averaged
    let refine = |base: f32| -> f32 {
        let mut periods = Vec::new();
        for m in 1..=8 {
            let c = (m as f32 * base).round() as usize;
            if c + 4 >= ac.len() {
                break;
            }
            let k = (c - 3..=c + 3).max_by(|&a, &b| ac[a].total_cmp(&ac[b])).unwrap();
            let (y0, y1, y2) = (ac[k - 1], ac[k], ac[k + 1]);
            let denom = y0 - 2.0 * y1 + y2;
            let off = if denom.abs() < 1e-9 { 0.0 } else { 0.5 * (y0 - y2) / denom };
            periods.push((k as f32 + off) / m as f32);
        }
        periods.iter().sum::<f32>() / periods.len() as f32
    };
    let period = refine(best as f32);
    let bpm_of = |p: f32| 60.0 / (p * hop_s);

    // fold into range if the folded period also rings: half and double
    // time, and the 3:2 / 4:3 dotted lags a swung hi-hat rings at (lo-fi
    // beats otherwise read at two thirds of their tempo)
    let mut candidates = vec![(period, ac[best])];
    for p in [0.5, 2.0, 2.0 / 3.0, 1.5, 0.75, 4.0 / 3.0].map(|f| period * f) {
        let k = p.round() as usize;
        if k >= lo / 2 && k + 1 < ac.len() {
            candidates.push((refine(p), ac[k]));
        }
    }
    let in_range: Vec<&(f32, f32)> = candidates.iter().filter(|(p, _)| (BPM_MIN..=BPM_MAX).contains(&bpm_of(*p))).collect();
    let chosen = in_range
        .iter()
        .filter(|(_, a)| *a >= 0.5 * ac[best])
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|c| c.0)
        .unwrap_or(period);
    (bpm_of(chosen), confidence)
}

/// The hit: the biggest sustained level step in the search window (four
/// seconds after against four before), refined to the strongest onset
/// within a third of a second of it.
fn accent(onset: &[f32], rms_s: &[f32], hop_s: f32) -> Option<f32> {
    let n = rms_s.len();
    let win = (4.0 / hop_s) as usize;
    let (a, b) = ((ACCENT_SEARCH_S.0 / hop_s) as usize, ((ACCENT_SEARCH_S.1 / hop_s) as usize).min(n - win - 1));
    let mean = |s: &[f32]| s.iter().sum::<f32>() / s.len().max(1) as f32;
    let (mut best_i, mut best_ratio) = (0usize, 0f32);
    for i in a..b {
        let ratio = mean(&rms_s[i..i + win]) / mean(&rms_s[i - win..i]).max(1e-6);
        if ratio > best_ratio {
            (best_i, best_ratio) = (i, ratio);
        }
    }
    if best_ratio < 1.4 {
        return None;
    }
    let r = (0.3 / hop_s) as usize;
    let i = (best_i.saturating_sub(r)..(best_i + r).min(n - 1)).max_by(|&x, &y| onset[x].total_cmp(&onset[y])).unwrap_or(best_i);
    Some(i as f32 * hop_s)
}

/// Per-hop onset envelope (half-wave rectified rise of log transient energy,
/// smoothed) and smoothed RMS level, from mono samples at SR.
fn envelopes(x: &[f32], n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut rms = vec![0f32; n];
    let mut hp = vec![0f32; n];
    for i in 0..n {
        let frame = &x[i * HOP..(i + 1) * HOP];
        let e: f32 = frame.iter().map(|v| v * v).sum::<f32>() / HOP as f32;
        rms[i] = e.sqrt();
        let start = if i == 0 { 1 } else { 0 };
        let d: f32 = (start..HOP)
            .map(|k| {
                let prev = if k == 0 { x[i * HOP - 1] } else { frame[k - 1] };
                let v = frame[k] - prev;
                v * v
            })
            .sum::<f32>()
            / HOP as f32;
        hp[i] = d.sqrt();
    }
    let rms_s = smooth(&rms, 9);
    let mut onset = vec![0f32; n];
    for i in 1..n {
        onset[i] = ((hp[i] + 1e-6).ln() - (hp[i - 1] + 1e-6).ln()).max(0.0);
    }
    (smooth(&onset, 3), rms_s)
}

/// Mean onset strength on a grid of `period` from `first` up to `until`.
fn grid_score(onset: &[f32], hop_s: f32, first: f32, period: f32, until: f32) -> f32 {
    let (mut sum, mut k) = (0.0f32, 0usize);
    let mut t = first;
    while t < until {
        if t >= 0.0 {
            if let Some(v) = onset.get((t / hop_s).round() as usize) {
                sum += v;
                k += 1;
            }
        }
        t += period;
    }
    if k == 0 { 0.0 } else { sum / k as f32 }
}

/// `retool music check <dir|track>`: does the stamped grid sit on the beat?
/// The onset envelope is scored on the grid over a phase scan of ± half a
/// beat, then on each of the four beat positions of the bar to find the
/// downbeat, and the track is rendered with a click on the grid to
/// `music-check/<name>.m4a` so the operator can hear it. A grid that is off
/// puts every cut of every film off by the same amount.
pub fn check(target: &Path) -> Result<()> {
    let stamps = crate::cut::read_stamps()?;
    let mut files: Vec<std::path::PathBuf> = if target.is_dir() {
        std::fs::read_dir(target)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| ["mp3", "wav", "m4a", "aac", "flac", "aif", "aiff"].contains(&e.to_string_lossy().to_lowercase().as_str())))
            .filter(|p| !p.file_name().unwrap_or_default().to_string_lossy().starts_with('.'))
            .collect()
    } else {
        vec![target.to_path_buf()]
    };
    files.sort();
    std::fs::create_dir_all("music-check")?;
    println!("| file | bpm | accent_s | grid phase | downbeat | verdict |");
    println!("|---|---|---|---|---|---|");
    for f in files {
        let name = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
        let Some(s) = stamps.iter().find(|t| t.file == name) else {
            println!("| {name} | – | – | – | – | no stamp in MUSIC.md |");
            continue;
        };
        let x = decode(&f)?;
        let hop_s = HOP as f32 / SR as f32;
        let n = x.len() / HOP;
        let (onset, _) = envelopes(&x, n);
        let beat = 60.0 / s.bpm as f32;
        let accent = s.accent_s as f32;
        let until = (s.end_s as f32).min(n as f32 * hop_s);
        let first_from = |t0: f32, period: f32| t0 - (t0 / period).floor() * period;
        // phase: where within ± half a beat the onsets actually sit
        let mut best = (0.0f32, f32::MIN);
        let mut phase = -beat / 2.0;
        while phase < beat / 2.0 {
            let sc = grid_score(&onset, hop_s, first_from(accent + phase, beat), beat, until);
            if sc > best.1 {
                best = (phase, sc);
            }
            phase += hop_s / 2.0;
        }
        let phase = best.0;
        // downbeat: which of the four beat positions carries the bar
        let bar_scores: Vec<f32> = (0..4).map(|d| grid_score(&onset, hop_s, first_from(accent + phase + d as f32 * beat, 4.0 * beat), 4.0 * beat, until)).collect();
        let (down, down_score) = bar_scores.iter().enumerate().fold((0, f32::MIN), |b, (i, &v)| if v > b.1 { (i, v) } else { b });
        let mean_bar = bar_scores.iter().sum::<f32>() / 4.0;
        let down_conf = if mean_bar > 0.0 { down_score / mean_bar } else { 1.0 };
        let mut verdict = Vec::new();
        if phase.abs() > 0.03 {
            verdict.push(format!("grid {} by {:.0} ms: accent_s → {:.2}", if phase > 0.0 { "late" } else { "early" }, phase.abs() * 1000.0, accent + phase));
        }
        if down != 0 && down_conf > 1.15 {
            verdict.push(format!("downbeat is beat {down} after the accent: accent_s → {:.2}", accent + phase + down as f32 * beat));
        }
        let verdict = if verdict.is_empty() { "on the beat".to_string() } else { verdict.join("; ") };
        println!("| {name} | {:.1} | {:.1} | {:+.0} ms | beat {down} ({down_conf:.2}×) | {verdict} |", s.bpm, s.accent_s, phase * 1000.0);

        // the click track: every beat, louder on the bar, longest on the accent
        let len = ((until + 1.0) * SR as f32) as usize;
        let mut click = vec![0f32; len];
        let mut t = first_from(accent + phase, beat);
        let mut k = ((t - (accent + phase)) / beat).round() as i64;
        while t < until {
            let (freq, dur, amp) = if k == 0 { (2000.0, 0.12, 0.9) } else if k.rem_euclid(4) == 0 { (1500.0, 0.05, 0.8) } else { (1000.0, 0.03, 0.55) };
            let start = (t * SR as f32) as usize;
            let n_s = (dur * SR as f32) as usize;
            for i in 0..n_s {
                if let Some(v) = click.get_mut(start + i) {
                    let env = (-(i as f32) / (n_s as f32) * 5.0).exp();
                    *v += amp * env * (2.0 * std::f32::consts::PI * freq * i as f32 / SR as f32).sin();
                }
            }
            t += beat;
            k += 1;
        }
        let stem = f.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let wav = Path::new("music-check").join(format!("{stem}.click.wav"));
        write_wav(&wav, &click)?;
        let out = Path::new("music-check").join(format!("{stem}.m4a"));
        let st = std::process::Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-i"])
            .arg(&f)
            .arg("-i")
            .arg(&wav)
            .args(["-filter_complex", "[0:a]volume=0.4[m];[m][1:a]amix=inputs=2:duration=first:normalize=0[a]", "-map", "[a]", "-c:a", "aac", "-b:a", "160k"])
            .arg(&out)
            .status()?;
        if !st.success() {
            bail!("ffmpeg failed rendering {}", out.display());
        }
        eprintln!("→ {}", out.display());
    }
    Ok(())
}

/// 16-bit mono WAV at SR.
fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let n = (samples.len() * 2) as u32;
    let mut d = Vec::with_capacity(44 + n as usize);
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + n).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes());
    d.extend_from_slice(&(SR as u32).to_le_bytes());
    d.extend_from_slice(&((SR * 2) as u32).to_le_bytes());
    d.extend_from_slice(&2u16.to_le_bytes());
    d.extend_from_slice(&16u16.to_le_bytes());
    d.extend_from_slice(b"data");
    d.extend_from_slice(&n.to_le_bytes());
    for s in samples {
        d.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, d)?;
    Ok(())
}
