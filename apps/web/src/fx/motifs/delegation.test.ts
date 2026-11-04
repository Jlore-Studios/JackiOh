// R2784000: the generic rows #2786 folds into motifs (docs/polish/9-card-animations.md): each
// delegated recipe plans exactly its motif with default parameters, so one thing is never drawn
// two ways. Hidden cards stay hidden (R202): `tithe` glints by count on backs, and `snap` looks
// the same for every Trap until the view names the card (R154, R202).

import type { Flavor, GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, planEntries, type AnimationEntry } from "../../game/animations.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { must } from "../../test/stubServer.ts";
import { FX_INTENSITY_SCALE, FX_MOTIF_TITHE_MAX, FX_MOTIF_TRAUMA } from "../constants.ts";
import { planFx } from "../cues.ts";
import { FLAVOR_LOOKS } from "../flavors.ts";
import { createFxMemory } from "../memory.ts";
import type { FxCue, FxPlanEnv } from "../types.ts";
import type { MotifSpec } from "./build.ts";
import { planMotif } from "./index.ts";
import type { DrawnMotif } from "./names.ts";

function envOf(): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
}

function entryFor(events: readonly GameEvent[], view: PlayerView): AnimationEntry {
  const entries = planEntries(events, view, false);
  expect(entries, "these events should plan exactly one entry").toHaveLength(1);
  const entry = must(entries[0], "an entry");
  return { ...entry, durationMs: ANIMATIONS[must(events[0], "an event").type].durationMs };
}

/** What the FX layer does on `start`: remember the entry's events, then plan it. */
function planed(events: readonly GameEvent[], view: PlayerView): { entry: AnimationEntry; env: FxPlanEnv } {
  const entry = entryFor(events, view);
  const env = envOf();
  env.memory.remember(entry.events);
  return { entry, env };
}

type Delegated = { recipe: string; events: () => GameEvent[]; motif: DrawnMotif; flavor: Flavor };

/* Fixture ids, read off the board rather than guessed */
const VIEW: PlayerView = fullBoardView();

/** The viewer's lane-3 unit. */
const MINE_3 = must(VIEW.you.units[2], "your lane-3 unit").instanceId;
/** The opponent's lane-1 unit. */
const ENEMY = must(VIEW.opponent.units[0], "the enemy lane-1 unit").instanceId;
/** The viewer's face-up Trap in backrow lane 5. */
const MY_TRAP = (() => {
  const slot = must(VIEW.you.backrow[4], "backrow lane 5");
  if (slot.faceDown) throw new Error("backrow lane 5 is not face-up");
  return slot.instanceId;
})();
/** The viewer's face-up Field Trap in backrow lane 2. */
const MY_FIELD_TRAP = (() => {
  const slot = must(VIEW.you.backrow[1], "backrow lane 2");
  if (slot.faceDown) throw new Error("backrow lane 2 is not face-up");
  return slot.instanceId;
})();

/** Every generic recipe #2786 folds into a motif, and the default parameters it plans with. */
const DELEGATED: readonly Delegated[] = [
  { recipe: "keyword", events: () => [{ type: "keywordGranted", instanceId: MINE_3, keyword: { kind: "Divine Shield" } }], motif: "bless", flavor: "Holy" },
  { recipe: "keyword", events: () => [{ type: "keywordGranted", instanceId: ENEMY, keyword: { kind: "Taunt" }, lost: true }], motif: "curse", flavor: "Dark" },
  { recipe: "buff", events: () => [{ type: "buffed", instanceId: MINE_3, attack: 2, health: 1 }], motif: "bless", flavor: "Holy" },
  { recipe: "buff", events: () => [{ type: "buffed", instanceId: ENEMY, attack: -2, health: 0 }], motif: "curse", flavor: "Dark" },
  {
    recipe: "buff",
    events: () => [{ type: "upgraded", instanceId: MINE_3, defId: "core-017", change: { kind: "stats", attack: 1, health: 1 } }],
    motif: "transmute",
    flavor: "Nerd",
  },
  {
    recipe: "buff",
    events: () => [{ type: "degraded", instanceId: MINE_3, defId: "core-017", change: { kind: "stats", attack: -1, health: -1 } }],
    motif: "transmute",
    flavor: "Nerd",
  },
  // Issue #2790: `armor` and `shieldBreak` draw in the rows (R2803000 folds `ward`), but their
  // entries now carry beat callouts too, so `planFx`-equals-`planMotif` never held for them;
  // beats.test.ts pins them instead.
  { recipe: "draw", events: () => [{ type: "drawn", player: "p1", instanceId: "cY", defId: "core-055" }], motif: "flight", flavor: "Spirit" },
  { recipe: "mana", events: () => [{ type: "manaChanged", player: "p1", current: 4, max: 4 }], motif: "mana", flavor: "Holy" },
  {
    recipe: "trap",
    events: () => [{ type: "trapFired", instanceId: MY_TRAP, defId: "core-084", controller: "p1", row: "backrow", lane: 5 }],
    motif: "snap",
    flavor: "Dark",
  },
  {
    recipe: "trap",
    // R2797000: a firing plays its card's own `cast` first, so this names the card the view shows there, whose entry has none.
    events: () => [{ type: "trapFired", instanceId: MY_FIELD_TRAP, defId: "core-061", controller: "p1", row: "backrow", lane: 1, again: true }],
    motif: "snap",
    flavor: "Dark",
  },
  { recipe: "lock", events: () => [{ type: "locked", player: "p2", row: "units", lane: 2 }], motif: "bind", flavor: "Dark" },
  { recipe: "lock", events: () => [{ type: "unlocked", player: "p2", row: "units", lane: 2 }], motif: "bind", flavor: "Dark" },
];

describe("R2784000 the generic rows delegate to their motif", () => {
  for (const row of DELEGATED) {
    it(`R2784000 ${row.recipe} plans ${row.motif} in ${row.flavor} at the intensity`, () => {
      const events = row.events();
      const view = withEvents(fullBoardView(), events);
      const { entry, env } = planed(events, view);
      const spec: MotifSpec = { motif: row.motif, look: FLAVOR_LOOKS[row.flavor], scale: env.intensity, row: "replace" };
      const fresh = planed(events, view);
      expect(planFx(entry, view, env)).toEqual(planMotif(spec, fresh.entry, view, fresh.env));
      expect(planFx(entry, view, env).length, "the delegation must draw something").toBeGreaterThan(0);
    });
  }

  it("R2784000 a buff that nets to zero plans nothing, by the row and by the motif", () => {
    const events: GameEvent[] = [{ type: "buffed", instanceId: MINE_3, attack: 2, health: -2 }];
    const view = withEvents(fullBoardView(), events);
    const { entry, env } = planed(events, view);
    expect(planFx(entry, view, env)).toEqual([]);
    const fresh = planed(events, view);
    expect(planMotif({ motif: "bless", look: FLAVOR_LOOKS.Holy, scale: fresh.env.intensity, row: "replace" }, fresh.entry, view, fresh.env)).toEqual([]);
  });
});

describe("R202 the hidden seats", () => {
  /** The fixture board from the other seat: p2 viewing, p1's hand a set of backs. */
  function otherSeat(): { view: PlayerView; hand: string[] } {
    const p1 = fullBoardView();
    const hand = Array.isArray(p1.you.hand) ? p1.you.hand.map((card) => card.instanceId) : [];
    return { view: { ...p1, viewer: "p2", you: p1.opponent, opponent: p1.you }, hand };
  }

  /** Plans every entry of `events` in order, as FxLayer does: remember, then plan. */
  function planLayer(events: readonly GameEvent[], view: PlayerView): FxCue[] {
    const seen = withEvents(view, [...events]);
    const env = envOf();
    const cues: FxCue[] = [];
    for (const planned of planEntries(events, seen, false)) {
      const entry = { ...planned, durationMs: ANIMATIONS[planned.type].durationMs };
      env.memory.remember(entry.events);
      cues.push(...planMotif({ motif: "tithe", look: FLAVOR_LOOKS.Nerd, scale: env.intensity, row: "replace" }, entry, seen, env));
    }
    return cues;
  }

  it("R202 tithe from the other seat glints by count on backs and names no card", () => {
    const { view, hand } = otherSeat();
    const cues = planLayer(
      [
        { type: "cardPlayed", player: "p1", instanceId: "fx-discount", defId: "core-002", costPaid: 1 },
        { type: "costChanged", instanceId: must(hand[0], "a hand card"), cost: 0, hiddenFrom: ["p2"] },
        { type: "costChanged", instanceId: must(hand[1], "a hand card"), cost: 1, hiddenFrom: ["p2"] },
        { type: "costChanged", instanceId: "unseen", cost: 2 },
      ],
      view,
    );
    const bursts = cues.flatMap((cue) => (cue.kind === "burst" ? [cue] : []));
    expect(bursts).toHaveLength(3);
    for (const cue of bursts) {
      expect(cue.preset).toBe("arcane");
      expect(cue.at).toEqual({ kind: "handCard", side: "opponent", pick: expect.any(Number) });
    }
    const picks = bursts.flatMap((cue) => (cue.kind === "burst" && cue.at.kind === "handCard" ? [cue.at.pick] : []));
    expect(picks).toEqual([0, 1, 2]);
    // One entry a change, and every entry shakes its glint.
    expect(cues.flatMap((cue) => (cue.kind === "shake" ? [cue.trauma] : []))).toEqual([FX_MOTIF_TRAUMA.tithe, FX_MOTIF_TRAUMA.tithe, FX_MOTIF_TRAUMA.tithe]);
    const text = JSON.stringify(cues);
    expect(text).not.toMatch(/core-\d{3}/);
    for (const id of [...hand, "unseen"]) expect(text.includes(id)).toBe(false);
  });

  it("R202 tithe caps its hidden glints at FX_MOTIF_TITHE_MAX", () => {
    const { view } = otherSeat();
    const cues = planLayer(
      [
        { type: "cardPlayed", player: "p1", instanceId: "fx-discount", defId: "core-002", costPaid: 1 },
        ...Array.from({ length: FX_MOTIF_TITHE_MAX + 2 }, (_, k): GameEvent => ({ type: "costChanged", instanceId: `unseen-${String(k)}`, cost: 0 })),
      ],
      view,
    );
    expect(cues.filter((cue) => cue.kind === "burst")).toHaveLength(FX_MOTIF_TITHE_MAX);
  });

  it("R154 R202 snap looks the same for every Trap until the view names the card", () => {
    const fired = (defId: string): GameEvent => ({ type: "trapFired", instanceId: "hidden", defId, controller: "p2", row: "backrow", lane: 4 });
    const planned = (defId: string): string => {
      const events = [fired(defId)];
      const seen = withEvents(fullBoardView(), events);
      const { entry, env } = planed(events, seen);
      return JSON.stringify(planMotif({ motif: "snap", look: FLAVOR_LOOKS.Dark, scale: env.intensity, row: "replace" }, entry, seen, env));
    };
    expect(planned("core-021")).toBe(planned("core-084"));
    const cues = JSON.parse(planned("core-084")) as { kind: string }[];
    expect(cues.length).toBeGreaterThan(0);
    expect(JSON.stringify(cues)).not.toMatch(/core-\d{3}|hidden/);
    // A trap the view names bursts in its own flavor instead of the default look.
    const named = [{ type: "trapFired", instanceId: MY_TRAP, defId: "core-084", controller: "p1", row: "backrow", lane: 5 }] as GameEvent[];
    const seen = withEvents(fullBoardView(), named);
    const { entry, env } = planed(named, seen);
    const readable = planMotif({ motif: "snap", look: FLAVOR_LOOKS.Dark, scale: env.intensity, row: "replace" }, entry, seen, env);
    expect(JSON.stringify(readable)).not.toBe(planned("core-084"));
    expect(JSON.stringify(readable)).toContain(FLAVOR_LOOKS.Holy.particle);
  });
});
