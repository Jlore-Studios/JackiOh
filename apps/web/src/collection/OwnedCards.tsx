// The owned-cards panel (#1307, SPEC §14): a completion count per shipped set and, behind each set's
// toggle, its cards, owned as `GET /api/collection` says (CLAUDE.md rule 7). An unowned card is a
// locked silhouette that still opens its view (R1655000), so a player sees what they miss. R1953001,
// R2730000: `token` lets a held token's offer show in a locked item's fullscreen preview, which a hold
// or the inspect key opens: a card token on an unowned card of the grid, a cosmetic token in the
// card's view. R1956000: an owned tile wears the viewer's picks. R3016000: a card resting under a mouse
// or a keyboard's focus previews its look as a card, in hand and on the board (LookPreview.tsx), and its
// fullscreen preview draws the same three with a Radiant toggle.

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
import ItemPreview, { usePreviewTrigger } from "./ItemPreview.tsx";
import { LookHover, LookViews, RadiantToggle } from "./LookPreview.tsx";
import TokenOffer, { useHeldTokens, type HeldTokens } from "./TokenOffer.tsx";

type Inspected = { def: CardDef; owned: boolean };

/**
 * R2730000: one grid card: a hold or the inspect key previews it fullscreen, a click opens its view. R3016000: a
 * resting mouse or a keyboard's focus opens its look's hover preview, which leaving closes.
 */
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
  const { handlers, overlay } = usePreviewTrigger(`card-${def.id}`, onPreview, (anchor) => <LookHover def={def} worn={owned} anchor={anchor} />);
  return (
    <>
      <button
        type="button"
        className="collection-card"
        data-testid={collectionTestid.card(def.id)}
        data-owned={String(owned)}
        aria-label={owned ? readerName(def) : t("{name}, not owned yet", { name: readerName(def) })}
        aria-keyshortcuts={INSPECT_KEY_SHORTCUT}
        {...handlers}
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
      {overlay}
    </>
  );
}

/** R3016000: a grid card's fullscreen preview (R2730000): its look three ways, base until the Radiant toggle turns it. */
function CardLookPreview({
  def,
  owned,
  offer,
  onClose,
}: {
  def: CardDef;
  owned: boolean;
  offer: ReactElement | null;
  onClose: () => void;
}): ReactElement {
  const [radiant, setRadiant] = useState(false);
  return (
    <ItemPreview
      id={def.id}
      name={readerName(def)}
      rarity={def.rarity}
      owned={owned}
      picture={<LookViews def={def} worn={owned} radiant={radiant} />}
      controls={
        <RadiantToggle
          radiant={radiant}
          onToggle={() => {
            setRadiant((shown) => !shown);
          }}
        />
      }
      offer={offer}
      onClose={onClose}
    />
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
        <CardLookPreview
          key={previewed.def.id}
          def={previewed.def}
          owned={previewed.owned}
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
