// T-AI-9 Refusal (SPEC §8.7 row T-AI-9, §7, B8). (1) Trap, AI, Token.
//   Base:    "Reveals when your opponent plays a Spell that targets one of your Units: Counter it."
//   Radiant: "Reveals when your opponent plays a Spell that targets you or one of your cards: Counter
//            it. Draw 1."
//   Engine:  "Counter (§6.3), in §10.5's announce window (`cardAnnounced`): a Spell whose declared
//            targets (R81) include a Unit you control (Radiant: your hero or any card you control or
//            hold). The countered Spell goes to its owner's graveyard, treated as never played: no
//            `cardPlayed`, no counts, no Echo repeats; its mana stays spent. Picks made during
//            resolution are not targets of the play. The Radiant's draw is its controller's. Tunes: none."
//
// E1, R448: the announce names the play's declared targets (a card by its id, a hero as `hero-<p>`), so
// the condition is a read of those ids against your side as the window opens — your units on the field
// (a carried Unit included, R446), or on the Radiant face your hero, your field and your hand. It lives
// in `when` (R99), so any other Spell, a Field Spell, a Trap or a Unit leaves it armed and face-down; a
// cast is announced like a play (R70), so a cast Spell that declared one of yours sets it off too.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import type { GameState, Script, TrapTrigger } from "@jackioh/engine";
import { activeUnitsOf, findInstance } from "@jackioh/engine";
import { counterPlay, draw } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-t-ai-09");

/** §8.7: the Radiant face's "Draw 1". An AI card declares no params (B8). */
const RADIANT_DRAW = 1;

/** "One of your Units": a Unit of yours on the field, as the announce names it. */
function yourUnit(state: GameState, you: PlayerId, id: string): boolean {
  return activeUnitsOf(state, you).some((unit) => unit.id === id);
}

/** "You or one of your cards": your hero, or a card you control on the field or hold in your hand. */
function youOrYours(state: GameState, you: PlayerId, id: string): boolean {
  if (id === `hero-${you}`) return true;
  const card = findInstance(state, id);
  return card !== undefined && (card.zone.z === "field" || card.zone.z === "hand") && card.controller === you;
}

/** The opponent's announced Spell whose declared targets `reaches` one of yours, or null. */
function refused(event: GameEvent, state: GameState, you: PlayerId, reaches: typeof yourUnit): string | null {
  if (event.type !== "cardAnnounced" || event.player === you || event.cardType !== "Spell") return null;
  return event.targets.some((id) => reaches(state, you, id)) ? event.instanceId : null;
}

function refusal(reaches: typeof yourUnit, drawn: number): TrapTrigger {
  return {
    id: "refusal",
    on: ["cardAnnounced"],
    when: ({ event, state, controller }) => refused(event, state, controller, reaches) !== null,
    run: ({ event, state, controller }) => [
      counterPlay({ target: { of: "instance", instanceId: refused(event, state, controller, reaches) ?? "" } }),
      ...(drawn > 0 ? [draw({ count: drawn })] : []),
    ],
  };
}

export const base: Script = { triggers: [refusal(yourUnit, 0)] };

export const radiant: Script = { triggers: [refusal(youOrYours, RADIANT_DRAW)] };
