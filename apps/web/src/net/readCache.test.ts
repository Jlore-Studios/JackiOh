// #2125 (R2125000): the menus' kept answers (`readCache.ts`).

import { afterEach, describe, expect, it, vi } from "vitest";

import { GLOBAL_PACK_NAME } from "@jackioh/server-config";

import { announceWallet } from "../economy/wallet.ts";
import { packsInfo, openedPack } from "../packs/fixtures.ts";
import {
  cosmeticPacksInfo,
  collectionOf,
  openedCosmeticPack,
  ownedCosmeticsOf,
  tokenBars,
} from "../shop/fixtures.ts";
import { jsonResponse, stubServer, type Reply } from "../test/stubServer.ts";
import {
  ApiUnreachableError,
  apiRequest,
  buyChase,
  buyPack,
  claimNews,
  deleteDeck,
  getCardStats,
  getCollection,
  getCosmeticPacks,
  getDecks,
  getMatchRanks,
  getNews,
  getOwnedCosmetics,
  getPacks,
  getReplays,
  getReplaySteps,
  getSeries,
  getSeriesForMatch,
  getTokens,
  getWallet,
  keptPaths,
  ladderPath,
  openCosmeticPack,
  openPack,
  putDeck,
  redeemCardToken,
  redeemCosmeticToken,
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
  READ_CACHE_FRESH_MS,
  READ_CACHE_MAX_ENTRIES,
  __readCacheSizeForTests,
  cachedRead,
  clearReadCache,
  expireRead,
  forgetRead,
  freshRead,
  keepRead,
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
    // R3206000: an answer inside the window is used with no read, so this one is aged to need a read.
    expireRead(A1, keptPaths.collection);
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

/** What a test that follows the clock sets `Date` to; only `Date` is faked, so no timer is held back. */
const T0 = 1_800_000_000_000;

/** A `fetch` the test answers by hand, as a real one stays out until the server replies or the signal aborts. */
function deferredFetch(): {
  calls: { path: string; signal: AbortSignal | null | undefined }[];
  reply: (index: number, body: unknown, status?: number) => void;
} {
  const calls: { path: string; signal: AbortSignal | null | undefined }[] = [];
  const replies: ((status: number, body: unknown) => void)[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      calls.push({ path: new URL(String(input), "http://localhost").pathname, signal: init?.signal });
      return new Promise<Response>((resolve, reject) => {
        replies.push((status, body) => {
          resolve(jsonResponse(status, body));
        });
        init?.signal?.addEventListener("abort", () => {
          reject(new DOMException("aborted", "AbortError"));
        });
      });
    }),
  );
  return {
    calls,
    reply: (index, body, status = 200) => {
      replies[index]?.(status, body);
    },
  };
}

/** The shop's and the menus' reads and the writes that change them, from the fixtures. */
function readsServer(): ReturnType<typeof stubServer> {
  return stubServer((call) => {
    if (call.method === "GET") {
      if (call.path === "/api/wallet") return { status: 200, body: { jlore: 250 } };
      if (call.path === "/api/packs") return { status: 200, body: packsInfo() };
      if (call.path === "/api/collection") return { status: 200, body: collectionOf() };
      if (call.path === "/api/tokens") return { status: 200, body: tokenBars() };
      if (call.path === "/api/cosmetics") return { status: 200, body: ownedCosmeticsOf() };
      if (call.path === "/api/cosmetic-packs") return { status: 200, body: cosmeticPacksInfo() };
      if (call.path === "/api/news") return { status: 200, body: { news: [] } };
      return undefined;
    }
    if (call.path === "/api/packs/open") return { status: 200, body: openedPack() };
    if (call.path === "/api/tokens/redeem-card" || call.path === "/api/tokens/redeem-cosmetic") {
      return { status: 200, body: { ...tokenBars(), tokens: { ...tokenBars().tokens, card: 0 } } };
    }
    if (call.path === "/api/cosmetic-packs/open") return { status: 200, body: openedCosmeticPack() };
    if (call.path === "/api/cosmetic-packs/chase") return { status: 200, body: cosmeticPacksInfo() };
    if (call.path === "/api/news/n1/claim") {
      const entry = { id: "n1", kind: "alert", version: null, title: "t", summary: "s", postedAt: 1 };
      return { status: 200, body: { news: [{ ...entry, reward: { kind: "jlore", amount: 100 }, claimed: true }] } };
    }
    return { status: 200, body: {} };
  });
}

describe("R3206000 a fresh answer needs no read, and a read already out is shared", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("R3206000 an answer is fresh for READ_CACHE_FRESH_MS and only kept after", () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(T0);
    keepRead(A1, keptPaths.wallet, { jlore: 5 });

    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS - 1);
    expect(freshRead<WalletResponse>(A2, keptPaths.wallet), "a renewed token is the same account").toEqual({ jlore: 5 });
    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS);
    expect(freshRead(A1, keptPaths.wallet), "the window is closed at its length").toBeUndefined();
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet), "an old answer is still shown").toEqual({ jlore: 5 });
    vi.setSystemTime(T0 - 1);
    expect(freshRead(A1, keptPaths.wallet), "a clock set back never keeps an answer fresh").toBeUndefined();
    expect(freshRead(null, keptPaths.wallet), "another account's is never used").toBeUndefined();
    expect(freshRead(B1, keptPaths.wallet)).toBeUndefined();

    // A read that finishes later makes it fresh again; expireRead ends it and keeps the value.
    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS);
    keepRead(A1, keptPaths.wallet, { jlore: 6 });
    expect(freshRead<WalletResponse>(A1, keptPaths.wallet)).toEqual({ jlore: 6 });
    expireRead(A1, keptPaths.wallet);
    expect(freshRead(A1, keptPaths.wallet)).toBeUndefined();
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet)).toEqual({ jlore: 6 });
  });

  it("R3206000 a fold into a kept answer keeps its age, and a write's answer is fresh", () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(T0);
    keepRead(A1, keptPaths.wallet, { jlore: 5 });
    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS - 1_000);

    updateCachedRead<WalletResponse>(A1, keptPaths.wallet, () => ({ jlore: 7 }));
    expect(freshRead<WalletResponse>(A1, keptPaths.wallet), "folded, and as fresh as the read it folded into").toEqual({
      jlore: 7,
    });
    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS);
    expect(freshRead(A1, keptPaths.wallet), "the fold did not make it fresher").toBeUndefined();
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet)).toEqual({ jlore: 7 });

    rememberRead(A1, keptPaths.wallet, { jlore: 8 });
    expect(freshRead<WalletResponse>(A1, keptPaths.wallet), "a write's answer is the server's latest word").toEqual({
      jlore: 8,
    });
  });

  it("R3206000 a second read inside the window sends no request, and one after it sends one", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(T0);
    const server = readsServer();

    await expect(getWallet(A1)).resolves.toEqual({ jlore: 250 });
    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS - 1);
    await expect(getWallet(A2), "the same account after a renewal").resolves.toEqual({ jlore: 250 });
    expect(server.to("GET", "/api/wallet")).toHaveLength(1);

    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS);
    await getWallet(A1);
    expect(server.to("GET", "/api/wallet")).toHaveLength(2);
    vi.setSystemTime(T0 + READ_CACHE_FRESH_MS + 1_000);
    await getWallet(A1);
    expect(server.to("GET", "/api/wallet"), "the read after the window made the answer fresh again").toHaveLength(2);
  });

  it("R3206000 two concurrent reads of one path send one fetch", async () => {
    const server = readsServer();

    const [first, second] = await Promise.all([getWallet(A1), getWallet(A1)]);
    expect(server.to("GET", "/api/wallet")).toHaveLength(1);
    expect(second).toBe(first);

    // A read that is not kept (the news) is shared as well while it is out, and read again after.
    await Promise.all([getNews(A1), getNews(A1)]);
    expect(server.to("GET", "/api/news")).toHaveLength(1);
    await getNews(A1);
    expect(server.to("GET", "/api/news")).toHaveLength(2);
  });

  it("R3206000 a write is never shared, and another token reads separately", async () => {
    const server = readsServer();
    for (const method of ["POST", "PUT", "DELETE"] as const) {
      await Promise.all([
        apiRequest("/api/x", { method, token: A1, body: {} }),
        apiRequest("/api/x", { method, token: A1, body: {} }),
      ]);
    }
    expect(server.calls.filter((call) => call.path === "/api/x"), "three methods, two sends each").toHaveLength(6);
    await Promise.all([apiRequest("/api/x", { method: "DELETE", token: A1 }), apiRequest("/api/x", { method: "DELETE", token: A1 })]);
    expect(server.calls.filter((call) => call.path === "/api/x"), "a DELETE with no body is no read").toHaveLength(8);

    await Promise.all([getWallet(A1), getWallet(B1)]);
    expect(server.to("GET", "/api/wallet"), "another account reads for itself").toHaveLength(2);
    clearReadCache();
    await Promise.all([getWallet(A1), getWallet(A2)]);
    expect(server.to("GET", "/api/wallet"), "two tokens out together are two reads").toHaveLength(4);

    clearReadCache();
    await getWallet(A1);
    await getWallet(A2);
    expect(server.to("GET", "/api/wallet"), "a renewed token finds its account's answer").toHaveLength(5);
  });

  it("R3206000 after a spend's forgetRead the next read fetches", async () => {
    const server = readsServer();
    await getWallet(A1);
    await getWallet(A1);
    expect(server.to("GET", "/api/wallet"), "inside the window").toHaveLength(1);

    await claimNews(A1, "n1");
    await getWallet(A1);
    expect(server.to("GET", "/api/wallet"), "a Jlore reward dropped the wallet").toHaveLength(2);

    await getWallet(A1);
    expect(server.to("GET", "/api/wallet"), "and the new read is fresh").toHaveLength(2);
  });

  it("R3206000 a pack opening, a redeemed token and a chase drop the reads their answers do not carry", async () => {
    readsServer();
    const warm = async (): Promise<void> => {
      await Promise.all([
        getCollection(A1),
        getOwnedCosmetics(A1),
        getTokens(A1),
        getCosmeticPacks(A1),
        getPacks(A1),
        getWallet(A1),
      ]);
    };
    const kept = (...paths: string[]): boolean[] => paths.map((path) => cachedRead(A1, path) !== undefined);
    const { collection, cosmetics, tokens, cosmeticPacks, packs, wallet } = keptPaths;

    await warm();
    expect(kept(collection, cosmetics, tokens, cosmeticPacks, packs, wallet)).toEqual([true, true, true, true, true, true]);
    await openPack(A1, "Core");
    expect(kept(collection, cosmetics, tokens, cosmeticPacks, packs, wallet), "an opening").toEqual([
      false,
      false,
      false,
      false,
      true,
      true,
    ]);

    await warm();
    await redeemCardToken(A1, "core-001");
    expect(kept(collection, cosmetics, tokens, cosmeticPacks), "a card token: its answer is the tokens'").toEqual([
      false,
      true,
      true,
      true,
    ]);

    await warm();
    await redeemCosmeticToken(A1, "Rare", "table-wood");
    expect(kept(collection, cosmetics, tokens, cosmeticPacks), "a cosmetic token").toEqual([true, false, true, true]);

    await warm();
    await openCosmeticPack(A1);
    expect(kept(collection, cosmetics, tokens, cosmeticPacks), "a cosmetic pack: its answer folds into the packs'").toEqual([
      false,
      false,
      false,
      true,
    ]);

    await warm();
    await buyChase(A1, "table-wood");
    expect(kept(collection, cosmetics, tokens, cosmeticPacks), "a chase: its answer is the packs'").toEqual([
      false,
      false,
      true,
      true,
    ]);
  });

  it("R3206000 a write that changes no kept read does not stop a read being kept, and a spend that settles does", async () => {
    const server = deferredFetch();
    const beside = getWallet(A1);
    const heartbeat = apiRequest("/api/presence", { method: "POST", token: A1, body: {} });
    server.reply(1, {});
    await heartbeat;
    server.reply(0, { jlore: 250 });
    await beside;
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet), "a heartbeat settled while it was out").toEqual({ jlore: 250 });

    forgetRead(A1, keptPaths.wallet);
    const spent = getWallet(A1);
    const spend = apiRequest("/api/packs/buy", { method: "POST", token: A1, body: {}, idempotencyKey: "k-1" });
    server.reply(3, { error: { code: "internal", message: "down" } }, 503);
    await expect(spend, "a spend that failed may still have run").rejects.toThrow("down");
    server.reply(2, { jlore: 250 });
    await spent;
    expect(cachedRead(A1, keptPaths.wallet), "the read beside it is not kept").toBeUndefined();
  });

  it("R3206000 signing out clears everything kept, and an answer landing after it is not kept", async () => {
    keepRead(A1, keptPaths.packs, packsInfo());
    keepRead(null, "/api/stats/cards", { cards: [] });
    const server = deferredFetch();
    const pending = getWallet(A1);
    expect(server.calls).toHaveLength(1);

    clearSession();
    expect(__readCacheSizeForTests(), "everything kept went with the sign-out").toBe(0);
    server.reply(0, { jlore: 99 });
    await expect(pending, "the caller still has its answer").resolves.toEqual({ jlore: 99 });

    expect(__readCacheSizeForTests(), "an answer to a read sent before it is not kept").toBe(0);
    expect(cachedRead(A1, keptPaths.wallet)).toBeUndefined();
  });

  it("R3206000 an aborted caller leaves the other's answer intact", async () => {
    const server = deferredFetch();
    const first = new AbortController();
    const second = new AbortController();
    const one = apiRequest<WalletResponse>("/api/wallet", { token: A1, signal: first.signal });
    const two = apiRequest<WalletResponse>("/api/wallet", { token: A1, signal: second.signal });
    const three = apiRequest<WalletResponse>("/api/wallet", { token: A1 });
    expect(server.calls, "one fetch for three callers").toHaveLength(1);

    first.abort();
    await expect(one).rejects.toBeInstanceOf(ApiUnreachableError);
    expect(server.calls[0]?.signal?.aborted, "the others still wait on the fetch").toBe(false);

    server.reply(0, { jlore: 41 });
    await expect(two).resolves.toEqual({ jlore: 41 });
    await expect(three).resolves.toEqual({ jlore: 41 });
    expect(server.calls).toHaveLength(1);
  });

  it("R3206000 the last caller to abort aborts the fetch, and a read already aborted sends nothing", async () => {
    const server = deferredFetch();
    const first = new AbortController();
    const second = new AbortController();
    const one = apiRequest("/api/wallet", { token: A1, signal: first.signal });
    const two = apiRequest("/api/wallet", { token: A1, signal: second.signal });

    first.abort();
    second.abort();
    await expect(one).rejects.toBeInstanceOf(ApiUnreachableError);
    await expect(two).rejects.toBeInstanceOf(ApiUnreachableError);
    expect(server.calls[0]?.signal?.aborted).toBe(true);

    // A later read asks again: it does not join the aborted one.
    const again = apiRequest("/api/wallet", { token: A1 });
    expect(server.calls).toHaveLength(2);
    server.reply(1, { jlore: 3 });
    await expect(again).resolves.toEqual({ jlore: 3 });

    const gone = new AbortController();
    gone.abort();
    await expect(apiRequest("/api/wallet", { token: A1, signal: gone.signal })).rejects.toBeInstanceOf(ApiUnreachableError);
    expect(server.calls).toHaveLength(2);
  });

  it("R3206000 a read that lands after a write is not kept, and a read asked after one does not join it", async () => {
    const server = deferredFetch();
    const before = getWallet(A1);
    rememberRead(A1, keptPaths.wallet, { jlore: 9 });
    server.reply(0, { jlore: 250 });
    await expect(before, "the caller has what it read").resolves.toEqual({ jlore: 250 });
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet), "the write's answer stays").toEqual({ jlore: 9 });

    forgetRead(A1, keptPaths.wallet);
    const first = getWallet(A1);
    forgetRead(A1, keptPaths.wallet);
    const second = getWallet(A1);
    expect(server.calls, "the read asked after a write went out of its own").toHaveLength(3);
    server.reply(1, { jlore: 1 });
    server.reply(2, { jlore: 2 });
    await Promise.all([first, second]);
    expect(cachedRead<WalletResponse>(A1, keptPaths.wallet), "the later read, which had the later answer, is kept").toEqual({
      jlore: 2,
    });
  });

  it("R3206000 a paid result drops the wallet, quests, tracks, packs, tokens and ladders", () => {
    writeSession({ accessToken: A1 });
    const constructed = ladderPath(keptPaths.leaderboard, "constructed");
    const random = ladderPath(keptPaths.leaderboard, "random");
    const paths = [
      keptPaths.wallet,
      keptPaths.quests,
      keptPaths.tracks,
      keptPaths.packs,
      keptPaths.tokens,
      constructed,
      random,
      keptPaths.collection,
    ];
    for (const path of paths) keepRead(A1, path, {});

    announceWallet(12);
    expect(paths.map((path) => cachedRead(A1, path) !== undefined), "a balance a server gave drops nothing").toEqual(
      paths.map(() => true),
    );

    announceWallet();
    expect(paths.map((path) => cachedRead(A1, path) !== undefined)).toEqual([
      false,
      false,
      false,
      false,
      false,
      false,
      false,
      true,
    ]);
  });
});
