// R1953001 (#1953): a held cosmetic token, offered where a locked cosmetic is shown. A Rare token
// covers Common and Rare, an Epic token Epic and below (R1305003, §14.2). THE SERVER DECIDES
// (CLAUDE.md rule 7): the offer only hides what it would refuse; a refusal shows the server's words
// and changes nothing.

import { useEffect, useState, type ReactElement } from "react";

import { rereadOwnedCosmetics } from "../cards/ownedFullArt.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getTokens, redeemCosmeticToken, type CosmeticTokenTier, type TokenBarsResponse } from "../net/api.ts";
import { cosmeticTier, type RegistryCosmetic } from "../shop/shop.ts";

import "./tokenOffer.css";

/** The cosmetic tokens an account holds, as `GET /api/tokens` answers them. */
export type HeldTokens = TokenBarsResponse["tokens"];

/** The tokens that buy a cosmetic, the lowest first, so an offer spends the cheapest that covers it. */
const OFFERED_TOKENS: readonly CosmeticTokenTier[] = ["Rare", "Epic"];

function heldOf(held: HeldTokens, tier: CosmeticTokenTier): number {
  return tier === "Rare" ? held.cosmeticRare : held.cosmeticEpic;
}

/**
 * The lowest held token whose tier covers `cosmetic`'s rarity on its own scale (R1305003). Null for a
 * chase, a default or a starter, and where nothing held covers it or the held tokens are unread.
 */
export function offerTier(
  cosmetic: Pick<RegistryCosmetic, "rarity" | "default" | "starter" | "chase">,
  held: HeldTokens | null,
): CosmeticTokenTier | null {
  if (held === null || cosmetic.chase || cosmetic.default || cosmetic.starter) return null;
  return OFFERED_TOKENS.find((tier) => cosmeticTier(cosmetic.rarity) <= cosmeticTier(tier) && heldOf(held, tier) > 0) ?? null;
}

/**
 * The tokens `token`'s account holds, read once as the screen mounts, and the setter a redemption's
 * answer moves them with. No token, a pending read or a failed one holds none, so nothing is offered.
 */
export function useHeldTokens(token: string | null | undefined): [HeldTokens | null, (held: HeldTokens) => void] {
  const [held, setHeld] = useState<HeldTokens | null>(null);
  useEffect(() => {
    if (token === null || token === undefined) return undefined;
    let current = true;
    getTokens(token).then(
      (answer) => {
        if (current) setHeld(answer.tokens);
      },
      () => {
        // Nothing is offered: the shop's readout still shows what is held.
      },
    );
    return () => {
      current = false;
    };
  }, [token]);
  return [held, setHeld];
}

function offerWords(tier: CosmeticTokenTier, count: number): string {
  return tier === "Rare"
    ? t("Unlock with a Rare token ({count} held)", { count: formatNumber(count) })
    : t("Unlock with an Epic token ({count} held)", { count: formatNumber(count) });
}

/**
 * The offer on one locked cosmetic: spends a `tier` token on `cosmeticId`, on `cardId` for a per-card
 * one and on the account for none, then hands the answer's held tokens to `onRedeemed`.
 */
export default function TokenOffer({
  token,
  tier,
  held,
  cosmeticId,
  cardId,
  onRedeemed,
  testid,
  errorTestid,
}: {
  token: string;
  tier: CosmeticTokenTier;
  held: HeldTokens;
  cosmeticId: string;
  cardId?: string;
  onRedeemed: (held: HeldTokens) => void;
  testid: string;
  errorTestid: string;
}): ReactElement | null {
  // Once redeemed the offer is spent: the cosmetic shows owned as the re-read answers.
  const [state, setState] = useState<"ready" | "busy" | "redeemed">("ready");
  const [error, setError] = useState<string | null>(null);

  const redeem = (): void => {
    if (state !== "ready") return;
    setState("busy");
    setError(null);
    redeemCosmeticToken(token, tier, cosmeticId, cardId).then(
      (answer) => {
        setState("redeemed");
        onRedeemed(answer.tokens);
        rereadOwnedCosmetics();
      },
      (cause: unknown) => {
        setState("ready");
        setError(cause instanceof Error ? cause.message : String(cause));
      },
    );
  };

  if (state === "redeemed") return null;
  return (
    <div className="token-offer">
      <button type="button" className="button-primary token-offer__button" data-testid={testid} disabled={state === "busy"} onClick={redeem}>
        {state === "busy" ? t("Redeeming…") : offerWords(tier, heldOf(held, tier))}
      </button>
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={errorTestid}>
          {t(error)}
        </p>
      )}
    </div>
  );
}
