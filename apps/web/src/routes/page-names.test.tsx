// R3230000: one name per page across menu, footer and document title.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { screenNameFor } from "../net/head.ts";
import { paths } from "../net/navigate.ts";
import { commandActions, commandPages } from "./commandEntries.ts";
import { SiteFooter, siteFooterTestid } from "./SiteFooter.tsx";

afterEach(() => {
  cleanup();
});

describe("R3230000 one name per page across menu, footer and document title", () => {
  it("R3230000 menu, footer and screenNameFor agree for each page", () => {
    render(<SiteFooter />);

    const menu = [...commandActions(), ...commandPages()];
    const labelForPath = (path: string): string | undefined => menu.find((entry) => entry.path === path)?.label;

    const cases: {
      path: string;
      expected: string;
      footerTestId?: string;
    }[] = [
      { path: paths.almanac, expected: "Almanac", footerTestId: siteFooterTestid.almanac },
      { path: paths.stats, expected: "Statistics", footerTestId: siteFooterTestid.stats },
      { path: paths.privacy, expected: "Privacy policy", footerTestId: siteFooterTestid.privacy },
      { path: paths.terms, expected: "Terms of service", footerTestId: siteFooterTestid.terms },
      { path: paths.ai, expected: "How JackiOh uses AI", footerTestId: siteFooterTestid.ai },
      { path: paths.dungeon, expected: "Dungeon run" },
      { path: paths.play, expected: "Play" },
      { path: paths.account, expected: "Account" },
    ];

    for (const { path, expected, footerTestId } of cases) {
      expect(screenNameFor(path), `screenNameFor(${path})`).toBe(expected);

      if (footerTestId !== undefined) {
        expect(screen.getByTestId(footerTestId).textContent, `footer text for ${path}`).toBe(expected);
      }

      const menuLabel = labelForPath(path);
      if (menuLabel !== undefined) {
        expect(menuLabel, `command menu label for ${path}`).toBe(expected);
      }
    }
  });
});
