// Plays seed 28 to turn 3's attack on a page: `await setup(p, shot)` where shot(name) takes a still.
const fs = require("fs");
const path = require("path");
const inject = fs.readFileSync(path.join(__dirname, "js/inject.js"), "utf8");
const lib = fs.readFileSync(path.join(__dirname, "js/lib.js"), "utf8");
const SEED = 28;
async function setup(p, shot, upTo = "attack") {
  await p.goto("http://localhost:5173/");
  await p.evaluate(`(${inject.replace("SEED", String(SEED))})()`);
  await p.goto(`http://localhost:5173/dev/hotseat?seed=${SEED}&a=brag-a&b=brag-b`);
  await p.waitForFunction(() => !!window.__jackioh, null, { timeout: 30000 });
  await p.evaluate(`(${lib})()`);
  const ev = (code) => p.evaluate(code);
  await ev("__brag.ready()");
  await ev("__brag.keep()");
  await ev("__brag.settle()");
  // Turn 1: p1 plays three units and does not attack.
  await ev(`__brag.play("p1", "meditative-100", "units", 2)`); await ev("__brag.settle()");
  await ev(`__brag.play("p1", "core-008", "units", 3)`); await ev("__brag.settle()");
  await ev(`__brag.play("p1", "traininggrounds-003", "units", 4)`); await ev("__brag.settle()");
  if (shot) await shot("board-t1");
  await ev(`__brag.endTurn("p1")`); await ev("__brag.settle()");
  // Turn 2: p2 plays the coin and two units, then sets My Pawn and Sheepish.
  if (upTo === "t2legal") return ev(`__brag.legal("p2")`);
  await ev(`__brag.play("p2", "core-t-coin")`); await ev("__brag.settle()");
  await ev(`__brag.play("p2", "meditative-002", "units", 3)`); await ev("__brag.settle()");
  await ev(`__brag.play("p2", "stinkyscapeplus-002", "units", 2)`); await ev("__brag.settle()");
  await ev(`__brag.play("p2", "core-096", "backrow", 3)`); await ev("__brag.settle()");
  await ev(`__brag.play("p2", "core-041", "backrow", 4)`); await ev("__brag.settle()");
  await ev(`window.__jackioh.setSeat("p1")`); await ev("__brag.settle()");
  if (shot) await shot("board-t2");
  await ev(`__brag.endTurn("p2")`);
  await ev(`window.__jackioh.setSeat("p1")`); await ev("__brag.settle()");
  if (shot) await shot("board-t3");
  return ev(`__brag.legal("p1").filter(a => a.type !== "play")`);
}
module.exports = { setup, SEED };
