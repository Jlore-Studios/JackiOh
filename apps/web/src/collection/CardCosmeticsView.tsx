// A card in context (#1662, R1655000, part of #1651): the Collection's card view. A card opens large,
// wearing its own picks (`OwnCopy own`: the viewer's own copy, as the deck builder's detail view is),
// under a strip of what that card may wear, by kind: its foil, finish and border and, when it has
// one, its full art. Each item is a small live preview of the card wearing it.
//
// Hovering or focusing an item shows it on the large card; clicking one the card owns equips it on
// this card alone (`writeCardPick`, `writeCardFullArt`), and a locked one, which `useCanEquip` says no
// to, equips nothing. `Use default` of a kind clears the card's pick, which leaves the card to the
// account's. The view decides nothing (CLAUDE.md rule 7): it sets a device pick that only the
// viewer's own faces wear, and `GET /api/cosmetics` is the answer to what the viewer owns.
//
// A preview writes no pick. A face wears only what the store holds, and `cards/cosmetics.ts` has no
// seam to draw one wearing an id it does not, so a preview of a foil, finish or border is a second
// copy of the face, drawn unworn, to which `SampleFace` gives the four attributes
// `useCosmeticAttributes` would (React never sets them on an unworn face, so nothing fights over
// them), by the same `wornCosmetics`. A full art changes how the face is built, which only the store
// reaches, so its preview holds the pick for as long as the item is shown and gives it back as the
// item stops being shown: a boolean comes back exactly, as a foil's id (which may be one a newer
// client wrote) would not.
//
// The card's text and meta stay one click away: `Card text` opens the detail view over this one.
// Every animation here is a transform or opacity (collection.css) and holds still under reduced motion.

import { useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { CardArt, CardDetail, CardFace, faceModel } from "../cards/index.ts";
import {
  DEFAULT_CARD_COSMETICS,
  OwnershipContext,
  normalizeCosmetics,
  ownsCosmetic,
  readCardPicks,
  resolveCosmetic,
  useCanEquip,
  useCardPicks,
  useCosmetics,
  wornCosmetics,
  writeCardFullArt,
  writeCardPick,
  type CardPicks,
  type CardSlotKind,
  type CosmeticId,
} from "../cards/cosmetics.ts";
import { OwnCopy, useOwnCopy } from "../cards/fullArt.ts";
import type { FaceModel } from "../cards/model.ts";
import { t } from "../i18n/i18n.ts";
import {
  NO_FULL_ART_ID,
  cardStrip,
  collectionTestid as T,
  cosmeticKindWords,
  type StripGroup,
  type StripItem,
  type StripKind,
} from "./collection.ts";

/** The attributes of a face's cosmetics (`CosmeticAttributes`, cards/cosmetics.ts). */
const COSMETIC_ATTRIBUTES = ["data-cosmetic-foil", "data-cosmetic-motion", "data-finish", "data-frame"] as const;

/**
 * What `useCosmeticAttributes` gives a face wearing `wears`: nothing for a default. A preview's foil
 * holds still, as the cap on moving foils (COSMETIC_FOIL_ANIMATED_MAX) belongs to the faces of a game.
 */
function attributesOf(wears: Record<CardSlotKind, CosmeticId>): Record<(typeof COSMETIC_ATTRIBUTES)[number], string | undefined> {
  const foiled = wears.foil !== DEFAULT_CARD_COSMETICS.foil;
  return {
    "data-cosmetic-foil": foiled ? wears.foil : undefined,
    "data-cosmetic-motion": foiled ? "static" : undefined,
    "data-finish": wears.finish === DEFAULT_CARD_COSMETICS.finish ? undefined : wears.finish,
    "data-frame": wears.border === DEFAULT_CARD_COSMETICS.border ? undefined : wears.border,
  };
}

/** `face` drawn unworn, wearing `wears` by the attributes its stylesheet (cards/cosmetics.css) keys on. */
function SampleFace({ face, layout, wears }: { face: FaceModel; layout: "full" | "compact"; wears: Record<CardSlotKind, CosmeticId> }): ReactElement {
  const host = useRef<HTMLSpanElement>(null);
  const attributes = attributesOf(wears);
  useLayoutEffect(() => {
    const root = host.current?.firstElementChild;
    if (root === null || root === undefined) return;
    for (const name of COSMETIC_ATTRIBUTES) {
      const value = attributes[name];
      if (value === undefined) root.removeAttribute(name);
      else root.setAttribute(name, value);
    }
  });
  return (
    <span ref={host} className="collection-sample">
      <CardFace face={face} layout={layout} />
    </span>
  );
}

/**
 * The large face: the card wearing its own picks, and over it, while a foil, finish or border is
 * shown, a copy wearing that item in place of the card's pick of its kind.
 */
function LargeFace({ face, preview }: { face: FaceModel; preview: StripItem | null }): ReactElement {
  const own = useOwnCopy();
  const account = useCosmetics();
  const card = useCardPicks(face.defId);
  const owned = useContext(OwnershipContext);
  let sample: ReactElement | null = null;
  if (preview !== null && preview.kind !== "fullArt") {
    const picks: CardPicks = { ...card };
    picks[preview.kind] = resolveCosmetic(preview.kind, preview.id);
    // What it would wear were it the card's pick: the shown item, and the card's other picks where they are owned.
    const wears = wornCosmetics(face, account, picks, (id) => id === preview.id || ownsCosmetic(owned, face.defId, id));
    sample = (
      <span className="collection-cardview__preview" data-testid={T.cardViewPreview} data-previewing={preview.id} aria-hidden="true">
        <SampleFace face={face} layout="full" wears={wears} />
      </span>
    );
  }
  return (
    <span className="collection-cardview__stage">
      <span className="collection-cardview__face" data-testid={T.cardViewFace}>
        <CardFace face={face} layout="full" worn={own} />
      </span>
      {sample}
    </span>
  );
}

/**
 * While `preview` is a full art the card owns, the card holds that pick so its face prints as it
 * would, and gives its own back when the item stops being shown, the view closes or the page is
 * left. The returned `settle` says the pick now stands (the item was clicked), so it is not given back.
 */
function useFullArtPreview(defId: string, preview: StripItem | null): () => void {
  const settled = useRef(false);
  const shown = preview !== null && preview.kind === "fullArt" && preview.owned ? preview.id !== NO_FULL_ART_ID : null;
  useEffect(() => {
    if (shown === null) return undefined;
    const before = readCardPicks(defId).fullArt ?? null;
    settled.current = false;
    writeCardFullArt(defId, shown);
    const giveBack = (): void => {
      if (!settled.current) writeCardFullArt(defId, before);
      settled.current = true;
    };
    window.addEventListener("pagehide", giveBack);
    return () => {
      window.removeEventListener("pagehide", giveBack);
      giveBack();
    };
  }, [defId, shown]);
  return () => {
    settled.current = true;
  };
}

/** Whether `item` is the card's own pick of its kind. */
function isEquipped(item: StripItem, picks: CardPicks): boolean {
  if (item.kind === "fullArt") return picks.fullArt !== undefined && picks.fullArt === (item.id !== NO_FULL_ART_ID);
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
}: {
  group: StripGroup;
  def: CardDef;
  face: FaceModel;
  picks: CardPicks;
  show: (item: StripItem) => void;
  hide: (item: StripItem) => void;
  equip: (item: StripItem) => void;
  clear: (kind: StripKind) => void;
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
          const equipped = isEquipped(item, picks);
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
  onClose,
}: {
  def: CardDef;
  owned: boolean;
  onClose: () => void;
}): ReactElement {
  const canEquip = useCanEquip();
  const picks = useCardPicks(def.id);
  const [preview, setPreview] = useState<StripItem | null>(null);
  const [textOpen, setTextOpen] = useState(false);
  const closeButton = useRef<HTMLButtonElement>(null);
  const face = useMemo(() => faceModel({ defId: def.id, def, radiant: false }), [def]);
  const settle = useFullArtPreview(def.id, preview);

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

  const equip = (item: StripItem): void => {
    if (!item.owned) return;
    if (item.kind === "fullArt") {
      settle();
      writeCardFullArt(def.id, item.id !== NO_FULL_ART_ID);
    } else {
      writeCardPick(def.id, item.kind, resolveCosmetic(item.kind, item.id));
    }
  };
  const clear = (kind: StripKind): void => {
    if (kind === "fullArt") writeCardFullArt(def.id, null);
    else writeCardPick(def.id, kind, null);
  };

  return (
    <OwnCopy own>
      <div className="collection-cardview" data-testid={T.cardView} data-card={def.id} role="dialog" aria-modal="true" aria-label={def.name} onKeyDown={trapTab}>
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
            <h3 className="collection-cardview__title">{def.name}</h3>
            <p className="collection-cardview__status">{owned ? t("Owned") : t("Not owned yet")}</p>
            <p className="collection-cardview__hint">
              {t("Hover or focus a cosmetic to see it on the card. Choose one to wear it on this card only.")}
            </p>
            {cardStrip(def.id, canEquip).map((group) => (
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
