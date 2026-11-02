// R2124000: `net/api.ts`'s spends go with an `Idempotency-Key`. A press mints one, its retries (after
// a timeout, a failed transport or a server error) send it again, and any answer from the server, a
// refusal included, ends the press, so the next press mints another. Reads send none. A 429 ran
// nothing, so it ends no press, and each spend is out from its press until it settles (R2378000).
//
// `fetch` is stubbed with one reply per call, in order: an answer, a failure of the transport, or
// none at all (the request hangs until `apiRequest` gives up). The keys live in the module for the
// page's life, so each test presses with a body of its own.

import { afterEach, describe, expect, it, vi } from "vitest";

import { API_REQUEST_TIMEOUT_SECONDS } from "@jackioh/server-config";

import {
  ApiRequestError,
  ApiUnreachableError,
  buyChase,
  buyPack,
  getPacks,
  isQueueMode,
  openCosmeticPack,
  openPack,
  redeemCardToken,
  redeemCode,
  redeemCosmeticToken,
  roomModeOf,
} from "./api.ts";
import { spendOutNow } from "./spending.ts";
import { jsonResponse } from "../test/stubServer.ts";

const TOKEN = "tok-r2124000";

/** What `crypto.randomUUID()` prints: a version 4 UUID, lower case. */
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

type Reply = { status: number; body?: unknown } | "hang" | "fail";

type Sent = { path: string; key: string | undefined };

/** Stubs `fetch` with `replies`, one per call in order, and records each call's path and key. */
function stubFetch(replies: Reply[]): Sent[] {
  const sent: Sent[] = [];
  const queue = [...replies];
  vi.stubGlobal(
    "fetch",
    vi.fn<(input: string, init?: RequestInit) => Promise<Response>>((input, init) => {
      const headers = init?.headers as Record<string, string> | undefined;
      sent.push({ path: new URL(input).pathname, key: headers?.["idempotency-key"] });
      const reply = queue.shift() ?? { status: 200, body: {} };
      if (reply === "hang") return new Promise<Response>(() => undefined);
      if (reply === "fail") return Promise.reject(new TypeError("Failed to fetch"));
      return Promise.resolve(jsonResponse(reply.status, reply.body));
    }),
  );
  return sent;
}

function refused(status: number, code: string): Reply {
  return { status, body: { error: { code, message: code } } };
}

const BOUGHT = { status: 200, body: { pack: "Core", jlore: 0, unopened: 1 } };

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("isQueueMode", () => {
  it("names the three queue modes and nothing else (#2160)", () => {
    for (const mode of ["bo1", "bo3", "random"]) expect(isQueueMode(mode)).toBe(true);
    for (const value of ["bo5", "", "BO1", null, undefined, 1, { mode: "bo1" }]) expect(isQueueMode(value)).toBe(false);
  });

  it("is what roomModeOf reads a mode-mismatch refusal's mode with", () => {
    const refusal = (mode: unknown): ApiRequestError => new ApiRequestError(409, { code: "conflict", message: "", details: { mode } });
    expect(roomModeOf(refusal("bo3"))).toBe("bo3");
    expect(roomModeOf(refusal("bo5"))).toBeNull();
    expect(roomModeOf(new Error("conflict"))).toBeNull();
  });
});

describe("R2124000 a spend runs once", () => {
  it("R2124000 a retry after a timeout sends the same key and the next press a new one", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    const sent = stubFetch(["hang", BOUGHT, BOUGHT]);

    const first = buyPack(TOKEN, "r2124000-timeout").then(
      () => null,
      (cause: unknown) => cause,
    );
    await vi.advanceTimersByTimeAsync(API_REQUEST_TIMEOUT_SECONDS * 1000);
    expect(await first).toBeInstanceOf(ApiUnreachableError);

    await expect(buyPack(TOKEN, "r2124000-timeout")).resolves.toEqual(BOUGHT.body);
    await expect(buyPack(TOKEN, "r2124000-timeout")).resolves.toEqual(BOUGHT.body);

    expect(sent.map((call) => call.path)).toEqual(["/api/packs/buy", "/api/packs/buy", "/api/packs/buy"]);
    const [timedOut, retry, nextPress] = sent.map((call) => call.key);
    expect(timedOut).toMatch(UUID);
    expect(retry).toBe(timedOut);
    expect(nextPress).toMatch(UUID);
    expect(nextPress).not.toBe(timedOut);
  });

  it("R2124000 a failed transport keeps the key for the retry", async () => {
    const sent = stubFetch(["fail", { status: 200, body: {} }]);

    await expect(openPack(TOKEN, "r2124000-transport")).rejects.toBeInstanceOf(ApiUnreachableError);
    await expect(openPack(TOKEN, "r2124000-transport")).resolves.toEqual({});

    expect(sent.map((call) => call.path)).toEqual(["/api/packs/open", "/api/packs/open"]);
    expect(sent[0]?.key).toMatch(UUID);
    expect(sent[1]?.key).toBe(sent[0]?.key);
  });

  it("R2124000 a server error keeps the key for the retry", async () => {
    const sent = stubFetch([refused(500, "internal"), refused(503, "unavailable"), { status: 200, body: {} }]);

    await expect(redeemCardToken(TOKEN, "r2124000-server-error")).rejects.toBeInstanceOf(ApiRequestError);
    await expect(redeemCardToken(TOKEN, "r2124000-server-error")).rejects.toBeInstanceOf(ApiRequestError);
    await expect(redeemCardToken(TOKEN, "r2124000-server-error")).resolves.toEqual({});

    const keys = sent.map((call) => call.key);
    expect(keys[0]).toMatch(UUID);
    expect(keys).toEqual([keys[0], keys[0], keys[0]]);
  });

  it("R2124000 a refusal is an answer, so the next press sends a new key", async () => {
    const sent = stubFetch([refused(409, "conflict"), refused(422, "idempotency_key_reused"), BOUGHT]);

    await expect(buyPack(TOKEN, "r2124000-refused")).rejects.toMatchObject({ status: 409, code: "conflict" });
    await expect(buyPack(TOKEN, "r2124000-refused")).rejects.toMatchObject({ status: 422, code: "idempotency_key_reused" });
    await expect(buyPack(TOKEN, "r2124000-refused")).resolves.toEqual(BOUGHT.body);

    const keys = sent.map((call) => call.key);
    for (const key of keys) expect(key).toMatch(UUID);
    expect(new Set(keys).size).toBe(3);
  });

  it("R2124000 another body is another press", async () => {
    const sent = stubFetch(["fail", BOUGHT, BOUGHT]);

    await expect(buyPack(TOKEN, "r2124000-body-a")).rejects.toBeInstanceOf(ApiUnreachableError);
    await buyPack(TOKEN, "r2124000-body-b");
    await buyPack(TOKEN, "r2124000-body-a");

    const [firstA, b, retryA] = sent.map((call) => call.key);
    expect(b).toMatch(UUID);
    expect(b).not.toBe(firstA);
    expect(retryA).toBe(firstA);
  });

  it("R2124000 every spend sends a key and a read sends none", async () => {
    const sent = stubFetch([]);

    await redeemCode(TOKEN, "HJKM-NPQR-STUV-WXYZ");
    await buyPack(TOKEN, "r2124000-every");
    await openPack(TOKEN, "r2124000-every");
    await redeemCardToken(TOKEN, "r2124000-every");
    await redeemCosmeticToken(TOKEN, "Rare", "r2124000-every");
    await openCosmeticPack(TOKEN);
    await buyChase(TOKEN, "r2124000-every");
    await getPacks(TOKEN);

    expect(sent.map((call) => call.path)).toEqual([
      "/api/codes/redeem",
      "/api/packs/buy",
      "/api/packs/open",
      "/api/tokens/redeem-card",
      "/api/tokens/redeem-cosmetic",
      "/api/cosmetic-packs/open",
      "/api/cosmetic-packs/chase",
      "/api/packs",
    ]);
    const spends = sent.slice(0, -1).map((call) => call.key);
    for (const key of spends) expect(key).toMatch(UUID);
    expect(new Set(spends).size).toBe(spends.length);
    expect(sent.at(-1)?.key).toBeUndefined();
  });
});

describe("R2378000 one spend at a time", () => {
  it("R2378000 a 429 ran nothing, so the press keeps its key", async () => {
    const sent = stubFetch([refused(429, "rate_limited"), BOUGHT]);

    await expect(buyPack(TOKEN, "r2378000-busy")).rejects.toMatchObject({ status: 429, code: "rate_limited" });
    await expect(buyPack(TOKEN, "r2378000-busy")).resolves.toEqual(BOUGHT.body);

    const [refusedKey, retryKey] = sent.map((call) => call.key);
    expect(refusedKey).toMatch(UUID);
    expect(retryKey).toBe(refusedKey);
  });

  it("R2378000 a spend is out from its press until its answer or its failure", async () => {
    stubFetch([BOUGHT, "fail"]);
    expect(spendOutNow()).toBe(false);

    const bought = buyPack(TOKEN, "r2378000-out");
    expect(spendOutNow(), "out from the press").toBe(true);
    await bought;
    expect(spendOutNow(), "in again on its answer").toBe(false);

    const failed = openPack(TOKEN, "r2378000-out");
    expect(spendOutNow()).toBe(true);
    await expect(failed).rejects.toBeInstanceOf(ApiUnreachableError);
    expect(spendOutNow(), "in again on its failure").toBe(false);
  });
});
