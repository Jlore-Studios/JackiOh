// Lesson 2's coach (SPEC §9.10, R2205000, R2434001): more Cries, and the keywords units fight with. No
// spell yet: lesson 3 teaches them. Picks read the view and the legal actions only (CLAUDE.md rule 7).

import type { LessonScript } from "../coach.ts";
import { unitOf } from "../steps.ts";
import { attackBy, either, finishOf, isCard, keyworded, momentOf, playOf } from "./kit.ts";

const KNIFE_JUGGLER = "traininggrounds-018";
const DUPLICATING_FELINORS = "traininggrounds-044";
const QUADRUPLING_FELINORS = "traininggrounds-023";
const PERSONAL_TRAINER = "traininggrounds-002";
const GUERILLA_FELINOR = "traininggrounds-033";
const FLAMING_FELINOR = "traininggrounds-017";
const SUN = "traininggrounds-006";
const MOON = "traininggrounds-007";
const OGRE = "traininggrounds-004";
const RAVAGER = "traininggrounds-009";

const any = (): boolean => true;

export const script: LessonScript = {
  lessonId: "keywords",
  teaches: ["cry", "taunt", "armor", "divine-shield", "rush", "pierce"],
  tips: [],
  steps: [
    momentOf({
      id: "cry",
      title: "A battle cry",
      text: "Some units do something the moment they arrive. Which of yours says so?",
      discovers: ["cry"],
      hints: ["Look for the word Cry on a card in your hand.", "Play a unit whose text starts with Cry, and watch what happens as it lands."],
      pick: playOf([DUPLICATING_FELINORS, QUADRUPLING_FELINORS, GUERILLA_FELINOR, PERSONAL_TRAINER, FLAMING_FELINOR]),
    }),
    momentOf({
      id: "wall",
      title: "A wall in the way",
      text: "That wall of Taunt is in your way. Is there a way around it, or through it?",
      discovers: ["taunt", "armor"],
      hints: ["Try to attack the hero. Where are your units allowed to go?", "Attack the Taunt unit itself, and watch its Armor take the first damage."],
      pick: either(attackBy(any, isCard(MOON)), attackBy(any, keyworded("Taunt"))),
    }),
    momentOf({
      id: "shield",
      title: "A golden glow",
      text: "Chosen by the Sun glows with a Divine Shield. What happens the first time it is hit?",
      discovers: ["divine-shield"],
      hints: ["Put it on the field, then let it fight.", "Attack with Chosen by the Sun, or let the AI hit it, and watch the shield."],
      pick: either(playOf([SUN]), attackBy(isCard(SUN))),
      when: (ctx) => unitOf(ctx.view, "opponent", SUN) === undefined,
    }),
    momentOf({
      id: "rush",
      title: "In a rush",
      text: "Some units look eager. Must they rest the turn they arrive, like the others?",
      discovers: ["rush"],
      hints: ["Read the keyword Rush on Boulder Fist Ogre or Ravager.", "Play a Rush unit, then drag it straight onto an enemy unit."],
      pick: either(playOf([OGRE, RAVAGER]), attackBy(isCard(OGRE)), attackBy(isCard(RAVAGER))),
    }),
    momentOf({
      id: "pierce",
      title: "Through the Armor",
      text: "Armor soaks up damage. Does any of your units hit straight through it?",
      discovers: ["pierce"],
      hints: ["Look for the word Pierce.", "Play Knife Juggler, then play more units and watch its knives."],
      pick: playOf([KNIFE_JUGGLER]),
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "You know your units now. How do you finish the enemy hero?",
      hints: ["Clear the Taunts first with a trade.", "Then send every ready unit at the hero."],
    }),
  ],
};
