// The real components, from the served client: every crest's markup (RankCrest.tsx) to comp/assets/crests.json,
// and the Jlorious card back (CardBack.tsx, back="card-back-jlorious") sampled over its 4 s breathe to back/*.png.
const fs = require("fs");
const { open, MOUNT } = require("./pw.cjs");
(async () => {
  const { browser, p } = await open({ width: 1920, height: 1080, dsf: 2 });
  await p.goto("http://localhost:5173/", { waitUntil: "networkidle" });
  const tiers = ["rotten", "normal", "large", "golden", "mythic"];
  const list = [];
  for (let n = 0; n <= 5; n++) list.push({ key: `raisin-${n}`, props: { rank: { tier: "raisin", placementsPlayed: n, placementGames: 5 }, size: "large" } });
  for (const tier of tiers) for (const d of [3, 2, 1]) for (const pips of [0, 1, 2, 3])
    list.push({ key: `${tier}-${d}-${pips}`, props: { rank: { tier, division: d, pips, pipsPerDivision: 3 }, size: "large" } });
  for (const pos of [1, 100]) list.push({ key: `jlorious-${pos}`, props: { rank: { tier: "jlorious", position: pos }, size: "large" } });
  const r = await p.evaluate(MOUNT, { src: "/src/rank/RankCrest.tsx", name: "default", propsList: list.map((x) => x.props) });
  if (!r.ok) throw new Error(JSON.stringify(r));
  const html = await p.evaluate(() => [...document.querySelectorAll("#brag-layer .brag-item")].map((e) => e.innerHTML));
  const out = {};
  list.forEach((x, i) => (out[x.key] = html[i]));
  fs.writeFileSync("comp/assets/crests.json", JSON.stringify(out));
  console.log("crests", Object.keys(out).length);

  // The card back, 400x560 CSS px at DSF 2.
  const r2 = await p.evaluate(MOUNT, { src: "/src/cards/CardBack.tsx", name: "CardBack",
    propsList: [{ back: "card-back-jlorious", __style: { position: "relative", width: "400px", height: "560px", borderRadius: "22px", overflow: "hidden" } }] });
  if (!r2.ok) throw new Error(JSON.stringify(r2));
  // React passes __style through as a prop the component ignores; size the wrapper instead.
  await p.evaluate(() => { const w = document.querySelector("#brag-layer .brag-item"); Object.assign(w.style, { position: "relative", width: "400px", height: "560px", borderRadius: "22px", overflow: "hidden" }); });
  await p.waitForTimeout(500);
  const anims = await p.evaluate(() => document.getAnimations().filter((a) => a.effect?.target?.closest?.("#brag-layer")).map((a) => a.animationName + ":" + a.effect.getTiming().duration));
  console.log("anims", JSON.stringify(anims));
  const el = p.locator("#brag-layer .brag-item");
  const N = 120; // 4 s at 30 fps, one breathe
  for (let i = 0; i < N; i++) {
    await p.evaluate((ms) => { for (const a of document.getAnimations()) if (a.effect?.target?.closest?.("#brag-layer")) { a.pause(); a.currentTime = ms; } }, (i / 30) * 1000);
    await el.screenshot({ path: `back/${String(i).padStart(3, "0")}.png`, omitBackground: true });
  }
  console.log("back frames", N);
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
