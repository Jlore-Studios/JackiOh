#!/bin/bash
# sheet.sh <l|v> <out.png> <t>…: renders stills at those times and tiles them, 5 to a row.
set -e
cd "$(dirname "$0")"
cut=$1; out=$2; shift 2
node render.cjs "$cut" at "$@" > /dev/null
FF=$(cat ffpath.txt)
ins=(); for t in "$@"; do ins+=(-i "check/$cut-$t.png"); done
n=$#; cols=5; lay=""
for ((i=0;i<n;i++)); do c=$((i%cols)); r=$((i/cols)); x=0; [ $c -gt 0 ] && x=$(printf 'w0%.0s' $(seq 1 $c) | sed 's/w0w0/w0+w0/g; s/w0w0/w0+w0/g; s/w0w0/w0+w0/g'); y=0; [ $r -gt 0 ] && y=$(printf 'h0%.0s' $(seq 1 $r) | sed 's/h0h0/h0+h0/g'); lay="$lay|${x}_${y}"; done
w=$([ "$cut" = v ] && echo 1800 || echo 2400)
"$FF" -y -loglevel error "${ins[@]}" -filter_complex "xstack=inputs=$n:layout=${lay#|}:fill=black,scale=$w:-1" "$out"
echo "$out"
