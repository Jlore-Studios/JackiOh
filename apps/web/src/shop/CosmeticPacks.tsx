// The cosmetic packs (#1309, #1667, SPEC §14.6): the unopened pack as a `PackObject` above Open, a
// simple reveal of an opening's slots with both token bars pinned at the top of the viewport while it
// is on screen, each slot's points flying into them before they fill (`OpeningBars`, R1953000, R1999000),
// and the Chase Shard bar, a `FillBar` that fills across an opening and, when it reaches full, bursts
// and reveals its Legendary-or-Mythic choice as a moment of its own (`RewardReveal`).
//
// THE SERVER DECIDES (CLAUDE.md rule 7). The packs held, every slot, both bars and the shard bar come
// from the answers; the reveal only reads them out, slot by slot in order. The shard bar plays from the
// points the panel read before the opening to the answer's, with no `fills`: a full bar holds (R1306002).
// The odds are not here: they are on `/purchases` (R1665000). A refused open or chase shows the
// server's words through `t()` (R711000) with nothing changed on screen. A slot's new cosmetic is
// equipped from the reveal too (R1955000, collection/EquipButton.tsx); the link to the Collection
// stays.
//
// A chase spends the whole bar and nothing takes it back, so its Buy asks first, inline in the row's
// place, with Cancel focused and Escape answering Cancel; only Confirm sends the one `buyChase`
// (R2683000, as the token offer's R2480000). Open and the chase Buy wait, as the Shop's Buy does,
// while one of the tab's spends is out (R2378000, `net/spending.ts`), and their handlers check again.

import { useEffect, useId, useMemo, useState, type CSSProperties, type KeyboardEvent, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { CosmeticLook, hasLook } from "../collection/CosmeticLook.tsx";
import EquipButton from "../collection/EquipButton.tsx";
import { cardNameOf } from "../collection/collection.ts";
import { BURST_MS, flightsOf, type RevealItem } from "../economy/bars.ts";
import FillBar, { useReducedMotion } from "../economy/FillBar.tsx";
import OpeningBars from "../economy/OpeningBars.tsx";
import RewardReveal from "../economy/RewardReveal.tsx";
import "../economy/spend.css";
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
import { spendOutNow, useSpendOut } from "../net/spending.ts";
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
import { messageOf } from "../net/errors.ts";
import "./shop.css";

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
  // R2378000: Open and the chase Buy wait while one of the tab's spends is out.
  const spendOut = useSpendOut();

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
            setLoadError(messageOf(cause));
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
    if (busy !== null || info === null || spendOutNow()) return;
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
            setError(messageOf(cause));
          },
        );
      },
      (cause: unknown) => {
        setError(messageOf(cause));
      },
    ).finally(() => {
      setBusy(null);
    });
  };

  const chase = (cosmetic: RegistryCosmetic, cardId: string | undefined): void => {
    if (busy !== null || spendOutNow()) return;
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
        setError(messageOf(cause));
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
                <button
                  type="button"
                  className="spend-wait"
                  data-testid={shopTestid.open}
                  disabled={busy !== null || spendOut}
                  aria-busy={spendOut}
                  onClick={open}
                >
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
            spendOut={spendOut}
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
  spendOut,
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
  /** Whether one of the tab's spends is out (R2378000). */
  spendOut: boolean;
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
  // R2683000: the first press on a Buy asks, in the row's place; `asking` is the cosmetic asked about,
  // `asked` the one whose Buy takes the focus back when Cancel or Escape answers.
  const [asking, setAsking] = useState<string | null>(null);
  const [asked, setAsked] = useState<string | null>(null);
  const questionId = useId();
  const ask = (cosmeticId: string): void => {
    if (busy !== null || spendOutNow()) return;
    setAsked(cosmeticId);
    setAsking(cosmeticId);
  };
  const cancel = (): void => {
    setAsking(null);
  };
  // Escape answers Cancel and nothing else: a page above may close on an Escape that reaches the window.
  const cancelOnEscape = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    cancel();
  };
  // The spend is checked before the question closes, so a Confirm met by a spend out keeps the question.
  const confirm = (cosmetic: RegistryCosmetic, cardId: string | undefined): void => {
    if (busy !== null || spendOutNow()) return;
    setAsking(null);
    onChase(cosmetic, cardId);
  };
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
            const cardId = fits === null ? undefined : chosen;
            if (asking === cosmetic.id) {
              return (
                <div
                  key={`ask-${cosmetic.id}`}
                  className="token-confirm"
                  role="alertdialog"
                  aria-labelledby={questionId}
                  onKeyDown={cancelOnEscape}
                >
                  <p id={questionId}>
                    {t("Buy {name}? This spends your full Chase Shard bar.", { name: t(cosmetic.name) })}
                  </p>
                  <div className="token-confirm__actions">
                    <button
                      type="button"
                      disabled={busy !== null || spendOut}
                      onClick={() => {
                        confirm(cosmetic, cardId);
                      }}
                    >
                      {t("Confirm")}
                    </button>
                    <button type="button" autoFocus onClick={cancel}>
                      {t("Cancel")}
                    </button>
                  </div>
                </div>
              );
            }
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
                    className="spend-wait"
                    data-testid={shopTestid.chasePick(cosmetic.id)}
                    disabled={busy !== null || asking !== null || spendOut || (fits !== null && chosen === undefined)}
                    aria-busy={spendOut && (fits === null || chosen !== undefined)}
                    autoFocus={asked === cosmetic.id}
                    onClick={() => {
                      ask(cosmetic.id);
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
