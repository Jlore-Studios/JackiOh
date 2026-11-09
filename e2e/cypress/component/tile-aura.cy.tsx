// R3342000: tile auras over a board in Cypress component testing (issue #3342).
// Desktop (1280x720), mobile (390x844), and under reduced motion.
// Run: E2E_COMPONENT_PORT=5283 pnpm --dir e2e test:component --spec cypress/component/tile-aura.cy.tsx

import { writeSettings } from "../../../apps/web/src/settings/store.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, emptySide, faceUpBackrow, unit } from "../../../apps/web/src/test/fixtures.ts";

const noop = (): void => undefined;

function fourTileBoard() {
  return baseView({
    you: emptySide("p1", {
      units: [
        unit("p1", { defId: "core-052", instanceId: "u-leg-1" }),
        unit("p1", { defId: "core-100", instanceId: "u-myth-1" }),
        null,
        null,
        null,
      ],
      backrow: [null, null, null, null, null],
    }),
    opponent: emptySide("p2", {
      units: [null, null, null, null, null],
      // A card's own aura replaces its tile aura (R3400000), and every Legendary and Mythic Field Spell wears one
      // (R3399001: Frostspatula's frostMist), as Paragon-X does (R3395000): a face-up Legendary Trap and a Mythic
      // Flying token standing in the backrow (R1081000), neither with an aura of its own, wear it instead.
      backrow: [
        faceUpBackrow("p2", { defId: "core-085", instanceId: "b-leg-2", type: "Trap" }),
        faceUpBackrow("p2", {
          defId: "stinkyscape-t-shield-layer",
          instanceId: "b-myth-2",
          type: "Unit",
          unit: unit("p2", { defId: "stinkyscape-t-shield-layer", instanceId: "b-myth-2" }),
        }),
        null,
        null,
        null,
      ],
    }),
  });
}

describe("R3342000 tile auras over the board", () => {
  it("R3342000 four tile auras render inside their zone boxes at desktop size without horizontal overflow", () => {
    cy.viewport(1280, 720);
    const view = fourTileBoard();
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>,
    );

    cy.get(".tile-aura").should("have.length", 4).each(($aura) => {
      const zone = $aura.parent()[0];
      const aura = $aura[0];
      expect(zone, "zone exists").to.not.equal(undefined);
      expect(aura, "aura exists").to.not.equal(undefined);
      if (!zone || !aura) return;
      // The aura fills the zone inside its border (`inset: 0`): 1 px, and 2 px on a units row's side toward the middle.
      const zoneRect = zone.getBoundingClientRect();
      const auraRect = aura.getBoundingClientRect();
      expect(auraRect.left).to.be.closeTo(zoneRect.left + zone.clientLeft, 2);
      expect(auraRect.top).to.be.closeTo(zoneRect.top + zone.clientTop, 2);
      expect(auraRect.width).to.be.closeTo(zone.clientWidth, 2);
      expect(auraRect.height).to.be.closeTo(zone.clientHeight, 2);
    });

    cy.document().should((doc) => {
      expect(doc.documentElement.scrollWidth, "no horizontal overflow").to.be.at.most(doc.defaultView?.innerWidth ?? 0);
    });
  });

  it("R3342000 four tile auras render inside their zone boxes at phone size without horizontal overflow", () => {
    cy.viewport(390, 844);
    const view = fourTileBoard();
    cy.mount(
      <div className="app-shell">
        <Game view={view} legal={[]} onAction={noop} />
      </div>,
    );

    cy.get(".tile-aura").should("have.length", 4).each(($aura) => {
      const zone = $aura.parent()[0];
      const aura = $aura[0];
      expect(zone, "zone exists").to.not.equal(undefined);
      expect(aura, "aura exists").to.not.equal(undefined);
      if (!zone || !aura) return;
      // The aura fills the zone inside its border (`inset: 0`): 1 px, and 2 px on a units row's side toward the middle.
      const zoneRect = zone.getBoundingClientRect();
      const auraRect = aura.getBoundingClientRect();
      expect(auraRect.left).to.be.closeTo(zoneRect.left + zone.clientLeft, 2);
      expect(auraRect.top).to.be.closeTo(zoneRect.top + zone.clientTop, 2);
      expect(auraRect.width).to.be.closeTo(zone.clientWidth, 2);
      expect(auraRect.height).to.be.closeTo(zone.clientHeight, 2);
    });

    cy.document().should((doc) => {
      expect(doc.documentElement.scrollWidth, "no horizontal overflow").to.be.at.most(doc.defaultView?.innerWidth ?? 0);
    });
  });

  it("R3342000 under reduced motion tile auras are still with no motes", () => {
    writeSettings({ reduceMotion: true });
    cy.viewport(1280, 720);
    const view = fourTileBoard();
    cy.mount(
      <div className="app-shell app-shell--wide">
        <Game view={view} legal={[]} onAction={noop} />
      </div>,
    );

    cy.get(".tile-aura").should("have.length", 4);
    cy.get('.tile-aura[data-tile-motion="still"]').should("have.length", 4);
    cy.get(".tile-aura-mote").should("not.exist");
  });
});
