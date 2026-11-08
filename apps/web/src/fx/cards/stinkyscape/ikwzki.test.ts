// R3407000: iKwzki's signature (ikwzki.ts, duoBond.ts), planned as FxLayer plans it: after her own
// takeover's hold the scene folds into her zone as motes from the board's four corners and dawn rises there;
// her Duo's ribbon ties its knot on the permanent it marks (duoBond, flourishes.test.ts), and at her next turn
// the knot blooms on the card it made Radiant. Her gallery play within a Mythic's ceiling, R200 at any length,
// nothing with the effects off; the bloom anchors on the shown card or its zone and names no card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { beatsOf, cuesOfRun, envOf, expectCuesR200, expectGalleryWithinCeiling, expectNoCard, planWithOffsets } from "../../../test/signaturePass.ts";
import { frac } from "../../build.ts";
import { FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { ENTRANCES } from "../../entrances.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA } from "../recipe.ts";
import { FX_CARD_TABLE } from "../tables.ts";
import { landMsAfter } from "./signature.ts";

const IKWZKI = "stinkyscape-110";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "ik", defId: IKWZKI, row: "units", lane: 3 });
const turn = (player: PlayerId): GameEvent => ({ type: "hookResolving", sourceId: "ik", defId: IKWZKI, controller: player, hook: "startOfTurn" });

describe("R3407000 iKwzki's signature", () => {
  it("R3407000 her table entry is her signature, with her own takeover look", () => {
    expect(FX_CARD_TABLE.get(IKWZKI)).toEqual({ kind: "bespoke", key: "duoBond", entrance: "ikwzkiDawn", aura: { look: "duohalo", palette: "Spirit", scope: "tile" }, takeover: "ikwzki", signature: true });
  });

  it("R3407000 she offers her hand and the scene folds into her zone from the board's four corners, dawn rising there at the slam", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.ikwzkiDawn({ D, tgt, intensity: 1, view: fullBoardView(), landing: landing("p1") });
    expect(kinds(cues)).toEqual(["burst", "burst", "burst", "burst", "burst", "shot", "shot", "shot", "shot", "rays", "ring", "ring", "burst", "shake"]);
    expect(cues[0]).toMatchObject({ at: tid(tgt) });
    for (const shot of cues.filter((cue) => cue.kind === "shot")) expect(shot).toMatchObject({ to: tid(tgt) });
    expect(cues[9]).toMatchObject({ kind: "rays", at: tid(tgt), delayMs: slam });
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.ikwzkiDawn, delayMs: slam });
    const wait = 1000;
    const held = ENTRANCES.ikwzkiDawn({ D: 1400, tgt, intensity: 1, landMs: landMsAfter(wait, 1400) });
    for (const cue of held) expect(cue.delayMs, cue.kind).toBeGreaterThanOrEqual(wait);
    expect(held.at(-1)?.delayMs).toBe(wait + frac(FX_SLAM_AT, 1400 - wait));
  });

  it("R3407000 at her next turn the knot blooms on the card it made Radiant: rays, gold, rose motes and a ring there, in place of the radiant row", () => {
    const view = fullBoardView();
    const D = 400;
    const unit = view.you.units[0];
    if (unit === null || unit === undefined) throw new Error("no unit to bloom");
    const radiant: Extract<GameEvent, { type: "radiantSet" }> = { type: "radiantSet", instanceId: unit.instanceId, defId: unit.defId, zone: { z: "field", player: "p1", row: "units", lane: 1 } };
    const [bloom] = beatsOf([radiant], view, D, [turn("p1")]);
    expect(bloom?.row).toBe("replace");
    expect(kinds(bloom?.cues)).toEqual(["rays", "burst", "burst", "ring"]);
    for (const cue of bloom?.cues ?? []) expect(cue, cue.kind).toMatchObject({ at: tid(testid.card(unit.instanceId)) });
    // A card the board shows face down blooms on its zone; one off the field draws nothing (R202).
    const [hidden] = beatsOf([{ ...radiant, instanceId: "not-shown", defId: "hidden" }], view, D, [turn("p1")]);
    expect(hidden?.cues[0]).toMatchObject({ at: tid(testid.zone("you", "units", 1)) });
    expect(beatsOf([{ ...radiant, zone: { z: "hand", player: "p1" } }], view, D, [turn("p1")])).toEqual([null]);
    // Outside a hook of hers, a radiantSet is not her bloom.
    expect(beatsOf([radiant], view, D, [{ type: "cardPlayed", player: "p1", instanceId: "ik", defId: IKWZKI, costPaid: 7 }])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "units", 3);
    const unit = view.you.units[0];
    const radiant: Extract<GameEvent, { type: "radiantSet" }> = { type: "radiantSet", instanceId: unit?.instanceId ?? "", defId: unit?.defId ?? "", zone: { z: "field", player: "p1", row: "units", lane: 1 } };
    for (const D of DS) {
      expectCuesR200(ENTRANCES.ikwzkiDawn({ D, tgt, intensity: 1, view, landing: landing("p1") }), D, "ikwzkiDawn", ENTRANCE_TRAUMA.ikwzkiDawn);
      const bloom = beatsOf([radiant], view, D, [turn("p1")]).flatMap((beat) => beat?.cues ?? []);
      expect(bloom.length).toBeGreaterThan(0);
      expectCuesR200(bloom, D, "duoBond bloom");
      expectNoCard(bloom, IKWZKI);
    }
    expect(cuesOfRun(planWithOffsets([{ type: "cardPlayed", player: "p1", instanceId: "ik", defId: IKWZKI, costPaid: 7 }, landing("p1")], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
    expect(cuesOfRun(planWithOffsets([turn("p1"), radiant], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 her gallery play ends within a Mythic's ceiling", () => {
    expectGalleryWithinCeiling(IKWZKI);
  });
});
