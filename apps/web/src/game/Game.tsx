// The whole client in one component: board (M5-T1), pickers (M5-T2), animation runner (M5-T4).
//
// Exactly three pieces of state, none a rule: `interaction` (the play or attack being built; `actions.ts`
// advances it by filtering the engine's `legal` array), `queue` (the animation runner: BUILD M5-T4 updates
// the view after an event's animation, so the board renders the view the events were planned against) and
// `shown` (that held-back view). Everything else is `props.view` and `props.legal`; `onAction` goes straight
// to the caller, the only thing that talks to the engine (CLAUDE.md rule 7). The chrome holds no rule either:
// Concede asks first (ConfirmConcede.tsx), and the draw offer's notices, R1838000's Trade control
// (TradeControl.tsx) and R1853003's Marketplace shop (Marketplace.tsx, #2645) come from the view, each live as `legal` lists its moves. `legal` belongs to
// `props.view`, so every consumer gets none until the board SHOWS that view: else the newer position's moves
// lit End turn before the player saw the turn was theirs.
// One intent is sent once per view: `dispatch` drops an endTurn, answer or mulligan equal to the last one sent
// against the same view (R3060000).

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactElement,
  type ReactNode,
} from "react";

import type { ActionBody, Aim, CombatForecast, EmoteGate, EmoteId, GameEvent, PlayerId, PlayerView, PortraitId } from "@jackioh/shared";

import type { DeckCosmetics } from "../cards/cosmetics.ts";
import type { SeatLoadouts } from "../cards/loadouts.ts";
import { useTiltHold } from "../cards/tilt.ts";
import { FullScreenArtHost } from "../cards/index.ts";
import type { EmoteShow } from "../emotes/session.ts";
import type { HeroEmotes } from "./Hero.tsx";
import type { ChoiceHover } from "./choiceHover.ts";
import Board from "./Board.tsx";
import { useMarkBoardShown } from "./boardShown.ts";
import ConfirmConcede from "./ConfirmConcede.tsx";
import { TradeBar } from "./TradeControl.tsx";
import DrawOfferNotice from "./DrawOffer.tsx";
import Prompt from "./Prompt.tsx";
import { promptOver } from "./promptOver.ts";
import DragLayer from "./drag/DragLayer.tsx";
import OpponentAim from "./aim/OpponentAim.tsx";
import { useAimEmitter } from "./aim/useAimEmitter.ts";
import type { DragPlan } from "./drag/model.ts";
import { IDLE, answerPlaguePlacement, highlightFor, onClickTarget, onControl, type Interaction } from "./actions.ts";
import { attackOutcomes, withOutcomes } from "./attackOutcome.ts";
import {
  HIDDEN_ID,
  SET_PIECE_SKIP_HINT_MS,
  TAKEOVER_SET_PIECE,
  animTestid,
  createAnimationQueue,
  entranceEntries,
  leadMs,
  newEventsSince,
  planEntries,
  slamEntries,
  type AnimationEntry,
  type AnimationQueue,
  type SetPieceClaimer,
} from "./animations.ts";
import { NO_HIGHLIGHT, sideView, testid, type BoardControl, type AnimatingMap, type ClickTarget, type Highlight, type HeroAside } from "./contract.ts";
import { CatalogContext } from "./catalog.ts";
import { cardMotionLevel } from "./cardMotion.ts";
import { aimDeal, clearDeal, planOpening, type OpeningPlan } from "./opening.ts";
import { damageFeel, damageTier, SLAM_LAND_AT, UNIT_SLAM, type DamageTier } from "./damageFeel.ts";
import { slamSand } from "./SandSurface.tsx";
import { createEntranceResolver, createSlamResolver } from "./slamResolver.ts";
import TakeoverStill from "./TakeoverStill.tsx";
import { GameResult, theirHandOf, type ResultForm } from "./Result.tsx";
import { useOsReducedMotion } from "./useOsReducedMotion.ts";
import { usePhonePortrait } from "./phoneBar.ts";
import FxLayer from "../fx/FxLayer.tsx";
import type { FxVisibility } from "../fx/types.ts";
import { getFxSettings, useFxSettings } from "../fx/settings.ts";
import { getAudioEngine } from "../audio/engine.ts";
import AuraLayer from "../fx/aura/AuraLayer.tsx";
import { hasSetPiece, setPieceShapeOf, signatureClaim, signatureRunOf, wipeClaim } from "../fx/setPieces.ts";
import { questClaim } from "../fx/systemMoments.ts";
import { anomalyClaim } from "../fx/anomaly.ts";
import { starshipClaim } from "../fx/starship.ts";
import CardShowcase from "./showcase/CardShowcase.tsx";
import { WearerProvider } from "./wearer.ts";
import { useSetting } from "../settings/index.ts";
import { SHORTCUTS } from "../shortcuts/shortcuts.ts";
import { isTyping } from "../replay/ReplayToolbar.tsx";
import "./animations.css";
import { useGameAudio, usePickupSound, useVoiceSpeaking } from "../audio/index.ts";
import { defaultVisibility } from "./visibility.ts";
import { useRootVars, useRootVarValues } from "./rootVars.ts";
import { useGameStats } from "../stats/useGameStats.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { SidelineStrip } from "../sideline/SidelineStrip.tsx";
import type { GameSideline } from "../sideline/useSideline.ts";

/** The attribute a mounted Game sets on its parent, the element board.css lays the game screen out by (#3633). */
export const GAME_HOST_ATTRIBUTE = "data-game-host";

function sameSet(a: ReadonlySet<string> | undefined, b: ReadonlySet<string> | undefined): boolean {
  if (a === b) return true;
  if (a === undefined || b === undefined || a.size !== b.size) return false;
  for (const item of a) if (!b.has(item)) return false;
  return true;
}

/** #3633: two highlights that light, select, glow, ring and forecast the same things. */
export function sameHighlight(a: Highlight, b: Highlight): boolean {
  if (a === b) return true;
  if (!sameSet(a.legal, b.legal) || !sameSet(a.selected, b.selected) || !sameSet(a.glow, b.glow) || !sameSet(a.condition, b.condition) || !sameSet(a.counted, b.counted)) return false;
  if (a.outcomes === b.outcomes) return true;
  if (a.outcomes === undefined || b.outcomes === undefined || a.outcomes.size !== b.outcomes.size) return false;
  for (const [key, outcome] of a.outcomes) if (b.outcomes.get(key) !== outcome) return false;
  return true;
}

/** The turn banner, which `Board` does not render: one `turn-banner` (M5-T4, `e2e/support/testids.ts` BANNER). */
function isTurnEvent(event: GameEvent): event is Extract<GameEvent, { type: "turnStarted" | "turnAutoEnded" }> {
  return event.type === "turnStarted" || event.type === "turnAutoEnded";
}

function bannerText(
  view: PlayerView,
  lastType: string | undefined,
  startedFor?: PlayerId,
  extra?: boolean,
): string | null {
  if (view.result !== null) return t("Game over");
  if (lastType === "turnAutoEnded") return t("No moves left. Turn ended.");
  // #37: while a `turnStarted` entry plays, the board is still the view from before the turn began,
  // so its banner names the turn that is starting. R845: an extra turn's banner says so.
  if (startedFor !== undefined) {
    if (extra === true) return startedFor === view.viewer ? t("Your extra turn") : t("Opponent's extra turn");
    return startedFor === view.viewer ? t("Your turn") : t("Opponent's turn");
  }
  if (view.phase === "mulligan") return t("Mulligan");
  return view.active === view.viewer ? t("Your turn") : t("Opponent's turn");
}

export type GameProps = {
  view: PlayerView;
  legal: readonly ActionBody[];
  onAction: (body: ActionBody) => void;
  /** The engine's refusal for the last action, if any. `PlayerView` has no error channel. */
  error?: string | null;
  /**
   * R3477000: the engine's forecast of this seat's attacks, sent with `view` (a seat's frame, a practice
   * snapshot). While an attacker is picked, each of its legal targets shows its entry. Absent, none does.
   */
  forecast?: CombatForecast;
  /** The route's ways on from a finished game, drawn in the result panel (Result.tsx). */
  resultActions?: ReactNode;
  /** `chip` when the route draws its own result dialog (practice); `panel` otherwise. */
  resultForm?: ResultForm;
  /** The opponent hero's seat label, drawn over the default `Opponent` (a practice game's `AI · Easy`). */
  enemyLabel?: string;
  /** Route controls drawn beside Concede on desktop and as rows of the `⋯` match menu on phones. */
  controlExtras?: ReactNode;
  /** The route's way back to its own result dialog from the board's result chip (practice). */
  onOpenResult?: () => void;
  /** The chip's `practice-outcome` button that opens the route's result dialog. */
  openResultTestId?: string;
  /** R345: pins the R82 preference this board sends; the tutorial's lessons are written around R82. */
  autoEndTurn?: boolean;
  /** Server-synchronised route clock, rendered by Board beside End Turn rather than as floating HUD. */
  turnClock?: ReactNode;
  /** Route connection status, rendered on the board rail rather than above it. */
  matchStatus?: ReactNode;
  /** Route navigation and match-series information, mounted in the board rail. */
  boardRail?: ReactNode;
  /**
   * #2932: on a phone held upright the rail rides the top strip, beside the turn banner, in a row that
   * already exists, so it takes no height from the field; the control bar keeps End turn, the log, the
   * menu and settings. Off, or on any other layout, the rail stays in the control bar. A route with a
   * back link and a rank line (a match, a replay, a watch) opts in; the Duos board keeps its own bar.
   */
  phoneRailTop?: boolean;
  /** DR14 (#2706): route chrome drawn beside each hero, as `boardRail` is in the rail (a dungeon run's relics). */
  heroAside?: HeroAside;
  /** A route warning mounted in the board rail. */
  boardNotice?: ReactNode;
  /** The route's emote session (R643–R644). Absent, the heroes draw default portraits and open no menus. */
  emotes?: GameEmotes;
  /** R2990004: the watchers' emotes, a networked route's `useSideline`. Absent, no sideline strip. */
  sideline?: GameSideline;
  /** R639: log this game in the device's statistics when it ends; not a lesson's or hotseat's. */
  trackStats?: boolean;
  /** R1295000: the viewer's own cards wear the device's card cosmetics; a replay's never do. */
  wearCosmetics?: boolean;
  /** R1297000: the picks of the deck the viewer plays, which beat the account's on the viewer's own side. */
  deckCosmetics?: DeckCosmetics | null;
  /** R1834004: both seats' frozen loadouts (R1834002): a seat not wearing the device's picks wears its own. */
  loadouts?: SeatLoadouts | null;
  /** R738: `emit` sends this seat's aim as it changes; `opponent` is the opponent's as last relayed. */
  aim?: GameAim;
  /** R2739001: `emit` sends the option of this seat's card picker under its pointer; `opponent` is theirs as last relayed. */
  choiceHover?: ChoiceHover;
  /** #511: a replay's board: no drag, glow or R345 preference, and no prompt but `replayPrompt`'s. */
  replay?: boolean;
  /** R2211000: on a `replay` board, the open prompt is drawn read-only (Prompt's `readOnly`). */
  replayPrompt?: boolean;
  /**
   * The board is a tutorial lesson's: its mulligan picker keeps the engine's instruction, its count
   * and its panel, which the lesson teaches, where every other game's is its cards alone and says
   * who goes first only in its accessible name (Prompt.tsx, #2738).
   */
  lesson?: boolean;
  /**
   * R2102000: the practice AI's lethal hint, a sure win this turn by what the human can see: it
   * outlines the opponent's hero and End turn and says "You have lethal", never how.
   */
  lethalHint?: boolean;
  /** R1838000: the board's own Trade bar; the Duos board turns it off and puts Trade in its card bar. */
  tradeBar?: boolean;
  /** §9.9, R188: the board's Offer draw (Board's `showOfferDraw`); the practice route turns it off, its AI declining every offer. */
  showOfferDraw?: boolean;
  /**
   * R2095000: no audio, effects layer or showcase, for the Duos teammate's read-only board: they anchor
   * on the whole page, so a second board would play the other duel's cues on the own board.
   */
  silent?: boolean;
  /**
   * R3497000: an online board drains its animation queue while the page is hidden (and when it turns
   * visible with steps still queued), so coming back shows the live board at once. Practice, dungeon
   * and opened-replay boards leave it off.
   */
  drainWhenHidden?: boolean;
  /** R3497000: the page-visibility source, injectable in tests; the page's own when absent. */
  visibility?: FxVisibility;
  /**
   * R2785000: claims an entry as a set piece, asked before the takeover; read at each enqueue. The
   * /dev/fx gallery's fixture set piece is its one caller until issue #2782's set pieces.
   */
  setPieceOf?: SetPieceClaimer;
  /**
   * R2972000: what the "Concede this game?" dialog says in place of its usual body, already translated.
   * A dungeon fight passes one saying the whole run ends with it; every other route keeps the usual.
   */
  concedeBody?: string;
};

/** What Game needs of a route that shows aims both ways (R738). */
export type GameAim = { emit: (aim: Aim | null) => void; opponent: Aim | null };

/** A route's `useEmotes`, keyed on PlayerId, not Side, so a hotseat hand-over moves no bubble. */
export type GameEmotes = {
  portraitOf: (player: PlayerId) => PortraitId;
  /** R1341, R1343: the seat's dealt emote hand, which its menu shows. */
  handOf: (player: PlayerId) => readonly EmoteId[];
  visible: (player: PlayerId) => EmoteShow | null;
  send: (player: PlayerId, emote: EmoteId) => boolean;
  gate: (player: PlayerId) => EmoteGate;
  muted: (player: PlayerId) => boolean;
  mute: (player: PlayerId) => void;
};

/** What the board is offered while it is still showing an older view than `legal` describes. */
const NOTHING_LEGAL: readonly ActionBody[] = [];

/**
 * R3060000: the intents whose repeat against one view is one press sent twice (a double click on End turn,
 * a double tap on a one-tap choice or on the mulligan's Ready). Concede, plays, attacks, activations and
 * buys stay out: a repeat of those can be meant, and the engine refuses what it must.
 */
const ONCE_PER_VIEW: ReadonlySet<ActionBody["type"]> = new Set(["endTurn", "answer", "mulligan"]);

export default function Game({
  view,
  legal: offered,
  onAction,
  error,
  forecast,
  resultActions,
  resultForm = "panel",
  enemyLabel,
  controlExtras,
  onOpenResult,
  openResultTestId,
  autoEndTurn: pinnedAutoEndTurn,
  turnClock,
  matchStatus,
  boardRail,
  phoneRailTop = false,
  heroAside,
  boardNotice,
  emotes,
  sideline,
  trackStats = false,
  wearCosmetics = false,
  deckCosmetics = null,
  loadouts = null,
  aim,
  choiceHover,
  replay = false,
  replayPrompt = false,
  lesson = false,
  lethalHint = false,
  tradeBar = true,
  showOfferDraw = true,
  silent = false,
  drainWhenHidden = false,
  visibility,
  setPieceOf,
  concedeBody,
}: GameProps): ReactElement {
  useLocale();
  // R2118000: no command menu over a board.
  useMarkBoardShown();
  const [interaction, setInteraction] = useState<Interaction>(IDLE);
  // #2924: the shown turn, active seat and viewer the selection was built on. The turn clock
  // (§10.10) can end the turn mid-choice, so a half-built play or attack is dropped when the
  // shown view moves to another turn, seat or viewer, as Escape already drops it.
  const [selectionKey, setSelectionKey] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  // The same element as state, for the Trade bar, which measures inside it (TradeControl.tsx).
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  const attach = useCallback((element: HTMLDivElement | null) => {
    root.current = element;
    setHost(element);
  }, []);
  // #3633: the shell's layout keys off this instead of `:has(> .game)`, whose universal tail made
  // Blink restyle the whole document on every effect (board.css, the game screen).
  useLayoutEffect(() => {
    const parent = host?.parentElement;
    if (!parent) return;
    parent.setAttribute(GAME_HOST_ATTRIBUTE, "true");
    return () => {
      // React runs this before it detaches `host`, so only another Game in the same parent keeps the mark.
      const another = Array.from(parent.children).some((child) => child !== host && child.classList.contains("game"));
      if (!another) parent.removeAttribute(GAME_HOST_ATTRIBUTE);
    };
  }, [host]);
  // #3633: the root's custom properties, which board.css and animations.css read (rootVars.ts).
  const vars = useRootVars(root);
  /** The seat whose Concede control asked; a hotseat hand-over or the game's end closes it. */
  const [concedeFor, setConcedeFor] = useState<PlayerId | null>(null);
  /**
   * R849000: the engine's reason a click on your own unit offered no attack (`UnitView.attackReason`),
   * held with the view it was read from so it disappears when the board shows another view.
   */
  const [notice, setNotice] = useState<{ text: string; view: PlayerView } | null>(null);
  /**
   * Which portrait's menu is open, bound to the viewer that opened it so a hotseat hand-over closes
   * it, and where: off the portrait (`panel`, a click) or in the hero's inspect view (`inspect`, a hold; R1449).
   */
  const [emoteMenu, setEmoteMenu] = useState<{
    side: "you" | "opponent";
    viewer: PlayerId;
    place: "panel" | "inspect";
  } | null>(null);

  // The view the DOM is showing: the newest one once the queue has settled, an older one while
  // an event is still animating over it (BUILD M5-T4).
  const [shown, setShown] = useState<PlayerView>(view);
  const latest = useRef<PlayerView>(view);
  latest.current = view;

  const [animating, setAnimating] = useState(() => new Map<string, never>());
  /**
   * The event in flight, NOT `animating.size > 0`: some `data-animating` rows resolve to an element
   * the view does not render (a destroyed card), and "no element is animating" is not "caught up".
   */
  const [inFlight, setInFlight] = useState<AnimationEntry | null>(null);
  // R3344001: no card tilts while the queue is draining an entry.
  useTiltHold(inFlight !== null);
  const inFlightDamage = inFlight?.events.find(
    (event): event is Extract<GameEvent, { type: "damage" }> => event.type === "damage",
  );
  const [lingering, setLingering] = useState<readonly AnimationEntry[]>([]);
  const lingerTimers = useRef(new Map<AnimationEntry, number>());
  const visibleDamage = [
    ...(inFlightDamage === undefined ? [] : [inFlightDamage]),
    ...lingering.flatMap((entry) => entry.events.filter((event): event is Extract<GameEvent, { type: "damage" }> => event.type === "damage")),
  ].sort((left, right) => right.amount - left.amount)[0];
  const impactTier: DamageTier | undefined = visibleDamage === undefined ? undefined : damageTier(visibleDamage.amount);
  const impactFeel = visibleDamage === undefined ? undefined : damageFeel(visibleDamage.amount);
  const [hitStop, setHitStop] = useState(false);
  /** Every entry started since the board last caught up, so one action's pops all stay up until `shown` swaps. */
  const [burst, setBurst] = useState<readonly AnimationEntry[]>([]);
  const queue = useRef<AnimationQueue | null>(null);
  // #258: the OS preference is followed live, as the queue below follows the setting.
  const osReducesMotion = useOsReducedMotion();
  // #2932: the rail rides the top strip on a phone held upright, when the route asks for it and has one.
  const portrait = usePhonePortrait();
  const railTop = phoneRailTop && portrait && boardRail !== undefined && boardRail !== null;
  const reducedMotion = useSetting("reduceMotion") || osReducesMotion;
  // #3633: the board's drag setting, which prompt.css and drag.css read off the game root (Board draws `data-drag` too).
  const dragToPlay = useSetting("dragToPlay");
  // #3347: how much of the card-motion flourishes the settings allow (game/cardMotion.ts): board.css reads it as `data-card-motion`.
  const [fx] = useFxSettings();
  const motion = cardMotionLevel(reducedMotion || fx.motion === "reduce", fx.intensity);
  // #3347: the opening deal, while it runs (game/opening.ts). Its effect below sounds the riffle and each card
  // dealt on timers that end with it, and takes the deal's aim off the hands once it is over.
  const [opening, setOpening] = useState<OpeningPlan | null>(null);
  const openingRuns = useRef(false);
  const endOpening = useCallback((): void => setOpening(null), []);
  useEffect(() => {
    openingRuns.current = opening !== null;
    if (opening === null) return undefined;
    const play = (id: "riffle" | "deal"): void => {
      try {
        getAudioEngine().playSfx(id);
      } catch {
        // Sound is never a rule: no engine (jsdom) or a refused sound drops the cue.
      }
    };
    play("riffle");
    const timers = opening.deals.map((deal) => window.setTimeout(() => play("deal"), deal.atMs));
    timers.push(window.setTimeout(endOpening, opening.totalMs));
    return () => {
      for (const timer of timers) window.clearTimeout(timer);
      openingRuns.current = false;
      if (root.current !== null) clearDeal(root.current);
    };
  }, [opening, endOpening]);
  const hitStopTimer = useRef<number | null>(null);
  const triggerHitStopMs = useCallback((durationMs: number): void => {
    if (durationMs <= 0) return;
    if (hitStopTimer.current !== null) window.clearTimeout(hitStopTimer.current);
    setHitStop(true);
    hitStopTimer.current = window.setTimeout(() => {
      hitStopTimer.current = null;
      setHitStop(false);
    }, durationMs);
  }, []);
  const triggerHitStop = useCallback((amount: number): void => triggerHitStopMs(damageFeel(amount).hitStopMs), [triggerHitStopMs]);
  useEffect(
    () => () => {
      if (hitStopTimer.current !== null) window.clearTimeout(hitStopTimer.current);
      for (const timer of lingerTimers.current.values()) window.clearTimeout(timer);
      lingerTimers.current.clear();
    },
    [],
  );

  // R345: "End turn automatically" is the player's intent for R82, which the engine holds, so it
  // goes out as an action when it differs from this seat's view, once per seat and value: a
  // refusal is not retried until the setting or the seat changes.
  const wantAutoEndTurn = useSetting("autoEndTurn");
  const autoEndTurn = pinnedAutoEndTurn ?? wantAutoEndTurn;
  const engineAutoEndTurn = view.autoEndTurn !== false;
  const autoEndTurnSent = useRef<string | null>(null);
  useEffect(() => {
    if (replay || view.result !== null) return;
    if (autoEndTurn === engineAutoEndTurn) {
      autoEndTurnSent.current = null;
      return;
    }
    const sent = `${view.viewer}:${String(autoEndTurn)}`;
    if (autoEndTurnSent.current === sent) return;
    autoEndTurnSent.current = sent;
    onAction({ type: "setAutoEndTurn", enabled: autoEndTurn });
  }, [replay, view.viewer, view.result, autoEndTurn, engineAutoEndTurn, onAction]);

  // "Reduce motion" makes every duration 0; the queue reads it once, when built, so a change builds
  // a new one. Hit-stop is a visual pause only, never delaying the queue, network, engine or turn
  // clock; reduced motion intentionally keeps it.
  useEffect(() => {
    if (!reducedMotion && inFlightDamage !== undefined) triggerHitStop(inFlightDamage.amount);
  }, [inFlightDamage, reducedMotion, triggerHitStop]);
  // #185: a landing Unit's hit-stop falls on its landing beat, after any anticipation (Reduce
  // motion has none, and starts it as the action arrives, below).
  const inFlightSlam = inFlight?.slam;
  const inFlightMs = inFlight?.durationMs ?? 0;
  // R2722001, R2785000: a slam's anticipation and a set piece's hold (a Mythic's takeover's) both come before the motion.
  const inFlightLead = inFlight === null ? 0 : leadMs(inFlight);
  const setPiece = inFlight?.setPiece;
  const takeoverHold = setPiece?.key === TAKEOVER_SET_PIECE ? setPiece.holdMs : 0;
  // R2800000: a board clear's set piece twists (or folds) the sides its wipe takes from, over its hold.
  const setPieceShape = setPiece === undefined ? undefined : setPieceShapeOf(setPiece.key);
  useEffect(() => {
    if (reducedMotion || inFlightSlam === undefined) return undefined;
    const landsAt = inFlightLead + (inFlightMs - inFlightLead) * SLAM_LAND_AT;
    const timer = window.setTimeout(() => {
      triggerHitStopMs(inFlightSlam.hitStopMs);
      // The cracks and the crater go into the sand under the cards, and fade there (SandSurface).
      const board = UNIT_SLAM[inFlightSlam.tier].board;
      const marks = board === "crater" ? (["crater", "cracks"] as const) : board === "cracks" ? (["cracks"] as const) : [];
      const zone = inFlightSlam.zone === null ? null : root.current?.querySelector(`[data-testid="${inFlightSlam.zone}"]`);
      if (marks.length > 0 && zone !== null && zone !== undefined) slamSand(zone, { marks });
    }, landsAt);
    return () => window.clearTimeout(timer);
  }, [inFlightSlam, inFlightMs, inFlightLead, reducedMotion, triggerHitStopMs]);
  // #185: a landing Unit's tier comes from the newest view and the catalog, read at each enqueue.
  const catalog = useContext(CatalogContext);
  const catalogRef = useRef(catalog);
  catalogRef.current = catalog;
  const slamResolver = useMemo(() => createSlamResolver(() => latest.current, () => catalogRef.current), []);
  // R2722001: a readable Legendary's or Mythic's entrance, and a Mythic's takeover, read at each enqueue.
  const entranceResolver = useMemo(() => createEntranceResolver(() => catalogRef.current), []);
  /** R2722001: under Reduce motion, the viewer's own Mythic shown still over the board, and the cards already shown so. */
  const [still, setStill] = useState<{ defId: string; instanceId: string; view: PlayerView } | null>(null);
  const stillSeen = useRef(new Set<string>());
  const endStill = useCallback(() => setStill(null), []);
  const setPieceOfRef = useRef(setPieceOf);
  setPieceOfRef.current = setPieceOf;
  const builtFor = useRef(reducedMotion);
  if (queue.current === null || builtFor.current !== reducedMotion) {
    builtFor.current = reducedMotion;
    queue.current = createAnimationQueue({
      reducedMotion,
      slamOf: slamResolver,
      entranceOf: entranceResolver,
      // R2785000: the Duos teammate's silent board plays no set piece (R2095000).
      setPieces: !silent,
      // R2800000: a board clear's hook plays its removals as one wipe, which its set piece claims.
      // R2785000: a quest's climax claims its entry after a wipe does (SET_PIECES_PER_ACTION stays 1).
      // R2801000: and a Starship's launch after both.
      wipeOf: hasSetPiece,
      // R3338000: a Terminus card's signature run is one entry its set piece claims.
      signatureOf: signatureRunOf,
      setPieceOf: (entry) =>  setPieceOfRef.current?.(entry) ?? wipeClaim(entry) ?? signatureClaim(entry) ?? questClaim(entry) ?? starshipClaim(entry) ?? anomalyClaim(entry),
      onSettled: () => {
        setShown(latest.current);
      },
    });
  }
  const runner = queue.current;
  // R3497000: the page-visibility source; the page's own unless a test injects one.
  const fallbackVisibility = useMemo(() => defaultVisibility(), []);
  const source = visibility ?? fallbackVisibility;
  useGameStats(view, trackStats);
  useGameAudio(runner, view, silent); // before the layout effects below: it must see each view before the runner is fed (audio/useGameAudio.ts)
  // A voice line holding the channel marks the board `data-speaking`, the one attribute practice's
  // pacing reads to hold the AI's next step (SPEC §9.9).
  const speaking = useVoiceSpeaking();

  // A layout effect declared before the one that enqueues: a passive one would miss the first
  // entry's `data-animating`.
  useLayoutEffect(() => {
    const stop = runner.subscribe(() => {
      setAnimating(runner.animating() as Map<string, never>);
      const entry = runner.inFlight();
      setInFlight(entry);
      setBurst((prev) => {
        if (entry === null) return prev.length === 0 ? prev : [];
        return prev.includes(entry) ? prev : [...prev, entry];
      });
      if (entry !== null) {
        const giga = entry.events.find(
          (event): event is Extract<GameEvent, { type: "damage" }> => event.type === "damage" && damageTier(event.amount) === "giga",
        );
        if (giga !== undefined && !lingerTimers.current.has(entry)) {
          const lingerMs = damageFeel(giga.amount).numberLingerMs;
          setLingering((previous) => [...previous, entry]);
          const timer = window.setTimeout(() => {
            lingerTimers.current.delete(entry);
            setLingering((previous) => previous.filter((candidate) => candidate !== entry));
          }, lingerMs);
          lingerTimers.current.set(entry, timer);
        }
      }
    });
    // A queue rebuilt for a new reduced-motion value starts empty; the one it replaced was reset
    // below without settling, so catch up with it here (a no-op on the first build).
    if (runner.idle()) {
      setAnimating((prev) => (prev.size === 0 ? prev : (runner.animating() as Map<string, never>)));
      setInFlight(null);
      setBurst((prev) => (prev.length === 0 ? prev : []));
      setShown(latest.current);
    }
    return () => {
      stop();
      runner.reset();
    };
  }, [runner]);

  // #3347: the riffle's and the deal's times, where board.css reads them. #3633: written onto their
  // readers rather than the root's own style (rootVars.ts), with the entry's below.
  const riffle = inFlight?.riffle;
  const riffleVars: Record<string, string> =
    opening !== null
      ? { "--riffle-ms": `${String(opening.riffleMs)}ms`, "--riffle-delay": "0ms", "--deal-ms": `${String(opening.flightMs)}ms` }
      : riffle === undefined
        ? {}
        : { "--riffle-ms": `${String(riffle.ms)}ms`, "--riffle-delay": `${String(riffle.delayMs)}ms` };
  useRootVarValues(vars, riffleVars);
  // The deal's aim is read from the hands as they are laid out, so after the render that shows `data-dealing`.
  useLayoutEffect(() => {
    if (opening !== null && root.current !== null) aimDeal(root.current, opening);
  }, [opening]);

  // R2785000: a tap or a click on the board, or Space, ends a set piece's hold at once and does
  // nothing else: the click reaches no control, and the Space that skipped presses no focused button
  // (its keyup, which would, is swallowed too). A key in a text field is the field's. Listening for
  // the board's life, not the hold's, so that keyup is still heard once the hold is over.
  useEffect(() => {
    const element = root.current;
    if (element === null) return undefined;
    let swallowUp = false;
    const onClick = (event: MouseEvent): void => {
      // #3347: a tap ends the opening deal at once, and goes on to whatever it was on (the mulligan's Keep
      // among them): a deal is no hold, and swallowing the tap would cost the player one press.
      if (openingRuns.current) endOpening();
      if (!runner.skip()) return;
      event.stopPropagation();
      event.preventDefault();
    };
    const isSkipKey = (event: KeyboardEvent): boolean =>
      event.key === SHORTCUTS.skipSetPiece[0] && !event.altKey && !event.ctrlKey && !event.metaKey && !isTyping(event.target);
    const onKeyDown = (event: KeyboardEvent): void => {
      if (!isSkipKey(event) || event.repeat || !runner.skip()) return;
      swallowUp = true;
      event.stopPropagation();
      event.preventDefault();
    };
    const onKeyUp = (event: KeyboardEvent): void => {
      if (!swallowUp || event.key !== SHORTCUTS.skipSetPiece[0]) return;
      swallowUp = false;
      event.stopPropagation();
      event.preventDefault();
    };
    element.addEventListener("click", onClick, true);
    document.addEventListener("keydown", onKeyDown, true);
    document.addEventListener("keyup", onKeyUp, true);
    return () => {
      element.removeEventListener("click", onClick, true);
      document.removeEventListener("keydown", onKeyDown, true);
      document.removeEventListener("keyup", onKeyUp, true);
    };
  }, [runner, endOpening]);

  // A LAYOUT effect: `data-animating` has to be on the DOM by the time the click that caused the
  // action has returned, or `cy.settled()` reads a stale view as up to date.
  const seen = useRef<PlayerView | null>(null);
  useLayoutEffect(() => {
    if (seen.current === view) return;
    const previous = seen.current;
    seen.current = view;

    // `view.events` is §10.8's sliding window over the whole match. Views of two seats are not
    // comparable: `viewFor` redacts per viewer, so after a hotseat hand-over replaying the window
    // would show the arriving player the whole recent history.
    const fresh =
      previous === null || previous.viewer !== view.viewer
        ? []
        : newEventsSince(previous.events, view.events);

    // #3347: the first view of a game in the mulligan phase is its opening deal, which has no event to animate.
    if (previous === null && !silent && !replay) {
      const plan = planOpening(view, getFxSettings().speed);
      if (plan !== null && motion !== "still") setOpening(plan);
    }

    if (previous !== null && previous.viewer !== view.viewer) runner.drain();
    // Planned against the view the board is STILL SHOWING, the only board that still has the card
    // an event destroys.
    if (fresh.length > 0 && previous !== null) {
      if (reducedMotion) {
        const largest = fresh
          .filter((event): event is Extract<GameEvent, { type: "damage" }> => event.type === "damage")
          .reduce((amount, event) => Math.max(amount, event.amount), 0);
        triggerHitStop(largest);
        // #185: and the heaviest landing's, from the same capped share the queue would give it.
        const slams = slamEntries(planEntries(fresh, previous, true), slamResolver, true);
        triggerHitStopMs(slams.reduce((ms, entry) => Math.max(ms, entry.slam?.hitStopMs ?? 0), 0));
        // R2722001, R3345000: a public Mythic's entrance under reduced motion shows TakeoverStill with
        // the card's own picture for both seats (hidden play shows nothing, R202). R3337003: a play that
        // lays an Anomaly yields it to the Anomaly's still title card, as its takeover yields to the rift.
        const rifted = new Set(slams.flatMap((entry) => entry.rift?.sourceId ?? []));
        const mythic = entranceEntries(slams, entranceResolver, { seen: stillSeen.current }).find(
          (entry) => entry.entrance?.tier === "mythic" && entry.entrance.defId !== HIDDEN_ID && !rifted.has(entry.entrance.instanceId),
        )?.entrance;
        if (mythic !== undefined) setStill({ defId: mythic.defId, instanceId: mythic.instanceId, view: previous });
      }
      runner.enqueue(fresh, previous);
      // R3497000: a hidden tab drains each new view instead of replaying the backlog on throttled
      // timers. Enqueue-then-drain, never skip-enqueue, so the drain signal still reaches `FxLayer`,
      // the director still owes then flushes on idle, and `onSettled` sets `shown`.
      if (drainWhenHidden && !silent && source.hidden()) runner.drain();
    }
    if (previous !== null && previous.viewer !== view.viewer) {
      stillSeen.current.clear();
      setStill(null);
    }
    if (runner.idle()) setShown(view);
  }, [view, runner, reducedMotion, triggerHitStop, triggerHitStopMs, slamResolver, entranceResolver, silent, replay, motion, drainWhenHidden, source]);

  // R3497000: a tab that turns visible with steps still queued shows the newest view at once.
  useEffect(() => {
    if (!drainWhenHidden || silent) return undefined;
    return source.subscribe(() => {
      if (!source.hidden() && !runner.idle()) runner.drain();
    });
  }, [drainWhenHidden, silent, source, runner]);

  // The moves `offered` are the newest view's; they apply once the board shows it (see the header).
  const legal = shown === view ? offered : NOTHING_LEGAL;
  // R2102000: and so does the lethal hint, which is the newest view's too.
  const lethalShown = lethalHint && !replay && shown === view && shown.result === null;

  // R3060000: the last End turn, answer or mulligan sent and the view it was sent against. The same body
  // against the same view is a repeat press, dropped; a new view is a new object, so it needs no clearing.
  const lastSent = useRef<{ key: string; view: PlayerView } | null>(null);
  // An `error` that gains text (a refusal, an offline send) means the intent did not land, so a retry goes
  // through. The line clearing is no refusal: net.ts clears it on every send, so releasing on that would let
  // the second click of a double click through whenever an old error was showing. A LAYOUT effect, so the
  // release is in place before the next click.
  useLayoutEffect(() => {
    if (error != null && error !== "") lastSent.current = null;
  }, [error]);

  const dispatch = useCallback(
    (body: ActionBody) => {
      if (ONCE_PER_VIEW.has(body.type)) {
        const key = JSON.stringify(body);
        const sent = lastSent.current;
        if (sent !== null && sent.view === latest.current && sent.key === key) return;
        lastSent.current = { key, view: latest.current };
      }
      setInteraction(IDLE);
      onAction(body);
    },
    [onAction],
  );

  // R655: picking up one of your Units to attack (a drag, or a click choosing the attacker) plays
  // its `attack` hook. Both lift only a Unit `legal` lets attack, and nothing about it is sent.
  const pickUp = usePickupSound(shown);
  const onLift = useCallback(
    (plan: DragPlan) => {
      if (plan.lifted.stage === "attacking") pickUp(plan.lifted.attackerId);
    },
    [pickUp],
  );

  // #3633: what a click reads, as the board last drew it. The handler reads it here rather than closing
  // over it, so it stays one function while the board shows one view, and the cards it is handed to
  // (Card.tsx, sameCardProps) are not drawn again whenever the shown view changes.
  const clickReads = useRef({ shown, legal, interaction });
  useLayoutEffect(() => {
    clickReads.current = { shown, legal, interaction };
  });
  const handleClick = useCallback(
    (target: ClickTarget) => {
      const { shown: board, legal: moves, interaction: building } = clickReads.current;
      // R821000: "Place N Plague Counters" is answered by a click on the board, not from a list.
      const placement = answerPlaguePlacement(board, moves, board.pending, target);
      if (placement !== null) {
        dispatch(placement);
        return;
      }
      const next = onClickTarget(board, moves, building, target);
      setInteraction(next.interaction);
      setNotice(next.notice === undefined ? null : { text: next.notice, view: board });
      if (next.interaction !== building && next.interaction.stage === "attacking") {
        pickUp(next.interaction.attackerId);
      }
      if (next.action !== undefined) onAction(next.action);
    },
    [onAction, dispatch, pickUp],
  );

  const handleControl = useCallback(
    (control: BoardControl) => {
      const body = onControl(legal, control);
      // No matching legal action means the control was greyed out; a click on it does nothing.
      if (body === undefined) return;
      // A concede cannot be taken back, so the control only asks; the dialog's Concede sends it.
      if (body.type === "concede") {
        setConcedeFor(view.viewer);
        return;
      }
      dispatch(body);
    },
    [legal, dispatch, view.viewer],
  );

  // A hand-over or the game's end drops the question for good (React's "adjust state while
  // rendering" pattern: no effect, so the dialog never shows for one frame on the wrong seat).
  if (concedeFor !== null && (concedeFor !== view.viewer || view.result !== null)) setConcedeFor(null);

  // Issue §2's board-state closes for an open emote menu: a pick or drag starting (targeting always
  // wins over the portrait) and a hotseat hand-over. Match end closes it as an effect, so the result
  // screen can still send (emotes are live "through the results screen").
  if (emoteMenu !== null && (interaction !== IDLE || emoteMenu.viewer !== shown.viewer)) {
    setEmoteMenu(null);
  }
  // #2924: drop a half-built play or attack when the shown view's turn, active seat or viewer
  // changes (React's "adjust state while rendering" pattern, as the concede dialog above does).
  // Keyed on the remembered shown key, never `shown` vs `view`: while an animation holds the
  // board back, `shown` is old and `view` is new, and comparing them would drop the selection
  // mid-animation on every animated turn change.
  const shownKey = `${String(shown.turn)}:${shown.active}:${shown.viewer}`;
  if (selectionKey !== shownKey) {
    setSelectionKey(shownKey);
    if (interaction.stage !== "idle") setInteraction(IDLE);
  }
  useEffect(() => {
    if (shown.result !== null) setEmoteMenu(null);
  }, [shown.result]);
  const concedeOpen = concedeFor !== null && concedeFor === view.viewer && view.result === null;
  /** Back to the control that opened the dialog, as a dialog should leave the focus (WAI-ARIA APG). */
  const refocusConcede = useCallback(() => {
    const concede = root.current?.querySelector<HTMLElement>(`[data-testid="${testid.concede}"]`) ?? null;
    // #2645: on a phone Concede is a row of the match menu, closed by now: back to its ⋯.
    const target =
      concede === null || concede.closest("[hidden]") !== null
        ? (root.current?.querySelector<HTMLElement>(`[data-testid="${testid.matchMenu}"]`) ?? null)
        : concede;
    target?.focus({ preventScroll: true });
  }, []);
  const cancelConcede = useCallback(() => {
    setConcedeFor(null);
    refocusConcede();
  }, [refocusConcede]);
  const confirmConcede = useCallback(() => {
    setConcedeFor(null);
    refocusConcede();
    // Sent as the intent it is, not looked up in `legal`: the board may be mid-animation, which
    // offers no moves for a moment, and the engine rules on a concede like any other action.
    dispatch({ type: "concede" });
  }, [dispatch, refocusConcede]);

  // R3477000: the aimed attack's outcomes, the forecast sent with the newest view; a board held back on an
  // older view shows none, as it offers no moves (`legal` above), and a replay none at all.
  const showOutcomes = useSetting("attackOutcomes");
  const outcomes = useMemo(
    () => (replay || !showOutcomes || shown !== view ? undefined : attackOutcomes(shown, interaction, forecast)),
    [replay, showOutcomes, shown, view, interaction, forecast],
  );
  // A replay lights nothing: not even the cells an open prompt in the view names (`pendingHighlight`).
  const computedHighlight = useMemo(
    () => withOutcomes(replay ? NO_HIGHLIGHT : highlightFor(shown, legal, interaction), outcomes),
    [replay, shown, legal, interaction, outcomes],
  );
  // #3633: the highlight the cards were drawn with while it lights the same things, so a new view that
  // only offers its moves later (`legal` held back above) draws no card again (Card.tsx, sameCardProps).
  const keptHighlight = useRef(computedHighlight);
  if (keptHighlight.current !== computedHighlight && !sameHighlight(keptHighlight.current, computedHighlight)) keptHighlight.current = computedHighlight;
  const highlight = keptHighlight.current;
  // R738: what this seat aims at goes to the opponent; theirs is drawn below until the game ends.
  useAimEmitter(shown, legal, interaction, aim?.emit);
  const animated = useMemo(
    () => [...burst, ...lingering].map((entry) => ({ frames: entry.frames, events: entry.events })),
    [burst, lingering],
  );

  /**
   * One hero's emote surface (R643–R644). The picker opens only on your own portrait and only while
   * nothing is being targeted: `onPortrait` is the click Hero took when its hero was NOT legal.
   */
  const heroEmotes = useCallback(
    (side: "you" | "opponent"): HeroEmotes | undefined => {
      if (emotes === undefined) return undefined;
      const player = sideView(shown, side).player;
      return {
        portrait: emotes.portraitOf(player),
        // R1343: the menu offers the viewer's own hand, the eight the session will send.
        hand: emotes.handOf(shown.viewer),
        show: emotes.visible(player),
        menu: emoteMenu?.side === side ? (side === "you" ? "emotes" : "mute") : null,
        inspect: emoteMenu?.side === side && emoteMenu.place === "inspect",
        muted: emotes.muted(player),
        gate: () => emotes.gate(shown.viewer),
        onPortrait: () => {
          if (interaction !== IDLE) return;
          setEmoteMenu((open) => (open?.side === side ? null : { side, viewer: shown.viewer, place: "panel" }));
        },
        onPortraitHold: () => {
          if (interaction !== IDLE) return;
          setEmoteMenu({ side, viewer: shown.viewer, place: "inspect" });
        },
        onPick: (emote) => {
          emotes.send(shown.viewer, emote);
        },
        onMute: () => emotes.mute(player),
        onCloseMenu: () => setEmoteMenu(null),
      };
    },
    [emotes, emoteMenu, shown, interaction],
  );

  // The newest turn event the burst has reached: the banner must not fall back to the held-back
  // view's turn before the board catches up (#37).
  const burstTurn = burst.flatMap((entry) => entry.events).findLast(isTurnEvent);
  const lastTurnEvent = burstTurn ?? [...shown.events].reverse().find(isTurnEvent);
  const banner = bannerText(
    shown,
    lastTurnEvent?.type,
    burstTurn?.type === "turnStarted" ? burstTurn.player : undefined,
    burstTurn?.type === "turnStarted" && burstTurn.extra === true,
  );
  // R1295000: the seat whose own cards wear the device's picks, for the layers drawn over the board.
  const wearer = wearCosmetics && !replay ? shown.viewer : null;
  // The entry's times for board.css: its numbers' scale, its slam's, its set piece's hold. #3633: written
  // onto their readers (rootVars.ts); on the root's own style they restyled the whole game as they changed.
  useRootVarValues(vars, {
    ...(impactFeel === undefined ? {} : {
      "--impact-number-scale": String(impactFeel.numberScale),
      "--impact-number-linger": `${String(impactFeel.numberLingerMs)}ms`,
    }),
    ...(inFlightSlam === undefined ? {} : {
      "--slam-anticipation": `${String(inFlightSlam.anticipationMs)}ms`,
      "--slam-land-ms": `${String(Math.round((inFlightMs - inFlightLead) * SLAM_LAND_AT))}ms`,
    }),
    ...(setPiece === undefined ? {} : {
      "--set-piece-hold": `${String(setPiece.holdMs)}ms`,
      "--set-piece-skip-delay": `${String(SET_PIECE_SKIP_HINT_MS)}ms`,
    }),
    ...(takeoverHold <= 0 ? {} : {
      "--takeover-land-ms": `${String(Math.round(inFlightLead + (inFlightMs - inFlightLead) * SLAM_LAND_AT))}ms`,
    }),
  });

  return (
    <div
      className="game"
      ref={attach}
      data-testid="game"
      data-rail-top={railTop ? "true" : undefined}
      data-replay={replay ? "true" : undefined}
      data-lethal-hint={lethalShown ? "true" : undefined}
      data-viewer={shown.viewer}
      data-speaking={speaking ? "true" : undefined}
      data-impact-tier={impactTier}
      data-slam-tier={inFlightSlam === undefined || inFlightSlam.tier === "tiny" ? undefined : inFlightSlam.tier}
      data-slam-side={inFlightSlam?.side}
      data-hit-stop={hitStop ? "true" : undefined}
      data-reduced-motion={reducedMotion ? "true" : undefined}
      data-card-motion={motion}
      data-dealing={opening !== null && motion === "full" ? "true" : undefined}
      data-riffle={opening !== null ? "you opponent" : inFlight?.riffle?.sides.join(" ")}
      data-takeover={takeoverHold > 0 ? "mythic" : undefined}
      // #3633: what board.css, animations.css, drag.css and prompt.css read off the game root, where they
      // read `.game:has(...)`: a card's play animating, the drag setting and the game's end.
      data-card-played={inFlight?.type === "cardPlayed" || [...(animating as AnimatingMap).values()].includes("cardPlayed") ? "true" : undefined}
      data-drag={dragToPlay ? "on" : "off"}
      data-game-over={shown.result !== null ? "true" : undefined}
      data-set-piece={setPiece?.key}
      data-set-piece-shape={setPieceShape}
      data-set-piece-sides={setPieceShape === undefined ? undefined : inFlight?.wipe?.sides.join(" ")}
    >
      {inFlight === null ? null : (
        <span data-testid="animation-queue" data-animating={inFlight.type} hidden aria-hidden="true" />
      )}
      {/* #2932: the rail in the top strip, with the turn banner beside it (board.css); the board gets none. */}
      {railTop ? <div className="board-rail board-rail--top">{boardRail}</div> : null}
      {banner !== null ? (
        <div
          className="turn-banner"
          data-testid={testid.banner}
          data-animating={animating.get(testid.banner)}
          role="status"
        >
          {banner}
        </div>
      ) : null}

      <Board
        view={shown}
        wearCosmetics={wearCosmetics && !replay}
        deckCosmetics={deckCosmetics}
        loadouts={loadouts}
        highlight={highlight}
        animating={animating}
        animated={animated}
        enemyLabel={enemyLabel}
        controlExtras={controlExtras}
        turnClock={turnClock}
        matchStatus={matchStatus}
        boardRail={railTop ? undefined : boardRail}
        heroAside={heroAside}
        market={replay ? undefined : { legal, onAction: dispatch }}
        showOfferDraw={showOfferDraw}
        boardNotices={
          <>
            {error != null && error !== "" ? (
              <p key={error} className="game-error" data-testid="action-error" role="alert">
                {t(error)}
              </p>
            ) : null}
            {notice !== null && notice.view === shown ? (
              <p key={notice.text} className="attack-reason" data-testid="attack-reason" role="status">
                {t(notice.text)}
              </p>
            ) : null}
            {boardNotice}
            {lethalShown ? (
              <p className="lethal-hint" data-testid={testid.lethalHint} role="status">
                {t("You have lethal")}
              </p>
            ) : null}
            {/* `view` is newest for this notice: it changes no board position, and its actions use
                the same held-back legal list as every other board control. */}
            <DrawOfferNotice
              view={view}
              legal={legal}
              animating={animating.get(animTestid.drawToast)}
              onAction={dispatch}
            />
          </>
        }
        sandDisabled={interaction.stage !== "idle"}
        draining={inFlight !== null}
        onClick={handleClick}
        onControl={handleControl}
        emotes={heroEmotes}
      />
      {/* R502: the effects read the newest view for a number no event carries, and the showcase holds a
          cast on draw up as the runner reaches it. */}
      {silent ? null : <FxLayer queue={runner} view={shown} latest={view} paused={hitStop} />}
      {/* R2787000: the standing look of every Field Spell and lasting modifier the board shows, begun and
          ended with the shown view; the newest view's events say which play added a modifier. */}
      {silent ? null : <AuraLayer view={shown} latest={view} />}
      {/* R1655000: the showcase of the viewer's own play and their mulligan wear their cards' picks, as the board does. */}
      <WearerProvider seat={wearer} loadouts={loadouts} viewer={shown.viewer}>
      {silent ? null : <CardShowcase view={view} queue={runner} />}
      {silent || still === null ? null : <TakeoverStill defId={still.defId} instanceId={still.instanceId} view={still.view} onDone={endStill} />}
      {/* R2785000: the first skip control there is, faded in SET_PIECE_SKIP_HINT_MS into the hold. */}
      {setPiece === undefined ? null : (
        <p className="set-piece-skip" data-testid="set-piece-skip" aria-hidden="true">
          {t("Tap to skip")}
        </p>
      )}
      <OpponentAim view={shown} aim={shown.result === null ? (aim?.opponent ?? null) : null} />

      {(replay && !replayPrompt) || promptOver(shown, burst, inFlight) ? null : (
        <Prompt
          view={shown}
          interaction={interaction}
          legal={legal}
          onAction={dispatch}
          onInteraction={setInteraction}
          animating={animating.get(animTestid.prompt)}
          lesson={lesson}
          // R2211000: a replay's prompt is drawn as offered and sends nothing.
          readOnly={replay}
          // R2739001: hovers go both ways until the game ends, as aims do.
          choiceHover={shown.result === null ? choiceHover : undefined}
          // Cancel backs out of a play still being built (R81). An engine prompt has paused the game
          // and must be answered, so it offers none: the button would do nothing.
          onCancel={shown.pending === null ? () => setInteraction(IDLE) : undefined}
        />
      )}
      </WearerProvider>
      {replay ? null : (
        <DragLayer
          view={shown}
          legal={legal}
          interaction={interaction}
          onInteraction={setInteraction}
          onAction={onAction}
          onLift={onLift}
        />
      )}

      {replay || !tradeBar ? null : <TradeBar container={host} view={shown} legal={legal} onTrade={dispatch} />}
      {sideline === undefined ? null : <SidelineStrip sideline={sideline} />}

      {/* R1834004: the opponent's revealed hand (R434) wears their seat's loadout, as their side does. */}
      {shown.result !== null ? (
        <WearerProvider seat={wearer} loadouts={loadouts} viewer={shown.viewer}>
          <GameResult
            result={shown.result}
            viewer={shown.viewer}
            form={resultForm}
            actions={resultActions}
            animating={animating.get(testid.result)}
            theirHand={theirHandOf(shown)}
            onOpenResult={onOpenResult}
            openResultTestId={openResultTestId}
          />
        </WearerProvider>
      ) : null}

      {concedeOpen ? <ConfirmConcede body={concedeBody} onConfirm={confirmConcede} onCancel={cancelConcede} /> : null}
      <FullScreenArtHost />
    </div>
  );
}
