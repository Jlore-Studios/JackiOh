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
import { LESSON_GAME_TIMEOUT_MS, playLesson } from "./harness.ts";
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

/**
 * R2205003 (§9.10): the lesson that teaches each ability a card can show on its face, so an AI deck
 * holds "nothing the lessons before it have not taught": its type, a keyword, or an ability its text
 * names. A keyword no lesson teaches is not listed (TG #14's Trample).
 */
const TYPE_LESSON: Readonly<Record<string, number>> = { Unit: 1, Spell: 2, Trap: 3, "Field Trap": 3, "Field Spell": 3 };
const KEYWORD_LESSON: Readonly<Record<string, number>> = { Taunt: 2, Armor: 2, "Divine Shield": 2, Rush: 2, Pierce: 2 };
const TEXT_LESSON: readonly (readonly [string, number])[] = [
  ["Cry:", 2],
  ["Aura:", 3],
  ["Death:", 4],
  ["Tribute", 4],
  ["Defense Position", 2],
];
/**
 * Lesson 1's AI deck is 12 distinct Training Grounds units, and the set has only three with no
 * ability (#1, #8, #43), so its other nine carry a Cry, one lesson early. Nothing else comes early.
 */
const EARLY_IN_LESSON_1: readonly string[] = ["Cry:"];
/** R2201000: all the idle human ever sends, the mulligan and a prompt's answer aside. */
const IDLE_MOVES: readonly string[] = ["mulligan", "answer", "endTurn"];

/** Every ability of a card's face with the lesson that teaches it. */
function abilitiesOf(card: CardDef): { ability: string; lesson: number }[] {
  const found: { ability: string; lesson: number }[] = [{ ability: card.type, lesson: TYPE_LESSON[card.type] ?? Infinity }];
  for (const keyword of card.base.keywords) {
    const lesson = KEYWORD_LESSON[keyword.kind];
    if (lesson !== undefined) found.push({ ability: keyword.kind, lesson });
  }
  for (const [words, lesson] of TEXT_LESSON) if (card.base.text.includes(words)) found.push({ ability: words, lesson });
  return found;
}

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

  it("R2205003 R2206001 give the AI only what this lesson or an earlier one teaches, lesson 1's Cry units aside: no Aura before lesson 3, no Death before lesson 4", () => {
    const early: string[] = [];
    const untaught: string[] = [];
    for (const lesson of TUTORIAL_LESSONS) {
      for (const id of lesson.aiDeck) {
        const card = def(id);
        for (const { ability, lesson: teaches } of abilitiesOf(card)) {
          if (teaches <= lesson.number) continue;
          if (lesson.number === 1 && teaches === SPELL_LESSON && EARLY_IN_LESSON_1.includes(ability)) early.push(id);
          else untaught.push(`${lesson.id}: ${id} ${card.name} shows ${ability}, taught in lesson ${String(teaches)}`);
        }
      }
    }
    expect(untaught).toEqual([]);
    // Only the units lesson 1 must hold for want of vanilla ones come early.
    const vanilla = Object.values(catalog).filter(
      (card) => card.set === LESSON_SET && !card.token && card.type === "Unit" && abilitiesOf(card).length === 1 && card.base.text === "",
    );
    const [basics] = TUTORIAL_LESSONS;
    if (basics === undefined) throw new Error("no lessons");
    for (const card of vanilla) expect(basics.aiDeck, `basics holds every vanilla unit first: ${card.id}`).toContain(card.id);
    expect(early.length, "the Cry units of lesson 1").toBe(AI_TUTORIAL.deckSize - vanilla.length);
  });

  for (const lesson of TUTORIAL_LESSONS) {
    it(
      `R2201000 ${lesson.id}: a human who only ends the turn still wins on its own seed and decks, once the AI's deck runs out, before the turn cap`,
      () => {
        const run = playLesson(lesson.id, { policy: "idle" });
        expect(run.winner, `${lesson.id}: the human wins`).toBe(lesson.humanSeat);
        expect(run.view.result?.reason, lesson.id).not.toBe("turn-cap");
        const moves = new Set(run.humanActions.map(({ action }) => action.type));
        for (const type of moves) expect(IDLE_MOVES, `${lesson.id}: the idle human only ends the turn`).toContain(type);
        expect(run.humanActions.filter((entry) => entry.refused !== null), lesson.id).toEqual([]);
      },
      LESSON_GAME_TIMEOUT_MS,
    );
  }

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
      expect(debug.debug.handicaps).toEqual({ [aiSeat]: handicaps.ai, [lesson.humanSeat]: handicaps.human });
      // R2433000: today's opponent decks are Training Grounds cards, so the lessons deal AI_TUTORIAL
      // without its Core-only bar until #2434 deals Core decks.
      expect(handicaps.ai).toEqual({ ...AI_TUTORIAL, coreOnly: undefined });
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
