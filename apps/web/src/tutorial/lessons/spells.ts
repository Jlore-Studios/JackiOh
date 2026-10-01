// Lesson "spells" as data (SPEC §9.10, R291): ../lessons.ts says what every field means and the
// rules every lesson keeps. Its coach script is ../scripts/spells.ts.
//
// The ORDER of each deck list is part of the deal: the seed shuffles by position, so with this seed
// the player is dealt Tempo Timmy, Mr. Vanilla and Twisted Sorcerer and then draws Lunar Eclipse,
// Deft Duelist, True Strike, Big D-fender, Hit Job, Jlockeed Shredder-10, Stockpile, Midrange
// Menace, ... — every card the coach teaches with, the turn it needs it (`lesson-deal.ts spells`
// prints the deal). Reordering a list, or swapping one card for another, deals a different game:
// re-run the lesson's tests.
//
// The player's deck is the strong one: the lesson's spells and keyword units, then big finishers.
// The AI's is deliberately weak and plain: small units and one wall (Big D-fender), a Taunt with
// Divine Shield and Reborn that it plays on its first turn (Right-house defender), a second Taunt
// and Divine Shield later (Jilliax), Lunar Eclipse and Stockpile, and nothing from the backrow
// (lesson 3) or beyond the lesson. One card of its twelve is dead weight on purpose: GIGA Glowy
// Jelly Bean costs (6) and the tutorial AI never has more than 4 mana (its cap of 3 and The Coin),
// so it is never cast and the player never sees it.
//
// Patch v0.1.1 made Mr. Vanilla a 4/4 and changed what the AI does with it, so the AI's list was
// reordered, no card changed: Jilliax comes up before Mr. Vanilla, so the AI's second Taunt stands
// on turn 4, when the coach asks the player to clear the way, and Gravedigger opens in its hand in
// Stockpile's place. Patch v0.1.1's sweep took Right-house defender off the AI's shadow ban and put
// none of this list on it, so the lesson names no banned card (R291).
//
// Patch v0.2.0 made Hit Job cost (3), and turn 4's 4 mana (MAX_MANA) no longer holds it beside Deft
// Duelist, so the player's list swapped Hit Job and Big D-fender, no card changed: turn 4 draws Big
// D-fender, which shares that turn's mana with Deft Duelist's Charge, and turn 5 draws Hit Job, which
// destroys the Taunt the AI has put in the way by then. The same patch made Prem Panther draw only
// after it attacks and survives (R426), so it swapped places with Stockpile: it now sits near the
// bottom of the deal, where neither the coach's line nor the autopilot's reaches it, and no line
// hangs on when it draws.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "spells",
  number: 2,
  title: "Spells and keywords",
  summary: "Cast spells at targets, and learn what Taunt, Rush, Charge and friends do.",
  mechanics: ["Spells and targets", "Cry", "Taunt", "Divine Shield", "Reborn", "Rush and Charge", "First Strike", "Defense Position"],
  seed: "tutorial-spells-367887",
  humanSeat: "p1",
  humanDeck: [
    "core-008", // Mr. Vanilla
    "core-011", // Tempo Timmy
    "core-035", // Lunar Eclipse
    "core-044", // True Strike
    "core-016", // Hit Job
    "core-068", // Twisted Sorcerer
    "core-001", // Big D-fender
    "core-012", // Duplicating Felinors
    "core-003", // Right-house defender
    "core-045", // Deft Duelist
    "core-019", // Midrange Menace
    "core-013", // Jlockeed Shredder-10
    "core-020", // Pointmaster
    "core-037", // Gravedigger
    "core-056", // Jilliax
    "core-032", // Prem Panther
    "core-025", // 4-mana 7/7
    "core-053", // Reno
    "core-005", // Stockpile
    "core-077", // Professor Curvature
  ],
  aiDeck: [
    "core-003", // Right-house defender
    "core-037", // Gravedigger
    "core-029", // GIGA Glowy Jelly Bean: costs (6), above the AI's 3 (+ The Coin), so it is never cast
    "core-015", // Me and Mr Token
    "core-008", // Mr. Vanilla
    "core-035", // Lunar Eclipse
    "core-030", // Archivist
    "core-077", // Professor Curvature
    "core-045", // Deft Duelist
    "core-005", // Stockpile
    "core-001", // Big D-fender
    "core-056", // Jilliax
  ],
  retryTip: "Break a Divine Shield with a small hit before a big one, and clear Taunts with spells, so your units can reach the hero.",
};
