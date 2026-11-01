// Coach placement (R2434003): the floating bubble docks to a corner of the viewport, never the middle
// of the board, and never over its anchor or the HUD when a corner is clear of them. The ring, not the
// bubble, points at the target. Phones use Coach.tsx's page panel: their boards leave no safe floating position.

export type Rect = { left: number; top: number; width: number; height: number };
export type Size = { width: number; height: number };

export type BubbleSide = "dock-bottom-right" | "dock-bottom-left" | "dock-top-right" | "dock-top-left";

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
  insetTop: number;
  avoid?: readonly Rect[];
  /** Play targets take priority over `avoid`. */
  keepClear?: readonly Rect[];
  margin: number;
  minHeight: number;
};

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

type Spot = { side: BubbleSide; left: number; top: number; maxHeight: number | null };

/** The four corners, under the HUD: the bottom ones first, beside the heroes' rows, the right (over the log) before the left. */
function spots(input: PlaceInput): Spot[] {
  const { viewport, bubble, margin, minHeight } = input;
  const topEdge = input.insetTop + margin;
  const bottomEdge = viewport.height - margin;
  const room = bottomEdge - topEdge;
  const height = Math.min(bubble.height, Math.max(minHeight, room));
  const maxHeight = bubble.height > room ? Math.max(minHeight, room) : null;
  const right = Math.max(margin, viewport.width - margin - bubble.width);
  const bottom = Math.max(topEdge, bottomEdge - height);
  return [
    { side: "dock-bottom-right", left: right, top: bottom, maxHeight },
    { side: "dock-bottom-left", left: margin, top: bottom, maxHeight },
    { side: "dock-top-right", left: right, top: topEdge, maxHeight },
    { side: "dock-top-left", left: margin, top: topEdge, maxHeight },
  ];
}

/** The anchor first, then `keepClear`, then `avoid` decide among the corners; a tie keeps the earlier corner. */
export function placeBubble(input: PlaceInput): BubblePlacement {
  const { bubble } = input;
  const covers = (spot: Spot, rects: readonly Rect[]): number => {
    const height = spot.maxHeight === null ? bubble.height : Math.min(bubble.height, spot.maxHeight);
    return rects.reduce((sum, rect) => sum + overlapArea({ left: spot.left, top: spot.top, width: bubble.width, height }, rect), 0);
  };
  let best: Spot | undefined;
  let score: [number, number, number] = [Infinity, Infinity, Infinity];
  for (const spot of spots(input)) {
    const next: [number, number, number] = [
      covers(spot, input.anchor === null ? [] : [input.anchor]),
      covers(spot, input.keepClear ?? []),
      covers(spot, input.avoid ?? []),
    ];
    if (next[0] < score[0] || (next[0] === score[0] && (next[1] < score[1] || (next[1] === score[1] && next[2] < score[2])))) {
      best = spot;
      score = next;
    }
    if (next[0] === 0 && next[1] === 0 && next[2] === 0) break;
  }
  const chosen = best ?? spots(input)[0];
  if (chosen === undefined) throw new Error("no spot");
  return { side: chosen.side, left: chosen.left, top: chosen.top, maxHeight: chosen.maxHeight };
}
