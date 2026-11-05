// R3016000 (#3016, part 1): a card's look drawn three ways in the Collection, so a player sees what it
// will look like before they play it: as the card face, as the viewer's own hand draws it (R2723000's
// art-only face for a full-art card, R1295000's worn cosmetics) and as it stands on the board, a Unit
// as the lane's minion and any other card in its face-up field form. The hand and board views are the
// game's own `Card` (game/Card.tsx), the one a match draws, never a copy, so the preview cannot drift
// from the board; it gets a one-seat wearer (game/wearer.ts) so an owned card wears the device's picks
// as the viewer's own cards do in a match, and an unowned card the defaults.
//
// It decides nothing (CLAUDE.md rule 7): ownership is `GET /api/collection`'s, the picks are the
// device's store's, the numbers are the printed card's. A hover or a focus opens `LookHover`, which
// takes no pointer and is aria-hidden as the game's hover preview is, so it never covers what a click
// aims at; the fullscreen preview a hold or the inspect key opens (R2730000) draws `LookViews` with a
// Radiant toggle beside it.

import { useLayoutEffect, useRef, type ReactElement } from "react";
import { createPortal } from "react-dom";

import { CATALOG } from "@jackioh/cards";
import type { CardDef, CardView, PlayerId, UnitView } from "@jackioh/shared";

import { CardFace, faceModel } from "../cards/index.ts";
import { placePreview, type Rect } from "../cards/inspect/placement.ts";
import { OVERLAY_ROOT_PROPS } from "../cards/inspect/store.ts";
import Card from "../game/Card.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { printedCost } from "../game/decks.ts";
import { WearerProvider } from "../game/wearer.ts";
import { t } from "../i18n/i18n.ts";
import { collectionTestid as T } from "./collection.ts";

import "../game/board.css";
import "./lookPreview.css";

/** The seat the preview's cards are the viewer's own in: the hand's and the board's owner. */
const LOOK_SEAT: PlayerId = "p1";

/** The three ways a look is drawn, in the order the preview shows them. */
export const LOOK_VIEWS = ["card", "hand", "board"] as const;
export type LookView = (typeof LOOK_VIEWS)[number];

const LOOK_VIEW_WORDS: Readonly<Record<LookView, string>> = {
  card: "The card",
  hand: "In your hand",
  board: "On the board",
};

/** The hover preview's size before it is laid out; mirrors lookPreview.css's `.collection-look-hover`. */
const LOOK_HOVER_ESTIMATE_PX = { width: 460, height: 300 } as const;

const LOOKUP = lookupFromDefs(CATALOG);

/** The printed card as a hand card's view: its printed cost, nothing live. Each view takes its own instance id, so each keeps its own inspect overlay. */
function cardViewOf(def: CardDef, radiant: boolean, view: LookView): CardView {
  return { instanceId: `collection-look-${view}-${def.id}`, defId: def.id, radiant, cost: printedCost(def.cost) };
}

/** The printed Unit as a lane's unit: its printed stats and keywords, at full health, in attack position, idle. */
function unitViewOf(def: CardDef, radiant: boolean): UnitView {
  const face = radiant ? def.radiant : def.base;
  const health = face.health ?? 0;
  return {
    ...cardViewOf(def, radiant, "board"),
    owner: LOOK_SEAT,
    controller: LOOK_SEAT,
    attack: face.attack ?? 0,
    maxHealth: health,
    health,
    keywords: face.keywords,
    armor: 0,
    position: "ATK",
    counters: {},
    buried: 0,
    canAct: false,
  };
}

/**
 * R3016000: `def` three ways, base or Radiant: its full face, the viewer's hand card and its board form. `worn`
 * (the account owns the card) dresses all three in the device's picks, as the grid's own tile is dressed.
 */
export function LookViews({ def, worn, radiant }: { def: CardDef; worn: boolean; radiant: boolean }): ReactElement {
  const hand = cardViewOf(def, radiant, "hand");
  const board = cardViewOf(def, radiant, "board");
  const views: Readonly<Record<LookView, ReactElement>> = {
    card: (
      <span className="collection-look__card">
        <CardFace face={faceModel({ defId: def.id, def, radiant })} layout="full" worn={worn} />
      </span>
    ),
    hand: (
      <span className="hand-you collection-look__hand">
        <Card card={hand} className="card-hand" inOwnHand inHandOf={LOOK_SEAT} />
      </span>
    ),
    board:
      def.type === "Unit" ? (
        <span className="zone collection-look__zone" data-row="units">
          <Card card={board} unit={unitViewOf(def, radiant)} onField />
        </span>
      ) : (
        <span className="zone collection-look__zone" data-row="backrow">
          <Card card={board} type={def.type} owner={LOOK_SEAT} controller={LOOK_SEAT} onField />
        </span>
      ),
  };
  return (
    <CatalogContext.Provider value={LOOKUP}>
      <WearerProvider seat={worn ? LOOK_SEAT : null}>
        <span className="collection-look" data-testid={T.look} data-card={def.id} data-radiant={String(radiant)} data-worn={String(worn)}>
          {LOOK_VIEWS.map((view) => (
            <span key={view} className="collection-look__view" data-testid={T.lookView(view)}>
              <span className="collection-look__stage">{views[view]}</span>
              <span className="collection-look__caption">{t(LOOK_VIEW_WORDS[view])}</span>
            </span>
          ))}
        </span>
      </WearerProvider>
    </CatalogContext.Provider>
  );
}

/** R3016000: the Radiant toggle beside the fullscreen preview's views, the stats drill-down's words. */
export function RadiantToggle({ radiant, onToggle }: { radiant: boolean; onToggle: () => void }): ReactElement {
  return (
    <button type="button" className="collection-look__radiant" data-testid={T.lookRadiant} aria-pressed={radiant} onClick={onToggle}>
      {t("Show Radiant")}
    </button>
  );
}

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

/**
 * R3016000: the views a resting pointer or keyboard focus opens beside `anchor`, the base face, placed as the
 * game's hover preview is (cards/inspect/placement.ts). Fixed, aria-hidden and taking no pointer; its trigger
 * closes it (ItemPreview.tsx `usePreviewTrigger`).
 */
export function LookHover({ def, worn, anchor }: { def: CardDef; worn: boolean; anchor: Rect }): ReactElement {
  const ref = useRef<HTMLDivElement>(null);
  const placed = placePreview(anchor, viewportSize(), LOOK_HOVER_ESTIMATE_PX);

  // Once laid out, place it again by its real size. jsdom has no layout and keeps the estimate.
  useLayoutEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const width = element.offsetWidth;
    const height = element.offsetHeight;
    if (width === 0 || height === 0) return;
    const measured = placePreview(anchor, viewportSize(), { width, height });
    element.style.left = `${measured.left}px`;
    element.style.top = `${measured.top}px`;
    element.dataset.side = measured.side;
  });

  return createPortal(
    <div
      ref={ref}
      className="collection-look-hover"
      data-testid={T.lookHover}
      data-side={placed.side}
      aria-hidden="true"
      style={{ left: placed.left, top: placed.top }}
      {...OVERLAY_ROOT_PROPS}
    >
      <LookViews def={def} worn={worn} radiant={false} />
    </div>,
    document.body,
  );
}
