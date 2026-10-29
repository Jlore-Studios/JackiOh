// Lesson 4's coach (SPEC §9.10, R2205000): the player goes second, so the lesson starts at the
// mulligan and The Coin. Then a card turns Radiant, a tribe's payoff spares its own, a big unit asks
// for a Tribute, a Death ability fires, and tokens made for one turn are gone at its end. Picks read
// the view and the legal actions only (CLAUDE.md rule 7).

import type { ActionBody } from "@jackioh/shared";

import type { CoachCtx, LessonScript } from "../coach.ts";
import { mulliganOpen, unitsOf } from "../steps.ts";
import { attackBy, either, finishOf, isCard, momentOf, playOf, type Pick } from "./kit.ts";

const COIN = "core-t-coin";
const GLOWY_JELLY_BEAN = "traininggrounds-050";
const HOPEFUL_JELLY_BEAN = "traininggrounds-027";
const DUPLICATING_FELINORS = "traininggrounds-044";
const QUADRUPLING_FELINORS = "traininggrounds-023";
const BIG_FELINOR = "traininggrounds-051";
const THE_ROCK = "traininggrounds-054";
const TRIGOR = "traininggrounds-025";
const SELFLESS_PROTECTOR = "traininggrounds-032";
const TRAINING_DUMMY = "traininggrounds-036";
const IN_A_HURRY = "traininggrounds-015";

/** The cards this lesson's moments are about: the coach's line keeps them and swaps the rest. */
const LESSON_CARDS: ReadonlySet<string> = new Set([
  GLOWY_JELLY_BEAN,
  HOPEFUL_JELLY_BEAN,
  DUPLICATING_FELINORS,
  QUADRUPLING_FELINORS,
  BIG_FELINOR,
  THE_ROCK,
  TRIGOR,
  SELFLESS_PROTECTOR,
  TRAINING_DUMMY,
  IN_A_HURRY,
]);

/** The mulligan that keeps exactly the lesson's own cards, else the nearest the prompt offers. */
const keepTheLessonsCards: Pick = (ctx) => {
  const pending = ctx.view.pending;
  if (pending === null || !pending.forYou) return undefined;
  const wanted = new Set(pending.options.filter((option) => LESSON_CARDS.has(option.defId ?? "")).map((option) => option.key));
  const keeps = ctx.legal.filter((action): action is Extract<ActionBody, { type: "mulligan" }> => action.type === "mulligan");
  const exact = keeps.find((action) => action.keep.length === wanted.size && action.keep.every((key) => wanted.has(key)));
  keeps.sort((a, b) => b.keep.length - a.keep.length);
  return exact ?? keeps[0];
};

function felinorOut(ctx: CoachCtx): boolean {
  const felinors = new Set([DUPLICATING_FELINORS, QUADRUPLING_FELINORS]);
  return unitsOf(ctx.view, "you").some((unit) => felinors.has(unit.defId));
}

export const script: LessonScript = {
  lessonId: "advanced",
  teaches: ["mulligan", "the-coin", "radiant", "tribe", "tribute", "death", "temporary-tokens"],
  tips: [],
  steps: [
    momentOf({
      id: "mulligan",
      title: "A second chance",
      text: "Before the game starts you may swap cards from your opening hand. Will you keep this one?",
      discovers: ["mulligan"],
      hints: ["Which cards do you want in the first turns, and which can wait?", "Choose the cards to swap for new ones, then confirm."],
      pick: keepTheLessonsCards,
      when: (ctx) => mulliganOpen(ctx.view),
      prompt: true,
    }),
    momentOf({
      id: "coin",
      title: "Going second",
      text: "You go second this time, and a card called The Coin came with it. What is it for?",
      discovers: ["the-coin"],
      hints: ["Read The Coin: it costs (0).", "Play The Coin to get one more mana this turn, then spend it."],
      pick: playOf([COIN]),
    }),
    momentOf({
      id: "radiant",
      title: "A shining face",
      text: "Glowy Jelly Bean makes a card Radiant. What changes about it?",
      discovers: ["radiant"],
      hints: ["A Radiant card is the stronger face of the same card.", "Play Glowy Jelly Bean on a card in your hand, then compare it."],
      pick: playOf([GLOWY_JELLY_BEAN, HOPEFUL_JELLY_BEAN]),
    }),
    momentOf({
      id: "tribe",
      title: "One of the family",
      text: "Big Felinor destroys every unit that isn't a Felinor. Whose units would it spare?",
      discovers: ["tribe"],
      hints: ["Read the tags on your units: Felinor, Human.", "Put a Felinor on the field first, then play Big Felinor."],
      // A Felinor first, so the payoff has one of its own to spare.
      pick: (ctx) => (felinorOut(ctx) ? playOf([BIG_FELINOR])(ctx) : playOf([DUPLICATING_FELINORS, QUADRUPLING_FELINORS])(ctx)),
    }),
    momentOf({
      id: "tribute",
      title: "A price in units",
      text: "The Rock asks for a Tribute. What does it want from you?",
      discovers: ["tribute"],
      hints: ["Tribute 1 means one of your own units must go to make room for it.", "Play The Rock, giving up one of your units."],
      pick: playOf([THE_ROCK, TRIGOR]),
    }),
    momentOf({
      id: "death",
      title: "A parting gift",
      text: "Selfless Protector's text starts with Death. When does that happen?",
      discovers: ["death"],
      hints: ["Death abilities happen when their unit dies.", "Play Selfless Protector and send it into a fight it can't win."],
      pick: either(playOf([SELFLESS_PROTECTOR, TRAINING_DUMMY]), attackBy(isCard(SELFLESS_PROTECTOR)), attackBy(isCard(TRAINING_DUMMY), () => true)),
    }),
    momentOf({
      id: "hurry",
      title: "No time to waste",
      text: "In a hurry gives you units right now. How long will they stay?",
      discovers: ["temporary-tokens"],
      hints: ["Read the end of its text.", "Play In a hurry, attack with the tokens, then end your turn and watch."],
      pick: playOf([IN_A_HURRY]),
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "You have every trick now. Can you finish the enemy hero?",
      hints: ["Spend your mana every turn.", "Send every ready unit at the hero."],
    }),
  ],
};
