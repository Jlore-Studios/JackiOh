// R3399000: Classic+ Death Boil (`classicplus-012-2`, entrance none, bespoke "batterBubble"), its signature (#3399):
// a Spell printed Legendary, so every burst starts by SPELL_BURST_LATEST_MS of its entry (1,200 ms ceiling).
// Cast (its `cardPlayed`): a cauldron of batter bubbles up on the caster's side, boiling green, and spits
// one fat bubble into the air. Text: at its `damage` the bubble arcs from the caster's side to the enemy and
// bursts in scalding green splashes; at its `healed` it arcs to the ally and pops into warm steam. Moved:
// the arc is flown inside the Spell's burst window, so the bubble's flight is short. R202: it draws only
// the caster's hero and zones and the event's public target, never a card's id.

import { sideOf, testid } from "../../../game/contract.ts";
import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { frac, ringCue, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flies, hitAnchor, motifBurst, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { FxCue } from "../../types.ts";
import { heroOf, projectile, type CardRecipe } from "../recipe.ts";

/** Every particle count the boil throws at scale 1 (rule 9). */
const TUNING = {
  boil: { count: 14, power: 0.7 },
  spit: { count: 8, power: 1.1 },
  splash: { count: 22, power: 1.1 },
  steam: { count: 14, power: 0.5 },
} as const;

/** A Spell's bursts start no later than this into its entry, so they end inside its 1,200 ms ceiling (#3399). */
const SPELL_BURST_LATEST_MS = 300;
/** Where in its entry each beat starts: the boil, the spit, the bubble's flight and its landing. */
const AT = { boil: 0, spit: 0.12, fly: 0.05, land: 0.4 } as const;
/** The cauldron's lane in the caster's backrow. */
const CAULDRON_LANE = 3;
/** The healing the gallery's Death Boil does is a damage: the fixture's amount. */
const FIXTURE_DAMAGE = 6;

function at(fraction: number, D: number): number {
  return Math.min(frac(fraction, D), SPELL_BURST_LATEST_MS);
}

/** Death Boil: the cauldron boils on its cast, its bubble lands as scalding splashes or warm steam. */
export const batterBubble: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || !(p.D > 0)) return null;
  const spec: CueSpec = { look: FLAVOR_LOOKS.Toxic, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const pot = tid(testid.zone(sideOf(p.view, play.player), "backrow", CAULDRON_LANE));
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    return {
      cues: [
        ringCue(p.D, "poison", pot, at(AT.boil, p.D)),
        motifBurst(spec, pot, "area", at(AT.boil, p.D), TUNING.boil),
        motifBurst(spec, pot, "point", at(AT.spit, p.D), TUNING.spit, "poison"),
      ],
      row: "keep",
    };
  }
  if ((event.type === "damage" && !event.combat) || event.type === "healed") {
    const to = hitAnchor(p.view, event.targetId);
    if (to === null) return null;
    const land = at(AT.land, p.D);
    const fly = at(AT.fly, p.D);
    const cues: FxCue[] = [];
    const from = heroOf(p.view, play);
    if (flies(from, to)) cues.push(projectile(spec.scale, "poison", from, to, fly, Math.max(0, land - fly)));
    if (event.type === "damage") {
      cues.push(motifBurst(spec, to, "area", land, TUNING.splash, "poison"), ringCue(p.D, "poison", to, land));
    } else {
      cues.push(tunedBurst(spec.scale, "smoke", to, "area", land, TUNING.steam), ringCue(p.D, "holy", to, land));
    }
    return { cues, row: "keep" };
  }
  return null;
};

/** The gallery's Death Boil: the bubble scalds the other player's first Unit. */
export function batterBubbleFixture(view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const enemy = player === view.viewer ? view.opponent : view.you;
  const unit = enemy.units.find((candidate) => candidate !== null);
  return unit === undefined || unit === null ? [] : [{ type: "damage", sourceId: instanceId, targetId: unit.instanceId, amount: FIXTURE_DAMAGE, combat: false }];
}
