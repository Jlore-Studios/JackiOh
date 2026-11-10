// R3350000 (#3350): every Mythic's diorama in a real browser. apps/web's vitest suite proves which faces draw one, what each
// slice holds and how far each turns (cards/diorama.test.tsx, cards/art/dioramas/dioramas.test.ts); what only a layout and
// style engine can say is measured here, at a desktop size (1280x720) and a phone's (390x844): every diorama in inspect's
// drag, base then Radiant, its slices stacked in the art window's box, turned by the pointer at the face's left edge, its
// middle and its right edge to -DIORAMA_TURN_MAX_DEG, level and DIORAMA_TURN_MAX_DEG, a screenshot at each; and Reduce
// motion, which draws the flat picture, still, with no diorama and no plane.
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/card-diorama.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { DIORAMAS } from "../../../apps/web/src/cards/art/dioramas/index.ts";
import { DIORAMA_TURN_MAX_DEG } from "../../../apps/web/src/cards/constants.ts";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { pointerPose, tiltDegrees } from "../../../apps/web/src/cards/tilt.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];

/** The two viewports a player holds the game at, and the width an inspected face is drawn at in each. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720, face: 300 },
  { label: "phone", width: 390, height: 844, face: 240 },
] as const;

/** The pointer's three places across the face, as a share of its width: at its left edge, its middle and its right edge. */
const STOPS = [
  { name: "left", x: 0.01 },
  { name: "middle", x: 0.5 },
  { name: "right", x: 0.99 },
] as const;

/** How far a computed angle may lie from the one asked for: a transition has settled, the matrix rounds. */
const ANGLE_SLACK_DEG = 0.8;
/** Enough room for the face to mount, the slices to draw and a turn's transition to settle. */
const SETTLE_TIMEOUT_MS = 8_000;

const HOST = '[data-testid="host"]';

function reset(): void {
  __resetSettingsForTests();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
}

/** An inspected face of `id`: the face in a host whose box the turn reads, turned by a drag as the inspect sheet's is. */
function Inspected({ id, width, radiant }: { id: string; width: number; radiant: boolean }) {
  return (
    <div style={{ padding: 32, background: "#1b1f2b", display: "flex" }}>
      <div data-testid="host" style={{ width, height: Math.round(width / FACE_ASPECT), flex: "none", pointerEvents: "auto" }}>
        <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" worn tilt="drag" />
      </div>
    </div>
  );
}

/** The rotateY a face's computed transform turns by, in degrees, from the matrix. */
function turnOf(face: JQuery<HTMLElement>): number {
  const transform = getComputedStyle(face[0] as HTMLElement).transform;
  if (transform === "none") return 0;
  const matrix = new DOMMatrix(transform);
  // rotateX(a) rotateY(b) has sin b at m31.
  return (Math.asin(Math.max(-1, Math.min(1, matrix.m31))) * 180) / Math.PI;
}

for (const viewport of VIEWPORTS) {
  describe(`R3350000 every Mythic's diorama at ${viewport.label} size (${String(viewport.width)}x${String(viewport.height)})`, () => {
    beforeEach(() => {
      reset();
      cy.viewport(viewport.width, viewport.height);
    });

    afterEach(() => {
      reset();
    });

    for (const diorama of DIORAMAS.values()) {
      for (const radiant of [false, true]) {
        const face = radiant ? "radiant" : "base";
        it(`R3350000 ${diorama.id}'s diorama, ${face}, turns to ${String(DIORAMA_TURN_MAX_DEG)} degrees each way`, () => {
          const height = Math.round(viewport.face / FACE_ASPECT);
          cy.mount(<Inspected id={diorama.id} width={viewport.face} radiant={radiant} />);
          cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-diorama", "true");
          // The farthest slice in the window, every other a plane of the window's own box over it, each drawing a picture.
          cy.get(".cf-art-frame > .cf-art").should("have.attr", "data-art-slice", "0");
          cy.get(".cf-planes > .cf-plane[data-slice]").should("have.length", diorama.slices.length - 1);
          cy.get(".cf-art-frame").then(($frame) => {
            const frame = ($frame[0] as HTMLElement).getBoundingClientRect();
            for (const plane of $frame.find(".cf-plane[data-slice]")) {
              const box = plane.getBoundingClientRect();
              expect(box.width, "a slice is the window's width").to.be.closeTo(frame.width, 1);
              expect(box.height, "a slice is the window's height").to.be.closeTo(frame.height, 1);
              expect(getComputedStyle(plane.querySelector(".cf-art") as HTMLElement).backgroundImage).to.contain("data:image/svg+xml");
            }
          });
          for (const stop of STOPS) {
            const px = stop.x * viewport.face;
            // About -DIORAMA_TURN_MAX_DEG, level and DIORAMA_TURN_MAX_DEG: the turn the pointer asks for, never past the cap.
            const expected = tiltDegrees(pointerPose(px, height / 2, { left: 0, top: 0, width: viewport.face, height }), DIORAMA_TURN_MAX_DEG).ry;
            cy.get(HOST).trigger("pointermove", px, height / 2, { force: true });
            cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-tilt", "on");
            cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should(($cf) => {
              const ry = turnOf($cf);
              expect(ry, `${face} ${stop.name}`).to.be.closeTo(expected, ANGLE_SLACK_DEG);
              expect(Math.abs(ry), `${face} ${stop.name} within the cap`).to.be.at.most(DIORAMA_TURN_MAX_DEG + ANGLE_SLACK_DEG);
            });
            cy.get(HOST).screenshot(`card-diorama-${viewport.label}-${diorama.id}-${face}-${stop.name}`);
          }
        });
      }
    }

    it("R3350000 under reduced motion a Mythic draws its flat picture, still: no diorama and no plane", () => {
      writeSettings({ reduceMotion: true });
      for (const id of ["core-100", "stinkyscape-154"]) {
        cy.mount(<Inspected id={id} width={viewport.face} radiant={false} />);
        cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("not.have.attr", "data-depth");
        cy.get("[data-diorama]").should("not.exist");
        cy.get(".cf-plane").should("not.exist");
        cy.get(".cf-art-frame > .cf-art").should("not.have.attr", "data-art-slice");
        cy.get(HOST).trigger("pointermove", viewport.face, 10, { force: true });
        cy.get(".cf").should("not.have.attr", "data-tilt");
        cy.get(HOST).screenshot(`card-diorama-${viewport.label}-${id}-reduced`);
      }
    });
  });
}
