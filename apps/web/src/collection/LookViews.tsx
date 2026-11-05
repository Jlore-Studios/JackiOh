// R3016000 (#3016): one card's look three ways, as a game draws it: the card (its whole face), in your
// hand (the hand's face, R2723000's art-only full art included) and on the board (a Unit as a lane's
// minion, a Spell as it resolves, a Trap or a Field card as the backrow shows it set or standing).
// Each is the cards module's own face, CardFace or MinionFace, the very ones game/Card.tsx draws, never
// a copy, so the preview can't drift from the board. It wears the viewer's picks (`worn`, R1655000),
// its foil, finish and border, and moves where the look is animated, honoring Reduce motion as every
// face does. Presentation only (CLAUDE.md rule 7): it reads the catalog and the viewer's own picks and
// decides nothing.

import type { ReactElement } from "react";

import type { CardDef, UnitView } from "@jackioh/shared";

import { CardFace, MinionFace, faceModel } from "../cards/index.ts";
import { t } from "../i18n/i18n.ts";
import { collectionTestid as T } from "./collection.ts";

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

/** R3016000: `def`'s look as the card, in hand and on the board, base or Radiant; `worn` is the viewer's own copy. */
export default function LookViews({ def, radiant, worn }: { def: CardDef; radiant: boolean; worn: boolean }): ReactElement {
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
        <CardFace face={face} layout={form === "backrow" ? "compact" : "full"} worn={worn} />
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

/** R3016000: the preview's Radiant toggle, a pressed button while the views show the Radiant face. */
export function RadiantToggle({ radiant, onToggle }: { radiant: boolean; onToggle: () => void }): ReactElement {
  return (
    <button type="button" className="collection-look__radiant" data-testid={T.lookRadiant} aria-pressed={radiant} onClick={onToggle}>
      {t("Show Radiant")}
    </button>
  );
}
