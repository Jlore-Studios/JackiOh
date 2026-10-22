// The cosmetic packs (#1309, #1667, SPEC §14.6): the unopened pack as a `PackObject` above Open, a
// simple reveal of an opening's slots with the token bars it moved (`OpeningBars`), and the Chase
// Shard bar, a `FillBar` that fills across an opening and, when it reaches full, bursts and reveals
// its Legendary-or-Mythic choice as a moment of its own (`RewardReveal`).
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The packs held, every slot, both bars and the shard bar come
// from the answers; the reveal only reads them out, slot by slot in order. The shard bar plays from the
// points the panel read before the opening to the answer's, with no `fills`: a full bar holds (R1306002).
// The odds are not here: they are on `/purchases` (R1665000). A refused open or chase shows the
// server's words verbatim with nothing changed on screen.

import { useEffect, useMemo, useState, type CSSProperties, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { BURST_MS, type RevealItem } from "../economy/bars.ts";
import FillBar, { useReducedMotion } from "../economy/FillBar.tsx";
import OpeningBars from "../economy/OpeningBars.tsx";
import RewardReveal from "../economy/RewardReveal.tsx";
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
import { paths } from "../net/navigate.ts";
import PackObject from "../packs/PackObject.tsx";
import { COSMETIC_PACK } from "../packs/packs.ts";
import { followInApp } from "../routes/nav.tsx";
import {
  REGISTRY,
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
 * changed the account); `onChanged` tells the page an opening or a chase changed it. `registry` is
 * the cosmetics the chase choice reads, the shipped one unless a test passes its own (no shipped
 * Legendary or Mythic is buyable yet).
 */
export default function CosmeticPacks({
  token,
  revision = 0,
  onChanged,
  registry = REGISTRY,
}: {
  token: string;
  revision?: number;
  onChanged?: () => void;
  registry?: readonly RegistryCosmetic[];
}): ReactElement {
  const [info, setInfo] = useState<CosmeticPacksResponse | null>(null);
  const [collection, setCollection] = useState<CollectionResponse | null>(null);
  const [owned, setOwned] = useState<OwnedCosmeticsResponse | null>(null);
  const [opened, setOpened] = useState<OpenedCosmeticPackResponse | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [cardFor, setCardFor] = useState<Record<string, string>>({});
  // The shard points the panel read before the last opening: where the bar plays from; none when the
  // bar was just read, or emptied by a chase, and it rests.
  const [shardFrom, setShardFrom] = useState<number | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setReading(true);
    Promise.all([getCosmeticPacks(token), getCollection(token), getOwnedCosmetics(token)]).then(
      ([packs, collectionAnswer, ownedAnswer]) => {
        if (!cancelled) {
          setInfo(packs);
          setShardFrom(undefined);
          setCollection(collectionAnswer);
          setOwned(ownedAnswer);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!cancelled) {
          setLoadError(wordsOf(cause));
          setReading(false);
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [token, revision, attempt]);

  const open = (): void => {
    if (busy !== null || info === null) return;
    const before = info.shardBar.points;
    setBusy("open");
    setError(null);
    openCosmeticPack(token).then(
      (next) => {
        setOpened(next);
        setShardFrom(before);
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
        setShardFrom(undefined);
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
        <>
          <p className="notice" role="alert" data-testid={shopTestid.packsLoadError}>
            {loadError}
          </p>
          <div className="account-actions">
            <button
              type="button"
              data-testid={shopTestid.packsRetry}
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
            <>
              <PackObject
                pack={COSMETIC_PACK}
                size="shelf"
                state={busy === "open" ? "pressed" : "idle"}
                testid={shopTestid.cosmeticPackObject}
              />
              <div className="account-actions">
                <button type="button" data-testid={shopTestid.open} disabled={busy !== null} onClick={open}>
                  {busy === "open" ? t("Opening…") : t("Open")}
                </button>
              </div>
            </>
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
          {opened === null ? null : <OpeningBars bars={opened.bars} testid={shopTestid.openingBars} />}
          {opened === null ? null : (
            <p>
              <a
                href={paths.collectionSection("cosmetics")}
                data-testid={shopTestid.equipLink}
                onClick={followInApp(paths.collectionSection("cosmetics"))}
              >
                {t("See and equip your cosmetics in your Collection")}
              </a>
            </p>
          )}
          <ShardBar
            key={`${String(shardFrom ?? "-")}:${String(info.shardBar.points)}:${String(info.shardBar.size)}`}
            info={info}
            from={shardFrom}
            registry={registry}
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

/** The moment a full bar reveals: the chase choice, as a reward flies out of the bar (`kind` and `rarity` set its sting and look). */
function chaseReveal(): RevealItem[] {
  return [
    { key: "chase", kind: "cosmeticToken", label: t("The bar is full — pick a chase:"), rarity: "Legendary" },
  ];
}

/**
 * The Chase Shard bar, and the Legendary-or-Mythic choice only when full. An opening that took the
 * bar from short of full to full plays the sweep, then the bar bursts and the choice is revealed as a
 * moment of its own; a bar read full, or one a further opening only held full, shows its choice at once.
 */
function ShardBar({
  info,
  from,
  registry,
  collection,
  owned,
  busy,
  cardFor,
  onCardFor,
  onChase,
}: {
  info: CosmeticPacksResponse;
  /** The points before the opening this answer follows, when it follows one. */
  from: number | undefined;
  registry: readonly RegistryCosmetic[];
  collection: CollectionResponse;
  owned: OwnedCosmeticsResponse;
  busy: string | null;
  cardFor: Record<string, string>;
  onCardFor: (next: Record<string, string>) => void;
  onChase: (cosmetic: RegistryCosmetic, cardId: string | undefined) => void;
}): ReactElement {
  const { points, size } = info.shardBar;
  const full = points >= size;
  const filled = from !== undefined && from < size && full;
  const [reached, setReached] = useState(!filled);
  const reduced = useReducedMotion();
  const reveal = useMemo(chaseReveal, []);
  const quantities = quantitiesOf(collection);
  const candidates = full && reached ? chaseCandidates(owned, quantities, registry) : [];
  return (
    <>
      <div className="shard-bar">
        <FillBar
          label={t("Chase Shard")}
          size={size}
          value={points}
          from={from}
          onDone={() => {
            setReached(true);
          }}
          testid={shopTestid.shardBar}
        />
        {filled && reached && !reduced ? (
          <span
            className="fill-bar__burst shard-bar__burst"
            aria-hidden="true"
            data-testid={shopTestid.shardBurst}
            style={{ "--burst-ms": `${String(BURST_MS)}ms` } as CSSProperties}
          />
        ) : null}
      </div>
      {!full || !reached ? null : candidates.length === 0 ? (
        <p data-testid={shopTestid.shardEmpty}>{t("Nothing to buy yet")}</p>
      ) : (
        <div data-testid={shopTestid.shardChoice}>
          <RewardReveal items={reveal} testid={shopTestid.shardReveal} />
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
