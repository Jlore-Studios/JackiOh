// Lesson "advanced" as data (SPEC §9.10, R291): ../lessons.ts says what every field means and the
// rules every lesson keeps. Its coach script is ../scripts/advanced.ts.

import type { TutorialLesson } from "../lessons.ts";

export const lesson: TutorialLesson = {
  id: "advanced",
  number: 4,
  title: "Tricks of the trade",
  summary: "The mulligan, The Coin, Radiant cards, tribes, tokens, Tribute and the yellow glow.",
  mechanics: ["Mulligan", "The Coin", "Radiant", "Tribes", "Tokens", "Tribute", "Yellow glow"],
  // The human goes second, so The Coin is theirs (R244). This seed deals Jlockeed Shredder-10,
  // Felinor Fiender, Glowy Jelly Bean and the 4-mana 7/7 as the opening hand: the 7/7 is the card to
  // send back, and The Rock comes in its place. Friend of Felinors is the second draw, just in time
  // for the second turn, and Reno the third, which glows yellow while the hero is hurt; the coach
  // asks for it on the fifth turn, once The Rock is down and the mana is there. The coach's line
  // (the policies "coach" and "coach-passive") and the autopilot both win it on the seventh turn with
  // the hero never below 22, and so does a follower who picks other lanes and tokens. Scanned with
  // scripts/lesson-deal.ts.
  //
  // Patch v0.2.0 made Hit Job cost (3), and a player who plays their dearest card first then spent a
  // whole turn on it, sometimes on a 1/1, which lost a game on another deal of these decks (the
  // autopilot's deal 9, ../scripts/advanced.test.ts). Gravedigger took its place in the list: no line
  // on the lesson's own seed draws that card, so the lesson plays exactly as before, and the other
  // deals end as they did before the patch.
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
  // A light opponent: a few cheap units that keep the board busy (Me and Mr Token's Rush Token
  // thins the Felinor Tokens, so The Rock and Reno find a free zone) and a few small spells, but too
  // little to race the player while the lesson's turns go on setup. Four cards cost more than the
  // AI's mana cap (AI_TUTORIAL.manaCap, 3) and are never cast; no card here can copy, steal or
  // remove The Rock.
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
