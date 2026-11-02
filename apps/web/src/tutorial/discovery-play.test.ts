// The discovery coach through the REAL practice core (harness.ts), and the reckless player's choices:
// R2205000 and R2205001. A fixture script of moments runs on lesson 1's deal, apart from its own coach;
// every lesson's own suite (scripts/testkit.ts) plays each lesson under the reckless player to the human's win.

import { describe, expect, it } from "vitest";

import type { ActionBody, CardDef, PlayerView } from "@jackioh/shared";

import { baseView, card, emptySide, resetIds, unit } from "../test/fixtures.ts";
import type { CoachCtx, LessonScript } from "./coach.ts";
import { LESSON_GAME_TIMEOUT_MS, playLesson, reckless } from "./harness.ts";
import { lessonById } from "./lessons.ts";
import { TUTORIAL_BUBBLES_PER_MOVE_MAX } from "./config.ts";
import { gotItRuns } from "./scripts/testkit.ts";
import { heroTargetId, legalPlays, moment } from "./steps.ts";

const LESSON = "basics";

const lesson = lessonById(LESSON);
if (lesson === undefined) throw new Error(`no lesson "${LESSON}"`);

const MOMENTS: LessonScript = {
  lessonId: LESSON,
  teaches: ["mana", "play-unit", "attack-hero", "winning"],
  tips: [],
  steps: [
    moment({
      id: "afford",
      title: "What can you afford?",
      text: "You have 1 mana crystal. What can you afford?",
      discovers: ["mana", "play-unit"],
      hints: ["Your cards show their cost in the corner.", "Drag a card that costs 1 onto a lane."],
      when: (ctx) => ctx.legal.some((action) => action.type === "play"),
      expect: (action) => action.type === "play",
    }),
    moment({
      id: "hero",
      title: "The enemy hero",
      text: "The enemy hero has 20 health. How do you bring it down?",
      discovers: ["attack-hero"],
      hints: ["A unit that glows green is ready.", "Drag your ready unit onto the enemy hero."],
      hintAnchor: { kind: "hero", side: "opponent" },
      when: (ctx) => ctx.legal.some((action) => action.type === "attack" && action.targetId === heroTargetId(ctx.view, "opponent")),
      expect: (action, ctx) => action.type === "attack" && action.targetId === heroTargetId(ctx.view, "opponent"),
    }),
    {
      ...moment({
        id: "win",
        title: "Finish it",
        text: "How many more hits does the enemy hero take?",
        discovers: ["winning"],
        hints: ["Every unit that can attack the hero brings it closer.", "Attack the hero with everything that is ready."],
      }),
      final: true,
    },
  ],
};

describe("R2205000 a moment script through the real practice core", () => {
  it(
    "R2205000 every moment shows, each is done by what the player found, and each discovery is named once",
    () => {
      const run = playLesson(LESSON, { policy: "coach", script: MOMENTS });
      expect(run.winner, "the human wins").toBe(lesson.humanSeat);
      const shown = run.shown.map((entry) => entry.id);
      expect(shown).toEqual(["afford", "hero", "win"]);
      expect(run.outcomes.afford).toBe("done");
      expect(run.outcomes.hero).toBe("done");
      for (const id of ["mana", "play-unit", "attack-hero", "winning"]) expect(run.coach.found, id).toContain(id);
      // Each bubble once; "winning" is found on the last view, which ends the coach with no bubble.
      expect(new Set(run.discoveries).size).toBe(run.discoveries.length);
      expect(run.discoveries).toEqual(expect.arrayContaining(["mana", "play-unit", "attack-hero"]));
      expect(run.humanActions.filter((entry) => entry.refused !== null)).toEqual([]);
      // However many discoveries one move trips, the player reads at most two bubbles before moving again.
      for (const bubbles of gotItRuns(run).runs) expect(bubbles.length, bubbles.join(", ")).toBeLessThanOrEqual(TUTORIAL_BUBBLES_PER_MOVE_MAX);
    },
    LESSON_GAME_TIMEOUT_MS,
  );

  it(
    "R2205000 a player who already met every entry sees no bubble but the lesson's own",
    () => {
      const run = playLesson(LESSON, { policy: "coach", script: { ...MOMENTS, teaches: ["attack-hero"] }, known: ["mana", "play-unit", "attack-hero", "trade", "summoning-sickness", "hero-health"] });
      expect(run.winner).toBe(lesson.humanSeat);
      expect(run.discoveries.filter((id) => ["mana", "play-unit", "trade", "summoning-sickness", "hero-health"].includes(id))).toEqual([]);
      expect(run.discoveries).toContain("attack-hero");
    },
    LESSON_GAME_TIMEOUT_MS,
  );
});

describe("R2205001 the reckless player", () => {
  function def(type: CardDef["type"], keywords: string[] = []): CardDef {
    const face = { keywords: keywords.map((kind) => ({ kind })), text: "" };
    return { type, base: face, radiant: face, token: false, rarity: "Common", tags: [] } as unknown as CardDef;
  }

  function ctxOf(view: PlayerView, legal: ActionBody[], defs: Record<string, CardDef>): CoachCtx {
    return { view, legal, fresh: [], aiToAct: false, defOf: (id) => defs[id] };
  }

  it("R2205001 attacks first, each attack into the enemy unit with the most Attack, before any play", () => {
    resetIds();
    const mine = unit("p1", { instanceId: "u1", attack: 2 });
    const small = unit("p2", { instanceId: "e1", attack: 1 });
    const big = unit("p2", { instanceId: "e2", attack: 9 });
    const view = baseView({
      you: emptySide("p1", { units: [mine, null, null], hand: [card({ instanceId: "h1", defId: "unit", cost: 1 })] }),
      opponent: emptySide("p2", { units: [small, big, null] }),
    });
    const legal: ActionBody[] = [
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 1 } },
      { type: "attack", attackerId: "u1", targetId: "hero-p2" },
      { type: "attack", attackerId: "u1", targetId: "e1" },
      { type: "attack", attackerId: "u1", targetId: "e2" },
      { type: "switchPosition", instanceId: "u1" },
      { type: "endTurn" },
    ];
    expect(reckless(ctxOf(view, legal, { unit: def("Unit") }))).toEqual({ type: "attack", attackerId: "u1", targetId: "e2" });
  });

  it("R2205001 plays its cheapest card that guards nothing, never a Trap or a Taunt, Shield or Armor unit, never Defense Position", () => {
    resetIds();
    const hand = [
      card({ instanceId: "trap", defId: "trap", cost: 0 }),
      card({ instanceId: "wall", defId: "wall", cost: 1 }),
      card({ instanceId: "shield", defId: "shield", cost: 1 }),
      card({ instanceId: "plain", defId: "plain", cost: 3 }),
      card({ instanceId: "brute", defId: "brute", cost: 2 }),
    ];
    const view = baseView({ you: emptySide("p1", { hand, units: [unit("p1", { instanceId: "u1" }), null, null] }) });
    const defs = {
      trap: def("Trap"),
      wall: def("Unit", ["Taunt"]),
      shield: def("Unit", ["Divine Shield"]),
      plain: def("Unit"),
      brute: def("Unit", ["Rush"]),
    };
    const plays = hand.map((each): ActionBody => ({ type: "play", instanceId: each.instanceId, zone: { row: "units", lane: 1 } }));
    const legal: ActionBody[] = [...plays, { type: "switchPosition", instanceId: "u1" }, { type: "endTurn" }];
    expect(reckless(ctxOf(view, legal, defs))).toEqual({ type: "play", instanceId: "brute", zone: { row: "units", lane: 1 } });
    // With only protective plays left, it ends the turn rather than guard.
    const guarded = legal.filter((action) => action.type !== "play" || ["trap", "wall", "shield"].includes(action.instanceId));
    expect(reckless(ctxOf(view, guarded, defs))).toEqual({ type: "endTurn" });
    expect(legalPlays(ctxOf(view, legal, defs), "trap")).toHaveLength(1);
  });
});
