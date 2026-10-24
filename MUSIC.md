# Music — the folder, the grid, the licence record

The tool never chooses music by taste. It reads one folder, rotates
through it, and lays the picture on the track's grid. Which tracks live in
the folder, and what a track must have to qualify, is the operator's call,
recorded here.

## The folder

`~/Documents/ShowReady/Music/ROTATION` — the tracks the tool may use.
Today three, all Anno Domini Beats from the YouTube Audio Library, screened
2026-09-02. The tool sorts the folder by name and uses track `job_number
mod count`; a `music:` line in a job's `cutlist.md` overrides it. Eight to
ten tracks is the right size for a rotation: agents who book twice should
not hear the same track twice; three is enough to build and test the cut.

The seven Epidemic Sound tracks in `../CORE` are set aside for now (four
of them pass the screener; the licence question decides whether they
return). Candidates under screening live in the repo's `yl/` folder,
ignored by git.

## Music essentials (operator's guidelines, 2026-09-02)

The track must have:

1. **A grid-compatible, steady BPM, 90–115, ideally 95–108.** A 4/4 bar at
   ~100 BPM is 2.4 s, which is what makes the 2.4 / 4.8 cut grid land on
   downbeats. No rubato; pushes get strung at one speed.
2. **A natural accent hit.** The single most load-bearing feature: the
   accent is the grid origin everything else is computed from. *Amended
   2026-09-02:* it may sit anywhere from 8 to 25 s in, because the master
   starts the track at `accent_s − shelf` (shelf 12 s by default, the
   establishing open), so the ramp-up before the hit is always the same
   length on screen whatever the track does before it.
3. **Enough full arrangement after the accent.** The usable phrases after
   the accent cap how long the master can be; the track has to carry the
   whole room sequence in 16-beat phrases after the hit.

Every edit uses it the same way:

4. **The opening rides the pre-accent shelf.** The master starts the track
   12 s before the accent, with picture at that point, and the cut into the
   first interior lands exactly on the accent. Verticals trim the head so the accent is
   frame zero. (In the MLS-wide idiom the shelf carries the *establish*:
   walk-up or front wide plus the address card; the "hook" concept lives in
   the verticals, per `RESEARCH-shoot-craft.md` §3.5.)
5. **All cuts sit on the stamped grid**: beat, bar and phrase markers
   counted forward from the accent. Wides 4.8 s, details 2.4 s.
6. **Back-timed ending.** The track's last real bar lands on the close, the
   held pull-back. A loop-fade bed has no final bar to back-time to, so a
   **composed ending is a requirement**, not a preference.

## The stamp — what the tool reads per track

`retool music stamp <folder>` proposes every value below from the audio in
about a third of a second per track, with a verdict against the six
essentials, and prints rows in this table's shape. The operator confirms or
corrects by ear, then pastes the rows here. Seconds are on the original
file. Verdicts are honest, not gentle: on 2026-09-02 one of the seven CORE
tracks passed as-is; five hit their accent at 17–19 s rather than 10–15,
which is a rule to decide on (see "The shelf" below), not a tempo problem.

Screening a download folder: `retool music stamp yl`, keep the PASS rows,
copy those files into `ROTATION`, add the licence column here.

*2026-09-06:* Uppbeat (a year of Premium, bought 2026-09-06) is the
catalogue under screening. Its site filters have no tempo control, but the
search index behind them does, so `python3 tools/uppbeat.py search` asks
the index directly: instrumental main versions at 92–112 BPM, 100–240 s,
calm or moderate energy, in the beats / chillhop / lo-fi / hip-hop genres,
minus the dark, sad and comic moods; it fetches the full-length streaming
previews into `uppbeat/` (ignored by git). `python3 tools/uppbeat.py rows`
stamps that folder and prints the passes as rows for the table below,
licence column filled, the catalogue's tempo checked against the stamper's
bpm; `python3 tools/uppbeat.py sheet` writes `uppbeat/sheet.html`, every
preview with a player, its stamp and the track-page link, passes first.
First run: 121 candidates, 38 pass; the operator's ear picks from
those. The preview is the screening copy only: the file that enters
`ROTATION` is the download from the track page while logged in, re-stamped
to confirm the seconds carry over.

*2026-09-06:* the stamper's tempo fold now also tries the 3:2 and 4:3
ratios of the raw autocorrelation peak. Swung lo-fi hi-hats had it reading
92 as 61 and 100 as 67 (the dotted lag rings louder than the beat) on 43 of
the 121 Uppbeat previews; 11 disagree now, and the four rotation rows above
stamp exactly as before.

*2026-09-06:* 23 Uppbeat tracks picked from the sheet, downloaded as WAV on
the account, are the rotation; the four Epidemic CORE files left `ROTATION`
(their rows stay for the record). The rows below are stamped on the WAVs:
22 match the preview stamp exactly, Retro moves by 0.1 s (17 phrases, not
16). Volt stamps 98 BPM on both copies while the catalogue says 110; check
by ear before the planner uses it. `retool music check` click tracks for all
23 are in `music-check/`; none has been listened to yet, so the accent
corrections it proposes (up to 263 ms of grid phase, and a downbeat one to
three beats later on most) are still open.

| track_id | file | bpm | band | accent_s | shelf_end_s | phrases_after_accent | ending_start_s | end_s | source / licence | added |
|---|---|---|---|---|---|---|---|---|---|---|
| paradise | Paradise - Anno Domini Beats.mp3 | 95.0 |  | 11.4 |  | 15 | 165.4 | 174.6 | YouTube Audio Library — confirm "attribution not required" on the track page; ending declines ~4 dB with the beat going, listen before relying on it | 2026-09-02 |
| reasons | Reasons - Anno Domini Beats.mp3 | 96.0 |  | 8.1 |  | 12 | 130.6 | 140.6 | YouTube Audio Library — confirm attribution flag | 2026-09-02 |
| touch | Touch - Anno Domini Beats.mp3 | 110.0 |  | 8.2 |  | 12 | 158.8 | 167.5 | YouTube Audio Library — confirm attribution flag | 2026-09-02 |
| mirage | ES_A Mirage (Instrumental Version) - I'MIN.wav | 102.0 |  | 19.41 |  | 16 | 173.6 | 183.0 | Epidemic Sound (CORE); subscription to confirm | 2026-09-03 |
| bestfriend | ES_Best Friend (Instrumental Version) - Nbhd Nick.wav | 92.0 |  | 10.4 |  | 9 | 161.7 | 172.2 | Epidemic Sound (CORE); subscription to confirm | 2026-09-03 |
| everything | ES_Everything You Need (Instrumental Version) - waykap.wav | 114.0 |  | 16.8 |  | 7 | 170.5 | 178.9 | Epidemic Sound (CORE); subscription to confirm | 2026-09-03 |
| xp | es_xp.wav | 109.2 |  | 18.40 |  | 7 | 162.6 | 171.4 | Epidemic Sound (CORE); 6 s ring-out after the last bar; subscription to confirm | 2026-09-03 |
| 1984-sunset-ride | 1984-sunset-ride-avbe-main-version-28116-02-34.wav | 102 |  | 9.7 |  | 6 | 143.8 | 153.2 | Uppbeat free — AVBE; https://uppbeat.io/music/tracks/avbe/1984-sunset-ride | 2026-09-06 |
| continual | continual-prigida-main-version-25319-02-39.wav | 92 |  | 8.8 |  | 13 | 149.7 | 159.1 | Uppbeat premium — Prigida; https://uppbeat.io/music/tracks/prigida/continual | 2026-09-06 |
| discolights | discolights-bosnow-main-version-36535-02-02.wav | 108 |  | 9.3 |  | 10 | 111.5 | 120.4 | Uppbeat premium — Bosnow; https://uppbeat.io/music/tracks/bosnow/discolights | 2026-09-06 |
| funky-burgers | funky-burgers-soundroll-main-version-39378-01-57.wav | 95 |  | 7.8 |  | 9 | 106.3 | 116.4 | Uppbeat premium — Soundroll; https://uppbeat.io/music/tracks/soundroll/funky-burgers | 2026-09-06 |
| hood-2-hood | hood-2-hood-pecan-pie-main-version-29910-01-40.wav | 102 |  | 9.4 |  | 7 | 91.8 | 101.2 | Uppbeat free — Pecan Pie; https://uppbeat.io/music/tracks/pecan-pie/hood-2-hood | 2026-09-06 |
| i-thought-i-knew | i-thought-i-knew-cinco-main-version-02-54-12435.wav | 104 |  | 17.9 |  | 15 | 163.3 | 172.5 | Uppbeat premium — Cinco; https://uppbeat.io/music/tracks/cinco/i-thought-i-knew | 2026-09-06 |
| important-to-you | important-to-you-pecan-pie-main-version-18025-02-06.wav | 97 |  | 20.3 |  | 9 | 114.3 | 124.2 | Uppbeat premium — Pecan Pie; https://uppbeat.io/music/tracks/pecan-pie/important-to-you | 2026-09-06 |
| low-rider | low-rider-sensho-main-version-30030-02-19.wav | 100 |  | 16.2 |  | 12 | 131.4 | 141.0 | Uppbeat premium — SENSHO; https://uppbeat.io/music/tracks/sensho/low-rider | 2026-09-06 |
| moonlight-city | moonlight-city-flow-state-main-version-46721-02-33.wav | 97 |  | 9.9 |  | 6 | 143.5 | 153.4 | Uppbeat premium — Flow State; https://uppbeat.io/music/tracks/flow-state/moonlight-city | 2026-09-06 |
| picture-frames | picture-frames-prigida-main-version-21374-01-44.wav | 92 |  | 10.3 |  | 8 | 96.4 | 106.8 | Uppbeat premium — Prigida; https://uppbeat.io/music/tracks/prigida/picture-frames | 2026-09-06 |
| rainbow | rainbow-tatami-main-version-02-55-12681.wav | 101 |  | 19.0 |  | 6 | 166.3 | 175.9 | Uppbeat premium — Tatami; https://uppbeat.io/music/tracks/tatami/rainbow | 2026-09-06 |
| reflections-in-time-and-space | reflections-in-time-space-danger-lion-x-main-version-11327-02-08.wav | 95 |  | 8.9 |  | 10 | 117.5 | 127.6 | Uppbeat premium — Danger Lion X; https://uppbeat.io/music/tracks/danger-lion-x/reflections-in-time-and-space | 2026-09-06 |
| retro | retro-jeff-kaale-main-version-27224-03-24.wav | 95 |  | 20.1 |  | 17 | 194.4 | 204.5 | Uppbeat premium — Jeff Kaale; https://uppbeat.io/music/tracks/jeff-kaale/retro | 2026-09-06 |
| searching | searching-danger-lion-x-main-version-15846-02-44.wav | 95 |  | 20.2 |  | 13 | 154.1 | 164.2 | Uppbeat premium — Danger Lion X; https://uppbeat.io/music/tracks/danger-lion-x/searching | 2026-09-06 |
| snowman | snowman-ra-main-version-44975-02-42.wav | 100 |  | 20.4 |  | 8 | 145.2 | 154.8 | Uppbeat premium — RA; https://uppbeat.io/music/tracks/ra/snowman | 2026-09-06 |
| soul | soul-danijel-zambo-main-version-01-56-1429.wav | 93 |  | 11.0 |  | 9 | 103.9 | 114.2 | Uppbeat premium — Danijel Zambo; https://uppbeat.io/music/tracks/danijel-zambo/soul | 2026-09-06 |
| star-champion | star-champion-prigida-main-version-28474-02-28.wav | 105 |  | 17.7 |  | 13 | 138.8 | 148.0 | Uppbeat premium — Prigida; https://uppbeat.io/music/tracks/prigida/star-champion | 2026-09-06 |
| sticks-and-stones | sticks-and-stones-kaleidoscope-main-version-22568-01-46.wav | 110 |  | 8.3 |  | 10 | 97.7 | 106.5 | Uppbeat free — Kaleidoscope; https://uppbeat.io/music/tracks/kaleidoscope/sticks-and-stones | 2026-09-06 |
| success | success-swoop-main-version-18178-01-40.wav | 110 |  | 10.9 |  | 9 | 91.6 | 100.4 | Uppbeat free — Swoop; https://uppbeat.io/music/tracks/swoop/success | 2026-09-06 |
| sun-set | sun-set-sega-williams-main-version-16559-02-05.wav | 100 |  | 9.0 |  | 6 | 117.0 | 126.6 | Uppbeat free — Sega Williams; https://uppbeat.io/music/tracks/sega-williams/sun-set | 2026-09-06 |
| tropical-life | tropical-life-kaleidoscope-main-version-27339-02-05.wav | 104 |  | 18.2 |  | 10 | 112.8 | 122.0 | Uppbeat free — Kaleidoscope; https://uppbeat.io/music/tracks/kaleidoscope/tropical-life | 2026-09-06 |
| universal | universal-sky-gienger-main-version-24268-02-01.wav | 111 |  | 17.5 |  | 11 | 112.6 | 121.3 | Uppbeat premium — Sky Gienger; https://uppbeat.io/music/tracks/sky-gienger/universal | 2026-09-06 |
| volt | volt-fass-main-version-28511-02-26.wav | 98 |  | 20.0 |  | 12 | 137.6 | 147.4 | Uppbeat premium — FASS; https://uppbeat.io/music/tracks/fass/volt; catalogue says 110 BPM | 2026-09-06 |

Stamps above are the tool's proposal, unedited; correct any value by ear
and the planner follows the table, not the audio.

*2026-09-03:* the three YouTube Audio Library tracks have vocals and left the
rotation (`../YL-vocals`); the four CORE tracks that pass the screener are
the rotation until the licence question is settled (Epidemic, or a cheaper
monthly catalogue screened the same way). Their rows above are the
stamper's proposals; `retool music check` verifies a grid by ear.

*2026-09-02:* Paradise's proposed `end_s` (175.6) lay past the file's end
(174.68 s), so the ending came up a second short in the first render;
corrected to 174.6. The planner now clamps `end_s` to the file and says so
in the cutlist header; the stamp tool should not propose past the end.

*2026-09-11:* the same on Continual — proposed `end_s` 160.1 lay past the
file's end (159.13 s), and the planner clamped it with a note on the first
video-only card's cutlist; corrected to 159.1.

- `bpm`: the steady tempo. `bar = 240 / bpm` seconds; `phrase = 4 bars`.
- `track_id`: the catalog id, a short slug; the cutlist header carries it as `track:`.
- `band`: a free tag per track (2026-09-05, from the operator's catalog spec;
  the tool reads it and does nothing with it yet, its meaning is to be set).
- `accent_s`: the hit, 8–25 s in. Grid origin; the master starts 12 s before it.
  The frame form the grammar speaks of is derived, `accent_frame = round(accent_s × fps)`
  at the timeline's 30 fps; seconds on the file are the stored value so the
  stamp survives a change of delivery frame rate.
- `shelf_end_s`: where the full arrangement after the accent ends (the first
  breakdown, or the ending), when stamped. The planner checks it against
  `phrases_after_accent` and notes a disagreement in the cutlist; it does not
  yet derive one from the other (2026-09-05, meaning to confirm).
- `phrases_after_accent`: full-arrangement phrases usable after the hit
  before the track thins or changes. Caps the master's room sequence.
- `ending_start_s`: the bar line where the composed ending begins.
- `end_s`: where the last real bar ends. The close lands here.

## The shelf (decided 2026-09-02; the cold open 2026-09-06)

*2026-09-06:* when the intro holds a full phrase before the accent
(`accent_s ≥ 4 bars`) and the card has two teases, the shelf is a phrase,
S 4-4-8: two details from the best hero rooms, then the walk-up as the
arrival eight landing on the accent, the address card over the walk-up.
Shorter intros keep the rule below.


The master starts the track at `accent_s − shelf`, with `shelf = min(12 s,
accent_s)`. The establish is 12 s on screen, or as long as the track's own
intro when the hit comes sooner (never under 8 s, the floor of the craft
research's establishing open), and any hit between 8 and 25 s qualifies. Tracks whose hit comes earlier than
8 s have no ramp to ride; later than 25 s means a long intro the shelf
would have to skip into, which sounds like a fade-in. Change the shelf per
job with a `shelf:` line in the cutlist header.

## How the tool lays the track

- **Master (MLS):** track from `accent_s − 12 s`. Picture starts there on
  the shelf (establish + address card, 12 s). At the accent: cut into the
  first interior. Then N phrases of rooms on the grid, N chosen so the room
  sequence fits and N ≤ `phrases_after_accent`. Then a splice at a bar line
  to the composed ending, crossfaded over 100 ms on the bar, so that the
  close occupies the last real bar and the film ends at `end_s`. Runtime
  = 12 s + N·phrase + (`end_s` − `ending_start_s`), which must land in
  60–110 s; the planner picks N to make it so.
- **Verticals:** head trimmed so the accent is frame zero; M phrases of
  details at one bar each; hard out on a phrase boundary, or the composed
  ending if the cut is long enough to earn it.
- Loudness normalised to −14 LUFS, true peak −1.5 dB. No fade out: the
  ending is composed.

## Licence record

One line per track above (source / licence column). Nothing enters
`ROTATION` without a line. The render log names the track used per job, so any claim
traces back here.

*2026-09-06:* a year of Uppbeat Premium is bought; its rows carry the
track page URL and the account holds the download record. Confirm the
Premium terms (credit, platforms) on the account page before the first
delivery uses one.

Open decision (2026-09-06: Uppbeat is the candidate answer, the screen
above decides): Epidemic keeps every platform clear but costs ~$400 a year
for a ten-track rotation. YouTube Audio Library tracks are free for use in
videos on any platform, some under attribution; claims on other platforms
are rare but not impossible. Either way, a replacement track has to pass
the six essentials above, and the composed-ending rule rules most library
"beds" out.

## The grammar (2026-09-05, the operator's rules; `src/cut.rs` follows them)

- Bar = 4 beats, phrase = 16 beats.
- **Two-duration rule:** 4 beats for details and static mediums; 8 beats for
  wides and gimbal segments. The tool's reading: a hero room's eight is its
  pan when the clip has one, else its hold; second angles, 18-50 details,
  phone verticals and support rooms' statics are fours (the old plan's
  "doorway wide, 4 beats" precedent for support rooms).
- **Phrase patterns:** 8-4-4 is the ShowReady signature, one per hero room
  (the eight, then a medium and a detail; support statics and the room's
  reserve verticals complete it). 4-4-8 is reserved for arrivals: the phrase
  before a room entered by a gimbal move ends on that move, the two fours
  before it are what was waiting (support statics, reserve verticals). A
  move with no two fours to arrive on is skipped, and says so. 4-4-4-4 is
  the hook: the reels open on one, verticals only.
- **Cuts land on beats by frame arithmetic**, from the origin, never by
  accumulating durations: the planner writes `at = shelf + k × bar` from the
  accent and the renderer and the Resolve build place frame `round(at × fps)`.
  Beats come from the BPM, never detected (Epidemic tracks are click-locked).
- **Accent = arrangement arrival**, the downbeat where the full kit lands
  after the sparse intro (peaks going tall and dense and staying there), not
  the melodic hook. The landscape grid counts from the accent; verticals
  place the accent at frame zero, so with one song per job the three reels
  ride the same section.
- **Catalog eligibility:** a track is eligible for a vertical only if it has
  at least that cut's phrase count of full arrangement after the accent,
  before any breakdown (`phrases_after_accent` ≥ 2 for a reel; the master
  needs N ≤ `phrases_after_accent`).
- The cutlist header's `patterns:` line names each phrase's pattern and room,
  and the Resolve build names the phrase markers the same way.

### Social blocks (2026-09-06)

The reels cut in twos. Eight beats is the whole scroll decision on one
picture, so the reel blocks are 2-2-4 and 4-2-2: two grabs and a settle.
Every reel is two phrases: the hook phrase 2-2-2-2 | 2-2-4 (seven cuts,
four inside the first bar pair), then 4-2-2 | 8 with the pan, centre-
cropped, as the payoff where the middle usually sags, or 4-2-2 | 4-2-2
when the reel has no pan. About ten to twelve cuts in the first 15 s, with
a shape rather than a rate. The evidence and the reel purposes are in
`RESEARCH-video-edit.md` §9. The master keeps its fours and eights, in the three blocks 8-4-4 (a hero
room's reveal), 4-4-8 (an arrival) and 4-4-4-4 (a montage of teases after
every second hero: spare verticals, 18-50 details, one glimpse each of the
support rooms over the fit floor). Support rooms under the floor are not
shown. The Resolve ruler is SR1's: every beat Cream, bars Sky, phrase
downbeats Green.
