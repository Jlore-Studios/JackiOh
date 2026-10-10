// Lesson data (SPEC §9.10, R291); ../lessons.ts defines its fields and ../scripts/advanced.ts is the coach.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "advanced",
  number: 4,
  title: "Tricks of the trade",
  summary: "The mulligan, The Coin, Radiant cards, tribes, tokens, Tribute and the yellow glow.",
  mechanics: ["Mulligan", "The Coin", "Radiant", "Tribes", "Tokens", "Tribute", "Yellow glow"],
  // The human goes second, so receives The Coin (R244). This seed exchanges the 7/7 for The Rock and
  // times Friend of Felinors and Reno for the coach; the coach and autopilot win by turn seven with
  // the hero no lower than 22 (verified with scripts/lesson-deal.ts).
  seed: "tutorial-advanced-3087",
  humanSeat: "p2",
  humanDeck: [
    "core-062", // Friend of Felinors (Common)
    "core-092", // Felinor Fiender (Legendary: the tribe's payoff)
    "core-026", // Glowy Jelly Bean (Rare)
    "core-066", // The Rock (Common)
    "core-081", // Radiant Saintess (Epic)
    "core-012", // Duplicating Felinors (Rare)
    "core-053", // Reno (Common)
    "core-025", // 4-mana 7/7 (Common)
    "core-015", // Me and Mr Token (Common)
    "core-008", // Mr. Vanilla (Common)
    "core-011", // Tempo Timmy (Common)
    "core-020", // Pointmaster (Common)
    "core-068", // Twisted Sorcerer (Common)
    "core-032", // Prem Panther (Rare)
    "core-045", // Deft Duelist (Rare)
    "core-019", // Midrange Menace (Common)
    "core-013", // Jlockeed Shredder-10 (Common)
    "core-044", // True Strike (Common)
    "core-037", // Gravedigger (Rare)
    "core-035", // Lunar Eclipse (Rare)
  ],
  // The AI applies cheap board pressure without racing the player during setup. Its 4+ mana cards are
  // uncast at AI_TUTORIAL.manaCap (3); none can copy, steal, or remove The Rock.
  aiDeck: [
    "core-008", // Mr. Vanilla
    "core-004", // Gary the Gambler
    "core-001", // Big D-fender
    "core-030", // Archivist
    "core-015", // Me and Mr Token
    "core-005", // Stockpile
    "core-010", // Rapid Replenish
    "core-072", // Reminisce
    "core-054", // Straaza: costs 4, never cast
    "core-025", // 4-mana 7/7: costs 4, never cast
    "core-029", // GIGA Glowy Jelly Bean: costs 6, never cast
    "core-014", // Jlockeed's Weapons: costs 4, never cast
  ],
  retryTip: "Send expensive cards back in the mulligan, play The Coin on your first turn, and let big units like The Rock do the fighting.",
};
