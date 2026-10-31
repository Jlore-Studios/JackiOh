// R437: a card's marks, drawn on the card while the view lists them (#50 K-Pop Fanatic's pending
// steal on its target, on both seats). The aura takes no pointer event, so every click still reaches
// the card; one badge per mark carries it by shape as well as color (marks.ts). It moves only under
// full motion (marks.css) and reads only the marks it is given (CLAUDE.md rule 7).
// #2270: given `bareAura` and no mark, it draws the aura alone, unmarked: the gray of a card on the
// field at its last Brittle count (game/Card.tsx).

import type { CSSProperties, ReactElement } from "react";

import type { CardMark } from "@jackioh/shared";

import { t, useLocale } from "../i18n/i18n.ts";
import { MARK_GLYPH, MARK_MOTES, markColorOf, markWords, paletteFor, type MarkColor } from "./marks.ts";
import "./marks.css";

/** The aura's testid: `marks-<instanceId>` (never `card-…`, so no board selector resolves to it). */
export function marksTestid(instanceId: string): string {
  return `marks-${instanceId}`;
}

function paletteStyle(color: string): CSSProperties {
  const palette = paletteFor(color);
  return { "--mark-rim": palette.rim, "--mark-core": palette.core, "--mark-glow": palette.glow } as CSSProperties;
}

/** The corruption aura over the card: the vignette, the crackling edge and the drifting motes (marks.css). */
function MarkAura(): ReactElement {
  return (
    <span className="card-marks__aura" aria-hidden="true">
      <span className="card-marks__vignette" />
      <span className="card-marks__edge" />
      {Array.from({ length: MARK_MOTES }, (_unused, index) => (
        <span key={index} className="card-marks__mote" data-mote={index} />
      ))}
    </span>
  );
}

export type CardMarksProps = {
  marks: readonly CardMark[];
  instanceId: string;
  /** #2270: with no mark to draw, the aura alone in this color: no badge, no words, no `data-marks`. */
  bareAura?: MarkColor;
};

export default function CardMarks({ marks, instanceId, bareAura }: CardMarksProps): ReactElement | null {
  useLocale();
  const first = marks[0];
  if (first === undefined) {
    if (bareAura === undefined) return null;
    return (
      <span className="card-marks" data-testid={marksTestid(instanceId)} data-mark-color={bareAura} style={paletteStyle(bareAura)}>
        <MarkAura />
      </span>
    );
  }
  return (
    <span
      className="card-marks"
      data-testid={marksTestid(instanceId)}
      data-marks={marks.map((entry) => entry.mark).join(" ")}
      data-mark-color={markColorOf(first.color)}
      style={paletteStyle(first.color)}
    >
      <MarkAura />
      <span className="card-marks__badges">
        {marks.map((entry, index) => {
          const words = markWords(entry.mark);
          return (
            <span
              key={`${String(index)}:${entry.mark}`}
              className="card-mark-badge"
              data-mark={entry.mark}
              data-mark-color={markColorOf(entry.color)}
              title={t(words.text)}
              style={paletteStyle(entry.color)}
            >
              <span className="card-mark-badge__glyph" aria-hidden="true">
                {MARK_GLYPH}
              </span>
              <span className="card-mark-badge__text">{t(words.text)}</span>
            </span>
          );
        })}
      </span>
    </span>
  );
}
