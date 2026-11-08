// R3407000: Stinkyscape #111 Shockwave447 (`stinkyscape-111`, a Mythic Unit): entrance "shockwaveRing",
// bespoke "shockwaveLecture", tile aura "arcflick" in Tech, takeover "shockwave" (takeover.ts draws his own
// art, the orbital auditorium).
//
// - Entrance (after the takeover's hold): diagrams of light scrawl outward at speed (a sheen over the board
//   and a ring of circuit lines from its centre), then crash into his zone from the board's edges on three
//   electric-blue bolts; at the slam a double shockwave ring and a burst of circuit lines, and the board
//   shakes. His combat keywords then snap on as icons orbiting his card, one `callout` each, as the landing
//   unit shows them.
// - Cry ("Replace both players' hands with random complicated cards"): the first card of each hand to enter
//   its graveyard sends his shockwave as a beam from his card (his hero when the board does not show him) to
//   that side's hand pile, where it rings, the old backs crumple into smoke and circuit lines, and one card
//   back falls to that side's graveyard; each card then added to a hand drops in as a card back from his card
//   or hero to that player's hand pile, in place of the hand's own glint.
//
// Moved from the issue's concept: the engine draws his keywords as he arrives with no event (R781004's
// `drawCombatKeywords` keeps them on the card), so no `keywordGranted` comes: the icons are the keywords the
// view shows on the landing unit, none when it shows none. The tile's arcs are the aura (`arcflick`, its
// table entry), not a cue here.
//
// R202: no cue names a card. A hand is only its pile and a card moving only its back; `addedToHand` is read
// for its `player` alone (its id and definition are "hidden" on the other seat), `enteredGraveyard` for its
// `owner` alone, and a new card's face is never drawn. Each beat starts inside its entry and ends within
// FX_MAX_TAIL_MS of it (R200).

import type { GameEvent, KeywordKind, PlayerId, PlayerView } from "@jackioh/shared";

import { locateInstance } from "../../../game/animations.ts";
import { sideOf, testid } from "../../../game/contract.ts";
import { frac, ringCue, shakeCues, tid } from "../../build.ts";
import { FX_CENTER, FX_SLAM_AT } from "../../constants.ts";
import { motifBurst, motifRing, motifShot } from "../../motifs/build.ts";
import type { FxAnchor, FxCue, FxPlay, FxPoint } from "../../types.ts";
import { ENTRANCE_TRAUMA, type CardRecipe, type Entrance, type EntrancePlan } from "../recipe.ts";
import { backTo, entranceSpec, entranceTimes, pileOf, sigSpec } from "./signature.ts";

/** His palette: Tech's electric blue. */
const TECH = "Tech";

/** The second shockwave ring's preset, an icier blue under Tech's circuit ring. */
const ECHO_RING = "frost";

/** Every particle count his signature throws at scale 1 (rule 9); `orbit` is the most icons that orbit him. */
const TUNING = {
  scrawl: { count: 22, power: 1.4 },
  crash: { count: 20, power: 1.2 },
  crumple: { count: 12, power: 0.6 },
  lines: { count: 10, power: 0.9 },
  orbit: 3,
} as const;

/**
 * Each beat's moments as fractions of its entry's D (the entrance's of its motion after the hold); the
 * keyword icons stay `calloutTailMs` past the entry and a crumpled back falls `fallTailMs` past it, both
 * inside FX_MAX_TAIL_MS.
 */
const TIMING = {
  shockwaveRing: { scrawlAt: 0, crashAt: 0.1, echoStep: 0.08, calloutAt: 0.05, calloutStep: 0.08, calloutTailMs: 600 },
  shockwaveLecture: { sweepAt: 0, hitAt: 0.3, fallTailMs: 400, dropAt: 0 },
} as const;

/** The board's edges his diagrams crash in from. */
const EDGES: readonly FxPoint[] = [
  { x: 0.05, y: FX_CENTER.y },
  { x: FX_CENTER.x, y: 0.05 },
  { x: 0.95, y: FX_CENTER.y },
];

/** Where each keyword icon orbits his card, in order. */
const ORBIT: readonly FxPoint[] = [
  { x: 0.12, y: 0.18 },
  { x: 0.88, y: 0.18 },
  { x: 0.5, y: 0.98 },
];

/** The keywords the landing unit shows, at most TUNING.orbit; none when the view does not show it in its zone. */
function landedKeywords(p: EntrancePlan): KeywordKind[] {
  if (p.view === undefined || p.landing === undefined || p.landing.row !== "units") return [];
  const unit = p.view[sideOf(p.view, p.landing.player)].units[p.landing.lane - 1];
  if (unit === null || unit === undefined || unit.instanceId !== p.landing.instanceId) return [];
  return unit.keywords.slice(0, TUNING.orbit).map((keyword) => keyword.kind);
}

export const shockwaveRing: Entrance = (p) => {
  const spec = entranceSpec(p, TECH);
  const { at, slam } = entranceTimes(p);
  const T = TIMING.shockwaveRing;
  const zone = tid(p.tgt);
  const scrawl = at(T.scrawlAt);
  const crash = at(T.crashAt);
  const cues: FxCue[] = [
    { kind: "sheen", at: tid(testid.board), delayMs: scrawl, durationMs: p.D - scrawl },
    motifBurst(spec, tid(testid.board, FX_CENTER), "ring", scrawl, TUNING.scrawl),
    ...EDGES.map((edge) => motifShot(spec, "bolt", tid(testid.board, edge), zone, crash, Math.max(0, slam - crash))),
    motifRing(spec, p.D, zone, slam),
    ringCue(p.D, ECHO_RING, zone, at(FX_SLAM_AT + T.echoStep)),
    motifBurst(spec, zone, "ring", slam, TUNING.crash),
    ...shakeCues(p.intensity, ENTRANCE_TRAUMA.shockwaveRing, slam),
  ];
  landedKeywords(p).forEach((keyword, k) => {
    const delayMs = at(FX_SLAM_AT + T.calloutAt + k * T.calloutStep);
    cues.push({ kind: "callout", keyword, at: tid(p.tgt, ORBIT[k]), delayMs, durationMs: p.D - delayMs + T.calloutTailMs });
  });
  return cues;
};

/** His card where the board shows it (never a hand card), else his controller's hero. */
function origin(view: PlayerView, play: FxPlay): FxAnchor {
  const at = locateInstance(view, play.instanceId);
  return tid(at !== null && at === testid.card(play.instanceId) ? at : testid.hero(sideOf(view, play.player)));
}

/** Whether `event` opens its owner's run of cards entering the graveyard: the event before it is not one of theirs. */
function firstOfHand(event: Extract<GameEvent, { type: "enteredGraveyard" }>, recent: readonly GameEvent[]): boolean {
  const before = recent[recent.lastIndexOf(event) - 1];
  return !(before?.type === "enteredGraveyard" && before.owner === event.owner);
}

export const shockwaveLecture: CardRecipe = (event, p, play) => {
  if (play.hook !== undefined) return null;
  const T = TIMING.shockwaveLecture;
  if (event.type === "enteredGraveyard") {
    if (!firstOfHand(event, p.env.memory.recent())) return null;
    const spec = sigSpec(p, play, TECH);
    const hand = pileOf(p.view, event.owner, "hand");
    const sweep = frac(T.sweepAt, p.D);
    const hit = frac(T.hitAt, p.D);
    return {
      cues: [
        motifShot(spec, "beam", origin(p.view, play), hand, sweep, hit - sweep),
        motifRing(spec, p.D, hand, hit),
        motifBurst(spec, hand, "area", hit, TUNING.crumple, "smoke"),
        motifBurst(spec, hand, "ring", hit, TUNING.lines),
        backTo(hand, pileOf(p.view, event.owner, "graveyard"), hit, p.D - hit + T.fallTailMs),
      ],
      row: "keep",
    };
  }
  if (event.type === "addedToHand") {
    const drop = frac(T.dropAt, p.D);
    return { cues: [backTo(origin(p.view, play), pileOf(p.view, event.player, "hand"), drop, p.D - drop)], row: "replace" };
  }
  return null;
};

/**
 * R3407000: his Cry, as the gallery plays it: the first card of his controller's hand enters its graveyard
 * (one stands for the sweep of both hands, so the run fits its ceiling). The cards the hands are refilled
 * with (`addedToHand`) are the module test's.
 */
export function shockwaveLectureFixture(_view: PlayerView, _instanceId: string, player: PlayerId, _defId?: string): GameEvent[] {
  return [{ type: "enteredGraveyard", instanceId: "fx-lectured", defId: "core-005", owner: player }];
}
