// The Play hub's cards (#3729, R3729000): an Online card opens the panel that holds the deck or trio and Find
// a match, so a spec that queues presses the card first, where it used to check the mode's radio.

import { timeouts } from "./config.ts";
import { PLAY_QUEUE_PANEL, playModeId, ts, type QueueMode } from "./testids.ts";

/** Presses `mode`'s card and waits for the panel it opens, its `data-mode` the mode pressed. */
export function openQueuePanel(mode: QueueMode): void {
  cy.get(ts(playModeId(mode)), { timeout: timeouts.view }).should("not.be.disabled").click();
  cy.get(ts(PLAY_QUEUE_PANEL), { timeout: timeouts.view }).should("have.attr", "data-mode", mode);
}
