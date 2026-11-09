// R3395000: what Stinkyscape 001 to 090's signatures (this folder) share: a palette's look, a figure's
// tint and side, where a card stands in the view, a card back's flight to a hand pile and the slam every
// entrance lands on, after its entry's lead (R3405000: a Mythic's takeover hold, before which it starts
// nothing). Each card's own file composes its beats from these and keys on its own play and how many
// events of a kind that play has seen; nothing here reads a card's definition, so no cue names a card
// (R202).

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid, type Side } from "../../../game/contract.ts";
import { frac, shakeCues, tid } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { figureCue, figureTint, type FxFigure } from "../../figures.ts";
import { FLAVOR_LOOKS, paletteLook, type FlavorLook, type FxPalette } from "../../flavors.ts";
import type { FxAnchor, FxCue, FxFigureCue, FxGhostCue, FxPlay } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardFxResult, type EntranceKey, type EntrancePlan } from "../recipe.ts";

/** An entrance's figure outlives its entry by this much, as Bob's plow does (rule 9). */
export const ENTRANCE_FIGURE_TAIL_MS = 300;

/** The look `palette` names, else Tech's (the set's own flavor). */
export function lookOf(palette: FxPalette): FlavorLook {
  return paletteLook(palette) ?? FLAVOR_LOOKS.Tech;
}

/** The side a landing stands on, when the planner gave the board and the landing. */
export function landingSide(plan: Pick<EntrancePlan, "view" | "landing">): Side | undefined {
  return plan.view === undefined || plan.landing === undefined ? undefined : sideOf(plan.view, plan.landing.player);
}

/** R3405000: where in its entry an entrance's motion starts: after the entry's lead (a Mythic's takeover hold), else at once. */
export function leadOf(plan: Pick<EntrancePlan, "leadMs">): number {
  return plan.leadMs ?? 0;
}

/** An entrance's figure over its zone from its lead, outliving its entry by ENTRANCE_FIGURE_TAIL_MS, mirrored for the other seat when `mirror`. */
export function entranceFigure(figure: FxFigure, palette: FxPalette, plan: EntrancePlan, mirror = false): FxFigureCue {
  const side = mirror ? landingSide(plan) : undefined;
  const lead = leadOf(plan);
  return figureCue(figure, tid(plan.tgt), figureTint(lookOf(palette)), lead, plan.D - lead + ENTRANCE_FIGURE_TAIL_MS, side === undefined ? {} : { side });
}

/** Where in its entry an entrance lands: at FX_SLAM_AT of the motion after its lead (R3405000), as every marquee entrance does. */
export function slamAt(plan: Pick<EntrancePlan, "D" | "leadMs">): number {
  const lead = leadOf(plan);
  return lead + frac(FX_SLAM_AT, plan.D - lead);
}

/** An entrance's one shake, at its slam, by its ENTRANCE_TRAUMA. */
export function slamShake(plan: EntrancePlan, key: EntranceKey): FxCue[] {
  return shakeCues(plan.intensity, ENTRANCE_TRAUMA[key], slamAt(plan));
}

/** The testid of the card `instanceId` where the view shows it face up on the field or in the viewer's hand, else null. */
export function placeOf(view: PlayerView, instanceId: string): string | null {
  return locateInstance(view, instanceId);
}

/**
 * Where the readable play's own card stands: its card where the view shows it, else the first zone of its
 * player's side showing its definition face up (a board that kept the zone's id, as /dev/fx's does), else
 * its player's hero. The play is readable, so its definition is the viewer's to read (R202).
 */
export function cardAt(view: PlayerView, play: FxPlay): FxAnchor {
  const own = placeOf(view, play.instanceId);
  if (own !== null) return tid(own);
  const side = sideOf(view, play.player);
  const unit = view[side].units.findIndex((card) => card !== null && card.defId === play.defId);
  if (unit >= 0) return tid(testid.zone(side, "units", unit + 1));
  const field = view[side].backrow.findIndex((slot) => slot !== null && !slot.faceDown && slot.defId === play.defId);
  if (field >= 0) return tid(testid.zone(side, "backrow", field + 1));
  return heroAt(view, play.player);
}

/** The zone a readable summon lands in. */
export function summonZone(view: PlayerView, event: Extract<GameEvent, { type: "summoned" }>): string {
  return testid.zone(sideOf(view, event.player), event.row, event.lane);
}

/** `player`'s hand pile, the only place a card back flies to (R202). */
export function handOf(view: PlayerView, player: PlayerId): FxAnchor {
  return tid(animTestid.hand(sideOf(view, player)));
}

/** `player`'s hero. */
export function heroAt(view: PlayerView, player: PlayerId): FxAnchor {
  return tid(testid.hero(sideOf(view, player)));
}

/** A card back flying from `from` to `to`, leaving at `delayMs` and landing as the entry ends. */
export function cardBack(from: FxAnchor, to: FxAnchor, delayMs: number, D: number): FxGhostCue {
  return { kind: "ghost", from, to, delayMs, durationMs: Math.max(0, D - delayMs) };
}

/** A beat that draws `cues` in place of its event's own row. */
export function replace(cues: FxCue[]): CardFxResult {
  return { cues, row: "replace" };
}

/** R3395001: a beat that draws `cues` in place of its event's own row and of its system moment too. */
export function replaceMoment(cues: FxCue[]): CardFxResult {
  return { cues, row: "replace", moment: "replace" };
}

/** The unit zones of `side` the view shows a card in, in lane order. */
export function shownUnitZones(view: PlayerView, side: Side): string[] {
  return view[side].units.flatMap((unit, k) => (unit === null ? [] : [testid.zone(side, "units", k + 1)]));
}
