# How this video was built

The scripts run from `brag-output/work/`, with the repository two levels up. Copy this folder's contents there.

1. Serve the client: `pnpm --dir apps/web dev --port 5173 --strictPort`.
2. Install the Playwright CLI once: `sh scripts/browse.sh --help`. The scripts require its `playwright-core` from `.cache/playwright-cli/` and launch Chrome (`channel: "chrome"`).
3. Write the path of an ffmpeg with libx264 and AAC into `ffpath.txt`. For example: `uv run --with imageio-ffmpeg python -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())" > ffpath.txt`.
4. `node capture.cjs` plays seed 28 (`setup.cjs`, `js/inject.js`, `js/lib.js`). It writes the board stills to `stills/`, then the declare, My Pawn's reveal and the random turn to `seq/` as 60 fps frames in page time, with the page slowed to 0.2× (`SLOW=`).
5. Shoot the faces:
   - `node faces.cjs both core-008`
   - `node faces.cjs base core-096` plus the wall's ids (the `FACES` list in `comp/index.html`)
   - `node faces.cjs radiant core-081 meditative-075 stinkyscape-003 traininggrounds-006 classicplus-012 core-043`
6. Set up the composition:
   - `mkdir -p comp/assets`
   - copy `apps/web/public/fonts/alegreya-5.3.0-latin-wght-normal.woff2`, `inter-5.3.0-latin-wght-normal.woff2` and `apps/web/public/brand/jackioh-logo.svg` into it;
   - link `comp/seq`, `comp/stills` and `comp/faces` to `../seq`, `../stills` and `../faces`;
   - serve it: `cd comp && python3 -m http.server 8765 --bind 127.0.0.1`.
7. `node render.cjs at <t>…` renders stills to `check/`. `node render.cjs all` renders the 600 frames to `frames/`.
8. `./audio.sh` writes `mix.wav`. `./encode.sh 165` makes frame 165 the poster (`../brag.jpg` and frame 0), then encodes `../brag.mp4`.

`seeds.cjs` (a seed search) and `cdp.cjs` drive a Chrome started by hand with `--remote-debugging-port=9333`.
