// The event feed (SPEC §10.10, BUILD M5-T1): `view.events` as readable lines. A plain list,
// deliberately: the animation runner owns motion (M5-T4) and nothing here animates.
//
// The lines are written in Rust: jackioh-shared's `log` module (#3108) turns the events a view gives into
// final sentences in the reader's language, as the spans a card face's text is made of, and says which card a
// line is about when it names one the viewer may read (R97: a card the viewer may not read is never named, and
// the sentinel opens nothing). This file draws them and decides none of their words (CLAUDE.md rule 7): a line
// about a card is a button that opens that card, the face of which the face model works out (`faces.ts`), and
// the history that joins each view's window to the last (R745, useLogHistory.ts) keeps the lines it was shown.

import { Fragment, memo, useContext, useLayoutEffect, useMemo, useRef, type MouseEvent, type ReactElement } from "react";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { RulesText, useInspectTrigger } from "../cards/index.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { CatalogContext, withMatchDefs, type CardLookup } from "./catalog.ts";
import { LOG_HISTORY_LIMIT } from "./config.ts";
import { testid } from "./contract.ts";
import { namedFace } from "./faces.ts";
import { logLines, type LogLine } from "./logLines.ts";
import { useLogHistory, type KeptLine, type LoggedLine } from "./useLogHistory.ts";

export type LogProps = {
  view: PlayerView;
  /** True once a hidden log (the phone's popover) is opened, so it can scroll to its newest line. */
  revealed?: boolean;
};

/** The button inside a line that names a card; `data-def-id` says which. */
export const LOG_CARD_TESTID = "log-card";

/** The face a line opens: the card it names, as the view lists it when `listed`, else as its definition shows it. */
function faceFor(lookup: CardLookup | null, view: PlayerView, line: LogLine | null, listed: boolean): LoggedLine["face"] {
  const card = line?.card;
  if (card === undefined) return null;
  return namedFace(lookup, view, listed ? card : { defId: card.defId, radiant: card.radiant });
}

/**
 * R745: an event's line as the log keeps it (useLogHistory.ts): this view's words, and the definition's
 * face built without the instance, so it carries no instance id.
 */
function keepFrom(
  view: PlayerView,
  lookup: CardLookup | null,
  lines: readonly (LogLine | null)[],
): (event: GameEvent) => LoggedLine | null {
  // The events a history is handed are the very objects of the view's window.
  const at = new Map<GameEvent, number>(view.events.map((event, index) => [event, index]));
  return (event) => {
    const line = lines[at.get(event) ?? -1] ?? null;
    return line === null ? null : { type: line.event, text: line.text, face: faceFor(lookup, view, line, false) };
  };
}

export function keptLineFor(view: PlayerView, lookup: CardLookup | null): (event: GameEvent) => LoggedLine | null {
  return keepFrom(view, lookup, logLines(view));
}

type Line = KeptLine;

/** One line; a line about a card is a button that opens it (see the header). Kept lines are the same objects from render to render, so they do not redraw. */
const LogItem = memo(function LogItem({ line }: { line: Line }): ReactElement {
  const face = line.face;
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(face === null ? null : { key: `log-${line.key}`, face }, { showLoc: false });
  if (face === null) {
    return (
      <li className="log-line" data-event={line.type}>
        <RulesText rich={line.text} />
      </li>
    );
  }
  // A button: Enter and Space click it, so the keyboard needs nothing of its own.
  const open = (event: MouseEvent<HTMLElement>): void => {
    inspect.openSheet(event.currentTarget);
  };
  return (
    <li className="log-line" data-event={line.type} data-inspectable="true">
      <button
        {...inspect.handlers}
        type="button"
        className="log-card"
        data-testid={LOG_CARD_TESTID}
        data-def-id={face.defId}
        aria-haspopup="dialog"
        onClick={open}
      >
        <RulesText rich={line.text} />
      </button>
      {inspect.overlay}
    </li>
  );
});

export default function Log({ view, revealed = false }: LogProps): ReactElement {
  const locale = useLocale();
  // R243: a match-made card is named from the definition the view carries for it.
  const catalog = useContext(CatalogContext);
  const lookup = withMatchDefs(catalog, view.defs);
  // One call for the whole window; the faces the lines open are made once for it too.
  const lines = useMemo(() => logLines(view, locale), [view, locale]);
  const faces = useMemo(() => {
    const matchLookup = withMatchDefs(catalog, view.defs);
    return lines.map((line) => faceFor(matchLookup, view, line, true));
  }, [catalog, view, lines]);
  // R745: the lines that have left the window, and the key each window event keeps as it slides.
  const history = useLogHistory(view, keepFrom(view, lookup, lines));

  // The newest line is the one worth reading, so a log taller than its box keeps its end in view. A
  // log that was hidden (the phone's popover) has no height to scroll, so it scrolls again as it opens.
  const listRef = useRef<HTMLOListElement>(null);
  useLayoutEffect(() => {
    const list = listRef.current;
    if (list !== null) list.scrollTop = list.scrollHeight;
  }, [view.events, revealed]);

  const windowLines = lines.flatMap((line, index): Line[] => {
    if (line === null) return [];
    return [{ key: history.keys[index] ?? `${view.viewer}-window-${String(index)}`, type: line.event, text: line.text, face: faces[index] ?? null }];
  });
  const shown = [...history.kept, ...windowLines].slice(-LOG_HISTORY_LIMIT);
  return (
    <ol ref={listRef} className="log" data-testid={testid.log} aria-label={t("Game log")}>
      {shown.map((line, index) => (
        <Fragment key={line.key}>
          {/* #757: drawn only; a list holds list items, and the turn's own line names the turn. */}
          {index > 0 && line.type === "turnStarted" ? <li className="log-divider" aria-hidden="true" /> : null}
          <LogItem line={line} />
        </Fragment>
      ))}
    </ol>
  );
}
