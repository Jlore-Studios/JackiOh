// render.cjs <land|vert> all [from to] → frames-<fmt>/NNNN.png; render.cjs <fmt> at <t>… → check/<fmt>-<t>.png.
// The composition is served at http://127.0.0.1:8771 (cd comp && python3 -m http.server 8771 --bind 127.0.0.1).
const fs = require("fs"), path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const FMT = { land: { w: 1920, h: 1080, frames: 628, page: "land.html" }, vert: { w: 1080, h: 1920, frames: 419, page: "vert.html" } };
(async () => {
  const [fmt, mode, ...rest] = process.argv.slice(2);
  const F = FMT[fmt];
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--hide-scrollbars", "--force-color-profile=srgb"] });
  const p = await (await browser.newContext({ viewport: { width: F.w, height: F.h }, deviceScaleFactor: 1 })).newPage();
  await p.goto(`http://127.0.0.1:8771/${F.page}${process.env.BANDS ? "#bands" : ""}`, { waitUntil: "load" });
  await p.evaluate(() => window.ready);
  const clip = { x: 0, y: 0, width: F.w, height: F.h };
  if (mode === "all") {
    const dir = `frames-${fmt}`; fs.mkdirSync(dir, { recursive: true });
    const from = Number(rest[0] || 0), to = Number(rest[1] || F.frames), t0 = Date.now();
    for (let i = from; i < to; i++) {
      await p.evaluate((t) => window.renderAt(t), i / 30);
      await p.screenshot({ path: `${dir}/${String(i).padStart(4, "0")}.png`, clip });
      if (i % 100 === 0) console.log("frame", i, ((Date.now() - t0) / 1000).toFixed(0) + "s");
    }
  } else {
    fs.mkdirSync("check", { recursive: true });
    for (const s of rest) { await p.evaluate((t) => window.renderAt(t), Number(s)); await p.screenshot({ path: `check/${fmt}-${s}.png`, clip }); }
    console.log("ok", rest.length);
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
