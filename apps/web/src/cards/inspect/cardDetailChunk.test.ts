// #4517: the card detail dialog's chunk starts on fan-card interest, never while the browser
// asks to save data.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { cardDetailChunk, prefetchCardDetail, resetCardDetailPrefetchForTests } from "./cardDetailChunk.ts";

const stubModule = { default: (): null => null };

beforeEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  resetCardDetailPrefetchForTests();
});

afterEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  Reflect.deleteProperty(navigator, "connection");
  vi.restoreAllMocks();
  resetCardDetailPrefetchForTests();
});

describe("#4517 prefetching the card detail dialog", () => {
  it("#4517 fan-card interest prefetches the dialog chunk once", () => {
    const load = vi.spyOn(cardDetailChunk, "load").mockResolvedValue(stubModule);
    prefetchCardDetail();
    prefetchCardDetail();
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("#4517 prefetching stops while the browser asks to save data", () => {
    Object.defineProperty(navigator, "connection", { value: { saveData: true }, configurable: true });
    const load = vi.spyOn(cardDetailChunk, "load").mockResolvedValue(stubModule);
    prefetchCardDetail();
    expect(load).not.toHaveBeenCalled();
  });
});
