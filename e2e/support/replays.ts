// R768000: what specs 13 and 39 share about a replay: step it forward and back, and check the last
// turn's GIF that "Share the last turn" makes. A headless browser has no share sheet, so the GIF
// downloads, and the File it downloads is read straight off `URL.createObjectURL` rather than out of
// the downloads folder.

import { REPLAY_GIF_MAX_BYTES } from "../../apps/web/src/replay/config.ts";
import { REPLAY_BACK_STEP, REPLAY_FORWARD_STEP, REPLAY_GIF_SHARE, REPLAY_PLAYER, REPLAY_READOUT, ts } from "./testids.ts";

/** Drawing and encoding a turn's GIF in the page's worker, on CI's slowest runner. */
export const GIF_TIMEOUT = 120_000;

/** A GIF's first bytes, "GIF8" (GIF87a or GIF89a). */
const GIF_MAGIC = [0x47, 0x49, 0x46, 0x38];

/** The player is on step `step`, as its root reads it. */
export function onStep(step: number): void {
  cy.get(ts(REPLAY_PLAYER), { timeout: 10_000 }).should("have.attr", "data-step", String(step));
}

/** From step 0: a step forward, then a step back. */
export function stepForwardAndBack(): void {
  onStep(0);
  cy.get(ts(REPLAY_FORWARD_STEP)).should("not.be.disabled").click();
  onStep(1);
  cy.get(ts(REPLAY_BACK_STEP)).should("not.be.disabled").click();
  onStep(0);
}

/** Before load: no share sheet, so a GIF downloads, and the Blob it downloads is spied on. */
export function spyOnDownloads(win: Cypress.AUTWindow): void {
  Object.defineProperty(win.navigator, "share", { value: undefined, configurable: true });
  Object.defineProperty(win.navigator, "canShare", { value: undefined, configurable: true });
  cy.spy(win.URL, "createObjectURL").as("gifUrl");
}

/** On the last step, the last turn's GIF made, downloaded, a GIF and under the cap. */
export function expectLastTurnGif(): void {
  cy.get(ts(REPLAY_READOUT), { timeout: 10_000 })
    .invoke("text")
    .should("match", /^Step (\d+) of \1 · Turn \d+$/u);
  cy.get(ts(REPLAY_GIF_SHARE), { timeout: GIF_TIMEOUT }).should("contain.text", "Download").click();
  cy.get("@gifUrl")
    .its("lastCall.args.0")
    .then((file: unknown) => {
      // The page's own Blob, from another window than the spec's: read by its shape, not `instanceof`.
      expect(file, "the downloaded GIF").to.have.property("type", "image/gif");
      const blob = file as Blob;
      expect(blob.size, "R513000: under REPLAY_GIF_MAX_BYTES").to.be.within(1, REPLAY_GIF_MAX_BYTES);
      cy.wrap(blob.arrayBuffer()).then((buffer) => {
        expect(Array.from(new Uint8Array(buffer as ArrayBuffer).slice(0, GIF_MAGIC.length)), "it starts GIF8").to.deep.eq(GIF_MAGIC);
      });
    });
}
