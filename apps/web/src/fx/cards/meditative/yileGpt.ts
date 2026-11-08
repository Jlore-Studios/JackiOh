// R3405001: Meditative #49 YileGPT (`meditative-049`, entrance "scrollScholar", bespoke "yileGpt", takeover
// "ai", a hologram drift on its tile), a Mythic Unit's signature in ink and light. After its takeover's hold
// the ink-and-light scholar steps out brush first, its robe trailing ink strokes, and folds into its zone at
// the slam in a ring of light; then a glitching ink drop skitters across its lane to the enemy's zone
// opposite, where the Yile's Virus its Cry summons boots up with the Virus's own entrance (yilesVirus.ts),
// the Virus's `summoned` never this recipe's. Its Cry emits nothing else. At its Death its light folds into
// a hologram card back that flies to its controller's Deck pile (`shuffledIn`), from its card while the
// board shows it, else from its Graveyard pile. It keys on its play or its Death and the shuffle's player,
// never on a card's definition, and draws a card back alone (R202).

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { otherSide } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance, type RecipeFixture } from "../recipe.ts";

/** The look of its light: Tech's hologram, its aura's. */
const LIGHT_PALETTE = "Tech/hologram";
/** The look of its ink: Nerd's ink, the strokes its robe trails and the drop that becomes the Virus. */
const INK_PALETTE = "Nerd/ink";

/** Every particle count it throws, at intensity "normal" (rule 9): the robe's strokes, its fold into the zone, its light at its Death. */
const TUNING = {
  strokes: { count: 16, power: 0.8 },
  fold: { count: 20, power: 1 },
  light: { count: 14, power: 0.7 },
} as const;

/**
 * The scholar stands from the lead to `scholarTailMs` past its entry, the drop from the slam to `dropTailMs`
 * past it; at its Death its card back leaves at `ghostAt` of the entry.
 */
const TIMING = { scholarTailMs: 350, dropTailMs: 250, ghostAt: 0.25 } as const;

/** The gallery's Prime, shuffled into the Deck at its Death. */
const FIXTURE_PRIME = { instanceId: "fx-prime", defId: "meditative-049-1", position: 0 } as const;

export const scrollScholar: Entrance = ({ D, tgt, intensity: i, view, landing, leadMs }) => {
  const light = paletteLook(LIGHT_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const ink = paletteLook(INK_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const at = tid(tgt);
  const lead = leadMs ?? 0;
  const slam = lead + frac(FX_SLAM_AT, D - lead);
  const side = view === undefined || landing === undefined ? undefined : sideOf(view, landing.player);
  const cues: FxCue[] = [
    figureCue("inkScholar", at, figureTint(light), lead, D - lead + TIMING.scholarTailMs, side === undefined ? {} : { side }),
    { ...tunedBurst(i, ink.particle, tid(tgt, FOOT), "area", lead, TUNING.strokes), colors: ink.colors },
    ringCue(D, light.particle, at, slam),
    { ...tunedBurst(i, light.particle, at, "ring", slam, TUNING.fold), colors: light.colors },
  ];
  // The ink drop skitters across its lane to the enemy's zone opposite, where the Virus boots up.
  if (side !== undefined && landing !== undefined) {
    const across = tid(testid.zone(otherSide(side), "units", landing.lane));
    cues.push(
      { kind: "shot", shot: { motion: "bolt", trail: "circuit" }, colors: ink.colors, preset: "circuit", from: at, to: across, delayMs: slam, flightMs: D - slam, density: i },
      figureCue("inkDrop", across, figureTint(ink), slam, D - slam + TIMING.dropTailMs, { side }),
    );
  }
  cues.push(...shakeCues(i, ENTRANCE_TRAUMA.scrollScholar, slam));
  return cues;
};

export const yileGpt: CardRecipe = (event, p, play) => {
  // Its Death opens an entry of its own (R2797000); the gallery's runs under its play.
  if (play.hook !== undefined && play.hook !== "death") return null;
  if (event.type !== "shuffledIn" || event.player !== play.player) return null;
  const light = paletteLook(LIGHT_PALETTE) ?? FLAVOR_LOOKS.Tech;
  const side = sideOf(p.view, play.player);
  const card = locateInstance(p.view, play.instanceId);
  const from = tid(card ?? animTestid.graveyard(side));
  const leave = frac(TIMING.ghostAt, p.D);
  return {
    cues: [
      { ...tunedBurst(p.env.intensity, light.particle, from, "area", 0, TUNING.light), colors: light.colors },
      { kind: "ghost", from, to: tid(animTestid.library(side)), delayMs: leave, durationMs: p.D - leave },
    ],
    row: "replace",
  };
};

/** The gallery's YileGPT: its Death's one beat, a YileGPT Prime shuffled into its Deck. */
export const yileGptFixture: RecipeFixture = (_view, _instanceId, player) => [{ type: "shuffledIn", player, ...FIXTURE_PRIME }];
