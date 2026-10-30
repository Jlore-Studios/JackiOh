// Lesson 2's coach (SPEC §9.10, R2205000): spells and the keywords units fight with, each a moment
// the player can find for themselves. A spell clears a unit too big to fight, a Cry fires as its unit
// is played, a wall of Taunt and Armor stands in the way, a Divine Shield pops, a Rush unit attacks at
// once, a Pierce card ignores Armor, and a unit can guard in Defense Position. Picks read the view and
// the legal actions only (CLAUDE.md rule 7).

import type { LessonScript } from "../coach.ts";
import { unitOf, unitsOf } from "../steps.ts";
import { attackBy, either, finishOf, isCard, keyworded, momentOf, playOf, type Pick } from "./kit.ts";

const THWOMP = "traininggrounds-011";
const HIT_JOB = "traininggrounds-046";
const PATH_OF_EXILE = "traininggrounds-026";
const TRUE_STRIKE = "traininggrounds-058";
const FAN_OF_KNIVES = "traininggrounds-010";
const KNIFE_JUGGLER = "traininggrounds-018";
const FLAMING_FELINOR = "traininggrounds-017";
const PERSONAL_TRAINER = "traininggrounds-002";
const SUN = "traininggrounds-006";
const MOON = "traininggrounds-007";
const OGRE = "traininggrounds-004";

const any = (): boolean => true;

/** Put one of the player's units, not yet guarding, into Defense Position. */
const guard: Pick = (ctx) => {
  const ready = new Set(unitsOf(ctx.view, "you").filter((unit) => unit.position === "ATK").map((unit) => unit.instanceId));
  return ctx.legal.find((action) => action.type === "switchPosition" && ready.has(action.instanceId));
};

export const script: LessonScript = {
  lessonId: "spells",
  teaches: ["spell", "removal", "cry", "taunt", "armor", "divine-shield", "rush", "pierce", "defense-position"],
  tips: [],
  steps: [
    momentOf({
      id: "wall",
      title: "A wall in the way",
      text: "That wall of Taunt is in your way. Is there a way around it, or through it?",
      discovers: ["taunt", "armor"],
      hints: ["Try to attack the hero. Where are your units allowed to go?", "Attack the Taunt unit itself, and watch its Armor take the first damage."],
      pick: either(attackBy(any, isCard(MOON)), attackBy(any, keyworded("Taunt"))),
    }),
    momentOf({
      id: "removal",
      title: "Too big to fight",
      text: "An enemy unit stands in your way, and a fight could cost you. Is there a card that doesn't need to fight it?",
      discovers: ["spell", "removal"],
      hints: ["Not every card is a unit. Read the cards in your hand.", "Play a spell that destroys a unit, aimed at the enemy's biggest."],
      pick: playOf([HIT_JOB, PATH_OF_EXILE, THWOMP, TRUE_STRIKE], { target: "enemy" }),
    }),
    momentOf({
      id: "cry",
      title: "A battle cry",
      text: "Some units do something the moment they arrive. Which of yours says so?",
      discovers: ["cry"],
      hints: ["Look for the word Cry on a card in your hand.", "Play a unit whose text starts with Cry, and watch what happens as it lands."],
      pick: playOf([FLAMING_FELINOR, PERSONAL_TRAINER]),
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
      text: "Boulder Fist Ogre looks eager. Must it rest the turn it arrives, like the others?",
      discovers: ["rush"],
      hints: ["Read its keyword: Rush.", "Play Boulder Fist Ogre, then drag it straight onto an enemy unit."],
      pick: either(playOf([OGRE]), attackBy(isCard(OGRE))),
    }),
    momentOf({
      id: "pierce",
      title: "Through the Armor",
      text: "Armor soaks up damage. Do any of your cards hit straight through it?",
      discovers: ["pierce"],
      hints: ["Look for the word Pierce.", "Play True Strike, Fan of Knives or Knife Juggler."],
      pick: either(playOf([TRUE_STRIKE], { target: "enemy" }), playOf([FAN_OF_KNIVES, KNIFE_JUGGLER])),
    }),
    momentOf({
      id: "guard",
      title: "Hold the line",
      text: "Not every unit has to attack. Can one of yours stand guard instead?",
      discovers: ["defense-position"],
      hints: ["Click one of your units to see what else it can do.", "Switch a unit to Defense Position: it gains Taunt and 1 Armor."],
      pick: guard,
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "You know your tools now. How do you finish the enemy hero?",
      hints: ["Clear the Taunts first, with a spell or a trade.", "Then send every ready unit at the hero."],
    }),
  ],
};
