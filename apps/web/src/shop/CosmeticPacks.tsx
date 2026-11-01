// The cosmetic packs (#1309, #1667, SPEC §14.6): the pack above Open, an opening's slots with both token
// bars pinned at the top (`OpeningBars`, R1953000, R1999000), and the Chase Shard bar (`RewardReveal`).
// THE SERVER DECIDES (CLAUDE.md rule 7): packs, slots and bars come from the answers. The shard bar plays
// from the points read before the opening to the answer's, with no `fills`: a full bar holds (R1306002).
// Odds are on `/purchases` (R1665000). A refusal shows the server's words through `t()` (R711000) with
// nothing changed; a slot's new cosmetic is equipped from the reveal (R1955000, collection/EquipButton.tsx).

import { useEffect, useMemo, useState, type CSSProperties, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CosmeticLook, hasLook } from "../collection/CosmeticLook.tsx";
import EquipButton from "../collection/EquipButton.tsx";
import { cardNameOf } from "../collection/collection.ts";
import { BURST_MS, flightsOf, type RevealItem } from "../economy/bars.ts";
import FillBar, { useReducedMotion } from "../economy/FillBar.tsx";
import OpeningBars from "../economy/OpeningBars.tsx";
import RewardReveal from "../economy/RewardReveal.tsx";
import { EmptyState } from "../empty/EmptyState.tsx";
import { t } from "../i18n/i18n.ts";
import {
  buyChase,
  getCollection,
  getCosmeticPacks,
  getOwnedCosmetics,
  keptPaths,
  openCosmeticPack,
  type CollectionResponse,
  type CosmeticPacksResponse,
  type OpenedCosmeticPackResponse,
  type OwnedCosmeticsResponse,
} from "../net/api.ts";
import { paths } from "../net/navigate.ts";
import { cachedRead } from "../net/readCache.ts";
import PackObject from "../packs/PackObject.tsx";
import { COSMETIC_PACK } from "../packs/packs.ts";
import { followInApp } from "../routes/nav.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
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

function wordsOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

/**
 * `revision` counts the page's asks to read again (another panel changed the account); `onChanged` tells
 * the page an opening or a chase changed it. `registry` is what the chase choice reads, the shipped one
 * unless a test passes its own (no shipped Legendary or Mythic is buyable yet).
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
  // R2125000: a revisit starts on the answers the last visit kept and reads again behind them.
  const [info, setInfo] = useState<CosmeticPacksResponse | null>(
    () => cachedRead<CosmeticPacksResponse>(token, keptPaths.cosmeticPacks) ?? null,
  );
  const [collection, setCollection] = useState<CollectionResponse | null>(
    () => cachedRead<CollectionResponse>(token, keptPaths.collection) ?? null,
  );
  const [owned, setOwned] = useState<OwnedCosmeticsResponse | null>(
    () => cachedRead<OwnedCosmeticsResponse>(token, keptPaths.cosmetics) ?? null,
  );
  const [opened, setOpened] = useState<OpenedCosmeticPackResponse | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [cardFor, setCardFor] = useState<Record<string, string>>({});
  // The points read before the last opening, where the bar plays from; none when it rests.
  const [shardFrom, setShardFrom] = useState<number | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [reading, setReading] = useState(true);

  useEffect(() => {
    let canceled = false;
    setReading(true);
    Promise.all([getCosmeticPacks(token), getCollection(token), getOwnedCosmetics(token)]).then(
      ([packs, collectionAnswer, ownedAnswer]) => {
        if (!canceled) {
          setInfo(packs);
          setShardFrom(undefined);
          setCollection(collectionAnswer);
          setOwned(ownedAnswer);
          setLoadError(null);
          setReading(false);
        }
      },
      (cause: unknown) => {
        if (!canceled) {
          setReading(false);
          // R2125000: a read that fails behind kept answers leaves them and says nothing; only a first visit says why.
          const keptInfo = cachedRead<CosmeticPacksResponse>(token, keptPaths.cosmeticPacks);
          const keptCollection = cachedRead<CollectionResponse>(token, keptPaths.collection);
          const keptOwned = cachedRead<OwnedCosmeticsResponse>(token, keptPaths.cosmetics);
          if (keptInfo === undefined || keptCollection === undefined || keptOwned === undefined) {
            setLoadError(wordsOf(cause));
          } else {
            setInfo(keptInfo);
            setCollection(keptCollection);
            setOwned(keptOwned);
            setLoadError(null);
          }
        }
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, revision, attempt]);

  const open = (): void => {
    if (busy !== null || info === null) return;
    setBusy("open");
    setError(null);
    openCosmeticPack(token).then(
      (next) => {
        setOpened(next);
        setShardFrom(next.shardBefore);
        setInfo((before) => (before === null ? before : { ...before, unopened: next.unopened, shardBar: next.shardBar }));
        onChanged?.();
        // The collection and owned cosmetics moved too: read them back so the chase choice prices them now.
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
            {t(loadError)}
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
          <Skeleton variant="panel" label={t("Loading your cosmetic packs…")} testId={shopTestid.packsLoading} />
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
            <EmptyState text={t("No unopened packs.")} />
          )}
          {opened === null ? null : (
            <div className="pack-slots" data-testid={shopTestid.reveal}>
              {opened.slots.map((slot, index) => {
                // R1827000, R1833000: a cosmetic shows its look, a per-card one on its card.
                const look =
                  slot.grant.kind === "cosmetic" && hasLook(slot.grant.cosmeticId, slot.grant.cardId)
                    ? { id: slot.grant.cosmeticId, cardId: slot.grant.cardId }
                    : null;
                return (
                  <div
                    key={index}
                    className="pack-slot"
                    data-testid={shopTestid.slot(index)}
                    data-rarity={slot.rarity}
                    data-grant={slot.grant.kind}
                    data-revealed="true"
                  >
                    <p className="pack-slot__outcome">{cosmeticSlotOutcome(slot, opened.bars.slotPoints[index] ?? 0)}</p>
                    {look === null ? null : <CosmeticLook id={look.id} cardId={look.cardId} />}
                    {slot.grant.kind === "cosmetic" && slot.grant.cardId !== null ? (
                      <p className="pack-slot__card">
                        {t("On {card}", { card: cardNameOf(slot.grant.cardId) })}
                        {slot.grant.cardGranted ? ` · ${t("Its card was granted too")}` : null}
                      </p>
                    ) : null}
                    {slot.grant.kind === "cosmetic" ? (
                      <EquipButton cosmeticId={slot.grant.cosmeticId} cardId={slot.grant.cardId} testid={shopTestid.equip(index)} />
                    ) : null}
                  </div>
                );
              })}
            </div>
          )}
          {opened === null ? null : (
            <OpeningBars
              bars={opened.bars}
              flights={flightsOf(opened.bars, opened.slots.map((slot) => slot.rarity), shopTestid.slot)}
              testid={shopTestid.openingBars}
            />
          )}
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
          {t(error)}
        </p>
      )}
    </div>
  );
}

/** The chase choice's moment, a reward flying out of the bar (`kind` and `rarity` set its sting and look). */
function chaseReveal(): RevealItem[] {
  return [
    { key: "chase", kind: "cosmeticToken", label: t("The bar is full — pick a chase:"), rarity: "Legendary" },
  ];
}

/**
 * The Chase Shard bar, and its choice only when full. An opening that filled it plays the sweep, then the
 * bar bursts and reveals the choice; a bar read full, or held full by a further opening, shows it at once.
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
          {/* The moment, with its sting, only for the opening that filled the bar; read full, the words at once. */}
          {filled ? <RewardReveal items={reveal} testid={shopTestid.shardReveal} /> : <p>{t("The bar is full — pick a chase:")}</p>}
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
                {/* R1833000: the chase's look, a per-card one on the card it would go on. */}
                {hasLook(cosmetic.id, chosen ?? null) ? (
                  <div className="token-preview" data-testid={shopTestid.chasePreview(cosmetic.id)}>
                    <CosmeticLook id={cosmetic.id} cardId={chosen ?? null} />
                  </div>
                ) : null}
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
                          {cardNameOf(cardId)}
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
