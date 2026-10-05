// #185 Unit Slam: the queue gives each landing its slam (R700, R701), read off the newest view.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { createAnimationQueue, landingOf, planEntries, slamEntries, type AnimationEntry, type SlamResolver } from "./animations.ts";
import type { CardLookup } from "./catalog.ts";
import { SLAM_BURST, UNIT_SLAM } from "./damageFeel.ts";
import { slamTierFor } from "./slamResolver.ts";
import { baseView, emptySide, unit } from "../test/fixtures.ts";

const summoned = (instanceId: string, lane: number, player: "p1" | "p2" = "p1", defId = "core-004"): GameEvent => ({
  type: "summoned",
  player,
  instanceId,
  defId,
  row: "units",
  lane,
});

const plan = (events: GameEvent[], view: PlayerView, slamOf: SlamResolver, reduced = false): AnimationEntry[] =>
  slamEntries(planEntries(events, view, reduced), slamOf, reduced);

describe("R700 a Unit the viewer can see lands with its size's weight", () => {
  it("R700 slams on a summon into a unit zone and on an Animated card, never behind the sentinel or in the backrow", () => {
    expect(landingOf(summoned("a", 1))).not.toBeNull();
    expect(landingOf({ type: "animated", player: "p1", instanceId: "b", defId: "core-001", backrowLane: 1, unitLane: 1 })).not.toBeNull();
    expect(landingOf(summoned("c", 1, "p2", "hidden"))).toBeNull();
    expect(landingOf({ type: "summoned", player: "p1", instanceId: "d", defId: "core-004", row: "backrow", lane: 1 })).toBeNull();
    expect(landingOf({ type: "animated", player: "p1", instanceId: "e", defId: "hidden", backrowLane: 1, unitLane: 1 })).toBeNull();
    const entries = plan([summoned("c", 1, "p2", "hidden")], baseView(), () => "massive");
    expect(entries.every((entry) => entry.slam === undefined), "a hidden Unit never slams").toBe(true);
  });

  it("R700 reads the Unit as it stands in the newest view: Radiant stats, the view's Armor, Indestructible and its printed Tribute", () => {
    const giant = unit("p1", { instanceId: "g", defId: "core-077", radiant: true, attack: 3, health: 3, armor: 2, keywords: [] });
    const view = baseView({ you: emptySide("p1", { units: [giant, null, null, null, null] }) });
    const lookup: CardLookup = (_defId, radiant) => ({ name: "G", type: "Unit", text: radiant ? "Tribute 1" : "Taunt", tags: [] });
    // 3 + 3 + 2 × 2 + 4 × 1 = 14: Medium (the Radiant face's text is the one read).
    expect(slamTierFor({ type: "summoned", player: "p1", instanceId: "g", defId: "core-077", row: "units", lane: 1 }, view, lookup)).toBe("medium");
    const titan = unit("p1", { instanceId: "t", attack: 3, health: 3, keywords: [{ kind: "Indestructible" }] });
    const view2 = baseView({ you: emptySide("p1", { units: [titan, null, null, null, null] }) });
    const two: CardLookup = () => ({ name: "T", type: "Unit", text: "Tribute 2, Indestructible", tags: [] });
    expect(slamTierFor({ type: "summoned", player: "p1", instanceId: "t", defId: "x", row: "units", lane: 1 }, view2, two)).toBe("large");
  });

  it("R700 a Unit already gone from the newest view is read off its printed face", () => {
    const lookup: CardLookup = () => ({ name: "Big", type: "Unit", text: "Tribute 3", tags: [], attack: 10, health: 10 });
    // 10 + 10 + 4 × 3 = 32: Huge.
    expect(slamTierFor({ type: "summoned", player: "p2", instanceId: "gone", defId: "x", row: "units", lane: 2 }, baseView(), lookup)).toBe("huge");
  });

  it("R700 the opponent's (and so the AI's) Units slam exactly as the viewer's do", () => {
    const mine = plan([summoned("m", 1, "p1")], baseView(), () => "huge")[0]?.slam;
    const theirs = plan([summoned("o", 1, "p2")], baseView(), () => "huge")[0]?.slam;
    expect(mine?.side).toBe("you");
    expect(theirs?.side).toBe("opponent");
    expect({ ...theirs, instanceId: "", side: "", zone: "" }).toEqual({ ...mine, instanceId: "", side: "", zone: "" });
  });
});

describe("R701 a landing is its own animation entry, staggered and capped per action", () => {
  it("R701 each tier's hit-stop, shake and anticipation are its UNIT_SLAM row, the anticipation inside the entry", () => {
    for (const tier of ["tiny", "small", "medium", "large", "huge", "massive"] as const) {
      const [entry] = plan([summoned("a", 1)], baseView(), () => tier);
      const feel = UNIT_SLAM[tier];
      expect(entry?.slam).toMatchObject({ tier, hitStopMs: feel.hitStopMs, shakePx: feel.shakePx, anticipationMs: feel.anticipationMs });
      const [plain] = planEntries([summoned("a", 1)], baseView(), false);
      expect(entry?.durationMs, tier).toBe((plain?.durationMs ?? 0) + feel.anticipationMs);
    }
    expect(UNIT_SLAM.huge.anticipationMs).toBeLessThanOrEqual(300);
    expect(UNIT_SLAM.massive.anticipationMs).toBeLessThanOrEqual(500);
  });

  it("R701 several Units in one action land one after another and share the action's hit-stop, shake and anticipation", () => {
    const events = [summoned("a", 1), summoned("b", 2), summoned("c", 3), summoned("d", 4)];
    const entries = plan(events, baseView(), () => "massive");
    const slams = entries.flatMap((entry) => (entry.slam === undefined ? [] : [entry.slam]));
    expect(slams.map((slam) => slam.instanceId), "one entry each, in order").toEqual(["a", "b", "c", "d"]);
    const sum = (key: "hitStopMs" | "shakePx" | "anticipationMs"): number => slams.reduce((total, slam) => total + slam[key], 0);
    expect(sum("hitStopMs")).toBeLessThanOrEqual(SLAM_BURST.hitStopMs);
    expect(sum("shakePx")).toBeLessThanOrEqual(SLAM_BURST.shakePx);
    expect(sum("anticipationMs")).toBeLessThanOrEqual(SLAM_BURST.anticipationMs);
    expect(slams[0]?.hitStopMs, "the first still lands at full weight").toBe(UNIT_SLAM.massive.hitStopMs);
    // The next action has its own totals again.
    expect(plan([summoned("e", 5)], baseView(), () => "massive")[0]?.slam?.hitStopMs).toBe(UNIT_SLAM.massive.hitStopMs);
  });

  it("R701 under Reduce motion there is no anticipation and no shake; the hit-stop stays", () => {
    const [entry] = plan([summoned("a", 1)], baseView(), () => "massive", true);
    expect(entry?.slam).toMatchObject({ anticipationMs: 0, shakePx: 0, hitStopMs: UNIT_SLAM.massive.hitStopMs });
    expect(entry?.durationMs).toBe(0);
  });

  it("R701 the queue asks at every enqueue and holds the view back for the anticipation too", () => {
    const waits: number[] = [];
    const queue = createAnimationQueue({
      reducedMotion: false,
      schedule: (_fn, ms) => {
        waits.push(ms);
      },
      settings: () => ({ speed: 1, motion: "system" }),
      slamOf: () => "huge",
    });
    queue.enqueue([summoned("a", 1)], baseView());
    const [plain] = planEntries([summoned("a", 1)], baseView(), false);
    expect(queue.inFlight()?.slam?.tier).toBe("huge");
    expect(waits[0]).toBe((plain?.durationMs ?? 0) + UNIT_SLAM.huge.anticipationMs);
  });
});
