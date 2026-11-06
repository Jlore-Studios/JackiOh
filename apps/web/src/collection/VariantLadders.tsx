// R3015007 (#3015): a card's variants in its Collection card view (CardCosmeticsView.tsx), each a ladder of its levels
// lowest first (collection.ts `cardLadders`): one `collection-ladder` section per variant, the Default first, each level
// a rung, owned or locked. The next level, every level below it owned, is marked, and a locked rung past it names what
// it needs. Hovering or focusing a rung shows it on the card, choosing an owned one wears it on this card alone (the
// view's `equip`, `writeCardLevel`), and choosing a locked one, or holding any (R2730000), opens its fullscreen
// preview, where only the next one is offered a held token. It decides nothing (CLAUDE.md rule 7): ownership is the
// server's (`useCanEquip`), and the server refuses any level past the next (R3015004).

import type { ReactElement } from "react";

import type { CardDef } from "@jackioh/shared";

import { CardArt, INSPECT_KEY_SHORTCUT, useCardSettings } from "../cards/index.ts";
import { normalizeCosmetics, wornCosmetics } from "../cards/cosmetics.ts";
import type { FaceModel } from "../cards/model.ts";
import { DEFAULT_VARIANT, artPartOf, artPictureOf, levelOf } from "../cards/versions.ts";
import { t } from "../i18n/i18n.ts";
import { SampleFace } from "./CosmeticLook.tsx";
import { NO_FULL_ART_ID, collectionTestid as T, type Ladder, type Rung } from "./collection.ts";
import { usePreviewHold } from "./ItemPreview.tsx";

/**
 * R3015002: one rung's small picture: its variant's picture across the whole face (every level of a variant draws its
 * first level's), Out of frame's subject alone; the normal face drawn small for the Default's levels.
 */
export function RungPicture({ def, face, rung }: { def: CardDef; face: FaceModel; rung: Rung }): ReactElement {
  const { rarityFoil } = useCardSettings();
  if (rung.id !== NO_FULL_ART_ID && levelOf(rung.id)?.variant !== DEFAULT_VARIANT) {
    return (
      <span className="collection-strip__picture collection-strip__picture--full-art">
        <CardArt
          defId={def.id}
          radiant={false}
          tags={def.tags}
          type={def.type}
          name={def.name}
          shape="full"
          picture={artPictureOf(rung.id, def.set)}
          part={artPartOf(rung.id)}
        />
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
}: {
  def: CardDef;
  face: FaceModel;
  rung: Rung;
  equipped: boolean;
  show: (rung: Rung) => void;
  hide: (rung: Rung) => void;
  equip: (rung: Rung) => void;
  onPreview: (rung: Rung) => void;
}): ReactElement {
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
      {...usePreviewHold(`rung-${rung.id}`, () => {
        onPreview(rung);
      })}
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
      onPointerEnter={() => {
        show(rung);
      }}
      onPointerLeave={() => {
        hide(rung);
      }}
      onFocus={() => {
        show(rung);
      }}
      onBlur={() => {
        hide(rung);
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
                />
              </li>
            ))}
          </ol>
        </section>
      ))}
    </>
  );
}
