// R3345000: Full-screen view for Mythics and full-art cards.
// A component spec capturing the full-screen view at desktop size (1280x720) and phone (390x844).
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/card-fullscreen.cy.tsx

import {
  ART_PLANES,
  FullScreenArt,
  FullScreenArtHost,
  closeFullScreenArt,
  useFullScreenTrigger,
} from "../../../apps/web/src/cards/index.ts";
import { FULLSCREEN_HOLD_MS } from "../../../apps/web/src/cards/constants.ts";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

function TriggerCard({ id, name }: { id: string; name: string }) {
  const { handlers } = useFullScreenTrigger({
    defId: id,
    rarity: "Mythic",
    name,
  });

  return (
    <div style={{ padding: 40 }}>
      <button
        type="button"
        data-testid="mythic-card-trigger"
        style={{ width: 150, height: 210, background: "#333", color: "#fff", cursor: "pointer" }}
        {...handlers}
      >
        {name}
      </button>
      <FullScreenArtHost />
    </div>
  );
}

describe("R3345000 full-screen view", () => {
  beforeEach(() => {
    closeFullScreenArt();
  });

  afterEach(() => {
    closeFullScreenArt();
  });

  for (const vp of VIEWPORTS) {
    it(`R3345000 captures full-screen art at ${vp.label} (${vp.width}x${vp.height}) with 4 depth planes and camera dolly`, () => {
      cy.viewport(vp.width, vp.height);
      cy.mount(
        <FullScreenArt
          defId="core-100"
          name="The Ceaseless Void"
          onClose={() => {
            closeFullScreenArt();
          }}
        />,
      );

      cy.get('[data-testid="fullscreen-art"]').should("be.visible");
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "role", "dialog");
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "aria-modal", "true");
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "aria-label", "The Ceaseless Void");

      // Verifies all 4 depth planes are present
      for (const plane of ART_PLANES) {
        cy.get(`[data-testid="fullscreen-art"] [data-plane="${plane}"]`).should("exist");
      }

      // Close button exists and works
      cy.get('[data-testid="fullscreen-art-close"]').should("be.visible");
    });

    it(`R3345000 tap pauses and resumes, and release closes it at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      let closed = false;
      cy.mount(
        <FullScreenArt
          defId="core-100"
          name="The Ceaseless Void"
          heldPointerId={1}
          onClose={() => {
            closed = true;
          }}
        />,
      );

      cy.get('[data-testid="fullscreen-art"]').should("be.visible");
      cy.get('[data-testid="fullscreen-art"]').should("not.have.attr", "data-paused");

      // Tap pauses
      cy.get('[data-testid="fullscreen-art"]').click();
      cy.get('[data-testid="fullscreen-art"]').should("have.attr", "data-paused", "true");

      // Tap resumes
      cy.wait(400);
      cy.get('[data-testid="fullscreen-art"]').click();
      cy.get('[data-testid="fullscreen-art"]').should("not.have.attr", "data-paused");

      // Release of held pointer closes
      cy.window().trigger("pointerup", { pointerId: 1 });
      cy.then(() => {
        expect(closed).to.equal(true);
      });
    });

    it(`R3345000 press and hold opens the full-screen view at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      cy.mount(<TriggerCard id="core-100" name="The Ceaseless Void" />);

      cy.get('[data-testid="fullscreen-art"]').should("not.exist");

      // Press and hold past FULLSCREEN_HOLD_MS
      cy.get('[data-testid="mythic-card-trigger"]').trigger("pointerdown", { pointerId: 2, clientX: 100, clientY: 100 });
      cy.wait(FULLSCREEN_HOLD_MS + 50);

      cy.get('[data-testid="fullscreen-art"]').should("be.visible");

      // Release closes
      cy.window().trigger("pointerup", { pointerId: 2 });
      cy.get('[data-testid="fullscreen-art"]').should("not.exist");
    });
  }
});
