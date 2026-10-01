// `shuffleRandomFromCatalog` (effects/shuffleRandom.ts): random catalog cards shuffled into a library,
// Radiant and enchanted (Classic+ #40 Appropriations' Education, E39, R60, R80, R387), through a
// test-only Book that shuffles three. The real card is proved in packages/cards
// (test/classic-plus/040-appropriations.test.ts).

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { LIBRARY_CAP } from "../src/config";
import { draw } from "../src/draw";
import { shuffleRandomFromCatalog } from "../src/effects";
import { hasEnchantment } from "../src/enchantments";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type GameState } from "../src/state";
import { settle } from "../src/triggers";
import { newGame, setLibrary, sinkFor } from "./fixtures/harness";
import { castNow } from "./fixtures/promptHarness";

function book(id: string, extra: Partial<CardDef> = {}): CardDef {
  return {
    id,
    index: id,
    name: id,
    set: "Core",
    type: "Spell",
    tags: ["Book"],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { keywords: [], text: id },
    radiant: { keywords: [], text: id },
    ...extra,
  };
}

/** The shuffling card is a Book itself, so R387 has something to keep out. */
const shuffler = book("sr-shuffler");
const pages = book("sr-pages");
const tokenBook = book("sr-token-book", { tags: ["Book", "Token"], token: true, rarity: "Token" });
const SCRIPTS: Record<string, CardScripts> = {
  [shuffler.id]: {
    base: {
      cry: () => [
        shuffleRandomFromCatalog({
          query: { tags: ["Book"] },
          count: 3,
          radiant: true,
          enchantments: [{ kind: "castOnDraw" }, { kind: "targetEnemies" }],
        }),
      ],
    },
    radiant: { cry: () => [] },
  },
};

function board(seed: string): GameState {
  const state = newGame(seed);
  registerCatalog({ ...registeredCatalog(), ...Object.fromEntries([shuffler, pages, tokenBook].map((d) => [d.id, d])) });
  registerScripts({ ...registeredScripts(), ...SCRIPTS });
  state.active = "p1";
  state.phase = "main";
  setLibrary(state, "p1", []);
  return state;
}

describe("shuffleRandomFromCatalog (E39)", () => {
  it("R60 R387 shuffles that many random pool cards in, never the running card's own nor a token, each Radiant and enchanted", () => {
    const state = board("sr-basic");
    castNow(state, shuffler.id);
    const library = state.players.p1.library;
    expect(library.map((card) => card.defId)).toEqual([pages.id, pages.id, pages.id]);
    for (const card of library) {
      expect(card.radiant).toBe(true);
      expect(hasEnchantment(card, "castOnDraw")).toBe(true);
      expect(hasEnchantment(card, "targetEnemies")).toBe(true);
      expect(card.knownAs).toBeDefined();
    }
  });

  it("E39 a card it made is cast as it is drawn", () => {
    const state = board("sr-draw");
    castNow(state, shuffler.id);
    const sink = sinkFor(state);
    draw(sink, "p1", 1);
    settle(sink);
    expect(sink.events.filter((event) => event.type === "cardPlayed")).toHaveLength(3);
    expect(state.players.p1.library).toHaveLength(0);
  });

  it("R80 a full library turns the rest away", () => {
    const state = board("sr-cap");
    state.players.p1.library = Array.from({ length: LIBRARY_CAP - 1 }, () => newInstance(state, pages.id, "p1", { z: "library", player: "p1" }));
    const sink = castNow(state, shuffler.id);
    expect(state.players.p1.library).toHaveLength(LIBRARY_CAP);
    expect(sink.events.filter((event) => event.type === "libraryOverflow")).toHaveLength(2);
  });
});
