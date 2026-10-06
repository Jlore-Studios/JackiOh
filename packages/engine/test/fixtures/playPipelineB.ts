// Test-only cards for play pipeline B (docs/classic-sets.md B5 E11, E12, E15, B4.5; R391, R396,
// R452–R455), each reproducing through the effects library the shape of a Classic or Classic+ card
// that the systems exist for. The engine never imports `packages/cards` (CLAUDE.md), so these are
// the engine's proof; the real cards' tests prove the same cases again.
//
// Ids are prefixed `pb-` and indexed from 4520 up, clear of every other fixture file (BUILD §0).

import type { Action, ActionBody, ActionInput, CardDef, Keyword, PlayerId } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { defOf, registerCatalog, registeredCatalog } from "../../src/catalog";
import {
  addCostRule,
  addToHand as addToHandEffect,
  castEach,
  castNew,
  castRandom,
  chosenOptions,
  damage,
  discoverFromCatalog,
  enchantNextSpell,
  plague,
} from "../../src/effects";
import { openPrompt, resumeSelf } from "../../src/prompts";
import type { CardScripts, Effect, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { beginGame, legalActions, reduce } from "../../src/reduce";
import { newInstance, type CardInstance, type GameState } from "../../src/state";
import { newGame } from "./harness";

let nextIndex = 4520;

function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `pb-${name}`,
    index: String(nextIndex),
    name: `${name} (pipeline B)`,
    set: "Core",
    type,
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: name },
    radiant: { keywords: [], text: name },
    ...extra,
  };
}

function unit(name: string, attack: number, health: number, extra: Partial<CardDef> = {}, keywords: Keyword[] = []): CardDef {
  return def(name, "Unit", {
    base: { attack, health, keywords, text: name },
    radiant: { attack: attack * 2, health: health * 2, keywords, text: name },
    ...extra,
  });
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

// ---------------------------------------------------------------------------
// E11: permissions to play from the graveyard (R454)
// ---------------------------------------------------------------------------

/** Classic #28 Second Wind's shape: every card from your graveyard; Radiant, only a price of (1)+. */
export const secondWind = def("second-wind", "Field Spell", { cost: 0 });
/** Classic #74 Corpse Plantation's shape: Units from your graveyard, paid with its Plague Counters. */
export const plantation = def("plantation", "Field Spell", { cost: 2 });
/** Classic #90 In Too Deep's reward L: the same permission, while its memory says it was earned. */
export const questCard = def("quest", "Field Spell", { cost: 1 });
export const QUEST_REWARD_KEY = "rewardL";

/** A Unit with a Cry that is easy to see fire: 2 damage to the enemy hero. */
export const graveUnit = unit("grave-unit", 3, 3, { cost: 2 });
/** A Spell with the same visible effect: 1 damage to the enemy hero. */
export const graveSpell = def("grave-spell", "Spell", { cost: 1 });
/** A (0) Spell, the loop Second Wind's Radiant face stops. */
export const zeroSpell = def("zero-spell", "Spell", { cost: 0 });
/** A Trap, set face-down wherever it is played from (R227). */
export const graveTrap = def("grave-trap", "Trap", { cost: 1 });

// ---------------------------------------------------------------------------
// E12: casts (R452, R453)
// ---------------------------------------------------------------------------

/** A Spell with one declared target, any unit or hero: 3 damage to it. */
export const targetSpell = def("target-spell", "Spell", { cost: 2 });
/** A Spell with a declared mode: "a" deals 1 to the enemy hero, "b" heals nothing and deals 2. */
export const modeSpell = def("mode-spell", "Spell", { cost: 1 });
/** A Spell whose resolution opens a Discover among three fixture units. */
export const discoverSpell = def("discover-spell", "Spell", { cost: 1 });
/** An X Spell: X damage to the enemy hero. */
export const xSpell = def("x-spell", "Spell", { cost: "X" });
/** An X Spell with a declared target: X damage to it. */
export const xTarget = def("x-target", "Spell", { cost: "X" });
/** A Field Spell with a Cry — a cast one with no zone fizzles (R453). */
export const castField = def("cast-field", "Field Spell", { cost: 1 });
/** A Trap a cast sets face-down. */
export const castTrap = def("cast-trap", "Trap", { cost: 1 });
/** A Spell whose resolution asks the OTHER player to choose (Classic #9's shape). */
export const theirChoice = def("their-choice", "Spell", { cost: 1 });
/** A Spell whose resolution asks its controller to choose a target (a prompt, not a declaration). */
export const askTarget = def("ask-target", "Spell", { cost: 1 });

/** The pool the random casters below cast from, named card by card. */
export const RANDOM_POOL = ["pb-target-spell", "pb-mode-spell", "pb-discover-spell", "pb-ask-target"];

/** Classic+ #47 Jogg's Box's shape: cast 3 random Spells from `RANDOM_POOL` (itself excluded). */
export const joggBox = def("jogg-box", "Spell", { cost: 4 });
/** Classic+ #38.1 Solarius-Prime's shape: its Cry casts 2 random Spells that target enemies. */
export const solarius = unit("solarius", 9, 5, { cost: 4 });
/** A random caster whose pool is itself and a Jogg's Box: random casts that cast at random (R452). */
export const chainCaster = def("chain-caster", "Spell", { cost: 1 });
/** Classic #56 Spell Tyrant's Radiant shape: cast every Spell in your graveyard, then exile them. */
export const tyrant = unit("tyrant", 5, 5, { cost: 4 });
/** Classic #47 Recurring Felinor's shape: its Cry casts a named card (`pb-target-spell`). */
export const namedCaster = unit("named-caster", 3, 2, { cost: 2 });
/** Classic+ #14 Forever&'s shape: the next Spell gains "return after resolving, can't cost < (2)". */
export const forever = def("forever", "Spell", { cost: 1 });

// ---------------------------------------------------------------------------
// E15: price rules (R455)
// ---------------------------------------------------------------------------

/** Classic #6 Cloaked Toe Cracker's shape: "Aura: Your Traps cost (0)". */
export const toeCracker = unit("toe-cracker", 3, 4, { cost: 2 });
/** Classic #68 Small Card Lobbyist's shape: (3)+ Cost cards cost (1) more; Radiant, the enemy can't play them. */
export const lobbyist = unit("lobbyist", 11, 13, { cost: 4 });
/** Classic #77 Anti-Magic Monkey's shape: Spells cost (1) more, (2) on the Radiant face. */
export const monkey = unit("monkey", 5, 5, { cost: 2 });
/** Classic #2 The Trickster's shape: your next Trap or Field Spell costs (2) less; Radiant, (0). */
export const trickster = unit("trickster", 2, 1, { cost: 1 });
/** AI Alignment Tax's shape: your opponent's cards cost (1) more during their next turn. */
export const tax = def("tax", "Spell", { cost: 1 });
/** A (3) Spell and a (5) Unit to price. */
export const threeSpell = def("three-spell", "Spell", { cost: 3 });
export const fiveUnit = unit("five-unit", 5, 5, { cost: 5 });
/** A (2) Field Spell to price. */
export const twoField = def("two-field", "Field Spell", { cost: 2 });
/** An X Unit and an embiggen Field Spell, for R396's reader. */
export const xUnit = unit("x-unit", 1, 1, { cost: "X" });
export const embiggenField = def("embiggen-field", "Field Spell", { cost: { base: 2, embiggen: 4 } });

// ---------------------------------------------------------------------------
// B4.5: Tribute zones (R391)
// ---------------------------------------------------------------------------

/** Classic #45 Nature Titan's shape: Tribute 1 on a big body. */
export const titan = unit("titan", 6, 6, { cost: 2 });
/** Tribute 2. */
export const titanTwo = unit("titan-two", 8, 8, { cost: 2 });
/** A backrow card with a Tribute cost (none in the new sets; B4.5 rule 4). */
export const tributeField = def("tribute-field", "Field Spell", { cost: 1 });
/** A body with Reborn: its zone is reserved for its return, so its Tribute frees nothing. */
export const rebornBody = unit("reborn-body", 1, 1, { cost: 1 }, [{ kind: "Reborn" }]);
/** A Stack body, so a pile of two can be built. */
export const stackBody = unit("stack-body", 1, 1, { cost: 1 }, [{ kind: "Stack" }]);

export const PB_DEFS: CardDef[] = [
  secondWind,
  plantation,
  questCard,
  graveUnit,
  graveSpell,
  zeroSpell,
  graveTrap,
  targetSpell,
  modeSpell,
  discoverSpell,
  xSpell,
  xTarget,
  castField,
  castTrap,
  theirChoice,
  askTarget,
  joggBox,
  solarius,
  chainCaster,
  tyrant,
  namedCaster,
  forever,
  toeCracker,
  lobbyist,
  monkey,
  trickster,
  tax,
  threeSpell,
  fiveUnit,
  twoField,
  xUnit,
  embiggenField,
  titan,
  titanTwo,
  tributeField,
  rebornBody,
  stackBody,
];

/** The discover pool: three vanilla fixture units every test catalog holds. */
export const DISCOVER_POOL = ["fx-1", "fx-2", "fx-3"];

/** Opens a prompt for the other player, whose answer deals 1 damage to that player's own hero. */
function askTheOpponent(): Effect {
  return {
    kind: "pb:askTheOpponent",
    apply(ctx): void {
      openPrompt(ctx, {
        player: opponentOf(ctx.controller),
        kind: "mode",
        prompt: "Choose",
        options: ["left", "right"].map((option) => ({ key: `mode:${option}`, label: option, selection: { pick: "mode", option } })),
        resume: resumeSelf(ctx, "theirs"),
      });
    },
  };
}

export const PB_SCRIPTS: Record<string, CardScripts> = {
  [secondWind.id]: {
    base: { graveyardPlay: () => [{}] },
    radiant: { graveyardPlay: () => [{ minPrice: 1 }] },
  },
  [plantation.id]: both({
    cry: () => [plague({ amount: 2 })],
    graveyardPlay: () => [{ units: true, plague: true }],
  }),
  [questCard.id]: both({ graveyardPlay: ({ self }) => (self.memory[QUEST_REWARD_KEY] === true ? [{}] : []) }),
  [graveUnit.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 2 })] }),
  [graveSpell.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [zeroSpell.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [targetSpell.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero"] } }],
    cry: () => [damage({ to: { of: "chosen" }, amount: 3 })],
  }),
  [modeSpell.id]: both({
    modes: [{ kind: "mode", options: ["a", "b"] }],
    cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.modes[0] === "b" ? 2 : 1 })],
  }),
  [discoverSpell.id]: both({
    cry: () => [discoverFromCatalog({ step: "picked", query: { defId: DISCOVER_POOL } })],
    resume: {
      picked: (ctx) => {
        const defId = chosenOptions(ctx)[0];
        return defId === undefined ? [] : [addToHandEffect({ defId })];
      },
    },
  }),
  [xSpell.id]: both({ cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.x })] }),
  [xTarget.id]: both({
    targets: [{ kind: "target", min: 1, max: 1, filter: { of: ["unit", "hero"] } }],
    cry: (ctx) => [damage({ to: { of: "chosen" }, amount: ctx.x })],
  }),
  [castField.id]: both({ cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [theirChoice.id]: both({
    cry: () => [askTheOpponent(), damage({ to: { of: "enemyHero" }, amount: 1 })],
    resume: { theirs: () => [damage({ to: { of: "selfHero" }, amount: 1 })] },
  }),
  [askTarget.id]: both({
    cry: () => [
      // A target prompt among every unit and hero: a random cast that targets enemies picks one of theirs.
      {
        kind: "pb:askTarget",
        apply(ctx): void {
          const heroes = (["p1", "p2"] as const).map((player) => ({
            key: `hero:${player}`,
            label: player,
            selection: { pick: "hero" as const, player },
          }));
          openPrompt(ctx, { player: ctx.controller, kind: "target", prompt: "Hit", options: heroes, resume: resumeSelf(ctx, "hit") });
        },
      },
    ],
    resume: { hit: () => [damage({ to: { of: "chosen" }, amount: 2 })] },
  }),
  [joggBox.id]: {
    base: { cry: () => [castRandom({ query: { defId: [...RANDOM_POOL, joggBox.id] }, count: 3 })] },
    radiant: { staticFlags: { echo: 1 }, cry: () => [castRandom({ query: { defId: [...RANDOM_POOL, joggBox.id] }, count: 3 })] },
  },
  [solarius.id]: {
    base: { cry: () => [castRandom({ query: { defId: RANDOM_POOL }, count: 2, targetEnemies: true })] },
    radiant: { cry: () => [castRandom({ query: { defId: RANDOM_POOL }, count: 2, targetEnemies: true, radiant: true })] },
  },
  [chainCaster.id]: both({
    // Its pool is a copy of itself (excluded, B4.1) and Jogg's Box, whose casts can be chain casters.
    cry: () => [castRandom({ query: { defId: [chainCaster.id, joggBox.id] }, count: 2 })],
  }),
  [tyrant.id]: both({
    cry: () => [
      castEach({
        cards: (ctx) => ctx.state.players[ctx.controller].graveyard.filter((card) => defOf(ctx.state, card.defId).type === "Spell"),
        afterward: "exile",
      }),
    ],
  }),
  [namedCaster.id]: both({ cry: () => [castNew({ def: targetSpell.id })] }),
  [forever.id]: {
    base: { cry: () => [enchantNextSpell({ enchantment: { kind: "returnAfterResolve", floor: 2 } })] },
    radiant: { cry: () => [enchantNextSpell({ enchantment: { kind: "returnAfterResolve", floor: 1 } })] },
  },
  [toeCracker.id]: both({ costAura: () => [{ whose: "yours", types: ["Trap", "Field Trap"], setTo: 0 }] }),
  [lobbyist.id]: {
    base: { costAura: () => [{ whose: "all", minCost: 3, amount: 1 }] },
    radiant: { costAura: () => [{ whose: "opponents", minCost: 3, ban: true }] },
  },
  [monkey.id]: {
    base: { costAura: () => [{ whose: "all", types: ["Spell"], amount: 1 }] },
    radiant: { costAura: () => [{ whose: "all", types: ["Spell"], amount: 2 }] },
  },
  [trickster.id]: {
    base: {
      cry: () => [addCostRule({ rule: { types: ["Trap", "Field Trap", "Field Spell"], amount: -2 }, lasts: "used" })],
    },
    radiant: {
      cry: () => [addCostRule({ rule: { types: ["Trap", "Field Trap", "Field Spell"], setTo: 0 }, lasts: "used" })],
    },
  },
  [tax.id]: both({ cry: () => [addCostRule({ player: "enemy", rule: { amount: 1 }, lasts: "theirNextTurn" })] }),
  [titan.id]: both({ staticFlags: { tribute: 1 }, cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [titanTwo.id]: both({ staticFlags: { tribute: 2 } }),
  [tributeField.id]: both({ staticFlags: { tribute: 1 } }),
};

/** Merge this file's cards into whatever catalog and scripts the test registered first. */
export function registerPipelineB(): void {
  const defs: Record<string, CardDef> = { ...registeredCatalog() };
  for (const entry of PB_DEFS) defs[entry.id] = entry;
  registerCatalog(defs);
  registerScripts({ ...registeredScripts(), ...PB_SCRIPTS });
}

// ---------------------------------------------------------------------------
// Harness shared by the pipeline B tests
// ---------------------------------------------------------------------------

let nonce = 0;

/** Reduce one action with a fresh nonce, returning the whole result. */
export function pbReduce(state: GameState, body: ActionInput): ReturnType<typeof reduce> {
  nonce += 1;
  return reduce(state, { ...body, nonce: `pb${nonce}` } as Action);
}

/** Reduce one action with a fresh nonce; a refusal throws. */
export function pbAct(state: GameState, body: ActionInput): GameState {
  const result = pbReduce(state, body);
  if (result.error !== undefined) throw new Error(result.error);
  return result.state;
}

/** Past the mulligans, in p1's main phase, with these cards registered and p1 at 4 mana. */
export function pbPlaying(seed: string): GameState {
  let state = beginGame(newGame(seed)).state;
  state = pbAct(state, { type: "mulligan", keep: state.players.p1.hand.map((card) => card.id), playerId: "p1" });
  state = pbAct(state, { type: "mulligan", keep: state.players.p2.hand.map((card) => card.id), playerId: "p2" });
  registerPipelineB();
  state.players.p1.mana = { current: 4, max: 4, nextTurnMod: 0, permMod: 0 };
  return state;
}

/** A card of `defId` put straight into `player`'s graveyard. */
export function inGraveyard(state: GameState, defId: string, player: PlayerId = "p1"): CardInstance {
  const card = newInstance(state, defId, player, { z: "graveyard", player });
  state.players[player].graveyard.push(card);
  return card;
}

export function only<T>(items: readonly T[]): T {
  const first = items[0];
  if (first === undefined) throw new Error("expected at least one item");
  return first;
}

/** The `play` actions `legalActions` offers for one instance. */
export function playsOf(state: GameState, instanceId: string, player: PlayerId = "p1"): Extract<ActionBody, { type: "play" }>[] {
  return legalActions(state, player).filter(
    (action): action is Extract<ActionBody, { type: "play" }> => action.type === "play" && action.instanceId === instanceId,
  );
}

/** `JSON.parse(JSON.stringify(state))`, the round trip a paused state must survive (§9.3). */
export function roundTrip(state: GameState): GameState {
  return JSON.parse(JSON.stringify(state)) as GameState;
}
