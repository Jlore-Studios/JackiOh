// The integration contract between the board (M5-T1), the action builders and prompts (M5-T2),
// the hotseat loop (M5-T3) and the animation runner (M5-T4).
//
// Everything here is presentation vocabulary. No rule lives in this file: the board reports what
// was clicked, `actions.ts` turns that into an `ActionBody` chosen from `legalActions`, and the
// engine decides. `Highlight` is a set of `data-testid`s the engine has already blessed.

import type { ReactNode } from "react";

import type {
  EmoteGate,
  EmoteId,
  GameEvent,
  GameEventType,
  PlayerId,
  PlayerView,
  PortraitId,
  Row,
} from "@jackioh/shared";

import type { EmoteShow } from "../emotes/session.ts";

/** Viewer-relative sides. `viewFor` already orients the view, so the DOM says "you"/"opponent". */
export type Side = "you" | "opponent";

export const UNIT_LANES = 5;
export const BACKROW_LANES = 5;

/**
 * A "Choose one" menu (a card's modes, on a play or in a prompt) renders as a Discover pop-up
 * while it offers this many options or fewer; more options keep the plain mode list.
 */
export const DISCOVER_OPTION_LIMIT = 5;

/**
 * #492: a play's X offered as this many values or fewer is picked from cards in the middle of the
 * screen, as a short "Choose one" is; more values keep the X stepper.
 */
export const X_CARD_LIMIT = 4;

/**
 * Lanes are 1-based, because the engine's are: `crates/engine/src/zones.rs` numbers a row's slots
 * from 1 and reads a unit pile at `lane - 1`, and `e2e/support/types.ts` declares
 * `Lane = 1 | 2 | 3 | 4 | 5`. A `ZoneChoice` inside a `play` action therefore carries 1..5, so
 * the client's zone testids must too or nothing the engine blesses would ever light up.
 */
export const LANES: readonly number[] = [1, 2, 3, 4, 5];
export const ROWS: readonly Row[] = ["units", "backrow"];

/** `SideView.units` / `.backrow` / `.locks` / `.reserved` are 0-based arrays over 1-based lanes. */
export function laneIndex(lane: number): number {
  return lane - 1;
}

/** The `data-testid`s BUILD M5-T1 fixes. Every clickable element gets its id from here. */
export const testid = {
  zone: (side: Side, row: Row, lane: number): string => `zone-${side}-${row}-${lane}`,
  card: (instanceId: string): string => `card-${instanceId}`,
  hero: (side: Side): string => `hero-${side}`,
  handCard: (instanceId: string): string => `hand-card-${instanceId}`,
  switchPosition: (instanceId: string): string => `switch-${instanceId}`,
  /** R371: the "Face down" tag on the viewer's own face-down trap. */
  unrevealed: (instanceId: string): string => `unrevealed-${instanceId}`,
  /** R667: the Plague Chalice warning on the viewer's own hand card. */
  countered: (instanceId: string): string => `countered-${instanceId}`,
  endTurn: "end-turn",
  offerDraw: "offer-draw",
  power: "power",
  concede: "concede",
  log: "log",
  board: "board",
  seatSwitch: "seat-switch",
  result: "result-overlay",
  banner: "turn-banner",
  /** "Concede this game?" (ConfirmConcede.tsx): the dialog the `concede` control opens. */
  concedeDialog: "concede-dialog",
  /** In that dialog: the only thing that sends `{ type: "concede" }`. */
  concedeConfirm: "concede-confirm",
  /** In that dialog: close it and carry on (also Escape and a click outside). */
  concedeCancel: "concede-cancel",
  /** The offerer's line while its draw offer stands: "Draw offered — waiting for reply". */
  drawOfferStatus: "draw-offer-status",
  /** The other seat's notice while an offer stands: "Your opponent offers a draw", with the two answers. */
  drawOffer: "draw-offer",
  drawAccept: "draw-accept",
  drawDecline: "draw-decline",
  /** What became of the last offer (declined, accepted, expired); `data-outcome` says which. */
  drawOutcome: "draw-outcome",
  /** In the mulligan picker: `data-ready="true|false"`, whether the opponent has answered its own. */
  mulliganOpponentStatus: "mulligan-opponent-status",
  /** Inside that status, only once the opponent has answered: "Opponent is ready". */
  mulliganOpponentReady: "mulligan-opponent-ready",
  /** After the viewer's own answer, until both are in: the hand with the cards going back marked. */
  mulliganWaiting: "mulligan-waiting",
  /**
   * R384, R510: the Activate control on a card the viewer controls (ActivateControl.tsx). `ability`
   * is named only when the card lists several (`CardView.activations`), so a card with one ability
   * is always `activate-<instanceId>`.
   */
  activate: (instanceId: string, ability?: string): string =>
    ability === undefined ? `activate-${instanceId}` : `activate-${instanceId}-${ability}`,
  /** R384, R510: that control's usable-count badge ("2", "∞"). */
  activateUses: (instanceId: string, ability?: string): string =>
    ability === undefined ? `activate-uses-${instanceId}` : `activate-uses-${instanceId}-${ability}`,
  /**
   * R43, R510: a Heroic Power after the first on the hero panel (the first is `power`): each is its
   * own control now, as each is separately once per turn.
   */
  powerOf: (instanceId: string): string => `power-${instanceId}`,
  /** B5 E11: "Play" on a card in the viewer's graveyard pile, present only while `legal` lists that play. */
  pilePlay: (instanceId: string): string => `pile-play-${instanceId}`,
  /**
   * A graveyard pile: the same string as `animTestid.graveyard` (animations.ts), named here so the
   * action layer can light the pile (B5 E11) without importing the animation table.
   */
  graveyard: (side: Side): string => `graveyard-${side}`,
} as const;

/**
 * R384, R510: the `ability` an Activate control names, in its testid and in its click: none when the
 * card lists one ability (the engine then takes the card's only one), the ability's id when it lists
 * several. The board's control and `actions.ts`'s highlight both ask this, so they agree.
 */
export function namedAbility(abilityCount: number, ability: string): string | undefined {
  return abilityCount > 1 ? ability : undefined;
}

export function sideOf(view: PlayerView, player: PlayerId): Side {
  return player === view.viewer ? "you" : "opponent";
}

export function playerOf(view: PlayerView, side: Side): PlayerId {
  return side === "you" ? view.you.player : view.opponent.player;
}

export function sideView(view: PlayerView, side: Side) {
  return side === "you" ? view.you : view.opponent;
}

/**
 * #165: what a touch hold on a card opens while this view is on the board. On the viewer's own
 * turn a hold stays the inspect sheet it has always been, so reading a card can never eat the
 * press that plays it; on the opponent's turn nothing is playable anyway, so the hold is the
 * mouse-over a touch screen lacks — the hover preview, open while the finger stays down.
 */
export function touchHoldMode(view: PlayerView): "sheet" | "preview" {
  return view.active === view.viewer ? "sheet" : "preview";
}

/**
 * Something the player clicked, dragged to, or dropped on. Never a decision, only a report.
 *
 * `activate` is an Activate control (R384): a card's own, or a Heroic Power's on the hero panel.
 * `ability` is set only when the card lists several abilities. `graveyard` is "Play" on a card in
 * the viewer's graveyard pile (B5 E11).
 */
export type ClickTarget =
  | { on: "hand"; instanceId: string }
  | { on: "unit"; instanceId: string; side: Side; lane: number }
  | { on: "backrow"; instanceId: string; side: Side; lane: number }
  | { on: "hero"; side: Side }
  | { on: "zone"; side: Side; row: Row; lane: number }
  | { on: "switch"; instanceId: string }
  | { on: "activate"; instanceId: string; ability?: string }
  | { on: "graveyard"; instanceId: string };

export type BoardControl = "end-turn" | "offer-draw" | "power" | "concede";

/**
 * What the board may offer. `legal` and `selected` hold `data-testid`s; anything clickable whose
 * testid is not in `legal` renders with `aria-disabled="true"` and `data-legal="false"` and does
 * not fire `onClick`. The client computes neither set from the rules — `actions.ts` derives them
 * from the `legalActions` array the engine returned.
 */
export type Highlight = {
  legal: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  /**
   * The green glow (Hearthstone's "can act"): a subset of `legal`, derived by `highlightFor` from
   * `legalActions` and the open prompt's options alone. Absent means nothing glows.
   */
  glow?: ReadonlySet<string>;
};

export const NO_HIGHLIGHT: Highlight = { legal: new Set(), selected: new Set() };

/** Elements currently mid-animation, keyed by `data-testid` (M5-T4). */
export type AnimatingMap = ReadonlyMap<string, GameEventType>;

/** One animation entry as the board reads it: what it marks, and the events it is playing. */
export type AnimationFrames = { frames: AnimatingMap; events: readonly GameEvent[] };

/**
 * The emote surface one hero carries (R643–R644, issue §1–§5): its portrait, what it is showing,
 * and which menu — yours' picker or the opponent's "Mute emotes" — is open on it. Game owns the
 * state; the board only draws and reports. `onPortrait` is the click that is NOT a target pick
 * (the legal branch still goes to `onClick`), which is what opens a menu (issue §2: targeting
 * wins).
 */
export type HeroEmotes = {
  portrait: PortraitId;
  show: EmoteShow | null;
  menu: "emotes" | "mute" | null;
  muted: boolean;
  gate: () => EmoteGate;
  onPortrait: () => void;
  onPick: (emote: EmoteId) => void;
  onMute: () => void;
  onCloseMenu: () => void;
};

export type BoardProps = {
  view: PlayerView;
  highlight?: Highlight;
  animating?: AnimatingMap;
  /**
   * The animation entries the runner has started since the board last caught up (M5-T4), each one
   * the elements it marks paired with the events it is playing. The number pops read from these
   * rather than from `view`, for two reasons: `view` is the view the runner is still holding back,
   * so it does not carry the event being animated at all; and one action deals several numbers —
   * an attack pops one on the defender and then one on the attacker — which have to stay on screen
   * together rather than each vanishing as the next entry starts.
   */
  animated?: readonly AnimationFrames[];
  /** A route's server-synchronised clock, mounted in the physical End Turn housing. */
  turnClock?: ReactNode;
  /** A route status line, mounted on the board rail rather than a floating match bar. */
  matchStatus?: ReactNode;
  /** Route chrome carved into the same physical side rail as the controls. */
  boardRail?: ReactNode;
  /** Alerts and offers inset into the board rail, never rendered as page overlays. */
  boardNotices?: ReactNode;
  /** Cosmetic-only input guard for the sand playmat while the action builder owns a card gesture. */
  sandDisabled?: boolean;
  onClick?: (target: ClickTarget) => void;
  onControl?: (control: BoardControl) => void;
  /** One hero's emote surface per side, or undefined where a route does not run emotes. */
  emotes?: (side: Side) => HeroEmotes | undefined;
};
