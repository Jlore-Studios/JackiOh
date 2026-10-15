// The live card win rates a pool sorts by (R377), shared by the deck editor and the Almanac.

import { useEffect, useState } from "react";

import { getCardStats } from "../../net/api.ts";
import type { CardWinRateInfo, SortKey } from "./filters.ts";

/**
 * Each card's win rate, fetched the first time the pool is sorted by it and kept from then on:
 * `null` until it arrives, and after a failed fetch.
 */
export function useCardWinRates(sortKey: SortKey): Map<string, CardWinRateInfo> | null {
  const [winRates, setWinRates] = useState<Map<string, CardWinRateInfo> | null>(null);

  useEffect(() => {
    if (sortKey === "winRate" && winRates === null) {
      let cancelled = false;
      getCardStats()
        .then((res) => {
          if (cancelled) return;
          const map = new Map<string, CardWinRateInfo>();
          for (const card of res.cards) {
            map.set(card.id, { winRate: card.winRate, hasEnoughGames: card.hasEnoughGames });
          }
          setWinRates(map);
        })
        .catch(() => {});
      return () => {
        cancelled = true;
      };
    }
  }, [sortKey, winRates]);

  return winRates;
}
