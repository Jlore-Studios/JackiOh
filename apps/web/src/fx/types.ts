// The effects layer's contract (docs/polish/1-animations.md, Surface S1). Every other fx module
// compiles against these types, so they are written exactly as the design document states them.

import type { CardType, GameEvent, PlayerId, PlayerView, Rarity, Row, Tag } from "@jackioh/shared";
import type { Side } from "../game/contract.ts";

/** A point inside a box as fractions of its width and height; {x:0.5,y:0.5} is the centre. */
export type FxPoint = { x: number; y: number };

/** A position in viewport CSS pixels. */
export type FxVec = { x: number; y: number };

/** A box in viewport CSS pixels (the space `getBoundingClientRect` reports). */
export type FxBox = { x: number; y: number; width: number; height: number };

/**
 * Where a cue plays. Resolved to an `FxBox` by the director at the moment the cue fires.
 *
 * `handCard` is one card of a hand as the board draws it: the `pick`-th `.card` in `hand-<side>`,
 * counted modulo the cards there, falling back to the hand's own box when it holds none. It exists for
 * a hand of backs (R202): the planner picks a back by a counter of its own, never by the hidden card
 * an event names, so the back it lands on says nothing about which card it was.
 */
export type FxAnchor =
  | { kind: "testid"; testid: string; at?: FxPoint }
  | { kind: "crystal"; side: Side; index: number }
  | { kind: "handCard"; side: Side; pick: number }
  | { kind: "viewport"; at: FxPoint };

export type FxPreset =
  | "fire"
  | "ember"
  | "holy"
  | "sparkle"
  | "arcane"
  | "poison"
  | "smoke"
  | "dust"
  | "shard"
  | "spark"
  | "gold"
  | "prismatic"
  | "void"
  | "confetti"
  | "blood"
  | "frost";

export type FxSpread = "point" | "area" | "ring";
export type FxSplatTone = "damage" | "heal" | "loss";
export type FxRayTone = "legendary" | "mythic" | "radiant" | "holy" | "victory";
export type FxBannerTone = "you" | "opponent" | "muted";
export type FxOutcome = "victory" | "defeat" | "draw";

/**
 * `scale` (issue #124) multiplies every particle's size, as `power` multiplies its speed, so one preset
 * gives a fine mist or a heavy spray. Absent: 1.
 */
export type FxBurstCue = {
  kind: "burst";
  preset: FxPreset;
  at: FxAnchor;
  delayMs: number;
  count: number;
  spread: FxSpread;
  power: number;
  scale?: number;
};
/** `density` is the intensity scale: it multiplies the trail and the arrival burst, as `count` does a burst's. */
export type FxProjectileCue = { kind: "projectile"; preset: FxPreset; from: FxAnchor; to: FxAnchor; delayMs: number; flightMs: number; density: number };
export type FxCrackCue = { kind: "crack"; at: FxAnchor; delayMs: number; durationMs: number };
export type FxRingCue = { kind: "ring"; preset: FxPreset; at: FxAnchor; delayMs: number; durationMs: number };
export type FxShakeCue = { kind: "shake"; trauma: number; delayMs: number };
/** `amount` is the event's positive amount; the tone gives the sign ("−" for damage and loss, "+" for heal). */
export type FxSplatCue = { kind: "splat"; tone: FxSplatTone; amount: number; at: FxAnchor; delayMs: number; durationMs: number };
export type FxRaysCue = { kind: "rays"; tone: FxRayTone; at: FxAnchor; delayMs: number; durationMs: number };
export type FxSheenCue = { kind: "sheen"; at: FxAnchor; delayMs: number; durationMs: number };
/** A card BACK flying between two anchors. It never carries a card identity (R202). */
export type FxGhostCue = { kind: "ghost"; from: FxAnchor; to: FxAnchor; delayMs: number; durationMs: number };
export type FxArrowsCue = { kind: "arrows"; direction: "up" | "down"; at: FxAnchor; delayMs: number; durationMs: number };
export type FxBannerCue = { kind: "banner"; text: string; tone: FxBannerTone; delayMs: number; durationMs: number };
export type FxResultCue = { kind: "result"; outcome: FxOutcome; text: string; delayMs: number; durationMs: number };
/**
 * R502: one mana crystal cracking and going dark, with frost over it (#21 Hinder's refresh loss). It
 * sits on the crystal's own box; the lasting mark after it is the board's (`manaMarks.ts`).
 */
export type FxFractureCue = { kind: "fracture"; at: FxAnchor; delayMs: number; durationMs: number };
/**
 * Classic+ #24 Crushing Walls: two spiked walls closing in from the left and right edges of `at`'s
 * box, each `reach` of its width deep, then sliding back out (fx.css draws them).
 */
export type FxWallsCue = { kind: "walls"; at: FxAnchor; reach: number; delayMs: number; durationMs: number };
/** A colour set for a DOM cue, as CSS colours: the rim, the bright core and the glow around it. */
export type FxTint = { rim: string; core: string; glow: string };
/** R437: a mark branded onto a card: a sigil in the mark's colours slams on and fades into the aura. */
export type FxBrandCue = { kind: "brand"; at: FxAnchor; tint: FxTint; delayMs: number; durationMs: number };
/**
 * One line of a Call to Chaos reveal: a reel of effect names that spins and lands on `text`, the
 * last name in `reel`, at `landMs` after the cue fires.
 */
export type FxChaosLine = { text: string; reel: readonly string[]; landMs: number };
/** R436: the slot-machine reveal of the effects a Call to Chaos rolled, one line each, over the board. */
export type FxChaosCue = { kind: "chaos"; title: string; lines: readonly FxChaosLine[]; delayMs: number; durationMs: number };
/** A small glyph a DOM cue draws (an emblem of `cards/art/emblems.ts`): SVG path data in a 24×24 box. */
export type FxIcon = { d: string; rule: "nonzero" | "evenodd" };
/**
 * Issue #124: a sweep's fog. A bank of cloud rolls over a whole row of units, left to right across the
 * boxes from `from` to `to` (the row's first and last unit zones), in `tint`, which says what cast it,
 * with small `icon`s drifting in it (the caster's emblem, or a flame or a heart). `tone` is hits or heals.
 */
export type FxFogCue = {
  kind: "fog";
  tone: "damage" | "heal";
  from: FxAnchor;
  to: FxAnchor;
  tint: FxTint;
  icon: FxIcon | null;
  delayMs: number;
  durationMs: number;
};
/**
 * Issue #124: a whole Deck, Graveyard or Exile reached at once. A wave in `tint` washes over the pile
 * once, pushing `direction` (a Degrade down, an Upgrade up), and `text` says how much it reached.
 */
export type FxZoneCue = {
  kind: "zone";
  at: FxAnchor;
  tint: FxTint;
  text: string;
  direction: "up" | "down" | "none";
  delayMs: number;
  durationMs: number;
};

/**
 * Stage cues (docs/polish/1-animations.md, B46–B48): they act on the board's own elements rather than
 * drawing over them, and they are planned by `stage.ts`, not by the S7 recipe table.
 *
 * A stand-in that carries a card to the zone the next view shows it in (a summoned unit, a stolen
 * one) and holds it there until the board catches up, because the board keeps showing the view from
 * before the burst until the whole burst has played (BUILD M5-T4). `from` is the card element it
 * takes off (a hand card, a board card, or a hand of backs, whose last back it copies); without one
 * the stand-in is a card-shaped light. It lands at `landMs`. It is removed the moment the board
 * shows the next view; `durationMs` (FX_HOLD_MAX_MS) is only the safety cap (R200).
 */
export type FxHoldCue = { kind: "hold"; from: FxAnchor | null; to: FxAnchor; delayMs: number; landMs: number; durationMs: number };
/**
 * Hides a card the burst has already taken away (played, destroyed, bounced, stolen) until the board
 * shows the view it is gone from. "now" hides it at once (a stand-in has taken its place); "after"
 * hides it once its own motion has ended, so its keyframes still play.
 */
export type FxConcealCue = { kind: "conceal"; testid: string; mode: "now" | "after"; delayMs: number; durationMs: number };
/** Aims an attacker's lunge at the element it attacks, for the attackDeclared entry it rides. */
export type FxLungeCue = { kind: "lunge"; attacker: string; target: string; delayMs: number; durationMs: number };

export type FxCanvasCue = FxBurstCue | FxProjectileCue | FxCrackCue | FxRingCue;
export type FxDomCue =
  | FxSplatCue
  | FxRaysCue
  | FxSheenCue
  | FxGhostCue
  | FxArrowsCue
  | FxBannerCue
  | FxResultCue
  | FxFractureCue
  | FxWallsCue
  | FxBrandCue
  | FxChaosCue
  | FxFogCue
  | FxZoneCue;
export type FxStageCue = FxHoldCue | FxConcealCue | FxLungeCue;
export type FxCue = FxCanvasCue | FxShakeCue | FxDomCue | FxStageCue;

export type FxRecipe =
  | "cast"
  | "summon"
  | "impact"
  | "drain"
  | "heal"
  | "shieldBreak"
  | "death"
  | "void"
  | "bounce"
  | "burn"
  | "discard"
  | "draw"
  | "handGlint"
  | "shuffle"
  | "buff"
  | "keyword"
  | "counter"
  | "glint"
  | "radiant"
  | "smoke"
  | "fuse"
  | "mindControl"
  | "lock"
  | "trap"
  | "lunge"
  | "fizzle"
  | "mana"
  | "banner"
  | "fatigue"
  | "overflow"
  | "chaos"
  | "brand"
  | "rewind";

/** The optional `fx` field of an `ANIMATIONS` row: which recipe decorates the event. Data only. */
export type FxDescriptor = { readonly recipe: FxRecipe };

/**
 * Public catalog facts about a readable defId (base face). `undefined` for "hidden" or unknown ids.
 * `type` and `tags` (issue #124) pick the card's look (`looks.ts`): its particles, tint and emblem.
 */
export type FxCardFacts = { rarity?: Rarity; attack?: number; health?: number; type?: CardType; tags?: readonly Tag[] };

export type FxTrapZone = { player: PlayerId; row: Row; lane: number };

/**
 * A play the planner has seen start and not yet seen finish: the `cardPlayed` it began with, and
 * whether it was cast the moment it was drawn (R502, `castOnDraw.ts`). Its `defId` is the sentinel
 * when the viewer may not read the card (R97), and then no per-card recipe ever keys off it (R202).
 */
export type FxPlay = {
  player: PlayerId;
  instanceId: string;
  defId: string;
  castOnDraw: boolean;
  /**
   * How many events the planner has seen since this play's `cardPlayed` (its next event is step 1).
   * A count of events, public on both seats, so a recipe may use it to vary what it draws without
   * reading anything a hidden card would change (R202).
   */
  step: number;
  /**
   * Those events by type: how many of each the planner has seen since the `cardPlayed`, the event
   * being planned included. Types and counts only, public on both seats (R202). Classic+ #24
   * Crushing Walls draws its walls at the first `destroyed` of its own play.
   */
  seen: Readonly<Partial<Record<GameEvent["type"], number>>>;
};

/** What the planner remembers across entries of one mount (who cast what, where a trap fired). */
export type FxMemory = {
  /**
   * Records `cardPlayed` (instanceId → player) and `trapFired` (instanceId → zone), ignoring "hidden"
   * ids; keeps the last few events in order, so a `cardPlayed` right after its own `drawn` reads as a
   * cast on draw; and keeps the plays still resolving (`cardPlayed` opens one, its `cardResolved` or
   * `countered` closes it).
   */
  remember(events: readonly GameEvent[]): void;
  casterOf(instanceId: string): PlayerId | undefined;
  trapZoneOf(instanceId: string): FxTrapZone | undefined;
  /** The innermost play still resolving, or undefined. */
  resolving(): FxPlay | undefined;
  /** R502: whether this very `cardPlayed` (the object remembered) was a cast on draw. */
  castOnDraw(event: GameEvent): boolean;
  clear(): void;
};

export type FxPlanEnv = {
  /** `FX_INTENSITY_SCALE[settings.intensity]`; multiplies burst counts and trauma; 0 plans nothing. */
  intensity: number;
  card: (defId: string) => FxCardFacts | undefined;
  memory: FxMemory;
  /**
   * The newest view the layer has been given (Game's `view`, the one the burst is heading to), when
   * it has one. A number an event does not carry but the view does is read here: how far #21 Hinder
   * lowered the next refresh (R502). Absent: the planner plans without it.
   */
  next?: PlayerView;
};

export type FxFrameSource = { request(callback: (timestampMs: number) => void): number; cancel(handle: number): void };
export type FxVisibility = { hidden(): boolean; subscribe(listener: () => void): () => void };
export type FxShakeOffset = { x: number; y: number; angle: number };
/** Where the shake goes. The default writes CSS `translate`/`rotate` on the board (S8). */
export type FxShakeSink = { apply(offset: FxShakeOffset): void; clear(): void };
