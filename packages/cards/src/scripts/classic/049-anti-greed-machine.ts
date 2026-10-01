// C #49 Anti-Greed Machine (SPEC §8.6 row 49, BUILD M9 Classic row C 49). (3) Unit 9/9 → 18/18, Common.
//   Base:    "Rush / Aura: Players can't draw more than 1 card each turn."
//   Radiant: "Rush / Aura: Your opponent can't draw more than {limit|card|cards} each turn." (1)
//   Engine:  "A draw limit (§2.4) of 1 on both players (Radiant: on the opponent only) while it is on
//            the field, on every turn, the start-of-turn draw included, read against the per-turn draw
//            count (§10.1), so draws made earlier in the turn count: a draw beyond it does not happen at
//            all (no card moves, no fatigue, no cast on draw), and with several limits the lowest holds.
//            Tunes: Radiant limit 1 ↓ (never below 1)."
//
// The aura is B5 E3's draw limit (`Script.drawLimit`): the engine asks every card acting on the
// field for its limits before each draw (`draw.drawLimitOf`), takes the lowest on the drawing player,
// and stops a draw past it (`drawLimited`). So the limit holds only while the Machine acts on the
// field — it is lifted the moment it leaves — and Rush is printed on both faces (§10.4 layer 1).
// The base face's 1 is printed, not declared: SPEC tunes only the Radiant face's limit, which reads
// `param(…, "limit")` (R386, never below 1 by the declaration's `min`).

import type { DrawLimitHook, Script } from "@jackioh/engine";
import { param } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-049");

/** "Players can't draw more than 1 card each turn": the base face's printed limit, on both players. */
const PLAYERS_LIMIT = 1;

const baseLimit: DrawLimitHook = () => [{ player: "both", count: PLAYERS_LIMIT }];

/** "Your opponent can't draw more than {limit} …": the declared, tunable number. */
const radiantLimit: DrawLimitHook = (args) => [{ player: "enemy", count: param(args, "limit") }];

export const base: Script = { drawLimit: baseLimit };

export const radiant: Script = { drawLimit: radiantLimit };
