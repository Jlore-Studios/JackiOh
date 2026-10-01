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

// ---- v0.2.0 subsystems: Classic+ cards #40–#78 (the cards-plus-d workstream) ----

// C+ #42 KY's Test's question bank (E31, R420):
export * from "./kyTest";
// C+ #44 Simplicity Audit and #45 Complexity Audit's lines-of-code sweep (E36):
export * from "./audit";

// C+ #62 KY's Papaya's curve targeting (E32, R422):
export * from "./papaya";

// C+ #73 Call to Chaos (Classic+ Edition)'s table (R423) and #74's fusing Field Trap (R425):

// C+ #46–#61:

// C+ #63–#76:
