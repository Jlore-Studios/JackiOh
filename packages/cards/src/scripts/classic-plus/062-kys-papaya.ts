// C+ #62 KY's Papaya (SPEC §8.7 row 62, R422; BUILD M9 row C+ 62). (1) Spell, Fruit, KY, Epic.
//   Base:    "Draw a curve y = ax³ + bx² + cx + d across the board, … Exile every card on the curve."
//   Radiant: "… Exile every enemy card on the curve."
//
// The curve is the engine's (`subsystems/papaya.ts`, E32): the player picks 1 to 4 cells in different
// lanes, one board-cell prompt at a time, the curve is the lowest-degree polynomial through them in
// exact rationals, and the top card at every cell on it is exiled. The running face decides the rows:
// `papayaAnswered` reads `ctx.radiant` and keeps to the enemy's rows 2 and 3 on the Radiant one.

import { subsystems, type Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-062");

export const base: Script = {
  cry: () => subsystems.papayaBegin(),
  resume: { [subsystems.PAPAYA_STEP]: subsystems.papayaAnswered },
};

// The same script: the Radiant face's "every enemy card" is the running face, read by the subsystem.
export const radiant: Script = base;
