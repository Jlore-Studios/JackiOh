// The Jlore shop (#1309, #1667, SPEC §14.1, §14.3): the wallet and one shelf row per pack kind, each
// a `PackObject` with its count, its price and Buy.
// THE SERVER DECIDES (CLAUDE.md rule 7): the price comes from `GET /api/packs` and the balance from
// `GET /api/wallet`; a refused buy shows the server's words through `t()` (R711000), and nothing moves
// before the answer.
// ONE SPEND AT A TIME (R2378000): while any spend of the tab is out (`net/spending.ts`), every Buy
// waits, disabled and turning a loading wheel (`economy/spend.css`), and a press sends nothing.

import { useEffect, useState, type CSSProperties, type ReactElement } from "react";

import { useReducedMotion } from "../economy/FillBar.tsx";
import { announceWallet } from "../economy/wallet.ts";
import "../economy/spend.css";
import { t } from "../i18n/i18n.ts";
import {
  buyPack,
  getPacks,
  getWallet,
  keptPaths,
  type BoughtPackResponse,
  type PacksResponse,
  type WalletResponse,
} from "../net/api.ts";
import { cachedRead } from "../net/readCache.ts";
import { spendOutNow, useSpendOut } from "../net/spending.ts";
import PackObject from "../packs/PackObject.tsx";
import { packName } from "../packs/packs.ts";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { SHOP_FLY_MS, shopPacks, shopTestid, shortfallOf } from "./shop.ts";
import "./shop.css";

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

/** `revision` counts the page's asks to read again; `onChanged` tells it a bought pack changed the account. */
export default function Shop({
  token,
  revision = 0,
  onChanged,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
}): ReactElement {
  // R2125000: a revisit starts on the answers the last visit kept and reads again behind them.
  const [info, setInfo] = useState<PacksResponse | null>(
    () => cachedRead<PacksResponse>(token, keptPaths.packs) ?? null,
  );
  const [wallet, setWallet] = useState<WalletResponse | null>(
    () => cachedRead<WalletResponse>(token, keptPaths.wallet) ?? null,
  );
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);
  // How many packs of each kind were bought this visit: a new count mounts a new flying copy.
  const [flights, setFlights] = useState<Readonly<Record<string, number>>>({});
  const reduced = useReducedMotion();
  const spendOut = useSpendOut();

  useEffect(() => {
    let canceled = false;
    setReading(true);
    Promise.all([getPacks(token), getWallet(token)]).then(
      ([packs, balance]) => {
        if (!canceled) {
          setInfo(packs);
          setWallet(balance);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!canceled) {
          setReading(false);
          // R2125000: a read that fails behind kept answers leaves them and says nothing; only a first visit says why.
          const keptInfo = cachedRead<PacksResponse>(token, keptPaths.packs);
          const keptWallet = cachedRead<WalletResponse>(token, keptPaths.wallet);
          if (keptInfo === undefined || keptWallet === undefined) {
            setLoadError(wordsOf(cause));
          } else {
            setInfo(keptInfo);
            setWallet(keptWallet);
            setLoadError(null);
          }
        }
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, revision, attempt]);

  const buy = (pack: string): void => {
    if (busy !== null || spendOutNow()) return;
    setBusy(pack);
    setError(null);
    buyPack(token, pack).then(
      (bought) => {
        setWallet({ jlore: bought.jlore });
        setInfo((before) => (before === null ? before : { ...before, unopened: withBought(before.unopened, bought) }));
        setFlights((before) => ({ ...before, [bought.pack]: (before[bought.pack] ?? 0) + 1 }));
        announceWallet(bought.jlore);
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
        <>
          <p className="notice" role="alert" data-testid={shopTestid.loadError}>
            {t(loadError)}
          </p>
          <div className="account-actions">
            <button
              type="button"
              data-testid={shopTestid.retry}
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
      {info === null || wallet === null ? (
        loadError === null ? (
          <Skeleton variant="rows" label={t("Loading your shop…")} testId={shopTestid.loading} />
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
                <div className="shop-row__pack">
                  <PackObject
                    pack={pack}
                    size="shelf"
                    state={busy === pack ? "pressed" : "idle"}
                    testid={shopTestid.packObject(pack)}
                  />
                  {flights[pack] === undefined || reduced ? null : (
                    <span
                      key={flights[pack]}
                      className="shop-fly"
                      aria-hidden="true"
                      data-testid={shopTestid.fly(pack)}
                      style={{ "--shop-fly-ms": `${String(SHOP_FLY_MS)}ms` } as CSSProperties}
                    >
                      <PackObject pack={pack} size="shelf" />
                    </span>
                  )}
                </div>
                <div className="shop-row__info">
                  <span className="shop-row__count">
                    {t("{pack}: {count} unopened", { pack: packName(pack), count: held })}
                  </span>
                  <span>{t("{price} Jlore", { price: info.price })}</span>
                  <div className="account-actions">
                    <button
                      type="button"
                      className="spend-wait"
                      data-testid={shopTestid.buy(pack)}
                      disabled={short > 0 || busy !== null || spendOut}
                      aria-busy={spendOut && short === 0}
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
              </div>
            );
          })}
        </>
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.error}>
          {t(error)}
        </p>
      )}
    </div>
  );
}
