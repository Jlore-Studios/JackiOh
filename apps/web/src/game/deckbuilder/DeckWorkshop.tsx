// `/decks` deck workshop (SPEC §9.4, R250–R256): up to ten saved decks and five trios, one open at
// a time. On phones the mounted list and editor alternate; CSS chooses the visible half (R339–R341).
// Server limits and validation remain authoritative (CLAUDE.md rule 7); this component only owns
// the local store and supplied API (R256). R2119000: Delete deck and Delete trio ask nothing. The item
// leaves the list at once (the store's `hide`) and its DELETE waits for the undo window
// (`undo/hold.ts`); anything that needs it gone first sends the held DELETE at once.

import { useEffect, useId, useRef, useState, useSyncExternalStore, type ReactElement, type RefObject } from "react";

import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import "../../auth/tavern.css";
import "./deckbuilder.css";
import "./workshop.css";
import { deckCosmeticPatch, pickedDeckCosmetics } from "../../cards/cosmetics.ts";
import { EmptyState } from "../../empty/EmptyState.tsx";
import { t, useLocale } from "../../i18n/i18n.ts";
import type { DecksResponse } from "../../net/api.ts";
import { navigate, paths, practiceDeckPath, useSearch, wantsNewDeck } from "../../net/navigate.ts";
import { BackLink } from "../../routes/nav.tsx";
import { deckChanges, type NewestChanges } from "../../patches/deckChanges.ts";
import { useShortcutHints } from "../../shortcuts/Kbd.tsx";
import { ariaKeyshortcuts, SHORTCUTS, shortcutTitle } from "../../shortcuts/shortcuts.ts";
import { useUndoHold } from "../../undo/hold.ts";
import { UndoToast } from "../../undo/UndoToast.tsx";
import DeckCosmeticsPicker from "./DeckCosmeticsPicker.tsx";
import DeckEditor from "./DeckEditor.tsx";
// R2100000: the kept versions' popover, under another name than #2099's undo history (history.ts).
import DeckHistoryPopover from "./DeckHistory.tsx";
import DeckPatchBadge, { useNewestChanges } from "./DeckPatchBadge.tsx";
import { emptyHistory, historyKey, inTextField, recorded, redone, sealed, undone, type DeckHistory, type DeckPatch } from "./history.ts";
import { DECK_SIZE } from "./deckSize.ts";
import type { PoolFilter, PoolSort } from "./filters.ts";
import ImportPanel from "./ImportPanel.tsx";
import { loadBrowse, saveBrowse, type SavedBrowse } from "./savedBrowse.ts";
import TrioImportPanel from "./TrioImportPanel.tsx";
import {
  browserStorage,
  createDeckStore,
  type DeckItem,
  type DeckStore,
  type DeckSyncApi,
  type StorageLike,
  type SyncClock,
  type SyncStatus,
  type TrioItem,
  type WorkshopSnapshot,
} from "./sync.ts";
import {
  DECK_CAP,
  DECK_CAP_REASON,
  DECK_IMPORT_OPEN,
  DECK_LIST,
  DECK_NEW,
  DECK_PRACTICE_ONLY,
  DECK_REDO,
  DECK_UNDO,
  SYNC_STATUS,
  TRIO_CAP,
  TRIO_CAP_REASON,
  TRIO_IMPORT_OPEN,
  TRIO_LIST,
  TRIO_NEW,
  WORKSHOP,
  WORKSHOP_BACK,
  WORKSHOP_EMPTY,
  deckDuplicateId,
  deckRowId,
  trioRowId,
} from "./testids.ts";
import TrioEditor from "./TrioEditor.tsx";
import { copyName, deckLabel, deckStatus, deckVerdict, nextName, practiceOnly, trioLabel, trioVerdict } from "./workshop.ts";

/** Keep this layout breakpoint aligned with workshop.css's `@media (max-width: 1100px)`. */
export const WORKSHOP_SPLIT_MIN_WIDTH_PX = 1101;

/** #2099: Undo's arrow and Redo's, its mirror (24 px, stroked). */
const UNDO_PATH = "M9 4 4 9l5 5M4 9h10.5a5.5 5.5 0 0 1 0 11H11";
const REDO_PATH = "M15 4l5 5-5 5M20 9H9.5a5.5 5.5 0 0 0 0 11H13";

/** Whether changing halves must also move focus. */
function halvesTakeTurns(): boolean {
  try {
    return window.matchMedia(`(max-width: ${String(WORKSHOP_SPLIT_MIN_WIDTH_PX - 1)}px)`).matches;
  } catch {
    return false;
  }
}

export type WorkshopOpen =
  | { kind: "deck"; id: string }
  | { kind: "trio"; id: string }
  | { kind: "import" }
  | { kind: "trio-import" }
  | null;

export type DeckWorkshopProps = {
  catalog: CatalogSnapshot;
  /** Null collection leaves ownership unknown. */
  collection: Collection | null;
  /** Server decks, overlaid by the local mirror (R256). */
  data: DecksResponse;
  /** Owner of the local mirror. */
  profileId: string;
  /** Route-bound deck writes. */
  api: DeckSyncApi;
  /** Optional initial item; phones otherwise start at the list. */
  initialOpen?: WorkshopOpen;
  /** Test seams for mirror storage, clock (the undo window's too, R2119000), and ID minting. */
  storage?: StorageLike | null;
  clock?: SyncClock;
  newId?: () => string;
};

/** Save-status wording; server errors follow ours (R256). */
export function syncWords(status: SyncStatus): string {
  switch (status.state) {
    case "saved":
      return t("Saved");
    case "saving":
      return t("Saving…");
    case "offline":
      return t("Offline. Kept on this device.");
    case "error":
      return status.message === null ? t("Couldn't save") : t("Couldn't save: {message}", { message: t(status.message) });
  }
}

function SyncStatusLine({ status }: { status: SyncStatus }): ReactElement {
  return (
    <p className="ws-sync" data-testid={SYNC_STATUS} data-state={status.state} role="status" aria-live="polite">
      <span className="ws-sync-dot" aria-hidden="true" />
      <span>{syncWords(status)}</span>
    </p>
  );
}

type RailProps = {
  railRef: RefObject<HTMLElement | null>;
  snapshot: WorkshopSnapshot;
  catalog: CatalogSnapshot;
  collection: Collection | null;
  open: WorkshopOpen;
  onOpen: (next: WorkshopOpen) => void;
  onNewDeck: () => void;
  onDuplicateDeck: (deck: DeckItem) => void;
  onNewTrio: () => void;
  /** R2097000: the newest patch's changes while it is the catalog shown, else null. */
  patch: NewestChanges | null;
};

function Rail(props: RailProps): ReactElement {
  const { railRef, snapshot, catalog, collection, open, onOpen, onNewDeck, onDuplicateDeck, onNewTrio, patch } = props;
  const { decks, trios, limits, unsynced } = snapshot;
  const decksAtCap = decks.length >= limits.decks;
  const triosAtCap = trios.length >= limits.trios;
  const decksTitle = useId();
  const triosTitle = useId();
  const deckReason = useId();
  const trioReason = useId();

  return (
    <nav ref={railRef} className="ws-rail" aria-label={t("Your decks and trios")}>
      <section className="ws-group" aria-labelledby={decksTitle}>
        <div className="ws-group-head">
          <h2 className="ws-group-title" id={decksTitle}>
            {t("Decks")}{" "}
            <span className="ws-cap" data-testid={DECK_CAP} data-count={String(decks.length)} data-limit={String(limits.decks)}>
              {`${String(decks.length)}/${String(limits.decks)}`}
            </span>
          </h2>
          <div className="ws-group-actions">
            <button
              type="button"
              className="ws-action ws-action--primary"
              data-testid={DECK_NEW}
              disabled={decksAtCap}
              aria-describedby={decksAtCap ? deckReason : undefined}
              onClick={onNewDeck}
            >
              {t("New deck")}
            </button>
            <button
              type="button"
              className="ws-action"
              data-testid={DECK_IMPORT_OPEN}
              aria-current={open?.kind === "import" ? "true" : undefined}
              onClick={() => {
                onOpen({ kind: "import" });
              }}
            >
              {t("Import")}
            </button>
          </div>
        </div>
        {decksAtCap ? (
          <p className="ws-cap-note" id={deckReason} data-testid={DECK_CAP_REASON}>
            {t("You have {count} decks, the most you can keep. Delete one to make room.", { count: limits.decks })}
          </p>
        ) : null}
        <ul className="ws-list" data-testid={DECK_LIST}>
          {decks.map((deck) => (
            <DeckRow
              key={deck.id}
              deck={deck}
              catalog={catalog}
              collection={collection}
              nameLength={limits.nameLength}
              current={open?.kind === "deck" && open.id === deck.id}
              unsynced={unsynced.has(deck.id)}
              atCap={decksAtCap}
              reasonId={decksAtCap ? deckReason : undefined}
              patch={patch}
              onOpen={() => {
                onOpen({ kind: "deck", id: deck.id });
              }}
              onDuplicate={() => {
                onDuplicateDeck(deck);
              }}
            />
          ))}
        </ul>
        {decks.length === 0 ? <p className="ws-hint">{t("No decks yet. Make one, or import a code someone shared.")}</p> : null}
      </section>

      <section className="ws-group" aria-labelledby={triosTitle}>
        <div className="ws-group-head">
          <h2 className="ws-group-title" id={triosTitle}>
            {t("Trios")}{" "}
            <span className="ws-cap" data-testid={TRIO_CAP} data-count={String(trios.length)} data-limit={String(limits.trios)}>
              {`${String(trios.length)}/${String(limits.trios)}`}
            </span>
          </h2>
          <div className="ws-group-actions">
            <button
              type="button"
              className="ws-action"
              data-testid={TRIO_NEW}
              disabled={triosAtCap}
              aria-describedby={triosAtCap ? trioReason : undefined}
              onClick={onNewTrio}
            >
              {t("New trio")}
            </button>
            {/* R340: never off at a cap — the panel says exactly how many slots an import needs. */}
            <button
              type="button"
              className="ws-action"
              data-testid={TRIO_IMPORT_OPEN}
              aria-current={open?.kind === "trio-import" ? "true" : undefined}
              onClick={() => {
                onOpen({ kind: "trio-import" });
              }}
            >
              {t("Import trio")}
            </button>
          </div>
        </div>
        {triosAtCap ? (
          <p className="ws-cap-note" id={trioReason} data-testid={TRIO_CAP_REASON}>
            {t("You have {count} trios, the most you can keep. Delete one to make room.", { count: limits.trios })}
          </p>
        ) : null}
        <ul className="ws-list" data-testid={TRIO_LIST}>
          {trios.map((trio) => (
            <TrioRow
              key={trio.id}
              trio={trio}
              decks={decks}
              catalog={catalog}
              collection={collection}
              nameLength={limits.nameLength}
              current={open?.kind === "trio" && open.id === trio.id}
              unsynced={unsynced.has(trio.id)}
              onOpen={() => {
                onOpen({ kind: "trio", id: trio.id });
              }}
            />
          ))}
        </ul>
        {trios.length === 0 ? (
          <p className="ws-hint">{t("A trio is three decks with no card in common, for Conquest.")}</p>
        ) : null}
      </section>
    </nav>
  );
}

type DeckRowProps = {
  deck: DeckItem;
  catalog: CatalogSnapshot;
  collection: Collection | null;
  nameLength: number;
  current: boolean;
  unsynced: boolean;
  atCap: boolean;
  reasonId: string | undefined;
  patch: NewestChanges | null;
  onOpen: () => void;
  onDuplicate: () => void;
};

function DeckRow({ deck, catalog, collection, nameLength, current, unsynced, atCap, reasonId, patch, onOpen, onDuplicate }: DeckRowProps): ReactElement {
  const verdict = deckVerdict(deck, catalog, collection, nameLength);
  const status = deckStatus(verdict, collection !== null);
  // R915005: a deck naming a custom card plays only in practice (L7), whatever else it lacks.
  const practice = practiceOnly(verdict);
  const label = deckLabel(deck, nameLength);
  const words = {
    deck: label,
    count: deck.cards.length,
    size: DECK_SIZE,
    status: practice ? t("{status}, practice only", { status: status.label }) : status.label,
  };
  return (
    <li className="ws-list-item">
      <button
        type="button"
        className="ws-row"
        data-testid={deckRowId(deck.id)}
        data-count={String(deck.cards.length)}
        data-status={status.kind}
        data-practice-only={practice ? "true" : undefined}
        data-unsynced={unsynced ? "true" : undefined}
        aria-current={current ? "true" : undefined}
        onClick={onOpen}
      >
        {/* A reader hears the row in words and the eye reads it drawn. An aria-label instead would hide
            the drawn "<count>/<DECK_SIZE>" from voice control, which axe's label-content-name-mismatch refuses. */}
        <span className="ws-row-words">
          {unsynced ? t("{deck}, {count} of {size} cards, {status}, not saved yet", words) : t("{deck}, {count} of {size} cards, {status}", words)}
        </span>
        <span className="ws-row-name" aria-hidden="true">
          {label}
        </span>
        <span className="ws-row-meta" aria-hidden="true">
          <span className="ws-row-count">{`${String(deck.cards.length)}/${String(DECK_SIZE)}`}</span>
          <span className="ws-chip" data-status={status.kind}>
            {status.label}
          </span>
          {practice ? (
            <span className="ws-chip" data-status="practice" data-testid={DECK_PRACTICE_ONLY} aria-hidden="true">
              {t("Practice only")}
            </span>
          ) : null}
        </span>
        {unsynced ? <span className="ws-row-dot" aria-hidden="true" /> : null}
      </button>
      {/* #2096: drawn as two sheets, in words for a reader and on hover: a worded button would wrap
          every row of the 208 px rail. */}
      <button
        type="button"
        className="ws-row-duplicate"
        data-testid={deckDuplicateId(deck.id)}
        aria-label={t("Duplicate {deck}", { deck: label })}
        title={t("Duplicate {deck}", { deck: label })}
        disabled={atCap}
        aria-describedby={reasonId}
        onClick={onDuplicate}
      />
      {patch === null ? null : <DeckPatchBadge deckId={deck.id} cards={deckChanges(deck, patch)} version={patch.version} />}
    </li>
  );
}

type TrioRowProps = {
  trio: TrioItem;
  decks: readonly DeckItem[];
  catalog: CatalogSnapshot;
  collection: Collection | null;
  nameLength: number;
  current: boolean;
  unsynced: boolean;
  onOpen: () => void;
};

function TrioRow({ trio, decks, catalog, collection, nameLength, current, unsynced, onOpen }: TrioRowProps): ReactElement {
  const ready = trioVerdict(trio, decks, catalog, collection, nameLength).ok;
  const filled = trio.deckIds.filter((id) => id !== null && decks.some((deck) => deck.id === id)).length;
  const label = trioLabel(trio, nameLength);
  const words = { trio: label, count: filled, size: trio.deckIds.length, status: ready ? t("ready for Conquest") : t("not ready") };
  return (
    <li>
      <button
        type="button"
        className="ws-row"
        data-testid={trioRowId(trio.id)}
        data-ready={ready ? "true" : "false"}
        data-unsynced={unsynced ? "true" : undefined}
        aria-current={current ? "true" : undefined}
        onClick={onOpen}
      >
        {/* As the deck row's: in words for a reader, drawn for the eye. */}
        <span className="ws-row-words">
          {unsynced ? t("{trio}, {count} of {size} decks, {status}, not saved yet", words) : t("{trio}, {count} of {size} decks, {status}", words)}
        </span>
        <span className="ws-row-name" aria-hidden="true">
          {label}
        </span>
        <span className="ws-row-meta" aria-hidden="true">
          <span className="ws-row-count">{t("{count}/{size} decks", { count: filled, size: trio.deckIds.length })}</span>
          <span className="ws-chip" data-status={ready ? "ready" : "incomplete"}>
            {ready ? t("Ready") : t("Not ready")}
          </span>
        </span>
        {unsynced ? <span className="ws-row-dot" aria-hidden="true" /> : null}
      </button>
    </li>
  );
}

function neighbourOf<T extends { id: string }>(items: readonly T[], id: string): T | null {
  const at = items.findIndex((item) => item.id === id);
  if (at < 0) return null;
  return items[at + 1] ?? items[at - 1] ?? null;
}

/**
 * #1991: where the open deck's "Practice with this deck" goes, or null for none. `saved:<n>` names the
 * deck by its place in the server's list, which the store's list is only while nothing waits on the
 * server: no deck unsynced (R256) or deletion pending (each keeps the status off `saved`), and no
 * deletion held (R2119000). Practice offers only a complete deck (§9.9).
 */
function practicePathOf(snapshot: WorkshopSnapshot, deck: DeckItem): string | null {
  if (deck.cards.length !== DECK_SIZE || snapshot.status.state !== "saved" || snapshot.held?.kind === "deck") return null;
  if (snapshot.decks.some((other) => snapshot.unsynced.has(other.id))) return null;
  return practiceDeckPath(snapshot.decks.findIndex((other) => other.id === deck.id) + 1);
}

export default function DeckWorkshop(props: DeckWorkshopProps): ReactElement {
  const { catalog, collection, data, profileId, api, initialOpen } = props;
  useLocale();
  const hints = useShortcutHints();
  const patch = useNewestChanges(catalog.version);

  const [store] = useState<DeckStore>(() =>
    createDeckStore({
      profileId,
      catalogVersion: catalog.version,
      server: data,
      api,
      ...(props.storage === undefined ? {} : { storage: props.storage }),
      ...(props.clock === undefined ? {} : { clock: props.clock }),
      ...(props.newId === undefined ? {} : { newId: props.newId }),
    }),
  );
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
  // R2119000: before the store's start and stop, so leaving the screen sends a held DELETE first.
  const { held, hold, undo: undoDeletion, sendNow } = useUndoHold(props.clock);

  useEffect(() => {
    store.start();
    return () => {
      store.stop();
    };
  }, [store]);

  const [open, setOpen] = useState<WorkshopOpen>(() => {
    if (initialOpen !== undefined) return initialOpen;
    const first = store.getSnapshot().decks[0];
    return first === undefined ? null : { kind: "deck", id: first.id };
  });
  const [view, setView] = useState<"list" | "editor">(
    initialOpen === undefined || initialOpen === null ? "list" : "editor",
  );
  // Keep filters and sort per device across deck switches.
  const [browseStorage] = useState<StorageLike | null>(() =>
    props.storage === undefined ? browserStorage() : props.storage,
  );
  const [browse, setBrowse] = useState<SavedBrowse>(() => loadBrowse(browseStorage));
  const { filter, sort } = browse;
  const updateBrowse = (next: SavedBrowse): void => {
    setBrowse(next);
    saveBrowse(browseStorage, next);
  };
  const setFilter = (next: PoolFilter): void => {
    updateBrowse({ filter: next, sort });
  };
  const setSort = (next: PoolSort): void => {
    updateBrowse({ filter, sort: next });
  };

  // On phones, follow a disappearing half's focus to the newly visible half.
  const railRef = useRef<HTMLElement>(null);
  const mainRef = useRef<HTMLElement>(null);
  const [moveFocus, setMoveFocus] = useState<"editor" | "list" | null>(null);
  useEffect(() => {
    if (moveFocus === null) return;
    setMoveFocus(null);
    if (!halvesTakeTurns()) return;
    const target =
      moveFocus === "editor"
        ? mainRef.current?.querySelector<HTMLElement>(`[data-testid="${WORKSHOP_BACK}"]`)
        : (railRef.current?.querySelector<HTMLElement>('[aria-current="true"]') ??
          railRef.current?.querySelector<HTMLElement>("button"));
    target?.focus();
  }, [moveFocus]);

  const { decks, trios, limits, unsynced, refused } = snapshot;
  const openDeck = open?.kind === "deck" ? (decks.find((deck) => deck.id === open.id) ?? null) : null;
  const openTrio = open?.kind === "trio" ? (trios.find((trio) => trio.id === open.id) ?? null) : null;

  // #2099: the open deck's history (history.ts), reset while rendering whenever the open deck changes.
  const openDeckId = openDeck?.id ?? null;
  const [history, setHistory] = useState<DeckHistory>(() => emptyHistory(openDeckId));
  if (history.deckId !== openDeckId) setHistory(emptyHistory(openDeckId));

  const openItem = (next: WorkshopOpen): void => {
    setOpen(next);
    setView(next === null ? "list" : "editor");
    // Import panels focus their paste fields themselves.
    if (next !== null && next.kind !== "import" && next.kind !== "trio-import") setMoveFocus("editor");
  };

  const newDeck = (): void => {
    // R2119000: the held deck still counts against the cap until its DELETE has gone.
    sendNow();
    const id = store.createDeck({ name: nextName((number) => t("Deck {number}", { number }), decks.map((deck) => deck.name)) });
    if (id !== null) openItem({ kind: "deck", id });
  };

  // R2118000: the command menu's New deck (`/decks?new=deck`). The query is read off the address
  // itself and dropped first, so a second run of this effect (StrictMode) makes none.
  const asksNewDeck = wantsNewDeck(useSearch());
  useEffect(() => {
    if (!asksNewDeck || !wantsNewDeck(window.location.search)) return;
    navigate(paths.decks, { replace: true, force: true });
    newDeck();
  }, [asksNewDeck]);

  // #2096: trios name decks by id (R252), so every trio naming the original still does.
  const duplicateDeck = (source: DeckItem): void => {
    sendNow();
    const name = copyName(deckLabel(source, limits.nameLength), decks.map((deck) => deckLabel(deck, limits.nameLength)), limits.nameLength);
    const id = store.createDeck({ ...pickedDeckCosmetics(source), name, cards: source.cards, portrait: source.portrait });
    if (id !== null) openItem({ kind: "deck", id });
  };

  const newTrio = (): void => {
    sendNow();
    const id = store.createTrio({ name: nextName((number) => t("Trio {number}", { number }), trios.map((trio) => trio.name)) });
    if (id !== null) openItem({ kind: "trio", id });
  };

  const backToList = (): void => {
    setView("list");
    setMoveFocus("list");
    setHistory((current) => emptyHistory(current.deckId));
  };

  /** #2099: an edit of deck `id`, one step of the history; `naming` folds a rename's keys into one. */
  const editDeck = (id: string, patch: DeckPatch, naming = false): void => {
    // Read fresh, not from this render, so two edits in one tick each hold what they overwrote.
    const deck = store.getSnapshot().decks.find((candidate) => candidate.id === id);
    if (deck === undefined) return;
    setHistory((current) => recorded(current, deck, patch, naming));
    store.updateDeck(id, patch);
  };

  const undo = (): void => {
    const step = undone(history);
    if (openDeck === null || step === null) return;
    setHistory(step.history);
    store.updateDeck(openDeck.id, step.patch);
  };

  const redo = (): void => {
    const step = redone(history);
    if (openDeck === null || step === null) return;
    setHistory(step.history);
    store.updateDeck(openDeck.id, step.patch);
  };

  // #2099: the keys, outside text fields; a ref, as ReplayPlayer's arrows, holds this render's history.
  const historyKeys = useRef<(event: KeyboardEvent) => void>(() => undefined);
  historyKeys.current = (event) => {
    const asked = historyKey(event);
    if (asked === null || openDeck === null || event.defaultPrevented || inTextField(event.target)) return;
    event.preventDefault();
    if (asked === "undo") undo();
    else redo();
  };
  useEffect(() => {
    const listener = (event: KeyboardEvent): void => {
      historyKeys.current(event);
    };
    window.addEventListener("keydown", listener);
    return () => {
      window.removeEventListener("keydown", listener);
    };
  }, []);

  let main: ReactElement;
  if (openDeck !== null) {
    main = (
      <DeckEditor
        key={openDeck.id}
        deck={openDeck}
        decks={decks}
        trios={trios}
        catalog={catalog}
        collection={collection}
        limits={limits}
        saved={!unsynced.has(openDeck.id)}
        refusal={refused.get(openDeck.id) ?? null}
        practicePath={practicePathOf(snapshot, openDeck)}
        patch={patch}
        filter={filter}
        onFilter={setFilter}
        sort={sort}
        onSort={setSort}
        onRename={(name) => {
          editDeck(openDeck.id, { name }, true);
        }}
        onRenameDone={() => {
          setHistory(sealed);
        }}
        onPortrait={(portrait) => {
          editDeck(openDeck.id, { portrait });
        }}
        onCards={(cards) => {
          editDeck(openDeck.id, { cards });
        }}
        onDelete={() => {
          const id = openDeck.id;
          const next = neighbourOf(decks, id);
          // R2119000: `hold` first, since it sends any deletion held before this one.
          hold({
            words: () => t("Deck deleted"),
            send: (keepalive) => {
              store.deleteDeck(id);
              void store.flush({ keepalive });
            },
            restore: () => {
              store.unhide();
              setOpen({ kind: "deck", id });
            },
          });
          store.hide("deck", id);
          setOpen(next === null ? null : { kind: "deck", id: next.id });
          setView("list");
        }}
        onDuplicate={() => {
          duplicateDeck(openDeck);
        }}
        onSaveNow={() => {
          void store.flush();
        }}
        onBack={backToList}
        // R2100000: mounted with the editor, which is keyed by the deck, so one deck's list never shows under another.
        history={
          <DeckHistoryPopover
            deckId={openDeck.id}
            store={store}
            catalog={catalog}
            onRestored={() => {
              // R2100000, #2099: Undo's steps were taken on the deck the restore replaced, so they end, as on closing the editor.
              setHistory((current) => (current.deckId === openDeck.id ? emptyHistory(current.deckId) : current));
            }}
          />
        }
      />
    );
  } else if (openTrio !== null) {
    main = (
      <TrioEditor
        key={openTrio.id}
        trio={openTrio}
        decks={decks}
        catalog={catalog}
        collection={collection}
        limits={limits}
        refusal={refused.get(openTrio.id) ?? null}
        onRename={(name) => {
          store.updateTrio(openTrio.id, { name });
        }}
        onSlots={(deckIds) => {
          store.updateTrio(openTrio.id, { deckIds });
        }}
        onDelete={() => {
          const id = openTrio.id;
          hold({
            words: () => t("Trio deleted"),
            send: (keepalive) => {
              store.deleteTrio(id);
              void store.flush({ keepalive });
            },
            restore: () => {
              store.unhide();
              setOpen({ kind: "trio", id });
            },
          });
          store.hide("trio", id);
          setOpen(null);
          setView("list");
        }}
        onOpenDeck={(deckId) => {
          openItem({ kind: "deck", id: deckId });
        }}
        onBack={backToList}
      />
    );
  } else if (open?.kind === "import") {
    main = (
      <ImportPanel
        catalog={catalog}
        collection={collection}
        atCap={decks.length >= limits.decks}
        deckLimit={limits.decks}
        onImport={(name, cards) => {
          sendNow();
          const id = store.createDeck({ name, cards });
          if (id !== null) openItem({ kind: "deck", id });
        }}
        onCancel={() => {
          const first = decks[0];
          setOpen(first === undefined ? null : { kind: "deck", id: first.id });
          setView("list");
        }}
      />
    );
  } else if (open?.kind === "trio-import") {
    main = (
      <TrioImportPanel
        catalog={catalog}
        collection={collection}
        saved={{ decks: decks.length, trios: trios.length }}
        limits={limits}
        onImport={(init) => {
          sendNow();
          return store.importTrio(init);
        }}
        onImported={(trioId) => {
          openItem({ kind: "trio", id: trioId });
        }}
        onCancel={() => {
          const first = trios[0];
          setOpen(first === undefined ? null : { kind: "trio", id: first.id });
          setView("list");
        }}
      />
    );
  } else {
    main = (
      <div className="ws-empty" data-testid={WORKSHOP_EMPTY}>
        {decks.length === 0 ? (
          <EmptyState
            text={t("Make your first deck, or import a code someone shared.")}
            action={{ label: t("Make a deck"), onPress: newDeck }}
          />
        ) : (
          <EmptyState text={t("Pick a deck or a trio from the list to work on it.")} />
        )}
      </div>
    );
  }

  return (
    <div className="app-shell app-shell--wide tavern deckbuilder workshop" data-testid={WORKSHOP} data-view={view}>
      <header className="db-header">
        <BackLink to={paths.collection} />
        <h1 className="db-title">{t("Decks")}</h1>
        {openDeck === null ? null : (
          <div className="ws-history">
            <button
              type="button"
              className="ws-history-button"
              data-testid={DECK_UNDO}
              aria-label={t("Undo")}
              title={shortcutTitle(t("Undo"), SHORTCUTS.undo, hints)}
              aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.undo)}
              disabled={history.past.length === 0}
              onClick={undo}
            >
              <span className="ws-cosmetics-button__icon" aria-hidden="true">
                <svg viewBox="0 0 24 24">
                  <path d={UNDO_PATH} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </span>
            </button>
            <button
              type="button"
              className="ws-history-button"
              data-testid={DECK_REDO}
              aria-label={t("Redo")}
              title={shortcutTitle(t("Redo"), SHORTCUTS.redo, hints)}
              aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.redo)}
              disabled={history.future.length === 0}
              onClick={redo}
            >
              <span className="ws-cosmetics-button__icon" aria-hidden="true">
                <svg viewBox="0 0 24 24">
                  <path d={REDO_PATH} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
                </svg>
              </span>
            </button>
          </div>
        )}
        {openDeck === null ? null : (
          <DeckCosmeticsPicker
            deck={openDeck}
            onCosmetic={(kind, id) => {
              editDeck(openDeck.id, deckCosmeticPatch(kind, id));
            }}
          />
        )}
        <SyncStatusLine status={snapshot.status} />
      </header>

      <div className="ws-layout">
        <Rail
          railRef={railRef}
          snapshot={snapshot}
          catalog={catalog}
          collection={collection}
          open={open}
          onOpen={openItem}
          onNewDeck={newDeck}
          onDuplicateDeck={duplicateDeck}
          onNewTrio={newTrio}
          patch={patch}
        />
        <main ref={mainRef} className="ws-main">
          {main}
        </main>
      </div>
      <UndoToast held={held} onUndo={undoDeletion} />
    </div>
  );
}
