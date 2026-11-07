// R3338000, S #4.3 Event Horizon (`eventHorizon`): time bends. Over the hold the board smears toward its
// centre (board.css's "bend") under a stretched clock, and its caster's turn banner shows twice, once and
// again; the extra turn it owes opens with the board snapping back (`planTerminusFollow`). It draws its
// run (the extra turn's `modifierChanged`) in place of its own row.

import { sideOf } from "../../game/contract.ts";
import { frac } from "../build.ts";
import { FX_TEXT } from "../constants.ts";
import type { FxCue } from "../types.ts";
import { BOARD, TERMINUS_TUNING, momentPiece, type TerminusPlanner } from "./build.ts";

export const horizon: TerminusPlanner = (m) => {
  const own = m.signature.player === m.view.viewer;
  const side = sideOf(m.view, m.signature.player);
  const text = own ? FX_TEXT.yourTurn : FX_TEXT.opponentTurn;
  const marks = TERMINUS_TUNING.horizonBannersAt.map((at) => frac(at, m.S));
  const span = frac(TERMINUS_TUNING.horizonBannerSpan, m.S);
  const cues: FxCue[] = [momentPiece(m, BOARD, marks), ...marks.map((at): FxCue => ({ kind: "banner", text, tone: side, delayMs: at, durationMs: span }))];
  return { cues, replaces: new Set(m.entry.events) };
};
