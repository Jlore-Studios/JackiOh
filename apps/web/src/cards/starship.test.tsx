// R2249000: a Starship's face in play (#2249) wears the Starship's own art, drawn from its pieces and its
// launch, whatever definition its fusions gave it; its rules box prints only the Starship term and the
// pieces fused into it, each a reference that opens the piece's card. R2801000: a Dormant one bolts on
// a hull plate for each piece fused into it. R3038000: Starship is a tag, and the real engine's view
// lists a Starship's Flying and Ranged, Dormant or launched, which the board draws as any Unit's.
// R3333000: its picture is built from its pieces oldest first, each its module on its hardpoint, in the
// hull design of the name it launched under, a Dormant one in a shipyard. R3333002: its hull plates and
// its pieces list show each piece's module icon. R3335000: a launched one wears its seat's Starship
// cosmetics, a hull design, a livery, an engine trail and a piece skin.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { STARSHIP_NAMES } from "@jackioh/engine/config";
import type { Action, ActionBody, CardDef, PlayerId, PlayerView, UnitView } from "@jackioh/shared";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { CardArt } from "./art/CardArt.tsx";
import {
  LAUNCHED_DESIGNS,
  liveryPaint,
  STARSHIP_CLASSES,
  STARSHIP_DESIGN_NAMES,
  STARSHIP_DESIGNS,
  STARSHIP_MODULES_MAX,
  starshipArtKey,
  starshipArtOf,
  starshipClassOf,
  starshipDesignOf,
  type StarshipArt,
} from "./art/starshipHulls.ts";
import { HULL_DESIGNS, LIVERIES, PIECE_SKINS, STARSHIP_LOOK_STANDARD, TRAILS, type StarshipLook } from "./art/starshipLooks.ts";
import { moduleMarkup, modulePaint, STARSHIP_MODULE_IDS, starshipModuleOf } from "./art/starshipModules.ts";
import { STARSHIP_ART_MAX_BYTES, starshipSvg } from "./art/stinkyscape.ts";
import { starshipArtUri, starshipModuleIconUri } from "./art/svg.ts";
import { CardFace } from "./CardFace.tsx";
import { faceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { STARSHIP_PIECES_TESTID } from "./StarshipPieces.tsx";
import { STARSHIP_HULL_PLATES_MAX } from "./StarshipHull.tsx";
import { renderBoard } from "../test/board.tsx";
import { RADIANT_GOLD_HUE_MAX_DEG, RADIANT_GOLD_HUE_MIN_DEG, RADIANT_GOLD_LIGHTNESS_MAX, RADIANT_GOLD_LIGHTNESS_MIN, RADIANT_GOLD_SATURATION_MIN } from "./constants.ts";
import { baseView, dormantStarship, emptySide, unit } from "../test/fixtures.ts";

afterEach(cleanup);

const HULL = "stinkyscape-t-dormant-starship";
const SHIELD = "stinkyscape-038";
const CAPSULE = "stinkyscape-043";
const AP_ROUNDS = "stinkyscape-100";
const BEAM = "stinkyscape-058";
const DRILL = "stinkyscape-066";
const NETHER = "core-088";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

type Piece = { defId: string; radiant: boolean };

/** `ids` as a Starship's pieces, oldest first, on their base faces. */
function pieces(ids: readonly string[]): Piece[] {
  return ids.map((defId) => ({ defId, radiant: false }));
}

function art(defId: string, starship: { launched: boolean; list: readonly Piece[] } | null, name?: string): HTMLElement {
  const card = def(defId);
  const { container } = render(
    <CardArt defId={defId} radiant={false} tags={card.tags} type={card.type} shape="oval" name={name} starship={starship} />,
  );
  const found = container.querySelector<HTMLElement>(".cf-art");
  if (found === null) throw new Error("no art");
  return found;
}

describe("R2249000: a Starship's face", () => {
  it("R2249000 a Starship wears its own art, the same whatever definition its fusions gave it", () => {
    const list = pieces([HULL, SHIELD, CAPSULE]);
    const hull = art(HULL, { launched: false, list });
    expect(hull.getAttribute("data-art-family")).toBe("starship");
    const hullArt = hull.style.backgroundImage;
    cleanup();
    // Fused, its definition is not the hull's; its art stays the Starship's.
    const fused = art("core-008", { launched: false, list }, def("core-008").name);
    expect(fused.getAttribute("data-art-family")).toBe("starship");
    expect(fused.style.backgroundImage).toBe(hullArt);
    expect(hullArt).toContain(starshipArtUri(starshipArtOf(list, false, undefined, false)));
  });

  it("R2249000 the art grows with its pieces and lights up once it launches", () => {
    const ship = (n: number, launched: boolean, radiant: boolean): string =>
      starshipSvg(starshipArtOf(pieces([SHIELD, CAPSULE, AP_ROUNDS, BEAM, DRILL].slice(0, n)), launched, launched ? "Ember of Departure" : undefined, radiant));
    const dormant = ship(2, false, false);
    expect(ship(5, false, false)).not.toBe(dormant);
    expect(ship(2, true, false)).not.toBe(dormant);
    expect(ship(2, false, true)).not.toBe(dormant);
    expect(ship(2, false, false)).toBe(dormant);
  });

  it("R2249000 a card that is no Starship keeps its own art", () => {
    expect(art(SHIELD, null).getAttribute("data-art-family")).not.toBe("starship");
  });

  it("R2249000 in play a Starship prints no text, only the Starship term and its pieces, each a card reference", () => {
    const pieces = [
      { defId: SHIELD, radiant: false },
      { defId: CAPSULE, radiant: true },
    ];
    const face = faceModel({
      defId: HULL,
      def: def(HULL),
      radiant: false,
      inPlay: { starship: { pieces: pieces.length, name: null, list: pieces } },
    });
    expect(face.text.full).toBe("");
    const { container } = render(
      <CardDefsProvider defs={CATALOG}>
        <CardFace face={face} layout="full" />
      </CardDefsProvider>,
    );
    expect(container.querySelector(".cf-text-base")).toBeNull();
    const list = screen.getByTestId(STARSHIP_PIECES_TESTID);
    expect(list.getAttribute("data-pieces")).toBe("2");
    expect(within(list).getByText(def(SHIELD).name)).toBeTruthy();
    expect(within(list).getByText(def(CAPSULE).name)).toBeTruthy();
    expect(list.querySelector('[data-term="Starship"]')).not.toBeNull();
    expect([...list.querySelectorAll("[data-piece]")].map((node) => node.getAttribute("data-piece"))).toEqual([SHIELD, CAPSULE]);
    // The Starship's printed text is nowhere on the face.
    expect(container.textContent).not.toContain("launch this");
  });

  it("R2801000 a Dormant hull shows min(pieces, STARSHIP_HULL_PLATES_MAX) plates; a launched one none", () => {
    const platesOf = (pieces: number, launched = false): number | null => {
      const list = Array.from({ length: pieces }, () => SHIELD);
      const ship = launched
        ? unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: list.map((defId) => ({ defId })), name: "Ember of Departure", launched: 1 } })
        : dormantStarship("p1", list, { instanceId: "ship" });
      const container = renderBoard(baseView({ you: emptySide("p1", { units: [null, null, ship, null, null] }) }));
      const hull = container.querySelector('[data-testid="card-ship"] .cf-starship-hull');
      const plates = hull === null ? null : hull.querySelectorAll(".cf-starship-plate").length;
      if (hull !== null && plates !== null) {
        expect(hull.getAttribute("data-plates")).toBe(String(plates));
        expect(hull.getAttribute("aria-hidden")).toBe("true");
        // Port and starboard in turn, as the art's modules are.
        expect(hull.querySelectorAll('[data-flank="port"] .cf-starship-plate')).toHaveLength(Math.ceil(plates / 2));
        expect(hull.querySelectorAll('[data-flank="starboard"] .cf-starship-plate')).toHaveLength(Math.floor(plates / 2));
      }
      cleanup();
      return plates;
    };
    // One plate a piece, the fuse's bolt-on, up to the most a face bolts on.
    expect(platesOf(1)).toBe(1);
    expect(platesOf(3)).toBe(3);
    expect(platesOf(STARSHIP_HULL_PLATES_MAX + 4)).toBe(STARSHIP_HULL_PLATES_MAX);
    // No pieces yet, or launched: none.
    expect(platesOf(0)).toBeNull();
    expect(platesOf(3, true)).toBeNull();
  });

  it("R2249000 out of play, in the collection, the Dormant Starship prints its short text", () => {
    const face = faceModel({ defId: HULL, def: def(HULL), radiant: false });
    expect(face.text.full).toBe(
      "Flying, Ranged\nDormant until it launches. Your Starship Pieces that die fuse into this.\nActivate: Pay (4) to launch this.",
    );
  });
});

/** A side's Units, standing in its unit zones or on its backrow (R2249000: a Starship stands there first). */
function unitsOf(side: PlayerView["you"]): UnitView[] {
  const backrow = side.backrow.flatMap((card) => (card !== null && !card.faceDown && card.unit !== undefined ? [card.unit] : []));
  return [...side.units.flatMap((unit) => (unit === null ? [] : [unit])), ...backrow];
}

/** p1's Starship as the real engine shows it to p1: the Unit with a Starship's memory (R1852005). */
function shipOf(view: PlayerView): UnitView {
  const found = unitsOf(view.you).find((unit) => unit.starship !== undefined);
  if (found === undefined) throw new Error("p1 has no Starship");
  return found;
}

/** The kinds of the keyword marks the board draws on `id`. */
function marksOn(view: PlayerView, id: string): string[] {
  const container = renderBoard(view);
  const card = container.querySelector(`[data-testid="card-${id}"]`);
  if (card === null) throw new Error(`the board draws no ${id}`);
  const kinds = [...card.querySelectorAll("[data-keyword]")].map((node) => node.getAttribute("data-keyword") ?? "");
  cleanup();
  return kinds;
}

/** Every piece definition's id, in the module table's order. */
const PIECE_IDS = Object.keys(STARSHIP_MODULE_IDS);

/** A Dormant or a launched Starship's picture for `list`, its name The Ninth Life once launched. */
function shipSvg(list: readonly Piece[], launched: boolean, name = "The Ninth Life", radiant = false): string {
  return starshipSvg(starshipArtOf(list, launched, launched ? name : undefined, radiant));
}

/** R2725004's gold window (cosmetics.test.tsx): whether `hex` is Radiant's gold. */
function inRadiantGold(hex: string): boolean {
  const digits = hex.length <= 4 ? [...hex].map((digit) => digit + digit).join("") : hex;
  const [r, g, b] = [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16) / 255) as [number, number, number];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return false;
  const sat = d / (1 - Math.abs(2 * l - 1));
  const sector = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  const h = sector * 60;
  return (
    h >= RADIANT_GOLD_HUE_MIN_DEG && h <= RADIANT_GOLD_HUE_MAX_DEG && sat >= RADIANT_GOLD_SATURATION_MIN && l >= RADIANT_GOLD_LIGHTNESS_MIN && l <= RADIANT_GOLD_LIGHTNESS_MAX
  );
}

function golds(svg: string): string[] {
  return [...svg.matchAll(/#([0-9a-fA-F]{3,8})\b/g)].map((match) => match[1] ?? "").filter(inRadiantGold);
}

describe("R3333000: a Starship's picture is built from what it holds", () => {
  it("R3333000 the picture differs for any two different piece lists", () => {
    const lists: Piece[][] = [[]];
    for (const id of PIECE_IDS) lists.push(pieces([id]));
    const ten = PIECE_IDS.filter((_, k) => k % 5 === 0);
    for (const a of ten) for (const b of ten) lists.push(pieces([a, b]));
    for (let n = 3; n <= STARSHIP_MODULES_MAX; n += 1) lists.push(pieces(PIECE_IDS.slice(0, n)));
    // A Radiant piece is drawn gilded on a base hull.
    lists.push([{ defId: BEAM, radiant: true }], [{ defId: SHIELD, radiant: false }, { defId: BEAM, radiant: true }]);
    const keys = new Set(lists.map((list) => list.map((piece) => `${piece.defId}${piece.radiant ? "*" : ""}`).join(",")));
    expect(keys.size).toBe(lists.length);
    for (const launched of [false, true]) {
      const drawn = new Set(lists.map((list) => shipSvg(list, launched)));
      expect(drawn.size, launched ? "launched" : "Dormant").toBe(lists.length);
    }
    // The memo key holds what the picture is drawn from, so two lists never share a cached picture.
    expect(new Set(lists.map((list) => starshipArtKey(starshipArtOf(list, false, undefined, false)))).size).toBe(lists.length);
    // Each piece's module, in its own paint, is on the hull, oldest first.
    const list = pieces([BEAM, DRILL, SHIELD, CAPSULE]);
    const svg = shipSvg(list, true);
    const at = list.map((piece) => {
      const m = starshipModuleOf(piece.defId);
      return svg.indexOf(moduleMarkup(m, modulePaint(m)));
    });
    for (const found of at) expect(found).toBeGreaterThan(0);
    expect([...at].sort((x, y) => x - y)).toEqual(at);
    // Past the most it mounts, the oldest pieces' modules are the ones drawn.
    const many = pieces(PIECE_IDS.slice(0, STARSHIP_MODULES_MAX + 4));
    const last = starshipModuleOf(PIECE_IDS[STARSHIP_MODULES_MAX + 3] ?? "");
    expect(shipSvg(many, true)).not.toContain(moduleMarkup(last, modulePaint(last)));
  });

  it("R3333000 each of the seven names wears its own design, and the shipyard differs from every launched one", () => {
    expect(Object.keys(STARSHIP_DESIGN_NAMES).sort()).toEqual(STARSHIP_NAMES.map(([name]) => name).sort());
    expect(new Set(Object.values(STARSHIP_DESIGN_NAMES)).size).toBe(STARSHIP_NAMES.length);
    const list = pieces([BEAM, DRILL, SHIELD, CAPSULE, AP_ROUNDS]);
    const dormant = shipSvg(list, false);
    const launched = STARSHIP_NAMES.map(([name]) => shipSvg(list, true, name));
    expect(new Set([dormant, ...launched]).size).toBe(STARSHIP_NAMES.length + 1);
    // Each design its own prow, windows, livery and emblem; the shipyard's hull is bare and bears none.
    for (const field of ["prow", "windows", "emblem"] as const) {
      expect(new Set(LAUNCHED_DESIGNS.map((id) => STARSHIP_DESIGNS[id][field])).size, field).toBe(LAUNCHED_DESIGNS.length);
    }
    expect(new Set(LAUNCHED_DESIGNS.map((id) => JSON.stringify(STARSHIP_DESIGNS[id].livery))).size).toBe(LAUNCHED_DESIGNS.length);
    expect(STARSHIP_DESIGNS.shipyard.emblem).toBeNull();
    for (const [name] of STARSHIP_NAMES) {
      expect(starshipDesignOf(false, name), name).toBe("shipyard");
      expect(starshipDesignOf(true, name), name).toBe(STARSHIP_DESIGN_NAMES[name]);
    }
    // A name no design has still wears one of the seven, the same each time.
    const odd = starshipDesignOf(true, "The Unlisted Barge");
    expect(LAUNCHED_DESIGNS).toContain(odd);
    expect(starshipDesignOf(true, "The Unlisted Barge")).toBe(odd);
    // The base face wears no gold; the Radiant face gilds it (R2725004).
    for (const svg of [dormant, ...launched]) expect(golds(svg)).toEqual([]);
    expect(golds(shipSvg(list, true, "The Lantern Ferry", true)).length).toBeGreaterThan(0);
  });

  it("R3333000 the silhouette grows with its count, from a frigate to a dreadnought", () => {
    const counts = [1, 2, 3, 4, 5, 7, 8, 12];
    expect(counts.map((n) => starshipClassOf(n).id)).toEqual(["frigate", "frigate", "destroyer", "destroyer", "cruiser", "cruiser", "dreadnought", "dreadnought"]);
    for (let k = 1; k < STARSHIP_CLASSES.length; k += 1) {
      const [smaller, bigger] = [STARSHIP_CLASSES[k - 1], STARSHIP_CLASSES[k]];
      expect(bigger?.length).toBeGreaterThan(smaller?.length ?? 0);
      expect(bigger?.height).toBeGreaterThan(smaller?.height ?? 0);
      // Drawn in its window, each class still stands longer than the last.
      expect((bigger?.length ?? 0) * (bigger?.zoom ?? 0)).toBeGreaterThan((smaller?.length ?? 0) * (smaller?.zoom ?? 0));
    }
    // The same eight pieces' first n, as each class: four hulls, each its own picture.
    const shown = new Set(counts.map((n) => shipSvg(pieces(PIECE_IDS.slice(0, n)), true)));
    expect(shown.size).toBe(counts.length);
  });

  it("R3333000 a design and a livery are the painter's inputs", () => {
    const ship: StarshipArt = starshipArtOf(pieces([BEAM, SHIELD]), true, "The Silent Choir", false);
    expect(ship).toMatchObject({ design: "silent-choir", livery: null, launched: true });
    const base = starshipSvg(ship);
    // Another design, however it is named (#3330's cosmetic track hands one over).
    expect(starshipSvg({ ...ship, design: "jlockeed-skylark" })).toBe(shipSvg(ship.pieces, true, "The Jlockeed Skylark"));
    // A livery of its own repaints it; the design's own livery is the picture's when none is given.
    const livery = { hull: "#2a6a5a", trim: "#e0f0ea", stripe: "#d03a6a", window: "#bfe4ff", glow: "#6affd0" };
    const repainted = starshipSvg({ ...ship, livery });
    expect(repainted).not.toBe(base);
    expect(repainted).toContain(livery.stripe);
    expect(starshipSvg({ ...ship, livery: STARSHIP_DESIGNS["silent-choir"].livery })).toBe(base);
    expect(starshipArtKey({ ...ship, livery })).not.toBe(starshipArtKey(ship));
    expect(starshipArtUri({ ...ship, livery })).not.toBe(starshipArtUri(ship));
  });

  it("R3333000 the biggest picture stays under STARSHIP_ART_MAX_BYTES", () => {
    const biggest = [...PIECE_IDS]
      .sort((a, b) => moduleMarkup(starshipModuleOf(b), modulePaint(starshipModuleOf(b))).length - moduleMarkup(starshipModuleOf(a), modulePaint(starshipModuleOf(a))).length)
      .slice(0, STARSHIP_MODULES_MAX)
      .map((defId) => ({ defId, radiant: true }));
    for (const launched of [false, true]) {
      for (const name of STARSHIP_NAMES.map(([each]) => each)) {
        for (const radiant of [false, true]) {
          const svg = shipSvg(biggest, launched, name, radiant);
          expect(svg.length, `${name} ${String(launched)} ${String(radiant)}`).toBeLessThanOrEqual(STARSHIP_ART_MAX_BYTES);
        }
      }
    }
  });

  it("R3333000 CardArt draws a launched Starship in its name's design and a Dormant one in the shipyard", () => {
    const list = pieces([BEAM, SHIELD, CAPSULE]);
    const launched = art(HULL, { launched: true, list }, "The Lantern Ferry");
    expect(launched.getAttribute("data-art-design")).toBe("lantern-ferry");
    expect(launched.style.backgroundImage).toContain(starshipArtUri(starshipArtOf(list, true, "The Lantern Ferry", false)));
    cleanup();
    expect(art(HULL, { launched: false, list }).getAttribute("data-art-design")).toBe("shipyard");
    cleanup();
    expect(art(SHIELD, null).getAttribute("data-art-design")).toBeNull();
    cleanup();
    // On the board, the face's name is the one it launched under, and its art wears that name's design.
    const board = (ship: UnitView): HTMLElement | null =>
      renderBoard(baseView({ you: emptySide("p1", { units: [null, ship, null, null, null] }) })).querySelector<HTMLElement>('[data-testid="card-ship"] .cf-art');
    const flown = board(unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: [{ defId: BEAM }, { defId: DRILL }], name: "Frontier's Promise", launched: 1 } }));
    expect(flown?.getAttribute("data-art-design")).toBe("frontiers-promise");
    cleanup();
    const docked = board(dormantStarship("p1", [BEAM, DRILL], { instanceId: "ship" }));
    expect(docked?.getAttribute("data-art-design")).toBe("shipyard");
  });
});

/** The four kinds of Starship cosmetic, each its record of looks by id. */
const STARSHIP_LOOK_RECORDS = { starshipHull: HULL_DESIGNS, starshipLivery: LIVERIES, starshipTrail: TRAILS, starshipPieceSkin: PIECE_SKINS } as const;
/** Each kind's ids' prefix. */
const STARSHIP_LOOK_PREFIXES: Readonly<Record<keyof StarshipLook, string>> = {
  starshipHull: "starship-hull-",
  starshipLivery: "starship-livery-",
  starshipTrail: "starship-trail-",
  starshipPieceSkin: "starship-piece-skin-",
};
const LOOK_KINDS = Object.keys(STARSHIP_LOOK_RECORDS) as (keyof StarshipLook)[];

/** Looks that between them pick every cosmetic of every kind, the k-th each kind's k-th id (wrapping). */
function everyLook(): StarshipLook[] {
  const ids = LOOK_KINDS.map((kind) => Object.keys(STARSHIP_LOOK_RECORDS[kind]));
  const most = Math.max(...ids.map((list) => list.length));
  return Array.from({ length: most }, (_, k) => {
    const pick = (at: number): string => ids[at]?.[k % (ids[at]?.length ?? 1)] ?? "";
    return { starshipHull: pick(0), starshipLivery: pick(1), starshipTrail: pick(2), starshipPieceSkin: pick(3) };
  });
}

/** The 16 pieces whose modules draw the most, all Radiant: R3333000's biggest picture. */
function biggestPieces(): Piece[] {
  const weight = (defId: string): number => moduleMarkup(starshipModuleOf(defId), modulePaint(starshipModuleOf(defId))).length;
  return [...PIECE_IDS]
    .sort((a, b) => weight(b) - weight(a))
    .slice(0, STARSHIP_MODULES_MAX)
    .map((defId) => ({ defId, radiant: true }));
}

describe("R3335000: a launched Starship wears its seat's Starship cosmetics", () => {
  const list = pieces([BEAM, DRILL, SHIELD, CAPSULE, AP_ROUNDS]);
  const named = "The Ninth Life";

  it("R3335000 a launched Starship wears its seat's hull, livery, trail and piece skin", () => {
    const look: StarshipLook = {
      starshipHull: "starship-hull-corsair",
      starshipLivery: "starship-livery-arctic",
      starshipTrail: "starship-trail-ion",
      starshipPieceSkin: "starship-piece-skin-verdigris",
    };
    const found = <T,>(value: T | null | undefined): T => {
      if (value === null || value === undefined) throw new Error("no such cosmetic");
      return value;
    };
    const livery = found(LIVERIES[look.starshipLivery]);
    const trail = found(TRAILS[look.starshipTrail]);
    const skin = found(PIECE_SKINS[look.starshipPieceSkin]);
    const ship = starshipArtOf(list, true, named, false, look);
    // The hull pick's design in place of its name's, and its livery, trail and skin.
    expect(ship).toEqual({ pieces: list, launched: true, design: "corsair", livery, trail, skin, radiant: false });
    // The standard picks keep what it draws without a look: its name's design in its own livery, no trail, no skin.
    const standard = starshipArtOf(list, true, named, false, STARSHIP_LOOK_STANDARD);
    expect(standard).toMatchObject({ design: "ninth-life", livery: null, trail: null, skin: null });
    expect(starshipSvg(standard)).toBe(shipSvg(list, true, named));
    expect(starshipArtKey(standard)).toBe(starshipArtKey(starshipArtOf(list, true, named, false)));
    // An id no cosmetic has reads as its kind's standard.
    const unknown = { starshipHull: "starship-hull-gone", starshipLivery: "constructor", starshipTrail: "", starshipPieceSkin: "toString" };
    expect(starshipSvg(starshipArtOf(list, true, named, false, unknown))).toBe(starshipSvg(standard));
    // The look is drawn, and each pick alone changes the picture and its memo key.
    const svg = starshipSvg(ship);
    expect(svg).not.toBe(starshipSvg(standard));
    expect(starshipSvg({ ...ship, trail: null, skin: null })).toBe(starshipSvg({ ...standard, design: "corsair", livery }));
    for (const kind of LOOK_KINDS) {
      const one = starshipArtOf(list, true, named, false, { ...STARSHIP_LOOK_STANDARD, [kind]: look[kind] });
      expect(starshipSvg(one), kind).not.toBe(starshipSvg(standard));
      expect(starshipArtKey(one), kind).not.toBe(starshipArtKey(standard));
      expect(starshipArtUri(one), kind).not.toBe(starshipArtUri(standard));
    }
    expect(svg).toContain(livery.stripe);
    for (const color of [trail.burn, trail.core, trail.streak]) expect(svg).toContain(color);
    // Each piece's module in its skin's tones over its own paint.
    for (const piece of list) {
      const m = starshipModuleOf(piece.defId);
      expect(svg, piece.defId).toContain(moduleMarkup(m, { ...modulePaint(m), ...skin }));
      expect(svg, piece.defId).not.toContain(moduleMarkup(m, modulePaint(m)));
    }
    // Every hull pick's design is the one drawn.
    for (const [id, design] of Object.entries(HULL_DESIGNS)) {
      if (design === null) continue;
      const worn = starshipArtOf(list, true, named, false, { ...STARSHIP_LOOK_STANDARD, starshipHull: id });
      expect(worn.design, id).toBe(design);
      expect(starshipSvg(worn), id).toBe(starshipSvg({ ...starshipArtOf(list, true, named, false), design }));
    }
  });

  it("R3335000 a Dormant one keeps the shipyard", () => {
    for (const radiant of [false, true]) {
      const plain = starshipArtOf(list, false, undefined, radiant);
      expect(plain).toEqual({ pieces: list, launched: false, design: "shipyard", livery: null, radiant });
      for (const look of everyLook()) {
        for (const name of [undefined, named]) {
          const docked = starshipArtOf(list, false, name, radiant, look);
          expect(docked, JSON.stringify(look)).toEqual(plain);
          expect(starshipSvg(docked)).toBe(starshipSvg(plain));
          expect(starshipArtKey(docked)).toBe(starshipArtKey(plain));
        }
      }
    }
  });

  it("R3335000 every Starship cosmetic has a look", () => {
    for (const kind of LOOK_KINDS) {
      const record: Readonly<Record<string, unknown>> = STARSHIP_LOOK_RECORDS[kind];
      const standard = STARSHIP_LOOK_STANDARD[kind];
      expect(standard, kind).toBe(`${STARSHIP_LOOK_PREFIXES[kind]}standard`);
      expect(record[standard], kind).toBeNull();
      const ids = Object.keys(record);
      expect(ids.length, kind).toBeGreaterThan(1);
      for (const id of ids) {
        expect(id.startsWith(STARSHIP_LOOK_PREFIXES[kind]), id).toBe(true);
        if (id !== standard) expect(record[id], id).not.toBeNull();
      }
      // Each look its own.
      const looks = Object.values(record).map((value) => JSON.stringify(value));
      expect(new Set(looks).size, kind).toBe(looks.length);
    }
    const every = LOOK_KINDS.flatMap((kind) => Object.keys(STARSHIP_LOOK_RECORDS[kind]));
    expect(new Set(every).size).toBe(every.length);
    // Each hull pick a design of its own, none the shipyard's nor a name's, and every design a name's, the
    // shipyard's or a pick's.
    const picked = Object.values(HULL_DESIGNS).flatMap((design) => (design === null ? [] : [design]));
    for (const design of picked) {
      expect(STARSHIP_DESIGNS[design].id, design).toBe(design);
      expect([...LAUNCHED_DESIGNS, "shipyard"], design).not.toContain(design);
    }
    expect(new Set(picked).size).toBe(picked.length);
    expect(new Set([...LAUNCHED_DESIGNS, "shipyard", ...picked])).toEqual(new Set(Object.keys(STARSHIP_DESIGNS)));
    // The names still wear the seven, so a name's design is the one it wore (R3333000).
    expect(LAUNCHED_DESIGNS).toHaveLength(STARSHIP_NAMES.length);
    // Each design its own pair of prow and windows and its own livery.
    const designs = Object.values(STARSHIP_DESIGNS);
    expect(new Set(designs.map((design) => `${design.prow}|${design.windows}`)).size).toBe(designs.length);
    expect(new Set(designs.map((design) => JSON.stringify(design.livery))).size).toBe(designs.length);
  });

  it("R3335000 the Stinkyscape hull draws the largest picture, within STARSHIP_ART_MAX_BYTES", () => {
    const biggest = biggestPieces();
    // Its bow and galleries are its own.
    const others = Object.values(STARSHIP_DESIGNS).filter((design) => design.id !== "stinkyscape");
    expect(others.map((design) => design.prow)).not.toContain(STARSHIP_DESIGNS.stinkyscape.prow);
    expect(others.map((design) => design.windows)).not.toContain(STARSHIP_DESIGNS.stinkyscape.windows);
    for (const radiant of [false, true]) {
      const flagship = (more: Partial<StarshipArt> = {}): string => starshipSvg({ ...starshipArtOf(biggest, true, named, radiant), design: "stinkyscape", ...more });
      const largest = flagship().length;
      for (const design of others) {
        expect(starshipSvg({ ...starshipArtOf(biggest, true, named, radiant), design: design.id }).length, design.id).toBeLessThan(largest);
      }
      expect(shipSvg(biggest, false, named, radiant).length).toBeLessThan(largest);
      // In every livery, trail and skin.
      for (const livery of Object.values(LIVERIES)) {
        for (const trail of Object.values(TRAILS)) {
          for (const skin of Object.values(PIECE_SKINS)) {
            const svg = flagship({ livery, trail, skin });
            expect(svg.length, `${String(radiant)} ${JSON.stringify([livery, trail, skin])}`).toBeLessThanOrEqual(STARSHIP_ART_MAX_BYTES);
          }
        }
      }
    }
  });

  it("R3335000 no Starship cosmetic is in Radiant's gold, and a Radiant face gilds them", () => {
    // Every color a cosmetic names, and the tones a livery's plating mixes from its hull (R2725004).
    const liveries = [...Object.values(LIVERIES), ...Object.values(STARSHIP_DESIGNS).map((design) => design.livery)];
    for (const livery of liveries) {
      if (livery === null) continue;
      const paint = liveryPaint(livery);
      expect(golds([paint.light, paint.hull, paint.dark, paint.accent, paint.glow, paint.trim, paint.window].join(" ")), JSON.stringify(livery)).toEqual([]);
    }
    for (const trail of Object.values(TRAILS)) if (trail !== null) expect(golds(Object.values(trail).join(" ")), JSON.stringify(trail)).toEqual([]);
    for (const skin of Object.values(PIECE_SKINS)) if (skin !== null) expect(golds(Object.values(skin).join(" ")), JSON.stringify(skin)).toEqual([]);
    // Drawn on a base face, every design and every look wears none; on a Radiant face it is gilded.
    for (const design of Object.keys(STARSHIP_DESIGNS) as StarshipArt["design"][]) {
      expect(golds(starshipSvg({ ...starshipArtOf(list, true, named, false), design })), design).toEqual([]);
    }
    for (const look of everyLook()) {
      expect(golds(starshipSvg(starshipArtOf(list, true, named, false, look))), JSON.stringify(look)).toEqual([]);
      expect(golds(starshipSvg(starshipArtOf(list, true, named, true, look))).length, JSON.stringify(look)).toBeGreaterThan(0);
    }
  });
});

describe("R3333002: a Starship's pieces as their module icons", () => {
  it("R3333002 a Dormant hull's plates are its pieces' module icons at 1, 4 and 8; a launched one wears none", () => {
    const ids = [BEAM, DRILL, SHIELD, CAPSULE, AP_ROUNDS, "stinkyscape-090", "stinkyscape-140", "stinkyscape-134"];
    for (const n of [1, 4, 8]) {
      const list = ids.slice(0, n);
      for (const launched of [false, true]) {
        const ship = launched
          ? unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: list.map((defId, k) => (k === 0 ? { defId, radiant: true } : { defId })), name: "The Silent Choir", launched: 1 } })
          : dormantStarship("p1", list, { instanceId: "ship", starship: { pieces: list.map((defId, k) => (k === 0 ? { defId, radiant: true } : { defId })) } });
        const container = renderBoard(baseView({ you: emptySide("p1", { units: [null, null, ship, null, null] }) }));
        // In the order they fused (`data-plate`), port and starboard in turn.
        const plates = [...container.querySelectorAll<HTMLElement>('[data-testid="card-ship"] .cf-starship-plate')].sort(
          (a, b) => Number(a.getAttribute("data-plate")) - Number(b.getAttribute("data-plate")),
        );
        if (launched) {
          expect(plates, `launched ${String(n)}`).toHaveLength(0);
        } else {
          expect(plates.map((plate) => plate.getAttribute("data-module"))).toEqual(list.map((defId) => starshipModuleOf(defId).kind));
          plates.forEach((plate, k) => {
            expect(plate.style.backgroundImage, `${String(n)} ${String(k)}`).toContain(starshipModuleIconUri(list[k] ?? "", k === 0));
          });
        }
        cleanup();
      }
    }
    // Each piece's icon is its own, and a Radiant piece's is gilded.
    const icons = PIECE_IDS.map((id) => starshipModuleIconUri(id, false));
    expect(new Set(icons).size).toBe(PIECE_IDS.length);
    expect(starshipModuleIconUri(BEAM, true)).not.toBe(starshipModuleIconUri(BEAM, false));
  });

  it("R3333002 the pieces list shows each piece's icon beside its name at 1, 4 and 8, Dormant and launched", () => {
    const ids = [BEAM, DRILL, SHIELD, CAPSULE, AP_ROUNDS, "stinkyscape-090", "stinkyscape-140", "stinkyscape-134"];
    for (const n of [1, 4, 8]) {
      for (const launched of [false, true]) {
        const list = ids.slice(0, n).map((defId, k) => ({ defId, radiant: k === 1 }));
        const face = faceModel({
          defId: HULL,
          def: def(HULL),
          radiant: false,
          inPlay: { starship: { pieces: list.length, name: launched ? "The Ninth Life" : null, list } },
        });
        render(
          <CardDefsProvider defs={CATALOG}>
            <CardFace face={face} layout="full" />
          </CardDefsProvider>,
        );
        const shown = screen.getByTestId(STARSHIP_PIECES_TESTID);
        const rows = [...shown.querySelectorAll<HTMLElement>("[data-piece]")];
        expect(rows.map((row) => row.getAttribute("data-piece"))).toEqual(list.map((piece) => piece.defId));
        rows.forEach((row, k) => {
          const piece = list[k];
          const icon = row.querySelector<HTMLElement>(".cf-starship-icon");
          expect(icon?.getAttribute("data-module")).toBe(starshipModuleOf(piece?.defId ?? "").kind);
          expect(icon?.getAttribute("aria-hidden")).toBe("true");
          expect(icon?.style.backgroundImage).toContain(starshipModuleIconUri(piece?.defId ?? "", piece?.radiant ?? false));
          // Beside its name, which it comes before.
          expect(row.textContent).toBe(def(piece?.defId ?? "").name);
        });
        cleanup();
      }
    }
  });
});

describe("R3038000: a Starship's Flying and Ranged on the board", () => {
  it("R3038000 the real engine's view lists Flying and Ranged on a Dormant and a launched Starship, and the board draws both", () => {
    const port = realPort();
    const cheap = devDeck("cheap20");
    let state = port.beginGame(port.createGame({ seed: "r3038000-icons", decks: [cheap, cheap], sandbox: true } as Parameters<typeof port.createGame>[0])).state;
    let nonce = 0;
    const act = (player: PlayerId, body: ActionBody): void => {
      nonce += 1;
      const result = port.reduce(state, { ...body, playerId: player, nonce: `r3038000-${String(nonce)}` } as Action);
      if (result.error !== undefined) throw new Error(`${player} ${body.type}: ${result.error}`);
      state = result.state;
    };
    const toMain = (): void => {
      for (let step = 0; step < 40; step += 1) {
        const view = port.viewFor(state, "p1");
        if (view.active === "p1" && view.phase === "main" && view.pending === null) return;
        const who = (["p1", "p2"] as const).find((p) => port.legalActions(state, p).some((a) => a.type !== "concede" && a.type !== "offerDraw"));
        if (who === undefined) return;
        const legal = port.legalActions(state, who);
        const hand = port.viewFor(state, who).you.hand;
        if (legal.some((a) => a.type === "mulligan")) act(who, { type: "mulligan", keep: Array.isArray(hand) ? hand.map((c) => c.instanceId) : [] });
        else act(who, legal.find((a) => a.type === "answer") ?? { type: "endTurn" });
      }
    };
    const sandbox = (op: unknown): void => {
      act("p1", { type: "sandbox", op } as unknown as ActionBody);
    };
    toMain();
    sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
    sandbox({ kind: "spawn", player: "p1", defId: AP_ROUNDS, to: { zone: "field", row: "units", lane: 1 } });
    sandbox({ kind: "spawn", player: "p1", defId: NETHER, to: { zone: "hand" } });
    // Nether kills the piece, which fuses into the Dormant Starship the pass summons (R1852000).
    const hand = port.viewFor(state, "p1").you.hand;
    const nether = Array.isArray(hand) ? hand.find((c) => c.defId === NETHER) : undefined;
    const play = port.legalActions(state, "p1").find((a) => a.type === "play" && a.instanceId === nether?.instanceId);
    if (play === undefined) throw new Error("no play of Nether");
    act("p1", play);

    const dormant = shipOf(port.viewFor(state, "p1"));
    expect(dormant.slumber).toBeDefined();
    expect(dormant.keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
    expect(dormant.keywords.map((k) => k.kind)).not.toContain("Starship");
    expect(marksOn(port.viewFor(state, "p1"), dormant.instanceId)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
    // The other seat's view lists them too.
    const seen = unitsOf(port.viewFor(state, "p2").opponent).find((unit) => unit.instanceId === dormant.instanceId);
    expect(seen?.keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));

    // Launched by its Activate (R1852003), it keeps both and takes no Starship keyword (R3038001).
    const findLaunch = () => port.legalActions(state, "p1").find((a) => a.type === "activate" && a.ability === "launch");
    if (findLaunch() === undefined) {
      act("p1", { type: "endTurn" });
      toMain();
    }
    sandbox({ kind: "setMana", player: "p1", current: 10, max: 10 });
    const launch = findLaunch();
    if (launch === undefined) throw new Error("no launch to activate");
    act("p1", launch);
    toMain();
    const launched = shipOf(port.viewFor(state, "p1"));
    expect(launched.instanceId).toBe(dormant.instanceId);
    expect(launched.slumber).toBeUndefined();
    expect(launched.keywords.map((k) => k.kind)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
    expect(launched.keywords.map((k) => k.kind)).not.toContain("Starship");
    expect(marksOn(port.viewFor(state, "p1"), launched.instanceId)).toEqual(expect.arrayContaining(["Flying", "Ranged"]));
  });
});
