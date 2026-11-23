const fs = require("fs");
const { page } = require("./cdp.cjs");
(async () => {
  const { browser, p } = await page();
  const inject = fs.readFileSync("js/inject.js", "utf8");
  const NAMES = { "core-008": "Vanilla", "meditative-100": "Greaser", "traininggrounds-032": "Selfless", "traininggrounds-008": "Clobberer", "core-045": "Deft", "traininggrounds-003": "GroupLdr", "core-096": "PAWN", "core-041": "SHEEP", "meditative-002": "Rhino", "stinkyscapeplus-002": "Drifter", "meditative-073": "Plate", "meditativeplus-008": "ArchMage", "core-t-coin": "coin" };
  for (let seed = Number(process.argv[2] || 1); seed <= Number(process.argv[3] || 30); seed++) {
    await p.goto("http://localhost:5173/");
    await p.evaluate(`(${inject.replace("SEED", String(seed))})()`);
    await p.goto(`http://localhost:5173/dev/hotseat?seed=${seed}&a=brag-a&b=brag-b`);
    const r = await p.evaluate(async (NAMES) => {
      const until = Date.now() + 15000;
      while (!window.__jackioh && Date.now() < until) await new Promise((r) => setTimeout(r, 50));
      const j = window.__jackioh;
      for (const seat of ["p1", "p2"]) { j.setSeat(seat); const keep = j.legal().filter((a) => a.type === "mulligan").sort((a, b) => b.keep.length - a.keep.length)[0]; j.dispatch({ ...keep, playerId: seat }); }
      const n = (h) => h.map((c) => NAMES[c.defId] ?? c.defId).join(",");
      const t1 = n(j.state.players.p1.hand);
      j.setSeat("p1"); j.dispatch({ type: "endTurn", playerId: "p1" });
      return { t1, t2: n(j.state.players.p2.hand), turn: j.state.turn };
    }, NAMES);
    console.log(seed, "| p1:", r.t1, "| p2:", r.t2, "| turn", r.turn);
  }
  await browser.close();
})();
