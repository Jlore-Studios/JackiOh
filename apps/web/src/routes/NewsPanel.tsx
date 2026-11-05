// The news (SPEC §14.13, #2875): the patches and alerts the server posts, newest first, each with a
// reward claimed with one press; the landing's bookmark (R2875002) shares `useNews` and `NewsList`.
// The server decides all of it (CLAUDE.md rule 7): which entries are live (R2875001), what each pays
// (R2875003) and whether a claim lands, so the count of unclaimed entries moves only on its answer.
// A title and a summary show as the server sent them. A claim is a spend (R2378000): while one of the
// tab's is out, every Claim waits, disabled and turning a loading wheel (`economy/spend.css`).
// R3074003: a dev's post has no reward: it shows no reward, its button marks it read, and the
// server's claim of it pays nothing.

import { useCallback, useEffect, useState, type ReactElement } from "react";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";
import { announceWallet } from "../economy/wallet.ts";
import "../economy/spend.css";
import "./entry-card.css";
import { formatNumber, t } from "../i18n/i18n.ts";
import { claimNews, getNews, type CosmeticTokenTier, type NewsEntry, type NewsReward } from "../net/api.ts";
import { messageOf } from "../net/errors.ts";
import { spendOutNow, useSpendOut } from "../net/spending.ts";

/** R2982000: Claim all shows only at this many unclaimed entries. */
const CLAIM_ALL_MIN_UNCLAIMED = 2;

export const newsTestid = {
  panel: "news-panel",
  entry: "news-entry",
  claim: "news-claim",
  claimed: "news-claimed",
  /** Why the server refused a claim. */
  error: "news-error",
  /** R2982000: claims every unclaimed entry, newest first. */
  claimAll: "news-claim-all",
} as const;

function cosmeticTokenWords(tier: CosmeticTokenTier): string {
  switch (tier) {
    case "Rare":
      return t("Rare cosmetic token");
    case "Epic":
      return t("Epic cosmetic token");
    case "Legendary":
      return t("Legendary cosmetic token");
  }
}

/** What a reward is, one of it. */
function itemWords(reward: Exclude<NewsReward, { kind: "jlore" }>): string {
  switch (reward.kind) {
    case "cardPack":
      // A set's pack reads as the packs panel names it (`packs.ts`'s `packName`).
      return reward.pack === GLOBAL_PACK_NAME ? t("Card pack") : t("{set} pack", { set: t(reward.pack) });
    case "cardToken":
      return t("Card token");
    case "cosmeticToken":
      return cosmeticTokenWords(reward.tier);
    case "cosmeticPack":
      return t("Cosmetic pack");
  }
}

/** R2875003: a reward in words; past one, the item counted (Jlore's amount is in its words). */
export function rewardWords(reward: NewsReward): string {
  if (reward.kind === "jlore") return t("{jlore} Jlore", { jlore: formatNumber(reward.amount) });
  const item = itemWords(reward);
  return reward.amount > 1 ? t("{item} × {count}", { item, count: formatNumber(reward.amount) }) : item;
}

/** The live entries, each with its reward and Claim, or Claimed once the server says so. */
export function NewsList({
  news,
  onClaim,
}: {
  news: readonly NewsEntry[];
  /** Claims an entry's reward; answers why it was refused, or null once it is done. */
  onClaim: (id: string) => Promise<string | null>;
}): ReactElement {
  const spendOut = useSpendOut();
  const [busy, setBusy] = useState<string | null>(null);
  const [claimingAll, setClaimingAll] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const waiting = busy !== null || claimingAll || spendOut;
  const claim = async (id: string): Promise<void> => {
    if (busy !== null || claimingAll || spendOutNow()) return;
    setBusy(id);
    setError(null);
    const failure = await onClaim(id);
    setBusy(null);
    setError(failure);
    // R2982000: a single claim announces its own Jlore pay, as the run below announces its run's.
    if (failure === null && news.find((entry) => entry.id === id)?.reward?.kind === "jlore") announceWallet();
  };
  // R2982000: one press claims every unclaimed entry through its own claim, newest first; the
  // first refusal stops the run, the earlier claims standing, and the wallet is announced once.
  const claimAll = async (): Promise<void> => {
    if (busy !== null || claimingAll || spendOutNow()) return;
    const targets = news.filter((entry) => !entry.claimed);
    if (targets.length < CLAIM_ALL_MIN_UNCLAIMED) return;
    setClaimingAll(true);
    setError(null);
    let paidJlore = false;
    for (const target of targets) {
      const failure = await onClaim(target.id);
      if (failure !== null) {
        setError(failure);
        break;
      }
      if (target.reward?.kind === "jlore") paidJlore = true;
    }
    setClaimingAll(false);
    if (paidJlore) announceWallet();
  };
  const unclaimed = news.filter((entry) => !entry.claimed).length;
  return (
    <div className="account-quests" data-testid={newsTestid.panel}>
      <h3 className="account-heading">{t("News")}</h3>
      {unclaimed >= CLAIM_ALL_MIN_UNCLAIMED ? (
        <button
          type="button"
          className="spend-wait"
          data-testid={newsTestid.claimAll}
          disabled={waiting}
          aria-busy={waiting}
          onClick={() => {
            void claimAll();
          }}
        >
          {t("Claim all")}
        </button>
      ) : null}
      <ul className="account-quests__list">
        {news.map((entry) => {
          const reward = entry.reward === null ? null : rewardWords(entry.reward);
          return (
            <li
              key={entry.id}
              className="account-quests__quest entry-card"
              data-testid={newsTestid.entry}
              data-news-id={entry.id}
              data-claimed={entry.claimed ? "true" : undefined}
            >
              <div className="entry-card__header">
                <span className="account-quests__title entry-card__title">{entry.title}</span>
                <span className="entry-card__tag">{t(entry.kind === "patch" ? "Patch" : "Alert")}</span>
              </div>
              <p className="news-panel__summary entry-card__body">{entry.summary}</p>
              <div className="entry-card__footer">
                {reward === null ? null : <span className="account-quests__reward entry-card__pill">{reward}</span>}
                {entry.claimed ? (
                  <span data-testid={newsTestid.claimed}>{reward === null ? t("Read") : t("Claimed")}</span>
                ) : (
                  <button
                    type="button"
                    className="spend-wait"
                    data-testid={newsTestid.claim}
                    aria-label={reward === null ? t("Mark read") : t("Claim {reward}", { reward })}
                    disabled={waiting}
                    aria-busy={waiting}
                    onClick={() => {
                      void claim(entry.id);
                    }}
                  >
                    {reward === null ? t("Mark read") : t("Claim")}
                  </button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
      {error !== null ? (
        <p className="notice" role="alert" data-testid={newsTestid.error}>
          {t(error)}
        </p>
      ) : null}
    </div>
  );
}

/** The caller's live news, read once per token; a failed read leaves it null, and nothing shows. */
export function useNews(token: string): {
  news: NewsEntry[] | null;
  claim: (id: string) => Promise<string | null>;
} {
  const [news, setNews] = useState<NewsEntry[] | null>(null);

  useEffect(() => {
    let canceled = false;
    getNews(token).then(
      (answer) => {
        if (!canceled) setNews(answer.news);
      },
      () => {
        // R2875002: no answer, no bookmark.
      },
    );
    return () => {
      canceled = true;
    };
  }, [token]);

  // The entries the server answers replace the ones shown; a refusal says why and changes nothing.
  // R2982000: no wallet announcement here; `NewsList` announces its own claim or run, once.
  const claim = useCallback(
    async (id: string): Promise<string | null> => {
      try {
        const answer = await claimNews(token, id);
        setNews(answer.news);
        return null;
      } catch (cause: unknown) {
        return messageOf(cause);
      }
    },
    [token],
  );
  return { news, claim };
}
