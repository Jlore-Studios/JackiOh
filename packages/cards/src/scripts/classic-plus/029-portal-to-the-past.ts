// C+ #29 Portal to the Past (SPEC §8.7 row 29, R417, R564). (3) Spell.
//   Base:    Discover a card from the board your last game ended with; it arrives costing (0).
//   Radiant: {cards} different random cards of that board straight to your hand, each costing (0).
// The board is the caster's last board, a setup input frozen into the match (B5 E30,
// `subsystems/lastBoards`); an empty one (hotseat, a first game) gives nothing (R129).

import { param, type Script } from "@jackioh/engine";
import { LAST_BOARD_CARD_COST, addFromLastBoard, addRandomFromLastBoard, discoverFromLastBoard } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-029");

/** The step the Discover's answer re-enters (§10.6). */
const PICKED = "picked";

export const base: Script = {
  cry: () => [discoverFromLastBoard({ step: PICKED })],
  resume: { [PICKED]: () => [addFromLastBoard({ costOverride: LAST_BOARD_CARD_COST })] },
};

export const radiant: Script = {
  cry: (ctx) => [addRandomFromLastBoard({ count: param(ctx, "cards"), costOverride: LAST_BOARD_CARD_COST })],
};
