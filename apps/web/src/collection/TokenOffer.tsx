// R1953001 (#1953): a held cosmetic token, offered where a locked cosmetic is shown (a card's own
// cosmetics view, the playmat, backdrop and card back tiles), and only where a held token covers it
// (R1305003, §14.2): no token covers a chase, a default or a starter.
//
// THE SERVER DECIDES (CLAUDE.md rule 7): the held counts are `GET /api/tokens`' and the offer only
// hides what the server would refuse. A refusal shows the server's words and changes nothing.
//
// R2480000 (#2480): a redemption cannot be undone, so the first press asks in the offer's place, as
// the shop's card token does (shop/TokenPicker.tsx), Cancel focused. The question is drawn inside the
// offer, so in the card view, a modal, it lies inside the modal.

import { useEffect, useId, useState, type KeyboardEvent, type ReactElement } from "react";

import { rereadOwnedCosmetics } from "../cards/ownedFullArt.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getTokens, redeemCosmeticToken, type CosmeticTokenTier, type TokenBarsResponse } from "../net/api.ts";
import { cosmeticTier, type RegistryCosmetic } from "../shop/shop.ts";
import { collectionTestid as T } from "./collection.ts";
import { messageOf } from "../net/errors.ts";

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

/** What the offer asks before it spends (R2480000): the cosmetic, and the token it spends. */
function questionWords(tier: CosmeticTokenTier, name: string): string {
  return tier === "Rare"
    ? t("Unlock {name}? This spends 1 Rare token.", { name })
    : t("Unlock {name}? This spends 1 Epic token.", { name });
}

/** The offer on one locked cosmetic: asks first (R2480000), then spends a `tier` token on `cosmeticId`. */
export default function TokenOffer({
  token,
  tier,
  held,
  cosmeticId,
  name,
  cardId,
  onRedeemed,
  testid,
  errorTestid,
}: {
  token: string;
  tier: CosmeticTokenTier;
  held: HeldTokens;
  cosmeticId: string;
  /** The cosmetic's name in the reader's language, which the question names. */
  name: string;
  cardId?: string;
  onRedeemed: (held: HeldTokens) => void;
  testid: string;
  errorTestid: string;
}): ReactElement | null {
  // Once redeemed the offer is spent: the cosmetic shows owned as the re-read answers. R2480000: the
  // press asks first; `asked` puts the focus back on the offer when Cancel or Escape answers.
  const [state, setState] = useState<"ready" | "asking" | "busy" | "redeemed">("ready");
  const [asked, setAsked] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const questionId = useId();

  const ask = (): void => {
    if (state !== "ready") return;
    setAsked(true);
    setState("asking");
  };
  const cancel = (): void => {
    setState("ready");
  };
  // Escape answers Cancel and nothing else: the card view closes on an Escape that reaches the window.
  const cancelOnEscape = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    cancel();
  };

  const redeem = (): void => {
    if (state !== "asking") return;
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
          data-testid={T.tokenOfferDialog(cosmeticId)}
          onKeyDown={cancelOnEscape}
        >
          <p id={questionId}>{questionWords(tier, name)}</p>
          <div className="token-offer__answers">
            <button type="button" data-testid={T.tokenOfferConfirm(cosmeticId)} onClick={redeem}>
              {t("Confirm")}
            </button>
            <button type="button" data-testid={T.tokenOfferCancel(cosmeticId)} autoFocus onClick={cancel}>
              {t("Cancel")}
            </button>
          </div>
        </div>
      ) : (
        <button
          type="button"
          className="button-primary token-offer__button"
          data-testid={testid}
          disabled={state === "busy"}
          autoFocus={asked}
          onClick={ask}
        >
          {state === "busy" ? t("Redeeming…") : offerWords(tier, heldOf(held, tier))}
        </button>
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={errorTestid}>
          {t(error)}
        </p>
      )}
    </div>
  );
}
