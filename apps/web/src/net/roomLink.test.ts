// A room shared as a link (R767): the link's shape, how it is read off the address bar and kept for
// this tab, and what it sends. The lobby's use of it is `routes/play.test.tsx`'s; its trip through a
// sign-in is `routes/shell-gate.test.tsx`'s.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  COOP_LINK_STORAGE_KEY,
  ROOM_LINK_SHARE_TITLE,
  ROOM_LINK_STORAGE_KEY,
  adoptCoopLink,
  adoptRoomLink,
  coopLinkOf,
  coopLinkPath,
  coopLinkUrl,
  findRoomLinkInText,
  forgetCoopLink,
  forgetRoomLink,
  readCoopLink,
  readRoomLink,
  roomLinkOf,
  roomLinkPath,
  roomLinkUrl,
  sendRoomLink,
} from "./roomLink.ts";

function visit(url: string): void {
  window.history.replaceState(null, "", url);
}

beforeEach(() => {
  window.sessionStorage.clear();
});

afterEach(() => {
  window.history.replaceState(null, "", "/");
  window.sessionStorage.clear();
  vi.restoreAllMocks();
  Reflect.deleteProperty(navigator, "clipboard");
  Reflect.deleteProperty(navigator, "share");
});

describe("R767 the room link", () => {
  it("R767 roomLinkPath builds /play?room=<code>&mode=<mode>, and roomLinkOf reads it back", () => {
    expect(roomLinkPath("ABC234", "bo1")).toBe("/play?room=ABC234&mode=bo1");
    expect(roomLinkPath("ABC234", "bo3")).toBe("/play?room=ABC234&mode=bo3");
    expect(roomLinkUrl("ABC234", "random")).toBe(`${window.location.origin}/play?room=ABC234&mode=random`);
    for (const mode of ["bo1", "bo3", "random"] as const) {
      const search = roomLinkPath("QRSTUV", mode).slice("/play".length);
      expect(roomLinkOf(search)).toEqual({ code: "QRSTUV", mode });
    }
  });

  it("R767 roomLinkOf reads a code as R191 does and refuses one that is no room code", () => {
    expect(roomLinkOf("?room=%20ab-c2%2034%20&mode=bo3")).toEqual({ code: "ABC234", mode: "bo3" });
    // R104's alphabet has no 1, and a room code is exactly six characters.
    expect(roomLinkOf("?room=abc123")).toBeNull();
    expect(roomLinkOf("?room=abc23")).toBeNull();
    expect(roomLinkOf("?room=abc2345")).toBeNull();
    expect(roomLinkOf("?room=")).toBeNull();
    expect(roomLinkOf("?mode=bo1")).toBeNull();
    // The mode is only a hint: one that is none of the three is dropped, the code kept.
    expect(roomLinkOf("?room=abc234&mode=bo5")).toEqual({ code: "ABC234", mode: null });
    expect(roomLinkOf("?room=abc234")).toEqual({ code: "ABC234", mode: null });
  });

  it("R767 adoptRoomLink acts only on /play: it strips room and mode, keeps other parameters, and keeps a valid link for this tab", () => {
    visit("/play?room=abc234&mode=random&x=1#top");
    expect(adoptRoomLink()).toBe(true);
    expect(window.location.pathname).toBe("/play");
    expect(window.location.search).toBe("?x=1");
    expect(window.location.hash).toBe("#top");
    expect(readRoomLink()).toEqual({ code: "ABC234", mode: "random" });
    // Reading removes nothing, so React StrictMode's second call gets the same answer.
    expect(readRoomLink()).toEqual({ code: "ABC234", mode: "random" });
    forgetRoomLink();
    expect(readRoomLink()).toBeNull();
    expect(window.sessionStorage.getItem(ROOM_LINK_STORAGE_KEY)).toBeNull();

    // `room` on another path is some other page's: nothing is read, nothing is taken out.
    visit("/decks?room=abc234");
    expect(adoptRoomLink()).toBe(false);
    expect(window.location.search).toBe("?room=abc234");
    expect(readRoomLink()).toBeNull();

    // A code that is no room code is stripped all the same, and nothing is stored.
    visit("/play?room=abc123&mode=bo3");
    expect(adoptRoomLink()).toBe(false);
    expect(window.location.search).toBe("");
    expect(readRoomLink()).toBeNull();
    expect(window.sessionStorage.getItem(ROOM_LINK_STORAGE_KEY)).toBeNull();
  });

  it("R767 readRoomLink takes in the address bar's link when nothing adopted it yet", () => {
    visit("/play?room=abc234&mode=bo3");
    expect(readRoomLink()).toEqual({ code: "ABC234", mode: "bo3" });
    expect(window.location.search).toBe("");
  });

  it("R767 a stored link is read again: a planted code that is no room code, or bad JSON, is no link", () => {
    const plant = (value: string): void => {
      window.sessionStorage.setItem(ROOM_LINK_STORAGE_KEY, value);
    };
    plant(JSON.stringify({ code: "abc123", mode: "bo1" }));
    expect(readRoomLink()).toBeNull();
    plant(JSON.stringify({ code: 234567, mode: "bo1" }));
    expect(readRoomLink()).toBeNull();
    plant(JSON.stringify("ABC234"));
    expect(readRoomLink()).toBeNull();
    plant("null");
    expect(readRoomLink()).toBeNull();
    plant("{not json");
    expect(readRoomLink()).toBeNull();
    // A good code with a mode that is none of the three keeps the code and drops the mode.
    plant(JSON.stringify({ code: "abc234", mode: "bo9" }));
    expect(readRoomLink()).toEqual({ code: "ABC234", mode: null });
  });

  it("R767 blocked storage is no link and no crash", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    visit("/play?room=abc234&mode=bo1");
    expect(adoptRoomLink()).toBe(true);
    expect(window.location.search).toBe("");
    expect(readRoomLink()).toBeNull();
    expect(() => {
      forgetRoomLink();
    }).not.toThrow();
  });
});

describe("R767 sending the link", () => {
  const LINK = "https://example.test/play?room=ABC234&mode=bo1";

  it("R767 sendRoomLink opens the share sheet where there is one, and does not touch the clipboard", async () => {
    const share = vi.fn(() => Promise.resolve());
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await expect(sendRoomLink(LINK)).resolves.toBe("shared");
    expect(share).toHaveBeenCalledWith({ title: ROOM_LINK_SHARE_TITLE, url: LINK });
    expect(writeText).not.toHaveBeenCalled();
  });

  it("R767 sendRoomLink writes the clipboard where there is no share sheet", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await expect(sendRoomLink(LINK)).resolves.toBe("copied");
    expect(writeText).toHaveBeenCalledWith(LINK);
  });

  it("R767 a refused share rejects and does not fall back to the clipboard; a refused clipboard rejects", async () => {
    const share = vi.fn(() => Promise.reject(new DOMException("closed", "AbortError")));
    const writeText = vi.fn(() => Promise.reject(new Error("denied")));
    Object.defineProperty(navigator, "share", { configurable: true, value: share });
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await expect(sendRoomLink(LINK)).rejects.toThrow("closed");
    expect(writeText).not.toHaveBeenCalled();

    Reflect.deleteProperty(navigator, "share");
    await expect(sendRoomLink(LINK)).rejects.toThrow("denied");
  });
});

describe("R2094000 the co-op room link", () => {
  it("R2094000 coopLinkPath builds /together?coop=<code>, and coopLinkOf reads it back R191-style", () => {
    expect(coopLinkPath("ABC234")).toBe("/together?coop=ABC234");
    expect(coopLinkUrl("ABC234")).toBe(`${window.location.origin}/together?coop=ABC234`);
    expect(coopLinkOf(coopLinkPath("QRSTUV").slice("/together".length))).toBe("QRSTUV");
  });

  it("R2094000 coopLinkOf reads a code as R191 does and refuses one that is no room code", () => {
    expect(coopLinkOf("?coop=%20ab-c2%2034%20")).toBe("ABC234");
    expect(coopLinkOf("?coop=abc123")).toBeNull();
    expect(coopLinkOf("?coop=abc23")).toBeNull();
    expect(coopLinkOf("?coop=abc2345")).toBeNull();
    expect(coopLinkOf("?coop=")).toBeNull();
    expect(coopLinkOf("?room=abc234")).toBeNull();
    expect(coopLinkOf("")).toBeNull();
  });

  it("R2094000 adoptCoopLink acts only on /together, strips coop, keeps other params, stores the valid code", () => {
    visit("/together?coop=abc234&x=1#top");
    expect(adoptCoopLink()).toBe(true);
    expect(window.location.pathname).toBe("/together");
    expect(window.location.search).toBe("?x=1");
    expect(window.location.hash).toBe("#top");
    expect(readCoopLink()).toBe("ABC234");
    expect(readCoopLink()).toBe("ABC234");
    forgetCoopLink();
    expect(readCoopLink()).toBeNull();
    expect(window.sessionStorage.getItem(COOP_LINK_STORAGE_KEY)).toBeNull();

    visit("/decks?coop=abc234");
    expect(adoptCoopLink()).toBe(false);
    expect(window.location.search).toBe("?coop=abc234");
    expect(readCoopLink()).toBeNull();

    visit("/together?coop=abc123");
    expect(adoptCoopLink()).toBe(false);
    expect(window.location.search).toBe("");
    expect(readCoopLink()).toBeNull();
    expect(window.sessionStorage.getItem(COOP_LINK_STORAGE_KEY)).toBeNull();
  });

  it("R2094000 readCoopLink takes in the address bar's link when nothing adopted it yet", () => {
    visit("/together?coop=abc234");
    expect(readCoopLink()).toBe("ABC234");
    expect(window.location.search).toBe("");
  });

  it("R2094000 readCoopLink re-reads a stored code and refuses a planted bad code or bad JSON", () => {
    const plant = (value: string): void => {
      window.sessionStorage.setItem(COOP_LINK_STORAGE_KEY, value);
    };
    plant(JSON.stringify({ code: "abc123" }));
    expect(readCoopLink()).toBeNull();
    plant(JSON.stringify({ code: 234567 }));
    expect(readCoopLink()).toBeNull();
    plant(JSON.stringify("ABC234"));
    expect(readCoopLink()).toBeNull();
    plant("null");
    expect(readCoopLink()).toBeNull();
    plant("{not json");
    expect(readCoopLink()).toBeNull();
    // A good code is read back, upper-cased as the Join button sends it.
    plant(JSON.stringify({ code: "abc234" }));
    expect(readCoopLink()).toBe("ABC234");
  });

  it("R2094000 blocked storage is no link and no crash", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    visit("/together?coop=abc234");
    expect(adoptCoopLink()).toBe(true);
    expect(window.location.search).toBe("");
    expect(readCoopLink()).toBeNull();
    expect(() => {
      forgetCoopLink();
    }).not.toThrow();
  });
});

describe("R2094000 a room link in chat text", () => {
  const origin = window.location.origin;

  it("R2094000 findRoomLinkInText accepts this origin's /play?room= and /together?coop= links and refuses another origin, a malformed code, or no link", () => {
    // Full URLs and bare paths both count; the code is canonicalized.
    expect(findRoomLinkInText(`join here: ${origin}/together?coop=abc234`, origin)).toEqual({
      path: "/together?coop=ABC234",
    });
    expect(findRoomLinkInText("join here: /together?coop=ABC234", origin)).toEqual({ path: "/together?coop=ABC234" });
    expect(findRoomLinkInText(`join here: ${origin}/play?room=abc234&mode=bo1`, origin)).toEqual({
      path: "/play?room=ABC234&mode=bo1",
    });
    // A 1v1 link with no usable mode still joins: the code fills in, the mode stays the last choice.
    expect(findRoomLinkInText(`join here: ${origin}/play?room=abc234`, origin)).toEqual({ path: "/play?room=ABC234" });
    // The invite's own sentence carries the link beside trailing punctuation.
    expect(findRoomLinkInText(`Join my co-op game against the AI here: ${origin}/together?coop=ABC234 (room code ABC234).`, origin)).toEqual({
      path: "/together?coop=ABC234",
    });

    expect(findRoomLinkInText("join here: https://evil.test/together?coop=ABC234", origin)).toBeNull();
    expect(findRoomLinkInText("join here: https://evil.test/play?room=ABC234&mode=bo1", origin)).toBeNull();
    expect(findRoomLinkInText(`join here: ${origin}/together?coop=abc123`, origin)).toBeNull();
    expect(findRoomLinkInText(`join here: ${origin}/play?room=abc123&mode=bo1`, origin)).toBeNull();
    // A link on the wrong path draws no button.
    expect(findRoomLinkInText(`join here: ${origin}/play?coop=ABC234`, origin)).toBeNull();
    expect(findRoomLinkInText(`join here: ${origin}/together?room=ABC234&mode=bo1`, origin)).toBeNull();
    expect(findRoomLinkInText("good luck, have fun", origin)).toBeNull();
    expect(findRoomLinkInText("", origin)).toBeNull();
  });

  it("R2094000 findRoomLinkInText reads only the first valid link", () => {
    expect(findRoomLinkInText(`nope https://evil.test/together?coop=ABC234 then ${origin}/together?coop=QRSTUV`, origin)).toEqual({
      path: "/together?coop=QRSTUV",
    });
  });
});

describe("R2094000 sending the co-op link", () => {
  const LINK = `${window.location.origin}/together?coop=ABC234`;

  it("R2094000 sendRoomLink writes the co-op link to the clipboard where there is no share sheet", async () => {
    const writeText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText } });
    await expect(sendRoomLink(LINK)).resolves.toBe("copied");
    expect(writeText).toHaveBeenCalledWith(LINK);
  });
});
