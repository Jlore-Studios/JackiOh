// R3337002: the Scanning Probes' scan (`probeScan`) and Anomaly Found!'s call (`anomalyCall`), planned the
// way FxLayer plans them (remember, then plan). The sweep at the probe's public play draws on both seats;
// a find pings and flies into the hand on its owner's seat alone, a Radiant one flashing; the other seat's
// find names the sentinel and keys nothing (R202). Each lands inside its entry and its tail (R200).

import type { CardView, GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANOMALY_FOUND_ID } from "../../../cards/anomalies.ts";
import { animTestid } from "../../../game/animations.ts";
import { testid } from "../../../game/contract.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import { planCardFx } from "../../cardFx.ts";
import { FX_INTENSITY_SCALE, FX_MAX_PARTICLE_LIFE_MS, FX_MAX_TAIL_MS, FX_MOTIF_CENTER_LANE } from "../../constants.ts";
import { createFxMemory } from "../../memory.ts";
import { galleryEntry, RECIPE_FIXTURES } from "../../motifs/gallery.ts";
import type { FxCue, FxPlanEnv } from "../../types.ts";
import { FX_CARD_TABLE } from "../tables.ts";

const MK1 = "stinkyscape-166";
const MK2 = "stinkyscape-167";
const D = 400;

function endOf(cue: FxCue): number {
  switch (cue.kind) {
    case "burst":
      return cue.delayMs + FX_MAX_PARTICLE_LIFE_MS;
    case "shake":
      return cue.delayMs;
    default:
      return cue.delayMs + ("durationMs" in cue ? cue.durationMs : 0);
  }
}

const played = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardPlayed", player, instanceId, defId, costPaid: 1 });
const resolved = (player: PlayerId, instanceId: string, defId: string): GameEvent => ({ type: "cardResolved", player, instanceId, defId, permanent: false, costPaid: 1 });
const found = (player: PlayerId, instanceId: string, defId: string = ANOMALY_FOUND_ID): GameEvent => ({ type: "addedToHand", player, instanceId, defId });

/** `view` with the viewer's hand holding an Anomaly Found! `instanceId`, Radiant or not (the next view). */
function holding(view: PlayerView, instanceId: string, radiant: boolean): PlayerView {
  const hand = Array.isArray(view.you.hand) ? view.you.hand : [];
  const card = { ...(hand[0] as CardView), instanceId, defId: ANOMALY_FOUND_ID, radiant };
  return { ...view, you: { ...view.you, hand: [...hand, card] } };
}

/** Each of `events`, one entry each lasting D, planned as FxLayer plans it: its cues. */
function run(events: readonly GameEvent[], view: PlayerView, next?: PlayerView): FxCue[][] {
  const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...(next === undefined ? {} : { next }) };
  return events.map((event) => {
    env.memory.remember([event]);
    const entry = { ...galleryEntry([event], view), durationMs: D };
    return planCardFx(event, { entry, view, env, D })?.cues ?? [];
  });
}

const kinds = (cues: readonly FxCue[]): string[] => cues.map((cue) => cue.kind);

describe("R3337002 a Scanning Probe's scan", () => {
  it("R3337002 both probes key probeScan, and Anomaly Found! anomalyCall", () => {
    expect(FX_CARD_TABLE.get(MK1)).toEqual({ kind: "bespoke", key: "probeScan" });
    expect(FX_CARD_TABLE.get(MK2)).toEqual({ kind: "bespoke", key: "probeScan" });
    expect(FX_CARD_TABLE.get(ANOMALY_FOUND_ID)).toEqual({ kind: "bespoke", key: "anomalyCall" });
  });

  it("R3337002 a find sweeps, pings on the board and flies into the hand; a Radiant one flashes; a miss sweeps only", () => {
    const view = fullBoardView();
    for (const probe of [MK1, MK2]) {
      const miss = run([played("p1", "probe", probe), resolved("p1", "probe", probe)], view);
      expect(kinds(miss[0] ?? []), probe).toEqual(["radar"]);
      expect(miss[0]?.[0]).toMatchObject({ kind: "radar", at: { kind: "testid", testid: animTestid.hand("you") }, side: "you", delayMs: 0 });
      expect(miss[1], probe).toEqual([]);

      const plain = run([played("p1", "probe", probe), found("p1", "af"), resolved("p1", "probe", probe)], view, holding(view, "af", false));
      const ping = plain[1] ?? [];
      expect(kinds(ping), probe).toEqual(["ring", "burst", "ghost", "burst"]);
      expect(ping.find((cue) => cue.kind === "ghost")).toMatchObject({ from: { kind: "testid", testid: testid.board }, to: { kind: "testid", testid: animTestid.hand("you") } });
      expect(ping.some((cue) => cue.kind === "rays" || cue.kind === "shake")).toBe(false);

      const radiant = run([played("p1", "probe", probe), found("p1", "af")], view, holding(view, "af", true))[1] ?? [];
      expect(kinds(radiant), probe).toEqual(expect.arrayContaining(["ring", "ghost", "rays", "burst", "shake"]));
      expect(radiant.find((cue) => cue.kind === "burst" && cue.preset === "prismatic")).toBeDefined();
      for (const cue of [...ping, ...radiant, ...(miss[0] ?? [])]) expect(endOf(cue), `${probe} ${cue.kind}`).toBeLessThanOrEqual(D + FX_MAX_TAIL_MS);
    }
  });

  it("R3337002 R202 the opponent's view of a find carries no ping or card", () => {
    const view = fullBoardView();
    // The other seat watches p2's probe: its play is public, its find names the sentinel.
    const theirs = run([played("p2", "probe", MK1), found("p2", "hidden", "hidden"), resolved("p2", "probe", MK1)], view);
    const miss = run([played("p2", "probe", MK1), resolved("p2", "probe", MK1)], view);
    expect(theirs[0]).toEqual(miss[0]);
    expect(theirs[0]?.[0]).toMatchObject({ kind: "radar", at: { kind: "testid", testid: animTestid.hand("opponent") }, side: "opponent" });
    expect(theirs[1]).toEqual([]);
    expect(JSON.stringify(theirs)).not.toMatch(/stinkyscape-|hidden/);
    // A hidden play keys nothing at all.
    expect(run([played("p2", "hidden", "hidden"), found("p2", "hidden", "hidden")], view).flat()).toEqual([]);
  });

  it("R3337003 Anomaly Found!'s call gathers where the rift tears, at its play's boardReplaced, and keeps the row", () => {
    const view = fullBoardView();
    const [, call] = run([played("p1", "af", ANOMALY_FOUND_ID), ...RECIPE_FIXTURES.anomalyCall(view, "af", "p1")], view);
    expect(kinds(call ?? [])).toEqual(["ring", "burst"]);
    const centre = { kind: "testid", testid: testid.zone("you", "units", FX_MOTIF_CENTER_LANE) };
    expect(call?.every((cue) => cue.kind !== "shake" && "at" in cue && JSON.stringify(cue.at) === JSON.stringify(centre))).toBe(true);
    expect(run([played("p1", "hidden", "hidden"), ...RECIPE_FIXTURES.anomalyCall(view, "hidden", "p1")], view).flat()).toEqual([]);
  });
});
