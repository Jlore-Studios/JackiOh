// R915004: a custom card's hook that failed did nothing, and the board says so (SPEC §12.5). The
// engine's `scriptFailed` names the card and the reason; this notice shows the newest one a view
// brings, "{card}'s script failed: {message}", beside the board until the player dismisses it or the
// next failure replaces it. The log keeps every one (Log.tsx), the animation table fades the card
// (animations.ts) and the cue sounds (audio/cues.ts); this is the line a player cannot miss.
//
// Read off the NEWEST view, as the draw offer's notices are: it changes no board position. Only the
// events a view adds count (`newEventsSince`, as the runner reads them), so a resumed game or a
// hotseat hand-over shows no failure from before. No rule lives here (CLAUDE.md rule 7).

import { useContext, useEffect, useRef, useState, type ReactElement } from "react";

import type { GameEvent, PlayerView } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { newEventsSince } from "./animations.ts";
import { CatalogContext, withMatchDefs } from "./catalog.ts";
import "./notices.css";

/** The notice and its dismiss button. */
export const scriptFailedTestid = {
  notice: "script-failed-notice",
  dismiss: "script-failed-dismiss",
} as const;

/** R97: the id an event carries in place of a card this seat may not read. */
const HIDDEN_CARD = "hidden";

export type ScriptFailure = Extract<GameEvent, { type: "scriptFailed" }>;

function isScriptFailed(event: GameEvent): event is ScriptFailure {
  return event.type === "scriptFailed";
}

/** The newest `scriptFailed` that `view` adds to `previous`; none for a first view or another seat's. */
export function newestScriptFailure(previous: PlayerView | null, view: PlayerView): ScriptFailure | null {
  if (previous === null || previous.viewer !== view.viewer) return null;
  return newEventsSince(previous.events, view.events).findLast(isScriptFailed) ?? null;
}

export default function ScriptFailedNotice({ view }: { view: PlayerView }): ReactElement | null {
  useLocale();
  const catalog = useContext(CatalogContext);
  const previous = useRef<PlayerView | null>(null);
  const [failure, setFailure] = useState<ScriptFailure | null>(null);

  useEffect(() => {
    const before = previous.current;
    previous.current = view;
    if (before === view) return;
    const newest = newestScriptFailure(before, view);
    if (newest !== null) setFailure(newest);
  }, [view]);

  if (failure === null) return null;
  // A custom card's definition is the view's own (R915000, `PlayerView.defs`), after the catalog.
  const def = failure.defId === HIDDEN_CARD ? undefined : withMatchDefs(catalog, view.defs)?.(failure.defId, false)?.def;
  const card = failure.defId === HIDDEN_CARD ? t("a card") : def === undefined ? failure.defId : readerName(def);

  return (
    <p className="script-failed" data-testid={scriptFailedTestid.notice} data-def-id={failure.defId} role="status">
      <span className="script-failed__text">{t("{card}'s script failed: {message}", { card, message: failure.message })}</span>
      <button
        type="button"
        className="script-failed__dismiss"
        data-testid={scriptFailedTestid.dismiss}
        aria-label={t("Close")}
        title={t("Close")}
        onClick={() => {
          setFailure(null);
        }}
      >
        <span aria-hidden="true">×</span>
      </button>
    </p>
  );
}
