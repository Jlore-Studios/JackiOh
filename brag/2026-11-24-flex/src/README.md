# How this video was built

The scripts run from `brag-output-flex/work/`, two levels below the repository root. Copy this folder's
contents there.

1. Serve the client with `pnpm --dir apps/web dev --port 5173 --strictPort`.
2. Start the test server with fixture accounts: `E2E=1 cargo run --release -p jackioh-server` (:8787).
3. Install the Playwright CLI once with `sh scripts/browse.sh --help`. The scripts require its `playwright-core`
   from `.cache/playwright-cli/` and launch Chrome (`channel: "chrome"`).
4. Write the path of an ffmpeg with libx264 and AAC into `ffpath.txt`. For example:
   `uv run --with imageio-ffmpeg python -c "import imageio_ffmpeg; print(imageio_ffmpeg.get_ffmpeg_exe())" > ffpath.txt`.
5. Capture both cuts' footage: `node grab.cjs landscape` and `node grab.cjs vertical`.
   - The script signs in as the fixture account `e2e-p1` and opens Nature Titan in the Collection.
   - It focuses each rung and strip item, so each look shows on the card's face.
   - It opens the Cosmetics tab's fullscreen previews.
   - It slows the page to 0.1× and records each look as a 30 fps page-time sequence into `grab-landscape/` or
     `grab-vertical/`.
   - The vertical cut needs only the looks its storyboard uses: pass their names, comma-separated, as the second
     argument.
6. Set up the composition:
   - Run `./manifest.sh`, which writes `comp/manifest.json`.
   - Copy `apps/web/public/fonts/alegreya-5.3.0-latin-wght-normal.woff2`,
     `inter-5.3.0-latin-wght-normal.woff2` and `apps/web/public/brand/jackioh-logo.svg` into `comp/assets/`.
   - Link `comp/gl` to `../grab-landscape` and `comp/gv` to `../grab-vertical`.
   - Serve it with `cd comp && python3 -m http.server 8772 --bind 127.0.0.1`.
7. Render:
   - `./sheet.sh <l|v> out.png <t>…` tiles check stills.
   - `node render.cjs l all` renders 600 frames to `frames-l/`.
   - `node render.cjs v all` renders 411 frames to `frames-v/`.
8. Encode:
   - `./audio.sh l` and `./audio.sh v` write the mixes.
   - `./encode.sh l 96` makes `../brag.mp4`, with its poster as frame 0.
   - `./encode.sh v 0` makes `../brag-vertical.mp4`, whose poster is its own first frame, so the loop's seam
     shows nothing new.
