// The token picker (#1309, SPEC §14.5): both token bars, the cosmetic cycle's next step, the
// card token's pick of an unowned card and the cosmetic tokens held. R1953001: a cosmetic token is
// spent where a locked cosmetic is shown, in the Collection (collection/TokenOffer.tsx), so the shop
// keeps no pick of one and links there.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The bars, their sizes, the next step and the held
// counts come from `GET /api/tokens`; the filter only chooses what to browse. A redemption sends
// intent and renders the answer: success re-reads the bars (and the collection, so the unlocked
// card leaves the list), refusal shows the server's words through `t()` (R711000) with nothing
// changed on screen.

import { useEffect, useId, useMemo, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CardDefsProvider, closeInspect } from "../cards/index.ts";
import { readerName } from "../cards/chinese.ts";
import FillBar from "../economy/FillBar.tsx";
import CardBrowser from "../game/deckbuilder/CardBrowser.tsx";
import { DEFAULT_FILTER, DEFAULT_SORT, type PoolFilter, type PoolSort } from "../game/deckbuilder/filters.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import {
  getCollection,
  getTokens,
  keptPaths,
  redeemCardToken,
  type CollectionResponse,
  type TokenBarsResponse,
} from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { nextStepWords, shopTestid, tokenPoolIds, unownedCards } from "./shop.ts";
import { messageOf } from "../net/errors.ts";
import "../game/deckbuilder/deckbuilder.css";
import "./shop.css";

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
  // R2125000: a revisit starts on the answers the last visit kept and reads again behind them.
  const [bars, setBars] = useState<TokenBarsResponse | null>(
    () => cachedRead<TokenBarsResponse>(token, keptPaths.tokens) ?? null,
  );
  const [collection, setCollection] = useState<CollectionResponse | null>(
    () => cachedRead<CollectionResponse>(token, keptPaths.collection) ?? null,
  );
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);

  useEffect(() => {
    let canceled = false;
    setReading(true);
    Promise.all([getTokens(token), getCollection(token)]).then(
      ([barsAnswer, collectionAnswer]) => {
        if (!canceled) {
          setBars(barsAnswer);
          setCollection(collectionAnswer);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!canceled) {
          setReading(false);
          // R2125000: a read that fails behind kept answers leaves them and says nothing; only a first visit says why.
          const keptBars = cachedRead<TokenBarsResponse>(token, keptPaths.tokens);
          const keptCollection = cachedRead<CollectionResponse>(token, keptPaths.collection);
          if (keptBars === undefined || keptCollection === undefined) {
            setLoadError(messageOf(cause));
          } else {
            setBars(keptBars);
            setCollection(keptCollection);
            setLoadError(null);
          }
        }
      },
    );
    return () => {
      canceled = true;
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
            setError(messageOf(cause));
          },
        );
      },
      (cause: unknown) => {
        setError(messageOf(cause));
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
            {t(loadError)}
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
          <Skeleton variant="panel" label={t("Loading your tokens…")} testId={shopTestid.tokenLoading} />
        ) : null
      ) : (
        <TokenBody bars={bars} collection={collection} busy={busy} onPickCard={pickCard} />
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.tokenError}>
          {t(error)}
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
  // The browse pane's filter, sort, open detail view and pending confirm live here, so the page's
  // reads (bars, collection) still own what the server decided (CLAUDE.md rule 7).
  const [filter, setFilter] = useState<PoolFilter>({ ...DEFAULT_FILTER, ownedOnly: false });
  const [sort, setSort] = useState<PoolSort>(DEFAULT_SORT);
  const [detailCardId, setDetailCardId] = useState<string | null>(null);
  // Whether the open detail asks before it spends the token.
  const [asking, setAsking] = useState(false);
  const questionId = useId();
  const locale = useLocale();
  const catalog = useMemo(() => ({ version: collection.catalogVersion, cards: CATALOG }), [collection.catalogVersion]);
  const unowned = useMemo(() => unownedCards(collection), [collection]);
  const pool = useMemo(() => tokenPoolIds(unowned, catalog, filter, sort), [unowned, catalog, filter, sort, locale]);
  const detailDef = detailCardId === null ? undefined : CATALOG[detailCardId];
  // The detail closes behind the inspect overlay, as in the deck editor: at most one at a time.
  const openDetail = (cardId: string): void => {
    closeInspect();
    setAsking(false);
    setDetailCardId(cardId);
  };
  const closeDetail = (): void => {
    setAsking(false);
    setDetailCardId(null);
  };
  // The question takes Unlock's place in the detail's own actions row: the detail is a modal over
  // the page (CardDetail.tsx), so a confirm drawn anywhere else lies under its scrim, out of Tab's
  // reach. Cancel, which spends nothing, takes the focus.
  const detail =
    detailCardId === null || detailDef === undefined
      ? null
      : {
          cardId: detailCardId,
          actions: asking ? (
            <div className="token-confirm" role="alertdialog" aria-labelledby={questionId} data-testid={shopTestid.cardDialog}>
              <p id={questionId}>{t("Unlock {name}? This spends 1 card token.", { name: readerName(detailDef) })}</p>
              <div className="token-confirm__actions">
                <button
                  type="button"
                  data-testid={shopTestid.cardConfirm}
                  onClick={() => {
                    onPickCard(detailCardId);
                    closeDetail();
                  }}
                >
                  {t("Confirm")}
                </button>
                <button
                  type="button"
                  data-testid={shopTestid.cardCancel}
                  autoFocus
                  onClick={() => {
                    setAsking(false);
                  }}
                >
                  {t("Cancel")}
                </button>
              </div>
            </div>
          ) : (
            <button
              type="button"
              data-testid={shopTestid.cardPick(detailCardId)}
              disabled={bars.tokens.card < 1 || busy !== null}
              onClick={() => {
                setAsking(true);
              }}
            >
              {busy === detailCardId ? t("Redeeming…") : t("Unlock")}
            </button>
          ),
        };
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
      {unowned.length === 0 ? (
        <p data-testid={shopTestid.cardNone}>{t("No cards to pick.")}</p>
      ) : (
        <CardDefsProvider defs={CATALOG}>
          <CardBrowser
            catalog={catalog}
            pool={pool}
            filter={filter}
            onFilter={setFilter}
            sort={sort}
            onSort={setSort}
            ownedControl={false}
            onInspect={openDetail}
            detail={detail}
            onCloseDetail={closeDetail}
          />
        </CardDefsProvider>
      )}
      {bars.tokens.card < 1 ? (
        <p className="token-held" data-testid={shopTestid.cardHint}>
          {t("A full card token bar pays a card token.")}
        </p>
      ) : null}

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
