// C+ #40 Appropriations (SPEC §8.7 row 40, E38, E39, R58, R60, R80, R81, R177, R311, R348, R380, R581).
// (X) Spell, Epic. Choose one, with the play (R81); X is at least 1 (R348):
//   Military:   your Units on the field, in your hand and in your deck get +2X Attack and Rush.
//   Education:  shuffle 2X random Radiant Books into your deck; they have Cast on draw and aim at
//               enemies when they harm and at your side when they help.
//   Culture:    each card on your field, in your hand and in your deck has a 10X% chance to become Radiant.
//   Healthcare: your Units on the field, in your hand and in your deck get +2X Health and Armor X.
//   Radiant:    +5X Attack; 5X Books; 25X%; +7X Health and Armor 2X.
//
// Military and Healthcare are E38's buffs and grants over your field (tops of piles), hand and deck,
// carried onto the field as a card enters, silent where a card is hidden (R440). Education's Books are
// non-token Books of every set (R380), repeats allowed (R60), each Radiant and enchanted (E39), R80's
// cap turning the rest away. Culture is `radiantChance`'s roll per card, #42 Eugenics's reading
// (R177, R581): every card is rolled, a Radiant one too, so neither the draws nor the cues count the
// hidden Radiant cards; a library card made Radiant is listed as it went in (R311). Tunes: none beyond X.

import type { Script } from "@jackioh/engine";
import { buffCards, chosenOptions, grantKeywordCards, radiantChance, shuffleRandomFromCatalog } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-040");

const MODES = ["Military", "Education", "Culture", "Healthcare"];
/** One face's multiples of X: attack, Books, percent, health, Armor. */
type Face = { attack: number; books: number; percent: number; health: number; armor: number };
const BASE: Face = { attack: 2, books: 2, percent: 10, health: 2, armor: 1 };
const RADIANT: Face = { attack: 5, books: 5, percent: 25, health: 7, armor: 2 };
const ALL = 100;

const UNITS = { side: "self" as const, zones: ["field" as const, "hand" as const, "library" as const], rows: ["units" as const], types: ["Unit" as const] };

function appropriations(face: Face): Script {
  return {
    modes: [{ kind: "mode", options: MODES }],
    cry: (ctx) => {
      const x = ctx.x;
      switch (chosenOptions(ctx)[0]) {
        case "Military":
          return [buffCards({ scope: UNITS, attack: face.attack * x }), grantKeywordCards({ scope: UNITS, keyword: { kind: "Rush" } })];
        case "Education":
          return [
            shuffleRandomFromCatalog({
              query: { tags: ["Book"] },
              count: face.books * x,
              radiant: true,
              enchantments: [{ kind: "castOnDraw" }, { kind: "targetEnemies" }],
            }),
          ];
        case "Culture":
          return [radiantChance({ zone: ["field", "hand", "library"], chance: Math.min(1, (face.percent * x) / ALL) })];
        case "Healthcare":
          return [
            buffCards({ scope: UNITS, health: face.health * x }),
            grantKeywordCards({ scope: UNITS, keyword: { kind: "Armor", n: face.armor * x } }),
          ];
        default:
          return [];
      }
    },
  };
}

export const base: Script = appropriations(BASE);

export const radiant: Script = appropriations(RADIANT);
