#!/bin/bash
# sheet.sh <out.jpg> <cols> <tile-width> <image>... : a contact sheet of the given images (all one type), in order.
set -e
out=$1; cols=$2; w=$3; shift 3
FF=$(cat "$(dirname "$0")/ffpath.txt")
ext="${1##*.}"
tmp=$(mktemp -d)
i=0; for f in "$@"; do cp "$f" "$tmp/$(printf %03d $i).$ext"; i=$((i+1)); done
rows=$(( (i + cols - 1) / cols ))
"$FF" -y -loglevel error -reinit_filter 0 -i "$tmp/%03d.$ext" -vf "format=rgb24,scale=$w:-1,tile=${cols}x${rows}" -frames:v 1 "$out"
rm -rf "$tmp"
