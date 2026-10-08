// R631 (SPEC §10.11 "Music"): the music player, on the real engine over the strict fake Web Audio
// context. Changes wait for a bar line and crossfade; a sting hands off on its last bar line; the
// opponent's turn is a mix; a station track resumes; nothing is scheduled on a stopped context; and
// the engine ducks the music under voice lines and the big effects. R1350, R1351: a card's intro on
// top of the music, the duck under it, and what cuts it.

import { afterEach, beforeEach, describe, expect, it } from "vitest";

import {
  MUSIC_BAR_WAIT_MAX_S,
  MUSIC_DUCK_GAIN,
  MUSIC_FADE_S,
  MUSIC_HANDOFF_FADE_S,
  MUSIC_INTRO_CUT_FADE_S,
  MUSIC_INTRO_DUCK_GAIN,
  MUSIC_INTRO_LATE_S,
  MUSIC_LEAD_S,
  MUSIC_OPEN_LOWPASS_HZ,
  MUSIC_OPPONENT_GAIN,
  MUSIC_OPPONENT_LOWPASS_HZ,
} from "./constants.ts";
import { createAudioEngine } from "./engine.ts";
import { createMusicPlayer, type FocusPort, type MusicPlayer } from "./music.ts";
import { resetAudioSettingsForTests, writeAudioSettings } from "./settings.ts";
import { FakeFetch, fakeContextFactory, settle, type FakeAudio, type FakeNode } from "./test/fakeAudio.ts";
import type { AudioEngine, MusicManifest, MusicTrack } from "./types.ts";

/** Every track at 120 bpm in 4/4 has a two-second bar, which keeps the arithmetic readable. */
function loop(over: Partial<MusicTrack> = {}): MusicTrack {
  return { hash: "x", bytes: 1, bpm: 120, beatsPerBar: 4, loop: true, intro: 0, duration: 19.5, loopStart: 3, loopEnd: 19, handoff: null, ...over };
}

const MANIFEST: MusicManifest = {
  version: 1,
  format: "test",
  files: {
    menu: loop({ intro: 4, loopStart: 7, loopEnd: 23, duration: 23.5 }),
    "tavern-1": loop(),
    "tavern-danger": loop({ bpm: 90, loopEnd: 3 + 8 * (8 / 3), duration: 3.5 + 8 * (8 / 3) }),
    "tavern-start": { hash: "x", bytes: 1, bpm: 120, beatsPerBar: 4, loop: false, intro: 6, duration: 8.5, loopStart: null, loopEnd: null, handoff: 6 },
    // Six-second bars: a next bar line can be further off than MUSIC_BAR_WAIT_MAX_S.
    victory: loop({ bpm: 40, intro: 2, loopStart: 5, loopEnd: 29, duration: 29.5 }),
    // R1350: two cards' intros, two bars of music (4 s) and a second of ring-out.
    "intro-a": { hash: "x", bytes: 1, bpm: 120, beatsPerBar: 4, loop: false, intro: 4, duration: 5, loopStart: null, loopEnd: null, handoff: 4 },
    "intro-b": { hash: "x", bytes: 1, bpm: 120, beatsPerBar: 4, loop: false, intro: 4, duration: 5, loopStart: null, loopEnd: null, handoff: 4 },
  },
};

type Rig = { engine: AudioEngine; audio: FakeAudio; player: MusicPlayer; fetch: FakeFetch; focus: { focused: boolean; fire(): void } };

const live: { engine: AudioEngine; player: MusicPlayer }[] = [];

async function rig(options: { unlock?: boolean } = {}): Promise<Rig> {
  const factory = fakeContextFactory({ decodedSeconds: 30 });
  const engine = createAudioEngine({ createContext: factory.create, speech: null, visibility: () => "visible" });
  const fetch = new FakeFetch();
  const listeners = new Set<() => void>();
  const focus = {
    focused: true,
    fire() {
      for (const l of [...listeners]) l();
    },
  };
  const port: FocusPort = {
    focused: () => focus.focused,
    subscribe: (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
  };
  const player = createMusicPlayer({ engine: () => engine, manifest: MANIFEST, fetchBytes: fetch.fetchBytes, focus: port });
  live.push({ engine, player });
  if (options.unlock !== false) {
    engine.unlock();
    await settle();
  }
  return {
    engine,
    get audio() {
      return factory.last();
    },
    player,
    fetch,
    focus,
  };
}

/** The player's turn low-pass: the one biquad in the graph (no effect plays in these tests). */
function filterOf(audio: FakeAudio): FakeNode {
  const filters = audio.nodesOf("biquad");
  expect(filters).toHaveLength(1);
  const filter = filters[0];
  if (filter === undefined) throw new Error("no filter");
  return filter;
}

/** Started music sources, oldest first, each with the gain it plays through. */
function voices(audio: FakeAudio): { source: FakeNode; gain: FakeNode }[] {
  const filter = filterOf(audio);
  return audio
    .startedSources()
    .filter((n) => n.kind === "bufferSource" && audio.reaches(n, filter))
    .map((source) => {
      const gain = source.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
      if (gain === undefined) throw new Error("a source with no gain");
      return { source, gain };
    });
}

function rampsTo(gain: FakeNode, value: number): number[] {
  return gain
    .param("gain")
    .events.filter((e) => e.method === "linearRampToValueAtTime" && Math.abs(e.value - value) < 1e-9)
    .map((e) => ("time" in e ? e.time : Number.NaN));
}

const started = (r: Rig): string[] => r.player.log().filter((e) => e.kind === "start").map((e) => e.track);

beforeEach(() => {
  localStorage.clear();
  resetAudioSettingsForTests();
  writeAudioSettings({ muted: false, music: 0.5, duckMusic: true, playMusicInBackground: false });
});

afterEach(() => {
  for (const { engine, player } of live.splice(0)) {
    player.dispose();
    engine.dispose();
  }
});

describe("R631 starting and stopping", () => {
  it("R631 schedules nothing before the context runs, then starts what was asked for", async () => {
    const r = await rig({ unlock: false });
    r.player.request({ track: "menu" });
    await settle();
    expect(r.fetch.urls()).toEqual([]);
    expect(started(r)).toEqual([]);

    r.engine.unlock();
    await settle();
    expect(started(r)).toEqual(["menu"]);
    expect(r.audio.violations).toEqual([]);
  });

  it("R631 a track from silence starts at once and loops between the manifest's loop points", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    const [v] = voices(r.audio);
    expect(v?.source.startTime).toBeCloseTo(MUSIC_LEAD_S, 9);
    expect(v?.source.startOffset).toBe(0);
    expect(v?.source.loop).toBe(true);
    expect(v?.source.loopStart).toBe(3);
    expect(v?.source.loopEnd).toBe(19);
    expect(r.fetch.urls()).toEqual(["/audio/music/tavern-1.m4a"]);
    expect(r.player.current()).toBe("tavern-1");
  });

  it("R631 a track that opens on a sting comes in at once; one that does not fades in", async () => {
    const r = await rig();
    r.player.request({ track: "menu" });
    await settle();
    const [menu] = voices(r.audio);
    expect(rampsTo(menu?.gain as FakeNode, 1)).toEqual([MUSIC_LEAD_S + MUSIC_HANDOFF_FADE_S]);
  });

  it("R631 asking for the playing track again starts nothing new", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.player.request({ track: "tavern-1", opponentTurn: false });
    await settle();
    expect(voices(r.audio)).toHaveLength(1);
  });

  it("R631 silence fades everything out", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.audio.advance(4);
    r.player.request({ track: null });
    const [v] = voices(r.audio);
    expect(rampsTo(v?.gain as FakeNode, 0)).toEqual([r.audio.currentTime + MUSIC_FADE_S]);
    expect(v?.source.stopTime).toBeGreaterThan(r.audio.currentTime + MUSIC_FADE_S);
    expect(r.player.current()).toBeNull();
  });
});

describe("R631 changes land on a bar line and crossfade", () => {
  it("R631 waits for the playing track's next bar line, then crossfades over MUSIC_FADE_S", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.audio.advance(3.1);
    r.player.request({ track: "tavern-danger" });
    await settle();
    const [old, next] = voices(r.audio);
    // tavern-1 started at the lead; its bar lines fall every 2 s from there.
    const bar = MUSIC_LEAD_S + 4;
    expect(next?.source.startTime).toBeCloseTo(bar, 9);
    expect(rampsTo(next?.gain as FakeNode, 1)).toEqual([bar + MUSIC_FADE_S]);
    expect(rampsTo(old?.gain as FakeNode, 0)).toEqual([bar + MUSIC_FADE_S]);
    expect(old?.source.stopTime).toBeGreaterThan(bar + MUSIC_FADE_S);
    expect(r.audio.violations).toEqual([]);
  });

  it("R631 a change undone before its bar line carries the playing track on from where it is, with no restart", async () => {
    // The menu theme keeps no resume point (only a station's tracks do), so only this rule keeps it going.
    const r = await rig();
    r.player.request({ track: "menu" });
    await settle();
    r.audio.advance(3.1);
    r.player.request({ track: "tavern-1" });
    await settle();
    r.player.request({ track: "menu" });
    await settle();
    const bar = MUSIC_LEAD_S + 4;
    const [old, next, again] = voices(r.audio).map((v) => v.source);
    // The in-game track is called off before it is ever heard.
    expect(next?.stopTime).toBeLessThanOrEqual(next?.startTime ?? 0);
    // The new copy starts exactly where the old one is at the bar line, and the two crossfade.
    expect(again?.startTime).toBeCloseTo(bar, 9);
    expect(again?.startOffset).toBeCloseTo(bar - MUSIC_LEAD_S, 9);
    expect(old?.stopTime).toBeGreaterThan(bar + MUSIC_FADE_S);
    expect(rampsTo(voices(r.audio)[2]?.gain as FakeNode, 1)).toEqual([bar + MUSIC_FADE_S]);
    expect(r.player.current()).toBe("menu");
  });

  it("R631 never waits longer than MUSIC_BAR_WAIT_MAX_S for a bar line", async () => {
    const r = await rig();
    r.player.request({ track: "victory" });
    await settle();
    r.audio.advance(1);
    r.player.request({ track: "tavern-1" });
    await settle();
    const next = voices(r.audio)[1];
    // victory's next bar line is 6 s after its start, more than MUSIC_BAR_WAIT_MAX_S away.
    expect(6 - 1).toBeGreaterThan(MUSIC_BAR_WAIT_MAX_S);
    expect(next?.source.startTime).toBeCloseTo(1 + MUSIC_LEAD_S, 9);
  });

  it("R631 a station track picks up within the match where it left off, on a bar line", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.audio.advance(10);
    r.player.request({ track: "tavern-danger" });
    await settle();
    r.audio.advance(6);
    r.player.request({ track: "tavern-1" });
    await settle();
    const back = voices(r.audio)[2];
    // It left at its bar line 10.05 s after its start: track time 10.0, a whole number of bars.
    expect(back?.source.startOffset).toBeCloseTo(10, 9);
    r.player.resetResume();
    r.audio.advance(6);
    r.player.request({ track: "tavern-danger" });
    await settle();
    const danger = voices(r.audio)[3];
    expect(danger?.source.startOffset).toBe(0);
  });
});

describe("R631 a sting hands off", () => {
  it("R631 a match's sting leads in from the menu, and the in-game track starts on its last bar line", async () => {
    const r = await rig();
    r.player.request({ track: "menu" });
    await settle();
    r.audio.advance(1);
    r.player.request({ track: "tavern-1", intro: "tavern-start" });
    await settle();
    const [, sting, game] = voices(r.audio);
    const bar = MUSIC_LEAD_S + 2;
    expect(sting?.source.startTime).toBeCloseTo(bar, 9);
    expect(game?.source.startTime).toBeCloseTo(bar + 6, 9);
    expect(started(r)).toEqual(["menu", "tavern-start", "tavern-1"]);
  });

  it("R631 a change while the sting plays replaces what follows it, never the sting", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1", intro: "tavern-start" });
    await settle();
    r.audio.advance(2);
    r.player.request({ track: "tavern-danger" });
    await settle();
    const [sting, follower, danger] = voices(r.audio);
    expect(sting?.source.stopTime).toBeNull();
    // The follower is called off before its start, so it never sounds.
    expect(follower?.source.stopTime).toBeLessThanOrEqual(follower?.source.startTime ?? 0);
    expect(danger?.source.startTime).toBeCloseTo(MUSIC_LEAD_S + 6, 9);
  });

  it("R631 a sting never leads in mid-match", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.player.request({ track: "tavern-danger", intro: "tavern-start" });
    await settle();
    expect(started(r)).toEqual(["tavern-1", "tavern-danger"]);
  });
});

describe("R631 the turn mix, focus and audibility", () => {
  it("R631 the opponent's turn ramps a low-pass and the level down, and the viewer's turn back up", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    const filter = filterOf(r.audio);
    const turn = filter.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
    r.player.request({ track: "tavern-1", opponentTurn: true });
    expect(filter.param("frequency").targets().at(-1)?.value).toBe(MUSIC_OPPONENT_LOWPASS_HZ);
    expect(turn?.param("gain").targets().at(-1)?.value).toBe(MUSIC_OPPONENT_GAIN);
    r.player.request({ track: "tavern-1", opponentTurn: false });
    expect(filter.param("frequency").targets().at(-1)?.value).toBe(MUSIC_OPEN_LOWPASS_HZ);
    expect(turn?.param("gain").targets().at(-1)?.value).toBe(1);
    expect(voices(r.audio)).toHaveLength(1);
  });

  it("R631 muted, or at zero volume, nothing loads; the music starts once it can be heard", async () => {
    writeAudioSettings({ muted: true });
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    expect(r.fetch.urls()).toEqual([]);
    writeAudioSettings({ muted: false, music: 0 });
    await settle();
    expect(r.fetch.urls()).toEqual([]);
    writeAudioSettings({ music: 0.4 });
    await settle();
    expect(started(r)).toEqual(["tavern-1"]);
  });

  it("R631 losing focus silences the music unless playMusicInBackground is on", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    const filter = filterOf(r.audio);
    const turn = filter.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
    const focusGain = turn?.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
    r.focus.focused = false;
    r.focus.fire();
    expect(focusGain?.param("gain").targets().at(-1)?.value).toBe(0);
    writeAudioSettings({ playMusicInBackground: true });
    expect(focusGain?.param("gain").targets().at(-1)?.value).toBe(1);
    writeAudioSettings({ playMusicInBackground: false });
    r.focus.focused = true;
    r.focus.fire();
    expect(focusGain?.param("gain").targets().at(-1)?.value).toBe(1);
  });

  it("R631 a file that cannot be fetched plays nothing, throws nothing, and is fetched again next time", async () => {
    const r = await rig();
    r.fetch.mode = "reject";
    r.player.request({ track: "tavern-1" });
    await settle();
    expect(started(r)).toEqual([]);
    expect(r.audio.violations).toEqual([]);
    r.fetch.mode = "resolve";
    r.player.request({ track: null });
    r.player.request({ track: "tavern-1" });
    await settle();
    expect(started(r)).toEqual(["tavern-1"]);
    expect(r.fetch.urls()).toEqual(["/audio/music/tavern-1.m4a", "/audio/music/tavern-1.m4a"]);
  });

  it("R631 a track that cannot be had is not tried again at every idle, only when a request names it anew", async () => {
    const r = await rig();
    r.fetch.mode = "reject";
    r.player.request({ track: "tavern-1" });
    await settle();
    for (let i = 0; i < 5; i += 1) {
      r.player.setBusy(true);
      r.player.setBusy(false);
      await settle();
    }
    expect(r.fetch.urls()).toEqual(["/audio/music/tavern-1.m4a"]);
    r.audio.decodeMode = "reject";
    r.fetch.mode = "resolve";
    r.player.request({ track: "tavern-danger" });
    await settle();
    r.player.setBusy(true);
    r.player.setBusy(false);
    await settle();
    expect(r.audio.decodeCalls).toHaveLength(1);
  });

  it("R631 a track that failed once (a passing network error) is tried again at the next turn boundary", async () => {
    const r = await rig();
    r.fetch.mode = "reject";
    r.player.request({ track: "tavern-1" });
    await settle();
    r.fetch.mode = "resolve";
    r.player.request({ track: "tavern-1" });
    await settle();
    expect(started(r)).toEqual([]);
    r.player.request({ track: "tavern-1", opponentTurn: true });
    await settle();
    expect(started(r)).toEqual(["tavern-1"]);
    expect(r.fetch.urls()).toEqual(["/audio/music/tavern-1.m4a", "/audio/music/tavern-1.m4a"]);
  });

  it("R631 a sting that cannot be fetched is skipped, and the track it leads into still plays", async () => {
    const r = await rig();
    r.fetch.modes.set("/audio/music/tavern-start.m4a", "reject");
    r.player.request({ track: "tavern-1", intro: "tavern-start" });
    await settle();
    expect(started(r)).toEqual(["tavern-1"]);
  });

  it("R631 a turn or focus change made while the context is suspended lands when it runs again", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    const filter = filterOf(r.audio);
    const turn = filter.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
    const focusGain = turn?.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
    r.audio.state = "interrupted";
    r.player.request({ track: "tavern-1", opponentTurn: true });
    r.focus.focused = false;
    r.focus.fire();
    expect(filter.param("frequency").targets()).toEqual([]);
    expect(focusGain?.param("gain").targets()).toEqual([]);
    r.engine.unlock();
    await settle();
    expect(filter.param("frequency").targets().at(-1)?.value).toBe(MUSIC_OPPONENT_LOWPASS_HZ);
    expect(turn?.param("gain").targets().at(-1)?.value).toBe(MUSIC_OPPONENT_GAIN);
    expect(focusGain?.param("gain").targets().at(-1)?.value).toBe(0);
  });

  it("R631 nothing is fetched while the board animates; the music loads when the burst ends", async () => {
    const r = await rig();
    r.player.setBusy(true);
    r.player.request({ track: "tavern-1" });
    r.player.preload(["tavern-danger"]);
    await settle();
    expect(r.fetch.urls()).toEqual([]);
    r.player.setBusy(false);
    await settle();
    expect(started(r)).toEqual(["tavern-1"]);
    expect(r.fetch.urls()).toContain("/audio/music/tavern-danger.m4a");
  });
});

describe("R631 the engine ducks the music", () => {
  /** The duck: the gain the music bus feeds, which feeds master. */
  function duckOf(audio: FakeAudio): FakeNode {
    const filter = filterOf(audio);
    let node: FakeNode = filter;
    for (let hop = 0; hop < 3; hop += 1) {
      const next = node.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
      if (next === undefined) throw new Error("the music chain ends early");
      node = next;
    }
    // filter → turn → focus → music bus; the duck is the bus's own target.
    const duck = node.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
    if (duck === undefined) throw new Error("no duck");
    return duck;
  }

  it("R631 dips under a big effect and lets go after it, while duckMusic is on", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    const duck = duckOf(r.audio);
    expect(r.engine.playSfx("trapSting")).toBe(true);
    const targets = duck.param("gain").targets();
    expect(targets.map((t) => t.value)).toEqual([MUSIC_DUCK_GAIN, 1]);
    expect(targets[1]?.time).toBeGreaterThan(targets[0]?.time ?? Number.POSITIVE_INFINITY);
  });

  it("R631 a routine effect does not duck, and nothing ducks with duckMusic off", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    const duck = duckOf(r.audio);
    r.engine.playSfx("draw");
    writeAudioSettings({ duckMusic: false });
    r.engine.playSfx("trapSting");
    expect(duck.param("gain").targets()).toEqual([]);
  });
});

describe("R1350 a card's intro on top of the music", () => {
  /** The engine's music bus: what the player's chain feeds. */
  function busOf(r: Rig): FakeNode {
    const out = r.engine.musicOutput();
    if (out === null) throw new Error("no music bus");
    return r.audio.nodeOf(out.input);
  }

  /** The bed every track plays into, ahead of the turn's low-pass: what an intro ducks. */
  function bedOf(audio: FakeAudio): FakeNode {
    const [bed] = audio.inputsOf(filterOf(audio));
    if (bed === undefined) throw new Error("no bed");
    return bed;
  }

  /** Started intro sources, oldest first: on the music bus, but not through the turn's low-pass. */
  function clips(r: Rig): { source: FakeNode; gain: FakeNode }[] {
    const filter = filterOf(r.audio);
    const bus = busOf(r);
    return r.audio
      .startedSources()
      .filter((n) => n.kind === "bufferSource" && !r.audio.reaches(n, filter) && r.audio.reaches(n, bus))
      .map((source) => {
        const gain = source.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain");
        if (gain === undefined) throw new Error("an intro with no gain");
        return { source, gain };
      });
  }

  async function playing(): Promise<Rig> {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.audio.advance(2.5);
    return r;
  }

  it("R1350 plays once, at once, on the music bus beside the track playing and not through the opponent's-turn low-pass", async () => {
    const r = await playing();
    r.player.request({ track: "tavern-1", opponentTurn: true });
    r.player.playIntro("intro-a");
    await settle();
    const [clip] = clips(r);
    expect(clip?.source.startTime).toBeCloseTo(2.5 + MUSIC_LEAD_S, 9);
    expect(clip?.source.loop).toBe(false);
    expect(r.fetch.urls()).toContain("/audio/music/intro-a.m4a");
    expect(r.player.intro()).toBe("intro-a");
    // The track goes on under it.
    expect(voices(r.audio)).toHaveLength(1);
    expect(voices(r.audio)[0]?.source.stopTime).toBeNull();
    expect(r.player.current()).toBe("tavern-1");
    expect(r.audio.violations).toEqual([]);
  });

  it("R1350 ducks the bed to MUSIC_INTRO_DUCK_GAIN for its music, and lets it back up as its last bar ends", async () => {
    const r = await playing();
    r.player.playIntro("intro-a");
    await settle();
    const start = 2.5 + MUSIC_LEAD_S;
    const targets = bedOf(r.audio).param("gain").targets();
    expect(targets.map((t) => t.value)).toEqual([MUSIC_INTRO_DUCK_GAIN, 1]);
    expect(targets[0]?.time).toBeCloseTo(start, 9);
    expect(targets[1]?.time).toBeCloseTo(start + 4, 9);
    // A track that comes in meanwhile (a theme) comes in ducked: it plays into the same bed.
    r.player.request({ track: "tavern-danger" });
    await settle();
    const danger = voices(r.audio)[1];
    expect(danger !== undefined && r.audio.reaches(danger.source, bedOf(r.audio))).toBe(true);
  });

  it("R1350 follows the music's volume and mute: through the music bus, and nothing fetched or played muted or at zero", async () => {
    writeAudioSettings({ muted: true });
    const r = await playing();
    r.player.playIntro("intro-a");
    writeAudioSettings({ muted: false, music: 0 });
    r.player.playIntro("intro-a");
    await settle();
    expect(r.fetch.urls()).not.toContain("/audio/music/intro-a.m4a");
    writeAudioSettings({ music: 0.5 });
    await settle();
    // Nothing was saved up: an intro asked for unheard is gone.
    expect(r.player.intro()).toBeNull();
    r.player.playIntro("intro-a");
    await settle();
    const [clip] = clips(r);
    expect(clip !== undefined && r.audio.reaches(clip.source, busOf(r))).toBe(true);
  });

  it("R1350 holds nothing: the voice channel stays free, so practice's pacing never waits on it", async () => {
    const r = await playing();
    r.player.playIntro("intro-a");
    await settle();
    expect(r.player.intro()).toBe("intro-a");
    expect(r.engine.speaking()).toBe(false);
  });

  it("R1350 fetches the intros asked for ahead only between animation bursts, and plays from what it fetched", async () => {
    const r = await playing();
    r.player.setBusy(true);
    r.player.preloadIntros(["intro-a"]);
    await settle();
    expect(r.fetch.urls()).not.toContain("/audio/music/intro-a.m4a");
    r.player.setBusy(false);
    await settle();
    expect(r.fetch.urls().filter((u) => u === "/audio/music/intro-a.m4a")).toHaveLength(1);
    r.player.playIntro("intro-a");
    await settle();
    expect(clips(r)).toHaveLength(1);
    expect(r.fetch.urls().filter((u) => u === "/audio/music/intro-a.m4a")).toHaveLength(1);
  });
});

describe("R1351 what cuts an intro short", () => {
  function introGains(r: Rig): FakeNode[] {
    const filter = filterOf(r.audio);
    return r.audio
      .startedSources()
      .filter((n) => n.kind === "bufferSource" && !r.audio.reaches(n, filter))
      .map((n) => n.connections.find((c): c is FakeNode => "kind" in c && c.kind === "gain"))
      .filter((g): g is FakeNode => g !== undefined);
  }

  it("R1351 a second intro cuts the first with a MUSIC_INTRO_CUT_FADE_S fade, and the bed stays down for the second", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.player.playIntro("intro-a");
    await settle();
    r.audio.advance(2);
    r.player.playIntro("intro-b");
    await settle();
    const at = 2 + MUSIC_LEAD_S;
    const [first, second] = introGains(r);
    expect(rampsTo(first as FakeNode, 0)).toEqual([at + MUSIC_INTRO_CUT_FADE_S]);
    expect(r.audio.startedSources().filter((n) => n.kind === "bufferSource").at(-1)?.startTime).toBeCloseTo(at, 9);
    expect(second).toBeDefined();
    expect(r.player.intro()).toBe("intro-b");
    const bed = r.audio.inputsOf(filterOf(r.audio))[0];
    expect(bed?.param("gain").targets().at(-1)).toMatchObject({ value: 1 });
    expect(bed?.param("gain").targets().at(-1)?.time).toBeCloseTo(at + 4, 9);
    expect(started(r)).toEqual(["tavern-1", "intro-a", "intro-b"]);
  });

  it("R1351 the same intro asked for again while it plays or loads (copies of one card arriving together) changes nothing", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.fetch.modes.set("/audio/music/intro-a.m4a", "hang");
    r.player.playIntro("intro-a");
    r.player.playIntro("intro-a");
    expect(r.fetch.urls().filter((u) => u === "/audio/music/intro-a.m4a")).toHaveLength(1);
    r.fetch.release("/audio/music/intro-a.m4a");
    await settle();
    r.audio.advance(0.5);
    r.player.playIntro("intro-a");
    await settle();
    expect(introGains(r)).toHaveLength(1);
    expect(started(r)).toEqual(["tavern-1", "intro-a"]);
    // Once it is over, the card played again opens it again.
    r.audio.advance(5);
    r.player.playIntro("intro-a");
    await settle();
    expect(introGains(r)).toHaveLength(2);
  });

  it("R1351 stopIntro cuts it with the same fade and lets the bed straight back up", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.player.playIntro("intro-a");
    await settle();
    r.audio.advance(1);
    r.player.stopIntro();
    const [gain] = introGains(r);
    expect(rampsTo(gain as FakeNode, 0)).toEqual([1 + MUSIC_INTRO_CUT_FADE_S]);
    expect(r.player.intro()).toBeNull();
    const bed = r.audio.inputsOf(filterOf(r.audio))[0];
    expect(bed?.param("gain").targets().at(-1)).toMatchObject({ value: 1, time: 1 });
    expect(r.audio.violations).toEqual([]);
  });

  it("R1351 an intro asked for while the context is not running is dropped, never played on its resume", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.audio.state = "interrupted";
    r.player.playIntro("intro-a");
    r.engine.unlock();
    await settle();
    expect(r.player.intro()).toBeNull();
    expect(introGains(r)).toHaveLength(0);
  });

  it("R1351 an intro whose file is not ready MUSIC_INTRO_LATE_S after its moment is dropped", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.fetch.modes.set("/audio/music/intro-a.m4a", "hang");
    r.player.playIntro("intro-a");
    await settle();
    r.audio.advance(MUSIC_INTRO_LATE_S + 0.1);
    r.fetch.release("/audio/music/intro-a.m4a");
    await settle();
    expect(r.player.intro()).toBeNull();
    expect(introGains(r)).toHaveLength(0);
  });

  it("R1351 a stop while its file loads calls it off", async () => {
    const r = await rig();
    r.player.request({ track: "tavern-1" });
    await settle();
    r.fetch.modes.set("/audio/music/intro-a.m4a", "hang");
    r.player.playIntro("intro-a");
    r.player.stopIntro();
    r.fetch.release("/audio/music/intro-a.m4a");
    await settle();
    expect(introGains(r)).toHaveLength(0);
  });
});
