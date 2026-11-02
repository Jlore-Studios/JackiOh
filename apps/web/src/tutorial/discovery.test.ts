// The discovery coach (coach.ts's moments, discoveries.ts's Field Guide), on hand-built views and
// fixture scripts: R2205000; past its turns a moment retires and the lesson's end names what it set up
// (R314). Each Field Guide entry is recognized from the human's view alone (R292).

import { describe, expect, it } from "vitest";

import type { ActionBody, CardDef, GameEvent, PlayerView } from "@jackioh/shared";

import zhCN from "../i18n/zh-CN.json";
import { baseView, card, emptySide, resetIds, unit } from "../test/fixtures.ts";
import {
  COACH_START,
  activeStep,
  coachAck,
  coachDisplay,
  coachHint,
  coachObserve,
  coachStart,
  hintLevel,
  missedOf,
  type CoachCtx,
  type CoachState,
  type LessonScript,
} from "./coach.ts";
import { TUTORIAL_BUBBLES_PER_MOVE_MAX, TUTORIAL_MOMENT_TURNS_MAX } from "./config.ts";
import { FIELD_GUIDE, discoveryById, type Discovery } from "./discoveries.ts";
import { moment } from "./steps.ts";
import { coachTargets } from "./targets.ts";

const VANILLA = "core-008";
/** The lesson ids in path order (§9.10). */
const LESSON_IDS: readonly string[] = ["basics", "keywords", "casting", "backrow", "finale"];

function ctx(view: PlayerView, fresh: GameEvent[] = [], legal: ActionBody[] = [{ type: "endTurn" }], aiToAct = false): CoachCtx {
  return { view, legal, fresh, aiToAct };
}

function myTurn(turn: number): PlayerView {
  return baseView({ turn, active: "p1", you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: VANILLA, cost: 1 })] }) });
}

function aiTurn(turn: number): PlayerView {
  return baseView({ turn, active: "p2" });
}

const played: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "h1", defId: VANILLA, costPaid: 1 };
const shieldPopped: GameEvent = { type: "divineShieldLost", instanceId: "u9" };

const GUIDE: readonly Discovery[] = [
  { id: "x", name: "Mana", rule: "each card costs mana, and you get one more crystal every turn.", lesson: "basics", found: (c) => c.fresh.some((event) => event.type === "cardPlayed" && event.player === c.view.viewer) },
  { id: "y", name: "Divine Shield", rule: "the first hit on a shielded unit does nothing but pop the shield.", lesson: "keywords", found: (c) => c.fresh.some((event) => event.type === "divineShieldLost") },
];

function scriptOf(over: Partial<LessonScript> = {}): LessonScript {
  return {
    lessonId: "t",
    guide: GUIDE,
    teaches: ["x"],
    tips: [],
    steps: [
      moment({
        id: "ask",
        title: "What can you afford?",
        text: "You have 1 mana crystal. What can you afford?",
        discovers: ["x"],
        hints: ["Your cards show their cost in the corner.", "Mr. Vanilla costs 1: drag it onto a lane."],
        hintAnchor: { kind: "handCard", defId: VANILLA },
        expect: (action) => action.type === "play",
      }),
    ],
    ...over,
  };
}

/** Observe the human's turns `from`, `from + 2`, … through `to`, with the AI's in between. */
function stall(script: LessonScript, start: CoachState, from: number, to: number): CoachState {
  let state = start;
  for (let turn = from; turn <= to; turn += 2) {
    state = coachObserve(script, state, ctx(aiTurn(turn - 1), [], [], true));
    state = coachObserve(script, state, ctx(myTurn(turn)));
  }
  return state;
}

describe("R2205000 the discovery coach", () => {
  it("R2205000 a moment asks its question with nothing pointed at, and is done once the player finds what it sets up", () => {
    resetIds();
    const script = scriptOf();
    let state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    const display = coachDisplay(script, state, ctx(myTurn(3)));
    expect(display.mode).toBe("step");
    if (display.mode !== "step") return;
    expect(display.text).toBe("You have 1 mana crystal. What can you afford?");
    expect(display.anchor, "a question points at nothing").toBeNull();
    expect(display.ack, "a moment has no Got it: finding it moves it on").toBe(false);
    expect(display.hint).toBeNull();
    expect(display.canHint).toBe(true);
    expect(activeStep(script, state)?.id).toBe("ask");

    state = coachObserve(script, state, ctx(myTurn(3), [played]));
    expect(state.found).toEqual(["x"]);
    expect(state.outcomes.ask).toBe("done");
    const found = coachDisplay(script, state, ctx(myTurn(3)));
    expect(found.mode).toBe("discovery");
    if (found.mode !== "discovery") return;
    expect(found.title).toBe("You found Mana");
    expect(found.text, "the rule as a sentence of its own").toBe("Each card costs mana, and you get one more crystal every turn.");
    expect(found.ack).toBe(true);
    expect(found.holdAi, "a discovery never holds the AI").toBe(false);
    state = coachAck(script, state, ctx(myTurn(3)));
    expect(coachDisplay(script, state, ctx(myTurn(3))).mode).toBe("finished");
  });

  it("R2205000 a moment whose mechanic was found before it showed is done without showing", () => {
    const script = scriptOf();
    const state = coachObserve(script, COACH_START, ctx(aiTurn(2), [played], [], true));
    expect(state.outcomes.ask).toBe("done");
    expect(state.discoveryQueue).toEqual(["x"]);
  });

  it("R2205000 hints escalate a turn at a time while the player stalls, and one step per Hint press", () => {
    const script = scriptOf();
    let state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    expect(hintLevel(script, state)).toBe(0);

    state = stall(script, state, 5, 5);
    expect(hintLevel(script, state)).toBe(1);
    let display = coachDisplay(script, state, ctx(myTurn(5)));
    if (display.mode !== "step") throw new Error("the moment shows");
    expect(display.hint).toBe("Your cards show their cost in the corner.");
    expect(display.anchor).toBeNull();

    state = coachHint(script, state);
    display = coachDisplay(script, state, ctx(myTurn(5)));
    if (display.mode !== "step") throw new Error("the moment shows");
    expect(display.hint).toBe("Mr. Vanilla costs 1: drag it onto a lane.");
    expect(coachTargets(display.anchor, myTurn(5))).toEqual(["hand-card-h1"]);
    expect(display.canHint, "no hint is left to ask for").toBe(false);
    expect(coachHint(script, state), "a press past the last hint changes nothing").toBe(state);
  });

  it("R2205000 Hint asked for on the question itself goes to the nudge, never past the hints there are", () => {
    const script = scriptOf();
    let state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    state = coachHint(script, state);
    expect(hintLevel(script, state)).toBe(1);
    state = coachHint(script, state);
    state = coachHint(script, state);
    expect(hintLevel(script, state)).toBe(2);
  });

  it("R2205000 a moment still unfound past its turns retires as expired, and its mechanic is named as missed", () => {
    const script = scriptOf();
    let state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    // Its question's turn, then TUTORIAL_MOMENT_TURNS_MAX more of the player's own turns.
    const lastTurn = 3 + 2 * TUTORIAL_MOMENT_TURNS_MAX;
    state = stall(script, state, 5, lastTurn);
    expect(state.outcomes.ask, "still current through its turns").toBeUndefined();
    state = stall(script, state, lastTurn + 2, lastTurn + 2);
    expect(state.outcomes.ask).toBe("expired");
    expect(missedOf(script, state)).toEqual(["x"]);
  });

  it("R2205000 a moment never holds the AI, whatever its script says", () => {
    const step = { ...scriptOf().steps[0], holdAi: true } as LessonScript["steps"][number];
    const script = scriptOf({ steps: [step] });
    const state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    const display = coachDisplay(script, state, ctx(myTurn(3)));
    expect(display.mode === "step" && display.holdAi).toBe(false);
  });

  it("R2205000 every entry is watched, each found once; one the Field Guide holds bubbles only if the lesson lists it", () => {
    const script = scriptOf({ steps: [], teaches: ["x"] });
    let state = coachObserve(script, coachStart(["x", "y"]), ctx(aiTurn(2), [shieldPopped], [], true));
    expect(state.found, "found, though no moment asked for it").toEqual(["y"]);
    expect(state.discoveryQueue, "met before and not this lesson's: recorded silently").toEqual([]);
    state = coachObserve(script, state, ctx(myTurn(3), [played]));
    expect(state.discoveryQueue, "met before, but this lesson's own: named again").toEqual(["x"]);
    state = coachAck(script, state, ctx(myTurn(3)));
    state = coachObserve(script, state, ctx(myTurn(3), [played, shieldPopped]));
    expect(state.found).toEqual(["y", "x"]);
    expect(state.discoveryQueue, "each bubble shows once").toEqual([]);
  });

  it("R2434001 an entry a later lesson holds is found without a bubble, even never met; one of this lesson or an earlier one bubbles", () => {
    const first = scriptOf({ lessonId: "basics", steps: [], teaches: ["x"] });
    let state = coachObserve(first, coachStart([]), ctx(aiTurn(2), [shieldPopped], [], true));
    expect(state.found).toEqual(["y"]);
    expect(state.discoveryQueue, "lesson 2's: recorded silently in lesson 1").toEqual([]);
    for (const lessonId of ["keywords", "finale"]) {
      const later = scriptOf({ lessonId, steps: [], teaches: [] });
      state = coachObserve(later, coachStart([]), ctx(aiTurn(2), [shieldPopped], [], true));
      expect(state.discoveryQueue, lessonId).toEqual(["y"]);
    }
  });

  it("R2205000 a revisit offers an earlier lesson's mechanic only while the Field Guide lacks it", () => {
    const revisit = moment({
      id: "again",
      title: "Shields",
      text: "That unit shimmers. What happens when it's hit?",
      discovers: ["y"],
      hints: ["Attack it and watch.", "Drag a unit onto it."],
      revisit: true,
    });
    const script = scriptOf({ steps: [revisit], teaches: [] });
    expect(coachObserve(script, coachStart(["y"]), ctx(myTurn(3))).outcomes.again).toBe("moot");
    const fresh = coachObserve(script, COACH_START, ctx(myTurn(3)));
    expect(fresh.outcomes.again).toBeUndefined();
    expect(activeStep(script, fresh)?.id).toBe("again");
  });

  it("R2205000 a moment waiting for its board lets a later moment whose board has come show first", () => {
    const waits = moment({
      id: "waits",
      title: "Shields",
      text: "That unit shimmers. What happens when it's hit?",
      discovers: ["y"],
      hints: ["Attack it and watch.", "Drag a unit onto it."],
      when: () => false,
    });
    const script = scriptOf({ steps: [waits, ...scriptOf().steps] });
    let state = coachObserve(script, COACH_START, ctx(myTurn(3)));
    expect(activeStep(script, state)?.id, "the later moment shows while the first waits").toBe("ask");
    state = coachObserve(script, state, ctx(myTurn(3), [played]));
    expect(state.outcomes.ask).toBe("done");
    expect(state.outcomes.waits, "the waiting moment is still to come").toBeUndefined();
    expect(coachDisplay(script, coachAck(script, state, ctx(myTurn(3))), ctx(myTurn(3))).mode).toBe("waiting");
  });

  it("R2205000 the coach records what the game's last view shows found, then finishes", () => {
    const script = scriptOf({ steps: [] });
    const over = baseView({ turn: 9, result: { winner: "p1", reason: "hero-death" } });
    const state = coachObserve(script, COACH_START, ctx(over, [played]));
    expect(state.finished).toBe(true);
    expect(state.found).toEqual(["x"]);
    expect(state.discoveryQueue).toEqual([]);
  });

  it("R2205000 one move that trips several discoveries shows at most TUTORIAL_BUBBLES_PER_MOVE_MAX bubbles; the rest are found quietly", () => {
    const hit: GameEvent = { type: "damage", sourceId: "e1", targetId: "u1", amount: 1, combat: true };
    const many: Discovery[] = ["a", "b", "c"].map((id) => ({
      id,
      name: "Mana",
      rule: "each card costs mana, and you get one more crystal every turn.",
      lesson: "basics",
      found: (c) => c.fresh.some((event) => event.type === "damage"),
    }));
    const script = scriptOf({ steps: [], guide: [...many, ...GUIDE], teaches: [] });
    let state = coachObserve(script, COACH_START, ctx(aiTurn(2), [hit], [], true));
    expect(state.found).toEqual(["a", "b", "c"]);
    expect(state.discoveryQueue).toHaveLength(TUTORIAL_BUBBLES_PER_MOVE_MAX);
    // Acknowledged, still no move of the player's: a further find has no room.
    state = coachAck(script, coachAck(script, state, ctx(aiTurn(2))), ctx(aiTurn(2)));
    state = coachObserve(script, state, ctx(aiTurn(2), [shieldPopped], [], true));
    expect(state.found).toContain("y");
    expect(state.discoveryQueue).toEqual([]);
    // The player's move starts the count again.
    state = coachObserve(script, state, ctx(myTurn(3), [played]));
    expect(state.discoveryQueue).toEqual(["x"]);
  });

  it("R2205000 a discovery bubble comes before a tip, and Got it takes them in that order", () => {
    const script = scriptOf({
      steps: [],
      tips: [{ id: "tip", title: "A tip", text: "Read this.", when: () => true }],
    });
    let state = coachObserve(script, COACH_START, ctx(myTurn(3), [played]));
    expect(coachDisplay(script, state, ctx(myTurn(3))).mode).toBe("discovery");
    state = coachAck(script, state, ctx(myTurn(3)));
    expect(coachDisplay(script, state, ctx(myTurn(3))).mode).toBe("tip");
  });
});

/** A public definition with only the fields the Field Guide reads. */
function defOf(over: Partial<CardDef> & { text?: string; keywords?: string[] }): CardDef {
  const { text = "", keywords = [], ...rest } = over;
  const face = { keywords: keywords.map((kind) => ({ kind })), text };
  return { id: "d", index: "1", name: "D", set: "Core", type: "Unit", tags: [], rarity: "Common", token: false, cost: 1, base: face, radiant: face, ...rest } as unknown as CardDef;
}

function found(view: PlayerView, fresh: GameEvent[], defs: Record<string, CardDef> = {}): string[] {
  const context: CoachCtx = { view, legal: [], fresh, aiToAct: false, defOf: (id) => defs[id] };
  return FIELD_GUIDE.filter((entry) => entry.found(context)).map((entry) => entry.id);
}

describe("R2205000 the Field Guide", () => {
  it("R2205000 holds unique slugs, each with a name, a rule and one of the five lessons", () => {
    const ids = FIELD_GUIDE.map((entry) => entry.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const entry of FIELD_GUIDE) {
      expect(entry.id).toMatch(/^[a-z]+(-[a-z]+)*$/);
      expect(entry.name.length, entry.id).toBeGreaterThan(0);
      expect(entry.rule.length, entry.id).toBeGreaterThan(0);
      expect(LESSON_IDS, entry.id).toContain(entry.lesson);
      expect(discoveryById(entry.id)).toBe(entry);
    }
    // In lesson order, so the Field Guide reads as the path does.
    const order = FIELD_GUIDE.map((entry) => LESSON_IDS.indexOf(entry.lesson));
    expect(order).toEqual([...order].sort((a, b) => a - b));
  });

  it("R2205000 prints every entry's name and rule in zh-CN too (R711000)", () => {
    const missing = FIELD_GUIDE.flatMap((entry) => [entry.name, entry.rule]).filter((text) => !Object.hasOwn(zhCN, text));
    expect(missing).toEqual([]);
  });

  it("R2205000 recognizes each entry from the human's view and its fresh events alone", () => {
    resetIds();
    const mine = unit("p1", { instanceId: "u1", defId: "rusher", keywords: [{ kind: "Rush" }] });
    const enemy = unit("p2", { instanceId: "e1", defId: "wall", keywords: [{ kind: "Taunt" }], position: "DEF" });
    const view = baseView({ you: emptySide("p1", { units: [mine, null, null] }), opponent: emptySide("p2", { units: [enemy, null, null] }) });
    const defs = {
      rusher: defOf({ keywords: ["Rush"] }),
      wall: defOf({ keywords: ["Taunt"] }),
      crier: defOf({ text: "Cry: Deal 2 damage." }),
      bolt: defOf({ type: "Spell" }),
      crown: defOf({ type: "Spell", rarity: "Legendary" }),
    };

    expect(found(view, [{ type: "manaSpent", player: "p1", amount: 1, for: "play" } as GameEvent])).toEqual(["mana"]);
    expect(found(view, [{ type: "manaSpent", player: "p2", amount: 1, for: "play" } as GameEvent]), "the AI's mana is not yours").toEqual([]);
    expect(found(view, [{ type: "cardPlayed", player: "p1", instanceId: "s1", defId: "unknown", costPaid: 2 }]), "a play's price is mana spent").toEqual(["mana"]);
    expect(found(view, [{ type: "cardPlayed", player: "p1", instanceId: "s1", defId: "unknown", costPaid: 0 }]), "a free play spends none").toEqual([]);
    expect(found(view, [{ type: "attackDeclared", attackerId: "u1", targetId: "hero-p2", forced: false }])).toEqual(
      expect.arrayContaining(["attack-hero", "rush"]),
    );
    expect(found(view, [{ type: "attackDeclared", attackerId: "u1", targetId: "e1", forced: false }], defs)).toEqual(
      expect.arrayContaining(["trade", "taunt", "defense-position", "rush"]),
    );
    expect(found(view, [{ type: "attackDeclared", attackerId: "u1", targetId: "e1", forced: true }], defs), "a forced attack is not the player's").toEqual([]);
    expect(found(view, [shieldPopped])).toEqual(["divine-shield"]);
    expect(found(view, [{ type: "damage", sourceId: "e1", targetId: "hero-p1", amount: 3, combat: true }])).toEqual(["hero-health"]);
    expect(found(view, [{ type: "trapFired", instanceId: "t1", defId: "trap", controller: "p1", row: "backrow", lane: 0 }])).toEqual(["trap"]);
    expect(found(view, [{ type: "trapFired", instanceId: "t1", defId: "trap", controller: "p2", row: "backrow", lane: 0 }])).toEqual(["enemy-backrow"]);
    expect(found(view, [{ type: "gameOver", winner: "p1", reason: "hero-death" }])).toEqual(["winning"]);
    expect(found(view, [{ type: "gameOver", winner: "p2", reason: "hero-death" }])).toEqual([]);

    const cry: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "c1", defId: "crier", costPaid: 1 },
      { type: "damage", sourceId: "c1", targetId: "e1", amount: 2, combat: false },
    ];
    expect(found(view, cry, defs)).toEqual(expect.arrayContaining(["play-unit", "cry"]));
    expect(found(view, cry.slice(0, 1), defs), "a Cry that did nothing is not found").not.toContain("cry");
    expect(found(view, cry, {}), "without the public catalog the coach reads only the price paid").toEqual(["mana"]);

    const spell: GameEvent[] = [
      { type: "cardPlayed", player: "p1", instanceId: "s1", defId: "bolt", costPaid: 1 },
      { type: "destroyed", instanceId: "e1", defId: "wall", owner: "p2" } as GameEvent,
    ];
    expect(found(view, spell, defs)).toEqual(expect.arrayContaining(["spell", "removal"]));
    expect(found(view, [{ type: "cardPlayed", player: "p1", instanceId: "s2", defId: "crown", costPaid: 1 }], defs)).toEqual(
      expect.arrayContaining(["spell", "legendary"]),
    );
    const quick = { ...defs, draw: defOf({ type: "Spell", tags: ["Quickdraw"] }) };
    expect(found(view, [{ type: "cardPlayed", player: "p1", instanceId: "s3", defId: "draw", costPaid: 0 }], quick)).toEqual(
      expect.arrayContaining(["spell", "quickdraw"]),
    );
    expect(found(view, [{ type: "cardPlayed", player: "p1", instanceId: "s1", defId: "bolt", costPaid: 0 }], defs)).not.toContain("quickdraw");
  });

  it("R2657001 a Field Trap firing again reveals nothing, so it finds no trap entry; its first firing does", () => {
    resetIds();
    const view = baseView();
    const defs = { field: defOf({ type: "Field Trap" }) };
    const fired: GameEvent = { type: "trapFired", instanceId: "t1", defId: "field", controller: "p1", row: "backrow", lane: 0 };
    expect(found(view, [fired], defs)).toEqual(["trap", "field-trap"]);
    expect(found(view, [{ ...fired, again: true } as GameEvent], defs)).toEqual([]);
    expect(found(view, [{ ...fired, controller: "p2", again: true } as GameEvent], defs)).toEqual([]);
  });
});
