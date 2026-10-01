// R437: a card's marks, drawn on the card while the view lists them (#50 K-Pop Fanatic's pending
// steal on its target, on both seats).
//
// A corruption aura in the mark's colours sits over the card: a slow shimmering vignette, a crackling
// edge and motes that drift up through it. It takes no pointer event, so every click still reaches the
// card. Beside it, one badge per mark carries the mark by shape as well as colour: a sparkle, a
// tooltip, and hidden text a screen reader reads, all from the mark → words table (marks.ts). The
// colours come from the colour key → palette table, an unknown key falling back to the default, so
// another card's mark in another colour needs one row there and nothing here.
//
// It moves only under full motion: the media query and the settings panel's "Reduce motion"
// (`<html data-reduce-motion="true">`) hold every part still, the aura and badge still drawn
// (marks.css). It reads the marks it is given and nothing else (CLAUDE.md rule 7).

import type { CSSProperties, ReactElement } from "react";

import type { CardMark } from "@jackioh/shared";

import { MARK_GLYPH, MARK_MOTES, markColorOf, markWords, paletteFor } from "./marks.ts";
import "./marks.css";

/** The aura's testid: `marks-<instanceId>` (never `card-…`, so no board selector resolves to it). */
export function marksTestid(instanceId: string): string {
  return `marks-${instanceId}`;
}

function paletteStyle(color: string): CSSProperties {
  const palette = paletteFor(color);
  return { "--mark-rim": palette.rim, "--mark-core": palette.core, "--mark-glow": palette.glow } as CSSProperties;
}

export type CardMarksProps = { marks: readonly CardMark[]; instanceId: string };

export default function CardMarks({ marks, instanceId }: CardMarksProps): ReactElement | null {
  const first = marks[0];
  if (first === undefined) return null;
  return (
    <span
      className="card-marks"
      data-testid={marksTestid(instanceId)}
      data-marks={marks.map((entry) => entry.mark).join(" ")}
      data-mark-color={markColorOf(first.color)}
      style={paletteStyle(first.color)}
    >
      <span className="card-marks__aura" aria-hidden="true">
        <span className="card-marks__vignette" />
        <span className="card-marks__edge" />
        {Array.from({ length: MARK_MOTES }, (_unused, index) => (
          <span key={index} className="card-marks__mote" data-mote={index} />
        ))}
      </span>
      <span className="card-marks__badges">
        {marks.map((entry, index) => {
          const words = markWords(entry.mark);
          return (
            <span
              key={`${String(index)}:${entry.mark}`}
              className="card-mark-badge"
              data-mark={entry.mark}
              data-mark-color={markColorOf(entry.color)}
              title={words.text}
              style={paletteStyle(entry.color)}
            >
              <span className="card-mark-badge__glyph" aria-hidden="true">
                {MARK_GLYPH}
              </span>
              <span className="card-mark-badge__text">{words.text}</span>
            </span>
          );
        })}
      </span>
    </span>
  );
}
