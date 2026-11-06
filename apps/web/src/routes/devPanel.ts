// R3156000 (#3156): where the front screen offers the Dev tools. The Dev button shows on a computer
// alone: any window but a phone's or a tablet's, which the game draws apart already (board.css,
// emotes.css, activate.css: `(max-width: 1024px), (pointer: coarse)`), a narrow window or a touch
// screen's main pointer. A window with no pointer at all (a test runner's) is no touch screen, so it
// counts as a computer.

import { BREAKPOINT_PX, COARSE_POINTER_QUERY, upTo } from "../breakpoints.ts";

/** The narrowest window, in CSS pixels, that shows the Dev button: the game's own desktop line (`HERO_DESKTOP_QUERY`). */
export const DEV_PANEL_MIN_WIDTH_PX = BREAKPOINT_PX.tablet + 1;

/** A phone's or a tablet's window, which shows no Dev button: narrower than the line, or a coarse main pointer. */
export const DEV_PANEL_HIDDEN_QUERY = `${upTo("tablet")}, ${COARSE_POINTER_QUERY}`;
