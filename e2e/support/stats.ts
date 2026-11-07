// R3324000: what specs 46 and 47 share about /stats. The E2E server's newest patch has too few games, so its
// Cards tab says its data is still being collected and shows no figures, no filters and no games tile; a spec
// that checks the page with its figures steps to the patch before, which shows them however few its games.

import { statsTestid } from "../../apps/web/src/stats/testids.ts";
import { timeouts } from "./config.ts";
import { ts } from "./testids.ts";

/**
 * Opens /stats and leaves it on a patch that shows its figures: the newest, or, while the newest is collecting,
 * the one before. `whileCollecting` runs on the collecting page first, when there is one.
 */
export function visitStatsFigures(whileCollecting?: () => void): void {
  cy.visit("/stats");
  cy.get(`${ts(statsTestid.summaryTiles)}, ${ts(statsTestid.patchCollecting)}`, { timeout: timeouts.view }).should("be.visible");
  cy.get("body").then(($body) => {
    if ($body.find(ts(statsTestid.patchCollecting)).length === 0) return;
    whileCollecting?.();
    cy.get(ts(statsTestid.patchCollectingBack)).click();
  });
  cy.get(ts(statsTestid.summaryTiles), { timeout: timeouts.view }).should("be.visible");
}
