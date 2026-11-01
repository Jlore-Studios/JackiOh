// A dungeon run's fight screen's `data-testid` vocabulary (DR14 part E, #2706): every id is
// prefixed `dungeon-`, as each part of the dungeon run keeps its own in its directory.

/** Whose relics a tray holds: the player's (beside their hero) or the enemy's (beside its hero). */
export type TraySide = "own" | "enemy";

export const fightTestid = {
  /** The fight's board, once the game has started. */
  screen: "dungeon-fight",
  /** The deal on its way; role="status". */
  loading: "dungeon-fight-loading",
  /** The game stopped (the worker refused it or failed); role="alert". */
  failed: "dungeon-fight-failed",
  /** Under `failed`: deal the fight again from its seed. */
  retry: "dungeon-fight-retry",
  /** The run stands on no fight the run can set up; role="alert". */
  noSetup: "dungeon-fight-no-setup",
  /** B22: the enemy's name, on the board's rail. */
  enemy: "dungeon-fight-enemy",
  /** B24: the run's time, on the board's rail. */
  timer: "dungeon-fight-timer",
  /** B25: in the result panel, sends the fight's `FightOver` to the run once the worker reported it. */
  continue: "dungeon-fight-continue",
  /** B25: the run's refusal of that `FightOver`; role="alert". */
  actRefusal: "dungeon-fight-act-refusal",
  /** B25: the worker could not report the fight; role="alert". */
  reportError: "dungeon-fight-report-error",
  /** Beside `reportError`: ask the worker again. */
  reportRetry: "dungeon-fight-report-retry",
  /** B23: a side's tray of relics and blessings, beside its hero. */
  tray: (side: TraySide): string => `dungeon-tray-${side}`,
  /** B23: the tray's button, which opens its list. */
  trayOpen: (side: TraySide): string => `dungeon-tray-open-${side}`,
  /** B23: the list the button opens. */
  trayList: (side: TraySide): string => `dungeon-tray-list-${side}`,
  /** B23: one relic's or blessing's icon, `data-tier` its tier ("none" for a blessing or an enemy's). */
  trayIcon: (side: TraySide, id: string): string => `dungeon-tray-icon-${side}-${id}`,
  /** B23: one relic's or blessing's entry in the open list. */
  trayItem: (side: TraySide, id: string): string => `dungeon-tray-item-${side}-${id}`,
} as const;
