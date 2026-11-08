// R2249000: a Starship's face in play (#2249) wears the Starship's own art, drawn from its pieces and its
// launch, whatever definition its fusions gave it; its rules box prints only the Starship term and the
// pieces fused into it, each a reference that opens the piece's card. R2801000: a Dormant one bolts on
// a hull plate for each piece fused into it. R3038000: Starship is a tag, and the real engine's view
// lists a Starship's Flying and Ranged, Dormant or launched, which the board draws as any Unit's.
// R3333000: its picture is built from its pieces oldest first, each its module on its hardpoint, in the
// hull design of the name it launched under, a Dormant one in a shipyard. R3333002: its hull plates and
// its pieces list show each piece's module icon. R3334000: a Dormant one with pieces idles in its shipyard,
// in one of the moving-art slots.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { STARSHIP_NAMES } from "@jackioh/engine/config";
import type { Action, ActionBody, CardDef, PlayerId, PlayerView, UnitView } from "@jackioh/shared";

import { devDeck, realPort } from "../audio/test/realGame.ts";
import { CardArt } from "./art/CardArt.tsx";
import {
  LAUNCHED_DESIGNS,
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
import { fmt } from "./art/hash.ts";
import { moduleMarkup, modulePaint, STARSHIP_MODULE_IDS, starshipModuleOf } from "./art/starshipModules.ts";
import { shipyardIdleOf, STARSHIP_ART_MAX_BYTES, starshipSvg } from "./art/stinkyscape.ts";
import { starshipArtUri, starshipModuleIconUri } from "./art/svg.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests } from "./cosmetics.ts";
import { faceModel } from "./model.ts";
import { CardDefsProvider } from "./refContext.tsx";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { STARSHIP_PIECES_TESTID } from "./StarshipPieces.tsx";
import { STARSHIP_HULL_PLATES_MAX } from "./StarshipHull.tsx";
import { renderBoard } from "../test/board.tsx";
import {
  COSMETIC_FOIL_ANIMATED_MAX,
  RADIANT_GOLD_HUE_MAX_DEG,
  RADIANT_GOLD_HUE_MIN_DEG,
  RADIANT_GOLD_LIGHTNESS_MAX,
  RADIANT_GOLD_LIGHTNESS_MIN,
  RADIANT_GOLD_SATURATION_MIN,
  SHIPYARD_IDLE_SPARKS,
  SHIPYARD_IDLE_SWEEP_MS,
  SHIPYARD_IDLE_WELD_MS,
} from "./constants.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, dormantStarship, emptySide, unit } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";

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

describe("R3334000: a Dormant Starship's shipyard idles", () => {
  beforeEach(() => {
    resetCosmeticsForTests();
  });

  afterEach(() => {
    cleanup();
    resetCosmeticsForTests();
    localStorage.clear();
    writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    __resetSettingsForTests();
    setReducedMotion(false);
  });

  /** A side's unit zones, as the fixtures lay them out. */
  const LANES = emptySide("p1").units.length;

  /** The board's shipyard idles, with p1's units and p2's. */
  function idlesOn(mine: readonly (UnitView | null)[], theirs: readonly (UnitView | null)[] = []): SVGSVGElement[] {
    const lane = (units: readonly (UnitView | null)[]): (UnitView | null)[] => Array.from({ length: LANES }, (_, k) => units[k] ?? null);
    const container = renderBoard(baseView({ you: emptySide("p1", { units: lane(mine) }), opponent: emptySide("p2", { units: lane(theirs) }) }));
    return [...container.querySelectorAll<SVGSVGElement>(".cf-shipyard-idle")];
  }

  it("R3334000 a Dormant ship with pieces idles; not with 0 pieces, launched or reduced", () => {
    const list = [BEAM, DRILL, "stinkyscape-090"];
    const docked = (): UnitView => dormantStarship("p1", list, { instanceId: "ship" });
    const [idle, ...more] = idlesOn([null, null, docked()]);
    expect(more).toHaveLength(0);
    if (idle === undefined) throw new Error("the Dormant Starship does not idle");
    // Over its picture, in its portrait, framed as the picture is.
    expect(idle.closest('[data-testid="card-ship"] .cf-portrait')).not.toBeNull();
    expect(idle.previousElementSibling?.classList.contains("cf-art")).toBe(true);
    expect(idle.getAttribute("data-shipyard-motion")).toBe("animated");
    expect(idle.getAttribute("viewBox")).toBe("0 0 100 100");
    expect(idle.getAttribute("preserveAspectRatio")).toBe("xMidYMid slice");
    expect(idle.getAttribute("aria-hidden")).toBe("true");
    expect(idle.style.getPropertyValue("--cf-sweep-ms")).toBe(`${String(SHIPYARD_IDLE_SWEEP_MS)}ms`);
    expect(idle.style.getPropertyValue("--cf-weld-ms")).toBe(`${String(SHIPYARD_IDLE_WELD_MS)}ms`);
    // The light's cone swings about the lamp, and the sparks fly where the newest piece is welded in.
    const yard = shipyardIdleOf(starshipArtOf(pieces(list), false, undefined, false));
    const sweep = idle.querySelector<SVGPolygonElement>("polygon.cf-shipyard-sweep");
    expect(sweep?.getAttribute("points")).toBe(yard.cone.map((point) => `${fmt(point.x)},${fmt(point.y)}`).join(" "));
    expect(sweep?.style.transformOrigin).toBe(`${fmt(yard.lamp.x)}px ${fmt(yard.lamp.y)}px`);
    const sparks = idle.querySelector<SVGGElement>("g.cf-shipyard-sparks");
    expect(yard.newest).not.toBeNull();
    expect(sparks?.getAttribute("transform")).toBe(`translate(${fmt(yard.newest?.x ?? NaN)} ${fmt(yard.newest?.y ?? NaN)})`);
    const thrown = [...(sparks?.querySelectorAll<SVGCircleElement>("circle") ?? [])];
    expect(thrown.map((spark) => spark.style.getPropertyValue("--cf-spark"))).toEqual(Array.from({ length: SHIPYARD_IDLE_SPARKS }, (_, k) => String(k)));
    cleanup();
    // No pieces yet, or launched: no idle.
    expect(idlesOn([dormantStarship("p1", [], { instanceId: "ship" })])).toHaveLength(0);
    cleanup();
    expect(idlesOn([unit("p1", { instanceId: "ship", defId: HULL, starship: { pieces: list.map((defId) => ({ defId })), name: "The Ninth Life", launched: 1 } })])).toHaveLength(0);
    cleanup();
    // The Reduce motion setting, the system's reduced motion and Animated foil off each hold it still: none drawn.
    const stills: [string, () => void][] = [
      ["the Reduce motion setting", () => writeSettings({ reduceMotion: true })],
      ["the system's reduced motion", () => setReducedMotion(true)],
      ["Animated foil off", () => writeCardSettings({ animatedFoil: false })],
    ];
    for (const [why, still] of stills) {
      still();
      expect(idlesOn([docked()]), why).toHaveLength(0);
      cleanup();
      localStorage.clear();
      __resetSettingsForTests();
      setReducedMotion(false);
      writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
    }
    // Back on, it idles again.
    expect(idlesOn([docked()])).toHaveLength(1);
  });

  it("R3334000 the idle shares COSMETIC_FOIL_ANIMATED_MAX", () => {
    const ships = (owner: "p1" | "p2"): UnitView[] =>
      Array.from({ length: LANES }, (_, k) => dormantStarship(owner, [BEAM, SHIELD], { instanceId: `${owner}-ship-${String(k)}` }));
    // A Dormant Starship in every unit zone of both seats, more than the cap: only the cap's worth idle.
    const mine = ships("p1");
    const theirs = ships("p2");
    expect(mine.length + theirs.length).toBeGreaterThan(COSMETIC_FOIL_ANIMATED_MAX);
    expect(idlesOn(mine, theirs)).toHaveLength(COSMETIC_FOIL_ANIMATED_MAX);
    cleanup();
    // Their slots freed, one ship idles again.
    expect(idlesOn([dormantStarship("p1", [BEAM], { instanceId: "ship" })])).toHaveLength(1);
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
