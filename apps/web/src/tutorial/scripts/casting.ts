// Lesson 3's coach (SPEC §9.10, R2205000, R2434001): spells, and nothing else new. Picks read the view
// and the legal actions only (CLAUDE.md rule 7).

import type { LessonScript } from "../coach.ts";
import { either, finishOf, momentOf, playOf } from "./kit.ts";

const THWOMP = "traininggrounds-011";
const HIT_JOB = "traininggrounds-046";
const PATH_OF_EXILE = "traininggrounds-026";
const TRUE_STRIKE = "traininggrounds-058";
const FAN_OF_KNIVES = "traininggrounds-010";
const FORTIFY = "traininggrounds-012";
const STOCKPILE = "traininggrounds-042";
const FIG_OF_LIFE = "traininggrounds-059";
const DINO_SIZE = "traininggrounds-034";
const IN_A_HURRY = "traininggrounds-015";

export const script: LessonScript = {
  lessonId: "casting",
  teaches: ["spell", "removal"],
  tips: [],
  steps: [
    momentOf({
      id: "spell",
      title: "Not every card is a unit",
      text: "Some of your cards are spells. What does one do when you play it?",
      discovers: ["spell"],
      hints: ["A spell has no Attack or Health. Read what it says.", "Play a spell, and watch it act and go to your graveyard."],
      pick: either(
        playOf([FORTIFY, DINO_SIZE], { target: "own" }),
        playOf([STOCKPILE, FAN_OF_KNIVES, IN_A_HURRY]),
        playOf([FIG_OF_LIFE, TRUE_STRIKE, THWOMP]),
      ),
    }),
    momentOf({
      id: "removal",
      title: "Too big to fight",
      text: "An enemy unit stands in your way, and a fight could cost you. Is there a card that doesn't need to fight it?",
      discovers: ["removal"],
      hints: ["Read the spells in your hand: some destroy or damage a unit.", "Play a spell that destroys a unit, aimed at the enemy's biggest."],
      pick: playOf([HIT_JOB, PATH_OF_EXILE, THWOMP, TRUE_STRIKE], { target: "enemy" }),
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "Your spells clear the way. Can you finish the enemy hero?",
      hints: ["Spend your mana every turn, on units and spells alike.", "Send every ready unit at the hero."],
    }),
  ],
};
