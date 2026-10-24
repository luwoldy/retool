#!/usr/bin/env python3
"""Search Uppbeat's catalogue by BPM and turn the passes into MUSIC.md rows.

The site's own filters have no tempo control, but the Typesense index behind
its search does (`tempo`, `vocal`, `version_length`, `energy`, `mood`, `genre`).
`search` asks that index the same way the site's front end does, with the
front end's public search-only key, writes one row per track to
`<out>/index.tsv`, and fetches the full-length streaming preview MP3
alongside. `rows` runs `retool music stamp` over the folder and prints the
PASS tracks as MUSIC.md table rows, licence column filled, with the
catalogue's tempo as a check on the stamper's bpm:

    python3 tools/uppbeat.py search      # defaults below -> uppbeat/
    python3 tools/uppbeat.py rows        # stamp + join -> rows to paste
    python3 tools/uppbeat.py sheet       # stamp + join -> <out>/sheet.html to audition

The preview is for screening only: the file that enters ROTATION is
downloaded from the track page (`url` column) while logged in, so the
licence is on the account; re-stamp it to confirm the seconds carry over.

Every track appears once per category page in the index, so the search
runs per category slug and dedupes by version.
"""
import argparse
import csv
import datetime
import html
import json
import os
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

SEARCH = "https://3feynu8vjgbqkl27p.a1.typesense.net/multi_search"
KEY = os.environ.get("UPPBEAT_SEARCH_KEY", "")  # uppbeat.io front-end search key, from the site's network tab
COLLECTION = "tracksWithEngagement"
FIELDS = ("name,contributor_name,contributor_slug,track_slug,track_id,"
          "track_version_id,tempo,version_length,energy,genre,mood,theme,"
          "is_premium,version_preview_uri")

SLUGS = ("chillhop,lo-fi-beats,hip-hop-beats,ambient-beats,lazin-beats,"
         "late-night-beats,lush-beats,stylish-beats,sunny-beats,jazzy-beats,"
         "easygoing,good-vibes,chill,beats,vintage-beats")
GENRES = "Chillhop,Lofi,Beats,Hip Hop"
ENERGY = "Calm,Moderate"
NO_MOOD = ("Dark,Sad,Scary,Intense,Dramatic,Epic,Funny,Sneaky,Angry,Cute,"
           "Suspense,Mysterious,Emotional")
RETOOL = Path(__file__).resolve().parent.parent / "target/release/retool"


def ts_list(csv_values):
    """Typesense `field:=[a,b]` list; values with spaces go in backticks."""
    vals = [v.strip() for v in csv_values.split(",") if v.strip()]
    return "[" + ",".join(f"`{v}`" if " " in v else v for v in vals) + "]"


def search_slug(slug, args):
    parts = [
        f"slug:=/music/category/{slug}",
        f"tempo:[{args.bpm[0]}..{args.bpm[1]}]",
        f"version_length:[{args.len[0]}..{args.len[1]}]",
        "vocal:=Instrumental",
        "is_alternative:=false",
    ]
    if args.genre:
        parts.append(f"genre:={ts_list(args.genre)}")
    if args.energy:
        parts.append(f"energy:={ts_list(args.energy)}")
    if args.premium_only:
        parts.append("is_premium:=true")
    query = {
        "q": "*", "query_by": "name", "collection": COLLECTION,
        "per_page": 250, "include_fields": FIELDS,
        "filter_by": " && ".join(parts),
        "sort_by": "premium_sort_score_v1:desc", "exhaustive_search": True,
    }
    if not KEY:
        sys.exit("set UPPBEAT_SEARCH_KEY (uppbeat.io's front-end Typesense key)")
    page = 1
    while True:
        body = json.dumps({"searches": [dict(query, page=page)]}).encode()
        req = urllib.request.Request(
            f"{SEARCH}?x-typesense-api-key={KEY}", data=body,
            headers={"content-type": "application/json"})
        with urllib.request.urlopen(req, timeout=30) as r:
            res = json.load(r)["results"][0]
        if "error" in res:
            sys.exit(f"search {slug}: {res['error']}")
        for hit in res["hits"]:
            yield hit["document"]
        if len(res["hits"]) < 250:
            return
        page += 1


def filename(doc):
    safe = lambda s: re.sub(r"[^A-Za-z0-9._-]+", "_", s)
    return f"{safe(doc['track_slug'])}--{safe(doc['contributor_slug'])}.mp3"


def search(args):
    no_mood = {m.strip() for m in args.no_mood.split(",") if m.strip()}
    tracks = {}
    for slug in args.slug.split(","):
        n = 0
        for doc in search_slug(slug.strip(), args):
            n += 1
            if doc["track_version_id"] not in tracks and not (set(doc["mood"]) & no_mood):
                tracks[doc["track_version_id"]] = doc
        print(f"{slug:>18}: {n:4} matches, {len(tracks)} unique so far", file=sys.stderr)

    args.out.mkdir(exist_ok=True)
    rows = sorted(tracks.values(), key=lambda d: (d["tempo"], d["name"]))
    cols = ["file", "track_id", "name", "artist", "tempo", "length_s", "energy",
            "genre", "mood", "premium", "url", "preview"]
    with open(args.out / "index.tsv", "w", encoding="utf-8") as f:
        f.write("\t".join(cols) + "\n")
        for d in rows:
            f.write("\t".join(str(v) for v in [
                filename(d), d["track_slug"], d["name"], d["contributor_name"],
                d["tempo"], d["version_length"], d["energy"],
                ",".join(d["genre"]), ",".join(d["mood"]),
                "premium" if d["is_premium"] else "free",
                f"https://uppbeat.io/music/tracks/{d['contributor_slug']}/{d['track_slug']}",
                d["version_preview_uri"],
            ]) + "\n")
    print(f"{len(rows)} tracks -> {args.out}/index.tsv", file=sys.stderr)
    if args.dry_run:
        return
    for i, d in enumerate(rows, 1):
        dest = args.out / filename(d)
        if dest.exists():
            continue
        print(f"[{i}/{len(rows)}] {d['name']} - {d['contributor_name']}", file=sys.stderr)
        urllib.request.urlretrieve(d["version_preview_uri"], dest)


def stamped(args):
    """Stamp the folder; yield (index row, stamp cells) per file in the index."""
    with open(args.out / "index.tsv", encoding="utf-8") as f:
        index = {r["file"]: r for r in csv.DictReader(f, delimiter="\t")}
    stamp = subprocess.run([RETOOL, "music", "stamp", args.out], check=True,
                           capture_output=True, text=True).stdout
    for line in stamp.splitlines()[2:]:
        file, length, bpm, accent, phrases, ending, end, verdict = \
            [c.strip() for c in line.strip().strip("|").split("|")]
        meta = index.get(file)
        if meta is None:
            continue
        bpm = float(bpm.split()[0])
        tempo = float(meta["tempo"])
        flag = "" if abs(bpm - tempo) < 1.5 else f" (stamp {bpm:g} vs catalogue {tempo:g})"
        yield meta, dict(length=length, accent=accent, phrases=phrases, ending=ending,
                         end=end, verdict=verdict, flag=flag)


def rows(args):
    """Print PASS tracks as MUSIC.md rows, then the rest."""
    today = datetime.date.today().isoformat()
    passed, failed = [], []
    for meta, s in stamped(args):
        if s["verdict"].startswith("PASS"):
            licence = (f"Uppbeat {meta['premium']} — {meta['artist']}; {meta['url']}"
                       f"; stamped on the preview, re-stamp the download")
            passed.append(
                f"| {meta['track_id']} | {meta['file']} | {float(meta['tempo']):g}{s['flag']} |  | "
                f"{s['accent']} |  | {s['phrases']} | {s['ending']} | {s['end']} | {licence} | {today} |")
        else:
            failed.append(f"{meta['name']} - {meta['artist']}: {s['verdict']}{s['flag']}")
    print("\n".join(passed))
    print(f"\n{len(passed)} pass, {len(failed)} fail:", file=sys.stderr)
    for f in failed:
        print("  " + f, file=sys.stderr)


def sheet(args):
    """Write <out>/sheet.html: every preview with a player, its stamp and the track link."""
    recs = sorted(stamped(args), key=lambda r: (not r[1]["verdict"].startswith("PASS"),
                                                float(r[0]["tempo"]), r[0]["name"]))
    e = html.escape
    lines = []
    for meta, s in recs:
        ok = s["verdict"].startswith("PASS")
        lines.append(
            f'<tr class="{"pass" if ok else "fail"}"><td><audio controls preload="none" '
            f'src="{e(meta["file"])}"></audio></td>'
            f'<td><a href="{e(meta["url"])}" target="_blank">{e(meta["name"])}</a><br><small>{e(meta["artist"])} '
            f'· {meta["premium"]}</small></td><td>{float(meta["tempo"]):g}{e(s["flag"])}</td>'
            f'<td>{s["length"]}</td><td>{s["accent"]}</td><td>{s["phrases"]}</td>'
            f'<td>{s["ending"]}</td><td>{e(s["verdict"])}</td></tr>')
    npass = sum(1 for _, s in recs if s["verdict"].startswith("PASS"))
    page = f"""<!doctype html><meta charset="utf-8"><title>Uppbeat previews</title>
<style>
body{{font:14px/1.4 -apple-system,sans-serif;margin:1.5em}} table{{border-collapse:collapse}}
th,td{{padding:.35em .6em;border-bottom:1px solid #ddd;text-align:left;vertical-align:top}}
th{{position:sticky;top:0;background:#fff}} audio{{width:260px;height:32px}}
tr.fail{{color:#888}} tr.fail a{{color:#888}} small{{color:#666}} tr.fail small{{color:#aaa}}
</style>
<h1>Uppbeat previews</h1>
<p>{len(recs)} tracks, {npass} pass the screen (passes first, by tempo). Name links to the track
page; download from there while logged in. Stamped on the preview: re-stamp the download.</p>
<table><tr><th>preview</th><th>track</th><th>bpm</th><th>length</th><th>accent s</th>
<th>phrases</th><th>ending s</th><th>verdict</th></tr>
{chr(10).join(lines)}
</table>
"""
    (args.out / "sheet.html").write_text(page, encoding="utf-8")
    print(f"{len(recs)} tracks, {npass} pass -> {args.out}/sheet.html", file=sys.stderr)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, default=Path("uppbeat"), help="folder (default uppbeat)")
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("search", help="query the catalogue, fetch previews")
    span = lambda v: tuple(float(x) for x in v.split("-"))
    s.add_argument("--bpm", type=span, default=(92, 112), help="tempo range (default 92-112)")
    s.add_argument("--len", type=span, default=(100, 240), help="length in seconds (default 100-240)")
    s.add_argument("--slug", default=SLUGS, help="category slugs to search, comma-separated")
    s.add_argument("--genre", default=GENRES, help="genre list (server-side); '' for any")
    s.add_argument("--energy", default=ENERGY, help="energy list (server-side); '' for any")
    s.add_argument("--no-mood", default=NO_MOOD, help="drop tracks tagged with any of these moods")
    s.add_argument("--premium-only", action="store_true")
    s.add_argument("--dry-run", action="store_true", help="write index.tsv, fetch nothing")
    s.set_defaults(run=search)
    r = sub.add_parser("rows", help="stamp the folder, print MUSIC.md rows for the passes")
    r.set_defaults(run=rows)
    h = sub.add_parser("sheet", help="stamp the folder, write sheet.html with players and links")
    h.set_defaults(run=sheet)
    args = ap.parse_args()
    args.run(args)


if __name__ == "__main__":
    main()
