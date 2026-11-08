// R3399000: Classic+ Wardrum (`classicplus-037`, entrance "drumLeap", bespoke "drumBeat"), its signature (#3399):
// his entrance and his end-of-turn drum.
// Entrance: three drumbeats boom from off the board (a ring and a burst each, staggered before the landing); on the
// third he leaps in, a shot from the board's edge, and lands in a fire shockwave ring under a holy burst and a sheen
// (his Divine Shield), with the Legendary's shake.
// Beat: at the end of turn each cast the drum launches (`cardAnnounced`) is one ring of sound at Wardrum's zone, a
// burst of fire off his chest, and a bolt from the drum to his hero where the copy's own events go on playing.
// R202: nothing here names a card or shows which non-Unit it copies; an unreadable zone falls back to the hero.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { frac, ringCue, shakeCues, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor, motifBurst, motifRing, motifShot, timesOf, type CueSpec } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, heroOf, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the drum throws, at intensity "normal" (rule 9). */
const TUNING = {
  boom: { count: 10, power: 0.9 },
  land: { count: 26, power: 1.2 },
  shield: { count: 14, power: 0.8 },
  chest: { count: 14, power: 1 },
} as const;

/** The three drumbeats of the entrance, as fractions of D (before the landing at FX_SLAM_AT). */
const DRUM_BEATS = [0.06, 0.2, 0.34] as const;
/** Where the beats boom from: the board's top edge. */
const OFF_BOARD: FxAnchor = { kind: "viewport", at: { x: 0.5, y: 0 } };
/** The drum's own shake at each beat, under a permanent's ceiling. */
const BEAT_TRAUMA = 0.3;

const look = paletteLook("Fire") ?? FLAVOR_LOOKS.Fire;

export const drumLeap: Entrance = ({ D, tgt, intensity: i }) => {
  const spec: CueSpec = { look, scale: i, row: "keep" };
  const at = tid(tgt);
  const slam = frac(FX_SLAM_AT, D);
  const beats = DRUM_BEATS.map((f) => frac(f, D));
  const cues: FxCue[] = beats.flatMap((delay) => [ringCue(D, "ember", OFF_BOARD, delay), motifBurst(spec, OFF_BOARD, "area", delay, TUNING.boom, "ember")]);
  const last = beats[beats.length - 1] ?? 0;
  cues.push(
    motifShot(spec, "bolt", OFF_BOARD, at, last, slam - last),
    motifRing(spec, D, at, slam),
    motifBurst(spec, at, "area", slam, TUNING.land),
    motifBurst(spec, at, "ring", slam, TUNING.shield, "holy"),
    { kind: "sheen", at, delayMs: slam, durationMs: D - slam },
    ...shakeCues(i, ENTRANCE_TRAUMA.drumLeap, slam),
  );
  return cues;
};

/** #37 Wardrum: each cast his end-of-turn drum launches is one ring of sound from his chest. */
export const drumBeat: CardRecipe = (event, p, play) => {
  if (play.hook !== "trigger" || event.type !== "cardAnnounced") return null;
  const t = timesOf(p.D);
  const spec: CueSpec = { look, scale: p.env.intensity, row: "keep" };
  const drum = hitAnchor(p.view, play.instanceId) ?? heroOf(p.view, play);
  const hero = heroOf(p.view, play);
  const cues: FxCue[] = [
    motifRing(spec, p.D, drum, t.G),
    motifBurst(spec, drum, "area", t.G, TUNING.chest),
    motifShot(spec, "bolt", drum, hero, t.T, t.L - t.T),
    ...shakeCues(p.env.intensity, BEAT_TRAUMA, t.G),
  ];
  return { cues, row: "keep" };
};

/** The gallery's end of turn: the drum casts one copy. */
export function drumBeatFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-037", controller: player, hook: "endOfTurn" },
    { type: "cardAnnounced", player, instanceId: "fx-drum-copy", defId: "core-005", cardType: "Spell", costPaid: 0, targets: [] },
  ];
}
