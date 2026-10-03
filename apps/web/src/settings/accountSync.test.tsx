// R633, R634: a player's game settings kept on the account as well as on the device. The sync is
// driven here against the real stores (`groups.ts`) and `localStorage`, with a fake API that merges
// the way the server does (a group replaces the stored one only when strictly later).

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { readAudioSettings, resetAudioSettingsForTests, writeAudioSettings } from "../audio/settings.ts";
import { readCardSettings, writeCardSettings } from "../cards/settings.ts";
import { getFxSettings, resetFxSettingsForTests, setFxSettings } from "../fx/settings.ts";
import type { PlayerSettingsAccountCopy, PlayerSettingsGroup } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import { clearSession, writeSession } from "../net/session.ts";
import AccountSettings from "./AccountSettings.tsx";
import {
  SETTINGS_SYNC_DEBOUNCE_MS,
  SETTINGS_SYNC_GRACE_MS,
  SETTINGS_SYNC_STORAGE_KEY,
  __resetSettingsSyncForTests,
  readSettingsSyncState,
  startSettingsAccountSync,
  useSettingsAccountSync,
  type SettingsAccountApi,
  type SettingsAccountSync,
} from "./accountSync.ts";
import { __resetSettingsForTests, readSettings, writeSettings } from "./store.ts";

const T0 = 1_800_000_000_000;
let wallClock = T0;
const now = (): number => wallClock;

type FakeApi = SettingsAccountApi & {
  account: Record<string, PlayerSettingsGroup>;
  load: ReturnType<typeof vi.fn>;
  save: ReturnType<typeof vi.fn>;
  /** The server's clock: a group timed after it is taken as made now. */
  serverNow: number;
  fail: boolean;
  gate: Promise<void> | null;
};

/** The server's side of R634 in a dozen lines: a strictly later group replaces, the rest stay. */
function fakeApi(initial: Record<string, PlayerSettingsGroup> = {}): FakeApi {
  const api: FakeApi = {
    account: structuredClone(initial),
    serverNow: T0 + 10 * 60_000,
    fail: false,
    gate: null,
    load: vi.fn(async () => {
      if (api.fail) throw new Error("offline");
      return { groups: structuredClone(api.account) } satisfies PlayerSettingsAccountCopy;
    }),
    save: vi.fn(async (_token: string, groups: Record<string, PlayerSettingsGroup>) => {
      if (api.gate !== null) await api.gate;
      if (api.fail) throw new Error("offline");
      for (const [id, sent] of Object.entries(groups)) {
        const held = api.account[id];
        const at = Math.min(sent.at, api.serverNow);
        if (held === undefined || at > held.at) api.account[id] = { at, values: structuredClone(sent.values) };
      }
      return { groups: structuredClone(api.account) } satisfies PlayerSettingsAccountCopy;
    }),
  };
  return api;
}

const started: SettingsAccountSync[] = [];

function start(api: SettingsAccountApi): SettingsAccountSync {
  const sync = startSettingsAccountSync({ token: () => "token", api, now, debounceMs: SETTINGS_SYNC_DEBOUNCE_MS });
  started.push(sync);
  return sync;
}

async function settle(): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
    for (const sync of started) await sync.settled();
  });
}

async function pass(ms: number): Promise<void> {
  await act(async () => {
    wallClock += ms;
    await vi.advanceTimersByTimeAsync(ms);
    for (const sync of started) await sync.settled();
  });
}

/** Time passes and timers run, without waiting for a request that is held back. */
async function tick(ms: number): Promise<void> {
  await act(async () => {
    wallClock += ms;
    await vi.advanceTimersByTimeAsync(ms);
  });
}

function storedClock(): Record<string, number> {
  return JSON.parse(localStorage.getItem(SETTINGS_SYNC_STORAGE_KEY) ?? "{}") as Record<string, number>;
}

beforeEach(() => {
  vi.useFakeTimers();
  wallClock = T0;
  localStorage.clear();
  __resetSettingsForTests();
  resetAudioSettingsForTests();
  resetFxSettingsForTests();
  __resetSettingsSyncForTests();
});

afterEach(() => {
  cleanup();
  for (const sync of started.splice(0)) sync.stop();
  __resetSettingsSyncForTests();
  vi.useRealTimers();
  localStorage.clear();
  __resetSettingsForTests();
  resetAudioSettingsForTests();
  resetFxSettingsForTests();
});

describe("R634 loading: group by group, the newer side wins", () => {
  it("R634 a new device takes up the account's groups and sends nothing back", async () => {
    const api = fakeApi({
      audio: { at: T0 - 5_000, values: { master: 0.3, muted: true, station: "lofi" } },
      gameplay: { at: T0 - 4_000, values: { dragToPlay: false, confirmEndTurn: true } },
      fx: { at: T0 - 3_000, values: { speed: 2, intensity: "low" } },
      cards: { at: T0 - 2_000, values: { animatedFoil: false } },
    });
    start(api);
    await settle();

    expect(readAudioSettings()).toMatchObject({ master: 0.3, muted: true, station: "lofi" });
    expect(readSettings()).toMatchObject({ dragToPlay: false, confirmEndTurn: true });
    expect(getFxSettings()).toMatchObject({ speed: 2, intensity: "low" });
    expect(readCardSettings().animatedFoil).toBe(false);
    // Adopted at the account's own times, so nothing is owed.
    expect(storedClock()).toEqual({ audio: T0 - 5_000, gameplay: T0 - 4_000, fx: T0 - 3_000, cards: T0 - 2_000 });
    expect(api.save).not.toHaveBeenCalled();
    expect(readSettingsSyncState().phase).toBe("saved");
  });

  it("R634 a change made here while signed out goes up when the account has not changed it since, and only that group", async () => {
    writeAudioSettings({ master: 0.2 });
    const changedAt = storedClock();
    expect(changedAt).toEqual({});
    // No sync was running: nothing stamped the change, so stamp it as the sync would have.
    localStorage.setItem(SETTINGS_SYNC_STORAGE_KEY, JSON.stringify({ audio: T0 - 1_000 }));
    const api = fakeApi({ gameplay: { at: T0 - 9_000, values: { dragToPlay: false } } });

    start(api);
    await settle();

    expect(api.save).toHaveBeenCalledTimes(1);
    const sent = (api.save.mock.calls[0] as [string, Record<string, PlayerSettingsGroup>])[1];
    expect(Object.keys(sent)).toEqual(["audio"]);
    expect(sent["audio"]).toMatchObject({ at: T0 - 1_000, values: { master: 0.2 } });
    // And the account's gameplay group, which this device never changed, was adopted.
    expect(readSettings().dragToPlay).toBe(false);
    expect(api.account["audio"]?.values).toMatchObject({ master: 0.2 });
  });

  it("R634 a stale device does not undo a newer change on the account, and a newer one here does not lose to an older one there", async () => {
    localStorage.setItem(SETTINGS_SYNC_STORAGE_KEY, JSON.stringify({ audio: T0 - 8_000, gameplay: T0 - 1_000 }));
    writeAudioSettings({ master: 0.1 });
    writeSettings({ confirmEndTurn: true });
    localStorage.setItem(SETTINGS_SYNC_STORAGE_KEY, JSON.stringify({ audio: T0 - 8_000, gameplay: T0 - 1_000 }));
    const api = fakeApi({
      audio: { at: T0 - 2_000, values: { master: 0.9 } },
      gameplay: { at: T0 - 6_000, values: { confirmEndTurn: false } },
    });

    start(api);
    await settle();

    // Audio: the account's is later (T0 - 2000 > T0 - 8000), so it is taken.
    expect(readAudioSettings().master).toBe(0.9);
    // Gameplay: this device's is later, so it is sent and kept.
    expect(readSettings().confirmEndTurn).toBe(true);
    expect(api.account["gameplay"]).toMatchObject({ at: T0 - 1_000, values: { confirmEndTurn: true } });
    expect(Object.keys((api.save.mock.calls[0] as [string, Record<string, unknown>])[1])).toEqual(["gameplay"]);
  });

  it("R634 values an old account sends are clamped and filtered by the store, and groups this client does not have are ignored", async () => {
    const api = fakeApi({
      audio: { at: T0 - 1_000, values: { master: 9, station: "polka", nonsense: 1 } },
      "store-from-the-future": { at: T0, values: { x: true } },
    });
    start(api);
    await settle();

    expect(readAudioSettings()).toMatchObject({ master: 1, station: "tavern" });
    expect(api.save).not.toHaveBeenCalled();
    expect(api.account["store-from-the-future"]).toBeDefined();
  });

  it("R634 an answer that is not settings changes nothing here and says so", async () => {
    const api = fakeApi();
    api.load.mockResolvedValueOnce({ groups: null } as unknown as PlayerSettingsAccountCopy);
    writeAudioSettings({ master: 0.4 });
    start(api);
    await settle();

    expect(readAudioSettings().master).toBe(0.4);
    expect(readSettingsSyncState().phase).toBe("error");
    expect(api.save).not.toHaveBeenCalled();
  });
});

describe("R634 after the load, each change is sent as it happens", () => {
  it("R634 a burst of changes is one request after the debounce, with the whole group and the time of the last change", async () => {
    const api = fakeApi();
    start(api);
    await settle();
    expect(api.save).not.toHaveBeenCalled();

    await pass(1_000);
    writeAudioSettings({ master: 0.1 });
    await pass(100);
    writeAudioSettings({ master: 0.2 });
    await pass(100);
    writeAudioSettings({ master: 0.3, muted: true });
    expect(api.save).not.toHaveBeenCalled();
    const lastChange = wallClock;

    await pass(SETTINGS_SYNC_DEBOUNCE_MS);

    expect(api.save).toHaveBeenCalledTimes(1);
    const sent = (api.save.mock.calls[0] as [string, Record<string, PlayerSettingsGroup>])[1];
    expect(sent["audio"]?.at).toBe(lastChange);
    expect(sent["audio"]?.values).toMatchObject({ master: 0.3, muted: true, station: "tavern" });
    expect(Object.keys(sent)).toEqual(["audio"]);
    expect(api.account["audio"]?.values).toMatchObject({ master: 0.3 });
    expect(readSettingsSyncState().phase).toBe("saved");
  });

  it("R634 changes in different stores go up together, each with its own time", async () => {
    const api = fakeApi();
    start(api);
    await settle();

    await pass(1_000);
    setFxSettings({ speed: 2 });
    await pass(50);
    writeCardSettings({ animatedFoil: false });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);

    const sent = (api.save.mock.calls[0] as [string, Record<string, PlayerSettingsGroup>])[1];
    expect(Object.keys(sent).sort()).toEqual(["cards", "fx"]);
    expect(sent["fx"]?.at).toBeLessThan(sent["cards"]?.at ?? 0);
  });

  it("R634 takes the account's values in without sending them back: adopting is not a change", async () => {
    const api = fakeApi({ audio: { at: T0 - 100, values: { master: 0.6 } } });
    start(api);
    await settle();
    await pass(SETTINGS_SYNC_DEBOUNCE_MS * 3);

    expect(readAudioSettings().master).toBe(0.6);
    expect(api.save).not.toHaveBeenCalled();
  });

  it("R634 keeps one request in flight: a change made meanwhile is sent when the first answers", async () => {
    const api = fakeApi();
    start(api);
    await settle();

    let release: () => void = () => undefined;
    api.gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    await tick(1_000);
    writeAudioSettings({ master: 0.1 });
    await tick(SETTINGS_SYNC_DEBOUNCE_MS);
    expect(api.save).toHaveBeenCalledTimes(1);

    writeAudioSettings({ master: 0.2 });
    await tick(SETTINGS_SYNC_DEBOUNCE_MS * 2);
    expect(api.save).toHaveBeenCalledTimes(1);

    api.gate = null;
    release();
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);
    expect(api.save).toHaveBeenCalledTimes(2);
    expect(api.account["audio"]?.values).toMatchObject({ master: 0.2 });
    expect(readAudioSettings().master).toBe(0.2);
  });

  it("R634 a reset is a change like any other: the defaults go up", async () => {
    const api = fakeApi({ audio: { at: T0 - 100, values: { master: 0.1 } } });
    start(api);
    await settle();
    expect(readAudioSettings().master).toBe(0.1);

    await pass(1_000);
    writeAudioSettings({ master: 0.8 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);

    expect(api.account["audio"]?.values).toMatchObject({ master: 0.8 });
  });
});

describe("R634 a request that fails is dropped, and the device keeps what it has", () => {
  it("R634 a failed send leaves the setting in force, says so, and goes again on the player's retry", async () => {
    const api = fakeApi();
    const sync = start(api);
    await settle();

    api.fail = true;
    await pass(1_000);
    writeAudioSettings({ master: 0.2 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);

    expect(readAudioSettings().master).toBe(0.2);
    expect(readSettingsSyncState().phase).toBe("error");
    expect(api.account["audio"]).toBeUndefined();

    api.fail = false;
    sync.retry();
    await settle();

    expect(readSettingsSyncState().phase).toBe("saved");
    expect(api.account["audio"]?.values).toMatchObject({ master: 0.2 });
  });

  it("R634 a load that fails is tried again by retry, and a change made meanwhile is not lost", async () => {
    const api = fakeApi({ gameplay: { at: T0 - 100, values: { dragToPlay: false } } });
    api.fail = true;
    const sync = start(api);
    await settle();
    expect(readSettingsSyncState().phase).toBe("error");

    await pass(1_000);
    writeAudioSettings({ master: 0.2 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS * 2);
    expect(api.save).not.toHaveBeenCalled();

    api.fail = false;
    sync.retry();
    await settle();

    expect(readSettings().dragToPlay).toBe(false);
    expect(api.account["audio"]?.values).toMatchObject({ master: 0.2 });
    expect(readSettingsSyncState().phase).toBe("saved");
  });

  it("R634 coming back online sends what a failure left owed", async () => {
    const api = fakeApi();
    start(api);
    await settle();
    api.fail = true;
    await pass(1_000);
    writeAudioSettings({ master: 0.5 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);
    expect(readSettingsSyncState().phase).toBe("error");

    api.fail = false;
    await act(async () => {
      window.dispatchEvent(new Event("online"));
      await vi.advanceTimersByTimeAsync(0);
      for (const sync of started) await sync.settled();
    });

    expect(api.account["audio"]?.values).toMatchObject({ master: 0.5 });
  });
});

describe("R634 a device whose clock runs ahead cannot pin its settings", () => {
  it("R634 takes the server's earlier time for a group it kept with this device's values, and stops sending it", async () => {
    const api = fakeApi();
    api.serverNow = T0 + 60_000;
    start(api);
    await settle();

    // This device's clock is a day ahead of the server's.
    wallClock = T0 + 86_400_000;
    writeAudioSettings({ master: 0.2 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);

    expect(api.save).toHaveBeenCalledTimes(1);
    expect(api.account["audio"]?.at).toBe(T0 + 60_000);
    // Level now: the device holds the server's time for it, so a later load sends nothing.
    expect(storedClock()["audio"]).toBe(T0 + 60_000);
    await pass(SETTINGS_SYNC_DEBOUNCE_MS * 5);
    expect(api.save).toHaveBeenCalledTimes(1);
  });
});

describe("R634 stopping", () => {
  it("R634 stop ends the listening: later changes send nothing and an answer in flight is ignored", async () => {
    const api = fakeApi();
    const sync = start(api);
    await settle();

    sync.stop();
    await pass(1_000);
    writeAudioSettings({ master: 0.2 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS * 3);

    expect(api.save).not.toHaveBeenCalled();
  });
});

describe("R634 the hook: one sync for the page, only for an active account", () => {
  function ready(profileId: string, status: "active" | "pending" | "banned" = "active", token = "tok"): Account {
    return { kind: "ready", token, me: { profile: { id: profileId, status } } } as unknown as Account;
  }

  function Screen({ account, api }: { account: Account; api: SettingsAccountApi }) {
    useSettingsAccountSync(account, api);
    return null;
  }

  it("R634 syncs for an active account, once however many screens ask, and sends nothing for a pending, banned or signed-out one", async () => {
    const api = fakeApi();
    for (const account of [
      ready("p1", "pending"),
      ready("p1", "banned"),
      { kind: "anonymous" } as unknown as Account,
      { kind: "loading" } as unknown as Account,
    ]) {
      const view = render(<Screen account={account} api={api} />);
      await settle();
      view.unmount();
    }
    expect(api.load).not.toHaveBeenCalled();

    const first = render(<Screen account={ready("p1")} api={api} />);
    const second = render(<Screen account={ready("p1")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(api.load).toHaveBeenCalledTimes(1);

    first.unmount();
    second.unmount();
  });

  it("R634 survives a move between screens, stops a moment after the last one leaves, and restarts for another profile", async () => {
    const api = fakeApi();
    const one = render(<Screen account={ready("p1")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    one.unmount();
    // The next screen mounts before the grace runs out: the same sync carries on.
    const two = render(<Screen account={ready("p1")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SETTINGS_SYNC_GRACE_MS * 2);
    });
    expect(api.load).toHaveBeenCalledTimes(1);
    expect(readSettingsSyncState().phase).toBe("saved");

    two.unmount();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(SETTINGS_SYNC_GRACE_MS + 1);
    });
    expect(readSettingsSyncState().phase).toBe("signedOut");

    const another = render(<Screen account={ready("p2")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(api.load).toHaveBeenCalledTimes(2);
    another.unmount();
  });

  it("R634 uses the screen's current token for each request", async () => {
    const api = fakeApi();
    const view = render(<Screen account={ready("p1", "active", "first")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    view.rerender(<Screen account={ready("p1", "active", "renewed")} api={api} />);
    await pass(1_000);
    writeAudioSettings({ master: 0.2 });
    await pass(SETTINGS_SYNC_DEBOUNCE_MS);

    expect(api.load.mock.calls[0]?.[0]).toBe("first");
    expect(api.save.mock.calls[0]?.[0]).toBe("renewed");
    view.unmount();
  });
});

describe("R633 the Account tab's status", () => {
  function ready(profileId: string): Account {
    return { kind: "ready", token: "tok", me: { profile: { id: profileId, status: "active" } } } as unknown as Account;
  }

  function Screen({ account, api }: { account: Account; api: SettingsAccountApi }) {
    useSettingsAccountSync(account, api);
    return <AccountSettings />;
  }

  it("R633 signed out it says the settings are on this device only and offers Sign in; signed in but not syncing here, it does not", () => {
    clearSession();
    const out = render(<AccountSettings />);
    expect(screen.getByRole("status").textContent).toContain("Saved on this device only");
    expect(screen.getByTestId("settings-sign-in")).toBeInTheDocument();
    expect(screen.getByTestId("settings-account")).toHaveAttribute("data-phase", "signedOut");
    out.unmount();

    writeSession({ accessToken: "a", refreshToken: "r", expiresAt: null });
    render(<AccountSettings />);
    expect(screen.getByRole("status").textContent).toContain("reach your account the next time");
    expect(screen.queryByTestId("settings-sign-in")).toBeNull();
    clearSession();
  });

  it("R633 shows saved with the time once the account has the settings, and saving while a request is out", async () => {
    const api = fakeApi();
    let release: () => void = () => undefined;
    const view = render(<Screen account={ready("p1")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByTestId("settings-account")).toHaveAttribute("data-phase", "saved");
    expect(screen.getByRole("status").textContent).toContain("Saved to your account.");
    expect(view.container.querySelector("time")).not.toBeNull();

    api.gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    writeAudioSettings({ master: 0.2 });
    await tick(SETTINGS_SYNC_DEBOUNCE_MS + 1);
    expect(screen.getByTestId("settings-account")).toHaveAttribute("data-phase", "syncing");
    expect(screen.getByRole("status").textContent).toContain("Saving to your account");

    api.gate = null;
    await act(async () => {
      release();
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByTestId("settings-account")).toHaveAttribute("data-phase", "saved");
    view.unmount();
  });

  it("R633 an error says so and offers Try again, which asks the account again", async () => {
    const api = fakeApi();
    api.fail = true;
    const view = render(<Screen account={ready("p1")} api={api} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(screen.getByTestId("settings-account")).toHaveAttribute("data-phase", "error");
    expect(screen.getByRole("status").textContent).toContain("Couldn't reach your account");
    expect(screen.queryByTestId("settings-sign-in")).toBeNull();

    api.fail = false;
    await act(async () => {
      screen.getByTestId("settings-sync-retry").click();
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(screen.getByTestId("settings-account")).toHaveAttribute("data-phase", "saved");
    expect(screen.queryByTestId("settings-sync-retry")).toBeNull();
    view.unmount();
  });
});
