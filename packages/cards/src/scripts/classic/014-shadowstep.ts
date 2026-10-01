// C #14 Shadowstep (SPEC §8.6 row 14, §4.5, §6.2 Replacement, §6.3 Flicker; R4, R11, R12, R33, R57,
// R61, R64, R69, R78, R97, R99, R317). Trap, cost 2, Common.
//   Base:    "Activates when any of your Units die: Return them to your hand. They cost ({setCost})."
//   Radiant: "Activates when any of your Units would die: Flicker them instead, so they survive. Add a
//            copy of each to your hand. The copies cost ({setCost})."
//   Engine:  "Base: one firing covers every Unit of yours that one state-check pass collects (§4.5);
//            each card still in a graveyard afterwards goes to its owner's hand (§3.2) with
//            `costOverride` 0, the hand cap applying (R4). Tokens have ceased to exist (R11); a Reborn
//            unit that came back is on the field, not in a graveyard, and is skipped. Radiant: a
//            replacement (§6.2 Replacement) at the "would die" point, §4.5 step 1, before cards move:
//            those units leave the collection and Flicker (§6.3): back in their zones, reset (R78), at
//            full health, summoning sick, no Cry, no Death; a fresh copy of each (radiant flag kept,
//            R57) goes to your hand with `costOverride` 0. Tunes: cost 0 ↓."
//
// THE BASE FACE answers a `destroyed` event of a Unit of yours (R99: the condition is its `when`, so
// an enemy's death leaves it set). §4.5 step 1 collects a pass's deaths together and reports each with
// its own `destroyed`, one after another, so the firing reads the whole run of `destroyed` events the
// one it answers stands in — that pass — and takes every Unit of yours among them: one firing for all
// of them. By the time a trap answers, the pass is over: a unit token has ceased to exist (R11) and a
// Reborn unit is back on the field, so only the cards still in a graveyard go back, each to its
// owner's hand (§3.2, R12) through §2.4's pipeline (a full hand burns it, R4, R317), costing the
// card's number (`costOverride`, `param(ctx, "setCost")`, which R78 keeps in every zone).
//
// "Your Units" are read off the event's `owner`: `destroyed` names the card's owner, not the player
// who controlled it as it died, which is the same card but for a unit stolen across sides.
//
// THE RADIANT FACE is a replacement at "would die" (B5 E5, `Script.replacements`): §4.5 step 1 offers
// it the units the check collected, and it takes its controller's among them — one firing for all —
// and Flickers them in place (E22: reset, full health, summoning sick, no Cry, no Death, no Reborn).
// A face-down Trap that replaces fires (`trapFired`, face-up) and is spent. Its follow-up, owed after
// the event (B5 E5), adds a fresh copy of each flickered card to your hand (its radiant flag kept, R57)
// costing the card's number; a copy in your hand is yours alone to read (R97).

import type { GameEvent } from "@jackioh/shared";
import type { EffectContext, Script } from "@jackioh/engine";
import { findInstance, param, replacementOf } from "@jackioh/engine";
import { addToHand } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-014");

type Destroyed = Extract<GameEvent, { type: "destroyed" }>;

/**
 * The `destroyed` events of the state-check pass the answered one belongs to: the unbroken run of
 * `destroyed` reports it stands in (a card a replacement took elsewhere reports its landing there
 * instead, and still belongs to the run). The answered event itself when the list holds no run.
 */
function passOf(ctx: EffectContext, answered: Destroyed): Destroyed[] {
  const events = ctx.events;
  let at = -1;
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (event?.type === "destroyed" && event.instanceId === answered.instanceId) {
      at = index;
      break;
    }
  }
  if (at < 0) return [answered];
  const inPass = (event: GameEvent | undefined): boolean =>
    event !== undefined && (event.type === "destroyed" || event.type === "exiled");
  let start = at;
  while (inPass(events[start - 1])) start -= 1;
  let end = at;
  while (inPass(events[end + 1])) end += 1;
  return events.slice(start, end + 1).filter((event): event is Destroyed => event.type === "destroyed");
}

export const base: Script = {
  triggers: [
    {
      id: "shadowstep",
      on: ["destroyed"],
      when: (ctx) => ctx.event.type === "destroyed" && ctx.event.owner === ctx.controller,
      run: (ctx) => {
        if (ctx.event.type !== "destroyed") return [];
        const cost = param(ctx, "setCost");
        return passOf(ctx, ctx.event)
          .filter((event) => event.owner === ctx.controller)
          .filter((event) => findInstance(ctx.state, event.instanceId)?.zone.z === "graveyard")
          .map((event) => addToHand({ instance: { of: "instance", instanceId: event.instanceId }, costOverride: cost }));
      },
    },
  ],
};

export const radiant: Script = {
  replacements: [{ id: "shadowstep", on: "wouldDie", instead: { flicker: "yours" }, then: "copies" }],
  resume: {
    copies: (ctx) => {
      const cost = param(ctx, "setCost");
      return (replacementOf(ctx)?.flickered ?? []).map((card) =>
        addToHand({ defId: card.defId, radiant: card.radiant, costOverride: cost }),
      );
    },
  },
};
