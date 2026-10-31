// Integration contract between board (M5-T1), action builders/prompts (M5-T2),
// hotseat loop (M5-T3), and animation runner (M5-T4). Presentation vocabulary only.

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

import type { DeckCosmetics } from "../cards/cosmetics.ts";
import type { SeatLoadouts } from "../cards/loadouts.ts";
import type { EmoteShow } from "../emotes/session.ts";

/** Viewer-relative sides. `viewFor` already orients the view, so the DOM says "you"/"opponent". */
export type Side = "you" | "opponent";

/** Maximum options for "Choose one" menu rendered as Discover pop-up. */
export const DISCOVER_OPTION_LIMIT = 5;

/** #492: maximum X values rendered as card options rather than stepper. */
export const X_CARD_LIMIT = 4;

/**
 * Lanes are 1-based matching the engine (`crates/engine/src/zones.rs` and `e2e/support/types.ts`).
 */
export const LANES: readonly number[] = [1, 2, 3, 4, 5];

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
  /** R1046: the reveal timing on the viewer's own face-down play, which the opponent never sees. */
  revealAt: (instanceId: string): string => `reveal-at-${instanceId}`,
  /** R667: the Plague Chalice warning on the viewer's own hand card. */
  countered: (instanceId: string): string => `countered-${instanceId}`,
  /** R90 (#992): the note badge on the viewer's own hand card whose listed plays R90's bound cut. */
  choicesTruncated: (instanceId: string): string => `choices-truncated-${instanceId}`,
  endTurn: "end-turn",
  offerDraw: "offer-draw",
  power: "power",
  concede: "concede",
  log: "log",
  board: "board",
  seatSwitch: "seat-switch",
  result: "result-overlay",
  banner: "turn-banner",
  /** R2102000: "You have lethal", in the board's notices, while the practice AI's lethal hint is lit. */
  lethalHint: "lethal-hint",
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
  /** R384, R510: Activate control testid (`activate-<id>` or `activate-<id>-<ability>`). */
  activate: (instanceId: string, ability?: string): string =>
    ability === undefined ? `activate-${instanceId}` : `activate-${instanceId}-${ability}`,
  /** R384, R510: that control's usable-count badge ("2", "∞"). */
  activateUses: (instanceId: string, ability?: string): string =>
    ability === undefined ? `activate-uses-${instanceId}` : `activate-uses-${instanceId}-${ability}`,
  /** R43, R510: additional Heroic Powers on hero panel beyond the first. */
  powerOf: (instanceId: string): string => `power-${instanceId}`,
  /** B5 E11: "Play" on a card in the viewer's graveyard pile, present only while `legal` lists that play. */
  pilePlay: (instanceId: string): string => `pile-play-${instanceId}`,
  /** Graveyard pile testid (matches animTestid.graveyard, B5 E11). */
  graveyard: (side: Side): string => `graveyard-${side}`,
  /** R1838000: Trade control on Tradable hand card (TradeControl.tsx). */
  trade: (instanceId: string): string => `trade-${instanceId}`,
  tradeBar: "trade-bar",
  /** R1853003: the Marketplace's toggle (Marketplace.tsx), and the panel it opens. */
  marketplace: "marketplace-button",
  marketplacePanel: "marketplace-panel",
  /** R1853003: "Buy" under the Marketplace's lot `offer` (its index, which `marketBuy` names). */
  marketBuy: (offer: number): string => `market-buy-${offer}`,
  /** R1853003: the sale of a Resource in the viewer's own tray, at the Marketplace's quote. */
  marketSell: (instanceId: string): string => `market-sell-${instanceId}`,
} as const;

/**
 * R384, R510, R1751000: Activate ability identifier (omitted on multi-ability cards to open modal).
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
 * #165: touch hold destination: inspect sheet on own turn, hover preview on opponent turn.
 */
export function touchHoldMode(view: PlayerView): "sheet" | "preview" {
  return view.active === view.viewer ? "sheet" : "preview";
}

/**
 * ClickTarget: target clicked, dragged to, or dropped on (R384, R43, R2250000, B5 E11).
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
 * Highlight: legal and selected testids computed from engine `legalActions` (CLAUDE.md rule 7).
 */
export type Highlight = {
  legal: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  /**
   * The green glow (Hearthstone's "can act"): a subset of `legal`, derived by `highlightFor` from
   * `legalActions` and the open prompt's options alone. Absent means nothing glows.
   */
  glow?: ReadonlySet<string>;
  /** MD-D5, R1121: yellow highlight for dragged/selected attack conditionTargets within `glow`. */
  condition?: ReadonlySet<string>;
  /** #991, R583: preview counted cards on field (`countedTestids`, R1447). */
  counted?: ReadonlySet<string>;
};

export const NO_HIGHLIGHT: Highlight = { legal: new Set(), selected: new Set() };

/** Elements currently mid-animation, keyed by `data-testid` (M5-T4). */
export type AnimatingMap = ReadonlyMap<string, GameEventType>;

/** One animation entry as the board reads it: what it marks, and the events it is playing. */
export type AnimationFrames = { frames: AnimatingMap; events: readonly GameEvent[] };

/**
 * HeroEmotes: hero emote surface (R643–R644, issue §1–§5, §2, R1449, R1330). Game owns state.
 */
export type HeroEmotes = {
  portrait: PortraitId;
  /** R1343: the emotes the picker offers — the viewer's dealt hand, in the pool's order. */
  hand: readonly EmoteId[];
  show: EmoteShow | null;
  menu: "emotes" | "mute" | null;
  /** R1449: the open menu sits in the hero's inspect view (a hold) rather than on the portrait (a click). */
  inspect: boolean;
  muted: boolean;
  gate: () => EmoteGate;
  onPortrait: () => void;
  onPortraitHold: () => void;
  onPick: (emote: EmoteId) => void;
  onMute: () => void;
  onCloseMenu: () => void;
};

export type BoardProps = {
  view: PlayerView;
  highlight?: Highlight;
  animating?: AnimatingMap;
  /** Active animation entries (M5-T4) marking elements with events in flight for number pops. */
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
  /** R1295000: device card cosmetics for viewer cards (game/wearer.ts). */
  wearCosmetics?: boolean;
  /** R1297000: deck cosmetics overriding device picks on viewer's side. */
  deckCosmetics?: DeckCosmetics | null;
  /** R1834004: frozen loadouts (R1834002) for non-wearer seats. */
  loadouts?: SeatLoadouts | null;
};
