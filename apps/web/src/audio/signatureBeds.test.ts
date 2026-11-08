// R3389000: each signature's sound bed (signatureBeds.ts): a short sequence of the existing `sfx.ts`
// recipes, its card's own, laid across the entry its card makes its entrance in and sent as that entry
// starts (director.ts), beside the card's intro sting and its events' own sounds.

import { CATALOG } from "@jackioh/cards";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { entranceEntries, newEventsSince, planEntries, type AnimationEntry, type EntranceResolver } from "../game/animations.ts";
import { lookupFromDefs } from "../game/catalog.ts";
import { createPlayTracker } from "../game/runs.ts";
import { FX_CARD_TABLE, isSignature } from "../fx/cards/tables.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { createSoundDirector } from "./director.ts";
import { SFX_IDS } from "./sfx.ts";
import { SIGNATURE_BED_MAX_STEPS, SIGNATURE_BEDS, signatureBed } from "./signatureBeds.ts";
import type { SfxId, SoundSink } from "./types.ts";
import { cueCard } from "./useGameAudio.ts";

const lookup = lookupFromDefs(CATALOG);
const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];

/** Every Classic signature, in catalog order. */
const CLASSIC_SIGNATURES = [...FX_CARD_TABLE].flatMap(([defId, entry]) => (defId.startsWith("classic-") && isSignature(entry) ? [defId] : []));

/** As the board resolves an entrance: the card's type and rarity, a token's the one it prints (B2.5). */
const resolve: EntranceResolver = (defId) => {
  const def = CATALOG[defId];
  return def === undefined ? null : { type: def.type, rarity: def.printedRarity ?? def.rarity };
};

function recorder(): SoundSink & { sent: { id: SfxId; delayMs: number }[] } {
  const sent: { id: SfxId; delayMs: number }[] = [];
  return {
    sent,
    playSfx(id, _params, delayMs) {
      sent.push({ id, delayMs: delayMs ?? 0 });
      return true;
    },
    playVoice() {
      return true;
    },
    playEffect() {
      return true;
    },
  };
}

describe("R3389000 a signature's sound bed", () => {
  it("R3389000 every Classic signature has its own bed: known ids, at in [0, 1], at most SIGNATURE_BED_MAX_STEPS, no two equal", () => {
    expect(CLASSIC_SIGNATURES).toHaveLength(11);
    expect(Object.keys(SIGNATURE_BEDS)).toEqual(CLASSIC_SIGNATURES);
    const beds = new Set<string>();
    for (const defId of CLASSIC_SIGNATURES) {
      const bed = SIGNATURE_BEDS[defId] ?? [];
      expect(bed.length, defId).toBeGreaterThan(0);
      expect(bed.length, defId).toBeLessThanOrEqual(SIGNATURE_BED_MAX_STEPS);
      for (const step of bed) {
        expect(SFX_IDS, defId).toContain(step.id);
        expect(step.at, defId).toBeGreaterThanOrEqual(0);
        expect(step.at, defId).toBeLessThanOrEqual(1);
      }
      // Its steps in order, and its own: no other card's bed is the same.
      expect(bed.map((step) => step.at), defId).toEqual([...bed.map((step) => step.at)].sort((a, b) => a - b));
      beds.add(JSON.stringify(bed));
    }
    expect(beds.size).toBe(CLASSIC_SIGNATURES.length);
    // The bed keys on the entry's readable entrance: none with no entrance, nor for a card with no bed.
    expect(signatureBed({ durationMs: 400 })).toEqual([]);
    expect(signatureBed({ durationMs: 400, entrance: { tier: "legendary", defId: "core-052", instanceId: "c1", player: "p1", anchor: null, reveal: false } })).toEqual([]);
    const bed = signatureBed({ durationMs: 1_000, entrance: { tier: "legendary", defId: "classic-080", instanceId: "c1", player: "p1", anchor: null, reveal: false } });
    expect(bed).toEqual((SIGNATURE_BEDS["classic-080"] ?? []).map((step) => ({ kind: "sfx", id: step.id, delayMs: Math.round(step.at * 1_000) })));
    expect(JSON.stringify(bed)).not.toContain("classic-");
  });

  it("R3389000 the bed plays as the entrance's entry starts", () => {
    for (const defId of ["classic-080", "classic-004"]) {
      const view = fullBoardView();
      const field = CATALOG[defId]?.type === "Field Spell";
      const events: GameEvent[] = [
        { type: "cardPlayed", player: "p1", instanceId: "sig", defId, costPaid: 4 },
        { type: "summoned", player: "p1", instanceId: "sig", defId, row: field ? "backrow" : "units", lane: 3 },
        { type: "cardResolved", player: "p1", instanceId: "sig", defId, permanent: true, costPaid: 4 },
      ];
      const before = withEvents(view, START);
      const after = withEvents(view, [...START, ...events]);
      const sink = recorder();
      const director = createSoundDirector(sink, undefined, (id) => cueCard(lookup, id), undefined, () => 0);
      director.onView(before);
      director.onView(after);
      const entries = entranceEntries(planEntries(newEventsSince(before.events, after.events), before, false, createPlayTracker()), resolve, { seen: new Set() });
      const entrance = entries.find((entry): entry is AnimationEntry & { entrance: NonNullable<AnimationEntry["entrance"]> } => entry.entrance !== undefined);
      if (entrance === undefined) throw new Error(`${defId} makes no entrance`);
      const from = sink.sent.length;
      director.onEntryStart(entrance);
      const sent = sink.sent.slice(from);
      const bed = (SIGNATURE_BEDS[defId] ?? []).map((step) => ({ id: step.id, delayMs: Math.round(step.at * entrance.durationMs) }));
      // Every step of its bed is sent as the entry starts, at its share of the entry, beside its events' own sounds.
      for (const step of bed) expect(sent, defId).toContainEqual(step);
      director.onIdle();
    }
  });
});
