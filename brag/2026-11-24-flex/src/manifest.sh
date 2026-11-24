#!/bin/bash
# Writes comp/manifest.json: the frame count of every captured look, for both cuts.
cd "$(dirname "$0")"
{ echo "{"; first=1; for g in gl:grab-landscape gv:grab-vertical; do k=${g%%:*}; d=${g#*:}; for s in "$d"/*/; do [ -d "$s" ] || continue; n=$(ls "$s" | grep -c '\.png$'); [ $first = 1 ] || echo ","; first=0; printf '"%s/%s": %s' "$k" "$(basename "$s")" "$n"; done; done; echo; echo "}"; } > comp/manifest.json
