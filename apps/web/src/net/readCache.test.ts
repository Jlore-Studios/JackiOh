// #2125 (R2125000): the menus' kept answers (`readCache.ts`).

import { afterEach, describe, expect, it, vi } from "vitest";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import { packsInfo, openedPack } from "../packs/fixtures.ts";
import { stubServer, type Reply } from "../test/stubServer.ts";
import {
  buyPack,
  deleteDeck,
  getCardStats,
  getCollection,
  getDecks,
  getMatchRanks,
  getPacks,
  getReplays,
  getReplaySteps,
  getSeries,
  getSeriesForMatch,
  getWallet,
  keptPaths,
  openPack,
  putDeck,
  rematchStatus,
  restoreDeckVersion,
  type DecksResponse,
  type PacksResponse,
  type SavedDeck,
  type SavedTrio,
  type WalletResponse,
} from "./api.ts";
import { getDuosAgain, getDuosLive, getDuosReplay, getDuosRoom } from "./duos.ts";
import {
  READ_CACHE_MAX_ENTRIES,
  __readCacheSizeForTests,
  cachedRead,
  clearReadCache,
  rememberRead,
  updateCachedRead,
} from "./readCache.ts";
import { clearSession, writeSession } from "./session.ts";
import {
  FRIENDS_PATH,
  acceptFriendRequest,
  cachedFriends,
  getFriends,
  removeFriend,
  type Friend,
  type FriendRequest,
  type FriendsList,
  type PlayerRef,
} from "./social.ts";

afterEach(() => {
  clearSession();
  vi.unstubAllGlobals();
});

/** A token whose `sub` is `sub`; `nonce` tells two tokens of one account apart, as a renewal does. */
function tokenOf(sub: string, nonce: string): string {
  return `h.${btoa(JSON.stringify({ sub, nonce }))}.s`;
}

const A1 = tokenOf("account-a", "1");
const A2 = tokenOf("account-a", "2");
const B1 = tokenOf("account-b", "1");

describe("R2125000 the read cache", () => {
  it("R2125000 keeps an answer by its path, the query included, and its account across a renewal", () => {
    rememberRead(A1, "/api/wallet", { jlore: 5 });
    expect(cachedRead<WalletResponse>(A2, "/api/wallet"), "a renewed token is the same account").toEqual({ jlore: 5 });
    expect(cachedRead(A1, "/api/wallet?x=1"), "another query is another answer").toBeUndefined();
    expect(cachedRead(A1, "/api/packs"), "another path is another answer").toBeUndefined();

    rememberRead(null, "/api/stats/cards?patch=1", { cards: [] });
    expect(cachedRead(null, "/api/stats/cards?patch=1")).toEqual({ cards: [] });
    expect(cachedRead(null, "/api/stats/cards")).toBeUndefined();
    expect(cachedRead(A1, "/api/wallet"), "a public answer leaves the account's").toEqual({ jlore: 5 });
  });

  it("R2125000 an account switch empties the cache and never shows another account's answer", () => {
    rememberRead(A1, "/api/wallet", { jlore: 5 });
    expect(cachedRead(B1, "/api/wallet")).toBeUndefined();
    expect(__readCacheSizeForTests(), "a read of another account's takes nothing away").toBe(1);

    rememberRead(B1, "/api/wallet", { jlore: 9 });
    expect(__readCacheSizeForTests(), "the first account's answers went with the switch").toBe(1);
    expect(cachedRead(A1, "/api/wallet")).toBeUndefined();
    expect(cachedRead<WalletResponse>(B1, "/api/wallet")).toEqual({ jlore: 9 });
  });

  it("R2125000 sign-out empties the cache", () => {
    rememberRead(A1, "/api/wallet", { jlore: 5 });
    rememberRead(null, "/api/stats/cards", { cards: [] });
    expect(__readCacheSizeForTests()).toBe(2);

    clearSession();

    expect(__readCacheSizeForTests()).toBe(0);
    expect(cachedRead(A1, "/api/wallet")).toBeUndefined();
    expect(cachedRead(null, "/api/stats/cards")).toBeUndefined();
  });

  it("R2125000 keeps at most READ_CACHE_MAX_ENTRIES answers, the oldest kept going first", () => {
    for (let index = 0; index < READ_CACHE_MAX_ENTRIES; index += 1) rememberRead(null, `/api/stats/cards?cost=${String(index)}`, index);
    expect(__readCacheSizeForTests()).toBe(READ_CACHE_MAX_ENTRIES);

    // Keeping the first again makes the second the oldest.
    rememberRead(null, "/api/stats/cards?cost=0", 0);
    rememberRead(null, "/api/stats/cards?cost=extra", -1);

    expect(__readCacheSizeForTests()).toBe(READ_CACHE_MAX_ENTRIES);
    expect(cachedRead(null, "/api/stats/cards?cost=1")).toBeUndefined();
    expect(cachedRead(null, "/api/stats/cards?cost=0")).toBe(0);
    expect(cachedRead(null, "/api/stats/cards?cost=extra")).toBe(-1);
  });

  it("R2125000 a write finds nothing to fold into when nothing is kept", () => {
    const update = vi.fn((before: WalletResponse) => before);
    updateCachedRead(A1, "/api/wallet", update);
    expect(update).not.toHaveBeenCalled();
    expect(__readCacheSizeForTests()).toBe(0);
  });
});

/** A server answering the shop's reads and the buy and the opening, from the fixtures. */
function economyServer(): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET" && call.path === "/api/wallet") return { status: 200, body: { jlore: 250 } };
    if (call.method === "GET" && call.path === "/api/packs") return { status: 200, body: packsInfo() };
    if (call.method === "POST" && call.path === "/api/packs/buy") {
      return { status: 200, body: { pack: "Core", jlore: 150, unopened: 3 } };
    }
    if (call.method === "POST" && call.path === "/api/packs/open") return { status: 200, body: openedPack() };
    return undefined;
  });
}

const countOf = (packs: PacksResponse | undefined, pack: string): number | undefined =>
  packs?.unopened.find((entry) => entry.pack === pack)?.count;

describe("R2125000 a write's answer updates what is kept", () => {
  it("R2125000 a buy's answer updates the kept wallet and counts", async () => {
    economyServer();
    await getWallet(A1);
    await getPacks(A1);
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet)).toEqual({ jlore: 250 });

    await buyPack(A1, "Core");

    expect(cachedRead<WalletResponse>(A2, keptPaths.wallet)).toEqual({ jlore: 150 });
    const packs = cachedRead<PacksResponse>(A2, keptPaths.packs);
    expect(countOf(packs, "Core")).toBe(3);
    expect(countOf(packs, GLOBAL_PACK_NAME), "the other kind is untouched").toBe(1);
  });

  it("R2125000 a pack opening and a deck saved or deleted fold into their kept answers", async () => {
    const deck = (id: string, name: string): SavedDeck => ({
      id,
      name,
      cards: [],
      catalogVersion: "v",
      portrait: null,
      createdAt: 1,
      updatedAt: 1,
    });
    const trio: SavedTrio = { id: "t-1", name: "Trio", deckIds: ["d-1", "d-2", null], createdAt: 1, updatedAt: 1 };
    const decks: DecksResponse = {
      catalogVersion: "v",
      decks: [deck("d-1", "One"), deck("d-2", "Two")],
      trios: [trio],
      limits: { decks: 10, trios: 5, nameLength: 20 },
    };
    stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/packs") return { status: 200, body: packsInfo() };
      if (call.method === "GET" && call.path === "/api/decks") return { status: 200, body: decks };
      if (call.method === "POST" && call.path === "/api/packs/open") {
        return { status: 200, body: { ...openedPack(), unopened: 0, pity: { epic: 1, legendary: 2, mythic: 3 } } };
      }
      if (call.method === "PUT" && call.path === "/api/decks/d-3") return { status: 200, body: { deck: deck("d-3", "Three") } };
      if (call.method === "PUT" && call.path === "/api/decks/d-1") return { status: 200, body: { deck: deck("d-1", "Renamed") } };
      if (call.method === "DELETE" && call.path === "/api/decks/d-2") return { status: 200, body: { deleted: true } };
      return undefined;
    });
    await getPacks(A1);
    await getDecks(A1);

    await openPack(A1, "Core");
    const packs = cachedRead<PacksResponse>(A1, keptPaths.packs);
    expect(countOf(packs, "Core"), "a kind with none left is dropped").toBeUndefined();
    expect(countOf(packs, GLOBAL_PACK_NAME)).toBe(1);
    expect(packs?.pity).toEqual({ epic: 1, legendary: 2, mythic: 3 });

    await putDeck(A1, "d-3", { name: "Three", cards: [], catalogVersion: "v" });
    await putDeck(A1, "d-1", { name: "Renamed", cards: [], catalogVersion: "v" });
    expect(cachedRead<DecksResponse>(A1, keptPaths.decks)?.decks.map((entry) => `${entry.id}:${entry.name}`)).toEqual([
      "d-1:Renamed",
      "d-2:Two",
      "d-3:Three",
    ]);

    await deleteDeck(A1, "d-2");
    const after = cachedRead<DecksResponse>(A1, keptPaths.decks);
    expect(after?.decks.map((entry) => entry.id)).toEqual(["d-1", "d-3"]);
    expect(after?.trios[0]?.deckIds, "the trio's slot that named the deck empties (R252)").toEqual(["d-1", null, null]);
  });

  it("R2100000 a restored version folds into the kept decks, as a save does", async () => {
    const deck = (cards: string[]): SavedDeck => ({
      id: "d-1",
      name: "One",
      cards,
      catalogVersion: "v",
      portrait: null,
      createdAt: 1,
      updatedAt: 1,
    });
    const decks: DecksResponse = {
      catalogVersion: "v",
      decks: [deck(["C001"])],
      trios: [],
      limits: { decks: 10, trios: 5, nameLength: 20 },
    };
    stubServer((call) => {
      if (call.method === "GET" && call.path === "/api/decks") return { status: 200, body: decks };
      if (call.method === "POST" && call.path === "/api/decks/d-1/versions/v-1/restore") {
        return { status: 200, body: { deck: deck(["C002"]), versions: [] } };
      }
      return undefined;
    });
    await getDecks(A1);

    await restoreDeckVersion(A1, "d-1", "v-1", "v");

    expect(cachedRead<DecksResponse>(A1, keptPaths.decks)?.decks.map((entry) => entry.cards)).toEqual([["C002"]]);
  });

  it("R2125000 a friend accepted or removed folds into the kept friends list", async () => {
    writeSession({ accessToken: A1 });
    const ann: PlayerRef = { profileId: "f-ann", username: "Ann#2" };
    const bob: PlayerRef = { profileId: "f-bob", username: "Bob#3" };
    const me: PlayerRef = { profileId: "p-me", username: "Me#1" };
    const friend = (who: PlayerRef): Friend => ({
      ...who,
      status: "online",
      matchId: null,
      watchable: false,
      since: "2026-10-01T00:00:00.000Z",
    });
    const request: FriendRequest = { requestId: "rq-1", from: ann, to: me, createdAt: "2026-10-14T09:00:00.000Z" };
    const list: FriendsList = { friends: [friend(bob)], incoming: [request], outgoing: [], blocked: [] };
    stubServer((call) => {
      if (call.method === "GET" && call.path === FRIENDS_PATH) return { status: 200, body: list };
      if (call.method === "POST" && call.path === "/api/friends/requests/rq-1/accept") {
        return { status: 200, body: { friend: friend(ann) } };
      }
      if (call.method === "DELETE" && call.path === "/api/friends/f-bob") return { status: 204 };
      return undefined;
    });
    expect(cachedFriends(), "nothing kept before the first read").toBeUndefined();
    await getFriends();
    expect(cachedFriends()?.incoming).toHaveLength(1);

    await acceptFriendRequest("rq-1");
    expect(cachedFriends()?.friends.map((entry) => entry.profileId)).toEqual(["f-bob", "f-ann"]);
    expect(cachedFriends()?.incoming).toEqual([]);

    await removeFriend("f-bob");
    expect(cachedFriends()?.friends.map((entry) => entry.profileId)).toEqual(["f-ann"]);
  });
});

describe("R2125000 what is never kept", () => {
  it("R2125000 match, series, duos, watch and replay reads never touch the cache", async () => {
    writeSession({ accessToken: A1 });
    const empty: Reply = { status: 200, body: {} };
    stubServer(() => empty);

    await getReplays(A1);
    await getReplaySteps(A1, "m-1", 0, 5);
    await getMatchRanks(A1, "m-1");
    await getSeriesForMatch(A1, "m-1");
    await getSeries(A1, "s-1");
    await rematchStatus(A1, "m-1");
    await getDuosRoom("ROOM01");
    await getDuosLive();
    await getDuosReplay("m-1");
    await getDuosAgain("m-1");
    await getFriends({ keep: false });

    expect(__readCacheSizeForTests()).toBe(0);
  });

  it("R2125000 a kept getter keeps its answer, and a failed read keeps what was there", async () => {
    let answer: Reply = { status: 200, body: { catalogVersion: "v", entries: [{ cardId: "core-001", quantity: 1 }] } };
    stubServer(() => answer);
    await getCollection(A1);
    expect(__readCacheSizeForTests()).toBe(1);

    answer = { status: 503, body: { error: { code: "unavailable", message: "down" } } };
    await expect(getCollection(A1)).rejects.toThrow("down");
    expect(cachedRead(A1, keptPaths.collection)).toEqual({ catalogVersion: "v", entries: [{ cardId: "core-001", quantity: 1 }] });
  });

  it("R2125000 the statistics table is kept for the public, each filter by its own path", async () => {
    stubServer((call) => ({ status: 200, body: { patch: call.query.get("patch") ?? "latest" } }));
    await getCardStats({});
    await getCardStats({ patch: "1.2" });
    expect(cachedRead(null, "/api/stats/cards")).toEqual({ patch: "latest" });
    expect(cachedRead(null, "/api/stats/cards?patch=1.2")).toEqual({ patch: "1.2" });
    clearReadCache();
    expect(__readCacheSizeForTests()).toBe(0);
  });
});
