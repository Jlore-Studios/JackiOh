// The cosmetic packs (#1309, SPEC §14.6): the unopened packs, a simple reveal of an opening's
// slots, and the Chase Shard bar with its Legendary-or-Mythic choice when full.
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The packs held, every slot and the shard bar come
// from the answers; the reveal only reads them out, slot by slot in order. A refused open or
// chase shows the server's words verbatim with nothing changed on screen.

import { useEffect, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { t } from "../i18n/i18n.ts";
import {
  buyChase,
  getCollection,
  getCosmeticPacks,
  getOwnedCosmetics,
  openCosmeticPack,
  type CollectionResponse,
  type CosmeticPacksResponse,
  type OpenedCosmeticPackResponse,
  type OwnedCosmeticsResponse,
} from "../net/api.ts";
import {
  chaseCandidates,
  cosmeticSlotOutcome,
  fittingCards,
  isPerCardKind,
  quantitiesOf,
  shopTestid,
  type RegistryCosmetic,
} from "./shop.ts";
import "./shop.css";

/** What a failed request says: the server's words, else the error's. */
function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/** A card's name from the catalog, else its id. */
function cardName(cardId: string): string {
  return CATALOG[cardId]?.name ?? cardId;
}

/**
 * The panel. `revision` is how many times the page has asked it to read again (another panel
 * changed the account); `onChanged` tells the page an opening or a chase changed it.
 */
export default function CosmeticPacks({
  token,
  revision = 0,
  onChanged,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
}): ReactElement {
  const [info, setInfo] = useState<CosmeticPacksResponse | null>(null);
  const [collection, setCollection] = useState<CollectionResponse | null>(null);
  const [owned, setOwned] = useState<OwnedCosmeticsResponse | null>(null);
  const [opened, setOpened] = useState<OpenedCosmeticPackResponse | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [cardFor, setCardFor] = useState<Record<string, string>>({});

  useEffect(() => {
    let cancelled = false;
    Promise.all([getCosmeticPacks(token), getCollection(token), getOwnedCosmetics(token)]).then(
      ([packs, collectionAnswer, ownedAnswer]) => {
        if (!cancelled) {
          setInfo(packs);
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

  const open = (): void => {
    if (busy !== null) return;
    setBusy("open");
    setError(null);
    openCosmeticPack(token).then(
      (next) => {
        setOpened(next);
        setInfo((before) => (before === null ? before : { ...before, unopened: next.unopened, shardBar: next.shardBar }));
        onChanged?.();
        // The collection and the owned cosmetics moved too: read them back, so the chase
        // choice prices what the account holds now.
        Promise.all([getCollection(token), getOwnedCosmetics(token)]).then(
          ([nextCollection, nextOwned]) => {
            setCollection(nextCollection);
            setOwned(nextOwned);
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

  const chase = (cosmetic: RegistryCosmetic, cardId: string | undefined): void => {
    if (busy !== null) return;
    setBusy(cosmetic.id);
    setError(null);
    buyChase(token, cosmetic.id, cardId).then(
      (next) => {
        // The bar emptied: the choice leaves with it.
        setInfo(next);
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
    <div className="form-card cosmetic-packs-panel" data-testid={shopTestid.packsPanel}>
      <h3 className="shop-heading">{t("Cosmetic packs")}</h3>
      {loadError === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.packsLoadError}>
          {loadError}
        </p>
      )}
      {info === null || collection === null || owned === null ? (
        loadError === null ? (
          <p className="notice" role="status" data-testid={shopTestid.packsLoading}>
            {t("Loading your cosmetic packs…")}
          </p>
        ) : null
      ) : (
        <>
          <p className="cosmetic-packs-held" data-testid={shopTestid.unopened}>
            {t("{pack}: {count} unopened", { pack: t("Cosmetic pack"), count: info.unopened })}
          </p>
          {info.unopened > 0 ? (
            <div className="account-actions">
              <button type="button" data-testid={shopTestid.open} disabled={busy !== null} onClick={open}>
                {busy === "open" ? t("Opening…") : t("Open")}
              </button>
            </div>
          ) : (
            <p>{t("No unopened packs.")}</p>
          )}
          {opened === null ? null : (
            <div className="pack-slots" data-testid={shopTestid.reveal}>
              {opened.slots.map((slot, index) => (
                <div
                  key={index}
                  className="pack-slot"
                  data-testid={shopTestid.slot(index)}
                  data-rarity={slot.rarity}
                  data-grant={slot.grant.kind}
                  data-revealed="true"
                >
                  <p className="pack-slot__outcome">{cosmeticSlotOutcome(slot)}</p>
                  {slot.grant.kind === "cosmetic" && slot.grant.cardId !== null ? (
                    <p className="pack-slot__card">
                      {t("On {card}", { card: cardName(slot.grant.cardId) })}
                      {slot.grant.cardGranted ? ` · ${t("Its card was granted too")}` : null}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          )}
          <ShardBar
            info={info}
            collection={collection}
            owned={owned}
            busy={busy}
            cardFor={cardFor}
            onCardFor={setCardFor}
            onChase={chase}
          />
        </>
      )}
      {error === null ? null : (
        <p className="notice" role="alert" data-testid={shopTestid.packsError}>
          {error}
        </p>
      )}
    </div>
  );
}

/** The Chase Shard bar, and the Legendary-or-Mythic choice only when full. */
function ShardBar({
  info,
  collection,
  owned,
  busy,
  cardFor,
  onCardFor,
  onChase,
}: {
  info: CosmeticPacksResponse;
  collection: CollectionResponse;
  owned: OwnedCosmeticsResponse;
  busy: string | null;
  cardFor: Record<string, string>;
  onCardFor: (next: Record<string, string>) => void;
  onChase: (cosmetic: RegistryCosmetic, cardId: string | undefined) => void;
}): ReactElement {
  const full = info.shardBar.points >= info.shardBar.size;
  const quantities = quantitiesOf(collection);
  const candidates = full ? chaseCandidates(owned, quantities) : [];
  return (
    <>
      <h4 className="shop-subheading">{t("Chase Shard")}</h4>
      <p className="shard-bar" data-testid={shopTestid.shardBar}>
        {info.shardBar.points} / {info.shardBar.size}
      </p>
      {!full ? null : candidates.length === 0 ? (
        <p data-testid={shopTestid.shardEmpty}>{t("Nothing to buy yet")}</p>
      ) : (
        <div data-testid={shopTestid.shardChoice}>
          <p>{t("The bar is full — pick a chase:")}</p>
          {candidates.map((cosmetic) => {
            const fits = fittingCards(cosmetic, quantities);
            const open = fits === null ? [] : fits.filter((cardId) => CATALOG[cardId] !== undefined);
            const chosen =
              cardFor[cosmetic.id] !== undefined && open.includes(cardFor[cosmetic.id]!)
                ? cardFor[cosmetic.id]!
                : open[0];
            return (
              <div key={cosmetic.id} className="token-row" data-rarity={cosmetic.rarity}>
                <span>
                  {cosmetic.name} · {t(cosmetic.rarity)}
                  {cosmetic.chase ? ` · ${t("Chase")}` : null}
                </span>
                {isPerCardKind(cosmetic.kind) && open.length > 1 ? (
                  <label className="token-card-choice">
                    {t("Card for {name}", { name: cosmetic.name })}
                    <select
                      data-testid={shopTestid.chaseCards(cosmetic.id)}
                      value={chosen ?? ""}
                      onChange={(event) => {
                        onCardFor({ ...cardFor, [cosmetic.id]: event.target.value });
                      }}
                    >
                      {open.map((cardId) => (
                        <option key={cardId} value={cardId}>
                          {cardName(cardId)}
                        </option>
                      ))}
                    </select>
                  </label>
                ) : null}
                <div className="account-actions">
                  <button
                    type="button"
                    data-testid={shopTestid.chasePick(cosmetic.id)}
                    disabled={busy !== null || (fits !== null && chosen === undefined)}
                    onClick={() => {
                      onChase(cosmetic, fits === null ? undefined : chosen);
                    }}
                  >
                    {busy === cosmetic.id ? t("Buying…") : t("Buy")}
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </>
  );
}
