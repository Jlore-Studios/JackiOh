// One backrow occupant (BUILD M5-T1): a public card, a face-down card, or nothing.
//
// `BackrowView` is the union that decides it, and the narrowing on `faceDown` is the whole
// privacy story: a `{ faceDown: true }` entry carries no `instanceId`, no `defId` and no type
// (SPEC §10.8, R33), so the back cannot render a name even by accident and cannot carry a
// `card-<instanceId>` testid. Such a zone is reportable only as a `zone` click, which is correct:
// there is nothing else the viewer is allowed to know about it. Since v0.1.1 the view gives it a
// cost, which the back shows as a gem, and its hover and sheet say what it is (R370).
//
// The viewer's own face-down trap is a public card to them (R33), and the view marks it
// `unrevealed` so the face can say the other player sees only a back (R371).
//
// B5 E21: a backrow zone may hold a pile, and the view counts the dormant cards under its top
// (`buried`, a count and never an identity, on a face-up card and on a face-down one alike, read
// defensively as the cost is). The zone shows the depth beside the card, as a unit pile does.

import type { ReactElement } from "react";

import type { BackrowView } from "@jackioh/shared";

import { PileDepth, faceModel } from "../cards/index.ts";
import { marksOf } from "../cards/marks.ts";
import Card, { type Pops } from "./Card.tsx";
import { useCardInfo } from "./catalog.ts";
import { testid, type AnimatingMap, type ClickTarget, type Highlight, type Side } from "./contract.ts";

export type BackrowProps = {
  entry: BackrowView;
  side: Side;
  lane: number;
  /** #165: what a touch hold on the card opens (contract.ts's `touchHoldMode`). */
  touchHold?: "sheet" | "preview";
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
      : { ...faceModel({ defId: entry.defId, def: info.def, name: info.name, radiant: entry.radiant }), type: entry.type };

  if (entry.faceDown) {
    // R370: read defensively, so a view without the cost draws the back as it always did.
    const cost = "cost" in entry && typeof entry.cost === "number" ? entry.cost : undefined;
    // R437, R33: a marked face-down card's back carries its mark (read through marks.ts, as defensively).
    const marks = marksOf(entry);
    return (
      <>
        <Card
          card={null}
          className="card-backrow"
          touchHold={props.touchHold}
          faceDown={{
            at: `${props.side}-${String(props.lane)}`,
            ...(cost === undefined ? {} : { cost }),
            ...(marks.length === 0 ? {} : { marks }),
          }}
        />
        <PileDepth buried={buried} top={null} />
      </>
    );
  }

  const testId = testid.card(entry.instanceId);
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
        className="card-backrow"
        touchHold={props.touchHold}
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
