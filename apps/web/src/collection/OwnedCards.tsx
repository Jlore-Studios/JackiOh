// The owned-cards panel (#1307, SPEC §14): a completion count per shipped set and, behind each set's
// toggle, its cards, owned as `GET /api/collection` says (CLAUDE.md rule 7). An unowned card is a
// locked silhouette that still opens its view (R1655000), so a player sees what they miss. R1953001,
// R2730000: `token` lets a held token's offer show in a locked item's fullscreen preview, which a hold
// or the inspect key opens: a card token on an unowned card of the grid, a cosmetic token in the
// card's view. R1956000: an owned tile wears the viewer's picks. R3016000: a mouse resting on a card, or the
// keyboard's focus, opens its hover preview (LookHover.tsx), its look as the card, in hand and on the board
// with a Radiant toggle; the fullscreen preview shows the same views. R3016001: "Equip or buy" on either opens the
// card's equip and buy screen (LooksScreen.tsx), over the fullscreen preview where it opened from there.

import { useId, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, SetName } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { useCanOpenFullScreen, CardDefsProvider, CardFace, closeInspect, faceModel, INSPECT_KEY_SHORTCUT, openFullScreenArt } from "../cards/index.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getCollection, type CollectionResponse } from "../net/api.ts";
import { quantitiesOf } from "../shop/shop.ts";
import CardCosmeticsView from "./CardCosmeticsView.tsx";
import { COLLECTED_DEFS, collectionTestid, setCompletions } from "./collection.ts";
import ItemPreview, { usePreviewHold } from "./ItemPreview.tsx";
import { LookHoverPanel, useLookHover, type LookHoverTrigger } from "./LookHover.tsx";
import LookViews, { LooksButton, RadiantToggle } from "./LookViews.tsx";
import { clearSeenForCard, hasUnseen, unseenForCard, useNewLevels } from "./newLevels.ts";
import LooksScreen from "./LooksScreen.tsx";
import TokenOffer, { useHeldTokens, type HeldTokens } from "./TokenOffer.tsx";

type Inspected = { def: CardDef; owned: boolean };

/**
 * R2730000: one grid card: a hold or the inspect key previews it fullscreen, a click opens its view.
 * R3016000: a resting mouse or the keyboard's focus opens its hover preview; a press closes it first.
 */
function CollectionCardTile({
  def,
  owned,
  isNew,
  hover,
  onOpen,
  onPreview,
}: {
  def: CardDef;
  owned: boolean;
  /** R3349000: the card earned a level the account has not viewed yet. */
  isNew: boolean;
  hover: LookHoverTrigger;
  onOpen: () => void;
  onPreview: () => void;
}): ReactElement {
  const isFs = useCanOpenFullScreen(def, def.id, owned);
  const hold = usePreviewHold(`card-${def.id}`, (pointerId) => {
    // The inspect key (no pointer) keeps the preview, with its equip and buy buttons; only a hold opens the art.
    if (isFs && pointerId !== undefined) {
      openFullScreenArt({
        defId: def.id,
        name: readerName(def),
        radiant: false,
        heldPointerId: pointerId,
      });
      return;
    }
    onPreview();
  });
  return (
    <button
      type="button"
      className="collection-card"
      data-testid={collectionTestid.card(def.id)}
      data-owned={String(owned)}
      aria-label={owned ? readerName(def) : t("{name}, not owned yet", { name: readerName(def) })}
      aria-keyshortcuts={INSPECT_KEY_SHORTCUT}
      data-new={isNew ? "true" : undefined}
      {...hold}
      {...hover}
      onPointerDown={(event) => {
        hover.onPointerDown();
        hold.onPointerDown(event);
      }}
      onClick={() => {
        closeInspect();
        onOpen();
      }}
    >
      <span className="collection-card__face">
        <CardFace face={faceModel({ defId: def.id, def, radiant: false })} layout="full" lazyArt worn={owned} tilt="host" />
      </span>
      {isNew ? (
        <span className="collection-card__new" data-testid={collectionTestid.newMark(def.id)}>
          {t("New")}
        </span>
      ) : null}
      {owned ? null : (
        <span className="collection-card__lock" aria-hidden="true">
          {t("Locked")}
        </span>
      )}
    </button>
  );
}

export default function OwnedCards({
  collection,
  defs = COLLECTED_DEFS,
  token = null,
}: {
  collection: CollectionResponse;
  defs?: readonly CardDef[];
  token?: string | null;
}): ReactElement {
  const [openSet, setOpenSet] = useState<SetName | null>(null);
  const [inspected, setInspected] = useState<Inspected | null>(null);
  const [previewing, setPreviewing] = useState<string | null>(null);
  // R3016000: the fullscreen preview's Radiant toggle, base whenever a preview opens.
  const [previewRadiant, setPreviewRadiant] = useState(false);
  // R3016001: the card whose equip and buy screen is open.
  const [looking, setLooking] = useState<string | null>(null);
  const hover = useLookHover();
  // R2730000: a redeemed card token re-reads the collection here, so the grid moves with the preview.
  const [reread, setReread] = useState<{ from: CollectionResponse; value: CollectionResponse } | null>(null);
  const shown = reread !== null && reread.from === collection ? reread.value : collection;
  const completions = setCompletions(shown, defs);
  const idBase = useId();
  // `held` names a card's ownership in the grid below; the tokens are `tokens`.
  const [tokens, setTokens] = useHeldTokens(completions.some((set) => set.owned < set.total) ? token : null);
  const shownCards = completions.flatMap((set) => set.cards);
  const previewed = shownCards.find((card) => card.def.id === previewing) ?? null;
  const hoveredId = hover.hovered?.id;
  const hovered = shownCards.find((card) => card.def.id === hoveredId) ?? null;
  const looked = shownCards.find((card) => card.def.id === looking) ?? null;
  // R3349000: marks written at a grant's success move the grid with it; viewing a card clears its own.
  useNewLevels();
  const clearNew = (cardId: string): void => {
    if (token !== null) clearSeenForCard(token, cardId);
  };
  // The hover preview plays the newest unseen level moving (R3016000), if the card earned one.
  const hoveredNew = token !== null && hovered !== null ? unseenForCard(token, hovered.def.id).at(-1) : undefined;

  const redeemed = (answer: HeldTokens): void => {
    setTokens(answer);
    if (token === null) return;
    getCollection(token).then(
      (next) => {
        setReread({ from: collection, value: next });
      },
      () => {
        // The next visit reads it.
      },
    );
  };

  return (
    <CardDefsProvider defs={CATALOG}>
      {completions.map(({ set, owned, total, cards }) => {
        const open = openSet === set;
        const gridId = `${idBase}-${set}`;
        return (
          <div key={set} className="collection-set" data-testid={collectionTestid.set(set)}>
            <button
              type="button"
              className="collection-set__toggle"
              data-testid={collectionTestid.setToggle(set)}
              aria-expanded={open}
              aria-controls={gridId}
              onClick={() => {
                setOpenSet(open ? null : set);
              }}
            >
              <span>{t(set)}</span>
              <span data-testid={collectionTestid.completion(set)}>
                {t("{progress} of {target}", { progress: formatNumber(owned), target: formatNumber(total) })}
              </span>
            </button>
            {open ? (
              <ul id={gridId} className="collection-grid">
                {cards.map(({ def, owned: held }) => (
                  <li key={def.id}>
                    <CollectionCardTile
                      def={def}
                      owned={held}
                      isNew={token !== null && hasUnseen(token, def.id)}
                      hover={hover.trigger(def.id)}
                      onOpen={() => {
                        hover.close();
                        clearNew(def.id);
                        setInspected({ def, owned: held });
                      }}
                      onPreview={() => {
                        hover.close();
                        clearNew(def.id);
                        setPreviewRadiant(false);
                        setPreviewing(def.id);
                      }}
                    />
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        );
      })}
      {hovered === null || hover.hovered === null || inspected !== null || previewed !== null || looked !== null ? null : (
        <LookHoverPanel
          key={hovered.def.id}
          def={hovered.def}
          owned={hovered.owned}
          anchor={hover.hovered.anchor}
          hover={hover}
          level={hoveredNew}
          onLooks={() => {
            hover.close();
            clearNew(hovered.def.id);
            setLooking(hovered.def.id);
          }}
        />
      )}
      {inspected === null ? null : (
        <CardCosmeticsView
          key={inspected.def.id}
          def={inspected.def}
          owned={inspected.owned}
          copies={quantitiesOf(shown).get(inspected.def.id) ?? 0}
          token={token}
          onClose={() => {
            setInspected(null);
          }}
        />
      )}
      {previewed === null ? null : (
        <ItemPreview
          id={previewed.def.id}
          name={readerName(previewed.def)}
          rarity={previewed.def.rarity}
          owned={previewed.owned}
          picture={<LookViews def={previewed.def} radiant={previewRadiant} worn={previewed.owned} />}
          controls={
            <div className="collection-look__actions">
              <RadiantToggle
                radiant={previewRadiant}
                onToggle={() => {
                  setPreviewRadiant((shown) => !shown);
                }}
              />
              <LooksButton
                onLooks={() => {
                  clearNew(previewed.def.id);
                  setLooking(previewed.def.id);
                }}
              />
            </div>
          }
          offer={
            previewed.owned || token === null || tokens === null || tokens.card < 1 ? null : (
              <TokenOffer
                key={previewed.def.id}
                token={token}
                spend={{ kind: "card", cardId: previewed.def.id }}
                held={tokens}
                name={readerName(previewed.def)}
                onRedeemed={redeemed}
              />
            )
          }
          onClose={() => {
            setPreviewing(null);
          }}
        />
      )}
      {looked === null ? null : (
        <LooksScreen
          key={looked.def.id}
          def={looked.def}
          copies={quantitiesOf(shown).get(looked.def.id) ?? 0}
          token={token}
          onClose={() => {
            setLooking(null);
          }}
          restoreFocus={() => {
            document.querySelector<HTMLElement>(`[data-testid="${collectionTestid.card(looked.def.id)}"]`)?.focus();
          }}
        />
      )}
    </CardDefsProvider>
  );
}
