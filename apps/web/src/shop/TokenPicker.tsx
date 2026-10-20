// The token picker (#1309, SPEC §14.5): both token bars, the cosmetic cycle's next step, the
// card token's pick of an unowned card and the cosmetic token's pick at its tier or below.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The bars, their sizes, the next step and the held
// counts come from `GET /api/tokens`; the filters only choose what to browse. Every redemption
// sends intent and renders the answer: success re-reads the bars (and the collection or the
// owned cosmetics, so a bought entry leaves the list), refusal shows the server's words verbatim
// with nothing changed on screen.

import { useEffect, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CardFace, faceModel } from "../cards/index.ts";
import { t } from "../i18n/i18n.ts";
import {
  getCollection,
  getOwnedCosmetics,
  getTokens,
  redeemCardToken,
  redeemCosmeticToken,
  type CollectionResponse,
  type CosmeticTokenTier,
  type OwnedCosmeticsResponse,
  type TokenBarsResponse,
} from "../net/api.ts";
import {
  fittingCards,
  nextStepWords,
  quantitiesOf,
  redeemableCosmetics,
  shopTestid,
  tokenTierWords,
  unownedCards,
  type RegistryCosmetic,
} from "./shop.ts";
import "./shop.css";

/** What a failed request says: the server's words, else the error's. */
function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/**
 * The panel. `revision` is how many times the page has asked it to read again (another panel
 * changed the account); `onChanged` tells the page a redemption changed it.
 */
export default function TokenPicker({
  token,
  revision = 0,
  onChanged,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
}): ReactElement {
  const [bars, setBars] = useState<TokenBarsResponse | null>(null);
  const [collection, setCollection] = useState<CollectionResponse | null>(null);
  const [owned, setOwned] = useState<OwnedCosmeticsResponse | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [tier, setTier] = useState<CosmeticTokenTier>("Rare");
  const [cardFor, setCardFor] = useState<Record<string, string>>({});

  useEffect(() => {
    let cancelled = false;
    Promise.all([getTokens(token), getCollection(token), getOwnedCosmetics(token)]).then(
      ([barsAnswer, collectionAnswer, ownedAnswer]) => {
        if (!cancelled) {
          setBars(barsAnswer);
          setCollection(collectionAnswer);
          setOwned(ownedAnswer);
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

  const pickCard = (cardId: string): void => {
    if (busy !== null) return;
    setBusy(cardId);
    setError(null);
    redeemCardToken(token, cardId).then(
      (answered) => {
        setBars(answered);
        onChanged?.();
        // The collection moved too: read it back, so the unlocked card leaves the list.
        getCollection(token).then(
          (next) => {
            setCollection(next);
          },
          (cause: unknown) => {
            setError(wordsOf(cause));
          },
        );
      },
      (cause: unknown) => {
        setError(wordsOf(cause));
      },
    ).finally(() => {
      setBusy(null);
    });
  };

  const pickCosmetic = (cosmetic: RegistryCosmetic, cardId: string | undefined): void => {
    if (busy !== null) return;
    setBusy(cosmetic.id);
    setError(null);
    redeemCosmeticToken(token, tier, cosmetic.id, cardId).then(
      (answered) => {
        setBars(answered);
        onChanged?.();
        // The owned cosmetics moved too: read them back, so a bought entry leaves the list.
        getOwnedCosmetics(token).then(
          (next) => {
            setOwned(next);
          },
          (cause: unknown) => {
            setError(wordsOf(cause));
          },
        );
      },
      (cause: unknown) => {
        setError(wordsOf(cause));
      },
    ).finally(() => {
      setBusy(null);
    });
  };

  return (
    <div className="form-card token-panel" data-testid={shopTestid.tokenPanel}>
      <h3 className="shop-heading">{t("Card and cosmetic tokens")}</h3>
      {loadError === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.tokenLoadError}>
          {loadError}
        </p>
      )}
      {bars === null || collection === null || owned === null ? (
        loadError === null ? (
          <p className="notice" role="status" data-testid={shopTestid.tokenLoading}>
            {t("Loading your tokens…")}
          </p>
        ) : null
      ) : (
        <TokenBody
          bars={bars}
          collection={collection}
          owned={owned}
          tier={tier}
          onTier={setTier}
          cardFor={cardFor}
          onCardFor={setCardFor}
          busy={busy}
          onPickCard={pickCard}
          onPickCosmetic={pickCosmetic}
        />
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.tokenError}>
          {error}
        </p>
      )}
    </div>
  );
}

/** The bars, the next step and both pickers, once every read has answered. */
function TokenBody({
  bars,
  collection,
  owned,
  tier,
  onTier,
  cardFor,
  onCardFor,
  busy,
  onPickCard,
  onPickCosmetic,
}: {
  bars: TokenBarsResponse;
  collection: CollectionResponse;
  owned: OwnedCosmeticsResponse;
  tier: CosmeticTokenTier;
  onTier: (tier: CosmeticTokenTier) => void;
  cardFor: Record<string, string>;
  onCardFor: (next: Record<string, string>) => void;
  busy: string | null;
  onPickCard: (cardId: string) => void;
  onPickCosmetic: (cosmetic: RegistryCosmetic, cardId: string | undefined) => void;
}): ReactElement {
  const quantities = quantitiesOf(collection);
  const cards = unownedCards(collection);
  const cosmetics = redeemableCosmetics(tier, owned, quantities);
  return (
    <>
      <p className="token-bar" data-testid={shopTestid.cardBar}>
        {t("Card token bar")}: {bars.cardBar.points} / {bars.cardBar.size}
      </p>
      <p className="token-bar" data-testid={shopTestid.cosmeticBar}>
        {t("Cosmetic token bar")}: {bars.cosmeticBar.points} / {bars.cosmeticBar.size}
      </p>
      <p className="token-next" data-testid={shopTestid.nextStep}>
        {t("Next: {step}", { step: nextStepWords(bars.nextStep) })}
      </p>

      <h4 className="shop-subheading">{t("Card tokens")}</h4>
      <p className="token-held" data-testid={shopTestid.heldCard}>
        {t("Card tokens held: {count}", { count: bars.tokens.card })}
      </p>
      {cards.length === 0 ? (
        <p data-testid={shopTestid.cardNone}>{t("No cards to pick.")}</p>
      ) : (
        cards.map((card) => (
          <div key={card.id} className="token-row">
            <span>{card.name}</span>
            <div className="account-actions">
              <button
                type="button"
                data-testid={shopTestid.cardPick(card.id)}
                disabled={bars.tokens.card < 1 || busy !== null}
                onClick={() => {
                  onPickCard(card.id);
                }}
              >
                {busy === card.id ? t("Redeeming…") : t("Pick")}
              </button>
            </div>
          </div>
        ))
      )}

      <h4 className="shop-subheading">{t("Cosmetic tokens")}</h4>
      <div className="account-actions" role="group" aria-label={t("Cosmetic tokens")}>
        {(["Rare", "Epic"] as const).map((choice) => (
          <button
            key={choice}
            type="button"
            data-testid={choice === "Rare" ? shopTestid.tierRare : shopTestid.tierEpic}
            aria-pressed={tier === choice}
            disabled={(choice === "Rare" ? bars.tokens.cosmeticRare : bars.tokens.cosmeticEpic) < 1}
            onClick={() => {
              onTier(choice);
            }}
          >
            {tokenTierWords(choice)}
          </button>
        ))}
      </div>
      <p className="token-held" data-testid={shopTestid.heldRare}>
        {t("Rare tokens held: {count}", { count: bars.tokens.cosmeticRare })}
      </p>
      <p className="token-held" data-testid={shopTestid.heldEpic}>
        {t("Epic tokens held: {count}", { count: bars.tokens.cosmeticEpic })}
      </p>
      {cosmetics.length === 0 ? (
        <p data-testid={shopTestid.cosmeticNone}>{t("No cosmetics to pick at this tier.")}</p>
      ) : (
        cosmetics.map((cosmetic) => {
          const fits = fittingCards(cosmetic, quantities);
          const open = fits === null ? [] : fits.filter((cardId) => CATALOG[cardId] !== undefined);
          const chosen = cardFor[cosmetic.id] !== undefined && open.includes(cardFor[cosmetic.id]!) ? cardFor[cosmetic.id]! : open[0];
          const held = (tier === "Rare" ? bars.tokens.cosmeticRare : bars.tokens.cosmeticEpic) < 1;
          return (
            <div key={cosmetic.id} className="token-row" data-rarity={cosmetic.rarity}>
              <span>
                {cosmetic.name} · {t(cosmetic.rarity)}
              </span>
              {chosen !== undefined && CATALOG[chosen] !== undefined ? (
                <div className="token-preview" data-testid={shopTestid.cosmeticPreview(cosmetic.id)}>
                  <CardFace face={faceModel({ defId: chosen, def: CATALOG[chosen], radiant: false })} />
                </div>
              ) : null}
              {open.length > 1 ? (
                <label className="token-card-choice">
                  {t("Card for {name}", { name: cosmetic.name })}
                  <select
                    data-testid={shopTestid.cosmeticCards(cosmetic.id)}
                    value={chosen ?? ""}
                    onChange={(event) => {
                      onCardFor({ ...cardFor, [cosmetic.id]: event.target.value });
                    }}
                  >
                    {open.map((cardId) => (
                      <option key={cardId} value={cardId}>
                        {CATALOG[cardId]?.name ?? cardId}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              <div className="account-actions">
                <button
                  type="button"
                  data-testid={shopTestid.cosmeticPick(cosmetic.id)}
                  disabled={held || busy !== null || (fits !== null && chosen === undefined)}
                  onClick={() => {
                    onPickCosmetic(cosmetic, fits === null ? undefined : chosen);
                  }}
                >
                  {busy === cosmetic.id ? t("Redeeming…") : t("Pick")}
                </button>
              </div>
            </div>
          );
        })
      )}
    </>
  );
}
