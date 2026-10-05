// The cue planner's issue-#124 entries (fx/cues.ts over game/runs.ts): a whole-pile impact plans
// one wave, a sweep plans one fog with timed hits, and a cast plans its flare at the caster.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { planEntries } from "../game/animations.ts";
import { baseView, card, emptySide, unit } from "../test/fixtures.ts";
import { FX_FOG_HIT_FROM, FX_FOG_HIT_TO, FX_INTENSITY_SCALE } from "./constants.ts";
import { planFx } from "./cues.ts";
import { createFxMemory } from "./memory.ts";
import type { FxCue, FxPlanEnv } from "./types.ts";

function pileView(): PlayerView {
  return baseView({
    you: emptySide("p1", {
      libraryCount: 12,
      graveyard: [card({ instanceId: "g1", defId: "core-003" }), card({ instanceId: "g2", defId: "core-003" })],
      units: [unit("p1", { instanceId: "y1" }), unit("p1", { instanceId: "y2" }), null, null, null],
    }),
    opponent: emptySide("p2", {
      hand: { count: 4 },
      units: [unit("p2", { instanceId: "e1" }), unit("p2", { instanceId: "e2" }), null, null, null],
    }),
  });
}

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

function plan(events: readonly GameEvent[], view: PlayerView, env: FxPlanEnv = envOf()): FxCue[] {
  const entries = planEntries(events, view, false);
  expect(entries, "these events should plan exactly one entry").toHaveLength(1);
  const entry = entries[0];
  if (entry === undefined) throw new Error("no entry");
  env.memory.remember(entry.events);
  return planFx(entry, view, env);
}

describe("planFx whole-pile impacts", () => {
  const degrades: GameEvent[] = [
    { type: "degraded", instanceId: "g1", defId: "core-003", change: { kind: "cost", delta: -1 } },
    { type: "degraded", instanceId: "g2", defId: "core-003", change: { kind: "cost", delta: -1 } },
  ];

  it("plans one wave over the pile instead of one cue per card", () => {
    const cues = plan(degrades, pileView());
    const zones = cues.filter((cue) => cue.kind === "zone");
    expect(zones).toHaveLength(1);
    expect(zones[0]).toMatchObject({ at: { kind: "testid", testid: "graveyard-you" }, direction: "down", delayMs: 0 });
    // The per-card recipes stay silent: nothing else is planned.
    expect(cues.filter((cue) => cue.kind !== "zone" && cue.kind !== "burst")).toHaveLength(0);
  });

  it("stays inside R200's bounds", () => {
    const D = 700;
    const cues = plan(degrades, pileView());
    for (const cue of cues) {
      if (!("delayMs" in cue)) continue;
      expect(cue.delayMs).toBeLessThanOrEqual(D);
      if ("durationMs" in cue) expect(cue.durationMs).toBeLessThanOrEqual(D + 900);
    }
  });
});

describe("planFx sweeps", () => {
  const hits: GameEvent[] = [
    { type: "damage", sourceId: null, targetId: "e1", amount: 2, combat: false },
    { type: "damage", sourceId: null, targetId: "e2", amount: 3, combat: false },
  ];

  it("rolls one fog over the swept side and lands each hit inside its pass", () => {
    const D = 600;
    const cues = plan(hits, pileView());
    const fogs = cues.filter((cue) => cue.kind === "fog");
    expect(fogs).toHaveLength(1);
    expect(fogs[0]).toMatchObject({ tone: "damage", delayMs: 0 });
    const splats = cues.filter((cue) => cue.kind === "splat");
    expect(splats.map((cue) => (cue.kind === "splat" ? cue.amount : 0)).sort()).toEqual([2, 3]);
    for (const cue of splats) {
      if (cue.kind !== "splat") continue;
      expect(cue.delayMs).toBeGreaterThanOrEqual(Math.round(D * FX_FOG_HIT_FROM));
      expect(cue.delayMs).toBeLessThanOrEqual(Math.round(D * FX_FOG_HIT_TO));
    }
  });

  it("tints the fog with what cast it when the view names it", () => {
    const view = pileView();
    const env = envOf({ card: (defId) => (defId === "core-010" ? { type: "Spell", tags: [] } : undefined) });
    const cues = plan(
      [
        { type: "damage", sourceId: "y1", targetId: "e1", amount: 2, combat: false },
        { type: "damage", sourceId: "y1", targetId: "e2", amount: 2, combat: false },
      ],
      view,
      env,
    );
    const fog = cues.find((cue) => cue.kind === "fog");
    expect(fog?.kind === "fog" ? fog.tint : null).not.toBe(null);
  });
});

describe("planFx casts", () => {
  it("flares at the caster's hero on top of the card's own cues", () => {
    const view = pileView();
    const events: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "box", defId: "classicplus-047", costPaid: 8 },
      { type: "cardAnnounced", player: "p1", instanceId: "c1", defId: "core-010", cardType: "Spell", costPaid: 0, targets: [] },
      { type: "cardPlayed", player: "p1", instanceId: "c1", defId: "core-010", costPaid: 0 },
    ];
    const entries = planEntries(events, view, false);
    const cast = entries.find((entry) => entry.cast !== undefined);
    expect(cast, "a cast entry").toBeDefined();
    if (cast === undefined) return;
    const env = envOf();
    env.memory.remember(cast.events);
    const cues = planFx(cast, view, env);
    // The announce's ring and sheen plus the cast flare's bursts.
    expect(cues.some((cue) => cue.kind === "ring")).toBe(true);
    expect(cues.filter((cue) => cue.kind === "burst").length).toBeGreaterThanOrEqual(2);
  });

  it("grows the flare with the burst's ordinal", () => {
    const view = pileView();
    const played = (id: string): GameEvent => ({ type: "cardPlayed", player: "p1", instanceId: id, defId: "core-010", costPaid: 0 });
    const first = planEntries(
      [
        { type: "cardPlayed", player: "p1", instanceId: "box", defId: "classicplus-047", costPaid: 8 },
        played("c1"),
        { type: "cardResolved", player: "p1", instanceId: "c1", defId: "core-010", permanent: false, costPaid: 0 },
        played("c2"),
      ],
      view,
      false,
    ).filter((entry) => entry.cast !== undefined);
    expect(first.map((entry) => entry.cast?.ordinal)).toEqual([1, 2]);
    const scales = first.map((entry) => {
      if (entry === undefined) return undefined;
      const cues = planFx(entry, view, envOf());
      return cues
        .filter((cue) => cue.kind === "burst")
        .map((cue) => (cue.kind === "burst" ? (cue.scale ?? 1) : 1));
    });
    expect(scales[0]?.every((scale) => scale === 1)).toBe(true);
    expect(scales[1]?.some((scale) => scale > 1)).toBe(true);
  });
});
