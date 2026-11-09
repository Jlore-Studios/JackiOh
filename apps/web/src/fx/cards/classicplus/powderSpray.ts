// R3399000: Classic+ Powder Spray (`classicplus-012-4`, entrance none, bespoke "flourCloud"), its signature (#3399):
// a Spell printed Legendary, so every burst starts by SPELL_BURST_LATEST_MS of its entry (1,200 ms ceiling).
// Cast (its `cardPlayed`): a giant flour sack thumps down at the caster's edge (a puff and a small shake at
// its hero) and a bellows huffs a white cloud across the board (a smoke shot to the far side). Text, at each
// `damage`: the cloud rolls over the enemy or enemy hero, a cough of flour where its number drops, and a
// second, slower drift of dust settling there a beat later. R202: it draws only heroes and the event's own
// public target.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { sideOf, testid } from "../../../game/contract.ts";
import { frac, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { flies, hitAnchor, motifBurst, motifScale, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FlavorLook } from "../../flavors.ts";
import type { FxCue } from "../../types.ts";
import { heroOf, type CardRecipe } from "../recipe.ts";

/** Every particle count the cloud throws at scale 1 (rule 9). */
const TUNING = {
  thump: { count: 12, power: 0.8 },
  cough: { count: 8, power: 0.8 },
  drift: { count: 5, power: 0.3 },
} as const;

/** Flour: white and floury, in the Snack look's own particle. */
const FLOUR: FlavorLook = { ...FLAVOR_LOOKS.Snack, colors: ["#ffffff", "#f3efe6", "#d8cfc0"], particle: "smoke" };

/** A Spell's bursts start no later than this into its entry, so they end inside its 1,200 ms ceiling (#3399). */
const SPELL_BURST_LATEST_MS = 300;
/** Where in its entry each beat starts: the thump, the cloud's flight, the cough and the drift. */
const AT = { thump: 0, huff: 0.05, cloudFly: 0.3, cough: 0.1, drift: 0.3 } as const;
/** The sack's thump, a Spell's at most 1 (#3399). */
const THUMP_TRAUMA = 0.4;
/** The damage the gallery's Powder Spray deals each enemy. */
const FIXTURE_DAMAGE = 3;

function at(fraction: number, D: number): number {
  return Math.min(frac(fraction, D), SPELL_BURST_LATEST_MS);
}

/** Powder Spray: the sack thumps and the cloud huffs on its cast; each hit coughs flour. */
export const flourCloud: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || !(p.D > 0)) return null;
  const spec: CueSpec = { look: FLOUR, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const home = heroOf(p.view, play);
    const far = tid(testid.hero(sideOf(p.view, play.player === "p1" ? "p2" : "p1")));
    const cues: FxCue[] = [motifBurst(spec, home, "area", at(AT.thump, p.D), TUNING.thump), ...shakeCues(spec.scale, THUMP_TRAUMA, at(AT.thump, p.D))];
    if (flies(home, far)) cues.push(motifShot(spec, "bolt", home, far, at(AT.huff, p.D), Math.max(0, at(AT.cloudFly, p.D) - at(AT.huff, p.D))));
    return { cues, row: "keep" };
  }
  if (event.type === "damage" && !event.combat) {
    const hit = hitAnchor(p.view, event.targetId);
    if (hit === null) return null;
    return {
      cues: [motifBurst(spec, hit, "area", at(AT.cough, p.D), TUNING.cough), tunedBurst(spec.scale, "dust", hit, "area", at(AT.drift, p.D), TUNING.drift)],
      row: "keep",
    };
  }
  return null;
};

/** The gallery's Powder Spray: every enemy Unit and the enemy hero takes 3. */
export function flourCloudFixture(view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const enemy = player === view.viewer ? view.opponent : view.you;
  const targets = [...enemy.units.flatMap((unit) => (unit === null ? [] : [unit.instanceId])), `hero-${enemy.player}`];
  return targets.map((targetId) => ({ type: "damage", sourceId: instanceId, targetId, amount: FIXTURE_DAMAGE, combat: false }));
}
