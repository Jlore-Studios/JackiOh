#!/bin/bash
# tile.sh <out.png> <dir> <frame>...: the frames side by side at 270x480, four to a row.
set -e
cd "$(dirname "$0")"
FF=$(cat ffpath.txt); out=$1; dir=$2; shift 2
in=(); fc=""; lay=""; i=0
for f in "$@"; do
  p="$dir/$f"; [ -f "$p" ] || p=$(printf "%s/%04d.jpg" "$dir" "$f")
  in+=(-i "$p"); fc="$fc[$i:v]scale=270:480,setsar=1[v$i];"; sep=""; [ $i -gt 0 ] && sep="|" || true; lay="$lay$sep$(( (i % 4) * 270 ))_$(( (i / 4) * 480 ))"; i=$((i+1))
done
labels=""; for j in $(seq 0 $((i-1))); do labels="$labels[v$j]"; done
"$FF" -y -loglevel error "${in[@]}" -filter_complex "${fc}${labels}xstack=inputs=$i:layout=$lay:fill=black" "$out"
echo "$out"
