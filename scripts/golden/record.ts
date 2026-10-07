// Golden traces (docs/v0.3.0/SURFACE.md §13): the oracle the Rust rewrite is held to, recorded from
// the TypeScript engine while it still runs (part 23 of #306), blind to the Rust, and replayed by
// `crates/engine/tests/golden.rs` and `cargo jackioh golden check`.
//
// WHAT IS RECORDED (§13.1). Seed k in 1..=200 is dealt, seeded and played exactly as
// `packages/cards/test/fuzz.test.ts` plays fuzz seed k: one shuffle of the deck-legal pool by
// `jackioh-fuzz-decks-<k>`, the first 20 cards to p1 and the next 20 to p2, game seed
// `jackioh-fuzz-<k>`, actions from SPEC §10.7's policy (`subsystems.chooseAction`) over the stream
// `jackioh-fuzz-policy-<k>`, and, while both mulligans are open (R265), the seat that answers first
// from its own stream `jackioh-fuzz-policy-order-<k>`. Seed k in 201..=240 is played exactly as
// `packages/cards/test/fuzz-handicap.test.ts` plays its seed k: the same loop with one seat on Medium
// or Hard, rotating by seed (R180–R184), and that file's `jackioh-fuzz-handicap-…` streams. Each
// action's nonce is `golden-<n>`, n the actions applied before it. A game is recorded to its end or
// to STEP_CAP actions, whichever comes first.
//
// The deck builders and the pool are COPIES of the two test files' (`decksForSeed`, `FUZZ_POOL`,
// `POOL_EXCLUSIONS`, `handicapForSeed`, …), never imports: a test file's top-level `describe` needs the
// vitest runner. If either file changes how it deals, change the copy here with it and re-record.
//
// WHAT IS HASHED (§13.2), with `replay.ts`'s `canonical` and its FNV-1a loop (copied: neither is
// exported), every hash 8 lower-case hex digits:
//   s  `hashState(state)` after the action: the state hash itself (§5.2);
//   v  `[fnv(canonical(viewFor(state, "p1"))), fnv(canonical(viewFor(state, "p2")))]`;
//   e  `fnv(canonical(events))`, the events the action produced;
//   l  the actor's legal actions before the action, AS A SET: each action's canonical text, distinct,
//      sorted, joined by "\n". Order is not pinned (the Rust policy and AI need not list them in TS's
//      order); membership is.
// `begin` holds s, v and e after `beginGame(createGame(args))`.
//
// THE FILE (§13.3), one game per line, keys in this order:
//   {"v":1,"seed":"jackioh-fuzz-1","args":{"seed":…,"decks":[[…],[…]],"handicaps":null},
//    "begin":{"s":…,"v":[…,…],"e":…},
//    "steps":[{"a":{…the Action, playerId and nonce included…},"l":…,"s":…,"v":[…,…],"e":…}, …],
//    "end":{"winner":"p1","reason":"hero-death","steps":153}}
// A game stopped by STEP_CAP ends `{"winner":null,"reason":null,"steps":3000}`.
//
// A NUMBER THAT IS NOT AN INTEGER anywhere in a hashed value (state, view, events, legal actions)
// would make the canonical text depend on how each language prints a float, so the recorder refuses
// it and names its JSON path (the brief's risk; part 32 decides what to do with one).
//
// Determinism (SPEC §9.3, CLAUDE.md rule 4): nothing here reads `Math.random` or the clock; every
// stream is `createRng` over a seed string, so two runs write the same bytes (the brief's test:
// record twice and `cmp`).
//
// HOW TO RUN (from the repository root)
//   pnpm exec tsx scripts/golden/record.ts > crates/engine/tests/golden/games.jsonl   every game
//   pnpm exec tsx scripts/golden/record.ts --seed 17                                  seed 17's line
//   pnpm exec tsx scripts/golden/record.ts --seed 17 --dump-step 42                   TS's side of a step
//
// `--dump-step <n>` (n the 0-based index into the line's `steps`, or `begin`) writes the canonical
// texts that step's hashes are taken over into `target/golden-diff/` (or `--out <dir>`), named as
// the Rust side names its own when a replay diverges (`<seed>-<step>-<which>.json`) with `.ts` before
// the extension: `s` (the state minus `applied` and `opening`), `v-p1`, `v-p2`, `e`, and for a step
// also `l` (one canonical action per line) and `a` (the action). Diff the two files of one `which`.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import type { Action, ActionBody, GameEvent, PlayerId } from "../../packages/shared/src/index";
import {
  AI_DIFFICULTY,
  DECK_SIZE,
  beginGame,
  createGame,
  createRng,
  hashState,
  legalActions,
  mulliganOwed,
  reduce,
  seatToAct,
  subsystems,
  viewFor,
  type GameState,
  type Handicap,
} from "../../packages/engine/src/index";
import { CATALOG, registerAll } from "../../packages/cards/src/index";

// ---------------------------------------------------------------------------------------------
// The recording's shape (§13.1, §13.3)
// ---------------------------------------------------------------------------------------------

/** §13.3: the file format's version, each line's first key. */
const FORMAT_VERSION = 1;

/** §13.1: seeds PLAIN_FIRST..=PLAIN_LAST are fuzz.test.ts's games. */
const PLAIN_FIRST = 1;
const PLAIN_LAST = 200;

/** §13.1: seeds HANDICAP_FIRST..=HANDICAP_LAST are fuzz-handicap.test.ts's games. */
const HANDICAP_FIRST = 201;
const HANDICAP_LAST = 240;

/** §13.1: a game is recorded to its end or to this many actions, whichever comes first. */
const STEP_CAP = 3000;

/** §13.3: where `--dump-step` writes by default, beside the Rust side's `<seed>-<step>-<which>.json`. */
const DIFF_DIR = fileURLToPath(new URL("../../target/golden-diff/", import.meta.url));

// ---------------------------------------------------------------------------------------------
// Copied from packages/cards/test/fuzz.test.ts (§13.1: never import the test file)
// ---------------------------------------------------------------------------------------------

/**
 * Cards deliberately kept OUT of the fuzz deck pool.
 *
 * This list is the only way coverage is ever narrowed, and it must stay empty in a green tree: the
 * BUILD definition of done is "1,000 seeds with the full card pool". Anything parked here is a
 * known-unimplemented or known-broken card that hides every other bug behind its own, and each
 * entry carries the reason and the issue it is waiting on. Emptying it is the fix; adding to it to
 * make the gate green is not.
 */
export const POOL_EXCLUSIONS: readonly { id: string; why: string }[] = [];

const EXCLUDED_IDS: ReadonlySet<string> = new Set(POOL_EXCLUSIONS.map((entry) => entry.id));

/**
 * Every deck-legal card: the whole catalog minus tokens (§2.6 L3, which `validateDeck` enforces)
 * minus `POOL_EXCLUSIONS`. Sorted by catalog id so the pool a seed shuffles is identical on every
 * machine and in every process, whatever order the registry handed the defs over.
 */
export const FUZZ_POOL: readonly string[] = Object.entries(CATALOG)
  .filter(([, def]) => def.token !== true && !def.tags.includes("Token"))
  .map(([id]) => id)
  .filter((id) => !EXCLUDED_IDS.has(id))
  .sort();

/**
 * The seed drives the deck draw (§9.3): one seeded shuffle of the whole pool, the first 20 cards to
 * p1 and the next 20 to p2. One shuffle rather than two draws means the 40 cards are distinct, so
 * `validateDeck`'s "no duplicate card ids" (§2.6 L3) holds by construction for both decks, and a
 * reported seed rebuilds its exact deck pair with no other input.
 */
export function decksForSeed(seed: number): [string[], string[]] {
  const rng = createRng(`jackioh-fuzz-decks-${seed}`);
  const shuffled = rng.shuffle(FUZZ_POOL);
  return [shuffled.slice(0, DECK_SIZE), shuffled.slice(DECK_SIZE, DECK_SIZE * 2)];
}

// ---------------------------------------------------------------------------------------------
// Copied from packages/cards/test/fuzz-handicap.test.ts (§13.1)
// ---------------------------------------------------------------------------------------------

/** Every deck-legal card, sorted, as fuzz.test.ts's FUZZ_POOL (its exclusion list is empty). */
const POOL: readonly string[] = Object.entries(CATALOG)
  .filter(([, def]) => def.token !== true && !def.tags.includes("Token"))
  .map(([id]) => id)
  .sort();

/** Which seat is handicapped, and how: seeds rotate over Medium and Hard and over p1 and p2. */
export function handicapForSeed(seed: number): { seat: PlayerId; tier: "medium" | "hard"; handicap: Handicap } {
  const tier = seed % 2 === 1 ? "hard" : "medium";
  const seat: PlayerId = Math.floor(seed / 2) % 2 === 0 ? "p1" : "p2";
  return { seat, tier, handicap: AI_DIFFICULTY[tier] };
}

/**
 * One shuffle of the pool: the handicapped seat takes its deckSize cards, the other seat the next 20.
 * (fuzz-handicap.test.ts's `decksForSeed`, renamed here beside fuzz.test.ts's.)
 */
function handicapDecksForSeed(seed: number, seat: PlayerId, handicap: Handicap): [string[], string[]] {
  const shuffled = createRng(`jackioh-fuzz-handicap-decks-${seed}`).shuffle(POOL);
  const big = shuffled.slice(0, handicap.deckSize);
  const small = shuffled.slice(handicap.deckSize, handicap.deckSize + DECK_SIZE);
  return seat === "p1" ? [big, small] : [small, big];
}

// ---------------------------------------------------------------------------------------------
// One seed's game: its setup and its two policy streams
// ---------------------------------------------------------------------------------------------

type GameSpec = {
  /** The game seed `createGame` takes. */
  seed: string;
  decks: [string[], string[]];
  /** R180, R187: the handicapped seat's handicap, or null for a game on this spec's resources. */
  handicaps: Partial<Record<PlayerId, Handicap>> | null;
  /** The policy's stream (§10.7), deliberately separate from the game rng, as the fuzz has it. */
  policy: string;
  /** R265: the stream that picks which seat answers first while both mulligans are open. */
  order: string;
};

/** §13.1: seed k's game, dealt and seeded as the fuzz file that owns k deals it. */
function specForSeed(k: number): GameSpec {
  if (k >= HANDICAP_FIRST && k <= HANDICAP_LAST) {
    const { seat, handicap } = handicapForSeed(k);
    return {
      seed: `jackioh-fuzz-handicap-${k}`,
      decks: handicapDecksForSeed(k, seat, handicap),
      handicaps: { [seat]: handicap },
      policy: `jackioh-fuzz-handicap-policy-${k}`,
      order: `jackioh-fuzz-handicap-policy-order-${k}`,
    };
  }
  if (k >= PLAIN_FIRST && k <= PLAIN_LAST) {
    return {
      seed: `jackioh-fuzz-${k}`,
      decks: decksForSeed(k),
      handicaps: null,
      policy: `jackioh-fuzz-policy-${k}`,
      order: `jackioh-fuzz-policy-order-${k}`,
    };
  }
  throw new Error(
    `seed ${k} is not a golden seed: ${PLAIN_FIRST}–${PLAIN_LAST} are fuzz.test.ts's games and ` +
      `${HANDICAP_FIRST}–${HANDICAP_LAST} fuzz-handicap.test.ts's (SURFACE §13.1)`,
  );
}

/** Every seed the file holds, in file order. */
function goldenSeeds(): number[] {
  const seeds: number[] = [];
  for (let k = PLAIN_FIRST; k <= PLAIN_LAST; k += 1) seeds.push(k);
  for (let k = HANDICAP_FIRST; k <= HANDICAP_LAST; k += 1) seeds.push(k);
  return seeds;
}

// ---------------------------------------------------------------------------------------------
// Hashing (§13.2, with §5.2's functions)
// ---------------------------------------------------------------------------------------------

/**
 * `packages/engine/src/replay.ts`'s `canonical`, copied (it is not exported).
 *
 * Canonical JSON: keys sorted, so two equal states always produce the same text.
 */
function canonical(value: unknown): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value) ?? "null";
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  const entries = Object.entries(value as Record<string, unknown>)
    .filter(([, v]) => v !== undefined)
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    .map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`);
  return `{${entries.join(",")}}`;
}

/** `hashState`'s FNV-1a loop over the UTF-16 code units of `text`, as 8 lower-case hex digits (§5.2 step 3). */
function fnv(text: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i += 1) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

/** §5.2 step 1: the state as `hashState` hashes it, without the nonce log and the opening (R676). */
function hashedState(state: GameState): Omit<GameState, "applied" | "opening"> {
  const { applied: _applied, opening: _opening, ...rest } = state;
  return rest;
}

/**
 * The brief's risk: refuse a number that is not an integer, naming its JSON path, because JS and
 * serde_json print floats differently (`1` against `1.0`, exponents) and the hash would then depend
 * on the printer rather than the rules.
 */
function assertIntegers(value: unknown, path: string): void {
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) {
      throw new Error(
        `non-integer number ${String(value)} at ${path}: the canonical text would depend on float ` +
          "printing (docs/v0.3.0/parts/23-golden-traces-recorded-from-the-typescript-engine.md, Risks)",
      );
    }
    return;
  }
  if (value === null || typeof value !== "object") return;
  if (Array.isArray(value)) {
    value.forEach((item: unknown, index) => assertIntegers(item, `${path}[${index}]`));
    return;
  }
  for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
    if (item !== undefined) assertIntegers(item, `${path}.${key}`);
  }
}

/** The canonical texts one snapshot's hashes are taken over: the state, both views, the events. */
type Texts = { s: string; v: [string, string]; e: string };

function textsOf(state: GameState, events: readonly GameEvent[], where: string): Texts {
  const rest = hashedState(state);
  const p1 = viewFor(state, "p1");
  const p2 = viewFor(state, "p2");
  assertIntegers(rest, `${where}: state $`);
  assertIntegers(p1, `${where}: viewFor(p1) $`);
  assertIntegers(p2, `${where}: viewFor(p2) $`);
  assertIntegers(events, `${where}: events $`);
  return { s: canonical(rest), v: [canonical(p1), canonical(p2)], e: canonical(events) };
}

/** §13.2's `l`: the legal actions as a set, each action's canonical text, distinct, sorted, one per line. */
function legalText(state: GameState, actor: PlayerId, where: string): string {
  const legal: ActionBody[] = legalActions(state, actor);
  assertIntegers(legal, `${where}: legalActions(${actor}) $`);
  return [...new Set(legal.map((action) => canonical(action)))].sort().join("\n");
}

type Hashes = { s: string; v: [string, string]; e: string };

/** The hashes of a snapshot, keys in §13.3's order. */
function hashesOf(texts: Texts): Hashes {
  return { s: fnv(texts.s), v: [fnv(texts.v[0]), fnv(texts.v[1])], e: fnv(texts.e) };
}

// ---------------------------------------------------------------------------------------------
// Playing and recording one game
// ---------------------------------------------------------------------------------------------

type StepLine = { a: Action; l: string; s: string; v: [string, string]; e: string };

type GameLine = {
  v: number;
  seed: string;
  args: { seed: string; decks: [string[], string[]]; handicaps: Partial<Record<PlayerId, Handicap>> | null };
  begin: Hashes;
  steps: StepLine[];
  end: { winner: string | null; reason: string | null; steps: number };
};

/** `--dump-step`'s target: an index into the line's `steps`, or the snapshot after `beginGame`. */
type DumpAt = number | "begin";

/** One step's canonical texts, for `--dump-step`. `a` and `l` are absent at `begin`. */
type StepTexts = Texts & { a?: Action; l?: string };

/**
 * Plays seed k's game as its fuzz file does (fuzz.test.ts's `playGame`, minus the invariant monitor,
 * which only reads) and records it. With `dumpAt`, stops once that step is taken and returns its
 * canonical texts as well.
 */
function recordSeed(k: number, dumpAt: DumpAt | null = null): { line: GameLine; dump: StepTexts | null } {
  const spec = specForSeed(k);
  registerAll();

  const begun = beginGame(
    createGame(spec.handicaps === null
      ? { seed: spec.seed, decks: spec.decks }
      : { seed: spec.seed, decks: spec.decks, handicaps: spec.handicaps }),
  );
  let state = begun.state;
  const beginTexts = textsOf(state, begun.events, `${spec.seed} begin`);
  const begin = hashesOf(beginTexts);
  // The copied `canonical` and FNV loop must be `hashState`'s, or every `s` below would be wrong.
  if (begin.s !== hashState(state)) {
    throw new Error(`${spec.seed}: the copied canonical/FNV hashed ${begin.s}, hashState ${hashState(state)}`);
  }

  const line: GameLine = {
    v: FORMAT_VERSION,
    seed: spec.seed,
    args: { seed: spec.seed, decks: spec.decks, handicaps: spec.handicaps },
    begin,
    steps: [],
    end: { winner: null, reason: null, steps: 0 },
  };
  if (dumpAt === "begin") return { line, dump: beginTexts };

  const policy = createRng(spec.policy);
  // R265: while both mulligans are open either seat may answer first; a stream of its own picks
  // which, so the fuzz plays both orders (the game is the same either way, R265).
  const order = createRng(spec.order);

  while (state.result === null && line.steps.length < STEP_CAP) {
    const n = line.steps.length;
    const where = `${spec.seed} step ${n}`;
    // With a prompt open only its holder may act (§9.3); otherwise it is the active player's turn.
    const player: PlayerId = mulliganOwed(state).length === 2 && order.coin() ? "p2" : seatToAct(state);
    const l = legalText(state, player, where);
    const chosen: ActionBody | null = subsystems.chooseAction(state, player, policy);
    if (chosen === null) {
      throw new Error(
        `${where}: no legal action for ${player} while the game is live (turn ${state.turn}, ` +
          `phase "${state.phase}"): R82 auto-ends a turn with nothing left to do, so this is a stall`,
      );
    }
    const action = { ...chosen, playerId: player, nonce: `golden-${n}` } as Action;
    const result = reduce(state, action);
    if (result.error !== undefined) {
      throw new Error(`${where}: legalActions offered "${action.type}" but reduce refused it: ${result.error}`);
    }
    state = result.state;
    const texts = textsOf(state, result.events, where);
    line.steps.push({ a: action, l: fnv(l), ...hashesOf(texts) });
    if (dumpAt === n) return { line, dump: { ...texts, a: action, l } };
  }

  const last = line.steps[line.steps.length - 1];
  if (last !== undefined && last.s !== hashState(state)) {
    throw new Error(`${spec.seed}: the copied canonical/FNV hashed ${last.s}, hashState ${hashState(state)}`);
  }
  line.end = {
    winner: state.result?.winner ?? null,
    reason: state.result?.reason ?? null,
    steps: line.steps.length,
  };
  if (dumpAt !== null) {
    throw new Error(`${spec.seed} has ${line.steps.length} steps (0–${line.steps.length - 1}): no step ${dumpAt}`);
  }
  return { line, dump: null };
}

// ---------------------------------------------------------------------------------------------
// --dump-step: TS's side of one step, next to the Rust side's diff
// ---------------------------------------------------------------------------------------------

function writeDump(seed: string, step: DumpAt, dump: StepTexts, dir: string): void {
  mkdirSync(dir, { recursive: true });
  const files: [string, string][] = [
    ["s", dump.s],
    ["v-p1", dump.v[0]],
    ["v-p2", dump.v[1]],
    ["e", dump.e],
  ];
  if (dump.l !== undefined) files.push(["l", dump.l]);
  if (dump.a !== undefined) files.push(["a", canonical(dump.a)]);
  for (const [which, text] of files) {
    const path = join(dir, `${seed}-${step}-${which}.ts.json`);
    writeFileSync(path, `${text}\n`);
    process.stderr.write(`${which.padEnd(4)} ${fnv(text)}  ${path}\n`);
  }
}

// ---------------------------------------------------------------------------------------------
// The command line
// ---------------------------------------------------------------------------------------------

function parseSeed(raw: string): number {
  const k = Number.parseInt(raw, 10);
  if (!Number.isSafeInteger(k) || String(k) !== raw.trim()) throw new Error(`--seed takes a seed number, not "${raw}"`);
  return k;
}

function parseStep(raw: string): DumpAt {
  if (raw === "begin") return "begin";
  const n = Number.parseInt(raw, 10);
  if (!Number.isSafeInteger(n) || n < 0 || String(n) !== raw.trim()) {
    throw new Error(`--dump-step takes a 0-based step index or "begin", not "${raw}"`);
  }
  return n;
}

function main(argv: readonly string[]): void {
  const { values } = parseArgs({
    args: [...argv],
    options: {
      seed: { type: "string" },
      "dump-step": { type: "string" },
      out: { type: "string" },
    },
    strict: true,
    allowPositionals: false,
  });

  if (values.seed === undefined) {
    if (values["dump-step"] !== undefined) throw new Error("--dump-step needs --seed <k>");
    let steps = 0;
    for (const k of goldenSeeds()) {
      const { line } = recordSeed(k);
      steps += line.steps.length;
      process.stdout.write(`${JSON.stringify(line)}\n`);
    }
    process.stderr.write(`[golden] ${goldenSeeds().length} games, ${steps} steps\n`);
    return;
  }

  const k = parseSeed(values.seed);
  if (values["dump-step"] === undefined) {
    process.stdout.write(`${JSON.stringify(recordSeed(k).line)}\n`);
    return;
  }

  const step = parseStep(values["dump-step"]);
  const { line, dump } = recordSeed(k, step);
  if (dump === null) throw new Error(`${line.seed}: no step ${step}`);
  writeDump(line.seed, step, dump, values.out ?? DIFF_DIR);
}

main(process.argv.slice(2));
