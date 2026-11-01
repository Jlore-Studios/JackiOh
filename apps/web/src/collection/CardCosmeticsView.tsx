// A card in context (#1662, R1655000, part of #1651): the Collection's card view. A card opens large,
// wearing its own picks (`OwnCopy own`: the viewer's own copy, as the deck builder's detail view is),
// under a strip of what that card may wear, by kind: its foil, finish and border and, when it has
// them, its full arts (R2655000: no full art, its Epic, its animated Legendary and its signed Mythic,
// one of which the card wears). Each item is a small live preview of the card wearing it.
//
// Hovering or focusing an item shows it on the large card; clicking one the card owns equips it on
// this card alone (`writeCardPick`, `writeCardFullArt`), and a locked one, which `useCanEquip` says no
// to, equips nothing. `Use default` of a kind clears the card's pick, which leaves the card to its
// kind's default (R1953001). The view decides nothing (CLAUDE.md rule 7): it sets a pick in the
// device's store, which `cards/cardPicksSync.ts` sends to the account (R1834000), for the viewer's own
// faces and, frozen as an online match begins, the opponent's sight of them (R1834002);
// `GET /api/cosmetics` is the answer to what the viewer owns. R1953001: a locked one a held cosmetic
// token covers offers that token in place (TokenOffer.tsx), on a card the account owns `MAX_COPIES`
// of, the only card a token's per-card cosmetic goes on.
//
// A preview writes no pick. A face wears only what the store holds, and `cards/cosmetics.ts` has no
// seam to draw one wearing an id it does not, so a preview of a foil, finish or border is a second
// copy of the face, drawn unworn, to which `SampleFace` (CosmeticLook.tsx) gives the four attributes
// `useCosmeticAttributes` would (React never sets them on an unworn face, so nothing fights over
// them), by the same `wornCosmetics`. A full art changes how the face is built, so its preview puts
// the large face below a `FullArtShown` (fullArt.ts), which prints it as that pick would, behind the
// same ownership. R1834000: the store is never held for a preview, since every write to it is sent to
// the account, and a preview the page never gave back would stand there as a pick opponents are shown.
//
// The card's text and meta stay one click away: `Card text` opens the detail view over this one.
// Every animation here is a transform or opacity (collection.css) and holds still under reduced motion.

import { useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactElement } from "react";

import { MAX_COPIES } from "@jackioh/engine/config";
import type { CardDef } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { CardArt, CardDetail, CardFace, faceModel } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  OwnershipContext,
  normalizeCosmetics,
  ownsCosmetic,
  resolveCosmetic,
  useCanEquip,
  useCardPicks,
  wornCosmetics,
  writeCardFullArt,
  writeCardPick,
  type CardPicks,
} from "../cards/cosmetics.ts";
import { FullArtShown, OwnCopy, fullArtIdOf, fullArtPickOf, useOwnCopy } from "../cards/fullArt.ts";
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
import TokenOffer, { offerTier, useHeldTokens } from "./TokenOffer.tsx";

/**
 * The large face: the card wearing its own picks, and over it, while a foil, finish or border is
 * shown, a copy wearing that item in place of the card's pick of its kind.
 */
function LargeFace({ face, preview }: { face: FaceModel; preview: StripItem | null }): ReactElement {
  const own = useOwnCopy();
  const card = useCardPicks(face.defId);
  const owned = useContext(OwnershipContext);
  const largeFace = <CardFace face={face} layout="full" worn={own} />;
  let sample: ReactElement | null = null;
  if (preview !== null && preview.kind !== "fullArt") {
    const picks: CardPicks = { ...card };
    picks[preview.kind] = resolveCosmetic(preview.kind, preview.id);
    // What it would wear were it the card's pick: the shown item, and the card's other picks where
    // they are owned, each kind it has none of its default (R1953001).
    const wears = wornCosmetics(face, DEFAULT_CARD_COSMETICS, picks, (id) => id === preview.id || ownsCosmetic(owned, face.defId, id));
    sample = (
      <span className="collection-cardview__preview" data-testid={T.cardViewPreview} data-previewing={preview.id} aria-hidden="true">
        <SampleFace face={face} layout="full" wears={wears} />
      </span>
    );
  }
  return (
    <span className="collection-cardview__stage">
      <span className="collection-cardview__face" data-testid={T.cardViewFace}>
        {preview !== null && preview.kind === "fullArt" && preview.owned ? (
          <FullArtShown defId={face.defId} pick={preview.id === NO_FULL_ART_ID ? false : fullArtPickOf(face.defId, preview.id)}>
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

/** Whether `item` is the card's own pick of its kind: for a full art, the tier the card's pick names (R2655000). */
function isEquipped(defId: string, item: StripItem, picks: CardPicks): boolean {
  if (item.kind === "fullArt") return item.id === NO_FULL_ART_ID ? picks.fullArt === false : fullArtIdOf(defId, picks.fullArt) === item.id;
  return picks[item.kind] === item.id;
}

/** One item's small live preview: the card wearing it, or for a full art the art across the whole face. */
function StripPicture({ def, face, item }: { def: CardDef; face: FaceModel; item: StripItem }): ReactElement {
  if (item.kind === "fullArt" && item.id !== NO_FULL_ART_ID) {
    return (
      <span className="collection-strip__picture collection-strip__picture--full-art">
        <CardArt defId={def.id} radiant={false} tags={def.tags} type={def.type} name={def.name} shape="full" />
      </span>
    );
  }
  const wears =
    item.kind === "fullArt"
      ? wornCosmetics(face, normalizeCosmetics({}))
      : wornCosmetics(face, normalizeCosmetics({ [item.kind]: item.id }));
  return (
    <span className="collection-strip__picture">
      <SampleFace face={face} layout="compact" wears={wears} />
    </span>
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
  offer,
}: {
  group: StripGroup;
  def: CardDef;
  face: FaceModel;
  picks: CardPicks;
  show: (item: StripItem) => void;
  hide: (item: StripItem) => void;
  equip: (item: StripItem) => void;
  clear: (kind: StripKind) => void;
  /** R1953001: what a locked item offers, a held token that covers it, else nothing. */
  offer: (item: StripItem) => ReactElement | null;
}): ReactElement {
  const own = group.kind === "fullArt" ? picks.fullArt : picks[group.kind];
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
        {group.items.map((item) => {
          const equipped = isEquipped(def.id, item, picks);
          return (
            <li key={item.id}>
              <button
                type="button"
                className="collection-strip__item"
                data-testid={T.stripItem(item.id)}
                data-owned={String(item.owned)}
                data-equipped={String(equipped)}
                data-rarity={item.rarity}
                aria-pressed={equipped}
                aria-disabled={item.owned ? undefined : true}
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
              </button>
              {item.owned ? null : offer(item)}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** Keeps Tab inside the dialog, which holds nothing but buttons. */
function trapTab(event: KeyboardEvent<HTMLElement>): void {
  if (event.key !== "Tab") return;
  const buttons = [...event.currentTarget.querySelectorAll<HTMLElement>("button:not([disabled])")];
  const first = buttons[0];
  const last = buttons[buttons.length - 1];
  if (first === undefined || last === undefined) return;
  const active = document.activeElement;
  if (event.shiftKey && active === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
  }
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
  const [textOpen, setTextOpen] = useState(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const face = useMemo(() => faceModel({ defId: def.id, def, radiant: false }), [def]);
  const strip = cardStrip(def.id, canEquip);
  // R1953001: the held tokens are read only where one may be offered: a locked item on a card owned at
  // MAX_COPIES, the only card a token's per-card cosmetic goes on (the server's `choice_of`).
  const offering = copies >= MAX_COPIES && strip.some((group) => group.items.some((item) => !item.owned));
  const [tokens, setTokens] = useHeldTokens(offering ? token : null);

  // Focus moves in and goes back to the card that opened the view; Escape closes it, unless the card's text is over it.
  useLayoutEffect(() => {
    const opener = document.activeElement;
    closeButton.current?.focus({ preventScroll: true });
    return () => {
      if (opener instanceof HTMLElement && opener.isConnected) opener.focus();
    };
  }, []);
  useEffect(() => {
    if (textOpen) return undefined;
    const onKeyDown = (event: globalThis.KeyboardEvent): void => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [textOpen, onClose]);
  // R2127000: Back closes the view as Close does, and the focus goes back as above. The card's text,
  // opened over it, is a layer of its own on top, so Back closes that first.
  useBackLayer(onClose);

  const equip = (item: StripItem): void => {
    if (!item.owned) return;
    if (item.kind === "fullArt") writeCardFullArt(def.id, item.id === NO_FULL_ART_ID ? false : fullArtPickOf(def.id, item.id));
    else writeCardPick(def.id, item.kind, resolveCosmetic(item.kind, item.id));
  };
  const clear = (kind: StripKind): void => {
    if (kind === "fullArt") writeCardFullArt(def.id, null);
    else writeCardPick(def.id, kind, null);
  };
  const offer = (item: StripItem): ReactElement | null => {
    const cosmetic = COSMETICS.find((entry) => entry.id === item.id);
    const tier = cosmetic === undefined || !offering ? null : offerTier(cosmetic, tokens);
    if (token === null || tokens === null || tier === null) return null;
    return (
      <TokenOffer
        token={token}
        tier={tier}
        held={tokens}
        cosmeticId={item.id}
        name={item.name}
        cardId={def.id}
        onRedeemed={setTokens}
        testid={T.tokenOffer(item.id)}
        errorTestid={T.tokenOfferError(item.id)}
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
                offer={offer}
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
    </OwnCopy>
  );
}
