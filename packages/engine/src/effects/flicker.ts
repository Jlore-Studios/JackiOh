// Flicker (docs/classic-sets.md B5 E22, R444): "the card leaves the field and re-enters the same zone
// at once" — R78's reset, summoning sick, no Cry, no Death — and "it counts as summoned". The move is
// `zones.flickerInPlace`; `flickerCard` is the whole of it with its events, for an effect list (the
// `flicker` verb below) and for an engine sequence that holds a sink rather than a context: the "would
// die" window of §4.5 step 1 flickers the dying units of Classic #14 Shadowstep's Radiant face instead.

import { defOf } from "../catalog";
import { animateOnEntry, isAnimated, type FieldSink } from "../animated";
import type { Effect } from "../script";
import type { CardInstance } from "../state";
import { flickerInPlace, freshFaceDownId, isCarried, landsFaceDown } from "../zones";
import { cardsInScope, instanceOf, type BoardScope, type TargetSpec } from "./targets";

/**
 * B5 E22: flicker one card acting on the field. It leaves the field — every effect aimed at it ends
 * (R174), the triggers it queued there go, an animated card's home is let go — and re-enters its zone
 * in the same place at once: reset (R78), summoning sick (R83), in Attack Position, on the same side.
 * No Cry (R1: it was not played) and no Death (it went to no graveyard). A unit token comes back like
 * any card (R444). A Trap re-enters face-down with a fresh id (§3.2, R33, R227); an animated card is
 * still a Unit in its unit zone and stays face-up; a Field Spell is public. Emits `flickered`, then the
 * `summoned` it counts as. False, changing nothing, for a card not acting on the field.
 */
export function flickerCard(sink: FieldSink, card: CardInstance): boolean {
  const state = sink.state;
  const zone = card.zone;
  if (zone.z !== "field") return false;
  const wasAnimated = isAnimated(state, card);
  const carried = isCarried(state, card);
  const oldId = card.id;
  if (!flickerInPlace(state, card)) return false;

  const faceDown = zone.row === "backrow" && !carried && landsFaceDown(state, card, "backrow");
  const formerId = faceDown ? freshFaceDownId(state, card) : undefined;
  if (wasAnimated || defOf(state, card.defId).type === "Field Spell") card.faceUp = true;

  sink.events.push({ type: "flickered", player: zone.player, instanceId: oldId, defId: card.defId, row: zone.row, lane: zone.lane });
  sink.events.push({
    type: "summoned",
    player: zone.player,
    instanceId: card.id,
    defId: card.defId,
    row: zone.row,
    lane: zone.lane,
    ...(formerId === undefined ? {} : { formerId }),
  });
  // B3.1 rule 4: an Animated Field Spell animates as it enters the field, and it has just re-entered.
  if (zone.row === "backrow" && !carried) animateOnEntry(sink, card);
  return true;
}

/**
 * B5 E22: flicker a card (`target`, the card running the script by default) or every card a board
 * scope names, read once before any of them moves, in R68's order (§3.1, §3.2).
 */
export function flicker(args: { target?: TargetSpec; scope?: BoardScope } = {}): Effect {
  return {
    kind: "flicker",
    apply(ctx): void {
      if (args.scope !== undefined) {
        for (const card of cardsInScope(ctx, args.scope)) flickerCard(ctx, card);
        return;
      }
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (card !== null) flickerCard(ctx, card);
    },
  };
}
