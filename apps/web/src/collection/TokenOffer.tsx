// R1953001 (#1953): a held token, offered only in a locked item's fullscreen preview (ItemPreview.tsx,
// R2730000), and only where a held token covers it (R1305003, §14.2): a Legendary token covers a
// Legendary chase too, and no token a Mythic (R2728000), a default or a starter; a card token covers
// any unowned card (§14.5). R3335002: nor an earned-only one (`earnedOnly`), a Fleet track reward or a Jlorious one.
//
// THE SERVER DECIDES (CLAUDE.md rule 7): the held counts are `GET /api/tokens`' and the offer only
// hides what the server would refuse. A refusal shows the server's words and changes nothing.
//
// R2480000 (#2480): a redemption cannot be undone, so the first press asks in the offer's place, Cancel
// focused. The question is drawn inside the offer, so in the preview, a modal, it lies inside the modal.

import { useEffect, useId, useState, type KeyboardEvent, type ReactElement } from "react";

import { earnedOnly } from "../cards/cosmetics.ts";
import { rereadOwnedCosmetics } from "../cards/ownedFullArt.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getTokens, redeemCardToken, redeemCosmeticToken, type CosmeticTokenTier, type TokenBarsResponse } from "../net/api.ts";
import { cosmeticTier, type RegistryCosmetic } from "../shop/shop.ts";
import { collectionTestid as T } from "./collection.ts";
import { messageOf } from "../net/errors.ts";

import "./tokenOffer.css";

/** The cosmetic tokens an account holds, as `GET /api/tokens` answers them. */
export type HeldTokens = TokenBarsResponse["tokens"];

/** The tokens that buy a cosmetic, the lowest first, so an offer spends the cheapest that covers it. */
const OFFERED_TOKENS: readonly CosmeticTokenTier[] = ["Rare", "Epic", "Legendary"];

export function heldOf(held: HeldTokens, tier: CosmeticTokenTier): number {
  switch (tier) {
    case "Rare":
      return held.cosmeticRare;
    case "Epic":
      return held.cosmeticEpic;
    case "Legendary":
      return held.cosmeticLegendary;
  }
}

/** What the token offers read of a registry entry. */
type OfferedCosmetic = Pick<RegistryCosmetic, "rarity" | "default" | "starter" | "kind" | "jlorious" | "track">;

/**
 * The lowest held token whose tier covers `cosmetic`'s rarity on its own scale (R1305003), so a chase
 * only a Legendary token covers, and a Mythic none (R2728000). Null for a default or a starter, an earned-only one
 * (R3335002), and where nothing held covers it or the held tokens are unread.
 */
export function offerTier(cosmetic: OfferedCosmetic, held: HeldTokens | null): CosmeticTokenTier | null {
  // R2725000: a retired full art is sold no more (the server's 400); R3015004: nor a retired augment.
  if (held === null || cosmetic.default || cosmetic.starter || cosmetic.kind === "fullArt" || cosmetic.kind === "augment") return null;
  // R3335002: a Fleet track reward and a Jlorious one are earned, never bought.
  if (earnedOnly(cosmetic)) return null;
  return OFFERED_TOKENS.find((tier) => cosmeticTier(cosmetic.rarity) <= cosmeticTier(tier) && heldOf(held, tier) > 0) ?? null;
}

/** R3016001: every held token whose tier covers `cosmetic`, the cheapest first (`offerTier` is the first); none where it offers none. */
export function coveringTiers(cosmetic: OfferedCosmetic, held: HeldTokens | null): CosmeticTokenTier[] {
  if (held === null || offerTier(cosmetic, held) === null) return [];
  return OFFERED_TOKENS.filter((tier) => cosmeticTier(cosmetic.rarity) <= cosmeticTier(tier) && heldOf(held, tier) > 0);
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

/** What the offer spends: a cosmetic token of `tier` on one locked cosmetic, or a card token on one unowned card. */
export type TokenSpend =
  | { kind: "cosmetic"; tier: CosmeticTokenTier; cosmeticId: string; cardId?: string }
  | { kind: "card"; cardId: string };

function offerWords(spend: TokenSpend, held: HeldTokens): string {
  if (spend.kind === "card") {
    return t("Unlock with a card token ({count} held)", { count: formatNumber(held.card) });
  }
  const count = heldOf(held, spend.tier);
  switch (spend.tier) {
    case "Rare":
      return t("Unlock with a Rare token ({count} held)", { count: formatNumber(count) });
    case "Epic":
      return t("Unlock with an Epic token ({count} held)", { count: formatNumber(count) });
    case "Legendary":
      return t("Unlock with a Legendary token ({count} held)", { count: formatNumber(count) });
  }
}

/** What the offer asks before it spends (R2480000): the item, and the token it spends. */
function questionWords(spend: TokenSpend, name: string): string {
  if (spend.kind === "card") {
    return t("Unlock {name}? This spends 1 card token.", { name });
  }
  switch (spend.tier) {
    case "Rare":
      return t("Unlock {name}? This spends 1 Rare token.", { name });
    case "Epic":
      return t("Unlock {name}? This spends 1 Epic token.", { name });
    case "Legendary":
      return t("Unlock {name}? This spends 1 Legendary token.", { name });
  }
}

/**
 * The offer on one locked item, drawn only in its fullscreen preview (R2730000): asks first
 * (R2480000), then spends the token on it and hands the answer's held tokens to `onRedeemed`.
 */
export default function TokenOffer({
  token,
  spend,
  held,
  name,
  onRedeemed,
}: {
  token: string;
  spend: TokenSpend;
  held: HeldTokens;
  /** The item's name in the reader's language, which the question names. */
  name: string;
  onRedeemed: (held: HeldTokens) => void;
}): ReactElement | null {
  // Once redeemed the offer is spent: the item shows owned as the re-read answers. R2480000: the
  // press asks first; `asked` puts the focus back on the offer when Cancel or Escape answers.
  const [state, setState] = useState<"ready" | "asking" | "busy" | "redeemed">("ready");
  const [asked, setAsked] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const questionId = useId();
  const id = spend.kind === "cosmetic" ? spend.cosmeticId : spend.cardId;

  const ask = (): void => {
    if (state !== "ready") return;
    setAsked(true);
    setState("asking");
  };
  const cancel = (): void => {
    setState("ready");
  };
  // Escape answers Cancel and nothing else: the preview closes on an Escape that reaches the window.
  const cancelOnEscape = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    cancel();
  };

  const redeem = (): void => {
    if (state !== "asking") return;
    setState("busy");
    setError(null);
    const sent =
      spend.kind === "card"
        ? redeemCardToken(token, spend.cardId)
        : redeemCosmeticToken(token, spend.tier, spend.cosmeticId, spend.cardId);
    sent.then(
      (answer) => {
        setState("redeemed");
        onRedeemed(answer.tokens);
        // A cosmetic's ownership is the owned-cosmetics read; a card's is the collection's own re-read.
        if (spend.kind === "cosmetic") rereadOwnedCosmetics();
      },
      (cause: unknown) => {
        setState("ready");
        setError(messageOf(cause));
      },
    );
  };

  if (state === "redeemed") return null;
  return (
    <div className="token-offer">
      {state === "asking" ? (
        <div
          className="token-offer__question"
          role="alertdialog"
          aria-labelledby={questionId}
          data-testid={T.tokenOfferDialog(id)}
          onKeyDown={cancelOnEscape}
        >
          <p id={questionId}>{questionWords(spend, name)}</p>
          <div className="token-offer__answers">
            <button type="button" data-testid={T.tokenOfferConfirm(id)} onClick={redeem}>
              {t("Confirm")}
            </button>
            <button type="button" data-testid={T.tokenOfferCancel(id)} autoFocus onClick={cancel}>
              {t("Cancel")}
            </button>
          </div>
        </div>
      ) : (
        <button
          type="button"
          className="button-primary token-offer__button"
          data-testid={T.tokenOffer(id)}
          disabled={state === "busy"}
          autoFocus={asked}
          onClick={ask}
        >
          {state === "busy" ? t("Redeeming…") : offerWords(spend, held)}
        </button>
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={T.tokenOfferError(id)}>
          {t(error)}
        </p>
      )}
    </div>
  );
}
