// An Infinite Assault fight screen's `data-testid` vocabulary (issue #4582): every id is
// prefixed `assault-fight-`, as the fight keeps its own in its directory. The screen itself is
// the frozen `assault-fight`.

import type { AssaultUpgradeKind } from "@jackioh/run";

export const assaultFightTestid = {
  /** The fight's board, once the game has started. */
  screen: "assault-fight",
  /** The deal on its way; role="status". */
  loading: "assault-fight-loading",
  /** The game stopped (the worker refused it or failed); role="alert". */
  failed: "assault-fight-failed",
  /** Under `failed`: deal the fight again from its seed. */
  retry: "assault-fight-retry",
  /** The attempt stands on no fight the Assault can set up; role="alert". */
  noSetup: "assault-fight-no-setup",
  /** R4582002: the enemy's name, on the board's rail. */
  enemy: "assault-fight-enemy",
  /** R4582002: the level's number, beside the enemy's name. */
  level: "assault-fight-level",
  /** R4582000: in the result panel, sends the fight's `fightOver` to the attempt once the worker reported it. */
  continue: "assault-fight-continue",
  /** R4582001: out to the day screen, or the Play menu with no way back: mid-fight it opens `leaveDialog`; once the fight has its result it leaves at once. */
  menu: "assault-fight-menu",
  /** R4582001: the question asked before a fight in progress is left. */
  leaveDialog: "assault-fight-leave",
  /** R4582001: stay in the fight. */
  leaveStay: "assault-fight-leave-stay",
  /** R4582001: leave the fight; the held move goes. */
  leaveConfirm: "assault-fight-leave-confirm",
  /** R4582000: the worker could not report the fight; role="alert". */
  reportError: "assault-fight-report-error",
  /** Beside `reportError`: ask the worker again. */
  reportRetry: "assault-fight-report-retry",
  /** R4582002: the enemy's upgrades, beside its hero. */
  upgrades: "assault-fight-upgrades",
  /** R4582002: the tray's button, which opens its list. */
  upgradesOpen: "assault-fight-upgrades-open",
  /** R4582002: the list the button opens. */
  upgradesList: "assault-fight-upgrades-list",
  /** R4582002: one upgrade's entry in the open list. */
  upgrade: (kind: AssaultUpgradeKind): string => `assault-fight-upgrade-${kind}`,
} as const;
