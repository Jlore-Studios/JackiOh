// #1650: the rank crest. One drawing per rank and badge, one division rule for every Grape tier, the
// words a screen reader reads unchanged, and a Jlorious shimmer that stops under reduced motion.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";

import type { GrapeTier, PeakBadge, VisibleRank } from "../net/api.ts";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import RankCrest, { CREST_SIZE_PX, divisionEmbellishment } from "./RankCrest.tsx";
import { badgeWords, rankWords } from "./rank.ts";

const GRAPE_TIERS: readonly GrapeTier[] = ["rotten", "normal", "large", "golden", "mythic"];
const DIVISIONS = [3, 2, 1] as const;

const RAISIN: VisibleRank = { tier: "raisin", placementsPlayed: 2, placementGames: 5 };
const JLORIOUS: VisibleRank = { tier: "jlorious", position: 7 };
const JLORIOUS_BADGE: PeakBadge = { seasonId: "v0.2", tier: "jlorious", position: 3 };

function grape(tier: GrapeTier, division: number): VisibleRank {
  return { tier, division, pips: 2, pipsPerDivision: 4, floor: "rotten" };
}

function badge(tier: GrapeTier, division: number): PeakBadge {
  return { seasonId: "v0.2", tier, division };
}

const RANKS: readonly VisibleRank[] = [
  RAISIN,
  ...GRAPE_TIERS.flatMap((tier) => DIVISIONS.map((division) => grape(tier, division))),
  JLORIOUS,
];

const BADGES: readonly PeakBadge[] = [
  ...GRAPE_TIERS.flatMap((tier) => DIVISIONS.map((division) => badge(tier, division))),
  JLORIOUS_BADGE,
];

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  window.localStorage.clear();
  __resetSettingsForTests();
});

/** The one crest on screen, found by its name. */
function crestNamed(name: string): HTMLElement {
  return screen.getByRole("img", { name });
}

/** A crest's drawing: its svg's markup. */
function drawing(crest: HTMLElement): string {
  return crest.querySelector("svg")?.outerHTML ?? "";
}

function embellishments(crest: HTMLElement): number {
  return crest.querySelectorAll(".rank-crest__stud, .rank-crest__frame-line").length;
}

describe("#1650 the rank crest", () => {
  it("#1650 draws a distinct crest for Raisin, every Grape tier at III, II and I, and Jlorious", () => {
    const drawings = new Set<string>();
    for (const rank of RANKS) {
      const { unmount } = render(<RankCrest rank={rank} size="small" />);
      drawings.add(drawing(crestNamed(rankWords(rank))));
      unmount();
    }
    expect(RANKS).toHaveLength(17);
    expect(drawings.size).toBe(17);
  });

  it("#1650 draws a distinct crest for every season badge", () => {
    const drawings = new Set<string>();
    for (const one of BADGES) {
      const { unmount } = render(<RankCrest badge={one} size="small" />);
      drawings.add(drawing(crestNamed(badgeWords(one))));
      unmount();
    }
    expect(BADGES).toHaveLength(16);
    expect(drawings.size).toBe(16);
  });

  it("#1650 one division rule: each step from III to I adds the same embellishment in every Grape tier", () => {
    /** The studs and frame lines of one tier's crest at III, II and I, drawn by `draw`. */
    function counts(draw: (tier: GrapeTier, division: number) => { name: string; crest: ReactElement }, tier: GrapeTier): number[] {
      return DIVISIONS.map((division) => {
        const { name, crest } = draw(tier, division);
        const { unmount } = render(crest);
        const count = embellishments(crestNamed(name));
        unmount();
        return count;
      });
    }
    const asRank = (tier: GrapeTier, division: number) => ({
      name: rankWords(grape(tier, division)),
      crest: <RankCrest rank={grape(tier, division)} size="small" />,
    });
    const asBadge = (tier: GrapeTier, division: number) => ({
      name: badgeWords(badge(tier, division)),
      crest: <RankCrest badge={badge(tier, division)} size="small" />,
    });

    const rotten = counts(asRank, "rotten");
    for (const tier of GRAPE_TIERS) {
      for (const found of [counts(asRank, tier), counts(asBadge, tier)]) {
        const [iii, ii, i] = found as [number, number, number];
        expect(iii).toBeLessThan(ii);
        expect(ii).toBeLessThan(i);
        expect(ii - iii).toBe(i - ii);
        expect(found).toEqual(rotten);
      }
    }
    expect(divisionEmbellishment(3)).toEqual({ studs: 1, frameLines: 0 });
    expect(divisionEmbellishment(2)).toEqual({ studs: 2, frameLines: 1 });
    expect(divisionEmbellishment(1)).toEqual({ studs: 3, frameLines: 2 });
  });

  it("#1650 is an img named and titled with rankWords or badgeWords", () => {
    for (const rank of RANKS) {
      const { unmount } = render(<RankCrest rank={rank} size="large" />);
      const crest = crestNamed(rankWords(rank));
      expect(crest).toHaveAttribute("title", rankWords(rank));
      expect(crest.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
      unmount();
    }
    for (const one of BADGES) {
      const { unmount } = render(<RankCrest badge={one} size="large" />);
      const crest = crestNamed(badgeWords(one));
      expect(crest).toHaveAttribute("title", badgeWords(one));
      expect(crest.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
      unmount();
    }
  });

  it("#1650 draws pips and placements as a pip bar", () => {
    const first = render(<RankCrest rank={grape("normal", 3)} size="small" />);
    let crest = crestNamed(rankWords(grape("normal", 3)));
    expect(crest.querySelectorAll(".rank-crest__pip")).toHaveLength(4);
    expect(crest.querySelectorAll('.rank-crest__pip[data-filled="true"]')).toHaveLength(2);
    first.unmount();

    const second = render(<RankCrest rank={RAISIN} size="small" />);
    crest = crestNamed(rankWords(RAISIN));
    expect(crest.querySelectorAll(".rank-crest__pip")).toHaveLength(5);
    expect(crest.querySelectorAll('.rank-crest__pip[data-filled="true"]')).toHaveLength(2);
    second.unmount();

    const third = render(<RankCrest badge={badge("normal", 3)} size="small" />);
    expect(crestNamed(badgeWords(badge("normal", 3))).querySelectorAll(".rank-crest__pip")).toHaveLength(0);
    third.unmount();

    render(<RankCrest rank={JLORIOUS} size="small" />);
    expect(crestNamed(rankWords(JLORIOUS)).querySelectorAll(".rank-crest__pip")).toHaveLength(0);
  });

  it("#1650 prints the Jlorious position and shimmers only under full motion", () => {
    const first = render(<RankCrest rank={JLORIOUS} size="large" />);
    let crest = crestNamed(rankWords(JLORIOUS));
    expect(crest.querySelector(".rank-crest__position")).toHaveTextContent("#7");
    expect(crest.querySelector(".rank-crest__shimmer")).not.toBeNull();
    expect(crest).toHaveAttribute("data-motion", "full");
    first.unmount();

    const second = render(<RankCrest badge={JLORIOUS_BADGE} size="large" />);
    expect(crestNamed(badgeWords(JLORIOUS_BADGE)).querySelector(".rank-crest__position")).toHaveTextContent("#3");
    second.unmount();

    // The OS preference.
    setReducedMotion(true);
    const third = render(<RankCrest rank={JLORIOUS} size="large" />);
    crest = crestNamed(rankWords(JLORIOUS));
    expect(crest.querySelector(".rank-crest__shimmer")).toBeNull();
    expect(crest).toHaveAttribute("data-motion", "reduced");
    third.unmount();

    // The settings panel's switch.
    setReducedMotion(false);
    __resetSettingsForTests();
    writeSettings({ reduceMotion: true });
    render(<RankCrest rank={JLORIOUS} size="large" />);
    crest = crestNamed(rankWords(JLORIOUS));
    expect(crest.querySelector(".rank-crest__shimmer")).toBeNull();
    expect(crest).toHaveAttribute("data-motion", "reduced");
  });

  it("#1650 the stylesheet stops the shimmer under prefers-reduced-motion", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "rankCrest.css"), "utf8").replace(/\s+/g, " ");
    expect(css).toContain("@media (prefers-reduced-motion: reduce) { .rank-crest__shimmer { animation: none; } }");
  });

  it("#1650 comes in two named sizes, and draws a tier alone for a ladder heading", () => {
    expect(CREST_SIZE_PX.small).toBeLessThan(CREST_SIZE_PX.large);
    for (const size of ["small", "large"] as const) {
      const { unmount } = render(<RankCrest rank={RAISIN} size={size} />);
      const crest = crestNamed(rankWords(RAISIN));
      expect(crest).toHaveAttribute("data-size", size);
      expect(crest.querySelector("svg")).toHaveAttribute("width", String(CREST_SIZE_PX[size]));
      expect(crest.querySelector("svg")).toHaveAttribute("height", String(CREST_SIZE_PX[size]));
      unmount();
    }

    const heading = render(<RankCrest tier="golden" size="small" />);
    expect(crestNamed("Golden Grape").querySelectorAll(".rank-crest__stud")).toHaveLength(0);
    heading.unmount();

    render(<RankCrest tier="golden" division={1} size="small" />);
    expect(crestNamed("Golden Grape I").querySelectorAll(".rank-crest__stud")).toHaveLength(3);
  });
});
