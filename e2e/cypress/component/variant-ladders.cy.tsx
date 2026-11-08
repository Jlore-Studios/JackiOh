// R3015007, R3015002 (#3015): one card's variants as ladders of levels, in a real browser. apps/web's vitest suite proves
// the ladders' model and markup (collection/collection.test.tsx) and every level's attributes (cards/variantLevels.test.tsx,
// cards/fullArtTiers.test.tsx); what only a layout and style engine can say is measured here, on The Mother Pancake, a
// Legendary Unit offered eight variants (the Default, Full art, Alt art, the Sketch and Pixel, and R3017001 Night,
// Stained-glass window and Tarot card, its side of each of part 6's pairs):
//  - its faces at every level of every ladder, base and Radiant: a variant draws one picture at every level, a level
//    adding only the full-art frame, its picture's motion and a signature (the signed level over its animated full
//    art's own picture, R3015002), the variants' pictures apart; a foil layers on a level (Full art's signed one, worn by
//    the other seat's copy); and a Radiant face keeps its gold frame at every level, which no level wears (R2725004);
//  - its Collection view's ladders (R3015007): each variant's rungs in rank order, joined by a rule, the next one ringed
//    in dashed teal with its picture half lit, a locked one past it dimmed, and the equipped one ringed as the strip's
//    equipped item is.
// R3343001: a Legendary's Default draws Extended art at both its levels, its picture past its window across the face, so
// its two levels are held to one picture inside the face, and the Default's picture is held apart from each other
// variant's, window to window, on Beam v4, an Epic Unit offered the same art levels, whose Default draws the normal face.
//
// Each face is the viewer's own copy (`worn`, below an `OwnCopy`) shown at a level by `FullArtShown`, as the Collection's
// preview shows one, so the levels stand side by side with no pick written. No `FullArtOwned` is mounted over the faces,
// so every level counts as owned where it is offered, as in practice.

import { CATALOG } from "@jackioh/cards";
import "../../../apps/web/src/auth/tavern.css";
import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";
import { DEFAULT_CARD_COSMETICS, resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { FullArtOwned, FullArtShown, OwnCopy } from "../../../apps/web/src/cards/fullArt.ts";
import { TheirCopy, type SeatLoadout } from "../../../apps/web/src/cards/loadouts.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import {
  DEFAULT_VARIANT,
  VERSION_ALT_FULL_ART_ANIMATED_ID,
  VERSION_ALT_FULL_ART_ID,
  VERSION_ALT_FULL_ART_SIGNED_ID,
  VERSION_ALT_ART_ID,
  VERSION_ANIMATED_ID,
  VERSION_FULL_ART_ANIMATED_ID,
  VERSION_FULL_ART_ID,
  VERSION_FULL_ART_SIGNED_ID,
  VERSION_NIGHT_ID,
  VERSION_PIXEL_ID,
  VERSION_SKETCH_FULL_ART_ANIMATED_ID,
  VERSION_SKETCH_FULL_ART_ID,
  VERSION_SKETCH_FULL_ART_SIGNED_ID,
  VERSION_SKETCH_ID,
  VERSION_STAINED_GLASS_ID,
  VERSION_TAROT_ID,
  artPictureOf,
  defaultLookOf,
  isFullBleed,
  ladderOf,
  levelsOn,
  lookOfLevel,
  variantsOn,
} from "../../../apps/web/src/cards/versions.ts";
import CardCosmeticsView from "../../../apps/web/src/collection/CardCosmeticsView.tsx";
import { collectionTestid as T } from "../../../apps/web/src/collection/collection.ts";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";

type CardDef = (typeof CATALOG)[string];
type Point = { x: number; y: number };
type Shot = { width: number; height: number; data: Uint8ClampedArray; scale: number };

/** The Mother Pancake: a Legendary Unit, one of the 21 full-art cards, offered eight variants; R3343001: its Default draws Extended art. */
const CARD = "classicplus-012";
/** R3343001: Beam v4, an Epic Unit offered the same art levels, below Legendary so its Default draws the normal face. */
const WINDOWED = "stinkyscape-054";
/** The foil the other seat's copy wears on Full art's signed level. */
const FOIL = "foil-rainbow";
/** Small enough that every face lies inside the captured viewport: three rows of eleven at most. */
const FACE_WIDTH = 100;
/** A Radiant face's gold bezel (cards.css), which no level wears on a base face (R2725004). */
const RADIANT_BEZEL = "#f6d77a";
/** The tavern's teal (auth/tavern.css `--tavern-teal`, #3fb6a8), which rings the next rung. */
const TAVERN_TEAL = "rgb(63, 182, 168)";

/** The grid the art windows are sampled on, in CSS pixels, and the inset from their edges (card-versions.cy.tsx's). */
const SAMPLE_STEP_PX = 6;
const SAMPLE_INSET_PX = 3;
/** A channel must differ this much for a point to count as different: more than compression noise. */
const PIXEL_DELTA_MIN = 4;
/** The share of an art window that must differ between two variants' pictures. */
const PICTURE_SHARE_MIN = 0.2;
/** The most of a window that may differ between two levels of one variant in the same frame: noise. */
const PICTURE_SAME_MAX = 0.05;
/**
 * How far a channel must move for two levels of one picture to count as drawn differently there: past the few levels a
 * signed face's light can shift its art by (card-versions.cy.tsx's), far below what another picture moves.
 */
const SAME_PIXEL_DELTA_MIN = 16;
/** Enough room for the faces to mount and the art to draw. */
const SETTLE_TIMEOUT_MS = 8_000;
/** How far apart two rungs' tops may lie and count as one row, in CSS pixels. */
const SAME_ROW_PX = 1;

/** One level of a variant a card wears: the Default's first, the normal face, is null. */
type Rung = { readonly card: string; readonly variant: string; readonly level: string | null };

/** R3015000: every level of every variant `id` is offered, lowest first, the Default's with the normal face. */
function rungsOf(id: string): Rung[] {
  return variantsOn(id).flatMap((variant) => [
    ...(variant === DEFAULT_VARIANT ? [{ card: id, variant, level: null }] : []),
    ...levelsOn(id, variant).map((level) => ({ card: id, variant, level })),
  ]);
}

const RUNGS = rungsOf(CARD);

/** R3343001: the levels of `WINDOWED` its pairs sample: the Default's two and each other variant's art level. */
const WINDOWED_LEVELS: readonly (string | null)[] = [
  null,
  VERSION_ANIMATED_ID,
  VERSION_ALT_ART_ID,
  VERSION_SKETCH_ID,
  VERSION_PIXEL_ID,
  VERSION_NIGHT_ID,
  VERSION_STAINED_GLASS_ID,
  VERSION_TAROT_ID,
];
const WINDOWED_RUNGS = rungsOf(WINDOWED).filter(({ level }) => WINDOWED_LEVELS.includes(level));

/** Every face the ladders mount but the foiled one. */
const ALL_RUNGS: readonly Rung[] = [...RUNGS, ...WINDOWED_RUNGS];

/** The level whose picture every level of `variant` draws (R3015002): its first, the normal face for the Default. */
function firstOf(variant: string): string | null {
  return variant === DEFAULT_VARIANT ? null : (ladderOf(variant)[0] ?? null);
}

function keyOf(level: string | null): string {
  return level ?? "normal";
}

/** The `data-face` key of `card`'s face at `level`: `CARD`'s by its level alone, `WINDOWED`'s marked. */
function faceKey(card: string, level: string | null): string {
  return card === CARD ? keyOf(level) : `windowed-${keyOf(level)}`;
}

/** The other seat's copy: Full art's signed level with a foil over it, as the server froze its owner's loadout. */
const FOILED: SeatLoadout = { seat: { ...DEFAULT_CARD_COSMETICS }, cards: { [CARD]: { foil: FOIL, fullArt: VERSION_FULL_ART_SIGNED_ID } } };

function Ladders({ radiant }: { radiant: boolean }) {
  const def = CATALOG[CARD] as CardDef;
  const size = { width: FACE_WIDTH, height: Math.round(FACE_WIDTH / FACE_ASPECT), flex: "none" } as const;
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 8, padding: 8, background: "#1b1f2b" }}>
      <OwnCopy own>
        {ALL_RUNGS.map(({ card, level }) => (
          <div key={faceKey(card, level)} data-face={`${faceKey(card, level)}-${radiant ? "radiant" : "base"}`} style={size}>
            <FullArtShown defId={card} pick={level ?? false}>
              <CardFace face={faceModel({ defId: card, def: CATALOG[card] as CardDef, radiant })} layout="full" worn />
            </FullArtShown>
          </div>
        ))}
      </OwnCopy>
      {radiant ? null : (
        <div data-face="foiled-base" style={size}>
          <TheirCopy loadout={FOILED}>
            <CardFace face={faceModel({ defId: CARD, def, radiant: false })} layout="full" />
          </TheirCopy>
        </div>
      )}
    </div>
  );
}

/** R3015007: the card's Collection view, owning Full art's first level and the Sketch's first two. */
function View() {
  const owned = new Map([[CARD, new Set([VERSION_FULL_ART_ID, VERSION_SKETCH_ID, VERSION_SKETCH_FULL_ART_ID])]]);
  return (
    <div className="tavern app-shell">
      <FullArtOwned owned={owned}>
        <CardCosmeticsView def={CATALOG[CARD] as CardDef} owned={false} onClose={() => undefined} />
      </FullArtOwned>
    </div>
  );
}

function faceIn(doc: Document, key: string, radiant: boolean): HTMLElement {
  const cf = doc.querySelector<HTMLElement>(`[data-face="${key}-${radiant ? "radiant" : "base"}"] > .cf`);
  if (cf === null) throw new Error(`no face for ${key}`);
  return cf;
}

/** The boxes of `cf`'s parts matching `selector` that draw. */
function boxes(cf: HTMLElement, selector: string): DOMRect[] {
  return [...cf.querySelectorAll<HTMLElement>(selector)]
    .filter((element) => getComputedStyle(element).display !== "none" && element.getBoundingClientRect().width > 0)
    .map((element) => element.getBoundingClientRect());
}

/** The viewport as Chrome painted it, decoded to RGBA, with the device pixels per CSS pixel (card-versions.cy.tsx's). */
function screenshot(name: string): Cypress.Chainable<Shot> {
  let path = "";
  return cy
    .screenshot(name, {
      capture: "viewport",
      overwrite: true,
      disableTimersAndAnimations: false,
      onAfterScreenshot: (_el, props) => {
        path = props.path;
      },
    })
    .then(() => cy.readFile(path, null))
    .then((bytes: Uint8Array) => cy.window().then((win) => ({ bytes, innerWidth: win.innerWidth })))
    .then(({ bytes, innerWidth }) =>
      cy.wrap(
        (async (): Promise<Shot> => {
          const bitmap = await createImageBitmap(new Blob([new Uint8Array(bytes)], { type: "image/png" }));
          const canvas = document.createElement("canvas");
          canvas.width = bitmap.width;
          canvas.height = bitmap.height;
          const ctx = canvas.getContext("2d");
          if (ctx === null) throw new Error("this browser has no 2D canvas");
          ctx.drawImage(bitmap, 0, 0);
          return { width: bitmap.width, height: bitmap.height, data: ctx.getImageData(0, 0, bitmap.width, bitmap.height).data, scale: bitmap.width / innerWidth };
        })(),
        { log: false },
      ),
    );
}

function pixelAt(shot: Shot, { x, y }: Point): [number, number, number] {
  const at = (Math.round(y * shot.scale) * shot.width + Math.round(x * shot.scale)) * 4;
  return [shot.data[at] ?? -1, shot.data[at + 1] ?? -1, shot.data[at + 2] ?? -1];
}

/** The offsets of the sampling grid over an art window `box`, down to `until` (from its top) where given: above a signature. */
function windowOffsets(box: DOMRect, until: number = box.height - SAMPLE_INSET_PX): Point[] {
  const points: Point[] = [];
  for (let y = SAMPLE_INSET_PX; y < until; y += SAMPLE_STEP_PX) {
    for (let x = SAMPLE_INSET_PX; x < box.width - SAMPLE_INSET_PX; x += SAMPLE_STEP_PX) points.push({ x, y });
  }
  return points;
}

/** The share of `offsets` at which the windows `a` and `b` of `shot` differ by `delta` or more in a channel. */
function differingShare(shot: Shot, a: DOMRect, b: DOMRect, offsets: readonly Point[], delta: number = PIXEL_DELTA_MIN): number {
  const differing = offsets.filter((offset) => {
    const one = pixelAt(shot, { x: a.left + offset.x, y: a.top + offset.y });
    const two = pixelAt(shot, { x: b.left + offset.x, y: b.top + offset.y });
    return one.some((channel, i) => Math.abs(channel - (two[i] ?? -1)) >= delta);
  });
  return differing.length / offsets.length;
}

/**
 * R3015002: the pairs of levels in one frame whose windows draw the same picture (above the signature where the second
 * signs), and the pairs of different variants' levels in one frame whose pictures lie apart.
 */
const SAME: readonly (readonly [string, string | null, string, boolean])[] = [
  // R3343001: the Default's Extended art at both its levels, inside the face, and the normal face at both in its window.
  [CARD, null, VERSION_ANIMATED_ID, false],
  [WINDOWED, null, VERSION_ANIMATED_ID, false],
  [CARD, VERSION_PIXEL_ID, `${VERSION_PIXEL_ID}-animated`, false],
  [CARD, VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID, true],
  [CARD, VERSION_ALT_FULL_ART_ANIMATED_ID, VERSION_ALT_FULL_ART_SIGNED_ID, true],
  [CARD, VERSION_SKETCH_FULL_ART_ANIMATED_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID, true],
  // R3017002: each of part 6's variants, its animated level over its art level's picture.
  [CARD, VERSION_NIGHT_ID, `${VERSION_NIGHT_ID}-animated`, false],
  [CARD, VERSION_STAINED_GLASS_ID, `${VERSION_STAINED_GLASS_ID}-animated`, false],
  [CARD, VERSION_TAROT_ID, `${VERSION_TAROT_ID}-animated`, false],
];
/** R3343001: a pair with the Default's normal face is sampled on `WINDOWED`, whose Default draws it in its window. */
const APART: readonly (readonly [string, string | null, string])[] = [
  [WINDOWED, null, VERSION_ALT_ART_ID],
  [WINDOWED, null, VERSION_SKETCH_ID],
  [WINDOWED, null, VERSION_PIXEL_ID],
  [CARD, VERSION_ALT_ART_ID, VERSION_SKETCH_ID],
  [CARD, VERSION_FULL_ART_ID, VERSION_ALT_FULL_ART_ID],
  [CARD, VERSION_FULL_ART_ID, VERSION_SKETCH_FULL_ART_ID],
  [CARD, VERSION_ALT_FULL_ART_ID, VERSION_SKETCH_FULL_ART_ID],
  // R3017002: part 6's pictures, apart from the normal one and from one another.
  [WINDOWED, null, VERSION_NIGHT_ID],
  [WINDOWED, null, VERSION_STAINED_GLASS_ID],
  [WINDOWED, null, VERSION_TAROT_ID],
  [CARD, VERSION_NIGHT_ID, VERSION_STAINED_GLASS_ID],
  [CARD, VERSION_NIGHT_ID, VERSION_TAROT_ID],
  [CARD, VERSION_STAINED_GLASS_ID, VERSION_TAROT_ID],
];

describe("R3015007 R3015002 a card's variants as ladders of levels", () => {
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

  it("R3015007 R3015002 one card's ladders, base and Radiant, a foil on one level", () => {
    expect(variantsOn(CARD), "the card's variants").to.deep.eq([DEFAULT_VARIANT, "full-art", "alt-art", "sketch", "pixel", "night", "stained-glass", "tarot"]);
    expect(WINDOWED_RUNGS.map(({ level }) => level), `${WINDOWED}'s levels`).to.have.members([...WINDOWED_LEVELS]);
    // Animated foil off holds every picture still, so one screenshot compares the pictures alone.
    writeCardSettings({ animatedFoil: false });
    for (const radiant of [false, true]) {
      const side = radiant ? "Radiant" : "base";
      cy.mount(<Ladders radiant={radiant} />);
      cy.get("[data-face] > .cf", { timeout: SETTLE_TIMEOUT_MS }).should("have.length", ALL_RUNGS.length + (radiant ? 0 : 1));
      cy.get("[data-face] .cf-art[data-art-pending]").should("not.exist");
      cy.get('[data-art-moving="true"]').should("not.exist");
      cy.document().its("fonts.status").should("eq", "loaded");
      cy.document().should((doc) => {
        for (const { card, variant, level } of ALL_RUNGS) {
          const cf = faceIn(doc, faceKey(card, level), radiant);
          const where = `${card} ${side} ${keyOf(level)}`;
          const look = lookOfLevel(level);
          const picture = artPictureOf(firstOf(variant));
          const art = cf.querySelector(".cf-art");
          expect(cf.getAttribute("data-level"), where).to.eq(level);
          // R3343001: the Default's levels draw the card's default look, Extended art on `CARD`, the normal face on `WINDOWED`.
          expect(cf.getAttribute("data-version"), `${where}: the look`).to.eq(variant === DEFAULT_VARIANT ? defaultLookOf(card) : look.version);
          // One picture per variant, at every level.
          expect(art?.getAttribute("data-art-picture") ?? "normal", `${where}: the variant's picture`).to.eq(picture);
          // What a level adds: the full-art frame, its picture's motion (held still here) and a signature.
          expect(cf.getAttribute("data-full-art"), `${where}: the frame`).to.eq(isFullBleed(level) ? "true" : null);
          expect(cf.getAttribute("data-art-motion"), `${where}: the motion`).to.eq(look.animated ? "static" : null);
          expect(boxes(cf, ".cf-signature").length, `${where}: the signature`).to.eq(look.signed ? 1 : 0);
          // R2725004: a Radiant face keeps its gold frame and its art's Radiant paint at every level; no level wears it.
          expect(cf.getAttribute("data-radiant-face"), where).to.eq(radiant ? "true" : null);
          expect(art?.getAttribute("data-art-variant"), where).to.eq(radiant ? "radiant" : "base");
          expect(getComputedStyle(cf).getPropertyValue("--cf-bezel").trim() === RADIANT_BEZEL, `${where}: the gold bezel`).to.eq(radiant);
          expect(cf.getAttribute("data-cosmetic-foil"), `${where}: no foil of its own`).to.eq(null);
        }
        if (!radiant) {
          // A foil layers on a level: the other seat's copy at Full art's signed level, foiled, signed.
          const foiled = faceIn(doc, "foiled", false);
          expect(foiled.getAttribute("data-level"), "the foiled face").to.eq(VERSION_FULL_ART_SIGNED_ID);
          expect(foiled.getAttribute("data-cosmetic-foil"), "the foiled face").to.eq(FOIL);
          expect(foiled.getAttribute("data-full-art"), "the foiled face").to.eq("true");
          expect(boxes(foiled, ".cf-signature").length, "the foiled face's signature").to.eq(1);
          expect(foiled.querySelector(".cf-art")?.getAttribute("data-art-picture") ?? null, "the foiled face's picture").to.eq(null);
        }
      });
      const taken: { windows?: Map<string, DOMRect>; signatures?: Map<string, DOMRect> } = {};
      cy.document().then((doc) => {
        const windows = new Map<string, DOMRect>();
        const signatures = new Map<string, DOMRect>();
        for (const { card, level } of ALL_RUNGS) {
          const key = faceKey(card, level);
          const cf = faceIn(doc, key, radiant);
          const art = cf.querySelector(".cf-art")?.getBoundingClientRect();
          if (art === undefined) throw new Error(`no art window for ${key}`);
          // R3343001: what of the picture lies inside the face, since Extended art's runs past its window and the face's edge.
          const face = cf.getBoundingClientRect();
          const [left, top] = [Math.max(art.left, face.left), Math.max(art.top, face.top)];
          windows.set(key, new DOMRect(left, top, Math.min(art.right, face.right) - left, Math.min(art.bottom, face.bottom) - top));
          const [signature] = boxes(cf, ".cf-signature");
          if (signature !== undefined) signatures.set(key, signature);
        }
        taken.windows = windows;
        taken.signatures = signatures;
      });
      screenshot(`variant-ladders-${side}`).then((shot) => {
        const windows = taken.windows ?? new Map<string, DOMRect>();
        const windowOf = (card: string, level: string | null): DOMRect => {
          const key = faceKey(card, level);
          const box = windows.get(key);
          if (box === undefined) throw new Error(`no window for ${key}`);
          // Every window sampled lies inside what the screenshot captured.
          expect(Math.ceil(box.bottom * shot.scale), `${key}: inside the shot`).to.be.at.most(shot.height);
          return box;
        };
        const shares: Record<string, number> = {};
        for (const [card, a, b, signs] of SAME) {
          const [one, two] = [windowOf(card, a), windowOf(card, b)];
          expect(Math.round(one.width), `${card} ${keyOf(a)} and ${b}: one frame`).to.eq(Math.round(two.width));
          // Where the second signs, only what lies above its signature, which it adds.
          const signature = taken.signatures?.get(faceKey(card, b));
          expect(signature !== undefined, `${card} ${b}: its signature`).to.eq(signs);
          const offsets = windowOffsets(one, signature === undefined ? undefined : signature.top - two.top - SAMPLE_INSET_PX);
          expect(offsets.length, `${card} ${keyOf(a)} and ${b}: sampled points`).to.be.greaterThan(20);
          const share = differingShare(shot, one, two, offsets, SAME_PIXEL_DELTA_MIN);
          shares[`${side} ${card} ${keyOf(a)} = ${b}`] = share;
          expect(share, `${side} ${card} ${keyOf(a)} and ${b}: one picture`).to.be.at.most(PICTURE_SAME_MAX);
        }
        for (const [card, a, b] of APART) {
          const [one, two] = [windowOf(card, a), windowOf(card, b)];
          expect(Math.round(one.width), `${card} ${keyOf(a)} and ${b}: one frame`).to.eq(Math.round(two.width));
          const offsets = windowOffsets(one);
          expect(offsets.length, `${card} ${keyOf(a)} and ${b}: sampled points`).to.be.greaterThan(20);
          const share = differingShare(shot, one, two, offsets);
          shares[`${side} ${card} ${keyOf(a)} / ${b}`] = share;
          expect(share, `${side} ${card} ${keyOf(a)} and ${b}: two variants' pictures`).to.be.at.least(PICTURE_SHARE_MIN);
        }
        expect(Object.keys(shares).length, "every pair sampled").to.eq(SAME.length + APART.length);
        cy.task("layout:report", { spec: "variant-ladders", side, shares });
      });
    }
  });

  it("R3015007 the card view lays each ladder out in rank order, joined, the next rung ringed in teal and a locked one dimmed", () => {
    // A border worn on the card, so the strip's equipped ring stands beside the ladders' to compare.
    writeCardPick(CARD, "border", "border-wood");
    cy.mount(<View />);
    cy.get('[data-testid^="collection-ladder-"]', { timeout: SETTLE_TIMEOUT_MS }).should("have.length", variantsOn(CARD).length);
    cy.document().should((doc) => {
      for (const section of doc.querySelectorAll<HTMLElement>('[data-testid^="collection-ladder-"]')) {
        const variant = section.dataset.variant ?? "";
        const items = [...section.querySelectorAll<HTMLElement>(":scope > ol > li")];
        expect(items.length, `${variant}: its rungs`).to.be.greaterThan(1);
        // Lowest first, in reading order.
        const rungs = items.map((item) => {
          const button = item.querySelector<HTMLElement>(".collection-rung");
          if (button === null) throw new Error(`${variant}: a rung is missing`);
          return button;
        });
        for (let i = 1; i < rungs.length; i += 1) {
          const [before, after] = [rungs[i - 1]?.getBoundingClientRect(), rungs[i]?.getBoundingClientRect()];
          if (before === undefined || after === undefined) continue;
          const later = after.top > before.top + SAME_ROW_PX || (Math.abs(after.top - before.top) <= SAME_ROW_PX && after.left > before.left);
          expect(later, `${variant}: rung ${String(i)} after rung ${String(i - 1)}`).to.eq(true);
          // Each rung past the first is joined to the one below it by a rule across the gap.
          const rule = getComputedStyle(items[i] as HTMLElement, "::before");
          expect(rule.content, `${variant}: the rule before rung ${String(i)}`).to.not.be.oneOf(["none", "normal"]);
          expect(rule.width, `${variant}: the rule's length`).to.eq("8px");
        }
        for (const rung of rungs) {
          const where = `${variant} ${rung.dataset.level ?? ""}`;
          const style = getComputedStyle(rung);
          const picture = rung.querySelector<HTMLElement>(".collection-strip__picture");
          const filter = picture === null ? "" : getComputedStyle(picture).filter;
          if (rung.dataset.next === "true") {
            // The next rung: a dashed teal ring, its picture half lit, its label teal.
            expect(style.borderTopStyle, `${where}: dashed`).to.eq("dashed");
            expect(style.borderTopColor, `${where}: teal`).to.eq(TAVERN_TEAL);
            expect(filter, `${where}: half lit`).to.contain("grayscale(0.5)");
            const label = rung.querySelector<HTMLElement>(".collection-rung__next");
            expect(label === null ? "" : getComputedStyle(label).color, `${where}: the label`).to.eq(TAVERN_TEAL);
          } else if (rung.dataset.owned === "true") {
            expect(style.borderTopStyle, `${where}: solid`).to.eq("solid");
            expect(filter, `${where}: lit`).to.eq("none");
          } else {
            // A locked rung past the next: dimmed, and naming what it needs.
            expect(style.borderTopStyle, `${where}: solid`).to.eq("solid");
            expect(filter, `${where}: dimmed`).to.contain("grayscale(1)");
            expect(rung.textContent ?? "", `${where}: what it needs`).to.contain("Needs");
          }
        }
      }
      // The states this ownership gives: Full art's first owned and its animated next; the Sketch's first two owned.
      const state = (id: string): string | undefined => {
        const rung = doc.querySelector<HTMLElement>(`[data-testid="${T.rung(id)}"]`);
        return rung === null ? undefined : `${rung.dataset.owned ?? ""} ${rung.dataset.next ?? ""}`;
      };
      expect(state(VERSION_FULL_ART_ID)).to.eq("true false");
      expect(state(VERSION_FULL_ART_ANIMATED_ID)).to.eq("false true");
      expect(state(VERSION_FULL_ART_SIGNED_ID)).to.eq("false false");
      expect(state(VERSION_SKETCH_FULL_ART_ID)).to.eq("true false");
      expect(state(VERSION_SKETCH_FULL_ART_ANIMATED_ID)).to.eq("false true");
      expect(state(VERSION_ALT_FULL_ART_ID)).to.eq("false false");
    });
    // Choosing an owned rung wears it: its ring is the strip's equipped ring, the face at that level.
    cy.get(`[data-testid="${T.rung(VERSION_SKETCH_FULL_ART_ID)}"]`).click();
    cy.get(`[data-testid="${T.rung(VERSION_SKETCH_FULL_ART_ID)}"]`).should("have.attr", "data-equipped", "true");
    cy.document().should((doc) => {
      const rung = doc.querySelector<HTMLElement>(`[data-testid="${T.rung(VERSION_SKETCH_FULL_ART_ID)}"]`);
      const strip = doc.querySelector<HTMLElement>(`[data-testid="${T.stripItem("border-wood")}"]`);
      if (rung === null || strip === null) throw new Error("the equipped rung or strip item is missing");
      expect(strip.dataset.equipped, "the strip's equipped border").to.eq("true");
      expect(getComputedStyle(rung).borderTopColor, "the equipped ring").to.eq(getComputedStyle(strip).borderTopColor);
      expect(getComputedStyle(rung).borderTopStyle, "the equipped ring").to.eq("solid");
      const face = doc.querySelector<HTMLElement>(`[data-testid="${T.cardViewFace}"] .cf`);
      expect(face?.getAttribute("data-level"), "the large card").to.eq(VERSION_SKETCH_FULL_ART_ID);
      expect(face?.querySelector(".cf-art")?.getAttribute("data-art-picture"), "the large card's picture").to.eq("sketch");
    });
  });
});
