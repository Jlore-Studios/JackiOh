#!/bin/bash
# encode.sh <l|v> <poster-frame-index>: the poster (brag.jpg / brag-vertical.jpg) from that frame, baked in as
# frame 0, then the mp4 from frames-<cut>/ and mix-<cut>.wav.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
cut=$1; P=$(printf %04d "$2")
if [ "$cut" = v ]; then OUT=../brag-vertical; else OUT=../brag; fi
cp "frames-$cut/$P.png" "poster-$cut.png"
"$FF" -y -loglevel error -i "poster-$cut.png" -q:v 2 "$OUT.jpg"
# Frame 0 becomes the poster, decoded back from the JPEG in the same pixel format as its neighbours.
fmt=$("$FF" -hide_banner -i "frames-$cut/0001.png" 2>&1 | grep -o "Video: png, [a-z0-9]*" | awk '{print $3}')
"$FF" -y -loglevel error -i "$OUT.jpg" -pix_fmt "$fmt" "frames-$cut/0000.png"
"$FF" -y -loglevel error -framerate 30 -i "frames-$cut/%04d.png" -i "mix-$cut.wav" -c:v libx264 -pix_fmt yuv420p -crf 20 -preset slow -c:a aac -b:a 192k -shortest -movflags +faststart "$OUT.mp4"
echo "$OUT.mp4 $(stat -f %z "$OUT.mp4") bytes"
