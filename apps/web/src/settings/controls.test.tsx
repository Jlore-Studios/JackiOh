// The panel's controls (`./controls.tsx`), each rendered alone: the speed slider's readout, track,
// hint and ticks (R435), the selects' options, hints and writes, the tilt switch's ask (R3344001),
// and the card controls' words, read from cards/settings.ts so the panel and the module agree. The
// panel that mounts them on a board is proved by `wiring.test.tsx`.

import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CARD_SETTINGS_DEFAULTS, CARD_SETTINGS_KEY, readCardSettings } from "../cards/settings.ts";
import { FX_INTENSITY_SCALE, FX_SETTINGS_KEY, FX_SPEED_DEFAULT, FX_SPEED_MAX, FX_SPEED_MIN } from "../fx/constants.ts";
import { getFxSettings, resetFxSettingsForTests } from "../fx/settings.ts";
import { readHapticsSettings, resetHapticsSettingsForTests } from "../haptics/settings.ts";
import { LOCALES, getLocale, resetLocaleForTests } from "../i18n/i18n.ts";
import {
  BoardNamesSelect,
  DeviceTiltSwitch,
  FxControls,
  LanguageSelect,
  RarityFoilSwitch,
  VibrationSwitch,
  speedLabel,
} from "./controls.tsx";

type Field = (typeof import("../cards/settings.ts"))["CARD_SETTINGS_FIELDS"][number];

/** The list controls.tsx reads as CARD_SETTINGS_FIELDS: the module's own, unless a test rewrites it. */
const fields = vi.hoisted(() => ({ list: [] as Field[], original: [] as readonly Field[] }));

// Runs again for each fresh module graph (`freshControls`), so it refills the one list rather than adding to it.
vi.mock("../cards/settings.ts", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../cards/settings.ts")>();
  fields.original = actual.CARD_SETTINGS_FIELDS;
  fields.list.splice(0, fields.list.length, ...actual.CARD_SETTINGS_FIELDS);
  return { ...actual, CARD_SETTINGS_FIELDS: fields.list };
});

type ControlsModule = typeof import("./controls.tsx");
type FxSettingsModule = typeof import("../fx/settings.ts");

/**
 * controls.tsx as a page load builds it, its option lists included, with the effects store it writes:
 * fresh instances imported inside the test, so a value made at load is made for this test.
 */
async function freshControls(): Promise<{ controls: ControlsModule; fx: FxSettingsModule }> {
  vi.resetModules();
  const controls = await import("./controls.tsx");
  const fx = await import("../fx/settings.ts");
  return { controls, fx };
}

/** Rewrites, for one test, what controls.tsx finds in CARD_SETTINGS_FIELDS: `null` drops a field. */
function editFields(edit: (field: Field) => Field | null): void {
  const next = fields.original.map(edit).filter((field): field is Field => field !== null);
  fields.list.splice(0, fields.list.length, ...next);
}

function fieldOf(key: Field["key"]): Field {
  const field = fields.original.find((entry) => entry.key === key);
  if (field === undefined) throw new Error(`cards/settings.ts has no field for ${key}`);
  return field;
}

/** What a run throws, or reports to the window as React does for a handler that throws. */
function errorsDuring(run: () => void): unknown[] {
  const errors: unknown[] = [];
  const onError = (event: ErrorEvent): void => {
    errors.push(event.error);
    event.preventDefault();
  };
  window.addEventListener("error", onError);
  try {
    run();
  } catch (cause) {
    errors.push(cause);
  } finally {
    window.removeEventListener("error", onError);
  }
  return errors;
}

beforeEach(() => {
  localStorage.clear();
  resetFxSettingsForTests();
  resetHapticsSettingsForTests();
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  fields.list.splice(0, fields.list.length, ...fields.original);
  resetLocaleForTests();
  resetFxSettingsForTests();
  resetHapticsSettingsForTests();
  localStorage.clear();
});

describe("speedLabel", () => {
  it("reads the speed to hundredths with a times sign", () => {
    expect(speedLabel(1)).toBe("1×");
    expect(speedLabel(0.25)).toBe("0.25×");
    expect(speedLabel(3)).toBe("3×");
  });

  it("rounds the readout, never the slider's value", () => {
    expect(speedLabel(1.005)).toBe("1×");
    expect(speedLabel(2.5)).toBe("2.5×");
  });
});

describe("R435 the effects speed slider", () => {
  it("R435 fills its track from empty at the slowest speed to full at the fastest", () => {
    render(<FxControls />);
    const range = screen.getByTestId<HTMLInputElement>("setting-fxSpeed");
    fireEvent.change(range, { target: { value: String(FX_SPEED_MIN) } });
    expect(range.style.getPropertyValue("--fx-speed-fill")).toBe("0");
    fireEvent.change(range, { target: { value: String(FX_SPEED_MAX) } });
    expect(range.style.getPropertyValue("--fx-speed-fill")).toBe("1");
  });

  it("R435 ties its readout to the slider, describes the slider by its hint, and lists its ticks", () => {
    render(<FxControls />);
    const range = screen.getByRole("slider", { name: "Effects speed" });
    expect(screen.getByTestId("setting-fxSpeed-value").getAttribute("for")).toBe(range.id);
    expect(range).toHaveAccessibleDescription(
      "How fast cards move, hit and die. Faster also shortens the pauses between them.",
    );
    const ticks = document.getElementById(range.getAttribute("list") ?? "");
    expect(ticks?.tagName).toBe("DATALIST");
    expect([...(ticks?.querySelectorAll("option") ?? [])].map((option) => [option.value, option.label])).toEqual([
      [String(FX_SPEED_MIN), speedLabel(FX_SPEED_MIN)],
      [String(FX_SPEED_DEFAULT), speedLabel(FX_SPEED_DEFAULT)],
      [String(FX_SPEED_MAX), speedLabel(FX_SPEED_MAX)],
    ]);
  });
});

describe("the effects selects", () => {
  it("as a page load builds them, offer every intensity the effects layer scales and every Big moments choice the store keeps, each labelled", async () => {
    const { controls, fx } = await freshControls();
    render(<controls.FxControls />);
    const intensity = screen.getByTestId<HTMLSelectElement>("setting-fxIntensity");
    const bigMoments = screen.getByTestId<HTMLSelectElement>("setting-fxBigMoments");
    expect([...intensity.options].map((option) => option.value).sort()).toEqual(Object.keys(FX_INTENSITY_SCALE).sort());
    // R2785000: Big moments is full, short or off.
    expect(bigMoments.options).toHaveLength(3);
    for (const select of [intensity, bigMoments]) {
      const labels = [...select.options].map((option) => option.text);
      expect(labels.every((label) => label.length > 0), select.dataset.testid).toBe(true);
      expect(new Set(labels).size, select.dataset.testid).toBe(labels.length);
    }
    // Each choice offered is one the store keeps, not one it reads back as its default.
    for (const option of [...intensity.options]) {
      fireEvent.change(intensity, { target: { value: option.value } });
      expect(fx.getFxSettings().intensity).toBe(option.value);
    }
    for (const option of [...bigMoments.options]) {
      fireEvent.change(bigMoments, { target: { value: option.value } });
      expect(fx.getFxSettings().bigMoments).toBe(option.value);
    }
  });

  it("describe each select by its hint", () => {
    render(<FxControls />);
    const intensity = screen.getByTestId<HTMLSelectElement>("setting-fxIntensity");
    expect(intensity).toHaveAccessibleDescription(
      "Fire, sparks, light rays and the board shake. Off keeps the card motion only.",
    );
    expect(screen.getByTestId("setting-fxBigMoments")).toHaveAccessibleDescription(
      "The biggest moments hold the board for a beat; a tap, a click or Space skips one. Short holds them briefly, Off not at all.",
    );
  });

  it("ignore a value that is none of their options: nothing is stored and nothing throws", () => {
    render(<FxControls />);
    const errors = errorsDuring(() => {
      fireEvent.change(screen.getByTestId("setting-fxIntensity"), { target: { value: "loud" } });
      fireEvent.change(screen.getByTestId("setting-fxBigMoments"), { target: { value: "endless" } });
    });
    expect(errors).toEqual([]);
    expect(getFxSettings()).toMatchObject({ intensity: "normal", bigMoments: "full" });
    expect(localStorage.getItem(FX_SETTINGS_KEY)).toBeNull();
  });
});

describe("the card switches read their words from cards/settings.ts", () => {
  it("R2721000 a switch is named by its field's label and described by its field's description", () => {
    render(<RarityFoilSwitch />);
    const field = fieldOf("rarityFoil");
    expect(screen.getByRole("switch", { name: field.label })).toHaveAccessibleDescription(field.description);
  });

  it("R2721000 a switch whose field is missing is named by its key and shows no hint", () => {
    editFields((field) => (field.key === "rarityFoil" ? null : field));
    render(<RarityFoilSwitch />);
    expect(screen.getByRole("switch", { name: "rarityFoil" })).not.toHaveAccessibleDescription();
  });
});

describe("R3344001 the Tilt with my phone switch", () => {
  it("R3344001 is named and described by its field in cards/settings.ts", () => {
    render(<DeviceTiltSwitch />);
    const field = fieldOf("deviceTilt");
    expect(screen.getByRole("switch", { name: field.label })).toHaveAccessibleDescription(field.description);
  });

  it("R3344001 with its field missing it is named by its key and shows no hint", () => {
    editFields((field) => (field.key === "deviceTilt" ? null : field));
    render(<DeviceTiltSwitch />);
    expect(screen.getByRole("switch", { name: "deviceTilt" })).not.toHaveAccessibleDescription();
  });

  it("R3344001 turns on at once on a platform with no DeviceOrientationEvent at all", () => {
    vi.stubGlobal("DeviceOrientationEvent", undefined);
    render(<DeviceTiltSwitch />);
    fireEvent.click(screen.getByTestId("setting-deviceTilt"));
    expect(readCardSettings().deviceTilt).toBe(true);
    expect(screen.getByTestId<HTMLInputElement>("setting-deviceTilt").checked).toBe(true);
  });

  it("R3344001 the last answer holds: an ask that fails after an earlier grant turns the switch back off", async () => {
    const asks: { grant: () => void; fail: () => void }[] = [];
    vi.stubGlobal(
      "DeviceOrientationEvent",
      class {
        static requestPermission(): Promise<string> {
          return new Promise<string>((resolve, reject) => {
            asks.push({ grant: () => resolve("granted"), fail: () => reject(new Error("blocked")) });
          });
        }
      },
    );
    render(<DeviceTiltSwitch />);
    const switchEl = screen.getByTestId<HTMLInputElement>("setting-deviceTilt");
    // Two taps before the phone answers the first: each one asks.
    fireEvent.click(switchEl);
    fireEvent.click(switchEl);
    expect(asks).toHaveLength(2);
    await act(async () => {
      asks[0]?.grant();
      await Promise.resolve();
    });
    expect(readCardSettings().deviceTilt).toBe(true);
    await act(async () => {
      asks[1]?.fail();
      await Promise.resolve();
    });
    expect(readCardSettings().deviceTilt).toBe(false);
    expect(switchEl.checked).toBe(false);
  });
});

describe("R3339000 the Names on the board select", () => {
  it("R3339000 takes its label and hint from its field in cards/settings.ts", () => {
    editFields((field) =>
      field.key === "boardNames" ? { ...field, label: "Board names, reworded", description: "A reworded hint." } : field,
    );
    render(<BoardNamesSelect />);
    expect(screen.getByRole("combobox", { name: "Board names, reworded" })).toHaveAccessibleDescription(
      "A reworded hint.",
    );
  });

  it("R3339000 with its field missing it keeps its own words", () => {
    editFields((field) => (field.key === "boardNames" ? null : field));
    render(<BoardNamesSelect />);
    expect(screen.getByRole("combobox", { name: "Names on the board" })).toHaveAccessibleDescription(
      "Full-art and Legendary/Mythic cards hide their name and words in play; hover or press to read. Always shows every name; Never hides all names.",
    );
  });

  it("R3339000 stores nothing for a value that is none of its options", () => {
    render(<BoardNamesSelect />);
    fireEvent.change(screen.getByTestId("setting-boardNames"), { target: { value: "sometimes" } });
    expect(readCardSettings().boardNames).toBe(CARD_SETTINGS_DEFAULTS.boardNames);
    expect(localStorage.getItem(CARD_SETTINGS_KEY)).toBeNull();
  });
});

describe("R669 the Vibration switch", () => {
  it("R669 is named, described and on by default, and pressing it writes the haptics store", () => {
    render(<VibrationSwitch />);
    const switchEl = screen.getByRole("switch", { name: "Vibration" }) as HTMLInputElement;
    expect(switchEl).toHaveAccessibleDescription(
      "A short buzz when your card lands, a hit lands and your turn starts, on a phone that can. Off under Reduce motion.",
    );
    expect(switchEl.checked).toBe(true);
    fireEvent.click(switchEl);
    expect(readHapticsSettings().vibration).toBe(false);
    expect(switchEl.checked).toBe(false);
  });
});

describe("R711000 the Language select", () => {
  it("R711000 as a page load builds it, offers every language, each named in its own words", async () => {
    const { controls } = await freshControls();
    render(<controls.LanguageSelect />);
    const select = screen.getByRole("combobox", { name: "Language" }) as HTMLSelectElement;
    expect([...select.options].map((option) => [option.value, option.text])).toEqual(
      Object.entries(LOCALES).map(([tag, spec]) => [tag, spec.name]),
    );
  });

  it("R711000 a value that is no language neither switches nor cancels a language still loading", async () => {
    render(<LanguageSelect />);
    const select = screen.getByTestId("setting-language");
    fireEvent.change(select, { target: { value: "zh-CN" } });
    fireEvent.change(select, { target: { value: "klingon" } });
    await waitFor(() => {
      expect(getLocale()).toBe("zh-CN");
    });
  });
});
