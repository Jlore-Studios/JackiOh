// The owned-cards panel (#1307, SPEC §14): a completion count per shipped set and, behind each
// set's toggle, its cards. A card the account does not own is a locked silhouette that still
// opens its view, so a player sees what they are missing. The server's `GET
// /api/collection` says which are owned (CLAUDE.md rule 7); this counts the rows and draws them.
// R1655000: a card opens in its own view (CardCosmeticsView), large, wearing its own picks, with
// a strip of what it may wear; the card's text is one click from there. R1953001, R2730000:
// `token` lets a held token's offer show in a locked item's fullscreen preview (ItemPreview), a
// cosmetic token in the card's view, a card token on an unowned card of the grid. R1956000: an
// owned card's tile in the grid is the viewer's own copy too, and wears the same picks; a locked
// one draws the defaults.

import { useId, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, SetName } from "@jackioh/shared";

import { readerName } from "../cards/chinese.ts";
import { CardDefsProvider, CardFace, closeInspect, faceModel, INSPECT_KEY_SHORTCUT } from "../cards/index.ts";
import { formatNumber, t } from "../i18n/i18n.ts";
import { getCollection, type CollectionResponse } from "../net/api.ts";
import { quantitiesOf } from "../shop/shop.ts";
import CardCosmeticsView from "./CardCosmeticsView.tsx";
import { COLLECTED_DEFS, collectionTestid, setCompletions } from "./collection.ts";
import ItemPreview, { usePreviewHold } from "./ItemPreview.tsx";
import TokenOffer, { useHeldTokens, type HeldTokens } from "./TokenOffer.tsx";

type Inspected = { def: CardDef; owned: boolean };

/** R2730000: one grid card: a hold or the inspect key previews it fullscreen, a click opens its view. */
function CollectionCardTile({
  def,
  owned,
  onOpen,
  onPreview,
}: {
  def: CardDef;
  owned: boolean;
  onOpen: () => void;
  onPreview: () => void;
}): ReactElement {
  return (
    <button
      type="button"
      className="collection-card"
      data-testid={collectionTestid.card(def.id)}
      data-owned={String(owned)}
      aria-label={owned ? readerName(def) : t("{name}, not owned yet", { name: readerName(def) })}
      aria-keyshortcuts={INSPECT_KEY_SHORTCUT}
      {...usePreviewHold(`card-${def.id}`, onPreview)}
      onClick={() => {
        closeInspect();
        onOpen();
      }}
    >
      <span className="collection-card__face">
        <CardFace face={faceModel({ defId: def.id, def, radiant: false })} layout="full" lazyArt worn={owned} />
      </span>
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
  // R2730000: a redeemed card token re-reads the collection here, so the grid moves with the preview.
  const [reread, setReread] = useState<{ from: CollectionResponse; value: CollectionResponse } | null>(null);
  const shown = reread !== null && reread.from === collection ? reread.value : collection;
  const completions = setCompletions(shown, defs);
  const idBase = useId();
  // `held` names a card's ownership in the grid below; the tokens are `tokens`.
  const [tokens, setTokens] = useHeldTokens(completions.some((set) => set.owned < set.total) ? token : null);
  const previewed = completions.flatMap((set) => set.cards).find((card) => card.def.id === previewing) ?? null;

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
                      onOpen={() => {
                        setInspected({ def, owned: held });
                      }}
                      onPreview={() => {
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
          picture={
            <span className="collection-preview__card">
              <CardFace face={faceModel({ defId: previewed.def.id, def: previewed.def, radiant: false })} layout="full" worn={previewed.owned} />
            </span>
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
    </CardDefsProvider>
  );
}
