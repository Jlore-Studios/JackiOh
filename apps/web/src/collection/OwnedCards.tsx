// The Cards tab (#1307, SPEC §14; issue #3734, R3734000): one grid of the whole catalog, owned as `GET /api/collection`
// says (CLAUDE.md rule 7), with an always-visible search, the sort, the filter chips (a sidebar over 1024 px, a sheet
// under it), an owned count and a bar for each set. The search, filters and sort are the deck builder pool's
// (`game/deckbuilder/filters.ts`, `FilterBar.tsx`) and the grid is virtual as it is (`CardGrid.tsx`). An unowned card
// is a locked silhouette that still opens its view (R1655000), so a player sees what they miss; "owned" and
// "missing" only hide cards. R1953001,
// R2730000: `token` lets a held token's offer show in a locked item's fullscreen preview, which a hold
// or the inspect key opens: a card token on an unowned card of the grid, a cosmetic token in the
// card's view. R1956000: an owned tile wears the viewer's picks. R3016000: a mouse resting on a card, or the
// keyboard's focus, opens its hover preview (LookHover.tsx), its look as the card, in hand and on the board
// with a Radiant toggle; the fullscreen preview shows the same views. R3016001: "Equip or buy" on either opens the
// card's equip and buy screen (LooksScreen.tsx), over the fullscreen preview where it opened from there.

import { useMemo, useRef, useState, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";
import { setShips, type CardDef } from "@jackioh/shared";
import type { CatalogSnapshot } from "@jackioh/validator";

import { readerName } from "../cards/chinese.ts";
import { useCanOpenFullScreen, CardDefsProvider, CardFace, closeInspect, faceModel, INSPECT_KEY_SHORTCUT, openFullScreenArt } from "../cards/index.ts";
import FilterBar from "../game/deckbuilder/FilterBar.tsx";
import { FILTER_KEYWORDS, collectionPool } from "../game/deckbuilder/filters.ts";
import { DB_EMPTY } from "../game/deckbuilder/testids.ts";
import { formatNumber, t, useLocale } from "../i18n/i18n.ts";
import { getCollection, type CollectionResponse } from "../net/api.ts";
import { quantitiesOf } from "../shop/shop.ts";
import BrowsePanel from "./BrowsePanel.tsx";
import CardCosmeticsView from "./CardCosmeticsView.tsx";
import CardGrid from "./CardGrid.tsx";
import {
  CARDS_SORTS,
  COLLECTED_DEFS,
  DEFAULT_CARDS_BROWSE,
  collectionTestid,
  ownedIds,
  setCompletions,
  type CardsBrowse,
} from "./collection.ts";
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
        {/* R3350000: the player turns a card here to look at it, so a Mythic's diorama turns as far as it does in inspect. */}
        <CardFace face={faceModel({ defId: def.id, def, radiant: false })} layout="full" lazyArt worn={owned} tilt="host" turn />
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
  browse,
  onBrowse,
}: {
  collection: CollectionResponse;
  defs?: readonly CardDef[];
  token?: string | null;
  /** The route holds the search, filters and sort, so they survive a switch of tab; without it the tab holds its own. */
  browse?: CardsBrowse;
  onBrowse?: (next: CardsBrowse) => void;
}): ReactElement {
  const [own, setOwn] = useState(DEFAULT_CARDS_BROWSE);
  const now = browse ?? own;
  const { filter, sort, ownership } = now;
  // Clear filters changes the filter and the ownership view in one press, as two calls: each patches the state the
  // one before it left, never the render's.
  const latest = useRef(now);
  latest.current = now;
  const patchBrowse = (patch: Partial<CardsBrowse>): void => {
    latest.current = { ...latest.current, ...patch };
    (onBrowse ?? setOwn)(latest.current);
  };
  const locale = useLocale();
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
  const owned = useMemo(() => ownedIds(shown), [shown]);
  // What the grid holds is what a set counts (§14): the non-token cards of the sets that ship. The pool's own filter
  // and sort read these defs, so a `defs` that is not the catalog still filters as itself.
  const collected = useMemo(() => defs.filter((def) => !def.token && setShips(def.set)), [defs]);
  const catalog = useMemo<CatalogSnapshot>(
    () => ({ version: "bundled", cards: Object.fromEntries(collected.map((def) => [def.id, def])) }),
    [collected],
  );
  const shelf = useMemo(() => collected.map((def) => def.id), [collected]);
  const ids = useMemo(
    () => collectionPool(shelf, catalog, owned, ownership, filter, sort),
    // The locale: names and Chinese card words are what a search and the name sort read.
    [shelf, catalog, owned, ownership, filter, sort, locale],
  );
  const ownedTotal = completions.reduce((sum, set) => sum + set.owned, 0);
  const total = completions.reduce((sum, set) => sum + set.total, 0);
  const activeFilters =
    filter.sets.size + filter.costs.size + filter.types.size + filter.tags.size + filter.keywords.size + filter.rarities.size + (ownership === "all" ? 0 : 1);
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
      <div className="collection-cards">
        <BrowsePanel
          active={activeFilters}
          controls={
            <FilterBar
              part="chips"
              filter={filter}
              onFilter={(next) => {
                patchBrowse({ filter: next });
              }}
              sort={sort}
              onSort={(next) => {
                patchBrowse({ sort: next });
              }}
              count={ids.length}
              ownedControl={false}
              keywords={FILTER_KEYWORDS}
              ownership={ownership}
              onOwnership={(next) => {
                patchBrowse({ ownership: next });
              }}
            />
          }
        />
        <div className="collection-bar">
          <FilterBar
            part="toolbar"
            filter={filter}
            onFilter={(next) => {
              patchBrowse({ filter: next });
            }}
            sort={sort}
            onSort={(next) => {
              patchBrowse({ sort: next });
            }}
            count={ids.length}
            ownedControl={false}
            sorts={CARDS_SORTS}
            keywords={FILTER_KEYWORDS}
            ownership={ownership}
            onOwnership={(next) => {
              patchBrowse({ ownership: next });
            }}
          />
        </div>
        <div className="collection-sets">
          <p className="collection-count">
            <span>{t("Owned")}</span>{" "}
            <strong data-testid={collectionTestid.count}>
              {formatNumber(ownedTotal)} / {formatNumber(total)}
            </strong>
          </p>
          <ul className="collection-set-list">
            {completions.map(({ set, owned: have, total: of }) => (
              <li key={set} className="collection-set" data-testid={collectionTestid.set(set)}>
                <span className="collection-set__name">{t(set)}</span>
                <span className="collection-set__count" data-testid={collectionTestid.completion(set)}>
                  {t("{progress} of {target}", { progress: formatNumber(have), target: formatNumber(of) })}
                </span>
                <progress
                  className="collection-set__bar"
                  data-testid={collectionTestid.setProgress(set)}
                  value={have}
                  max={Math.max(of, 1)}
                  aria-label={t(set)}
                />
              </li>
            ))}
          </ul>
        </div>
        <div className="collection-scroll">
          {ids.length === 0 ? (
            <p className="db-empty" data-testid={DB_EMPTY}>
              {t("No card matches these filters.")}
            </p>
          ) : (
            <CardGrid
              ids={ids}
              render={(id) => {
                const def = catalog.cards[id];
                if (def === undefined) return <></>;
                const held = owned.has(id);
                return (
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
                );
              }}
            />
          )}
        </div>
      </div>
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
