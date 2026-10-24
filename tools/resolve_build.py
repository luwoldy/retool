#!/usr/bin/env python3
"""Build the job's DaVinci Resolve project from its cutlists (spike, 2026-09-05).

    tools/resolve_build.py <job_dir> <out_dir> [--replace] [--render none|check|all] [--template <file.drp>] [--unapproved]

Reads out/video/cutlist.md and reel_01..03.md (retool plan), clips.tsv, job.md.
Writes out/video/resolve/: the music beds (spliced + loudness-normalised WAVs),
the cards (ProRes 4444 with alpha), build.log, and the project's .drp.
Then, over the scripting API with Resolve running: one project named after the
job folder, colour-managed (no LUTs), five timelines on the grid, render jobs
queued. Resolve's own value strings are read back and logged, never assumed.

--template imports a .drp (its timelines and media stripped) instead of creating
a project from Resolve's defaults: the one route to settings the API refuses,
such as the playback frame rate, which follows the operator's UI choice once.
"""

import json
import re
import subprocess
import sys
import time
from pathlib import Path

RESOLVE_MODULES = "/Library/Application Support/Blackmagic Design/DaVinci Resolve/Developer/Scripting/Modules"
ROTATION = Path.home() / "Documents/ShowReady/Music/ROTATION"
FPS = 30.0
# one stop in DaVinci Intermediate code values (the node's working space under RCM)
DI_STOP = 0.07329
GIMBAL_KINDS = {"walkup", "entry", "hallway", "lookback", "grounds", "close"}
ROLE_COLOR = {"establish": "Navy", "tease": "Yellow", "wide": "Green", "pan": "Teal", "detail": "Yellow", "angle": "Lime", "move": "Purple", "static": "Olive", "close": "Orange"}
# clip markers take marker colours, not clip colours
ROLE_MARKER = {"establish": "Blue", "tease": "Yellow", "wide": "Mint", "pan": "Cyan", "detail": "Yellow", "angle": "Lemon", "move": "Purple", "static": "Sand", "close": "Red"}
# the ruler, as SR1 stamped it: every beat, bars promoted, phrase downbeats promoted and named
BEAT_MARKER, BAR_MARKER, PHRASE_MARKER = "Cream", "Sky", "Green"
# candidates, first that Resolve accepts wins; the accepted one is logged
FUJI_CS = ["Fujifilm F-Log2", "F-Gamut/F-Log2", "Fujifilm F-Log2 / F-Gamut", "FujiFilm F-Log2"]
APPLE_CS = ["Apple Log", "Rec.2020/Apple Log", "Apple Log Rec.2020"]
OUTPUT_DRT = ["Luminance Mapping", "DaVinci", "Saturation Preserving"]

LOG = []


def log(msg):
    print(msg)
    LOG.append(msg)


# ---- the job's files -------------------------------------------------------

def read_cutlist(path):
    header, rows, skipped, in_skipped = {}, [], [], False
    for line in path.read_text().splitlines():
        if line.startswith("## skipped"):
            in_skipped = True
            continue
        if line.startswith("|"):
            cells = [c.strip() for c in line.split("|")]
            if len(cells) < 4 or cells[1] in ("#", "clip") or cells[1].startswith("---"):
                continue
            if in_skipped:
                skipped.append(cells[1:4])
            elif len(cells) >= 11:
                rows.append({"n": int(cells[1]), "at": float(cells[2]), "role": cells[3], "clip": cells[4], "in": float(cells[5]), "out": float(cells[6]), "gain": float(cells[8]), "room": cells[9], "why": cells[10]})
            continue
        m = re.match(r"^([A-Za-z_]+): (.*)$", line)
        if m and m.group(1) not in header:
            header[m.group(1)] = m.group(2)
    return header, rows, skipped


def read_job(job_dir):
    text = (job_dir / "job.md").read_text()
    title = next(l for l in text.splitlines() if l.startswith("# "))[2:].strip()
    address, _, rest = title.partition(" — ")
    agent, _, brokerage = rest.partition(", ")
    phone = next((l[len("Phone:"):].strip() for l in text.splitlines() if l.startswith("Phone:")), "")
    return {"address": address.strip(), "agent": agent.strip(), "brokerage": brokerage.strip(), "phone": phone}


def read_clips_tsv(out_dir):
    clips = {}
    lines = (out_dir / "video" / "clips.tsv").read_text().splitlines()
    cols = lines[0].split("\t")
    for l in lines[1:]:
        c = dict(zip(cols, l.split("\t")))
        clips[c["file"]] = c
    return clips


def locate(job_dir, clip):
    for sub in ("video", "phone"):
        p = job_dir / sub / clip
        if p.exists():
            return p.resolve()
    raise FileNotFoundError(f"{clip} is in neither {job_dir}/video nor {job_dir}/phone")


def track_path(name):
    p = Path(name)
    return p if p.exists() else ROTATION / name


# ---- ffmpeg: beds and cards --------------------------------------------------

def run(args):
    r = subprocess.run(args, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"{args[0]} failed: {r.stderr.strip()[-800:]}\n  args: {' '.join(map(str, args))}")
    return r


def bed(track, out, master=None, reel=None):
    """The music under one cut, loudness-normalised in two passes, as a 48 kHz WAV."""
    if master:
        start_s, splice, es, e = master
        total = splice + (e - es)
        graph = lambda measured, tail: (f"[0:a]asplit[m1][m2];[m1]atrim=start={start_s:.3f}:end={start_s + splice + 0.1:.3f},asetpts=PTS-STARTPTS[a1];"
                                        f"[m2]atrim=start={es:.3f}:end={e:.3f},asetpts=PTS-STARTPTS[a2];[a1][a2]acrossfade=d=0.1:c1=tri:c2=tri[am];"
                                        f"[am]loudnorm=I=-14:TP=-1.5:LRA=11{measured},aresample=48000{tail}[a]")
    else:
        start_s, length = reel
        total = length
        graph = lambda measured, tail: (f"[0:a]atrim=start={start_s:.3f}:end={start_s + length:.3f},asetpts=PTS-STARTPTS,afade=t=out:st={length - 0.1:.3f}:d=0.1[am];"
                                        f"[am]loudnorm=I=-14:TP=-1.5:LRA=11{measured},aresample=48000{tail}[a]")
    r = subprocess.run(["ffmpeg", "-y", "-loglevel", "info", "-i", str(track), "-filter_complex", graph(":print_format=json", ""), "-map", "[a]", "-f", "null", "-"], capture_output=True, text=True)
    j = r.stderr[r.stderr.rfind("{"):]
    m = json.loads(j[: j.find("}") + 1])
    measured = f":measured_I={m['input_i']}:measured_TP={m['input_tp']}:measured_LRA={m['input_lra']}:measured_thresh={m['input_thresh']}:offset={m['target_offset']}:linear=true"
    run(["ffmpeg", "-y", "-loglevel", "error", "-i", str(track), "-filter_complex", graph(measured, f",apad=whole_dur={total:.3f}"), "-map", "[a]", "-t", f"{total:.3f}", "-c:a", "pcm_s24le", str(out)])
    return total


def card_filters(lines, dirpath, tag, alpha, height=1080):
    """drawtext chain for a card: lines lower-left, as retool cut draws them."""
    y = height - 80
    out = []
    for i, (text, size) in reversed(list(enumerate(lines))):
        if not text:
            continue
        f = dirpath / f"card_{tag}_{i}.txt"
        f.write_text(text)
        y -= size + 16
        out.append(f"drawtext=font=Helvetica:textfile={f}:fontsize={size}:fontcolor=white:x=96:y={y}:shadowcolor=black@0.7:shadowx=2:shadowy=2:box=1:boxcolor=black@0.35:boxborderw=14:alpha='{alpha}'")
    out.reverse()
    return ",".join(out)


def card(out, dur, filters, fade_from_black=False):
    """A ProRes 4444 clip with alpha: transparent except the text (and the open's fade from black)."""
    base = "black" if fade_from_black else "black@0.0"
    vf = ("fade=t=out:st=0:d=0.4:alpha=1," if fade_from_black else "") + filters + ",format=yuva444p10le"
    run(["ffmpeg", "-y", "-loglevel", "error", "-f", "lavfi", "-i", f"color=c={base}:s=1920x1080:r=30:d={dur:.3f},format=rgba", "-vf", vf, "-c:v", "prores_ks", "-profile:v", "4444", "-pix_fmt", "yuva444p10le", str(out)])


def address_lines(job):
    street, _, city = job["address"].rpartition(", ")
    return [(street or job["address"], 64), (city if street else "", 40)]


# ---- Resolve -----------------------------------------------------------------

def connect():
    sys.path.append(RESOLVE_MODULES)
    import DaVinciResolveScript as dvr
    r = dvr.scriptapp("Resolve")
    if not r:
        raise SystemExit("Resolve is not running, or external scripting is off (Preferences > System > General)")
    return r


def set_first(setter, getter, key, candidates, what):
    """Try candidate values until Resolve accepts one and reads it back; log the outcome."""
    for v in candidates:
        ok = setter(key, v)
        back = getter(key)
        if ok and str(back) == str(v):
            log(f"  {what}: {key} = {v!r}")
            return v
        log(f"  {what}: {key} = {v!r} refused (ok={ok}, reads back {back!r})")
    return None


def main():
    args = sys.argv[1:]
    replace = "--replace" in args
    render = args[args.index("--render") + 1] if "--render" in args else "none"
    template = Path(args[args.index("--template") + 1]).resolve() if "--template" in args else None
    pos = [a for a in args if not a.startswith("--") and a not in ("none", "check", "all") and (template is None or a != str(args[args.index("--template") + 1]))]
    job_dir, out_dir = Path(pos[0]).resolve(), Path(pos[1]).resolve()
    video_dir = out_dir / "video"
    rdir = video_dir / "resolve"
    rdir.mkdir(parents=True, exist_ok=True)
    job = read_job(job_dir)
    clips_tsv = read_clips_tsv(out_dir)
    master_h, master_rows, master_skipped = read_cutlist(video_dir / "cutlist.md")
    # the gate (PLAN-review.md V3): nothing is built from a plan nobody signed off
    if not master_h.get("approved", "").strip() and "--unapproved" not in args:
        raise SystemExit(f"{video_dir / 'cutlist.md'} is not approved: `retool review {job_dir} {out_dir}` to review and approve, or --unapproved to build anyway")
    reels = []
    for k in (1, 2, 3):
        rp = video_dir / f"reel_{k:02}.md"
        if rp.exists():
            h, rows, _ = read_cutlist(rp)
            if rows:
                reels.append((k, h, rows))
    track = track_path(master_h["music"])
    if not track.exists():
        raise SystemExit(f"music {track} not found")
    shelf, accent, splice = float(master_h["shelf"]), float(master_h["accent_s"]), float(master_h["splice"])
    es, e_s, end = float(master_h["ending_start_s"]), float(master_h["end_s"]), float(master_h["end"])
    bar = 240.0 / float(master_h["bpm"])
    t0 = time.time()
    log(f"# resolve build {time.strftime('%Y-%m-%d %H:%M')} — {job['address']} — {track.name}")

    # -- beds and cards (ffmpeg) --
    beds = {"master": rdir / "bed_master.wav"}
    total = bed(track, beds["master"], master=(accent - shelf, splice, es, e_s))
    log(f"bed_master.wav: {total:.3f} s (cutlist end {end:.3f})")
    for k, h, rows in reels:
        beds[k] = rdir / f"bed_reel_{k:02}.wav"
        total = bed(track_path(h["music"]), beds[k], reel=(float(h["accent_s"]), float(h["end"])))
        log(f"bed_reel_{k:02}.wav: {total:.3f} s from {h['music']} at {h['accent_s']} s")
    close = next((r for r in master_rows if r["role"] == "close"), None)
    close_at = close["at"] if close else end
    cards = {"open": rdir / "card_open.mov", "end_b": rdir / "card_end_b.mov", "end_u": rdir / "card_end_u.mov"}
    est_at = next((r["at"] for r in master_rows if r["role"] == "establish"), 0.0)
    if shelf > 0:
        card(cards["open"], shelf, card_filters(address_lines(job), rdir, "open", f"clip((t-{est_at + 1:.3f})/0.5,0,1)*clip(({shelf:.3f}-0.5-t)/0.5,0,1)"), fade_from_black=True)
    else:
        # no shelf: the picture starts on the accent, so there is nothing for the address card to ride
        del cards["open"]
    card(cards["end_b"], end - close_at, card_filters([(job["agent"], 56), (job["brokerage"], 40), (job["phone"], 40)], rdir, "end_b", "clip((t-0.5)/0.5,0,1)"))
    card(cards["end_u"], end - close_at, card_filters(address_lines(job), rdir, "end_u", "clip((t-0.5)/0.5,0,1)"))
    log(f"cards: open {shelf:.1f} s{' (skipped: no shelf)' if 'open' not in cards else ''}, end {end - close_at:.1f} s ({time.time() - t0:.0f} s so far)")

    # -- the project --
    resolve = connect()
    pm = resolve.GetProjectManager()
    cur = pm.GetCurrentProject()
    name = job_dir.name
    if cur and cur.GetName() != name:
        log(f"saving and leaving the open project {cur.GetName()!r}: {pm.SaveProject()}")
    pm.GotoRootFolder()
    if "re" not in pm.GetFolderListInCurrentFolder():
        pm.CreateFolder("re")
    pm.OpenFolder("re")
    if name in pm.GetProjectListInCurrentFolder():
        if not replace:
            raise SystemExit(f"project {name!r} exists in the re folder; --replace to rebuild it")
        if cur and cur.GetName() == name:
            pm.CloseProject(cur)
        log(f"deleting project {name!r}: {pm.DeleteProject(name)}")
    if template:
        if not pm.ImportProject(str(template), name):
            raise SystemExit(f"could not import {template} as {name!r}")
        p = pm.LoadProject(name)
        mp = p.GetMediaPool()
        old_tls = [p.GetTimelineByIndex(i) for i in range(1, p.GetTimelineCount() + 1)]
        if old_tls:
            mp.DeleteTimelines(old_tls)
        root = mp.GetRootFolder()
        if root.GetClipList():
            mp.DeleteClips(root.GetClipList())
        if root.GetSubFolderList():
            mp.DeleteFolders(root.GetSubFolderList())
        log(f"project {name!r} imported from {template.name} in folder 're', stripped; playback rate reads {p.GetSetting('timelinePlaybackFrameRate')!r}")
    else:
        p = pm.CreateProject(name)
        if not p:
            raise SystemExit(f"could not create project {name!r}")
        log(f"project {name!r} created in folder 're'")

    # project settings: frame rate first (it locks once media is in), then colour management
    # the playback rate follows the video monitoring format and is read-only itself
    for key, val in [("timelineFrameRate", "30"), ("videoMonitorFormat", "HD 1080p 30"), ("timelineResolutionWidth", "1920"), ("timelineResolutionHeight", "1080")]:
        set_first(p.SetSetting, p.GetSetting, key, [val], "project")
    log(f"  project: timelinePlaybackFrameRate reads {p.GetSetting('timelinePlaybackFrameRate')!r} (follows videoMonitorFormat)")
    log("colour management (frame.io cheat sheet: managed, custom, DWG/Intermediate, Rec.709 2.4, input DRT none):")
    set_first(p.SetSetting, p.GetSetting, "colorScienceMode", ["davinciYRGBColorManagedv2"], "project")
    set_first(p.SetSetting, p.GetSetting, "isAutoColorManage", ["0"], "project")
    set_first(p.SetSetting, p.GetSetting, "rcmPresetMode", ["Custom"], "project")
    set_first(p.SetSetting, p.GetSetting, "separateColorSpaceAndGamma", ["0"], "project")
    set_first(p.SetSetting, p.GetSetting, "colorSpaceTimeline", ["DaVinci WG/Intermediate"], "project")
    set_first(p.SetSetting, p.GetSetting, "colorSpaceOutput", ["Rec.709 Gamma 2.4"], "project")
    fuji_cs = set_first(p.SetSetting, p.GetSetting, "colorSpaceInput", FUJI_CS, "project")
    set_first(p.SetSetting, p.GetSetting, "inputDRT", ["None"], "project")
    set_first(p.SetSetting, p.GetSetting, "outputDRT", OUTPUT_DRT, "project")
    set_first(p.SetSetting, p.GetSetting, "useCATransform", ["1"], "project")
    snap = p.GetSetting()
    log("  colour keys now: " + ", ".join(f"{k}={snap[k]!r}" for k in sorted(snap) if re.search(r"colorScience|colorSpace(Input|Timeline|Output)$|DRT$|Luminance|rcm|CATransform|AutoColor", k)))

    # media
    mp = p.GetMediaPool()
    root = mp.GetRootFolder()
    bins = {b: mp.AddSubFolder(root, b) for b in ("camera", "phone", "music", "cards")}
    used = sorted({r["clip"] for r in master_rows} | {r["clip"] for _, _, rows in reels for r in rows})
    # the whole card goes in, used or not, so a swap in QC is a drag from the same bin
    items = {}
    for src in ("fuji", "phone"):
        files = [c for c in sorted(set(clips_tsv) | set(used)) if clips_tsv.get(c, {}).get("source", "fuji") == src]
        mp.SetCurrentFolder(bins["camera" if src == "fuji" else "phone"])
        for it in mp.ImportMedia([str(locate(job_dir, c)) for c in files]) or []:
            fname = it.GetClipProperty("File Name")
            items[fname] = it
            c = clips_tsv.get(fname, {})
            it.SetMetadata({"Comments": " · ".join(x for x in (c.get("station", ""), c.get("kind", ""), c.get("destination", ""), c.get("segments", ""), "" if fname in used else "not in the cut") if x)})
            it.SetClipColor("Tan" if fname not in used else "Green")
    mp.SetCurrentFolder(bins["music"])
    for it in mp.ImportMedia([str(b) for b in beds.values()]) or []:
        items[it.GetClipProperty("File Name")] = it
    mp.SetCurrentFolder(bins["cards"])
    for it in mp.ImportMedia([str(c) for c in cards.values()]) or []:
        items[it.GetClipProperty("File Name")] = it
    missing = [c for c in used if c not in items]
    log(f"media: {len(items)} items imported" + (f", MISSING {missing}" if missing else ""))

    # clip attributes: conform 2× (60p played as 30p), input colour space per source
    apple_cs = None
    for c in sorted(items):
        if c not in clips_tsv and c not in used:
            continue
        it = items.get(c)
        if not it:
            continue
        src = clips_tsv.get(c, {}).get("source", "fuji")
        fps = float(it.GetClipProperty("FPS"))
        half = f"{fps / 2:.2f}".rstrip("0").rstrip(".")
        ok = it.SetClipProperty("FPS", half)
        back = it.GetClipProperty("FPS")
        log(f"  {c}: FPS {fps:g} -> {half} (ok={ok}, reads back {back}); frames {it.GetClipProperty('Frames')}, {it.GetClipProperty('Resolution')}, data level {it.GetClipProperty('Data Level')}, ICS was {it.GetClipProperty('Input Color Space')!r}")
        if src == "phone":
            apple_cs = set_first(it.SetClipProperty, it.GetClipProperty, "Input Color Space", [apple_cs] if apple_cs else APPLE_CS, c)
        elif fuji_cs:
            it.SetClipProperty("Input Color Space", fuji_cs)

    def lay(tl, rows, end_s, wide, offset0=0):
        """Rows onto V1 of tl, cut-to-cut on the grid; returns [(row, item)]."""
        start = tl.GetStartFrame()
        infos, meta = [], []
        for i, r in enumerate(rows):
            it = items.get(r["clip"])
            if not it:
                continue
            next_at = rows[i + 1]["at"] if i + 1 < len(rows) else end_s
            slot = max(1, round(next_at * FPS) - round(r["at"] * FPS))
            src_fps = float(it.GetClipProperty("FPS")) * 2  # the file's own rate, before the conform
            sf = round(r["in"] * src_fps)
            frames = int(it.GetClipProperty("Frames") or 0)
            ef = sf + slot
            if frames and ef > frames:
                log(f"  row {r['n']} {r['clip']}: slot runs {ef - frames} frames past the media, clipped")
                ef = frames
            infos.append({"mediaPoolItem": it, "mediaType": 1, "trackIndex": 1, "startFrame": sf, "endFrame": ef, "recordFrame": start + round(r["at"] * FPS)})
            meta.append((r, sf, slot))
        placed = mp.AppendToTimeline(infos) or []
        out = []
        for (r, sf, slot), ti in zip(meta, placed):
            out.append((r, ti))
            got = (ti.GetStart() - start, ti.GetDuration(), ti.GetSourceStartFrame())
            want = (round(r["at"] * FPS), slot, sf)
            if got != want:
                log(f"  row {r['n']} {r['clip']}: placed (start, dur, src) {got} wanted {want}")
            src = clips_tsv.get(r["clip"], {})
            portrait = src.get("orientation") == "portrait"
            if portrait == wide:  # a vertical on the master, a landscape on a reel: fill = centre crop
                ti.SetProperty("Scaling", 3)
            if src.get("source", "fuji") == "fuji" and abs(r["gain"]) >= 0.05:
                o = f"{r['gain'] * DI_STOP:.4f}"
                ti.SetCDL({"NodeIndex": "1", "Slope": "1 1 1", "Offset": f"{o} {o} {o}", "Power": "1 1 1", "Saturation": "1"})
            ti.SetClipColor(ROLE_COLOR.get(r["role"], "Tan"))
            ti.SetName(f"{r['n']:02} {r['role']} {r['room']}")
            ti.AddMarker(0, ROLE_MARKER.get(r["role"], "Rose"), f"{r['role']} · {r['room']}", f"{r['why']} (gain {r['gain']:+.1f}, src {r['in']:.2f}–{r['out']:.2f} s)", 1, "")
        log(f"  {tl.GetName()}: {len(placed)} of {len(rows)} rows placed")
        return out

    def grid(tl, origin_s, end_s, patterns=""):
        """Every beat from the origin, Cream; bars Sky; phrase downbeats Green, named by the pattern."""
        labels = [x for x in patterns.split(" · ") if x]
        beat = bar / 4
        if labels and labels[0].startswith("S "):
            origin_s -= 16 * beat  # the cold open is a phrase before the accent; the ruler covers it
        n = 0
        while origin_s + n * beat < end_s + 1e-6:
            f = round((origin_s + n * beat) * FPS)
            if n % 16 == 0:
                tl.AddMarker(f, PHRASE_MARKER, labels[n // 16] if n // 16 < len(labels) else f"phrase {n // 16 + 1}", "", 1, "")
            elif n % 4 == 0:
                tl.AddMarker(f, BAR_MARKER, "bar", "", 1, "")
            else:
                tl.AddMarker(f, BEAT_MARKER, "b", "", 1, "")
            n += 1
        return n

    def music(tl, bed_item, length_s, at_s=0.0):
        idx = 1
        if tl.GetTrackSubType("audio", 1) != "stereo":
            tl.AddTrack("audio", "stereo")
            idx = tl.GetTrackCount("audio")
        ok = mp.AppendToTimeline([{"mediaPoolItem": bed_item, "mediaType": 2, "trackIndex": idx, "startFrame": 0, "endFrame": round(length_s * FPS), "recordFrame": tl.GetStartFrame() + round(at_s * FPS)}])
        return f"A{idx}" if ok else "MUSIC FAILED"

    def stabilise(pairs):
        for r, ti in pairs:
            if clips_tsv.get(r["clip"], {}).get("kind") in GIMBAL_KINDS:
                t = time.time()
                ok = ti.Stabilize()
                log(f"  stabilize row {r['n']} {r['clip']}: {ok} ({time.time() - t:.0f} s)")

    def render_job(tl, w, h, custom):
        p.SetCurrentTimeline(tl)
        p.LoadRenderPreset("H.264 Master")
        p.SetCurrentRenderFormatAndCodec("mp4", "H264")
        ok = p.SetRenderSettings({"SelectAllFrames": True, "TargetDir": str(rdir), "CustomName": custom, "FormatWidth": w, "FormatHeight": h, "FrameRate": 30.0, "ExportVideo": True, "ExportAudio": True, "AudioCodec": "aac", "NetworkOptimization": True, "VideoQuality": 16000})
        jid = p.AddRenderJob()
        log(f"  render job {custom}: {w}x{h} settings ok={ok} id={jid}")
        return jid

    # -- MASTER -- (timelines are created in the current bin: the root)
    mp.SetCurrentFolder(root)
    log("MASTER branded:")
    tl = mp.CreateEmptyTimeline("MASTER branded")
    p.SetCurrentTimeline(tl)
    tl.AddTrack("video")
    pairs = lay(tl, master_rows, end, wide=True)
    start = tl.GetStartFrame()
    v2 = [{"mediaPoolItem": items[cards["open"].name], "mediaType": 1, "trackIndex": 2, "startFrame": 0, "endFrame": round(shelf * FPS), "recordFrame": start}] if "open" in cards else []
    v2.append({"mediaPoolItem": items[cards["end_b"].name], "mediaType": 1, "trackIndex": 2, "startFrame": 0, "endFrame": round((end - close_at) * FPS), "recordFrame": start + round(close_at * FPS)})
    mp.AppendToTimeline(v2)
    log(f"  cards on V2: {len(tl.GetItemListInTrack('video', 2))}; music on {music(tl, items[beds['master'].name], end)}; grid markers {grid(tl, shelf, end, master_h.get('patterns', ''))} (accent at {shelf:.1f} s)")
    if master_h.get("stabilise", "").strip() == "on":
        stabilise(pairs)
    for r in master_skipped:
        tl.AddMarker(round(end * FPS) - 1, "Red", f"skipped {r[0]} {r[1]}", r[2], 1, "") or None
    jobs = {"mls_branded": render_job(tl, 1920, 1080, "mls_branded")}

    # -- MASTER unbranded: the same cut, the address card at the close --
    tu = tl.DuplicateTimeline("MASTER unbranded")
    p.SetCurrentTimeline(tu)
    for it in tu.GetItemListInTrack("video", 2):
        if it.GetName().startswith("card_end"):
            tu.DeleteClips([it])
    mp.AppendToTimeline([{"mediaPoolItem": items[cards["end_u"].name], "mediaType": 1, "trackIndex": 2, "startFrame": 0, "endFrame": round((end - close_at) * FPS), "recordFrame": tu.GetStartFrame() + round(close_at * FPS)}])
    log(f"MASTER unbranded: duplicated, V1 {len(tu.GetItemListInTrack('video', 1))} items, V2 {[i.GetName() for i in tu.GetItemListInTrack('video', 2)]}")
    jobs["mls_unbranded"] = render_job(tu, 1920, 1080, "mls_unbranded")

    # -- REELS: 1080x1920 timelines, the vertical pool first --
    for k, h, rows in reels:
        nm = f"REEL {k:02} {h.get('purpose') or h.get('lead', '')}"
        log(f"{nm}:")
        tr = mp.CreateEmptyTimeline(nm)
        p.SetCurrentTimeline(tr)
        set_first(tr.SetSetting, tr.GetSetting, "useCustomSettings", ["1"], "  timeline")
        set_first(tr.SetSetting, tr.GetSetting, "timelineResolutionWidth", ["1080"], "  timeline")
        set_first(tr.SetSetting, tr.GetSetting, "timelineResolutionHeight", ["1920"], "  timeline")
        # a custom-settings timeline carries its own colour management, unmanaged by default: copy the project's, in this order
        for key in ("colorScienceMode", "isAutoColorManage", "rcmPresetMode", "separateColorSpaceAndGamma", "colorSpaceTimeline", "colorSpaceOutput", "inputDRT", "outputDRT", "useCATransform"):
            set_first(tr.SetSetting, tr.GetSetting, key, [str(p.GetSetting(key))], "  timeline")
        reel_end = float(h["end"])
        lay(tr, rows, reel_end, wide=False)
        log(f"  music on {music(tr, items[beds[k].name], reel_end)}; grid markers {grid(tr, 0.0, reel_end, h.get('patterns', ''))}")
        jobs[f"reel_{k:02}"] = render_job(tr, 1080, 1920, f"reel_{k:02}")

    p.SetCurrentTimeline(tl)
    log(f"saved: {pm.SaveProject()}; exported {rdir / (name + '.drp')}: {pm.ExportProject(name, str(rdir / (name + '.drp')))}")
    log(f"build: {time.time() - t0:.0f} s")

    if render != "none":
        ids = [jobs["mls_branded"], jobs["reel_01"]] if render == "check" else list(jobs.values())
        t = time.time()
        log(f"rendering {ids}: {p.StartRendering(ids)}")
        while p.IsRenderingInProgress():
            time.sleep(2)
        for i in ids:
            log(f"  {i}: {p.GetRenderJobStatus(i)}")
        log(f"render: {time.time() - t:.0f} s")
    (rdir / "build.log").write_text("\n".join(LOG) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
