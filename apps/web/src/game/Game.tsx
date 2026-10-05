// The whole client in one component: board (M5-T1), pickers (M5-T2), animation runner (M5-T4).
//
// It holds exactly three pieces of state, and none of them is a rule:
//
//  - `interaction` — the play or attack the player is halfway through building. `actions.ts` is
//    the only thing that advances it, and it advances it by filtering the `legal` array the
//    engine handed us. Nothing here asks whether a card is affordable or a target is reachable.
//  - `queue` — the animation runner. BUILD M5-T4 says the view updates after the animation for an
//    event completes, so the view this component renders is the one the events were planned
//    against; it swaps to the newest view when the queue settles.
//  - `shown` — that held-back view.
//
// Everything else is `props.view` and `props.legal`. `onAction` goes straight out to the caller,
// which is the only thing that talks to the engine (CLAUDE.md rule 7). Two pieces of chrome sit
// beside the board and hold no rule either: the Concede control asks "Concede this game?" before it
// sends anything (ConfirmConcede.tsx; `concedeFor` is the seat that asked), and the draw offer's
// notices, with the answering seat's Accept and Decline, come from the view (DrawOffer.tsx).
//
// `legal` belongs to `props.view`, so it only reaches the board once the board SHOWS that view.
// While the runner holds it back (the AI's last attack still playing out, say), the board draws the
// older position, and the newer position's moves on it lit End turn and glowed the hand before the
// player could see the turn was theirs: a click there ended a turn they had not seen. So until the
// board catches up, every consumer of `legal` gets none.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactElement,
  type ReactNode,
} from "react";

import type { ActionBody, Aim, EmoteGate, EmoteId, GameEvent, PlayerId, PlayerView, PortraitId } from "@jackioh/shared";

import type { EmoteShow } from "../emotes/session.ts";
import type { HeroEmotes } from "./Hero.tsx";
import Board from "./Board.tsx";
import ConfirmConcede from "./ConfirmConcede.tsx";
import DrawOfferNotice from "./DrawOffer.tsx";
import Prompt from "./Prompt.tsx";
import { promptOver } from "./promptOver.ts";
import DragLayer from "./drag/DragLayer.tsx";
import OpponentAim from "./aim/OpponentAim.tsx";
import { useAimEmitter } from "./aim/useAimEmitter.ts";
import type { DragPlan } from "./drag/model.ts";
import { IDLE, highlightFor, onClickTarget, onControl, type Interaction } from "./actions.ts";
import {
  animTestid,
  createAnimationQueue,
  newEventsSince,
  type AnimationEntry,
  type AnimationQueue,
} from "./animations.ts";
import { sideView, testid, type BoardControl, type ClickTarget } from "./contract.ts";
import { damageFeel, damageTier, type DamageTier } from "./damageFeel.ts";
import { GameResult, theirHandOf, type ResultForm } from "./Result.tsx";
import { useOsReducedMotion } from "./useOsReducedMotion.ts";
import FxLayer from "../fx/FxLayer.tsx";
import CardShowcase from "./showcase/CardShowcase.tsx";
import { useSetting } from "../settings/index.ts";
import "./animations.css";
import { useGameAudio, usePickupSound, useVoiceSpeaking } from "../audio/index.ts";
import { useGameStats } from "../stats/useGameStats.ts";

/**
 * The `turnStarted` / `turnAutoEnded` banner. `Board` deliberately does not render it — one
 * `turn-banner` in the tree, and the shell owns it (M5-T4, `e2e/support/testids.ts` BANNER).
 */
function isTurnEvent(event: GameEvent): event is Extract<GameEvent, { type: "turnStarted" | "turnAutoEnded" }> {
  return event.type === "turnStarted" || event.type === "turnAutoEnded";
}

function bannerText(view: PlayerView, lastType: string | undefined, startedFor?: PlayerId): string | null {
  if (view.result !== null) return "Game over";
  if (lastType === "turnAutoEnded") return "No moves left. Turn ended.";
  // #37: while a `turnStarted` entry plays, the board is still the view from before the turn began,
  // so its banner names the turn that is starting, not the one (or the mulligan) being left.
  if (startedFor !== undefined) return startedFor === view.viewer ? "Your turn" : "Opponent's turn";
  if (view.phase === "mulligan") return "Mulligan";
  return view.active === view.viewer ? "Your turn" : "Opponent's turn";
}

export type GameProps = {
  view: PlayerView;
  legal: readonly ActionBody[];
  onAction: (body: ActionBody) => void;
  /** The engine's refusal for the last action, if any. `PlayerView` has no error channel. */
  error?: string | null;
  /** The route's ways on from a finished game, drawn in the result panel (Result.tsx). */
  resultActions?: ReactNode;
  /** `chip` when the route draws its own result dialog (practice); `panel` otherwise. */
  resultForm?: ResultForm;
  /**
   * R345: pins the R82 preference this board sends, whatever the player's "End turn automatically"
   * setting says. The tutorial pins it on, since its lessons are written around R82.
   */
  autoEndTurn?: boolean;
  /** Server-synchronised route clock, rendered by Board beside End Turn rather than as floating HUD. */
  turnClock?: ReactNode;
  /** Route connection status, rendered on the board rail rather than above it. */
  matchStatus?: ReactNode;
  /** Route navigation and match-series information, mounted in the board rail. */
  boardRail?: ReactNode;
  /** A route warning mounted in the board rail. */
  boardNotice?: ReactNode;
  /**
   * The route's emote session (R643–R644): portraits for both seats, what each is showing, the
   * send that admits through the shared gate, and the mutes. Absent, the heroes still draw their
   * default portraits and simply open no menus — tests that render a bare board get that.
   */
  emotes?: GameEmotes;
  /**
   * R639: log this game in the device's statistics when it ends. The routes that play the player's
   * own game (an online match, a practice game) set it; a tutorial lesson, a hotseat game on one
   * screen and a test fixture leave it off.
   */
  trackStats?: boolean;
  /**
   * R738: the opponent's aim. `emit` sends this seat's own as it changes; `opponent` is the
   * opponent's as last relayed, drawn as its arrow. Online play sets it; hotseat and tests do not.
   */
  aim?: GameAim;
};

/** What Game needs of a route that shows aims both ways (R738). */
export type GameAim = { emit: (aim: Aim | null) => void; opponent: Aim | null };

/**
 * What Game needs of a route's `useEmotes` (emotes/useEmotes.ts): the pure reads plus the two
 * verbs the portraits report. Every method takes a PlayerId, not a Side — the session keys on
 * seats so a hotseat hand-over moves no bubble.
 */
export type GameEmotes = {
  portraitOf: (player: PlayerId) => PortraitId;
  visible: (player: PlayerId) => EmoteShow | null;
  send: (player: PlayerId, emote: EmoteId) => boolean;
  gate: (player: PlayerId) => EmoteGate;
  muted: (player: PlayerId) => boolean;
  mute: (player: PlayerId) => void;
};

/** What the board is offered while it is still showing an older view than `legal` describes. */
const NOTHING_LEGAL: readonly ActionBody[] = [];

export default function Game({
  view,
  legal: offered,
  onAction,
  error,
  resultActions,
  resultForm = "panel",
  autoEndTurn: pinnedAutoEndTurn,
  turnClock,
  matchStatus,
  boardRail,
  boardNotice,
  emotes,
  trackStats = false,
  aim,
}: GameProps): ReactElement {
  const [interaction, setInteraction] = useState<Interaction>(IDLE);
  const root = useRef<HTMLDivElement>(null);
  /**
   * The seat whose Concede control asked "Concede this game?". The question is that seat's alone: a
   * hotseat hand-over, or a game that ends while it is open, closes it without a word.
   */
  const [concedeFor, setConcedeFor] = useState<PlayerId | null>(null);
  /**
   * Which portrait's menu is open (`emoteMenu.side`), bound to the viewer that opened it so a
   * hotseat hand-over closes it the way a hand-over closes the concede question. Issue §2's other
   * closes — outside press, Escape, picking — live in the menus themselves; the ones that are a
   * board state (a drag or targeting starting) are the render-adjust below.
   */
  const [emoteMenu, setEmoteMenu] = useState<{ side: "you" | "opponent"; viewer: PlayerId } | null>(null);

  // The view the DOM is showing: the newest one once the queue has settled, an older one while
  // an event is still animating over it (BUILD M5-T4).
  const [shown, setShown] = useState<PlayerView>(view);
  const latest = useRef<PlayerView>(view);
  latest.current = view;

  const [animating, setAnimating] = useState(() => new Map<string, never>());
  /**
   * The event the runner has in flight, which is NOT the same thing as `animating.size > 0`.
   *
   * BUILD M5-T4 puts `data-animating="<eventType>"` on the element that animates an event, and
   * several rows of its table resolve to an element the current view does not render at all — a
   * destroyed card, a face-down trap the viewer may not identify (the `trapFired` finding in
   * `animations.ts`), an event like `promptAnswered` whose target has already left the DOM. While
   * such an entry is in flight the runner is still holding the newest view back and yet nothing
   * carries the attribute, so "no element is animating" reads as "the board has caught up" when it
   * has not. `animation-queue` below is that missing element: it carries the in-flight event for
   * exactly as long as the runner has one, and it is `hidden`, so it animates nothing itself.
   */
  const [inFlight, setInFlight] = useState<AnimationEntry | null>(null);
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
  /**
   * Every entry the runner has started since the board last caught up.
   *
   * A `.damage-pop` / `.heal-pop` / `.loss-pop` shows the amount from the event being animated
   * (BUILD M5-T4), and one action routinely deals several: an attack pops a number on the
   * defender and then one on the attacker. Reading only the entry in flight would make each pop
   * vanish the instant the next entry starts, so a player watching a trade sees the first number
   * flash and disappear before the second arrives. The burst is kept whole instead — every pop it
   * produced stays up until the board catches up, which is also when `shown` swaps and the numbers
   * become the card's own stats.
   */
  const [burst, setBurst] = useState<readonly AnimationEntry[]>([]);
  const queue = useRef<AnimationQueue | null>(null);
  // #258: the OS preference is followed live, as the queue below follows the setting.
  const osReducesMotion = useOsReducedMotion();
  const reducedMotion = useSetting("reduceMotion") || osReducesMotion;
  const hitStopTimer = useRef<number | null>(null);
  const triggerHitStop = useCallback((amount: number): void => {
    const durationMs = damageFeel(amount).hitStopMs;
    if (durationMs <= 0) return;
    if (hitStopTimer.current !== null) window.clearTimeout(hitStopTimer.current);
    setHitStop(true);
    hitStopTimer.current = window.setTimeout(() => {
      hitStopTimer.current = null;
      setHitStop(false);
    }, durationMs);
  }, []);
  useEffect(
    () => () => {
      if (hitStopTimer.current !== null) window.clearTimeout(hitStopTimer.current);
      for (const timer of lingerTimers.current.values()) window.clearTimeout(timer);
      lingerTimers.current.clear();
    },
    [],
  );

  // R345: "End turn automatically" is the player's intent for R82, and the engine holds it, so it
  // goes out as an action whenever it differs from what this seat's view says. It is sent once per
  // seat and value: a refusal is not retried until the setting or the seat changes.
  const wantAutoEndTurn = useSetting("autoEndTurn");
  const autoEndTurn = pinnedAutoEndTurn ?? wantAutoEndTurn;
  const engineAutoEndTurn = view.autoEndTurn !== false;
  const autoEndTurnSent = useRef<string | null>(null);
  useEffect(() => {
    if (view.result !== null) return;
    if (autoEndTurn === engineAutoEndTurn) {
      autoEndTurnSent.current = null;
      return;
    }
    const sent = `${view.viewer}:${String(autoEndTurn)}`;
    if (autoEndTurnSent.current === sent) return;
    autoEndTurnSent.current = sent;
    onAction({ type: "setAutoEndTurn", enabled: autoEndTurn });
  }, [view.viewer, view.result, autoEndTurn, engineAutoEndTurn, onAction]);

  // Polish task 7: the "Reduce motion" setting does what the OS preference does, so every duration
  // is 0 and the queue drains synchronously (BUILD M5-T4). The queue reads it once, when it is
  // built, so a change of setting builds a new queue; the subscription effect below tears the old
  // one down and shows the newest view.
  // Hit-stop is a visual pause only. It never delays the queue, network, engine or turn clock;
  // reduced-motion suppresses shake/wobble but intentionally leaves this tactile pause in place.
  useEffect(() => {
    if (!reducedMotion && inFlightDamage !== undefined) triggerHitStop(inFlightDamage.amount);
  }, [inFlightDamage, reducedMotion, triggerHitStop]);
  const builtFor = useRef(reducedMotion);
  if (queue.current === null || builtFor.current !== reducedMotion) {
    builtFor.current = reducedMotion;
    queue.current = createAnimationQueue({
      reducedMotion,
      onSettled: () => {
        setShown(latest.current);
      },
    });
  }
  const runner = queue.current;
  useGameStats(view, trackStats);
  useGameAudio(runner, view); // before the layout effects below: it must see each view before the runner is fed (audio/useGameAudio.ts)
  // A voice line holding the channel marks the board `data-speaking`, the one attribute practice's
  // pacing reads to hold the AI's next step (SPEC §9.9); hotseat and online play simply carry it.
  const speaking = useVoiceSpeaking();

  // Also a layout effect, and declared before the one that enqueues, so the subscription is in
  // place before the very first batch of events is planned — a passive one here would run after
  // the layout effect below and miss the first entry's `data-animating`.
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

  // A new view arrives with the events that produced it. Plan them against the view they
  // describe, then let the runner decide when the board may show it.
  //
  // A LAYOUT effect, not a passive one: `data-animating` has to be on the DOM by the time the
  // click that caused the action has returned. A passive effect runs a scheduler tick later, and
  // in that gap the board says "nothing is animating" while still showing the previous view — so
  // anything that treats a drained queue as "the board is up to date" (BUILD M5-T4, and
  // `cy.settled()` in e2e/support) would read a view the engine has already moved past.
  const seen = useRef<PlayerView | null>(null);
  useLayoutEffect(() => {
    if (seen.current === view) return;
    const previous = seen.current;
    seen.current = view;

    // `view.events` is §10.8's sliding window over the whole match, not this action's delta, so
    // only the part of it the runner has not seen is enqueued (`newEventsSince`). Two views are
    // comparable only when they belong to the same seat: `viewFor` redacts per viewer, so after a
    // hotseat hand-over the windows have nothing in common and replaying them would mean the
    // arriving player watching the whole recent history. The first view of all is the same case —
    // the board it describes has simply always been there.
    const fresh =
      previous === null || previous.viewer !== view.viewer
        ? []
        : newEventsSince(previous.events, view.events);

    if (previous !== null && previous.viewer !== view.viewer) runner.drain();
    // Planned against the view the board is STILL SHOWING, not the one that has just arrived.
    // BUILD M5-T4: "the state view updates after the animation for that event completes", so an
    // event animates over the board as it was before it happened — which is the only board that
    // still has the card it destroys. Planning against the new view leaves `damage` and
    // `destroyed` with no element for the unit that just died, so nothing shakes and no number
    // pops on the very card the event is about.
    if (fresh.length > 0 && previous !== null) {
      if (reducedMotion) {
        const largest = fresh
          .filter((event): event is Extract<GameEvent, { type: "damage" }> => event.type === "damage")
          .reduce((amount, event) => Math.max(amount, event.amount), 0);
        triggerHitStop(largest);
      }
      runner.enqueue(fresh, previous);
    }
    if (runner.idle()) setShown(view);
  }, [view, runner, reducedMotion, triggerHitStop]);

  // The moves `offered` are the newest view's; they apply once the board shows it (see the header).
  const legal = shown === view ? offered : NOTHING_LEGAL;

  // A seat hand-over or a game over must not sit behind a queue of animations.
  const settleNow = useCallback(() => {
    runner.drain();
    setShown(latest.current);
  }, [runner]);

  useEffect(() => {
    if (view.result !== null) settleNow();
  }, [view.result, settleNow]);

  const dispatch = useCallback(
    (body: ActionBody) => {
      setInteraction(IDLE);
      onAction(body);
    },
    [onAction],
  );

  // R655: picking up one of your Units to attack plays its `attack` hook: a drag lifting it, or a
  // click choosing it as the attacker (click-click counts as a pick-up). Both lift only a Unit
  // `legal` lets attack, and nothing about it is sent.
  const pickUp = usePickupSound(shown);
  const onLift = useCallback(
    (plan: DragPlan) => {
      if (plan.lifted.stage === "attacking") pickUp(plan.lifted.attackerId);
    },
    [pickUp],
  );

  const handleClick = useCallback(
    (target: ClickTarget) => {
      const next = onClickTarget(shown, legal, interaction, target);
      setInteraction(next.interaction);
      if (next.interaction !== interaction && next.interaction.stage === "attacking") {
        pickUp(next.interaction.attackerId);
      }
      if (next.action !== undefined) onAction(next.action);
    },
    [shown, legal, interaction, onAction, pickUp],
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

  // Issue §2's board-state closes for an open emote menu: a pick or drag starting (interaction
  // leaving IDLE — targeting always wins over the portrait), and the hotseat hand-over that made
  // the open menu another seat's. Match end also closes it, as an effect so the result screen can
  // still send (emotes are live "through the results screen", issue §2).
  if (emoteMenu !== null && (interaction !== IDLE || emoteMenu.viewer !== shown.viewer)) {
    setEmoteMenu(null);
  }
  useEffect(() => {
    if (shown.result !== null) setEmoteMenu(null);
  }, [shown.result]);
  const concedeOpen = concedeFor !== null && concedeFor === view.viewer && view.result === null;
  /** Back to the control that opened the dialog, as a dialog should leave the focus (WAI-ARIA APG). */
  const refocusConcede = useCallback(() => {
    root.current?.querySelector<HTMLElement>(`[data-testid="${testid.concede}"]`)?.focus({ preventScroll: true });
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

  const highlight = useMemo(() => highlightFor(shown, legal, interaction), [shown, legal, interaction]);
  // R738: what this seat aims at goes to the opponent; theirs is drawn below until the game ends.
  useAimEmitter(shown, legal, interaction, aim?.emit);
  const animated = useMemo(
    () => [...burst, ...lingering].map((entry) => ({ frames: entry.frames, events: entry.events })),
    [burst, lingering],
  );

  /**
   * One hero's emote surface, built per side (R643–R644). The picker opens only on your own
   * portrait — in hotseat "you" is always the seat on move, which is the issue's "only the active
   * seat's portrait opens a menu" for free — and only while nothing is being targeted: `onPortrait`
   * is the click Hero took when its hero was NOT legal, and a selection in flight still wins.
   */
  const heroEmotes = useCallback(
    (side: "you" | "opponent"): HeroEmotes | undefined => {
      if (emotes === undefined) return undefined;
      const player = sideView(shown, side).player;
      return {
        portrait: emotes.portraitOf(player),
        show: emotes.visible(player),
        menu: emoteMenu?.side === side ? (side === "you" ? "emotes" : "mute") : null,
        muted: emotes.muted(player),
        gate: () => emotes.gate(shown.viewer),
        onPortrait: () => {
          if (interaction !== IDLE) return;
          setEmoteMenu((open) => (open?.side === side ? null : { side, viewer: shown.viewer }));
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

  // The newest turn event the burst has reached, the one in flight included: the board is still the
  // view from before it, and the banner must not fall back to that view's turn between its entry and
  // the board catching up (#37).
  const burstTurn = burst.flatMap((entry) => entry.events).findLast(isTurnEvent);
  const lastTurnEvent = burstTurn ?? [...shown.events].reverse().find(isTurnEvent);
  const banner = bannerText(shown, lastTurnEvent?.type, burstTurn?.type === "turnStarted" ? burstTurn.player : undefined);

  return (
    <div
      className="game"
      ref={root}
      data-testid="game"
      data-viewer={shown.viewer}
      data-speaking={speaking ? "true" : undefined}
      data-impact-tier={impactTier}
      data-hit-stop={hitStop ? "true" : undefined}
      data-reduced-motion={reducedMotion ? "true" : undefined}
      style={impactFeel === undefined ? undefined : {
        "--impact-shake-px": `${String(impactFeel.shakePx)}px`,
        "--impact-shake-ms": `${String(impactFeel.shakeMs)}ms`,
        "--impact-number-scale": String(impactFeel.numberScale),
        "--impact-number-linger": `${String(impactFeel.numberLingerMs)}ms`,
      } as CSSProperties}
    >
      {inFlight === null ? null : (
        <span data-testid="animation-queue" data-animating={inFlight.type} hidden aria-hidden="true" />
      )}
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
        highlight={highlight}
        animating={animating}
        animated={animated}
        turnClock={turnClock}
        matchStatus={matchStatus}
        boardRail={boardRail}
        boardNotices={
          <>
            {error != null && error !== "" ? (
              <p key={error} className="game-error" data-testid="action-error" role="alert">
                {error}
              </p>
            ) : null}
            {boardNotice}
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
        onClick={handleClick}
        onControl={handleControl}
        emotes={heroEmotes}
      />
      {/* R502: the effects read the newest view for a number no event carries, and the showcase holds a
          cast on draw up as the runner reaches it. */}
      <FxLayer queue={runner} view={shown} latest={view} paused={hitStop} />
      <CardShowcase view={view} queue={runner} />
      <OpponentAim view={shown} aim={shown.result === null ? (aim?.opponent ?? null) : null} />

      {promptOver(shown, burst, inFlight) ? null : (
        <Prompt
          view={shown}
          interaction={interaction}
          legal={legal}
          onAction={dispatch}
          onInteraction={setInteraction}
          animating={animating.get(animTestid.prompt)}
          // Cancel backs out of a play still being built (R81). An engine prompt has paused the game
          // and must be answered, so it offers none: the button would do nothing (the mulligan, a
          // Discover, a trigger's choice).
          onCancel={shown.pending === null ? () => setInteraction(IDLE) : undefined}
        />
      )}
      <DragLayer
        view={shown}
        legal={legal}
        interaction={interaction}
        onInteraction={setInteraction}
        onAction={onAction}
        onLift={onLift}
      />

      {shown.result !== null ? (
        <GameResult
          result={shown.result}
          viewer={shown.viewer}
          form={resultForm}
          actions={resultActions}
          animating={animating.get(testid.result)}
          theirHand={theirHandOf(shown)}
        />
      ) : null}

      {concedeOpen ? <ConfirmConcede onConfirm={confirmConcede} onCancel={cancelConcede} /> : null}
    </div>
  );
}
