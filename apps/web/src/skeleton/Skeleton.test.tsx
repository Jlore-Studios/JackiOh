// #2115, R2115000: the skeleton. Four variants of aria-hidden blocks over a status line, the words kept,
// and a shimmer that stops under reduced motion.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import {
  SKELETON_DELAY_MS,
  SKELETON_PANEL_LINES,
  SKELETON_ROWS,
  SKELETON_TABLE_COLUMNS,
  SKELETON_TABLE_ROWS,
  SKELETON_TILES,
  Skeleton,
  skeletonTestid,
  type SkeletonVariant,
} from "./Skeleton.tsx";

const LABEL = "Loading the test…";
const TEST_ID = "test-loading";
const VARIANTS: readonly SkeletonVariant[] = ["grid", "rows", "table", "panel"];

afterEach(() => {
  cleanup();
  setReducedMotion(false);
  window.localStorage.clear();
  __resetSettingsForTests();
});

/** The skeleton on screen, found by its status line. */
function rootOf(status: HTMLElement): HTMLElement {
  const root = status.closest<HTMLElement>(`[data-testid="${skeletonTestid.root}"]`);
  if (root === null) throw new Error("no skeleton around the status line");
  return root;
}

describe("R2115000 the skeleton", () => {
  it("R2115000 draws each variant's blocks under an aria-hidden layer, its words a status line", () => {
    for (const variant of VARIANTS) {
      const { unmount } = render(<Skeleton variant={variant} label={LABEL} testId={TEST_ID} />);
      const status = screen.getByTestId(TEST_ID);
      expect(status).toHaveTextContent(LABEL);
      expectSkeleton(status, variant);
      const root = rootOf(status);
      switch (variant) {
        case "grid":
          expect(root.querySelectorAll(".skeleton__tile")).toHaveLength(SKELETON_TILES);
          break;
        case "rows":
          expect(root.querySelectorAll(".skeleton__row")).toHaveLength(SKELETON_ROWS);
          break;
        case "table": {
          const rows = root.querySelectorAll(".skeleton__table-row");
          expect(rows).toHaveLength(SKELETON_TABLE_ROWS + 1);
          for (const row of rows) expect(row.querySelectorAll(".skeleton__cell")).toHaveLength(SKELETON_TABLE_COLUMNS);
          break;
        }
        case "panel":
          expect(root.querySelectorAll(".skeleton__title")).toHaveLength(1);
          expect(root.querySelectorAll(".skeleton__line")).toHaveLength(SKELETON_PANEL_LINES);
          break;
      }
      unmount();
    }

    render(<Skeleton variant="panel" label={LABEL} />);
    expect(screen.getByTestId(skeletonTestid.status)).toHaveTextContent(LABEL);
  });

  it("R2115000 the blocks stand still under Reduce motion, the OS's or the settings panel's", () => {
    const first = render(<Skeleton variant="rows" label={LABEL} testId={TEST_ID} />);
    expect(rootOf(screen.getByTestId(TEST_ID))).toHaveAttribute("data-motion", "full");
    first.unmount();

    // The OS preference.
    setReducedMotion(true);
    const second = render(<Skeleton variant="rows" label={LABEL} testId={TEST_ID} />);
    expect(rootOf(screen.getByTestId(TEST_ID))).toHaveAttribute("data-motion", "reduced");
    second.unmount();

    // The settings panel's switch.
    setReducedMotion(false);
    __resetSettingsForTests();
    writeSettings({ reduceMotion: true });
    render(<Skeleton variant="rows" label={LABEL} testId={TEST_ID} />);
    expect(rootOf(screen.getByTestId(TEST_ID))).toHaveAttribute("data-motion", "reduced");
  });

  it("R2115000 the stylesheet shimmers only under full motion and keeps the words off screen inside the skeleton", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "skeleton.css"), "utf8")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/\s+/g, " ");
    const animations = [...css.matchAll(/\banimation\s*:/g)];
    expect(animations).toHaveLength(1);
    const full = css.indexOf('@media (prefers-reduced-motion: no-preference) { .skeleton[data-motion="full"] .skeleton__block {');
    expect(full).toBeGreaterThan(-1);
    expect(animations[0]?.index).toBeGreaterThan(full);

    const root = css.slice(css.indexOf(".skeleton {"), css.indexOf("}", css.indexOf(".skeleton {")));
    expect(root).toContain("position: relative;");
    expect(root).toContain("overflow: hidden;");
    const status = css.slice(css.indexOf(".skeleton__status {"), css.indexOf("}", css.indexOf(".skeleton__status {")));
    expect(status).toContain("position: absolute;");
    expect(status).toContain("clip-path: inset(50%);");
  });

  it("R2115000 the blocks' layer draws the JackiOh mark, still, but in a shell panel, and the mark is shipped", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const flat = (path: string): string =>
      readFileSync(path, "utf8")
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .replace(/\s+/g, " ");
    const css = flat(join(here, "skeleton.css"));
    const start = css.indexOf(".skeleton__blocks::before {");
    expect(start).toBeGreaterThan(-1);
    const mark = css.slice(start, css.indexOf("}", start));
    expect(mark).toContain('url("/brand/jackioh-mark.svg")');
    expect(mark).toContain("position: absolute;");
    expect(mark).not.toMatch(/\banimation\b/);
    expect(readFileSync(join(here, "..", "..", "public", "brand", "jackioh-mark.svg"), "utf8")).toContain("<svg");
    // The shell's panels carry the logo over the skeleton, so the mark is not drawn twice.
    expect(flat(join(here, "..", "auth", "tavern.css"))).toContain(".tavern.shell-panel .skeleton__blocks::before { content: none; }");
  });
});

describe("R2108000 the skeleton waits a moment", () => {
  it("R2108000 a read that answers within SKELETON_DELAY_MS never shows the blocks, and the words are there at once", () => {
    try {
      vi.useFakeTimers();
      const { unmount } = render(<Skeleton variant="rows" label={LABEL} testId={TEST_ID} />);
      const status = screen.getByTestId(TEST_ID);
      expect(status).toHaveTextContent(LABEL);
      const root = rootOf(status);
      expect(root).toHaveAttribute("data-shown", "false");
      act(() => {
        vi.advanceTimersByTime(SKELETON_DELAY_MS - 1);
      });
      expect(root).toHaveAttribute("data-shown", "false");
      unmount();
      expect(vi.getTimerCount()).toBe(0);
    } finally {
      vi.useRealTimers();
    }
  });

  it("R2108000 a wait that outlasts SKELETON_DELAY_MS shows its blocks, still under Reduce motion", () => {
    try {
      vi.useFakeTimers();
      setReducedMotion(true);
      render(<Skeleton variant="rows" label={LABEL} testId={TEST_ID} />);
      act(() => {
        vi.advanceTimersByTime(SKELETON_DELAY_MS);
      });
      const root = rootOf(screen.getByTestId(TEST_ID));
      expect(root).toHaveAttribute("data-shown", "true");
      expect(root).toHaveAttribute("data-motion", "reduced");
    } finally {
      vi.useRealTimers();
    }
  });

  it("R2108000 the stylesheet hides the blocks until shown and draws them in the tavern's brass", () => {
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "skeleton.css"), "utf8")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/\s+/g, " ");
    expect(css).toContain('.skeleton[data-shown="false"] .skeleton__blocks { visibility: hidden; }');
    expect(css).toContain("--skeleton-ink: color-mix(in srgb, var(--pal-brass-500) 18%, transparent);");
  });
});
