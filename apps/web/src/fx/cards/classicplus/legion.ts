// R3399000: Classic+ Legion of the Hungry (`classicplus-012-7`, entrance none, bespoke "hungryPot"), its signature (#3399):
// a Spell printed Legendary, so every burst starts by SPELL_BURST_LATEST_MS of its entry (1,200 ms ceiling).
// Cast (its `cardPlayed`): the caster's deck pile rattles and its top lifts like a pot lid, forks and
// grasping hands (sparks and crumbs) poking out. Text: at each `exiled` a card back bursts out of the deck
// pile to the exile pile, one after another, crumbling into crumbs there; at each `summoned` the Unit
// crawls out of the exile pile onto its zone, hungry (a ghost back, a ring and crumbs). R202: it draws only
// piles and the event's own public zone, never a card's id.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";

import { animTestid } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid, tunedBurst } from "../../build.ts";
import { FLAVOR_LOOKS } from "../../flavors.ts";
import { motifBurst, motifScale, type CueSpec } from "../../motifs/build.ts";
import type { CardRecipe } from "../recipe.ts";

/** Every particle count the pot throws at scale 1 (rule 9). */
const TUNING = {
  lid: { count: 12, power: 0.8 },
  forks: { count: 8, power: 1 },
  crumble: { count: 6, power: 0.6 },
  crawl: { count: 10, power: 0.8 },
} as const;

/** A Spell's bursts start no later than this into its entry, so they end inside its 1,200 ms ceiling (#3399). */
const SPELL_BURST_LATEST_MS = 300;
/** Where in its entry each beat starts: the lid, a card's burst and its stagger, the crawl, and a card's flight. */
const AT = { lid: 0, burst: 0.05, stagger: 0.06, crawl: 0.25, fly: 0.3 } as const;
/** The rattling lid's shake, a Spell's at most 1 (#3399). */
const LID_TRAUMA = 0.4;
/** How many cards the gallery's Legion exiles, and how many of them are Units. */
const FIXTURE_CARDS = 5;
const FIXTURE_UNITS = 2;
/** The Unit the gallery's Legion summons, a core Unit never named in a cue (R202). */
const FIXTURE_UNIT = "core-019";

function at(fraction: number, D: number): number {
  return Math.min(frac(fraction, D), SPELL_BURST_LATEST_MS);
}

/** Legion of the Hungry: the lid lifts on its cast, each exile bursts out and each Unit crawls onto the board. */
export const hungryPot: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined || !(p.D > 0)) return null;
  const spec: CueSpec = { look: FLAVOR_LOOKS.Snack, scale: motifScale(play.defId, p.view, p.env.intensity), row: "keep" };
  const deck = tid(animTestid.library(sideOf(p.view, play.player)));
  if (event.type === "cardPlayed" && event.instanceId === play.instanceId) {
    return {
      cues: [
        motifBurst(spec, deck, "area", at(AT.lid, p.D), TUNING.lid),
        tunedBurst(spec.scale, "spark", deck, "point", at(AT.lid, p.D), TUNING.forks),
        ringCue(p.D, "crumb", deck, at(AT.lid, p.D)),
        ...shakeCues(spec.scale, LID_TRAUMA, at(AT.lid, p.D)),
      ],
      row: "keep",
    };
  }
  if (event.type === "exiled") {
    const own = p.entry.events.filter((candidate) => candidate.type === "exiled");
    const k = Math.max(0, own.indexOf(event));
    const exile = tid(animTestid.exile(sideOf(p.view, event.owner)));
    const start = at(AT.burst + k * AT.stagger, p.D);
    return {
      cues: [
        { kind: "ghost", from: deck, to: exile, delayMs: start, durationMs: frac(AT.fly, p.D) },
        motifBurst(spec, exile, "area", Math.min(SPELL_BURST_LATEST_MS, start + frac(AT.fly, p.D)), TUNING.crumble),
      ],
      // The ghost and its crumbs are the exile, drawn in place of its row (the phone frame budget).
      row: "replace",
    };
  }
  if (event.type === "summoned") {
    const zone = tid(testid.zone(sideOf(p.view, event.player), event.row, event.lane));
    const exile = tid(animTestid.exile(sideOf(p.view, event.player)));
    return {
      cues: [
        { kind: "ghost", from: exile, to: zone, delayMs: at(AT.burst, p.D), durationMs: frac(AT.crawl, p.D) },
        ringCue(p.D, "crumb", zone, at(AT.crawl, p.D)),
        motifBurst(spec, zone, "area", at(AT.crawl, p.D), TUNING.crawl),
      ],
      row: "keep",
    };
  }
  return null;
};

/** The gallery's Legion: five cards burst out of the player's deck, and the first two, Units, crawl onto the board. */
export function hungryPotFixture(_view: PlayerView, _instanceId: string, player: PlayerId): GameEvent[] {
  const ids = Array.from({ length: FIXTURE_CARDS }, (_, k) => `fx-hungry-${String(k)}`);
  const exiled: GameEvent[] = ids.map((instanceId) => ({ type: "exiled", instanceId, defId: FIXTURE_UNIT, owner: player }));
  const summoned: GameEvent[] = ids.slice(0, FIXTURE_UNITS).map((instanceId, k) => ({ type: "summoned", player, instanceId, defId: FIXTURE_UNIT, row: "units", lane: k + 1 }));
  return [...exiled, ...summoned];
}
