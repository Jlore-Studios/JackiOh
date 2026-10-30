// Lesson 4 data (SPEC §9.10, R291, R2205003); ../scripts/advanced.ts is the coach. Training Grounds
// cards only, none Legendary or Mythic, the pool hold on. The player goes second, so The Coin is
// theirs (R244). Deck order fixes this seed's deal (`scripts/lesson-deal.ts advanced`).

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "advanced",
  number: 4,
  title: "Tricks of the trade",
  summary: "The mulligan, The Coin, Radiant cards, tribes, Tribute, Death and tokens that don't last.",
  discoveries: ["mulligan", "the-coin", "radiant", "tribe", "tribute", "death", "temporary-tokens"],
  seed: "tg-advanced-10",
  humanSeat: "p2",
  poolHold: true,
  humanDeck: [
    "traininggrounds-044", // Duplicating Felinors, Rare
    "traininggrounds-050", // Glowy Jelly Bean, Rare
    "traininggrounds-015", // In a hurry, Common
    "traininggrounds-032", // Selfless Protector, Epic
    "traininggrounds-051", // Big Felinor, Rare
    "traininggrounds-054", // The Rock, Common
    "traininggrounds-023", // Quadrupling Felinors, Rare
    "traininggrounds-027", // Hopeful Jelly Bean, Rare
    "traininggrounds-031", // Level Up!, Epic
    "traininggrounds-025", // Trigor, Rare
    "traininggrounds-036", // Training Dummy, Epic
    "traininggrounds-033", // Guerilla Felinor, Epic
    "traininggrounds-030", // Floodgates, Epic
    "traininggrounds-035", // Corpse Recycler, Epic
    "traininggrounds-049", // Reoccurring Dream, Rare
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-017", // Flaming Felinor, Rare
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-008", // Clobberer, Common
  ],
  aiDeck: [
    "traininggrounds-043", // Mr. Vanilla, Common
    "traininggrounds-001", // Spikey Steven, Common
    "traininggrounds-002", // Personal Trainer, Common
    "traininggrounds-005", // Midlane Menace, Common
    "traininggrounds-006", // Chosen by the Sun, Common
    "traininggrounds-007", // Chosen by the Moon, Common
    "traininggrounds-022", // Right-Hand Felinor, Rare
    "traininggrounds-053", // Prejudiced Postdoc, Rare
    "traininggrounds-013", // Explosive Runes, Common
    "traininggrounds-019", // Extra Rations, Rare
    "traininggrounds-047", // Midrange Menace, Common
    "traininggrounds-011", // Thwomp, Common
  ],
  retryTip: "Send expensive cards back in the mulligan, play The Coin on your first turn, and let big units like The Rock do the fighting.",
};
