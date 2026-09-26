// #92 Felinor Fiender (SPEC §8.4, §3.2, §10.4 layer 2, R13, R39, R362, BUILD M4-T4 row 92).
//
// Base: "Stack. Has the stats of all your Felinors, including those under Stack." Radiant: "Stack.
// Has twice the stats of all your Felinors, including those under Stack." (patch v0.1.1: the Radiant
// face traded Charge for twice the count). Both faces print Stack alone (§8 Conventions), so §10.4
// layer 1 already grants it and this file grants nothing. The printed 5/7 → 10/14 stays: "has the
// stats of" is R39's printed-plus-the-sum, and R362 doubles only the sum on the Radiant face.
//
// The whole card is §10.4 LAYER 2, the set-stat layer that exists for this card alone, expressed
// through `Script.setStat` ("a card that sets its own stats from the board (#92 Felinor Fiender,
// R39)"). §10.4 writes layer 2 as "Felinor Fiender adds the sum of your Felinors' LAYER-4 stats",
// so each Felinor is measured with `statsWithBuffs` — layers 1 to 4, printed face plus permanent
// buffs, BEFORE auras. `unitView` would be layer 5 and is the wrong number here; it would also make
// the layers recurse, since `unitView` is what calls this hook.
//
// R13 is the reason both zone readers appear: "cards under a Stack are not on the field except for
// Felinor Fiender's count" — the one dormancy exception in the game (§3.2). `activeUnitsOf` gives
// the top card of every pile and `dormantUnitsOf` the cards beneath them.
//
// R39 (decide, `FIENDER_STATS_MODE = "printed-plus-sum"`): "printed plus the combined Felinor
// stats, never below printed", so each sum floors at 0 — a Felinor carrying a negative health buff
// can pull the total down toward printed but never past it.
//
// Fiender itself is tagged Human, not Felinor (§8, catalog), so "all your Felinors" cannot include
// it. It is excluded by id anyway, because Fuse unions its ingredients' tags (R77) and a fused
// Felinor Fiender would otherwise count its own layer-4 stats into its own layer 2. R131 rules on
// exactly that: it never counts itself even once a Fuse has given it the tag, matched by instance
// rather than by tag, and a second Fiender contributes only printed and buffed stats (R116), so the
// layer cannot recurse.
//
// Layer 2 is now live: `unitView` in `engine/src/layers.ts` calls `scriptOf(instance).setStat` after
// layer 1's printed face and BEFORE the layer-4 buffs and layer-5 auras, and it ADDS what the hook
// returns, flooring each component at 0 (R116). So this hook returns the sum alone — adding the
// printed face here would count it twice — and R115 skips the hook entirely on a Vanilla'd card.

import type { CardInstance, GameState, Script } from "@jackioh/engine";
import { activeUnitsOf, defOf, dormantUnitsOf, statsWithBuffs } from "@jackioh/engine";
import type { Tag } from "@jackioh/shared";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-092");

/** §5, §8: the tag that makes a unit one of "your Felinors". */
const FELINOR: Tag = "Felinor";

/**
 * "All your Felinors, including ones under a Stack": every Felinor this card's controller has on
 * the field, dormant ones included (R13, §3.2). Reading state to find them is not mutation; nothing
 * here writes (CLAUDE.md rule 5).
 */
function felinorsOf(state: GameState, self: CardInstance): CardInstance[] {
  const mine = [...activeUnitsOf(state, self.controller), ...dormantUnitsOf(state, self.controller)];
  return mine.filter(
    (unit) => unit.id !== self.id && defOf(state, unit.defId).tags.includes(FELINOR),
  );
}

/**
 * §10.4 layer 2, which ADDS this result to layer 1's printed face — so this returns only the sum,
 * never printed plus the sum (R116). Each Felinor contributes its layer-4 stats (`statsWithBuffs`):
 * printed plus permanent buffs, before auras, which is also what stops the layers recursing.
 */
function setStatTimes(multiple: number): NonNullable<Script["setStat"]> {
  return ({ state, self }) => {
    let attack = 0;
    let maxHealth = 0;
    for (const felinor of felinorsOf(state, self)) {
      const stats = statsWithBuffs(state, felinor);
      attack += stats.attack;
      maxHealth += stats.maxHealth;
    }

    // R116: the hook returns the DELTA layer 2 adds to the printed face, not an absolute total, so
    // the printed stats must not be added here — layer 2 does that. R39's "never below printed" is
    // the floor at 0: a Felinor carrying a negative buff can pull the sum toward 0 but not past it.
    return { attack: Math.max(0, attack) * multiple, maxHealth: Math.max(0, maxHealth) * multiple };
  };
}

/** "Has the stats of all your Felinors": the sum once (R39). */
const BASE_MULTIPLE = 1;
/** "Has twice the stats of all your Felinors": the sum twice (R362). */
const RADIANT_MULTIPLE = 2;

export const base: Script = { setStat: setStatTimes(BASE_MULTIPLE) };

export const radiant: Script = { setStat: setStatTimes(RADIANT_MULTIPLE) };
