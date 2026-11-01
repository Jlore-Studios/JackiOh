// R2655000, R2655003: the full-art tiers in a real browser. apps/web's vitest suite proves the classes
// and attributes a tier's face wears (cards/fullArtTiers.test.tsx) and what the stylesheet declares
// (cards/cosmetics.test.tsx); what only a layout and style engine can say is measured here: one
// full-art face of each card type (a Unit, a Spell, a Field Spell, a Field Trap, and two tokens that
// wear their parent's full art, R2655002) at each tier, at the hand and inspect sizes, keeps its cost
// gem, rarity gem, set mark, crest, name banner and stats apart, and a Mythic's signature is on the
// art, clear of all of them; under Reduce motion no tier moves and the signature stands whole.
//
// Each face is the viewer's own copy (`OwnCopy`) shown at a tier by `FullArtShown`, as the
// Collection's preview shows one, so a card's three tiers stand side by side with no pick written.
// No `FullArtOwned` is mounted, so every full art counts as owned, as in practice.

import { CATALOG } from "@jackioh/cards";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import {
  FULL_ARTS_OF_CARD,
  FULL_ART_TIERS,
  FullArtShown,
  OwnCopy,
  fullArtPickOf,
  fullArtSource,
  type FullArtTier,
} from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];

/** One full-art face of each card type, and two listed tokens (R2655002). */
const CARDS = [
  { id: "classicplus-046", why: "a Unit (Felinor Flagbearer: stats in the bottom corners, a Legendary crest)" },
  { id: "classicplus-019", why: "a Spell (League of Losers: no stats, a Legendary crest)" },
  { id: "classic-090", why: "a Field Spell (In Too Deep: a Mythic crest)" },
  { id: "core-018", why: "a Field Trap (Bread and Butter, the Trap row: no crest)" },
  { id: "classicplus-019-1", why: "a token Unit (Top Loser, printed Legendary)" },
  { id: "classicplus-012-1", why: "a token Spell (Devour, printed Legendary)" },
] as const;

/**
 * The inspect-sized and hand-sized full faces card-faces.cy.tsx measures (B15), and a small face (a
 * hand card under FACE_TEXT_MIN_HEIGHT_PX tall, 140 px), whose name takes the band a signature sits
 * in on a larger face.
 */
const WIDTHS = [270, 170, 100] as const;

/** What a face's parts must keep apart: none of them may cover another. */
const PARTS = [".cost-gem", ".cf-gem", ".cf-set", ".cf-crest", ".card-name", ".cf-atk", ".cf-hp", ".cf-signature"] as const;

/**
 * What a full-art face adds to the small face's layout: the signature. A small face
 * (FACE_TEXT_MIN_HEIGHT_PX) is every hand card's layout, full art or not, whose grown cost gem
 * reaches under a Legendary's crest; there only it must keep clear of everything else.
 */
const SMALL_FACE_PARTS: readonly string[] = [".cf-signature"];

/** The widest face drawn at the small layout (100 px wide is 140 px tall, under 150). */
const SMALL_FACE_MAX_WIDTH = 100;

/** Retries cover `useFitText`'s passes, and a signature's wipe, which ends 2.8 s after it mounts. */
const SETTLE_TIMEOUT_MS = 8_000;

function heightFor(width: number): number {
  return Math.round(width / FACE_ASPECT);
}

function overlaps(a: DOMRect, b: DOMRect): boolean {
  const SLACK = 0.5;
  return a.left < b.right - SLACK && b.left < a.right - SLACK && a.top < b.bottom - SLACK && b.top < a.bottom - SLACK;
}

/** The pick that shows `tier` of the full art the card `id` prints (its parent's, for a token). */
function pickFor(id: string, tier: FullArtTier): boolean | string {
  const source = fullArtSource(id);
  const fullArt = FULL_ARTS_OF_CARD.get(source)?.get(tier);
  if (fullArt === undefined) throw new Error(`no ${tier} full art for ${source}`);
  return fullArtPickOf(source, fullArt);
}

function Faces({ width }: { width: number }) {
  return (
    <OwnCopy own>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
        {CARDS.flatMap(({ id }) =>
          FULL_ART_TIERS.map((tier) => (
            <div key={`${id}-${tier}`} data-face={`${id}-${tier}`} style={{ width, height: heightFor(width), flex: "none" }}>
              <FullArtShown defId={fullArtSource(id)} pick={pickFor(id, tier)}>
                <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant: false })} layout="full" />
              </FullArtShown>
            </div>
          )),
        )}
      </div>
    </OwnCopy>
  );
}

/** The face of `id` at `tier`. */
function faceIn(doc: Document, id: string, tier: FullArtTier): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-face="${id}-${tier}"] > .cf`);
  if (cf === null) throw new Error(`no face for ${id} at ${tier}`);
  return cf;
}

/** The parts of `cf` that draw: those whose box is not hidden. */
function drawnParts(cf: HTMLElement): { selector: string; box: DOMRect }[] {
  return PARTS.flatMap((selector) =>
    [...cf.querySelectorAll<HTMLElement>(selector)]
      .filter((element) => getComputedStyle(element).display !== "none" && element.getBoundingClientRect().width > 0)
      .map((element) => ({ selector, box: element.getBoundingClientRect() })),
  );
}

describe("R2655003 the full-art tiers on real faces", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    cy.viewport(1280, 900);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
  });

  for (const width of WIDTHS) {
    it(`R2655003 every type at every tier keeps the cost gem, rarity gem, crest and banner clear at ${width}px`, () => {
      cy.mount(<Faces width={width} />);
      cy.document().should((doc) => {
        for (const { id, why } of CARDS) {
          for (const tier of FULL_ART_TIERS) {
            const where = `${id} (${why}) at ${tier}, ${width}px`;
            const cf = faceIn(doc, id, tier);
            const face = cf.getBoundingClientRect();
            // The control: the face fills its box, and it is the tier's full art.
            expect(face.width, `${where}: the face fills its box`).to.be.greaterThan(width / 2);
            expect(cf.getAttribute("data-full-art"), where).to.eq("true");
            expect(cf.getAttribute("data-full-art-tier"), where).to.eq(tier);
            const parts = drawnParts(cf);
            for (const selector of [".cost-gem", ".cf-gem", ".card-name"]) {
              expect(parts.some((part) => part.selector === selector), `${where}: ${selector} draws`).to.eq(true);
            }
            expect(parts.some((part) => part.selector === ".cf-signature"), `${where}: the signature`).to.eq(tier === "Mythic");
            for (const part of parts) {
              expect(part.box.left, `${where}: ${part.selector} inside`).to.be.at.least(face.left - 0.5);
              expect(part.box.right, `${where}: ${part.selector} inside`).to.be.at.most(face.right + 0.5);
              expect(part.box.top, `${where}: ${part.selector} inside`).to.be.at.least(face.top - 0.5);
              expect(part.box.bottom, `${where}: ${part.selector} inside`).to.be.at.most(face.bottom + 0.5);
            }
            for (const [at, part] of parts.entries()) {
              for (const other of parts.slice(at + 1)) {
                const measured = width > SMALL_FACE_MAX_WIDTH || SMALL_FACE_PARTS.includes(part.selector) || SMALL_FACE_PARTS.includes(other.selector);
                if (!measured) continue;
                expect(overlaps(part.box, other.box), `${where}: ${part.selector} covers ${other.selector}`).to.eq(false);
              }
            }
            // The signature stands on the art, its name a real width, the frame clipping what moves.
            if (tier !== "Epic") {
              const frame = cf.querySelector<HTMLElement>(".cf-art-frame");
              expect(frame === null ? "" : getComputedStyle(frame).overflow, `${where}: the frame clips`).to.eq("hidden");
            }
            if (tier === "Mythic") {
              const text = cf.querySelector<SVGTextElement>(".cf-signature text");
              const box = text?.getBoundingClientRect();
              expect(box?.width ?? 0, `${where}: the name has a width`).to.be.greaterThan(width / 4);
              expect(getComputedStyle(text as Element).fontFamily, `${where}: the script face`).to.contain("Dancing Script");
            }
          }
        }
      });
    });
  }

  it("R2655003 with motion allowed the Legendary and Mythic move and the Epic holds still, and the name writes itself once", () => {
    cy.mount(<Faces width={170} />);
    cy.document().should((doc) => {
      for (const { id } of CARDS) {
        const animations = (tier: FullArtTier) => {
          const cf = faceIn(doc, id, tier);
          return [cf.querySelector(".cf-art-frame > .cf-art"), cf.querySelector(".cf-full-art-sweep")].map((element) =>
            element === null ? "none" : getComputedStyle(element).animationName,
          );
        };
        expect(animations("Epic"), `${id} Epic`).to.deep.eq(["none", "none"]);
        expect(animations("Legendary"), `${id} Legendary`).to.deep.eq(["full-art-drift", "full-art-sweep"]);
        expect(animations("Mythic"), `${id} Mythic`).to.deep.eq(["full-art-drift", "full-art-sweep"]);
        const wipe = faceIn(doc, id, "Mythic").querySelector(".cf-signature-wipe");
        expect(wipe === null ? "none" : getComputedStyle(wipe).animationName, `${id}'s wipe`).to.eq("full-art-sign-wipe");
      }
    });
    // Once the wipe has run, the name stands whole and still: no transform left on the wipe or the ink.
    cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
      for (const { id } of CARDS) {
        const cf = faceIn(doc, id, "Mythic");
        for (const selector of [".cf-signature-wipe", ".cf-signature-ink"]) {
          const element = cf.querySelector(selector);
          const transform = element === null ? "" : getComputedStyle(element).transform;
          expect(["none", "matrix(1, 0, 0, 1, 0, 0)"], `${id} ${selector}`).to.include(transform);
        }
        const running = cf.getAnimations({ subtree: true }).filter((animation) => animation.playState === "running");
        expect(running.length, `${id}: the drift and the sweep go on`).to.be.greaterThan(0);
      }
    });
  });

  it("R2655003 under Reduce motion no tier moves and the signature stands whole", () => {
    writeSettings({ reduceMotion: true });
    cy.mount(<Faces width={170} />);
    cy.get("html").should("have.attr", "data-reduce-motion", "true");
    cy.document().should((doc) => {
      for (const { id } of CARDS) {
        for (const tier of FULL_ART_TIERS) {
          const cf = faceIn(doc, id, tier);
          const where = `${id} at ${tier}`;
          expect(cf.getAttribute("data-full-art-tier"), where).to.eq(tier);
          expect(
            cf.getAnimations({ subtree: true }).filter((animation) => animation.playState === "running").length,
            `${where}: nothing moves`,
          ).to.eq(0);
          for (const element of cf.querySelectorAll(".cf-art, .cf-full-art-sweep, .cf-signature-wipe, .cf-signature-ink")) {
            expect(getComputedStyle(element).animationName, `${where}: ${element.className.toString()}`).to.eq("none");
          }
          if (tier === "Mythic") {
            const wipe = cf.querySelector(".cf-signature-wipe");
            expect(wipe === null ? "" : getComputedStyle(wipe).transform, `${where}: the wipe is open`).to.eq("none");
            const signature = cf.querySelector(".cf-signature")?.getBoundingClientRect();
            const text = cf.querySelector(".cf-signature text")?.getBoundingClientRect();
            expect(text?.width ?? 0, `${where}: the name is drawn`).to.be.greaterThan(0);
            expect(text?.left ?? 0, `${where}: the name starts inside`).to.be.at.least((signature?.left ?? 0) - 0.5);
            expect(text?.right ?? 0, `${where}: the name ends inside`).to.be.at.most((signature?.right ?? 0) + 0.5);
          }
        }
      }
    });
  });
});
