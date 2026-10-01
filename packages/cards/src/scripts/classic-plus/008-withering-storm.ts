// C+ #8 Withering Storm (SPEC §8.7 row 8). (2) Spell, Rare.
// One Degrade (R386) on each of {cards} different random cards of the opponent's deck, drawn among the
// cards a Degrade can change (R60; all of them when fewer), then draw {draw}. Radiant: one Degrade on
// every card of their deck. The changes stay hidden from both players while in the deck (R311).
//
// R569: so the count of `degraded` cues never says how many deck cards could change (R440), the pick is
// padded to {cards} (or the deck's size) with cues on cards no change reaches, which the Degrade leaves
// alone with the change `none` — the same cue R440 gives every unchangeable card of the Radiant sweep.

import { param, zoneCards, type CardInstance, type EffectContext, type Script } from "@jackioh/engine";
import { applicableChanges, degrade, draw, forEachCard } from "@jackioh/engine/effects";
import { opponentOf } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-008");

/** The deck cards the base face reaches, in deck order (R242); read once, so a pause resumes over them (R113). */
function picks(ctx: EffectContext): CardInstance[] {
  const deck = zoneCards(ctx.state, opponentOf(ctx.controller), "library");
  const count = Math.min(param(ctx, "cards"), deck.length);
  const changeable = deck.filter((card) => applicableChanges(ctx.state, card, "degrade").length > 0);
  // R60, R129: N different cards, or all of them (and no draw) when there are no more than N.
  const ids = new Set((changeable.length <= count ? changeable : ctx.rng.shuffle(changeable).slice(0, count)).map((card) => card.id));
  for (const card of deck) if (ids.size < count && !changeable.includes(card)) ids.add(card.id);
  return deck.filter((card) => ids.has(card.id));
}

export const base: Script = {
  cry: (ctx) => [
    forEachCard({ cards: picks, each: (instanceId) => degrade({ instanceId }) }),
    draw({ count: param(ctx, "draw") }),
  ],
};

export const radiant: Script = {
  cry: (ctx) => [degrade({ scope: { side: "enemy", zones: ["library"] } }), draw({ count: param(ctx, "draw") })],
};
