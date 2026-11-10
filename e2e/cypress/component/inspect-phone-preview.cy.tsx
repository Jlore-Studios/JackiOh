// #3602: on a phone-width screen a touch hold opens the hover preview (R2617000) with the
// full-text panel stacked across the card's width, and the preview, the face and the panel all stay
// inside the viewport. jsdom cannot measure boxes, so this layout proof lives here, not in vitest.
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/inspect-phone-preview.cy.tsx

import type { ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { faceModel, useInspectTrigger } from "../../../apps/web/src/cards/index.ts";
import { LONG_PRESS_MS } from "../../../apps/web/src/cards/inspect/constants.ts";
import { closeInspect } from "../../../apps/web/src/cards/inspect/store.ts";
import { INSPECT_EXTENDED, INSPECT_FACE, INSPECT_HOVER } from "../../../apps/web/src/cards/inspect/testids.ts";

const VIEWPORTS = [
  { label: "390x844", width: 390, height: 844 },
  { label: "430x932", width: 430, height: 932 },
] as const;

/** Subpixel slack for edges the browser rounds. */
const SLACK_PX = 1;

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** A landing-fan-style card: the hold opens the hover preview, `prefer: "above"` (landing.tsx). */
function FanHold(): ReactElement {
  const found = CATALOG["core-001"];
  if (found === undefined) throw new Error("no core-001");
  // core-001 is full art with an extended panel (Extended.test.tsx, R2718000).
  const face = faceModel({ defId: "core-001", def: found, radiant: false });
  const inspect = useInspectTrigger({ key: "phone-preview-fan", face }, { hover: false, prefer: "above" });
  return (
    <>
      <div style={{ display: "flex", justifyContent: "center", paddingTop: 120 }}>
        <button type="button" data-testid="fan-card" style={{ width: 100, height: 140 }} {...inspect.handlers}>
          Jlock Shredder
        </button>
      </div>
      {inspect.overlay}
    </>
  );
}

describe("#3602 a held full-art fan card's preview stays on a phone screen", () => {
  beforeEach(() => {
    closeInspect();
  });

  afterEach(() => {
    closeInspect();
  });

  for (const vp of VIEWPORTS) {
    it(`#3602 at ${vp.label} holding the fan card keeps the preview, the face and the full text inside`, () => {
      cy.viewport(vp.width, vp.height);
      cy.mount(<FanHold />);
      cy.get(ts("fan-card")).should("be.visible");
      // A touch held past LONG_PRESS_MS is the hover (R2617000).
      cy.get(ts("fan-card")).then(($card) => {
        const box = ($card[0] as HTMLElement).getBoundingClientRect();
        cy.get(ts("fan-card")).trigger("pointerdown", {
          pointerId: 7,
          pointerType: "touch",
          isPrimary: true,
          button: 0,
          clientX: box.left + box.width / 2,
          clientY: box.top + box.height / 2,
        });
      });
      cy.get(ts(INSPECT_HOVER), { timeout: LONG_PRESS_MS + 4000 }).should("be.visible");
      cy.get(ts(INSPECT_EXTENDED)).should("be.visible");
      cy.window().should((win) => {
        const doc = win.document;
        const hover = doc.querySelector<HTMLElement>(ts(INSPECT_HOVER)) as HTMLElement;
        const face = doc.querySelector<HTMLElement>(ts(INSPECT_FACE)) as HTMLElement;
        const extended = doc.querySelector<HTMLElement>(ts(INSPECT_EXTENDED)) as HTMLElement;
        const inView = (box: DOMRect, name: string): void => {
          expect(box.left, `${name} left`).to.be.at.least(-SLACK_PX);
          expect(box.top, `${name} top`).to.be.at.least(-SLACK_PX);
          expect(box.right, `${name} right`).to.be.at.most(win.innerWidth + SLACK_PX);
          expect(box.bottom, `${name} bottom`).to.be.at.most(win.innerHeight + SLACK_PX);
        };
        const hoverBox = hover.getBoundingClientRect();
        const faceBox = face.getBoundingClientRect();
        const extendedBox = extended.getBoundingClientRect();
        inView(hoverBox, "preview");
        inView(faceBox, "face");
        inView(extendedBox, "full text");
        // The card is drawn whole within the preview.
        expect(faceBox.left, "face in preview left").to.be.at.least(hoverBox.left - SLACK_PX);
        expect(faceBox.top, "face in preview top").to.be.at.least(hoverBox.top - SLACK_PX);
        expect(faceBox.right, "face in preview right").to.be.at.most(hoverBox.right + SLACK_PX);
        expect(faceBox.bottom, "face in preview bottom").to.be.at.most(hoverBox.bottom + SLACK_PX);
      });
      cy.get(ts("fan-card")).trigger("pointerup", { pointerId: 7, pointerType: "touch", isPrimary: true, force: true });
      cy.get(ts(INSPECT_HOVER)).should("not.exist");
    });
  }
});
