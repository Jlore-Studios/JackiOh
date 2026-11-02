// R2725003, R2725004: a card's versions and augments in a real browser. apps/web's vitest suite proves the
// attributes each look wears (cards/fullArtTiers.test.tsx) and the stylesheet's colors (cards/cosmetics.test.tsx);
// what only a layout and style engine can say is measured here. Part 1's two versions, the normal face and Full art,
// on one Unit and one Spell, base and Radiant, each bare and with an augment stack (Animated, Signature and, on the
// Unit, a foil): every part keeps clear of every other, the Signature on the normal face stays inside its art window
// and off the words, a Radiant face keeps its gold frame and its art's Radiant paint under every look, and under
// Reduce motion nothing moves.
//
// Each face is the viewer's own copy (`OwnCopy`) shown in a look by `FullArtShown`, as the Collection's preview
// shows one, so the looks stand side by side with no pick written. No `FullArtOwned` is mounted, so every cosmetic
// counts as owned where it is offered, as in practice.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, VERSION_FULL_ART_ID } from "../../../apps/web/src/cards/versions.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];

/** One Unit and one Spell offered both versions and both augments (R2725001). */
const CARDS = [
  { id: "classicplus-012", why: "a Unit (The Mother Pancake: stats in the bottom corners, a Legendary crest)" },
  { id: "stinkyscape-154", why: "a Spell (Dice of Doom: no stats, a Mythic crest)" },
] as const;

/** The Unit wears a foil under its augment stack. */
const FOIL = "foil-rainbow";
const STACK = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID] as const;

/** Each version, bare and with the stack. */
const LOOKS = [
  { name: "normal", pick: false, augments: [] },
  { name: "normal-stack", pick: false, augments: STACK },
  { name: "full", pick: VERSION_FULL_ART_ID, augments: [] },
  { name: "full-stack", pick: VERSION_FULL_ART_ID, augments: STACK },
] as const;

/** The inspect-sized and hand-sized full faces card-faces.cy.tsx measures (B15). */
const WIDTHS = [270, 170] as const;

/** What a face's parts must keep apart from the signature: none of them may cover it, nor it them. */
const PARTS = [".cost-gem", ".cf-gem", ".cf-set", ".cf-crest", ".card-name", ".card-type", ".card-text", ".cf-atk", ".cf-hp"] as const;

function heightFor(width: number): number {
  return Math.round(width / FACE_ASPECT);
}

function overlaps(a: DOMRect, b: DOMRect): boolean {
  const SLACK = 0.5;
  return a.left < b.right - SLACK && b.left < a.right - SLACK && a.top < b.bottom - SLACK && b.top < a.bottom - SLACK;
}

function Faces({ width }: { width: number }) {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
        {CARDS.flatMap(({ id }) =>
          LOOKS.flatMap((look) =>
            [false, true].map((radiant) => (
              <div key={`${id}-${look.name}-${String(radiant)}`} data-face={`${id}-${look.name}-${radiant ? "radiant" : "base"}`} style={{ width, height: heightFor(width), flex: "none" }}>
                <FullArtShown defId={id} pick={look.pick} augments={look.augments}>
                  <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" />
                </FullArtShown>
              </div>
            )),
          ),
        )}
      </div>
    </OwnCopy>
  );
}

function faceIn(doc: Document, id: string, look: string, radiant: boolean): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-face="${id}-${look}-${radiant ? "radiant" : "base"}"] > .cf`);
  if (cf === null) throw new Error(`no face for ${id} ${look}`);
  return cf;
}

/** The boxes of `cf`'s parts matching `selector` that draw. */
function boxes(cf: HTMLElement, selector: string): DOMRect[] {
  return [...cf.querySelectorAll<HTMLElement>(selector)]
    .filter((element) => getComputedStyle(element).display !== "none" && element.getBoundingClientRect().width > 0)
    .map((element) => element.getBoundingClientRect());
}

describe("R2725003 a card's versions and augments on real faces", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    writeCardPick(CARDS[0].id, "foil", FOIL);
    cy.viewport(1280, 900);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  for (const width of WIDTHS) {
    it(`R2725003 every look keeps its parts apart, the Signature on its art, at ${width}px`, () => {
      cy.mount(<Faces width={width} />);
      cy.document().should((doc) => {
        for (const { id, why } of CARDS) {
          for (const look of LOOKS) {
            for (const radiant of [false, true]) {
              const where = `${id} (${why}) ${look.name} ${radiant ? "Radiant" : "base"}, ${width}px`;
              const cf = faceIn(doc, id, look.name, radiant);
              const face = cf.getBoundingClientRect();
              const full = look.pick !== false;
              expect(face.width, `${where}: the face fills its box`).to.be.greaterThan(width / 2);
              expect(cf.getAttribute("data-full-art"), where).to.eq(full ? "true" : null);
              expect(cf.getAttribute("data-augments"), where).to.eq(look.augments.length > 0 ? look.augments.join(" ") : null);
              // The foil wears under the stack on the Unit's base faces; a Radiant face wears none.
              if (id === CARDS[0].id) expect(cf.getAttribute("data-cosmetic-foil"), `${where}: the foil`).to.eq(radiant ? null : FOIL);
              const signature = boxes(cf, ".cf-signature");
              expect(signature.length, `${where}: the signature`).to.eq(look.augments.length > 0 ? 1 : 0);
              for (const box of signature) {
                expect(box.left, `${where}: the signature inside`).to.be.at.least(face.left - 0.5);
                expect(box.right, `${where}: the signature inside`).to.be.at.most(face.right + 0.5);
                if (!full) {
                  // R2725003: on the normal face it is the art window's.
                  const frame = boxes(cf, ".cf-art-frame")[0];
                  expect(frame, `${where}: the art window`).to.not.eq(undefined);
                  if (frame !== undefined) {
                    expect(box.top, `${where}: in the window`).to.be.at.least(frame.top - 0.5);
                    expect(box.bottom, `${where}: in the window`).to.be.at.most(frame.bottom + 0.5);
                  }
                }
                for (const selector of PARTS) {
                  for (const part of boxes(cf, selector)) {
                    expect(overlaps(box, part), `${where}: the signature covers ${selector}`).to.eq(false);
                  }
                }
                const text = cf.querySelector<SVGTextElement>(".cf-signature text")?.getBoundingClientRect();
                expect(text?.width ?? 0, `${where}: the name has a width`).to.be.greaterThan(width / 5);
              }
              // R2725004: the gold-foil frame and the art's Radiant paint on a Radiant face alone.
              expect(cf.getAttribute("data-radiant-face"), where).to.eq(radiant ? "true" : null);
              expect(cf.querySelector(".cf-art")?.getAttribute("data-art-variant"), where).to.eq(radiant ? "radiant" : "base");
              expect(getComputedStyle(cf).getPropertyValue("--cf-bezel").trim() === "#f6d77a", `${where}: the gold bezel`).to.eq(radiant);
            }
          }
        }
      });
    });
  }

  it("R2725003 under Reduce motion no look moves and the signature stands whole", () => {
    writeSettings({ reduceMotion: true });
    cy.mount(<Faces width={170} />);
    cy.get("html").should("have.attr", "data-reduce-motion", "true");
    cy.document().should((doc) => {
      for (const { id } of CARDS) {
        for (const look of LOOKS) {
          for (const radiant of [false, true]) {
            const cf = faceIn(doc, id, look.name, radiant);
            const where = `${id} ${look.name} ${radiant ? "Radiant" : "base"}`;
            expect(
              cf.getAnimations({ subtree: true }).filter((animation) => animation.playState === "running").length,
              `${where}: nothing moves`,
            ).to.eq(0);
            if (look.augments.length > 0) {
              expect(cf.getAttribute("data-art-motion"), `${where}: the art holds still`).to.eq("static");
              const text = cf.querySelector(".cf-signature text")?.getBoundingClientRect();
              expect(text?.width ?? 0, `${where}: the name is drawn`).to.be.greaterThan(0);
            }
          }
        }
      }
    });
  });
});
