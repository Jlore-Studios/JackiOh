// The subsystems a card script leans on (BUILD M3-T7). Each one is the machinery of a card or of a
// rule too large for the effects library: Fuse (R77), the rotation rings (R14), the Zephyrs scorer
// (§10.7), the random AI policy (R44), Heroic Power (R43), Combo-Index (R27), Call to Chaos (R28)
// and the lethal projection (R44).

export * from "./fuse";
export * from "./rotation";
export * from "./scorer";
export * from "./aiPolicy";
export * from "./lethal";
export * from "./heroPower";
export * from "./comboIndex";
export * from "./callToChaos";
// B3.2, R384: Activate abilities — the `activate` action, its refusal, its listing and its view.
export * from "./activate";
export * from "./lastBoards";
// B5 E34, R416: R29's scorer choosing a whole hand (Classic+ #27 Zephrys Zealotism).
export * from "./perfectHand";
// B5 E29, R419: the board snapshots C+ #35 Rollback returns the field to.
export * from "./boardHistory";
// B5 E33, R404: quests — Classic #90 In Too Deep's count, completion and view (`Script.quests`).
export * from "./quests";
// B5 E14, R399, R545–R547: a card that has the text of the last Spell played (Classic #57 Echo).
export * from "./copiedText";
