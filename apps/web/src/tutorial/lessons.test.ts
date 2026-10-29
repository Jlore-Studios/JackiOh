// R291 lessons are deterministic R187 practice games with the R290 handicap; the practice core exposes
// only the human's view (rule 7). R2205003: five lessons dealt from Training Grounds alone, no
// Legendary or Mythic card in either deck before the last and the human's pool hold on, and the last
// lesson's human deck holding the set's Legendary and Mythic cards.

import { describe, expect, it } from "vitest";

import { beginGame, createGame, hashState, registeredCatalog } from "@jackioh/engine";
import { AI_TUTORIAL, HERO_HEALTH, TUTORIAL_HUMAN } from "@jackioh/engine/config";
import { SHADOW_BAN_IDS } from "@jackioh/ai";
import { opponentOf, type CardDef, type PlayerId } from "@jackioh/shared";

import { PRACTICE_SEED_MAX_LENGTH } from "../practice/config.ts";
import { createPracticeCore } from "../practice/core.ts";
import type { PracticeResponse } from "../practice/protocol.ts";
import zhCN from "../i18n/zh-CN.json";
import { FIELD_GUIDE } from "./discoveries.ts";
import { TUTORIAL_LESSONS, lessonById, lessonHandicaps, nextLessonOf, type TutorialLesson } from "./lessons.ts";
import { lessonStartConfig } from "./start.ts";
import { scriptFor } from "./scripts/index.ts";

const catalog = registeredCatalog();

const SHOWPIECE_RARITIES: readonly string[] = ["Legendary", "Mythic"];
/** R2205003: the set every lesson's cards come from. */
const LESSON_SET = "Training Grounds";
/** R2205003: the lesson ids, the four kept from the Core tutorial so progress carries over, and the fifth. */
const LESSON_IDS: readonly string[] = ["basics", "spells", "traps", "advanced", "finale"];
/** R2205003: the Legendary and Mythic cards the last lesson's human holds, at the least. */
const FINALE_SHOWPIECES: readonly string[] = ["traininggrounds-037", "traininggrounds-038", "traininggrounds-039", "traininggrounds-040"];

// R291: the AI gets each mechanic only after its lesson teaches it.
const BACKROW_TYPES: readonly string[] = ["Trap", "Field Trap", "Field Spell"];
const BACKROW_LESSON = 3;
const SPELL_LESSON = 2;
// Lesson 1 excludes combat keywords (§4, §6.1).
const COMBAT_KEYWORDS: readonly string[] = [
  "Taunt",
  "Rush",
  "Charge",
  "First Strike",
  "Divine Shield",
  "Reborn",
  "Poisonous",
  "Lifesteal",
  "Trample",
  "Cleave",
  "Indestructible",
  "Stack",
  "Can't attack",
];


function def(id: string): CardDef {
  const found = catalog[id];
  if (found === undefined) throw new Error(`${id} is not in the catalog`);
  return found;
}

function decksOf(lesson: TutorialLesson): [string[], string[]] {
  return lesson.humanSeat === "p1" ? [[...lesson.humanDeck], [...lesson.aiDeck]] : [[...lesson.aiDeck], [...lesson.humanDeck]];
}

function dealOf(lesson: TutorialLesson, seed = lesson.seed) {
  const aiSeat = opponentOf(lesson.humanSeat);
  const { human, ai } = lessonHandicaps(lesson);
  return beginGame(createGame({ seed, decks: decksOf(lesson), handicaps: { [aiSeat]: ai, [lesson.humanSeat]: human } })).state;
}

describe("R291 the tutorial's lessons", () => {
  it("R291 R2205003 are the five lessons, in path order, each with a script whose last step is final", () => {
    expect(TUTORIAL_LESSONS.map((lesson) => lesson.id)).toEqual(LESSON_IDS);
    expect(TUTORIAL_LESSONS.map((lesson) => lesson.number)).toEqual(TUTORIAL_LESSONS.map((_lesson, index) => index + 1));
    expect(new Set(TUTORIAL_LESSONS.map((lesson) => lesson.id)).size).toBe(TUTORIAL_LESSONS.length);

    for (const lesson of TUTORIAL_LESSONS) {
      expect(lessonById(lesson.id)).toBe(lesson);
      expect(lesson.title.length, lesson.id).toBeGreaterThan(0);
      expect(lesson.discoveries.length, lesson.id).toBeGreaterThan(0);
      for (const id of lesson.discoveries) expect(FIELD_GUIDE.map((entry) => entry.id), `${lesson.id}: ${id}`).toContain(id);
      expect(lesson.retryTip.length, lesson.id).toBeGreaterThan(0);
      expect(lesson.seed.length, lesson.id).toBeGreaterThan(0);
      expect(lesson.seed.length, lesson.id).toBeLessThanOrEqual(PRACTICE_SEED_MAX_LENGTH);

      const script = scriptFor(lesson.id);
      expect(script, lesson.id).toBeDefined();
      if (script === undefined) continue;
      expect(script.lessonId).toBe(lesson.id);
      const ids = [...script.steps.map((step) => step.id), ...script.tips.map((tip) => tip.id)];
      expect(new Set(ids).size, `${lesson.id}: step and tip ids are unique`).toBe(ids.length);
      expect(script.steps.at(-1)?.final, `${lesson.id}: the last step ends with the game`).toBe(true);
      for (const step of script.steps) {
        if (step.kind === "act") expect(step.done, `${lesson.id}/${step.id}: an act step says when it is done`).toBeDefined();
      }
    }
    const [first] = TUTORIAL_LESSONS;
    expect(first === undefined ? undefined : nextLessonOf(first.id)).toBe(TUTORIAL_LESSONS[1]);
    expect(nextLessonOf(TUTORIAL_LESSONS.at(-1)?.id ?? "")).toBeUndefined();
  });

  it("R291 R2205003 give the human a deck of TUTORIAL_HUMAN's size from Training Grounds, its Legendary and Mythic cards only in the last lesson", () => {
    for (const lesson of TUTORIAL_LESSONS) {
      expect(lesson.humanDeck.length, lesson.id).toBe(TUTORIAL_HUMAN.deckSize);
      expect(new Set(lesson.humanDeck).size, `${lesson.id}: no duplicates (§2.6)`).toBe(TUTORIAL_HUMAN.deckSize);
      for (const id of [...lesson.humanDeck, ...lesson.aiDeck]) {
        expect(def(id).token, `${lesson.id}: ${id} is no Token`).toBe(false);
        expect(def(id).set, `${lesson.id}: ${id} is a Training Grounds card`).toBe(LESSON_SET);
      }
      const showpieces = lesson.humanDeck.filter((id) => SHOWPIECE_RARITIES.includes(def(id).rarity));
      const last = lesson.number === TUTORIAL_LESSONS.length;
      if (last) for (const id of FINALE_SHOWPIECES) expect(lesson.humanDeck, `${lesson.id} deals ${id}`).toContain(id);
      else expect(showpieces, `${lesson.id}: none before the last lesson`).toEqual([]);
      // The pool hold is on exactly where the decks hold none, so nothing random brings one in.
      expect(lesson.poolHold, `${lesson.id}: the pool hold`).toBe(!last);
      expect(lessonHandicaps(lesson).human.poolHold === true, lesson.id).toBe(!last);
    }
  });

  it("R291 give the AI exactly the tutorial handicap's deck, free of the shadow ban and of anything not yet taught", () => {
    for (const lesson of TUTORIAL_LESSONS) {
      const deck = lesson.aiDeck;
      expect(deck.length, lesson.id).toBe(AI_TUTORIAL.deckSize);
      expect(new Set(deck).size, `${lesson.id}: no duplicates`).toBe(deck.length);
      for (const id of deck) {
        const card = def(id);
        const where = `${lesson.id}: ${id} ${card.name}`;
        expect(card.token, where).toBe(false);
        if (lesson.aiShadowBanned?.[id] === undefined) expect(SHADOW_BAN_IDS, where).not.toContain(id);
        expect(SHOWPIECE_RARITIES, where).not.toContain(card.rarity);
        if (lesson.number < BACKROW_LESSON) expect(BACKROW_TYPES, `${where}: no backrow card before lesson ${String(BACKROW_LESSON)}`).not.toContain(card.type);
        if (lesson.number < SPELL_LESSON) {
          expect(card.type, `${where}: units only in lesson 1`).toBe("Unit");
          const keywords = card.base.keywords.map((keyword) => keyword.kind);
          for (const keyword of keywords) expect(COMBAT_KEYWORDS, `${where}: ${keyword}`).not.toContain(keyword);
        }
      }
    }
  });

  it("R291 name every shadow-banned card of an AI deck with its reason, and name no other", () => {
    for (const lesson of TUTORIAL_LESSONS) {
      for (const [id, reason] of Object.entries(lesson.aiShadowBanned ?? {})) {
        expect(lesson.aiDeck, `${lesson.id}: ${id} is in the AI's deck`).toContain(id);
        expect(SHADOW_BAN_IDS, `${lesson.id}: ${id} is on the shadow ban`).toContain(id);
        expect(reason.length, `${lesson.id}: ${id} says why`).toBeGreaterThan(0);
      }
    }
  });

  it("R291 deal the same opening hands and libraries every time, and a different seed deals differently", () => {
    for (const lesson of TUTORIAL_LESSONS) {
      const a = dealOf(lesson);
      const b = dealOf(lesson);
      expect(hashState(a), lesson.id).toBe(hashState(b));
      const human = lesson.humanSeat;
      expect(a.players[human].hand.map((card) => card.defId), lesson.id).toEqual(b.players[human].hand.map((card) => card.defId));
      const other = dealOf(lesson, `${lesson.seed}:other`);
      expect(hashState(other), lesson.id).not.toBe(hashState(a));
    }
  });

  it("R291 start through the practice core with the lesson's decks and the tutorial handicap, showing the page only the human's side", () => {
    for (const lesson of TUTORIAL_LESSONS) {
      const core = createPracticeCore({ now: () => 0, dev: true });
      const started: PracticeResponse = core.handle({ id: 1, type: "start", config: lessonStartConfig(lesson) });
      expect(started.type, lesson.id).toBe("started");
      if (started.type !== "started") continue;
      const aiSeat: PlayerId = opponentOf(lesson.humanSeat);
      expect(started.aiSeat).toBe(aiSeat);

      const { view } = started.snapshot;
      expect(view.viewer).toBe(lesson.humanSeat);
      expect(view.you.hero.health).toBe(HERO_HEALTH);
      expect(view.opponent.hero.health).toBe(AI_TUTORIAL.heroHealth);
      // Rule 7: the AI's hand is a count, never its cards.
      expect(Array.isArray(view.opponent.hand), lesson.id).toBe(false);

      const debug = core.handle({ id: 2, type: "debug" });
      expect(debug.type).toBe("debug");
      if (debug.type !== "debug") continue;
      expect(debug.debug.lesson).toBe(lesson.id);
      expect(debug.debug.seed).toBe(lesson.seed);
      expect(debug.debug.decks).toEqual(decksOf(lesson));
      const handicaps = lessonHandicaps(lesson);
      expect(debug.debug.handicaps).toEqual({ [aiSeat]: AI_TUTORIAL, [lesson.humanSeat]: handicaps.human });
    }
  });

  it("R291 refuse an unknown lesson rather than play some other game", () => {
    const core = createPracticeCore({ now: () => 0, dev: true });
    const [lesson] = TUTORIAL_LESSONS;
    if (lesson === undefined) throw new Error("no lessons");
    const response = core.handle({ id: 1, type: "start", config: { ...lessonStartConfig(lesson), lesson: "no-such-lesson" } });
    expect(response.type).toBe("failed");
  });

  it("R291 seat the human second in the lesson that teaches The Coin, and first before it", () => {
    for (const lesson of TUTORIAL_LESSONS) {
      const coin = lesson.discoveries.includes("the-coin");
      expect(lesson.humanSeat, lesson.id).toBe(coin ? "p2" : "p1");
    }
  });

  it("R2205003 prints every lesson's words and every line its coach can show in zh-CN too (R711000)", () => {
    const lines: string[] = [];
    for (const lesson of TUTORIAL_LESSONS) {
      lines.push(lesson.title, lesson.summary, lesson.retryTip);
      const script = scriptFor(lesson.id);
      for (const step of [...(script?.steps ?? []), ...(script?.tips ?? [])]) {
        for (const text of [step.title, step.text, ...("hints" in step ? (step.hints ?? []) : [])]) if (typeof text === "string") lines.push(text);
      }
    }
    expect(lines.filter((line) => !Object.hasOwn(zhCN, line))).toEqual([]);
  });
});
