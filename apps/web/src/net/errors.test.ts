// #2160: the one `messageOf` and `attempt` every screen and worker takes.

import { describe, expect, it } from "vitest";

import { ApiRequestError } from "./api.ts";
import { attempt, isAbort, messageOf } from "./errors.ts";

describe("messageOf", () => {
  it("reads an Error's message, a server refusal's included", () => {
    expect(messageOf(new Error("the socket closed"))).toBe("the socket closed");
    expect(messageOf(new TypeError("not a function"))).toBe("not a function");
    expect(messageOf(new ApiRequestError(409, { code: "conflict", message: "Finish your current match first." }))).toBe(
      "Finish your current match first.",
    );
  });

  it("writes anything else as text", () => {
    expect(messageOf("offline")).toBe("offline");
    expect(messageOf(404)).toBe("404");
    expect(messageOf(undefined)).toBe("undefined");
    expect(messageOf({ message: "not an Error" })).toBe("[object Object]");
  });
});

describe("isAbort", () => {
  it("is true for an AbortError and nothing else", () => {
    expect(isAbort(new DOMException("canceled", "AbortError"))).toBe(true);
    expect(isAbort(new DOMException("blocked", "NotAllowedError"))).toBe(false);
    expect(isAbort(new Error("AbortError"))).toBe(false);
    expect(isAbort("AbortError")).toBe(false);
  });
});

describe("attempt", () => {
  it("passes the call's answer and its rejection through", async () => {
    await expect(attempt(() => Promise.resolve(7))).resolves.toBe(7);
    await expect(attempt(() => Promise.reject(new Error("refused")))).rejects.toThrow("refused");
  });

  it("turns a synchronous throw into a rejection, and runs the call only after the caller returns", async () => {
    let ran = false;
    const pending = attempt((): Promise<never> => {
      ran = true;
      throw new Error("thrown");
    });
    expect(ran).toBe(false);
    await expect(pending).rejects.toThrow("thrown");
    expect(ran).toBe(true);
  });
});
