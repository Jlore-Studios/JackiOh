// The audio panel (SPEC §10.11 "Settings"; R631, its music controls; docs/polish/2-sound.md, B30 and B54) at the
// edges ui.test.tsx and music-ui.test.tsx leave open: its class, its three titled groups, the labels and the
// background switch's hint, which sliders answer with a sound, a station the picker does not offer, and its
// tables as a page load builds them.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import AudioControls from "./AudioControls.tsx";
import { AUDIO_SETTINGS_KEY } from "./constants.ts";
import { setAudioEngineForTests } from "./engine.ts";
import { resetAudioSettingsForTests, subscribeAudioSettings } from "./settings.ts";
import { fakeAudioEngine } from "./test/fakeAudio.ts";
import type { AudioSettings } from "./types.ts";

type ControlsModule = typeof import("./AudioControls.tsx");

/** R631: what the background switch says it does. */
const BACKGROUND_HINT =
  "Off: the music fades out when you switch to another tab or window, and back in when you return. Sound effects and voices stay quiet either way.";

/** The text of the element `el` names as its description, or null with none. */
function description(el: HTMLElement): string | null {
  const id = el.getAttribute("aria-describedby");
  return id === null ? null : (document.getElementById(id)?.textContent ?? null);
}

beforeEach(() => {
  localStorage.clear();
  resetAudioSettingsForTests();
  setAudioEngineForTests(null);
});

afterEach(() => {
  cleanup();
  setAudioEngineForTests(null);
  resetAudioSettingsForTests();
  localStorage.clear();
});

describe("B30 the audio panel's frame", () => {
  it("B30 is audio-controls alone with no className or an empty one, and audio-controls and the className with one", () => {
    const panel = (): HTMLElement => screen.getByTestId("audio-controls");
    const { rerender } = render(<AudioControls />);
    expect(panel().getAttribute("class")).toBe("audio-controls");
    rerender(<AudioControls className="" />);
    expect(panel().getAttribute("class")).toBe("audio-controls");
    rerender(<AudioControls className="settings-audio" />);
    expect(panel().getAttribute("class")).toBe("audio-controls settings-audio");
  });

  it("B30 R631 titles its groups Volume, Music and Voice, each holding its own controls", () => {
    render(<AudioControls />);
    const groups: Record<string, readonly string[]> = {
      Volume: ["audio-master", "audio-sfx", "audio-crowd", "audio-voice", "audio-music", "audio-mute"],
      Music: ["audio-station", "audio-dynamic-music", "audio-duck-music", "audio-music-background"],
      Voice: ["audio-voice-on"],
    };
    for (const [title, ids] of Object.entries(groups)) {
      const group = screen.getByRole("group", { name: title });
      for (const id of ids) expect(group, `${title} holds ${id}`).toContainElement(screen.getByTestId(id));
    }
  });

  it("B30 names the mute and voice-line switches by their labels", () => {
    render(<AudioControls />);
    expect(screen.getByLabelText("Mute all sound")).toBe(screen.getByTestId("audio-mute"));
    expect(screen.getByLabelText("Voice lines")).toBe(screen.getByTestId("audio-voice-on"));
  });

  it("R631 the background switch, and no other, is described by its hint", () => {
    const { container } = render(<AudioControls />);
    expect(description(screen.getByTestId("audio-music-background"))).toBe(BACKGROUND_HINT);
    expect(container.querySelectorAll(".audio-controls__hint")).toHaveLength(1);
    for (const id of ["audio-dynamic-music", "audio-duck-music"]) expect(screen.getByTestId(id).hasAttribute("aria-describedby"), id).toBe(false);
  });
});

describe("B54 which controls answer with a sound", () => {
  it("B54 the music slider plays no tick as it moves (the music is its own preview), and only the voice slider answers a release", () => {
    const engine = fakeAudioEngine();
    setAudioEngineForTests(engine);
    render(<AudioControls />);

    fireEvent.change(screen.getByTestId("audio-music"), { target: { value: "25" } });
    expect(engine.playSfx).not.toHaveBeenCalled();

    for (const id of ["audio-master", "audio-sfx", "audio-crowd", "audio-music"]) {
      fireEvent.pointerUp(screen.getByTestId(id));
      fireEvent.keyUp(screen.getByTestId(id), { key: "ArrowRight" });
    }
    expect(engine.playSfx).not.toHaveBeenCalled();
    expect(engine.playVoice).not.toHaveBeenCalled();
  });
});

describe("R631 the station picker", () => {
  it("R631 a value the picker does not offer writes nothing and tells no one", () => {
    render(<AudioControls />);
    const picker = screen.getByTestId<HTMLSelectElement>("audio-station");
    // An option the panel never wrote (stale markup, an extension's), picked.
    picker.append(new Option("Polka", "polka"));
    const heard: AudioSettings[] = [];
    const unsubscribe = subscribeAudioSettings((settings) => heard.push(settings));

    fireEvent.change(picker, { target: { value: "polka" } });
    unsubscribe();

    expect(heard).toEqual([]);
    expect(localStorage.getItem(AUDIO_SETTINGS_KEY)).toBeNull();
  });
});

describe("B30 R631 the panel's controls as a page load builds them", () => {
  /**
   * The panel's sliders, stations and switches are tables built as the module loads; loading it inside the test
   * (`vi.resetModules()`) builds them there, so one built wrong fails here by name. Stryker runs a mutant of a
   * value built at load only against the tests that load its module (#3518), which this file's import is not.
   */
  async function freshControls(): Promise<ControlsModule> {
    vi.resetModules();
    return import("./AudioControls.tsx");
  }

  it("B30 R631 each slider, station and switch shows its own label, test id and stored setting", async () => {
    // A level of its own for each slider and every switch on, so a control read from the wrong setting shows.
    const stored: Partial<AudioSettings> = { master: 0.1, sfx: 0.2, crowd: 0.3, voice: 0.4, music: 0.6, dynamicMusic: true, duckMusic: true, playMusicInBackground: true };
    localStorage.setItem(AUDIO_SETTINGS_KEY, JSON.stringify(stored));
    const { default: FreshControls } = await freshControls();
    render(<FreshControls />);

    for (const [id, label, value] of [
      ["audio-master", "Master volume", "10"],
      ["audio-sfx", "Sound effects volume", "20"],
      ["audio-crowd", "Crowd volume", "30"],
      ["audio-voice", "Voice volume", "40"],
      ["audio-music", "Music volume", "60"],
    ] as const) {
      const slider = screen.getByTestId<HTMLInputElement>(id);
      expect(screen.getByLabelText(label), id).toBe(slider);
      expect(slider.value, id).toBe(value);
    }

    const station = screen.getByTestId<HTMLSelectElement>("audio-station");
    expect([...station.options].map((option) => [option.value, option.text])).toEqual([
      ["tavern", "Tavern"],
      ["edm", "EDM"],
      ["lofi", "Lo-fi"],
      ["epic", "Epic orchestral"],
    ]);

    for (const [id, label] of [
      ["audio-dynamic-music", "Dynamic music"],
      ["audio-duck-music", "Lower music under voices and big moments"],
      ["audio-music-background", "Keep playing music when this tab is in the background"],
    ] as const) {
      const box = screen.getByTestId<HTMLInputElement>(id);
      expect(screen.getByLabelText(label), id).toBe(box);
      expect(box.checked, id).toBe(true);
    }
    expect(description(screen.getByTestId("audio-music-background"))).toBe(BACKGROUND_HINT);
  });
});
