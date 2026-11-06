// #2932: on a phone held upright a field card's Activate control and keyword marks hang off the
// card's top-left corner instead of sitting on its face, so the face keeps its whole width for the
// name, art, cost, attack and health (board.css, "Phone portrait"; the card is not made wider).
//
// The stack is a sibling of the card root, as the switch (`SwitchButton`, Card.tsx) is, so DEF's
// rotation never turns or moves it: its corner is still the card's own top-left (board.css works it
// out from the card's size; the zone centres the card), though it starts below the target of the
// switch of the zone to its left, which sits at that very corner (board.css, `--badges-top`). It draws view data only (CLAUDE.md rule 7):
// the control is `ActivateControls`, which decides nothing, and the marks are `KeywordIcons`, the
// face's own, with the same "+n" fold. Card.tsx renders it only for a field card on the phone layout
// (`usePhonePortrait`); every other card keeps both on its face.

import type { ReactElement } from "react";

import type { CardView, UnitView } from "@jackioh/shared";

import { KeywordIcons } from "../cards/index.ts";
import { keywordChips } from "../cards/MinionFace.tsx";
import ActivateControls from "./ActivateControl.tsx";
import type { AnimatingMap, ClickTarget, Highlight } from "./contract.ts";

export type FieldBadgesProps = {
  /** The card as the view lists it: the `UnitView` of a unit, else the face-up `BackrowView`; its `activations` are the control's. */
  card: CardView;
  unit: UnitView | null | undefined;
  /** The unit's keywords in the face's words, in order (`FaceModel.keywordWords`; R1301: a Chinese unit's are the table's). */
  words: readonly string[];
  /** What joins a list of them (`FaceModel.keywordJoin`). */
  join: string;
  highlight?: Highlight;
  animating?: AnimatingMap;
  onClick?: (target: ClickTarget) => void;
};

export default function FieldBadges({ card, unit, words, join, highlight, animating, onClick }: FieldBadgesProps): ReactElement | null {
  const keywords = unit?.keywords ?? [];
  // A unit whose keywords the face draws as states (Taunt's frame, a Divine Shield's bubble) or that do nothing on the field has no mark to hang.
  const { shown, folded } = keywordChips(keywords, unit?.armor ?? 0);
  if ((card.activations ?? []).length === 0 && shown.length === 0 && folded.length === 0) return null;
  return (
    <span className="field-badges">
      <ActivateControls card={card} highlight={highlight} animating={animating} onClick={onClick} />
      {unit != null && <KeywordIcons keywords={keywords} words={words} join={join} armor={unit.armor} />}
    </span>
  );
}
