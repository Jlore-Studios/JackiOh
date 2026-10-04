// The sound director (SPEC §10.11 "Timing"): turns the view stream into cues, in step with the
// animation runner.
//
// A view arrives with §10.8's sliding window of events. The director remembers the events it has
// not voiced yet (`owed`), each with the view it was planned against, and sends an event's cues
// only when the runner STARTS the entry that animates it, so sound and motion land together.
// Whatever the runner never starts — every entry under reduced motion, `gameOver`'s zero-length
// entry, a queue drained at a game's end or a skip — is flushed once, condensed, when the runner
// goes idle. A condensed burst plays at most FLUSH_MAX_SFX plain effects, one card effect and one
// line: the most important of each (R655).
//
// R203: the first view, and a view for a different seat (a hotseat hand-over), voices nothing and
// drops everything owed, so the arriving seat hears nothing its own view did not produce.
//
// WHICH EVENTS ARE NEW. The director diffs two views with the runner's own `newEventsSince`
// (game/animations.ts), so what it owes is exactly what Game enqueues. Two windows overlap where the
// tail of the older one is the head of the newer one, but not always byte for byte: R97 judges a
// card by where it sits NOW, so when the opponent plays a card it drew earlier in the window, that
// older `drawn` (or `shuffledIn`, …) changes from the sentinel to the card's real id between the two
// views, and `sameOccurrence` lets it match. Every event of a view the director has already seen and
// not owed also stays silent should the runner ever start it (`known`): old news does not speak twice.
//
// Events are matched by object identity: `newEventsSince` and `planEntries` both hand out the very
// objects in `view.events`, so the entry the runner starts carries the same objects the director
// owes.
//
// PLAYS IN PROGRESS (R506). The director resolves every event it voices in stream order, so it can
// follow the plays the stream opens and closes: a `cardPlayed` opens one, its `cardResolved` closes
// it, and a turn's start forgets any left open. A cue reads the innermost one (`playing`), which is
// how #21 Hinder's rider and #27's Radiant cards know whose they are. A `cardPlayed` that directly
// follows a readable `drawn` of the same card (a `cardAnnounced`, a Gifted Program `radiantSet` or
// a cost change of that card may come between) is that card cast as it was drawn. Both read only
// what the stream shows the viewer: a sentinel play opens a frame no moment matches, and a sentinel
// draw marks nothing.

import type { GameEvent, PlayerId, PlayerView, UnitView } from "@jackioh/shared";

import { newEventsSince, type AnimationEntry } from "../game/animations.ts";
import { FLUSH_GAP_MS, FLUSH_MAX_SFX, HIDDEN_DEF_ID, PAIR_OFFSET_MS, PLAY_STACK_MAX } from "./constants.ts";
import { cuesFor, type CueCard, type CueContext, type PlayFrame } from "./cues.ts";
import type { CardAudioTable, SoundCue, SoundSink } from "./types.ts";
import { CARD_AUDIO } from "./voiceData.ts";

export type SoundDirector = {
  /** Feed every newest view (Game's props.view). */
  onView(view: PlayerView): void;
  /** The runner has just started this entry. */
  onEntryStart(entry: AnimationEntry): void;
  /** The runner is idle (inFlight() === null). */
  onIdle(): void;
  /** Events seen but not yet voiced (tests). */
  owedCount(): number;
};

type Owed = { event: GameEvent; view: PlayerView };

function findUnit(view: PlayerView | null, instanceId: string): UnitView | null {
  if (view === null) return null;
  for (const side of [view.you, view.opponent]) {
    for (const u of side.units) if (u !== null && u.instanceId === instanceId) return u;
  }
  return null;
}

/**
 * Events that may come between a card's `drawn` and its cast's `cardPlayed` (R506): the card's own
 * announce, Radiance or cost, and the prompts its cast asks the drawer (#21 Hinder's discard).
 */
function keepsDrawnCard(event: GameEvent, drawn: { instanceId: string; player: PlayerId }): boolean {
  switch (event.type) {
    case "cardAnnounced":
    case "radiantSet":
    case "costChanged":
      return event.instanceId === drawn.instanceId || drawn.instanceId === HIDDEN_DEF_ID;
    case "promptOpened":
    case "promptAnswered":
      return event.player === drawn.player;
    default:
      return false;
  }
}

function sideMana(view: PlayerView, player: PlayerId): number {
  if (view.you.player === player) return view.you.mana.current;
  if (view.opponent.player === player) return view.opponent.mana.current;
  return 0;
}

/**
 * `card` is the public catalog (`useGameAudio` reads the board's `CatalogContext`), asked only for
 * a defId the viewer can read; without it every card makes its type's plain sounds. `observe` hears
 * every event as it is resolved, with the view it was planned against, in the same step as its cues:
 * the music director's casts and hero hits (R631).
 */
export function createSoundDirector(
  sink: SoundSink,
  lines: CardAudioTable = CARD_AUDIO,
  card?: (defId: string) => CueCard | undefined,
  observe?: (event: GameEvent, view: PlayerView) => void,
): SoundDirector {
  let seen: PlayerView | null = null;
  let owed: Owed[] = [];
  const voiced = new WeakSet<GameEvent>();
  /** Every event object of every view fed in: one the director did not owe is old news. */
  const known = new WeakSet<GameEvent>();
  const lastMana = new Map<PlayerId, number>();
  /** Instances whose `cardPlayed` has sounded, so their `summoned` does not speak again (R204). */
  const played = new Set<string>();
  /** R506: the plays in progress, innermost last. */
  let plays: PlayFrame[] = [];
  /**
   * R506: the card the last event drew, while nothing else has happened since. A draw behind the
   * sentinel counts too: what it drew is named only by its own public cast, as the effects layer
   * reads it (fx/castOnDraw.ts), and the sting it gets names nothing (R203).
   */
  let drawnLast: { instanceId: string; player: PlayerId } | null = null;

  function ctx(view: PlayerView, castOnDraw: string | null): CueContext {
    return {
      view,
      lines,
      manaBefore: (player) => lastMana.get(player) ?? sideMana(view, player),
      wasPlayed: (instanceId) => played.has(instanceId),
      unitNow: (instanceId) => findUnit(seen, instanceId) ?? findUnit(view, instanceId),
      newestView: () => seen,
      playing: () => plays[plays.length - 1] ?? null,
      castOnDraw: (instanceId) => castOnDraw !== null && instanceId === castOnDraw,
      ...(card === undefined ? {} : { card }),
    };
  }

  /** R506: a `cardPlayed` of the card a readable `drawn` has just drawn, by the same player. */
  function castAsDrawn(event: GameEvent): string | null {
    if (event.type !== "cardPlayed" || drawnLast === null) return null;
    const same = event.instanceId === drawnLast.instanceId || drawnLast.instanceId === HIDDEN_DEF_ID;
    return same && event.player === drawnLast.player ? event.instanceId : null;
  }

  /** R506: the plays in progress and the card just drawn, moved on past `event`. */
  function follow(event: GameEvent, castOnDraw: boolean): void {
    switch (event.type) {
      case "turnStarted":
        plays = [];
        drawnLast = null;
        return;
      case "drawn":
        drawnLast = { instanceId: event.instanceId, player: event.player };
        return;
      case "cardPlayed":
        drawnLast = null;
        plays.push({ defId: event.defId, instanceId: event.instanceId, player: event.player, castOnDraw });
        if (plays.length > PLAY_STACK_MAX) plays = plays.slice(plays.length - PLAY_STACK_MAX);
        return;
      case "cardResolved": {
        drawnLast = null;
        let at = -1;
        plays.forEach((frame, i) => {
          if (frame.instanceId === event.instanceId) at = i;
        });
        if (at >= 0) plays.splice(at, 1);
        return;
      }
      default:
        if (drawnLast !== null && !keepsDrawnCard(event, drawnLast)) drawnLast = null;
    }
  }

  /** The event's cues against `view`, then the mana baseline, the played set and the plays move on (B22). */
  function resolve(event: GameEvent, view: PlayerView): readonly SoundCue[] {
    const cast = castAsDrawn(event);
    const cues = cuesFor(event, ctx(view, cast));
    if (event.type === "manaChanged") lastMana.set(event.player, event.current);
    if (event.type === "cardPlayed" && event.instanceId !== HIDDEN_DEF_ID) played.add(event.instanceId);
    follow(event, cast !== null);
    observe?.(event, view);
    return cues;
  }

  function send(cue: SoundCue, delayMs: number): void {
    if (cue.kind === "sfx") sink.playSfx(cue.id, cue.params, delayMs);
    else if (cue.kind === "effect") sink.playEffect(cue.defId, cue.hook, delayMs);
    else sink.playVoice(cue.defId, cue.line, delayMs, cue.priority);
  }

  function isOwed(event: GameEvent): boolean {
    return owed.some((item) => item.event === event);
  }

  function flush(items: readonly Owed[]): void {
    const effects: SoundCue[] = [];
    let endCue: SoundCue | null = null;
    let line: Extract<SoundCue, { kind: "voice" }> | null = null;
    let cardEffect: Extract<SoundCue, { kind: "effect" }> | null = null;

    for (const item of items) {
      voiced.add(item.event);
      for (const cue of resolve(item.event, item.view)) {
        if (cue.kind === "voice") {
          // The most important line of the burst, the first of those on a tie.
          if (line === null || cue.priority > line.priority) line = cue;
        } else if (cue.kind === "effect") {
          // R655: a card's effect is chosen as its line is: the burst's most important, the first on a tie.
          if (cardEffect === null || cue.priority > cardEffect.priority) cardEffect = cue;
        } else if (item.event.type === "gameOver") {
          if (endCue === null) endCue = cue;
        } else {
          effects.push(cue);
        }
      }
    }

    // First cue of each SfxId, in stream order. The game-over stinger is held back for the end, so
    // a same-id cue earlier in the burst gives way to it.
    const ids = new Set<string>();
    if (endCue !== null && endCue.kind === "sfx") ids.add(endCue.id);
    const unique: SoundCue[] = [];
    for (const cue of effects) {
      if (cue.kind !== "sfx" || ids.has(cue.id)) continue;
      ids.add(cue.id);
      unique.push(cue);
    }
    const kept = unique.slice(0, endCue === null ? FLUSH_MAX_SFX : FLUSH_MAX_SFX - 1);
    if (endCue !== null) kept.push(endCue);
    kept.forEach((cue, j) => {
      send(cue, j * FLUSH_GAP_MS);
    });

    // One card effect and one voice: the burst's most important of each, at its own delay.
    if (cardEffect !== null) send(cardEffect, cardEffect.delayMs);
    if (line !== null) send(line, line.delayMs);
  }

  return {
    onView(view) {
      if (view === seen) return;
      if (seen === null || seen.viewer !== view.viewer) {
        owed = [];
        lastMana.clear();
        played.clear();
        plays = [];
        drawnLast = null;
        for (const event of view.events) known.add(event);
        seen = view;
        return;
      }
      const planned = seen;
      for (const event of newEventsSince(planned.events, view.events)) {
        if (voiced.has(event) || isOwed(event)) continue;
        owed.push({ event, view: planned });
      }
      for (const event of view.events) known.add(event);
      seen = view;
    },

    onEntryStart(entry) {
      const first = entry.events[0];
      if (first === undefined) return;
      const at = owed.findIndex((item) => item.event === first);
      if (at > 0) flush(owed.splice(0, at));

      entry.events.forEach((event, k) => {
        const index = owed.findIndex((item) => item.event === event);
        const item = index >= 0 ? owed[index] : undefined;
        if (index >= 0) owed.splice(index, 1);
        if (voiced.has(event)) return;
        // A view's event the director did not owe is old news the runner is replaying: silent.
        if (item === undefined && known.has(event)) return;
        voiced.add(event);
        const view = item?.view ?? seen;
        if (view === null) return;
        const offset = k > 0 ? PAIR_OFFSET_MS : 0;
        for (const cue of resolve(event, view)) send(cue, cue.delayMs + offset);
      });
    },

    onIdle() {
      if (owed.length === 0) return;
      flush(owed.splice(0, owed.length));
    },

    owedCount() {
      return owed.length;
    },
  };
}
