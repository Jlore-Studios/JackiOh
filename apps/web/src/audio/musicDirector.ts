// The music director (SPEC §10.11, R631; musicPlan.ts) follows one board and only the viewer's own
// screen (R203). It applies after the board settles and switches stations at readable cards' R204
// moments; a Trap counts at its first firing. No card plays a theme (R822000): a readable Legendary's
// or Mythic's intro (R1350) is its music, and R1351 stops it on result, hand-over, dynamic off or exit.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { HERO_HEALTH } from "@jackioh/engine/config";

import { HIDDEN_DEF_ID, MUSIC_LOW_HEALTH_FRACTION } from "./constants.ts";
import type { MusicRequest } from "./music.ts";
import { MUSIC_CARDS, MUSIC_MANIFEST, RESULT_TRACKS, dangerTrack, nextRotation, startTrack } from "./musicData.ts";
import { chooseMusic, type MusicMoment } from "./musicPlan.ts";
import { readAudioSettings, subscribeAudioSettings } from "./settings.ts";
import type { AudioSettings, CardAudioTable, MusicCardEntry, MusicManifest, MusicStation } from "./types.ts";
import { CARD_AUDIO, entryFor } from "./voiceData.ts";

/** Where the director's choices go: the scene, which plays them while this board holds the music. */
export type MusicSink = {
  request(request: MusicRequest): void;
  resetResume(): void;
  preload(ids: readonly string[]): void;
  /** R1350: a card's intro, on top of the music. */
  playIntro(id: string): void;
  /** R1351: the intro playing is cut short. */
  stopIntro(): void;
  /** R1350: intros the viewer may soon play, fetched ahead. */
  preloadIntros(ids: readonly string[]): void;
};

export type MusicDirector = {
  /** Every newest view (Game's props.view). */
  onView(view: PlayerView): void;
  /** The runner is idle: the newest view is what the board shows. */
  settle(): void;
  /** An event the sound director has just resolved, against the view it was planned on. */
  onEvent(event: GameEvent, view: PlayerView): void;
  dispose(): void;
};

export type MusicDirectorOptions = {
  sink: MusicSink;
  manifest?: MusicManifest;
  cards?: Readonly<Record<string, MusicCardEntry>>;
  lines?: CardAudioTable;
  settings?: () => AudioSettings;
  subscribeSettings?: (listener: (s: AudioSettings) => void) => () => void;
  /** The rotation the match whose first view this is uses (default: `rotationFor`). */
  rotation?: (first: PlayerView) => number;
};

/** The viewer's hero at or under this is at low health. */
export const LOW_HEALTH_AT = Math.floor(HERO_HEALTH * MUSIC_LOW_HEALTH_FRACTION);

/** The rotation each match took, by its first view. */
const rotations = new WeakMap<PlayerView, number>();

/**
 * The device's counter, moved on once per match. A board that mounts twice on the same first view
 * (React's StrictMode does, in development) gets the same rotation rather than moving it twice.
 */
export function rotationFor(first: PlayerView): number {
  const known = rotations.get(first);
  if (known !== undefined) return known;
  const n = nextRotation();
  rotations.set(first, n);
  return n;
}

/**
 * The card a cast names and who cast it, at R204's moments: a unit or spell's play, a trap's first
 * firing. `fired` remembers the trap instances that have fired this match.
 */
function castOf(event: GameEvent, lines: CardAudioTable, fired: Set<string>): { defId: string; player: PlayerId } | null {
  if (event.type === "cardPlayed") {
    if (event.defId === HIDDEN_DEF_ID) return null;
    // A trap's play is its set, face down: it is cast when it fires.
    return entryFor(lines, event.defId)?.kind === "trap" ? null : { defId: event.defId, player: event.player };
  }
  if (event.type === "trapFired") {
    if (event.defId === HIDDEN_DEF_ID || fired.has(event.instanceId)) return null;
    fired.add(event.instanceId);
    return { defId: event.defId, player: event.controller };
  }
  return null;
}

/**
 * R1350: a Unit an effect put onto the field without playing it (a token, a copy, Recruit, Reborn),
 * which R204 gives its play line on its `summoned`: on a units row, readable, and no play of it heard.
 */
function arrivalOf(event: GameEvent, playedUnits: ReadonlySet<string>): string | null {
  if (event.type !== "summoned" || event.row !== "units" || event.defId === HIDDEN_DEF_ID) return null;
  return playedUnits.has(event.instanceId) ? null : event.defId;
}

export function createMusicDirector(options: MusicDirectorOptions): MusicDirector {
  const { sink } = options;
  const manifest = options.manifest ?? MUSIC_MANIFEST;
  const cards = options.cards ?? MUSIC_CARDS;
  const lines = options.lines ?? CARD_AUDIO;
  const settings = options.settings ?? readAudioSettings;
  const subscribeSettings = options.subscribeSettings ?? subscribeAudioSettings;
  const rotationOf = options.rotation ?? rotationFor;

  let viewer: PlayerId | null = null;
  let latest: PlayerView | null = null;
  let rotation = 0;
  const stations = new Map<PlayerId, MusicStation>();
  const fired = new Set<string>();
  /** R1350: the Units whose own play was heard, so their `summoned` opens nothing again (R204). */
  const playedUnits = new Set<string>();
  /** R1350: the intros already asked to be fetched ahead. */
  const preloaded = new Set<string>();
  let lowHealth = false;
  let opponentTurn = false;
  let result: MusicMoment["result"] = null;
  let intro: string | null = null;
  let disposed = false;

  function station(): MusicStation {
    const own = settings().station;
    if (!settings().dynamicMusic || viewer === null) return own;
    return stations.get(viewer) ?? own;
  }

  function push(): void {
    if (disposed || viewer === null) return;
    const dynamic = settings().dynamicMusic;
    if (!dynamic) sink.stopIntro();
    const choice = chooseMusic({ station: station(), rotation, dynamic, result, lowHealth, opponentTurn }, manifest);
    sink.request({ track: choice.track, intro, opponentTurn: choice.opponentTurn });
    intro = null;
  }

  function read(view: PlayerView): void {
    viewer = view.viewer;
    lowHealth = view.you.hero.health > 0 && view.you.hero.health <= LOW_HEALTH_AT;
    const inPlay = view.phase === "start" || view.phase === "main" || view.phase === "end";
    opponentTurn = inPlay && view.active !== view.viewer;
    result = view.result === null ? null : view.result.winner === "draw" ? "draw" : view.result.winner === view.viewer ? "victory" : "defeat";
    if (result !== null) sink.stopIntro();
  }

  /** R1350: the intros of the cards in the viewer's own hand (every one readable), fetched ahead once each. */
  function preloadHand(view: PlayerView): void {
    if (!settings().dynamicMusic || !Array.isArray(view.you.hand)) return;
    const ids: string[] = [];
    for (const card of view.you.hand) {
      const id = cards[card.defId]?.intro;
      if (id === undefined || preloaded.has(id)) continue;
      preloaded.add(id);
      ids.push(id);
    }
    if (ids.length > 0) sink.preloadIntros(ids);
  }

  const unsubscribe = subscribeSettings(() => push());

  return {
    onView(view) {
      if (disposed) return;
      latest = view;
      if (viewer === null) {
        // The match's first view: its rotation, a fresh station memory, and the sting (dynamic only).
        rotation = rotationOf(view);
        sink.resetResume();
        read(view);
        const s = station();
        if (settings().dynamicMusic && view.result === null && view.turn <= 1) intro = startTrack(s);
        push();
        // What this match may need next; the sting and the in-game track were just asked for.
        sink.preload([dangerTrack(s), ...Object.values(RESULT_TRACKS)]);
        return;
      }
      if (view.viewer !== viewer) {
        // A hotseat hand-over: the arriving seat's own music, at once, and nothing of the last seat's.
        sink.stopIntro();
        playedUnits.clear();
        read(view);
        push();
      }
    },

    settle() {
      if (disposed || latest === null) return;
      read(latest);
      push();
      preloadHand(latest);
    },

    onEvent(event, view) {
      if (disposed || viewer === null || view.viewer !== viewer) return;
      const cast = castOf(event, lines, fired);
      if (cast !== null && event.type === "cardPlayed" && entryFor(lines, cast.defId)?.kind === "unit") playedUnits.add(event.instanceId);
      if (!settings().dynamicMusic) return;
      // R1350: the card's intro first, so the station it may switch to comes in under it.
      const opens = cast?.defId ?? arrivalOf(event, playedUnits);
      const intro = opens === null ? undefined : cards[opens]?.intro;
      if (intro !== undefined) sink.playIntro(intro);
      const switched = cast === null ? undefined : cards[cast.defId]?.station;
      if (cast === null || switched === undefined) return;
      stations.set(cast.player, switched);
      if (cast.player === viewer) sink.preload([dangerTrack(switched)]);
      push();
    },

    dispose() {
      if (disposed) return;
      disposed = true;
      sink.stopIntro();
      unsubscribe();
    },
  };
}
