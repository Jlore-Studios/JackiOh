// #4444: the Friends drawer's chunk starts on rail interest, never while the browser asks to
// save data and never signed out.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { signIn } from "../test/stubServer.ts";
import { friendsDrawerChunk, prefetchFriendsDrawer, resetFriendsDrawerPrefetchForTests } from "./friendsDrawerChunk.ts";

const stubModule = { default: (): null => null };

beforeEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  resetFriendsDrawerPrefetchForTests();
});

afterEach(() => {
  window.localStorage.clear();
  window.sessionStorage.clear();
  Reflect.deleteProperty(navigator, "connection");
  vi.restoreAllMocks();
  resetFriendsDrawerPrefetchForTests();
});

describe("#4444 prefetching the Friends drawer", () => {
  it("#4444 rail interest prefetches the drawer chunk once", () => {
    signIn("tok-drawer");
    const load = vi.spyOn(friendsDrawerChunk, "load").mockResolvedValue(stubModule);
    prefetchFriendsDrawer();
    prefetchFriendsDrawer();
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("#4444 prefetching stops while the browser asks to save data", () => {
    signIn("tok-drawer");
    Object.defineProperty(navigator, "connection", { value: { saveData: true }, configurable: true });
    const load = vi.spyOn(friendsDrawerChunk, "load").mockResolvedValue(stubModule);
    prefetchFriendsDrawer();
    expect(load).not.toHaveBeenCalled();
  });

  it("#4444 a signed-out visitor prefetches no drawer bytes", () => {
    const load = vi.spyOn(friendsDrawerChunk, "load").mockResolvedValue(stubModule);
    prefetchFriendsDrawer();
    expect(load).not.toHaveBeenCalled();
  });
});
