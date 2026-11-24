// Renders the vertical composition (served at http://127.0.0.1:8774) at 1080x1920.
// `node render.cjs <cut> all <dir> [from] [to]` → <dir>/0000.jpg … (t = i / 30, JPEG q95);
// `node render.cjs <cut> at [--bands] 0.5 3.4 …` → check/<cut>-<t>.png.
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const FPS = 30, FRAMES = 420;
(async () => {
  let [cut, mode, ...rest] = process.argv.slice(2);
  const bands = rest[0] === "--bands" ? (rest.shift(), 1) : 0;
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--hide-scrollbars", "--force-color-profile=srgb"] });
  const ctx = await browser.newContext({ viewport: { width: 1080, height: 1920 }, deviceScaleFactor: 1 });
  const p = await ctx.newPage();
  await p.goto(`http://127.0.0.1:8774/index.html?cut=${cut}${bands ? "&bands=1" : ""}`, { waitUntil: "load" });
  await p.evaluate(() => window.ready);
  const clip = { x: 0, y: 0, width: 1080, height: 1920 };
  if (mode === "all") {
    const dir = rest[0];
    fs.mkdirSync(dir, { recursive: true });
    const from = Number(rest[1] || 0), to = Number(rest[2] || FRAMES);
    const t0 = Date.now();
    for (let i = from; i < to; i++) {
      await p.evaluate((t) => window.renderAt(t), i / FPS);
      await p.screenshot({ path: `${dir}/${String(i).padStart(4, "0")}.jpg`, clip, type: "jpeg", quality: 95 });
      if (i % 50 === 0) console.log("frame", i, ((Date.now() - t0) / 1000).toFixed(0) + "s");
    }
  } else {
    fs.mkdirSync("check", { recursive: true });
    for (const s of rest) {
      await p.evaluate((t) => window.renderAt(t), Number(s));
      await p.screenshot({ path: `check/${cut}-${s}.png`, clip });
    }
    console.log("ok", cut, rest.join(" "));
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
