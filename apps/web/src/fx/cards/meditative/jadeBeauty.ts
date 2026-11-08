// R3405001: Meditative #39.5 Jade Beauty (`meditative-039-5`, a token printed Mythic, entrance "jadeBoulder",
// bespoke "jadeBeauty", takeover "quarry", jade steam on her tile), a Mythic Unit's signature. Her takeover
// (its "quarry" art, fx/takeover.ts) holds the entry first; after that hold a jade boulder cracks along its
// veins in her zone and splits at the slam, jade shards flying as she steps out in silk robes, dust
// falling from her sleeves. At the end of her turn her Allure marks every enemy Unit (`marked`, "allure"):
// at the first she lifts a silk fan, and to each a pink-jade ribbon drifts from her, charming it in a ring
// of sparkles. The steal or the heartbreak at the start of her next turn is Allure's delayed effect, which
// opens no entry of hers, so those beats are dropped and the rows play them. It keys on her own play or
// turn hook and what it has seen (`seen`), never on a card's definition, and names no card (R202).

import type { GameEvent, PlayerId } from "@jackioh/shared";

import { ALLURE_MARK, ALLURE_MARK_COLOR, paletteFor } from "../../../cards/marks.ts";
import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CENTER, FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import type { FxCue, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look her jade wears: Hustle's jade sub-look, her aura's. */
const JADE_PALETTE = "Hustle/jade";

/** Her sleeves, where dust falls as she steps out. */
const SLEEVES: readonly FxPoint[] = [
  { x: 0.2, y: 0.55 },
  { x: 0.8, y: 0.55 },
];

/** Every particle count her signature throws, at intensity "normal" (rule 9). */
const TUNING = {
  crack: { count: 22, power: 1.1 },
  sleeve: { count: 6, power: 0.4 },
  charm: { count: 10, power: 0.6 },
} as const;

/**
 * As fractions of the motion after the lead: dust falls from her sleeves at `sleevesAt`; the boulder lasts
 * the entry and `tailMs` more. At a mark, as fractions of the entry: the ribbon leaves her at `ribbonAt`
 * and charms its Unit at `ribbonEnd`; the fan lasts until `tailMs` past the entry.
 */
const TIMING = { sleevesAt: 0.8, tailMs: 300, ribbonAt: 0.15, ribbonEnd: 0.7 } as const;

export const jadeBoulder: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const look = paletteLook(JADE_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const span = D - lead;
  const slam = lead + frac(FX_SLAM_AT, span);
  const sleeves = lead + frac(TIMING.sleevesAt, span);
  const side = view === undefined || landing === undefined ? {} : { side: sideOf(view, landing.player) };
  const cues: FxCue[] = [
    figureCue("jadeBoulder", at, figureTint(look), lead, D - lead + TIMING.tailMs, side),
    { ...tunedBurst(i, look.particle, at, "ring", slam, TUNING.crack), colors: look.colors },
    ringCue(D, look.particle, at, slam),
    ...SLEEVES.map((sleeve) => ({ ...tunedBurst(i, "dust", tid(tgt, sleeve), "point", sleeves, TUNING.sleeve), colors: look.colors })),
    ...shakeCues(i, ENTRANCE_TRAUMA.jadeBoulder, slam),
  ];
  return cues;
};

export const jadeBeauty: CardRecipe = (event, p, play) => {
  // Her Allure's marks, her end-of-turn hook's (a `trigger` entry; her play's in the gallery).
  if (event.type !== "marked" || event.mark !== ALLURE_MARK || !event.added) return null;
  if (play.hook !== undefined && play.hook !== "trigger") return null;
  const look = paletteLook(JADE_PALETTE) ?? FLAVOR_LOOKS.Hustle;
  const pink = paletteFor(ALLURE_MARK_COLOR);
  const ribbon = [pink.core, pink.rim, look.colors[1]];
  const i = p.env.intensity;
  const caster = sideOf(p.view, play.player);
  const her = tid(locateInstance(p.view, play.instanceId) ?? testid.hero(caster));
  const shown = locateInstance(p.view, event.instanceId);
  const charmed = shown === null ? tid(testid.board, FX_CENTER) : tid(shown);
  const drift = frac(TIMING.ribbonAt, p.D);
  const land = frac(TIMING.ribbonEnd, p.D);
  const cues: FxCue[] = [];
  // The first mark of the hook: she lifts her silk fan.
  if ((play.seen.marked ?? 1) === 1) cues.push(figureCue("silkFan", her, figureTint(look), 0, p.D + TIMING.tailMs, { side: caster }));
  cues.push(
    { kind: "shot", shot: { motion: "bolt", trail: "sparkle", wave: true }, colors: ribbon, preset: "sparkle", from: her, to: charmed, delayMs: drift, flightMs: land - drift, density: i },
    { ...tunedBurst(i, "sparkle", charmed, "ring", land, TUNING.charm), colors: ribbon },
  );
  return { cues, row: "replace" };
};

/** R3405001: her Allure on the other player's first Unit (her Mythic play's entry is long, so one mark). */
export const jadeBeautyFixture: RecipeFixture = (view, _instanceId, player: PlayerId): GameEvent[] => {
  const foe = (player === view.viewer ? view.opponent : view.you).units.find((unit) => unit !== null);
  return foe === undefined || foe === null ? [] : [{ type: "marked", instanceId: foe.instanceId, mark: ALLURE_MARK, color: ALLURE_MARK_COLOR, added: true }];
};
