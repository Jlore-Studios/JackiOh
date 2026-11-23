#!/bin/bash
# encode.sh <poster-frame-index>: brag.jpg from that frame, baked in as frame 0, then brag.mp4 from frames/ and mix.wav.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt)
P=$(printf %04d "$1")
cp "frames/$P.png" poster.png
"$FF" -y -loglevel error -i poster.png -q:v 2 ../brag.jpg
# Frame 0 becomes the poster, decoded back from the JPEG in the same pixel format as its neighbours.
fmt=$("$FF" -hide_banner -i frames/0001.png 2>&1 | grep -o "Video: png, [a-z0-9]*" | awk '{print $3}')
"$FF" -y -loglevel error -i ../brag.jpg -pix_fmt "$fmt" frames/0000.png
echo "poster frame $P, frame 0 now $("$FF" -hide_banner -i frames/0000.png 2>&1 | grep -o 'Video: png, [a-z0-9]*')"
"$FF" -y -loglevel error -framerate 30 -i frames/%04d.png -i mix.wav -c:v libx264 -pix_fmt yuv420p -crf 18 -preset slow -c:a aac -b:a 192k -shortest ../brag.mp4
echo "encoded"
