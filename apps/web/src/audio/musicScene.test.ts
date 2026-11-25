// R4606000 (SPEC §10.11 "Music"): a board leaving hands the music back on a microtask. The board
// tests prove it through a mounted `Game`, which holds the menu theme itself (`useGameAudio` →
// `retainAppAudio`); this file proves it on `musicScene.ts` alone, with no board and no incidental hold.

import { afterEach, describe, expect, it } from "vitest";

import { setMusicPlayerForTests, type MusicPlayer, type MusicRequest } from "./music.ts";
import { MENU_TRACK } from "./musicData.ts";
import { enterGameMusic, gameMusicClaims, holdMenuMusic } from "./musicScene.ts";

/** A player that records what it is asked for and plays nothing. */
function recordingPlayer(): MusicPlayer & { requests: MusicRequest[] } {
  const requests: MusicRequest[] = [];
  return {
    requests,
    request: (r: MusicRequest) => {
      requests.push(r);
    },
    playIntro: () => undefined,
    stopIntro: () => undefined,
    playSting: () => undefined,
    preloadIntros: () => undefined,
    intro: () => null,
    resetResume: () => undefined,
    preload: () => undefined,
    setBusy: () => undefined,
    current: () => null,
    wanted: () => requests.at(-1)?.track ?? null,
    opponentTurn: () => requests.at(-1)?.opponentTurn ?? false,
    log: () => [],
    dispose: () => undefined,
  };
}

const GAME: MusicRequest = { track: "tavern-1", opponentTurn: false };
const MENU: MusicRequest = { track: MENU_TRACK, opponentTurn: false };
const SILENCE: MusicRequest = { track: null };

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
}

let player: MusicPlayer & { requests: MusicRequest[] };

afterEach(() => {
  setMusicPlayerForTests(null);
});

describe("R4606000 a board leaving hands the music back on a microtask", () => {
  it("R4606000 a board leaving asks for nothing in the leave itself and hands the menu theme back on a microtask", async () => {
    player = recordingPlayer();
    setMusicPlayerForTests(player);
    const release = holdMenuMusic();
    const board = enterGameMusic();
    board.request(GAME);
    const before = player.requests.length;
    board.leave();
    expect(player.requests.length, "the leave itself asks for nothing").toBe(before);
    await flushMicrotasks();
    expect(player.requests.slice(before)).toEqual([MENU]);
    expect(gameMusicClaims()).toBe(0);
    release();
  });

  it("R4606000 a board entering before the microtask claims the music with no menu theme or silence between", async () => {
    player = recordingPlayer();
    setMusicPlayerForTests(player);
    const release = holdMenuMusic();
    const first = enterGameMusic();
    first.request(GAME);
    const before = player.requests.length;
    first.leave();
    const second = enterGameMusic();
    second.request(GAME);
    await flushMicrotasks();
    const asked = player.requests.slice(before);
    expect(asked.some((r) => r.track === MENU_TRACK)).toBe(false);
    expect(asked.some((r) => r.track === null)).toBe(false);
    expect(asked.at(-1)).toEqual(GAME);
    second.leave();
    await flushMicrotasks();
    release();
  });

  it("R4606000 a board leaving with nothing holding the menu theme asks for silence on the microtask", async () => {
    player = recordingPlayer();
    setMusicPlayerForTests(player);
    const board = enterGameMusic();
    board.request(GAME);
    const before = player.requests.length;
    board.leave();
    expect(player.requests.length).toBe(before);
    await flushMicrotasks();
    expect(player.requests.slice(before)).toEqual([SILENCE]);
  });
});
