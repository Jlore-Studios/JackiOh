#!/bin/sh
# usage: shot.sh <name> [width] [height]  -- renders /tmp/preview/<name>.html to /tmp/preview/<name>.png
name="$1"; w="${2:-1600}"; h="${3:-1000}"
google-chrome --headless=new --no-sandbox --disable-gpu --hide-scrollbars --window-size="$w,$h" --screenshot="/tmp/preview/$name.png" "file:///tmp/preview/$name.html" >/dev/null 2>&1
echo "/tmp/preview/$name.png"
