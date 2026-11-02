// The hotseat device in a spec (BUILD M5-T3, #2160): the one copy of handing it to a seat, for the
// specs that check the dev handle names the seat holding it.

import type { PlayerId } from "./types.ts";

/** BUILD M5-T3: hand the hotseat device to the active seat. */
export function ensureSeat(player: PlayerId): void {
  cy.jackioh().then((handle) => {
    expect(handle.seat, "window.__jackioh.seat (the hotseat handle names the seat holding it)").to.not.eq(
      undefined,
    );
    if (handle.seat !== player) cy.handOver();
  });
}
