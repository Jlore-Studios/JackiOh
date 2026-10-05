export * from "./config";
export * from "./catalog";
export * from "./rng";
export * from "./state";
export * from "./zones";
// B3.1, R383: what an Animated card is and where it stands (field workstream).
export * from "./animated";
// The read-only board queries a card script asks its questions with (BUILD M3-T1, §10.9): the read
// half of the card-facing surface, where `./effects` is the write half.
export * from "./query";
export * from "./layers";
export * from "./mana";
// B5 E11, E12, E15 (R452–R455): the price rules and their readers (`costNow`, R396), graveyard play
// permissions, and random casts' modes.
export * from "./costRules";
export * from "./graveyardPlay";
export * from "./randomCast";
export * from "./damage";
export * from "./combat";
export * from "./draw";
export * from "./resolve";
export * from "./script";
export * from "./scripts";
export * from "./modifiers";
export * from "./stateCheck";
export * from "./setup";
export * from "./turn";
export * from "./reduce";
export * from "./playChoices";
export * from "./prompts";
export * from "./triggers";
export * from "./traps";
export * from "./viewFor";
// R310–R312: what a player may know of their own library, and the record behind it.
export * from "./ownLibrary";
// R437: the marks a card carries while an effect aimed at it waits.
export * from "./marks";
// R429: the times a card has been played, which #31 KY's Math Equation reads.
export * from "./timesPlayed";
export * from "./replay";
// Patch v0.2.0, instance data (docs/classic-sets.md B2.7, B3.3, B3.4, E39): what a card is now — its
// face's type, its tuning, its declared numbers (`param`), the numbers on it, its Brittle count and
// its enchantments — read by card scripts, `viewFor` and every rule that asks.
export * from "./faces";
export * from "./tuning";
export * from "./params";
export * from "./numbers";
export * from "./brittle";
export * from "./enchantments";
// Patch v0.2.0, damage and combat (docs/classic-sets.md B5 E5, E6, E8, E9, E35): the replacement
// windows and their declarations, and the restriction and status readers a card script asks with.
export * from "./replacements";
export * from "./restrictions";
// R42, R412: a kill credited to another unit (Classic+ #19.2); R58: whether a card is being cast on draw (Classic+ #26).
export * from "./killCredit";
export * from "./castOnDrawNow";
// Patch v0.2.0, play pipeline A: the announce window's record (B5 E1, R448) and the targeting point's
// rules (B5 E5, E9, R450), which the AI's redaction and the combat module read.
export * from "./announce";
export * from "./targeting";
export * from "./targetingPoint";
// R376: a finished game's record for the card statistics, read off its replay (§9.11).
export * from "./gameSummary";
export * as effects from "./effects";
export * as subsystems from "./subsystems";
// B5 E30, R417: last boards, a setup input; the server reads `lastBoardFor` as a game ends.
export { lastBoardFor } from "./subsystems/lastBoards";
// R677: which seat each account plays after a Glitch's swap; the server and practice read it.
export { seatPlayedBy, seatsSwapped } from "./subsystems/glitch";
