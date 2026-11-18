// The social read's unread counts (#4015): friend ids from outside never become object keys.

import { describe, expect, it } from "vitest";

import { unreadCountsOf } from "./session.ts";

describe("unreadCountsOf", () => {
  it("#4015 ignores prototype-pollution friend ids and keeps the real counts", () => {
    const counts = unreadCountsOf({
      unread: JSON.parse(
        '{"friend-1":3,"__proto__":{"polluted":true},"constructor":{"prototype":{"polluted":true}}}',
      ),
    });
    expect(counts).toEqual({ "friend-1": 3 });
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });

  it("#4015 drops numeric proto keys instead of shadowing the result", () => {
    const counts = unreadCountsOf({
      unread: JSON.parse('{"friend-1":3,"__proto__":7,"constructor":7,"prototype":7}'),
    });
    expect(counts).toEqual({ "friend-1": 3 });
    expect(Object.hasOwn(counts, "constructor")).toBe(false);
    expect(Object.hasOwn(counts, "prototype")).toBe(false);
    expect(counts.constructor).toBe(Object);
    expect(({} as Record<string, unknown>)["polluted"]).toBeUndefined();
  });
});
