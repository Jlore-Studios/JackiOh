// What a card view carries of the card's instance data (docs/classic-sets.md B2.7, B3.3, B3.4, B5 E38,
// E39; R243's "the view carries what a card is made of"): its type where a face gives it one of its
// own, its Brittle count, its declared numbers as they stand, what Degrade, Upgrade and KY's Constant
// changed, its enchantments, and — on a hand card — the keywords it has gained there.
//
// `viewFor` builds every card view it shows through `cardView`, and only for a card the viewer may
// read there (the viewer's own hand, a unit, a face-up backrow card or the controller's face-down
// one, a graveyard, an exile pile, the resolving zone), so none of this reaches a player who may not
// read the card: the opponent's hand and every library travel as counts, a face-down trap as its
// zone and cost (R351), and the owner's library list reads what the owner was shown going in, never
// the card (R311), so a change made inside a deck shows once the card leaves it. Every key is absent
// when it has nothing to say, so a card with no instance data views exactly as before patch v0.2.0.

import type { CardView, Keyword } from "@jackioh/shared";
import { keywordKey } from "@jackioh/shared";
import { activeBrittleCount } from "./brittleCount";
import { defOf } from "./catalog";
import { enchantmentsOf } from "./enchantments";
import { cardTypeOf, runningFace } from "./faces";
import { cardKeywords } from "./layers";
import { paramsView } from "./params";
import type { CardInstance, GameState } from "./state";
import { copyTuning, isTuned } from "./tuning";

type InstanceData = Pick<CardView, "type" | "brittle" | "params" | "tuning" | "enchantments">;

/** The instance-data keys of a card view (this file's header), each only when it has something to say. */
export function instanceDataView(state: GameState, card: CardInstance): InstanceData {
  const out: InstanceData = {};
  // B2.7: the type now, where the running face's differs from the definition's (Blood Moon's Radiant).
  const type = cardTypeOf(state, card);
  if (type !== defOf(state, card.defId).type) out.type = type;
  // B3.3 rule 6: public on the field, the owner's in a hand — both are where this view is built.
  const brittle = activeBrittleCount(card);
  if (brittle !== null) out.brittle = brittle;
  // B3.4 rule 5, R386: the declared numbers the client fills the face's `{key}`s with.
  const params = paramsView(state, card);
  if (params !== null) out.params = params;
  // B3.4 rule 7: what changed, for the client's Degrade and Upgrade marks.
  const tuning = isTuned(card) ? copyTuning(card.tuning) : undefined;
  if (tuning !== undefined) out.tuning = tuning;
  // E39.
  const enchantments = enchantmentsOf(card);
  if (enchantments.length > 0) out.enchantments = enchantments.map((entry) => ({ ...entry }));
  return out;
}

function sameKeywords(a: readonly Keyword[], b: readonly Keyword[]): boolean {
  const key = (list: readonly Keyword[]): string => list.map(keywordKey).sort().join("|");
  return key(a) === key(b);
}

/**
 * B5 E38, R243: a hand card's keywords as it will carry them onto the field — its printed ones as
 * tuning leaves them and the ones it was granted in the hand or the deck (`layers.cardKeywords`) —
 * set only where they differ from its running face's printed keywords, which the client reads off
 * the definition. Null otherwise.
 */
export function handKeywordsView(state: GameState, card: CardInstance): Keyword[] | null {
  const now = cardKeywords(state, card);
  return sameKeywords(now, runningFace(state, card).keywords) ? null : now.map((keyword) => ({ ...keyword }));
}
