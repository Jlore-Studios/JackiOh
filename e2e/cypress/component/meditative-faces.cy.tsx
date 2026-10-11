// MN09 (#552): every Meditative face reads, measured in a real layout engine. The set is read off the
// catalog (every entry of `Meditative`, its tokens included), so a card a later part adds is measured
// the day it lands, with no list to update here.
//
//   FIT    every Meditative card's base and Radiant face, and its Chinese face on both (ME-CN, #516,
//          R1301: `crates/cards/chinese.json` gives every Meditative card one), at the inspect view's
//          two sizes — the hover preview's (PREVIEW_HEIGHT_PX tall) and the touch sheet's on the
//          smallest phone (`.inspect-face--sheet`'s 56dvh of a 640 px screen) — keeps `.card-name` and
//          `.card-text` inside their boxes on both axes (cards/fit.ts), and nothing clamps but a face
//          whose printed text runs past TEXT_TIER_MAX.xl (which may clamp only on the sheet, at the
//          reading floor, since the sheet prints the rules whole beside it).
//   BOARD  the same cards where the board draws them: in the hand, as a Unit on the field and as a
//          face-up Field Spell or Trap in the backrow, through `Game` with the real catalog at
//          1280x720, 390x844 and 360x780. Every name stays inside its plate unclamped, except that a
//          board minion's plate may end a long name in an ellipsis at its floor (cards.css, fit.ts:
//          the hover preview shows it whole), and every face has a box.
//   MARKS  the set mark (R503, the ensō) is on the face and clear of the gems, the name, the rules
//          box, the stats and the crest at every size it shows, and gone below SET_MARK_MIN_FACE_PX;
//          "All Tribes" (R1382) leads the tags line of a card with every tribal tag (R1424) and sits
//          inside the face, clear of the rules box and the stats — on the catalog's cards that have
//          them, and on a Meditative Unit given them, so the check holds before #87 lands.
//   ART    each card's procedural art is its own: no two Meditative cards draw the same picture,
//          base or Radiant.
//
// "With the set previewed": no face reads what ships (CardFace.tsx and cards/setMark.ts take the
// set straight off the definition), so a face drawn from the catalog is the face a player will see
// once the set ships (R1420); nothing here has to pretend `SHIPPED_SETS` lists it.
//
// The Created mark (ME-CREATED, `CardView.created`) belongs to its system, which #31 The Conductor's
// part builds; until the view carries it there is no mark to place, and this spec has nothing to say.
//
// Every measurement sits inside `.should()`, so it retries while `useFitText` settles, and each
// carries a control (the face fills its box, the name has a box and reads the card's name), since a
// collapsed box "fits" perfectly.

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID, TRIBAL_TAGS } from "@jackioh/engine/config";
import type { BackrowView, CardDef, CardView, PlayerView, UnitView } from "@jackioh/shared";

import { ALL_TRIBES, CardFace, FACE_ASPECT, TEXT_TIER_MAX, faceModel } from "../../../apps/web/src/cards/index.ts";
import { FIT_FLOOR_PX } from "../../../apps/web/src/cards/constants.ts";
import { PREVIEW_HEIGHT_PX } from "../../../apps/web/src/cards/inspect/constants.ts";
import { SET_MARK_MIN_FACE_PX } from "../../../apps/web/src/cards/setMark.ts";
import { CatalogContext, lookupFromDefs } from "../../../apps/web/src/game/catalog.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { baseView, card, emptySide, faceUpBackrow, unit } from "../../../apps/web/src/test/fixtures.ts";

/** Every Meditative entry in the catalog, tokens included, in catalog order. */
const MEDITATIVE: readonly CardDef[] = Object.values(CATALOG).filter((def) => def.set === "Meditative" && def.id !== GLITCH_DEF_ID);

/** Retries cover `useFitText`'s rounds and the ResizeObserver passes behind them. */
const SETTLE_TIMEOUT_MS = 20_000;

/** Subpixel slack for edges the browser rounds. */
const SLACK_PX = 1;

/** `.inspect-face--sheet`'s height, `min(380px, 56dvh)` (inspect.css), on the smallest phone screen. */
const SHEET_SHARE = 0.56;
const SMALLEST_PHONE = { width: 360, height: 640 } as const;

type Face = { label: string; radiant: boolean; chinese: boolean };

const FACES: readonly Face[] = [
  { label: "base", radiant: false, chinese: false },
  { label: "radiant", radiant: true, chinese: false },
  { label: "Chinese base", radiant: false, chinese: true },
  { label: "Chinese radiant", radiant: true, chinese: true },
];

type InspectSize = { label: string; height: number; clampLong: boolean };

/** The inspect view's two face sizes: on the hover preview nothing clamps, on the phone sheet only the longest texts may. */
const INSPECT_SIZES: readonly InspectSize[] = [
  { label: "the hover preview", height: PREVIEW_HEIGHT_PX, clampLong: false },
  { label: "the phone sheet", height: Math.round(Math.min(PREVIEW_HEIGHT_PX, SMALLEST_PHONE.height * SHEET_SHARE)), clampLong: true },
];

function widthFor(height: number): number {
  return Math.round(height * FACE_ASPECT);
}

function modelOf(def: CardDef, face: Face) {
  return faceModel({ defId: def.id, def, radiant: face.radiant, ...(face.chinese ? { chinese: true } : {}) });
}

/** What a face prints: its whole text, a Radiant face's included (R277). */
function printedLength(def: CardDef, face: Face): number {
  return modelOf(def, face).text.full.length;
}

function where(def: CardDef, what: string): string {
  return `${def.id} (${def.name}) ${what}`;
}

/** Overflow on either axis, with the one pixel of rounding fit.ts allows itself. */
function spills(el: HTMLElement): string | null {
  if (el.scrollHeight > el.clientHeight + SLACK_PX) return `scrollHeight ${String(el.scrollHeight)} > clientHeight ${String(el.clientHeight)}`;
  if (el.scrollWidth > el.clientWidth + SLACK_PX) return `scrollWidth ${String(el.scrollWidth)} > clientWidth ${String(el.clientWidth)}`;
  return null;
}

function shown(el: Element): boolean {
  return getComputedStyle(el).display !== "none" && getComputedStyle(el).visibility !== "hidden";
}

function inside(inner: DOMRect, outer: DOMRect): boolean {
  return inner.left >= outer.left - SLACK_PX && inner.right <= outer.right + SLACK_PX && inner.top >= outer.top - SLACK_PX && inner.bottom <= outer.bottom + SLACK_PX;
}

/**
 * A board minion's name plate (cards.css, `.cf[data-layout="minion"] .card-name`) prints at no less than
 * this many pixels; a name that would need less ends in an ellipsis at it instead (`data-clamped`, fit.ts),
 * and the hover preview shows it whole. Mirrors the plate's `max(7.5px, …)`.
 */
const MINION_NAME_FLOOR_PX = 7.5;

/**
 * Every way the name plate of one face fails: it must read the card's name, have a box, and fit — except
 * a board minion's plate, whose name may end in an ellipsis at its floor, inside the minion, by design.
 */
function nameProblems(root: Element, expected: string, at: string): string[] {
  const name = root.querySelector<HTMLElement>(".card-name");
  if (name === null) return [`${at}: no .card-name`];
  const problems: string[] = [];
  if ((name.textContent ?? "") !== expected) problems.push(`${at}: .card-name reads "${name.textContent ?? ""}", not "${expected}"`);
  if (name.clientWidth === 0 || name.clientHeight === 0) problems.push(`${at}: .card-name has no box`);
  const minion = name.closest<HTMLElement>('.cf[data-layout="minion"]');
  if (minion !== null && name.getAttribute("data-clamped") === "true") {
    const style = getComputedStyle(name);
    if (style.textOverflow !== "ellipsis") problems.push(`${at}: a clamped minion name has no ellipsis`);
    if (parseFloat(style.fontSize) < MINION_NAME_FLOOR_PX - 0.05) problems.push(`${at}: a clamped minion name prints at ${style.fontSize}, under the floor`);
    if (!inside(name.getBoundingClientRect(), minion.getBoundingClientRect())) problems.push(`${at}: a clamped minion name runs off the minion`);
    return problems;
  }
  const spill = spills(name);
  if (spill !== null) problems.push(`${at}: .card-name ${spill}`);
  if (name.getAttribute("data-clamped") !== null) problems.push(`${at}: the name is clamped`);
  return problems;
}

/* --------------------------------------------------------------------------------------- FIT */

function InspectGrid({ height, face }: { height: number; face: Face }) {
  const width = widthFor(height);
  return (
    <div style={{ display: "flex", flexWrap: "wrap", alignItems: "flex-start", gap: 8, padding: 8 }}>
      {MEDITATIVE.map((def) => (
        <div key={def.id} data-fit-box={def.id} style={{ width, height, flex: "none" }}>
          <CardFace face={modelOf(def, face)} layout="full" />
        </div>
      ))}
    </div>
  );
}

function inspectProblems(doc: Document, def: CardDef, size: InspectSize, face: Face): string[] {
  const at = where(def, `${face.label} at ${size.label} (${String(widthFor(size.height))}x${String(size.height)})`);
  const cf = doc.querySelector<HTMLElement>(`[data-fit-box="${def.id}"] > .cf`);
  if (cf === null) return [`${at}: no .cf in its box`];
  const problems: string[] = [];

  const box = cf.getBoundingClientRect();
  if (Math.abs(box.width - widthFor(size.height)) > SLACK_PX || Math.abs(box.height - size.height) > SLACK_PX) {
    problems.push(`${at}: .cf is ${String(box.width)}x${String(box.height)}`);
  }
  problems.push(...nameProblems(cf, modelOf(def, face).name, at));

  const text = cf.querySelector<HTMLElement>(".card-text");
  const printed = modelOf(def, face).text.full;
  if (text === null) {
    if (printed !== "") problems.push(`${at}: no .card-text`);
    return problems;
  }
  if (printed !== "" && (text.clientWidth === 0 || text.clientHeight === 0)) problems.push(`${at}: .card-text has no box`);
  const clamped = text.getAttribute("data-clamped") === "true";
  const mayClamp = size.clampLong && printedLength(def, face) > TEXT_TIER_MAX.xl;
  if (clamped && !mayClamp) {
    problems.push(`${at}: the rules text is clamped (${String(printedLength(def, face))} characters printed)`);
  } else if (clamped) {
    // A clamped box is cut at a line by design (fit.ts): it stays on the face at the reading floor.
    const own = text.getBoundingClientRect();
    if (own.bottom > box.bottom + SLACK_PX || own.top < box.top - SLACK_PX) problems.push(`${at}: a clamped .card-text runs off the face`);
    const px = parseFloat(getComputedStyle(text).fontSize);
    if (px < FIT_FLOOR_PX - 0.05) problems.push(`${at}: a clamped .card-text prints at ${px.toFixed(2)}px, under the floor`);
  } else {
    const spill = spills(text);
    if (spill !== null) problems.push(`${at}: .card-text ${spill}`);
  }
  return problems;
}

describe("MN09: every Meditative face fits its frame at the inspect view's sizes", () => {
  it("MN09 the premise: the set is in the catalog with a Chinese face for every card, and its longest faces are named", () => {
    expect(MEDITATIVE.length, "Meditative entries in the catalog").to.be.greaterThan(0);
    for (const def of MEDITATIVE) {
      const chinese = modelOf(def, { label: "Chinese base", radiant: false, chinese: true });
      expect(chinese.chinese, `${def.id} has a Chinese face (#516)`).to.eq(true);
      expect(chinese.name, `${def.id}'s Chinese name`).to.not.eq("");
    }
    // The faces the phone sheet may clamp: today Knowledge Breaker's Radiant face alone. A card that
    // joins it is not an error, but its text is long enough that a person should read it once.
    const long = MEDITATIVE.flatMap((def) => FACES.filter((face) => printedLength(def, face) > TEXT_TIER_MAX.xl).map((face) => `${def.id} ${face.label}`));
    expect(long).to.have.members(["meditative-045 radiant"]);
  });

  for (const size of INSPECT_SIZES) {
    for (const face of FACES) {
      it(`MN09 every Meditative ${face.label} face at ${size.label} (${String(size.height)} px tall): name and rules within their boxes`, () => {
        cy.viewport(1280, 900);
        cy.mount(<InspectGrid height={size.height} face={face} />);
        cy.get("[data-fit-box] > .cf").should("have.length", MEDITATIVE.length);
        cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
          const problems = MEDITATIVE.flatMap((def) => inspectProblems(doc, def, size, face));
          expect(problems, `${face.label} faces at ${size.label}`).to.deep.equal([]);
        });
      });
    }
  }
});

/* ------------------------------------------------------------------------------------- BOARD */

/** The viewports the board is played on: BUILD M5-T1's two and the narrowest phone (docs/polish/7-mobile-ux.md). */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
  { label: "narrow phone", width: 360, height: 780 },
] as const;

const HAND_SIZE = 10;
const ZONES = 5;

const UNITS = MEDITATIVE.filter((def) => def.type === "Unit");
const BACKROW = MEDITATIVE.filter((def) => def.type !== "Unit" && def.type !== "Spell");

function chunks<T>(items: readonly T[], size: number): T[][] {
  const out: T[][] = [];
  for (let at = 0; at < items.length; at += size) out.push(items.slice(at, at + size));
  return out;
}

function printedCost(def: CardDef): number {
  const cost = def.cost;
  if (typeof cost === "number") return cost;
  if (cost === "X") return 0;
  return cost.base;
}

function handCard(def: CardDef): CardView {
  return card({ defId: def.id, cost: printedCost(def) });
}

function fieldUnit(owner: "p1" | "p2", def: CardDef): UnitView {
  const attack = def.base.attack ?? 0;
  const health = Math.max(def.base.health ?? 1, 1);
  return unit(owner, { defId: def.id, cost: printedCost(def), attack, maxHealth: health, health, keywords: def.base.keywords });
}

function backrowCard(owner: "p1" | "p2", def: CardDef): BackrowView {
  return faceUpBackrow(owner, { defId: def.id, cost: printedCost(def), type: def.type });
}

type Batch = { hand: CardDef[]; units: CardDef[]; backrow: CardDef[] };

/** Every card in a hand once, every Unit on the field once and every backrow card in a backrow once, a board at a time. */
const BATCHES: readonly Batch[] = (() => {
  const hands = chunks(MEDITATIVE, HAND_SIZE);
  const units = chunks(UNITS, ZONES * 2);
  const backrow = chunks(BACKROW, ZONES * 2);
  const count = Math.max(hands.length, units.length, backrow.length);
  return Array.from({ length: count }, (_, at) => ({ hand: hands[at] ?? [], units: units[at] ?? [], backrow: backrow[at] ?? [] }));
})();

function row<T>(items: readonly T[]): (T | null)[] {
  return Array.from({ length: ZONES }, (_, lane) => items[lane] ?? null);
}

function batchView(batch: Batch): PlayerView {
  return baseView({
    you: emptySide("p1", {
      mana: { current: 10, max: 10 },
      hand: batch.hand.map(handCard),
      units: row(batch.units.slice(0, ZONES).map((def) => fieldUnit("p1", def))),
      backrow: row(batch.backrow.slice(0, ZONES).map((def) => backrowCard("p1", def))),
    }),
    opponent: emptySide("p2", {
      hand: { count: 4 },
      units: row(batch.units.slice(ZONES).map((def) => fieldUnit("p2", def))),
      backrow: row(batch.backrow.slice(ZONES).map((def) => backrowCard("p2", def))),
    }),
  });
}

const DEFS_BY_ID = new Map(MEDITATIVE.map((def) => [def.id, def]));

function boardProblems(doc: Document, batch: Batch, at: string): string[] {
  const problems: string[] = [];
  const seen = new Set<string>();
  const faces = [...doc.querySelectorAll<HTMLElement>('[data-testid="board"] [data-testid^="hand-card-"], [data-testid="board"] [data-testid^="card-"]')];
  for (const root of faces) {
    const named = root.querySelector<HTMLElement>(".card-name");
    if (named === null || !shown(named)) continue;
    const name = named.textContent ?? "";
    const def = [...DEFS_BY_ID.values()].find((candidate) => candidate.name === name);
    if (def === undefined) {
      problems.push(`${at}: a face reads "${name}", which is no Meditative card's name`);
      continue;
    }
    const kind = (root.getAttribute("data-testid") ?? "").startsWith("hand-card-") ? "in the hand" : "on the field";
    seen.add(`${def.id} ${kind}`);
    problems.push(...nameProblems(root, def.name, where(def, `${kind} at ${at}`)));
    const face = root.querySelector<HTMLElement>(".cf");
    if (face !== null) {
      const own = face.getBoundingClientRect();
      if (own.width < 1 || own.height < 1) problems.push(`${where(def, `${kind} at ${at}`)}: its face has no box`);
    }
  }
  for (const def of batch.hand) if (!seen.has(`${def.id} in the hand`)) problems.push(`${where(def, `at ${at}`)}: not drawn in the hand`);
  for (const def of [...batch.units, ...batch.backrow]) if (!seen.has(`${def.id} on the field`)) problems.push(`${where(def, `at ${at}`)}: not drawn on the field`);
  return problems;
}

describe("MN09: every Meditative card where the board draws it — the hand, a unit zone and the backrow", () => {
  for (const viewport of VIEWPORTS) {
    for (const [index, batch] of BATCHES.entries()) {
      const label = `${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`;
      it(`MN09 board ${String(index + 1)} of ${String(BATCHES.length)} at ${label}: every name inside its plate`, () => {
        cy.viewport(viewport.width, viewport.height);
        cy.mount(
          <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
            <div className="app-shell app-shell--wide">
              <Game view={batchView(batch)} legal={[]} onAction={() => undefined} />
            </div>
          </CatalogContext.Provider>,
        );
        cy.get('[data-testid="board"]').should("be.visible");
        cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
          expect(boardProblems(doc, batch, label), `board ${String(index + 1)} at ${label}`).to.deep.equal([]);
        });
      });
    }
  }
});

/* ------------------------------------------------------------------------------------- MARKS */

/** What the set mark must never cover (R503). */
const CLEAR_OF = [".cost-gem", ".card-name", ".cf-gem", ".card-text", ".cf-atk", ".cf-hp", ".cf-crest"] as const;

function overlaps(a: DOMRect, b: DOMRect): boolean {
  const SLACK = 0.5;
  return a.left < b.right - SLACK && b.left < a.right - SLACK && a.top < b.bottom - SLACK && b.top < a.bottom - SLACK;
}

/** The mark's sizes: the two inspect sizes, and R503's 170 px and 100 px wide faces (the smaller a hand card's). */
const MARK_HEIGHTS = [PREVIEW_HEIGHT_PX, INSPECT_SIZES[1]?.height ?? PREVIEW_HEIGHT_PX, Math.round(170 / FACE_ASPECT), Math.round(100 / FACE_ASPECT)] as const;

function MarkGrid({ height, radiant }: { height: number; radiant: boolean }) {
  const width = widthFor(height);
  return (
    <div style={{ display: "flex", flexWrap: "wrap", alignItems: "flex-start", gap: 8, padding: 8 }}>
      {MEDITATIVE.map((def) => (
        <div key={def.id} data-marked={def.id} style={{ width, height, flex: "none" }}>
          <CardFace face={faceModel({ defId: def.id, def, radiant })} layout="full" />
        </div>
      ))}
    </div>
  );
}

function markProblems(doc: Document, def: CardDef, at: string): string[] {
  const cf = doc.querySelector<HTMLElement>(`[data-marked="${def.id}"] > .cf`);
  const mark = cf?.querySelector<HTMLElement>(".cf-set") ?? null;
  if (cf === null || mark === null) return [`${where(def, at)}: no set mark`];
  const problems: string[] = [];
  if (mark.getAttribute("data-set-mark") !== "meditative") problems.push(`${where(def, at)}: the mark is "${mark.getAttribute("data-set-mark") ?? ""}", not the ensō`);
  if (!shown(mark)) return [...problems, `${where(def, at)}: the mark is hidden`];
  const own = mark.getBoundingClientRect();
  if (own.width < 1) problems.push(`${where(def, at)}: the mark has no box`);
  if (!inside(own, cf.getBoundingClientRect())) problems.push(`${where(def, at)}: the mark is off the face`);
  for (const selector of CLEAR_OF) {
    for (const other of cf.querySelectorAll<HTMLElement>(selector)) {
      if (!shown(other)) continue;
      if (overlaps(own, other.getBoundingClientRect())) problems.push(`${where(def, at)}: the mark covers ${selector}`);
    }
  }
  return problems;
}

describe("MN09: the set mark and the All Tribes label sit where they should on every Meditative face", () => {
  for (const height of MARK_HEIGHTS) {
    for (const radiant of [false, true]) {
      const at = `${radiant ? "radiant" : "base"} at ${String(height)} px tall`;
      it(`MN09 R503 the ensō is on every Meditative ${at} face, clear of the gems, name, text, stats and crest`, () => {
        cy.viewport(1280, 900);
        cy.mount(<MarkGrid height={height} radiant={radiant} />);
        cy.get("[data-marked] > .cf").should("have.length", MEDITATIVE.length);
        cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
          expect(MEDITATIVE.flatMap((def) => markProblems(doc, def, at))).to.deep.equal([]);
        });
      });
    }
  }

  it("MN09 R503 below SET_MARK_MIN_FACE_PX the mark is gone from every Meditative face", () => {
    cy.viewport(1280, 900);
    cy.mount(<MarkGrid height={SET_MARK_MIN_FACE_PX - 10} radiant={false} />);
    cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
      const marks = [...doc.querySelectorAll<HTMLElement>("[data-marked] .cf-set")];
      expect(marks, "every face still carries it in the DOM").to.have.length(MEDITATIVE.length);
      expect(marks.filter(shown).map((mark) => mark.closest("[data-marked]")?.getAttribute("data-marked"))).to.deep.equal([]);
    });
  });

  /** The catalog's cards with every tribal tag (R1424), and one Meditative Unit given them, so the label is placed before #87 lands. */
  const TRIBAL: readonly CardDef[] = (() => {
    const printed = MEDITATIVE.filter((def) => TRIBAL_TAGS.every((tribe) => def.tags.includes(tribe)));
    const sample = UNITS[0];
    const given: CardDef[] = sample === undefined ? [] : [{ ...sample, id: `${sample.id}-all-tribes`, tags: [...TRIBAL_TAGS, ...sample.tags.filter((tag) => !TRIBAL_TAGS.includes(tag))] }];
    return [...printed, ...given];
  })();

  for (const chinese of [false, true]) {
    it(`MN09 R1382 "All Tribes" leads the tags line, inside the face and clear of the rules and the stats${chinese ? ", on a Chinese face" : ""}`, () => {
      cy.viewport(1280, 900);
      cy.mount(
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8 }}>
          {[PREVIEW_HEIGHT_PX, Math.round(170 / FACE_ASPECT)].flatMap((height) =>
            TRIBAL.map((def) => (
              <div key={`${def.id}-${String(height)}`} data-tribal={def.id} style={{ width: widthFor(height), height, flex: "none" }}>
                <CardFace face={faceModel({ defId: def.id, def, radiant: false, ...(chinese ? { chinese: true } : {}) })} layout="full" />
              </div>
            )),
          )}
        </div>,
      );
      cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
        const faces = [...doc.querySelectorAll<HTMLElement>("[data-tribal] > .cf")];
        expect(faces.length, "a tribal face per size").to.eq(TRIBAL.length * 2);
        expect(TRIBAL.length, "at least the given one").to.be.greaterThan(0);
        for (const cf of faces) {
          const id = cf.parentElement?.getAttribute("data-tribal") ?? "";
          const tags = [...cf.querySelectorAll<HTMLElement>(".cf-tag")];
          expect(tags[0]?.getAttribute("data-tag"), `${id}: the first tag`).to.eq(ALL_TRIBES);
          expect(tags.filter((tag) => TRIBAL_TAGS.includes(tag.getAttribute("data-tag") as (typeof TRIBAL_TAGS)[number])), `${id}: no tribe printed beside it`).to.deep.equal([]);
          const line = cf.querySelector<HTMLElement>(".cf-tags");
          expect(line, `${id}: a tags line`).to.not.eq(null);
          if (line === null || tags[0] === undefined) continue;
          const own = tags[0].getBoundingClientRect();
          expect(own.width, `${id}: the label has a box`).to.be.greaterThan(0);
          expect(inside(own, cf.getBoundingClientRect()), `${id}: the label is on the face`).to.eq(true);
          expect(spills(tags[0]), `${id}: the label's words fit it`).to.eq(null);
          for (const selector of [".card-text", ".cf-atk", ".cf-hp"]) {
            const other = cf.querySelector<HTMLElement>(selector);
            if (other === null || !shown(other)) continue;
            expect(overlaps(own, other.getBoundingClientRect()), `${id}: the label covers ${selector}`).to.eq(false);
          }
        }
      });
    });
  }
});

/* --------------------------------------------------------------------------------------- ART */

describe("MN09: each Meditative card's procedural art is its own", () => {
  for (const radiant of [false, true]) {
    it(`MN09 no two Meditative ${radiant ? "radiant" : "base"} faces draw the same picture`, () => {
      cy.viewport(1280, 900);
      cy.mount(<MarkGrid height={Math.round(170 / FACE_ASPECT)} radiant={radiant} />);
      cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
        const pictures = new Map<string, string[]>();
        for (const def of MEDITATIVE) {
          const art = doc.querySelector<HTMLElement>(`[data-marked="${def.id}"] .cf-art`);
          expect(art, `${def.id} has an art window`).to.not.eq(null);
          if (art === null) continue;
          const picture = getComputedStyle(art).backgroundImage;
          expect(picture, `${def.id} draws a picture`).to.not.eq("none");
          expect(art.getBoundingClientRect().height, `${def.id}: the art has a box`).to.be.greaterThan(20);
          pictures.set(picture, [...(pictures.get(picture) ?? []), def.id]);
        }
        const shared = [...pictures.values()].filter((ids) => ids.length > 1);
        expect(shared, "cards sharing one picture").to.deep.equal([]);
      });
    });
  }
});
