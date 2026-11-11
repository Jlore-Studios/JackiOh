// R670: Spell Tyrant (`classic-056`, entrance "tyrantSigil"): an arcane sigil that flares as it lands.
// R3389001: grown into its signature (R3389000): entrance "tyrantSigil", bespoke "tyrantCommand" and its
// aura "crownSigil" on its own tile (R3400000). A jagged violet crown-sigil slams onto its zone, its points cracking
// the stone, and the Tyrant rises through it, its robe of scrolls flaring. Its Cry casts Spells from its
// graveyard: each rises from the graveyard pile as a ghost, a card back, to bow at its crown (the cast's
// own `cardPlayed`, inside its play: R3389000's claim of the casts a signature's play makes), casts on
// its command in its own entry, then burns to ash into the exile pile (that cast's own `exiled`, a card
// back in place of the exile's own row), one after another. Standing, a small violet crown-sigil turns
// above it.

import { animTestid } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_CRACK_TAIL_MS, FX_SLAM_AT } from "../../constants.ts";
import type { FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, timesOf } from "../../motifs/build.ts";
import type { FxPoint } from "../../types.ts";
import { cardAnchor, ENTRANCE_TRAUMA, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";
import { flourishSpec, type FlourishFixture } from "./flourish.ts";

/** The look its casts rise and burn in. */
const PALETTE: FxPalette = "Nerd";

/** Every particle count the sigil throws, the entrance's at intensity "normal", the recipe's at scale 1 (rule 9). */
const TUNING = {
  sigilSparkle: { count: 22, power: 0.9 },
  sigilArcane: { count: 30, power: 1.4 },
  scrolls: { count: 12, power: 0.8 },
  bow: { count: 8, power: 0.6 },
} as const;

/** The sigil's crown, above the zone, where its casts bow. */
const CROWN: FxPoint = { x: 0.5, y: 0.1 };

export const tyrantSigil: Entrance = ({ D, tgt, intensity: i, landMs }) => {
  const at = tid(tgt);
  const slam = landMs ?? frac(FX_SLAM_AT, D);
  return [
    ringCue(D, "arcane", at, 0),
    tunedBurst(i, "sparkle", tid(tgt, CROWN), "ring", 0, TUNING.sigilSparkle),
    raysCue(D, "mythic", at, slam),
    // The sigil's points stab the stone; the Tyrant rises through it, its robe of scrolls flaring.
    { kind: "crack", at, delayMs: slam, durationMs: D - slam + FX_CRACK_TAIL_MS },
    tunedBurst(i, "arcane", at, "area", slam, TUNING.sigilArcane),
    tunedBurst(i, "spark", tid(tgt, CROWN), "point", slam, TUNING.scrolls),
    ...shakeCues(i, ENTRANCE_TRAUMA.tyrantSigil, slam),
  ];
};

/** #56 Spell Tyrant raises each Spell its Cry casts to its crown, and burns it into exile after. */
export const tyrantCommand: CardRecipe = (event, p, play) => {
  // A cast its play makes is the play resolving now, inside the Tyrant's.
  const cast = p.env.memory.resolving();
  if (cast === undefined || cast.instanceId === play.instanceId || cast.player !== play.player) return null;
  const spec = flourishSpec(PALETTE, play, p.view, p.env);
  if (!(p.D > 0) || !(spec.scale > 0)) return null;
  const t = timesOf(p.D);
  const side = sideOf(p.view, play.player);
  const crown = cardAnchor(p, play.instanceId) ?? heroOf(p.view, play);
  if (event.type === "cardPlayed" && event.instanceId === cast.instanceId && event.player === play.player) {
    return {
      cues: [
        { kind: "ghost", from: tid(animTestid.graveyard(side)), to: crown, delayMs: t.G, durationMs: t.L - t.G },
        motifBurst(spec, crown, "point", t.T, TUNING.bow, "sparkle"),
        motifRing(spec, p.D, crown, t.L),
      ],
      row: "keep",
    };
  }
  if (event.type === "exiled" && event.instanceId === cast.instanceId) {
    return { cues: [{ kind: "ghost", from: crown, to: tid(animTestid.exile(side)), delayMs: t.G, durationMs: p.D - t.G }], row: "replace" };
  }
  return null;
};

/** The gallery's Spell Tyrant: its Cry casts a Glitch from its graveyard, which it exiles after. */
export const tyrantCommandFixture: FlourishFixture = (_view, instanceId, player) => {
  const cast = `${instanceId}-cast`;
  return [
    { type: "cardPlayed", player, instanceId: cast, defId: "classic-t-glitch", costPaid: 0 },
    { type: "exiled", instanceId: cast, defId: "classic-t-glitch", owner: player },
    { type: "cardResolved", player, instanceId: cast, defId: "classic-t-glitch", permanent: false, costPaid: 0 },
  ];
};
