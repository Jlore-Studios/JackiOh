// The trio editor: a name, three slots over the saved decks, R253's Conquest verdict, the three
// decks side by side with every shared card marked (SPEC §9.4, R251–R253), and the trio's code
// (R339).
//
// A trio is saved loose (R252): an empty slot or a shared card (R251: a card is its catalog id)
// stops it from queueing Conquest, not from saving. Slots refuse T3's "the same deck twice".

import { useCallback, useEffect, useId, useMemo, useRef, useState, type ReactElement } from "react";

import type { CardCost } from "@jackioh/shared";
import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { CardArt } from "../../cards/index.ts";
import { readerName } from "../../cards/chinese.ts";
import { t, useLocale } from "../../i18n/i18n.ts";
import { DECK_SIZE } from "./deckSize.ts";
import { deckListOrder } from "./filters.ts";
import { UNTITLED_TRIO, type DeckItem, type TrioItem, type WorkshopLimits } from "./sync.ts";
import {
  LOADOUT_ERRORS,
  TRIO_CODE_OUTPUT,
  TRIO_COMPARE,
  TRIO_COPY_CODE,
  TRIO_DELETE,
  TRIO_DELETE_CANCEL,
  TRIO_DELETE_CONFIRM,
  TRIO_EDITOR,
  TRIO_L4_HINT,
  TRIO_NAME_INPUT,
  TRIO_VERDICT,
  WORKSHOP_BACK,
  loadoutErrorId,
  trioCardId,
  trioOpenDeckId,
  trioSlotId,
} from "./testids.ts";
import { encodeTrioCode } from "./trioCode.ts";
import { clampName, deckLabel, joinNames, trioLabel, trioSharedCards, trioSlots, trioVerdict } from "./workshop.ts";
import { conquestText } from "../../routes/conquestCopy.ts";
import type { TrioSlots } from "../../net/api.ts";

export type TrioEditorProps = {
  trio: TrioItem;
  decks: readonly DeckItem[];
  catalog: CatalogSnapshot;
  collection: Collection | null;
  limits: WorkshopLimits;
  /** The server's refusal of this trio's last save, verbatim. */
  refusal: string | null;
  onRename: (name: string) => void;
  onSlots: (slots: TrioSlots) => void;
  onDelete: () => void;
  onOpenDeck: (deckId: string) => void;
  /** A phone's way back to the list. */
  onBack: () => void;
};

/** The tile's gem: the printed price, and an embiggen card's base price, as its full face shows. */
function tileCost(cost: CardCost | undefined): string {
  if (cost === undefined) return "";
  if (typeof cost === "number") return String(cost);
  if (cost === "X") return cost;
  return String(cost.base);
}

export default function TrioEditor(props: TrioEditorProps): ReactElement {
  const { trio, decks, catalog, collection, limits, refusal, onRename, onSlots, onDelete, onOpenDeck, onBack } = props;
  useLocale();
  const label = trioLabel(trio, limits.nameLength);
  const [confirming, setConfirming] = useState(false);
  const [codeShown, setCodeShown] = useState(false);
  const [codeStatus, setCodeStatus] = useState<string | null>(null);
  const keepButton = useRef<HTMLButtonElement>(null);
  const nameId = useId();
  const slotIdBase = useId();
  const verdictTitleId = useId();
  const confirmTextId = useId();

  useEffect(() => {
    if (confirming) keepButton.current?.focus();
  }, [confirming]);

  const slots = useMemo(() => trioSlots(trio, decks), [trio, decks]);
  const shared = useMemo(() => trioSharedCards(slots, limits.nameLength), [slots, limits.nameLength]);
  const verdict = useMemo(
    () => trioVerdict(trio, decks, catalog, collection, limits.nameLength),
    [trio, decks, catalog, collection, limits.nameLength],
  );
  const errors = verdict.ok ? [] : verdict.errors;

  // R339: the trio as a code — its name and each slot's deck, names as they are saved, and an empty
  // slot left empty — shown in a field and copied where the browser allows it, as a deck's is.
  const code = useMemo(
    () =>
      codeShown
        ? encodeTrioCode(
            label,
            slots.map((deck) => (deck === null ? null : { name: deckLabel(deck, limits.nameLength), cards: deck.cards })),
            catalog,
          )
        : "",
    [codeShown, label, slots, limits.nameLength, catalog],
  );
  const copyCode = useCallback(() => {
    setCodeShown(true);
    const text = encodeTrioCode(
      label,
      slots.map((deck) => (deck === null ? null : { name: deckLabel(deck, limits.nameLength), cards: deck.cards })),
      catalog,
    );
    const clipboard = typeof navigator === "undefined" ? undefined : navigator.clipboard;
    const fallback = "Copy the code below to share this trio.";
    if (clipboard === undefined || typeof clipboard.writeText !== "function") {
      setCodeStatus(fallback);
      return;
    }
    // Without clipboard permission the code is still in the field under the button.
    clipboard.writeText(text).then(
      () => {
        setCodeStatus("Trio code copied. Paste it anywhere to share this trio and its three decks.");
      },
      () => {
        setCodeStatus(fallback);
      },
    );
  }, [label, slots, limits.nameLength, catalog]);

  const setSlot = (slot: number, deckId: string | null): void => {
    const next = [...trio.deckIds] as TrioSlots;
    next[slot] = deckId;
    onSlots(next);
  };

  return (
    <section className="ws-editor ws-editor--trio" data-testid={TRIO_EDITOR} data-trio={trio.id} aria-label={t("Trio: {trio}", { trio: label })}>
      <div className="ws-trio">
        <div className="ws-editor-head">
          <button type="button" className="ws-back" data-testid={WORKSHOP_BACK} onClick={onBack}>
            {t("← All decks")}
          </button>
          <label className="ws-name" htmlFor={nameId}>
            <span className="ws-field-label">{t("Trio name")}</span>
            <input
              id={nameId}
              className="ws-name-input"
              data-testid={TRIO_NAME_INPUT}
              value={trio.name}
              placeholder={t(UNTITLED_TRIO)}
              autoComplete="off"
              spellCheck={false}
              onChange={(event) => {
                onRename(clampName(event.target.value, limits.nameLength));
              }}
              onBlur={() => {
                if (trio.name !== label) onRename(label);
              }}
            />
          </label>
          {refusal === null ? null : (
            <p className="notice ws-save-error" role="alert">
              {t(refusal)}
            </p>
          )}
        </div>

        <p className="ws-hint" data-testid={TRIO_L4_HINT}>
          {conquestText("trioL4Hint")}
        </p>

        <div className="ws-slots" role="group" aria-label={t("The trio's decks")}>
          {trio.deckIds.map((deckId, slot) => {
            const selectId = `${slotIdBase}-${String(slot)}`;
            const deck = slots[slot] ?? null;
            return (
              <div key={slot} className="ws-slot">
                <label className="ws-field-label" htmlFor={selectId}>{t("Deck {number}", { number: slot + 1 })}</label>
                <div className="ws-slot-row">
                  <select
                    id={selectId}
                    className="ws-select"
                    data-testid={trioSlotId(slot + 1)}
                    value={deck === null ? "" : (deckId ?? "")}
                    onChange={(event) => {
                      setSlot(slot, event.target.value === "" ? null : event.target.value);
                    }}
                  >
                    <option value="">{t("Empty")}</option>
                    {decks.map((candidate) => {
                      const elsewhere = trio.deckIds.some((id, other) => other !== slot && id === candidate.id);
                      return (
                        <option key={candidate.id} value={candidate.id} disabled={elsewhere}>
                          {elsewhere
                            ? t("{deck} (in another slot)", { deck: deckLabel(candidate, limits.nameLength) })
                            : deckLabel(candidate, limits.nameLength)}
                        </option>
                      );
                    })}
                  </select>
                  {deck === null ? null : (
                    <button
                      type="button"
                      className="ws-action"
                      data-testid={trioOpenDeckId(slot + 1)}
                      aria-label={t("Open {deck}", { deck: deckLabel(deck, limits.nameLength) })}
                      onClick={() => {
                        onOpenDeck(deck.id);
                      }}
                    >
                      {t("Open")}
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>

        <section className="ws-verdict" data-testid={TRIO_VERDICT} data-ready={verdict.ok ? "true" : "false"} aria-labelledby={verdictTitleId}>
          <h3 className="ws-verdict-title" id={verdictTitleId}>
            {verdict.ok ? t("Ready for Conquest") : t("Before this trio can queue Conquest")}
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
                key={`${issue.rule}:${String(issue.deck ?? "")}:${issue.cardId ?? ""}:${String(position)}`}
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

        <div className="ws-compare-grid" data-testid={TRIO_COMPARE} role="group" aria-label={t("The three decks side by side")}>
          {slots.map((deck, slot) => {
            const cards = deck === null ? [] : deckListOrder(deck.cards, catalog);
            const clashes = shared[slot] ?? new Map<string, readonly string[]>();
            return (
              <section key={slot} className="ws-compare-col" data-slot={String(slot + 1)} aria-label={deck === null ? t("Deck {number}: empty", { number: slot + 1 }) : deckLabel(deck, limits.nameLength)}>
                <header className="ws-compare-head">
                  <span className="ws-compare-name">{deck === null ? t("Empty slot") : deckLabel(deck, limits.nameLength)}</span>
                  {deck === null ? null : (
                    <span className="ws-compare-count">{`${String(deck.cards.length)}/${String(DECK_SIZE)}`}</span>
                  )}
                </header>
                {deck === null ? (
                  <p className="ws-hint">{t("Pick a deck for this slot.")}</p>
                ) : cards.length === 0 ? (
                  <p className="ws-hint">{t("No cards yet.")}</p>
                ) : (
                  <ul className="ws-compare-list">
                    {cards.map((cardId, position) => {
                      const def = catalog.cards[cardId];
                      const with_ = clashes.get(cardId);
                      return (
                        <li key={`${cardId}:${String(position)}`}>
                          <span
                            className="db-tile ws-ctile"
                            data-testid={trioCardId(slot + 1, cardId)}
                            data-card={cardId}
                            data-rarity={def?.rarity}
                            data-conflict={with_ === undefined ? "false" : "true"}
                            data-conflict-with={with_ === undefined ? undefined : with_.join(", ")}
                          >
                            <span className="db-tile-cost" data-digits={tileCost(def?.cost).length >= 3 ? "3" : undefined}>
                              {tileCost(def?.cost)}
                            </span>
                            <span className="db-tile-name">{def === undefined ? cardId : readerName(def)}</span>
                            <span className="db-tile-art">
                              {def === undefined ? null : (
                                <CardArt defId={cardId} radiant={false} tags={def.tags} type={def.type} shape="strip" />
                              )}
                            </span>
                            {with_ === undefined ? null : <span className="db-tile-flag" aria-hidden="true" />}
                            <span className="db-tile-pip" data-rarity={def?.rarity} aria-hidden="true" />
                          </span>
                          {with_ === undefined ? null : <span className="ws-also">{t("Also in {deck}", { deck: joinNames(with_) })}</span>}
                        </li>
                      );
                    })}
                  </ul>
                )}
              </section>
            );
          })}
        </div>

        <div className="ws-actions ws-actions--trio">
          <button type="button" className="ws-action" data-testid={TRIO_COPY_CODE} onClick={copyCode}>
            {t("Copy trio code")}
          </button>
          <button
            type="button"
            className="ws-action ws-action--danger"
            data-testid={TRIO_DELETE}
            aria-expanded={confirming}
            onClick={() => {
              setConfirming(true);
            }}
          >
            {t("Delete trio")}
          </button>
        </div>
        {codeStatus === null ? null : (
          <p className="ws-hint" role="status">
            {t(codeStatus)}
          </p>
        )}
        {codeShown ? (
          <label className="ws-code">
            <span className="ws-field-label">{t("Trio code")}</span>
            <input
              className="ws-code-input"
              data-testid={TRIO_CODE_OUTPUT}
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
            <p id={confirmTextId}>{t("Delete “{trio}”? Its decks stay saved. This can’t be undone.", { trio: label })}</p>
            <div className="ws-confirm-actions">
              <button type="button" className="ws-action ws-action--danger" data-testid={TRIO_DELETE_CONFIRM} onClick={onDelete}>
                {t("Delete")}
              </button>
              <button
                ref={keepButton}
                type="button"
                className="ws-action"
                data-testid={TRIO_DELETE_CANCEL}
                onClick={() => {
                  setConfirming(false);
                }}
              >
                {t("Keep it")}
              </button>
            </div>
          </div>
        ) : null}
      </div>
    </section>
  );
}
