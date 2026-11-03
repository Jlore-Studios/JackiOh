// R2784000, R2785000: the system moments of issue #2802. Every event in the table has its
// moment, the transform family shares one transmute, questCompleted holds as the one set piece,
// hidden Market and vote cards stay hidden (R202), and chaos/rolled keep their overlays. R200:
// every cue starts inside its entry and ends within FX_MAX_TAIL_MS of it.

import type { GameEvent, PlayerView } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { ANIMATIONS, MIN_ENTRY_MS, planEntries, scaleForSpeed, setPieceEntries, setPieceHoldMs, type AnimationEntry } from "../game/animations.ts";
import { testid } from "../game/contract.ts";
import { fullBoardView, withEvents } from "../test/fixtures.ts";
import { must } from "../test/stubServer.ts";
import {
  FX_INTENSITY_SCALE,
  FX_JADE_SCALE_MAX,
  FX_JADE_SCALE_STEP,
  FX_MAX_TAIL_MS,
  FX_MOTIF_PHASES,
  FX_QUEST_HOLD_MS,
  FX_QUEST_TRAUMA,
  FX_SYSTEM_TRAUMA,
  FX_TEXT,
} from "./constants.ts";
import { planFx } from "./cues.ts";
import { FLAVOR_LOOKS } from "./flavors.ts";
import { createFxMemory } from "./memory.ts";
import { systemMomentFixture } from "./motifs/gallery.ts";
import { SYSTEM_MOMENTS, questClaim, transmuteCues, planSystemMoment, QUEST_SET_PIECE } from "./systemMoments.ts";
import type { FxCue, FxPlanEnv } from "./types.ts";

const VIEW: PlayerView = fullBoardView();

function unitId(side: "you" | "opponent", index: number): string {
  return must(VIEW[side].units[index], `${side} unit ${index}`).instanceId;
}

const MINE = unitId("you", 0);
const MINE_3 = unitId("you", 2);
const ENEMY = unitId("opponent", 0);

function envOf(over: Partial<FxPlanEnv> = {}): FxPlanEnv {
  return { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory(), ...over };
}

function entryFor(events: readonly GameEvent[], view: PlayerView, D: number): AnimationEntry {
  const entries = planEntries(events, view, false);
  expect(entries, "these events should plan exactly one entry").toHaveLength(1);
  return { ...must(entries[0], "an entry"), durationMs: D };
}

function plan(events: readonly GameEvent[], D: number, view: PlayerView = fullBoardView()): FxCue[] {
  const seen = withEvents(view, [...events]);
  const env = envOf();
  const entry = entryFor(events, seen, D);
  env.memory.remember(entry.events);
  return planFx(entry, seen, env);
}

function kinds(cues: readonly FxCue[]): string[] {
  return cues.map((cue) => cue.kind);
}

describe("R2784000 every system event has its moment", () => {
  it("R2784000 the gallery fixtures cover every moment and each plans something", () => {
    expect([...SYSTEM_MOMENTS].sort()).toEqual(
      [
        "questProgressed",
        "questCompleted",
        "jadeChanged",
        "marketCreditsChanged",
        "marketBought",
        "marketSold",
        "voteCast",
        "votedOut",
        "starscapeTravelled",
        "fled",
        "glitched",
        "awakened",
        "boardReplaced",
        "rolledBack",
        "fengShui",
        "transformed",
        "fused",
        "upgraded",
        "degraded",
      ].sort(),
    );
    for (const moment of SYSTEM_MOMENTS) {
      const events = systemMomentFixture(moment, fullBoardView());
      expect(events.length, `${moment} has a fixture`).toBeGreaterThan(0);
      const D = ANIMATIONS[events[0]?.type ?? "damage"].durationMs;
      expect(plan(events, D).length, `${moment} plans something`).toBeGreaterThan(0);
    }
  });

  it("R2784000 questProgressed ticks as tally on the tracker", () => {
    const D = ANIMATIONS.questProgressed.durationMs;
    const cues = plan([{ type: "questProgressed", player: "p1", instanceId: MINE_3, quest: "q", progress: 1, goal: 3 }], D);
    expect(kinds(cues)).toContain("burst");
    const burst = cues.find((cue) => cue.kind === "burst");
    expect(burst).toMatchObject({ preset: "sparkle", delayMs: Math.round(FX_MOTIF_PHASES.land * D) });
    expect(JSON.stringify(cues)).toContain(FLAVOR_LOOKS.Holy.colors[0]);
  });

  it("R2785000 questCompleted banners the completion and bursts the reward to the hero", () => {
    const D = ANIMATIONS.questCompleted.durationMs;
    const cues = plan([{ type: "questCompleted", player: "p1", instanceId: MINE_3, quest: "q" }], D);
    const banner = cues.find((cue) => cue.kind === "banner");
    expect(banner).toMatchObject({ text: FX_TEXT.questComplete, tone: "you" });
    expect(kinds(cues)).toContain("rays");
    expect(kinds(cues)).toContain("projectile");
    const flight = cues.find((cue) => cue.kind === "projectile");
    expect(flight).toMatchObject({ preset: "gold" });
    expect(cues.some((cue) => cue.kind === "shake")).toBe(true);
  });

  it("R961 jadeChanged ticks bigger with the value, capped", () => {
    const D = ANIMATIONS.jadeChanged.durationMs;
    const at = (value: number): FxCue[] => plan([{ type: "jadeChanged", player: "p1", value }], D);
    const low = at(0);
    const mid = at(3);
    const high = at(100);
    const burstCount = (cues: FxCue[]): number => {
      const burst = cues.find((cue) => cue.kind === "burst");
      return burst?.kind === "burst" ? burst.count : 0;
    };
    expect(burstCount(mid)).toBeGreaterThan(burstCount(low));
    expect(burstCount(high)).toBe(Math.round(8 * FX_JADE_SCALE_MAX));
    expect(mid.filter((cue) => cue.kind === "shake").length).toBeGreaterThan(0);
    expect(1 + 3 * FX_JADE_SCALE_STEP).toBeCloseTo(1.45, 6);
  });

  it("R2784000 marketCreditsChanged bursts gold and rings the hero", () => {
    const D = ANIMATIONS.marketCreditsChanged.durationMs;
    const cues = plan([{ type: "marketCreditsChanged", player: "p1", value: 5 }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["burst", "ring"]));
    expect(cues.find((cue) => cue.kind === "burst")).toMatchObject({ preset: "gold" });
  });

  it("R2784000 marketBought flies a coin to the Market and lands the card in hand", () => {
    const D = ANIMATIONS.marketBought.durationMs;
    const cues = plan([{ type: "marketBought", player: "p1", instanceId: "fx-bought", defId: "core-002", credits: 2 }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["projectile", "ghost", "burst"]));
    expect(cues.find((cue) => cue.kind === "projectile")).toMatchObject({ preset: "gold" });
    expect(cues.find((cue) => cue.kind === "ghost")).toMatchObject({
      from: { kind: "testid", testid: "marketplace-button" },
      to: { kind: "testid", testid: "hand-you" },
    });
  });

  it("R2784000 marketSold flies the card back and pays the hero", () => {
    const D = ANIMATIONS.marketSold.durationMs;
    const cues = plan([{ type: "marketSold", player: "p1", instanceId: "fx-sold", defId: "core-002", credits: 1 }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["ghost", "projectile"]));
  });

  it("R2784000 voteCast drops a ballot onto the voted hero", () => {
    const D = ANIMATIONS.voteCast.durationMs;
    const cues = plan([{ type: "voteCast", player: "p1", instanceId: MINE, defId: "core-004", votes: 1, needed: 3 }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["ghost", "burst"]));
  });

  it("R2784000 votedOut stamps the hero and keeps its banner", () => {
    const D = ANIMATIONS.votedOut.durationMs;
    const cues = plan([{ type: "votedOut", player: "p2", ballots: [] }], D);
    expect(cues.find((cue) => cue.kind === "banner")).toMatchObject({ text: FX_TEXT.votedOut });
    expect(kinds(cues)).toContain("crack");
    expect(kinds(cues)).toContain("shake");
  });

  it("R2784000 fled dashes the hero off the board under its banner", () => {
    const D = ANIMATIONS.fled.durationMs;
    const cues = plan([{ type: "fled", player: "p2", opponent: "draw" }], D);
    expect(cues.find((cue) => cue.kind === "banner")).toMatchObject({ text: FX_TEXT.escaped });
    expect(kinds(cues)).toContain("ghost");
  });

  it("R2784000 glitched splits the board and keeps its banner", () => {
    const D = ANIMATIONS.glitched.durationMs;
    const cues = plan([{ type: "glitched", player: "p1", outcome: "swap" }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["sheen", "burst", "shake", "banner"]));
    expect(cues.find((cue) => cue.kind === "burst")).toMatchObject({ preset: "prismatic" });
  });

  it("R2784000 awakened rises the card with holy rays", () => {
    const D = ANIMATIONS.awakened.durationMs;
    const cues = plan([{ type: "awakened", player: "p1", instanceId: MINE_3, defId: "core-017" }], D);
    expect(kinds(cues)).toContain("ghost");
    expect(kinds(cues)).toContain("rays");
  });

  it("R2784000 boardReplaced rings the board", () => {
    const D = ANIMATIONS.boardReplaced.durationMs;
    const cues = plan([{ type: "boardReplaced", player: "p1" }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["ring", "burst", "shake"]));
  });

  it("R2784000 rolledBack rewinds the board", () => {
    const D = ANIMATIONS.rolledBack.durationMs;
    const cues = plan([{ type: "rolledBack", player: "p1", turnsAgo: 1, sides: ["p1"] }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["sheen", "burst", "shake"]));
  });

  it("R2784000 fengShui rings the judged card calmly and keeps its banner", () => {
    const D = ANIMATIONS.fengShui.durationMs;
    const cues = plan([{ type: "fengShui", instanceId: MINE, sourceId: ENEMY, player: "p1", outcome: "positive" }], D);
    expect(cues.find((cue) => cue.kind === "banner")).toMatchObject({ text: FX_TEXT.auspicious });
    expect(kinds(cues)).toContain("ring");
    expect(kinds(cues)).not.toContain("shake");
  });

  it("R2784000 starscapeTravelled warps across the board", () => {
    const D = ANIMATIONS.starscapeTravelled.durationMs;
    const cues = plan([{ type: "starscapeTravelled", player: "p1", node: 1, reward: "draw" }], D);
    expect(kinds(cues)).toEqual(expect.arrayContaining(["shot", "burst", "ring"]));
    expect(cues.find((cue) => cue.kind === "shot")).toMatchObject({
      from: { kind: "viewport", at: { x: 0, y: 0.5 } },
      to: { kind: "viewport", at: { x: 1, y: 0.5 } },
    });
  });
});

describe("R2784000 the transform family shares one transmute", () => {
  const eventsOf = (type: GameEvent["type"]): GameEvent[] => systemMomentFixture(type as never, fullBoardView()) as GameEvent[];

  it("R2784000 transformed, fused, upgraded and degraded share transmute's beats, with rays up and a crack down", () => {
    const D = 400;
    const byType: Record<string, FxCue[]> = {};
    for (const type of ["transformed", "fused", "upgraded", "degraded"] as const) {
      byType[type] = plan(eventsOf(type), D);
      expect(byType[type]?.length, type).toBeGreaterThan(0);
    }
    const base = (byType.transformed ?? []).slice(0, 4).map((cue) => cue.kind);
    expect(base).toEqual(["burst", "sheen", "burst", "burst"]);
    for (const type of ["fused", "upgraded", "degraded"] as const) {
      expect((byType[type] ?? []).slice(0, 4).map((cue) => cue.kind), type).toEqual(base);
    }
    for (const type of ["transformed", "fused", "upgraded", "degraded"] as const) {
      const cues = byType[type] ?? [];
      expect(cues.some((cue) => cue.kind === "projectile"), `${type} draws no fuse projectile`).toBe(false);
      expect(cues.some((cue) => cue.kind === "arrows"), `${type} draws no bless/curse arrows`).toBe(false);
    }
    expect(kinds(byType.upgraded ?? [])).toContain("rays");
    expect(kinds(byType.upgraded ?? [])).toContain("ring");
    expect(kinds(byType.degraded ?? [])).toContain("crack");
    expect(kinds(byType.transformed ?? [])).not.toContain("rays");
    expect(kinds(byType.transformed ?? [])).not.toContain("crack");
  });

  it("R2784000 transmuteCues is planMotif's transmute, whatever the catalog says", () => {
    const view = fullBoardView();
    const events: GameEvent[] = [{ type: "transformed", instanceId: MINE_3, fromDefId: "core-017", toDefId: "token-sheep", newInstanceId: "c90" }];
    const seen = withEvents(view, events);
    const entry = entryFor(events, seen, 400);
    for (const card of [() => undefined, () => ({ rarity: "Mythic", attack: 9, health: 9 }) as const]) {
      const env = envOf({ card });
      env.memory.remember(entry.events);
      expect(transmuteCues(entry, seen, env)).toEqual(plan(events, 400, view));
    }
    expect(JSON.stringify(plan(events, 400))).not.toMatch(/core-017|token-sheep/);
  });
});

describe("R200 every system moment stays inside its entry", () => {
  it("R200 every system moment starts inside its entry and ends within FX_MAX_TAIL_MS of its end", () => {
    const problems: string[] = [];
    for (const moment of SYSTEM_MOMENTS) {
      const events = systemMomentFixture(moment, fullBoardView());
      if (events.length === 0) continue;
      const seen = withEvents(fullBoardView(), events);
      const env = envOf();
      const base = entryFor(events, seen, 400);
      env.memory.remember(base.events);
      for (const D of [120, 400, 1400]) {
        for (const cue of planFx({ ...base, durationMs: D }, seen, env)) {
          const where = `${moment} at D=${String(D)}: ${JSON.stringify(cue)}`;
          if (!(cue.delayMs >= 0 && cue.delayMs <= D)) problems.push(`delay outside [0, D]: ${where}`);
          if (cue.kind === "projectile" && !(cue.delayMs + cue.flightMs <= D)) problems.push(`lands after the entry ends: ${where}`);
          if (cue.kind === "shot" && !(cue.delayMs + cue.flightMs <= D)) problems.push(`lands after the entry ends: ${where}`);
          if ("durationMs" in cue && !(cue.delayMs + cue.durationMs <= D + FX_MAX_TAIL_MS)) problems.push(`outlives the tail: ${where}`);
        }
      }
    }
    expect(problems).toEqual([]);
  });
});

describe("R202 hidden Market and vote cards stay hidden", () => {
  it("R202 hidden Market and vote cards plan byte-identical cues and name no defId", () => {
    const D = 400;
    // Market lots anchor on regions, never the card: hidden and readable plan the same.
    const readableBought: GameEvent = { type: "marketBought", player: "p2", instanceId: "c80", defId: "core-090", credits: 2 };
    const hiddenBought: GameEvent = { type: "marketBought", player: "p2", instanceId: "hidden", defId: "hidden", credits: 2 };
    expect(plan([hiddenBought], D)).toEqual(plan([readableBought], D));
    const readableSold: GameEvent = { type: "marketSold", player: "p2", instanceId: "c81", defId: "core-090", credits: 1 };
    const hiddenSold: GameEvent = { type: "marketSold", player: "p2", instanceId: "hidden", defId: "hidden", credits: 1 };
    expect(plan([hiddenSold], D)).toEqual(plan([readableSold], D));
    // A ballot anchors on its card when readable and the backrow when hidden (public where it
    // stands, §10.8): the same card with another definition plans the same, and hidden names none.
    const voteA: GameEvent = { type: "voteCast", player: "p1", instanceId: MINE, defId: "core-090", votes: 1, needed: 3 };
    const voteB: GameEvent = { type: "voteCast", player: "p1", instanceId: MINE, defId: "core-001", votes: 1, needed: 3 };
    expect(plan([voteB], ANIMATIONS.voteCast.durationMs)).toEqual(plan([voteA], ANIMATIONS.voteCast.durationMs));
    const hiddenVote: GameEvent = { type: "voteCast", player: "p1", instanceId: "hidden", defId: "hidden", votes: 1, needed: 3 };
    const backrow = plan([hiddenVote], ANIMATIONS.voteCast.durationMs);
    expect(backrow.length).toBeGreaterThan(0);
    for (const cues of [plan([hiddenBought], D), plan([hiddenSold], D), plan([voteA], ANIMATIONS.voteCast.durationMs), backrow]) {
      expect(JSON.stringify(cues)).not.toMatch(/core-090|core-001/);
    }
  });
});

describe("R2785000 questCompleted holds once per action", () => {
  const quest = (instanceId = MINE_3): GameEvent => ({ type: "questCompleted", player: "p1", instanceId, quest: "q" });

  it("R2785000 questClaim asks FX_QUEST_HOLD_MS for a questCompleted and nothing otherwise", () => {
    const view = fullBoardView();
    const withQuest = entryFor([quest()], withEvents(view, [quest()]), 500);
    expect(questClaim(withQuest)).toEqual({ key: QUEST_SET_PIECE, holdMs: FX_QUEST_HOLD_MS });
    expect(QUEST_SET_PIECE).toBe("quest");
    const without = entryFor([{ type: "jadeChanged", player: "p1", value: 1 }], view, 200);
    expect(questClaim(without)).toBeNull();
  });

  it("R2785000 a quest holds once per action, scaled and capped like every set piece", () => {
    const view = fullBoardView();
    const two = setPieceEntries(planEntries([quest("a"), quest("b")], view, false), questClaim, {
      reduced: false,
      bigMoments: "full",
      speed: 1,
    });
    expect(two.map((entry) => entry.setPiece?.holdMs ?? 0)).toEqual([setPieceHoldMs(FX_QUEST_HOLD_MS, "full", 1), 0]);
    expect(setPieceHoldMs(FX_QUEST_HOLD_MS, "full", 1)).toBe(scaleForSpeed(FX_QUEST_HOLD_MS, 1));
    expect(setPieceHoldMs(FX_QUEST_HOLD_MS, "off", 1)).toBe(0);
    expect(setPieceHoldMs(FX_QUEST_HOLD_MS, "short", 1)).toBeLessThanOrEqual(600);
    // Reduced motion holds nothing, like every set piece.
    const still = setPieceEntries(planEntries([quest("a")], view, false), questClaim, {
      reduced: true,
      bigMoments: "full",
      speed: 1,
    });
    expect(still.map((entry) => entry.setPiece?.holdMs ?? 0)).toEqual([0]);
  });

  it("R2785000 the quest's hold shakes at FX_QUEST_TRAUMA and the Signatures at FX_SYSTEM_TRAUMA", () => {
    expect(FX_QUEST_TRAUMA).toBe(0.5);
    expect(FX_SYSTEM_TRAUMA).toBe(0.3);
    const D = ANIMATIONS.questCompleted.durationMs;
    const cues = plan([quest()], D);
    const shake = cues.find((cue) => cue.kind === "shake");
    expect(shake).toMatchObject({ trauma: expect.closeTo(FX_QUEST_TRAUMA, 6), delayMs: Math.round(FX_MOTIF_PHASES.land * D) });
  });
});

describe("R436 R2652001 chaos and roll keep their overlays", () => {
  it("R436 R2652001 chaosRolled and rolled keep their overlays", () => {
    const chaos: GameEvent = { type: "chaosRolled", player: "p1", instanceId: MINE_3, defId: "core-001", effects: ["fx"] };
    const chaosCues = plan([chaos], ANIMATIONS.chaosRolled.durationMs);
    expect(chaosCues.some((cue) => cue.kind === "chaos")).toBe(true);
    expect(chaosCues.some((cue) => cue.kind === "banner")).toBe(false);
    const rollEvents: GameEvent[] = [{ type: "rolled", player: "p1", instanceId: "c1", defId: "core-001", kind: "coin", sides: 2, rolls: [{ faces: [1, 0], kept: 0 }] }];
    expect(plan(rollEvents, ANIMATIONS.rolled.durationMs).some((cue) => cue.kind === "roll")).toBe(true);
  });
});

describe("R2784000 planSystemMoment replaces only its rows", () => {
  it("R2784000 questCompleted, awakened and rolledBack replace their rows; votedOut, fled, glitched and fengShui keep their banners", () => {
    const view = fullBoardView();
    const env = envOf();
    const check = (events: GameEvent[]): Set<GameEvent> => {
      const entry = entryFor(events, withEvents(view, events), 400);
      env.memory.remember(entry.events);
      return new Set(planSystemMoment(entry, withEvents(view, events), env, new Set()).replaces);
    };
    expect(check([{ type: "questCompleted", player: "p1", instanceId: MINE_3, quest: "q" }]).size).toBe(1);
    expect(check([{ type: "awakened", player: "p1", instanceId: MINE_3, defId: "core-017" }]).size).toBe(1);
    expect(check([{ type: "rolledBack", player: "p1", turnsAgo: 1, sides: ["p1"] }]).size).toBe(1);
    expect(check([{ type: "votedOut", player: "p2", ballots: [] }]).size).toBe(0);
    expect(check([{ type: "fled", player: "p2", opponent: "draw" }]).size).toBe(0);
    expect(check([{ type: "fengShui", instanceId: MINE, sourceId: ENEMY, player: "p1", outcome: "positive" }]).size).toBe(0);
    expect(testid.board).toBe("board");
    expect(MIN_ENTRY_MS).toBe(120);
  });
});
