// capture.cjs land|port: plays seed 12 with the greedy policy from p1's seat, a still after every action
// (stills/<tag>/NNN.png, NNN.json), then the final action (Clobberer onto the hero) as the real drag, recorded
// with the page slowed to F (seq/<tag>/####.jpg at 60 fps of page time, marks.json).
const fs = require("fs"), path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const { A, B } = require("./decks.cjs");
const SEED = 12, LAST = 29; // the scan: the 29th action is the lethal swing
const F = Number(process.env.SLOW || 0.2), HOLD = 1.4, AFTER = 6.5, FPS = 60;
const tag = process.argv[2] || "land";
const VP = tag === "land" ? { w: 1920, h: 1080, dsf: 1 } : { w: 540, h: 960, dsf: 2 };
const lib = fs.readFileSync("js/lib.js", "utf8"), policy = fs.readFileSync("js/policy.js", "utf8");
const inject = fs.readFileSync("js/inject.js", "utf8").replace("SEED", String(SEED)).replace("DECK_A", JSON.stringify(A)).replace("DECK_B", JSON.stringify(B));
(async () => {
  const SD = `stills/${tag}`, QD = `seq/${tag}`;
  fs.rmSync(SD, { recursive: true, force: true }); fs.mkdirSync(SD, { recursive: true });
  fs.rmSync(QD, { recursive: true, force: true }); fs.mkdirSync(QD, { recursive: true });
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio", "--hide-scrollbars"] });
  const ctx = await browser.newContext({ viewport: { width: VP.w, height: VP.h }, deviceScaleFactor: VP.dsf });
  await ctx.addInitScript(() => { const add = () => { const s = document.createElement("style"); s.textContent = ".hotseat-bar{display:none !important}"; document.head.appendChild(s); }; if (document.head) add(); else document.addEventListener("DOMContentLoaded", add); });
  const p = await ctx.newPage();
  await p.goto("http://localhost:5173/");
  await p.evaluate(`(${inject})()`);
  await p.goto(`http://localhost:5173/dev/hotseat?seed=${SEED}&a=fast-a&b=fast-b`);
  await p.waitForFunction(() => !!window.__jackioh, null, { timeout: 60000 });
  await p.evaluate(`(${lib})()`); await p.evaluate(`(${policy})()`);
  await p.evaluate("__brag.ready()"); await p.evaluate("__brag.keep()"); await p.evaluate("__brag.settle()");
  const info = () => p.evaluate(() => { const s = window.__jackioh.state; return { turn: s.turn, active: s.active, h1: s.players.p1.hero.health, h2: s.players.p2.hero.health, m1: s.players.p1.mana, m2: s.players.p2.mana }; });
  const shot = async (i, extra) => {
    await p.evaluate(() => window.__jackioh.setSeat("p1"));
    const st = await p.evaluate("__brag.settle()");
    const n = String(i).padStart(3, "0");
    await p.screenshot({ path: `${SD}/${n}.png` });
    fs.writeFileSync(`${SD}/${n}.json`, JSON.stringify({ ...(await info()), ...extra, st }));
  };
  await shot(0, { kind: "start" });
  let x;
  for (let i = 1; i < LAST; i++) {
    x = await p.evaluate(() => { const x = window.__fast.next(); const h = {}; for (const q of ["p1", "p2"]) for (const c of window.__jackioh.state.players[q].hand) h[c.id] = c.defId; x.card = x.a.instanceId ? h[x.a.instanceId] : null; return x; });
    await p.evaluate((x) => window.__jackioh.dispatch({ ...x.a, playerId: x.seat }), x);
    await shot(i, { kind: x.kind, seat: x.seat, card: x.card, a: x.a });
  }
  x = await p.evaluate(() => { const x = window.__fast.next(); x.def = window.__fast.unitOf(x.a.attackerId)?.defId; return x; });
  if (x.kind !== "attackHero" || x.def !== "traininggrounds-008") throw new Error("unexpected last action " + JSON.stringify(x));
  await p.evaluate(() => window.__jackioh.setSeat("p1")); await p.evaluate("__brag.settle()");
  // The real declare: press Clobberer, drag onto the enemy hero, hold, release.
  const u = await p.locator(`[data-testid="card-${x.a.attackerId}"]`).boundingBox();
  const hero = await p.locator('[data-testid="hero-opponent"]').boundingBox();
  const target = { x: hero.x + Math.min(42, hero.width / 4), y: hero.y + hero.height / 2 };
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
  let last = null, next = 0;
  const marks = {};
  cdp.on("Page.screencastFrame", async ({ data, metadata, sessionId }) => {
    cdp.send("Page.screencastFrameAck", { sessionId }).catch(() => {});
    const v = vt(metadata.timestamp * 1000);
    if (last) while (next / FPS < v) { fs.writeFileSync(`${QD}/${String(next).padStart(4, "0")}.jpg`, Buffer.from(last, "base64")); next++; }
    last = data;
  });
  await cdp.send("Page.startScreencast", { format: "jpeg", quality: 92, maxWidth: VP.w * VP.dsf, maxHeight: VP.h * VP.dsf, everyNthFrame: 1 });
  const waitPage = (sec) => p.waitForTimeout((sec / F) * 1000);
  await waitPage(0.3);
  marks.press = vt(Date.now());
  await p.mouse.move(u.x + u.width / 2, u.y + u.height / 2);
  await p.mouse.down();
  await p.mouse.move(target.x, target.y, { steps: 30 });
  marks.aimed = vt(Date.now());
  await waitPage(HOLD / 2);
  await p.mouse.move(target.x + 2, target.y + 1, { steps: 3 });
  await waitPage(HOLD / 2);
  marks.release = vt(Date.now());
  await p.mouse.up();
  await waitPage(AFTER);
  marks.end = vt(Date.now());
  await cdp.send("Page.stopScreencast");
  await new Promise((r) => setTimeout(r, 500));
  if (last) { fs.writeFileSync(`${QD}/${String(next).padStart(4, "0")}.jpg`, Buffer.from(last, "base64")); next++; }
  Object.assign(marks, { frames: next, fps: FPS, F, state: await info(), result: await p.evaluate(() => window.__jackioh.state.result) });
  fs.writeFileSync(`${QD}/marks.json`, JSON.stringify(marks, null, 1));
  console.log(JSON.stringify(marks));
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
