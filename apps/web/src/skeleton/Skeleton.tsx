// #2115, R2115000: the shape of what a menu panel waits for, drawn in place of its "Loading …" line.
// The words stay as a status line a screen reader hears; the blocks are aria-hidden, and their shimmer
// stops under Reduce motion (skeleton.css). Presentation only (CLAUDE.md rule 7): a fixed number of
// blank blocks, never a count from the read. The blocks are brass (#2108, R2108000).

import { useEffect, useState, type CSSProperties, type ReactElement, type ReactNode } from "react";

import { useOsReducedMotion } from "../game/useOsReducedMotion.ts";
import { useSetting } from "../settings/store.ts";

import "./skeleton.css";

export type SkeletonVariant = "grid" | "rows" | "table" | "panel";

/** Card tiles: two rows at most in a phone's region, so a skeleton fits one screen (R1581000). */
export const SKELETON_TILES = 8;
export const SKELETON_ROWS = 4;
/** Table rows under the header row, and the cells of each. */
export const SKELETON_TABLE_ROWS = 5;
export const SKELETON_TABLE_COLUMNS = 4;
/** A panel's lines under its heading bar. */
export const SKELETON_PANEL_LINES = 2;

/** #2108, R2108000: how long a wait runs before it shows, so a read that answers sooner never flashes a skeleton. */
export const SKELETON_DELAY_MS = 300;

/** #2108, R2108000: true once `SKELETON_DELAY_MS` has passed since mounting. A timer, not a CSS delay: Reduce motion zeroes every CSS delay (index.css). */
export function useShownAfterDelay(): boolean {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const timer = window.setTimeout(() => {
      setShown(true);
    }, SKELETON_DELAY_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, []);
  return shown;
}

export const skeletonTestid = { root: "skeleton", status: "skeleton-status", blocks: "skeleton-blocks" } as const;

export type SkeletonProps = {
  /** What is loading, in the player's words, already through `t()`. */
  label: string;
  variant: SkeletonVariant;
  /** The status line's testid: the one the panel's loading line had. */
  testId?: string;
};

function times(count: number): number[] {
  return Array.from({ length: count }, (_, index) => index);
}

function tableRow(key: string): ReactElement {
  return (
    <span key={key} className="skeleton__table-row">
      {times(SKELETON_TABLE_COLUMNS).map((cell) => (
        <span key={cell} className="skeleton__block skeleton__cell" />
      ))}
    </span>
  );
}

function blocksOf(variant: SkeletonVariant): ReactNode {
  switch (variant) {
    case "grid":
      return times(SKELETON_TILES).map((tile) => <span key={tile} className="skeleton__block skeleton__tile" />);
    case "rows":
      return times(SKELETON_ROWS).map((row) => (
        <span key={row} className="skeleton__row">
          <span className="skeleton__block skeleton__mark" />
          <span className="skeleton__block skeleton__line" />
        </span>
      ));
    case "table":
      return [tableRow("head"), ...times(SKELETON_TABLE_ROWS).map((row) => tableRow(String(row)))];
    case "panel":
      return [
        <span key="title" className="skeleton__block skeleton__title" />,
        ...times(SKELETON_PANEL_LINES).map((line) => <span key={line} className="skeleton__block skeleton__line" />),
      ];
  }
}

export function Skeleton({ label, variant, testId = skeletonTestid.status }: SkeletonProps): ReactElement {
  const panelReduces = useSetting("reduceMotion");
  const osReduces = useOsReducedMotion();
  const reduced = panelReduces || osReduces;
  const shown = useShownAfterDelay();
  return (
    <div
      className="skeleton"
      data-testid={skeletonTestid.root}
      data-variant={variant}
      data-shown={shown ? "true" : "false"}
      data-motion={reduced ? "reduced" : "full"}
      style={{ "--skeleton-columns": SKELETON_TABLE_COLUMNS } as CSSProperties}
    >
      <p className="skeleton__status" role="status" data-testid={testId}>
        {label}
      </p>
      <div className="skeleton__blocks" aria-hidden="true" data-testid={skeletonTestid.blocks}>
        {blocksOf(variant)}
      </div>
    </div>
  );
}
