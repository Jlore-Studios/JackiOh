// Fixture cards for patch v0.2.0's instance data (docs/classic-sets.md B2.7, B3.3, B3.4, E38, E39):
// Brittle, Degrade and Upgrade, declared numbers, KY's Constant, faces with their own type and X
// stats, enchantments. Each reproduces the shape of the Classic or Classic+ card named in its doc
// comment through the effects library (BUILD §0); the real cards and their own tests come with the
// card workstreams. `instanceGame` registers them on top of the shared fixture catalog.

import type { CardDef, CardDefs, PlayerId } from "@jackioh/shared";
import { registerCatalog, registeredCatalog } from "../../src/catalog";
import { DECK_SIZE } from "../../src/config";
import {
  buffCards,
  chooseMode,
  chooseTarget,
  chosenTuningNumber,
  damage,
  degrade,
  discoverNumber,
  draw,
  enchant,
  gainBrittle,
  giveBrittle,
  grantKeywordCards,
  setNumber,
  upgrade,
} from "../../src/effects";
import { param } from "../../src/params";
import type { CardScripts, Script } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { createGame, type GameState } from "../../src/state";
import { vanillaDeck } from "./catalog";
import { setupCatalog } from "./harness";

let nextIndex = 4400;

function def(name: string, overrides: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `id-${name}`,
    index: String(nextIndex),
    name: `${name} (instance-data fixture)`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { attack: 2, health: 2, keywords: [], text: name },
    radiant: { attack: 4, health: 4, keywords: [], text: name },
    ...overrides,
  };
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

/** Classic+ #74's printed Brittle on a Unit: Brittle 2, Radiant Brittle 4. */
export const brittleUnit = def("brittle-unit", {
  base: { attack: 2, health: 3, keywords: [{ kind: "Brittle", n: 2 }], text: "Brittle 2" },
  radiant: { attack: 4, health: 6, keywords: [{ kind: "Brittle", n: 4 }], text: "Brittle 4" },
});

/** Classic+ #74 Twice Forward One Step Backwards' shape: a Field Trap printing Brittle 3 (it never fires here). */
export const brittleTrap = def("brittle-trap", {
  type: "Field Trap",
  cost: 2,
  base: { keywords: [{ kind: "Brittle", n: 3 }], text: "Brittle 3" },
  radiant: { keywords: [{ kind: "Brittle", n: 6 }], text: "Brittle 6" },
});

/** A plain 3/4 with no keywords, cost 2: every menu row but the keyword add has room on it. */
export const body = def("body", {
  cost: 2,
  base: { attack: 3, health: 4, keywords: [], text: "3/4" },
  radiant: { attack: 6, health: 8, keywords: [], text: "6/8" },
});

/** A Unit whose Death asks its controller something, so a crumble's death pauses the settle after it (R113). */
export const asker = def("asker", {
  base: { attack: 1, health: 1, keywords: [], text: "Death: choose" },
  radiant: { attack: 2, health: 2, keywords: [], text: "Death: choose" },
});

/** Numbered keywords that print as `Keyword`s (B3.4 rule 3's X row): Armor 2, Lucky 1, Spell Damage 1, Taunt. */
export const numberedBody = def("numbered", {
  base: {
    attack: 2,
    health: 2,
    keywords: [{ kind: "Armor", n: 2 }, { kind: "Lucky", n: 1 }, { kind: "Spell Damage", n: 1 }, { kind: "Taunt" }],
    text: "Armor 2, Lucky 1, Spell Damage +1, Taunt",
  },
  radiant: {
    attack: 4,
    health: 4,
    keywords: [{ kind: "Armor", n: 4 }, { kind: "Lucky", n: 2 }, { kind: "Spell Damage", n: 2 }, { kind: "Taunt" }],
    text: "Armor 4, Lucky 2, Spell Damage +2, Taunt",
  },
});

/**
 * Declared numbers (B3.4 rule 5): `damage` 2 → 4 (more is better), `threshold` 3 (less is better, never
 * below 2), `big` 8 → 16 (default step 2, then 4), `huge` 20 → 40 (default step a quarter: 5, then 10;
 * never above 22 on the base face). Cry: deal {damage} to the enemy hero.
 */
export const numbered = def("params", {
  type: "Spell",
  cost: 2,
  params: [
    { key: "damage", base: 2, radiant: 4, better: "up" },
    { key: "threshold", base: 3, radiant: 3, better: "down", min: 2 },
    { key: "big", base: 8, radiant: 16, better: "up" },
    { key: "huge", base: 20, radiant: 40, better: "up", max: 44 },
  ],
  base: { keywords: [], text: "Deal {damage} damage. {threshold} {big} {huge}" },
  radiant: { keywords: [], text: "Deal {damage} damage. {threshold} {big} {huge}" },
});

/** Classic+ #69 Buff Billy's shape: (X) Unit, "[3X/3X]", Radiant "[7X/7X]"; Cry: Upgrade this X (2X) times. */
export const billy = def("billy", {
  cost: "X",
  base: { attack: 0, health: 0, xStats: { attack: 3, health: 3 }, keywords: [], text: "This is a 3X/3X." },
  radiant: { attack: 0, health: 0, xStats: { attack: 7, health: 7 }, keywords: [], text: "This is a 7X/7X." },
});

/** An X-cost Spell dealing X to the enemy hero, so its tuned X shows at resolution (B3.4 rule 3). */
export const xBolt = def("x-bolt", { type: "Spell", cost: "X", base: { keywords: [], text: "Deal X." }, radiant: { keywords: [], text: "Deal X." } });

/** A Spell with Echo 1 (§6.1), dealing 1 to the enemy hero each time it resolves. */
export const echoBolt = def("echo-bolt", { type: "Spell", cost: 1, base: { keywords: [], text: "Echo 1. Deal 1." }, radiant: { keywords: [], text: "Echo 1. Deal 1." } });

/** A Unit with "Tribute 2" (§6.3) as a static flag. */
export const tributer = def("tributer", { cost: 3, base: { attack: 5, health: 5, keywords: [], text: "Tribute 2" }, radiant: { attack: 10, health: 10, keywords: [], text: "Tribute 2" } });

/** A Unit with "Activate 2: deal 1 damage" (B3.2). */
export const activator = def("activator", { base: { attack: 1, health: 1, keywords: [], text: "Activate 2" }, radiant: { attack: 2, health: 2, keywords: [], text: "Activate 2" } });

/** Classic+ #22 Blood Moon's shape: a Trap whose Radiant face is a Field Trap (B2.7). */
export const bloodMoon = def("blood-moon", {
  type: "Trap",
  base: { keywords: [], text: "Trap" },
  radiant: { type: "Field Trap", keywords: [], text: "Field Trap" },
});

/** Classic #5 Tesla's shape: a Field Trap printing Animated and a 1/4 unit face (B3.1). */
export const tesla = def("tesla", {
  type: "Field Trap",
  base: { attack: 1, health: 4, keywords: [{ kind: "Animated" }], text: "Animated" },
  radiant: { attack: 2, health: 8, keywords: [{ kind: "Animated" }], text: "Animated" },
});

/** An Immutable 3/3 (§6.1, R23): Degrade and Upgrade never change it (B3.4 rule 2). */
export const stoic = def("stoic", {
  base: { attack: 3, health: 3, keywords: [{ kind: "Immutable" }], text: "Immutable" },
  radiant: { attack: 6, health: 6, keywords: [{ kind: "Immutable" }], text: "Immutable" },
});

/** Cost (0) and cost (4) bodies for the cost row's two bounds (B3.4 rule 3). */
export const freeBody = def("free", { cost: 0 });
export const dearBody = def("dear", { cost: 4 });

/** A Unit printing "Can't attack" and Rush: a Degrade may take Rush and never "Can't attack". */
export const shackled = def("shackled", {
  base: { attack: 2, health: 2, keywords: [{ kind: "Can't attack" }, { kind: "Rush" }], text: "Can't attack, Rush" },
  radiant: { attack: 4, health: 4, keywords: [{ kind: "Can't attack" }, { kind: "Rush" }], text: "Can't attack, Rush" },
});

/** Classic+ #72 Book of Nerf's shape: choose a unit, then Degrade it {times} times — a prompt, then the change. */
export const nerfer = def("nerfer", {
  type: "Spell",
  params: [{ key: "times", base: 3, radiant: 5, better: "up" }],
  base: { keywords: [], text: "Choose a unit. Degrade it {times} times." },
  radiant: { keywords: [], text: "Choose a unit. Degrade it {times} times." },
});

/** Classic+ #41 KY's Constant's shape: a hand card, then a random number on it (Radiant: Discover one) to 3. */
export const constant = def("constant", {
  type: "Spell",
  base: { keywords: [], text: "Change a random number to 3." },
  radiant: { keywords: [], text: "Discover a number and change it to 3." },
});

/** Classic+ #8 Withering Storm's shape: Degrade 4 random cards in the opponent's deck (Radiant: every one). Draw 1. */
export const withering = def("withering", {
  type: "Spell",
  cost: 2,
  params: [{ key: "cards", base: 4, radiant: 4, better: "up" }],
  base: { keywords: [], text: "Degrade {cards} random cards in your opponent's deck. Draw 1." },
  radiant: { keywords: [], text: "Degrade every card in your opponent's deck. Draw 1." },
});

/** Classic+ #70 Chaos Machine's shape: at your start and end of turn, Upgrade one of yours and Degrade one of theirs. */
export const chaosMachine = def("chaos-machine", { type: "Field Spell", cost: 2, base: { keywords: [], text: "Chaos" }, radiant: { keywords: [], text: "Chaos" } });

/** Classic+ #23 Dropshipping's shape, cut down: give every card in your hand Brittle {brittle}. */
export const dropship = def("dropship", {
  type: "Spell",
  params: [{ key: "brittle", base: 2, radiant: 2, better: "up" }],
  base: { keywords: [], text: "Give each card in your hand Brittle {brittle}." },
  radiant: { keywords: [], text: "Give each card in your hand Brittle {brittle}." },
});

/** Classic+ #40 Appropriations' Military: your Units on the field, in hand and in deck get +2 Attack and Rush. */
export const military = def("military", { type: "Spell", cost: 2, base: { keywords: [], text: "Military" }, radiant: { keywords: [], text: "Military" } });

/** Classic+ #40's Education: every card in your hand gains Cast on draw (an enchantment). */
export const educator = def("educator", { type: "Spell", base: { keywords: [], text: "Education" }, radiant: { keywords: [], text: "Education" } });

export const INSTANCE_DEFS: CardDef[] = [
  brittleUnit,
  brittleTrap,
  body,
  asker,
  numberedBody,
  numbered,
  billy,
  xBolt,
  echoBolt,
  tributer,
  activator,
  bloodMoon,
  tesla,
  stoic,
  freeBody,
  dearBody,
  shackled,
  nerfer,
  constant,
  withering,
  chaosMachine,
  dropship,
  military,
  educator,
];

/** The step `asker`'s Death opens and the answer re-enters. */
export const ASKER_STEP = "asked";
/** The step `nerfer`'s target prompt re-enters. */
export const NERF_STEP = "nerf";
/** The step `constant`'s Radiant Discover re-enters. */
export const CONSTANT_STEP = "picked";

export const INSTANCE_SCRIPTS: Record<string, CardScripts> = {
  [asker.id]: both({
    death: () => [chooseMode({ options: ["one", "two"], step: ASKER_STEP })],
    resume: {
      [ASKER_STEP]: (ctx) => {
        const pick = ctx.targets[0];
        return [damage({ to: { of: "enemyHero" }, amount: pick?.pick === "mode" && pick.option === "two" ? 2 : 1 })];
      },
    },
  }),
  [numbered.id]: both({ cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: param(ctx, "damage") })] }),
  [billy.id]: {
    base: { cry: (ctx) => [upgrade({ target: { of: "self" }, times: ctx.x })] },
    radiant: { cry: (ctx) => [upgrade({ target: { of: "self" }, times: 2 * ctx.x })] },
  },
  [xBolt.id]: both({ cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: ctx.x })] }),
  [echoBolt.id]: both({ staticFlags: { echo: 1 }, cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }),
  [tributer.id]: both({ staticFlags: { tribute: 2 } }),
  [activator.id]: both({
    activations: [{ id: "ping", label: "Deal 1 damage", uses: 2, run: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] }],
  }),
  [nerfer.id]: both({
    cry: () => [chooseTarget({ step: NERF_STEP, scope: { side: "any" } })],
    resume: { [NERF_STEP]: (ctx) => [degrade({ target: { of: "chosen" }, times: param(ctx, "times") })] },
  }),
  [constant.id]: {
    base: {
      targets: [{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"] } }],
      cry: () => [setNumber({ target: { of: "chosen" }, which: "random", value: 3 })],
    },
    radiant: {
      targets: [{ kind: "hand", min: 1, max: 1, filter: { of: ["hand"] } }],
      cry: () => [discoverNumber({ target: { of: "chosen" }, value: 3, count: 3, step: CONSTANT_STEP })],
      resume: {
        [CONSTANT_STEP]: (ctx) => {
          const chosen = chosenTuningNumber(ctx);
          return chosen === null ? [] : [setNumber({ instanceId: chosen.instanceId, which: chosen.which, value: 3 })];
        },
      },
    },
  },
  [withering.id]: {
    base: {
      cry: (ctx) => [degrade({ scope: { side: "enemy", zones: ["library"] }, random: param(ctx, "cards") }), draw({ count: 1 })],
    },
    radiant: { cry: () => [degrade({ scope: { side: "enemy", zones: ["library"] } }), draw({ count: 1 })] },
  },
  [chaosMachine.id]: both({
    startOfTurn: () => [
      upgrade({ scope: { side: "self", zones: ["hand", "field"] }, random: 1 }),
      degrade({ scope: { side: "enemy", zones: ["hand", "field"] }, random: 1 }),
    ],
    endOfTurn: () => [
      upgrade({ scope: { side: "self", zones: ["hand", "field"] }, random: 1 }),
      degrade({ scope: { side: "enemy", zones: ["hand", "field"] }, random: 1 }),
    ],
  }),
  [dropship.id]: both({ cry: (ctx) => [giveBrittle({ scope: { zones: ["hand"] }, n: param(ctx, "brittle") }), gainBrittle({ scope: { zones: ["field"] }, n: 1 })] }),
  [military.id]: both({
    cry: () => [
      buffCards({ scope: { zones: ["field", "hand", "library"], types: ["Unit"] }, attack: 2 }),
      grantKeywordCards({ scope: { zones: ["field", "hand", "library"], types: ["Unit"] }, keyword: { kind: "Rush" } }),
    ],
  }),
  [educator.id]: both({ cry: () => [enchant({ scope: { zones: ["hand"] }, enchantment: { kind: "castOnDraw" } })] }),
};

/** The fixture catalog with these cards, registered on top of the shared one; returns the whole catalog. */
export function registerInstanceFixtures(): CardDefs {
  setupCatalog();
  const catalog: Record<string, CardDef> = { ...registeredCatalog() };
  for (const card of INSTANCE_DEFS) catalog[card.id] = card;
  registerCatalog(catalog);
  registerScripts({ ...registeredScripts(), ...INSTANCE_SCRIPTS });
  return catalog;
}

/** A game in setup with these fixtures registered; decks default to the vanilla fixture decks. */
export function instanceGame(seed = "instance-data", decks?: [string[], string[]]): GameState {
  registerInstanceFixtures();
  return createGame({ seed, decks: decks ?? [vanillaDeck(DECK_SIZE, 1), vanillaDeck(DECK_SIZE, 21)] });
}

/** A 20-card deck of these fixtures and vanilla fillers, for the played-out replay games. */
export function instanceDeck(player: PlayerId): string[] {
  const own = [
    brittleUnit,
    body,
    asker,
    numberedBody,
    numbered,
    billy,
    xBolt,
    echoBolt,
    nerfer,
    constant,
    withering,
    chaosMachine,
    dropship,
    military,
    educator,
    brittleTrap,
    stoic,
    shackled,
  ].map((card) => card.id);
  const filler = vanillaDeck(DECK_SIZE - own.length, player === "p1" ? 1 : 21);
  return [...own, ...filler];
}
