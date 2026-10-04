// Reveal (Classic #88 Siphon Squad's "Start of Turn: Reveal", Classic #65 Ace in the Hole's Radiant
// "Revealed regardless of the coin flip"; R658): a backrow Trap or Field Trap whose identity is
// public while the card stays armed.
//
// A revealed card is no longer face-down (`preview.isFaceDown`), so both players read its face —
// but it is NOT face-up: a Trap that has not fired still fires (`traps.isSpent` keys on `faceUp`
// alone, as do the replacement and text guards that read it), and a Field Trap keeps firing either
// way. R78's reset clears it with the card leaving the field. No event is emitted: the next view
// shows the card, which is the whole of the reveal.

import { startBrittleOnField } from "../brittleCount";
import type { Effect } from "../script";
import { instanceOf, type TargetSpec } from "./targets";

/** Show a backrow card's face to both players without firing it (default the running card). */
export function reveal(args: { target?: TargetSpec } = {}): Effect {
  return {
    kind: "reveal",
    apply(ctx): void {
      const card = instanceOf(ctx, args.target ?? { of: "self" });
      if (card === null || card.zone.z !== "field" || card.zone.row !== "backrow") return;
      card.revealed = true;
      // R659: a printed Brittle a face-down arrival never started begins now that it shows.
      startBrittleOnField(ctx.state, card, false);
    },
  };
}
