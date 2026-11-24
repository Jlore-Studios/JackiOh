# How these cuts were built

The scripts run from `brag-output-lethal-shorts/work/`, with the repository two levels up. Copy this folder's
contents there.

1. Serve the client: `pnpm --dir apps/web dev --port 5173 --strictPort`. Then install the Playwright CLI once with
   `sh scripts/browse.sh --help`. The scripts require its `playwright-core` from `.cache/playwright-cli/` and launch
   Chrome (`channel: "chrome"`).
2. Write the path of an ffmpeg with libx264 and AAC into `ffpath.txt`, e.g.
   `uv run --with imageio-ffmpeg python -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())" > ffpath.txt`.
3. `node capture.cjs` plays seed 28 at a 540×960 viewport, deviceScaleFactor 2, so the client draws its phone layout
   (`setup.cjs`, `js/inject.js`, `js/lib.js`; `node peek.cjs` takes only the stills).
   - It writes the board stills to `stills/` and the declare, the reveal and the random turn to `seq/` as 60 fps
     frames in page time, with the page slowed to 0.2× (`SLOW=`).
   - Chrome runs with `--force-device-scale-factor=2`, or the screencast's frames come out 540×960.
   - At this size the recording can overrun, because node falls behind the frames. The composition uses page time
     0–9 s only, and the script stops writing frames past the end.
4. Faces: copy the launch video's `faces/` (`brag/2026-11-23-launch/src/README.md`, step 5) into `faces/`.
5. Set up the composition:
   - `mkdir -p comp/assets`, then copy `apps/web/public/fonts/alegreya-5.3.0-latin-wght-normal.woff2`,
     `inter-5.3.0-latin-wght-normal.woff2` and `apps/web/public/brand/jackioh-logo.svg` into it;
   - link `comp/seq`, `comp/stills` and `comp/faces` to `../seq`, `../stills` and `../faces`;
   - serve it: `cd comp && python3 -m http.server 8774 --bind 127.0.0.1`.
   - `index.html?cut=a|b|c` picks the hook; `&bands=1` draws the platforms' UI bands for checks.
6. Check stills with `node render.cjs <cut> at [--bands] <t>…` (they go to `check/`, and `./tile.sh` tiles them).
7. Render the frames:
   - the shared body: `node render.cjs a all body 74 413`;
   - each cut's hook and loop tail: `node render.cjs <cut> all hook-<cut> 0 74` and `… 413 420`.
8. Encode:
   - `./audio.sh <cut>` writes `mix-<cut>.wav`;
   - `./encode.sh a 36`, `./encode.sh b 48` and `./encode.sh c 18` make those frames the posters (`../brag-vertical-<cut>.jpg`, baked in as frame 0), then encode `../brag-vertical-<cut>.mp4`.
