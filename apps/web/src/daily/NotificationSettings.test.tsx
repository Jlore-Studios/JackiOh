// R4039001: the notification settings. A server with no mailer (`emailAvailable` false) offers no email toggle and
// says where JackiOh tells the player instead; a browser that shows notifications offers desktop notifications
// while JackiOh is open, which ask its permission and stay off when it says no. The server is a stubbed `fetch`
// as `DailyRoute.test.tsx` stubs it, and the browser's `Notification` is stubbed.

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DESKTOP_NOTIFY_KEY } from "../inbox/config.ts";
import type { DailySettings } from "../net/daily.ts";
import { signIn, stubServer } from "../test/stubServer.ts";
import { NotificationSettings } from "./NotificationSettings.tsx";
import { dailyTestid } from "./testids.ts";

const SLOW = { timeout: 5_000 } as const;

const SETTINGS: DailySettings = { emailOn: true, hasEmail: true, vapidPublicKey: null, pushDevices: 0, emailAvailable: true };

function serve(settings: DailySettings): void {
  stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/daily/settings") return { status: 200, body: settings };
    return undefined;
  });
}

/** The browser's `Notification`, answering a request for permission with `answer`. */
function stubNotifications(answer: NotificationPermission): { requestPermission: ReturnType<typeof vi.fn> } {
  const fake = {
    permission: "default" as NotificationPermission,
    requestPermission: vi.fn(() => {
      fake.permission = answer;
      return Promise.resolve(answer);
    }),
  };
  vi.stubGlobal("Notification", fake);
  return fake;
}

beforeEach(() => {
  window.localStorage.clear();
  signIn("tok-settings");
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("R4039001 the notification settings", () => {
  it("R4039001 a server with no mailer offers no email toggle and says where JackiOh tells the player", async () => {
    serve({ ...SETTINGS, emailAvailable: false });
    render(<NotificationSettings />);
    expect(await screen.findByTestId(dailyTestid.emailUnavailable, undefined, SLOW)).toHaveTextContent(
      "JackiOh tells you here: in the bell, with a toast, and on this computer if you turn that on.",
    );
    expect(screen.queryByTestId(dailyTestid.email)).toBeNull();
    expect(screen.queryByTestId(dailyTestid.noEmail)).toBeNull();
  });

  it("R4039001 a server with a mailer, or one that does not say, keeps the email toggle", async () => {
    for (const settings of [SETTINGS, { emailOn: true, hasEmail: true, vapidPublicKey: null, pushDevices: 0 }]) {
      serve(settings);
      render(<NotificationSettings />);
      expect(await screen.findByTestId(dailyTestid.email, undefined, SLOW)).toBeChecked();
      expect(screen.queryByTestId(dailyTestid.emailUnavailable)).toBeNull();
      cleanup();
    }
  });

  it("R4039001 the desktop toggle asks the browser's permission; a refusal leaves it off and says so", async () => {
    const browser = stubNotifications("denied");
    serve(SETTINGS);
    render(<NotificationSettings />);
    const toggle = await screen.findByTestId(dailyTestid.desktop, undefined, SLOW);
    expect(toggle).not.toBeChecked();
    expect(screen.getByText("Show notifications on this computer while JackiOh is open")).toBeInTheDocument();

    fireEvent.click(toggle);
    expect(await screen.findByTestId(dailyTestid.desktopRefused, undefined, SLOW)).toHaveTextContent("This browser said no to notifications.");
    expect(browser.requestPermission).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId(dailyTestid.desktop)).not.toBeChecked();
    expect(window.localStorage.getItem(DESKTOP_NOTIFY_KEY)).toBeNull();
  });

  it("R4039001 the desktop toggle turns on with the browser's permission and off again, kept in this browser", async () => {
    stubNotifications("granted");
    serve(SETTINGS);
    render(<NotificationSettings />);
    const toggle = await screen.findByTestId(dailyTestid.desktop, undefined, SLOW);
    fireEvent.click(toggle);
    await waitFor(() => {
      expect(screen.getByTestId(dailyTestid.desktop)).toBeChecked();
    }, SLOW);
    expect(window.localStorage.getItem(DESKTOP_NOTIFY_KEY)).toBe("on");
    expect(screen.queryByTestId(dailyTestid.desktopRefused)).toBeNull();

    fireEvent.click(screen.getByTestId(dailyTestid.desktop));
    expect(screen.getByTestId(dailyTestid.desktop)).not.toBeChecked();
    expect(window.localStorage.getItem(DESKTOP_NOTIFY_KEY)).toBeNull();
  });

  it("R4039001 a browser with no notifications offers no desktop toggle", async () => {
    serve(SETTINGS);
    render(<NotificationSettings />);
    await screen.findByTestId(dailyTestid.email, undefined, SLOW);
    expect(screen.queryByTestId(dailyTestid.desktop)).toBeNull();
  });
});
