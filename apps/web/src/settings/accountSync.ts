// Game settings kept on the account as well as on the device (SPEC §9.1, R633, R634).
//
// The device's stores (`groups.ts`) are the ones the game reads, and they stay authoritative for the
// session: nothing here ever waits on the network before a setting takes effect, and no failure
// reaches the game. For an ACTIVE account (a pending one has only the code screen, §9.4, and the
// routes answer it 403), this module keeps the account's copy level with the device:
//
//   1. On load it reads the account's copy (`GET /api/settings`) and, group by group, takes the
//      newer side: a group the account holds with a later time than this device last changed it is
//      applied to the device's store; a group this device changed later than the account holds is
//      sent up (`PUT /api/settings`), which the server merges by the same rule.
//   2. After that, every change on the device (a switch, a slider, another tab's change, a reset) is
//      stamped with the time it was made and sent up, a moment later so a slider being dragged is one
//      request, and the server's answer is reconciled again.
//
// A group is one store (gameplay, audio, effects, card display); its time is the device's clock when
// it last changed here, kept in `localStorage` beside the stores (`SETTINGS_SYNC_STORAGE_KEY`). A
// group no change was ever made to on this device has no time, so the account's copy always wins over
// it: a new device takes up the account's settings, and a device that changed one while signed out
// brings that change to the account if no one has changed it since. Signing in on a device adds its
// later changes to the account, as R321 does for the tutorial: settings belong to the device as much
// as to the account.
//
// A request that fails, or never answers, is dropped: the device keeps what it has, the status says
// so (`useSettingsSyncState`), and the next change, the next load, coming back online or the
// player's own retry sends what is owed. At most one request is in flight; changes made meanwhile are
// sent together once it answers. Signed out, there is no account to sync, so nothing is sent at all.

import { useEffect, useRef, useSyncExternalStore } from "react";

import { getPlayerSettings, putPlayerSettings, type PlayerSettingsAccountCopy, type PlayerSettingsGroup } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import { SETTINGS_GROUPS, type SettingsGroup } from "./groups.ts";

/** Where this device keeps, per group, when it last changed here (epoch ms). Not a setting. */
export const SETTINGS_SYNC_STORAGE_KEY = "jackioh.settings.sync.v1";
/** How long after a change the groups it touched are sent, so a dragged slider is one request. */
export const SETTINGS_SYNC_DEBOUNCE_MS = 600;
/** How long the sync outlives the last screen using it, so moving between screens does not restart it. */
export const SETTINGS_SYNC_GRACE_MS = 1500;

/** The two requests, injectable so the tests never touch the network. */
export type SettingsAccountApi = {
  load(token: string): Promise<PlayerSettingsAccountCopy>;
  save(token: string, groups: Record<string, PlayerSettingsGroup>): Promise<PlayerSettingsAccountCopy>;
};

export const settingsAccountApi: SettingsAccountApi = {
  load: getPlayerSettings,
  save: putPlayerSettings,
};

// ---------------------------------------------------------------------------------------------
// The status the Account tab shows
// ---------------------------------------------------------------------------------------------

/** `signedOut`: nothing is syncing (no active account on this screen); the rest are the account's copy. */
export type SettingsSyncPhase = "signedOut" | "syncing" | "saved" | "error";

export type SettingsSyncState = {
  phase: SettingsSyncPhase;
  /** When the account last took this device's settings or confirmed it holds them (epoch ms), else null. */
  savedAt: number | null;
};

let state: SettingsSyncState = { phase: "signedOut", savedAt: null };
const stateListeners = new Set<() => void>();
let activeSync: SettingsAccountSync | null = null;

function setState(next: Partial<SettingsSyncState>): void {
  const merged = { ...state, ...next };
  if (merged.phase === state.phase && merged.savedAt === state.savedAt) return;
  state = merged;
  for (const listener of [...stateListeners]) listener();
}

export function readSettingsSyncState(): SettingsSyncState {
  return state;
}

export function subscribeSettingsSyncState(listener: () => void): () => void {
  stateListeners.add(listener);
  return () => {
    stateListeners.delete(listener);
  };
}

export function useSettingsSyncState(): SettingsSyncState {
  return useSyncExternalStore(subscribeSettingsSyncState, readSettingsSyncState, readSettingsSyncState);
}

/** The player's own "Try again": the running sync loads or sends again. Does nothing signed out. */
export function retrySettingsSync(): void {
  activeSync?.retry();
}

// ---------------------------------------------------------------------------------------------
// The device's clock per group
// ---------------------------------------------------------------------------------------------

function readClock(): Record<string, number> {
  try {
    const raw: unknown = JSON.parse(window.localStorage.getItem(SETTINGS_SYNC_STORAGE_KEY) ?? "null");
    if (typeof raw !== "object" || raw === null || Array.isArray(raw)) return {};
    const clock: Record<string, number> = {};
    for (const [id, at] of Object.entries(raw)) {
      if (typeof at === "number" && Number.isSafeInteger(at) && at > 0) clock[id] = at;
    }
    return clock;
  } catch {
    return {};
  }
}

function writeClock(clock: Record<string, number>): void {
  try {
    window.localStorage.setItem(SETTINGS_SYNC_STORAGE_KEY, JSON.stringify(clock));
  } catch {
    // Blocked or full storage: the time lives in memory for this page, and the next one starts from
    // the account's copy.
  }
}

function sameValues(a: Record<string, unknown>, b: Record<string, unknown>): boolean {
  const keys = Object.keys(a);
  return keys.length === Object.keys(b).length && keys.every((key) => a[key] === b[key]);
}

/** The server's answer, read as untrusted input: an object of groups, or it is not an answer at all. */
function accountCopy(raw: unknown): PlayerSettingsAccountCopy {
  const groups = (raw as { groups?: unknown } | null)?.groups;
  if (typeof groups !== "object" || groups === null || Array.isArray(groups)) {
    throw new Error("not a settings answer");
  }
  const copy: PlayerSettingsAccountCopy = { groups: {} };
  for (const [id, group] of Object.entries(groups)) {
    const held = group as { at?: unknown; values?: unknown } | null;
    if (typeof held?.at !== "number" || typeof held.values !== "object" || held.values === null) continue;
    copy.groups[id] = { at: held.at, values: held.values as PlayerSettingsGroup["values"] };
  }
  return copy;
}

// ---------------------------------------------------------------------------------------------
// The sync
// ---------------------------------------------------------------------------------------------

export type SettingsAccountSync = {
  /** Stop listening; an answer still in flight is ignored. */
  stop(): void;
  /** Resolves once no request is in flight (tests; nothing in the app waits on it). */
  settled(): Promise<void>;
  /** Loads again if the first load never answered, else sends what is owed. */
  retry(): void;
};

export type SettingsAccountSyncOptions = {
  /** Read at each request, so a renewed session's token is the one used. */
  token: () => string;
  api: SettingsAccountApi;
  groups?: readonly SettingsGroup[];
  now?: () => number;
  debounceMs?: number;
};

/** Starts syncing this device's settings with the account `token()` names. */
export function startSettingsAccountSync(options: SettingsAccountSyncOptions): SettingsAccountSync {
  const groups = options.groups ?? SETTINGS_GROUPS;
  const now = options.now ?? Date.now;
  const debounceMs = options.debounceMs ?? SETTINGS_SYNC_DEBOUNCE_MS;
  const clock = readClock();

  let stopped = false;
  let loaded = false;
  /** True while a group's store is being written with the account's values, so that is not a change here. */
  let applying = false;
  /** The groups this device holds a later change of than the account (as far as this page knows). */
  const owed = new Set<string>();
  let timer: ReturnType<typeof setTimeout> | null = null;
  let running: Promise<void> | null = null;
  let again = false;

  /** Groups whose time this sync just lowered to the server's, which another tab's higher entry must not undo. */
  const lowered = new Set<string>();

  /**
   * Writes the clock. Another tab of this device may have stamped a group since this page read it,
   * so a higher time already stored for a group wins, except where this page lowered it on purpose.
   */
  const persist = (): void => {
    for (const [id, at] of Object.entries(readClock())) {
      if (!lowered.has(id) && at > (clock[id] ?? 0)) clock[id] = at;
    }
    lowered.clear();
    writeClock(clock);
  };

  /** Takes the account's group into the device's store, at the account's time. */
  const adopt = (group: SettingsGroup, held: PlayerSettingsGroup): void => {
    applying = true;
    try {
      group.apply(held.values);
    } finally {
      applying = false;
    }
    clock[group.id] = held.at;
    owed.delete(group.id);
    persist();
  };

  /** Group by group, the newer side wins: adopt the account's, or owe the device's. */
  const reconcile = (copy: PlayerSettingsAccountCopy): void => {
    for (const group of groups) {
      const held = copy.groups[group.id];
      const at = clock[group.id] ?? 0;
      if (held !== undefined && (held.at > at || at === 0)) adopt(group, held);
      else if (at > 0 && (held === undefined || at > held.at)) owed.add(group.id);
    }
  };

  /** One PUT of the groups owed, and the answer reconciled. Throws if the request fails. */
  const send = async (): Promise<void> => {
    if (owed.size === 0) {
      setState({ phase: "saved", savedAt: state.savedAt ?? now() });
      return;
    }
    const sent: Record<string, PlayerSettingsGroup> = {};
    for (const group of groups) {
      if (owed.has(group.id)) sent[group.id] = { at: clock[group.id] ?? 0, values: group.read() };
    }
    owed.clear();
    setState({ phase: "syncing" });
    let answer: PlayerSettingsAccountCopy;
    try {
      answer = accountCopy(await options.api.save(options.token(), sent));
    } catch (cause) {
      for (const id of Object.keys(sent)) owed.add(id);
      throw cause;
    }
    if (stopped) return;
    // The server takes a time after its own clock as now. A group it kept, with our values, at an
    // earlier time is ours as the server timed it: take that time, or a device whose clock runs
    // ahead would send the same group again for as long as its clock stays ahead.
    for (const [id, mine] of Object.entries(sent)) {
      const held = answer.groups[id];
      if (held !== undefined && held.at < mine.at && sameValues(held.values, mine.values)) {
        clock[id] = held.at;
        lowered.add(id);
      }
    }
    persist();
    reconcile(answer);
    setState({ phase: "saved", savedAt: now() });
  };

  const load = async (): Promise<void> => {
    setState({ phase: "syncing" });
    const copy = accountCopy(await options.api.load(options.token()));
    if (stopped) return;
    reconcile(copy);
    loaded = true;
    await send();
  };

  /** Runs `first`, then sends again for as long as a change arrived meanwhile. Never rejects. */
  const run = (first: () => Promise<void>): void => {
    running = (async () => {
      try {
        await first();
      } catch {
        // R634: offline, a server error or an answer that is not one. The device keeps what it has.
        if (!stopped) setState({ phase: "error" });
        return;
      }
      while (again && !stopped) {
        again = false;
        try {
          await send();
        } catch {
          if (!stopped) setState({ phase: "error" });
          return;
        }
      }
    })().finally(() => {
      running = null;
    });
  };

  const kick = (): void => {
    if (stopped) return;
    if (running !== null) {
      again = true;
      return;
    }
    run(loaded ? send : load);
  };

  const clearTimer = (): void => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
  };

  const onChange = (id: string): void => {
    if (applying || stopped) return;
    clock[id] = Math.max(now(), (clock[id] ?? 0) + 1);
    persist();
    owed.add(id);
    // Before the first load answers, `reconcile` finds the change newer than the account's anyway.
    if (!loaded) return;
    clearTimer();
    timer = setTimeout(() => {
      timer = null;
      kick();
    }, debounceMs);
  };

  const unsubscribes = groups.map((group) =>
    group.subscribe(() => {
      onChange(group.id);
    }),
  );

  // Leaving the page, or the tab going to the background: send what is owed without waiting.
  const flushNow = (): void => {
    if (owed.size === 0 || !loaded) return;
    clearTimer();
    kick();
  };
  const retry = (): void => {
    if (stopped) return;
    clearTimer();
    if (running !== null) {
      again = true;
      return;
    }
    run(loaded ? send : load);
  };
  const onVisibility = (): void => {
    if (document.visibilityState === "hidden") flushNow();
    else if (state.phase === "error") retry();
  };
  const onOnline = (): void => {
    if (state.phase === "error") retry();
  };
  document.addEventListener("visibilitychange", onVisibility);
  window.addEventListener("pagehide", flushNow);
  window.addEventListener("online", onOnline);

  run(load);

  return {
    stop: () => {
      stopped = true;
      clearTimer();
      for (const unsubscribe of unsubscribes) unsubscribe();
      document.removeEventListener("visibilitychange", onVisibility);
      window.removeEventListener("pagehide", flushNow);
      window.removeEventListener("online", onOnline);
    },
    settled: async () => {
      while (running !== null) await running;
    },
    retry,
  };
}

// ---------------------------------------------------------------------------------------------
// One sync for the page, however many screens ask
// ---------------------------------------------------------------------------------------------

type Shared = {
  profileId: string;
  sync: SettingsAccountSync;
  users: number;
  /** The newest screen's token reader replaces the one the sync was started with. */
  setToken: (token: () => string) => void;
  stopTimer: ReturnType<typeof setTimeout> | null;
};

let shared: Shared | null = null;

function attach(profileId: string, token: () => string, api: () => SettingsAccountApi): () => void {
  if (shared !== null && shared.profileId !== profileId) {
    shared.sync.stop();
    shared = null;
    activeSync = null;
    setState({ phase: "signedOut", savedAt: null });
  }
  if (shared === null) {
    let readToken = token;
    const sync = startSettingsAccountSync({
      token: () => readToken(),
      api: {
        load: (bearer) => api().load(bearer),
        save: (bearer, groups) => api().save(bearer, groups),
      },
    });
    shared = {
      profileId,
      users: 0,
      sync,
      setToken: (next) => {
        readToken = next;
      },
      stopTimer: null,
    };
    activeSync = sync;
  }
  const mine = shared;
  if (mine.stopTimer !== null) clearTimeout(mine.stopTimer);
  mine.stopTimer = null;
  mine.users += 1;
  mine.setToken(token);
  return () => {
    mine.users -= 1;
    if (mine.users > 0 || shared !== mine) return;
    mine.stopTimer = setTimeout(() => {
      if (shared !== mine || mine.users > 0) return;
      mine.sync.stop();
      shared = null;
      activeSync = null;
      setState({ phase: "signedOut", savedAt: null });
    }, SETTINGS_SYNC_GRACE_MS);
  };
}

/**
 * R634 for a screen: syncs while `account` is an active signed-in account, and does nothing (sends
 * nothing) otherwise. One sync serves every screen in turn; it restarts when the account changes to
 * another profile, and a renewed token for the same profile is simply used for the next request.
 */
export function useSettingsAccountSync(account: Account, api: SettingsAccountApi = settingsAccountApi): void {
  const active = account.kind === "ready" && account.me.profile.status === "active";
  const profileId = active ? account.me.profile.id : null;
  const token = useRef<string>("");
  token.current = active ? account.token : "";
  const requests = useRef(api);
  requests.current = api;

  useEffect(() => {
    if (profileId === null) return;
    return attach(
      profileId,
      () => token.current,
      () => requests.current,
    );
  }, [profileId]);
}

/** Test seam: stops any running sync and forgets the status. */
export function __resetSettingsSyncForTests(): void {
  if (shared?.stopTimer !== null && shared?.stopTimer !== undefined) clearTimeout(shared.stopTimer);
  shared?.sync.stop();
  shared = null;
  activeSync = null;
  state = { phase: "signedOut", savedAt: null };
  stateListeners.clear();
}
