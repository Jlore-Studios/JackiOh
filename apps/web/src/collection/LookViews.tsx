// R3016000 (#3016): one card's look three ways, as a game draws it: the card (its whole face), in your
// hand (the hand's face, R2723000's art-only full art included) and on the board (a Unit as a lane's
// minion, a Spell as it resolves, a Trap or a Field card as the backrow shows it set or standing).
// Each is the cards module's own face, CardFace or MinionFace, the very ones game/Card.tsx draws, never
// a copy, so the preview can't drift from the board. It wears the viewer's picks (`worn`, R1655000),
// its foil, finish and border, and moves where the look is animated, honoring Reduce motion as every
// face does. Presentation only (CLAUDE.md rule 7): it reads the catalog and the viewer's own picks and
// decides nothing. R3016001: `level` dresses the three in one level of the card's variants, owned or not, as the
// equip and buy screen's hovered rung, writing no pick (`ShownLevel`).

import { useContext, useMemo, type ReactElement, type ReactNode } from "react";

import type { CardDef, UnitView } from "@jackioh/shared";

import { OwnershipContext, type CardOwnership } from "../cards/cosmetics.ts";
import { FullArtShown } from "../cards/fullArt.ts";
import { CardFace, MinionFace, faceModel } from "../cards/index.ts";
import { t } from "../i18n/i18n.ts";
import { NO_FULL_ART_ID, collectionTestid as T } from "./collection.ts";

/** Where a card stands on the board: a Unit in a lane, a Spell resolving, the rest in the backrow. */
export type BoardForm = "minion" | "resolving" | "backrow";

/** The three views, in the order they are drawn. */
export const LOOK_VIEWS = ["card", "hand", "board"] as const;
export type LookView = (typeof LOOK_VIEWS)[number];

/** The board form of a face of type `type`: R3016000. */
export function boardFormOf(type: CardDef["type"]): BoardForm {
  if (type === "Unit") return "minion";
  if (type === "Spell") return "resolving";
  return "backrow";
}

/** A copy of `def` just summoned into a lane: its printed side's numbers and keywords, in attack position, awake. */
export function boardUnitOf(def: CardDef, radiant: boolean): UnitView {
  const side = radiant ? def.radiant : def.base;
  const health = side.health ?? 0;
  return {
    instanceId: `look-${def.id}`,
    defId: def.id,
    radiant,
    cost: typeof def.cost === "number" ? def.cost : 0,
    owner: "p1",
    controller: "p1",
    attack: side.attack ?? 0,
    maxHealth: health,
    health,
    keywords: side.keywords,
    armor: 0,
    position: "ATK",
    counters: {},
    buried: 0,
    canAct: false,
  };
}

const CAPTIONS: Readonly<Record<LookView, string>> = {
  card: "The card",
  hand: "In your hand",
  board: "On the board",
};

/** R3016001: `owned` with `level` owned on `defId` too, so a copy shown wearing it wears it, a level it lacks among them. */
export function lendLevel(owned: CardOwnership, defId: string, level: string): CardOwnership {
  if (owned === "every" || level === NO_FULL_ART_ID) return owned;
  const lent = new Map(owned);
  lent.set(defId, new Set([...(owned.get(defId) ?? []), level]));
  return lent;
}

/** R3016001: the viewer's copies of `defId` below wear `level` (the normal face for `NO_FULL_ART_ID`), owned or not, and no pick is written. */
export function ShownLevel({ defId, level, children }: { defId: string; level: string; children: ReactNode }): ReactElement {
  const owned = useContext(OwnershipContext);
  const lent = useMemo(() => lendLevel(owned, defId, level), [owned, defId, level]);
  return (
    <OwnershipContext.Provider value={lent}>
      <FullArtShown defId={defId} pick={level === NO_FULL_ART_ID ? false : level}>
        {children}
      </FullArtShown>
    </OwnershipContext.Provider>
  );
}

/**
 * R3016000: `def`'s look as the card, in hand and on the board, base or Radiant; `worn` is the viewer's own copy.
 * R3016001: with `level`, the viewer's copy wearing that one level.
 */
export default function LookViews({
  def,
  radiant,
  worn,
  level,
}: {
  def: CardDef;
  radiant: boolean;
  worn: boolean;
  level?: string | undefined;
}): ReactElement {
  if (level !== undefined) {
    return (
      <ShownLevel defId={def.id} level={level}>
        <LookViews def={def} radiant={radiant} worn />
      </ShownLevel>
    );
  }
  const face = faceModel({ defId: def.id, def, radiant });
  // B2.7: a Radiant face may be another type than the base (Blood Moon's is a Field Trap), so the face's type decides.
  const form = boardFormOf(face.type);
  const views: Readonly<Record<LookView, ReactElement>> = {
    card: <CardFace face={face} layout="full" worn={worn} />,
    // R2723000: game/Card.tsx sets `ownHand` on the viewer's own worn hand card, as here.
    hand: <CardFace face={face} layout="full" worn={worn} ownHand={worn} />,
    board:
      form === "minion" ? (
        <MinionFace face={face} unit={boardUnitOf(def, radiant)} worn={worn} />
      ) : (
        <CardFace face={face} layout={form === "backrow" ? "compact" : "full"} worn={worn} onBoard={form === "backrow"} />
      ),
  };
  return (
    <div className="collection-look" data-testid={T.look} data-def-id={def.id} data-radiant={String(radiant)}>
      {LOOK_VIEWS.map((view) => (
        <figure
          key={view}
          className={`collection-look__view collection-look__view--${view}`}
          data-testid={T.lookView(view)}
          data-form={view === "board" ? form : undefined}
        >
          <span className="collection-look__face">{views[view]}</span>
          <figcaption className="collection-look__caption">{t(CAPTIONS[view])}</figcaption>
        </figure>
      ))}
    </div>
  );
}

/** R3016001: what opens a card's equip and buy screen (LooksScreen.tsx), on its hover panel and its fullscreen preview. */
export function LooksButton({ onLooks }: { onLooks: () => void }): ReactElement {
  return (
    <button type="button" className="collection-look__looks" data-testid={T.looksOpen} onClick={onLooks}>
      {t("Equip or buy")}
    </button>
  );
}

/** R3016000: the preview's Radiant toggle, a pressed button while the views show the Radiant face. */
export function RadiantToggle({ radiant, onToggle }: { radiant: boolean; onToggle: () => void }): ReactElement {
  return (
    <button type="button" className="collection-look__radiant" data-testid={T.lookRadiant} aria-pressed={radiant} onClick={onToggle}>
      {t("Show Radiant")}
    </button>
  );
}
