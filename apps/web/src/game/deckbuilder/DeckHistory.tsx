// R2100000: the open deck's kept versions, behind a button in the editor's head after the deck's
// name, in the cosmetics picker's popover (DeckCosmeticsPicker.tsx: outside press and Escape close
// it). The top bar has no room left for it on a phone's first screen (BUILD M5-T1). Each open reads
// the list again, newest first: each version's name and date, and the cards in and out since the
// version kept before it. Every version but the newest offers Restore, which the server keeps as a
// new version, so the one it replaced stays listed, and ends the workshop's Undo and Redo of the deck
// (#2099), whose steps were taken on the deck it replaced. The server decides what a version is and
// whether a restore stands (CLAUDE.md rule 7); this shows its list and its refusal.

import { useCallback, useEffect, useId, useRef, useState, type ReactElement } from "react";

import type { CatalogSnapshot } from "@jackioh/validator";

import { formatDateTime, t, useLocale } from "../../i18n/i18n.ts";
import type { DeckVersion } from "../../net/api.ts";
import { BackLayer } from "../../net/backLayers.ts";
import { versionChange, type VersionChange } from "./deckHistory.ts";
import type { DeckStore } from "./sync.ts";
import { DECK_HISTORY_BUTTON, DECK_HISTORY_PANEL, DECK_HISTORY_ROW, deckHistoryRestoreId } from "./testids.ts";
import { cardNames } from "./workshop.ts";

/** A clock face: the deck's history, drawn like the cosmetics sparkle (24 px, current color), in strokes. */
const CLOCK_PATH = "M12 3.5 A8.5 8.5 0 1 1 11.99 3.5 Z M12 7.5 V12 L15 14";

type Listing = { kind: "loading" } | { kind: "failed"; message: string } | { kind: "ready"; versions: readonly DeckVersion[] };

/** What a row says under its name: the oldest kept, or its rename and its cards in and out, or that no card changed. */
function changeLines(change: VersionChange | null, catalog: CatalogSnapshot): string[] {
  if (change === null) return [t("The oldest version kept")];
  const lines: string[] = [];
  if (change.renamedFrom !== null) lines.push(t("Renamed from “{name}”", { name: change.renamedFrom }));
  if (change.added.length > 0) lines.push(t("In: {cards}", { cards: cardNames(change.added, catalog) }));
  if (change.removed.length > 0) lines.push(t("Out: {cards}", { cards: cardNames(change.removed, catalog) }));
  return lines.length > 0 ? lines : [t("No card changed")];
}

function HistoryRow({
  version,
  older,
  latest,
  catalog,
  restoring,
  onRestore,
}: {
  version: DeckVersion;
  /** The version kept before this one, if any. */
  older: DeckVersion | undefined;
  latest: boolean;
  catalog: CatalogSnapshot;
  /** The version a restore is on its way for, if any: every Restore waits for it. */
  restoring: string | null;
  onRestore: (versionId: string) => void;
}): ReactElement {
  const headId = useId();
  return (
    <li className="ws-versions-row" data-testid={DECK_HISTORY_ROW} data-version={version.id}>
      <div className="ws-versions-head" id={headId}>
        <span className="ws-versions-name">{version.name}</span>
        <span className="ws-versions-date">{formatDateTime(version.savedAt, { dateStyle: "medium", timeStyle: "short" })}</span>
      </div>
      <ul className="ws-versions-change">
        {changeLines(versionChange(version, older), catalog).map((line) => (
          <li key={line}>{line}</li>
        ))}
      </ul>
      {latest ? (
        <span className="ws-versions-latest">{t("Latest save")}</span>
      ) : (
        <button
          type="button"
          className="ws-action"
          data-testid={deckHistoryRestoreId(version.id)}
          aria-describedby={headId}
          aria-busy={restoring === version.id ? "true" : undefined}
          disabled={restoring !== null}
          onClick={() => {
            onRestore(version.id);
          }}
        >
          {restoring === version.id ? t("Restoring…") : t("Restore")}
        </button>
      )}
    </li>
  );
}

export default function DeckHistory({
  deckId,
  store,
  catalog,
  onRestored,
}: {
  deckId: string;
  /** The workshop's store, which reads the list and asks for a restore (sync.ts). */
  store: Pick<DeckStore, "deckVersions" | "restoreDeckVersion">;
  catalog: CatalogSnapshot;
  /** A restore stood: the workshop ends the deck's Undo and Redo (#2099). */
  onRestored: () => void;
}): ReactElement {
  useLocale();
  const [open, setOpen] = useState(false);
  const [listing, setListing] = useState<Listing>({ kind: "loading" });
  const [restoring, setRestoring] = useState<string | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const root = useRef<HTMLDivElement>(null);

  // Focus moves before the panel unmounts, so it never falls back to <body> on the way.
  const close = useCallback((): void => {
    setOpen(false);
    buttonRef.current?.focus();
  }, []);

  // Outside press and Escape close the panel, like the cosmetics picker's; Back does too (R2127000, the panel's BackLayer).
  useEffect(() => {
    if (!open) return undefined;
    const onPointerDown = (event: PointerEvent): void => {
      if (root.current !== null && !root.current.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") close();
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [close, open]);

  // Each open reads the list again: a save since the last look may have kept a version.
  useEffect(() => {
    if (!open) return undefined;
    let canceled = false;
    setListing({ kind: "loading" });
    setRefusal(null);
    void store.deckVersions(deckId).then((result) => {
      if (canceled) return;
      setListing(result.ok ? { kind: "ready", versions: result.versions } : { kind: "failed", message: result.message });
    });
    return () => {
      canceled = true;
    };
  }, [store, deckId, open]);

  const restore = (versionId: string): void => {
    setRestoring(versionId);
    setRefusal(null);
    void store
      .restoreDeckVersion(deckId, versionId)
      .then((result) => {
        if (result.ok) {
          setListing({ kind: "ready", versions: result.versions });
          onRestored();
        } else {
          setRefusal(result.message);
        }
      })
      .finally(() => {
        setRestoring(null);
      });
  };

  return (
    <div className="ws-cosmetics ws-versions" ref={root}>
      <button
        ref={buttonRef}
        type="button"
        className="ws-cosmetics-button"
        data-testid={DECK_HISTORY_BUTTON}
        aria-label={t("Deck history")}
        title={t("Deck history")}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={() => {
          if (open) close();
          else setOpen(true);
        }}
      >
        <span className="ws-cosmetics-button__icon" aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d={CLOCK_PATH} fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </span>
      </button>
      {open ? (
        <div className="ws-cosmetics-panel ws-versions-panel" role="dialog" aria-label={t("Deck history")} data-testid={DECK_HISTORY_PANEL}>
          <BackLayer onClose={close} />
          {listing.kind === "loading" ? (
            <p className="ws-hint" role="status">
              {t("Loading…")}
            </p>
          ) : null}
          {listing.kind === "failed" ? (
            <p className="notice" role="alert">
              {t(listing.message)}
            </p>
          ) : null}
          {refusal === null ? null : (
            <p className="notice" role="alert">
              {t(refusal)}
            </p>
          )}
          {listing.kind === "ready" && listing.versions.length === 0 ? (
            <p className="ws-hint">{t("No versions kept yet. Each save keeps one.")}</p>
          ) : null}
          {listing.kind === "ready" && listing.versions.length > 0 ? (
            <ol className="ws-versions-list">
              {listing.versions.map((version, at) => (
                <HistoryRow
                  key={version.id}
                  version={version}
                  older={listing.versions[at + 1]}
                  latest={at === 0}
                  catalog={catalog}
                  restoring={restoring}
                  onRestore={restore}
                />
              ))}
            </ol>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
