// #95 Call to Chaos (Core Edition) (SPEC §8.4, R28, R58, R60, R70, R87, R423, R436, BUILD M4-T4 row 95).
//
// Base: "One random effect" out of ten. Radiant (patch v0.2.0, R423): "Three different random
// effects, resolved in the order listed" — the shape Classic+ #73 has, so both editions follow one
// rule. The recursion is one of the ten like any other: rolled only when it falls among the three,
// and resolved where the list puts it.
//
// The whole card is `engine/src/subsystems/callToChaos.ts`, whose header names this file's shape:
// "#95's own file is a one-line hook that returns `[callToChaos()]`". The subsystem owns the ten
// effects, the roll, the announcement and the chain counter, for reasons that are all engine concerns:
//   * each effect must read the board when it RESOLVES, not when the hook builds it, because a
//     rolled recursion resolves its whole chain before the effects after it (R87), and a nested cast
//     draws cards, summons units and changes costs in between. Every effect there is a lazy wrapper.
//   * the chain length is game state, on the cast instance's `memory` (§10.1,
//     `CHAOS_CHAIN_KEY`), so a paused and serialized game resumes with the same cap left and two
//     independent Calls in one turn never share a counter.
//   * R28's cap of 20 (`CALL_TO_CHAOS_CHAIN_CAP`) is a HARD stop: a recursion rolled once the chain
//     is at the cap resolves into nothing and no substitute effect is rolled (R87), so a Radiant
//     Call at the cap runs only its other two.
//   * R436: before anything resolves, `chaosRolled` names the rolled clauses to both players.
// Rebuilding any of that here would be a second source of truth for the same rules.
//
// The face is passed explicitly, so each face states which text of §8 it is, rather than leaning on
// the flag `makeContext` puts in the context (the same flag `scriptOf` used to pick this face).
//
// Two rulings worth naming here because they are invisible in the one-liner: R70 makes the
// recursion a Cast — free, counted as a play, running the card's own script, with the caster
// picking targets — and R87 sends a card cast from no zone to the caster's graveyard when it
// resolves (§10.5 step 7), which is what feeds Gravedigger and Reminisce down a long chain.

import type { Script } from "@jackioh/engine";
import { subsystems } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-095");

/** §8: "One random effect" out of the ten. */
export const base: Script = { cry: () => [subsystems.callToChaos({ radiant: false })] };

/** §8, R423: three different random effects of the ten, resolved in the order the list writes them. */
export const radiant: Script = { cry: () => [subsystems.callToChaos({ radiant: true })] };
