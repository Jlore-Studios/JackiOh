// R3332001: Felinor Fiender's signature entrance (felinorFiender.ts), planned the way FxLayer plans it: a
// streak of cat light from each Felinor its controller's side shows face up, in lane order, all landing at
// the slam; none from a card buried under a Stack, a face-down card or the other side's; a Felinor it
// stacks onto streaks up from its own foot; and no cue names a card (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { fullBoardView, withEvents } from "../../../test/fixtures.ts";
import { CATALOG } from "../../../wire/cards.ts";
import { frac } from "../../build.ts";
import { FX_FELINOR_STREAK_AT, FX_FELINOR_STREAK_STEP, FX_INTENSITY_SCALE, FX_SLAM_AT } from "../../constants.ts";
import { planFx } from "../../cues.ts";
import { paletteLook } from "../../flavors.ts";
import { createFxMemory } from "../../memory.ts";
import type { FxCue, FxPlanEnv, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT } from "../recipe.ts";
import { GALLERY_FELINORS, GALLERY_FELINOR_LANES, felinorAssembly, felinorBoard, felinorZones } from "./felinorFiender.ts";

const FIENDER = "core-092";
const I = FX_INTENSITY_SCALE.normal;

function landingOf(player: PlayerId, lane: number, instanceId = "ff"): Extract<GameEvent, { type: "summoned" }> {
  return { type: "summoned", player, instanceId, defId: FIENDER, row: "units", lane };
}

function envOf(): FxPlanEnv {
  return { intensity: I, card: () => undefined, memory: createFxMemory() };
}

/**
 * Its play and its landing, planned as the board plans them over `view` (the view it still shows), into
 * an empty zone, so no Stack beat (R2791000) lands beside the entrance.
 */
function planPlay(board: PlayerView, landing: Extract<GameEvent, { type: "summoned" }>): { cues: FxCue[]; D: number } {
  const side = sideOf(board, landing.player);
  const view: PlayerView = { ...board, [side]: { ...board[side], units: board[side].units.map((unit, k) => (k === landing.lane - 1 ? null : unit)) } };
  const events: GameEvent[] = [{ type: "cardPlayed", player: landing.player, instanceId: landing.instanceId, defId: FIENDER, costPaid: 2 }, landing];
  const env = envOf();
  const [entry] = planEntries(events, withEvents(view, events), false);
  if (entry === undefined) throw new Error("no entry");
  env.memory.remember(entry.events);
  return { cues: planFx(entry, view, env), D: entry.durationMs };
}

const shotsOf = (cues: readonly FxCue[]): FxShotCue[] => cues.filter((cue): cue is FxShotCue => cue.kind === "shot");
const fromOf = (shot: FxShotCue): string => (shot.from.kind === "testid" ? shot.from.testid : "");

describe("R3332001 Felinor Fiender's signature", () => {
  it("R3332001 Felinor Fiender assembles from each Felinor its controller shows", () => {
    // The gallery's board: Felinors in lanes 1, 2 and 5; lane 4 a Stack pile of three buried cards.
    const base = fullBoardView();
    const view = felinorBoard(base);
    expect(GALLERY_FELINOR_LANES.map((lane) => view.you.units[lane - 1]?.defId)).toEqual([...GALLERY_FELINORS]);
    for (const defId of GALLERY_FELINORS) expect(CATALOG[defId]?.tags, defId).toContain("Felinor");
    expect(GALLERY_FELINOR_LANES.map((lane) => view.you.units[lane - 1]?.instanceId)).toEqual(GALLERY_FELINOR_LANES.map((lane) => base.you.units[lane - 1]?.instanceId));
    expect(view.you.units[3]?.buried).toBe(3);

    // A Felinor of the other side, which streaks nothing into this one's landing.
    const theirs: PlayerView = { ...view, opponent: { ...view.opponent, units: view.opponent.units.map((unit, k) => (k === 0 && unit !== null ? { ...unit, defId: "core-043" } : unit)) } };
    const landing = landingOf("p1", 3);
    expect(felinorZones(theirs, landing)).toEqual(["zone-you-units-1", "zone-you-units-2", "zone-you-units-5"]);

    const { cues, D } = planPlay(theirs, landing);
    const tgt = testid.zone("you", "units", 3);
    expect(cues).toEqual(felinorAssembly({ D, tgt, intensity: I, view: theirs, landing }));
    const slam = frac(FX_SLAM_AT, D);
    const shots = shotsOf(cues);
    // Three shots, in lane order, each leaving a step after the last and all landing at the slam.
    expect(shots.map(fromOf)).toEqual(["zone-you-units-1", "zone-you-units-2", "zone-you-units-5"]);
    shots.forEach((shot, n) => {
      expect(shot.delayMs).toBe(frac(FX_FELINOR_STREAK_AT + n * FX_FELINOR_STREAK_STEP, D));
      expect(shot.delayMs + shot.flightMs).toBe(slam);
      expect(shot.to).toEqual({ kind: "testid", testid: tgt });
      expect(shot.colors).toEqual(paletteLook("Beast/felinor")?.colors);
    });
    // None from lane 4's pile (its top no Felinor, its buried cards face down) or the other side.
    expect(shots.some((shot) => fromOf(shot) === "zone-you-units-4" || fromOf(shot).startsWith("zone-opponent-"))).toBe(false);
    // At the slam: a ring, the gathering and a crown of gold, and one shake, no gold area burst.
    const landed = cues.filter((cue) => cue.kind !== "shot");
    expect(landed.map((cue) => cue.kind)).toEqual(["ring", "burst", "burst", "shake"]);
    for (const cue of landed) expect(cue.delayMs).toBe(slam);
    expect(cues.some((cue) => cue.kind === "burst" && cue.preset === "gold" && cue.spread === "area")).toBe(false);
    expect(cues.filter((cue) => cue.kind === "shake")).toEqual([{ kind: "shake", trauma: ENTRANCE_TRAUMA.felinorAssembly * I, delayMs: slam }]);
    // R202: no cue names a card.
    const text = JSON.stringify(cues);
    for (const defId of [FIENDER, ...GALLERY_FELINORS]) expect(text).not.toContain(defId);

    // The beats scale with the entry.
    for (const d of [250, 400, 800, 1400]) {
      const scaled = felinorAssembly({ D: d, tgt, intensity: I, view, landing });
      expect(shotsOf(scaled).map((shot) => shot.delayMs)).toEqual([0, 1, 2].map((n) => frac(FX_FELINOR_STREAK_AT + n * FX_FELINOR_STREAK_STEP, d)));
      for (const shot of shotsOf(scaled)) expect(shot.delayMs + shot.flightMs).toBe(frac(FX_SLAM_AT, d));
    }
  });

  it("R3332001 a Felinor it stacks onto streaks from its foot, and the other seat's play reads its own side", () => {
    const view = felinorBoard(fullBoardView());
    // Landing on lane 1's Felinor, which the board still shows on top until it lands.
    const onto = felinorAssembly({ D: 400, tgt: testid.zone("you", "units", 1), intensity: I, view, landing: landingOf("p1", 1) });
    const [first] = shotsOf(onto);
    expect(first?.from).toEqual({ kind: "testid", testid: "zone-you-units-1", at: FOOT });
    expect(shotsOf(onto).slice(1).map((shot) => shot.from)).toEqual([
      { kind: "testid", testid: "zone-you-units-2" },
      { kind: "testid", testid: "zone-you-units-5" },
    ]);
    // Played by the other seat, it counts that side's Felinors, none here.
    expect(felinorZones(view, landingOf("p2", 3))).toEqual([]);
    // A unit granted the tag counts (its view's own tags); a face-down card and the landing card itself do not.
    const granted: PlayerView = {
      ...view,
      opponent: { ...view.opponent, units: view.opponent.units.map((unit, k) => (k === 1 && unit !== null ? { ...unit, tags: ["Felinor"] } : unit)) },
      you: { ...view.you, units: view.you.units.map((unit, k) => (k === 0 && unit !== null ? { ...unit, defId: "hidden" } : k === 1 && unit !== null ? { ...unit, instanceId: "ff" } : unit)) },
    };
    expect(felinorZones(granted, landingOf("p2", 3))).toEqual(["zone-opponent-units-2"]);
    expect(felinorZones(granted, landingOf("p1", 3))).toEqual(["zone-you-units-5"]);
  });

  it("R3332001 with no Felinor it lands alone and no cue names a card", () => {
    const view = fullBoardView();
    const landing = landingOf("p1", 3);
    expect(felinorZones(view, landing)).toEqual([]);
    const { cues, D } = planPlay(view, landing);
    expect(cues.map((cue) => cue.kind)).toEqual(["ring", "burst", "burst", "shake"]);
    // Planned with no board at all, it is the same landing.
    expect(felinorAssembly({ D, tgt: testid.zone("you", "units", 3), intensity: I })).toEqual(cues);
    expect(JSON.stringify(cues)).not.toMatch(/core-\d{3}|Felinor/);
  });
});
