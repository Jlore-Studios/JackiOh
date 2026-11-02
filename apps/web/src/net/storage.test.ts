// #2160: the one read, write and removal every module makes of the browser's storage. A storage
// that throws (blocked site data, a full quota) or is missing reads as empty and keeps nothing.

import { afterEach, describe, expect, it } from "vitest";

import { getStored, removeStored, setStored } from "./storage.ts";

const KEY = "jackioh.test.storage";

/** A storage whose every call throws, as a blocked or full one does. */
const refusing = {
  getItem: () => {
    throw new DOMException("blocked", "SecurityError");
  },
  setItem: () => {
    throw new DOMException("full", "QuotaExceededError");
  },
  removeItem: () => {
    throw new DOMException("blocked", "SecurityError");
  },
} as unknown as Storage;

afterEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
});

describe("getStored, setStored and removeStored", () => {
  it("keep, read and forget a value, in localStorage unless another storage is named", () => {
    expect(getStored(KEY)).toBeNull();
    expect(setStored(KEY, "kept")).toBe(true);
    expect(window.localStorage.getItem(KEY)).toBe("kept");
    expect(getStored(KEY)).toBe("kept");
    removeStored(KEY);
    expect(getStored(KEY)).toBeNull();

    expect(setStored(KEY, "this tab", window.sessionStorage)).toBe(true);
    expect(getStored(KEY, window.sessionStorage)).toBe("this tab");
    expect(getStored(KEY)).toBeNull();
    removeStored(KEY, window.sessionStorage);
    expect(window.sessionStorage.getItem(KEY)).toBeNull();
  });

  it("read nothing and keep nothing from a storage that throws or is missing", () => {
    expect(getStored(KEY, refusing)).toBeNull();
    expect(setStored(KEY, "lost", refusing)).toBe(false);
    expect(() => removeStored(KEY, refusing)).not.toThrow();

    expect(getStored(KEY, null)).toBeNull();
    expect(setStored(KEY, "lost", null)).toBe(false);
    expect(() => removeStored(KEY, null)).not.toThrow();
  });
});
