// A card in context (#1662, R1655000, part of #1651): the Collection's card view, the card large in its
// own picks under a strip of live previews of what it may wear (R2655000: a full-art card's full arts
// are none, its Epic, its animated Legendary and its signed Mythic). Clicking an owned item equips it
// on this card alone; `Use default` clears the pick (R1953001). Holding an item, or its inspect key,
// previews it fullscreen (ItemPreview.tsx), and there only a locked one a held token covers offers it
// (R2730000). It decides nothing (CLAUDE.md rule 7): a pick goes to the device's store, which
// `cards/cardPicksSync.ts` sends to the account (R1834000) and an online match freezes for the
// opponent's sight (R1834002). So a preview writes no pick, lest it stand as one opponents are shown:
// it is an unworn copy of the face dressed by `SampleFace` (CosmeticLook.tsx), or a `FullArtShown`.
// R2725000: a card's version is one of the normal face and the versions it is offered, and each augment it is
// offered toggles on top; either write keeps the other (`writeCardVersion`), so a retired full art's pick becomes
// the version and the augments it combines on its first edit (R2725002). R2949000: Signature waits, listed and
// previewed but not chosen, until the card wears the Full art version and Animated, and turning either off turns
// it off (`updateCardPicks`).

import { useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactElement } from "react";

import { MAX_COPIES } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { CardArt, CardDetail, CardFace, INSPECT_KEY_SHORTCUT, faceModel, useCardSettings } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  OwnershipContext,
  normalizeCosmetics,
  ownsCosmetic,
  resolveCosmetic,
  useCanEquip,
  useCardPicks,
  wornCosmetics,
  writeCardPick,
  writeCardVersion,
  type CardPicks,
} from "../cards/cosmetics.ts";
import { FullArtShown, OwnCopy, useOwnCopy } from "../cards/fullArt.ts";
import { AUGMENT_SIGNATURE_ID, artPictureOf, augmentPicksOf, augmentsOf, lookOf, signatureFits, versionPickOf } from "../cards/versions.ts";
import type { FaceModel } from "../cards/model.ts";
import { t } from "../i18n/i18n.ts";
import { useBackLayer } from "../net/backLayers.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { SampleFace } from "./CosmeticLook.tsx";
import {
  NO_FULL_ART_ID,
  cardStrip,
  collectionTestid as T,
  cosmeticKindWords,
  type StripGroup,
  type StripItem,
  type StripKind,
} from "./collection.ts";
import ItemPreview, { trapTab, usePreviewHold } from "./ItemPreview.tsx";
import TokenOffer, { offerTier, useHeldTokens } from "./TokenOffer.tsx";

/** The card in its own picks and, over it while a foil, finish or border is shown, a copy wearing that. */
function LargeFace({ face, preview }: { face: FaceModel; preview: StripItem | null }): ReactElement {
  const { rarityFoil } = useCardSettings();
  const own = useOwnCopy();
  const card = useCardPicks(face.defId);
  const owned = useContext(OwnershipContext);
  const largeFace = <CardFace face={face} layout="full" worn={own} />;
  let sample: ReactElement | null = null;
  if (preview !== null && preview.kind !== "version" && preview.kind !== "augment") {
    const picks: CardPicks = { ...card };
    picks[preview.kind] = resolveCosmetic(preview.kind, preview.id);
    // What it would wear were it the card's pick: the shown item, and the card's other picks where
    // they are owned, each kind it has none of its default (R1953001).
    const wears = wornCosmetics(
      face,
      DEFAULT_CARD_COSMETICS,
      picks,
      (id) => id === preview.id || ownsCosmetic(owned, face.defId, id),
      rarityFoil,
    );
    sample = (
      <span className="collection-cardview__preview" data-testid={T.cardViewPreview} data-previewing={preview.id} aria-hidden="true">
        <SampleFace face={face} layout="full" wears={wears} />
      </span>
    );
  }
  return (
    <span className="collection-cardview__stage">
      <span className="collection-cardview__face" data-testid={T.cardViewFace}>
        {preview !== null && (preview.kind === "version" || preview.kind === "augment") && preview.owned ? (
          <FullArtShown defId={face.defId} {...lookShown(card, preview)}>
            {largeFace}
          </FullArtShown>
        ) : (
          largeFace
        )}
      </span>
      {sample}
    </span>
  );
}

/**
 * R2725000: the pick a card would hold were `item` chosen: a version in place of its own, or an augment toggled
 * (on where it is off, off where it is on), the other part kept as `versionPickOf` and `augmentPicksOf` keep it.
 */
function chosen(picks: CardPicks, item: StripItem): { fullArt: boolean | string | null; augments: string[] } {
  const augments = augmentPicksOf(picks.fullArt, picks.augments);
  if (item.kind === "version") return { fullArt: item.id === NO_FULL_ART_ID ? false : item.id, augments };
  const on = augments.includes(item.id);
  return { fullArt: versionPickOf(picks.fullArt), augments: on ? augments.filter((id) => id !== item.id) : [...augments, item.id] };
}

/** R2725000: what the large face shows while a version or an augment is previewed: that version, or that augment on. */
function lookShown(picks: CardPicks, item: StripItem): { pick?: boolean | string; augments: string[] } {
  const augments = augmentPicksOf(picks.fullArt, picks.augments);
  if (item.kind === "version") return { pick: item.id === NO_FULL_ART_ID ? false : item.id, augments };
  const pick = versionPickOf(picks.fullArt);
  return { ...(pick === null ? {} : { pick }), augments: augments.includes(item.id) ? augments : [...augments, item.id] };
}

/** R2725000: a version the card's pick names, or an augment it layers on (a retired full art's among them). */
function isEquipped(item: StripItem, picks: CardPicks): boolean {
  if (item.kind === "version") {
    if (picks.fullArt === undefined) return false;
    const version = lookOf(picks.fullArt).version;
    return item.id === NO_FULL_ART_ID ? version === null : version === item.id;
  }
  if (item.kind === "augment") return augmentsOf(lookOf(picks.fullArt, picks.augments)).includes(item.id);
  return picks[item.kind] === item.id;
}

/** R2949000: whether `item` is Signature on a card whose pick does not wear the Full art version and Animated, which it needs. */
function signatureWaits(item: StripItem, picks: CardPicks): boolean {
  return item.kind === "augment" && item.id === AUGMENT_SIGNATURE_ID && !signatureFits(picks.fullArt, picks.augments);
}

/**
 * One item's small live preview: the card wearing it, or for a version the art across the whole face, R2940001 the
 * picture that version draws.
 */
function StripPicture({ def, face, item }: { def: CardDef; face: FaceModel; item: StripItem }): ReactElement {
  const { rarityFoil } = useCardSettings();
  if (item.kind === "version" && item.id !== NO_FULL_ART_ID) {
    return (
      <span className="collection-strip__picture collection-strip__picture--full-art">
        <CardArt defId={def.id} radiant={false} tags={def.tags} type={def.type} name={def.name} shape="full" picture={artPictureOf(item.id)} />
      </span>
    );
  }
  const wears =
    item.kind === "version" || item.kind === "augment"
      ? wornCosmetics(face, normalizeCosmetics({}), {}, () => true, rarityFoil)
      : wornCosmetics(face, normalizeCosmetics({ [item.kind]: item.id }), {}, () => true, rarityFoil);
  return (
    <span className="collection-strip__picture">
      <SampleFace face={face} layout="compact" wears={wears} />
    </span>
  );
}

/**
 * R2730000: one strip item: a hold or the inspect key previews it fullscreen, a click equips it. R2949000: one that
 * `waits` previews as ever and says what it needs, and a click equips nothing.
 */
function StripItemButton({
  def,
  face,
  item,
  equipped,
  waits,
  show,
  hide,
  equip,
  onPreview,
}: {
  def: CardDef;
  face: FaceModel;
  item: StripItem;
  equipped: boolean;
  waits: boolean;
  show: (item: StripItem) => void;
  hide: (item: StripItem) => void;
  equip: (item: StripItem) => void;
  onPreview: (item: StripItem) => void;
}): ReactElement {
  return (
    <button
      type="button"
      className="collection-strip__item"
      data-testid={T.stripItem(item.id)}
      data-owned={String(item.owned)}
      data-equipped={String(equipped)}
      data-rarity={item.rarity}
      data-waits={waits ? "true" : undefined}
      aria-pressed={equipped}
      aria-disabled={item.owned && !waits ? undefined : true}
      aria-keyshortcuts={INSPECT_KEY_SHORTCUT}
      {...usePreviewHold(`strip-${item.id}`, () => {
        onPreview(item);
      })}
      onClick={() => {
        equip(item);
      }}
      onMouseEnter={() => {
        show(item);
      }}
      onMouseLeave={() => {
        hide(item);
      }}
      onPointerEnter={() => {
        show(item);
      }}
      onPointerLeave={() => {
        hide(item);
      }}
      onFocus={() => {
        show(item);
      }}
      onBlur={() => {
        hide(item);
      }}
    >
      <StripPicture def={def} face={face} item={item} />
      <span className="collection-strip__name">{item.name}</span>
      <span className="collection-strip__rarity">{t(item.rarity)}</span>
      {item.owned ? null : <span className="collection-strip__lock">{t("Locked")}</span>}
      {item.owned && waits ? <span className="collection-strip__lock">{t("Needs Full art and Animated")}</span> : null}
    </button>
  );
}

function StripKindGroup({
  group,
  def,
  face,
  picks,
  show,
  hide,
  equip,
  clear,
  onPreview,
}: {
  group: StripGroup;
  def: CardDef;
  face: FaceModel;
  picks: CardPicks;
  show: (item: StripItem) => void;
  hide: (item: StripItem) => void;
  equip: (item: StripItem) => void;
  clear: (kind: StripKind) => void;
  /** R2730000: what a held or inspected item opens: its fullscreen preview. */
  onPreview: (item: StripItem) => void;
}): ReactElement {
  const own =
    group.kind === "version"
      ? picks.fullArt
      : group.kind === "augment"
        ? augmentPicksOf(picks.fullArt, picks.augments)[0]
        : picks[group.kind];
  return (
    <section className="collection-strip" data-testid={T.strip(group.kind)} aria-label={cosmeticKindWords(group.kind)}>
      <div className="collection-strip__head">
        <h4 className="collection-strip__title">{cosmeticKindWords(group.kind)}</h4>
        <button
          type="button"
          className="collection-strip__default"
          data-testid={T.stripDefault(group.kind)}
          aria-pressed={own === undefined}
          onClick={() => {
            clear(group.kind);
          }}
        >
          {t("Use default")}
        </button>
      </div>
      <ul className="collection-strip__items">
        {group.items.map((item) => (
          <li key={item.id}>
            <StripItemButton
              def={def}
              face={face}
              item={item}
              equipped={isEquipped(item, picks)}
              waits={signatureWaits(item, picks)}
              show={show}
              hide={hide}
              equip={equip}
              onPreview={onPreview}
            />
          </li>
        ))}
      </ul>
    </section>
  );
}

export default function CardCosmeticsView({
  def,
  owned,
  copies = 0,
  token = null,
  onClose,
}: {
  def: CardDef;
  owned: boolean;
  /** The copies of the card the account owns, as `GET /api/collection` answers. */
  copies?: number;
  /** R1953001: the session's token, which reads the held cosmetic tokens; none offers none. */
  token?: string | null;
  onClose: () => void;
}): ReactElement {
  const canEquip = useCanEquip();
  const picks = useCardPicks(def.id);
  const [preview, setPreview] = useState<StripItem | null>(null);
  const [previewing, setPreviewing] = useState<string | null>(null);
  const [textOpen, setTextOpen] = useState(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const face = useMemo(() => faceModel({ defId: def.id, def, radiant: false }), [def]);
  const strip = cardStrip(def.id, canEquip);
  // R1953001: the held tokens are read only where one may be offered: a locked item on a card owned at
  // MAX_COPIES, the only card a token's per-card cosmetic goes on (the server's `choice_of`).
  const offering = copies >= MAX_COPIES && strip.some((group) => group.items.some((item) => !item.owned));
  const [tokens, setTokens] = useHeldTokens(offering ? token : null);
  const previewed = strip.flatMap((group) => group.items).find((item) => item.id === previewing) ?? null;

  // Focus moves in and goes back to the card that opened the view; Escape closes it, unless the card's text is over it.
  useLayoutEffect(() => {
    const opener = document.activeElement;
    closeButton.current?.focus({ preventScroll: true });
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  useEffect(() => {
    if (textOpen || previewing !== null) return undefined;
    const onKeyDown = (event: globalThis.KeyboardEvent): void => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [textOpen, previewing, onClose]);
  // R2127000: Back closes the view as Close does, and the focus goes back as above. The card's text,
  // opened over it, is a layer of its own on top, so Back closes that first.
  useBackLayer(onClose);

  const equip = (item: StripItem): void => {
    if (!item.owned || signatureWaits(item, picks)) return;
    if (item.kind === "version" || item.kind === "augment") {
      const next = chosen(picks, item);
      writeCardVersion(def.id, next.fullArt, next.augments);
    } else writeCardPick(def.id, item.kind, resolveCosmetic(item.kind, item.id));
  };
  const clear = (kind: StripKind): void => {
    if (kind === "version") writeCardVersion(def.id, null, augmentPicksOf(picks.fullArt, picks.augments));
    else if (kind === "augment") writeCardVersion(def.id, versionPickOf(picks.fullArt), []);
    else writeCardPick(def.id, kind, null);
  };
  const offer = (item: StripItem): ReactElement | null => {
    const cosmetic = COSMETICS.find((entry) => entry.id === item.id);
    const tier = cosmetic === undefined || !offering ? null : offerTier(cosmetic, tokens);
    if (token === null || tokens === null || tier === null) return null;
    return (
      <TokenOffer
        key={item.id}
        token={token}
        spend={{ kind: "cosmetic", tier, cosmeticId: item.id, cardId: def.id }}
        held={tokens}
        name={item.name}
        onRedeemed={setTokens}
      />
    );
  };

  return (
    <OwnCopy own>
      <div className="collection-cardview" data-testid={T.cardView} data-card={def.id} role="dialog" aria-modal="true" aria-label={readerName(def)} onKeyDown={trapTab}>
        <div
          className="collection-cardview__scrim"
          aria-hidden="true"
          onClick={() => {
            onClose();
          }}
        />
        <div className="collection-cardview__panel">
          <LargeFace face={face} preview={preview} />
          <div className="collection-cardview__side">
            <h3 className="collection-cardview__title">{readerName(def)}</h3>
            <p className="collection-cardview__status">{owned ? t("Owned") : t("Not owned yet")}</p>
            <p className="collection-cardview__hint">
              {t("Hover or focus a cosmetic to see it on the card. Choose one to wear it on this card only.")}
            </p>
            {strip.map((group) => (
              <StripKindGroup
                key={group.kind}
                group={group}
                def={def}
                face={face}
                picks={picks}
                show={(item) => {
                  setPreview(item);
                }}
                hide={(item) => {
                  setPreview((current) => (current?.id === item.id ? null : current));
                }}
                equip={equip}
                clear={clear}
                onPreview={(item) => {
                  setPreviewing(item.id);
                }}
              />
            ))}
            <div className="collection-cardview__actions">
              <button
                type="button"
                className="collection-cardview__text"
                data-testid={T.cardViewText}
                onClick={() => {
                  setTextOpen(true);
                }}
              >
                {t("Card text")}
              </button>
              <button type="button" ref={closeButton} className="collection-cardview__close" data-testid={T.cardViewClose} onClick={onClose}>
                {t("Close")}
              </button>
            </div>
          </div>
        </div>
        {textOpen ? (
          <CardDetail
            def={def}
            onClose={() => {
              setTextOpen(false);
            }}
            meta={<span>{owned ? t("Owned") : t("Not owned yet")}</span>}
          />
        ) : null}
      </div>
      {previewed === null ? null : (
        <ItemPreview
          id={previewed.id}
          name={previewed.name}
          rarity={previewed.rarity}
          owned={previewed.owned}
          picture={
            (previewed.kind === "version" || previewed.kind === "augment") && !previewed.owned ? (
              <StripPicture def={def} face={face} item={previewed} />
            ) : (
              <LargeFace face={face} preview={previewed} />
            )
          }
          offer={previewed.owned ? null : offer(previewed)}
          onClose={() => {
            setPreviewing(null);
          }}
        />
      )}
    </OwnCopy>
  );
}
