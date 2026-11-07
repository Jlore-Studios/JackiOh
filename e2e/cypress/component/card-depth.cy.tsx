// R3344000, R3344001 (#3344): a face with depth in a real browser. apps/web's vitest suite proves which faces draw depth
// planes and what each plane holds (cards/depth.test.tsx), the tilt's poses, caps and slot (cards/tilt.test.tsx) and the
// stylesheet's colours (cards/cosmetics.test.tsx); what only a layout and style engine can say is measured here, at a
// desktop size (1280x720) and a phone's (390x844): a Legendary and a Mythic in the viewer's own hand, each tilted at three
// angles under the pointer, never past CARD_TILT_MAX_DEG, the planes parallaxing by their depth farther the nearer they
// are, the glare lit over the art and the frame in their blend modes, and the subject breaking the window's top edge;
// a Radiant face, whose glare takes none on its frame and whose own shimmer stays; and Reduce motion, which draws the
// flat picture and tilts nothing.
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/card-depth.cy.tsx

import { CATALOG } from "@jackioh/cards";

import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_DEPTH_STEP_CQW, CARD_FRAME_BREAK_PX, CARD_FRAME_BREAK_SCALE, CARD_TILT_MAX_DEG } from "../../../apps/web/src/cards/constants.ts";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import { GLARE_BY_FOIL, NO_FOIL_ID } from "../../../apps/web/src/cards/depth.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { pointerPose, tiltDegrees } from "../../../apps/web/src/cards/tilt.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];

/** The two viewports a player holds the game at, and the width a hand card is drawn at in each. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720, face: 220 },
  { label: "phone", width: 390, height: 844, face: 150 },
] as const;

/** A Legendary and a Mythic that are no full-art card, the rarities that may break the frame. */
const CARDS = [
  { id: "core-052", rarity: "Legendary" },
  { id: "core-100", rarity: "Mythic" },
] as const;

/** The planes over the window, nearest last, and what each shifts by per step of depth (`--cf-plane`). */
const OVER_PLANES = ["middle", "near", "nearest"] as const;

/** How far a computed angle may lie from the one asked for: a transition has settled, the matrix rounds. */
const ANGLE_SLACK_DEG = 0.6;
/** How far a computed shift may lie from the one asked for, in px. */
const SHIFT_SLACK_PX = 0.6;
/** Enough room for the faces to mount, the pictures to draw and a tilt's transition to settle. */
const SETTLE_TIMEOUT_MS = 8_000;

function must<T>(found: T | null | undefined, what: string): T {
  if (found === null || found === undefined) throw new Error(`no ${what}`);
  return found;
}

function reset(): void {
  __resetSettingsForTests();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
}

/** A hand card of `id`: the face, in a host whose box the tilt reads, as game/Card.tsx draws it in the viewer's hand. */
function Hand({ id, width, radiant = false }: { id: string; width: number; radiant?: boolean }) {
  return (
    <div style={{ padding: 24, background: "#1b1f2b", display: "flex", gap: 16 }}>
      <div data-testid="host" style={{ width, height: Math.round(width / FACE_ASPECT), flex: "none", pointerEvents: "auto" }}>
        <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" worn ownHand tilt="host" />
      </div>
    </div>
  );
}

const HOST = '[data-testid="host"]';

/** The angles a face's computed transform turns by, in degrees: its rotateX and rotateY, from the matrix. */
function anglesOf(face: JQuery<HTMLElement>): { rx: number; ry: number } {
  const transform = getComputedStyle(face[0] as HTMLElement).transform;
  if (transform === "none") return { rx: 0, ry: 0 };
  const matrix = new DOMMatrix(transform);
  const RAD = Math.PI / 180;
  // rotateX(a) rotateY(b) is [[cb, 0, sb], [sa sb, ca, -sa cb], [-ca sb, sa, ca cb]].
  return { rx: Math.asin(Math.max(-1, Math.min(1, matrix.m23))) / RAD, ry: Math.asin(Math.max(-1, Math.min(1, matrix.m31))) / RAD };
}

/** A plane's picture's computed shift in px along x, from its `translate`. */
function shiftOf(art: HTMLElement): number {
  const translate = getComputedStyle(art).translate;
  return translate === "none" ? 0 : Number.parseFloat(translate.split(" ")[0] ?? "0");
}

for (const viewport of VIEWPORTS) {
  describe(`R3344000 R3344001 a face with depth at ${viewport.label} size (${String(viewport.width)}x${String(viewport.height)})`, () => {
    beforeEach(() => {
      reset();
      cy.viewport(viewport.width, viewport.height);
    });

    afterEach(() => {
      reset();
    });

    for (const card of CARDS) {
      it(`R3344000 a ${card.rarity} draws four planes, each the art window's own box, over the window's picture`, () => {
        cy.mount(<Hand id={card.id} width={viewport.face} />);
        cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-depth", "true");
        cy.get(".cf-art-frame [data-art]").should("have.length", 4);
        cy.get(".cf-art-frame [data-art]").then(($arts) => {
          expect([...$arts].map((art) => art.getAttribute("data-art-part"))).to.deep.equal(["far", "middle", "near", "nearest"]);
          for (const art of $arts) expect(getComputedStyle(art).backgroundImage, "each plane draws a picture").to.contain("data:image/svg+xml");
        });
        cy.get(".cf-art-frame").then(($frame) => {
          const frame = $frame[0]?.getBoundingClientRect();
          for (const plane of OVER_PLANES) {
            const box = must($frame.find(`.cf-plane[data-plane="${plane}"]`)[0], plane).getBoundingClientRect();
            expect(box.width, `${plane} width`).to.be.closeTo(must(frame, "frame").width, 1);
            expect(box.height, `${plane} height`).to.be.closeTo(must(frame, "frame").height, 1);
          }
        });
        // Nothing of the planes takes the pointer: the host does.
        cy.get(".cf-planes").should("have.css", "pointer-events", "none");
        cy.get(".cf-plane").each(($plane) => {
          expect(getComputedStyle($plane[0] as HTMLElement).pointerEvents).to.eq("none");
        });
        // The face's parent has the perspective.
        cy.get(HOST).should("have.css", "perspective", "900px");
        cy.get(HOST).screenshot(`card-depth-${viewport.label}-${card.rarity.toLowerCase()}-rest`);
      });

      it(`R3344001 a ${card.rarity} tilts toward the pointer at three angles, never past ${String(CARD_TILT_MAX_DEG)} degrees, and springs back`, () => {
        cy.mount(<Hand id={card.id} width={viewport.face} />);
        cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-depth", "true");
        cy.get(".cf").should("not.have.attr", "data-tilt");
        const height = Math.round(viewport.face / FACE_ASPECT);
        // The middle, the far lower right and the upper left, each as a fraction of the host's box.
        const stops = [
          { name: "level", x: 0.5, y: 0.5 },
          { name: "lower right", x: 0.95, y: 0.9 },
          { name: "upper left", x: 0.1, y: 0.08 },
        ] as const;
        cy.get(HOST).trigger("pointerenter", viewport.face / 2, height / 2);
        for (const stop of stops) {
          const px = stop.x * viewport.face;
          const py = stop.y * height;
          cy.get(HOST).trigger("pointermove", px, py);
          const expected = tiltDegrees(pointerPose(px, py, { left: 0, top: 0, width: viewport.face, height }));
          cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-tilt", "on");
          // The transition settles on the angle the pointer asked for (the retry waits it out).
          cy.get(".cf").should(($cf) => {
            const { rx, ry } = anglesOf($cf);
            expect(ry, `${stop.name} ry`).to.be.closeTo(expected.ry, ANGLE_SLACK_DEG);
            expect(rx, `${stop.name} rx`).to.be.closeTo(expected.rx, ANGLE_SLACK_DEG);
            expect(Math.abs(ry), `${stop.name} within the cap`).to.be.at.most(CARD_TILT_MAX_DEG + ANGLE_SLACK_DEG);
            expect(Math.abs(rx), `${stop.name} within the cap`).to.be.at.most(CARD_TILT_MAX_DEG + ANGLE_SLACK_DEG);
          });
          cy.get(HOST).screenshot(`card-depth-${viewport.label}-${card.rarity.toLowerCase()}-${stop.name.replace(" ", "-")}`);
        }
        // Far past the box is the cap and no more.
        cy.get(HOST).trigger("pointermove", viewport.face * 4, height * 4, { force: true });
        cy.get(".cf").should(($cf) => {
          const { rx, ry } = anglesOf($cf);
          expect(ry, "far past, ry").to.be.closeTo(CARD_TILT_MAX_DEG, ANGLE_SLACK_DEG);
          expect(rx, "far past, rx").to.be.closeTo(-CARD_TILT_MAX_DEG, ANGLE_SLACK_DEG);
        });
        // The pointer leaves: every property goes and the face springs back to level.
        cy.get(HOST).trigger("pointerleave");
        cy.get(".cf").should("not.have.attr", "data-tilt");
        cy.get(".cf").should(($cf) => {
          const { rx, ry } = anglesOf($cf);
          expect(ry, "level, ry").to.be.closeTo(0, 0.1);
          expect(rx, "level, rx").to.be.closeTo(0, 0.1);
        });
        cy.get(".cf").should(($cf) => {
          expect($cf[0]?.style.getPropertyValue("--cf-tilt-ry")).to.eq("");
        });
      });

      it(`R3344000 a tilted ${card.rarity} parallaxes its planes by depth and lights its glare over the art and the frame`, () => {
        cy.mount(<Hand id={card.id} width={viewport.face} />);
        cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-depth", "true");
        // At rest the glare is unseen and no plane has shifted.
        cy.get(".cf-glare--art").should("have.css", "opacity", "0");
        cy.get(".cf-glare--frame").should("have.css", "opacity", "0");
        const height = Math.round(viewport.face / FACE_ASPECT);
        cy.get(HOST).trigger("pointermove", viewport.face * 0.95, height / 2);
        cy.get(".cf-glare--art").should("have.css", "opacity", "1");
        cy.get(".cf-glare--frame").should("have.css", "opacity", "1");
        // Their blend modes, and each in a group of its own area's.
        cy.get(".cf-glare--art").should("have.css", "mix-blend-mode", "color-dodge");
        cy.get(".cf-glare--frame").should("have.css", "mix-blend-mode", "overlay");
        cy.get(".cf-art-frame").should("have.css", "isolation", "isolate");
        cy.get(".cf-scale").should("have.css", "isolation", "isolate");
        // The glare lies left of the middle for a pointer on the right: opposite the tilt.
        cy.get(".cf").should(($cf) => {
          expect(Number.parseFloat($cf[0]?.style.getPropertyValue("--cf-glare-x") ?? "NaN"), "glare x").to.be.lessThan(50);
        });
        // The planes shift with the pose, each farther the nearer it is; the window's own picture rests.
        cy.get(".cf-art-frame > .cf-art").should(($far) => {
          expect(shiftOf($far[0] as HTMLElement), "the far plane rests").to.eq(0);
        });
        cy.get(".cf").should(($cf) => {
          const scaleWidth = must($cf[0]?.querySelector(".cf-scale"), "scale").getBoundingClientRect().width;
          const step = (CARD_DEPTH_STEP_CQW / 100) * scaleWidth;
          const pose = pointerPose(viewport.face * 0.95, height / 2, { left: 0, top: 0, width: viewport.face, height }).x;
          const shifts = OVER_PLANES.map((plane) => shiftOf(must($cf[0]?.querySelector<HTMLElement>(`.cf-plane[data-plane="${plane}"] > .cf-art`), plane)));
          OVER_PLANES.forEach((plane, index) => {
            expect(shifts[index], `${plane} shift`).to.be.closeTo(pose * (index + 1) * step, SHIFT_SLACK_PX);
          });
          expect(shifts[0] ?? 0).to.be.lessThan(shifts[1] ?? 0);
          expect(shifts[1] ?? 0).to.be.lessThan(shifts[2] ?? 0);
        });
        cy.get(HOST).trigger("pointerleave");
        cy.get(".cf-glare--art").should("have.css", "opacity", "0");
      });

      it(`R3344000 a ${card.rarity} in hand lets its subject break the window's top edge by ${String(CARD_FRAME_BREAK_PX)}px, no further than the face`, () => {
        cy.mount(<Hand id={card.id} width={viewport.face} />);
        cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-frame-break", "true");
        cy.get('.cf-plane[data-plane="near"]').should("have.css", "overflow", "visible");
        cy.get('.cf-plane[data-plane="near"]').should(($near) => {
          expect(getComputedStyle($near[0] as HTMLElement).clipPath).to.match(new RegExp(`^inset\\(-${String(CARD_FRAME_BREAK_PX)}px`));
          // The window keeps its shape (a Unit's oval): the raised clip rounds its corners, it never squares them.
          expect(getComputedStyle($near[0] as HTMLElement).clipPath).to.contain("round");
        });
        cy.get('.cf-plane[data-plane="near"] > .cf-art').should("have.css", "scale", String(CARD_FRAME_BREAK_SCALE));
        // The planes that are not the subject's keep the window's clip.
        cy.get('.cf-plane[data-plane="middle"]').should("have.css", "overflow", "hidden");
        cy.get('.cf-plane[data-plane="nearest"]').should("have.css", "overflow", "hidden");
        // The face's own clip still stops it: the near plane's picture never passes the scale's top edge.
        cy.get(".cf").then(($cf) => {
          const scale = must($cf[0]?.querySelector(".cf-scale"), "scale").getBoundingClientRect();
          expect(getComputedStyle(must($cf[0]?.querySelector(".cf-scale"), "scale")).overflow).to.eq("hidden");
          const near = must($cf[0]?.querySelector('.cf-plane[data-plane="near"] > .cf-art'), "near").getBoundingClientRect();
          // The picture is taller than its plane and reaches over the window's top, but its box is clipped by the scale.
          expect(near.top, "the subject rises over the window").to.be.lessThan(must($cf[0]?.querySelector(".cf-art-frame"), "frame").getBoundingClientRect().top);
          expect(scale.top).to.be.lessThan(near.bottom);
        });
      });
    }

    it("R3344000 a Radiant face takes no glare on its frame, keeps its own shimmer and its gold, and a plane draws the Radiant picture", () => {
      cy.mount(<Hand id="core-052" width={viewport.face} radiant />);
      cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.attr", "data-depth", "true");
      cy.get(".cf").should("have.attr", "data-radiant-face", "true");
      cy.get(".cf-art-frame [data-art]").should(($arts) => {
        expect([...$arts].map((art) => art.getAttribute("data-art-variant"))).to.deep.equal(["radiant", "radiant", "radiant", "radiant"]);
      });
      cy.get(".cf").should(($cf) => {
        expect($cf[0]?.style.getPropertyValue("--cf-glare-frame")).to.eq("0");
        expect($cf[0]?.style.getPropertyValue("--cf-glare-art")).to.eq(String(GLARE_BY_FOIL[NO_FOIL_ID]?.art));
        // Radiant's own shimmer is the face's `::after`, drawn as ever; the planes' own pseudo-elements are off.
        expect(getComputedStyle($cf[0] as HTMLElement, "::after").content).to.not.eq("none");
        for (const art of $cf[0]?.querySelectorAll(".cf-plane > .cf-art") ?? []) {
          expect(getComputedStyle(art, "::after").content, "a plane's art draws no pseudo-element").to.be.oneOf(["none", "normal"]);
        }
      });
      const height = Math.round(viewport.face / FACE_ASPECT);
      cy.get(HOST).trigger("pointermove", viewport.face * 0.9, height * 0.2);
      cy.get(".cf").should("have.attr", "data-tilt", "on");
      // Tilted, the frame's glare is at none: it draws no colour over the gold frame.
      cy.get(".cf-glare--frame").should(($glare) => {
        expect(getComputedStyle($glare[0] as HTMLElement).backgroundImage).to.contain("radial-gradient");
      });
      cy.get(HOST).screenshot(`card-depth-${viewport.label}-radiant-tilted`);
    });

    it("R3344000 R3344001 under Reduce motion a face draws the flat picture and tilts nothing", () => {
      writeSettings({ reduceMotion: true });
      cy.mount(<Hand id="core-052" width={viewport.face} />);
      cy.get(".cf", { timeout: SETTLE_TIMEOUT_MS }).should("exist");
      cy.get(".cf").should("not.have.attr", "data-depth");
      cy.get(".cf-planes").should("not.exist");
      cy.get(".cf-glare").should("not.exist");
      cy.get(".cf-art-frame [data-art]").should("have.length", 1);
      const height = Math.round(viewport.face / FACE_ASPECT);
      cy.get(HOST).trigger("pointerenter", viewport.face / 2, height / 2);
      cy.get(HOST).trigger("pointermove", viewport.face * 0.95, height * 0.9);
      cy.get(".cf").should("not.have.attr", "data-tilt");
      cy.get(".cf").should("have.css", "transform", "none");
      cy.get(HOST).screenshot(`card-depth-${viewport.label}-reduced-motion`);
    });
  });
}
