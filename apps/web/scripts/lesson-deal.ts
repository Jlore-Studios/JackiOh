// Prints a tutorial lesson's deal (SPEC §9.10): both opening hands and both libraries top down, as
// the lesson's seed shuffles them, so the coach can name the cards the seed deals. Writes nothing.
//
//   sh scripts/build-wasm.sh                     # from the repository root, once: src/wasm/pkg
//   pnpm --dir apps/web exec tsx scripts/lesson-deal.ts <lessonId> [seed]
//   pnpm --dir apps/web exec tsx scripts/lesson-deal.ts <lessonId> --scan <prefix> <from> <to> [top]
//
// `--scan` prints one line per seed `<prefix><n>`: the human's opening hand and the first `top`
// (default 6) cards of its library, then the AI's, so a shell `grep` can pick a seed by its curve.
//
// Node has no fetch of a file URL, so the WebAssembly module's bytes (§10.3) are read from disk and
// instantiated before the first call, as the web's tests do (`src/test/setup.ts`).

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { beginGame, createGame, registeredCatalog, type GameState } from "@jackioh/engine";
import { AI_TUTORIAL } from "@jackioh/engine/config";
import { opponentOf, type PlayerId } from "@jackioh/shared";

import { lessonById, type TutorialLesson } from "../src/tutorial/lessons.ts";
import { loadWasmSync } from "../src/wasm/index.ts";

const wasmPath = join(dirname(fileURLToPath(import.meta.url)), "../src/wasm/pkg/jackioh_wasm_bg.wasm");
if (!existsSync(wasmPath)) {
  console.error(`lesson-deal: ${wasmPath} is missing; build it first with \`sh scripts/build-wasm.sh\` from the repository root`);
  process.exit(1);
}
loadWasmSync(readFileSync(wasmPath));
const catalog = registeredCatalog();

function name(defId: string): string {
  const def = catalog[defId];
  return def === undefined ? defId : `${def.name} (${typeof def.cost === "number" ? String(def.cost) : "X"})`;
}

function deal(lesson: TutorialLesson, seed: string): GameState {
  const aiSeat = opponentOf(lesson.humanSeat);
  const decks: [string[], string[]] =
    lesson.humanSeat === "p1" ? [[...lesson.humanDeck], [...lesson.aiDeck]] : [[...lesson.aiDeck], [...lesson.humanDeck]];
  const created = createGame({ seed, decks, handicaps: { [aiSeat]: AI_TUTORIAL } });
  return beginGame(created).state;
}

function side(state: GameState, player: PlayerId, top: number): { hand: string[]; library: string[] } {
  const p = state.players[player];
  return { hand: p.hand.map((card) => name(card.defId)), library: p.library.slice(0, top).map((card) => name(card.defId)) };
}

const [lessonId, ...rest] = process.argv.slice(2);
const lesson = lessonId === undefined ? undefined : lessonById(lessonId);
if (lesson === undefined) {
  console.error("usage: lesson-deal.ts <lessonId> [seed] | --scan <prefix> <from> <to> [top]");
  process.exit(2);
}
const human = lesson.humanSeat;
const ai = opponentOf(human);

if (rest[0] === "--scan") {
  const [, prefix = "", from = "1", to = "100", top = "6"] = rest;
  for (let n = Number(from); n <= Number(to); n += 1) {
    const seed = `${prefix}${String(n)}`;
    const state = deal(lesson, seed);
    const h = side(state, human, Number(top));
    const a = side(state, ai, Number(top));
    console.log(`${seed} | YOU ${h.hand.join(", ")} || draws ${h.library.join(", ")} | AI ${a.hand.join(", ")} || draws ${a.library.join(", ")}`);
  }
} else {
  const seed = rest[0] ?? lesson.seed;
  const state = deal(lesson, seed);
  const h = side(state, human, lesson.humanDeck.length);
  const a = side(state, ai, lesson.aiDeck.length);
  console.log(`lesson ${lesson.id}, seed ${seed}; the human is ${human}`);
  console.log(`YOU opening: ${h.hand.join(", ")}`);
  console.log(`YOU library, top down: ${h.library.join(", ")}`);
  console.log(`AI opening: ${a.hand.join(", ")}`);
  console.log(`AI library, top down: ${a.library.join(", ")}`);
}
