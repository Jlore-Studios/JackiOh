// Renders the composition (served on 127.0.0.1:8773): `node render.cjs land|port all [from to]` → frames-<mode>/NNNN.png
// (t = i / 30); `node render.cjs land|port at 0.5 3.4 …` → check/<mode>-<t>.png.
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const FPS = 30;
(async () => {
  const [mode, what, ...rest] = process.argv.slice(2);
  const V = mode === "port", W = V ? 1080 : 1920, H = V ? 1920 : 1080, FRAMES = Math.round((V ? 14.4 : 19.2) * FPS);
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--hide-scrollbars", "--force-color-profile=srgb"] });
  const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  const p = await ctx.newPage();
  await p.goto(`http://127.0.0.1:8773/index.html${V ? "?v=1" : ""}`, { waitUntil: "load" });
  await p.evaluate(() => window.ready);
  const clip = { x: 0, y: 0, width: W, height: H };
  if (what === "all") {
    const dir = `frames-${mode}`; fs.mkdirSync(dir, { recursive: true });
    const from = Number(rest[0] || 0), to = Number(rest[1] || FRAMES);
    const t0 = Date.now();
    for (let i = from; i < to; i++) {
      await p.evaluate((t) => window.renderAt(t), i / FPS);
      await p.screenshot({ path: `${dir}/${String(i).padStart(4, "0")}.png`, clip });
      if (i % 100 === 0) console.log("frame", i, ((Date.now() - t0) / 1000).toFixed(0) + "s");
    }
    console.log("done", to - from);
  } else {
    fs.mkdirSync("check", { recursive: true });
    for (const s of rest) { await p.evaluate((t) => window.renderAt(t), Number(s)); await p.screenshot({ path: `check/${mode}-${s}.png`, clip }); }
    console.log("ok", rest.join(" "));
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
