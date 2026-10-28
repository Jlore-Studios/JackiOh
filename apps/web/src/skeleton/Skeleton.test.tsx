// #2115, R2115000: the skeleton. Four variants of aria-hidden blocks over a status line, the words kept,
// and a shimmer that stops under reduced motion.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { setReducedMotion } from "../test/setup.ts";
import { expectSkeleton } from "../test/skeleton.ts";
import {
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
});
