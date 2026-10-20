// The Jlore shop (#1309, SPEC §14.1, §14.3): the wallet and one row per pack kind, bought at
// the server's pack price.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The price comes from `GET /api/packs` and the balance
// from `GET /api/wallet`; a short wallet disables the button and says the shortfall, and a
// refused buy shows the server's words verbatim with nothing changed on screen (no optimistic
// update: the wallet and the counts move only on the answer).

import { useEffect, useState, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import {
  buyPack,
  getPacks,
  getWallet,
  type BoughtPackResponse,
  type PacksResponse,
  type WalletResponse,
} from "../net/api.ts";
import { packName } from "../packs/packs.ts";
import { shopPacks, shopTestid, shortfallOf } from "./shop.ts";
import "./shop.css";

/** What a failed request says: the server's words, else the error's. */
function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** The counts after a bought pack: the answer's count for its kind, every other kind untouched. */
function withBought(
  unopened: PacksResponse["unopened"],
  bought: BoughtPackResponse,
): PacksResponse["unopened"] {
  const counts = new Map(unopened.map((entry) => [entry.pack, entry.count]));
  counts.set(bought.pack, bought.unopened);
  return shopPacks()
    .filter((pack) => (counts.get(pack) ?? 0) > 0)
    .map((pack) => ({ pack, count: counts.get(pack) ?? 0 }));
}

/**
 * The panel. `revision` is how many times the page has asked it to read again (another panel
 * changed the account); `onChanged` tells the page a bought pack changed it.
 */
export default function Shop({
  token,
  revision = 0,
  onChanged,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
}): ReactElement {
  const [info, setInfo] = useState<PacksResponse | null>(null);
  const [wallet, setWallet] = useState<WalletResponse | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    Promise.all([getPacks(token), getWallet(token)]).then(
      ([packs, balance]) => {
        if (!cancelled) {
          setInfo(packs);
          setWallet(balance);
        }
      },
      (cause: unknown) => {
        if (!cancelled) setLoadError(wordsOf(cause));
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token, revision]);

  const buy = (pack: string): void => {
    if (busy !== null) return;
    setBusy(pack);
    setError(null);
    buyPack(token, pack).then(
      (bought) => {
        setWallet({ jlore: bought.jlore });
        setInfo((before) => (before === null ? before : { ...before, unopened: withBought(before.unopened, bought) }));
        onChanged?.();
      },
      (cause: unknown) => {
        setError(wordsOf(cause));
      },
    ).finally(() => {
      setBusy(null);
    });
  };

  return (
    <div className="form-card shop-panel" data-testid={shopTestid.panel}>
      <h3 className="shop-heading">{t("Jlore shop")}</h3>
      {loadError === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.loadError}>
          {loadError}
        </p>
      )}
      {info === null || wallet === null ? (
        loadError === null ? (
          <p className="notice" role="status" data-testid={shopTestid.loading}>
            {t("Loading your shop…")}
          </p>
        ) : null
      ) : (
        <>
          <p className="shop-wallet" data-testid={shopTestid.wallet}>
            {t("{jlore} Jlore", { jlore: wallet.jlore })}
          </p>
          {shopPacks().map((pack) => {
            const short = shortfallOf(info.price, wallet.jlore);
            const held = info.unopened.find((entry) => entry.pack === pack)?.count ?? 0;
            return (
              <div key={pack} className="shop-row" data-testid={shopTestid.held(pack)}>
                <span>{t("{pack}: {count} unopened", { pack: packName(pack), count: held })}</span>
                <span>{t("{price} Jlore", { price: info.price })}</span>
                <div className="account-actions">
                  <button
                    type="button"
                    data-testid={shopTestid.buy(pack)}
                    disabled={short > 0 || busy !== null}
                    onClick={() => {
                      buy(pack);
                    }}
                  >
                    {busy === pack ? t("Buying…") : t("Buy")}
                  </button>
                </div>
                {short > 0 ? (
                  <p className="notice" data-testid={shopTestid.shortfall(pack)}>
                    {t("Short by {shortfall} Jlore", { shortfall: short })}
                  </p>
                ) : null}
              </div>
            );
          })}
        </>
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.error}>
          {error}
        </p>
      )}
    </div>
  );
}
