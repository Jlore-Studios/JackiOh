#!/bin/bash
# encode.sh land|vert <poster-frame-index>: the poster from that frame (brag.jpg / brag-vertical.jpg), baked in
# as frame 0, then the mp4 from frames-<fmt>/ and mix-<fmt>.wav (h264 yuv420p + AAC).
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
fmt=$1; P=$(printf %04d "$2")
if [ "$fmt" = land ]; then OUT=../brag.mp4; JPG=../brag.jpg; else OUT=../brag-vertical.mp4; JPG=../brag-vertical.jpg; fi
D=frames-$fmt
cp "$D/$P.png" poster-$fmt.png
"$FF" -y -loglevel error -i poster-$fmt.png -q:v 2 "$JPG"
pf=$("$FF" -hide_banner -i "$D/0001.png" 2>&1 | grep -o "Video: png, [a-z0-9]*" | awk '{print $3}')
"$FF" -y -loglevel error -i "$JPG" -pix_fmt "$pf" "$D/0000.png"
"$FF" -y -loglevel error -framerate 30 -i "$D/%04d.png" -i mix-$fmt.wav -c:v libx264 -pix_fmt yuv420p -crf 20 -preset slow -c:a aac -b:a 192k -movflags +faststart -shortest "$OUT"
echo "encoded $OUT (poster frame $P)"
