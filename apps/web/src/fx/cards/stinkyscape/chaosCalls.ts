// R3407000: Stinkyscape's two Calls to Chaos and the Golem the first summons, each a signature:
//
// - Call to Chaos (Stinky Edition) (`stinkyscape-150`, bespoke "stinkCloud", a Legendary Spell): a cracked
//   chaos egg bursts on the board's centre into a lime-green stink cloud (Toxic poison and smoke), buzzing
//   flies spiralling out of it (embers in the swarm's colors). Its roll (`chaosRolled`) spins the cloud like
//   a wheel: R436's panel names what it rolled, a Toxic ring as its first line lands. Then it spits the
//   outcome out: a card back from the centre to the hand pile of whoever gets each card (`addedToHand`), a
//   stink puff on each zone a card lands in (`summoned`; the Golem's own entrance draws its landing), and a
//   tether in the mark's own colors from the centre to each enemy it Freezes or Allures (`marked`).
// - Stinkyscape Golem (`stinkyscape-150-1`, entrance "golemCrash", aura `stinkvapour` Toxic on its tile, a
//   token Unit printed Legendary): it crash-lands from the board's top edge, a green exhaust trail behind its
//   thruster boots, and hits its zone in a ring, scrap stones shaking loose and a stink puff rolling out,
//   the board shaking by ENTRANCE_TRAUMA.golemCrash. Its keywords are the shared keyword beats' (R2789000).
// - Call to Chaos (Scape Edition) (`stinkyscape-151`, bespoke "tabletopTilt", a Legendary Spell): an indigo
//   sky falls over the board (the `dusk` figure) and a starfield pours over its top rim (star bursts), its
//   light rays turning, as the board tilts like a tabletop (a shake). Its roll: the panel and a ring of
//   stars. Drones drop in starlight (`summoned`); a replacement naming no Anomaly rings each zone of the side
//   it replaced (`boardReplaced`).
//
// What moved from the issue's concept: R436's panel shows each roll for FX_CHAOS_DICE_PANEL_MS, its reels
// landing within FX_CHAOS_DICE_REEL_MS, as Training Grounds' Call to Chaos does (R3400001), not for the
// panel's own D + FX_BANNER_TAIL_MS, so the roll keeps the Legendary Spell's ceiling (R3332000) and the
// gallery plays it; the panel's landing sparks are left out for the same reason. The Scape Edition's
// Anomaly is its rift's set piece (anomaly.ts, R3337003) and its Starship's fuse and launch are the
// Starship's (starship.ts, R2801000): one set piece per action, so inside a rift's entry and on
// `starshipFused` and `starshipLaunched` this draws nothing. Its side sliding off and the Anomaly sliding on
// is the rift's pull and landing. The cloud's flies are embers, not a figure (a new one needs fx.css).
//
// A Call to Chaos's outcome always follows its roll's entry, so no cue of it can end within the Spell's
// ceiling from its cast: each outcome beat ends within it from its own entry (its marker), and the gallery
// plays the roll.
//
// R202: the roll is public on both seats (R97 redacts only the card), and a summon's zone, a mark and the
// seat a card goes to are public; nothing reads an `addedToHand`'s `defId` or `instanceId` ("hidden" on
// the other seat): a hand is its pile and a card its back. R200: every cue starts inside its entry and
// ends within FX_MAX_TAIL_MS of it.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { LANES, sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, shakeCues, tid } from "../../build.ts";
import { chaosCues } from "../../chaos.ts";
import { FX_CENTER, FX_CHAOS_DICE_PANEL_MS, FX_CHAOS_DICE_REEL_MS, FX_MAX_TAIL_MS } from "../../constants.ts";
import { figureCue, figureTint } from "../../figures.ts";
import type { FxPalette } from "../../flavors.ts";
import { motifBurst, motifRing, motifShot, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardFxPlan, type CardRecipe, type Entrance } from "../recipe.ts";
import { markLook } from "./markLook.ts";
import { backTo, entranceSpec, entranceTimes, lookOf, pileOf, sigSpec } from "./signature.ts";

/** The Golem the Stinky Edition summons: its landing is its own entrance's. */
export const STINKYSCAPE_GOLEM_ID = "stinkyscape-150-1";

/** Every particle count the three throw at scale 1, and the Scape Edition's tilt (rule 9). */
const TUNING = {
  cloud: { count: 28, power: 0.8 },
  stink: { count: 20, power: 0.6 },
  flies: { count: 16, power: 1.1 },
  puff: { count: 12, power: 0.6 },
  exhaust: { count: 14, power: 1 },
  stones: { count: 16, power: 1.3 },
  crashPuff: { count: 22, power: 0.8 },
  pour: { count: 18, power: 0.7 },
  drop: { count: 12, power: 0.8 },
  /** The board tilting under the Scape Edition, inside SIGNATURE_SPELL_MAX_TRAUMA. */
  tiltTrauma: 0.5,
} as const;

/** Each beat's moments as fractions of its entry's D (an entrance's of its motion after its hold), and the tails past it. */
const TIMING = {
  stinkCloud: { cloudAt: 0.15, fliesAt: 0.45, sendAt: 0, puffAt: 0.1, tetherAt: 0.05, tetherFor: 0.45 },
  golemCrash: { trailAt: 0, puffAt: 0.08 },
  tabletopTilt: { skyTailMs: 400, pourStep: 0.15, raysAt: 0.2, tiltAt: 0.35, dropAt: 0.1, zoneStep: 0.06 },
} as const;

/** The palettes: the stink's lime green, the flies' swarm, the starfield and its electric cyan. */
const TOXIC = "Toxic";
const FLIES = "Beast/swarm";
const STARFIELD = "Tech/starfield";
const CYAN = "Tech";

/** The board's centre, where each cloud and each tilt plays and the outcome leaves from. */
const CENTRE: FxAnchor = tid(testid.board, FX_CENTER);

/** The board's top rim, where the stars pour over and the Golem drops from. */
const TOP_RIM: readonly FxPoint[] = [
  { x: 0.25, y: 0 },
  { x: 0.5, y: 0 },
  { x: 0.75, y: 0 },
];
const DROP_FROM: FxPoint = { x: 0.5, y: 0 };

/** Whether `event` is the play's own cast. */
function ownCast(event: GameEvent, play: FxPlay): boolean {
  return event.type === "cardPlayed" && event.instanceId === play.instanceId;
}

/**
 * R436, R3400001: the play's own roll as R436's panel, its reels landing inside the entry and within
 * FX_CHAOS_DICE_REEL_MS, the panel gone after FX_CHAOS_DICE_PANEL_MS, and a ring in `spec` as its first
 * line lands; null for any other event.
 */
function rollCues(event: GameEvent, p: CardFxPlan, play: FxPlay, spec: CueSpec): FxCue[] | null {
  if (event.type !== "chaosRolled" || event.instanceId !== play.instanceId) return null;
  const panel = chaosCues(event, Math.min(p.D, FX_CHAOS_DICE_REEL_MS), p.env.intensity).find((cue) => cue.kind === "chaos");
  if (panel === undefined) return [];
  const landMs = Math.min(p.D, panel.lines[0]?.landMs ?? 0);
  return [{ ...panel, durationMs: Math.min(FX_CHAOS_DICE_PANEL_MS, p.D + FX_MAX_TAIL_MS) }, motifRing(spec, p.D, CENTRE, landMs)];
}

/* Call to Chaos (Stinky Edition) */

export const stinkCloud: CardRecipe = (event, p, play) => {
  const T = TIMING.stinkCloud;
  const toxic = sigSpec(p, play, TOXIC);
  if (ownCast(event, play)) {
    const cloudAt = frac(T.cloudAt, p.D);
    return {
      cues: [
        motifBurst(toxic, CENTRE, "area", cloudAt, TUNING.cloud),
        motifBurst(toxic, CENTRE, "area", cloudAt, TUNING.stink, "smoke"),
        motifRing(toxic, p.D, CENTRE, cloudAt),
        motifBurst(sigSpec(p, play, FLIES), CENTRE, "ring", frac(T.fliesAt, p.D), TUNING.flies, "ember"),
      ],
      row: "replace",
    };
  }
  const roll = rollCues(event, p, play, toxic);
  if (roll !== null) return { cues: roll, row: "replace" };
  if (event.type === "addedToHand") {
    // R202: the seat it goes to is public; the card is not read.
    const delayMs = frac(T.sendAt, p.D);
    return { cues: [backTo(CENTRE, pileOf(p.view, event.player, "hand"), delayMs, p.D - delayMs)], row: "replace" };
  }
  if (event.type === "summoned") {
    // The Golem's landing is its own entrance (golemCrash), puff and all.
    if (event.defId === STINKYSCAPE_GOLEM_ID) return null;
    const zone = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane), FOOT);
    return { cues: [motifBurst(toxic, zone, "area", frac(T.puffAt, p.D), TUNING.puff, "smoke")], row: "keep" };
  }
  if (event.type === "marked" && event.added) {
    const at = locateInstance(p.view, event.instanceId);
    if (at === null) return null;
    const spec: CueSpec = { ...toxic, look: markLook(event.color, toxic.look) };
    const leave = frac(T.tetherAt, p.D);
    return { cues: [motifShot(spec, "bolt", CENTRE, tid(at), leave, frac(T.tetherFor, p.D))], row: "keep" };
  }
  return null;
};

/* Stinkyscape Golem */

export const golemCrash: Entrance = (p) => {
  const T = TIMING.golemCrash;
  const toxic = entranceSpec(p, TOXIC);
  const { at, slam } = entranceTimes(p);
  const zone = tid(p.tgt);
  const foot = tid(p.tgt, FOOT);
  const trailAt = at(T.trailAt);
  // Its thruster boots' green exhaust, from the board's top edge down onto its zone as it lands.
  const boots: CueSpec = { ...toxic, look: { ...toxic.look, particle: "exhaust" } };
  return [
    motifShot(boots, "bolt", tid(testid.board, DROP_FROM), zone, trailAt, slam - trailAt),
    motifRing(toxic, p.D, zone, slam),
    motifBurst(boots, foot, "point", slam, TUNING.exhaust),
    motifBurst(lookSpec(toxic, "Hustle/ore"), foot, "ring", slam, TUNING.stones),
    motifBurst(toxic, foot, "area", Math.min(p.D, slam + frac(T.puffAt, p.D)), TUNING.crashPuff, "smoke"),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.golemCrash, slam),
  ];
};

/** `spec` in another palette's look (the Golem's scrap stones in ore). */
function lookSpec(spec: CueSpec, palette: FxPalette): CueSpec {
  return { ...spec, look: lookOf(palette) };
}

/* Call to Chaos (Scape Edition) */

export const tabletopTilt: CardRecipe = (event, p, play) => {
  // R3337003: an Anomaly's rift is its set piece, its side's cards going and its board's coming: one per action.
  if (p.entry.rift !== undefined) return null;
  const T = TIMING.tabletopTilt;
  const stars = sigSpec(p, play, STARFIELD);
  if (ownCast(event, play)) {
    const tiltAt = frac(T.tiltAt, p.D);
    return {
      cues: [
        figureCue("dusk", tid(testid.board), figureTint(lookOf(STARFIELD)), 0, p.D + T.skyTailMs),
        ...TOP_RIM.map((rim, k) => motifBurst(stars, tid(testid.board, rim), "area", frac(k * T.pourStep, p.D), TUNING.pour)),
        raysCue(p.D, "radiant", CENTRE, frac(T.raysAt, p.D)),
        motifBurst(sigSpec(p, play, CYAN), CENTRE, "ring", tiltAt, TUNING.drop),
        ...shakeCues(p.env.intensity, TUNING.tiltTrauma, tiltAt),
      ],
      row: "replace",
    };
  }
  const roll = rollCues(event, p, play, stars);
  if (roll !== null) return { cues: roll, row: "replace" };
  if (event.type === "summoned") {
    const zone = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
    return { cues: [motifBurst(stars, zone, "area", frac(T.dropAt, p.D), TUNING.drop)], row: "keep" };
  }
  if (event.type === "boardReplaced") {
    // A replacement naming no Anomaly (no rift): a ring over each zone of the side it replaced.
    const side = sideOf(p.view, event.player);
    return { cues: LANES.map((lane, k) => motifRing(stars, p.D, tid(testid.zone(side, "units", lane)), frac(k * T.zoneStep, p.D))), row: "keep" };
  }
  // The Starship's fuse and launch are its own set piece (starship.ts).
  return null;
};

/* The gallery's fixtures */

/** R3407000: the edition's roll of one effect, named as the engine names it (its printed clause, R436). */
function rolled(instanceId: string, player: PlayerId, defId: string, effect: string): GameEvent {
  return { type: "chaosRolled", player, instanceId, defId, effects: [effect] };
}

/** The Stinky Edition's ninth effect and the Scape Edition's second, the clauses `chaosRolled` names. */
export const STINKY_GALLERY_EFFECT = "Summon a Stinkyscape Golem";
export const SCAPE_GALLERY_EFFECT = "Summon 2 Drone Corvettes";

/**
 * R3407000: the Stinky Edition's roll. Its outcome follows the roll's entry, past the Spell's ceiling from
 * the cast, so each outcome beat is the module test's.
 */
export function stinkCloudFixture(_view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-150"): GameEvent[] {
  return [rolled(instanceId, player, defId, STINKY_GALLERY_EFFECT)];
}

/** R3407000: the Scape Edition's roll; as the Stinky Edition's, its outcome beats are the module test's. */
export function tabletopTiltFixture(_view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-151"): GameEvent[] {
  return [rolled(instanceId, player, defId, SCAPE_GALLERY_EFFECT)];
}
