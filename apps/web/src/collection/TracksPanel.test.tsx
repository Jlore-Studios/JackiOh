// The tracks panel's bar (#1664): the economy's `FillBar` at rest in place of the bare `<progress>`,
// still `levelXp − xpToNext` of `levelXp` as the server sends them (the client knows no curve), still
// a native `<progress>` the e2e specs read, and every test id the panel had. The panel's other words
// are `collection.test.tsx`'s.

import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { SHIPPED_SETS, newestShippedSet } from "@jackioh/shared";

import { collectionTestid as T } from "./collection.ts";
import { track, tracksOf } from "./fixtures.ts";
import TracksPanel from "./TracksPanel.tsx";

afterEach(cleanup);

describe("#1664 the tracks panel's XP bar", () => {
  it("shows levelXp − xpToNext of levelXp, as a native progress the bar keeps", () => {
    render(<TracksPanel tracks={[track({ level: 4, xp: 600, levelXp: 400, xpToNext: 130, unlocked: true })]} />);

    const bar = within(screen.getByTestId(T.track("Core"))).getByRole("progressbar", { name: "Core track XP" });
    expect(bar.tagName).toBe("PROGRESS");
    expect(bar).toHaveAttribute("max", "400");
    expect(bar).toHaveAttribute("value", "270");
    // The bar is the economy's, resting at its value: it never plays a sweep.
    const picture = bar.closest(".fill-bar");
    expect(picture).toHaveAttribute("data-state", "resting");
    expect(picture).toHaveTextContent("Core track XP");
    expect(picture).toHaveTextContent("270 / 400");
    expect(picture?.querySelector<HTMLElement>(".fill-bar__fill")?.style.transform).toBe("scaleX(0.675)");
    expect(picture?.querySelector(".fill-bar__fill--sweep")).toBeNull();
  });

  it("keeps every test id of the track's row", () => {
    const newest = newestShippedSet();
    render(<TracksPanel tracks={tracksOf().tracks} />);

    for (const set of SHIPPED_SETS) {
      const row = screen.getByTestId(T.track(set));
      expect(within(row).getByTestId(T.trackLevel(set))).toBeInTheDocument();
      expect(within(row).getByTestId(T.trackXp(set))).toBeInTheDocument();
      expect(within(row).getByTestId(T.trackRewards(set))).toBeInTheDocument();
      expect(within(row).getByRole("progressbar")).toBeInTheDocument();
    }
    // A locked track has its lock line, an unlocked one does not.
    expect(screen.getByTestId(T.track(newest))).toHaveAttribute("data-unlocked", "true");
    expect(screen.queryByTestId(T.trackLock(newest))).toBeNull();
    expect(screen.getByTestId(T.trackLock("Core"))).toBeInTheDocument();
    expect(screen.getByTestId(T.trackXp(newest))).toHaveTextContent("650 XP to the next level");

    const bar = within(screen.getByTestId(T.track(newest))).getByRole("progressbar");
    expect(bar).toHaveAttribute("max", "700");
    expect(bar).toHaveAttribute("value", "50");
  });

  it("a track on its last level draws no bar", () => {
    render(<TracksPanel tracks={[track({ levelXp: null, xpToNext: null, nextRewards: null, unlocked: true })]} />);

    expect(screen.queryByRole("progressbar")).toBeNull();
    expect(screen.getByTestId(T.trackXp("Core"))).toHaveTextContent("Last level reached.");
  });
});
