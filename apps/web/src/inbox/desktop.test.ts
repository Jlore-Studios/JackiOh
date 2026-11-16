// R4039001: desktop notifications while JackiOh is open. Turning them on asks the browser's permission and keeps
// the choice in this browser; a pushed row shows one only while the tab is hidden, for the kinds that ask
// something of the player, and its click opens where it happened. The browser's `Notification` is stubbed.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { FeedNotification } from "../net/notifications.ts";
import { resetVisibility, setVisibility } from "../test/setup.ts";
import { at } from "../test/stubServer.ts";
import { DESKTOP_NOTIFY_KEY } from "./config.ts";
import { desktopOn, desktopSupported, disableDesktop, enableDesktop, notifyDesktop } from "./desktop.ts";

const ANN = { profileId: "f-ann", username: "Ann#2" };

function row(id: number, kind: FeedNotification["kind"], over: Partial<FeedNotification> = {}): FeedNotification {
  return { id, kind, actor: ANN, subject: `m-${String(id)}`, payload: {}, createdAt: Date.now(), read: false, ...over };
}

/** The browser's `Notification`: its permission, the answer it gives when asked, and what it showed. */
class FakeNotification {
  static permission: NotificationPermission = "default";
  static answer: NotificationPermission = "granted";
  static shown: FakeNotification[] = [];
  static requestPermission = vi.fn(() => {
    FakeNotification.permission = FakeNotification.answer;
    return Promise.resolve(FakeNotification.answer);
  });

  onclick: (() => void) | null = null;
  closed = false;

  constructor(
    readonly title: string,
    readonly options: NotificationOptions,
  ) {
    FakeNotification.shown.push(this);
  }

  close(): void {
    this.closed = true;
  }
}

beforeEach(() => {
  window.localStorage.clear();
  FakeNotification.permission = "default";
  FakeNotification.answer = "granted";
  FakeNotification.shown = [];
  FakeNotification.requestPermission.mockClear();
  vi.stubGlobal("Notification", FakeNotification);
  at("/collection");
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  resetVisibility();
  window.localStorage.clear();
});

describe("R4039001 desktop notifications", () => {
  it("R4039001 turning them on asks the browser; a refusal leaves them off, and off forgets the choice", async () => {
    expect(desktopSupported()).toBe(true);
    expect(desktopOn()).toBe(false);

    FakeNotification.answer = "denied";
    await expect(enableDesktop()).resolves.toBe(false);
    expect(FakeNotification.requestPermission).toHaveBeenCalledTimes(1);
    expect(desktopOn()).toBe(false);
    expect(window.localStorage.getItem(DESKTOP_NOTIFY_KEY)).toBeNull();

    FakeNotification.answer = "granted";
    await expect(enableDesktop()).resolves.toBe(true);
    expect(window.localStorage.getItem(DESKTOP_NOTIFY_KEY)).toBe("on");
    expect(desktopOn()).toBe(true);

    // The browser's permission taken back turns them off, whatever this browser kept.
    FakeNotification.permission = "denied";
    expect(desktopOn()).toBe(false);
    FakeNotification.permission = "granted";

    disableDesktop();
    expect(desktopOn()).toBe(false);
    expect(window.localStorage.getItem(DESKTOP_NOTIFY_KEY)).toBeNull();
  });

  it("R4039001 a row shows one only while the tab is hidden, for the kinds that ask something of the player, and its click opens the row's place", async () => {
    // Off: nothing, hidden or not.
    setVisibility("hidden");
    notifyDesktop(row(1, "dailyMove"));
    expect(FakeNotification.shown).toHaveLength(0);

    await enableDesktop();
    setVisibility("visible");
    notifyDesktop(row(2, "dailyMove"));
    expect(FakeNotification.shown).toHaveLength(0);

    setVisibility("hidden");
    for (const kind of ["dailyEnded", "dailyAnswered", "friendAccepted", "replayShared"] as const) notifyDesktop(row(3, kind));
    expect(FakeNotification.shown).toHaveLength(0);

    notifyDesktop(row(8, "dailyAnswered", { payload: { answer: "declined" } }));
    expect(FakeNotification.shown).toHaveLength(0);

    notifyDesktop(row(9, "dailyAnswered", { payload: { answer: "accepted", matchId: "m-9" } }));
    notifyDesktop(row(4, "dailyChallenge", { payload: { mode: "random", perMoveDays: 3 } }));
    notifyDesktop(row(5, "lobbyInvite", { subject: "K7Q2XM", payload: { format: "duel" } }));
    notifyDesktop(row(6, "friendRequest"));
    notifyDesktop(row(7, "dailyMove"));
    expect(FakeNotification.shown.map((shown) => shown.options.body)).toEqual([
      "Ann#2 accepted your daily challenge",
      "Ann#2 challenged you to a daily game",
      "Ann#2 invited you to their lobby",
      "Ann#2 sent you a friend request",
      "Your move against Ann#2",
    ]);
    const last = FakeNotification.shown.at(-1);
    expect(last?.title).toBe("JackiOh");
    expect(last?.options.tag).toBe("jackioh-7");

    const focus = vi.spyOn(window, "focus").mockImplementation(() => undefined);
    last?.onclick?.();
    expect(focus).toHaveBeenCalled();
    expect(window.location.pathname).toBe("/daily/m-7");
    expect(last?.closed).toBe(true);
  });

  it("R4039001 a browser with no notifications offers none", async () => {
    // jsdom has none: the stub goes.
    vi.unstubAllGlobals();
    expect(desktopSupported()).toBe(false);
    await expect(enableDesktop()).resolves.toBe(false);
    expect(desktopOn()).toBe(false);
  });
});
