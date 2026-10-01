// "…, then …" after the deaths a list's destroys caused (SPEC §4.5, R59, R113, R174): Classic #43
// Plague Nuke's "Destroy all Units. Gain 1 mana for each Plague Token that was on them. Then summon …
// each of those Units … from its owner's graveyard". A card-specific verb of the Classic #1–#45
// workstream.
//
// A destroy only marks (`destroy.ts`), and §4.5's check that moves the marked cards follows the whole
// effect (R59), so nothing later in the same list sees them dead: a Unit a sweep destroyed is still on
// the field for the rest of the list. `afterStateCheck` is the list's own "then": at its point it runs
// the check — one check, which collects everything marked so far and resolves their Deaths and
// Reborns (§4.5 steps 1–5) — and only then builds the rest of the text and applies it.
//
// THE REST IS A NEW STAY (R174). A card the check moved off the field left it after the run began,
// so the run's own mark (`ctx.exitsFrom`) calls it gone wherever it is now, and a verb aimed at it by
// id finds nothing. The rest is the text AFTER the deaths, aimed at the cards where the check put
// them, so it runs with a fresh mark taken once the check is over: a Unit in its graveyard is
// nameable there, and a Reborn body the check put back is a new arrival (R83).
//
// A PAUSE (R113). A Death hook in the check may ask something. The check then owes the rest of its
// pass on `state.work`, and the list this effect stands in parks what follows it behind that, as
// plain data (`prompts.runResumableList` parks a part by its index and memo): the check effect is
// the part's first entry and the rest its second, so a resume re-enters the part at the rest, which
// is then built — after the answer finished the pass — and runs exactly once. The mark it takes is
// kept as the rest's memo, so a pause inside the rest resumes on the stay it began with.

import { lazyPart } from "../resolve";
import type { Effect, EffectContext } from "../script";
import { stateCheck } from "../stateCheck";
import { exitMark } from "../stays";

/** The same effect, applied — and, for a part, built and its entries applied — with the fresh mark. */
function atMark(effect: Effect, exitsFrom: number): Effect {
  const expand = effect.expand;
  return {
    kind: effect.kind,
    apply(ctx): void {
      effect.apply({ ...ctx, exitsFrom });
    },
    ...(expand === undefined
      ? {}
      : {
          expand: (ctx: EffectContext, memo: unknown) => {
            const built = expand({ ...ctx, exitsFrom }, memo);
            return { ...built, effects: built.effects.map((inner) => atMark(inner, exitsFrom)) };
          },
        }),
  };
}

/** The outer part's memo: its shape is fixed, so it only needs to be JSON. */
const PART_MEMO = "afterStateCheck";

/** §4.5 at this point of the list: collect and resolve everything marked so far. */
const checkNow: Effect = {
  kind: "stateCheckNow",
  apply(ctx): void {
    if (ctx.state.result !== null) return;
    stateCheck(ctx);
  },
};

/**
 * §4.5, R59: run the state check here, then build the rest of the text with `build` and apply it,
 * on a stay that begins after the check (R174). `build` is called once the check is over — after the
 * answer, when a Death hook in it asked (R113) — with a context whose `exitsFrom` is that stay's.
 */
export function afterStateCheck(build: (ctx: EffectContext) => Effect[]): Effect {
  return lazyPart("afterStateCheck", () => ({
    effects: [
      checkNow,
      lazyPart("afterStateCheck:rest", (ctx, memo) => {
        const exitsFrom = typeof memo === "number" ? memo : exitMark(ctx.state);
        const effects = build({ ...ctx, exitsFrom }).map((effect) => atMark(effect, exitsFrom));
        return { effects, memo: exitsFrom };
      }),
    ],
    // A memo that survives JSON as it is (a pause records one per part it stands in, and a bare
    // `undefined` there would come back from a round trip as `null`); the part's shape needs none.
    memo: PART_MEMO,
  }));
}
