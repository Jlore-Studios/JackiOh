// R3399000: Classic+ KY's Gift (`classicplus-042-1`, entrance "giftDesk", bespoke "giftBoxes"), its signature (#3399):
// Entrance: a teacher's stone desk rises in the zone (dust at its foot), gift boxes stack on it (two ribbon
// confetti bursts, one box each, before the slam), the desk lands at the slam in a ring and a shake, and an
// apple rolls to a stop on top (a short shot just after it).
// Text: on activation the boxes pop open (confetti at the desk): a mana gem fills (sparkle at the mana
// gems), an opponent's card flutters from their hand pile to their graveyard, the hero glows healed, and four
// card backs fly from the desk to the owner's hand pile, one after another (ghosts, backs to everyone).
// R202: no cue names a card; the discarded card is a ghost back, never drawn face up.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid, HIDDEN_ID, targetFor } from "../../../game/animations.ts";
import { sideOf } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FX_SLAM_AT } from "../../constants.ts";
import { FLAVOR_LOOKS, paletteLook } from "../../flavors.ts";
import { hitAnchor } from "../../motifs/build.ts";
import type { FxAnchor, FxCue } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, type CardRecipe, type Entrance } from "../recipe.ts";

/** Every particle count the desk and its boxes throw, at intensity "normal" (rule 9). */
const TUNING = {
  rise: { count: 10, power: 0.7 },
  box: { count: 8, power: 0.7 },
  slam: { count: 16, power: 1 },
  apple: { count: 6, power: 0.5 },
  pop: { count: 10, power: 0.9 },
  mana: { count: 8, power: 0.6 },
  flutter: { count: 8, power: 0.5 },
  heal: { count: 14, power: 0.8 },
  back: { count: 6, power: 0.6 },
} as const;

const PALETTE = "Nerd/chalk";
/** Entrance: the desk rises early, a box stacks at each of these, the apple rolls after the slam. */
const RISE_AT = 0.1;
const BOX_ATS = [0.25, 0.4] as const;
const APPLE_AT = 0.2;
const APPLE_FROM = { x: 0.2, y: 0.3 } as const;
const APPLE_TO = { x: 0.5, y: 0.3 } as const;
/** Activation: each beat's start, the ghosts' stagger and their flight, as fractions of the entry. */
const POP_AT = 0.05;
const FLUTTER_AT = 0.1;
const FLUTTER_LAND_AT = 0.4;
const HEAL_AT = 0.1;
const MANA_AT = 0.1;
const BACK_AT = 0.1;
const BACK_STEP = 0.06;
const BACK_LAND_AT = 0.4;

export const giftDesk: Entrance = ({ D, tgt, intensity: i }) => {
  const at = tid(tgt);
  const foot = tid(tgt, FOOT);
  const slam = frac(FX_SLAM_AT, D);
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const boxes = BOX_ATS.map((boxAt): FxCue => ({ ...tunedBurst(i, "confetti", at, "area", frac(boxAt, D), TUNING.box), colors: look.colors }));
  return [
    { ...tunedBurst(i, "dust", foot, "area", frac(RISE_AT, D), TUNING.rise), colors: look.colors },
    ...boxes,
    ringCue(D, "dust", at, slam),
    { ...tunedBurst(i, "dust", at, "area", slam, TUNING.slam), colors: look.colors },
    {
      kind: "shot",
      shot: { motion: "bolt", trail: "leaf" },
      colors: look.colors,
      preset: "leaf",
      from: tid(tgt, APPLE_FROM),
      to: tid(tgt, APPLE_TO),
      delayMs: slam,
      flightMs: frac(APPLE_AT, D),
      density: i,
    },
    ...shakeCues(i, ENTRANCE_TRAUMA.giftDesk, slam),
  ];
};

/** KY's Gift pops open on its activation: each of its results is drawn at its own place. */
export const giftBoxes: CardRecipe = (event, p, play) => {
  if (play.hook !== "activate") return null;
  const i = p.env.intensity;
  const look = paletteLook(PALETTE) ?? FLAVOR_LOOKS.Nerd;
  const mine = sideOf(p.view, play.player);
  const desk: FxAnchor = (play.instanceId === HIDDEN_ID ? null : hitAnchor(p.view, play.instanceId)) ?? tid(animTestid.hand(mine));
  const pop = (delay: number): FxCue => ({ ...tunedBurst(i, "confetti", desk, "area", delay, TUNING.pop), colors: look.colors });
  if (event.type === "manaChanged") {
    const gem = tid(animTestid.mana(sideOf(p.view, event.player)));
    return { cues: [pop(frac(POP_AT, p.D)), tunedBurst(i, "sparkle", gem, "point", frac(MANA_AT, p.D), TUNING.mana)], row: "keep" };
  }
  if (event.type === "discarded") {
    const side = sideOf(p.view, event.owner);
    const hand = tid(animTestid.hand(side));
    const grave = tid(animTestid.graveyard(side));
    const cues: FxCue[] = [
      pop(frac(POP_AT, p.D)),
      { kind: "ghost", from: hand, to: grave, delayMs: frac(FLUTTER_AT, p.D), durationMs: frac(FLUTTER_LAND_AT - FLUTTER_AT, p.D) },
      { ...tunedBurst(i, "smoke", grave, "point", frac(FLUTTER_LAND_AT, p.D), TUNING.flutter), colors: look.colors },
    ];
    return { cues, row: "keep" };
  }
  if (event.type === "healed") {
    const tgt = targetFor(event, p.view);
    if (tgt === null) return null;
    return { cues: [pop(frac(POP_AT, p.D)), tunedBurst(i, "holy", tid(tgt), "area", frac(HEAL_AT, p.D), TUNING.heal)], row: "keep" };
  }
  if (event.type === "addedToHand") {
    const k = Math.max(0, p.entry.events.filter((e) => e.type === "addedToHand").indexOf(event));
    const start = frac(BACK_AT + k * BACK_STEP, p.D);
    const land = frac(BACK_LAND_AT + k * BACK_STEP, p.D);
    const hand = tid(animTestid.hand(sideOf(p.view, event.player)));
    const cues: FxCue[] = [
      { kind: "ghost", from: desk, to: hand, delayMs: start, durationMs: land - start },
      tunedBurst(i, "sparkle", hand, "point", land, TUNING.back),
    ];
    if (k === 0) cues.push(pop(frac(POP_AT, p.D)));
    return { cues, row: "keep" };
  }
  return null;
};

const FIXTURE_MANA = 1;
const FIXTURE_HEAL = 2;

/** The gallery's Gift: mana, one discard of the opponent, the heal and four cards, in the card's order. */
export function giftBoxesFixture(_view: PlayerView, instanceId: string, player: PlayerId): GameEvent[] {
  const other: PlayerId = player === "p1" ? "p2" : "p1";
  return [
    { type: "hookResolving", sourceId: instanceId, defId: "classicplus-042-1", controller: player, hook: "activate" },
    { type: "manaChanged", player, current: FIXTURE_MANA, max: FIXTURE_MANA },
    { type: "discarded", instanceId: "fx-gift-discard", defId: "core-005", owner: other },
    { type: "healed", targetId: `hero-${player}`, amount: FIXTURE_HEAL },
    ...["a", "b", "c", "d"].map((tag): GameEvent => ({ type: "addedToHand", player, instanceId: `fx-gift-${tag}`, defId: "core-005" })),
  ];
}
