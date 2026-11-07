// The effects layer's contract (docs/polish/1-animations.md, Surface S1).

import type { CardType, GameEvent, KeywordKind, PlayerId, PlayerView, Rarity, Row, Tag } from "@jackioh/shared";
import type { Side } from "../game/contract.ts";
import type { FxCardHook } from "./motifs/names.ts";

/** A point in fractions of a box's width and height. */
export type FxPoint = { x: number; y: number };

export type FxVec = { x: number; y: number };

export type FxBox = { x: number; y: number; width: number; height: number };

/** Where a cue plays. `handCard` uses a counter, never an event's hidden card (R202). */
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
  | "frost"
  | "leaf"
  | "circuit"
  | "crumb"
  | "star"
  | "exhaust";

export type FxSpread = "point" | "area" | "ring";
/** Issue #2790: `absorb` is Armor's grey splat of the amount it took whole (R1361). */
export type FxSplatTone = "damage" | "heal" | "loss" | "absorb";
export type FxRayTone = "legendary" | "mythic" | "radiant" | "holy" | "victory";
export type FxBannerTone = "you" | "opponent" | "muted";
export type FxOutcome = "victory" | "defeat" | "draw";

/** `scale` multiplies particle size as `power` does speed (absent: 1). `colors` (#1447, hot first) replace the preset's palette. */
export type FxBurstCue = {
  kind: "burst";
  preset: FxPreset;
  at: FxAnchor;
  delayMs: number;
  count: number;
  spread: FxSpread;
  power: number;
  scale?: number;
  colors?: readonly string[];
};
/** `density` scales the trail and the arrival burst, as `count` does a burst's. */
export type FxProjectileCue = { kind: "projectile"; preset: FxPreset; from: FxAnchor; to: FxAnchor; delayMs: number; flightMs: number; density: number };
/** #1442: what a flavor's shot throws; `shots.ts` draws each with paths, never an image (B27). */
export type FxShotSprite = "arrow" | "boulder" | "book" | "seed" | "goo" | "snack" | "coin" | "bill" | "heart" | "emoji";
/** #1442: a flavor's projectile: a lobbed `arc` of sprites, a glowing straight `bolt`, an instant `beam` or a claw-slash `swipe`. */
export type FxShot =
  | {
      motion: "arc";
      sprites: readonly FxShotSprite[];
      turn: "point" | "spin" | "float";
      count: number;
      trail?: FxPreset;
      scatter?: true;
      shake?: true;
    }
  | { motion: "bolt"; trail: FxPreset; wave?: true }
  | { motion: "beam" }
  | { motion: "swipe" };
export type FxShotMotion = FxShot["motion"];
/** #1442: a flavor's shot in `colors` (hot first), bursting `preset` where it lands; `density` as the projectile's. */
export type FxShotCue = {
  kind: "shot";
  shot: FxShot;
  colors: readonly string[];
  preset: FxPreset;
  from: FxAnchor;
  to: FxAnchor;
  delayMs: number;
  flightMs: number;
  density: number;
};
export type FxCrackCue = { kind: "crack"; at: FxAnchor; delayMs: number; durationMs: number };
export type FxRingCue = { kind: "ring"; preset: FxPreset; at: FxAnchor; delayMs: number; durationMs: number };
export type FxShakeCue = { kind: "shake"; trauma: number; delayMs: number };
/** `amount` is the event's positive amount; the tone gives the sign ("−" for damage, loss and absorb, "+" for heal). */
export type FxSplatCue = { kind: "splat"; tone: FxSplatTone; amount: number; at: FxAnchor; delayMs: number; durationMs: number };
export type FxRaysCue = { kind: "rays"; tone: FxRayTone; at: FxAnchor; delayMs: number; durationMs: number };
export type FxSheenCue = { kind: "sheen"; at: FxAnchor; delayMs: number; durationMs: number };
/** A card BACK flying between two anchors. It never carries a card identity (R202). */
export type FxGhostCue = { kind: "ghost"; from: FxAnchor; to: FxAnchor; delayMs: number; durationMs: number };
/**
 * R2789000: a combat keyword's icon popping over `at` for `durationMs` (the shared callout of the keyword
 * beats, fx/motifs/callout.ts). `still` draws it as a static icon, for reduced motion.
 */
export type FxCalloutCue = { kind: "callout"; keyword: KeywordKind; at: FxAnchor; delayMs: number; durationMs: number; still?: true };
export type FxArrowsCue = { kind: "arrows"; direction: "up" | "down"; at: FxAnchor; delayMs: number; durationMs: number };
export type FxBannerCue = { kind: "banner"; text: string; tone: FxBannerTone; delayMs: number; durationMs: number };
export type FxResultCue = { kind: "result"; outcome: FxOutcome; text: string; delayMs: number; durationMs: number };
/** R502: a mana crystal cracking under #21 Hinder's refresh loss; the lasting mark is `manaMarks.ts`'s. */
export type FxFractureCue = { kind: "fracture"; at: FxAnchor; delayMs: number; durationMs: number };
/** Classic+ #24 Crushing Walls: walls close in from `at`'s left and right edges, each `reach` of its width deep. */
export type FxWallsCue = { kind: "walls"; at: FxAnchor; reach: number; delayMs: number; durationMs: number };
export type FxTint = { rim: string; core: string; glow: string };
/** R437: a mark's sigil branded onto a card (brand.ts). */
export type FxBrandCue = { kind: "brand"; at: FxAnchor; tint: FxTint; delayMs: number; durationMs: number };
/** R1363 (MN05): Armor taking a hit: `small` for half or more of a hit that still landed, `full` for the whole hit. */
export type FxShieldCue = { kind: "shield"; size: "small" | "full"; at: FxAnchor; delayMs: number; durationMs: number };
export type FxChaosLine = { text: string; reel: readonly string[]; landMs: number };
/** R436: the slot-machine reveal of what a Call to Chaos rolled (chaos.ts). */
export type FxChaosCue = { kind: "chaos"; title: string; lines: readonly FxChaosLine[]; delayMs: number; durationMs: number };
/**
 * R2652001: one coin or die of a batch. `faces` is every face it drew, `kept` the index of the one it
 * kept (Lucky's extra draws are the others), `tumble` the faces its reel shows before it lands on the
 * kept one (that face last), each as the words it shows, and `landMs` when it lands inside the entry.
 */
export type FxRollItem = { faces: readonly string[]; kept: number; tumble: readonly string[]; landMs: number };
/** R2652001: one panel over the board for a batch of coin flips or die rolls, every item at once. */
export type FxRollCue = {
  kind: "roll";
  title: string;
  die: "coin" | "die";
  items: readonly FxRollItem[];
  delayMs: number;
  durationMs: number;
  /** R3338000: Dice of Doom's d100, a big die tumbling across the board into the panel's place. */
  look?: "doom";
};
/** A small glyph a DOM cue draws (an emblem of `cards/art/emblems.ts`): SVG path data in a 24×24 box. */
export type FxIcon = { d: string; rule: "nonzero" | "evenodd" };
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
 * R2722001: a Mythic's takeover, its look a marquee card's own or the shared prismatic one. The board
 * darkens, a card-shaped silhouette (never a face, R202) sweeps across it into `at`, light rays turn
 * and a shockwave crosses both sides when it lands at `landMs`, all inside the entry and gone within
 * the effects' tail (R200).
 */
export type TakeoverLook = "prismatic" | "void" | "pawn" | "genie" | "ai" | "rotunda" | "forge";
export type FxTakeoverCue = { kind: "takeover"; look: TakeoverLook; tint: FxTint; at: FxAnchor; landMs: number; delayMs: number; durationMs: number };
/**
 * R2800000: a board clear's set piece over the whole layer, centred on `at`: a `vortex` (the board
 * twisting into a spiral that swallows what the wipe takes) or a `fold` (its edges folding inward into
 * black, the light coming back at `at`). `holdMs` is when the cards are swallowed, `spin` which way the
 * spiral turns, and `side` the one side it darkens (null: both). It names no card (R202).
 */
export type SetPieceShape = "vortex" | "fold" | "rift" | "lamp" | "chalk" | "bend" | "freeze" | "abyss" | "crunch" | "doom";
/**
 * R3338000: the Terminus cards' shapes join them, each its card's own (fx/terminus/): `beats` are the
 * moments inside the cue its parts mark one by one (ms from its start: a star lighting, a lane chaining
 * shut), and `still` is the one frame Reduce Motion shows, holding nothing.
 */
export type FxSetPieceCue = {
  kind: "setPiece";
  shape: SetPieceShape;
  tint: FxTint;
  spin: 1 | -1;
  side: "you" | "opponent" | null;
  at: FxAnchor;
  holdMs: number;
  delayMs: number;
  durationMs: number;
  beats?: readonly number[];
  still?: true;
};
/**
 * R2801000: a Starship's launch over it, `at`: a 3-2-1 countdown, each count popping in at its
 * `countsMs` (none for a launch the queue does not hold), its engines igniting at `igniteMs`, and a
 * flame and a contrail as big as `size` says. `still` draws one still frame of it, for reduced motion.
 * It names no card (R202).
 */
export type StarshipSize = "small" | "medium" | "large";
export type FxLaunchCue = {
  kind: "launch";
  at: FxAnchor;
  tint: FxTint;
  size: StarshipSize;
  countsMs: readonly number[];
  igniteMs: number;
  delayMs: number;
  durationMs: number;
  still?: true;
};
/**
 * R3336000: a Drone Battleship's layer over it, `at`: its armor plates glowing down as the Shield took a hit
 * ("plates", `count` of them, the first `lit` bright), or a bay of the Propagation layer opening ("bay": two
 * doors sliding apart, `count` lights staying on while drones remain). It names no card (R202).
 */
export type FxShipCue = {
  kind: "ship";
  move: "plates" | "bay";
  at: FxAnchor;
  tint: FxTint;
  count: number;
  lit: number;
  delayMs: number;
  durationMs: number;
};
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
 * R2800000: a card on the board drawn toward `to` (the card itself: it sinks where it stands) as a set
 * piece swallows it; a `conceal` hides it at the swallow. The stage undoes it at `durationMs`, or when
 * the board shows its next view.
 */
/**
 * R3338000: `staggerMs`, how long into the hold this card waits before it is drawn in (one after another),
 * and `travelMs`, how long it takes to get there.
 */
export type FxPullCue = { kind: "pull"; testid: string; to: FxAnchor; delayMs: number; durationMs: number; staggerMs?: number; travelMs?: number };

/** B46–B48 stage cues come from `stage.ts`, not the S7 recipe table. `hold` waits for board catch-up (BUILD M5-T4); `durationMs` is a safety cap (R200). */
export type FxHoldCue = {
  kind: "hold";
  from: FxAnchor | null;
  to: FxAnchor;
  delayMs: number;
  landMs: number;
  durationMs: number;
  /** #2111, R2111000: it lands in a backrow zone, so it is laid down as a set, not slammed. */
  set?: true;
};
export type FxConcealCue = { kind: "conceal"; testid: string; mode: "now" | "after"; delayMs: number; durationMs: number };
export type FxLungeCue = {
  kind: "lunge";
  attacker: string;
  target: string;
  delayMs: number;
  durationMs: number;
  /** #1442: a Ranged attacker fires from where it stands. */
  still?: true;
  /** R2789000: a Flying attacker's lunge arcs over the lane. */
  arc?: true;
};

export type FxCanvasCue = FxBurstCue | FxProjectileCue | FxShotCue | FxCrackCue | FxRingCue;
export type FxDomCue =
  | FxSplatCue
  | FxRaysCue
  | FxSheenCue
  | FxGhostCue
  | FxCalloutCue
  | FxArrowsCue
  | FxBannerCue
  | FxResultCue
  | FxFractureCue
  | FxWallsCue
  | FxBrandCue
  | FxShieldCue
  | FxChaosCue
  | FxRollCue
  | FxFogCue
  | FxZoneCue
  | FxTakeoverCue
  | FxSetPieceCue
  | FxLaunchCue
  | FxShipCue;
export type FxStageCue = FxHoldCue | FxConcealCue | FxLungeCue | FxPullCue;
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
  | "roll"
  | "brand"
  | "rewind"
  | "armor"
  | "curve";

export type FxDescriptor = { readonly recipe: FxRecipe };

/** Public catalog facts of a readable defId (base face); `undefined` for "hidden" or unknown. `type` and `tags` pick its look (`looks.ts`). */
export type FxCardFacts = { rarity?: Rarity; attack?: number; health?: number; type?: CardType; tags?: readonly Tag[] };

export type FxTrapZone = { player: PlayerId; row: Row; lane: number };

/** An unresolved play. `defId` is hidden when unreadable (R97), so recipes cannot key off it (R202). */
export type FxPlay = {
  player: PlayerId;
  instanceId: string;
  defId: string;
  castOnDraw: boolean;
  /** Events since this play's `cardPlayed` (the next is step 1), public on both seats (R202). */
  step: number;
  /** Public event types and counts, including the planned event (R202). */
  seen: Readonly<Partial<Record<GameEvent["type"], number>>>;
  /**
   * R2784000: the public ids its damage, destroyed, exiled and bounced events landed on, oldest first,
   * the planned entry's included (at most FX_MEMORY_HITS); absent before the first. A chain hops on from the last.
   */
  hits?: readonly string[];
  /**
   * R2797000: the card hook this entry is, when it is no play: a Death, a turn hook or Whenever
   * (`trigger`) or an Activate, opened by its `hookResolving` marker, or a Trap's `cast`, opened by its
   * `trapFired`. Absent for a play, from its `cardPlayed` on.
   */
  hook?: FxCardHook;
};

export type FxMemory = {
  /** Ignores hidden ids; tracks event order for cast-on-draw and plays until `cardResolved` or `countered`. */
  remember(events: readonly GameEvent[]): void;
  casterOf(instanceId: string): PlayerId | undefined;
  trapZoneOf(instanceId: string): FxTrapZone | undefined;
  resolving(): FxPlay | undefined;
  /** R502: whether this very `cardPlayed` (the object remembered) was a cast on draw. */
  castOnDraw(event: GameEvent): boolean;
  /** R2789000: the last FX_MEMORY_RECENT events, the planned entry's last. */
  recent(): readonly GameEvent[];
  /** R2789000: the targets `attackerId` declared attacks on since the last `turnStarted`, oldest first. */
  attacksThisTurn(attackerId: string): readonly string[];
  /**
   * R3338000: the Terminus card whose turn the last `remember` opened (Event Horizon's extra turn, Last
   * Word's last one), when its events started a turn that card owed; else undefined.
   */
  opening(): { defId: string; player: PlayerId } | undefined;
  /** R3338000: the outermost play of `defId` still resolving (The Big Crunch's, around its casts), or undefined. */
  openPlay(defId: string): FxPlay | undefined;
  clear(): void;
};

export type FxPlanEnv = {
  /** `FX_INTENSITY_SCALE[settings.intensity]`; 0 plans nothing. */
  intensity: number;
  card: (defId: string) => FxCardFacts | undefined;
  memory: FxMemory;
  /** Facts no event carries, such as #21 Hinder's refresh loss (R502). */
  next?: PlayerView;
};

export type FxFrameSource = { request(callback: (timestampMs: number) => void): number; cancel(handle: number): void };
export type FxVisibility = { hidden(): boolean; subscribe(listener: () => void): () => void };
export type FxShakeOffset = { x: number; y: number; angle: number };
/** Where the shake goes. The default writes CSS `translate`/`rotate` on the board (S8). */
export type FxShakeSink = { apply(offset: FxShakeOffset): void; clear(): void };
