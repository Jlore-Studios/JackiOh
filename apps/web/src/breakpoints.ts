// #3227: the client's named breakpoints; breakpoints.test.ts holds every sheet's `@media` and every script's
// query to them or to its KEEP. Each is the widest (or tallest) window of the smaller layout: `(max-width: N)`
// holds up to N and the layout above starts at `(min-width: N + 1)`, so no width matches both.

/** In CSS px. Wider than `tablet` is the desktop. */
export const BREAKPOINT_PX = {
  /** A phone held upright: the touch rule's and the top bar's (R2122000, R3218000). */
  phone: 600,
  /** A small tablet held upright: the board's narrow cards, the deck builder's one column. */
  narrow: 760,
  /** A tablet held upright: the menus' two columns, and a landscape tablet's desktop board, start at 900. */
  medium: 899,
  /** A tablet either way up: the desktop layout starts at 1025. */
  tablet: 1024,
  /** A phone on its side: at most this tall. */
  short: 500,
} as const;

export type Breakpoint = keyof typeof BREAKPOINT_PX;

const AXIS: Readonly<Record<Breakpoint, "width" | "height">> = { phone: "width", narrow: "width", medium: "width", tablet: "width", short: "height" };

/** At or under `name`: `(max-width: 600px)`. */
export function upTo(name: Breakpoint): string {
  return `(max-${AXIS[name]}: ${String(BREAKPOINT_PX[name])}px)`;
}

/** Past `name`: `(min-width: 601px)`. */
export function over(name: Breakpoint): string {
  return `(min-${AXIS[name]}: ${String(BREAKPOINT_PX[name] + 1)}px)`;
}

/** The touch query: a coarse main pointer. */
export const COARSE_POINTER_QUERY = "(pointer: coarse)";

/** A phone either way up: the board's match bar, the coach's dock, the craft editor's sheet. */
export const PHONE_SHEET_QUERY = `${upTo("phone")}, (orientation: landscape) and ${upTo("short")}`;
