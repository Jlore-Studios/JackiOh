// R3335000: a seat's frozen picks carry its four Starship picks; a `loadouts` frame frozen before #3335 names none of
// them, and each reads as its kind's default, while a frame missing one of the six older picks is still malformed.

import { describe, expect, it } from "vitest";

import { CARD_SLOT_KINDS, DECK_SLOT_KINDS, DEFAULT_CARD_COSMETICS, STARSHIP_SLOT_KINDS } from "./cosmetics.ts";
import { createLoadoutsHolder, parseCardLoadouts, parseSeatPicks } from "./loadouts.ts";

/** A seat's six older picks, as a frame from before #3335 sends them. */
const OLD_SEAT = {
  table: "table-felt",
  field: "field-rain",
  cardBack: "card-back-starfield",
  foil: DEFAULT_CARD_COSMETICS.foil,
  finish: DEFAULT_CARD_COSMETICS.finish,
  border: DEFAULT_CARD_COSMETICS.border,
};

const FLEET_LOOK = {
  starshipHull: "starship-hull-corsair",
  starshipLivery: "starship-livery-arctic",
  starshipTrail: "starship-trail-ion",
  starshipPieceSkin: "starship-piece-skin-verdigris",
};

describe("R3335000 a seat's Starship picks in its frozen loadout", () => {
  it("R3335000 a frame without Starship picks reads their defaults", () => {
    const picks = parseSeatPicks({ p1: OLD_SEAT, p2: { ...OLD_SEAT, ...FLEET_LOOK } });
    expect(picks?.p1).toEqual({ ...DEFAULT_CARD_COSMETICS, ...OLD_SEAT });
    for (const kind of STARSHIP_SLOT_KINDS) expect(picks?.p1[kind], kind).toBe(DEFAULT_CARD_COSMETICS[kind]);
    // A frame that names them is read as it names them, an unknown one or one of another kind its default.
    expect(picks?.p2).toEqual({ ...DEFAULT_CARD_COSMETICS, ...OLD_SEAT, ...FLEET_LOOK });
    const odd = parseSeatPicks({ p1: { ...OLD_SEAT, starshipHull: "starship-hull-gone", starshipLivery: "table-felt" }, p2: OLD_SEAT });
    expect(odd?.p1.starshipHull).toBe(DEFAULT_CARD_COSMETICS.starshipHull);
    expect(odd?.p1.starshipLivery).toBe(DEFAULT_CARD_COSMETICS.starshipLivery);
    // A Starship pick that is there but no text is malformed, as is a frame missing any of the six older picks.
    expect(parseSeatPicks({ p1: { ...OLD_SEAT, starshipTrail: 3 }, p2: OLD_SEAT })).toBeNull();
    for (const kind of [...CARD_SLOT_KINDS, ...DECK_SLOT_KINDS]) {
      const short = Object.fromEntries(Object.entries(OLD_SEAT).filter(([key]) => key !== kind));
      expect(parseSeatPicks({ p1: short, p2: OLD_SEAT }), kind).toBeNull();
    }
    // A replay's page from before #3335 reads the same.
    const page = createLoadoutsHolder().page({ loadouts: { p1: OLD_SEAT, p2: OLD_SEAT } });
    expect(page.p2.seat).toEqual({ ...DEFAULT_CARD_COSMETICS, ...OLD_SEAT });
  });
});

describe("#4015 prototype pollution", () => {
  it("parseCardLoadouts drops proto cardIds", () => {
    const parsed = parseCardLoadouts({
      seat: "p1",
      cards: [{ cardId: "__proto__", foil: "x" }, { cardId: "constructor", foil: "x" }, { cardId: "real", foil: "x" }],
      stamps: [{ cardId: "__proto__", stamp: 1 }, { cardId: "constructor", stamp: 1 }, { cardId: "real", stamp: 1 }],
    });
    // An unknown foil id reads as its kind's default; the proto ids are dropped.
    expect(parsed?.cards).toEqual({ real: { foil: DEFAULT_CARD_COSMETICS.foil } });
    // Without the guard the `__proto__` entry would sit on the result's prototype.
    expect("border" in (parsed?.cards ?? {})).toBe(false);
    expect(parsed?.stamps).toEqual({ real: 1 });
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });

  it("holder merge keeps proto out", () => {
    const holder = createLoadoutsHolder();
    holder.seats({ p1: { ...DEFAULT_CARD_COSMETICS }, p2: { ...DEFAULT_CARD_COSMETICS } });
    const update = parseCardLoadouts({ seat: "p1", cards: [{ cardId: "__proto__" }, { cardId: "constructor" }, { cardId: "real" }] });
    expect(update).not.toBeNull();
    if (update !== null) holder.cards(update);
    const current = holder.current();
    expect(current?.p1.cards).toEqual({ real: {} });
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
    const page = holder.page({
      loadouts: { p1: OLD_SEAT, p2: OLD_SEAT },
      cardLoadouts: [{ seat: "p2", cards: [{ cardId: "__proto__" }, { cardId: "real" }] }],
    });
    expect(page.p2.cards).toEqual({ real: {} });
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });
});
