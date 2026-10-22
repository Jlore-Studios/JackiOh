// The XP reveal (#1664), from crafted answers: a track fills from where a game found it, its level
// pops as the bar bursts and the server's rewards for it fly out one by one with the sting; levels
// chain in order, each bar the size the answer gives; a quest the game completed is listed with its
// pay; reduced motion shows the end state and every reward at once. The audio engine is the recording
// fake the opening bars' tests use; fake timers move everything. No number here is a shipped one: a
// test that finds one on screen has found the answer's.

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";

import { setAudioEngineForTests } from "../audio/engine.ts";
import type { AudioEngine } from "../audio/types.ts";
import { resetFxSettingsForTests } from "../fx/settings.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { BURST_MS, REVEAL_STAGGER_MS, segmentMs } from "./bars.ts";
import { parseGameEconomy, stagesOf, type GameEconomy, type QuestPaid, type TrackGain, type TrackPoint } from "./xp.ts";
import XpReveal from "./XpReveal.tsx";

type FakeEngine = { [K in keyof AudioEngine]: Mock<AudioEngine[K]> };

function fakeEngine(): FakeEngine {
  return {
    state: vi.fn<AudioEngine["state"]>(() => "running"),
    unlock: vi.fn<AudioEngine["unlock"]>(),
    preloadVoices: vi.fn<AudioEngine["preloadVoices"]>(),
    setBusy: vi.fn<AudioEngine["setBusy"]>(),
    log: vi.fn<AudioEngine["log"]>(() => []),
    clearLog: vi.fn<AudioEngine["clearLog"]>(),
    contextsCreated: vi.fn<AudioEngine["contextsCreated"]>(() => 0),
    speaking: vi.fn<AudioEngine["speaking"]>(() => false),
    subscribeSpeaking: vi.fn<AudioEngine["subscribeSpeaking"]>(() => () => undefined),
    musicOutput: vi.fn<AudioEngine["musicOutput"]>(() => null),
    subscribeState: vi.fn<AudioEngine["subscribeState"]>(() => () => undefined),
    dispose: vi.fn<AudioEngine["dispose"]>(),
    playSfx: vi.fn<AudioEngine["playSfx"]>(() => true),
    playVoice: vi.fn<AudioEngine["playVoice"]>(() => true),
    playEffect: vi.fn<AudioEngine["playEffect"]>(() => true),
    playPickup: vi.fn<AudioEngine["playPickup"]>(() => true),
  };
}

let engine: FakeEngine;

beforeEach(() => {
  vi.useFakeTimers();
  engine = fakeEngine();
  setAudioEngineForTests(engine);
  __resetSettingsForTests();
  resetFxSettingsForTests();
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  vi.restoreAllMocks();
  vi.useRealTimers();
  __resetSettingsForTests();
  resetFxSettingsForTests();
  window.localStorage.clear();
});

const ID = "xp-reveal";
const SET = "Core";

/** Fake time in steps short enough for each timer's effect to run before the next one is due. */
function advance(ms: number): void {
  for (let left = ms; left > 0; left -= 25) {
    act(() => {
      vi.advanceTimersByTime(Math.min(25, left));
    });
  }
}

/** A point on a track, as the answer gives it: `have` XP into a level that spans `span`. */
function at(level: number, xp: number, span: number | null, have = 0): TrackPoint {
  return { level, xp, levelXp: span, xpToNext: span === null ? null : span - have };
}

const NOTHING = { jlore: 0, setPacks: 0, cardTokens: 0 };

/** Level 1 → 2: from 60 of a 100 span to 30 of a 200 span, level 2 paying 25 Jlore, 2 packs and 3 tokens. */
function oneLevel(): TrackGain {
  return {
    setId: SET,
    before: at(1, 60, 100, 60),
    after: at(2, 130, 200, 30),
    levels: [{ level: 2, rewards: { jlore: 25, setPacks: 2, cardTokens: 3 } }],
  };
}

/** Level 1 → 3: from 60 of 100 to 20 of 300, level 2 paying 25 Jlore and level 3 one pack. */
function twoLevels(): TrackGain {
  return {
    setId: SET,
    before: at(1, 60, 100, 60),
    after: at(3, 320, 300, 20),
    levels: [
      { level: 2, rewards: { ...NOTHING, jlore: 25 } },
      { level: 3, rewards: { ...NOTHING, setPacks: 1 } },
    ],
  };
}

const QUEST: QuestPaid = { questId: "win-3", title: "Win 3 games", jlore: 80, xp: 4000 };

function show(tracks: TrackGain[], quests: QuestPaid[] = []): ReturnType<typeof render> {
  return render(<XpReveal economy={{ tracks, quests }} />);
}

const bar = (): HTMLElement => screen.getByTestId(`${ID}-bar-${SET}`);
const track = (): HTMLElement => screen.getByTestId(`${ID}-track-${SET}`);
const level = (): HTMLElement => screen.getByTestId(`${ID}-level-${SET}`);
const progress = (): HTMLProgressElement => bar().querySelector("progress")!;

/** The rewards on screen, in the order they flew out. */
function rewards(): string[] {
  return Array.from(screen.getByTestId(`${ID}-rewards-${SET}`).querySelectorAll("li")).map((item) => item.textContent ?? "");
}

describe("#1664 a track that gains a level", () => {
  it("fills from where the game found it, pops the level as the bar bursts and reveals the server's rewards", () => {
    show([oneLevel()]);

    // From 60 of 100, the first bar's size, towards a full bar; the level is still the old one.
    expect(track()).toHaveAttribute("data-state", "filling");
    expect(bar()).toHaveAttribute("data-state", "filling");
    expect(bar()).toHaveTextContent("Core track");
    expect(progress().max).toBe(100);
    expect(level()).toHaveAttribute("data-level", "1");
    expect(level()).toHaveTextContent("Level 1 of 100");
    expect(level()).not.toHaveClass("xp-reveal__level--pop");
    expect(rewards()).toEqual([]);

    // It bursts: the level pops and the rewards start to fly out.
    advance(segmentMs({ from: 60, to: 100, burst: true }, 100));
    expect(bar()).toHaveAttribute("data-state", "bursting");
    expect(level()).toHaveAttribute("data-level", "2");
    expect(level()).toHaveTextContent("Level 2 of 100");
    expect(level()).toHaveClass("xp-reveal__level--pop");
    expect(rewards()).toEqual([]);

    // One at a time, Jlore, set packs, then card tokens, each in the collection's words and sting.
    advance(REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["25 Jlore"]);
    expect(engine.playSfx).toHaveBeenLastCalledWith("sting");

    // The burst ends and the next level's bar starts empty, of the size the answer gives it.
    advance(BURST_MS - REVEAL_STAGGER_MS + 25);
    expect(track()).toHaveAttribute("data-state", "filling");
    expect(bar()).toHaveAttribute("data-state", "filling");
    expect(progress().max).toBe(200);
    expect(progress().value).toBe(30);
    expect(level()).toHaveAttribute("data-level", "2");
    expect(rewards()).toEqual(["25 Jlore"]);

    // It rests where the answer says, the rest of the rewards still to fly out, each with its sting.
    advance(segmentMs({ from: 0, to: 30, burst: false }, 200));
    expect(track()).toHaveAttribute("data-state", "done");
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(bar()).toHaveTextContent("30 / 200");
    advance(REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["25 Jlore", "2 × Core pack"]);
    expect(engine.playSfx).toHaveBeenLastCalledWith("sting", { tier: "rare" });
    advance(REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["25 Jlore", "2 × Core pack", "3 × card token"]);
    expect(engine.playSfx).toHaveBeenCalledTimes(3);
  });

  it("a track that gains XP inside a level only fills", () => {
    show([{ setId: SET, before: at(4, 60, 100, 60), after: at(4, 90, 100, 90), levels: [] }]);

    expect(progress().max).toBe(100);
    expect(progress().value).toBe(90);
    expect(bar()).toHaveAttribute("data-state", "filling");
    advance(segmentMs({ from: 60, to: 90, burst: false }, 100));
    expect(track()).toHaveAttribute("data-state", "done");
    expect(bar()).toHaveAttribute("data-bursts", "0");
    expect(level()).toHaveAttribute("data-level", "4");
    expect(rewards()).toEqual([]);
    expect(engine.playSfx).not.toHaveBeenCalled();
  });

  it("two levels chain in order, a bar for each, each reward flying out after its level's burst", () => {
    show([twoLevels()]);
    expect(level()).toHaveAttribute("data-level", "1");

    // The first bar to full: level 2, and its Jlore.
    advance(segmentMs({ from: 60, to: 100, burst: true }, 100));
    expect(level()).toHaveAttribute("data-level", "2");
    advance(REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["25 Jlore"]);

    // Then the level in between, whole, then the last level's bar to where the answer says.
    advance(BURST_MS);
    expect(bar()).toHaveAttribute("data-state", "filling");
    expect(level()).toHaveAttribute("data-level", "2");
    advance(segmentMs({ from: 0, to: 300, burst: true }, 300));
    expect(level()).toHaveAttribute("data-level", "3");
    expect(level()).toHaveTextContent("Level 3 of 100");
    advance(REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["25 Jlore", "1 × Core pack"]);
  });

  it("the last of them rests at the answer's points, and every reward has been shown", () => {
    show([twoLevels()]);
    advance(2_000 + 3 * BURST_MS + 2 * REVEAL_STAGGER_MS);

    expect(track()).toHaveAttribute("data-state", "done");
    expect(progress().max).toBe(300);
    expect(progress().value).toBe(20);
    expect(bar()).toHaveTextContent("20 / 300");
    expect(level()).toHaveAttribute("data-level", "3");
    expect(rewards()).toEqual(["25 Jlore", "1 × Core pack"]);
  });

  it("a level the track started on shows what it paid at once, a level it reached as its bar bursts", () => {
    // A track the game unlocked: it stood on level 1 with nothing and pays that level too.
    show([
      {
        setId: SET,
        before: at(1, 0, 100, 0),
        after: at(2, 100, 200, 0),
        levels: [
          { level: 1, rewards: { ...NOTHING, jlore: 40 } },
          { level: 2, rewards: { ...NOTHING, cardTokens: 1 } },
        ],
      },
    ]);

    advance(REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["40 Jlore"]);
    advance(segmentMs({ from: 0, to: 100, burst: true }, 100) + REVEAL_STAGGER_MS);
    expect(rewards()).toEqual(["40 Jlore", "1 × card token"]);
  });

  it("a track that reaches its last level ends full, with no size of its own to name", () => {
    show([
      {
        setId: SET,
        before: at(99, 4_000, 400, 395),
        after: at(100, 4_500, null),
        levels: [{ level: 100, rewards: { ...NOTHING, setPacks: 3 } }],
      },
    ]);
    advance(2_000 + 2 * BURST_MS + REVEAL_STAGGER_MS);

    expect(track()).toHaveAttribute("data-state", "done");
    expect(progress().value).toBe(progress().max);
    expect(bar()).toHaveTextContent("Last level reached.");
    expect(level()).toHaveAttribute("data-level", "100");
    expect(rewards()).toEqual(["3 × Core pack"]);
  });

  it("a track already on its last level shows full and never moves", () => {
    show([{ setId: SET, before: at(100, 4_500, null), after: at(100, 4_700, null), levels: [] }]);

    expect(track()).toHaveAttribute("data-state", "done");
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(progress().value).toBe(progress().max);
    advance(60_000);
    expect(level()).toHaveAttribute("data-level", "100");
    expect(rewards()).toEqual([]);
  });

  it("every track the game moved has its own bar", () => {
    show([oneLevel(), { setId: "Meditative", before: at(2, 10, 200, 10), after: at(2, 50, 200, 50), levels: [] }]);

    expect(screen.getByTestId(`${ID}-bar-Core`)).toHaveTextContent("Core track");
    expect(screen.getByTestId(`${ID}-bar-Meditative`)).toHaveTextContent("Meditative track");
  });

  it("the same answer sent again plays on; a different one plays from its start", () => {
    const { rerender } = show([oneLevel()]);
    advance(segmentMs({ from: 60, to: 100, burst: true }, 100));
    expect(level()).toHaveAttribute("data-level", "2");

    rerender(<XpReveal economy={{ tracks: [oneLevel()], quests: [] }} />);
    expect(level()).toHaveAttribute("data-level", "2");

    rerender(<XpReveal economy={{ tracks: [twoLevels()], quests: [] }} />);
    expect(level()).toHaveAttribute("data-level", "1");
    expect(rewards()).toEqual([]);
  });
});

describe("#1664 the quests a game completed", () => {
  it("are listed with their Jlore and XP, beside the tracks or alone", () => {
    const { rerender } = show([oneLevel()], [QUEST, { questId: "play-5", title: "Play 5 games", jlore: 10, xp: 500 }]);

    expect(screen.getByTestId(`${ID}-quest-win-3`)).toHaveTextContent("Win 3 games");
    expect(screen.getByTestId(`${ID}-quest-win-3`)).toHaveTextContent("80 Jlore · 4,000 XP");
    expect(screen.getByTestId(`${ID}-quest-play-5`)).toHaveTextContent("Play 5 games10 Jlore · 500 XP");

    rerender(<XpReveal economy={{ tracks: [], quests: [QUEST] }} />);
    expect(screen.queryByTestId(`${ID}-track-${SET}`)).toBeNull();
    expect(screen.getByTestId(`${ID}-quests`).querySelectorAll("li")).toHaveLength(1);
  });

  it("show at once, whatever the bars are playing", () => {
    show([oneLevel()], [QUEST]);
    expect(track()).toHaveAttribute("data-state", "filling");
    expect(screen.getByTestId(`${ID}-quest-win-3`)).toBeInTheDocument();
  });

  it("a game that moved no track and completed no quest draws nothing", () => {
    const { container } = show([]);
    expect(container).toBeEmptyDOMElement();
  });
});

describe("#1664 reduced motion", () => {
  it("shows the end state and every reward at once, with the strongest sting once", () => {
    writeSettings({ reduceMotion: true });
    show([twoLevels()], [QUEST]);

    expect(track()).toHaveAttribute("data-state", "done");
    expect(bar()).toHaveAttribute("data-state", "resting");
    expect(progress().max).toBe(300);
    expect(progress().value).toBe(20);
    expect(level()).toHaveAttribute("data-level", "3");
    expect(rewards()).toEqual(["25 Jlore", "1 × Core pack"]);
    expect(screen.getByTestId(`${ID}-quest-win-3`)).toBeInTheDocument();
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
    expect(engine.playSfx).toHaveBeenCalledWith("sting", { tier: "rare" });

    advance(60_000);
    expect(rewards()).toEqual(["25 Jlore", "1 × Core pack"]);
    expect(engine.playSfx).toHaveBeenCalledTimes(1);
  });

  it("a track that only gained XP rests at the answer's points", () => {
    writeSettings({ reduceMotion: true });
    show([{ setId: SET, before: at(4, 60, 100, 60), after: at(4, 90, 100, 90), levels: [] }]);
    expect(bar()).toHaveTextContent("90 / 100");
    expect(bar()).toHaveAttribute("data-state", "resting");
  });
});

describe("#1664 the stages a gain plays", () => {
  it("lays one bar out for each level, the size the answer gives each end and the first's and last's points", () => {
    expect(stagesOf(twoLevels())).toEqual([
      { level: 1, size: 100, from: 60, value: 0, fills: 1 },
      { level: 2, size: 300, from: 0, value: 0, fills: 1 },
      { level: 3, size: 300, from: 0, value: 20, fills: 0 },
    ]);
    expect(stagesOf({ setId: SET, before: at(4, 60, 100, 60), after: at(4, 90, 100, 90), levels: [] })).toEqual([
      { level: 4, size: 100, from: 60, value: 90, fills: 0 },
    ]);
  });
});

describe("#1664 reading an economy off a frame", () => {
  const WIRE = {
    jlore: 30,
    xp: 540,
    jloreCapped: false,
    tracks: [
      {
        setId: SET,
        before: { level: 1, xp: 60, levelXp: 100, xpToNext: 40 },
        after: { level: 2, xp: 130, levelXp: 200, xpToNext: 170 },
        levels: [{ level: 2, rewards: { jlore: 25, setPacks: 2, cardTokens: 3 } }],
      },
    ],
    quests: [{ questId: "win-3", title: "Win 3 games", jlore: 80, xp: 4000 }],
  };

  it("reads a whole answer as it is", () => {
    const economy: GameEconomy | null = parseGameEconomy(WIRE);
    expect(economy).toEqual(WIRE);
  });

  it("is none for no answer or one that is not whole, and leaves out a part that is not", () => {
    expect(parseGameEconomy(undefined)).toBeNull();
    expect(parseGameEconomy(null)).toBeNull();
    expect(parseGameEconomy({ ...WIRE, tracks: "no" })).toBeNull();
    expect(parseGameEconomy({ ...WIRE, jlore: "30" })).toBeNull();
    expect(parseGameEconomy({ ...WIRE, jloreCapped: undefined })).toBeNull();

    const odd = parseGameEconomy({
      ...WIRE,
      tracks: [...WIRE.tracks, { ...WIRE.tracks[0], setId: "NotASet" }, { ...WIRE.tracks[0], after: { level: "2" } }],
      quests: [...WIRE.quests, { questId: 5 }],
    });
    expect(odd?.tracks).toEqual(WIRE.tracks);
    expect(odd?.quests).toEqual(WIRE.quests);
  });
});
