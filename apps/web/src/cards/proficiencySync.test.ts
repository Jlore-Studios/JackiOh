// R3013001: the account's card proficiency lives on the account alone, and this page holds a copy
// (`cards/proficiencySync.ts`, `cards/stamps.ts`). The fake account keeps it as the server does: a stamp the
// card's level has not reached is refused, and every answer is all of the account's cards.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ProficiencyEntry, ProficiencyStampPut } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import {
  resetProficiencySyncForTests,
  startProficiencyAccountSync,
  useProficiencyAccountSync,
  type ProficiencyAccountApi,
  type ProficiencyAccountSync,
} from "./proficiencySync.ts";
import { readOwnProficiency, resetProficiencyForTests, writeOwnStamp } from "./stamps.ts";

type FakeAccount = ProficiencyAccountApi & {
  stored: Map<string, ProficiencyEntry>;
  calls: string[];
  puts: ProficiencyStampPut[];
  /** Makes the named request fail at the network until set back to null. */
  failing: "load" | "put" | null;
};

/** The server's 400 for a stamp the card's level has not unlocked. */
class Refused extends Error {}

function fakeAccount(initial: ProficiencyEntry[] = []): FakeAccount {
  const answer = () => ({ cards: [...account.stored.values()].sort((a, b) => a.cardId.localeCompare(b.cardId)).map((entry) => ({ ...entry })) });
  const account: FakeAccount = {
    stored: new Map(initial.map((entry) => [entry.cardId, { ...entry }])),
    calls: [],
    puts: [],
    failing: null,
    load: vi.fn(async (token: string) => {
      account.calls.push(`load ${token}`);
      if (account.failing === "load") throw new TypeError("Failed to fetch");
      return answer();
    }),
    put: vi.fn(async (token: string, change: ProficiencyStampPut) => {
      account.calls.push(`put ${token}`);
      account.puts.push({ ...change });
      if (account.failing === "put") throw new TypeError("Failed to fetch");
      const held = account.stored.get(change.cardId);
      if (change.stamp !== null && (held === undefined || change.stamp > held.level)) throw new Refused("\"stamp\" needs a higher level");
      if (held !== undefined) account.stored.set(change.cardId, { ...held, stamp: change.stamp });
      return answer();
    }),
  };
  return account;
}

const PLAYED: ProficiencyEntry[] = [
  { cardId: "core-001", plays: 20, level: 2, stamp: null },
  { cardId: "core-004", plays: 3, level: 1, stamp: 1 },
];

let syncs: ProficiencyAccountSync[] = [];

function start(account: FakeAccount, token = "token-1"): ProficiencyAccountSync {
  const sync = startProficiencyAccountSync({ token: () => token, api: account });
  syncs.push(sync);
  return sync;
}

beforeEach(() => {
  resetProficiencyForTests();
  resetProficiencySyncForTests();
});

afterEach(() => {
  for (const sync of syncs) sync.stop();
  syncs = [];
  resetProficiencySyncForTests();
  resetProficiencyForTests();
  vi.useRealTimers();
});

describe("R3013001 the account's proficiency on this page", () => {
  it("R3013001 the account's proficiency loads, a pick is sent and the answer replaces the store", async () => {
    const account = fakeAccount(PLAYED);
    const sync = start(account);
    await sync.settled();
    expect(account.calls).toEqual(["load token-1"]);
    expect(readOwnProficiency("core-001")).toEqual({ plays: 20, level: 2, stamp: null });
    expect(readOwnProficiency("core-004")).toEqual({ plays: 3, level: 1, stamp: 1 });
    expect(readOwnProficiency("core-002")).toBeNull();

    // A pick shows at once and is sent; the account's answer stands.
    expect(writeOwnStamp("core-001", 2)).toBe(true);
    expect(readOwnProficiency("core-001")?.stamp).toBe(2);
    await sync.settled();
    expect(account.puts).toEqual([{ cardId: "core-001", stamp: 2 }]);
    expect(account.stored.get("core-001")?.stamp).toBe(2);
    expect(readOwnProficiency("core-001")?.stamp).toBe(2);
    // Another device's change comes in with the answer of the next pick.
    account.stored.set("core-009", { cardId: "core-009", plays: 40, level: 3, stamp: 3 });
    expect(writeOwnStamp("core-004", null)).toBe(true);
    await sync.settled();
    expect(account.puts.at(-1)).toEqual({ cardId: "core-004", stamp: null });
    expect(readOwnProficiency("core-004")?.stamp).toBeNull();
    expect(readOwnProficiency("core-009")).toEqual({ plays: 40, level: 3, stamp: 3 });

    // A stamp the card's level has not unlocked, and a card never played, write nothing and send nothing.
    const sent = account.puts.length;
    expect(writeOwnStamp("core-001", 3)).toBe(false);
    expect(writeOwnStamp("core-002", 1)).toBe(false);
    expect(writeOwnStamp("core-001", 0)).toBe(false);
    await sync.settled();
    expect(account.puts).toHaveLength(sent);
    expect(readOwnProficiency("core-001")?.stamp).toBe(2);

    // Stopping, as a sign-out does, clears the page's copy and sends nothing more.
    sync.stop();
    expect(readOwnProficiency("core-001")).toBeNull();
  });

  it("R3013001 a refused pick reloads the account's copy", async () => {
    const account = fakeAccount(PLAYED);
    const sync = start(account);
    await sync.settled();
    // The page's copy says level 2, but the account's is what the server checks: here it fell out of step (the
    // page is stale), so the server refuses the pick and the page loads the account's copy again.
    account.stored.set("core-001", { cardId: "core-001", plays: 2, level: 0, stamp: null });
    expect(writeOwnStamp("core-001", 2)).toBe(true);
    await sync.settled();
    expect(account.calls).toEqual(["load token-1", "put token-1", "load token-1"]);
    expect(readOwnProficiency("core-001")).toEqual({ plays: 2, level: 0, stamp: null });

    // A pick that never reached the server reloads too, so the page shows what the account still wears.
    account.failing = "put";
    expect(writeOwnStamp("core-004", null)).toBe(true);
    expect(readOwnProficiency("core-004")?.stamp).toBeNull();
    await sync.settled();
    expect(account.calls.slice(-2)).toEqual(["put token-1", "load token-1"]);
    expect(readOwnProficiency("core-004")?.stamp).toBe(1);
  });

  it("R3013001 every gated screen shares one sync, which only an active account starts", async () => {
    vi.useFakeTimers();
    const account = fakeAccount(PLAYED);
    const ready = (status: "active" | "pending"): Account => ({
      kind: "ready",
      token: "token-1",
      me: { profile: { id: "profile-1", status }, needsInviteCode: status === "pending", emailVerified: true, currentMatchId: null, email: "profile-1@example.com" },
    });
    const pending = renderHook(() => useProficiencyAccountSync(ready("pending"), account));
    expect(account.calls).toEqual([]);
    pending.unmount();
    const screenA = renderHook(() => useProficiencyAccountSync(ready("active"), account));
    const screenB = renderHook(() => useProficiencyAccountSync(ready("active"), account));
    await act(async () => {
      await vi.runOnlyPendingTimersAsync();
    });
    expect(account.calls).toEqual(["load token-1"]);
    expect(readOwnProficiency("core-004")?.stamp).toBe(1);
    screenA.unmount();
    screenB.unmount();
    await act(async () => {
      await vi.runAllTimersAsync();
    });
    // The last screen gone, the sync stops after its grace and the page's copy goes with it.
    expect(readOwnProficiency("core-004")).toBeNull();
  });
});
