// #5099: the stand-ins the hotseat fixture decks deal in place of a card whose whole text is its keyword line
// (`crates/engine/src/testkit/stand_in.rs`, STAND_IN_GRAMMAR). A stand-in's id spells its cost, numbers and
// keywords, base face then Radiant, and is its name too, so `cy.playByName` and `cy.handCardByName` find it by
// its id. Each spells the card it replaced as that card stood, so every seed deals as it did; no balance patch
// moves it, so a spec that leans on a 4/4 for 1 keeps it. Only `cy.seedGame`'s hotseat engine carries the
// testkit: a fixture the server installs keeps its catalog cards (`e2e/scripts/check-fixtures.mjs`).

/** A (1) 4/4 with no text, Radiant 12/12; was #8 Mr. Vanilla. */
export const VANILLA_4_4 = "stand-in (1) 4/4 R12/12";
/** A (1) 3/3 with Rush and First Strike, Radiant 6/6 with Charge and First Strike; was #11 Tempo Timmy. */
export const FIRST_STRIKE_3_3 = "stand-in (1) 3/3 Rush, First Strike R6/6 Charge, First Strike";
/** A (2) 6/1 with Rush and First Strike, Radiant 12/2 adding Divine Shield; was #20 Pointmaster. */
export const FIRST_STRIKE_6_1 = "stand-in (2) 6/1 Rush, First Strike R12/2 Rush, First Strike, Divine Shield";
/** A (4) 7/7 with Armor 7, Radiant 14/14 adding Reborn; was #25 4-mana 7/7. */
export const ARMORED_7_7 = "stand-in (4) 7/7 Armor 7 R14/14 Armor 7, Reborn";
