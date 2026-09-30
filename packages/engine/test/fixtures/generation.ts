// Fixture cards for patch v0.2.0's generation systems (docs/classic-sets.md B5 E19, E23, E24, E25):
// Plague placements, the Fuse variants, the Transform variants and the Recruit extensions. The engine
// never imports `packages/cards` (CLAUDE.md), so each shape a Classic or Classic+ card will take is
// written here once, in the smallest script that has it, and the tests drive them through `reduce`.
//
// Ids are `gen-*` and indexes 4601 upward, so nothing collides with another fixture file's.

import type { Action, ActionBody, CardDef, CardFace, CardType, GameEvent, PlayerId, Selection, SetName, Tag } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import {
  chooseMode,
  chosenOptions,
  damage,
  discoverFromCatalog,
  fuseCards,
  fuseGenerated,
  fuseOntoYourCard,
  fuseRandomInto,
  placePlague,
  placePlagueEach,
  placePlagueRandom,
  placePlagueTokens,
  recruit,
  recruitAll,
  transformBeneath,
} from "../../src/effects";
import { plagueOn } from "../../src/plague";
import { beginGame, reduce } from "../../src/reduce";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { newInstance, type CardInstance, type GameState } from "../../src/state";
import { activeUnitsOf } from "../../src/zones";
import { newGame } from "./harness";

let nextIndex = 4600;

function face(text: string, extra: Partial<CardFace> = {}): CardFace {
  return { keywords: [], text, ...extra };
}

function def(name: string, type: CardType, extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `gen-${name}`,
    index: String(nextIndex),
    name: `Gen ${name}`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: face(`${name} base`),
    radiant: face(`${name} radiant`),
    ...extra,
  };
}

function unit(name: string, attack: number, health: number, extra: Partial<CardDef> = {}): CardDef {
  return def(name, "Unit", {
    base: face(`${name} ${attack}/${health}`, { attack, health }),
    radiant: face(`${name} radiant ${attack * 2}/${health * 2}`, { attack: attack * 2, health: health * 2 }),
    ...extra,
  });
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

// ---------------------------------------------------------------------------
// E19 Plague Tokens.
// ---------------------------------------------------------------------------

/** Classic #70's shape: "Place 2 Plague Tokens" (Radiant 3), then 1 damage to the enemy hero. */
export const plagueBook = def("plague-book", "Spell");
/** Classic #27's shape: Plague Tokens placed on this are doubled (Radiant tripled). */
export const slime = unit("slime", 1, 1);
/** Classic #53's shape: whenever Plague Tokens are placed on this, 1 damage to the enemy hero. */
export const crawler = unit("crawler", 2, 2);
/** Classic #42's aura: your Units +1/+1 per Plague Token on them, enemy Units −1/−1 per token. */
export const toxins = def("toxins", "Field Spell");
/** Classic #39's shape: one placement of 1 (Radiant 2) on a declared permanent. */
export const outbreak = def("outbreak", "Spell");
/** Classic #63's shape: one placement of 1 on each permanent on the field. */
export const dusting = def("dusting", "Spell");
/** Classic #42's Activate, as a Spell: one token on each of 2 different random Units. */
export const scatter = def("scatter", "Spell");
/** Classic #69's self layer: +2 attack for each Plague Token on this. */
export const charger = unit("charger", 4, 2);
/** A face-down trap with no text, to carry tokens in the backrow. */
export const quietTrap = def("quiet-trap", "Trap");
/** A plain body. */
export const body = unit("body", 1, 1);
export const bigBody = unit("big-body", 3, 5);

// ---------------------------------------------------------------------------
// E23 Fuse.
// ---------------------------------------------------------------------------

/** An ingredient with its own lines of code and a Radiant face that is not a doubling. */
export const fuseA = unit("fuse-a", 2, 3, {
  cost: 2,
  loc: 10,
  tags: ["Human"],
  radiant: face("fuse-a radiant 7/1", { attack: 7, health: 1, keywords: [{ kind: "Taunt" }] }),
});
export const fuseB = unit("fuse-b", 1, 1, { cost: 1, loc: 7 });
/** An X-cost Unit and an embiggen Unit, for R470's printed forms. */
export const xUnit = unit("x-unit", 1, 1, { cost: "X" });
export const bigUnit = unit("embiggen-unit", 1, 1, { cost: { base: 2, embiggen: 4 } });
/** An Immutable Unit, which no Fuse keeps (R23). */
export const immutable = unit("immutable", 2, 2, {
  base: face("immutable 2/2", { attack: 2, health: 2, keywords: [{ kind: "Immutable" }] }),
  radiant: face("immutable 4/4", { attack: 4, health: 4, keywords: [{ kind: "Immutable" }] }),
});
/** A Field Trap and a Trap, which count as one type for "of its type". */
export const fieldTrap = def("field-trap", "Field Trap");
export const plainTrap = def("plain-trap", "Trap");
/** Classic+ #31 Fusion Lab's Cry: fuse a random card of the pool into a declared hand card. */
export const lab = def("lab", "Field Spell");
/** The pool Fusion Lab and the deck fusion draw from: the lab itself (never drawn, B4.1) and two cards. */
export const LAB_POOL = [lab.id, fuseA.id, fuseB.id];
/** Classic+ #43 AI Slop: fuse 3 random AI cards into the hand, costing (0). */
export const slop = def("slop", "Spell", { cost: 4 });
export const aiUnit = def("ai-unit", "Unit", {
  token: true,
  tags: ["AI", "Token"],
  rarity: "Token",
  cost: 1,
  base: face("ai unit 1/3", { attack: 1, health: 3 }),
  radiant: face("ai unit radiant 2/6", { attack: 2, health: 6 }),
});
export const aiSpell = def("ai-spell", "Spell", { token: true, tags: ["AI", "Token"], rarity: "Token", cost: 2 });
/** Classic+ #73's deck fusion: every card of your deck, each keeping its cost. */
export const deckFusion = def("deck-fusion", "Spell", { cost: 4 });
/** Classic #78 Radiant's shape: fuse the declared enemy card onto one of yours of its type, then 1 damage. */
export const mutate = def("mutate", "Spell");
/** Classic+ #30 Felinor Fuser: Discover a Felinor Unit, then another, and fuse both into this. */
export const fuser = unit("fuser", 3, 3, { tags: ["Felinor"], cost: 3 });
export const felinorA = unit("felinor-a", 1, 2, { tags: ["Felinor"] });
export const felinorB = unit("felinor-b", 2, 1, { tags: ["Felinor"] });
export const felinorC = unit("felinor-c", 3, 3, { tags: ["Felinor"] });

// ---------------------------------------------------------------------------
// E24 Transform.
// ---------------------------------------------------------------------------

/** Classic+ #4 Juhan: Stack; its Cry makes the cards beneath it copies of it. */
export const juhan = unit("juhan", 9, 6, {
  cost: 3,
  base: face("juhan 9/6", { attack: 9, health: 6, keywords: [{ kind: "Stack" }] }),
  radiant: face("juhan 18/12", { attack: 18, health: 12, keywords: [{ kind: "Stack" }, { kind: "First Strike" }] }),
});
/** Classic and Classic+ Units for a Classic Golem's pool, and a Classic Spell no Unit pool holds. */
export const classicUnit = unit("classic-unit", 2, 2, { set: "Classic" as SetName });
export const classicPlusUnit = unit("classic-plus-unit", 3, 3, { set: "Classic+" as SetName });
export const classicSpell = def("classic-spell", "Spell", { set: "Classic" as SetName });

// ---------------------------------------------------------------------------
// E25 Recruit.
// ---------------------------------------------------------------------------

/** Classic #60 Pile On's shape: Recruit every permanent in your deck. */
export const pileOn = def("pile-on", "Spell", { cost: 5 });
/** A Field Spell that asks a question as it arrives anywhere (R151), so a Recruit can pause. */
export const asker = def("asker", "Field Spell");
export const deckSpell = def("deck-spell", "Spell");
export const cheapUnit = unit("cheap-unit", 1, 1, { cost: 1 });
export const pricyUnit = unit("pricy-unit", 4, 4, { cost: 4 });

/** How many times `asker`'s question was answered, by option. */
export const askerAnswers: string[] = [];

const tokensOn = (card: CardInstance): number => plagueOn(card);

const SCRIPTS: Record<string, CardScripts> = {
  [plagueBook.id]: {
    base: { cry: () => [placePlagueTokens({ count: 2 }), damage({ to: { of: "enemyHero" }, amount: 1 })] },
    radiant: { cry: () => [placePlagueTokens({ count: 3 }), damage({ to: { of: "enemyHero" }, amount: 1 })] },
  },
  [slime.id]: both({ plagueMultiplier: ({ radiant }) => (radiant ? 3 : 2) }),
  [crawler.id]: both({
    triggers: [
      {
        id: "placed-on-this",
        on: ["counterChanged"],
        when: (ctx) => placedOnSelf(ctx.event, ctx.self),
        run: (ctx) => (placedOnSelf(ctx.event, ctx.self) ? [damage({ to: { of: "enemyHero" }, amount: 1 })] : []),
      },
    ],
  }),
  [toxins.id]: {
    base: { aura: ({ state, self }) => toxinsAura(state, self, 1) },
    radiant: { aura: ({ state, self }) => toxinsAura(state, self, 2) },
  },
  [outbreak.id]: {
    base: {
      targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }],
      cry: () => [placePlague({ target: { of: "chosen" }, amount: 1 })],
    },
    radiant: {
      targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "backrow"] } }],
      cry: () => [placePlague({ target: { of: "chosen" }, amount: 2 })],
    },
  },
  [dusting.id]: both({
    cry: () => [placePlagueEach({ scope: { side: "any", rows: ["units", "backrow"] }, amount: 1 })],
  }),
  [scatter.id]: both({ cry: () => [placePlagueRandom({ count: 2, amount: 1 })] }),
  [charger.id]: both({
    aura: ({ self }) => [{ applies: (unit) => unit.id === self.id, mod: { attack: 2 * tokensOn(self) } }],
  }),
  [lab.id]: {
    base: {
      targets: [{ kind: "hand", min: 1, max: 1 }],
      cry: () => [fuseRandomInto({ into: { target: { of: "chosen" } }, query: { defId: LAB_POOL } })],
    },
    radiant: {
      targets: [{ kind: "hand", min: 1, max: 1 }],
      cry: () => [fuseRandomInto({ into: { target: { of: "chosen" } }, query: { defId: LAB_POOL }, radiant: true })],
    },
  },
  [slop.id]: {
    base: { cry: () => [fuseGenerated({ count: 3, query: { tags: ["AI" as Tag], token: true } })] },
    radiant: { cry: () => [fuseGenerated({ count: 3, query: { tags: ["AI" as Tag], token: true }, radiant: true })] },
  },
  [deckFusion.id]: both({
    cry: () => [fuseRandomInto({ into: { pile: "library" }, query: { defId: LAB_POOL } })],
  }),
  [mutate.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit", "backrow"] } }],
    cry: () => [fuseOntoYourCard({ target: { of: "chosen" } }), damage({ to: { of: "enemyHero" }, amount: 1 })],
  }),
  [fuser.id]: {
    base: fuserScript(),
    radiant: fuserScript(),
  },
  [juhan.id]: both({ cry: () => [transformBeneath()] }),
  [pileOn.id]: both({ cry: () => [recruitAll()] }),
  [asker.id]: both({
    startOfGame: () => [chooseMode({ options: ["left", "right"], step: "picked" })],
    resume: {
      picked: (ctx) => {
        askerAnswers.push(...chosenOptions(ctx));
        return [];
      },
    },
  }),
};

/** Classic #53: a placement on this card — `counterChanged` with `placed` naming it. */
function placedOnSelf(event: GameEvent, self: CardInstance | null): boolean {
  return (
    self !== null &&
    event.type === "counterChanged" &&
    event.counter === "plague" &&
    event.instanceId === self.id &&
    event.placed !== undefined
  );
}

/**
 * Classic+ #30's two chained Discovers (the Stitching pattern, R352) and the fusion onto this Unit:
 * the picks travel in the next prompt's data, and the second answer fuses both onto the card itself.
 */
function fuserScript(): Script {
  const discover = (step: string, picks: readonly string[]) =>
    discoverFromCatalog({ step, query: { tags: ["Felinor"], type: "Unit" }, data: { picks: [...picks] } });
  const picksOf = (data: Record<string, unknown>): string[] =>
    Array.isArray(data.picks) ? data.picks.filter((pick): pick is string => typeof pick === "string") : [];
  return {
    cry: () => [discover("first", [])],
    resume: {
      first: (ctx) => [discover("second", [...picksOf(ctx.data), ...chosenOptions(ctx)])],
      second: (ctx) => {
        const picks = [...picksOf(ctx.data), ...chosenOptions(ctx)];
        return ctx.self === null ? [] : [fuseCards({ defIds: picks, targetInstanceId: ctx.self.id })];
      },
    },
  };
}

/**
 * Classic #42's aura: each Unit on the field with Plague Tokens on it gets `per` × its tokens, up on
 * its controller's side and down on the other, as one entry per Unit (the amount is the Unit's own).
 */
function toxinsAura(state: GameState, self: CardInstance, per: number): ReturnType<NonNullable<Script["aura"]>> {
  return (["p1", "p2"] as const).flatMap((player) =>
    activeUnitsOf(state, player).flatMap((unit) => {
      const tokens = tokensOn(unit);
      if (tokens === 0) return [];
      const sign = unit.controller === self.controller ? 1 : -1;
      return [
        {
          applies: (candidate: CardInstance) => candidate.id === unit.id,
          mod: { attack: sign * per * tokens, maxHealth: sign * per * tokens },
        },
      ];
    }),
  );
}

export const GEN_DEFS: CardDef[] = [
  plagueBook,
  slime,
  crawler,
  toxins,
  outbreak,
  dusting,
  scatter,
  charger,
  quietTrap,
  body,
  bigBody,
  fuseA,
  fuseB,
  xUnit,
  bigUnit,
  immutable,
  fieldTrap,
  plainTrap,
  lab,
  slop,
  aiUnit,
  aiSpell,
  deckFusion,
  mutate,
  fuser,
  felinorA,
  felinorB,
  felinorC,
  juhan,
  classicUnit,
  classicPlusUnit,
  classicSpell,
  pileOn,
  asker,
  deckSpell,
  cheapUnit,
  pricyUnit,
];

export const GEN_SCRIPTS = SCRIPTS;

/** Register this file's fixtures on top of whatever is registered. */
export function registerGeneration(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(GEN_DEFS.map((entry) => [entry.id, entry])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
}

// ---------------------------------------------------------------------------
// Driving a game.
// ---------------------------------------------------------------------------

export type ActionInput = ActionBody & { playerId: PlayerId };

/** A replayable run: the state it started from (as JSON) and every action applied since. */
export type Run = { start: string; log: Action[]; state: GameState };

let nonce = 0;

/** Past the mulligans, in p1's main phase with 4 mana, this file's fixtures registered. */
export function playing(seed: string): Run {
  let state = beginGame(newGame(seed)).state;
  for (const player of ["p1", "p2"] as const) {
    const result = reduce(state, {
      type: "mulligan",
      keep: state.players[player].hand.map((card) => card.id),
      playerId: player,
      nonce: `gen-mull-${seed}-${player}`,
    } as Action);
    if (result.error !== undefined) throw new Error(result.error);
    state = result.state;
  }
  registerGeneration();
  state.players.p1.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  return { start: "", log: [], state };
}

/** Freeze the run's starting point: everything the test set up by hand is in it from here on. */
export function frozen(run: Run): Run {
  return { start: JSON.stringify(run.state), log: [], state: run.state };
}

/** Apply one action and record it; throws on a refusal. */
export function act(run: Run, body: ActionInput): Run {
  nonce += 1;
  const action = { ...body, nonce: `gen${nonce}` } as Action;
  const result = reduce(run.state, action);
  if (result.error !== undefined) throw new Error(`${body.type} refused: ${result.error}`);
  return { start: run.start, log: [...run.log, action], state: result.state };
}

/** The refusal an action meets, or undefined when it is legal (the state is left as it was). */
export function refusal(run: Run, body: ActionInput): string | undefined {
  nonce += 1;
  return reduce(run.state, { ...body, nonce: `gen${nonce}` } as Action).error;
}

/** The run's log applied again from its frozen start: the replay a server folds (§9.3). */
export function replayed(run: Run): GameState {
  let state = JSON.parse(run.start) as GameState;
  for (const action of run.log) {
    const result = reduce(state, action);
    if (result.error !== undefined) throw new Error(`replay refused ${action.type}: ${result.error}`);
    state = result.state;
  }
  return state;
}

/** A card put straight into a player's hand. */
export function handCard(state: GameState, defId: string, player: PlayerId = "p1"): CardInstance {
  const card = newInstance(state, defId, player, { z: "hand", player });
  state.players[player].hand.push(card);
  return card;
}

/** Answer the open prompt with one option, picked by its selection. */
export function answer(run: Run, selection: Selection, player?: PlayerId): Run {
  const pending = run.state.pending;
  if (pending === null) throw new Error("no prompt is open");
  return act(run, { type: "answer", choiceId: pending.id, selection: [selection], playerId: player ?? pending.playerId });
}

/** The instance selection for a card. */
export function pick(card: CardInstance | string): Selection {
  return { pick: "instance", instanceId: typeof card === "string" ? card : card.id };
}
