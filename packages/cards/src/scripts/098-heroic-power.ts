// #98 Heroic Power (SPEC §8.5, §6.2 "Start of Game", Quickdraw, Activate; R43, R103, R352, R752–R761).
// Field Spell, tags Quickdraw, cost (0), Mythic.
//   Base:    "Indestructible. Start of game: Gain one of 13 random powers, each 'Activate: Spend
//             (X)': (3) Expedition Map; (1) Life Tap; (1) Steady Shot; (2) Ranching; (1) Cat Cafe;
//             (1) Ping; (2) Witness Value; (2) Stitching; (1) Armor Up; (2) Die Insect; (2) KY
//             Brainstorm; (2) Pluck; (3) Terminus Tricks" — each power's words are its catalog line.
//   Radiant: every power's Radiant words, Armor Up named Tank Up (R757).
//
// THE THIRTEEN POWERS LIVE IN `subsystems/heroPower.ts`, NOT HERE. R43 makes this card a subsystem:
// `HERO_POWERS` is the table with each power's X, its names, its base words and its Radiant words;
// `rollPower` is the roll and `heroPower` the continuation a Discover comes back to. Since the Heroic
// Power patch (R752) each power is an Activate ability (R384) — `powerAbilities` declares all thirteen,
// each paying its X in mana, declaring Ping's target and present only while the card rolled it — so
// this file is the wiring of the card's `Script` to them. Everything is on the instance
// (`memory.power`, Activate's `memory.activations`), never in a module variable (R43, §10.1), which
// keeps two Heroic Powers in one game independent.
//
// COST (R752). The card costs (0), printed in the catalog, and playing it uses nothing: it has no Cry.
// The power's X is the ability's mana price, paid as it is activated (R384's costs), so the play
// validator, `legalActions` and the client read the printed (0) and the ability's price with no
// special case for this card.
//
// THE PROMPTED POWERS (§10.6). Witness Value, Stitching and Terminus Tricks Discover, and `heroPower`
// is the step they resume at, so the card exposes it under the key the subsystem names
// (`POWER_RESUME`). Ping's target is declared with the activation (R81), so it never prompts.
//
// WHAT THIS FILE DELIBERATELY DOES NOT SAY:
//   * Indestructible is a printed keyword on both catalog faces, so it is a §10.4 layer, not a
//     script. R46: "an Indestructible Field Spell (Heroic Power) simply stays".
//   * Quickdraw is `staticFlags.quickdraw`, which `setup.ts` step 2 reads to put the card in the
//     opening hand instead of a draw (§6.2). The catalog's Quickdraw *tag* is what a filter sees;
//     the flag is what setup sees.
//   * The roll on arrival (R151) is the engine's: `draw.ts`'s `runArrivalHooks` fires this card's
//     `startOfGame` as it reaches a hand or library, and a summon onto the field does the same, so
//     the roll this file declares is the one that runs everywhere.

import type { Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-098");

/**
 * Both faces are the same wiring: `powerAbilities(radiant)` builds each power on the face that is
 * running (§5.2), and `has` keeps only the one the instance rolled.
 */
function heroicPower(radiant: boolean): Script {
  return {
    // §6.2: starts in the opening hand instead of a draw (setup step 2).
    staticFlags: { quickdraw: true },
    // R43: every copy in either hand or library rolls its power after the mulligan (§6.2).
    startOfGame: () => [subsystems.rollPower()],
    // R752: each power is an "Activate: Spend (X)" ability; the card has the one it rolled.
    activations: subsystems.powerAbilities(radiant),
    // §10.6: where a Discover comes back to.
    resume: { [subsystems.POWER_RESUME]: subsystems.heroPower },
  };
}

export const base: Script = heroicPower(false);

export const radiant: Script = heroicPower(true);
