// The live card win rates a pool sorts by (R377), shared by the deck editor and the Almanac.
// Until they arrive, and after a failed fetch, the pool is in its tie-break order (cost, then name,
// filters.ts's `sortPool`); the browse pane says so (CardBrowser.tsx) rather than pass it off as
// the win-rate order.

import { useCallback, useEffect, useState } from "react";

import { getCardStats } from "../../net/api.ts";
import type { CardWinRateInfo, SortKey } from "./filters.ts";

/** Where the win rates stand: not asked for, on their way, arrived, or the fetch failed. */
export type WinRatesStatus = "idle" | "loading" | "ready" | "failed";

/** What `useCardWinRates` hands a screen. */
export type CardWinRates = {
  /** Each card's win rate; `null` until a fetch succeeds. */
  rates: Map<string, CardWinRateInfo> | null;
  status: WinRatesStatus;
  /** Asks again after a failed fetch. */
  retry: () => void;
};

/**
 * Each card's win rate, fetched the first time the pool is sorted by it and kept from then on. A
 * failed fetch is asked again by `retry`, or by leaving the win-rate sort and coming back to it.
 */
export function useCardWinRates(sortKey: SortKey): CardWinRates {
  const [rates, setRates] = useState<Map<string, CardWinRateInfo> | null>(null);
  const [failed, setFailed] = useState(false);
  const wanted = sortKey === "winRate";

  useEffect(() => {
    if (!wanted) {
      // Leaving the sort forgets a failure, so coming back to it asks again, as it always has.
      setFailed(false);
      return undefined;
    }
    if (rates !== null || failed) return undefined;
    let cancelled = false;
    getCardStats()
      .then((res) => {
        if (cancelled) return;
        const map = new Map<string, CardWinRateInfo>();
        for (const card of res.cards) {
          map.set(card.id, { winRate: card.winRate, hasEnoughGames: card.hasEnoughGames });
        }
        setRates(map);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });
    return () => {
      cancelled = true;
    };
  }, [wanted, rates, failed]);

  const retry = useCallback(() => {
    setFailed(false);
  }, []);

  const status: WinRatesStatus = rates !== null ? "ready" : !wanted ? "idle" : failed ? "failed" : "loading";
  return { rates, status, retry };
}
