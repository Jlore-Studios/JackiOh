// Test-only cards for the prompts-and-movement systems of patch v0.2.0 (docs/classic-sets.md B5 E13,
// E16, E17, E18, E26). Each reproduces the engine half of a Classic or Classic+ card through the
// effects library — the card itself arrives with its own script and test in the card waves — so the
// engine is proved without `packages/cards` (CLAUDE.md: the engine never imports it).

import type { CardDef, GameEvent, PlayerId } from "@jackioh/shared";
import { opponentOf } from "@jackioh/shared";
import { defOf, registerCatalog, registeredCatalog } from "../../src/catalog";
import {
  addToHand,
  answeredCorrectly,
  buff,
  chooseAnswer,
  chooseCell,
  chooseCostInHand,
  chooseFromHand,
  chooseMode,
  chooseNumber,
  choosePick,
  chooseReward,
  chosenCells,
  chosenNumber,
  chosenOptions,
  damage,
  discard,
  draw,
  drawFromOpponent,
  exile,
  exileBottomOfLibrary,
  exileMatching,
  giveFromHand,
  heal,
  summon,
  summonThis,
  takeFromLibrary,
  triggerCry,
} from "../../src/effects";
import type { CardScripts, Effect, EffectContext, Script, TriggerDef } from "../../src/script";
import { registerScripts, registeredScripts } from "../../src/scripts";
import { cardAt, moveToZone } from "../../src/zones";

let nextIndex = 3100;
function def(name: string, type: CardDef["type"], extra: Partial<CardDef> = {}): CardDef {
  nextIndex += 1;
  return {
    id: `pm-${name}`,
    index: String(nextIndex),
    name: `${name} (prompts)`,
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

function unit(name: string, attack: number, health: number, extra: Partial<CardDef> = {}): CardDef {
  return def(name, "Unit", {
    base: { attack, health, keywords: [], text: name },
    radiant: { attack: attack * 2, health: health * 2, keywords: [], text: name },
    ...extra,
  });
}

function both(script: Script): CardScripts {
  return { base: script, radiant: script };
}

// ---------------------------------------------------------------------------------------------
// E18: the prompt kinds
// ---------------------------------------------------------------------------------------------

/** What every fixture hands out when its chain reached the end it should have. */
export const prize = unit("prize", 1, 1);

/** Classic #8 Pickle's shape: the opponent chooses three times, their own hand pick for "discard". */
export const pickle = def("pickle", "Spell", { cost: 1 });
export const PICKLE_OPTIONS = ["discard", "exile", "draw"] as const;
const PICKLE_CHOICES = 3;

function pickleAsk(n: number): Effect[] {
  if (n > PICKLE_CHOICES) return [];
  return [
    chooseMode({
      by: "enemy",
      options: [...PICKLE_OPTIONS],
      step: "chosen",
      prompt: `Pickle: choose (${n} of ${PICKLE_CHOICES})`,
      data: { n },
    }),
  ];
}

function pickleCount(ctx: EffectContext): number {
  return typeof ctx.data.n === "number" ? ctx.data.n : 1;
}

const pickleScript: Script = {
  cry: () => pickleAsk(1),
  resume: {
    chosen: (ctx) => {
      const n = pickleCount(ctx);
      switch (chosenOptions(ctx)[0]) {
        case "discard":
          return [chooseFromHand({ of: "enemy", by: "enemy", step: "discarded", prompt: "Pickle: discard", data: { n } })];
        case "exile":
          return [exileBottomOfLibrary({ player: "enemy" }), ...pickleAsk(n + 1)];
        default:
          return [draw({ count: 1 }), ...pickleAsk(n + 1)];
      }
    },
    discarded: (ctx) => [discard({ target: { of: "chosen" } }), ...pickleAsk(pickleCount(ctx) + 1)],
  },
};

/** Classic #18 Glitch in the System's shape: a `number` declared with the play (R81), 0 to 10. */
export const glitch = def("glitch", "Spell", { cost: 0 });
export const GLITCH_OPTIONS = Array.from({ length: 11 }, (_, n) => String(n));
const glitchScript: Script = {
  modes: [{ kind: "number", options: GLITCH_OPTIONS }],
  cry: (ctx) => [damage({ to: { of: "enemyHero" }, amount: chosenNumber(ctx) ?? 0 })],
};

/** A `number` prompt asked at resolution, 1 to 3: the chosen number is the damage. */
export const numberer = def("numberer", "Spell", { cost: 0 });
const numbererScript: Script = {
  cry: () => [chooseNumber({ from: 1, to: 3, step: "n", prompt: "Pick 1 to 3" })],
  resume: { n: (ctx) => [damage({ to: { of: "enemyHero" }, amount: chosenNumber(ctx) ?? 0 })] },
};

/** Classic+ #42 KY's Test's answer half: right answer, the prize; wrong, nothing (R465). */
export const quiz = def("quiz", "Spell", { cost: 1 });
export const QUIZ = { statement: "2 + 2 = ?", options: ["4", "3", "5", "22"], correct: 0 } as const;
const quizScript: Script = {
  cry: () => [
    chooseAnswer({
      step: "answered",
      statement: QUIZ.statement,
      options: [...QUIZ.options],
      correct: QUIZ.correct,
      data: { difficulty: "easy" },
    }),
    // The tail after the question: it runs after the answer, and it reads no key.
    heal({ target: { of: "selfHero" }, amount: 1 }),
  ],
  resume: {
    answered: (ctx) => (answeredCorrectly(ctx) ? [addToHand({ defId: prize.id })] : []),
  },
};

/** Classic+ #62 KY's Papaya's cell half: cells one at a time, a lane each, "done" after the first. */
export const papaya = def("papaya", "Spell", { cost: 1 });
export const PAPAYA_CELLS = 4;
function cellsSoFar(ctx: EffectContext): { player: PlayerId; row: string; lane: number }[] {
  const held = Array.isArray(ctx.data.cells) ? (ctx.data.cells as { player: PlayerId; row: string; lane: number }[]) : [];
  return [...held, ...chosenCells(ctx)];
}
const papayaScript: Script = {
  cry: () => [chooseCell({ step: "cell", prompt: "Papaya: a point", data: { cells: [] } })],
  resume: {
    cell: (ctx) => {
      const cells = cellsSoFar(ctx);
      const done = ctx.targets.some((selection) => selection.pick === "none") || cells.length >= PAPAYA_CELLS;
      if (!done) {
        return [
          chooseCell({
            step: "cell",
            done: true,
            cells: { exceptLanes: cells.map((cell) => cell.lane) },
            prompt: "Papaya: another point",
            data: { cells },
          }),
        ];
      }
      // Exile whatever stands on the chosen cells: the engine half of "every card on the curve".
      return cells.flatMap((cell) => {
        const card = cardAt(ctx.state, { player: cell.player, row: cell.row === "units" ? "units" : "backrow", lane: cell.lane });
        return card === null ? [] : [exile({ target: { of: "instance", instanceId: card.id } })];
      });
    },
  },
};

/** Classic #90 In Too Deep's reward half: asked of its controller whenever the other player draws. */
export const quest = def("quest", "Field Spell", { cost: 1 });
export const QUEST_REWARDS = [
  { id: "A", label: "Heal your hero 6" },
  { id: "B", label: "Deal 3 damage to the enemy hero" },
] as const;
const questTrigger: TriggerDef = {
  id: "quest-done",
  on: ["drawn"],
  run: (ctx) => {
    const event = ctx.event as Extract<GameEvent, { type: "drawn" }>;
    if (event.player === ctx.controller) return [];
    return [chooseReward({ step: "reward", rewards: [...QUEST_REWARDS], prompt: "Quest complete" })];
  },
};
const questScript: Script = {
  triggers: [questTrigger],
  resume: {
    reward: (ctx) =>
      chosenOptions(ctx)[0] === "A"
        ? [heal({ target: { of: "selfHero" }, amount: 6 })]
        : [damage({ to: { of: "enemyHero" }, amount: 3 })],
  },
};

/** Classic #34 Ancient Acquisition's shape: up to 2 (Radiant 4, graveyard or exile) to your hand. */
export const acquire = def("acquire", "Spell", { cost: 1 });
function returnPicks(ctx: EffectContext): Effect[] {
  return ctx.targets.map((_, index) => addToHand({ instance: { of: "chosen", index } }));
}
export const acquireScripts: CardScripts = {
  base: {
    cry: () => [choosePick({ step: "picked", from: [{ zone: "graveyard" }], max: 2, prompt: "Return 2" })],
    resume: { picked: returnPicks },
  },
  radiant: {
    cry: () => [
      choosePick({ step: "picked", from: [{ zone: "graveyard" }, { zone: "exile" }], max: 4, prompt: "Return 4" }),
    ],
    resume: { picked: returnPicks },
  },
};

/** Classic #44 Back from the GY's shape: graveyard Units costing (5) or less together, summoned. */
export const backFromGy = def("back-from-gy", "Spell", { cost: 1 });
export const BACK_BUDGET = 5;
const backScript: Script = {
  cry: () => [
    choosePick({
      step: "picked",
      from: [{ zone: "graveyard" }],
      filter: { type: "Unit" },
      max: 5,
      budget: BACK_BUDGET,
      prompt: "Summon Units costing (5) or less",
    }),
  ],
  resume: {
    picked: (ctx) => ctx.targets.map((_, index) => summon({ instance: { of: "chosen", index } })),
  },
};

// ---------------------------------------------------------------------------------------------
// E17: the opponent's hand as the options
// ---------------------------------------------------------------------------------------------

/**
 * Classic #11 Mind Melt's shape (SPEC §8.6): base, a `pick` of one card of their hand, exiled; Radiant,
 * a `mode` prompt of the costs in their hand, and every card of the chosen cost exiled.
 */
export const mindMelt = def("mind-melt", "Spell", { cost: 1 });
export const mindMeltScripts: CardScripts = {
  base: {
    cry: () => [
      choosePick({ step: "exile", from: [{ zone: "hand", player: "enemy" }], min: 1, max: 1, prompt: "Exile a card from their hand" }),
    ],
    resume: { exile: () => [exile({ target: { of: "chosen" } })] },
  },
  radiant: {
    cry: () => [chooseCostInHand({ step: "cost", prompt: "Choose a cost" })],
    resume: {
      cost: (ctx) => {
        const cost = chosenNumber(ctx);
        return cost === undefined ? [] : [exileMatching({ zones: ["hand"], player: "enemy", cost })];
      },
    },
  },
};

// ---------------------------------------------------------------------------------------------
// E16, E2: cards between the players' piles
// ---------------------------------------------------------------------------------------------

/** Classic #9 Income Tax's shape: when the opponent draws, they keep one card and give you the rest. */
export const incomeTax = def("income-tax", "Trap", { cost: 2 });
function incomeTaxScript(radiant: boolean): Script {
  return {
    triggers: [
      {
        id: "tax",
        on: ["drawn"],
        when: (ctx) => (ctx.event as Extract<GameEvent, { type: "drawn" }>).player === opponentOf(ctx.controller),
        run: () => [chooseFromHand({ of: "enemy", by: "enemy", step: "keep", prompt: "Keep one card" })],
      },
    ],
    resume: {
      keep: () => [giveFromHand({ from: "enemy", cards: "unchosen", ...(radiant ? { costMod: -1 } : {}) })],
    },
  };
}

/** Classic+ #12.3 Fluffy Grip's shape: a random Unit of their deck to your hand, costing (0). */
export const fluffyGrip = def("fluffy-grip", "Spell", { cost: 1 });
export const fluffyGripScripts: CardScripts = {
  base: { cry: () => [takeFromLibrary({ from: "enemy", filter: { type: "Unit" }, costOverride: 0 })] },
  radiant: { cry: () => [takeFromLibrary({ from: "enemy", filter: { type: "Unit" }, costOverride: 0, radiant: true })] },
};

/** Classic #58 Common Resources' shape: start of turn, draw the bottom card of their deck. */
export const commonResources = def("common-resources", "Field Spell", { cost: 2 });
const commonResourcesScript: Script = { startOfTurn: () => [drawFromOpponent()] };

/** A card that casts itself on draw and says so: it proves a stolen draw casts for the drawer. */
export const castOnDrawMarker = def("cast-marker", "Spell", { cost: 0 });
const castOnDrawMarkerScript: Script = {
  staticFlags: { castOnDraw: true },
  cry: () => [damage({ to: { of: "enemyHero" }, amount: 2 })],
};

// ---------------------------------------------------------------------------------------------
// E13: trigger a Cry
// ---------------------------------------------------------------------------------------------

/** A Cry with no choices: 2 to the enemy hero, and +1/+1 on "this". */
export const crier = unit("crier", 2, 3, { cost: 1 });
const crierScript: Script = {
  cry: () => [damage({ to: { of: "enemyHero" }, amount: 2 }), buff({ target: { of: "self" }, attack: 1, health: 1 })],
};

/** A Cry with a declared target (R81): 3 damage to it. */
export const aimer = unit("aimer", 1, 1, { cost: 1 });
const aimerScript: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit", "hero"] } }],
  cry: () => [damage({ to: { of: "chosen" }, amount: 3 })],
};

/** A Cry with a mode, and a target that belongs to one mode only (`forModes`, R90). */
export const moder = unit("moder", 1, 1, { cost: 1 });
const moderScript: Script = {
  modes: [{ kind: "mode", options: ["hit", "heal"] }],
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit", "hero"] }, forModes: ["hit"] }],
  cry: (ctx) =>
    ctx.modes[0] === "hit"
      ? [damage({ to: { of: "chosen" }, amount: 4 })]
      : [heal({ target: { of: "selfHero" }, amount: 4 })],
};

/** A Cry that asks in the middle of its list, so its own tail is parked (R113). */
export const asker = unit("asker", 1, 1, { cost: 1 });
const askerScript: Script = {
  cry: () => [
    damage({ to: { of: "enemyHero" }, amount: 1 }),
    chooseMode({ options: ["left", "right"], step: "after", prompt: "asker" }),
    damage({ to: { of: "enemyHero" }, amount: 2 }),
  ],
  resume: { after: () => [damage({ to: { of: "enemyHero" }, amount: 4 })] },
};

/** A Cry whose first declaration is a Tribute (a play's price), then a target (R90, R123). */
export const tributeCrier = unit("tribute-crier", 3, 3, { cost: 2 });
const tributeCrierScript: Script = {
  staticFlags: { tribute: 1 },
  targets: [
    { kind: "tribute", min: 1, max: 1, filter: { side: "ally", of: ["unit"] }, amount: 1 },
    { kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["hero"] } },
  ],
  cry: (ctx) => [
    damage({ to: { of: "chosen", index: 1 }, amount: ctx.targets[0]?.pick === "none" ? 5 : 1 }),
  ],
};

/**
 * Classic #54 Rewind's shape. Base: a Unit of yours on the field, declared with the play. The graveyard
 * form picks the Unit out of the graveyard at resolution, as a card with `TargetFilter.of: "graveyard"`
 * will declare it once the play pipeline offers graveyard targets. Radiant: twice, each run its own.
 */
export const rewind = def("rewind", "Spell", { cost: 0 });
export const rewindScripts: CardScripts = {
  base: {
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "ally", of: ["unit"] } }],
    cry: () => [triggerCry({ target: { of: "chosen" } }), heal({ target: { of: "selfHero" }, amount: 1 })],
  },
  radiant: {
    targets: [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit"] } }],
    cry: () => [
      triggerCry({ target: { of: "chosen" } }),
      triggerCry({ target: { of: "chosen" } }),
      heal({ target: { of: "selfHero" }, amount: 1 }),
    ],
  },
};
export const graveRewind = def("grave-rewind", "Spell", { cost: 1 });
const graveRewindScript: Script = {
  cry: () => [
    choosePick({ step: "picked", from: [{ zone: "graveyard" }], filter: { type: "Unit" }, min: 1, max: 1 }),
  ],
  resume: { picked: () => [triggerCry({ target: { of: "chosen" } })] },
};

// ---------------------------------------------------------------------------------------------
// E26: deck and graveyard triggers, "summon this"
// ---------------------------------------------------------------------------------------------

/** "After you play …": the play's `cardResolved` (§10.5 step 7), once the played card has resolved. */
function playedEvent(ctx: EffectContext): Extract<GameEvent, { type: "cardResolved" }> | null {
  const event = (ctx as EffectContext & { event?: GameEvent }).event;
  return event !== undefined && event.type === "cardResolved" ? event : null;
}

/** Classic #66 EU Striker's shape: from your hand, after you play a Unit, summon this (its Cry would ping). */
export const striker = unit("striker", 5, 4, { cost: 2 });
const strikerTrigger: TriggerDef = {
  id: "striker-arrive",
  on: ["cardResolved"],
  run: (ctx) => {
    const played = playedEvent(ctx);
    if (played === null || played.player !== ctx.controller || played.instanceId === ctx.self?.id) return [];
    return defOf(ctx.state, played.defId).type === "Unit" ? [summonThis()] : [];
  },
};
const strikerScript: Script = {
  handTriggers: [strikerTrigger],
  cry: () => [damage({ to: { of: "enemyHero" }, amount: 9 })],
};

/** Classic+ #37 Wardrum's shape: from your hand or deck, after you play a Spell, summon this. */
export const wardrum = unit("wardrum", 5, 5, { cost: 5 });
const wardrumTrigger: TriggerDef = {
  id: "wardrum-arrive",
  on: ["cardResolved"],
  run: (ctx) => {
    const played = playedEvent(ctx);
    if (played === null || played.player !== ctx.controller) return [];
    return defOf(ctx.state, played.defId).type === "Spell" ? [summonThis()] : [];
  },
};
const wardrumScript: Script = { handTriggers: [wardrumTrigger], deckTriggers: [wardrumTrigger] };

/** A deck trigger that asks, so its list parks under its id and resumes (R113, `work.scriptStepFor`). */
export const deckAsker = unit("deck-asker", 1, 1, { cost: 1 });
const deckAskerScript: Script = {
  deckTriggers: [
    {
      id: "deck-ask",
      on: ["cardResolved"],
      run: (ctx) => {
        const played = playedEvent(ctx);
        if (played === null || played.player !== ctx.controller || defOf(ctx.state, played.defId).type !== "Spell") return [];
        return [
          chooseMode({ options: ["stay", "come"], step: "decided", prompt: "Deck asker" }),
          damage({ to: { of: "enemyHero" }, amount: 1 }),
        ];
      },
    },
  ],
  resume: { decided: (ctx) => (chosenOptions(ctx)[0] === "come" ? [summonThis()] : []) },
};

/** A deck trigger that answers every play and does nothing: the hidden card that must stay hidden. */
export const deckWatcher = unit("deck-watcher", 1, 1, { cost: 1 });
const deckWatcherScript: Script = {
  deckTriggers: [{ id: "deck-watch", on: ["cardPlayed", "cardResolved"], run: () => [] }],
};

/** Classic #47 Recurring Felinor's shape: in your graveyard, when one of your Traps fires, return this. */
export const recurring = unit("recurring", 3, 2, { cost: 2 });
function recurringScript(radiant: boolean): Script {
  return {
    graveyardTriggers: [
      {
        id: "recur",
        on: ["trapFired"],
        run: (ctx) => {
          const event = ctx.event as Extract<GameEvent, { type: "trapFired" }>;
          if (event.controller !== ctx.controller) return [];
          return [addToHand({ instance: { of: "self" }, ...(radiant ? { costOverride: 0 } : {}) })];
        },
      },
    ],
  };
}

/** A graveyard trigger that answers every play and does nothing: R464's last place in a side's order. */
export const graveWatcher = unit("grave-watcher", 1, 1, { cost: 1 });
const graveWatcherScript: Script = {
  graveyardTriggers: [{ id: "grave-watch", on: ["cardPlayed", "cardResolved"], run: () => [] }],
};

/** Classic #78 Radiant's pick across zones: one Unit of yours on the field, in your hand or in your deck. */
export const crossPick = def("cross-pick", "Spell", { cost: 0 });
const crossPickScript: Script = {
  cry: () => [
    choosePick({
      step: "picked",
      from: [{ zone: "field" }, { zone: "hand" }, { zone: "library" }],
      filter: { type: "Unit" },
      min: 1,
      max: 1,
      prompt: "A Unit of yours",
    }),
  ],
  resume: { picked: () => [exile({ target: { of: "chosen" } })] },
};

/** A Trap that fires when the other player plays a card, and does nothing else. */
export const snare = def("snare", "Trap", { cost: 1 });
const snareScript: Script = {
  triggers: [
    {
      id: "snare",
      on: ["cardPlayed"],
      when: (ctx) => (ctx.event as Extract<GameEvent, { type: "cardPlayed" }>).player !== ctx.controller,
      run: () => [],
    },
  ],
};

/** Mills the top three cards of its controller's library into their graveyard, for a pile to pick from. */
export const mill = def("mill", "Spell", { cost: 0 });
const MILL_COUNT = 3;
const millScript: Script = {
  cry: () => [
    {
      kind: "fixture:mill",
      apply(ctx): void {
        for (const card of ctx.state.players[ctx.controller].library.slice(0, MILL_COUNT)) {
          moveToZone(ctx.state, card, "graveyard");
        }
      },
    },
  ],
};

/** A plain Spell and a plain Unit to play. */
export const spark = def("spark", "Spell", { cost: 0 });
const sparkScript: Script = { cry: () => [damage({ to: { of: "enemyHero" }, amount: 1 })] };
export const grunt = unit("grunt", 2, 2, { cost: 0 });

// ---------------------------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------------------------

/** A Quickdraw copy of a fixture, so a replay test's opening hand holds it whatever the shuffle (§2.1, R225). */
export function quickdrawOf(card: CardDef): CardDef {
  return { ...card, id: `${card.id}-qd`, index: `${card.index}-qd`, name: `${card.name} (quickdraw)` };
}

const QUICKDRAW_OF = [
  pickle,
  quiz,
  papaya,
  backFromGy,
  rewind,
  mindMelt,
  graveRewind,
  spark,
  grunt,
  mill,
  crier,
  aimer,
  fluffyGrip,
  striker,
  wardrum,
  quest,
];

export const PROMPT_DEFS: CardDef[] = [
  prize,
  pickle,
  glitch,
  numberer,
  quiz,
  papaya,
  quest,
  acquire,
  backFromGy,
  mindMelt,
  incomeTax,
  fluffyGrip,
  commonResources,
  castOnDrawMarker,
  crier,
  aimer,
  moder,
  asker,
  tributeCrier,
  rewind,
  graveRewind,
  striker,
  wardrum,
  deckAsker,
  deckWatcher,
  recurring,
  snare,
  spark,
  grunt,
  mill,
  graveWatcher,
  crossPick,
  ...QUICKDRAW_OF.map(quickdrawOf),
];

function quickdraw(scripts: CardScripts): CardScripts {
  const flag = (script: Script): Script => ({ ...script, staticFlags: { ...script.staticFlags, quickdraw: true } });
  return { base: flag(scripts.base), radiant: flag(scripts.radiant) };
}

const BASE_SCRIPTS: Record<string, CardScripts> = {
  [pickle.id]: both(pickleScript),
  [glitch.id]: both(glitchScript),
  [numberer.id]: both(numbererScript),
  [quiz.id]: both(quizScript),
  [papaya.id]: both(papayaScript),
  [quest.id]: both(questScript),
  [acquire.id]: acquireScripts,
  [backFromGy.id]: both(backScript),
  [mindMelt.id]: mindMeltScripts,
  [incomeTax.id]: { base: incomeTaxScript(false), radiant: incomeTaxScript(true) },
  [fluffyGrip.id]: fluffyGripScripts,
  [commonResources.id]: both(commonResourcesScript),
  [castOnDrawMarker.id]: both(castOnDrawMarkerScript),
  [crier.id]: both(crierScript),
  [aimer.id]: both(aimerScript),
  [moder.id]: both(moderScript),
  [asker.id]: both(askerScript),
  [tributeCrier.id]: both(tributeCrierScript),
  [rewind.id]: rewindScripts,
  [graveRewind.id]: both(graveRewindScript),
  [striker.id]: both(strikerScript),
  [wardrum.id]: both(wardrumScript),
  [deckAsker.id]: both(deckAskerScript),
  [deckWatcher.id]: both(deckWatcherScript),
  [recurring.id]: { base: recurringScript(false), radiant: recurringScript(true) },
  [snare.id]: both(snareScript),
  [spark.id]: both(sparkScript),
  [mill.id]: both(millScript),
  [graveWatcher.id]: both(graveWatcherScript),
  [crossPick.id]: both(crossPickScript),
};

export const PROMPT_SCRIPTS: Record<string, CardScripts> = {
  ...BASE_SCRIPTS,
  ...Object.fromEntries(
    QUICKDRAW_OF.map((card) => [quickdrawOf(card).id, quickdraw(BASE_SCRIPTS[card.id] ?? both({}))]),
  ),
};

/** Add these fixtures to whatever the harness registered (`newGame` registers its own first). */
export function registerPromptFixtures(): void {
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries(PROMPT_DEFS.map((card) => [card.id, card])) });
  registerScripts({ ...registeredScripts(), ...PROMPT_SCRIPTS });
}
