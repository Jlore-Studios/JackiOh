import { describe, expect, it } from "vitest";

import type { AssaultAction, AssaultFile, AssaultRecord, AssaultSetup } from "@jackioh/run";
import { DECK_SIZE } from "../wire/engineConfig.ts";

import {
  assaultCatalogContent,
  assaultConstants,
  assaultContent,
  assaultContentAt,
  assaultCreate,
  assaultDay,
  assaultDayDeck,
  assaultDaySeed,
  assaultFightReport,
  assaultFightSetup,
  assaultFold,
  assaultPlaceholderContent,
  assaultRecord,
  assaultReduce,
  registeredCatalog,
  type ReplayInput,
} from "./index.ts";

function fnv1a(text: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < text.length; i++) {
    hash ^= text.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

describe("assault WebAssembly exports", () => {
  it("every assault wrapper crosses the module as JSON and returns refusals", () => {
    const content = assaultContent();
    expect(content).toBeDefined();

    const catalog = registeredCatalog();
    const catalogContent = assaultCatalogContent(catalog);
    expect(catalogContent).toBeDefined();
    const placeholderContent = assaultPlaceholderContent(catalog);
    expect(placeholderContent).toBeDefined();

    const constants = assaultConstants();
    expect(constants.levels).toBe(20);
    expect(constants.commanderLevels).toEqual([5, 10, 15, 20]);
    expect(constants.checkpoints).toEqual([6, 11, 16]);

    expect(assaultDaySeed("2026-11-23")).toBe("assault-2026-11-23");
    expect(() => assaultDaySeed("invalid-date")).toThrow();

    const day = assaultDay("2026-11-23", content);
    expect(day.date).toBe("2026-11-23");
    expect(day.levels).toHaveLength(20);
    expect(() => assaultDay("invalid-date", content)).toThrow();

    const deck = assaultDayDeck("2026-11-23");
    expect(deck).toHaveLength(DECK_SIZE);

    const setup: AssaultSetup = {
      date: "2026-11-23",
      deck,
      source: "dayDeck",
      startLevel: 1,
      attempt: 1,
    };
    const record: AssaultRecord = {
      date: "2026-11-23",
      best: 0,
      checkpoints: [],
      // R4579002: the day's record at the attempt's start, before it counts.
      attempts: 0,
      repelledOn: null,
    };

    const created = assaultCreate(setup, record, day);
    if ("error" in created) throw new Error(created.error);
    expect(created.state.level).toBe(1);

    const refusedSetup: AssaultSetup = {
      ...setup,
      startLevel: 6,
    };
    const refusedCreate = assaultCreate(refusedSetup, record, day);
    if (!("error" in refusedCreate)) throw new Error("checkpoint level 6 not reached is refused");
    expect(refusedCreate.error.length).toBeGreaterThan(0);

    const fightAction: AssaultAction = { kind: "fight" };
    const moved = assaultReduce(created.state, day, fightAction);
    if ("error" in moved) throw new Error(moved.error);
    expect(moved.state.phase).toBe("fight");

    const fight = assaultFightSetup(moved.state, day);
    if (fight === null) throw new Error("a fight phase has a fight setup");
    expect(fight.humanDeck).toEqual(deck);
    expect(fight.seed).toBe(`${day.seed}-L1-A1`);

    const game: ReplayInput = {
      seed: fight.seed,
      decks: [fight.humanDeck, fight.enemyDeck],
      handicaps: { p1: fight.humanHandicap, p2: fight.enemyHandicap },
      deckMarks: { p1: [] },
      log: [],
    };
    const report = assaultFightReport(game, "p1");
    if (!("error" in report)) throw new Error("an unfinished fight reports nothing");
    expect(report.error.length).toBeGreaterThan(0);

    const file: AssaultFile = {
      version: "v0.0.0",
      setup,
      record,
      actions: [fightAction],
    };
    const folded = assaultFold(file, day);
    if ("error" in folded) throw new Error(folded.error);
    expect(folded.state).toEqual(moved.state);

    const updatedRecord = assaultRecord(record, moved.state);
    expect(updatedRecord).toBeDefined();

    const endedState = { ...created.state, phase: "lost" as const };
    const refusedAction = assaultReduce(endedState, day, fightAction);
    if (!("error" in refusedAction)) throw new Error("action on ended attempt is refused");
    expect(refusedAction.error.length).toBeGreaterThan(0);

    expect(assaultContentAt("unknown-version")).toBeNull();
  });

  it("assaultDay is byte-equal twice, hash-pinned, and differs across dates", () => {
    const content = assaultContent();
    const day1 = JSON.stringify(assaultDay("2026-11-23", content));
    const day2 = JSON.stringify(assaultDay("2026-11-23", content));
    expect(day1).toBe(day2);

    const otherDay = JSON.stringify(assaultDay("2026-11-24", content));
    expect(day1).not.toBe(otherDay);

    const hash = fnv1a(day1);
    // The deal of 2026-11-23 on this build's stand-ins, as the module answered it; a change to the
    // catalog's deckable cards or to the deal moves it.
    expect(hash).toBe("88d6c333");
  });

  it("the day deck has DECK_SIZE distinct ids, is equal twice, and every id is in the catalog", () => {
    const deck1 = assaultDayDeck("2026-11-23");
    const deck2 = assaultDayDeck("2026-11-23");
    expect(deck1).toEqual(deck2);
    expect(deck1).toHaveLength(DECK_SIZE);
    expect(new Set(deck1).size).toBe(DECK_SIZE);

    const catalog = registeredCatalog();
    for (const id of deck1) {
      expect(catalog[id]).toBeDefined();
    }
  });
});
