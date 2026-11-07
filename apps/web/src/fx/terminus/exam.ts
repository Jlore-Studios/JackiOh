// R3338000, S #4.2 KY's Last Exam (`lastExam`): the impossible problem chalks itself across the board
// over the hold, line by line; the right answer cracks through the chalk and stamps the card's Echo glyph
// on it as the hold ends, and the answer is the bolt into the enemy hero, the `bolt` motif in chalk over
// the motion after the hold, drawing the damage in place of its own row (a wrong answer deals none, and
// its problem stays chalked: nothing cracks).

import { tunedBurst } from "../build.ts";
import { paletteLook } from "../flavors.ts";
import { motifScale, type MotifSpec } from "../motifs/build.ts";
import { motifReplaces, planMotif } from "../motifs/index.ts";
import type { FxCue } from "../types.ts";
import { BOARD, TERMINUS_TUNING, beats, momentPiece, shiftCues, type TerminusPlanner } from "./build.ts";

export const exam: TerminusPlanner = (m) => {
  const chalk = paletteLook("Nerd/chalk") ?? m.look;
  const cues: FxCue[] = [
    momentPiece(m, BOARD, beats(TERMINUS_TUNING.chalkLines, m.S)),
    { ...tunedBurst(m.i, chalk.particle, BOARD, "area", m.S, TERMINUS_TUNING.chalk), colors: chalk.colors },
  ];
  const spec: MotifSpec = { motif: "bolt", look: chalk, scale: motifScale(m.signature.by, m.view, m.i), row: "replace" };
  // The bolt flies over the motion after the hold: planned over M, then moved past the hold.
  const motion = { ...m.entry, durationMs: m.M };
  cues.push(...shiftCues(planMotif(spec, motion, m.view, m.env), m.H));
  return { cues, replaces: motifReplaces(spec, motion, m.view, m.env) };
};
