// The game's music director (SPEC §10.11 "Music", R631): follows one mounted board and tells the
// music what the viewer's situation calls for (musicPlan.ts holds the priority stack).
//
// Everything comes from what the viewer's own screen shows (R203). Each player's music is their own:
// their station, their hero's health, their turn.
//
// - The newest view is held, and applied when the animation runner goes idle (`settle`), so the
//   urgency track and the opponent's-turn mix follow the board as it is drawn, not ahead of it.
// - Events arrive as the sound director resolves them, in step with their animations: a cast of a
//   card in music-cards.json (at the moment its cast line would speak, R204, and only when the
//   viewer can read it) starts its Mythic theme or switches its caster's station; a hit on the
//   viewer's hero ends a Mythic theme.
// - A Mythic theme holds until the viewer's hero is next hit, another Mythic replaces it, or the game
//   ends. Away from low health it also ends once it has played MUSIC_MYTHIC_PLAYS times; at low
//   health it holds until that hit, then the urgency track comes back.
// - A match starts on its station's sting into the in-game track this match's rotation picks, unless
//   the first view is already past the first turn (a reconnect). The urgency track and the result
//   stings are fetched ahead, between animation bursts. A card's station switch lasts the match:
//   the next match starts on the setting again.
// - A Field Trap fires again and again (Classic+ #74 on every fuse); its theme starts at its first
//   firing only, as a cast would.
// - A hotseat hand-over is a new viewer: the arriving seat hears its own station and situation, and
//   no theme carries over.

import type { GameEvent, PlayerId, PlayerView } from "@jackioh/shared";
import { HERO_HEALTH } from "@jackioh/engine/config";

import { HIDDEN_DEF_ID, MUSIC_LOW_HEALTH_FRACTION, MUSIC_MYTHIC_PLAYS } from "./constants.ts";
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
  /** A timer, returning its cancel: a Mythic theme's end away from low health. */
  later?: (ms: number, fn: () => void) => () => void;
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

function defaultLater(ms: number, fn: () => void): () => void {
  const id = setTimeout(fn, ms);
  return () => clearTimeout(id);
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

function heroHit(event: GameEvent, viewer: PlayerId): boolean {
  if (event.type === "damage") return event.amount > 0 && event.targetId === `hero-${viewer}`;
  if (event.type === "healthLost") return event.amount > 0 && event.player === viewer;
  return false;
}

export function createMusicDirector(options: MusicDirectorOptions): MusicDirector {
  const { sink } = options;
  const manifest = options.manifest ?? MUSIC_MANIFEST;
  const cards = options.cards ?? MUSIC_CARDS;
  const lines = options.lines ?? CARD_AUDIO;
  const settings = options.settings ?? readAudioSettings;
  const subscribeSettings = options.subscribeSettings ?? subscribeAudioSettings;
  const rotationOf = options.rotation ?? rotationFor;
  const later = options.later ?? defaultLater;

  let viewer: PlayerId | null = null;
  let latest: PlayerView | null = null;
  let rotation = 0;
  const stations = new Map<PlayerId, MusicStation>();
  const fired = new Set<string>();
  let lowHealth = false;
  let opponentTurn = false;
  let result: MusicMoment["result"] = null;
  /** The Mythic theme holding the music; `spent` once it has played its MUSIC_MYTHIC_PLAYS times. */
  let theme: { defId: string; track: string; spent: boolean; cancel: () => void } | null = null;
  let intro: string | null = null;
  let disposed = false;

  function station(): MusicStation {
    const own = settings().station;
    if (!settings().dynamicMusic || viewer === null) return own;
    return stations.get(viewer) ?? own;
  }

  function endTheme(): void {
    theme?.cancel();
    theme = null;
  }

  function push(): void {
    if (disposed || viewer === null) return;
    const dynamic = settings().dynamicMusic;
    if (!dynamic) endTheme();
    // A spent theme lets go as soon as the viewer is away from low health.
    if (theme?.spent === true && !lowHealth) endTheme();
    const choice = chooseMusic(
      { station: station(), rotation, dynamic, result, theme: theme?.track ?? null, lowHealth, opponentTurn },
      manifest,
    );
    sink.request({ track: choice.track, intro, opponentTurn: choice.opponentTurn });
    intro = null;
  }

  function read(view: PlayerView): void {
    viewer = view.viewer;
    lowHealth = view.you.hero.health > 0 && view.you.hero.health <= LOW_HEALTH_AT;
    const inPlay = view.phase === "start" || view.phase === "main" || view.phase === "end";
    opponentTurn = inPlay && view.active !== view.viewer;
    result = view.result === null ? null : view.result.winner === "draw" ? "draw" : view.result.winner === view.viewer ? "victory" : "defeat";
    if (result !== null) endTheme();
  }

  function startTheme(defId: string, track: string): void {
    endTheme();
    const entry = manifest.files[track];
    const { loopStart, loopEnd } = entry ?? { loopStart: null, loopEnd: null };
    // Its first pass runs to the loop's end; each further play is one more loop.
    const seconds = loopStart === null || loopEnd === null ? 0 : loopEnd + (MUSIC_MYTHIC_PLAYS - 1) * (loopEnd - loopStart);
    const mine = { defId, track, spent: false, cancel: () => {} };
    mine.cancel = later(seconds * 1000, () => {
      if (theme !== mine) return;
      mine.spent = true;
      push();
    });
    theme = mine;
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
        // A hotseat hand-over: the arriving seat's own music, at once.
        endTheme();
        read(view);
        push();
      }
    },

    settle() {
      if (disposed || latest === null) return;
      read(latest);
      push();
    },

    onEvent(event, view) {
      if (disposed || viewer === null || view.viewer !== viewer) return;
      if (heroHit(event, viewer)) {
        if (theme !== null) {
          endTheme();
          push();
        }
        return;
      }
      const cast = castOf(event, lines, fired);
      if (cast === null || !settings().dynamicMusic) return;
      const entry = cards[cast.defId];
      if (entry === undefined) return;
      if (entry.station !== undefined) {
        stations.set(cast.player, entry.station);
        if (cast.player === viewer) sink.preload([dangerTrack(entry.station)]);
      }
      // The same Mythic again while its theme plays changes nothing.
      if (entry.theme !== undefined && theme?.defId !== cast.defId) startTheme(cast.defId, entry.theme);
      push();
    },

    dispose() {
      if (disposed) return;
      disposed = true;
      endTheme();
      unsubscribe();
    },
  };
}
