// Lesson 1's coach (SPEC §9.10, R2205000): a welcome, then moments that ask rather than tell. The
// player finds mana and playing a unit, that a unit rests the turn it arrives, that any ready unit can
// hit the enemy hero, and what two units do when they fight; the AI's own hits teach hero health, and
// the last moment ends with the win. Every pick reads the view and the legal actions (CLAUDE.md rule 7).

import type { CoachCtx, LessonScript } from "../coach.ts";
import { info, myHand, unitsOf } from "../steps.ts";
import { endTurnPick, finishOf, hitHero, momentOf, tradeUnits, type Pick } from "./kit.ts";

/** The cheapest unit the player may play now. */
const cheapestUnit: Pick = (ctx) => {
  const units = myHand(ctx.view).filter((card) => (card.attack ?? -1) >= 0);
  units.sort((a, b) => a.cost - b.cost);
  for (const card of units) {
    const play = ctx.legal.find((action) => action.type === "play" && action.instanceId === card.instanceId);
    if (play !== undefined) return play;
  }
  return undefined;
};

/** A unit of the player's is resting, and none can attack yet. */
function resting(ctx: CoachCtx): boolean {
  return unitsOf(ctx.view, "you").some((unit) => unit.sick === true) && !ctx.legal.some((action) => action.type === "attack");
}

export const script: LessonScript = {
  lessonId: "basics",
  teaches: ["mana", "play-unit", "summoning-sickness", "attack-hero", "trade", "hero-health", "winning"],
  tips: [],
  steps: [
    info({
      id: "welcome",
      title: "Welcome to the Training Grounds",
      text: "Your hero is at the bottom and the enemy's is at the top. Bring the enemy hero from 20 health to 0 and you win. You can't lose here, so try things out.",
    }),
    momentOf({
      id: "afford",
      title: "What can you afford?",
      text: "You have 1 mana crystal. Which of your cards can you afford?",
      discovers: ["mana", "play-unit"],
      hints: ["Every card shows its cost in its top corner.", "Drag a (1) Cost card onto one of your lanes, or click it and then a lane."],
      pick: cheapestUnit,
    }),
    momentOf({
      id: "resting",
      title: "Ready to fight?",
      text: "Your unit is on the field. Can it attack this turn?",
      discovers: ["summoning-sickness"],
      hints: ["A unit that can attack glows green. Does yours?", "A unit rests on the turn it arrives. Press End turn and look again on your next turn."],
      pick: endTurnPick,
      when: resting,
    }),
    momentOf({
      id: "hero",
      title: "The enemy hero",
      text: "The enemy hero has 20 health. How do you bring it down?",
      discovers: ["attack-hero"],
      hints: ["Your ready units glow green. Where can they go?", "Drag a green unit onto the enemy hero."],
      pick: hitHero,
    }),
    momentOf({
      id: "trade",
      title: "Two units meet",
      text: "An enemy unit stands in your way. What happens when two units fight?",
      discovers: ["trade"],
      hints: ["Compare the two numbers at the bottom of each card: Attack, then Health.", "Drag your unit onto an enemy unit it can beat, and watch both."],
      pick: tradeUnits,
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "The enemy hero is wearing down. How many more hits will it take?",
      hints: ["Play what you can, then send every ready unit at the hero.", "Units that can hit the hero glow green: drag each one onto it."],
    }),
  ],
};
