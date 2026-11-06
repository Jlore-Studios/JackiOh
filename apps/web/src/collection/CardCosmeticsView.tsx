// A card in context (#1662, R1655000, part of #1651): the Collection's card view, the card large in its
// own picks under a strip of live previews of what it may wear. Clicking an owned item equips it
// on this card alone; `Use default` clears the pick (R1953001). Holding an item, or its inspect key,
// previews it fullscreen (ItemPreview.tsx), and there only a locked one a held token covers offers it
// (R2730000). It decides nothing (CLAUDE.md rule 7): a pick goes to the device's store, which
// `cards/cardPicksSync.ts` sends to the account (R1834000) and an online match freezes for the
// opponent's sight (R1834002). So a preview writes no pick, lest it stand as one opponents are shown:
// it is an unworn copy of the face dressed by `SampleFace` (CosmeticLook.tsx), or a `FullArtShown`.
// R3015007: the strip holds the foils, finishes and borders, and the card's versions are its variants' ladders
// (VariantLadders.tsx): choosing an owned level wears that one level (`writeCardLevel`, R3015005), so a pick made
// before #3015 becomes the level it names on its first edit, and only the next level of a ladder, every level below it
// owned, is offered a held token (R3015004). R3013001: an owned card also shows its proficiency and its stamps
// (ProficiencyPanel.tsx).

import { useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactElement } from "react";

import { MAX_COPIES } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { CardDetail, CardFace, INSPECT_KEY_SHORTCUT, faceModel, useCardSettings } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  OwnershipContext,
  normalizeCosmetics,
  ownsCosmetic,
  resolveCosmetic,
  useCanEquip,
  useCardPicks,
  wornCosmetics,
  writeCardLevel,
  writeCardPick,
  type CardPicks,
} from "../cards/cosmetics.ts";
import { FullArtShown, OwnCopy, useOwnCopy } from "../cards/fullArt.ts";
import { levelOfPick, wornLevel } from "../cards/versions.ts";
import type { FaceModel } from "../cards/model.ts";
import { t } from "../i18n/i18n.ts";
import { useBackLayer } from "../net/backLayers.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { SampleFace } from "./CosmeticLook.tsx";
import {
  NO_FULL_ART_ID,
  cardLadders,
  cardStrip,
  collectionTestid as T,
  cosmeticKindWords,
  type Rung,
  type StripGroup,
  type StripItem,
  type StripKind,
} from "./collection.ts";
import ItemPreview, { trapTab, usePreviewHold } from "./ItemPreview.tsx";
import ProficiencyPanel from "./ProficiencyPanel.tsx";
import TokenOffer, { offerTier, useHeldTokens } from "./TokenOffer.tsx";
import VariantLadders, { RungPicture } from "./VariantLadders.tsx";

/** What the view shows on the large card: a strip item, or a ladder's rung (R3015007). */
type Shown = StripItem | Rung;

function isStripItem(shown: Shown): shown is StripItem {
  return "kind" in shown;
}

/**
 * The card in its own picks and, over it while a foil, finish or border is shown, a copy wearing that; R3015007: or,
 * while an owned rung is shown, its own copy wearing that level (`FullArtShown`).
 */
function LargeFace({ face, preview }: { face: FaceModel; preview: Shown | null }): ReactElement {
  const { rarityFoil } = useCardSettings();
  const own = useOwnCopy();
  const card = useCardPicks(face.defId);
  const owned = useContext(OwnershipContext);
  const largeFace = <CardFace face={face} layout="full" worn={own} />;
  let sample: ReactElement | null = null;
  if (preview !== null && isStripItem(preview)) {
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
        {preview !== null && !isStripItem(preview) && preview.owned ? (
          <FullArtShown defId={face.defId} pick={preview.id === NO_FULL_ART_ID ? false : preview.id}>
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
 * R3015005, R3015007: the rung a card's pick wears: the level it names (an older shape mapped as the server maps it),
 * else the highest one below it that `owns` takes; the normal face where that is none, and none with no pick at all
 * (the Full-art cards setting decides).
 */
function equippedRung(defId: string, picks: CardPicks, owns: (defId: string, id: string) => boolean): string | null {
  if (picks.fullArt === undefined && picks.augments === undefined) return null;
  return wornLevel(levelOfPick(defId, picks.fullArt, picks.augments), (id) => owns(defId, id)) ?? NO_FULL_ART_ID;
}

/** One foil's, finish's or border's small live preview: the card wearing it. */
function StripPicture({ face, item }: { face: FaceModel; item: StripItem }): ReactElement {
  const { rarityFoil } = useCardSettings();
  const wears = wornCosmetics(face, normalizeCosmetics({ [item.kind]: item.id }), {}, () => true, rarityFoil);
  return (
    <span className="collection-strip__picture">
      <SampleFace face={face} layout="compact" wears={wears} />
    </span>
  );
}

/** R2730000: one strip item: a hold or the inspect key previews it fullscreen, a click equips it. */
function StripItemButton({
  face,
  item,
  equipped,
  show,
  hide,
  equip,
  onPreview,
}: {
  face: FaceModel;
  item: StripItem;
  equipped: boolean;
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
      aria-pressed={equipped}
      aria-disabled={item.owned ? undefined : true}
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
      <StripPicture face={face} item={item} />
      <span className="collection-strip__name">{item.name}</span>
      <span className="collection-strip__rarity">{t(item.rarity)}</span>
      {item.owned ? null : <span className="collection-strip__lock">{t("Locked")}</span>}
    </button>
  );
}

function StripKindGroup({
  group,
  face,
  picks,
  show,
  hide,
  equip,
  clear,
  onPreview,
}: {
  group: StripGroup;
  face: FaceModel;
  picks: CardPicks;
  show: (item: StripItem) => void;
  hide: (item: StripItem) => void;
  equip: (item: StripItem) => void;
  clear: (kind: StripKind) => void;
  /** R2730000: what a held or inspected item opens: its fullscreen preview. */
  onPreview: (item: StripItem) => void;
}): ReactElement {
  return (
    <section className="collection-strip" data-testid={T.strip(group.kind)} aria-label={cosmeticKindWords(group.kind)}>
      <div className="collection-strip__head">
        <h4 className="collection-strip__title">{cosmeticKindWords(group.kind)}</h4>
        <button
          type="button"
          className="collection-strip__default"
          data-testid={T.stripDefault(group.kind)}
          aria-pressed={picks[group.kind] === undefined}
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
              face={face}
              item={item}
              equipped={picks[item.kind] === item.id}
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
  const [preview, setPreview] = useState<Shown | null>(null);
  const [previewing, setPreviewing] = useState<string | null>(null);
  const [textOpen, setTextOpen] = useState(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const face = useMemo(() => faceModel({ defId: def.id, def, radiant: false }), [def]);
  const strip = cardStrip(def.id, canEquip);
  const ladders = cardLadders(def.id, canEquip);
  const rungs = ladders.flatMap((ladder) => ladder.rungs);
  // R1953001: the held tokens are read only where one may be offered: a locked item, or R3015004 a ladder's next
  // level, on a card owned at MAX_COPIES, the only card a token's per-card cosmetic goes on (the server's `choice_of`).
  const offering = copies >= MAX_COPIES && (strip.some((group) => group.items.some((item) => !item.owned)) || rungs.some((rung) => rung.next));
  const [tokens, setTokens] = useHeldTokens(offering ? token : null);
  const shown: Shown[] = [...strip.flatMap((group) => group.items), ...rungs];
  const previewed = shown.find((item) => item.id === previewing) ?? null;

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
    if (item.owned) writeCardPick(def.id, item.kind, resolveCosmetic(item.kind, item.id));
  };
  // R3015005: an owned rung wears that one level, the normal face's `false`.
  const equipLevel = (rung: Rung): void => {
    if (rung.owned) writeCardLevel(def.id, rung.id === NO_FULL_ART_ID ? null : rung.id);
  };
  const clear = (kind: StripKind): void => {
    writeCardPick(def.id, kind, null);
  };
  // R3015004: a locked strip item, or a ladder's next level alone, is offered a held token that covers it.
  const offer = (item: Shown): ReactElement | null => {
    const cosmetic = COSMETICS.find((entry) => entry.id === item.id);
    const tier = cosmetic === undefined || !offering || (!isStripItem(item) && !item.next) ? null : offerTier(cosmetic, tokens);
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
            <VariantLadders
              def={def}
              face={face}
              ladders={ladders}
              equipped={equippedRung(def.id, picks, canEquip)}
              show={(rung) => {
                setPreview(rung);
              }}
              hide={(rung) => {
                setPreview((current) => (current?.id === rung.id ? null : current));
              }}
              equip={equipLevel}
              onPreview={(rung) => {
                setPreviewing(rung.id);
              }}
            />
            {/* R3013001: an owned card's proficiency and the stamps it has unlocked. */}
            {owned ? <ProficiencyPanel def={def} /> : null}
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
            !isStripItem(previewed) && !previewed.owned ? (
              <RungPicture def={def} face={face} rung={previewed} />
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
