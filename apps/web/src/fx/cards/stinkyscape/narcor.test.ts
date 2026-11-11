// R3407000: the Narcors' signatures (narcor.ts), planned as FxLayer plans them: Narcor's dust blows from its
// caster's hero to the hand pile and the card it adds settles there as a back; Red Narcor's crystal strikes
// its caster's hero as its takeover's hold ends, its mana rings and its draw slides in as a back. Each run
// within its ceiling, R200 at any length, nothing with the effects off, and from the opponent's seat no cue
// names a card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import {
  asTheirsAll,
  beatsOf,
  ceilingOf,
  cuesOfRun,
  envOf,
  expectCuesR200,
  expectGalleryWithinCeiling,
  expectNoCard,
  expectR200,
  latestEnd,
  planWithOffsets,
} from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_CENTER, FX_INTENSITY_SCALE, SIGNATURE_SPELL_MAX_TRAUMA } from "../../constants.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { narcorDustFixture, redNarcorHumFixture } from "./narcor.ts";

const NARCOR = "stinkyscape-t-narcor";
const RED = "stinkyscape-t-red-narcor";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const played = (player: PlayerId, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId: "narcor", defId, costPaid: 0 });
const resolved = (player: PlayerId, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId: "narcor", defId, permanent: false, costPaid: 0 });
const drawn = (player: PlayerId): GameEvent => ({ type: "drawn", player, instanceId: "fx-drawn", defId: "core-003" });
/** Each card's play, its fixture's events and, Red Narcor's, the draw its test proves. */
const playOf = (defId: string, player: PlayerId): GameEvent[] => {
  const view = fullBoardView();
  const inner = defId === NARCOR ? narcorDustFixture(view, "narcor", player) : [...redNarcorHumFixture(view, "narcor", player), drawn(player)];
  return [played(player, defId), ...inner, resolved(player, defId)];
};

describe("R3407000 the Narcors' signatures", () => {
  it("R3407000 their table entries are their signatures, Red Narcor with its own takeover look", () => {
    expect(FX_CARD_TABLE.get(NARCOR)).toEqual({ kind: "bespoke", key: "narcorDust", signature: true });
    expect(FX_CARD_TABLE.get(RED)).toEqual({ kind: "bespoke", key: "redNarcorHum", takeover: "redNarcor", signature: true });
  });

  it("R3407000 Narcor blows its dust from its caster's hero to the hand pile, and the card it adds settles there as a back", () => {
    const view = fullBoardView();
    const D = 400;
    const hand = tid(animTestid.hand("you"));
    const [cast, added] = beatsOf([played("p1", NARCOR), ...narcorDustFixture(view, "narcor", "p1")], view, D);
    expect(cast?.row).toBe("replace");
    expect(kinds(cast?.cues)).toEqual(["burst", "shot", "ring", "burst"]);
    expect(cast?.cues[0]).toMatchObject({ at: tid(testid.hero("you")), preset: "dust" });
    expect(cast?.cues[1]).toMatchObject({ from: tid(testid.hero("you")), to: hand });
    expect(cast?.cues[3]).toMatchObject({ at: hand, delayMs: frac(0.45, D) });
    expect(added).toEqual({ cues: [{ kind: "ghost", from: tid(testid.board, FX_CENTER), to: hand, delayMs: 0, durationMs: frac(0.8, D) }, expect.objectContaining({ kind: "ring", at: hand })], row: "replace" });
    // Another card's add is not its beat.
    expect(beatsOf([{ type: "addedToHand", player: "p1", instanceId: "x", defId: "core-003" }], view, D, [played("p1", "core-002")])).toEqual([null]);
  });

  it("R3407000 Red Narcor's crystal strikes its caster's hero as the hold ends; its mana rings and its draw slides in as a back", () => {
    const view = fullBoardView();
    const D = 400;
    const hero = tid(testid.hero("you"));
    const [cast, mana, draw] = beatsOf([played("p1", RED), ...redNarcorHumFixture(view, "narcor", "p1"), drawn("p1")], view, D);
    expect(kinds(cast?.cues)).toEqual(["projectile", "ring", "burst", "shake"]);
    expect(cast?.cues[0]).toMatchObject({ preset: "fire", from: tid(testid.board, FX_CENTER), to: hero });
    expect(cast?.cues[1]).toMatchObject({ at: hero });
    expect(mana).toEqual({ cues: [expect.objectContaining({ kind: "ring", at: tid(animTestid.mana("you")) })], row: "replace" });
    expect(draw).toEqual({ cues: [{ kind: "ghost", from: tid(animTestid.library("you")), to: tid(animTestid.hand("you")), delayMs: 0, durationMs: frac(0.8, D) }], row: "replace" });
    // The other player's mana is not its beat.
    expect(beatsOf([{ type: "manaChanged", player: "p2", current: 1, max: 1 }], view, D, [played("p1", RED)])).toEqual([null]);
  });

  it("R3407000 R200 every cue of both starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = fullBoardView();
    for (const D of DS) {
      for (const defId of [NARCOR, RED]) {
        const cues = beatsOf(playOf(defId, "p1"), view, D).flatMap((beat) => beat?.cues ?? []);
        expect(cues.length, defId).toBeGreaterThan(0);
        expectCuesR200(cues, D, defId, SIGNATURE_SPELL_MAX_TRAUMA);
      }
    }
    for (const defId of [NARCOR, RED]) expect(cuesOfRun(planWithOffsets(playOf(defId, "p1"), view, envOf(FX_INTENSITY_SCALE.off))), defId).toEqual([]);
  });

  it("R3407000 each gallery play, and Red Narcor's draw after its mana, end within its ceiling", () => {
    expectGalleryWithinCeiling(NARCOR);
    expectGalleryWithinCeiling(RED);
    const run = planWithOffsets(playOf(RED, "p1"), fullBoardView());
    expect(latestEnd(run)).toBeLessThanOrEqual(ceilingOf(RED));
    expectR200(run, RED);
  });

  it("R3407000 R202 from the opponent's seat the cards they add and draw are backs to their piles and no cue names a card", () => {
    const view = fullBoardView();
    for (const defId of [NARCOR, RED]) {
      const run = planWithOffsets(asTheirsAll(playOf(defId, "p1")), view);
      const cues = cuesOfRun(run);
      expect(cues.length, defId).toBeGreaterThan(0);
      expectNoCard(cues, defId);
      const ghosts = cues.filter((cue) => cue.kind === "ghost");
      expect(ghosts.length, defId).toBeGreaterThan(0);
      for (const ghost of ghosts) expect(ghost, defId).toMatchObject({ to: tid(animTestid.hand("opponent")) });
    }
  });
});
