// R4039001: while the tab is hidden its title counts what waits on the player; the screen's own title comes back
// as the tab shows, and a board's and the Daily screen's titles are their own.

import { cleanup, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { resetVisibility, setVisibility } from "../test/setup.ts";
import { at } from "../test/stubServer.ts";
import { titleWithCount, useHiddenTitleCount } from "./title.ts";

const TITLE = "Collection · JackiOh";

beforeEach(() => {
  at("/collection");
  document.title = TITLE;
});

afterEach(() => {
  cleanup();
  resetVisibility();
});

describe("R4039001 the hidden tab's title", () => {
  it("R4039001 the count goes before the title, and none at zero", () => {
    expect(titleWithCount(TITLE, 3)).toBe(`(3) ${TITLE}`);
    expect(titleWithCount(TITLE, 0)).toBe(TITLE);
  });

  it("R4039001 the title counts what waits only while the tab is hidden, and the screen's own title comes back as it shows", () => {
    const { rerender, unmount } = renderHook(({ count }: { count: number }) => {
      useHiddenTitleCount(count);
    }, { initialProps: { count: 2 } });
    expect(document.title).toBe(TITLE);

    setVisibility("hidden");
    expect(document.title).toBe(`(2) ${TITLE}`);
    rerender({ count: 3 });
    expect(document.title).toBe(`(3) ${TITLE}`);
    rerender({ count: 0 });
    expect(document.title).toBe(TITLE);
    rerender({ count: 1 });
    expect(document.title).toBe(`(1) ${TITLE}`);

    setVisibility("visible");
    expect(document.title).toBe(TITLE);

    setVisibility("hidden");
    expect(document.title).toBe(`(1) ${TITLE}`);
    unmount();
    expect(document.title).toBe(TITLE);
  });

  it("R4039001 a board's and the Daily screen's titles are their own", () => {
    for (const path of ["/daily", "/daily/m-1", "/match/m-1"]) {
      at(path);
      document.title = "Your turn · JackiOh";
      const { unmount } = renderHook(() => {
        useHiddenTitleCount(2);
      });
      setVisibility("hidden");
      expect(document.title, path).toBe("Your turn · JackiOh");
      setVisibility("visible");
      unmount();
    }
  });
});
