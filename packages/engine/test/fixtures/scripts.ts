// Test-only cards for M1-M3 engine tests (BUILD §0): each reproduces one behaviour of a real card
// through the effects library. The real cards, with their own tests, arrive in M4.

import type { CardDef, CardDefs } from "@jackioh/shared";
import type { CardScripts } from "../../src/script";
import type { Effect } from "../../src/script";
import { dealDamage } from "../../src/damage";
import { activeUnitsOf } from "../../src/zones";
import { opponentOf } from "@jackioh/shared";
import {
  addRandomFromGraveyard,
  damage,
  draw,
  gainMana,
  nextTurnMana,
  shuffleCopiesOfSelf,
} from "../../src/effects";
import {
  HERO_POWER_NAMES,
  POWER_RESUME,
  STEADY_SHOT_PARAM,
  heroPower,
  heroPowerActivations,
  rollPower,
} from "../../src/subsystems/heroPower";

/** "All enemies": every enemy unit plus the enemy hero, one instance each (R51). */
function damageAllEnemies(amount: number): Effect {
  return {
    kind: "fixture:damageAllEnemies",
    apply(ctx): void {
      const enemy = opponentOf(ctx.controller);
      for (const unit of activeUnitsOf(ctx.state, enemy)) {
        dealDamage(ctx, { source: ctx.self, target: { kind: "unit", instance: unit }, amount });
      }
      dealDamage(ctx, { source: ctx.self, target: { kind: "hero", player: enemy }, amount });
    },
  };
}

function def(overrides: Partial<CardDef> & Pick<CardDef, "id" | "index" | "name" | "type">): CardDef {
  return {
    set: "Core",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: "" },
    radiant: { keywords: [], text: "" },
    ...overrides,
  };
}

/** #21 Hinder: cast on draw, the opponent's next refresh is 1 lower (2 radiant). */
export const hinder = def({ id: "fx-hinder", index: "21", name: "Hinder (fixture)", type: "Spell", cost: 0 });
const hinderScripts: CardScripts = {
  base: { staticFlags: { castOnDraw: true }, cry: () => [nextTurnMana({ player: "enemy", amount: -1 })] },
  radiant: { staticFlags: { castOnDraw: true }, cry: () => [nextTurnMana({ player: "enemy", amount: -2 })] },
};

/** #90.1 CN-Virus: cast on draw, 1 damage to your hero, 2 copies shuffled in (3 radiant). */
export const cnVirus = def({
  id: "fx-cn-virus",
  index: "90.1",
  name: "CN-Virus (fixture)",
  type: "Spell",
  tags: ["Token", "CN"],
  rarity: "Token",
  token: true,
});
const cnVirusScripts: CardScripts = {
  base: {
    staticFlags: { castOnDraw: true },
    cry: () => [damage({ to: { of: "selfHero" }, amount: 1 }), shuffleCopiesOfSelf({ count: 2 })],
  },
  radiant: {
    staticFlags: { castOnDraw: true },
    cry: () => [damage({ to: { of: "selfHero" }, amount: 1 }), shuffleCopiesOfSelf({ count: 3 })],
  },
};

/** #37 Gravedigger: at the start of your turn, a random graveyard card returns to your hand. */
export const gravedigger = def({
  id: "fx-gravedigger",
  index: "37",
  name: "Gravedigger (fixture)",
  type: "Unit",
  base: { attack: 4, health: 5, keywords: [], text: "" },
  radiant: { attack: 8, health: 10, keywords: [], text: "" },
  cost: 2,
});
const gravediggerScripts: CardScripts = {
  base: { startOfTurn: () => [addRandomFromGraveyard()] },
  radiant: { startOfTurn: () => [addRandomFromGraveyard()] },
};

/** #5 Stockpile: draw 2. */
export const stockpile = def({ id: "fx-stockpile", index: "5", name: "Stockpile (fixture)", type: "Spell" });
const stockpileScripts: CardScripts = {
  base: { cry: () => [draw({ count: 2 })] },
  radiant: { cry: () => [draw({ count: 5 })] },
};

/** #75 Infinite Reserves: an empty-library draw gives a Rush Token card instead of fatigue. */
export const infiniteReserves = def({
  id: "fx-infinite-reserves",
  index: "75",
  name: "Infinite Reserves (fixture)",
  type: "Field Spell",
  cost: 0,
});
const infiniteReservesScripts: CardScripts = {
  base: { staticFlags: { infiniteReserves: true } },
  radiant: { staticFlags: { infiniteReserves: true } },
};

/** #73 Anti-oneshot Armor: your hero takes at most 5 damage per instance (3 radiant). */
export const antiOneshot = def({
  id: "fx-anti-oneshot",
  index: "73",
  name: "Anti-oneshot Armor (fixture)",
  type: "Field Spell",
  cost: 2,
});
const antiOneshotScripts: CardScripts = {
  base: { staticFlags: { antiOneshot: true } },
  radiant: { staticFlags: { antiOneshot: true } },
};

/** #6 Mana Well: at the start of your turn, gain 1 temporary mana. */
export const manaWell = def({
  id: "fx-mana-well",
  index: "6",
  name: "Mana Well (fixture)",
  type: "Field Spell",
  cost: 3,
});
const manaWellScripts: CardScripts = {
  base: { startOfTurn: () => [gainMana({ amount: 1 })] },
  radiant: { startOfTurn: () => [gainMana({ amount: 2 })] },
};

/** #84 Going Long: a Quickdraw Field Spell that gives the hero Armor. */
export const goingLong = def({
  id: "fx-going-long",
  index: "84",
  name: "Going Long (fixture)",
  type: "Field Spell",
  tags: ["Quickdraw"],
  cost: { base: 2, embiggen: 4 },
});
const goingLongScripts: CardScripts = {
  base: { staticFlags: { quickdraw: true } },
  radiant: { staticFlags: { quickdraw: true } },
};

/**
 * #98 Heroic Power as patch v0.2.1 wires it (R43): costs (0), rolls one of the thirteen powers at start
 * of game, and has that power as its one Activate ability. It declares Steady Shot's number (R608).
 */
export const HERO_POWERS = HERO_POWER_NAMES;
export const heroicPower = def({
  id: "fx-heroic-power",
  index: "98",
  name: "Heroic Power (fixture)",
  type: "Field Spell",
  tags: ["Quickdraw"],
  cost: 0,
  params: [{ key: STEADY_SHOT_PARAM, base: 2, radiant: 4, better: "up", step: 2 }],
});
const heroicPowerScripts: CardScripts = {
  base: {
    staticFlags: { quickdraw: true },
    startOfGame: () => [rollPower()],
    activations: heroPowerActivations(false),
    resume: { [POWER_RESUME]: heroPower },
  },
  radiant: {
    staticFlags: { quickdraw: true },
    startOfGame: () => [rollPower()],
    activations: heroPowerActivations(true),
    resume: { [POWER_RESUME]: heroPower },
  },
};

/** #74 Adaptive UI, trimmed to the X part: deal X damage to the enemy hero. */
export const xBolt = def({ id: "fx-x-bolt", index: "74", name: "X Bolt (fixture)", type: "Spell", cost: "X" });
const xBoltScripts: CardScripts = {
  base: { cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.x })] },
  radiant: { cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.x * 2 })] },
};

/** #13 Jlockeed Shredder-10: at your end of turn, 2 damage to every enemy unit and the enemy hero. */
export const shredder = def({
  id: "fx-shredder",
  index: "13",
  name: "Shredder (fixture)",
  type: "Unit",
  cost: 3,
  base: { attack: 8, health: 10, keywords: [], text: "" },
  radiant: { attack: 16, health: 20, keywords: [], text: "" },
});
const shredderScripts: CardScripts = {
  base: { endOfTurn: () => [damageAllEnemies(2)] },
  radiant: { endOfTurn: () => [damageAllEnemies(5)] },
};

/**
 * A fixture with no Core counterpart: it damages the enemy hero and then draws, so one effect can
 * leave both heroes at 0 before the state check runs (R59, BUILD M1-T8).
 */
export const doubleEdge = def({
  id: "fx-double-edge",
  index: "999",
  name: "Double Edge (fixture)",
  type: "Spell",
  cost: 0,
});
const doubleEdgeScripts: CardScripts = {
  base: { cry: () => [damage({ to: { of: "enemyHero" }, amount: 30 }), draw({ count: 1 })] },
  radiant: { cry: () => [damage({ to: { of: "enemyHero" }, amount: 30 }), draw({ count: 1 })] },
};

export const FIXTURE_DEFS: CardDef[] = [
  hinder,
  cnVirus,
  gravedigger,
  stockpile,
  infiniteReserves,
  antiOneshot,
  manaWell,
  goingLong,
  heroicPower,
  xBolt,
  shredder,
  doubleEdge,
];

export const FIXTURE_SCRIPTS: Record<string, CardScripts> = {
  [hinder.id]: hinderScripts,
  [cnVirus.id]: cnVirusScripts,
  [gravedigger.id]: gravediggerScripts,
  [stockpile.id]: stockpileScripts,
  [infiniteReserves.id]: infiniteReservesScripts,
  [antiOneshot.id]: antiOneshotScripts,
  [manaWell.id]: manaWellScripts,
  [goingLong.id]: goingLongScripts,
  [heroicPower.id]: heroicPowerScripts,
  [xBolt.id]: xBoltScripts,
  [shredder.id]: shredderScripts,
  [doubleEdge.id]: doubleEdgeScripts,
};

export function fixtureCatalog(base: CardDefs = {}): CardDefs {
  const defs: Record<string, CardDef> = { ...base };
  for (const entry of FIXTURE_DEFS) defs[entry.id] = entry;
  return defs;
}
