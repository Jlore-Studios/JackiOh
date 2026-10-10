// R502: `cardPlayed` after its own `drawn` identifies Cast on draw (SPEC §2.4, R58).
// Its announcement (B5 E1) and prompts can fall between those events.
// R97: the sentinel matches either id; R227: a face-down cast carries its drawn id as `formerId`.

import type { GameEvent } from "@jackioh/shared";

import { HIDDEN_ID } from "../game/animations.ts";

function sameCard(a: string, b: string): boolean {
  return a === b || a === HIDDEN_ID || b === HIDDEN_ID;
}

/** Whether `events[at]` is a `cardPlayed` whose card was cast as it was drawn. */
export function castOnDrawAt(events: readonly GameEvent[], at: number): boolean {
  const played = events[at];
  if (played === undefined || played.type !== "cardPlayed") return false;
  const ids = [played.instanceId, ...(played.formerId === undefined ? [] : [played.formerId])];
  const same = (id: string): boolean => ids.some((own) => sameCard(own, id));
  for (let i = at - 1; i >= 0; i -= 1) {
    const before = events[i];
    if (before === undefined) return false;
    if (before.type === "cardAnnounced" && before.player === played.player && same(before.instanceId)) continue;
    // Its caster's choices can occur before it is announced.
    if ((before.type === "promptOpened" || before.type === "promptAnswered") && before.player === played.player) continue;
    return before.type === "drawn" && before.player === played.player && same(before.instanceId);
  }
  return false;
}
