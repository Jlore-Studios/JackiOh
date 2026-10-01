// A kill credited to another unit (R42, R412): Classic+ #19.2 Jungle Loser's Radiant face credits the
// kill its attack makes to Classic+ #19.5 Bot Loser, whose "Whenever this destroys a Unit" then fires.
//
// R42's killer is set the moment a hit dooms its victim (`damage.creditKiller`), and it is what the
// `destroyed` event names and every kill trigger reads. A credit moves that name: while one is in
// force on the striking unit, its lethal hit on the named victim names the paired unit instead.
// Nothing else about the hit changes — its source is still the striker. The record is plain JSON in
// the striker's `memory` (under a key no card script writes), written and cleared around one effect by
// `effects/killCredit.ts`, and R78's reset takes it off a unit that leaves the field.

import type { CardInstance } from "./state";

export const KILL_CREDIT_KEY = "@killCredit";

/** One credit: a lethal hit on `victimId` is credited to `toId`. */
export type KillCredit = { victimId: string; toId: string };

/** R42, R412: whom a lethal hit by `source` on `victim` names as its killer. */
export function creditedKillerId(source: CardInstance, victim: Pick<CardInstance, "id">): string {
  const credits = source.memory[KILL_CREDIT_KEY];
  const credit = Array.isArray(credits) ? (credits as KillCredit[]).find((each) => each.victimId === victim.id) : undefined;
  return credit?.toId ?? source.id;
}
