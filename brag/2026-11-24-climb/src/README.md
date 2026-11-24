# How this video was built

The scripts run from `brag-output-climb/work/`, with the repository two levels up. Copy this folder's contents there.

1. Serve the client: `pnpm --dir apps/web dev --port 5173 --strictPort`.
2. Install the Playwright CLI once: `sh scripts/browse.sh --help`. The scripts require its `playwright-core` from `.cache/playwright-cli/` and launch Chrome (`channel: "chrome"`).
3. Write the path of an ffmpeg with libx264 and AAC into `ffpath.txt`. For example: `uv run --with imageio-ffmpeg python -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())" > ffpath.txt`.
4. `node real.cjs` renders the client's own components on a page of the served client, through `pw.cjs`, which mounts them with the app's own React:
   - every `RankCrest` (Raisin 0–5 placements, each Grape tier and division, Jlorious #1 and #100) into `comp/assets/crests.json`;
   - the Jlorious `CardBack`, sampled over its 4 s breathe at 30 fps, into `back/`.
5. `MODE=land node capture-win.cjs` and `MODE=port node capture-win.cjs` play seed 28 (`setup.cjs`, `js/inject.js`, `js/lib.js`) and record turn 3's winning swing and the VICTORY.
   - The page is slowed to 0.2×, and the captures are 60 fps frames in page time, in `seq-land/` and `seq-port/`.
   - The portrait capture is the client's phone layout: 540×960 CSS at 2×, with Chrome started with `--force-device-scale-factor=2`.
6. Set up the composition:
   - `mkdir -p comp/assets`
   - copy `apps/web/public/fonts/alegreya-5.3.0-latin-wght-normal.woff2`, `inter-5.3.0-latin-wght-normal.woff2`, `apps/web/public/brand/jackioh-logo.svg` and `apps/web/src/rank/rankCrest.css` into it;
   - link `comp/seq-land`, `comp/seq-port` and `comp/back` to `../seq-land`, `../seq-port` and `../back`;
   - serve it: `cd comp && python3 -m http.server 8773 --bind 127.0.0.1`.
7. `node render.cjs land|port at <t>…` renders stills to `check/`. `node render.cjs land all` and `node render.cjs port all` render the frames (576 and 432) to `frames-land/` and `frames-port/`. `comp/index.html?v=1` is the vertical cut.
8. `./audio.sh` writes `mix-land.wav` and `mix-port.wav`. `./encode.sh land 408` and `./encode.sh port 48` make the posters, bake each in as frame 0, and encode `../brag.mp4` and `../brag-vertical.mp4`.
