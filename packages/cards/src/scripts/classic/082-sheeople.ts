// C #82 Sheeople (SPEC §8.6 row 82). (1) Unit, Common, 1/1 → 2/2.
//   Base:    "Worth {worth|Tribute|Tributes}. Death: Draw {draw}." — worth 2, draw 2
//   Radiant: the same text — worth 3, draw 3
//   Engine:  "The Sheep Token's `tributeWorth` (§3.2, §7), 2 (Radiant 3), counted toward a Tribute X only
//            (R101); a Tribute that takes one unit (C #21 Turtinator's) counts it once. A Tribute is a
//            death (§6.2), so the draw happens when it is tributed. Tunes: draw 2 ↑; worth 2 ↑."
//
// The worth is the Sheep Token's static flag, which `playChoices.tributeValueOf` reads through the
// declared `worth` (R386), toward a play's Tribute X only; a script's Tribute counts units. The Death
// draw is read through `param` (R386).

import { param, type Script } from "@jackioh/engine";
import { draw } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classic-082");

const WORTH = def.params?.find((entry) => entry.key === "worth");
if (WORTH === undefined) throw new Error("classic-082 declares its worth (catalog params)");

const death: Script["death"] = (ctx) => [draw({ count: param(ctx, "draw") })];

export const base: Script = { staticFlags: { tributeWorth: WORTH.base }, death };

export const radiant: Script = { staticFlags: { tributeWorth: WORTH.radiant }, death };
