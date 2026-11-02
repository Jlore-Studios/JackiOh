// R1834000: each card's picks live on the account, and the device's store is their cache
// (`cards/cardPicksSync.ts`). The fake account keeps picks as the server does: the upload writes only
// the cards it holds no row for, a change patches one card, and every answer is all of them.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { CardPickPut, CardPicksEntry, CardPicksResponse } from "../net/api.ts";
import type { Account } from "../net/gate.ts";
import {
  CARD_PICKS_UPLOADED_KEY,
  resetCardPicksSyncForTests,
  startCardPicksAccountSync,
  useCardPicksAccountSync,
  type CardPicksAccountApi,
  type CardPicksAccountSync,
} from "./cardPicksSync.ts";
import {
  cosmeticsOf,
  readAllCardPicks,
  resetCosmeticsForTests,
  writeCardFullArt,
  writeCardPick,
  type CosmeticId,
  type CosmeticKind,
} from "./cosmetics.ts";

/** The registry's `index`th entry of `kind` that is not its default. */
function item(kind: CosmeticKind, index: number): CosmeticId {
  const found = cosmeticsOf(kind).filter((cosmetic) => !cosmetic.default)[index];
  if (found === undefined) throw new Error(`no ${kind} item ${String(index)}`);
  return found.id;
}

const [FOIL, OTHER_FOIL] = [item("foil", 0), item("foil", 1)];
const [BORDER, OTHER_BORDER] = [item("border", 0), item("border", 1)];

type FakeAccount = CardPicksAccountApi & {
  stored: Map<string, CardPicksEntry>;
  calls: string[];
  uploads: CardPicksEntry[][];
  puts: CardPickPut[];
  /** Makes the named request fail at the network until set back to null. */
  failing: "load" | "upload" | "put" | null;
};

function fakeAccount(initial: CardPicksEntry[] = []): FakeAccount {
  const answer = () => ({ cards: [...account.stored.values()].sort((a, b) => a.cardId.localeCompare(b.cardId)).map((entry) => ({ ...entry })) });
  const account: FakeAccount = {
    stored: new Map(initial.map((entry) => [entry.cardId, { ...entry }])),
    calls: [],
    uploads: [],
    puts: [],
    failing: null,
    load: vi.fn(async (token: string) => {
      account.calls.push(`load ${token}`);
      if (account.failing === "load") throw new TypeError("Failed to fetch");
      return answer();
    }),
    upload: vi.fn(async (token: string, cards: CardPicksEntry[]) => {
      account.calls.push(`upload ${token}`);
      account.uploads.push(structuredClone(cards));
      if (account.failing === "upload") throw new TypeError("Failed to fetch");
      for (const entry of cards) if (!account.stored.has(entry.cardId)) account.stored.set(entry.cardId, { ...entry });
      return answer();
    }),
    put: vi.fn(async (token: string, change: CardPickPut) => {
      account.calls.push(`put ${token}`);
      account.puts.push({ ...change });
      if (account.failing === "put") throw new TypeError("Failed to fetch");
      const next: Record<string, unknown> = { ...account.stored.get(change.cardId), cardId: change.cardId };
      for (const [field, value] of Object.entries(change)) {
        if (field === "cardId") continue;
        if (value === null) delete next[field];
        else next[field] = value;
      }
      account.stored.set(change.cardId, next as CardPicksEntry);
      return answer();
    }),
  };
  return account;
}

const running: CardPicksAccountSync[] = [];

function start(account: FakeAccount, profileId = "u1"): CardPicksAccountSync {
  const sync = startCardPicksAccountSync({ profileId, token: () => `tok-${profileId}`, api: account });
  running.push(sync);
  return sync;
}

function uploaded(): unknown {
  const raw = window.localStorage.getItem(CARD_PICKS_UPLOADED_KEY);
  return raw === null ? null : JSON.parse(raw);
}

beforeEach(() => {
  window.localStorage.clear();
  resetCosmeticsForTests();
  resetCardPicksSyncForTests();
});

afterEach(() => {
  for (const sync of running.splice(0)) sync.stop();
  resetCardPicksSyncForTests();
  resetCosmeticsForTests();
  window.localStorage.clear();
});

describe("R1834000 each card's picks live on the account", () => {
  it("R1834000 the first sign-in uploads once, then the server wins", async () => {
    expect(CARD_PICKS_UPLOADED_KEY).toBe("jackioh.cosmetics.cards.uploaded.v1");
    writeCardPick("core-004", "border", BORDER);
    writeCardFullArt("core-008", false);
    // The account already holds a pick for core-004, which the upload does not overwrite.
    const account = fakeAccount([{ cardId: "core-004", border: OTHER_BORDER }]);

    const first = start(account);
    await first.settled();
    expect(account.calls).toEqual(["upload tok-u1"]);
    expect(account.uploads).toEqual([
      [
        { cardId: "core-004", border: BORDER },
        { cardId: "core-008", fullArt: false },
      ],
    ]);
    expect(uploaded()).toEqual(["u1"]);
    expect(readAllCardPicks()).toEqual({ "core-004": { border: OTHER_BORDER }, "core-008": { fullArt: false } });
    first.stop();

    // The next sign-in reads the account's copy, which replaces the device's, and sends nothing back.
    account.stored.set("core-005", { cardId: "core-005", foil: FOIL });
    account.stored.delete("core-008");
    writeCardPick("core-006", "foil", OTHER_FOIL);
    const second = start(account);
    await second.settled();
    expect(account.calls).toEqual(["upload tok-u1", "load tok-u1"]);
    expect(readAllCardPicks()).toEqual({ "core-004": { border: OTHER_BORDER }, "core-005": { foil: FOIL } });
    expect(account.puts).toEqual([]);
    expect(uploaded()).toEqual(["u1"]);
  });

  it("R1834000 a device with no picks reads the account's and uploads nothing", async () => {
    const account = fakeAccount([{ cardId: "core-004", foil: FOIL, fullArt: true }]);
    await start(account).settled();
    expect(account.calls).toEqual(["load tok-u1"]);
    expect(readAllCardPicks()).toEqual({ "core-004": { foil: FOIL, fullArt: true } });
    expect(uploaded()).toEqual(["u1"]);
  });

  it("R1834000 a pick cleared on another device never comes back from this one's cache", async () => {
    const account = fakeAccount([{ cardId: "core-004", foil: FOIL }]);
    const first = start(account);
    await first.settled();
    expect(readAllCardPicks()).toEqual({ "core-004": { foil: FOIL } });
    first.stop();

    // Another device clears the pick; this one still holds it as the account's copy.
    account.stored.delete("core-004");
    await start(account).settled();
    expect(account.calls).toEqual(["load tok-u1", "load tok-u1"]);
    expect(account.uploads).toEqual([]);
    expect(account.stored.has("core-004")).toBe(false);
    expect(readAllCardPicks()).toEqual({});
  });

  it("R1834000 a second profile never sends the first's copy up as its own", async () => {
    writeCardPick("core-004", "border", BORDER);
    const one = fakeAccount();
    const first = start(one, "u1");
    await first.settled();
    expect(one.calls).toEqual(["upload tok-u1"]);
    first.stop();

    const two = fakeAccount([{ cardId: "core-005", foil: FOIL }]);
    await start(two, "u2").settled();
    expect(two.calls).toEqual(["load tok-u2"]);
    expect(two.stored.has("core-004")).toBe(false);
    expect(readAllCardPicks()).toEqual({ "core-005": { foil: FOIL } });
    expect(uploaded()).toEqual(["u1", "u2"]);
  });

  it("R1834000 a failed upload lists no profile and leaves the device's picks, and the next sign-in uploads them", async () => {
    writeCardPick("core-004", "border", BORDER);
    const account = fakeAccount();
    account.failing = "upload";

    const first = start(account);
    await expect(first.settled()).resolves.toBeUndefined();
    expect(uploaded()).toBeNull();
    expect(readAllCardPicks()).toEqual({ "core-004": { border: BORDER } });
    first.stop();

    account.failing = null;
    await start(account).settled();
    expect(account.calls).toEqual(["upload tok-u1", "upload tok-u1"]);
    expect(account.stored.get("core-004")).toEqual({ cardId: "core-004", border: BORDER });
    expect(uploaded()).toEqual(["u1"]);
  });

  it("R1834000 an equip is sent", async () => {
    const account = fakeAccount();
    const sync = start(account);
    await sync.settled();

    writeCardPick("core-004", "foil", FOIL);
    writeCardFullArt("core-008", true);
    writeCardPick("core-004", "foil", null);
    await sync.settled();
    expect(account.puts).toEqual([
      { cardId: "core-004", foil: FOIL },
      { cardId: "core-008", fullArt: true },
      { cardId: "core-004", foil: null },
    ]);
    expect([...account.stored.values()]).toEqual([{ cardId: "core-004" }, { cardId: "core-008", fullArt: true }]);

    // A failed send throws nothing and leaves the pick worn here; the next one is still sent.
    account.failing = "put";
    writeCardPick("core-005", "border", BORDER);
    await sync.settled();
    account.failing = null;
    writeCardPick("core-006", "border", OTHER_BORDER);
    await sync.settled();
    expect(account.puts.slice(-2)).toEqual([
      { cardId: "core-005", border: BORDER },
      { cardId: "core-006", border: OTHER_BORDER },
    ]);
    expect(readAllCardPicks()).toMatchObject({ "core-005": { border: BORDER }, "core-006": { border: OTHER_BORDER } });

    // Once stopped, a write is the device's alone.
    sync.stop();
    writeCardPick("core-007", "foil", FOIL);
    await sync.settled();
    expect(account.puts).toHaveLength(5);
  });

  it("R1834000 an equip made while the account's copy is on its way stands, and is sent after it", async () => {
    const account = fakeAccount([{ cardId: "core-005", foil: FOIL }]);
    let answer: () => void = () => undefined;
    const load = account.load;
    account.load = vi.fn(
      (token: string) =>
        new Promise<CardPicksResponse>((resolve) => {
          answer = () => {
            resolve(load(token));
          };
        }),
    );
    const sync = start(account);
    await Promise.resolve();

    writeCardPick("core-004", "border", BORDER);
    expect(account.puts).toEqual([]);
    answer();
    await sync.settled();
    expect(readAllCardPicks()).toEqual({ "core-004": { border: BORDER }, "core-005": { foil: FOIL } });
    expect(account.puts).toEqual([{ cardId: "core-004", border: BORDER }]);
  });

  it("R1834000 only an active account syncs, once for every screen that mounts the hook", async () => {
    const account = fakeAccount();
    const ready = (status: "active" | "pending"): Account => ({
      kind: "ready",
      token: "tok-u1",
      me: { profile: { id: "u1", status }, needsInviteCode: status === "pending", emailVerified: true, currentMatchId: null, email: "u1@example.com" },
    });

    const pending = renderHook(() => useCardPicksAccountSync(ready("pending"), account));
    await act(async () => {
      await Promise.resolve();
    });
    expect(account.calls).toEqual([]);
    pending.unmount();

    const screenA = renderHook(() => useCardPicksAccountSync(ready("active"), account));
    const screenB = renderHook(() => useCardPicksAccountSync(ready("active"), account));
    await act(async () => {
      await Promise.resolve();
    });
    expect(account.calls).toEqual(["load tok-u1"]);
    screenA.unmount();
    screenB.unmount();
  });
});
