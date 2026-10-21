// #1443: a card's non-combat damage fires its flavor's projectile at the target, with
// `FX_FLAVOR_LIVE` on since #1444. Every plan here is `planEntries` then `planFx`, against a
// view's public facts alone (CLAUDE.md rule 7, R202).

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { card, fullBoardView, withEvents } from "../test/fixtures.ts";
import { FLAVOR_LOOKS, chaosFlavor } from "../fx/flavors.ts";
import {
  FX_FLAVOR_VOLLEY_MAX,
  FX_FOG_HIT_FROM,
  FX_FOG_HIT_TO,
  FX_INTENSITY_SCALE,
  FX_SHOT_LAUNCH_AT,
} from "../fx/constants.ts";
import { planFx } from "../fx/cues.ts";
import { createFxMemory } from "../fx/memory.ts";
import { SWEEP_MS, planEntries, type AnimationEntry } from "./animations.ts";
import { testid } from "./contract.ts";
import type { FxCue, FxPlanEnv } from "../fx/types.ts";

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  localStorage.clear();
});

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`the fixture is missing ${what}`);
  return value;
}

function dmg(sourceId: string | null, targetId: string, amount: number, combat: boolean): GameEvent {
  return { type: "damage", sourceId, targetId, amount, combat };
}

function enemyUnit(view: PlayerView, index: number): string {
  return must(view.opponent.units[index], `enemy unit ${index}`).instanceId;
}

function yourUnit(view: PlayerView, index: number): string {
  return must(view.you.units[index], `your unit ${index}`).instanceId;
}

/** A view with `instanceId`/`defId` resolving on your seat, and the damage it deals. */
function resolvingView(defId: string, instanceId: string): PlayerView {
  const view = fullBoardView();
  return { ...view, you: { ...view.you, resolving: [card({ instanceId, defId })] } };
}

/** Every entry planned for `events` against `view`, with each entry's cues in order. */
function planned(events: readonly GameEvent[], view: PlayerView): { entries: AnimationEntry[]; byEntry: FxCue[][] } {
  const shown = withEvents(view, [...events]);
  const entries = planEntries([...events], shown, false);
  const byEntry = entries.map((entry) => {
    const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    env.memory.remember(entry.events);
    return planFx(entry, shown, env);
  });
  return { entries, byEntry };
}

function shotsOf(cues: readonly FxCue[]): FxCue[] {
  return cues.filter((cue) => cue.kind === "shot");
}

function splatsOf(cues: readonly FxCue[]): FxCue[] {
  return cues.filter((cue) => cue.kind === "splat");
}

function burstsOf(cues: readonly FxCue[], preset: string): FxCue[] {
  return cues.filter((cue) => cue.kind === "burst" && cue.preset === preset);
}

function shakesOf(cues: readonly FxCue[]): FxCue[] {
  return cues.filter((cue) => cue.kind === "shake");
}

describe("#1443 the volley cap", () => {
  it("#1443 the volley cap is four", () => {
    expect(FX_FLAVOR_VOLLEY_MAX).toBe(4);
  });
});

describe("#1443 a Spell's non-combat damage fires its flavor's shot from its controller's hero", () => {
  it("#1443 Burn throws Fire from your hero", () => {
    const view = resolvingView("classic-036", "spell-1");
    const target = enemyUnit(view, 0);
    const { entries, byEntry } = planned([dmg("spell-1", target, 4, false)], view);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.events).toHaveLength(1);
    const cues = must(byEntry[0], "the entry's cues");
    const D = must(entries[0], "the entry").durationMs;
    expect(D).toBe(300);
    const launch = Math.round(FX_SHOT_LAUNCH_AT * D);
    const fire = FLAVOR_LOOKS.Fire;
    expect(shotsOf(cues)).toEqual([
      {
        kind: "shot",
        shot: fire.projectile,
        colors: fire.colors,
        preset: fire.particle,
        from: { kind: "testid", testid: testid.hero("you") },
        to: { kind: "testid", testid: testid.card(target) },
        delayMs: launch,
        flightMs: D - launch,
        density: FX_INTENSITY_SCALE.normal,
      },
    ]);
    // The number and the spark wait for the landing.
    expect(splatsOf(cues).map((cue) => cue.kind === "splat" && cue.delayMs)).toEqual([D]);
    expect(burstsOf(cues, "spark").map((cue) => cue.kind === "burst" && cue.delayMs)).toEqual([D]);
  });

  it("#1443 Book of Flame throws its book", () => {
    const view = resolvingView("classic-016", "book-1");
    const target = enemyUnit(view, 1);
    const { byEntry } = planned([dmg("book-1", target, 3, false)], view);
    const cues = must(byEntry[0], "the entry's cues");
    const nerd = FLAVOR_LOOKS.Nerd;
    expect(shotsOf(cues)).toHaveLength(1);
    expect(shotsOf(cues)[0]).toMatchObject({
      kind: "shot",
      shot: nerd.projectile,
      colors: nerd.colors,
      preset: nerd.particle,
      from: { kind: "testid", testid: testid.hero("you") },
      to: { kind: "testid", testid: testid.card(target) },
    });
  });
});

describe("#1443 a Unit's non-combat damage flies from the Unit's zone", () => {
  it("#1443 a Unit on the field fires from its card", () => {
    const view = fullBoardView();
    const source = yourUnit(view, 0);
    const target = enemyUnit(view, 0);
    const { byEntry } = planned([dmg(source, target, 4, false)], view);
    const cues = must(byEntry[0], "the entry's cues");
    const hustle = FLAVOR_LOOKS.Hustle;
    expect(shotsOf(cues)).toHaveLength(1);
    expect(shotsOf(cues)[0]).toMatchObject({
      kind: "shot",
      shot: hustle.projectile,
      colors: hustle.colors,
      preset: hustle.particle,
      from: { kind: "testid", testid: testid.card(source) },
      to: { kind: "testid", testid: testid.card(target) },
    });
  });

  it("#1443 a face-up Field Spell fires from its zone, on either seat", () => {
    const view = fullBoardView();
    const yours = must(view.you.backrow[0], "your field spell");
    const theirs = must(view.opponent.backrow[2], "the opponent's field spell");
    if (yours.faceDown !== false || theirs.faceDown !== false) throw new Error("both field spells are face-up");
    const fromYours = planned([dmg(yours.instanceId, enemyUnit(view, 0), 2, false)], view).byEntry[0];
    expect(shotsOf(must(fromYours, "your spell's cues"))[0]).toMatchObject({
      from: { kind: "testid", testid: testid.card(yours.instanceId) },
    });
    const fromTheirs = planned([dmg(theirs.instanceId, yourUnit(view, 0), 2, false)], view).byEntry[0];
    expect(shotsOf(must(fromTheirs, "their spell's cues"))[0]).toMatchObject({
      from: { kind: "testid", testid: testid.card(theirs.instanceId) },
    });
  });
});

describe("#1443 a card in the graveyard fires from its controller's hero", () => {
  it("#1443 the shot leaves your hero", () => {
    const view = fullBoardView();
    const grave = must(view.you.graveyard[0], "your graveyard card");
    const target = enemyUnit(view, 0);
    const { byEntry } = planned([dmg(grave.instanceId, target, 2, false)], view);
    const cues = must(byEntry[0], "the entry's cues");
    expect(shotsOf(cues)).toHaveLength(1);
    expect(shotsOf(cues)[0]).toMatchObject({
      from: { kind: "testid", testid: testid.hero("you") },
      to: { kind: "testid", testid: testid.card(target) },
    });
  });
});

describe("#1443 combat damage fires nothing", () => {
  it("#1443 a combat hit from a readable source plans no shot and hits at once", () => {
    const view = fullBoardView();
    const { byEntry } = planned([dmg(yourUnit(view, 0), enemyUnit(view, 0), 4, true)], view);
    const cues = must(byEntry[0], "the entry's cues");
    expect(shotsOf(cues)).toHaveLength(0);
    expect(burstsOf(cues, "spark").map((cue) => cue.kind === "burst" && cue.delayMs)).toEqual([0]);
    expect(splatsOf(cues).map((cue) => cue.kind === "splat" && cue.delayMs)).toEqual([0]);
  });
});

describe("#1443 a source the view does not show face-up fires nothing", () => {
  it.each([null, "hidden", "nope", "power-1"])("#1443 source %s hits at once with no shot", (source) => {
    const view = fullBoardView();
    const { byEntry } = planned([dmg(source, enemyUnit(view, 0), 4, false)], view);
    const cues = must(byEntry[0], "the entry's cues");
    expect(shotsOf(cues)).toHaveLength(0);
    expect(burstsOf(cues, "spark").map((cue) => cue.kind === "burst" && cue.delayMs)).toEqual([0]);
    expect(splatsOf(cues).map((cue) => cue.kind === "splat" && cue.delayMs)).toEqual([0]);
  });
});

describe("#1443 a volley starts together and caps its shots", () => {
  it("#1443 five targets hit by one source fly four shots, and all five still hit", () => {
    const view = resolvingView("classic-036", "spell-1");
    const targets = [enemyUnit(view, 0), enemyUnit(view, 1), enemyUnit(view, 2), enemyUnit(view, 3), "hero-p2"];
    const events = targets.map((target) => dmg("spell-1", target, 3, false));
    const { entries, byEntry } = planned(events, view);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.events).toHaveLength(5);
    const cues = must(byEntry[0], "the entry's cues");
    const D = must(entries[0], "the entry").durationMs;
    const launch = Math.round(FX_SHOT_LAUNCH_AT * D);
    const shots = shotsOf(cues);
    expect(shots).toHaveLength(FX_FLAVOR_VOLLEY_MAX);
    for (const shot of shots) {
      expect(shot).toMatchObject({ delayMs: launch, flightMs: D - launch });
    }
    // Every target still plays its impact, at the landing.
    expect(splatsOf(cues)).toHaveLength(5);
    for (const splat of splatsOf(cues)) {
      expect(splat).toMatchObject({ delayMs: D });
    }
  });

  it("#1443 the number and the shake wait for the landing", () => {
    const view = resolvingView("classic-036", "spell-1");
    const { entries, byEntry } = planned([dmg("spell-1", enemyUnit(view, 0), 8, false)], view);
    const cues = must(byEntry[0], "the entry's cues");
    const D = must(entries[0], "the entry").durationMs;
    expect(burstsOf(cues, "spark").map((cue) => cue.kind === "burst" && cue.delayMs)).toEqual([D]);
    expect(splatsOf(cues).map((cue) => cue.kind === "splat" && cue.delayMs)).toEqual([D]);
    expect(shakesOf(cues)).toHaveLength(1);
    expect(shakesOf(cues)[0]).toMatchObject({ delayMs: D });
  });
});

describe("#1443 a Chaos volley is uniform", () => {
  it("#1443 three targets share one projectile, colours and particle", () => {
    const view = resolvingView("core-052", "chaos-1");
    const events = [enemyUnit(view, 0), enemyUnit(view, 1), enemyUnit(view, 2)].map((target) =>
      dmg("chaos-1", target, 2, false),
    );
    const { entries, byEntry } = planned(events, view);
    expect(entries).toHaveLength(1);
    const shots = shotsOf(must(byEntry[0], "the entry's cues"));
    expect(shots).toHaveLength(3);
    const pick = FLAVOR_LOOKS[chaosFlavor("chaos-1")];
    for (const shot of shots) {
      expect(shot).toMatchObject({ shot: pick.projectile, colors: pick.colors, preset: pick.particle });
    }
  });
});

describe("#1443 a sweep volley lands each shot on its fog hit", () => {
  it("#1443 four shots at delay 0, five splats at fog times", () => {
    const view = resolvingView("classic-036", "spell-1");
    const targets = [0, 1, 2, 3, 4].map((index) => enemyUnit(view, index));
    const events = targets.map((target) => dmg("spell-1", target, 2, false));
    const { entries, byEntry } = planned(events, view);
    expect(entries).toHaveLength(1);
    expect(entries[0]?.sweep).toBeDefined();
    const D = must(entries[0], "the entry").durationMs;
    expect(D).toBe(SWEEP_MS);
    const fogHit = (k: number): number =>
      Math.round(D * (FX_FOG_HIT_FROM + (FX_FOG_HIT_TO - FX_FOG_HIT_FROM) * (k / (targets.length - 1))));
    const cues = must(byEntry[0], "the entry's cues");
    const shots = shotsOf(cues);
    expect(shots).toHaveLength(FX_FLAVOR_VOLLEY_MAX);
    shots.forEach((shot, k) => {
      expect(shot).toMatchObject({ delayMs: 0, flightMs: fogHit(k) });
    });
    expect(splatsOf(cues).map((cue) => cue.kind === "splat" && cue.delayMs)).toEqual(targets.map((_, k) => fogHit(k)));
  });
});

describe("#1443 separate sources and combat hits keep their own entries", () => {
  it("#1443 two of A, one combat of A, one of B plan three entries with at most one shot each", () => {
    const view = resolvingView("classic-036", "spell-1");
    const unitA = yourUnit(view, 0);
    const events = [
      dmg("spell-1", enemyUnit(view, 0), 2, false),
      dmg("spell-1", enemyUnit(view, 1), 2, true),
      dmg(unitA, enemyUnit(view, 2), 2, false),
    ];
    const { entries, byEntry } = planned(events, view);
    expect(entries).toHaveLength(3);
    expect(byEntry.map((cues) => shotsOf(cues).length)).toEqual([1, 0, 1]);
  });
});

describe("#1443 heals, loss, set health and absorbed damage fire nothing", () => {
  it("#1443 no shot for healed, healthLost, healthSet or damageAbsorbed", () => {
    const view = resolvingView("classic-036", "spell-1");
    const target = enemyUnit(view, 0);
    const events: GameEvent[] = [
      { type: "healed", targetId: target, amount: 3 },
      { type: "healthLost", player: "p2", amount: 3 },
      { type: "healthSet", player: "p2", health: 10, sourceId: null },
      { type: "damageAbsorbed", sourceId: "spell-1", targetId: target, absorbed: 5, combat: false },
    ];
    const { byEntry } = planned(events, view);
    for (const cues of byEntry) {
      expect(shotsOf(cues)).toHaveLength(0);
    }
  });
});

describe("#1444 a Spell's play into damage plays live", () => {
  it("#1444 the fire cast bursts and the Fire shot flies", () => {
    const view = resolvingView("classic-036", "spell-1");
    const target = enemyUnit(view, 0);
    const events: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "spell-1", defId: "classic-036", costPaid: 2 },
      dmg("spell-1", target, 4, false),
    ];
    const { entries, byEntry } = planned(events, view);
    expect(entries).toHaveLength(2);
    const playedCues = must(byEntry[0], "the play's cues");
    expect(playedCues.find((cue) => cue.kind === "burst" && cue.preset === "fire")).toMatchObject({ delayMs: 0 });
    const damageCues = must(byEntry[1], "the damage's cues");
    const fire = FLAVOR_LOOKS.Fire;
    const D = must(entries[1], "the damage entry").durationMs;
    const launch = Math.round(FX_SHOT_LAUNCH_AT * D);
    expect(shotsOf(damageCues)).toHaveLength(1);
    expect(shotsOf(damageCues)[0]).toMatchObject({
      kind: "shot",
      shot: fire.projectile,
      colors: fire.colors,
      preset: fire.particle,
      delayMs: launch,
      flightMs: D - launch,
    });
  });
});
