// C+ #74 Twice Forward One Step Backwards (SPEC §8.7 row 74, R425): a Field Trap that counts the
// opponent's plays from the moment it is set and, on every `plays`-th one, once that card has resolved,
// fuses it (or, on the Radiant face, a Radiant copy of it) into itself and gains Brittle.
//
// The count is the card's own (`memory.plays`, §10.1), so it survives JSON, a replay and the Fuse that
// keeps this instance (R77 keeps the target's memory; nothing here is a `remember` note, so a Fuse
// never re-roots it, `work.rerootRemembered`). It is kept by the trap trigger's own predicate: §10.3
// offers a trap each event once (`traps.fireTrap`; one a predicate declined is never owed it again,
// R99), and the predicate is the only part of a trap that runs without firing it — a fired Field Trap
// is face-up from then on (R33), and this one must stay face-down until it first activates. So the
// predicate counts each play as it is played (`cardPlayed`, §10.5 step 4), notes the card an even
// count names (`memory.fuseOn`), and admits that card's `cardResolved` — firing the trap — only when
// there is a card to fuse; with nothing left to fuse it reveals and gains its Brittle there (R639).
// Counting plays, not resolutions, keeps "every second card your opponent plays" right when a play
// casts a card that resolves before it (R70): the cast is the later play.
// ponytail: a trap predicate that writes its card's own counter; a "watch without firing" trigger kind in
// traps.ts is the upgrade path if a second card ever needs one.

import type { GameEvent } from "@jackioh/shared";
import { startPrintedBrittle } from "../brittleCount";
import { fuseCards } from "../effects/fuse";
import { gainBrittle } from "../effects/brittle";
import { param } from "../params";
import type { EffectContext, TriggerDef } from "../script";
import { findInstance, type CardInstance, type GameState } from "../state";

/** §10.1: where the card keeps the opponent's plays since it was set (R425). */
export const TWICE_FORWARD_PLAYS_KEY = "plays";
/** §10.1: the plays an even count named, still to resolve (a play's cast resolves before it, R70). */
const FUSE_ON_KEY = "fuseOn";

/** The declared numbers the text reads (`params`, R386): every N plays, and the Brittle each fuse gains. */
const EVERY = "plays";
const GAIN = "brittleGain";

type Resolved = Extract<GameEvent, { type: "cardResolved" }>;

/** R425: the opponent's plays counted on this card so far — 0 before its first. */
export function twiceForwardPlays(card: Pick<CardInstance, "memory">): number {
  const value = card.memory[TWICE_FORWARD_PLAYS_KEY];
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.trunc(value) : 0;
}

/** The plays an even count named that have not resolved yet, read back defensively (JSON). */
function owed(card: Pick<CardInstance, "memory">): string[] {
  const value = card.memory[FUSE_ON_KEY];
  return Array.isArray(value) ? value.filter((id): id is string => typeof id === "string") : [];
}

/**
 * §10.5 step 7: a play or cast of the opponent's that an even count named has resolved (R70: a cast
 * counts; a countered card is never played, R448, so it never counts).
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
 * R639: turn the card face-up, public to both players, and start the printed Brittle its face-down
 * arrival never started. Firing already turned a fusing card face-up; the nothing-left-to-fuse path
 * reveals it here, so no Brittle ever sits on an unrevealed card.
 */
function revealSelf(state: GameState, self: CardInstance): void {
  self.faceUp = true;
  startPrintedBrittle(state, self);
}

/**
 * R425: the trigger both faces carry. `radiantCopy` is the Radiant face's "a Radiant copy of it is
 * fused into this", which leaves the played card where it is and so always has something to fuse.
 */
export function twiceForwardTrigger(args: { radiantCopy: boolean }): TriggerDef {
  return {
    id: "twice-forward",
    on: ["cardPlayed", "cardResolved"],
    when: (ctx) => {
      const self = ctx.self;
      const event = ctx.event;
      if (self === null) return false;
      if (event.type === "cardPlayed" && event.player !== ctx.controller) {
        const plays = twiceForwardPlays(self) + 1;
        self.memory[TWICE_FORWARD_PLAYS_KEY] = plays;
        if (plays % param(ctx, EVERY) === 0) self.memory[FUSE_ON_KEY] = [...owed(self), event.instanceId];
        return false;
      }
      const play = opponentsPlay(ctx);
      if (play === null || !owed(self).includes(play.instanceId)) return false;
      self.memory[FUSE_ON_KEY] = owed(self).filter((id) => id !== play.instanceId);
      if (args.radiantCopy || stillThere(ctx.state, play) !== null) return true;
      // Nothing left to fuse: the card reveals and its Brittle starts now (R639 — no Brittle while
      // unrevealed), then the gain lands on the started count. The trap stays armed (R33).
      revealSelf(ctx.state, self);
      gainBrittle({ instanceId: self.id, n: param(ctx, GAIN) }).apply(ctx);
      return false;
    },
    run: (ctx) => {
      const self = ctx.self;
      const play = opponentsPlay(ctx);
      if (self === null || play === null) return [];
      // The first fuse reveals the card (R639): firing turned it face-up, and its printed Brittle
      // starts now, before the gain lands on it.
      revealSelf(ctx.state, self);
      const fused = args.radiantCopy
        ? fuseCards({ defIds: [play.defId], targetInstanceId: self.id, radiantIngredients: true })
        : fuseCards({ instanceIds: [play.instanceId], targetInstanceId: self.id });
      return [fused, gainBrittle({ instanceId: self.id, n: param(ctx, GAIN) })];
    },
  };
}
