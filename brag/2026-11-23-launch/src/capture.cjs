// Plays seed 28 from p1's seat at 1920x1080 and captures: board stills (stills/*.png) and the declare →
// My Pawn reveal → random turn as a 60 fps image sequence in the page's own (slowed) time (seq/*.jpg).
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const { setup } = require("./setup.cjs");
const F = Number(process.env.SLOW || 0.2); // page time runs at F of real time while recording
const HOLD = 1.6; // seconds of page time the arrow is held before release
const AFTER = 9.5; // seconds of page time recorded after release
const FPS = 60;

(async () => {
  fs.mkdirSync("stills", { recursive: true });
  fs.rmSync("seq", { recursive: true, force: true });
  fs.mkdirSync("seq", { recursive: true });
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio", "--hide-scrollbars"] });
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  await ctx.addInitScript(() => {
    const add = () => { const s = document.createElement("style"); s.textContent = ".hotseat-bar{display:none !important}"; document.head.appendChild(s); };
    if (document.head) add(); else document.addEventListener("DOMContentLoaded", add);
  });
  const p = await ctx.newPage();
  const shot = async (n) => {
    const st = await p.evaluate("__brag.settle()");
    await p.screenshot({ path: `stills/${n}.png` });
    console.log("still", n, st);
  };
  await setup(p, shot);

  // The arrow: press Mr. Vanilla, drag to the enemy hero, hold, release (the real declare).
  const u = await p.locator('[data-testid="card-c3"]').boundingBox();
  const hero = await p.locator('[data-testid="hero-opponent"]').boundingBox();
  const target = { x: hero.x + 42, y: hero.y + hero.height / 2 };
  await p.mouse.move(u.x + u.width / 2, u.y + u.height / 2);
  await p.mouse.down();
  await p.mouse.move(target.x, target.y, { steps: 24 });
  await p.waitForTimeout(700);
  await p.screenshot({ path: "stills/aim.png" });
  console.log("still aim");

  // Slow the page: its clocks, timers and frames in JS, and every CSS/Web animation through CDP.
  const cdp = await ctx.newCDPSession(p);
  await cdp.send("Animation.enable");
  const d0 = await p.evaluate((F) => {
    const rp = performance.now.bind(performance), rd = Date.now.bind(Date);
    const rST = window.setTimeout.bind(window), rSI = window.setInterval.bind(window), rRAF = window.requestAnimationFrame.bind(window);
    const p0 = rp(), d0 = rd();
    performance.now = () => p0 + (rp() - p0) * F;
    Date.now = () => d0 + (rd() - d0) * F;
    window.setTimeout = (fn, ms = 0, ...a) => rST(fn, Math.max(0, Number(ms) || 0) / F, ...a);
    window.setInterval = (fn, ms = 0, ...a) => rSI(fn, Math.max(1, Number(ms) || 0) / F, ...a);
    window.requestAnimationFrame = (cb) => rRAF((ts) => cb(p0 + (ts - p0) * F));
    return d0;
  }, F);
  await cdp.send("Animation.setPlaybackRate", { playbackRate: F });
  const vt = (wallMs) => ((wallMs - d0) * F) / 1000; // page seconds since the slow-down

  let last = null, next = 0, written = 0;
  const marks = {};
  cdp.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
    cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
    const v = vt(metadata.timestamp * 1000);
    if (last) {
      while (next / FPS < v) {
        fs.writeFileSync(`seq/${String(next).padStart(4, "0")}.jpg`, Buffer.from(last, "base64"));
        next++; written++;
      }
    }
    last = data;
  });
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 95, maxWidth: 1920, maxHeight: 1080, everyNthFrame: 1 });
  const waitPage = async (sec) => { await p.waitForTimeout((sec / F) * 1000); };
  // A small wiggle keeps the arrow live, then release on the hero.
  await waitPage(HOLD / 2);
  await p.mouse.move(target.x + 2, target.y + 1, { steps: 3 });
  await waitPage(HOLD / 2);
  marks.release = vt(Date.now());
  await p.mouse.up();
  await waitPage(AFTER);
  marks.end = vt(Date.now());
  await cdp.send("Page.stopScreencast");
  await new Promise((r) => setTimeout(r, 500));
  if (last) fs.writeFileSync(`seq/${String(next).padStart(4, "0")}.jpg`, Buffer.from(last, "base64")), next++;
  marks.frames = next; marks.fps = FPS; marks.F = F;
  marks.state = await p.evaluate(() => ({ turn: window.__jackioh.state.turn, result: window.__jackioh.state.result, h2: window.__jackioh.state.players.p2.hero }));
  fs.writeFileSync("seq/marks.json", JSON.stringify(marks, null, 1));
  console.log(JSON.stringify(marks));
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 2000)); process.exit(1); });
