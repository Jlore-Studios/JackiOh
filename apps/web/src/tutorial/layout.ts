// Coach placement never covers its anchor or the HUD. Prefer clear sides, protecting `keepClear`
// over `avoid`; dock to the roomier edge only when none fit. Phones use Coach.tsx's page panel:
// their boards leave no safe floating position.

export type Rect = { left: number; top: number; width: number; height: number };
export type Size = { width: number; height: number };

export type BubbleSide = "below" | "above" | "right" | "left" | "center" | "corner" | "dock-top" | "dock-bottom";

export type BubblePlacement = {
  side: BubbleSide;
  left: number;
  top: number;
  maxHeight: number | null;
};

export type PlaceInput = {
  anchor: Rect | null;
  bubble: Size;
  viewport: Size;
  slim: boolean;
  insetTop: number;
  avoid?: readonly Rect[];
  /** Play targets take priority over `avoid`. */
  keepClear?: readonly Rect[];
  gap: number;
  margin: number;
  minHeight: number;
};

function clamp(value: number, low: number, high: number): number {
  return high < low ? low : Math.min(Math.max(value, low), high);
}

export function overlapArea(a: Rect, b: Rect): number {
  const width = Math.min(a.left + a.width, b.left + b.width) - Math.max(a.left, b.left);
  const height = Math.min(a.top + a.height, b.top + b.height) - Math.max(a.top, b.top);
  return width > 0 && height > 0 ? width * height : 0;
}

export function unionRect(rects: readonly Rect[]): Rect | null {
  if (rects.length === 0) return null;
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const rect of rects) {
    left = Math.min(left, rect.left);
    top = Math.min(top, rect.top);
    right = Math.max(right, rect.left + rect.width);
    bottom = Math.max(bottom, rect.top + rect.height);
  }
  return { left, top, width: right - left, height: bottom - top };
}

export function padRect(rect: Rect, pad: number): Rect {
  return { left: rect.left - pad, top: rect.top - pad, width: rect.width + 2 * pad, height: rect.height + 2 * pad };
}

function dock(input: PlaceInput, anchor: Rect): BubblePlacement {
  const { viewport, bubble, margin, gap, minHeight } = input;
  const topEdge = input.insetTop + margin;
  const bottomEdge = viewport.height - margin;
  const centreX = anchor.left + anchor.width / 2;
  const left = clamp(centreX - bubble.width / 2, margin, viewport.width - margin - bubble.width);

  const roomAbove = anchor.top - gap - topEdge;
  const roomBelow = bottomEdge - (anchor.top + anchor.height + gap);
  if (roomAbove >= roomBelow) {
    return { side: "dock-top", left, top: topEdge, maxHeight: Math.max(minHeight, roomAbove) };
  }
  const maxHeight = Math.max(minHeight, roomBelow);
  const height = Math.min(bubble.height, maxHeight);
  return { side: "dock-bottom", left, top: bottomEdge - height, maxHeight };
}

type Candidate = { side: BubbleSide; left: number; top: number; fits: boolean };

function candidates(input: PlaceInput, anchor: Rect): Candidate[] {
  const { viewport, bubble, gap, margin } = input;
  const topEdge = input.insetTop + margin;
  const bottomEdge = viewport.height - margin;
  const right = anchor.left + anchor.width;
  const bottom = anchor.top + anchor.height;
  const centreX = anchor.left + anchor.width / 2;
  const centreY = anchor.top + anchor.height / 2;
  const alongX = clamp(centreX - bubble.width / 2, margin, viewport.width - margin - bubble.width);
  const alongY = clamp(centreY - bubble.height / 2, topEdge, bottomEdge - bubble.height);

  const below: Candidate = { side: "below", left: alongX, top: bottom + gap, fits: bottom + gap + bubble.height <= bottomEdge };
  const above: Candidate = {
    side: "above",
    left: alongX,
    top: anchor.top - gap - bubble.height,
    fits: anchor.top - gap - bubble.height >= topEdge,
  };
  const toRight: Candidate = {
    side: "right",
    left: right + gap,
    top: alongY,
    fits: right + gap + bubble.width <= viewport.width - margin && bubble.height <= bottomEdge - topEdge,
  };
  const toLeft: Candidate = {
    side: "left",
    left: anchor.left - gap - bubble.width,
    top: alongY,
    fits: anchor.left - gap - bubble.width >= margin && bubble.height <= bottomEdge - topEdge,
  };

  // Try the roomier side first.
  const vertical = centreY > (topEdge + bottomEdge) / 2 ? [above, below] : [below, above];
  const horizontal = viewport.width - right >= anchor.left ? [toRight, toLeft] : [toLeft, toRight];
  return [...vertical, ...horizontal];
}

export function placeBubble(input: PlaceInput): BubblePlacement {
  const { anchor, bubble, viewport, margin } = input;

  if (anchor === null) {
    if (input.slim) {
      return {
        side: "corner",
        left: Math.max(margin, viewport.width - margin - bubble.width),
        top: Math.max(input.insetTop + margin, viewport.height - margin - bubble.height),
        maxHeight: null,
      };
    }
    const centre: BubblePlacement = {
      side: "center",
      left: Math.max(margin, (viewport.width - bubble.width) / 2),
      top: Math.max(input.insetTop + margin, (viewport.height - bubble.height) / 2),
      maxHeight: null,
    };
    // Prompts open in the middle, so step beside one.
    const blocked = [...(input.keepClear ?? []), ...(input.avoid ?? [])].find(
      (rect) => overlapArea({ left: centre.left, top: centre.top, ...bubble }, rect) > 0,
    );
    return blocked === undefined ? centre : placeBubble({ ...input, anchor: blocked, avoid: [], keepClear: [] });
  }

  const fitting = candidates(input, anchor).filter((candidate) => candidate.fits);
  if (fitting.length === 0) return dock(input, anchor);
  const covers = (candidate: Candidate, rects: readonly Rect[]): number =>
    rects.reduce(
      (sum, rect) => sum + overlapArea({ left: candidate.left, top: candidate.top, ...bubble }, rect),
      0,
    );
  // When all sides cover something, protect `keepClear` before soft obstacles (e2e spec 22).
  let best: Candidate | undefined;
  let leastClear = Infinity;
  let leastSoft = Infinity;
  for (const candidate of fitting) {
    const clear = covers(candidate, input.keepClear ?? []);
    const soft = covers(candidate, input.avoid ?? []);
    if (clear < leastClear || (clear === leastClear && soft < leastSoft)) {
      best = candidate;
      leastClear = clear;
      leastSoft = soft;
    }
    if (clear === 0 && soft === 0) break;
  }
  if (best === undefined) return dock(input, anchor);
  return { side: best.side, left: best.left, top: best.top, maxHeight: null };
}
