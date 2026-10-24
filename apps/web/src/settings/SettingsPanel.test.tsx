// R1758000: the settings dialog says when its save to the account failed, with Try again (#1758).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayerSettingsAccountCopy, PlayerSettingsGroup } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import {
  SETTINGS_SYNC_DEBOUNCE_MS,
  __resetSettingsSyncForTests,
  readSettingsSyncState,
  useSettingsAccountSync,
  type SettingsAccountApi,
} from "./accountSync.ts";
import SettingsPanel from "./SettingsPanel.tsx";
import { __resetSettingsForTests, readSettings } from "./store.ts";

type FlakyApi = SettingsAccountApi & {
  load: ReturnType<typeof vi.fn>;
  save: ReturnType<typeof vi.fn>;
  fail: boolean;
  gate: Promise<void> | null;
};

/** An account that holds nothing, answers every request and fails or waits on the test's say. */
function flakyApi(): FlakyApi {
  const api: FlakyApi = {
    fail: false,
    gate: null,
    load: vi.fn(async () => {
      if (api.gate !== null) await api.gate;
      if (api.fail) throw new Error("offline");
      return { groups: {} } satisfies PlayerSettingsAccountCopy;
    }),
    save: vi.fn(async (_token: string, groups: Record<string, PlayerSettingsGroup>) => {
      if (api.gate !== null) await api.gate;
      if (api.fail) throw new Error("offline");
      return { groups: structuredClone(groups) } satisfies PlayerSettingsAccountCopy;
    }),
  };
  return api;
}

function ready(profileId: string): Account {
  return { kind: "ready", token: "tok", me: { profile: { id: profileId, status: "active" } } } as unknown as Account;
}

function Screen({ account, api }: { account: Account; api: SettingsAccountApi }): ReactElement {
  useSettingsAccountSync(account, api);
  return <SettingsPanel onClose={() => undefined} slots={[]} />;
}

async function flush(ms = 0): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

function notice(): HTMLElement | null {
  return screen.queryByTestId("settings-sync-error");
}

beforeEach(() => {
  vi.useFakeTimers();
  localStorage.clear();
  __resetSettingsForTests();
  __resetSettingsSyncForTests();
});

afterEach(() => {
  cleanup();
  __resetSettingsSyncForTests();
  vi.useRealTimers();
  localStorage.clear();
  __resetSettingsForTests();
});

describe("R1758000 the settings dialog's notice of a failed account save", () => {
  it("R1758000 a load that fails shows the notice with Try again, outside the tabs, and Try again loads again and clears it", async () => {
    const api = flakyApi();
    api.fail = true;
    render(<Screen account={ready("p1")} api={api} />);
    await flush();
    expect(readSettingsSyncState().phase).toBe("error");
    expect(notice()).toHaveTextContent("Settings didn’t save to your account.");
    expect(notice()?.closest('[role="tabpanel"]')).toBeNull();
    expect(screen.getByTestId("settings-sync-retry")).toHaveTextContent("Try again");

    api.fail = false;
    fireEvent.click(screen.getByTestId("settings-sync-retry"));
    expect(readSettingsSyncState().phase).toBe("syncing");
    expect(notice()).toBeNull();
    await flush();
    expect(api.load).toHaveBeenCalledTimes(2);
    expect(readSettingsSyncState().phase).toBe("saved");
    expect(notice()).toBeNull();
  });

  it("R1758000 a save that fails keeps the setting, shows the notice, and Try again sends it and clears the notice", async () => {
    const api = flakyApi();
    render(<Screen account={ready("p1")} api={api} />);
    await flush();
    expect(notice()).toBeNull();

    api.fail = true;
    const before = readSettings().confirmEndTurn;
    fireEvent.click(screen.getByTestId("setting-confirmEndTurn"));
    await flush(SETTINGS_SYNC_DEBOUNCE_MS);
    expect(readSettings().confirmEndTurn).toBe(!before);
    expect(notice()).toBeInTheDocument();

    api.fail = false;
    fireEvent.click(screen.getByTestId("settings-sync-retry"));
    await flush();
    expect(api.save).toHaveBeenCalledTimes(2);
    expect(Object.keys((api.save.mock.calls[1] as [string, Record<string, PlayerSettingsGroup>])[1])).toEqual([
      "gameplay",
    ]);
    expect(notice()).toBeNull();
  });

  it("R1758000 shows no notice with nothing syncing, while a request is out, or once saved", async () => {
    const api = flakyApi();
    const view = render(<Screen account={{ kind: "anonymous" } as unknown as Account} api={api} />);
    await flush();
    expect(readSettingsSyncState().phase).toBe("signedOut");
    expect(notice()).toBeNull();
    expect(api.load).not.toHaveBeenCalled();

    let release: () => void = () => undefined;
    api.gate = new Promise<void>((resolve) => {
      release = resolve;
    });
    view.rerender(<Screen account={ready("p1")} api={api} />);
    await flush();
    expect(readSettingsSyncState().phase).toBe("syncing");
    expect(notice()).toBeNull();

    api.gate = null;
    release();
    await flush();
    expect(readSettingsSyncState().phase).toBe("saved");
    expect(notice()).toBeNull();
  });

  it("R1758000 R634's automatic retry on coming back online clears the notice too", async () => {
    const api = flakyApi();
    api.fail = true;
    render(<Screen account={ready("p1")} api={api} />);
    await flush();
    expect(notice()).toBeInTheDocument();

    api.fail = false;
    await act(async () => {
      window.dispatchEvent(new Event("online"));
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(api.load).toHaveBeenCalledTimes(2);
    expect(notice()).toBeNull();
  });
});
