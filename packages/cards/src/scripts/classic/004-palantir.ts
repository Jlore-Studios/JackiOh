// C #4 Palantir (SPEC §8.6 row 4). Field Spell, cost 1, Legendary.
//   Base:    "Aura: Your opponent can't draw more than {drawLimit|card|cards} each turn.
//             When your opponent plays a Book, you may Tribute this to steal it."
//   Radiant: "Aura: Your opponent can't draw more than {drawLimit|card|cards} each turn.
//             When your opponent plays a Spell, steal it. Once the Spells this has stolen cost
//             ({threshold}) or more in total, Tribute this."
//
// The Aura is a draw limit (§2.4, B5 E3) on the opponent, on every turn, the start-of-turn draw
// included: a draw past the limit does not happen at all (no card moves, no fatigue, nothing is cast
// on draw), and with several limits the lowest holds (`Script.drawLimit`, read by `draw.ts` while this
// card stands on the field; it goes when Palantir leaves).
//
// The steal answers the opponent's announce (`cardAnnounced`, §10.5 step 3a) as a Counter (§6.3, B5
// E1, R448): the card is cancelled before it moves and goes to your hand as your own card (B5 E2, the
// Steal off the field: its owner changes, §3.2, R12; a full hand burns it into your graveyard, R317). A
// cast is a play and is answered too (R70).
//   - Base: only a Spell with the Book tag, and "Tribute this to steal it" is a price, so it asks its
//     controller in a prompt during the opponent's turn (a non-active player's prompt, on R79's
//     30-second prompt clock): "steal" sacrifices Palantir, then counters the Book into your hand;
//     "pass" lets the Book resolve and Palantir stays. Palantir is not a Trap, so it answers after the
//     traps the announce woke (§10.3), and the paused play waits on `state.work` meanwhile (R113).
//   - Radiant: every opponent Spell, with no question. Each one's own cost as it stands out of play
//     (R65: its override or printed cost plus its cost changes, no player discounts, an X Spell 0,
//     R396) is added to `memory.stolenCost`, and once that total reaches ({threshold}) Palantir
//     Tributes itself — never before.

import type { GameEvent } from "@jackioh/shared";
import type { Effect, EffectContext, Script, TriggerDef } from "@jackioh/engine";
import { defOf, findInstance, ownCost, param, recalled } from "@jackioh/engine";
import { chooseMode, chosenOptions, counterPlay, remember, sacrifice } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-004");

type Announced = Extract<GameEvent, { type: "cardAnnounced" }>;

const STEAL = "steal";
const PASS = "pass";
/** The continuation the base face's question re-enters. */
const DECIDE = "palantir-decide";
/** Where the Radiant face keeps the cost it has stolen so far. */
const STOLEN_COST = "stolenCost";

/** The opponent's announced Spell (a Book only, on the base face), or null. */
function answered(ctx: EffectContext & { event: GameEvent }, booksOnly: boolean): Announced | null {
  const event = ctx.event;
  if (event.type !== "cardAnnounced" || event.player === ctx.controller || event.cardType !== "Spell") return null;
  if (booksOnly && !defOf(ctx.state, event.defId).tags.includes("Book")) return null;
  return event;
}

const drawLimit: Script["drawLimit"] = ({ state, self, radiant }) => [
  { player: "enemy", count: param({ state, self, radiant }, "drawLimit") },
];

/** Base: "you may Tribute this to steal it" — a question to its controller. */
const mayStealBook: TriggerDef = {
  id: "palantir-book",
  on: ["cardAnnounced"],
  when: (ctx) => answered(ctx, true) !== null,
  run: (ctx) => {
    const book = answered(ctx, true);
    if (book === null) return [];
    return [
      chooseMode({
        options: [STEAL, PASS],
        step: DECIDE,
        prompt: "Tribute Palantir to steal the Book?",
        data: { stolen: book.instanceId },
      }),
    ];
  },
};

function decide(ctx: EffectContext): Effect[] {
  if (!chosenOptions(ctx).includes(STEAL)) return [];
  return [
    sacrifice({ target: { of: "self" } }),
    counterPlay({ to: "thief", target: { of: "instance", instanceId: String(ctx.data.stolen) } }),
  ];
}

/** Radiant: steal every Spell, count what it cost, and Tribute this at the threshold. */
const stealSpells: TriggerDef = {
  id: "palantir-spell",
  on: ["cardAnnounced"],
  when: (ctx) => answered(ctx, false) !== null,
  run: (ctx) => {
    const spell = answered(ctx, false);
    if (spell === null) return [];
    const card = findInstance(ctx.state, spell.instanceId);
    const cost = card === undefined ? 0 : (ownCost(ctx.state, card) ?? 0);
    const before = recalled(ctx, STOLEN_COST);
    const total = (typeof before === "number" ? before : 0) + Math.max(0, cost);
    const steal = counterPlay({ to: "thief", target: { of: "instance", instanceId: spell.instanceId } });
    const kept = remember({ key: STOLEN_COST, value: total });
    return total >= param(ctx, "threshold") ? [steal, kept, sacrifice({ target: { of: "self" } })] : [steal, kept];
  },
};

export const base: Script = { drawLimit, triggers: [mayStealBook], resume: { [DECIDE]: decide } };

export const radiant: Script = { drawLimit, triggers: [stealSpells] };
