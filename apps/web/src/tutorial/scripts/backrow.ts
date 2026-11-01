// Lesson 4's coach (SPEC §9.10, R2205000, R2434001): Field Spells and Traps. The player finds that a
// Trap waits face-down and springs by itself, that a Field Trap springs and stays, that a Field Spell
// keeps working turn after turn, and that the AI's face-down cards are hidden until they spring. Picks
// read the view and the legal actions only (CLAUDE.md rule 7).

import type { LessonScript } from "../coach.ts";
import { finishOf, momentOf, playOf } from "./kit.ts";

const EXPLOSIVE_TRAP = "traininggrounds-014";
const EXPLOSIVE_RUNES = "traininggrounds-013";
const GOLIATH_SITUATION = "traininggrounds-024";
const EXTRA_RATIONS = "traininggrounds-019";
const RECRUITMENT_POSTERS = "traininggrounds-020";
const RUSH_TOKEN_FARM = "traininggrounds-060";
const CORPSE_RECYCLER = "traininggrounds-035";

export const script: LessonScript = {
  lessonId: "backrow",
  teaches: ["trap", "enemy-backrow", "field-spell", "field-trap"],
  tips: [],
  steps: [
    momentOf({
      id: "field",
      title: "A card that stays",
      text: "A Field Spell goes in your backrow and stays there. What does it do for you?",
      discovers: ["field-spell"],
      hints: ["Read the words Start of turn on it.", "Play Legacy: Rush Token Farm or Extra Rations into your backrow."],
      pick: playOf([RUSH_TOKEN_FARM, EXTRA_RATIONS, CORPSE_RECYCLER, RECRUITMENT_POSTERS]),
    }),
    momentOf({
      id: "trap",
      title: "A card face-down",
      text: "A Trap goes face-down in your backrow. When do you think it goes off?",
      discovers: ["trap"],
      hints: ["Read what it reacts to, then set it and wait.", "Play Explosive Trap into your backrow, and keep playing: it springs by itself when its moment comes."],
      pick: playOf([EXPLOSIVE_TRAP, EXPLOSIVE_RUNES]),
    }),
    momentOf({
      id: "goliath",
      title: "A trap that stays",
      text: "Goliath Situation is a Field Trap. What sets it off, and what does it leave behind?",
      discovers: ["field-trap"],
      hints: ["It reacts to the AI playing a unit.", "Set Goliath Situation in your backrow and end your turn."],
      pick: playOf([GOLIATH_SITUATION]),
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "Your backrow is working for you. Can you finish the enemy hero?",
      hints: ["Mind the AI's face-down cards: they may be Traps.", "Send every ready unit at the hero."],
    }),
  ],
};
