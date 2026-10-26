// The token picker (#1309, SPEC §14.5): both token bars, the cosmetic cycle's next step, the
// card token's pick of an unowned card and the cosmetic tokens held. R1953001: a cosmetic token is
// spent where a locked cosmetic is shown, in the Collection (collection/TokenOffer.tsx), so the shop
// keeps no pick of one and links there.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The bars, their sizes, the next step and the held
// counts come from `GET /api/tokens`; the filter only chooses what to browse. A redemption sends
// intent and renders the answer: success re-reads the bars (and the collection, so the unlocked
// card leaves the list), refusal shows the server's words verbatim with nothing changed on screen.

import { useEffect, useState, type ReactElement } from "react";

import FillBar from "../economy/FillBar.tsx";
import { t } from "../i18n/i18n.ts";
import { getCollection, getTokens, redeemCardToken, type CollectionResponse, type TokenBarsResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { followInApp } from "../routes/nav.tsx";
import { nextStepWords, shopTestid, unownedCards } from "./shop.ts";
import "./shop.css";

/** What a failed request says: the server's words, else the error's. */
function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/**
 * The panel. `revision` is how many times the page has asked it to read again (another panel
 * changed the account); `onChanged` tells the page a redemption changed it.
 */
export default function TokenPicker({
  token,
  revision = 0,
  onChanged,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
}): ReactElement {
  const [bars, setBars] = useState<TokenBarsResponse | null>(null);
  const [collection, setCollection] = useState<CollectionResponse | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setReading(true);
    Promise.all([getTokens(token), getCollection(token)]).then(
      ([barsAnswer, collectionAnswer]) => {
        if (!cancelled) {
          setBars(barsAnswer);
          setCollection(collectionAnswer);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!cancelled) {
          setLoadError(wordsOf(cause));
          setReading(false);
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token, revision, attempt]);

  const pickCard = (cardId: string): void => {
    if (busy !== null) return;
    setBusy(cardId);
    setError(null);
    redeemCardToken(token, cardId).then(
      (answered) => {
        setBars(answered);
        onChanged?.();
        // The collection moved too: read it back, so the unlocked card leaves the list.
        getCollection(token).then(
          (next) => {
            setCollection(next);
          },
          (cause: unknown) => {
            setError(wordsOf(cause));
          },
        );
      },
      (cause: unknown) => {
        setError(wordsOf(cause));
      },
    ).finally(() => {
      setBusy(null);
    });
  };

  return (
    <div className="form-card token-panel" data-testid={shopTestid.tokenPanel}>
      <h3 className="shop-heading">{t("Card and cosmetic tokens")}</h3>
      {loadError === null ? null : (
        <>
          <p className="notice" role="alert" data-testid={shopTestid.tokenLoadError}>
            {loadError}
          </p>
          <div className="account-actions">
            <button
              type="button"
              data-testid={shopTestid.tokenRetry}
              disabled={reading}
              onClick={() => {
                setAttempt((before) => before + 1);
              }}
            >
              {t("Try again")}
            </button>
          </div>
        </>
      )}
      {bars === null || collection === null ? (
        loadError === null ? (
          <p className="notice" role="status" data-testid={shopTestid.tokenLoading}>
            {t("Loading your tokens…")}
          </p>
        ) : null
      ) : (
        <TokenBody bars={bars} collection={collection} busy={busy} onPickCard={pickCard} />
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.tokenError}>
          {error}
        </p>
      )}
    </div>
  );
}

/** The bars, the next step, the card picker and the cosmetic tokens held, once every read has answered. */
function TokenBody({
  bars,
  collection,
  busy,
  onPickCard,
}: {
  bars: TokenBarsResponse;
  collection: CollectionResponse;
  busy: string | null;
  onPickCard: (cardId: string) => void;
}): ReactElement {
  const cards = unownedCards(collection);
  return (
    <>
      <FillBar
        label={t("Card token bar")}
        size={bars.cardBar.size}
        value={bars.cardBar.points}
        testid={shopTestid.cardBar}
      />
      <div className="fill-bar-row">
        <FillBar
          label={t("Cosmetic token bar")}
          size={bars.cosmeticBar.size}
          value={bars.cosmeticBar.points}
          testid={shopTestid.cosmeticBar}
        />
        <p className="token-next" data-testid={shopTestid.nextStep}>
          {t("Next: {step}", { step: nextStepWords(bars.nextStep) })}
        </p>
      </div>

      <h4 className="shop-subheading">{t("Card tokens")}</h4>
      <p className="token-held" data-testid={shopTestid.heldCard}>
        {t("Card tokens held: {count}", { count: bars.tokens.card })}
      </p>
      {cards.length === 0 ? (
        <p data-testid={shopTestid.cardNone}>{t("No cards to pick.")}</p>
      ) : (
        cards.map((card) => (
          <div key={card.id} className="token-row">
            <span>{card.name}</span>
            <div className="account-actions">
              <button
                type="button"
                data-testid={shopTestid.cardPick(card.id)}
                disabled={bars.tokens.card < 1 || busy !== null}
                onClick={() => {
                  onPickCard(card.id);
                }}
              >
                {busy === card.id ? t("Redeeming…") : t("Pick")}
              </button>
            </div>
          </div>
        ))
      )}

      <h4 className="shop-subheading">{t("Cosmetic tokens")}</h4>
      <p className="token-held" data-testid={shopTestid.heldRare}>
        {t("Rare tokens held: {count}", { count: bars.tokens.cosmeticRare })}
      </p>
      <p className="token-held" data-testid={shopTestid.heldEpic}>
        {t("Epic tokens held: {count}", { count: bars.tokens.cosmeticEpic })}
      </p>
      <p>
        <a href={paths.collection} data-testid={shopTestid.cosmeticSpend} onClick={followInApp(paths.collection)}>
          {t("Spend cosmetic tokens on a locked cosmetic in your Collection")}
        </a>
      </p>
    </>
  );
}
