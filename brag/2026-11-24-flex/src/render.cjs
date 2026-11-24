// Renders the composition: `node render.cjs <l|v> all` → frames-<l|v>/0000.png …; `node render.cjs <l|v> at 0.5 3.4 …` → check/<l|v>-<t>.png.
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const FPS = 30;
(async () => {
  const [cut, mode, ...rest] = process.argv.slice(2);
  const V = cut === "v";
  const W = V ? 1080 : 1920, H = V ? 1920 : 1080, FRAMES = V ? 411 : 600;
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--hide-scrollbars", "--force-color-profile=srgb"] });
  const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  const p = await ctx.newPage();
  await p.goto(`http://127.0.0.1:8772/index.html${V ? "?v=1" : ""}`, { waitUntil: "load" });
  await p.evaluate(() => window.ready);
  const clip = { x: 0, y: 0, width: W, height: H };
  if (mode === "all") {
    const dir = `frames-${cut}`;
    fs.mkdirSync(dir, { recursive: true });
    const from = Number(rest[0] || 0), to = Number(rest[1] || FRAMES);
    const t0 = Date.now();
    for (let i = from; i < to; i++) {
      await p.evaluate((t) => window.renderAt(t), i / FPS);
      await p.screenshot({ path: `${dir}/${String(i).padStart(4, "0")}.png`, clip });
      if (i % 100 === 0) console.log("frame", i, ((Date.now() - t0) / 1000).toFixed(0) + "s");
    }
  } else {
    fs.mkdirSync("check", { recursive: true });
    for (const s of rest) {
      await p.evaluate((t) => window.renderAt(t), Number(s));
      await p.screenshot({ path: `check/${cut}-${s}.png`, clip });
    }
    console.log("ok", rest.join(" "));
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
