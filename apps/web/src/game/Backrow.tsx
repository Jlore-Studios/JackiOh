// One backrow occupant (BUILD M5-T1): a public card, a face-down card, or nothing.
//
// `BackrowView`'s `faceDown` narrowing is the privacy story: a `{ faceDown: true }` entry carries no
// `instanceId`, `defId` or type (SPEC §10.8, R33), so the back cannot render a name or a
// `card-<instanceId>` testid, and reports only a `zone` click. The view gives it a cost, shown as a
// gem (R370). The viewer's own face-down trap is a public card the view marks `unrevealed` (R371).
// B5 E21: a pile's `buried` count (never an identity) shows as depth beside the card.
// R471: a face-down card's Plague Counters (`plague`) ride its back.
// R1297000, R1834004: a face-down back wears the card back of the seat whose zone it lies in (B16).

import type { ReactElement } from "react";

import type { BackrowView, PlayerId } from "@jackioh/shared";

import { PileDepth, faceModel } from "../cards/index.ts";
import { marksOf } from "../cards/marks.ts";
import Card, { type Pops } from "./Card.tsx";
import { useCardInfo } from "./catalog.ts";
import { testid, type AnimatingMap, type ClickTarget, type Highlight, type Side } from "./contract.ts";

export type BackrowProps = {
  entry: BackrowView;
  side: Side;
  /** R1834004: the seat whose zone this is, whose card back a face-down card wears. */
  seat?: PlayerId;
  lane: number;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
  pops?: ReadonlyMap<string, Pops>;
};

export default function Backrow(props: BackrowProps): ReactElement | null {
  const { entry } = props;
  // The pile's top card with its printed face for the wheel (issue #124): a backrow card carries
  // no live numbers, so the catalog face is what the board draws. Hooks run on every render.
  const info = useCardInfo(entry === null || entry.faceDown ? "" : entry.defId, entry !== null && !entry.faceDown && entry.radiant);

  if (entry === null) return null;

  // E21: how many cards lie under this one in its pile.
  const buried = "buried" in entry && typeof entry.buried === "number" ? entry.buried : 0;
  const top =
    entry.faceDown || buried <= 0
      ? null
      : {
          // R1301: a Chinese pile top is drawn in Chinese on the wheel too.
          ...faceModel({ defId: entry.defId, def: info.def, name: info.name, radiant: entry.radiant, chinese: entry.chinese === true }),
          type: entry.type,
        };

  if (entry.faceDown) {
    // R370: read defensively, so a view without the cost draws the back.
    const cost = "cost" in entry && typeof entry.cost === "number" ? entry.cost : undefined;
    // R437, R33: a marked face-down card's back carries its mark (read through marks.ts, as defensively).
    const marks = marksOf(entry);
    // R471: Plague Counters are public; read defensively, as the cost is.
    const plague = "plague" in entry && typeof entry.plague === "number" ? entry.plague : undefined;
    return (
      <>
        <Card
          card={null}
          className="card-backrow"
          backOf={props.seat}
          faceDown={{
            at: `${props.side}-${String(props.lane)}`,
            ...(cost === undefined ? {} : { cost }),
            ...(marks.length === 0 ? {} : { marks }),
            ...(plague === undefined ? {} : { plague }),
          }}
        />
        <PileDepth buried={buried} top={null} />
      </>
    );
  }

  const testId = testid.card(entry.instanceId);
  // R1081000: a Unit standing in the backrow is drawn as the minion it is — the unit block the view
  // carries — with the same switch, target, highlight and click wiring a unit-zone minion has
  // (Zone.tsx). Anything else keeps the backrow face below.
  if (entry.type === "Unit" && entry.unit !== undefined && entry.unit !== null) {
    const unit = entry.unit;
    return (
      <>
        <Card
          testId={testId}
          card={entry}
          unit={unit}
          switchTarget
          type={entry.type}
          owner={entry.owner}
          controller={entry.controller}
          counters={entry.counters}
          unrevealed={entry.unrevealed === true}
          revealAt={entry.revealAt}
          className="card-backrow"
          target={{ on: "unit", instanceId: unit.instanceId, side: props.side, lane: props.lane }}
          highlight={props.highlight}
          animating={props.animating}
          onClick={props.onClick}
          pops={props.pops?.get(testId)}
        />
        <PileDepth buried={buried} top={top} />
      </>
    );
  }
  return (
    <>
      <Card
        testId={testId}
        card={entry}
        type={entry.type}
        owner={entry.owner}
        controller={entry.controller}
        counters={entry.counters}
        unrevealed={entry.unrevealed === true}
        revealAt={entry.revealAt}
        className="card-backrow"
        target={{ on: "backrow", instanceId: entry.instanceId, side: props.side, lane: props.lane }}
        highlight={props.highlight}
        animating={props.animating}
        onClick={props.onClick}
        pops={props.pops?.get(testId)}
      />
      <PileDepth buried={buried} top={top} />
    </>
  );
}
