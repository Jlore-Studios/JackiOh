// The win: p1's turn-3 swing of Mr. Vanilla on the hero at 3 and the VICTORY that follows, recorded as a
// 60 fps sequence in the page's own slowed time. MODE=land (1920x1080) or MODE=port (540x960 CSS at 2x,
// the client's phone layout). Writes stills-<mode>/ and seq-<mode>/ (+ marks.json).
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const { setup } = require("./setup.cjs");
const MODE = process.env.MODE || "land";
const PORT = MODE === "port";
const VW = PORT ? 540 : 1920, VH = PORT ? 960 : 1080, DSF = PORT ? 2 : 1;
const F = 0.2, HOLD = 1.2, AFTER = Number(process.env.AFTER || 7), FPS = 60;
const SD = `stills-${MODE}`, SQ = `seq-${MODE}`;
(async () => {
  fs.mkdirSync(SD, { recursive: true });
  fs.rmSync(SQ, { recursive: true, force: true });
  fs.mkdirSync(SQ, { recursive: true });
  const args = ["--mute-audio", "--hide-scrollbars", ...(PORT ? ["--force-device-scale-factor=2"] : [])];
  const browser = await chromium.launch({ channel: "chrome", headless: true, args });
  const ctx = await browser.newContext({ viewport: { width: VW, height: VH }, deviceScaleFactor: DSF, ...(PORT ? { isMobile: false, hasTouch: false } : {}) });
  await ctx.addInitScript(() => {
    const add = () => { const s = document.createElement("style"); s.textContent = ".hotseat-bar{display:none !important}"; document.head.appendChild(s); };
    if (document.head) add(); else document.addEventListener("DOMContentLoaded", add);
  });
  const p = await ctx.newPage();
  const shot = async (n) => { const st = await p.evaluate("__brag.settle()"); await p.screenshot({ path: `${SD}/${n}.png` }); console.log("still", n, st); };
  await setup(p, shot);
  const u = await p.locator('[data-testid="card-c3"]').boundingBox();
  const hero = await p.locator('[data-testid="hero-opponent"]').boundingBox();
  if (!u || !hero) throw new Error("no unit or hero box " + JSON.stringify({ u, hero }));
  const target = { x: hero.x + hero.width / 2, y: hero.y + hero.height / 2 };
  await p.mouse.move(u.x + u.width / 2, u.y + u.height / 2);
  await p.mouse.down();
  await p.mouse.move(target.x, target.y, { steps: 24 });
  await p.waitForTimeout(700);
  await p.screenshot({ path: `${SD}/aim.png` });
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
  const vt = (wallMs) => ((wallMs - d0) * F) / 1000;
  let last = null, next = 0, size = null;
  const marks = {};
  cdp.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
    cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
    const v = vt(metadata.timestamp * 1000);
    if (!size) size = [metadata.deviceWidth, metadata.deviceHeight];
    if (last) while (next / FPS < v) { fs.writeFileSync(`${SQ}/${String(next).padStart(4, "0")}.jpg`, Buffer.from(last, "base64")); next++; }
    last = data;
  });
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 95, maxWidth: VW * DSF, maxHeight: VH * DSF, everyNthFrame: 1 });
  const waitPage = async (sec) => { await p.waitForTimeout((sec / F) * 1000); };
  await waitPage(HOLD / 2);
  await p.mouse.move(target.x + 2, target.y + 1, { steps: 3 });
  await waitPage(HOLD / 2);
  marks.release = vt(Date.now());
  await p.mouse.up();
  // Note when the result arrives in state, in page time.
  const t0 = Date.now();
  while (Date.now() - t0 < (AFTER / F) * 1000) {
    if (marks.result === undefined && (await p.evaluate(() => !!window.__jackioh.state.result))) marks.result = vt(Date.now());
    if (marks.overlay === undefined && (await p.evaluate(() => /victory/i.test(document.body.innerText.slice(0, 20000))))) marks.overlay = vt(Date.now());
    await p.waitForTimeout(250);
  }
  marks.end = vt(Date.now());
  await cdp.send("Page.stopScreencast");
  await new Promise((r) => setTimeout(r, 500));
  if (last) fs.writeFileSync(`${SQ}/${String(next).padStart(4, "0")}.jpg`, Buffer.from(last, "base64")), next++;
  Object.assign(marks, { frames: next, fps: FPS, F, size });
  fs.writeFileSync(`${SQ}/marks.json`, JSON.stringify(marks));
  console.log(JSON.stringify(marks));
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
