#!/bin/zsh
# test/synth-card.sh — the synthetic video card (PLAN-video-cut.md, step 0).
#
# Builds a job folder at test/synth/ from finals already on disk, so every
# video step can be verified without a shoot:
#
#   out/       stNN.jpg finals (symlinks), .meta/stNN.json with first_at and
#              pop_at, scorecard.md with hand-set type + fit
#   video/     DSCF####.MOV like the SD card: per station one 8 s clip
#              (3 s still · 3 s ¾-frame pan · 2 s still), a 3 s detail static
#              in hero rooms, then the gimbal pass (2 s still · 2 s push or
#              pull · 2 s still). 4K 60p 10-bit HEVC in F-Log2 / F-Gamut: the
#              inverse cube generated below takes the sRGB finals into log so
#              the WDR cube in `retool cut` round-trips them like real footage.
#   phone/     IMG_####.MOV, 2160×3840 60p Rec.709, 4 s slow tilt, on a clock
#              one hour behind the camera
#   music      → ~/Documents/ShowReady/Music/ROTATION
#   truth.tsv  what every clip is (station, kind, segment times, destination)
#   job.md     address + agent + phone for the title and end cards
#
# Camera clock: station N brackets at 10:00 + 4(N−1) min, pop 20 s later,
# station clip at pop+40 s, detail at pop+70 s, phone clips at pop+100 s and
# pop+110 s (minus one hour on the phone), gimbal pass from the last pop
# +5 min, 30 s apart.
#
# Run from anywhere: test/synth-card.sh    (a few minutes; ~1 GB in test/synth)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=test/synth
rm -rf "$ROOT"
mkdir -p "$ROOT/out/.meta" "$ROOT/video" "$ROOT/phone" "$ROOT/check"
ln -s "$HOME/Documents/ShowReady/Music/ROTATION" "$ROOT/music"

# st  type          fit  source                                  detail-clip  verticals
STATIONS=(
  "01 house_view   55 out/st01/final.jpg                      0 1"
  "02 living_room  60 out/st10/final.jpg                      1 2"
  "03 living_room  45 out/st15/final.jpg                      0 0"
  "04 dining_room  40 out/st06/final.jpg                      0 1"
  "05 kitchen      58 out/st54/final.jpg                      1 2"
  "06 kitchen      45 test/st09/final.jpg                     0 1"
  "07 kitchen      30 out/st47/final.jpg                      0 0"
  "08 bedroom      62 out/st36/final.jpg                      1 2"
  "09 bathroom     50 photocorpus/bathroom/5183419611_16.jpg  1 2"
  "10 bedroom      35 photocorpus/bedroom/5225318086_19.jpg   0 0"
  "11 bathroom     30 photocorpus/bathroom/5775151694_33.jpg  0 0"
  "12 patio        40 photocorpus/patio/5183419611_28.jpg     0 1"
  "13 yard         38 photocorpus/yard/5047001591_0.jpg       0 0"
)
# kind     destination station   move   (the gimbal pass, in shoot order)
GIMBAL=(
  "walkup   01 push"
  "entry    02 push"
  "hallway  08 push"
  "grounds  13 push"
  "close    01 pull"
)

T0=$(TZ=UTC date -j -f "%Y-%m-%d %H:%M:%S" "2026-09-03 10:00:00" +%s)
iso()  { TZ=UTC date -r "$1" +"%Y-%m-%dT%H:%M:%SZ"; }   # the Z makes ffmpeg store the wall clock verbatim, as Fuji does
exif() { TZ=UTC date -r "$1" +"%Y:%m:%d %H:%M:%S"; }

# sRGB (Rec.709) → linear → F-Gamut (Rec.2020 primaries) → F-Log2 curve
# (Fujifilm F-Log2 data sheet constants). JPEG white maps to 90% reflectance.
CUBE=$ROOT/sRGB_to_FLog2_FGamut_33.cube
awk 'function lin(c){return c<=0.04045?c/12.92:((c+0.055)/1.055)^2.4}
     function flog2(x){return x>=0.000889?0.245281*log(5.555556*x+0.064829)/log(10)+0.384316:8.799461*x+0.092864}
     BEGIN{N=33; print "TITLE \"sRGB to F-Log2/F-Gamut, JPEG white = 90% reflectance\""; print "LUT_3D_SIZE " N
       for(b=0;b<N;b++)for(g=0;g<N;g++)for(r=0;r<N;r++){
         lr=lin(r/(N-1))*0.9; lg=lin(g/(N-1))*0.9; lb=lin(b/(N-1))*0.9
         R=0.6274*lr+0.3293*lg+0.0433*lb; G=0.0691*lr+0.9195*lg+0.0114*lb; B=0.0164*lr+0.0880*lg+0.8956*lb
         printf "%.6f %.6f %.6f\n", flog2(R), flog2(G), flog2(B)}}' > "$CUBE"

FUJI=(-c:v hevc_videotoolbox -pix_fmt p010le -b:v 60M -tag:v hvc1)
PHONE=(-c:v hevc_videotoolbox -pix_fmt yuv420p -b:v 40M -tag:v hvc1)
# The still's 16:9 field of view at 6400 wide; a zoom of iw/3840 is a 1:1 window.
PRE="scale=6400:-2,crop=6400:3600"
CENTRE="x='(iw-iw/zoom)/2':y='(ih-ih/zoom)/2'"

# fuji_clip <out.MOV> <src.jpg> <epoch> <frames> <zoompan z:x:y>
fuji_clip() {
  ffmpeg -y -loglevel error -i "$2" \
    -vf "$PRE,zoompan=${5}:d=${4}:s=3840x2160:fps=60,lut3d=file=$CUBE" \
    -frames:v "$4" -r 60 "${FUJI[@]}" -metadata creation_time="$(iso "$3")" "$1"
}
# phone_clip <out.MOV> <src.jpg> <epoch> <third 0|1|2>: a 9:16 window on that
# third of the still, tilting down over 4 s.
phone_clip() {
  ffmpeg -y -loglevel error -i "$2" \
    -vf "scale=6400:-2,crop=2400:ih:${4}*(iw-2400)/2:0,zoompan=z='ih/3840':x='(iw-iw/zoom)/2':y='(ih-ih/zoom)*clip(on/240,0,1)':d=240:s=2160x3840:fps=60" \
    -frames:v 240 -r 60 "${PHONE[@]}" -metadata creation_time="$(iso "$3")" "$1"
}

TRUTH=$ROOT/truth.tsv
printf 'file\tsource\tstation\tkind\tsegments\tdest\n' > "$TRUTH"
SC=$ROOT/out/scorecard.md
{
  echo "# Scorecard — $ROOT/out"
  echo
  echo "Synthetic card from test/synth-card.sh: type and fit are hand-set, not scored."
  echo
  echo "| image | type | fit | verticals | recipe | flags |"
  echo "|---|---|---|---|---|---|"
} > "$SC"

n=1000; m=1000; last_pop=0; check_clip=""
for row in "${STATIONS[@]}"; do
  read -r st type fit src detail nvert <<< "$row"
  first=$((T0 + (10#$st - 1) * 240)); pop=$((first + 20)); last_pop=$pop
  ln -s "$PWD/$src" "$ROOT/out/st$st.jpg"
  cat > "$ROOT/out/.meta/st$st.json" <<JSON
{
  "station": "st$st",
  "brackets": ["DSCF0${st}1.RAF", "DSCF0${st}2.RAF", "DSCF0${st}3.RAF", "DSCF0${st}4.RAF", "DSCF0${st}5.RAF"],
  "pop": "_DSF0${st}6.RAF",
  "first_at": "$(exif $first)",
  "pop_at": "$(exif $pop)",
  "recipe": [],
  "match_gain": 16.0,
  "verticals": "synthetic",
  "rotated_deg": 0.0,
  "crop_pct": 0.0,
  "tone_gain": 1.0
}
JSON
  echo "| st$st.jpg | $type | $fit | synthetic |  |  |" >> "$SC"

  n=$((n + 1)); f=DSCF$n.MOV; echo "st$st $type  $f station clip"
  fuji_clip "$ROOT/video/$f" "$src" $((pop + 40)) 480 "z='iw/3840':x='2560*clip((on-180)/180,0,1)':y='(ih-ih/zoom)/2'"
  printf '%s\tfuji\tst%s\tstation\tstill:0-3,pan:3-6,still:6-8\t\n' "$f" "$st" >> "$TRUTH"
  [[ -z $check_clip && $detail == 1 ]] && check_clip=$f
  if [[ $detail == 1 ]]; then
    n=$((n + 1)); f=DSCF$n.MOV; echo "st$st $type  $f detail static"
    fuji_clip "$ROOT/video/$f" "$src" $((pop + 70)) 180 "z='iw/2560':$CENTRE"
    printf '%s\tfuji\tst%s\tdetail\tstill:0-3\t\n' "$f" "$st" >> "$TRUTH"
  fi
  for ((k = 0; k < nvert; k++)); do
    m=$((m + 1)); f=IMG_$m.MOV; third=$(( nvert == 1 ? 1 : k * 2 )); echo "st$st $type  $f vertical"
    phone_clip "$ROOT/phone/$f" "$src" $((pop + 100 + 10 * k - 3600)) $third
    printf '%s\tphone\tst%s\tvertical\ttilt:0-4\t\n' "$f" "$st" >> "$TRUTH"
  done
done
echo >> "$SC"; echo "coverage: every required room type present" >> "$SC"

t=$((last_pop + 300))
for row in "${GIMBAL[@]}"; do
  read -r kind st move <<< "$row"
  src=$(readlink "$ROOT/out/st$st.jpg")
  n=$((n + 1)); f=DSCF$n.MOV; echo "gimbal $kind → st$st  $f"
  if [[ $move == push ]]; then z="iw/3840*(1+0.4*clip((on-120)/120,0,1))"; else z="iw/3840*(1.4-0.4*clip((on-120)/120,0,1))"; fi
  fuji_clip "$ROOT/video/$f" "$src" $t 360 "z='$z':$CENTRE"
  printf '%s\tfuji\t\t%s\tstill:0-2,%s:2-4,still:4-6\tst%s\n' "$f" "$kind" "$move" "$st" >> "$TRUTH"
  t=$((t + 30))
done

cat > "$ROOT/job.md" <<'MD'
# 12 Synthetic Lane, Testville — Jane Agent, Example Realty

Phone: (555) 010-0100

Synthetic job built by test/synth-card.sh; not a listing.
MD

echo; echo "== verify: every clip decodes, and exiftool reads the tags the ingest needs"
printf 'errs\tfile\tCreateDate\tDuration\tFPS\tW\tH\tRot\tcodec\n'
for f in "$ROOT"/video/*.MOV "$ROOT"/phone/*.MOV; do
  errs=$(ffmpeg -v error -i "$f" -f null - 2>&1 | wc -l | tr -d ' ')
  printf '%s\t' "$errs"; exiftool -T -FileName -CreateDate -Duration -VideoFrameRate -ImageWidth -ImageHeight -Rotation -CompressorID "$f"
done

WDR=luts/XH2_FLog2_FGamut_to_WDR_BT.709_33grid_V.1.00.cube
if [[ -f $WDR ]]; then
  echo; echo "== check frames in $ROOT/check (log vs WDR-graded), look at them"
  ffmpeg -y -loglevel error -ss 4.5 -i "$ROOT/video/$check_clip" -frames:v 1 -vf "scale=1400:-2" "$ROOT/check/station_log.png"
  ffmpeg -y -loglevel error -ss 4.5 -i "$ROOT/video/$check_clip" -frames:v 1 -vf "lut3d=file=$WDR,eq=saturation=1.08,scale=1400:-2" "$ROOT/check/station_wdr.png"
  ffmpeg -y -loglevel error -ss 5.5 -i "$ROOT/video/DSCF$n.MOV" -frames:v 1 -vf "lut3d=file=$WDR,eq=saturation=1.08,scale=1400:-2" "$ROOT/check/close_end_wdr.png"
  ffmpeg -y -loglevel error -ss 2 -i "$ROOT/phone/IMG_1001.MOV" -frames:v 1 -vf "scale=-2:1000" "$ROOT/check/phone.png"
fi
echo; echo "done: $(ls "$ROOT"/video | wc -l | tr -d ' ') camera clips, $(ls "$ROOT"/phone | wc -l | tr -d ' ') phone clips, $(du -sh "$ROOT" | cut -f1)"
