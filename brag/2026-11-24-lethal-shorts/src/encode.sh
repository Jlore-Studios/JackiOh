#!/bin/bash
# encode.sh <cut> <poster-frame-index>: frames-<cut>/ from hook-<cut>/ (0-73, 413-419) and the shared body/ (74-412),
# the poster ../brag-vertical-<cut>.jpg from that frame, baked in as frame 0, then ../brag-vertical-<cut>.mp4.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt); CUT=$1; P=$(printf %04d "$2")
rm -rf "frames-$CUT"; mkdir "frames-$CUT"
for i in $(seq 0 419); do
  f=$(printf %04d.jpg $i)
  if [ $i -ge 74 ] && [ $i -le 412 ]; then src=body; else src=hook-$CUT; fi
  [ -f "$src/$f" ] || { echo "missing $src/$f"; exit 1; }
  ln -s "../$src/$f" "frames-$CUT/$f"
done
"$FF" -y -loglevel error -i "frames-$CUT/$P.jpg" -q:v 2 "../brag-vertical-$CUT.jpg"
rm "frames-$CUT/0000.jpg"; cp "../brag-vertical-$CUT.jpg" "frames-$CUT/0000.jpg"
"$FF" -y -loglevel error -framerate 30 -i "frames-$CUT/%04d.jpg" -i "mix-$CUT.wav" -vf "scale=in_range=pc:out_range=tv,format=yuv420p" -color_range tv -c:v libx264 -pix_fmt yuv420p -crf 20 -maxrate 6M -bufsize 12M -preset slow \
  -c:a aac -b:a 160k -shortest -movflags +faststart "../brag-vertical-$CUT.mp4"
echo "encoded ../brag-vertical-$CUT.mp4 (poster frame $P)"
