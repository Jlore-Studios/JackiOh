// The event feed (SPEC §10.10, BUILD M5-T1): `view.events` as readable lines. A plain list,
// deliberately: the animation runner owns motion (M5-T4) and nothing here animates.
//
// The lines are written in Rust: jackioh-shared's `log` module (#3108) turns the events a view gives into
// final sentences in the reader's language, as the spans a card face's text is made of, and says which card a
// line is about when it names one the viewer may read (R97: a card the viewer may not read is never named, and
// the sentinel opens nothing). This file draws them and decides none of their words (CLAUDE.md rule 7): a line
// about a card is a button that opens that card, the face of which the face model works out (`faces.ts`), and
// the history that joins each view's window to the last (R745, useLogHistory.ts) keeps the lines it was shown.

import { Fragment, memo, useContext, useDeferredValue, useEffect, useLayoutEffect, useMemo, useRef, useState, type MouseEvent, type ReactElement } from "react";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { RulesText, useInspectTrigger } from "../cards/index.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { CatalogContext, withMatchDefs, type CardLookup } from "./catalog.ts";
import { LOG_HISTORY_LIMIT, LOG_PIN_TOLERANCE_PX } from "./config.ts";
import { testid } from "./contract.ts";
import { namedFace } from "./faces.ts";
import { logLines, type LogLine } from "./logLines.ts";
import { useLogHistory, type KeptLine, type LoggedLine } from "./useLogHistory.ts";
import { useLogWindow } from "./useLogWindow.ts";

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

/**
 * One line; a line about a card is a button that opens it (see the header). Kept lines are the
 * same objects from render to render, so they do not redraw. `value` pins the line's number
 * where rows above it are unmounted (#4614); elsewhere the list numbers it.
 */
const LogItem = memo(function LogItem({ line, value }: { line: Line; value?: number }): ReactElement {
  const face = line.face;
  // Lines of code is a hidden stat in matches.
  const inspect = useInspectTrigger(face === null ? null : { key: `log-${line.key}`, face }, { showLoc: false });
  if (face === null) {
    return (
      <li className="log-line" data-event={line.type} data-log-key={line.key} value={value}>
        <RulesText rich={line.text} />
      </li>
    );
  }
  // A button: Enter and Space click it, so the keyboard needs nothing of its own.
  const open = (event: MouseEvent<HTMLElement>): void => {
    inspect.openSheet(event.currentTarget);
  };
  return (
    <li className="log-line" data-event={line.type} data-log-key={line.key} data-inspectable="true" value={value}>
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

function Log({ view: newest, revealed = false }: LogProps): ReactElement {
  // #3633: the log follows the board a render behind, at low priority, so the frame a view lands in is
  // not also the one that reads its whole window into lines (R3351000's phone budget).
  const view = useDeferredValue(newest);
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
  // #4614: only the rows near the viewport mount; the rest keep their place as blank
  // spacers. Everything the log says is `shown` itself: the window only decides which of
  // those same lines draw (CLAUDE.md rule 7).
  const keys = shown.map((line) => line.key);
  // A row with an open inspect sheet or focus stays mounted, as the collection's grid keeps
  // its held card (#3734): unmounting it would drop the focus and close the sheet.
  const [heldKey, setHeldKey] = useState<string | null>(null);
  const windowing = useLogWindow(listRef, keys, heldKey);
  const windowed = windowing !== null && windowing !== "all";
  const first = windowed ? windowing.first : 0;
  const last = windowed ? windowing.last : shown.length - 1;
  const head = windowed ? windowing.head : 0;
  const tail = windowed ? windowing.tail : 0;
  // Whether the list sits at its newest line: arrivals re-pin only a pinned list, so one
  // scrolled up is never yanked down.
  const pinnedRef = useRef(true);
  useLayoutEffect(() => {
    const list = listRef.current;
    if (list !== null && pinnedRef.current) list.scrollTop = list.scrollHeight;
  }, [windowing]);
  // #4140: the screen-reader announcer's words: the newest shown line's plain words (`RichText.full`,
  // never an instance id or a def id; CLAUDE.md rule 7). The ref primes silently on mount and on
  // hotseat hand-over, so history never replays; only a new newest key speaks.
  const newestLine: Line | null = shown.length === 0 ? null : (shown[shown.length - 1] ?? null);
  const [said, setSaid] = useState<{ key: string | null; text: string }>({ key: null, text: "" });
  const lastKey = useRef<{ viewer: PlayerView["viewer"]; key: string | null } | null>(null);
  useEffect(() => {
    const key = newestLine?.key ?? null;
    if (lastKey.current === null || lastKey.current.viewer !== view.viewer) {
      lastKey.current = { viewer: view.viewer, key };
      return;
    }
    if (key !== lastKey.current.key) {
      lastKey.current = { viewer: view.viewer, key };
      setSaid({ key, text: newestLine?.text.full ?? "" });
    }
  });
  return (
    <>
      <ol
        ref={listRef}
        className={windowed ? "log log--virtual" : "log"}
        data-testid={testid.log}
        aria-label={t("Game log")}
        onScroll={(event) => {
          const list = event.currentTarget;
          pinnedRef.current = list.scrollHeight - list.scrollTop - list.clientHeight <= LOG_PIN_TOLERANCE_PX;
        }}
        onFocus={(event) => {
          const row = event.target instanceof Element ? event.target.closest<HTMLElement>("li[data-log-key]") : null;
          setHeldKey(row?.dataset.logKey ?? null);
        }}
        onBlur={(event) => {
          // Focus moving to another row of the log is the next row's focus to say.
          if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) setHeldKey(null);
        }}
      >
        {/* #4614: the unmounted head's blank place; out of the numbering, as the divider is. */}
        {head > 0 ? <li className="log-spacer" aria-hidden="true" style={{ height: head }} /> : null}
        {shown.slice(first, last + 1).map((line, at) => {
          const whole = first + at;
          return (
            <Fragment key={line.key}>
              {/* #757: drawn only; a list holds list items, and the turn's own line names the turn. */}
              {whole > 0 && line.type === "turnStarted" ? (
                <li className="log-divider" aria-hidden="true" data-log-divider={line.key} />
              ) : null}
              <LogItem line={line} value={windowed && at === 0 && whole > 0 ? whole + 1 : undefined} />
            </Fragment>
          );
        })}
        {/* #4614: the unmounted tail's blank place. */}
        {tail > 0 ? <li className="log-spacer" aria-hidden="true" style={{ height: tail }} /> : null}
      </ol>
      {/* #4140: always in the tree (as the draw notices are) so an arriving line is read out; `said`
          only changes on arrival, so mount, reveal and re-render announce nothing. */}
      <p className="log-live" data-testid={testid.logLive} role="status" aria-live="polite" aria-atomic="true">
        <span key={said.key}>{said.text}</span>
      </p>
    </>
  );
}

/** #3633: a queue step re-renders the board but leaves the shown view, so the log draws nothing new and is skipped. */
export default memo(Log);
