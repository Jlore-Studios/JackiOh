// The token picker (#1309, SPEC §14.5): both token bars, the cosmetic cycle's next step and the
// tokens held. R1953001: a cosmetic token is spent where a locked cosmetic is shown, in the
// Collection (collection/TokenOffer.tsx), so the shop keeps no pick of one and links there.
// R2730000: a card token is spent the same way, from a locked card's fullscreen preview in the
// Collection, so the shop's card browser is gone too: the picker shows the bars, the next step,
// the held counts and the link.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The bars, their sizes, the next step and the held
// counts come from `GET /api/tokens`; nothing here is picked or redeemed.

import { useEffect, useState, type ReactElement } from "react";

import FillBar from "../economy/FillBar.tsx";
import { t } from "../i18n/i18n.ts";
import { getTokens, keptPaths, type TokenBarsResponse } from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { nextStepWords, shopTestid } from "./shop.ts";
import { messageOf } from "../net/errors.ts";
import "./shop.css";

/** The panel. `revision` is how many times the page has asked it to read again (another panel changed the account). */
export default function TokenPicker({ token, revision = 0 }: { token: string; revision?: number }): ReactElement {
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

/** The bars, the next step, the tokens held and the link to where each is spent, once the read has answered. */
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

      <h4 className="shop-subheading">{t("Card tokens")}</h4>
      <p className="token-held" data-testid={shopTestid.heldCard}>
        {t("Card tokens held: {count}", { count: bars.tokens.card })}
      </p>
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
          {t("Hold a locked card or cosmetic in your Collection to spend a token")}
        </a>
      </p>
    </>
  );
}
