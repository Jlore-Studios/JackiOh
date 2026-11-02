// Lesson 5's coach (SPEC §9.10, R2205000), the final exam: Goetzmann Game Design, in the opening hand,
// finds Quickdraw, the rarest cards and a random effect at once. Picks read the view and the legal
// actions only (CLAUDE.md rule 7).

import type { LessonScript } from "../coach.ts";
import { either, finishOf, momentOf, playOf } from "./kit.ts";

const GOETZMANN_GAME_DESIGN = "traininggrounds-040";
const SEANCE = "traininggrounds-037";

export const script: LessonScript = {
  lessonId: "finale",
  teaches: ["quickdraw", "legendary", "random", "copying"],
  tips: [],
  steps: [
    momentOf({
      id: "quickdraw",
      title: "Already in hand",
      text: "Goetzmann Game Design was in your opening hand, and its frame is the rarest there is. What does it do?",
      discovers: ["quickdraw", "legendary", "random"],
      hints: ["Read its tags and its frame: Quickdraw, and Mythic.", "It costs (0): play it now and see what it gives you."],
      pick: playOf([GOETZMANN_GAME_DESIGN]),
    }),
    momentOf({
      id: "copy",
      title: "Another one",
      text: "Seance copies a card on the field. Which one is worth having twice?",
      discovers: ["copying"],
      hints: ["The copy keeps any changes the card has had.", "Play Seance on your strongest unit, or on the AI's."],
      pick: either(playOf([SEANCE], { target: "own" }), playOf([SEANCE])),
    }),
    finishOf({
      id: "finish",
      title: "The final test",
      text: "This is everything you've learned. Can you finish the enemy hero?",
      hints: ["Spend your mana every turn, and use your rarest cards well.", "Send every ready unit at the hero."],
    }),
  ],
};
