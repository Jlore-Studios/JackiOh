// R3156000 (#3156): where the front screen offers the Dev tools. The Dev button shows on a computer
// alone, a window at least this wide whose main pointer is a mouse or a trackpad, and opens the
// tools in a dialog laid out for that screen. The width is the game's own desktop line
// (`HERO_DESKTOP_QUERY`, game/config.ts), so a tablet held either way and every phone get no button.

/** The narrowest window, in CSS pixels, that shows the Dev button. */
export const DEV_PANEL_MIN_WIDTH_PX = 1025;

/** The media query the Dev button follows: a wide window and a fine main pointer. */
export const DEV_PANEL_QUERY = `(min-width: ${String(DEV_PANEL_MIN_WIDTH_PX)}px) and (pointer: fine)`;
