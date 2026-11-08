// R3407000: Stinkyscape #112 Job Application (`stinkyscape-112`, a Legendary Field Spell and Quest: entrance
// "resumeTube", recipe "jobInterview", aura "steplights" in Hustle/cash on its zone) and its Stage 3 reward,
// #112.1 Paycheck (`stinkyscape-112-1`, a token Spell printed Legendary: recipe "payday"), each a signature.
//
// - Job Application's entrance: a pneumatic capsule shoots down into its zone and lands with a thunk (a ring,
//   a puff of dust, a small shake); a blank resume sheet (the `scroll` figure, paper white) rolls itself up
//   there and a tiny office door (the `scaffold` figure, small) is laid in its place as a fluorescent light
//   flickers on over it (two sparks, then holy rays).
// - Its text, on its own entries (its play, its Activates, its hooks): Stage 1's Activate holds a hand card
//   beneath it (`held`), and the resume, a card back, zips from its owner's hand pile into its zone. Each
//   `questProgressed` of its own lights a step over the door (a spark at the zone's top) under a spotlight
//   (holy rays); each `questCompleted` of its own swings the door open (a ring and a gold burst); each card
//   its hooks add to a hand (`addedToHand` in its Whenever or turn hook: the resume coming back Buffed at The
//   Interview's reward, a Paycheck at each of its controller's turns once hired) flies out of the door to
//   that hand pile as a card back.
// - Paycheck's cast: a blank green pay slip (the `scroll` figure, small, cash green) feeds out of a slot over
//   its caster's hero and folds itself into a credit chip with a crisp snap (a sheen and a gold burst there).
//   Payday: its crystals fill with a gold ring (`manaChanged`), a card back slides from the deck pile to the
//   hand pile (`drawn`), and its Credits tick up with a gold ring on the hero (`marketCreditsChanged`).
//
// Moved from the issue's concept: the resume goes in on `held`, the Activate's own effect, not on
// `activated`, which comes before the Activate's marker, so no entry of the card's is open yet (memory.ts).
// The door's card back flies out on its hooks' `addedToHand`, not on `questCompleted`: no card moves when a
// stage completes inside its own entries (Stage 1's sends the resume in, Stage 3's accept only notes the
// hire), The Interview completes in another card's play, which its recipe never keys, and the resume and
// each Paycheck reach the hand in its Whenever and turn hooks. The spotlight lights each step it sees, since
// The Interview's own steps (its opening at a turn's start, its goals counted in other plays) run in no entry
// of the card's and draw the shared tally alone. Paycheck's roulette wheel and its smoke puff are dropped:
// its mode is chosen with the play and no event names it, a loss on red emits nothing, and a win emits the
// same three events with doubled numbers, so it draws Payday's beats, the rows carrying the amounts. The
// Credits' tick is a ring with no burst: a burst lasts 900 ms past its delay, past Paycheck's ceiling from
// any entry after its cast.
//
// R202: the resume and every card added or drawn is a card back between piles; no beat reads `held`'s,
// `addedToHand`'s or `drawn`'s ids (the other seat's are the sentinel), only their `player`. Its zone is its
// own card where the view shows it face up, else the zone its `summoned` named, else its side's backrow.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { HIDDEN_ID, animTestid, locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, raysCue, ringCue, shakeCues, tid } from "../../build.ts";
import { figureCue, figureTint } from "../../figures.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, FOOT, heroOf, type CardFxPlan, type CardRecipe, type Entrance } from "../recipe.ts";
import { backTo, entranceSpec, entranceTimes, lookOf, pileOf, sigSpec } from "./signature.ts";

/** Every particle count the office and the pay slip throw at scale 1 (rule 9). */
const TUNING = {
  thunk: { count: 16, power: 0.9 },
  flicker: { count: 5, power: 0.4 },
  sent: { count: 8, power: 0.5 },
  step: { count: 6, power: 0.5 },
  open: { count: 18, power: 1 },
  snap: { count: 14, power: 0.8 },
} as const;

/** Each beat's moments as fractions of its entry's D (an entrance's of its motion after its hold), and its tails in ms. */
const TIMING = {
  resumeTube: { tubeAt: 0.05, doorAt: 0.75, flickerAt: 0.8, flickerStep: 0.08, tailMs: 300 },
  jobInterview: { sendAt: 0.05, sendFor: 0.6, stepAt: 0.1, openAt: 0.15, outAt: 0.05, outFor: 0.7 },
  payday: { slipFor: 0.6, snapAt: 0.5, fillAt: 0.1, tickAt: 0.1 },
} as const;

/** The palettes: the office's cash green and the resume's paper white. */
const CASH = "Hustle/cash";
const PAPER = "Holy";

/** Where the capsule comes from, above its zone, and where a step light hangs, at the zone's top (fractions of the zone). */
const TUBE_FROM: FxPoint = { x: 0.5, y: -0.8 };
const STEP: FxPoint = { x: 0.5, y: 0.1 };
/** The slot over the caster's hero the pay slip feeds out of. */
const SLOT: FxPoint = { x: 0.5, y: 0 };

/**
 * R3407000: where the play's card stands: its card while the view shows it face up on the board, else the zone
 * its `summoned` named (a card landing now is not in the view yet), else `fallback`.
 */
function standing(p: CardFxPlan, play: FxPlay, fallback: string): FxAnchor {
  const at = locateInstance(p.view, play.instanceId);
  if (at !== null && at.startsWith("card-")) return tid(at);
  const landed = [...p.env.memory.recent()].reverse().find((event) => event.type === "summoned" && event.instanceId === play.instanceId);
  if (landed?.type === "summoned") return tid(testid.zone(sideOf(p.view, landed.player), landed.row, landed.lane));
  return tid(fallback);
}

/* Job Application */

export const resumeTube: Entrance = (p) => {
  const spec = entranceSpec(p, CASH);
  const paper = figureTint(lookOf(PAPER));
  const { at, slam } = entranceTimes(p);
  const T = TIMING.resumeTube;
  const zone = tid(p.tgt);
  const lit = at(T.flickerAt + T.flickerStep);
  return [
    motifShot(spec, "bolt", tid(p.tgt, TUBE_FROM), zone, at(T.tubeAt), slam - at(T.tubeAt)),
    motifRing(spec, p.D, zone, slam),
    motifBurst(spec, tid(p.tgt, FOOT), "area", slam, TUNING.thunk, "dust"),
    figureCue("scroll", zone, paper, slam, p.D - slam + T.tailMs),
    figureCue("scaffold", zone, figureTint(spec.look), at(T.doorAt), p.D - at(T.doorAt) + T.tailMs, { size: "small" }),
    motifBurst(spec, tid(p.tgt, STEP), "point", at(T.flickerAt), TUNING.flicker, "spark"),
    motifBurst(spec, tid(p.tgt, STEP), "point", lit, TUNING.flicker, "spark"),
    raysCue(p.D, "holy", zone, lit),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.resumeTube, slam),
  ];
};

export const jobInterview: CardRecipe = (event, p, play) => {
  const spec = sigSpec(p, play, CASH);
  const T = TIMING.jobInterview;
  const office = (): FxAnchor => standing(p, play, animTestid.backrow(sideOf(p.view, play.player)));
  // Stage 1's Activate: the resume, a card back from its owner's hand pile, zips into the tube.
  if (event.type === "held" && play.hook === "activate") {
    const delayMs = frac(T.sendAt, p.D);
    const land = delayMs + frac(T.sendFor, p.D);
    const at = office();
    return { cues: [backTo(pileOf(p.view, event.player, "hand"), at, delayMs, land - delayMs), motifBurst(spec, at, "point", land, TUNING.sent)], row: "keep" };
  }
  // A step lights over the door under the spotlight.
  if (event.type === "questProgressed" && event.instanceId === play.instanceId) {
    const delayMs = frac(T.stepAt, p.D);
    const at = office();
    const top = at.kind === "testid" ? tid(at.testid, STEP) : at;
    return { cues: [motifBurst(spec, top, "point", delayMs, TUNING.step, "spark"), raysCue(p.D, "holy", at, delayMs)], row: "keep" };
  }
  // The door swings open.
  if (event.type === "questCompleted" && event.instanceId === play.instanceId) {
    const delayMs = frac(T.openAt, p.D);
    const at = office();
    return { cues: [motifRing(spec, p.D, at, delayMs), motifBurst(spec, at, "area", delayMs, TUNING.open, "gold")], row: "keep" };
  }
  // A card its hooks add (the resume coming back, a Paycheck) flies out of the door as a card back.
  if (event.type === "addedToHand" && play.hook === "trigger") {
    const delayMs = frac(T.outAt, p.D);
    return { cues: [backTo(office(), pileOf(p.view, event.player, "hand"), delayMs, frac(T.outFor, p.D))], row: "keep" };
  }
  return null;
};

/* Paycheck */

export const payday: CardRecipe = (event, p, play) => {
  const spec = sigSpec(p, play, CASH);
  const T = TIMING.payday;
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    const hero = heroOf(p.view, play);
    const snap = frac(T.snapAt, p.D);
    return {
      cues: [
        figureCue("scroll", tid(testid.hero(sideOf(p.view, play.player)), SLOT), figureTint(spec.look), 0, frac(T.slipFor, p.D), { size: "small" }),
        { kind: "sheen", at: hero, delayMs: snap, durationMs: p.D - snap },
        motifBurst(spec, hero, "point", snap, TUNING.snap, "gold"),
      ],
      row: "replace",
    };
  }
  if (play.hook !== undefined) return null;
  // Its crystals fill.
  if (event.type === "manaChanged" && event.player === play.player) {
    return { cues: [ringCue(p.D, "gold", pileOf(p.view, event.player, "mana"), frac(T.fillAt, p.D))], row: "replace" };
  }
  // The card it draws slides in as a card back.
  if (event.type === "drawn" && event.player === play.player) {
    return { cues: [backTo(pileOf(p.view, event.player, "library"), pileOf(p.view, event.player, "hand"), 0, p.D)], row: "replace" };
  }
  // Its Credits tick up on its hero.
  if (event.type === "marketCreditsChanged" && event.player === play.player) {
    return { cues: [ringCue(p.D, "gold", heroOf(p.view, play), frac(T.tickAt, p.D))], row: "keep" };
  }
  return null;
};

/* The gallery's fixtures */

/**
 * R3407000: Job Application's Stage 1 Activate: the viewer's first hand card (a sentinel on the other seat) is
 * held beneath it as the resume (one keyed event, so the run fits its ceiling after its entrance).
 */
export function jobInterviewFixture(view: PlayerView, instanceId: string, player: PlayerId, defId = "stinkyscape-112"): GameEvent[] {
  const hand = player === view.viewer && Array.isArray(view.you.hand) ? view.you.hand[0] : undefined;
  return [
    { type: "hookResolving", sourceId: instanceId, defId, controller: player, hook: "activate" },
    { type: "held", player, instanceId: hand?.instanceId ?? HIDDEN_ID, defId: hand?.defId ?? HIDDEN_ID, holderId: instanceId },
  ];
}

/** R3407000: Paycheck's Payday draws its card (its mana and Credits are the module test's: one keyed event fits its ceiling). */
export function paydayFixture(view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const seen = player === view.viewer;
  return [{ type: "drawn", player, instanceId: seen ? "fx-paid" : HIDDEN_ID, defId: seen ? "core-003" : HIDDEN_ID }];
}
