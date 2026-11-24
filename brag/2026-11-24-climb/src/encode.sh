#!/bin/bash
# encode.sh <land|port> <poster-frame-index>: the poster from that frame, baked in as frame 0, then the mp4 from
# frames-<mode>/ and mix-<mode>.wav. land → ../brag.mp4 + ../brag.jpg; port → ../brag-vertical.mp4 + ../brag-vertical.jpg.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
M=$1; P=$(printf %04d "$2"); D=frames-$M
OUT=$([ "$M" = port ] && echo brag-vertical || echo brag)
"$FF" -y -loglevel error -i "$D/$P.png" -q:v 2 "../$OUT.jpg"
fmt=$("$FF" -hide_banner -i $D/0001.png 2>&1 | grep -o "Video: png, [a-z0-9]*" | awk '{print $3}')
"$FF" -y -loglevel error -i "../$OUT.jpg" -pix_fmt "$fmt" $D/0000.png
"$FF" -y -loglevel error -framerate 30 -i $D/%04d.png -i mix-$M.wav -c:v libx264 -pix_fmt yuv420p -crf 18 -preset slow -c:a aac -b:a 192k -shortest -movflags +faststart "../$OUT.mp4"
echo "encoded ../$OUT.mp4 (poster frame $P)"
