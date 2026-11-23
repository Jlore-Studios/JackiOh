// The menus' shell's chrome (#3728, R3728000–R3728002, R3729001): no imports, so e2e's support code and specs read
// it as they read `auth/testids.ts`.

export const menuShellTestid = {
  /** Around every menu screen; `data-board` while a board is on screen. */
  root: "menu-shell",
  /** The rail, or a phone's tab bar: Home, Play, Collection, Shop, Friends. */
  nav: "menu-shell-nav",
  home: "menu-shell-home",
  play: "menu-shell-play",
  collection: "menu-shell-collection",
  shop: "menu-shell-shop",
  /** Friends as a link to Play together, while no friends panel can open (signed out). */
  friends: "menu-shell-friends",
  /** R3728002: this tab's queue search, in the status row on every menu screen (`data-mode`, R3729001). */
  queue: "menu-shell-queue",
  /** In the queue banner: Cancel. */
  queueLeave: "menu-shell-queue-leave",
  /** R3729001: in the queue banner, the time waited as m:ss. */
  queueClock: "menu-shell-queue-clock",
  /** R3729001: in the queue banner, the expected wait for the mode, once the server has counted one. */
  queueWait: "menu-shell-queue-wait",
} as const;
