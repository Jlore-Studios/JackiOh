// R2379000: giving a practice seed back (`DELETE /api/practice/seeds/:seed`).

import { afterEach, describe, expect, it, vi } from "vitest";

import { PRACTICE_EARNING_API } from "./practice.ts";
import { must, stubServer } from "../test/stubServer.ts";

/** The caller's token, built from parts rather than written as one credential-like literal. */
const TOKEN = ["tok", "1"].join("-");

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("R2379000 giving a practice seed back", () => {
  it("R2379000 the route's release is a keepalive DELETE of the seed carrying the caller's token", async () => {
    const server = stubServer(() => ({ status: 200, body: { released: true } }));
    await expect(PRACTICE_EARNING_API.release(TOKEN, "seed-1")).resolves.toEqual({ released: true });
    expect(server.calls, "exactly one request").toHaveLength(1);
    const call = must(server.calls[0], "the release");
    expect(call.method).toBe("DELETE");
    expect(call.path).toBe("/api/practice/seeds/seed-1");
    expect(call.keepalive).toBe(true);
    expect(call.headers["authorization"]).toBe(`Bearer ${TOKEN}`);
  });
});
