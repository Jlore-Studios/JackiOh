// #2932: `usePhonePortrait` (phoneBar.ts) reads board.css's phone portrait query, `PHONE_PORTRAIT_QUERY`,
// and follows it live. jsdom has no media queries, so the tests answer `matchMedia` themselves.

import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { PHONE_BAR_QUERY, PHONE_PORTRAIT_QUERY } from "./config.ts";
import { usePhoneBar, usePhonePortrait } from "./phoneBar.ts";
import { stubPhoneBar, stubPhonePortrait } from "../test/phoneBar.ts";

let restore: (() => void) | null = null;

afterEach(() => {
  cleanup();
  restore?.();
  restore = null;
});

describe("#2932 usePhonePortrait follows the phone portrait query", () => {
  it("reads true on a portrait phone and false off it", () => {
    restore = stubPhonePortrait(true);
    expect(renderHook(() => usePhonePortrait()).result.current).toBe(true);
    restore();
    restore = stubPhonePortrait(false);
    expect(renderHook(() => usePhonePortrait()).result.current).toBe(false);
  });

  it("a portrait phone matches the phone bar's query too, a landscape phone only the bar's", () => {
    restore = stubPhonePortrait(true);
    expect(renderHook(() => usePhoneBar()).result.current).toBe(true);
    restore();
    restore = stubPhoneBar(true);
    expect(renderHook(() => usePhonePortrait()).result.current).toBe(false);
    expect(renderHook(() => usePhoneBar()).result.current).toBe(true);
  });

  it("asks for the named queries", () => {
    const asked: string[] = [];
    const original = window.matchMedia;
    window.matchMedia = ((query: string) => {
      asked.push(query);
      return original(query);
    }) as typeof window.matchMedia;
    try {
      renderHook(() => usePhonePortrait());
    } finally {
      window.matchMedia = original;
    }
    expect(asked).toContain(PHONE_PORTRAIT_QUERY);
    expect(asked).not.toContain(PHONE_BAR_QUERY);
  });

  it("re-renders its caller when the query flips", () => {
    let matches = false;
    let onChange: (() => void) | null = null;
    const original = window.matchMedia;
    window.matchMedia = ((query: string) => {
      if (query !== PHONE_PORTRAIT_QUERY) return original(query);
      return {
        media: query,
        get matches() {
          return matches;
        },
        addEventListener: (_type: string, listener: () => void) => {
          onChange = listener;
        },
        removeEventListener: () => {
          onChange = null;
        },
      } as unknown as MediaQueryList;
    }) as typeof window.matchMedia;
    try {
      const { result } = renderHook(() => usePhonePortrait());
      expect(result.current).toBe(false);
      matches = true;
      act(() => {
        onChange?.();
      });
      expect(result.current).toBe(true);
    } finally {
      window.matchMedia = original;
    }
  });

  it("reads false in a host with no matchMedia", () => {
    const host = window as unknown as { matchMedia: unknown };
    const original = host.matchMedia;
    // A host without the API, as an old WebView is.
    host.matchMedia = undefined;
    try {
      expect(renderHook(() => usePhonePortrait()).result.current).toBe(false);
    } finally {
      host.matchMedia = original;
    }
  });
});
