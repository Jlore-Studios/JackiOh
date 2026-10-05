// C #42 Transmutable Toxins (SPEC §8.6 row 42, §6.2 Activate, §6.3 Plague Counter, §10.4 layer 5; R60,
// R69, R384). Field Spell, cost 2, Rare.
//   Base:    "Aura: Your Units have +{stats}/+{stats} for each Plague Counter on them. Enemy Units have
//            −{stats}/−{stats} for each Plague Counter on them.\nActivate: Place a Plague Counter on each
//            of {tokens|random Unit|random Units}." (stats 1, tokens 2)
//   Radiant: the same words with stats 2.
//   Engine:  "An aura (§10.4 layer 5) reading each unit's `counters.plague`; −1/−1 lowers max health,
//            so an enemy can die of it at the state check (#46 Suppressive Aura's rule). Activate
//            (§6.2, R384, once per turn): one Plague Counter (§6.3) on each of two different random
//            units on the field, either side (R60: a random pick of N picks N different cards; with
//            one unit on the field, it gets one token); a C #27 Pestilent Slime multiplies its own.
//            Tunes: tokens 2 ↑; stats per token 1 ↑."
//
// THE AURA is §10.4's layer 5, recomputed on every read: for each unit on the field carrying Plague
// Counters, one entry naming that unit with its stats per token (`param(ctx, "stats")`) times its
// tokens — up for its controller's own Units, down for the enemy's. Lowering max health is what kills
// an enemy at the state check when it reaches 0 (§4.5), an Indestructible one too (R69). A unit
// dormant under a Stack pile is not on the field (R13), so it is not read. The hook is a pure read of
// instance data: it never asks the layers for a stat, so it cannot recurse.
//
// THE ACTIVATE (B3.2, R384: once per turn, by its controller, while it acts on the field) is one
// placement of a single token on each of `param(ctx, "tokens")` different random units on the field,
// either side (`placePlagueRandom`, R60: fewer units, fewer placements; none, nothing). Each is a
// placement, so a C #27 Pestilent Slime's multiplier doubles the one it receives (B5 E19). It is not a
// play, so nothing that answers plays sees it.

import type { AuraHook, Script } from "@jackioh/engine";
import { activeUnitsOf, param } from "@jackioh/engine";
import { PLAYER_IDS } from "@jackioh/shared";
import { placePlagueRandom } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-042");

const aura: AuraHook = ({ state, self, radiant }) => {
  const perToken = param({ state, self, radiant }, "stats");
  return PLAYER_IDS.flatMap((player) =>
    activeUnitsOf(state, player).flatMap((unit) => {
      const tokens = unit.counters.plague ?? 0;
      if (tokens <= 0) return [];
      const change = (unit.controller === self.controller ? 1 : -1) * perToken * tokens;
      return [{ applies: (candidate) => candidate.id === unit.id, mod: { attack: change, maxHealth: change } }];
    }),
  );
};

const toxins: Script = {
  aura,
  activations: [
    {
      id: "transmutable-toxins",
      label: "Place a Plague Counter on each of several random Units",
      uses: 1,
      run: (ctx) => [placePlagueRandom({ count: param(ctx, "tokens"), amount: 1, scope: { side: "any", rows: ["units"] } })],
    },
  ],
};

export const base: Script = toxins;

// The same script: the Radiant face differs only in its declared stats per token (2), which `param`
// reads off the running face.
export const radiant: Script = toxins;
