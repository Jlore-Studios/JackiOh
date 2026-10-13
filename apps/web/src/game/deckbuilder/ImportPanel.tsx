// The import panel: paste a deck code, see what it holds, and make it a new deck (SPEC §9.4,
// R255). An import always makes a NEW deck, so it never overwrites work.
//
// Cards the player does not own are KEPT and only flagged: they are judged when the deck is queued
// (R253). At the deck cap the import is off, with the reason beside it; the pasted code stays.

import { useEffect, useId, useMemo, useRef, useState, type ReactElement } from "react";

import type { CatalogSnapshot, Collection } from "@jackioh/validator";

import { DECK_CODE_VERSION } from "@jackioh/server-config";
import { t } from "../../i18n/i18n.ts";
import { DECK_CODE_PREFIX, decodeDeckCode, type DroppedCards } from "./deckCode.ts";
import { DECK_SIZE, MAX_COPIES } from "./deckSize.ts";
import {
  DECK_IMPORT,
  DECK_IMPORT_CANCEL,
  DECK_IMPORT_CAP_REASON,
  DECK_IMPORT_INPUT,
  DECK_IMPORT_PREVIEW,
  DECK_IMPORT_SUBMIT,
  WORKSHOP_BACK,
} from "./testids.ts";
import { cardNames, cardsWord } from "./workshop.ts";

export type ImportPanelProps = {
  catalog: CatalogSnapshot;
  collection: Collection | null;
  /** The player already has `deckLimit` decks: an import would be refused at the server. */
  atCap: boolean;
  deckLimit: number;
  onImport: (name: string, cards: readonly string[]) => void;
  onCancel: () => void;
};

/** One sentence per kind of card the code had and the new deck will not. */
export function droppedLines(dropped: DroppedCards, catalog: CatalogSnapshot): string[] {
  const lines: string[] = [];
  if (dropped.unknown.length > 0) {
    const numbers = dropped.unknown.map((number) => `#${String(number)}`).join(t(", "));
    lines.push(t("Left out {cards} this version of JackiOh doesn’t know ({numbers}).", { cards: cardsWord(dropped.unknown.length), numbers }));
  }
  if (dropped.tokens.length > 0) {
    lines.push(t("Left out {names}: a Token never goes in a deck.", { names: cardNames(dropped.tokens, catalog) }));
  }
  if (dropped.duplicates.length > 0) {
    const count = dropped.duplicates.length;
    const names = cardNames([...new Set(dropped.duplicates)], catalog);
    lines.push(
      count === 1
        ? t("Left out {count} extra copy (a deck holds {max} of each card): {names}.", { count, max: MAX_COPIES, names })
        : t("Left out {count} extra copies (a deck holds {max} of each card): {names}.", { count, max: MAX_COPIES, names }),
    );
  }
  if (dropped.overflow.length > 0) {
    lines.push(
      t("Left out {cards} past the {size} a deck holds: {names}.", {
        cards: cardsWord(dropped.overflow.length),
        size: DECK_SIZE,
        names: cardNames(dropped.overflow, catalog),
      }),
    );
  }
  return lines;
}

export default function ImportPanel(props: ImportPanelProps): ReactElement {
  const { catalog, collection, atCap, deckLimit, onImport, onCancel } = props;
  const [text, setText] = useState("");
  const input = useRef<HTMLTextAreaElement>(null);
  const titleId = useId();
  const inputId = useId();
  const reasonId = useId();

  useEffect(() => {
    input.current?.focus();
  }, []);

  const decoded = useMemo(
    () => (text.trim().length === 0 ? null : decodeDeckCode(text, catalog, collection)),
    [text, catalog, collection],
  );

  return (
    <section className="ws-editor ws-import" data-testid={DECK_IMPORT} aria-labelledby={titleId}>
      <div className="ws-editor-head">
        <button type="button" className="ws-back" data-testid={WORKSHOP_BACK} onClick={onCancel}>
          {t("← All decks")}
        </button>
        <h2 className="ws-import-title" id={titleId}>
          {t("Import a deck")}
        </h2>
      </div>
      <p className="ws-hint">{t("Paste a deck code someone shared. It becomes a new deck; nothing you have is changed.")}</p>
      <label className="ws-field-label" htmlFor={inputId}>
        {t("Deck code")}
      </label>
      <textarea
        ref={input}
        id={inputId}
        className="ws-import-input"
        data-testid={DECK_IMPORT_INPUT}
        value={text}
        rows={3}
        spellCheck={false}
        autoComplete="off"
        autoCapitalize="off"
        placeholder={`${DECK_CODE_PREFIX}${String(DECK_CODE_VERSION)}.…`}
        onChange={(event) => {
          setText(event.target.value);
        }}
      />

      <div
        className="ws-import-preview"
        data-testid={DECK_IMPORT_PREVIEW}
        data-ok={decoded === null ? undefined : decoded.ok ? "true" : "false"}
        aria-live="polite"
      >
        {decoded === null ? null : decoded.ok ? (
          <>
            <p className="ws-import-name">
              <strong>{decoded.name}</strong>
              {` · ${t("{count}/{size} cards", { count: decoded.cards.length, size: DECK_SIZE })}`}
            </p>
            {decoded.nameFellBack ? <p>{t("The code’s name couldn’t be used, so the deck gets this one. You can rename it.")}</p> : null}
            <ul className="ws-import-notes">
              {droppedLines(decoded.dropped, catalog).map((line) => (
                <li key={line} data-kind="dropped">
                  {line}
                </li>
              ))}
              {decoded.unowned.length === 0 ? null : (
                <li data-kind="unowned">
                  {decoded.unowned.length === 1
                    ? t("{cards} you don’t own yet is kept: {names}. You’ll need it before this deck can queue.", {
                        cards: cardsWord(decoded.unowned.length),
                        names: cardNames(decoded.unowned, catalog),
                      })
                    : t("{cards} you don’t own yet are kept: {names}. You’ll need them before this deck can queue.", {
                        cards: cardsWord(decoded.unowned.length),
                        names: cardNames(decoded.unowned, catalog),
                      })}
                </li>
              )}
            </ul>
          </>
        ) : (
          <p className="notice" role="alert">
            {t(decoded.message)}
          </p>
        )}
      </div>

      {atCap ? (
        <p className="ws-cap-note" id={reasonId} data-testid={DECK_IMPORT_CAP_REASON}>
          {t("You have {count} decks, the most you can keep. Delete one to import this.", { count: deckLimit })}
        </p>
      ) : null}

      <div className="ws-actions">
        <button
          type="button"
          className="ws-action ws-action--primary"
          data-testid={DECK_IMPORT_SUBMIT}
          disabled={decoded === null || !decoded.ok || atCap}
          aria-describedby={atCap ? reasonId : undefined}
          onClick={() => {
            if (decoded === null || !decoded.ok || atCap) return;
            onImport(decoded.name, decoded.cards);
          }}
        >
          {t("Import as new deck")}
        </button>
        <button type="button" className="ws-action" data-testid={DECK_IMPORT_CANCEL} onClick={onCancel}>
          {t("Cancel")}
        </button>
      </div>
    </section>
  );
}
