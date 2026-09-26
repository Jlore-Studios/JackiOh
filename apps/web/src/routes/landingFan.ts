// The hand the landing hero fans out (routes/landing.tsx): real Core cards drawn by the cards
// module's CardFace, as they look everywhere else in the game, rather than a second card style of the
// landing's own (integration QA: glyph gems, an empty name ribbon and grey bars for rules text).
//
// The four definitions are copied out of packages/cards/catalog.json so the landing, the one page
// every visitor loads, does not bundle the whole catalog for decoration. landingFan.test.ts holds
// each copy equal to its catalog entry, so a catalog edit cannot leave the landing showing an old
// card.

import type { CardDef } from "@jackioh/shared";

/** One face-up card of the fan: its catalog definition and the face it shows. */
export type FanFace = { readonly def: CardDef; readonly radiant: boolean };

/** Rare Spell, Epic Trap, a Common Unit on its Radiant face (the middle card), a Legendary Unit. */
const DEFS: readonly CardDef[] = [
  {
    id: "core-017",
    index: "17",
    name: "Flood",
    set: "Core",
    type: "Spell",
    tags: [],
    rarity: "Rare",
    token: false,
    cost: 3,
    base: { keywords: [], text: "Bounce all Units." },
    radiant: {
      keywords: [],
      text: "Choose one: Bounce all Units, bounce all enemy Units, or destroy all enemy Units. Then draw 1.",
    },
  },
  {
    id: "core-041",
    index: "41",
    name: "Sheepish",
    set: "Core",
    type: "Trap",
    tags: [],
    rarity: "Epic",
    token: false,
    cost: 1,
    refs: ["core-t-sheep", "core-055"],
    base: {
      keywords: [],
      text: "When your opponent plays a Unit: Transform it into a Sheep Token.",
    },
    radiant: {
      keywords: [],
      text: "When your opponent plays a Unit: Transform it into a Sheep Token. Add a Lava Golem to your hand. It costs (0).",
    },
  },
  {
    id: "core-011",
    index: "11",
    name: "Tempo Timmy",
    set: "Core",
    type: "Unit",
    tags: ["Human"],
    rarity: "Common",
    token: false,
    cost: 1,
    base: {
      attack: 3,
      health: 3,
      keywords: [{ kind: "Rush" }, { kind: "First Strike" }],
      text: "Rush, First Strike",
    },
    radiant: {
      attack: 6,
      health: 6,
      keywords: [{ kind: "Charge" }, { kind: "First Strike" }],
      text: "Charge, First Strike",
    },
  },
  {
    id: "core-092",
    index: "92",
    name: "Felinor Fiender",
    set: "Core",
    type: "Unit",
    tags: ["Human"],
    rarity: "Legendary",
    token: false,
    cost: 2,
    base: {
      attack: 5,
      health: 7,
      keywords: [{ kind: "Stack" }],
      text: "Stack. Has the stats of all your Felinors, including those under Stack.",
    },
    radiant: {
      attack: 10,
      health: 14,
      keywords: [{ kind: "Stack" }],
      text: "Stack. Has twice the stats of all your Felinors, including those under Stack.",
    },
  },
];

/** The face-up cards, left to right; a card back follows them as the fan's fifth card. */
export const LANDING_FAN: readonly FanFace[] = DEFS.map((def, index) => ({ def, radiant: index === 2 }));
