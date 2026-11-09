// R3015007 (#3015): a card's variants in its Collection card view (CardCosmeticsView.tsx), each a ladder of its levels
// lowest first (collection.ts `cardLadders`): one `collection-ladder` section per variant, the Default first, each level
// a rung, owned or locked. The next level, every level below it owned, is marked, and a locked rung past it names what
// it needs. Hovering or focusing a rung shows it on the card, choosing an owned one wears it on this card alone (the
// view's `equip`, `writeCardLevel`), and choosing a locked one, or holding any (R2730000), opens its fullscreen
// preview, where only the next one is offered a held token. It decides nothing (CLAUDE.md rule 7): ownership is the
// server's (`useCanEquip`), and the server refuses any level past the next (R3015004). R3016001: a rung's `hover`, where
// given, opens that level's hover preview as well (LookHover.tsx): the card, in hand and on the board. R3343002: a
// Default rung of a card with a default look pictures that look (versions.ts `lookOn`).

import type { ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { CardArt, INSPECT_KEY_SHORTCUT, useCardSettings } from "../cards/index.ts";
import { normalizeCosmetics, useArtMotion, wornCosmetics } from "../cards/cosmetics.ts";
import { artMotionOf } from "../cards/fullArt.ts";
import { FullArtSignature } from "../cards/FullArtSignature.tsx";
import type { FaceModel } from "../cards/model.ts";
import { DEFAULT_VARIANT, artPartOf, artPictureOf, levelOf, lookOfLevel, lookOn } from "../cards/versions.ts";
import { t } from "../i18n/i18n.ts";
import { SampleFace } from "./CosmeticLook.tsx";
import { NO_FULL_ART_ID, collectionTestid as T, type Ladder, type Rung } from "./collection.ts";
import { usePreviewHold } from "./ItemPreview.tsx";
import type { LookHoverTrigger } from "./LookHover.tsx";

/**
 * R3015002: one rung's small picture: its variant's picture across the whole face (every level of a variant draws its
 * first level's), Out of frame's subject alone; the normal face drawn small for the Default's levels. R3343002: but the
 * card's default look across the whole face for those, where the card has one (`lookOn`, the setting's say included).
 */
export function RungPicture({ def, face, rung }: { def: CardDef; face: FaceModel; rung: Rung }): ReactElement {
  const { rarityFoil, fullArt, animatedFoil } = useCardSettings();
  const variant = rung.id === NO_FULL_ART_ID ? DEFAULT_VARIANT : levelOf(rung.id)?.variant;
  const look = variant === DEFAULT_VARIANT ? lookOn(def.id, null, fullArt).version : rung.id;
  // R3349000: each rung draws its own level's look: a level at or above an animated one moves its picture's own
  // layers while a motion slot is theirs (the face's rule, `useArtMotion`), and a signed one signs.
  const { animated, signed } = lookOfLevel(rung.id);
  const artMotion = useArtMotion(animated && animatedFoil);
  const motion = artMotion && animated ? artMotionOf(def.id) : null;
  if (look !== null) {
    return (
      <span
        className="collection-strip__picture collection-strip__picture--full-art"
        data-art-motion={animated ? (artMotion ? "animated" : "static") : undefined}
        data-signed={signed ? "true" : undefined}
      >
        <CardArt
          defId={def.id}
          radiant={false}
          tags={def.tags}
          type={def.type}
          name={def.name}
          shape="full"
          picture={artPictureOf(look, def.set)}
          part={artPartOf(look)}
          motion={motion}
        />
        {signed ? <FullArtSignature name={def.name} defId={def.id} /> : null}
      </span>
    );
  }
  return (
    <span className="collection-strip__picture">
      <SampleFace face={face} layout="compact" wears={wornCosmetics(face, normalizeCosmetics({}), {}, () => true, rarityFoil)} />
    </span>
  );
}

/** R3015007: one rung: a click wears an owned one and previews a locked one; a hold or the inspect key previews either. */
function RungButton({
  def,
  face,
  rung,
  equipped,
  show,
  hide,
  equip,
  onPreview,
  hover,
}: {
  def: CardDef;
  face: FaceModel;
  rung: Rung;
  equipped: boolean;
  show: (rung: Rung) => void;
  hide: (rung: Rung) => void;
  equip: (rung: Rung) => void;
  onPreview: (rung: Rung) => void;
  hover?: LookHoverTrigger | undefined;
}): ReactElement {
  const hold = usePreviewHold(`rung-${rung.id}`, () => {
    onPreview(rung);
  });
  return (
    <button
      type="button"
      className="collection-strip__item collection-rung"
      data-testid={T.rung(rung.id)}
      data-level={rung.id}
      data-owned={String(rung.owned)}
      data-next={String(rung.next)}
      data-equipped={String(equipped)}
      data-rarity={rung.rarity}
      aria-pressed={equipped}
      aria-keyshortcuts={INSPECT_KEY_SHORTCUT}
      {...hold}
      onPointerDown={(event) => {
        hover?.onPointerDown();
        hold.onPointerDown(event);
      }}
      onClick={() => {
        if (rung.owned) equip(rung);
        else onPreview(rung);
      }}
      onMouseEnter={() => {
        show(rung);
      }}
      onMouseLeave={() => {
        hide(rung);
      }}
      onPointerEnter={(event) => {
        show(rung);
        hover?.onPointerEnter(event);
      }}
      onPointerLeave={() => {
        hide(rung);
        hover?.onPointerLeave();
      }}
      onFocus={(event) => {
        show(rung);
        hover?.onFocus(event);
      }}
      onBlur={(event) => {
        hide(rung);
        hover?.onBlur(event);
      }}
    >
      <RungPicture def={def} face={face} rung={rung} />
      <span className="collection-strip__name">{rung.name}</span>
      <span className="collection-strip__rarity">{t(rung.rarity)}</span>
      {rung.next ? <span className="collection-strip__lock collection-rung__next">{t("Next level")}</span> : null}
      {rung.needs === null ? null : <span className="collection-strip__lock">{t("Needs {name}", { name: rung.needs })}</span>}
    </button>
  );
}

/** R3015007: every ladder of a card, one section per variant, `equipped` the level the card wears (null for none). */
export default function VariantLadders({
  def,
  face,
  ladders,
  equipped,
  show,
  hide,
  equip,
  onPreview,
  hover,
}: {
  def: CardDef;
  face: FaceModel;
  ladders: readonly Ladder[];
  equipped: string | null;
  show: (rung: Rung) => void;
  hide: (rung: Rung) => void;
  equip: (rung: Rung) => void;
  /** R2730000: what a held, inspected or locked rung opens: its fullscreen preview. */
  onPreview: (rung: Rung) => void;
  /** R3016001: a rung's hover preview handlers (LookHover.tsx `useLookHover`); none opens none. */
  hover?: ((rung: Rung) => LookHoverTrigger) | undefined;
}): ReactElement {
  return (
    <>
      {ladders.map((ladder) => (
        <section
          key={ladder.variant}
          className="collection-strip collection-ladder"
          data-testid={T.ladder(ladder.variant)}
          data-variant={ladder.variant}
          aria-label={ladder.name}
        >
          <div className="collection-strip__head">
            <h4 className="collection-strip__title">{ladder.name}</h4>
          </div>
          <ol className="collection-strip__items collection-ladder__rungs">
            {ladder.rungs.map((rung) => (
              <li key={rung.id}>
                <RungButton
                  def={def}
                  face={face}
                  rung={rung}
                  equipped={rung.id === equipped}
                  show={show}
                  hide={hide}
                  equip={equip}
                  onPreview={onPreview}
                  hover={hover?.(rung)}
                />
              </li>
            ))}
          </ol>
        </section>
      ))}
    </>
  );
}
