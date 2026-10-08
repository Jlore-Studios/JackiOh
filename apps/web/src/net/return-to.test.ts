// Where a sign-in lands (issue #479): the main menu, unless a gated screen sent the player to sign
// in, and then that screen, once. Never a URL (B35). The screens that use it are tested through the
// real App in `routes/sign-in-landing.test.tsx`.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { paths } from "./navigate.ts";
import {
  RETURN_TO_STORAGE_KEY,
  SIGN_IN_HOME,
  forgetReturnTo,
  rememberReturnTo,
  signInDestination,
  takeReturnTo,
} from "./return-to.ts";

beforeEach(() => {
  window.sessionStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
  window.sessionStorage.clear();
});

describe("issue #479 where a sign-in lands", () => {
  it("the main menu is the landing page", () => {
    expect(SIGN_IN_HOME).toBe(paths.landing);
  });

  it("with no gated screen waiting, a sign-in lands on the main menu", () => {
    expect(signInDestination()).toBe(paths.landing);
  });

  it.each([paths.decks, paths.play, paths.invite, paths.account])(
    "a sign-in the gate sent from %s goes back there, once",
    (path) => {
      rememberReturnTo(path);
      expect(signInDestination()).toBe(path);
      expect(window.sessionStorage.getItem(RETURN_TO_STORAGE_KEY)).toBeNull();
      // Read once: the next sign-in in this tab starts from the main menu again.
      expect(signInDestination()).toBe(paths.landing);
    },
  );

  it("B35 a destination that is not a fixed gated path lands on the main menu instead", () => {
    for (const planted of ["https://evil.example/play", "//evil.example", "/match/some-id", paths.resetPassword]) {
      window.sessionStorage.setItem(RETURN_TO_STORAGE_KEY, planted);
      expect(signInDestination()).toBe(paths.landing);
    }
  });

  it("a gate that sends the player from a screen it does not return to forgets the one before", () => {
    rememberReturnTo(paths.play);
    rememberReturnTo(paths.leaderboard);
    expect(signInDestination()).toBe(paths.landing);
  });

  it("back on the main menu, the screen that asked is forgotten", () => {
    rememberReturnTo(paths.decks);
    forgetReturnTo();
    expect(takeReturnTo()).toBeNull();
    expect(signInDestination()).toBe(paths.landing);
  });

  it("blocked storage lands every sign-in on the main menu and throws nothing", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new DOMException("blocked", "SecurityError");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("blocked", "SecurityError");
    });
    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
      throw new DOMException("blocked", "SecurityError");
    });
    expect(() => {
      rememberReturnTo(paths.play);
    }).not.toThrow();
    expect(() => {
      forgetReturnTo();
    }).not.toThrow();
    expect(signInDestination()).toBe(paths.landing);
  });
});
