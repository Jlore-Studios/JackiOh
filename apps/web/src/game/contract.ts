// The integration contract between the board (M5-T1), the action builders and prompts (M5-T2),
// the hotseat loop (M5-T3) and the animation runner (M5-T4). No rule lives in this file: the board
// reports what was clicked, `actions.ts` turns that into an `ActionBody` chosen from `legalActions`,
// and the engine decides.

import type { ReactNode } from "react";

import type {
  ActionBody,
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

/**
 * A "Choose one" menu (a card's modes, on a play or in a prompt) renders as a Discover pop-up
 * while it offers this many options or fewer; more options keep the plain mode list. #2646: so is
 * every other choice of options that name no card, each option a choice card (ChoiceCard.tsx).
 */
export const DISCOVER_OPTION_LIMIT = 4;

/**
 * #492: a play's X offered as this many values or fewer is picked from cards in the middle of the
 * screen, as a short "Choose one" is; more values keep the X stepper.
 */
export const X_CARD_LIMIT = 4;

/**
 * Lanes are 1-based, as the engine's are (`crates/engine/src/zones.rs`; `e2e/support/types.ts`
 * declares `Lane = 1 | 2 | 3 | 4 | 5`): a `ZoneChoice` carries 1..5, so the zone testids must too.
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
  /** The offerer's line while its draw offer stands. */
  drawOfferStatus: "draw-offer-status",
  /** The other seat's notice while an offer stands, with the two answers. */
  drawOffer: "draw-offer",
  drawAccept: "draw-accept",
  drawDecline: "draw-decline",
  /** What became of the last offer (declined, accepted, expired); `data-outcome` says which. */
  drawOutcome: "draw-outcome",
  /** In the mulligan picker: `data-ready="true|false"`, whether the opponent has answered its own. */
  mulliganOpponentStatus: "mulligan-opponent-status",
  /** Inside that status, only once the opponent has answered. */
  mulliganOpponentReady: "mulligan-opponent-ready",
  /** After the viewer's own answer, until both are in: the hand with the cards going back marked. */
  mulliganWaiting: "mulligan-waiting",
  /**
   * R384, R510: the Activate control on a card the viewer controls (ActivateControl.tsx). `ability`
   * is named only when the card lists several (`CardView.activations`).
   */
  activate: (instanceId: string, ability?: string): string =>
    ability === undefined ? `activate-${instanceId}` : `activate-${instanceId}-${ability}`,
  /** R384, R510: that control's usable-count badge ("2", "∞"). */
  activateUses: (instanceId: string, ability?: string): string =>
    ability === undefined ? `activate-uses-${instanceId}` : `activate-uses-${instanceId}-${ability}`,
  /** R43, R510: a Heroic Power after the first on the hero panel (the first is `power`), each once per turn. */
  powerOf: (instanceId: string): string => `power-${instanceId}`,
  /** B5 E11: "Play" on a card in the viewer's graveyard pile, present only while `legal` lists that play. */
  pilePlay: (instanceId: string): string => `pile-play-${instanceId}`,
  /**
   * A graveyard pile: the same string as `animTestid.graveyard` (animations.ts), named here so the
   * action layer can light the pile (B5 E11) without importing the animation table.
   */
  graveyard: (side: Side): string => `graveyard-${side}`,
  /**
   * R1838000: the Trade control on the viewer's own Tradable hand card (TradeControl.tsx), in its bar
   * over the card on a 1v1 board (`tradeBar`) or in the Duos card bar.
   */
  trade: (instanceId: string): string => `trade-${instanceId}`,
  tradeBar: "trade-bar",
  /** R1853003: the Marketplace's toggle (Marketplace.tsx), and the panel it opens. */
  marketplace: "marketplace-button",
  marketplacePanel: "marketplace-panel",
  /**
   * #2645: on a phone, the control bar's ⋯ (MatchMenu.tsx) and the menu it opens, which holds Offer
   * draw, Concede and Friends.
   */
  matchMenu: "match-menu",
  matchMenuPanel: "match-menu-panel",
  /** R1853003: "Buy" under the Marketplace's lot `offer` (its index, which `marketBuy` names). */
  marketBuy: (offer: number): string => `market-buy-${offer}`,
  /** R1853003: the sale of a Resource in the viewer's own tray, at the Marketplace's quote. */
  marketSell: (instanceId: string): string => `market-sell-${instanceId}`,
} as const;

/** The CSS selector for the element whose testid is `id`, its quotes and backslashes escaped. */
export function testidSelector(id: string): string {
  return `[data-testid="${id.replace(/["\\]/g, "\\$&")}"]`;
}

/** The page's element whose testid is `id`, or null. */
export function byTestid(id: string): Element | null {
  return document.querySelector(testidSelector(id));
}

/**
 * R384, R510, R1751000: the `ability` an Activate control names. Under R1751000, board Activate
 * controls on cards with multiple abilities target `{ on: "activate", instanceId }` (without ability)
 * to open the Discover choice modal; `namedAbility` is retained for backwards compatibility.
 */
export function namedAbility(abilityCount: number, ability: string): string | undefined {
  return abilityCount > 1 ? ability : undefined;
}

export function sideOf(view: PlayerView, player: PlayerId): Side {
  return player === view.viewer ? "you" : "opponent";
}

/** R370: the cost the view gives the face-down card in `player`'s backrow lane `lane`, while one stands there. */
export function faceDownCost(view: PlayerView, player: PlayerId, lane: number): number | undefined {
  const seat = sideOf(view, player) === "you" ? view.you : view.opponent;
  const entry = seat.backrow[lane - 1];
  if (entry === null || entry === undefined || !entry.faceDown) return undefined;
  return "cost" in entry && typeof entry.cost === "number" ? entry.cost : undefined;
}

export function playerOf(view: PlayerView, side: Side): PlayerId {
  return side === "you" ? view.you.player : view.opponent.player;
}

export function sideView(view: PlayerView, side: Side) {
  return side === "you" ? view.you : view.opponent;
}

/**
 * Something the player clicked, dragged to, or dropped on. Never a decision, only a report.
 * `activate` is an Activate control (R384): a card's own, or a Heroic Power's on the hero panel.
 * `graveyard` is "Play" on a card in the viewer's graveyard pile. A Resource is a hand card (R2250000).
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
 * What the board may offer. `legal` and `selected` hold `data-testid`s; a clickable whose testid is
 * not in `legal` renders `aria-disabled="true"` and `data-legal="false"` and fires no `onClick`.
 * `actions.ts` derives both sets from the engine's `legalActions`, never from the rules.
 */
export type Highlight = {
  legal: ReadonlySet<string>;
  selected: ReadonlySet<string>;
  /**
   * The green glow (Hearthstone's "can act"): a subset of `legal`, derived by `highlightFor` from
   * `legalActions` and the open prompt's options alone. Absent means nothing glows.
   */
  glow?: ReadonlySet<string>;
  /**
   * MD-D5, R1121: the yellow a dragged or selected attack paints — the selected attacker's
   * `conditionTargets` inside `glow`, derived by `highlightFor` while attacking alone.
   */
  condition?: ReadonlySet<string>;
  /**
   * #991, R583: the cards the hovered or selected hand card's preview counts, as `card-<id>` testids:
   * its values' `ids` off the view (R1447 keeps them to cards the viewer reads), which Board.tsx adds
   * for the field alone (`countedTestids`, glow.ts). Never from `legalActions`; no click reads it.
   */
  counted?: ReadonlySet<string>;
};

export const NO_HIGHLIGHT: Highlight = { legal: new Set(), selected: new Set() };

/** Elements currently mid-animation, keyed by `data-testid` (M5-T4). */
export type AnimatingMap = ReadonlyMap<string, GameEventType>;

/** One animation entry as the board reads it: what it marks, and the events it is playing. */
export type AnimationFrames = { frames: AnimatingMap; events: readonly GameEvent[] };

/**
 * The emote surface one hero carries (R643–R644, issue §1–§5). Game owns the state; the board only
 * draws and reports. `onPortrait` is the click that is NOT a target pick, `onPortraitHold` the hold
 * that opens the hero's inspect view (R1449, R1330; issue §2: targeting wins).
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
  /**
   * The animation entries the runner has started since the board last caught up (M5-T4). The number pops
   * read these, not `view`, which the runner holds back without the event being animated; one action's
   * several numbers stay on screen together.
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
  /** R1853003: the Marketplace's shop, drawn in the control bar (#2645); absent (a replay), none. */
  market?: { legal: readonly ActionBody[]; onAction: (body: ActionBody) => void };
  /** Cosmetic-only input guard for the sand playmat while the action builder owns a card gesture. */
  sandDisabled?: boolean;
  onClick?: (target: ClickTarget) => void;
  onControl?: (control: BoardControl) => void;
  /** One hero's emote surface per side, or undefined where a route does not run emotes. */
  emotes?: (side: Side) => HeroEmotes | undefined;
  /**
   * R1295000: the viewer's own cards wear the device's card cosmetics (game/wearer.ts). A board
   * the viewer plays sets it; absent, every card draws the defaults.
   */
  wearCosmetics?: boolean;
  /**
   * R1297000: the picks of the deck the viewer plays (playmat, backdrop, card back), which beat the
   * account's on the viewer's own side. Null or absent: the account's choice applies.
   */
  deckCosmetics?: DeckCosmetics | null;
  /**
   * R1834004: both seats' frozen loadouts (R1834002) on an online board. Every seat but the wearer
   * wears its own; null or absent, they draw the defaults.
   */
  loadouts?: SeatLoadouts | null;
};
