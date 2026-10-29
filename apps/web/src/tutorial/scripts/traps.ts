// Lesson 3's coach (SPEC §9.10, R2205000): where a unit stands and what waits in the backrow. The
// player finds that the middle lane matters to Midlane Menace, that some cards care about their
// neighbours, that an Aura lasts, that a Trap springs by itself, that the AI's face-down card is a
// Trap too, that a Field Spell and a Field Trap keep working, and that a card can make tokens. Picks
// read the view and the legal actions only (CLAUDE.md rule 7).

import type { ActionBody } from "@jackioh/shared";

import type { CoachCtx, LessonScript } from "../coach.ts";
import { inHand, unitsOf } from "../steps.ts";
import { either, finishOf, momentOf, playOf, type Pick } from "./kit.ts";

const MIDLANE_MENACE = "traininggrounds-005";
const EXPLOSIVE_TRAP = "traininggrounds-014";
const EXPLOSIVE_RUNES = "traininggrounds-013";
const RIGHT_HAND_FELINOR = "traininggrounds-022";
const FLAME_TONGUE_TOTEM = "traininggrounds-021";
const CELESTIAL_CALLER = "traininggrounds-016";
const GROUP_LEADER = "traininggrounds-003";
const EXTRA_RATIONS = "traininggrounds-019";
const RECRUITMENT_POSTERS = "traininggrounds-020";
const RUSH_TOKEN_FARM = "traininggrounds-060";
const CORPSE_RECYCLER = "traininggrounds-035";
const GOLIATH_SITUATION = "traininggrounds-024";
/** §3, R685: the middle of five lanes. */
const MIDLANE = 3;

/** Right-Hand Felinor into the lane just right of one of the player's units. */
const besideAFriend: Pick = (ctx) => {
  const card = inHand(ctx.view, RIGHT_HAND_FELINOR);
  if (card === undefined) return undefined;
  const lanes = ctx.view.you.units;
  return ctx.legal.find(
    (action): action is Extract<ActionBody, { type: "play" }> =>
      action.type === "play" &&
      action.instanceId === card.instanceId &&
      action.zone?.row === "units" &&
      lanes[action.zone.lane - 2] != null,
  );
};

function hasUnit(ctx: CoachCtx): boolean {
  return unitsOf(ctx.view, "you").length > 0;
}

export const script: LessonScript = {
  lessonId: "traps",
  teaches: ["lanes", "adjacency", "aura", "trap", "enemy-backrow", "field-spell", "field-trap", "token"],
  tips: [],
  steps: [
    momentOf({
      id: "midlane",
      title: "Where to stand",
      text: "Midlane Menace looks ordinary. Does it matter where you put it?",
      discovers: ["lanes"],
      hints: ["Read its text: it cares about one lane in particular.", "Play Midlane Menace into the middle lane, the third of five."],
      pick: playOf([MIDLANE_MENACE], { lane: MIDLANE }),
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
    momentOf({
      id: "neighbours",
      title: "Good neighbours",
      text: "Right-Hand Felinor wants company. Where should it stand?",
      discovers: ["adjacency"],
      hints: ["Read its text: it cares about a unit on one side of it.", "Play it in the lane just to the right of one of your units."],
      pick: either(besideAFriend, playOf([CELESTIAL_CALLER, FLAME_TONGUE_TOTEM])),
    }),
    momentOf({
      id: "aura",
      title: "Stronger together",
      text: "Group leader makes the others better. For how long?",
      discovers: ["aura"],
      hints: ["An Aura works while its card is on the field.", "Play Group leader beside your other units, and watch their Attack."],
      pick: playOf([GROUP_LEADER]),
      when: hasUnit,
    }),
    momentOf({
      id: "field",
      title: "A card that stays",
      text: "A Field Spell goes in your backrow and stays there. What does it do each turn?",
      discovers: ["field-spell"],
      hints: ["Read the words Start of turn on it.", "Play Rush Token Farm or Extra Rations into your backrow."],
      pick: playOf([RUSH_TOKEN_FARM, EXTRA_RATIONS, RECRUITMENT_POSTERS, CORPSE_RECYCLER]),
    }),
    momentOf({
      id: "token",
      title: "Made, not drawn",
      text: "Some cards make units of their own. Where do they come from?",
      discovers: ["token"],
      hints: ["A Field Spell like Rush Token Farm works at the start of each of your turns.", "Play Rush Token Farm, then end the turn and see what arrives."],
      pick: either(playOf([RUSH_TOKEN_FARM, CELESTIAL_CALLER]), (ctx) => ctx.legal.find((action) => action.type === "endTurn")),
      when: (ctx) => inHand(ctx.view, RUSH_TOKEN_FARM) !== undefined || ctx.view.you.backrow.some((card) => card !== null && "defId" in card && card.defId === RUSH_TOKEN_FARM),
    }),
    finishOf({
      id: "finish",
      title: "Finish it",
      text: "Your backrow is working for you. Can you finish the enemy hero?",
      hints: ["Mind the AI's face-down card: it may be a Trap.", "Send every ready unit at the hero."],
    }),
  ],
};
