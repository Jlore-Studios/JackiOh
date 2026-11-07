// R1953001 (#1953): a held token, offered only in a locked item's fullscreen preview (ItemPreview.tsx,
// R2730000), and only where a held token covers it (R1305003, §14.2): a Legendary token covers a
// Legendary chase too, and no token a Mythic (R2728000), a default or a starter; a card token covers
// any unowned card (§14.5).
//
// THE SERVER DECIDES (CLAUDE.md rule 7): the held counts are `GET /api/tokens`' and the offer only
// hides what the server would refuse. A refusal shows the server's words and changes nothing.
//
// R2480000 (#2480): a redemption cannot be undone, so the first press asks in the offer's place, Cancel
// focused. R3221000: the question is an `alertdialog` drawn through `Dialog.tsx`, over the preview it
// opens from: Cancel first and its close, Confirm last, and Escape and Back answer Cancel alone.

import { useEffect, useState, type ReactElement } from "react";

import { rereadOwnedCosmetics } from "../cards/ownedFullArt.ts";
import { Dialog } from "../game/Dialog.tsx";
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

/**
 * The lowest held token whose tier covers `cosmetic`'s rarity on its own scale (R1305003), so a chase
 * only a Legendary token covers, and a Mythic none (R2728000). Null for a default or a starter, and
 * where nothing held covers it or the held tokens are unread.
 */
export function offerTier(
  cosmetic: Pick<RegistryCosmetic, "rarity" | "default" | "starter" | "kind">,
  held: HeldTokens | null,
): CosmeticTokenTier | null {
  // R2725000: a retired full art is sold no more (the server's 400); R3015004: nor a retired augment.
  if (held === null || cosmetic.default || cosmetic.starter || cosmetic.kind === "fullArt" || cosmetic.kind === "augment") return null;
  return OFFERED_TOKENS.find((tier) => cosmeticTier(cosmetic.rarity) <= cosmeticTier(tier) && heldOf(held, tier) > 0) ?? null;
}

/** R3016001: every held token whose tier covers `cosmetic`, the cheapest first (`offerTier` is the first); none where it offers none. */
export function coveringTiers(
  cosmetic: Pick<RegistryCosmetic, "rarity" | "default" | "starter" | "kind">,
  held: HeldTokens | null,
): CosmeticTokenTier[] {
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
  const id = spend.kind === "cosmetic" ? spend.cosmeticId : spend.cardId;

  const ask = (): void => {
    if (state !== "ready") return;
    setAsked(true);
    setState("asking");
  };
  const cancel = (): void => {
    setState("ready");
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
        // R3221000: drawn in the offer's place, a dialog over the preview's; Escape answers it alone.
        <Dialog
          role="alertdialog"
          testid={T.tokenOfferDialog(id)}
          title={questionWords(spend, name)}
          onClose={cancel}
          cancel={{ label: t("Cancel"), kind: "secondary", testid: T.tokenOfferCancel(id) }}
          actions={[{ label: t("Confirm"), kind: "primary", onClick: redeem, testid: T.tokenOfferConfirm(id) }]}
        />
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
