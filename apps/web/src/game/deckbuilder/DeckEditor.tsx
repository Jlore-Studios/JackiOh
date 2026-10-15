// Deck editor: saved deck, card pool and actions (SPEC §9.4, R250–R255; docs/polish/6-cards.md, Surface D).
//
// Validator messages avoid a second source of truth (messages.test.ts).
//
// `addCard` rejects duplicates, full decks (D2, D4), and compared cards (R251); existing clashes stay visible.
//
// The store autosaves (R256); completing a deck saves immediately.

import { useCallback, useEffect, useId, useMemo, useRef, useState, type DragEvent, type ReactElement } from "react";

import type { CatalogSnapshot, Collection } from "@jackioh/validator";
import { portraitOrDefault } from "@jackioh/shared";

import { PortraitPicker } from "../../emotes/PortraitPicker.tsx";
import { closeInspect } from "../../cards/index.ts";
import { readerName } from "../../cards/chinese.ts";
import { t, useLocale } from "../../i18n/i18n.ts";
import CardBrowser, { type BrowserDetail } from "./CardBrowser.tsx";
import { encodeDeckCode } from "./deckCode.ts";
import DeckSidebar from "./DeckSidebar.tsx";
import { DECK_SIZE } from "./deckSize.ts";
import { visiblePool, type PoolFilter, type PoolSort } from "./filters.ts";
import { UNTITLED_DECK, type DeckItem, type TrioItem, type WorkshopLimits } from "./sync.ts";
import { useCardWinRates } from "./useCardWinRates.ts";
import {
  DB_DETAIL_ADD,
  DECK_CODE_OUTPUT,
  DECK_COMPARE_SELECT,
  DECK_CONFLICTS,
  DECK_COPY_CODE,
  DECK_DELETE,
  DECK_DELETE_CANCEL,
  DECK_DELETE_CONFIRM,
  DECK_DRAG_MIME,
  DECK_EDITOR,
  DECK_NAME_INPUT,
  DECK_SAVE_ERROR,
  DECK_STATUS,
  DECK_VERDICT,
  LOADOUT_ERRORS,
  WORKSHOP_BACK,
  deckCompareChipId,
  loadoutErrorId,
} from "./testids.ts";
import {
  NO_COMPARE,
  addCard,
  clampName,
  comparedDecks,
  deckLabel,
  deckVerdict,
  holdersOf,
  joinNames,
  removeCard,
  trioLabel,
  triosHolding,
  withComparedDeck,
  withoutComparedDeck,
  type Compare,
  type Holder,
} from "./workshop.ts";

/** How long the status line shows what the last add, removal or refusal did. */
export const DECK_STATUS_MS = 2600;

/** What the status line says once a deck reaches `DECK_SIZE` and the server has it (R256). */
export const DECK_COMPLETE_SAVED = "Deck complete and saved";

/** What the status line says, worded when it is drawn, so a language switch changes it too. */
type StatusLine = { say: () => string };

export type DeckEditorProps = {
  deck: DeckItem;
  decks: readonly DeckItem[];
  trios: readonly TrioItem[];
  catalog: CatalogSnapshot;
  /** Null when `GET /api/collection` could not be read: ownership is then neither claimed nor denied. */
  collection: Collection | null;
  limits: WorkshopLimits;
  saved: boolean;
  refusal: string | null;
  filter: PoolFilter;
  onFilter: (next: PoolFilter) => void;
  sort: PoolSort;
  onSort: (next: PoolSort) => void;
  onRename: (name: string) => void;
  /** R641, issue §8: a portrait is saved through the deck upsert. */
  onPortrait: (portrait: string) => void;
  onCards: (cards: readonly string[]) => void;
  onDelete: () => void;
  onSaveNow: () => void;
  onBack: () => void;
};

function droppedCardId(event: DragEvent<HTMLElement>, held: string | null): string | null {
  if (held !== null && held.length > 0) return held;
  for (const mime of [DECK_DRAG_MIME, "text/plain"]) {
    try {
      const carried = event.dataTransfer.getData(mime);
      if (carried.length > 0) return carried;
    } catch {
      // No DataTransfer on a synthesised event: try the next type, then give up.
    }
  }
  return null;
}

export default function DeckEditor(props: DeckEditorProps): ReactElement {
  const { deck, decks, trios, catalog, collection, limits, saved, refusal } = props;
  const { filter, onFilter, sort, onSort, onRename, onPortrait, onCards, onDelete, onSaveNow, onBack } = props;

  useLocale();
  const label = deckLabel(deck, limits.nameLength);
  const [compare, setCompare] = useState<Compare>(NO_COMPARE);
  const [status, setStatus] = useState<StatusLine | null>(null);
  const [refusedCardId, setRefusedCardId] = useState<string | null>(null);
  const [detailCardId, setDetailCardId] = useState<string | null>(null);
  const [codeShown, setCodeShown] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [awaitingComplete, setAwaitingComplete] = useState(false);
  const dragged = useRef<string | null>(null);
  const keepButton = useRef<HTMLButtonElement>(null);
  const nameId = useId();
  const verdictTitleId = useId();
  const confirmTextId = useId();

  // Timed by the words it shows, as when it held them: saying the same again leaves its timer running.
  const shown = status === null ? null : status.say();
  useEffect(() => {
    if (shown === null) return undefined;
    const timer = window.setTimeout(() => {
      setStatus(null);
    }, DECK_STATUS_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [shown]);

  useEffect(() => {
    if (!awaitingComplete) return;
    if (deck.cards.length < DECK_SIZE) {
      setAwaitingComplete(false);
      return;
    }
    if (saved) {
      setAwaitingComplete(false);
      setStatus({ say: () => t(DECK_COMPLETE_SAVED) });
    }
  }, [awaitingComplete, saved, deck.cards.length]);

  // Focus the safe answer so a stray Enter keeps the deck.
  useEffect(() => {
    if (confirming) keepButton.current?.focus();
  }, [confirming]);

  const compared = useMemo(() => comparedDecks(compare, deck.id, decks, trios), [compare, deck.id, decks, trios]);
  const holders = useMemo(() => holdersOf(compared, limits.nameLength), [compared, limits.nameLength]);
  const inDeck = useMemo(() => new Set(deck.cards), [deck.cards]);
  const conflicts = useMemo(() => {
    const clashing = new Map<string, Holder>();
    for (const cardId of deck.cards) {
      const holder = holders.get(cardId);
      if (holder !== undefined) clashing.set(cardId, holder);
    }
    return clashing;
  }, [deck.cards, holders]);

  const winRates = useCardWinRates(sort.key);

  const pool = useMemo(
    () => visiblePool(catalog, collection, filter, sort, winRates ?? undefined),
    [catalog, collection, filter, sort, winRates],
  );
  const verdict = useMemo(
    () => deckVerdict(deck, catalog, collection, limits.nameLength),
    [deck, catalog, collection, limits.nameLength],
  );
  const code = useMemo(() => (codeShown ? encodeDeckCode(label, deck.cards, catalog) : ""), [codeShown, label, deck.cards, catalog]);

  // The card's name in the reader's words, read when a status line is drawn, not when it is made.
  const nameOf = useCallback(
    (cardId: string) => {
      const def = catalog.cards[cardId];
      return def === undefined ? cardId : readerName(def);
    },
    [catalog],
  );

  const add = useCallback(
    (cardId: string) => {
      const move = addCard(deck.cards, cardId, holders);
      if (!move.ok) {
        setRefusedCardId(cardId);
        if (move.reason === "here") setStatus({ say: () => t("{card} is already in this deck.", { card: nameOf(cardId) }) });
        else if (move.reason === "held") setStatus({ say: () => t("{card} is in {deck}.", { card: nameOf(cardId), deck: move.holder.name }) });
        else setStatus({ say: () => t("{deck} has {size} cards. Take one out to add another.", { deck: label, size: DECK_SIZE }) });
        return;
      }
      setRefusedCardId(null);
      onCards(move.cards);
      const count = move.cards.length;
      setStatus({ say: () => t("{card} added · {count}/{size}", { card: nameOf(cardId), count, size: DECK_SIZE }) });
      if (move.cards.length === DECK_SIZE) {
        setAwaitingComplete(true);
        onSaveNow();
      }
    },
    [deck.cards, holders, label, nameOf, onCards, onSaveNow],
  );

  const remove = useCallback(
    (cardId: string) => {
      const next = removeCard(deck.cards, cardId);
      if (next === null) return;
      setRefusedCardId(null);
      onCards(next);
      setStatus({ say: () => t("{card} removed · {count}/{size}", { card: nameOf(cardId), count: next.length, size: DECK_SIZE }) });
    },
    [deck.cards, nameOf, onCards],
  );

  const startDrag = useCallback((cardId: string, event: DragEvent<HTMLElement>) => {
    dragged.current = cardId;
    // A drag is not an inspection: any preview still open would sit over the drop target.
    closeInspect();
    try {
      event.dataTransfer.setData(DECK_DRAG_MIME, cardId);
      event.dataTransfer.setData("text/plain", cardId);
      event.dataTransfer.effectAllowed = "move";
    } catch {
      // Cypress and jsdom lack DataTransfer; drops read `dragged` first.
    }
  }, []);

  const endDrag = useCallback(() => {
    dragged.current = null;
  }, []);

  const drop = useCallback(
    (event: DragEvent<HTMLElement>) => {
      event.preventDefault();
      const cardId = droppedCardId(event, dragged.current);
      if (cardId !== null) add(cardId);
    },
    [add],
  );

  const openDetail = useCallback((cardId: string) => {
    // Close a hover preview before opening detail.
    closeInspect();
    setDetailCardId(cardId);
  }, []);

  const copyCode = useCallback(() => {
    setCodeShown(true);
    const text = encodeDeckCode(label, deck.cards, catalog);
    const clipboard = typeof navigator === "undefined" ? undefined : navigator.clipboard;
    const fallback: StatusLine = { say: () => t("Copy the code below to share this deck.") };
    if (clipboard === undefined || typeof clipboard.writeText !== "function") {
      setStatus(fallback);
      return;
    }
    // The always-visible field is the fallback for unavailable clipboard access.
    clipboard.writeText(text).then(
      () => {
        setStatus({ say: () => t("Code copied. Paste it anywhere to share this deck.") });
      },
      () => {
        setStatus(fallback);
      },
    );
  }, [catalog, deck.cards, label]);

  const pickCompare = useCallback(
    (value: string) => {
      if (value === "none") setCompare(NO_COMPARE);
      else if (value.startsWith("trio:")) setCompare({ kind: "trio", trioId: value.slice("trio:".length) });
      else if (value.startsWith("deck:")) setCompare((current) => withComparedDeck(current, value.slice("deck:".length)));
    },
    [],
  );

  const inTrios = triosHolding(deck.id, trios);
  const others = decks.filter((candidate) => candidate.id !== deck.id);
  const errors = verdict.ok ? [] : verdict.errors;
  const detailPlace: "deck" | Holder | null =
    detailCardId === null ? null : inDeck.has(detailCardId) ? "deck" : (holders.get(detailCardId) ?? null);
  const detailOwned = detailCardId === null || collection === null ? null : (collection[detailCardId] ?? 0) > 0;
  const full = deck.cards.length >= DECK_SIZE;
  const triosNamingIt = inTrios.length;
  const conflictNames = joinNames([...new Set([...conflicts.values()].map((holder) => holder.name))]);
  const detail: BrowserDetail | null =
    detailCardId === null
      ? null
      : {
          cardId: detailCardId,
          meta: (
            <span className="db-detail-meta">
              {detailPlace === null ? t("Not in this deck") : detailPlace === "deck" ? t("In this deck") : t("In {name}", { name: detailPlace.name })}
              {detailOwned === false ? ` · ${t("not in your collection")}` : ""}
            </span>
          ),
          actions: (
            <button
              type="button"
              className="db-detail-add"
              data-testid={DB_DETAIL_ADD}
              // Detail uses the same availability as add and drop.
              disabled={detailPlace !== null || full}
              onClick={() => {
                add(detailCardId);
              }}
            >
              {t("Add to {deck}", { deck: label })}
            </button>
          ),
        };

  const head = (
    <div className="ws-editor-head">
      <button type="button" className="ws-back" data-testid={WORKSHOP_BACK} onClick={onBack}>
        {t("← All decks")}
      </button>
      {/* Issue §8: the deck's portrait. The picker holds the whole roster and the previews;
          picking one saves like any edit. `null` in the store reads `vanilla` (R641). */}
      <PortraitPicker
        portrait={portraitOrDefault(deck.portrait)}
        onPick={(next) => {
          onPortrait(next);
        }}
      />
      <label className="ws-name" htmlFor={nameId}>
        <span className="ws-field-label">{t("Deck name")}</span>
        <input
          id={nameId}
          className="ws-name-input"
          data-testid={DECK_NAME_INPUT}
          value={deck.name}
          placeholder={t(UNTITLED_DECK)}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => {
            onRename(clampName(event.target.value, limits.nameLength));
          }}
          onBlur={() => {
            // D1, gently: the field settles on the name the save sends ("Untitled deck" for none).
            if (deck.name !== label) onRename(label);
          }}
        />
      </label>
      {refusal === null ? null : (
        <p className="notice ws-save-error" data-testid={DECK_SAVE_ERROR} role="alert">
          {t(refusal)}
        </p>
      )}
    </div>
  );

  return (
    <section className="ws-editor ws-editor--deck" data-testid={DECK_EDITOR} data-deck={deck.id} aria-label={t("Deck: {deck}", { deck: label })}>
      <div className="db-layout">
        <DeckSidebar
          deckName={label}
          cards={deck.cards}
          catalog={catalog}
          conflicts={conflicts}
          head={head}
          onDropCard={drop}
          onRemove={remove}
          onInspect={openDetail}
        >
          <div className="ws-compare">
            <label className="ws-compare-pick">
              <span className="ws-field-label">{t("Compare with")}</span>
              <select
                className="ws-select"
                data-testid={DECK_COMPARE_SELECT}
                value=""
                disabled={others.length === 0}
                onChange={(event) => {
                  pickCompare(event.target.value);
                }}
              >
                <option value="" disabled>
                  {others.length === 0 ? t("No other decks yet") : t("Another deck or a trio…")}
                </option>
                {inTrios.length === 0 ? null : (
                  <optgroup label={t("Trios with this deck")}>
                    {inTrios.map((trio) => (
                      <option key={trio.id} value={`trio:${trio.id}`}>
                        {t("{trio}: its other decks", { trio: trioLabel(trio, limits.nameLength) })}
                      </option>
                    ))}
                  </optgroup>
                )}
                {others.length === 0 ? null : (
                  <optgroup label={t("Other decks")}>
                    {others.map((other) => (
                      <option key={other.id} value={`deck:${other.id}`}>
                        {deckLabel(other, limits.nameLength)}
                      </option>
                    ))}
                  </optgroup>
                )}
                {compare.kind === "none" ? null : <option value="none">{t("Stop comparing")}</option>}
              </select>
            </label>
            {compared.length === 0 ? null : (
              <ul className="ws-chips" aria-label={t("Compared decks")}>
                {compared.map((other) => {
                  const otherLabel = deckLabel(other, limits.nameLength);
                  return (
                    <li key={other.id}>
                      <button
                        type="button"
                        className="ws-chip-button"
                        data-testid={deckCompareChipId(other.id)}
                        aria-label={t("Stop comparing with {deck}", { deck: otherLabel })}
                        onClick={() => {
                          setCompare((current) => withoutComparedDeck(current, compared, other.id));
                        }}
                      >
                        <span>{otherLabel}</span>
                        <span aria-hidden="true">×</span>
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
            {compared.length === 0 ? null : (
              <p className="ws-hint">{t("Cards in these decks are marked in the pool and can’t be added here.")}</p>
            )}
            {conflicts.size === 0 ? null : (
              <p className="ws-conflicts" data-testid={DECK_CONFLICTS} data-count={String(conflicts.size)}>
                {conflicts.size === 1
                  ? t("{count} card here is also in {names}. They stay until you take them out.", { count: conflicts.size, names: conflictNames })
                  : t("{count} cards here are also in {names}. They stay until you take them out.", { count: conflicts.size, names: conflictNames })}
              </p>
            )}
          </div>

          <section className="ws-verdict" data-testid={DECK_VERDICT} data-ready={verdict.ok ? "true" : "false"} aria-labelledby={verdictTitleId}>
            <h3 className="ws-verdict-title" id={verdictTitleId}>
              {verdict.ok ? (collection === null ? t("Complete") : t("Ready to queue")) : t("Before you can queue this deck")}
            </h3>
            <ul
              className="db-errors"
              data-testid={LOADOUT_ERRORS}
              data-count={String(errors.length)}
              data-tone="hint"
              aria-labelledby={verdictTitleId}
            >
              {errors.map((issue, position) => (
                <li
                  key={`${issue.rule}:${issue.cardId ?? ""}:${String(position)}`}
                  data-testid={loadoutErrorId(issue.rule)}
                  data-rule={issue.rule}
                  data-deck={issue.deck === undefined ? undefined : String(issue.deck)}
                  data-card={issue.cardId}
                  data-source="client"
                >
                  {t(issue.message)}
                </li>
              ))}
            </ul>
            {collection === null ? (
              <p className="ws-hint">{t("Your collection couldn’t be loaded, so cards you don’t own aren’t flagged here.")}</p>
            ) : null}
          </section>

          <div className="ws-actions">
            <button type="button" className="ws-action" data-testid={DECK_COPY_CODE} onClick={copyCode}>
              {t("Copy code")}
            </button>
            <button
              type="button"
              className="ws-action ws-action--danger"
              data-testid={DECK_DELETE}
              aria-expanded={confirming}
              onClick={() => {
                setConfirming(true);
              }}
            >
              {t("Delete deck")}
            </button>
          </div>
          {codeShown ? (
            <label className="ws-code">
              <span className="ws-field-label">{t("Deck code")}</span>
              <input
                className="ws-code-input"
                data-testid={DECK_CODE_OUTPUT}
                readOnly
                value={code}
                onFocus={(event) => {
                  event.currentTarget.select();
                }}
              />
            </label>
          ) : null}
          {confirming ? (
            <div className="ws-confirm" role="group" aria-labelledby={confirmTextId}>
              <p id={confirmTextId}>
                {triosNamingIt === 0
                  ? t("Delete “{deck}”? This can’t be undone.", { deck: label })
                  : triosNamingIt === 1
                    ? t("Delete “{deck}”? Its slot in {count} trio will be empty. This can’t be undone.", { deck: label, count: triosNamingIt })
                    : t("Delete “{deck}”? Its slot in {count} trios will be empty. This can’t be undone.", { deck: label, count: triosNamingIt })}
              </p>
              <div className="ws-confirm-actions">
                <button type="button" className="ws-action ws-action--danger" data-testid={DECK_DELETE_CONFIRM} onClick={onDelete}>
                  {t("Delete")}
                </button>
                <button
                  ref={keepButton}
                  type="button"
                  className="ws-action"
                  data-testid={DECK_DELETE_CANCEL}
                  onClick={() => {
                    setConfirming(false);
                  }}
                >
                  {t("Keep it")}
                </button>
              </div>
            </div>
          ) : null}
        </DeckSidebar>

        <CardBrowser
          catalog={catalog}
          pool={pool}
          filter={filter}
          onFilter={onFilter}
          sort={sort}
          onSort={onSort}
          ownedUnavailable={collection === null}
          deck={{
            deckName: label,
            collection,
            inDeck,
            holders,
            refusedCardId,
            onAdd: add,
            onDragStart: startDrag,
            onDragEnd: endDrag,
          }}
          status={
            // A non-blocking, polite toast keeps pool-side changes audible on phones (deckbuilder.css).
            <p className="db-deck-status" data-testid={DECK_STATUS} role="status" aria-live="polite">
              {shown ?? ""}
            </p>
          }
          onInspect={openDetail}
          detail={detail}
          showStats
          onCloseDetail={() => {
            setDetailCardId(null);
          }}
        />
      </div>
    </section>
  );
}
