// The token picker (#1309, SPEC §14.5): the bars, the next step and the held counts. R1953001, R2730000:
// a token is spent in the Collection, a cosmetic or card token from a locked item's fullscreen preview
// (collection/TokenOffer.tsx), so the shop keeps no pick or card browser and links there. THE SERVER
// DECIDES (CLAUDE.md rule 7): the bars, the next step and the held counts come from `GET /api/tokens`.

import { useEffect, useState, type ReactElement } from "react";

import FillBar from "../economy/FillBar.tsx";
import { t } from "../i18n/i18n.ts";
import { getTokens, keptPaths, type OpeningBarsAnswer, type TokenBarsResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { nextStepWords, shopTestid, withOpeningBars } from "./shop.ts";
import { messageOf } from "../net/errors.ts";
import "./shop.css";

/**
 * The panel. `revision` is how many times the page has asked it to read again (the held counts, which no
 * spend's answer carries); `opening` is the last opening's bars, whose points and sizes fold in with no read
 * (R3211000).
 */
export default function TokenPicker({
  token,
  revision = 0,
  opening,
}: {
  token: string;
  revision?: number;
  opening?: OpeningBarsAnswer | null;
}): ReactElement {
  // R2125000: a revisit starts on the answers the last visit kept and reads again behind them.
  const [bars, setBars] = useState<TokenBarsResponse | null>(
    () => cachedRead<TokenBarsResponse>(token, keptPaths.tokens) ?? null,
  );
  const [loadError, setLoadError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);

  useEffect(() => {
    let canceled = false;
    setReading(true);
    getTokens(token).then(
      (barsAnswer) => {
        if (!canceled) {
          setBars(barsAnswer);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!canceled) {
          setReading(false);
          // R2125000: a read that fails behind kept answers leaves them and says nothing; only a first visit says why.
          const keptBars = cachedRead<TokenBarsResponse>(token, keptPaths.tokens);
          if (keptBars === undefined) {
            setLoadError(messageOf(cause));
          } else {
            setBars(keptBars);
            setLoadError(null);
          }
        }
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, revision, attempt]);

  useEffect(() => {
    if (opening === undefined || opening === null) return;
    setBars((before) => (before === null ? before : withOpeningBars(before, opening)));
  }, [opening]);

  return (
    <div className="form-card token-panel" data-testid={shopTestid.tokenPanel}>
      <h2 className="shop-heading">{t("Card and cosmetic tokens")}</h2>
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
      {bars === null ? (
        loadError === null ? (
          <Skeleton variant="panel" label={t("Loading your tokens…")} testId={shopTestid.tokenLoading} />
        ) : null
      ) : (
        <TokenBody bars={bars} />
      )}
    </div>
  );
}

function TokenBody({ bars }: { bars: TokenBarsResponse }): ReactElement {
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

      <h3 className="shop-subheading">{t("Card tokens")}</h3>
      <p className="token-held" data-testid={shopTestid.heldCard}>
        {t("Card tokens held: {count}", { count: bars.tokens.card })}
      </p>
      {bars.tokens.card < 1 ? (
        <p className="token-held" data-testid={shopTestid.cardHint}>
          {t("A full card token bar pays a card token.")}
        </p>
      ) : null}

      <h3 className="shop-subheading">{t("Cosmetic tokens")}</h3>
      <p className="token-held" data-testid={shopTestid.heldRare}>
        {t("Rare tokens held: {count}", { count: bars.tokens.cosmeticRare })}
      </p>
      <p className="token-held" data-testid={shopTestid.heldEpic}>
        {t("Epic tokens held: {count}", { count: bars.tokens.cosmeticEpic })}
      </p>
      <p className="token-held" data-testid={shopTestid.heldLegendary}>
        {t("Legendary tokens held: {count}", { count: bars.tokens.cosmeticLegendary })}
      </p>
      <p>
        <a href={paths.collection} data-testid={shopTestid.cosmeticSpend} onClick={followInApp(paths.collection)}>
          {t("Hold a locked card or cosmetic in your Collection to spend a token")}
        </a>
      </p>
    </>
  );
}
