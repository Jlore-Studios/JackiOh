# How this video was built

The scripts run from `brag-output-fast/work/`, with the repository two levels up. Copy this folder's contents there.

1. Serve the client: `pnpm --dir apps/web dev --port 5173 --strictPort`.
2. Install the Playwright CLI once: `sh scripts/browse.sh --help`. The scripts require its `playwright-core` from `.cache/playwright-cli/0.1.22/` and launch Chrome (`channel: "chrome"`).
3. Write the path of an ffmpeg with libx264 and AAC into `ffpath.txt`. For example: `uv run --with imageio-ffmpeg python -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())" > ffpath.txt`.
4. `node scan.cjs scan 1 24` plays seeds with the greedy policy (`js/policy.js`) and the decks (`decks.cjs`, `js/inject.js`) and prints each result. `node scan.cjs detail 12` lists seed 12 action by action.
5. `node capture.cjs land` and `node capture.cjs port` play seed 12 from p1's seat:
   - a still after every action goes to `stills/<tag>/NNN.png`, with its `.json`;
   - the final action, Clobberer's drag onto the hero, records with the page slowed to 0.2× into `seq/<tag>/` (60 fps of page time, `marks.json`);
   - `port` is the phone layout, 540×960 CSS px at deviceScaleFactor 2.
6. `mkdir -p faces && node faces.cjs base <the 25 ids of decks.cjs's A>` shoots the deck's faces from `/almanac`.
7. Set up the composition:
   - `mkdir -p comp/assets`
   - copy `apps/web/public/fonts/alegreya-5.3.0-latin-wght-normal.woff2`, `inter-5.3.0-latin-wght-normal.woff2` and `apps/web/public/brand/jackioh-logo.svg` into it;
   - link `comp/seq`, `comp/stills` and `comp/faces` to `../seq`, `../stills` and `../faces`;
   - serve it: `cd comp && python3 -m http.server 8771 --bind 127.0.0.1`.
8. Render:
   - `node render.cjs land at <t>…` (or `vert`) renders stills to `check/`;
   - `BANDS=1` draws the vertical safe-zone bands over them;
   - `node render.cjs land all` and `node render.cjs vert all` render `frames-land/` (628) and `frames-vert/` (419).
9. `./audio.sh land` and `./audio.sh vert` write the mixes. Then `./encode.sh land 159` and `./encode.sh vert 159` bake in the posters and encode `../brag.mp4` and `../brag-vertical.mp4`.

`sheet.sh` makes contact sheets for checking.
