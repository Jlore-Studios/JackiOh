// Animation table and event-queue runner (BUILD M5-T4, SPEC §10.10; SPEC §10.3 GameEventType map).
// Timing/presentation only, no rules (CLAUDE.md rule 7); emits AnimatingMap for data-animating.
// Effects layer (apps/web/src/fx, docs/polish/1-animations.md, R200) decorates without pacing;
// speed scales burst budget (R201); reduced motion collapses durations.

import type { GameEvent, GameEventType, PlayerId, PlayerView, Zone } from "@jackioh/shared";

import type { FxDescriptor } from "../fx/types.ts";
import { getFxSettings, normalizeSpeed, type FxSettings } from "../fx/settings.ts";
import { readSettings as readPanelSettings } from "../settings/store.ts";
import { type AnimatingMap, type Side, sideOf, testid } from "./contract";
import { SLAM_BURST, UNIT_SLAM, type SlamTier } from "./damageFeel.ts";
import { createPlayTracker, pileOf, pileRunAt, sweepAt, volleyAt, type Pile, type PlayTracker, type Sweep, type ZoneImpact } from "./runs.ts";
import { FX_FLAVOR_LIVE } from "../fx/constants.ts";
import { isRecord } from "../wire/json.ts";
import { prefersReducedMotion } from "../settings/motion.ts";

// Testids this table needs that `contract.ts` does not define

/** Animation DOM hooks not named in contract.ts testids (BUILD M5-T1). */
export const animTestid = {
  /** The hand as a region: an opponent hand is a `count` only (§10.8), so it has no card ids. */
  hand: (side: Side): string => `hand-${side}`,
  library: (side: Side): string => `library-${side}`,
  graveyard: (side: Side): string => `graveyard-${side}`,
  exile: (side: Side): string => `exile-${side}`,
  /** The mana crystal tray. */
  mana: (side: Side): string => `mana-${side}`,
  /** The player-modifier badge list beside the hero (§6.4). */
  modifiers: (side: Side): string => `modifiers-${side}`,
  /** The backrow as a region: a face-down `BackrowView` carries no `instanceId` (§10.8). */
  backrow: (side: Side): string => `backrow-${side}`,
  /** The prompt modal; BUILD M5-T4 wants `data-prompt-kind` on it. */
  prompt: "prompt-modal",
  /** The draw-offer toast. */
  drawToast: "draw-toast",
} as const;

/* ------------------------------------------------------------------------------------------- *
 * Row types
 * ------------------------------------------------------------------------------------------- */

export type AnimationSpec = {
  /** The CSS animation name defined in `animations.css`, applied via `[data-animating="…"]`. */
  animation: string;
  /** The BUILD M5-T4 duration in milliseconds. */
  durationMs: number;
  /** The data-testid template (<…> payload field, <side> viewer-relative side). */
  testid: string;
  /** The effect recipe that decorates this row (docs/polish/1-animations.md, S7). Absent: no effect. */
  fx?: FxDescriptor;
};

/** Resolves the element for one event type; `null` when this seat shows nothing for it. */
export type TargetResolver<K extends GameEventType> = (
  event: Extract<GameEvent, { type: K }>,
  view: PlayerView,
) => string | null;

export type AnimationRow<K extends GameEventType = GameEventType> = AnimationSpec & {
  target: TargetResolver<K>;
};

/* ------------------------------------------------------------------------------------------- *
 * Payload → testid helpers
 * ------------------------------------------------------------------------------------------- */

/** The engine writes a hero damage/heal target as `hero-<playerId>` (`crates/engine/src/damage.rs`). */
const HERO_TARGET_ID = /^hero-(p1|p2)$/;

function heroSide(view: PlayerView, targetId: string): Side | null {
  const match = HERO_TARGET_ID.exec(targetId);
  const player = match?.[1];
  if (player === undefined) return null;
  return sideOf(view, player as PlayerId);
}

/** Where an instance is rendered right now, or null if nowhere (omits face-down cards). */
export function locateInstance(view: PlayerView, instanceId: string): string | null {
  for (const side of ["you", "opponent"] as const) {
    const sv = view[side];
    for (const unit of sv.units) {
      if (unit !== null && unit.instanceId === instanceId) return testid.card(instanceId);
    }
    for (const slot of sv.backrow) {
      if (slot !== null && slot.faceDown === false && slot.instanceId === instanceId) {
        return testid.card(instanceId);
      }
    }
  }
  const hand = view.you.hand;
  if (Array.isArray(hand) && hand.some((c) => c.instanceId === instanceId)) {
    return testid.handCard(instanceId);
  }
  return null;
}

/** The card element if it is on the board or in the viewer's hand, else the named pile. */
function instanceOrPile(view: PlayerView, instanceId: string, pile: string): string {
  return locateInstance(view, instanceId) ?? pile;
}

function zoneTestid(view: PlayerView, player: PlayerId, row: "units" | "backrow", lane: number): string {
  return testid.zone(sideOf(view, player), row, lane);
}

/** Resolves the `Zone` carried by `radiantSet` (§10.3) to whatever renders that zone. */
function zoneOfCard(view: PlayerView, zone: Zone, instanceId: string): string {
  const side = sideOf(view, zone.player);
  if (zone.z === "field") {
    return locateInstance(view, instanceId) ?? testid.zone(side, zone.row, zone.lane);
  }
  if (zone.z === "hand") return locateInstance(view, instanceId) ?? animTestid.hand(side);
  if (zone.z === "library") return animTestid.library(side);
  if (zone.z === "graveyard") return animTestid.graveyard(side);
  if (zone.z === "exile") return animTestid.exile(side);
  // "resolving": a card mid-resolution has no zone on the board, so the board stands in.
  return testid.board;
}

// The table

/**
 * `eventType → { animation, durationMs, testid }` with one row per `GameEvent["type"]` (BUILD M5-T4).
 * `animations.test.ts` pins durations literally.
 */
export const ANIMATIONS: { [K in GameEventType]: AnimationRow<K> } = {
  // Card lifts from hand and lands in the zone (unit) or flashes center then to GY (spell).
  // Only the viewer's own hand renders cards, so an opponent's play animates the hand region.
  // A card set face-down took a fresh id (R227): the hand card still carries `formerId`.
  cardPlayed: {
    animation: "jk-card-played",
    durationMs: 400,
    testid: "hand-card-<instanceId>",
    fx: { recipe: "cast" },
    target: (e, view) =>
      sideOf(view, e.player) === "you"
        ? (locateInstance(view, e.formerId ?? e.instanceId) ?? testid.handCard(e.formerId ?? e.instanceId))
        : animTestid.hand("opponent"),
  },
  // Card scales in at the zone. Collapsed into one motion with the `cardPlayed` that precedes it
  // for the same instance — see `planEntries`.
  summoned: {
    animation: "jk-summon-scale",
    durationMs: 250,
    testid: "zone-<side>-<row>-<lane>",
    fx: { recipe: "summon" },
    target: (e, view) => zoneTestid(view, e.player, e.row, e.lane),
  },
  // Red number pops on the target, target shakes; a hero portrait shakes.
  damage: {
    animation: "jk-damage-shake",
    durationMs: 300,
    testid: "card-<targetId> | hero-<side>",
    fx: { recipe: "impact" },
    target: (e, view) => {
      const side = heroSide(view, e.targetId);
      return side !== null ? testid.hero(side) : locateInstance(view, e.targetId);
    },
  },
  // Purple number pops on the hero, no shake.
  healthLost: {
    animation: "jk-loss-pop",
    durationMs: 300,
    testid: "hero-<side>",
    fx: { recipe: "drain" },
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // Green number pops.
  healed: {
    animation: "jk-heal-pop",
    durationMs: 300,
    testid: "card-<targetId> | hero-<side>",
    fx: { recipe: "heal" },
    target: (e, view) => {
      const side = heroSide(view, e.targetId);
      return side !== null ? testid.hero(side) : locateInstance(view, e.targetId);
    },
  },
  // Shield shatter.
  divineShieldLost: {
    animation: "jk-shield-shatter",
    durationMs: 250,
    testid: "card-<instanceId>",
    fx: { recipe: "shieldBreak" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Card dissolves, then slides to the GY count. A card destroyed off the board (R89 carries
  // `owner`) has no element of its own, so the pile it lands in stands in.
  destroyed: {
    animation: "jk-dissolve",
    durationMs: 350,
    testid: "card-<instanceId>",
    fx: { recipe: "death" },
    target: (e, view) =>
      instanceOrPile(view, e.instanceId, animTestid.graveyard(sideOf(view, e.owner))),
  },
  // §3.4, §10.5 step 7: brief settle on card in play or on graveyard pile (R17).
  cardResolved: {
    animation: "jk-card-resolved",
    durationMs: 150,
    testid: "resolved-<instanceId>",
    target: (e, view) =>
      e.permanent
        ? (locateInstance(view, e.instanceId) ?? animTestid.graveyard(sideOf(view, e.player)))
        : animTestid.graveyard(sideOf(view, e.player)),
  },
  enteredGraveyard: {
    animation: "jk-pile-pulse",
    durationMs: 150,
    testid: "graveyard-<side>",
    target: (e, view) => animTestid.graveyard(sideOf(view, e.owner)),
  },
  // Card fades to black and shrinks, then the exile counter increments.
  exiled: {
    animation: "jk-exile-fade",
    durationMs: 350,
    testid: "card-<instanceId>",
    fx: { recipe: "void" },
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.exile(sideOf(view, e.owner))),
  },
  // Card flies to its controller's hand: the event's owner is the hand's player (R747).
  bounced: {
    animation: "jk-bounce-to-hand",
    durationMs: 350,
    testid: "card-<instanceId>",
    fx: { recipe: "bounce" },
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.hand(sideOf(view, e.owner))),
  },
  // R317, R318: burned card catches fire over owner's hand toward graveyard (OverflowNotices.tsx).
  burned: {
    animation: "jk-hand-full",
    durationMs: 700,
    testid: "hand-<side>",
    fx: { recipe: "burn" },
    target: (e, view) => animTestid.hand(sideOf(view, e.owner)),
  },
  // R315, R318: a draw from an empty library. The pile shakes and dims as it comes up empty and a
  // "Fatigue N" badge rises from it (`pile-notice`); the `damage` row after it lands the hit.
  fatigue: {
    animation: "jk-fatigue",
    durationMs: 600,
    testid: "library-<side>",
    fx: { recipe: "fatigue" },
    target: (e, view) => animTestid.library(sideOf(view, e.player)),
  },
  // R316, R318: a full library turns a card away. "Library full" flashes on the pile, and the card
  // (face or back by R97) bounces off it, then fizzles or drops toward the graveyard (`pile-notice`).
  libraryOverflow: {
    animation: "jk-library-full",
    durationMs: 500,
    testid: "library-<side>",
    fx: { recipe: "overflow" },
    target: (e, view) => animTestid.library(sideOf(view, e.player)),
  },
  // Card drops from hand to GY.
  discarded: {
    animation: "jk-discard-drop",
    durationMs: 300,
    testid: "hand-card-<instanceId>",
    fx: { recipe: "discard" },
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.hand(sideOf(view, e.owner))),
  },
  // Card slides from library to hand (own) or back to the hand count (opponent). The motion
  // starts at the library pile, which both seats render; the drawn card is not in the hand yet.
  drawn: {
    animation: "jk-draw-slide",
    durationMs: 250,
    testid: "library-<side>",
    fx: { recipe: "draw" },
    target: (e, view) => animTestid.library(sideOf(view, e.player)),
  },
  // Card appears at the hand edge (own) or the hand count bumps (opponent).
  addedToHand: {
    animation: "jk-hand-edge",
    durationMs: 250,
    testid: "hand-<side>",
    fx: { recipe: "handGlint" },
    target: (e, view) => animTestid.hand(sideOf(view, e.player)),
  },
  // Card flies into the library, library pulses.
  shuffledIn: {
    animation: "jk-shuffle-in",
    durationMs: 300,
    testid: "library-<side>",
    fx: { recipe: "shuffle" },
    target: (e, view) => animTestid.library(sideOf(view, e.player)),
  },
  // Stat numbers flash and tick to their new values.
  buffed: {
    animation: "jk-stat-tick",
    durationMs: 250,
    testid: "card-<instanceId>",
    fx: { recipe: "buff" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Keyword icon pops in; a loss (R46, `lost`) pops it out, on the card keywordLoss.ts marks `data-lost`.
  keywordGranted: {
    animation: "jk-icon-pop",
    durationMs: 200,
    testid: "card-<instanceId>",
    fx: { recipe: "keyword" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Counter badge ticks.
  counterChanged: {
    animation: "jk-badge-tick",
    durationMs: 200,
    testid: "card-<instanceId>",
    fx: { recipe: "counter" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Cost gem flashes and ticks to the new value. A discount usually lands on a card in hand.
  costChanged: {
    animation: "jk-gem-tick",
    durationMs: 200,
    testid: "card-<instanceId> | hand-card-<instanceId>",
    fx: { recipe: "glint" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Player modifier badge appears or fades by the hero.
  modifierChanged: {
    animation: "jk-badge-fade",
    durationMs: 200,
    testid: "modifiers-<side>",
    fx: { recipe: "glint" },
    target: (e, view) => animTestid.modifiers(sideOf(view, e.player)),
  },
  // Gold glow pulse, stats swap. The `zone` payload says where the card is standing.
  radiantSet: {
    animation: "jk-radiant-pulse",
    durationMs: 400,
    testid: "card-<instanceId>",
    fx: { recipe: "radiant" },
    target: (e, view) => zoneOfCard(view, e.zone, e.instanceId),
  },
  // The gold glow goes out, stats swap back. The `zone` payload says where the card is standing.
  deradianted: {
    animation: "jk-radiant-pulse",
    durationMs: 400,
    testid: "card-<instanceId>",
    target: (e, view) => zoneOfCard(view, e.zone, e.instanceId),
  },
  // Card spins and shows its new face. Pre-update the old instance is the one on the board.
  transformed: {
    animation: "jk-spin-face",
    durationMs: 400,
    testid: "card-<instanceId>",
    fx: { recipe: "smoke" },
    target: (e, view) =>
      locateInstance(view, e.instanceId) ?? locateInstance(view, e.newInstanceId),
  },
  // Two cards merge into one. The first listed instance is where the merge is drawn from.
  fused: {
    animation: "jk-fuse-merge",
    durationMs: 500,
    testid: "card-<instanceIds[0]>",
    fx: { recipe: "fuse" },
    target: (e, view) => {
      for (const id of e.instanceIds) {
        const found = locateInstance(view, id);
        if (found !== null) return found;
      }
      return locateInstance(view, e.resultInstanceId);
    },
  },
  // Rotate 90° / back.
  positionSwitched: {
    animation: "jk-rotate-def",
    durationMs: 250,
    testid: "card-<instanceId>",
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Card slides across the center line to the new zone, which the payload names outright.
  controlChanged: {
    animation: "jk-cross-center",
    durationMs: 450,
    testid: "zone-<side>-<row>-<lane>",
    fx: { recipe: "mindControl" },
    target: (e, view) => zoneTestid(view, e.controller, e.row, e.lane),
  },
  // All cards slide one lane.
  rotated: {
    animation: "jk-lane-slide",
    durationMs: 500,
    testid: "board",
    target: () => testid.board,
  },
  // Swapped health, boards or library counts cross the center line together.
  swapped: {
    animation: "jk-swap-cross",
    durationMs: 500,
    testid: "board",
    target: () => testid.board,
  },
  // Chain icon closes over the zone.
  locked: {
    animation: "jk-chain-close",
    durationMs: 250,
    testid: "zone-<side>-<row>-<lane>",
    fx: { recipe: "lock" },
    target: (e, view) => zoneTestid(view, e.player, e.row, e.lane),
  },
  // Backrow trap flips face-up. R154 provides row/lane for seat lacking card instanceId (§10.8, R33).
  // Visible card is preferred over zone fallback (BUILD M5-T4).
  trapFired: {
    animation: "jk-trap-flip",
    durationMs: 700,
    testid: "card-<instanceId>",
    fx: { recipe: "trap" },
    target: (e, view) =>
      locateInstance(view, e.instanceId) ?? zoneTestid(view, e.controller, e.row, e.lane),
  },
  // Attacker lunges toward the target and back.
  attackDeclared: {
    animation: "jk-lunge",
    durationMs: 350,
    testid: "card-<attackerId>",
    fx: { recipe: "lunge" },
    target: (e, view) => locateInstance(view, e.attackerId),
  },
  // Attacker snaps back with a "Canceled" tag.
  attackCancelled: {
    animation: "jk-snap-back",
    durationMs: 350,
    testid: "card-<attackerId>",
    fx: { recipe: "fizzle" },
    target: (e, view) => locateInstance(view, e.attackerId),
  },
  // Crystals fill/empty.
  manaChanged: {
    animation: "jk-crystal-fill",
    durationMs: 150,
    testid: "mana-<side>",
    fx: { recipe: "mana" },
    target: (e, view) => animTestid.mana(sideOf(view, e.player)),
  },
  // Mana paid for a play or an activation; `manaChanged` sounds it.
  manaSpent: {
    animation: "jk-crystal-fill",
    durationMs: 150,
    testid: "mana-<side>",
    target: (e, view) => animTestid.mana(sideOf(view, e.player)),
  },
  // Banner "Your turn" / "Opponent's turn".
  turnStarted: {
    animation: "jk-banner",
    durationMs: 600,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // End-turn button greys out.
  turnEnded: {
    animation: "jk-grey-out",
    durationMs: 150,
    testid: "end-turn",
    target: () => testid.endTurn,
  },
  // Banner "No moves left — turn ended": the same banner motion as `turnStarted`.
  turnAutoEnded: {
    animation: "jk-banner",
    durationMs: 600,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // Modal fades in. Only the seat holding the prompt has a modal; the other shows a wait notice
  // (§10.6, M5-T2) and animates nothing, though the entry still spends its 150 ms so both seats
  // advance through the stream in step.
  promptOpened: {
    animation: "jk-fade-in",
    durationMs: 150,
    testid: "prompt-modal",
    target: (e, view) => (e.player === view.viewer ? animTestid.prompt : null),
  },
  // Modal fades out.
  promptAnswered: {
    animation: "jk-fade-out",
    durationMs: 150,
    testid: "prompt-modal",
    target: (e, view) => (e.player === view.viewer ? animTestid.prompt : null),
  },
  // Offer toast on the opponent's seat: `player` offered, so the toast is the other seat's.
  drawOffered: {
    animation: "jk-toast-in",
    durationMs: 150,
    testid: "draw-toast",
    target: (e, view) => (e.player === view.viewer ? null : animTestid.drawToast),
  },
  // Toast resolves to Accepted or Declined — both seats see the resolution.
  drawAnswered: {
    animation: "jk-toast-resolve",
    durationMs: 300,
    testid: "draw-toast",
    target: () => animTestid.drawToast,
  },
  // MD-D29, R1127: the emoter's hero shakes — the emote layer shows the emote itself (R644), so the
  // game animation only marks who sent it, and resolves instantly on an existing keyframe.
  emoted: {
    animation: "jk-loss-pop",
    durationMs: 0,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // Result overlay. BUILD's Duration column is "—": the overlay is terminal, nothing renders
  // behind it and nothing waits on it, so its duration is 0 and the entry resolves instantly.
  gameOver: {
    animation: "jk-result-overlay",
    durationMs: 0,
    testid: "result-overlay",
    target: () => testid.result,
  },

  // ---- Patch v0.2.0 (docs/classic-sets.md B3, B5) ----

  // B5 E1: the card lifts and hangs at its hand edge while the window for a Counter is open. A card
  // set face-down is announced by its zone only (R97), so the other seat's hand region lifts.
  cardAnnounced: {
    animation: "jk-card-played",
    durationMs: 300,
    testid: "hand-card-<instanceId> | hand-<side>",
    fx: { recipe: "cast" },
    target: (e, view) =>
      sideOf(view, e.player) === "you"
        ? (locateInstance(view, e.instanceId) ?? animTestid.hand("you"))
        : animTestid.hand("opponent"),
  },
  // B5 E1: the announced card fizzles out where it hung, then drops to the pile it went to.
  countered: {
    animation: "jk-exile-fade",
    durationMs: 450,
    testid: "hand-card-<instanceId> | graveyard-<side> | exile-<side> | hand-<side>",
    fx: { recipe: "fizzle" },
    target: (e, view) => {
      const side = sideOf(view, e.player);
      const pile =
        e.to === "exile" ? animTestid.exile(side) : e.to === "hand" ? animTestid.hand(side) : animTestid.graveyard(side);
      return instanceOrPile(view, e.instanceId, pile);
    },
  },
  // B5 E2, E16: the card slides across the center line into the thief's hand.
  stolen: {
    animation: "jk-cross-center",
    durationMs: 450,
    testid: "hand-<side>",
    fx: { recipe: "mindControl" },
    target: (e, view) => animTestid.hand(sideOf(view, e.to)),
  },
  // B5 E20: the chain over the zone opens.
  unlocked: {
    animation: "jk-chain-close",
    durationMs: 250,
    testid: "zone-<side>-<row>-<lane>",
    fx: { recipe: "lock" },
    target: (e, view) => zoneTestid(view, e.player, e.row, e.lane),
  },
  // B3.2, R384: the card flares as its ability is used.
  activated: {
    animation: "jk-icon-pop",
    durationMs: 300,
    testid: "card-<instanceId>",
    fx: { recipe: "cast" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // B3.1, R383: the backrow card rises into its unit zone and scales in there as a Unit.
  animated: {
    animation: "jk-summon-scale",
    durationMs: 350,
    testid: "zone-<side>-units-<unitLane>",
    fx: { recipe: "summon" },
    target: (e, view) => zoneTestid(view, e.player, "units", e.unitLane),
  },
  // B3.1, R383: the Unit sinks back into its backrow zone.
  deanimated: {
    animation: "jk-summon-scale",
    durationMs: 300,
    testid: "zone-<side>-backrow-<backrowLane>",
    fx: { recipe: "glint" },
    target: (e, view) => zoneTestid(view, e.player, "backrow", e.backrowLane),
  },
  // B3.3, R385: the card cracks and crumbles where it stands (a pile stands in off the board).
  crumbled: {
    animation: "jk-dissolve",
    durationMs: 400,
    testid: "card-<instanceId> | hand-card-<instanceId> | graveyard-<side>",
    fx: { recipe: "death" },
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.graveyard(sideOf(view, e.owner))),
  },
  // B3.4, R386: the changed numbers flash down.
  degraded: {
    animation: "jk-stat-tick",
    durationMs: 300,
    testid: "card-<instanceId> | hand-card-<instanceId>",
    fx: { recipe: "buff" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // B3.4, R386: the changed numbers flash up.
  upgraded: {
    animation: "jk-stat-tick",
    durationMs: 300,
    testid: "card-<instanceId> | hand-card-<instanceId>",
    fx: { recipe: "buff" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Classic+ #41: the number KY's Constant set flashes on the card.
  numberChanged: {
    animation: "jk-stat-tick",
    durationMs: 300,
    testid: "card-<instanceId> | hand-card-<instanceId>",
    fx: { recipe: "glint" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // B5 E9: the new target shakes as the hit, the attack or the pick swings onto it.
  redirected: {
    animation: "jk-snap-back",
    durationMs: 350,
    testid: "card-<toId> | hero-<side>",
    fx: { recipe: "lunge" },
    target: (e, view) => {
      const side = heroSide(view, e.toId);
      return side !== null ? testid.hero(side) : locateInstance(view, e.toId);
    },
  },
  // B5 E7: the hero's health number is struck and rewritten.
  healthSet: {
    animation: "jk-loss-pop",
    durationMs: 400,
    testid: "hero-<side>",
    fx: { recipe: "drain" },
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // Classic #90: the quest badge ticks.
  questProgressed: {
    animation: "jk-badge-tick",
    durationMs: 200,
    testid: "card-<instanceId> | backrow-<side>",
    fx: { recipe: "counter" },
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.backrow(sideOf(view, e.player))),
  },
  // Classic #90: the quest card pulses gold as a quest completes.
  questCompleted: {
    animation: "jk-radiant-pulse",
    durationMs: 500,
    testid: "card-<instanceId> | backrow-<side>",
    fx: { recipe: "radiant" },
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.backrow(sideOf(view, e.player))),
  },
  // Classic+ #35: the whole board rewinds.
  rolledBack: {
    animation: "jk-swap-cross",
    durationMs: 600,
    testid: "board",
    fx: { recipe: "rewind" },
    target: () => testid.board,
  },
  // R436: a slot-machine reveal over the board names each effect Call to Chaos rolled, one line per
  // effect, on both seats (fx/chaos.ts); the showcase's live region says it in words, and draws it
  // still where the effects layer draws nothing (game/showcase/ChaosBanner.tsx).
  chaosRolled: {
    animation: "jk-banner",
    durationMs: 900,
    testid: "turn-banner",
    fx: { recipe: "chaos" },
    target: () => testid.banner,
  },
  // B5 E22: the card blinks out and back in its zone.
  flickered: {
    animation: "jk-summon-scale",
    durationMs: 300,
    testid: "zone-<side>-<row>-<lane>",
    fx: { recipe: "void" },
    target: (e, view) => zoneTestid(view, e.player, e.row, e.lane),
  },
  // B5 E3: the deck shakes and gives nothing.
  drawLimited: {
    animation: "jk-fatigue",
    durationMs: 300,
    testid: "library-<side>",
    fx: { recipe: "fizzle" },
    target: (e, view) => animTestid.library(sideOf(view, e.player)),
  },
  // B5 E10: a banner says the turn was cut short.
  turnCutShort: {
    animation: "jk-banner",
    durationMs: 600,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // R437: the mark's sigil brands the card in the mark's colors (fx/brand.ts), and its aura stays
  // on the card while the view lists the mark (cards/CardMarks.tsx).
  marked: {
    animation: "jk-radiant-pulse",
    durationMs: 400,
    testid: "card-<instanceId>",
    fx: { recipe: "brand" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // R676: a Glitch's outcome is announced on the turn banner; the view after it (a new mulligan, the
  // swapped seat, the other games' boards or the voided result) is the rest of the news.
  glitched: {
    animation: "jk-banner",
    durationMs: 600,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // R983: a Feng Shui judged a play; the turn banner shows 吉 or 凶, the view after it the rewarded
  // or punished card.
  fengShui: {
    animation: "jk-banner",
    durationMs: 600,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // ME-CN, R1301: the card pulses as its words turn Chinese; the view after it draws them so. Only
  // the language changed, so no effect decorates it.
  translated: {
    animation: "jk-radiant-pulse",
    durationMs: 400,
    testid: "card-<instanceId>",
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // R800: a discard a discard guard stopped — the hand shakes once; nothing leaves it.
  discardPrevented: {
    animation: "jk-fatigue",
    durationMs: 300,
    testid: "hand-<side>",
    fx: { recipe: "fizzle" },
    target: (e, view) => animTestid.hand(sideOf(view, e.player)),
  },
  // R1361, R1363 (MN05): the Armor took the whole hit. The target braces with a steel glint and no
  // number, since nothing was dealt, and the effects layer blooms a shield over it (fx/shield.ts).
  damageAbsorbed: {
    animation: "jk-armor-absorb",
    durationMs: 300,
    testid: "card-<targetId> | hero-<side>",
    fx: { recipe: "armor" },
    target: (e, view) => {
      const side = heroSide(view, e.targetId);
      return side !== null ? testid.hero(side) : locateInstance(view, e.targetId);
    },
  },
  // R961 (Meditative #39.2): the Jade Counter rose; the hero's Jade badge ticks, no fx.
  jadeChanged: {
    animation: "jk-badge-tick",
    durationMs: 200,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // ME-SECRET (MB05, R860–R864): a secret is kept, revealed or judged. The badge by the hero
  // fades in or out — no number, no effects-layer work (FX "—"): the badge itself is the change.
  secretChosen: {
    animation: "jk-badge-fade",
    durationMs: 300,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  secretRevealed: {
    animation: "jk-badge-fade",
    durationMs: 400,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  predicted: {
    animation: "jk-badge-fade",
    durationMs: 400,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // R915004: a custom card's hook failed and did nothing. The card fades once where it stands, or its
  // controller's hero when the card is nowhere the viewer sees.
  scriptFailed: {
    animation: "jk-badge-fade",
    durationMs: 400,
    testid: "card-<instanceId>",
    target: (e, view) =>
      e.instanceId === null
        ? testid.hero(sideOf(view, e.player))
        : instanceOrPile(view, e.instanceId, testid.hero(sideOf(view, e.player))),
  },
  // C+ #62 KY's Papaya (R831003): the curve is drawn across the board, cell to cell in lane order,
  // before what it meets is exiled.
  curveDrawn: {
    animation: "jk-curve-drawn",
    durationMs: 700,
    testid: "board",
    fx: { recipe: "curve" },
    target: () => testid.board,
  },
  // R1183002: a banner says the AI plays the rest of the turn; the actions it takes follow as their
  // own events, each animated as a player's would be.
  autopilotEngaged: {
    animation: "jk-banner",
    durationMs: 600,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // R1839002: a Dormant card awakens where it lies; its Dormant badge falls away with the next view.
  awakened: {
    animation: "jk-radiant-pulse",
    durationMs: 400,
    testid: "card-<instanceId>",
    fx: { recipe: "brand" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // R1838001: a Trade. The trader's deck pulses as a card back goes from their hand into it; the
  // `shuffledIn` and `drawn` after it move the cards, backs for a seat that may not read them.
  traded: {
    animation: "jk-shuffle-in",
    durationMs: 300,
    testid: "library-<side>",
    fx: { recipe: "shuffle" },
    target: (e, view) => animTestid.library(sideOf(view, e.player)),
  },
  // R1840005: the Credits count ticks by the hero, as the Jade Counter's does.
  marketCreditsChanged: {
    animation: "jk-badge-tick",
    durationMs: 200,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // R1853003: a card bought at the Marketplace reaches its buyer's hand, and a card sold leaves it: the
  // hand's edge glints as for `addedToHand`, with no effect (FX "—"); the other seat's view names no
  // card bought.
  marketBought: {
    animation: "jk-hand-edge",
    durationMs: 250,
    testid: "hand-<side>",
    target: (e, view) => animTestid.hand(sideOf(view, e.player)),
  },
  marketSold: {
    animation: "jk-hand-edge",
    durationMs: 250,
    testid: "hand-<side>",
    target: (e, view) => animTestid.hand(sideOf(view, e.player)),
  },
  // R1852000: a dead piece's copy merges into its owner's Dormant Starship, which the `fused` before it
  // drew; the Starship's piece count rises with the next view.
  starshipFused: {
    animation: "jk-fuse-merge",
    durationMs: 400,
    testid: "card-<instanceId>",
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // R1852003: the Starship launches where it stands, its Dormant badge gone and its name on its face.
  starshipLaunched: {
    animation: "jk-radiant-pulse",
    durationMs: 600,
    testid: "card-<instanceId>",
    fx: { recipe: "brand" },
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Issue #1843 (R1843000): a card votes to remove a player; it flares where it stands, or the voted
  // player's backrow when the viewer cannot see it, and the log counts the tally.
  voteCast: {
    animation: "jk-radiant-pulse",
    durationMs: 400,
    testid: "card-<instanceId> | backrow-<side>",
    target: (e, view) => instanceOrPile(view, e.instanceId, animTestid.backrow(sideOf(view, e.player))),
  },
  // R1843000: the cards agree, and the banner says the player is voted out before the result.
  votedOut: {
    animation: "jk-banner",
    durationMs: 1200,
    testid: "turn-banner",
    fx: { recipe: "banner" },
    target: () => testid.banner,
  },
  // R1843002: the fleer's hero fades out of the universe under a banner, before the result.
  fled: {
    animation: "jk-exile-fade",
    durationMs: 900,
    testid: "hero-<side>",
    fx: { recipe: "banner" },
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // R1847001 (Stinkyscape #148): a player traveled one node of their starscape map; their hero ticks
  // as on `jadeChanged`, no fx, and the map redraws from the view.
  starscapeTravelled: {
    animation: "jk-badge-tick",
    durationMs: 200,
    testid: "hero-<side>",
    target: (e, view) => testid.hero(sideOf(view, e.player)),
  },
  // R1850003: a permanent goes beneath the card that holds it, fading off the board as an exiled card
  // does. Pre-update it is still on the board; the holder's own arrival, when it is new, is `summoned`.
  held: {
    animation: "jk-exile-fade",
    durationMs: 350,
    testid: "card-<instanceId>",
    target: (e, view) => locateInstance(view, e.instanceId),
  },
  // Issue #1855 (R1855000): an Anomaly replaces one side's board; the board tilts as for `swapped`,
  // before its cards' `exiled` and `summoned` play the cards going and coming.
  boardReplaced: {
    animation: "jk-swap-cross",
    durationMs: 500,
    testid: "board",
    target: () => testid.board,
  },
};

/**
 * Table's row for `type`, or `undefined` if unseen by this build (issue #769).
 * Total map over `GameEventType`.
 */
export function animationRow(type: string): AnimationRow | undefined {
  return (ANIMATIONS as unknown as Partial<Record<string, AnimationRow>>)[type];
}

// Durations and reduced motion

// settings/motion.ts holds these three so the router can read them (R2117000); this file's importers keep them.
export { REDUCED_MOTION_QUERY, prefersReducedMotion, reducedMotionNow } from "../settings/motion.ts";

/** `prefers-reduced-motion` collapses every duration to 0 (BUILD M5-T4); an unknown type has none (issue #769). */
export function durationFor(type: GameEventType, reducedMotion: boolean): number {
  return reducedMotion ? 0 : (animationRow(type)?.durationMs ?? 0);
}

/** Resolves one event's element against a view, without the caller narrowing the union itself. */
export function targetFor(event: GameEvent, view: PlayerView): string | null {
  const row = animationRow(event.type);
  if (row === undefined) return null;
  const resolve = row.target as (e: GameEvent, v: PlayerView) => string | null;
  return resolve(event, view);
}

// Planning: events → entries

/**
 * Issue #124: cast duration holding the board (C+ #47 Jogg's Box).
 * Each cast gets `CAST_BUDGET_MS` to prevent excessive burst compression.
 */
export const CAST_ENTRY_MS = 800;
export const CAST_BUDGET_MS = 2200;
/** Issue #124: a run of events that reached a whole Deck, Graveyard or Exile, played once. */
export const ZONE_IMPACT_MS = 700;
/** Issue #124: a run of hits or heals that swept a whole side, played as one roll of fog. */
export const SWEEP_MS = 600;

/**
 * One unit of motion: single event, collapsed pair (BUILD M5-T4), or cast/pile/sweep shapes (issue #124).
 */
export type AnimationEntry = {
  events: readonly GameEvent[];
  /** The type `data-animating` reports for this entry's leading element. */
  type: GameEventType;
  durationMs: number;
  /** Element testid → `data-animating` type. Collapsed pair carries hand card and landing zone. */
  frames: ReadonlyMap<string, GameEventType>;
  /** The view this entry was planned against: the board as it stood before its events. */
  view: PlayerView;
  /** Cast card metadata (§6.3 Cast, R97): casting card defId and 1-based ordinal. */
  cast?: { by: string; ordinal: number };
  /** A run of events that reached every card of one pile, played once on that pile. */
  zone?: ZoneImpact;
  /** A run of hits or heals that reached every unit of a side, played at once. */
  sweep?: Sweep;
  /** #185: a Unit the viewer can see lands in this entry, this hard (`slamEntries`). */
  slam?: EntrySlam;
};

/** #185: one landing's share of its action's slam (SLAM_BURST). */
export type EntrySlam = {
  instanceId: string;
  side: Side;
  /** The zone the Unit lands in, as the entry's board draws it (null when it draws none). */
  zone: string | null;
  tier: SlamTier;
  hitStopMs: number;
  shakePx: number;
  shakeMs: number;
  /** Inside the entry's `durationMs`: the Unit hangs this long before its landing motion starts. */
  anticipationMs: number;
};

/** The events that put a Unit on the field: a summon into a unit zone, a backrow card animating. */
export type LandingEvent = Extract<GameEvent, { type: "summoned" | "animated" }>;

/** #185: the tier a landing Unit slams at, or null when it slams not at all (a Unit the viewer cannot see). */
export type SlamResolver = (event: LandingEvent) => SlamTier | null;

/** The Unit an event lands, when it lands one the viewer may see (R97: never behind the sentinel). */
export function landingOf(event: GameEvent): LandingEvent | null {
  if (event.type === "animated") return event.defId === HIDDEN_ID ? null : event;
  if (event.type === "summoned") return event.row === "units" && event.defId !== HIDDEN_ID ? event : null;
  return null;
}

/**
 * #185: assigns landing slam from SLAM_BURST totals. Reduced motion drops anticipation and shake; hit-stop stays (#57).
 */
export function slamEntries(entries: readonly AnimationEntry[], slamOf: SlamResolver, reducedMotion: boolean): AnimationEntry[] {
  const left: { hitStopMs: number; shakePx: number; anticipationMs: number } = { ...SLAM_BURST };
  return entries.map((entry) => {
    for (const event of entry.events) {
      const landing = landingOf(event);
      const tier = landing === null ? null : slamOf(landing);
      if (landing === null || tier === null) continue;
      const feel = UNIT_SLAM[tier];
      const take = (want: number, key: keyof typeof left): number => {
        const got = Math.max(0, Math.min(want, left[key]));
        left[key] -= got;
        return got;
      };
      const slam: EntrySlam = {
        instanceId: landing.instanceId,
        side: sideOf(entry.view, landing.player),
        zone: targetFor(landing, entry.view),
        tier,
        hitStopMs: take(feel.hitStopMs, "hitStopMs"),
        shakePx: reducedMotion ? 0 : take(feel.shakePx, "shakePx"),
        shakeMs: reducedMotion ? 0 : feel.shakeMs,
        anticipationMs: reducedMotion || entry.durationMs <= 0 ? 0 : take(feel.anticipationMs, "anticipationMs"),
      };
      return { ...entry, slam, durationMs: entry.durationMs + slam.anticipationMs };
    }
    return entry;
  });
}

/** The pile element that stands in for a card the board does not draw (a library card, a graveyard card). */
export function pileTestid(pile: Pile): string {
  if (pile.pile === "library") return animTestid.library(pile.side);
  return pile.pile === "graveyard" ? animTestid.graveyard(pile.side) : animTestid.exile(pile.side);
}

/** Entries the burst budget never squeezes: each stands for many events, or holds a cast up to be read. */
function pinned(entry: AnimationEntry): boolean {
  return entry.cast !== undefined || entry.zone !== undefined || entry.sweep !== undefined;
}

function frameMap(entries: readonly (readonly [string | null, GameEventType])[]): Map<string, GameEventType> {
  const map = new Map<string, GameEventType>();
  for (const [id, type] of entries) {
    if (id !== null) map.set(id, type);
  }
  return map;
}

/** True when `summoned` is the same card's arrival for the `cardPlayed` immediately before it. */
function isSummonOfPlay(played: GameEvent, next: GameEvent | undefined): boolean {
  return (
    played.type === "cardPlayed" &&
    next !== undefined &&
    next.type === "summoned" &&
    next.instanceId === played.instanceId
  );
}

/** The two ids name the same card, R97's sentinel naming any. */
function sameCard(a: string, b: string): boolean {
  return a === b || a === HIDDEN_ID || b === HIDDEN_ID;
}

/** Where an event plays: its row's element or pile (runs.ts). */
function frameFor(event: GameEvent, view: PlayerView, plays: PlayTracker): string | null {
  const own = targetFor(event, view);
  if (own !== null) return own;
  const pile = pileOf(event, view, plays.current());
  return pile === null ? null : pileTestid(pile);
}

/**
 * What a cast's events mark: zone for summoned unit, and cast card only if drawn on board (never hand region).
 */
function castFrame(event: GameEvent, view: PlayerView): string | null {
  if (event.type === "summoned") return targetFor(event, view);
  return event.type === "cardPlayed" ? locateInstance(view, event.instanceId) : null;
}

/**
 * Turns event stream into entries, collapsing `cardPlayed` + `summoned` pairs (reduced motion zeroes durations).
 * Issue #124: sweeps, pile runs, and casts collapse into single entries.
 */
export function planEntries(
  events: readonly GameEvent[],
  view: PlayerView,
  reducedMotion: boolean,
  plays: PlayTracker = createPlayTracker(),
): AnimationEntry[] {
  const out: AnimationEntry[] = [];
  const timed = (ms: number): number => (reducedMotion ? 0 : ms);
  const take = (from: number, length: number): GameEvent[] => {
    const run = events.slice(from, from + length);
    for (const event of run) plays.see(event);
    return run;
  };
  let i = 0;
  while (i < events.length) {
    const event = events[i];
    if (event === undefined) {
      i += 1;
      continue;
    }

    // A sweep: every hit (or heal) of the run lands at once, under one roll of fog.
    const sweep = sweepAt(events, i, view);
    if (sweep !== null) {
      const run = take(i, sweep.length);
      out.push({
        events: run,
        type: event.type,
        durationMs: timed(SWEEP_MS),
        frames: frameMap(run.map((e) => [targetFor(e, view), e.type] as const)),
        view,
        sweep: sweep.sweep,
      });
      i += sweep.length;
      continue;
    }

    // #1443: one volley's same-source damage flies as one entry that starts together, behind
    // FX_FLAVOR_LIVE (off until part 8). Sweeps still win (checked first).
    if (FX_FLAVOR_LIVE) {
      const volley = volleyAt(events, i, view);
      if (volley !== null) {
        const run = take(i, volley.length);
        out.push({
          events: run,
          type: "damage",
          durationMs: durationFor("damage", reducedMotion),
          frames: frameMap(run.map((e) => [targetFor(e, view), e.type] as const)),
          view,
        });
        i += volley.length;
        continue;
      }
    }

    // A run that reached every card of one pile: one "affecting this zone" entry on that pile.
    const pileRun = pileRunAt(events, i, view, plays.current());
    if (pileRun !== null && pileRun.whole) {
      const run = take(i, pileRun.length);
      out.push({
        events: run,
        type: event.type,
        durationMs: timed(ZONE_IMPACT_MS),
        frames: frameMap([[pileTestid(pileRun.pile), event.type]]),
        view,
        zone: { ...pileRun.pile, count: pileRun.count, events: pileRun.length },
      });
      i += pileRun.length;
      continue;
    }

    // A cast: its announce, its `cardPlayed` and, for a Unit, the `summoned` that lands it. A cast card
    // comes from no hand the board draws (or from a pile), so only a card the board does draw marks.
    const played = event.type === "cardAnnounced" ? events[i + 1] : event;
    const casting =
      played?.type === "cardPlayed" &&
      played.costPaid === 0 &&
      plays.current() !== undefined &&
      (event.type === "cardPlayed" || (event.type === "cardAnnounced" && sameCard(event.instanceId, played.instanceId)));
    if (casting && played.type === "cardPlayed") {
      const at = event === played ? i : i + 1;
      const landed = events[at + 1];
      const length = at - i + 1 + (isSummonOfPlay(played, landed) ? 1 : 0);
      const run = events.slice(i, i + length);
      let cast: { by: string; ordinal: number } | null = null;
      for (const seen of run) {
        const made = plays.see(seen);
        if (made !== null) cast = { by: made.by.defId, ordinal: made.ordinal };
      }
      out.push({
        events: run,
        type: "cardPlayed",
        durationMs: timed(CAST_ENTRY_MS),
        frames: frameMap(run.map((e) => [castFrame(e, view), e.type] as const)),
        view,
        cast: cast ?? { by: HIDDEN_ID, ordinal: 1 },
      });
      i += length;
      continue;
    }

    const next = events[i + 1];
    if (isSummonOfPlay(event, next) && next !== undefined) {
      const frames = frameMap([
        [frameFor(event, view, plays), event.type],
        [frameFor(next, view, plays), next.type],
      ]);
      take(i, 2);
      out.push({
        events: [event, next],
        type: event.type,
        // The pair's duration is the `cardPlayed` duration: one motion, one span of time.
        durationMs: durationFor(event.type, reducedMotion),
        frames,
        view,
      });
      i += 2;
      continue;
    }
    const frames = frameMap([[frameFor(event, view, plays), event.type]]);
    take(i, 1);
    out.push({
      events: [event],
      type: event.type,
      durationMs: durationFor(event.type, reducedMotion),
      frames,
      view,
    });
    i += 1;
  }
  return out;
}

/* ------------------------------------------------------------------------------------------- *
 * The runner
 * ------------------------------------------------------------------------------------------- */

export type AnimationQueue = {
  /** Plans `events` against `view` and appends them; starts the pump if it is idle. */
  enqueue(events: readonly GameEvent[], view: PlayerView): void;
  /** Elements currently mid-animation, for `data-animating` (M5-T4). */
  animating(): AnimatingMap;
  /** The entry in flight, or `null`. */
  inFlight(): AnimationEntry | null;
  /** Entries still waiting behind the one in flight. */
  pending(): number;
  idle(): boolean;
  /** Finishes remaining entries immediately and fires onSettled. */
  drain(): void;
  /** Same as `drain`, without firing `onSettled` — for tearing the queue down. */
  reset(): void;
  /** Called on every change to `animating()`; returns its own unsubscribe. */
  subscribe(listener: () => void): () => void;
  /** Lifecycle signals for the effects layer. Synchronous; returns its own unsubscribe. */
  subscribeSignals(listener: (signal: RunnerSignal) => void): () => void;
};

/** Effects layer signals from the runner (docs/polish/1-animations.md, S4, R200). */
export type RunnerSignal =
  | { kind: "start"; entry: AnimationEntry } // an entry went in flight (durationMs > 0 only)
  | { kind: "idle" } // emitted each time the pump fires onSettled
  /** drain() ran; entries cut short in order. */
  | { kind: "drain"; entries: readonly AnimationEntry[] }
  | { kind: "reset" }; // reset() ran

export type AnimationQueueOptions = {
  now?: () => number;
  /** Injected timer, so tests are synchronous and never race a real clock. */
  schedule?: (fn: () => void, ms: number) => void;
  /** Defaults to `prefersReducedMotion()` at construction. */
  reducedMotion?: boolean;
  /** Fired once each time the queue goes from busy to empty. */
  onSettled?: () => void;
  /** Overrides for the burst budget below; the defaults are what the client ships with. */
  burstBudgetMs?: number;
  /** A cast's own segment of the budget (issue #124, `CAST_BUDGET_MS`). */
  castBudgetMs?: number;
  minEntryMs?: number;
  /** Read at every enqueue (R201). Defaults to `getFxSettings`. */
  settings?: () => Pick<FxSettings, "speed" | "motion">;
  /** #185: the tier a landing Unit slams at, read at every enqueue. Absent: nothing slams. */
  slamOf?: SlamResolver;
};

const EMPTY_ANIMATING: AnimatingMap = new Map<string, GameEventType>();

/**
 * Action animation budget and floor (R82). Bursts past the budget play proportionally faster
 * so clicks do not freeze the game.
 */
export const BURST_BUDGET_MS = 2_400;
export const MIN_ENTRY_MS = 120;

/**
 * R201: effects speed divides non-zero duration (floored at `MIN_ENTRY_MS`), clamped to speed bounds.
 */
export function scaleForSpeed(durationMs: number, speed: number): number {
  const s = normalizeSpeed(speed);
  if (durationMs <= 0 || s === 1) return durationMs;
  return Math.max(MIN_ENTRY_MS, Math.round(durationMs / s));
}

/** R97's sentinel: what a redacted event names in place of a card (engine `viewFor.ts` `HIDDEN_ID`). */
export const HIDDEN_ID = "hidden";

/**
 * Fields rewritten with card identity when hidden (R97, R227, R177, R385, R386, R448, R316, R1853003, R317, R315).
 * Missing types cause mismatch in `newEventsSince`, re-animating the whole window.
 */
const REWRITTEN_WITH_IDENTITY: Partial<Record<GameEventType, readonly string[]>> = {
  cardResolved: ["radiant"],
  cardPlayed: ["formerId"],
  summoned: ["formerId"],
  controlChanged: ["formerId"],
  buffed: ["attack", "health"],
  costChanged: ["cost"],
  counterChanged: ["value"],
  degraded: ["change"],
  upgraded: ["change"],
  numberChanged: ["value"],
  cardAnnounced: ["cardType"],
  radiantSet: ["zone"],
  deradianted: ["zone"],
  libraryOverflow: ["radiant"],
  marketBought: ["radiant"],
};

const NOTHING_REWRITTEN: readonly string[] = [];

/**
 * Whether two copies of an event are the same occurrence (R97 redaction).
 * Sentinel matches any ID; fields rewritten with identity are ignored when comparing redacted copies.
 */
export function sameOccurrence(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a === "string" && typeof b === "string") return a === HIDDEN_ID || b === HIDDEN_ID;
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((item, i) => sameOccurrence(item, b[i]));
  }
  if (!isRecord(a) || !isRecord(b)) return false;
  const redacted = a["instanceId"] === HIDDEN_ID || b["instanceId"] === HIDDEN_ID;
  const rewritten = redacted ? (REWRITTEN_WITH_IDENTITY[a["type"] as GameEventType] ?? NOTHING_REWRITTEN) : NOTHING_REWRITTEN;
  for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
    if (rewritten.includes(key)) continue;
    const left = a[key];
    const right = b[key];
    if (left === undefined && right === undefined) continue;
    if (left === undefined || right === undefined || !sameOccurrence(left, right)) return false;
  }
  return true;
}

/**
 * Events in `next` not yet given to runner. SPEC §10.8 sliding window of last N events.
 * Compares shared events with `sameOccurrence` (R97 redaction).
 */
export function newEventsSince(prev: readonly GameEvent[], next: readonly GameEvent[]): GameEvent[] {
  if (prev.length === 0 || next.length === 0) return [...next];
  const most = Math.min(prev.length, next.length);
  for (let overlap = most; overlap > 0; overlap -= 1) {
    const from = prev.length - overlap;
    let matches = true;
    for (let i = 0; i < overlap && matches; i += 1) matches = sameOccurrence(prev[from + i], next[i]);
    if (matches) return next.slice(overlap);
  }
  return [...next];
}

export function createAnimationQueue(options: AnimationQueueOptions = {}): AnimationQueue {
  const schedule =
    options.schedule ??
    ((fn: () => void, ms: number): void => {
      setTimeout(fn, ms);
    });
  const reducedMotion = options.reducedMotion ?? prefersReducedMotion();
  const readSettings = options.settings ?? getFxSettings;
  const onSettled = options.onSettled;
  const burstBudgetMs = options.burstBudgetMs ?? BURST_BUDGET_MS;
  const castBudgetMs = options.castBudgetMs ?? CAST_BUDGET_MS;
  const minEntryMs = options.minEntryMs ?? MIN_ENTRY_MS;
  // `options.now` is accepted for parity with the client's other injected clocks and is
  // deliberately unread: `schedule` owns every deadline, so the runner keeps no timestamps and
  // stays free of a clock it would have to mock.

  const queue: AnimationEntry[] = [];
  let current: AnimationEntry | null = null;
  /** The plays still resolving, across batches (issue #124): a cast can come in the batch after a prompt. */
  const plays = createPlayTracker();
  /** Bumped by `drain`/`reset` so a timer already in flight cannot resurrect a cleared queue. */
  let epoch = 0;
  let owed = false;
  const listeners = new Set<() => void>();
  const signalListeners = new Set<(signal: RunnerSignal) => void>();

  function notify(): void {
    for (const listener of [...listeners]) listener();
  }

  function signal(value: RunnerSignal): void {
    for (const listener of [...signalListeners]) listener(value);
  }

  /** Fires `onSettled` if a batch is owed one; reports whether it did. */
  function settle(): boolean {
    if (!owed) return false;
    owed = false;
    onSettled?.();
    return true;
  }

  /** Squeezes queue[from..to) so its entries fit budgetMs (pinned entries keep time). */
  function squeeze(from: number, to: number, budgetMs: number): void {
    let fixed = 0;
    let total = 0;
    for (let i = from; i < to; i += 1) {
      const entry = queue[i];
      if (entry === undefined) continue;
      if (pinned(entry)) fixed += entry.durationMs;
      else total += entry.durationMs;
    }
    const room = Math.max(0, budgetMs - fixed);
    if (total <= room) return;
    const factor = room / total;
    for (let i = from; i < to; i += 1) {
      const entry = queue[i];
      if (entry === undefined || entry.durationMs <= 0 || pinned(entry)) continue;
      queue[i] = { ...entry, durationMs: Math.max(minEntryMs, Math.round(entry.durationMs * factor)) };
    }
  }

  /** Squeeze waiting backlog to burst budget (#124: casts get castBudgetMs). */
  function fitBudget(budgetMs: number, castBudgetMs: number): void {
    let start = 0;
    let budget = budgetMs;
    if (queue[0]?.cast !== undefined) budget = castBudgetMs;
    else if (current?.cast !== undefined) budget = Math.max(0, castBudgetMs - current.durationMs);
    for (let k = 0; k < queue.length; k += 1) {
      if (queue[k]?.cast === undefined || k === start) continue;
      squeeze(start, k, budget);
      start = k;
      budget = castBudgetMs;
    }
    squeeze(start, queue.length, budget);
  }

  function pump(): void {
    // Zero-duration entries (every entry under reduced motion, and `gameOver` always) are run
    // straight through in this loop: no timer is scheduled, so a full game's event queue drains
    // synchronously inside `enqueue` (BUILD M5-T4 acceptance).
    for (;;) {
      if (current !== null) return;
      const next = queue.shift();
      if (next === undefined) {
        notify();
        if (settle()) signal({ kind: "idle" });
        return;
      }
      if (next.durationMs <= 0) continue;
      current = next;
      notify();
      // Captured before the signal, so a listener that drains or resets inside `start` leaves this
      // entry's timer a no-op instead of ending whatever runs next.
      const mine = epoch;
      signal({ kind: "start", entry: next });
      schedule(() => {
        if (mine !== epoch) return;
        current = null;
        pump();
      }, next.durationMs);
      return;
    }
  }

  return {
    enqueue(events, view) {
      // Read live, so a change in the settings panel applies from the next action on (R201).
      const settings = readSettings();
      const reduced = reducedMotion || settings.motion === "reduce" || readPanelSettings().reduceMotion;
      const planned = planEntries(events, view, reduced, plays);
      const slammed = options.slamOf === undefined ? planned : slamEntries(planned, options.slamOf, reduced);
      const entries = slammed.map((entry) => {
        const durationMs = scaleForSpeed(entry.durationMs, settings.speed);
        return durationMs === entry.durationMs ? entry : { ...entry, durationMs };
      });
      if (entries.length > 0) queue.push(...entries);
      owed = true;
      const speed = normalizeSpeed(settings.speed);
      fitBudget(burstBudgetMs / speed, castBudgetMs / speed);
      pump();
    },
    animating() {
      return current === null ? EMPTY_ANIMATING : current.frames;
    },
    inFlight() {
      return current;
    },
    pending() {
      return queue.length;
    },
    idle() {
      return current === null && queue.length === 0;
    },
    drain() {
      epoch += 1;
      const cut = current === null ? [...queue] : [current, ...queue];
      queue.length = 0;
      current = null;
      notify();
      settle();
      signal({ kind: "drain", entries: cut });
    },
    reset() {
      epoch += 1;
      queue.length = 0;
      current = null;
      owed = false;
      plays.clear();
      notify();
      signal({ kind: "reset" });
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    subscribeSignals(listener) {
      signalListeners.add(listener);
      return () => {
        signalListeners.delete(listener);
      };
    },
  };
}

/**
 * Findings against SPEC §10.3 / §10.8: payloads that cannot name their own element fall back
 * to region targets (e.g. backrow, board) or parse target prefixes.
 */
