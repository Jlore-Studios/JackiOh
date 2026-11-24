// Seed scan: plays each seed to the end with the greedy policy (no waiting on animations) and prints the result.
// `node scan.cjs probe <seed>` prints the attack actions' shape and a unit's keys instead.
const fs = require("fs"), path = require("path");
const { chromium } = require(path.resolve(__dirname, "../../.cache/playwright-cli/0.1.22/node_modules/playwright-core"));
const { A, B } = require("./decks.cjs");
const lib = fs.readFileSync("js/lib.js", "utf8"), policy = fs.readFileSync("js/policy.js", "utf8");
const inj = (seed) => fs.readFileSync("js/inject.js", "utf8").replace("SEED", String(seed)).replace("DECK_A", JSON.stringify(A)).replace("DECK_B", JSON.stringify(B));
(async () => {
  const [mode, a1, a2] = process.argv.slice(2);
  const browser = await chromium.launch({ channel: "chrome", headless: true, args: ["--mute-audio"] });
  const p = await (await browser.newContext({ viewport: { width: 1280, height: 720 } })).newPage();
  const seeds = mode === "probe" || mode === "detail" ? [Number(a1)] : Array.from({ length: Number(a2) - Number(a1) + 1 }, (_, i) => Number(a1) + i);
  for (const seed of seeds) {
    await p.goto("http://localhost:5173/");
    await p.evaluate(`(${inj(seed)})()`);
    await p.goto(`http://localhost:5173/dev/hotseat?seed=${seed}&a=fast-a&b=fast-b`);
    await p.waitForFunction(() => !!window.__jackioh, null, { timeout: 30000 });
    await p.evaluate(`(${lib})()`); await p.evaluate(`(${policy})()`);
    await p.evaluate("__brag.keep()");
    const out = await p.evaluate((probe) => {
      const log = []; let n = 0, sample = null;
      while (n++ < 400) {
        const s = window.__jackioh.state;
        if (probe && !sample) {
          const atk = window.__jackioh.legal().filter((a) => a.type === "attack");
          if (atk.length) { const u = s.players[s.active].units.find(Boolean); sample = { atk: atk.slice(0, 4), unit: u, other: [...new Set(window.__jackioh.legal().map((a) => a.type))] }; }
        }
        const x = window.__fast.next();
        window.__lastHand = {}; for (const q of ["p1", "p2"]) for (const c of window.__jackioh.state.players[q].hand) window.__lastHand[c.id] = c.defId;
        if (!x) break;
        if (!x.a) return { err: "no action", log: log.slice(-5) };
        const r = window.__jackioh.dispatch({ ...x.a, playerId: x.seat });
        const S = window.__jackioh.state, def = (id) => { const u = window.__fast.unitOf(id); return u ? u.defId : id; };
        const card = x.a.instanceId ? (window.__lastHand || {})[x.a.instanceId] : null;
        log.push([S.turn, x.seat, x.kind, x.kind === "play" ? card + "@" + (x.a.zone ? x.a.zone.row[0] + x.a.zone.lane : "-") : x.a.attackerId ? def(x.a.attackerId) + ">" + def(x.a.targetId) : "", S.players.p1.hero.health, S.players.p2.hero.health].join(" "));
        if (probe && sample && n > 60) break;
      }
      const s = window.__jackioh.state;
      const traps = (s.log || []).filter((e) => JSON.stringify(e).includes("reveal")).length;
      return { sample, result: s.result, turn: s.turn, actions: n, h: [s.players.p1.hero.health, s.players.p2.hero.health], traps, log: probe === "detail" ? log : log.length };
    }, mode === "detail" ? "detail" : mode === "probe");
    console.log(seed, JSON.stringify(out).slice(0, mode === "scan" ? 300 : 6000));
  }
  await browser.close();
})().catch((e) => { console.error(e.stack.slice(0, 1500)); process.exit(1); });
