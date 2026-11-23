// Renders the composition: `node render.cjs all` → frames/0000.png … 0599.png (t = i / 30);
// `node render.cjs at 0.5 3.4 …` → check/<t>.png.
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const FPS = 30, FRAMES = 600;
(async () => {
  const [mode, ...rest] = process.argv.slice(2);
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--hide-scrollbars", "--force-color-profile=srgb"] });
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  const p = await ctx.newPage();
  await p.goto("http://127.0.0.1:8765/index.html", { waitUntil: "load" });
  await p.evaluate(() => window.ready);
  const clip = { x: 0, y: 0, width: 1920, height: 1080 };
  if (mode === "all") {
    fs.mkdirSync("frames", { recursive: true });
    const from = Number(rest[0] || 0), to = Number(rest[1] || FRAMES);
    const t0 = Date.now();
    for (let i = from; i < to; i++) {
      await p.evaluate((t) => window.renderAt(t), i / FPS);
      await p.screenshot({ path: `frames/${String(i).padStart(4, "0")}.png`, clip });
      if (i % 50 === 0) console.log("frame", i, ((Date.now() - t0) / 1000).toFixed(0) + "s");
    }
  } else {
    fs.mkdirSync("check", { recursive: true });
    for (const s of rest) {
      const t = Number(s);
      await p.evaluate((t) => window.renderAt(t), t);
      await p.screenshot({ path: `check/${s}.png`, clip });
    }
    console.log("ok", rest.join(" "));
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
