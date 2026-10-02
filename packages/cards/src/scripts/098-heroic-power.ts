// #98 Heroic Power (SPEC §8.5, §6.2 "Start of Game"/Activate/Quickdraw, R43, R46, R103, R352, R384,
// patch v0.2.1's R603–R610). Field Spell, tags Quickdraw, cost (0), Mythic.
//   Base:    "Indestructible. Start of game: Gain one of 13 random powers, each 'Activate: Spend (X)':
//             (3) Expedition Map; (1) Life Tap; (1) Steady Shot; (2) Ranching; (1) Cat Cafe; (1) Ping;
//             (2) Witness Value; (2) Stitching; (1) Armor Up; (2) Die Insect; (2) KY Brainstorm;
//             (2) Pluck; (3) Terminus Tricks" — each power's clause is in §8.5 and in `HERO_POWERS`.
//   Radiant: the same thirteen, each on its Radiant clause (Armor Up's is named Tank Up). Indestructible,
//             the start-of-game roll and "Activate: Spend (X)" are kept (§8 Conventions).
//
// THE THIRTEEN POWERS LIVE IN `subsystems/heroPower.ts`, NOT HERE. R43 makes this card a subsystem:
// `HERO_POWERS` is the table with each power's X, its base clause and its Radiant clause;
// `rollPower` is the roll, `heroPowerActivations` the powers as Activate abilities, and `heroPower`
// the continuation a prompted power (a Discover) comes back to. This file is the lines that wire the
// card's `Script` to them, which is what keeps two Heroic Powers in one game independent: everything
// is on the instance (`memory.power`, the Activate count), never in a module variable (R43, §10.1).
//
// COST (R43, patch v0.2.1). The card costs (0) to play, the catalog's printed cost, and playing it
// uses nothing: v0.2.0's "Playing it costs the power's X and activates it once" is gone, so the
// card has no `cost` hook and no `cry`.
//
// THE POWERS (R43, R384). Each is "Activate: Spend (X): …" — once per turn, in its controller's main
// phase, paying the power's X in mana as it is activated (`subsystems/activate.ts`). The card declares
// all thirteen and has only the one it rolled (`ActivationDecl.has`), so only that one is listed,
// shown or accepted. Ping's target is declared, so it travels in the `activate` action (R81) and the
// power can be dragged to it.
//
// WHAT THIS FILE DELIBERATELY DOES NOT SAY:
//   * Indestructible is a printed keyword on both catalog faces, so it is a §10.4 layer, not a
//     script. R46: "an Indestructible Field Spell (Heroic Power) simply stays".
//   * Quickdraw is `staticFlags.quickdraw`, which `setup.ts` step 2 reads to put the card in the
//     opening hand instead of a draw (§6.2). The catalog's Quickdraw *tag* is what a filter sees;
//     the flag is what setup sees.
//   * Steady Shot's damage is the card's declared number `{shot}` (catalog `params`), which its
//     Radiant face Upgrades (R608); `param` reads it in the subsystem.
//
// R43'S LAST ROLL IS THE ENGINE'S, NOT THIS FILE'S (R151). "one created later rolls when it is
// created, and one that ends up in a hand or library with no `memory.power` (a bounced or reset
// instance, R78) rolls as it arrives". `startOfGame` covers the copies R43 names at setup —
// `setup.finishSetup` runs it for every card in both hands and both libraries, a mulliganed one
// included, and `ensurePower` is idempotent so a re-run keeps the roll — and the engine runs the same
// hook on arrival (`draw.ts`'s `runArrivalHooks`, a summon, a Fuse), so nothing is added here for it.

import type { Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-098");

/**
 * Both faces are the same wiring: `heroPowerActivations` gives each face the thirteen abilities on
 * that face's clauses, and `radiant` is passed explicitly so the face that is running decides (§5.2)
 * rather than the instance being read a second time.
 */
function heroicPower(radiant: boolean): Script {
  return {
    // §6.2: starts in the opening hand instead of a draw (setup step 2).
    staticFlags: { quickdraw: true },
    // R43: every copy in either hand or library rolls its power after the mulligan (§6.2).
    startOfGame: () => [subsystems.rollPower()],
    // R43, R384: one "Activate: Spend (X)" per power, of which the card has the one it rolled.
    activations: subsystems.heroPowerActivations(radiant),
    // §10.6: where a Discover's answer comes back to.
    resume: { [subsystems.POWER_RESUME]: subsystems.heroPower },
  };
}

export const base: Script = heroicPower(false);

export const radiant: Script = heroicPower(true);
