// R1916000: the language button leaves the top bar and the board once the device has settled on its language; settings' Language row stays.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import Board from "../game/Board.tsx";
import { BackLink } from "../routes/nav.tsx";
import { LanguageSelect } from "../settings/controls.tsx";
import { SETTINGS_SLOTS } from "../settings/slots.ts";
import { fullBoardView } from "../test/fixtures.ts";
import {
  countLocaleVisit,
  getLocale,
  isLocaleSettled,
  LANGUAGE_SETTLED_VISITS,
  LOCALE_VISITS_STORAGE_KEY,
  resetLocaleForTests,
} from "./i18n.ts";

beforeEach(() => {
  resetLocaleForTests();
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  resetLocaleForTests();
});

function screens(): void {
  render(
    <>
      <BackLink />
      <Board view={fullBoardView()} />
    </>,
  );
}

function settleDevice(): void {
  act(() => {
    for (let i = 0; i < LANGUAGE_SETTLED_VISITS; i++) countLocaleVisit();
  });
}

describe("R1916000 the language button once the device has settled", () => {
  it("R1916000 the nav and the board show the language button below LANGUAGE_SETTLED_VISITS boots and drop it once reached", () => {
    for (let visit = 1; visit < LANGUAGE_SETTLED_VISITS; visit++) {
      countLocaleVisit();
      expect(window.localStorage.getItem(LOCALE_VISITS_STORAGE_KEY)).toBe(String(visit));
      screens();
      expect(screen.getByTestId("language-nav")).toBeInTheDocument();
      expect(screen.getByTestId("language-game")).toBeInTheDocument();
      cleanup();
    }
    countLocaleVisit();
    expect(window.localStorage.getItem(LOCALE_VISITS_STORAGE_KEY)).toBe(String(LANGUAGE_SETTLED_VISITS));
    screens();
    expect(screen.queryByTestId("language-nav")).toBeNull();
    expect(screen.queryByTestId("language-game")).toBeNull();
    expect(screen.getByTestId("settings-open-nav")).toBeInTheDocument();
    expect(screen.getByTestId("settings-open-game")).toBeInTheDocument();
  });

  it("R1916000 a language change from settings or the button, or settings' reset, sets the count to 0 and brings the button back", async () => {
    settleDevice();
    render(
      <>
        <BackLink />
        <LanguageSelect />
      </>,
    );
    expect(screen.queryByTestId("language-nav")).toBeNull();
    fireEvent.change(screen.getByTestId("setting-language"), { target: { value: "zh-CN" } });
    await vi.waitFor(() => {
      expect(screen.getByTestId("language-nav")).toBeInTheDocument();
    });
    expect(window.localStorage.getItem(LOCALE_VISITS_STORAGE_KEY)).toBe("0");
    expect(getLocale()).toBe("zh-CN");

    act(() => {
      for (let i = 0; i < LANGUAGE_SETTLED_VISITS - 1; i++) countLocaleVisit();
    });
    expect(screen.getByTestId("language-nav")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("language-nav"));
    await vi.waitFor(() => {
      expect(getLocale()).toBe("en");
    });
    expect(window.localStorage.getItem(LOCALE_VISITS_STORAGE_KEY)).toBe("0");
    act(() => {
      for (let i = 0; i < LANGUAGE_SETTLED_VISITS - 1; i++) countLocaleVisit();
    });
    // Without the reset the count would have passed LANGUAGE_SETTLED_VISITS and the button would be gone.
    expect(screen.getByTestId("language-nav")).toBeInTheDocument();

    settleDevice();
    expect(screen.queryByTestId("language-nav")).toBeNull();
    act(() => {
      SETTINGS_SLOTS.find((slot) => slot.id === "language")?.reset?.();
    });
    await vi.waitFor(() => {
      expect(screen.getByTestId("language-nav")).toBeInTheDocument();
    });
    expect(window.localStorage.getItem(LOCALE_VISITS_STORAGE_KEY)).toBe("0");
  });

  it("R1916000 with storage blocked the device never settles and the button always shows", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    for (let i = 0; i < LANGUAGE_SETTLED_VISITS + 1; i++) countLocaleVisit();
    expect(isLocaleSettled()).toBe(false);
    vi.restoreAllMocks();
    screens();
    // The button reads no storage at render, only the boot's count does.
    expect(screen.getByTestId("language-nav")).toBeInTheDocument();
    expect(screen.getByTestId("language-game")).toBeInTheDocument();

    // Write-only failure.
    cleanup();
    window.localStorage.setItem(LOCALE_VISITS_STORAGE_KEY, String(LANGUAGE_SETTLED_VISITS));
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    countLocaleVisit();
    expect(isLocaleSettled()).toBe(false);
  });

  it("R1916000 settings' Language row renders at every count and its hint names the top button only while it shows", () => {
    render(<LanguageSelect />);
    expect(screen.getByTestId("setting-language")).toBeInTheDocument();
    expect(
      screen.getByText(
        "The screens and the cards read in this language. The button at the top of every screen switches it too.",
      ),
    ).toBeInTheDocument();
    settleDevice();
    expect(screen.getByTestId("setting-language")).toBeInTheDocument();
    expect(screen.getByText("The screens and the cards read in this language.")).toBeInTheDocument();
    expect(screen.queryByText(/The button at the top of every screen/u)).toBeNull();
  });
});
