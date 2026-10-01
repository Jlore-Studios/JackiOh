// C #19 Lizard's Breath (SPEC §8.6 row 19). (1) Spell, Rare.
//   Base:    "Deal {damage} damage. Your largest pile adds its effect: Deck, draw {draw}; Graveyard,
//            gain {mana} mana; Exile, deal {extraDamage} more damage. Ties go to the pile listed
//            first." — 2, 1, 2, 4
//   Radiant: "Deal {damage} damage. Your two largest piles add their effects: …" — 4, 1, 2, 4
//   Engine:  "Your own deck, graveyard and exile, counted as it resolves. The damage is one hit on the
//            play's target: 2, or 6 with exile (Radiant 4, or 8); then the draw and the temporary mana.
//            A `preview` (R280) names the pile or piles that would count now. Tunes: damage 2 ↑; draw
//            1 ↑; mana 2 ↑; extra damage 4 ↑."
//
// "Deal N damage" with no target named is targeted (§8's Conventions): any unit or hero, either side,
// chosen with the play (R81).
//
// The piles are counted as the Spell resolves — it is in the resolving zone then, so in none of them
// (§10.5) — and ranked by size, a tie going to the pile the text lists first (Deck, then Graveyard,
// then Exile); the base face takes the largest, the Radiant face the two largest. `countingPiles` is
// that ranking, and both the resolution and the preview read it. So three empty piles pick the Deck,
// which draws (from an empty deck, a fatigue, §2.4).
//
// Exile's effect is part of the one hit: the damage is {damage}, plus {extraDamage} when Exile counts,
// dealt once. Then the Deck's draw and the Graveyard's mana — temporary mana, §2.3 — in that order.
//
// R280: the preview names the pile or piles that count now, one value per pile in rank order, each
// carrying the pile's name as `display` beside its size, after the face's own words for the ranking
// ("Your largest pile", "Your two largest piles"). It reads only the controller's pile sizes, which
// are public (§10.8), never a pile's contents.
//
// The numbers are the declared `damage`, `draw`, `mana` and `extraDamage` (R386), read through `param`.

import { param, zoneCount, type ConditionContext, type EffectContext, type Effect, type GameState, type Script } from "@jackioh/engine";
import { damage, draw, gainMana } from "@jackioh/engine/effects";
import type { PlayerId, PreviewValue, TargetDecl } from "@jackioh/shared";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-019");

/** The three piles in the text's order, which is the tie order. */
const PILES = [
  { zone: "library", name: "Deck" },
  { zone: "graveyard", name: "Graveyard" },
  { zone: "exile", name: "Exile" },
] as const;

type PileName = (typeof PILES)[number]["name"];
type RankedPile = { name: PileName; size: number };

/** §8 Conventions: any unit or hero, either side. */
const targets: TargetDecl[] = [{ kind: "target", min: 1, max: 1, filter: { side: "any", of: ["unit", "hero"] } }];

/** The `count` largest of `player`'s piles, largest first, a tie going to the pile listed first. */
function countingPiles(state: GameState, player: PlayerId, count: number): RankedPile[] {
  const sized = PILES.map((pile, order) => ({ name: pile.name, size: zoneCount(state, player, pile.zone), order }));
  return [...sized]
    .sort((a, b) => b.size - a.size || a.order - b.order)
    .slice(0, count)
    .map(({ name, size }) => ({ name, size }));
}

function lizardsBreath(pilesCounted: number, label: string): Script {
  return {
    targets,
    cry: (ctx: EffectContext): Effect[] => {
      const counting = countingPiles(ctx.state, ctx.controller, pilesCounted).map((pile) => pile.name);
      const extra = counting.includes("Exile") ? param(ctx, "extraDamage") : 0;
      return [
        damage({ to: { of: "chosen" }, amount: param(ctx, "damage") + extra }),
        ...(counting.includes("Deck") ? [draw({ count: param(ctx, "draw") })] : []),
        ...(counting.includes("Graveyard") ? [gainMana({ amount: param(ctx, "mana") })] : []),
      ];
    },
    // R280: the pile or piles that would count now, in rank order.
    preview: (ctx: ConditionContext): PreviewValue[] =>
      countingPiles(ctx.state, ctx.controller, pilesCounted).map((pile) => ({ label, value: pile.size, display: pile.name })),
  };
}

export const base: Script = lizardsBreath(1, "Your largest pile");

export const radiant: Script = lizardsBreath(2, "Your two largest piles");
