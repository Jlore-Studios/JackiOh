// Test-only cards for the turn systems of patch v0.2.0 (docs/classic-sets.md B5): ending a turn from
// an effect (E10: Classic+ #26 Tommy Tempo, the AI card Rate Limit), draw limits and draw counts (E3,
// E4: Classic #4 Palantir, #49 Anti-Greed Machine, #9 Income Tax), the cast-on-draw enchantment (E39,
// Classic+ #40), the new delayed kinds (E27: Classic #20 The Power to Punish, #37 Last Hurrah) and the
// rest-of-game effect (E28: Classic+ #52), plus the start-of-turn and cleanup stages the turn loop runs
// for Brittle and Animated (B3.3, B3.1). Each reproduces one shape through the engine's own verbs; the
// engine never imports `packages/cards`.
//
// Ids are prefixed `tn-` and indexed from 4300, so they cannot collide with another fixture file's.

import type { CardDef, CardDefs, GameEvent, Keyword } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import {
  delay,
  destroyAtNextTurnStart,
  discardHandAtTurnEnd,
  draw,
  endTurn,
  endTurnAfterActions,
  forRestOfGame,
} from "../../src/effects";
import { openPrompt, resumeSelf } from "../../src/prompts";
import type { CardScripts, DrawLimit, Effect, EffectContext, Script } from "../../src/script";
import type { GameState } from "../../src/state";

let nextIndex = 4300;

function def(
  name: string,
  type: CardDef["type"],
  extra: Partial<CardDef> & { attack?: number; health?: number; keywords?: Keyword[] } = {},
): CardDef {
  nextIndex += 1;
  const { attack = 2, health = 2, keywords = [], ...rest } = extra;
  const face = type === "Unit" ? { attack, health, keywords, text: name } : { keywords, text: name };
  const radiantFace = type === "Unit" ? { ...face, attack: attack * 2, health: health * 2 } : { ...face };
  return {
    id: `tn-${name}`,
    index: String(nextIndex),
    name: `${name} (turn)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 0,
    base: face,
    radiant: radiantFace,
    ...rest,
  };
}

// ---------------------------------------------------------------------------
// The note log: a Field Spell in p2's backrow lane 5 whose memory records what ran, in order.
// ---------------------------------------------------------------------------

export const logCard = def("log", "Field Spell");
export const LOG_LANE = 5;

function logOf(state: GameState): { memory: Record<string, unknown> } | null {
  return state.players.p2.backrow[LOG_LANE - 1] ?? null;
}

export function notes(state: GameState): string[] {
  const log = logOf(state);
  return Array.isArray(log?.memory.steps) ? (log.memory.steps as string[]) : [];
}

/** Append to the log directly, for a test double standing in for another module's body. */
export function write(state: GameState, entry: string): void {
  const log = logOf(state);
  if (log === null) return;
  const steps = Array.isArray(log.memory.steps) ? (log.memory.steps as string[]) : [];
  log.memory.steps = [...steps, entry];
}

export function note(entry: string): Effect {
  return {
    kind: "tn:note",
    apply(ctx): void {
      write(ctx.state, entry);
    },
  };
}

/** §10.6: a prompt for the controller with one answer, whose answer re-enters `step`. */
export function askController(step: string): Effect {
  return {
    kind: "tn:ask",
    apply(ctx): void {
      openPrompt(ctx, {
        player: ctx.controller,
        kind: "target",
        prompt: "the card asks its controller",
        options: [{ key: "none", label: "nothing", selection: { pick: "none" } }],
        resume: resumeSelf(ctx, step),
      });
    },
  };
}

function faces(base: Script, radiant: Script = base): CardScripts {
  return { base, radiant };
}

// ---------------------------------------------------------------------------
// E10: ending the turn from an effect
// ---------------------------------------------------------------------------

/** "End your turn", then more of the same list: the rest resolves first (R456). */
export const cutter = def("cutter", "Spell");
/** "End your turn", then a question: the turn waits for the answer (R456). */
export const cutAsker = def("cut-asker", "Spell", { tags: ["Quickdraw"] });
/** Classic+ #26's base: a Unit with Cast on draw: End your turn. Radiant: one more action. */
export const tempo = def("tempo", "Unit", { attack: 9, health: 9, keywords: [{ kind: "Taunt" }] });
/** "You may take N more actions, then your turn ends" as a Spell, for the count's own tests. */
export const oneMore = def("one-more", "Spell");
export const twoMore = def("two-more", "Spell");
/** The AI card Rate Limit's shape: the opponent's play sets it off, and their turn ends after it. */
export const rateLimit = def("rate-limit", "Trap");
/** A plain Spell that notes its own resolution. */
export const marker = def("marker", "Spell");
/** A Spell that asks its caster, and notes the answer. */
export const questioner = def("questioner", "Spell");

// ---------------------------------------------------------------------------
// E3, E4, E39: draws
// ---------------------------------------------------------------------------

/** Classic #4's aura: "Your opponent can't draw more than 1 card each turn." */
export const palantir = def("palantir", "Field Spell", { tags: ["Quickdraw"] });
/** Classic #49's aura, loosened to 2 so the lowest-holds rule shows: "Players can't draw more than 2". */
export const antiGreed = def("anti-greed", "Unit", { attack: 9, health: 9 });
/** "Draw 2", for a second and third draw inside one turn. */
export const drawTwo = def("draw-two", "Spell", { tags: ["Quickdraw"] });
/** A cast-on-draw Spell and a cast-on-draw Unit that note being cast. */
export const castSpell = def("cast-spell", "Spell");
export const castUnit = def("cast-unit", "Unit", { attack: 1, health: 1 });
/** A plain card, drawn to hand. */
export const plain = def("plain", "Spell");
/** Classic #9's trigger, as a Field Spell that notes each of the opponent's 2nd draws in a turn. */
export const taxman = def("taxman", "Field Spell");

// ---------------------------------------------------------------------------
// E27, E28: delayed kinds and the rest of the game
// ---------------------------------------------------------------------------

/** Classic #20's third mode: a chosen Unit is destroyed at the start of your next turn. */
export const doom = def("doom", "Spell");
/** Classic #20 Radiant's: all enemy Units then. */
export const doomAll = def("doom-all", "Spell");
/** Classic #37's clause: discard your hand at the end of this turn (Radiant: of your next turn). */
export const hurrah = def("hurrah", "Spell");
/** `delay` with `next`: a card step at the end of the controller's next turn. */
export const later = def("later", "Spell");
/** Classic+ #52's shape: for the rest of the game, at the start of your turn, a note (a card, in #52). */
export const contract = def("contract", "Spell");
/** The same, whose start-of-turn effect asks. */
export const contractAsk = def("contract-ask", "Spell", { tags: ["Quickdraw"] });

// ---------------------------------------------------------------------------
// The start-of-turn stages: a delayed effect and a start-of-turn hook to order them against.
// ---------------------------------------------------------------------------

/** A Spell that schedules a note for the start of its controller's next turn (R62's delayed stage). */
export const reminder = def("reminder", "Spell");
/** A Field Spell with a start-of-turn hook (R62's triggers stage) and an end-of-turn one. */
export const clock = def("clock", "Field Spell");
/** A Field Spell whose trigger on `crumbled` asks: a start-of-turn stage's own events can pause it. */
export const crumbleWatcher = def("crumble-watcher", "Field Spell");

function limits(list: DrawLimit[]): Script["drawLimit"] {
  return () => list;
}

const secondDraw = (ctx: EffectContext & { event: GameEvent }): boolean =>
  ctx.event.type === "drawn" && ctx.event.player === opponentOf(ctx.controller) && ctx.event.turnDraw === 2;

export const TURN_DEFS: CardDef[] = [
  logCard,
  cutter,
  cutAsker,
  tempo,
  oneMore,
  twoMore,
  rateLimit,
  marker,
  questioner,
  palantir,
  antiGreed,
  drawTwo,
  castSpell,
  castUnit,
  plain,
  taxman,
  doom,
  doomAll,
  hurrah,
  later,
  contract,
  contractAsk,
  reminder,
  clock,
  crumbleWatcher,
];

export const TURN_SCRIPTS: Record<string, CardScripts> = {
  [logCard.id]: faces({}),
  [cutter.id]: faces({ cry: () => [endTurn(), note("after the cut")] }),
  [cutAsker.id]: faces({
    staticFlags: { quickdraw: true },
    cry: () => [endTurn(), askController("asked"), note("cut:tail")],
    resume: { asked: () => [note("cut:answered")] },
  }),
  [tempo.id]: faces(
    { staticFlags: { castOnDraw: true }, cry: () => [endTurn()] },
    { staticFlags: { castOnDraw: true }, cry: () => [endTurnAfterActions({ actions: 1 })] },
  ),
  [oneMore.id]: faces({ cry: () => [endTurnAfterActions({ actions: 1 })] }),
  [twoMore.id]: faces({ cry: () => [endTurnAfterActions({ actions: 2 })] }),
  [rateLimit.id]: faces({
    triggers: [
      {
        id: "rate",
        on: ["cardPlayed"],
        when: (ctx) => ctx.event.type === "cardPlayed" && ctx.event.player !== ctx.controller,
        run: () => [endTurn({ player: "enemy" })],
      },
    ],
  }),
  [marker.id]: faces({ cry: (ctx) => [note(`marker:${ctx.controller}`)] }),
  [questioner.id]: faces({
    cry: () => [askController("asked")],
    resume: { asked: () => [note("questioner:answered")] },
  }),
  [palantir.id]: faces({ staticFlags: { quickdraw: true }, drawLimit: limits([{ player: "enemy", count: 1 }]) }),
  [antiGreed.id]: faces({ drawLimit: limits([{ player: "both", count: 2 }]) }),
  [drawTwo.id]: faces({ staticFlags: { quickdraw: true }, cry: () => [draw({ count: 2 })] }),
  [castSpell.id]: faces({ staticFlags: { castOnDraw: true }, cry: () => [note("cast-spell")] }),
  [castUnit.id]: faces({ staticFlags: { castOnDraw: true }, cry: () => [note("cast-unit")] }),
  [plain.id]: faces({ cry: () => [note("plain")] }),
  [taxman.id]: faces({
    // A Field Spell's trigger reads its condition in `run` (`when` is a trap's, R99).
    triggers: [{ id: "tax", on: ["drawn"], run: (ctx) => (secondDraw(ctx) ? [note("taxed")] : []) }],
  }),
  [doom.id]: faces({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit"] } }],
    cry: () => [destroyAtNextTurnStart({ target: { of: "chosen" } })],
  }),
  [doomAll.id]: faces({ cry: () => [destroyAtNextTurnStart({ scope: { side: "enemy" } })] }),
  [hurrah.id]: faces(
    { cry: () => [discardHandAtTurnEnd({ turn: "this" })] },
    { cry: () => [discardHandAtTurnEnd({ turn: "next" })] },
  ),
  [later.id]: faces({
    cry: () => [delay({ at: { phase: "end", player: "self" }, step: "later", next: true })],
    delayed: (ctx) => [note(`later:${ctx.state.turn}`)],
  }),
  [contract.id]: faces({
    cry: () => [forRestOfGame({ step: "tick", label: "At the start of your turn, take a note", data: { n: 1 } })],
    delayed: (ctx) => [note(`tick:${ctx.controller}:${ctx.state.turn}:${String(ctx.data.n)}:${ctx.self === null ? "no-self" : "self"}`)],
  }),
  [contractAsk.id]: faces({
    staticFlags: { quickdraw: true },
    cry: () => [forRestOfGame({ step: "tick", label: "At the start of your turn, answer" })],
    delayed: () => [note("ask-tick"), askController("ticked"), note("ask-tick:tail")],
    resume: { ticked: () => [note("ask-tick:answered")] },
  }),
  [reminder.id]: faces({
    cry: () => [delay({ at: { phase: "start", player: "self" }, step: "remind" })],
    delayed: () => [note("delayed")],
  }),
  [clock.id]: faces({
    startOfTurn: (ctx) => [note(`start-of-turn:${ctx.controller}`)],
    endOfTurn: (ctx) => [note(`end-of-turn:${ctx.controller}`)],
  }),
  [crumbleWatcher.id]: faces({
    triggers: [{ id: "watch", on: ["crumbled"], run: () => [note("crumble-seen"), askController("seen")] }],
    resume: { seen: () => [note("crumble-answered")] },
  }),
};

export function turnCatalog(base: CardDefs = {}): CardDefs {
  return { ...base, ...Object.fromEntries(TURN_DEFS.map((entry) => [entry.id, entry])) };
}
