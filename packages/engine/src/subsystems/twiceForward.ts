// C+ #74 Twice Forward One Step Backwards (SPEC §8.7 row 74, R425): a Field Trap that counts the
// opponent's plays from the moment it is set and, on every `plays`-th one, once that card has resolved,
// fuses it (or, on the Radiant face, a Radiant copy of it) into itself and gains Brittle.
//
// The count is the card's own (`memory.plays`, §10.1), so it survives JSON, a replay and the Fuse that
// keeps this instance (R77 keeps the target's memory; nothing here is a `remember` note, so a Fuse
// never re-roots it, `work.rerootRemembered`). It is kept by the trap trigger's own predicate: §10.3
// offers a trap each event once (`traps.fireTrap`; one a predicate declined is never owed it again,
// R99), and the predicate is the only part of a trap that runs without firing it — a fired Field Trap
// is face-up from then on (R33), and this one must stay face-down until it first fuses. So the
// predicate counts the play, and admits it — firing the trap — only on a count that fuses; a count
// with nothing left to fuse gains its Brittle there, face-down.
// ponytail: a trap predicate that writes its card's own counter; a "watch without firing" trigger kind in
// traps.ts is the upgrade path if a second card ever needs one.

import type { GameEvent } from "@jackioh/shared";
import { fuseCards } from "../effects/fuse";
import { gainBrittle } from "../effects/brittle";
import { param } from "../params";
import type { EffectContext, TriggerDef } from "../script";
import { findInstance, type CardInstance, type GameState } from "../state";

/** §10.1: where the card keeps the opponent's plays since it was set (R425). */
export const TWICE_FORWARD_PLAYS_KEY = "plays";

/** The declared numbers the text reads (`params`, R386): every N plays, and the Brittle each fuse gains. */
const EVERY = "plays";
const GAIN = "brittleGain";

type Resolved = Extract<GameEvent, { type: "cardResolved" }>;

/** R425: the opponent's plays counted on this card so far — 0 before its first. */
export function twiceForwardPlays(card: Pick<CardInstance, "memory">): number {
  const value = card.memory[TWICE_FORWARD_PLAYS_KEY];
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.trunc(value) : 0;
}

/**
 * §10.5 step 7: a play or cast of the opponent's has resolved (R70: a cast counts; a countered card
 * never resolves, R448, so it never counts).
 */
function opponentsPlay(ctx: EffectContext & { event: GameEvent }): Resolved | null {
  const event = ctx.event;
  return event.type === "cardResolved" && event.player !== ctx.controller ? event : null;
}

/**
 * R425, R589: the played card "if it still exists: a Unit on the field, a Spell in the graveyard, a trap
 * in the backrow" — on the field (either row) or in a graveyard now. Exiled (a pile nothing takes a card
 * back out of, §6.3), back in a hand or a deck, or ceased to exist, it is no card left to fuse.
 */
function stillThere(state: GameState, play: Resolved): CardInstance | null {
  const card = findInstance(state, play.instanceId);
  return card !== undefined && (card.zone.z === "field" || card.zone.z === "graveyard") ? card : null;
}

/**
 * R425: the trigger both faces carry. `radiantCopy` is the Radiant face's "a Radiant copy of it is
 * fused into this", which leaves the played card where it is and so always has something to fuse.
 */
export function twiceForwardTrigger(args: { radiantCopy: boolean }): TriggerDef {
  return {
    id: "twice-forward",
    on: ["cardResolved"],
    when: (ctx) => {
      const self = ctx.self;
      const play = opponentsPlay(ctx);
      if (self === null || play === null) return false;
      const plays = twiceForwardPlays(self) + 1;
      self.memory[TWICE_FORWARD_PLAYS_KEY] = plays;
      if (plays % param(ctx, EVERY) !== 0) return false;
      if (args.radiantCopy || stillThere(ctx.state, play) !== null) return true;
      // Nothing left to fuse: the Brittle still comes, and the trap stays as it was (R33).
      gainBrittle({ instanceId: self.id, n: param(ctx, GAIN) }).apply(ctx);
      return false;
    },
    run: (ctx) => {
      const self = ctx.self;
      const play = opponentsPlay(ctx);
      if (self === null || play === null) return [];
      const fused = args.radiantCopy
        ? fuseCards({ defIds: [play.defId], targetInstanceId: self.id, radiantIngredients: true })
        : fuseCards({ instanceIds: [play.instanceId], targetInstanceId: self.id });
      return [fused, gainBrittle({ instanceId: self.id, n: param(ctx, GAIN) })];
    },
  };
}
