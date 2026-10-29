// Lesson 3 data (SPEC §9.10, R291, R2205003); ../scripts/traps.ts is the coach. Training Grounds
// cards only, none Legendary or Mythic, the pool hold on. Deck order fixes this seed's deal
// (`scripts/lesson-deal.ts traps`). The player's lanes, neighbours, Auras, backrow and tokens each
// meet a moment; the AI brings a Trap of its own, which this lesson is the first to allow (R291).

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "traps",
  number: 3,
  title: "Lanes and the backrow",
  summary: "Where a unit stands matters. Set Traps face-down, play Field Spells, and watch the AI's own backrow.",
  discoveries: ["lanes", "adjacency", "aura", "trap", "enemy-backrow", "field-spell", "field-trap", "token"],
  seed: "tg-traps-16",
  humanSeat: "p1",
  poolHold: true,
  humanDeck: [
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-014", // Explosive Trap, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-024", // Goliath Situation, Rare
    "traininggrounds-003", // Group leader, Common
    "traininggrounds-060", // Rush Token Farm, Common
    "traininggrounds-019", // Extra Rations, Rare
    "traininggrounds-021", // Flame Tongue Totem, Rare
    "traininggrounds-013", // Explosive Runes, Common
    "traininggrounds-016", // Celestial Caller, Rare
    "traininggrounds-020", // Recruitment Posters, Rare
    "traininggrounds-028", // Occidental, Epic
    "traininggrounds-029", // Oriental, Epic
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-008", // Clobberer, Common
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-011", // Thwomp, Common
    "traininggrounds-035", // Corpse Recycler, Epic
  ],
  aiDeck: [
    "traininggrounds-014", // Explosive Trap, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-053", // Prejudiced Postdoc, Rare
    "traininggrounds-019", // Extra Rations, Rare
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-047", // Midrange Menace, Common
  ],
  retryTip: "A face-down card could be a Trap. Test it with a cheap unit before you play your best one.",
};
