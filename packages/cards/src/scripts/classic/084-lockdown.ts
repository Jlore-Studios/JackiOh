// C #84 Lockdown (SPEC §8.6 row 84). (2) Field Spell, Rare.
//   Base:    "Indestructible\nAfter a permanent is played, Lock its zone.\nActivate: Tribute this."
//   Radiant: "Indestructible\nAfter your opponent plays a permanent, Lock its zone.\nActivate: Tribute this."
//   Engine:  "A trigger on `cardPlayed` of a permanent (either player's; Radiant: the opponent's) that
//            Locks (Lock / Unlock, §6.3) the zone the card entered; the card stays, since Lock evicts
//            nothing. The designer's "cast" is "played": a cast by an effect counts (R70); a summon
//            that is no play (a token, a Recruit) does not; Lockdown doesn't answer its own arrival
//            (R119). Activate (§6.2, R384), once per turn: Tribute this, which bypasses Indestructible
//            (§6.3 Sacrifice). Tunes: none."
//
// The trigger answers `cardPlayed` — a play or a cast (R70), never a summon, a Recruit, a Reborn or a
// countered play, none of which emits it — and `lockPlayedZone` Locks the zone the played card stands
// in, the card staying (§3.2); a played Spell stands in none and locks nothing. R119: a permanent that
// is not a trap gets no filter for its own arrival, so the trigger passes over the `cardPlayed` naming
// Lockdown itself. The `locked` event names the zone only, so a face-down Trap's zone locks without the
// other player learning the Trap (R33). The Radiant face answers only the opponent's plays.
//
// Indestructible is the catalog keyword: a destroy leaves it (R46); an exile removes it. The Activate is
// R384's once-per-turn ability whose cost is "Tribute this" (`tributeSelf`): a Sacrifice, which
// bypasses Indestructible (§6.3), and which is the whole of the ability. The Locks it made stay after it
// leaves.
//
// Rulings: R33, R46, R70, R119, R384. Its proof: `test/classic/084-lockdown.test.ts`.

import type { Script, TriggerDef } from "@jackioh/engine";
import { lockPlayedZone } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-084");

function lockAfterPlay(opponentOnly: boolean): TriggerDef {
  return {
    id: "lock-played-zone",
    on: ["cardPlayed"],
    run: (ctx) => {
      const event = ctx.event;
      if (event.type !== "cardPlayed") return [];
      // R119: not its own arrival.
      if (event.instanceId === ctx.self?.id) return [];
      if (opponentOnly && event.player === ctx.controller) return [];
      return [lockPlayedZone({ event })];
    },
  };
}

function lockdown(opponentOnly: boolean): Script {
  return {
    triggers: [lockAfterPlay(opponentOnly)],
    activations: [{ id: "tribute", label: "Tribute this", uses: 1, cost: { tributeSelf: true }, run: () => [] }],
  };
}

export const base: Script = lockdown(false);

export const radiant: Script = lockdown(true);
