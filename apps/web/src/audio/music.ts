// The music player (SPEC §10.11 "Music", R631): plays one track at a time into the engine's music
// bus, and moves between tracks musically.
//
//   voice gain ─┐
//   voice gain ─┴─▶ turn low-pass ─▶ turn gain ─▶ focus gain ─▶ engine music bus ─▶ duck ─▶ master
//
// A REQUEST NAMES WHAT SHOULD PLAY, never when. Asking for the track already playing does nothing,
// so a Mythic played again while its theme runs does not restart it. A change waits for the playing
// track's next bar line (at most MUSIC_BAR_WAIT_MAX_S away), then the new track fades in over
// MUSIC_FADE_S while the old one fades out, with no hard cut. A track that opens on a sting (a
// result, a Mythic theme, a station's match start) comes in at once rather than fading, so its
// first note lands. A sting hands off to the next track on its own last bar line, and a change that
// arrives while a sting plays replaces the track it hands off to, never the sting. A station's own
// tracks pick up within a match where they left off, on a bar line.
//
// The opponent's turn is a mix, not a track: a low-pass and a little less level, ramped. Losing
// focus (the page hidden, or the window blurred) fades the music out while `pauseMusicOnBlur` is
// on. Muted, or with the music at zero, nothing new loads or starts; the music picks up the moment
// it can be heard again.
//
// Like the engine, nothing is scheduled on a context that is not running, and the player never
// throws. A turn or focus change that arrives while the context is suspended is applied the moment
// it runs again. A track's file is fetched once (MUSIC_BYTES_MAX kept, compressed) and decoded only
// to play (MUSIC_DECODED_MAX kept); one that cannot be fetched or decoded is tried again when a
// request next names it or at the next turn boundary, never at every idle. No file is fetched
// or decoded while the board animates (B58): a preload fetches bytes only, and both wait for the
// burst to end. A sting whose file cannot be had is skipped, never waited on.

import {
  MUSIC_BAR_WAIT_MAX_S,
  MUSIC_BYTES_MAX,
  MUSIC_DECODED_MAX,
  MUSIC_FADE_S,
  MUSIC_FOCUS_TC_S,
  MUSIC_HANDOFF_FADE_S,
  MUSIC_LEAD_S,
  MUSIC_OPEN_LOWPASS_HZ,
  MUSIC_OPPONENT_GAIN,
  MUSIC_OPPONENT_LOWPASS_HZ,
  MUSIC_TURN_TC_S,
} from "./constants.ts";
import { getAudioEngine } from "./engine.ts";
import { MUSIC_MANIFEST, isStationTrack, musicUrl } from "./musicData.ts";
import { readAudioSettings, subscribeAudioSettings } from "./settings.ts";
import type { AudioEngine, AudioSettings, MusicManifest, MusicTrack } from "./types.ts";

export type MusicRequest = {
  /** The track that should play; null for silence. */
  track: string | null;
  /** A sting to play first, when the request starts from silence or the menu (a match's start). */
  intro?: string | null;
  /** The opponent's-turn mix on or off. */
  opponentTurn?: boolean;
};

export type MusicLogEntry = { kind: "start" | "stop"; track: string; at: number };

export type MusicPlayer = {
  request(request: MusicRequest): void;
  /** Forgets where each station track left off (a new match). */
  resetResume(): void;
  /** Fetches these tracks' files in the background, between animation bursts. */
  preload(ids: readonly string[]): void;
  setBusy(busy: boolean): void;
  /** The track playing or about to take over, else null. */
  current(): string | null;
  /** The track last asked for. */
  wanted(): string | null;
  opponentTurn(): boolean;
  log(): readonly MusicLogEntry[];
  dispose(): void;
};

/** Where focus comes from: the page's visibility and the window's focus, or a test's. */
export type FocusPort = { focused(): boolean; subscribe(listener: () => void): () => void };

export type MusicPlayerOptions = {
  engine?: () => AudioEngine;
  manifest?: MusicManifest;
  fetchBytes?: (url: string) => Promise<ArrayBuffer>;
  focus?: FocusPort | null;
  settings?: () => AudioSettings;
  subscribeSettings?: (listener: (s: AudioSettings) => void) => () => void;
};

const LOG_MAX = 100;
/** How long after a faded voice falls silent its source is stopped. */
const STOP_GRACE_S = 0.02;

async function defaultFetchBytes(url: string): Promise<ArrayBuffer> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`music: GET ${url} answered ${response.status}`);
  return response.arrayBuffer();
}

export function browserFocus(): FocusPort | null {
  if (typeof document === "undefined" || typeof window === "undefined") return null;
  return {
    focused: () => document.visibilityState !== "hidden" && document.hasFocus(),
    subscribe(listener) {
      document.addEventListener("visibilitychange", listener);
      window.addEventListener("blur", listener);
      window.addEventListener("focus", listener);
      return () => {
        document.removeEventListener("visibilitychange", listener);
        window.removeEventListener("blur", listener);
        window.removeEventListener("focus", listener);
      };
    },
  };
}

type Fade = { t0: number; v0: number; t1: number; v1: number };

type Voice = {
  id: string;
  track: MusicTrack;
  source: AudioBufferSourceNode;
  gain: GainNode;
  /** The context time at which the track's own time is 0 (its bar grid hangs off this). */
  origin: number;
  startAt: number;
  fade: Fade;
  stopping: boolean;
};

type Graph = { ctx: AudioContext; filter: BiquadFilterNode; turn: GainNode; focus: GainNode };

function barLength(track: MusicTrack): number {
  return (60 / track.bpm) * track.beatsPerBar;
}

function gainAt(fade: Fade, t: number): number {
  if (t <= fade.t0) return fade.v0;
  if (t >= fade.t1) return fade.v1;
  return fade.v0 + ((fade.v1 - fade.v0) * (t - fade.t0)) / (fade.t1 - fade.t0);
}

/** The track's own time at context time `t`, wrapped into its loop. */
function trackTime(voice: Voice, t: number): number {
  let u = t - voice.origin;
  const { loop, loopStart, loopEnd } = voice.track;
  if (loop && loopStart !== null && loopEnd !== null && u >= loopEnd) u = loopStart + ((u - loopStart) % (loopEnd - loopStart));
  return u;
}

export function createMusicPlayer(options: MusicPlayerOptions = {}): MusicPlayer {
  const engineOf = options.engine ?? getAudioEngine;
  const manifest = options.manifest ?? MUSIC_MANIFEST;
  const fetchBytes = options.fetchBytes ?? defaultFetchBytes;
  const focusPort = options.focus === undefined ? browserFocus() : options.focus;
  const settings = options.settings ?? readAudioSettings;
  const subscribeSettings = options.subscribeSettings ?? subscribeAudioSettings;

  let graph: Graph | null = null;
  let voices: Voice[] = [];
  let want: { track: string | null; intro: string | null } = { track: null, intro: null };
  let opponent = false;
  /** The turn mix and focus level the graph was last set to; null until the graph exists. */
  let appliedTurn: boolean | null = null;
  let appliedFocus: number | null = null;
  let busy = false;
  let disposed = false;
  let entries: MusicLogEntry[] = [];
  const resumeAt = new Map<string, number>();
  const bytes = new Map<string, Promise<ArrayBuffer | null>>();
  const decoded = new Map<string, AudioBuffer>();
  const decoding = new Map<string, Promise<AudioBuffer | null>>();
  /** Tracks whose file could not be fetched or decoded: tried again when a request names them anew, or at a turn boundary. */
  const failed = new Set<string>();
  let heldPreload: string[] = [];
  let watched: AudioEngine | null = null;
  let unwatch: (() => void) | null = null;
  const unsubscribeSettings = subscribeSettings(() => {
    applyFocus();
    sync();
  });
  const unsubscribeFocus = focusPort?.subscribe(() => applyFocus()) ?? null;

  function quietly(run: () => void): void {
    try {
      run();
    } catch {
      // Music is never worth an error: drop the change, keep the page.
    }
  }

  function push(entry: MusicLogEntry): void {
    entries.push(entry);
    if (entries.length > LOG_MAX) entries.splice(0, entries.length - LOG_MAX);
  }

  function engine(): AudioEngine {
    const e = engineOf();
    if (e !== watched) {
      unwatch?.();
      watched = e;
      unwatch = e.subscribeState(() => quietly(sync));
    }
    return e;
  }

  function audible(): boolean {
    const s = settings();
    return !s.muted && s.music > 0;
  }

  /* ----- graph ----- */

  function ensureGraph(): Graph | null {
    const out = engine().musicOutput();
    if (out === null) return null;
    if (graph !== null && graph.ctx === out.context) return graph;
    // A new context (a test swapped the engine): the old one's voices went with it.
    voices = [];
    const ctx = out.context;
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = opponent ? MUSIC_OPPONENT_LOWPASS_HZ : MUSIC_OPEN_LOWPASS_HZ;
    const turn = ctx.createGain();
    turn.gain.value = opponent ? MUSIC_OPPONENT_GAIN : 1;
    const focus = ctx.createGain();
    focus.gain.value = focusTarget();
    filter.connect(turn);
    turn.connect(focus);
    focus.connect(out.input);
    graph = { ctx, filter, turn, focus };
    appliedTurn = opponent;
    appliedFocus = focus.gain.value;
    return graph;
  }

  function running(): boolean {
    return graph !== null && graph.ctx.state === "running";
  }

  function focusTarget(): number {
    if (!settings().pauseMusicOnBlur || focusPort === null) return 1;
    return focusPort.focused() ? 1 : 0;
  }

  /** Ramps the focus level to where it should be, once the context runs (`sync` calls it again after a resume). */
  function applyFocus(): void {
    if (graph === null || !running()) return;
    const target = focusTarget();
    if (target === appliedFocus) return;
    appliedFocus = target;
    graph.focus.gain.setTargetAtTime(target, graph.ctx.currentTime, MUSIC_FOCUS_TC_S);
  }

  /** Ramps the turn mix to where it should be, once the context runs (`sync` calls it again after a resume). */
  function applyTurn(): void {
    if (graph === null || !running() || appliedTurn === opponent) return;
    appliedTurn = opponent;
    const t = graph.ctx.currentTime;
    graph.filter.frequency.setTargetAtTime(opponent ? MUSIC_OPPONENT_LOWPASS_HZ : MUSIC_OPEN_LOWPASS_HZ, t, MUSIC_TURN_TC_S);
    graph.turn.gain.setTargetAtTime(opponent ? MUSIC_OPPONENT_GAIN : 1, t, MUSIC_TURN_TC_S);
  }

  /* ----- loading ----- */

  function fetchCached(id: string): Promise<ArrayBuffer | null> {
    const cached = bytes.get(id);
    if (cached !== undefined) {
      bytes.delete(id);
      bytes.set(id, cached);
      return cached;
    }
    const pending = (async (): Promise<ArrayBuffer | null> => {
      try {
        return await fetchBytes(musicUrl(id));
      } catch {
        return null;
      }
    })();
    bytes.set(id, pending);
    // A failure is not kept: the next time the track is wanted, it is fetched again.
    void pending.then((raw) => {
      if (raw === null && bytes.get(id) === pending) bytes.delete(id);
    });
    while (bytes.size > MUSIC_BYTES_MAX) {
      const oldest = bytes.keys().next().value;
      if (oldest === undefined) break;
      bytes.delete(oldest);
    }
    return pending;
  }

  /** Decodes a track to play it; resolves null if its file cannot be had. */
  function decode(id: string): Promise<AudioBuffer | null> {
    const ready = decoded.get(id);
    if (ready !== undefined) {
      decoded.delete(id);
      decoded.set(id, ready);
      return Promise.resolve(ready);
    }
    const pending = decoding.get(id);
    if (pending !== undefined) return pending;
    const c = graph?.ctx ?? null;
    const job = (async (): Promise<AudioBuffer | null> => {
      try {
        const raw = await fetchCached(id);
        if (raw === null || c === null) return null;
        const buffer = await c.decodeAudioData(raw.slice(0));
        decoded.delete(id);
        decoded.set(id, buffer);
        while (decoded.size > MUSIC_DECODED_MAX) {
          const oldest = decoded.keys().next().value;
          if (oldest === undefined) break;
          decoded.delete(oldest);
        }
        return buffer;
      } catch {
        return null;
      } finally {
        decoding.delete(id);
      }
    })();
    decoding.set(id, job);
    return job;
  }

  /* ----- voices ----- */

  function top(): Voice | null {
    for (let i = voices.length - 1; i >= 0; i -= 1) {
      const v = voices[i];
      if (v !== undefined && !v.stopping) return v;
    }
    return null;
  }

  function startVoice(id: string, buffer: AudioBuffer, at: number, offset: number, fadeIn: number): Voice | null {
    const g = graph;
    const track = manifest.files[id];
    if (g === null || track === undefined) return null;
    const source = g.ctx.createBufferSource();
    source.buffer = buffer;
    if (track.loop && track.loopStart !== null && track.loopEnd !== null) {
      source.loop = true;
      source.loopStart = track.loopStart;
      source.loopEnd = track.loopEnd;
    }
    const gain = g.ctx.createGain();
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(1, at + fadeIn);
    source.connect(gain);
    gain.connect(g.filter);
    const voice: Voice = { id, track, source, gain, origin: at - offset, startAt: at, fade: { t0: at, v0: 0, t1: at + fadeIn, v1: 1 }, stopping: false };
    source.onended = () => {
      voices = voices.filter((v) => v !== voice);
      quietly(() => gain.disconnect());
    };
    source.start(at, offset);
    voices.push(voice);
    push({ kind: "start", track: id, at });
    return voice;
  }

  function fadeOut(voice: Voice, at: number, fade: number): void {
    if (voice.stopping || graph === null) return;
    voice.stopping = true;
    push({ kind: "stop", track: voice.id, at });
    if (at <= voice.startAt) {
      // Never heard: it is called off before its start.
      voices = voices.filter((v) => v !== voice);
      quietly(() => voice.source.stop(graph?.ctx.currentTime ?? 0));
      quietly(() => voice.gain.disconnect());
      return;
    }
    if (voice.track.loop && isStationTrack(voice.id)) {
      const bar = barLength(voice.track);
      resumeAt.set(voice.id, Math.floor(trackTime(voice, at) / bar + 1e-6) * bar);
    }
    const g = gainAt(voice.fade, at);
    const param = voice.gain.gain;
    param.cancelScheduledValues(at);
    if (voice.fade.t1 > at && voice.fade.t0 < at) param.linearRampToValueAtTime(g, at);
    else param.setValueAtTime(g, at);
    param.linearRampToValueAtTime(0, at + fade);
    voice.fade = { t0: at, v0: g, t1: at + fade, v1: 0 };
    voice.source.stop(at + fade + STOP_GRACE_S);
  }

  /** Fades a track in at a sting's speed if it opens on one, else at a crossfade's. */
  function fadeInFor(id: string, offset: number): number {
    const track = manifest.files[id];
    if (track === undefined) return MUSIC_FADE_S;
    return offset === 0 && (!track.loop || track.intro > 0) ? MUSIC_HANDOFF_FADE_S : MUSIC_FADE_S;
  }

  /** The playing track's next bar line from `soon`, unless that is further off than MUSIC_BAR_WAIT_MAX_S. */
  function nextBar(voice: Voice, now: number, soon: number): number {
    const bar = barLength(voice.track);
    const next = voice.origin + Math.ceil((soon - voice.origin) / bar - 1e-6) * bar;
    return next - now > MUSIC_BAR_WAIT_MAX_S ? soon : next;
  }

  /** Brings the speakers to what is wanted, loading first if needed. Never throws. */
  function sync(): void {
    if (disposed) return;
    quietly(() => {
      if (ensureGraph() === null || !running() || graph === null) return;
      // Whatever changed while the context was suspended lands now.
      applyTurn();
      applyFocus();
      const now = graph.ctx.currentTime;
      const soon = now + MUSIC_LEAD_S;
      const target = want.track;
      const active = voices.filter((v) => !v.stopping);
      if (target === null) {
        for (const v of active) fadeOut(v, now, MUSIC_FADE_S);
        return;
      }
      if (top()?.id === target) return;
      if (!audible() || manifest.files[target] === undefined) return;

      // What is sounding now, and what waits to follow it (a sting's next track).
      const waiting = active.filter((v) => v.startAt > soon);
      const sounding = active.filter((v) => v.startAt <= soon);
      const lead = sounding[sounding.length - 1] ?? null;
      const sting = lead !== null && !lead.track.loop && lead.track.handoff !== null ? lead : null;
      // A sting leads in only from silence or a screen's music, never inside a match's own music.
      const intro = want.intro !== null && manifest.files[want.intro] !== undefined && sting === null && (lead === null || !isStationTrack(lead.id)) ? want.intro : null;

      const missing = (intro === null ? [target] : [intro, target]).filter((id) => !decoded.has(id));
      // A track that failed waits for the next request that names it, not every idle (setBusy).
      if (missing.includes(target) && failed.has(target)) return;
      if (missing.length > 0) {
        // Nothing is fetched or decoded during an animation burst (B58); setBusy(false) comes back here.
        if (busy) return;
        for (const id of missing) {
          void decode(id).then((buffer) => {
            // A sting that cannot be had is skipped, so the track it leads into still plays.
            if (buffer === null) failed.add(id);
            if (buffer === null && id === want.intro) want = { ...want, intro: null };
            if (buffer !== null || id === intro) sync();
          });
        }
        return;
      }

      let at: number;
      let fadeIn: number;
      let offset = 0;
      if (sting !== null) {
        // A sting plays on: the new track takes the place of whatever was to follow it.
        at = waiting[0]?.startAt ?? Math.max(soon, sting.origin + (sting.track.handoff ?? 0));
        for (const v of waiting) fadeOut(v, at, MUSIC_FADE_S);
        fadeIn = MUSIC_HANDOFF_FADE_S;
      } else {
        if (waiting.length > 0) {
          // A change is already on its way to a bar line: this one takes its place there.
          at = waiting[0]?.startAt ?? soon;
          for (const v of waiting) fadeOut(v, at, MUSIC_FADE_S);
        } else {
          at = lead === null ? soon : nextBar(lead, now, soon);
          for (const v of sounding) fadeOut(v, at, MUSIC_FADE_S);
        }
        const introBuffer = intro === null ? undefined : decoded.get(intro);
        const started = intro === null || introBuffer === undefined ? null : startVoice(intro, introBuffer, at, 0, MUSIC_HANDOFF_FADE_S);
        if (intro !== null) want = { ...want, intro: null };
        // The target may be the very track fading out from `at` (a change undone before it landed):
        // the new copy then starts exactly where the old one is, so the two crossfade into one.
        const fading = voices.find((v) => v.stopping && v.id === target && Math.abs(v.fade.t0 - at) < 1e-6 && v.startAt < at);
        if (started !== null && started.track.handoff !== null) {
          at += started.track.handoff;
          fadeIn = MUSIC_HANDOFF_FADE_S;
        } else if (fading !== undefined) {
          offset = trackTime(fading, at);
          fadeIn = MUSIC_FADE_S;
        } else {
          offset = resumeAt.get(target) ?? 0;
          fadeIn = fadeInFor(target, offset);
        }
      }
      const buffer = decoded.get(target);
      if (buffer !== undefined) startVoice(target, buffer, at, offset, fadeIn);
    });
  }

  function pumpPreload(): void {
    if (busy || disposed || !audible()) return;
    const ids = heldPreload;
    heldPreload = [];
    for (const id of ids) if (manifest.files[id] !== undefined) void fetchCached(id);
  }

  return {
    request(request) {
      if (disposed) return;
      const intro = request.intro ?? null;
      const changed = request.track !== want.track;
      if (changed) {
        want = { track: request.track, intro };
        if (request.track !== null) failed.delete(request.track);
        if (intro !== null) failed.delete(intro);
      }
      const turn = request.opponentTurn ?? false;
      let retry = false;
      if (turn !== opponent) {
        opponent = turn;
        quietly(applyTurn);
        // A turn boundary is when a track that failed to load (a passing network error) is tried
        // again: often enough to recover within a match, rarely enough to cost nothing.
        retry = failed.size > 0;
        failed.clear();
      }
      if (changed || retry) sync();
    },
    resetResume() {
      resumeAt.clear();
    },
    preload(ids) {
      heldPreload = [...new Set([...heldPreload, ...ids])];
      pumpPreload();
    },
    setBusy(next) {
      busy = next;
      if (!busy) {
        pumpPreload();
        sync();
      }
    },
    current: () => top()?.id ?? null,
    wanted: () => want.track,
    opponentTurn: () => opponent,
    log: () => entries.slice(),
    dispose() {
      if (disposed) return;
      disposed = true;
      unsubscribeSettings();
      unsubscribeFocus?.();
      unwatch?.();
      for (const v of voices) {
        quietly(() => v.source.stop());
        quietly(() => v.gain.disconnect());
      }
      voices = [];
      entries = [];
    },
  };
}

let singleton: MusicPlayer | null = null;

/** The page's one player, made on first use against whichever engine `getAudioEngine()` returns. */
export function getMusicPlayer(): MusicPlayer {
  singleton ??= createMusicPlayer();
  return singleton;
}

/** Replace (or with null, drop) the singleton. Tests only. */
export function setMusicPlayerForTests(player: MusicPlayer | null): void {
  singleton?.dispose();
  singleton = player;
}
