// Which lines of a long game log to mount (#4614). The log keeps the whole game (R745,
// up to LOG_HISTORY_LIMIT lines), too many to mount when every arrival lays the list out
// again, so a virtual log mounts only the rows near what the player reads, with an overscan
// margin, and leaves the rest's height to a head and a tail spacer, so the scrollbar and every
// line's place are those of the whole log. `useLogWindow.ts` reads these as the log scrolls.
// Log lines vary in height (wrapped rich text, turn dividers), so each row's height is measured
// where the pool's grid is uniform (`deckbuilder/poolWindow.ts`, whose arithmetic is not reused
// here). Nothing here is a rule (CLAUDE.md rule 7): it is where lines are drawn, never which.

/** The mounted rows, `first` to `last` inclusive. Empty where `last` is below `first`. */
export type LogWindow = { first: number; last: number };

function clamp(value: number, low: number, high: number): number {
  return Math.min(Math.max(value, low), high);
}

/**
 * The rows to mount for `count` lines when the viewport shows `scrollTop` to
 * `scrollTop + viewport` of the list's content, `heightAt` giving each row's height in
 * pixels (a divider before the row included). The visible range grows by `overscanRows`
 * rows on each side, clamped to the log, so a scroll brings mounted rows into view.
 */
export function logWindow(
  count: number,
  heightAt: (index: number) => number,
  scrollTop: number,
  viewport: number,
  overscanRows: number,
): LogWindow {
  if (count <= 0) return { first: 0, last: -1 };
  const bottom = scrollTop + Math.max(viewport, 0);
  let first = count;
  let last = -1;
  let offset = 0;
  for (let index = 0; index < count; index += 1) {
    const start = offset;
    offset += Math.max(heightAt(index), 0);
    if (offset > scrollTop && start < bottom) {
      if (index < first) first = index;
      last = index;
    } else if (start >= bottom) break;
  }
  if (last < 0) {
    // The view is past either end: hold the nearest edge with its overscan, so the first
    // rows, or the last, are always mounted.
    const reach = Math.max(Math.floor(overscanRows), 0);
    if (scrollTop >= offset - Math.max(heightAt(count - 1), 0)) {
      return { first: clamp(count - 1 - reach, 0, count - 1), last: count - 1 };
    }
    return { first: 0, last: clamp(reach, 0, count - 1) };
  }
  return {
    first: clamp(first - Math.max(Math.floor(overscanRows), 0), 0, count - 1),
    last: clamp(last + Math.max(Math.floor(overscanRows), 0), 0, count - 1),
  };
}

/** The blank space the unmounted head and tail keep, in pixels, so the scrollbar is the whole log's. */
export function logSpacers(
  count: number,
  heightAt: (index: number) => number,
  window: LogWindow,
): { head: number; tail: number } {
  const first = clamp(window.first, 0, count);
  const last = clamp(window.last, -1, count - 1);
  let head = 0;
  for (let index = 0; index < first; index += 1) head += Math.max(heightAt(index), 0);
  let tail = 0;
  for (let index = last + 1; index < count; index += 1) tail += Math.max(heightAt(index), 0);
  return { head, tail };
}

/** `mounted` held to the lines `count` holds, for a log that shrank since it was made. */
export function fitLogWindow(mounted: LogWindow, count: number): LogWindow {
  if (count <= 0) return { first: 0, last: -1 };
  return {
    first: clamp(mounted.first, 0, count - 1),
    last: clamp(mounted.last, 0, count - 1),
  };
}

export function sameLogWindow(before: LogWindow, after: LogWindow): boolean {
  return before.first === after.first && before.last === after.last;
}

/** What the list lays out: null where it lays out none (no layout engine, as in a test's jsdom). */
export type LogViewport = { scrollTop: number; clientHeight: number };

export function readLogViewport(list: HTMLElement): LogViewport | null {
  const clientHeight = list.clientHeight;
  if (!(clientHeight > 0)) return null;
  return { scrollTop: list.scrollTop, clientHeight };
}
