// R3013002: a card's proficiency stamp (apps/web/src/cards/ProficiencyStamp.tsx, stamps.css) laid out in a real
// browser. apps/web's vitest suite proves which faces wear one (cards/stamps.test.tsx) and that no stamp colour is
// Radiant's gold (cards/cosmetics.test.tsx); what only a layout engine can say is measured here: on the full face at
// the inspect and hand-preview widths (normal, full art and the longest text), on a hand-size face and on the board
// minion at its zone sizes, base and Radiant, the stamp draws inside the face and keeps clear of every part: the
// cost gem, the rarity gem, the set mark, the crest, the name, the type line, the rules box (with Radiant's gold
// marks in it), the stats, the state rail and, on the minion, the keyword marks, the zzz, the Vanilla stamp and the
// portrait's oval.
//
// Each face is the viewer's own copy (`worn`, below an `OwnCopy`) of a card whose account copy wears the top stamp
// (the most ornate, with its ribbon tails), shown in a look by `FullArtShown` as card-versions.cy.tsx shows one.

import { CATALOG } from "@jackioh/cards";
import { PROFICIENCY_LEVELS } from "@jackioh/server-config";
import type { UnitView } from "@jackioh/shared";
import { CardFace, FACE_ASPECT, MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { resetCosmeticsForTests } from "../../../apps/web/src/cards/cosmetics.ts";
import { FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { replaceOwnProficiency, resetProficiencyForTests } from "../../../apps/web/src/cards/stamps.ts";
import { VERSION_FULL_ART_ID } from "../../../apps/web/src/cards/versions.ts";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

/** A Unit and a Spell offered the Full art version (card-versions.cy.tsx's), and the longest text there is. */
const CARDS = [
  { id: "classicplus-012", why: "a Unit (The Mother Pancake: stats in the bottom corners, a Legendary crest)" },
  { id: "stinkyscape-154", why: "a Spell (Dice of Doom: no stats, a Mythic crest)" },
] as const;
const LONGEST = { id: "meditative-008", why: "a Spell (Reach the Summit: eleven Ascent lines, its rules box full)" } as const;
/** The board minion's card: Gary the Gambler, a Unit with a crest. */
const MINION = "core-004";

/** The inspect-sized and hand-preview-sized full faces card-faces.cy.tsx measures (B15), and a hand card's. */
const WIDTHS = [270, 170] as const;
const HAND_WIDTH = 80;
/** Unit boxes on the board (keyword-visuals.cy.tsx's) and a hover preview's. */
const MINION_SIZES = [
  { width: 43, height: 60 },
  { width: 58, height: 80 },
  { width: 81, height: 113 },
  { width: 120, height: 150 },
] as const;

/** What a face's parts are: none may cover the stamp, nor it them. */
const PARTS = [".cost-gem", ".cf-gem", ".cf-set", ".cf-crest", ".cf-element", ".card-name", ".card-type", ".card-text", ".cf-tags", ".cf-atk", ".cf-hp", ".cf-states"] as const;
/** The normal face's art window too, which a full-art face spreads under everything. */
const NORMAL_PARTS = [...PARTS, ".cf-art-frame"] as const;
/** The board minion's. */
const MINION_PARTS = [".cost-gem", ".cf-crest", ".card-name", ".stat-attack", ".stat-health", ".keywords", ".cf-sick", ".cf-vanilla", ".cf-states"] as const;

/** A sick, Vanilla unit with keyword marks down its left side: every mark a minion can wear. */
const BUSY: Partial<UnitView> = { keywords: [{ kind: "Taunt" }, { kind: "Poisonous" }, { kind: "Lifesteal" }, { kind: "Charge" }], sick: true, vanilla: true };

/** How far a part may run into the stamp's box before it counts as covering it: sub-pixel rounding. */
const SLACK_PX = 0.5;
/** The unit circle an ellipse scales to: a point nearer its centre than this lies inside it. */
const UNIT_RADIUS = 1;

function heightFor(width: number): number {
  return Math.round(width / FACE_ASPECT);
}

function overlaps(a: DOMRect, b: DOMRect): boolean {
  return a.left < b.right - SLACK_PX && b.left < a.right - SLACK_PX && a.top < b.bottom - SLACK_PX && b.top < a.bottom - SLACK_PX;
}

/** Whether the box `box` reaches into the ellipse inscribed in `oval`: the box's point nearest the centre, scaled to a unit circle. */
function entersOval(box: DOMRect, oval: DOMRect): boolean {
  const cx = (oval.left + oval.right) / 2;
  const cy = (oval.top + oval.bottom) / 2;
  const rx = oval.width / 2;
  const ry = oval.height / 2;
  const nx = (Math.min(Math.max(cx, box.left), box.right) - cx) / rx;
  const ny = (Math.min(Math.max(cy, box.top), box.bottom) - cy) / ry;
  return nx * nx + ny * ny < UNIT_RADIUS;
}

/** The boxes of `cf`'s parts matching `selector` that draw. */
function boxes(cf: HTMLElement, selector: string): DOMRect[] {
  return [...cf.querySelectorAll<HTMLElement>(selector)]
    .filter((element) => getComputedStyle(element).display !== "none" && element.getBoundingClientRect().width > 0)
    .map((element) => element.getBoundingClientRect());
}

function Faces() {
  const looks = [
    { name: "normal", pick: false as const },
    { name: "full", pick: VERSION_FULL_ART_ID },
  ];
  return (
    <OwnCopy own>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
        {WIDTHS.flatMap((width) =>
          CARDS.flatMap(({ id }) =>
            looks.flatMap((look) =>
              [false, true].map((radiant) => (
                <div key={`${String(width)}-${id}-${look.name}-${String(radiant)}`} data-face={`${String(width)}-${id}-${look.name}-${radiant ? "radiant" : "base"}`} style={{ width, height: heightFor(width), flex: "none" }}>
                  <FullArtShown defId={id} pick={look.pick} augments={[]}>
                    <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" worn />
                  </FullArtShown>
                </div>
              )),
            ),
          ),
        )}
        {WIDTHS.flatMap((width) =>
          [false, true].map((radiant) => (
            <div key={`${String(width)}-longest-${String(radiant)}`} data-face={`${String(width)}-${LONGEST.id}-normal-${radiant ? "radiant" : "base"}`} style={{ width, height: heightFor(width), flex: "none" }}>
              <CardFace face={faceModel({ defId: LONGEST.id, def: CATALOG[LONGEST.id] as CardDef, radiant })} layout="full" worn />
            </div>
          )),
        )}
        {CARDS.flatMap(({ id }) =>
          [false, true].map((radiant) => (
            <div key={`hand-${id}-${String(radiant)}`} data-face={`hand-${id}-${radiant ? "radiant" : "base"}`} style={{ width: HAND_WIDTH, height: heightFor(HAND_WIDTH), flex: "none" }}>
              <CardFace face={faceModel({ defId: id, def: CATALOG[id] as CardDef, radiant })} layout="full" worn ownHand />
            </div>
          )),
        )}
        {MINION_SIZES.flatMap(({ width, height }) =>
          [false, true].flatMap((radiant) =>
            [false, true].map((busy) => {
              const u = unit("p1", { defId: MINION, radiant, ...(busy ? BUSY : {}) });
              const face = faceModel({ defId: MINION, def: CATALOG[MINION] as CardDef, radiant, live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords } });
              return (
                <div key={`minion-${String(width)}-${String(radiant)}-${String(busy)}`} data-face={`minion-${String(width)}-${radiant ? "radiant" : "base"}-${busy ? "busy" : "bare"}`} style={{ width, height, flex: "none", position: "relative" }}>
                  <MinionFace face={face} unit={u} worn />
                </div>
              );
            }),
          ),
        )}
      </div>
    </OwnCopy>
  );
}

/** Every face of the mount, by its `data-face`. */
function faces(doc: Document): [string, HTMLElement][] {
  return [...doc.querySelectorAll<HTMLElement>("[data-face] > .cf")].map((cf) => [cf.parentElement?.getAttribute("data-face") ?? "", cf]);
}

describe("R3013002 a card's proficiency stamp on real faces", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    resetProficiencyForTests();
    const top = { plays: 200, level: PROFICIENCY_LEVELS, stamp: PROFICIENCY_LEVELS };
    const played: readonly string[] = [...CARDS.map((card) => card.id), LONGEST.id, MINION];
    replaceOwnProficiency(played.map((cardId) => ({ cardId, ...top })));
    cy.viewport(1600, 1200);
  });

  after(() => {
    __resetSettingsForTests();
    resetCosmeticsForTests();
    resetProficiencyForTests();
  });

  it("R3013002 a stamp keeps clear of every part", () => {
    cy.mount(<Faces />);
    cy.document().should((doc) => {
      const all = faces(doc);
      expect(all.length).to.eq(WIDTHS.length * CARDS.length * 2 * 2 + WIDTHS.length * 2 + CARDS.length * 2 + MINION_SIZES.length * 2 * 2);
      for (const [where, cf] of all) {
        const face = cf.getBoundingClientRect();
        const minion = cf.getAttribute("data-layout") === "minion";
        const full = cf.getAttribute("data-full-art") === "true";
        const radiant = where.includes("radiant");
        expect(cf.getAttribute("data-radiant-face"), `${where}: the Radiant frame`).to.eq(radiant ? "true" : null);
        const stamps = boxes(cf, ".cf-stamp");
        expect(stamps.length, `${where}: one stamp`).to.eq(1);
        const stamp = stamps[0] as DOMRect;
        // It draws, inside the face.
        expect(stamp.width, `${where}: the stamp has a size`).to.be.greaterThan(4);
        expect(Math.abs(stamp.width - stamp.height), `${where}: the stamp is square`).to.be.lessThan(1);
        expect(stamp.left, `${where}: inside`).to.be.at.least(face.left - SLACK_PX);
        expect(stamp.right, `${where}: inside`).to.be.at.most(face.right + SLACK_PX);
        expect(stamp.top, `${where}: inside`).to.be.at.least(face.top - SLACK_PX);
        expect(stamp.bottom, `${where}: inside`).to.be.at.most(face.bottom + SLACK_PX);
        const parts = minion ? MINION_PARTS : full ? PARTS : NORMAL_PARTS;
        for (const selector of parts) {
          for (const part of boxes(cf, selector)) {
            expect(overlaps(stamp, part), `${where}: the stamp covers ${selector}`).to.eq(false);
          }
        }
        if (minion) {
          for (const portrait of boxes(cf, ".cf-portrait")) expect(entersOval(stamp, portrait), `${where}: the stamp covers the portrait`).to.eq(false);
        }
      }
    });
  });
});
