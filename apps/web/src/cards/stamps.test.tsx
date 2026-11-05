// R3013002: a card's proficiency stamp on its faces (stamps.ts, ProficiencyStamp.tsx): the viewer's own copy wears
// the account's pick, another seat's copy the stamp frozen with its loadout once the server sent it, and a token or a
// Created card none. Where the stamp sits, clear of every part, only a browser can measure
// (e2e/cypress/component/proficiency-stamp.cy.tsx); its colours are held out of Radiant's gold by cosmetics.test.tsx.

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { PROFICIENCY_LEVELS } from "@jackioh/server-config";

import { unit } from "../test/fixtures.ts";
import { CardFace } from "./CardFace.tsx";
import { DEFAULT_CARD_COSMETICS, resetCosmeticsForTests } from "./cosmetics.ts";
import { TheirCopy, createLoadoutsHolder, parseCardLoadouts, type SeatLoadout } from "./loadouts.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel, type FaceModel } from "./model.ts";
import { ProficiencyStamp } from "./ProficiencyStamp.tsx";
import { STAMP_LEVELS, isStampLevel, replaceOwnProficiency, resetProficiencyForTests } from "./stamps.ts";

/** A Unit, and a card whose Radiant face prints R277's gold marks (CardFace.test.tsx's). */
const UNIT = "core-004";
const MARKED = "core-038";
const TOKEN = Object.values(CATALOG).find((def) => def.token)?.id ?? "";

function face(defId: string, radiant = false): FaceModel {
  return faceModel({ defId, def: CATALOG[defId], radiant });
}

function stampOf(container: HTMLElement): string | null {
  return container.querySelector(".cf .cf-stamp")?.getAttribute("data-stamp") ?? null;
}

/** Another seat's loadout wearing `stamps`, every kind its default. */
function theirs(stamps: Record<string, number>): SeatLoadout {
  return { seat: { ...DEFAULT_CARD_COSMETICS }, cards: {}, stamps };
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  resetProficiencyForTests();
});

afterEach(() => {
  cleanup();
  resetProficiencyForTests();
});

describe("R3013002 a card's stamp on its faces", () => {
  it("R3013002 a worn face wears its own stamp on the full face, a hand-size face and a board unit, base and Radiant", () => {
    replaceOwnProficiency([
      { cardId: UNIT, plays: 40, level: 3, stamp: 3 },
      { cardId: MARKED, plays: 200, level: PROFICIENCY_LEVELS, stamp: PROFICIENCY_LEVELS },
    ]);
    for (const radiant of [false, true]) {
      const full = render(<CardFace face={face(UNIT, radiant)} layout="full" worn />).container;
      expect(stampOf(full), `full ${String(radiant)}`).toBe("3");
      expect(full.querySelector(".cf")?.getAttribute("data-radiant-face")).toBe(radiant ? "true" : null);
      cleanup();
      // A hand card on a board the viewer plays (game/Card.tsx sets `ownHand`).
      const hand = render(<CardFace face={face(UNIT, radiant)} layout="full" worn ownHand />).container;
      expect(stampOf(hand), `hand ${String(radiant)}`).toBe("3");
      cleanup();
      const board = render(<MinionFace face={face(UNIT, radiant)} unit={unit("p1", { defId: UNIT, radiant })} worn />).container;
      expect(stampOf(board), `board ${String(radiant)}`).toBe("3");
      expect(board.querySelector(".cf")?.getAttribute("data-radiant-face")).toBe(radiant ? "true" : null);
      cleanup();
    }
    // A Radiant face keeps every Radiant mark beside its stamp (R2725004).
    const marked = render(<CardFace face={face(MARKED, true)} layout="full" worn />).container;
    expect(stampOf(marked)).toBe(String(PROFICIENCY_LEVELS));
    expect(marked.querySelector(".cf")?.getAttribute("data-radiant-face")).toBe("true");
    expect(marked.querySelectorAll(".cf-mark").length).toBeGreaterThan(0);
    cleanup();
    // The same copy, not the viewer's, wears none of the account's: nothing above it says whose it is.
    expect(stampOf(render(<CardFace face={face(UNIT)} layout="full" />).container)).toBeNull();
    cleanup();
    // A card the account never played, or wears no stamp on, wears none.
    replaceOwnProficiency([{ cardId: UNIT, plays: 40, level: 3, stamp: null }]);
    expect(stampOf(render(<CardFace face={face(UNIT)} layout="full" worn />).container)).toBeNull();
    cleanup();
    expect(stampOf(render(<CardFace face={face(MARKED)} layout="full" worn />).container)).toBeNull();
  });

  it("R3013002 a token and a Created card wear no stamp", () => {
    expect(TOKEN).not.toBe("");
    replaceOwnProficiency([
      { cardId: UNIT, plays: 3, level: 1, stamp: 1 },
      { cardId: TOKEN, plays: 3, level: 1, stamp: 1 },
    ]);
    const created = faceModel({ defId: UNIT, def: CATALOG[UNIT], radiant: false, inPlay: { created: true } });
    expect(stampOf(render(<CardFace face={created} layout="full" worn />).container)).toBeNull();
    cleanup();
    expect(stampOf(render(<CardFace face={face(TOKEN)} layout="full" worn />).container)).toBeNull();
    cleanup();
    const loadout = theirs({ [UNIT]: 2, [TOKEN]: 2 });
    expect(
      stampOf(
        render(
          <TheirCopy loadout={loadout}>
            <CardFace face={created} layout="full" />
          </TheirCopy>,
        ).container,
      ),
    ).toBeNull();
  });

  it("R3013002 another seat's face wears its loadout's stamp, and none without one", () => {
    // The viewer's own stamp on the card is not what another seat's copy wears.
    replaceOwnProficiency([{ cardId: UNIT, plays: 200, level: PROFICIENCY_LEVELS, stamp: 1 }]);
    const loadout = theirs({ [UNIT]: 4 });
    let { container } = render(
      <TheirCopy loadout={loadout}>
        <CardFace face={face(UNIT)} layout="full" />
        <MinionFace face={face(UNIT, true)} unit={unit("p2", { defId: UNIT, radiant: true })} />
      </TheirCopy>,
    );
    expect([...container.querySelectorAll(".cf .cf-stamp")].map((stamp) => stamp.getAttribute("data-stamp"))).toEqual(["4", "4"]);
    cleanup();
    // A card the loadout sent no stamp for (unshown, or wearing none) wears none.
    ({ container } = render(
      <TheirCopy loadout={loadout}>
        <CardFace face={face(MARKED)} layout="full" />
      </TheirCopy>,
    ));
    expect(stampOf(container)).toBeNull();
    cleanup();
    // A loadout from before #3013 has no stamps; R1834004's switch, off, leaves no loadout above the face.
    for (const none of [{ seat: { ...DEFAULT_CARD_COSMETICS }, cards: {} }, null]) {
      ({ container } = render(
        <TheirCopy loadout={none}>
          <CardFace face={face(UNIT)} layout="full" />
        </TheirCopy>,
      ));
      expect(stampOf(container)).toBeNull();
      cleanup();
    }
    // The viewer's own copy, below it, wears its own pick.
    ({ container } = render(
      <TheirCopy loadout={loadout}>
        <CardFace face={face(UNIT)} layout="full" worn />
      </TheirCopy>,
    ));
    expect(stampOf(container)).toBe("1");
  });

  it("R3013002 each level draws its own, more ornate design", () => {
    expect(STAMP_LEVELS).toEqual(Array.from({ length: PROFICIENCY_LEVELS }, (_, at) => at + 1));
    const drawn = STAMP_LEVELS.map((level) => {
      const { container } = render(<ProficiencyStamp level={level} />);
      const stamp = container.querySelector(".cf-stamp");
      expect(stamp?.getAttribute("data-stamp"), String(level)).toBe(String(level));
      expect(stamp?.getAttribute("role")).toBe("img");
      expect(stamp?.getAttribute("aria-label")).toBe(`Level ${String(level)} stamp`);
      expect(stamp?.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
      const parts = stamp?.querySelectorAll("svg > *").length ?? 0;
      const markup = stamp?.innerHTML ?? "";
      cleanup();
      return { parts, markup };
    });
    for (let at = 1; at < drawn.length; at += 1) {
      expect(drawn[at]?.parts, `level ${String(at + 1)} adds a part`).toBeGreaterThan(drawn[at - 1]?.parts ?? Infinity);
    }
    expect(new Set(drawn.map((each) => each.markup)).size).toBe(drawn.length);
    // No stamp for a level none has.
    for (const level of [0, -1, PROFICIENCY_LEVELS + 1, 1.5, Number.NaN]) {
      expect(isStampLevel(level), String(level)).toBe(false);
      expect(render(<ProficiencyStamp level={level} />).container.innerHTML, String(level)).toBe("");
      cleanup();
    }
  });

  it("R3013002 cardLoadouts frames and replay pages carry stamps", () => {
    const frame = {
      type: "cardLoadouts",
      seat: "p2",
      cards: [{ cardId: "core-001", foil: "foil-rainbow" }],
      stamps: [
        { cardId: "core-001", stamp: 2 },
        { cardId: "core-002", stamp: PROFICIENCY_LEVELS + 1 },
        { cardId: "core-003", stamp: "1" },
        { stamp: 1 },
      ],
    };
    expect(parseCardLoadouts(frame)?.stamps).toEqual({ "core-001": 2 });
    // A frame without stamps keeps its shape.
    expect(parseCardLoadouts({ seat: "p1", cards: [] })).toEqual({ seat: "p1", cards: {} });

    const held = createLoadoutsHolder();
    const seats = { p1: { ...DEFAULT_CARD_COSMETICS }, p2: { ...DEFAULT_CARD_COSMETICS } };
    expect(held.seats(seats).p2.stamps).toBeUndefined();
    const parsed = parseCardLoadouts(frame);
    if (parsed === null) throw new Error("the frame parses");
    expect(held.cards(parsed)?.p2.stamps).toEqual({ "core-001": 2 });
    // A later frame adds its cards, and one sent again replaces its stamp; the other seat has none.
    const later = parseCardLoadouts({ seat: "p2", cards: [], stamps: [{ cardId: "core-001", stamp: 3 }, { cardId: "core-009", stamp: 1 }] });
    if (later === null) throw new Error("the frame parses");
    expect(held.cards(later)?.p2.stamps).toEqual({ "core-001": 3, "core-009": 1 });
    expect(held.current()?.p1.stamps).toBeUndefined();

    // A replay page's cardLoadouts carry them the same way.
    const pages = createLoadoutsHolder();
    const page = pages.page({
      loadouts: { p1: seats.p1, p2: seats.p2 },
      cardLoadouts: [
        { seat: "p1", cards: [], stamps: [{ cardId: "core-004", stamp: 5 }] },
        { seat: "p2", cards: [{ cardId: "core-001", border: "border-wood" }] },
      ],
    });
    expect(page.p1.stamps).toEqual({ "core-004": 5 });
    expect(page.p2.stamps).toBeUndefined();
  });
});
