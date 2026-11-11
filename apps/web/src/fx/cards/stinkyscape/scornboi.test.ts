// R3407000: Scornboi's signature (scornboi.ts), planned as FxLayer plans it: a crimson gate slams down over
// his zone and he strides through it with his short list; each Unit his aura exiles as he lands is picked out
// by a crimson beam and dragged into a void hatch, and on the first the Units he spares glint. His gallery play
// within a Legendary's ceiling, R200 at any length, nothing with the effects off; the beam reaches only a card
// the view shows face up and no cue names a card (R202).

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
import { scornSweepFixture } from "./scornboi.ts";

const SCORNBOI = "stinkyscape-115";
const DS = [120, 200, 400, 732, 1400] as const;

const tid = (value: string, at?: { x: number; y: number }): FxAnchor => (at === undefined ? { kind: "testid", testid: value } : { kind: "testid", testid: value, at });
const kinds = (cues: readonly FxCue[] | undefined): string[] => (cues ?? []).map((cue) => cue.kind);
const played = (player: PlayerId): GameEvent => ({ type: "cardPlayed", player, instanceId: "scorn", defId: SCORNBOI, costPaid: 3 });
const landing = (player: PlayerId): Extract<GameEvent, { type: "summoned" }> => ({ type: "summoned", player, instanceId: "scorn", defId: SCORNBOI, row: "units", lane: 3 });

describe("R3407000 Scornboi's signature", () => {
  it("R3407000 his table entry is his signature", () => {
    expect(FX_CARD_TABLE.get(SCORNBOI)).toEqual({ kind: "bespoke", key: "scornSweep", entrance: "scornGate", aura: { look: "searchlight", palette: "Dark/blood", scope: "tile" }, signature: true });
  });

  it("R3407000 a crimson gate slams down over his zone, cracking its foot, and he strides through with his short list", () => {
    const D = 400;
    const tgt = testid.zone("you", "units", 3);
    const slam = frac(FX_SLAM_AT, D);
    const cues = ENTRANCES.scornGate({ D, tgt, intensity: 1, view: fullBoardView(), landing: landing("p1") });
    expect(kinds(cues)).toEqual(["shot", "ring", "crack", "burst", "sheen", "burst", "figure", "shake"]);
    expect(cues[0]).toMatchObject({ to: tid(tgt, { x: 0.5, y: 1 }) });
    expect(cues[2]).toMatchObject({ at: tid(tgt, { x: 0.5, y: 1 }), delayMs: slam });
    expect(cues.at(-1)).toEqual({ kind: "shake", trauma: ENTRANCE_TRAUMA.scornGate, delayMs: slam });
  });

  it("R3407000 each Unit his aura exiles is beamed into a void hatch, and on the first the Units he spares glint", () => {
    const view = fullBoardView();
    const D = 350;
    const exiles = scornSweepFixture(view, "scorn", "p1");
    expect(exiles).toHaveLength(1);
    const [first] = beatsOf(exiles, view, D, [played("p1"), landing("p1")]);
    expect(first?.row).toBe("replace");
    const foe = exiles[0]?.type === "exiled" ? exiles[0].instanceId : "";
    expect(kinds(first?.cues).slice(0, 3)).toEqual(["shot", "ring", "burst"]);
    expect(first?.cues[0]).toMatchObject({ from: tid(testid.zone("you", "units", 3)), to: tid(testid.card(foe)) });
    const shown = [...view.you.units, ...view.opponent.units].filter((unit) => unit !== null && unit.defId !== "hidden" && unit.instanceId !== foe).length;
    expect(first?.cues.filter((cue) => cue.kind === "sheen")).toHaveLength(shown);
    // A second exile beams again, with no second glint; a card the board does not show draws nothing.
    const second = view.opponent.units.find((unit) => unit !== null && unit.instanceId !== foe && unit.defId !== "hidden");
    const [, again] = beatsOf([...exiles, { type: "exiled", instanceId: second?.instanceId ?? "", defId: second?.defId ?? "", owner: "p2" }], view, D, [played("p1"), landing("p1")]);
    expect(kinds(again?.cues)).toEqual(["shot", "ring", "burst"]);
    expect(beatsOf([{ type: "exiled", instanceId: "not-shown", defId: "core-002", owner: "p2" }], view, D, [played("p1")])).toEqual([null]);
    // Another card's exile is not his sweep.
    expect(beatsOf(exiles, view, D, [{ type: "cardPlayed", player: "p1", instanceId: "x", defId: "core-002", costPaid: 1 }])).toEqual([null]);
  });

  it("R3407000 R200 every cue starts inside its entry and ends within FX_MAX_TAIL_MS, at any length; nothing draws with the effects off", () => {
    const view = fullBoardView();
    const tgt = testid.zone("you", "units", 3);
    for (const D of DS) {
      expectCuesR200(ENTRANCES.scornGate({ D, tgt, intensity: 1, view, landing: landing("p1") }), D, "scornGate", ENTRANCE_TRAUMA.scornGate);
      const sweep = beatsOf(scornSweepFixture(view, "scorn", "p1"), view, D, [played("p1"), landing("p1")]).flatMap((beat) => beat?.cues ?? []);
      expect(sweep.length).toBeGreaterThan(0);
      expectCuesR200(sweep, D, "scornSweep");
      expectNoCard(sweep, SCORNBOI);
    }
    expect(cuesOfRun(planWithOffsets([played("p1"), landing("p1"), ...scornSweepFixture(view, "scorn", "p1")], view, envOf(FX_INTENSITY_SCALE.off)))).toEqual([]);
  });

  it("R3407000 his gallery play ends within a Legendary's ceiling", () => {
    expectGalleryWithinCeiling(SCORNBOI);
  });
});
