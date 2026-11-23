// Element shots of card faces from /almanac?card=<id> at deviceScaleFactor 2: faces/<id>-base.png, -radiant.png.
const fs = require("fs");
const path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const [want, ...ids] = process.argv.slice(2); // want: base | radiant | both
(async () => {
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio"] });
  const ctx = await browser.newContext({ viewport: { width: 1600, height: 1000 }, deviceScaleFactor: 2 });
  const p = await ctx.newPage();
  for (const id of ids) {
    await p.goto(`http://localhost:5173/almanac?card=${id}`, { waitUntil: "load" });
    for (const side of want === "both" ? ["base", "radiant"] : [want]) {
      const loc = p.getByTestId(`inspect-face-${side}`).first();
      try {
        await loc.waitFor({ state: "visible", timeout: 20000 });
      } catch {
        console.log(id, side, "MISSING");
        continue;
      }
      await p.evaluate(() => document.fonts.ready);
      await p.waitForTimeout(900);
      await loc.screenshot({ path: `faces/${id}-${side}.png`, animations: "disabled" });
      const box = await loc.boundingBox();
      console.log(id, side, Math.round(box.width), "x", Math.round(box.height));
    }
  }
  await p.screenshot({ path: "x-almanac.png" });
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
