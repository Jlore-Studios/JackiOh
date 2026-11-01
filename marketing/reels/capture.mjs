#!/usr/bin/env node
// Records the practice AI playing itself on the dev hotseat board, as a portrait (9:16) silent clip
// for a reel (#2616). The client renders `viewFor` as it always does (CLAUDE.md rule 7): this only
// picks each seat's action with the AI's own `decide` and hands it to the dev handle's `dispatch`.
//
//   pnpm --dir apps/web dev --port 5173 --strictPort      # a dev build: /dev/hotseat and the source modules
//   node marketing/reels/capture.mjs [--url http://localhost:5173] [--seed 42] [--a <deck>] [--b <deck>]
//                                    [--from-turn 5] [--seconds 60] [--out marketing/reels/out/gameplay.mp4]
//
// The turns before --from-turn are played unfilmed with reduced motion, so the clip opens on a board
// with units on it.
//
// Needs Playwright with Chromium (`require('playwright')`) and ffmpeg on PATH.
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { parseArgs } from "node:util";

const { values: opt } = parseArgs({
  options: {
    url: { type: "string", default: "http://localhost:5173" },
    seed: { type: "string", default: "42" },
    a: { type: "string" },
    b: { type: "string" },
    seconds: { type: "string", default: "60" },
    "from-turn": { type: "string", default: "5" },
    out: { type: "string", default: "marketing/reels/out/gameplay.mp4" },
  },
});

// A phone's CSS size at a phone's density: the client lays out its phone board, and the frames come
// out at 1080x1920.
const VIEWPORT = { width: 432, height: 768 };
const SCALE = 2.5;
const VIDEO = { width: VIEWPORT.width * SCALE, height: VIEWPORT.height * SCALE };
const FPS = 30;
// How long a settled board stays on screen between actions, so a viewer can follow the play.
const BEAT_MS = 700;

const { chromium } = createRequire(import.meta.url)("playwright");
const out = resolve(opt.out);
mkdirSync(dirname(out), { recursive: true });
const raw = mkdtempSync(join(tmpdir(), "jackioh-reel-"));

const params = new URLSearchParams({ seed: opt.seed, ...(opt.a ? { a: opt.a } : {}), ...(opt.b ? { b: opt.b } : {}) });
const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: VIEWPORT,
  deviceScaleFactor: SCALE,
  isMobile: true,
  hasTouch: true,
});
const page = await context.newPage();
await page.goto(`${opt.url}/dev/hotseat?${params}`);
await page.waitForFunction(() => window.__jackioh !== undefined, null, { timeout: 30_000 });

// The dev bar (the seed, "Hand over to p2") is for testers, not viewers.
await page.addStyleTag({ content: ".hotseat-bar { display: none !important; }" });

// One AI action for every seat that owes one, then a wait until the board settles. `decide` is the
// practice AI's (SURFACE §9); Vite serves the client's own modules, so the import is the page's WASM
// engine, already loaded.
await page.evaluate(async (seed) => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const wasm = await import("/src/wasm/index.ts");
  const j = window.__jackioh;
  let cursor = 0;
  window.__reel = {
    async step(beatMs) {
      const seats = ["p1", "p2"].filter((seat) => wasm.aiToAct(j.state, seat));
      for (const seat of seats) {
        const { decision, rngCursor } = wasm.decide(j.state, seat, { rngSeed: `reel-${seed}`, rngCursor: cursor }, 0);
        cursor = rngCursor;
        const action = decision?.action ?? j.legal().find((a) => a.type === "endTurn");
        if (action === undefined) continue;
        j.dispatch({ ...action, playerId: seat });
        // Keep p1's side of the board on screen: take the device back after p2 acts.
        if (seat !== "p1") j.setSeat("p1");
        const deadline = Date.now() + 10_000;
        await sleep(50);
        while (document.querySelector("[data-animating], [data-speaking]") && Date.now() < deadline) await sleep(50);
        await sleep(beatMs);
      }
      return seats.length > 0 && j.state.result === null;
    },
    turn: () => j.state.turn,
    result: () => j.state.result,
  };
}, opt.seed);

await page.emulateMedia({ reducedMotion: "reduce" });
const fromTurn = Number(opt["from-turn"]);
while ((await page.evaluate(() => window.__reel.turn())) < fromTurn && (await page.evaluate(() => window.__reel.step(0))));
await page.emulateMedia({ reducedMotion: "no-preference" });

// Playwright's own recordVideo films the page at its CSS size; Chromium's screencast sends frames at
// the device's pixels. Each frame is kept with the time it was painted, and shown until the next.
const frames = [];
const cdp = await context.newCDPSession(page);
cdp.on("Page.screencastFrame", ({ data, metadata, sessionId }) => {
  const file = join(raw, `${String(frames.length).padStart(6, "0")}.jpg`);
  writeFileSync(file, Buffer.from(data, "base64"));
  frames.push({ file, t: metadata.timestamp });
  cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
});
await cdp.send("Page.startScreencast", { format: "jpeg", quality: 92, maxWidth: VIDEO.width, maxHeight: VIDEO.height });

let actions = 0;
const stop = Date.now() + Number(opt.seconds) * 1000;
while (Date.now() < stop && (await page.evaluate((beat) => window.__reel.step(beat), BEAT_MS))) actions += 1;
const summary = { actions, turn: await page.evaluate(() => window.__reel.turn()), result: await page.evaluate(() => window.__reel.result()) };

await cdp.send("Page.stopScreencast");
await browser.close();
if (frames.length === 0) throw new Error("the screencast sent no frames");

// The concat demuxer's list: each frame for as long as it stayed on screen, then a constant frame rate.
const list = frames.map((f, i) => `file '${f.file}'\nduration ${((frames[i + 1]?.t ?? f.t + 1 / FPS) - f.t).toFixed(4)}`);
writeFileSync(join(raw, "frames.txt"), `${list.join("\n")}\nfile '${frames.at(-1).file}'\n`);
execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", join(raw, "frames.txt"),
  "-vf", `fps=${FPS},scale=${VIDEO.width}:${VIDEO.height}:force_original_aspect_ratio=decrease,pad=${VIDEO.width}:${VIDEO.height}:(ow-iw)/2:(oh-ih)/2,format=yuv420p`,
  "-c:v", "libx264", "-crf", "18", "-movflags", "+faststart", out]);
rmSync(raw, { recursive: true, force: true });
console.log(JSON.stringify({ out, frames: frames.length, ...summary }));
