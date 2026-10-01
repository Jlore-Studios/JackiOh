// A Tribute can pay for its own zone (docs/classic-sets.md B4.5; SPEC §3.2, R391): with its row full,
// a Tribute card may be played into a zone its own Tribute empties — a pile of exactly one tributed
// unit, without Reborn, not Locked — and `legalActions` pairs each zone with the paying sets that
// leave it open, while the play's check reads the same pair (R90, R101, R210).

import type { CardDef } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { registerCatalog, registeredCatalog } from "../src/catalog";
import { chosenOptions, discoverFromCatalog, addToHand, summon } from "../src/effects";
import { freedByTribute, legalZonesFor, whyChoicesRefused } from "../src/playChoices";
import { legalActions } from "../src/reduce";
import { hashState } from "../src/replay";
import type { CardScripts } from "../src/script";
import { registerScripts, registeredScripts } from "../src/scripts";
import { newInstance, type CardInstance, type GameState } from "../src/state";
import { activeUnitsOf, isReserved, lockZone, placeOnField } from "../src/zones";
import { eventsOfType, inHand, put, slot } from "./fixtures/harness";
import {
  DISCOVER_POOL,
  graveTrap,
  only,
  pbAct,
  pbPlaying,
  pbReduce,
  playsOf,
  rebornBody,
  stackBody,
  titan,
  titanTwo,
  tributeField,
} from "./fixtures/playPipelineB";

function unitDef(name: string, index: number): CardDef {
  return {
    id: `pbt-${name}`,
    index: String(index),
    name: `${name} (tribute zones)`,
    set: "Core",
    type: "Unit",
    tags: [],
    rarity: "Common",
    token: false,
    cost: 1,
    base: { attack: 1, health: 1, keywords: [], text: name },
    radiant: { attack: 2, health: 2, keywords: [], text: name },
  };
}
/** Its Death summons a Rush Token: a card R210's held zone must keep out. */
const summoner = unitDef("summoner", 4611);
/** Its Death Discovers, so the play pauses at step 2 with its zone held. */
const asker = unitDef("asker", 4612);

const INLINE: Record<string, CardScripts> = {
  [summoner.id]: {
    base: { death: () => [summon({ defId: "fx-token-rush" })] },
    radiant: { death: () => [summon({ defId: "fx-token-rush" })] },
  },
  [asker.id]: {
    base: {
      death: () => [discoverFromCatalog({ step: "picked", query: { defId: DISCOVER_POOL } })],
      resume: {
        picked: (ctx) => {
          const defId = chosenOptions(ctx)[0];
          return defId === undefined ? [] : [addToHand({ defId })];
        },
      },
    },
    radiant: {},
  },
};

function playing(seed: string): GameState {
  const state = pbPlaying(seed);
  registerCatalog({ ...registeredCatalog(), [summoner.id]: summoner, [asker.id]: asker });
  registerScripts({ ...registeredScripts(), ...INLINE });
  return state;
}

/** Five units in p1's row, lane by lane (a different def per lane where asked). */
function fillRow(state: GameState, defs: Partial<Record<number, string>> = {}): CardInstance[] {
  return [1, 2, 3, 4, 5].map((lane) => put(state, defs[lane] ?? `fx-${lane}`, slot("p1", "units", lane)));
}

describe("B4.5 a Tribute can pay for its own zone (R391)", () => {
  it("R391 with a full row, each zone is offered with the paying sets that empty it, and the play lands there", () => {
    const state = playing("r391-full");
    const row = fillRow(state);
    const card = only(inHand(state, titan.id, "p1"));

    const plays = playsOf(state, card.id);
    // Tribute 1 on a full row: one play per lane, paying with that lane's unit.
    expect(plays.map((play) => [play.zone?.lane, play.tributes])).toEqual(
      row.map((unit, at) => [at + 1, [unit.id]]),
    );
    // The check reads the pair: lane 2 with lane 3's unit does not empty lane 2.
    expect(
      whyChoicesRefused(state, "p1", card, { type: "play", instanceId: card.id, zone: { row: "units", lane: 2 }, tributes: [row[2]!.id] }),
    ).toMatch(/not open/);

    const before = state.players.p2.hero.health;
    const result = pbReduce(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 2 }, tributes: [row[1]!.id], playerId: "p1" });
    expect(result.error).toBeUndefined();
    const after = result.state;
    expect(activeUnitsOf(after, "p1")[1]?.id).toBe(card.id);
    expect(after.players.p1.graveyard.map((unit) => unit.id)).toContain(row[1]!.id);
    // Its Cry fired; the zone step 2 held is released.
    expect(after.players.p2.hero.health).toBe(before - 1);
    expect(isReserved(after, slot("p1", "units", 2))).toBe(false);
  });

  it("R391 a play that names no zone takes the leftmost open one, or on a full row the leftmost its Tribute empties", () => {
    const state = playing("r391-default");
    const row = fillRow(state);
    const card = only(inHand(state, titan.id, "p1"));
    const after = pbAct(state, { type: "play", instanceId: card.id, tributes: [row[3]!.id], playerId: "p1" });
    expect(activeUnitsOf(after, "p1")[3]?.id).toBe(card.id);
  });

  it("R391 a pile of two frees nothing (the card beneath resumes, R13), a unit with Reborn frees nothing (R64), and a Locked zone stays shut", () => {
    const state = playing("r391-not-freed");
    const row = fillRow(state, { 2: rebornBody.id });
    // Lane 1: a Stack pile of two.
    const top = newInstance(state, stackBody.id, "p1", { z: "hand", player: "p1" });
    expect(placeOnField(state, top, slot("p1", "units", 1), { stack: true })).toBe(true);
    expect(state.players.p1.units[0]?.map((card) => card.id)).toEqual([top.id, row[0]!.id]);
    // Lane 3: Locked under its unit.
    lockZone(state, slot("p1", "units", 3));

    expect(freedByTribute(state, slot("p1", "units", 1), [top.id])).toBe(false);
    expect(freedByTribute(state, slot("p1", "units", 2), [row[1]!.id])).toBe(false);
    expect(freedByTribute(state, slot("p1", "units", 3), [row[2]!.id])).toBe(false);
    expect(freedByTribute(state, slot("p1", "units", 4), [row[3]!.id])).toBe(true);

    const card = only(inHand(state, titan.id, "p1"));
    expect(playsOf(state, card.id).map((play) => play.zone?.lane)).toEqual([4, 5]);
    for (const [lane, unit] of [[1, top], [2, row[1]!], [3, row[2]!]] as const) {
      expect(
        whyChoicesRefused(state, "p1", card, { type: "play", instanceId: card.id, zone: { row: "units", lane }, tributes: [unit.id] }),
      ).toMatch(/not open/);
    }
  });

  it("R391 a Tribute 2 pairs each zone with the sets that hold its unit, and an open zone with every set", () => {
    const state = playing("r391-two");
    const row = [1, 2, 3, 4].map((lane) => put(state, `fx-${lane}`, slot("p1", "units", lane)));
    const card = only(inHand(state, titanTwo.id, "p1"));
    const plays = playsOf(state, card.id);
    // Six sets of two out of four; lane 5 is open for all six, and each full lane for the three
    // sets that hold its unit.
    expect(plays.filter((play) => play.zone?.lane === 5)).toHaveLength(6);
    for (const [at, unit] of row.entries()) {
      const here = plays.filter((play) => play.zone?.lane === at + 1);
      expect(here).toHaveLength(3);
      for (const play of here) expect(play.tributes).toContain(unit.id);
    }
    expect(legalZonesFor(state, "p1", card, [row[0]!.id, row[1]!.id]).map((zone) => zone.lane)).toEqual([1, 2, 5]);
  });

  it("R391 R210 the emptied zone is held for the play: a tributed unit's Death summons elsewhere, or nowhere", () => {
    const state = playing("r391-held");
    const row = fillRow(state, { 3: summoner.id });
    const card = only(inHand(state, titan.id, "p1"));
    const result = pbReduce(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 3 }, tributes: [row[2]!.id], playerId: "p1" });
    expect(result.error).toBeUndefined();
    expect(activeUnitsOf(result.state, "p1")[2]?.id).toBe(card.id);
    // The Death's summon found no other open zone and fizzled (R64).
    expect(eventsOfType(result.events, "summoned").map((event) => event.defId)).not.toContain("fx-token-rush");
  });

  it("R391 a Death that asks at step 2 pauses the play with its emptied zone still held, across JSON, and the play lands there", () => {
    const state = playing("r391-pause");
    const row = fillRow(state, { 5: asker.id });
    const card = only(inHand(state, titan.id, "p1"));
    const paused = pbAct(state, { type: "play", instanceId: card.id, zone: { row: "units", lane: 5 }, tributes: [row[4]!.id], playerId: "p1" });
    expect(paused.pending?.kind).toBe("discover");
    expect(isReserved(paused, slot("p1", "units", 5))).toBe(true);
    const round = JSON.parse(JSON.stringify(paused)) as GameState;
    const answer = only(legalActions(paused, "p1").filter((action) => action.type === "answer").slice(0, 1));
    const live = pbAct(paused, { ...answer, playerId: "p1" });
    const again = pbAct(round, { ...answer, playerId: "p1" });
    expect(hashState(again)).toBe(hashState(live));
    expect(activeUnitsOf(live, "p1")[4]?.id).toBe(card.id);
    expect(isReserved(live, slot("p1", "units", 5))).toBe(false);
  });

  it("R391 a backrow card with a Tribute cost reads the rule the same way: a Tribute of units empties no backrow zone", () => {
    const state = playing("r391-backrow");
    put(state, "fx-1", slot("p1", "units", 1));
    for (const lane of [1, 2, 3, 4, 5]) put(state, graveTrap.id, slot("p1", "backrow", lane));
    const card = only(inHand(state, tributeField.id, "p1"));
    expect(playsOf(state, card.id)).toEqual([]);
    expect(pbReduce(state, { type: "play", instanceId: card.id, playerId: "p1", tributes: [activeUnitsOf(state, "p1")[0]!.id] }).error).toMatch(
      /no free backrow zone/,
    );
    // With a backrow zone open it is played as ever, paying its Tribute.
    state.players.p1.backrow[4] = null;
    expect(playsOf(state, card.id).map((play) => play.zone?.lane)).toEqual([5]);
  });
});
