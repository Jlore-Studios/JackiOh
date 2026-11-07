// R3402001: #93 Combo-Index (`core-093`), a Legendary Field Spell's signature (R3332000): entrance
// "comboStadium", recipe "gradeRings" and a `stadium` aura on its zone. It lands as a glass stadium of
// stacked rings rising out of its zone, a spotlight of radiant rays snapping onto it and a whirl of card
// confetti spiralling up. Its trigger runs a beat of its own (a later action than its play): on each
// `questProgressed` the next ring lights in storm blue and fires its grade, and the grade's own events
// show it, each kind at most FX_SIGNATURE_BURSTS_MAX times an entry: a card back into the hand pile
// (`addedToHand`), tags dropping in gold at the pile (`costChanged`), an enemy hand card burning at their
// pile (`exiled`), a spark (`radiantSet`) and a Lifesteal beam (`damage`). It anchors on its zone, the
// piles and readable targets alone, so a hidden card it adds, discounts or burns shows only its pile (R202).

import type { PlayerId } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_GRADE_TRAUMA } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor, timesOf } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint, FxShotCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, heroOf, type CardFxPlan, type CardRecipe, type Entrance } from "../recipe.ts";
import { withinBursts } from "./flourish.ts";

/** Every particle count the stadium throws, at intensity "normal" (rule 9). */
const TUNING = {
  confetti: { count: 28, power: 1.2 },
  grade: { count: 14, power: 0.9 },
  tag: { count: 10, power: 0.8 },
  burn: { count: 14, power: 1 },
} as const;

/** The stadium's look: storm blue. */
const STADIUM_PALETTE = "Tech/starfield";

/** Its three rings, rising from the zone's foot, each a share of the entry after the last. */
const RINGS: readonly { at: FxPoint; share: number }[] = [
  { at: { x: 0.5, y: 0.85 }, share: 0 },
  { at: { x: 0.5, y: 0.5 }, share: 0.2 },
  { at: { x: 0.5, y: 0.15 }, share: 0.4 },
];

export const comboStadium: Entrance = ({ D, tgt, intensity: i }) => {
  const t = timesOf(D);
  const look = paletteLook(STADIUM_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  return [
    ...RINGS.map((ring) => ringCue(D, "frost", tid(tgt, ring.at), frac(ring.share, D))),
    raysCue(D, "radiant", tid(tgt), 0),
    { ...tunedBurst(i, "confetti", tid(tgt), "ring", t.L, TUNING.confetti), colors: look.colors },
    ...shakeCues(i, ENTRANCE_TRAUMA.comboStadium, t.L),
  ];
};

/** Where the stadium stands: its own card, else its controller's hero. */
function stadiumOf(p: CardFxPlan, play: FxPlay): FxAnchor {
  const at = locateInstance(p.view, play.instanceId);
  return at === null ? heroOf(p.view, play) : tid(at);
}

/** The hand pile of `player`, public on both seats. */
function handOf(p: CardFxPlan, player: PlayerId): FxAnchor {
  return tid(animTestid.hand(sideOf(p.view, player)));
}

export const gradeRings: CardRecipe = (event, p, play) => {
  if (play.hook !== "trigger") return null;
  const i = p.env.intensity;
  const t = timesOf(p.D);
  const look = paletteLook(STADIUM_PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const stadium = stadiumOf(p, play);
  if (event.type === "questProgressed") {
    return {
      cues: [
        ringCue(p.D, "frost", stadium, t.G),
        { ...tunedBurst(i, "sparkle", stadium, "ring", t.T, TUNING.grade), colors: look.colors },
        ...shakeCues(i, FX_GRADE_TRAUMA, t.L),
      ],
      row: "keep",
    };
  }
  if (!withinBursts(event, p)) return null;
  const cues = ((): FxCue[] => {
    switch (event.type) {
      case "addedToHand":
        return [{ kind: "ghost", from: stadium, to: handOf(p, event.player), delayMs: t.G, durationMs: p.D }];
      case "costChanged":
        return [tunedBurst(i, "gold", handOf(p, play.player), "point", t.L, TUNING.tag)];
      case "exiled":
        return [tunedBurst(i, "fire", handOf(p, event.owner), "area", t.L, TUNING.burn)];
      case "radiantSet":
        return [tunedBurst(i, "spark", stadium, "point", t.L, TUNING.tag)];
      case "damage": {
        const to = hitAnchor(p.view, event.targetId);
        if (to === null) return [];
        const beam: FxShotCue = { kind: "shot", shot: { motion: "beam" }, colors: look.colors, preset: "holy", from: stadium, to, delayMs: t.T, flightMs: t.L - t.T, density: i };
        return [beam];
      }
      default:
        return [];
    }
  })();
  return cues.length === 0 ? null : { cues, row: "keep" };
};
