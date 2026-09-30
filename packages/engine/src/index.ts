export * from "./config";
export * from "./catalog";
export * from "./rng";
export * from "./state";
export * from "./zones";
// The read-only board queries a card script asks its questions with (BUILD M3-T1, §10.9): the read
// half of the card-facing surface, where `./effects` is the write half.
export * from "./query";
export * from "./layers";
export * from "./mana";
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
export * as effects from "./effects";
export * as subsystems from "./subsystems";
