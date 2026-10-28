// The four seats of a Duos game and how they map onto its two duels (#936 SPEC "The core design",
// S2's `DuosSeat` methods). Structure, not rules: duel 0 is A1 against B1, duel 1 is A2 against B2,
// team A is `p1` in both and team B `p2`, and every per-seat array of a `DuosTeamView` is in duel
// order, so a seat's slot in its team's arrays is its duel.

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { Team } from "../wire/generated/Team.ts";

/** S2's `DUOS_SEATS`, in its order. */
export const DUOS_SEATS: readonly DuosSeat[] = ["a1", "a2", "b1", "b2"];

/** A1, A2 → a; B1, B2 → b. */
export function teamOf(seat: DuosSeat): Team {
  return seat === "a1" || seat === "a2" ? "a" : "b";
}

/** A1, B1 → duel 0; A2, B2 → duel 1. Also the seat's slot in its team's per-seat arrays. */
export function duelOf(seat: DuosSeat): 0 | 1 {
  return seat === "a1" || seat === "b1" ? 0 : 1;
}

/** A1 ↔ A2, B1 ↔ B2. */
export function teammateOf(seat: DuosSeat): DuosSeat {
  switch (seat) {
    case "a1":
      return "a2";
    case "a2":
      return "a1";
    case "b1":
      return "b2";
    case "b2":
      return "b1";
  }
}

/** A1 ↔ B1, A2 ↔ B2: the seat across the same duel. */
export function acrossOf(seat: DuosSeat): DuosSeat {
  switch (seat) {
    case "a1":
      return "b1";
    case "b1":
      return "a1";
    case "a2":
      return "b2";
    case "b2":
      return "a2";
  }
}

/** A seat's name in a player's words, as the rail and the dev switcher print it ("A1"). */
export function seatLabel(seat: DuosSeat): string {
  return seat.toUpperCase();
}
