// R631 (SPEC §10.11 "Music" and "Settings"): the music's controls in the audio panel, and the music
// a mounted board takes over from the main menu theme and hands back. R1350, R1351: a mounted board
// plays a card's intro through the page's player, under Reduce motion too, and cuts it as it leaves.

import type { GameEvent } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import AudioControls from "./AudioControls.tsx";
import { setAudioEngineForTests } from "./engine.ts";
import { setMusicPlayerForTests, type MusicPlayer, type MusicRequest } from "./music.ts";
import { gameMusicClaims, holdMenuMusic } from "./musicScene.ts";
import { readAudioSettings, resetAudioSettingsForTests } from "./settings.ts";
import Game from "../game/Game.tsx";
import { baseView, withEvents } from "../test/fixtures.ts";
import { setReducedMotion } from "../test/setup.ts";

/** A player that records what it is asked for and plays nothing. */
function recordingPlayer(): MusicPlayer & { requests: MusicRequest[]; intros: string[]; stops: number } {
  const requests: MusicRequest[] = [];
  const intros: string[] = [];
  const rec = {
    requests,
    intros,
    stops: 0,
    request: (r: MusicRequest) => {
      requests.push(r);
    },
    playIntro: (id: string) => {
      intros.push(id);
    },
    stopIntro: () => {
      rec.stops += 1;
    },
    preloadIntros: () => undefined,
    intro: () => intros.at(-1) ?? null,
    resetResume: () => undefined,
    preload: () => undefined,
    setBusy: () => undefined,
    current: () => null,
    wanted: () => requests.at(-1)?.track ?? null,
    opponentTurn: () => requests.at(-1)?.opponentTurn ?? false,
    log: () => [],
    dispose: () => undefined,
  };
  return rec;
}

function input(testid: string): HTMLInputElement {
  return screen.getByTestId(testid);
}

let player: ReturnType<typeof recordingPlayer>;
const releases: (() => void)[] = [];

beforeEach(() => {
  localStorage.clear();
  resetAudioSettingsForTests();
  setAudioEngineForTests(null);
  player = recordingPlayer();
  setMusicPlayerForTests(player);
});

afterEach(() => {
  cleanup();
  for (const release of releases.splice(0)) release();
  setMusicPlayerForTests(null);
  setAudioEngineForTests(null);
  setReducedMotion(false);
  vi.useRealTimers();
  resetAudioSettingsForTests();
  localStorage.clear();
});

describe("R631 the audio panel's music controls", () => {
  it("R631 shows the music volume, the station picker and the three music switches at their defaults", () => {
    render(<AudioControls />);
    expect(input("audio-music").value).toBe("50");
    const station = screen.getByTestId<HTMLSelectElement>("audio-station");
    expect([...station.options].map((o) => [o.value, o.text])).toEqual([
      ["tavern", "Tavern"],
      ["edm", "EDM"],
      ["lofi", "Lo-fi"],
      ["epic", "Epic Orchestral"],
    ]);
    expect(station.value).toBe("tavern");
    for (const id of ["audio-dynamic-music", "audio-duck-music", "audio-music-background"]) expect(input(id).checked, id).toBe(id !== "audio-music-background");
    const labels: Record<string, string> = {
      "audio-music": "Music volume",
      "audio-station": "Music station",
      "audio-dynamic-music": "Dynamic music",
      "audio-duck-music": "Lower music under voices and big moments",
      "audio-music-background": "Keep playing music when this tab is in the background",
    };
    for (const [id, label] of Object.entries(labels)) expect(screen.getByLabelText(label), id).toBe(screen.getByTestId(id));
  });

  it("R631 each control writes its setting, which persists", () => {
    render(<AudioControls />);
    fireEvent.change(input("audio-music"), { target: { value: "25" } });
    fireEvent.change(screen.getByTestId("audio-station"), { target: { value: "epic" } });
    fireEvent.click(input("audio-dynamic-music"));
    fireEvent.click(input("audio-duck-music"));
    fireEvent.click(input("audio-music-background"));
    expect(readAudioSettings()).toMatchObject({ music: 0.25, station: "epic", dynamicMusic: false, duckMusic: false, playMusicInBackground: true });
    resetAudioSettingsForTests();
    expect(readAudioSettings()).toMatchObject({ music: 0.25, station: "epic", dynamicMusic: false, duckMusic: false, playMusicInBackground: true });
  });
});

describe("R631 the menu theme and the board's music", () => {
  it("R631 a screen without a board plays the menu theme; a board takes over and hands it back", () => {
    releases.push(holdMenuMusic());
    expect(player.requests.at(-1)).toEqual({ track: "menu", opponentTurn: false });

    const { unmount } = render(<Game view={baseView({ turn: 1, phase: "mulligan" })} legal={[]} onAction={() => undefined} />);
    expect(gameMusicClaims()).toBe(1);
    // The board's first request opens on the sting; later ones (the same track) carry none, and the
    // player ignores a request for the track it already wants.
    expect(player.requests.find((r) => r.track !== "menu")).toEqual({ track: "tavern-1", intro: "tavern-start", opponentTurn: false });
    expect(player.requests.at(-1)?.track).toBe("tavern-1");

    unmount();
    expect(gameMusicClaims()).toBe(0);
    expect(player.requests.at(-1)).toEqual({ track: "menu", opponentTurn: false });
  });

  it("R631 the next match plays the station's other in-game track", () => {
    const first = render(<Game view={baseView()} legal={[]} onAction={() => undefined} />);
    expect(player.requests.at(-1)?.track).toBe("tavern-1");
    first.unmount();
    render(<Game view={baseView()} legal={[]} onAction={() => undefined} />);
    expect(player.requests.at(-1)?.track).toBe("tavern-2");
  });

  it("R631 a Mythic's theme starts as the sound director voices its cast", () => {
    setReducedMotion(true);
    vi.useFakeTimers();
    const first = baseView();
    const { rerender } = render(<Game view={first} legal={[]} onAction={() => undefined} />);
    const zephyrs: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "c9", defId: "core-097", costPaid: 7 };
    rerender(<Game view={withEvents(first, [zephyrs])} legal={[]} onAction={() => undefined} />);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(player.requests.map((r) => r.track)).toContain("mythic-zephyrs");
  });
});

describe("R1350 a mounted board's intros", () => {
  it("R1350 a played Mythic's intro plays under Reduce motion too, before its theme: it is sound, not motion", () => {
    setReducedMotion(true);
    vi.useFakeTimers();
    const first = baseView();
    const { rerender } = render(<Game view={first} legal={[]} onAction={() => undefined} />);
    const zephyrs: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "c9", defId: "core-097", costPaid: 7 };
    rerender(<Game view={withEvents(first, [zephyrs])} legal={[]} onAction={() => undefined} />);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(player.intros).toEqual(["intro-core-097"]);
    expect(player.requests.map((r) => r.track)).toContain("mythic-zephyrs");
  });

  it("R1351 a board that leaves cuts the intro it started", () => {
    setReducedMotion(true);
    vi.useFakeTimers();
    const first = baseView();
    const { rerender, unmount } = render(<Game view={first} legal={[]} onAction={() => undefined} />);
    const silas: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "c3", defId: "core-052", costPaid: 4 };
    rerender(<Game view={withEvents(first, [silas])} legal={[]} onAction={() => undefined} />);
    act(() => {
      vi.advanceTimersByTime(5_000);
    });
    expect(player.intros).toEqual(["intro-core-052"]);
    const before = player.stops;
    unmount();
    expect(player.stops).toBeGreaterThan(before);
  });
});
