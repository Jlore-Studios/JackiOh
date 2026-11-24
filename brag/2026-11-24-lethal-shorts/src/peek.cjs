// Plays seed 28 at a portrait viewport and takes stills of each turn and the aim: `node peek.cjs`.
const path = require("path");
const fs = require("fs");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const { setup } = require("./setup.cjs");
(async () => {
  fs.mkdirSync("peek", { recursive: true });
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio", "--hide-scrollbars"] });
  const ctx = await browser.newContext({ viewport: { width: 540, height: 960 }, deviceScaleFactor: 2 });
  await ctx.addInitScript(() => {
    const add = () => { const s = document.createElement("style"); s.textContent = ".hotseat-bar{display:none !important}"; document.head.appendChild(s); };
    if (document.head) add(); else document.addEventListener("DOMContentLoaded", add);
  });
  const p = await ctx.newPage();
  const shot = async (n) => { const st = await p.evaluate("__brag.settle()"); await p.screenshot({ path: `peek/${n}.png` }); console.log("still", n, st); };
  const legal = await setup(p, shot);
  console.log("legal", JSON.stringify(legal).slice(0, 400));
  for (const id of ["card-c3", "hero-opponent"]) {
    const b = await p.locator(`[data-testid="${id}"]`).first().boundingBox().catch(() => null);
    console.log(id, JSON.stringify(b));
  }
  const ids = await p.evaluate(() => [...document.querySelectorAll("[data-testid]")].map((e) => e.dataset.testid).filter((x) => /card-c|hero|hand|lane|slot/.test(x)).slice(0, 60));
  console.log(ids.join(" "));
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
