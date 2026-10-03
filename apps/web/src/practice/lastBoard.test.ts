// R508: the human's last practice board on the device (lastBoard.ts), read as untrusted input.

import { afterEach, describe, expect, it, vi } from "vitest";

import { LAST_BOARD_STORAGE_KEY, readLastBoard, writeLastBoard } from "./lastBoard.ts";

afterEach(() => {
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe("R508 the last practice board on the device", () => {
  it("R508 is none until a board is written, then the board written, card and face only", () => {
    expect(readLastBoard()).toEqual([]);
    writeLastBoard([{ defId: "core-008", radiant: true }, { defId: "classic-090", radiant: false }]);
    expect(readLastBoard()).toEqual([{ defId: "core-008", radiant: true }, { defId: "classic-090", radiant: false }]);
  });

  it("R508 reads anything malformed as no board, and drops entries that are not a card and a face", () => {
    window.localStorage.setItem(LAST_BOARD_STORAGE_KEY, "{not json");
    expect(readLastBoard()).toEqual([]);
    window.localStorage.setItem(LAST_BOARD_STORAGE_KEY, JSON.stringify({ defId: "core-008", radiant: false }));
    expect(readLastBoard()).toEqual([]);
    window.localStorage.setItem(
      LAST_BOARD_STORAGE_KEY,
      JSON.stringify([{ defId: "core-008", radiant: false, attack: 9 }, { defId: 8, radiant: false }, null, { defId: "core-002" }]),
    );
    expect(readLastBoard()).toEqual([{ defId: "core-008", radiant: false }]);
  });

  it("R508 blocked storage is no board on read and keeps nothing on write, without throwing", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(() => {
      writeLastBoard([{ defId: "core-008", radiant: false }]);
    }).not.toThrow();
    expect(readLastBoard()).toEqual([]);
  });
});
