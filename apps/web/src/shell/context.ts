// What the menus' shell (#3728, `MenuShell.tsx`) hands the screens inside it: the status row the top bar
// draws under itself (R3728002). Outside the shell (a board, the series screen, a test that renders a
// screen alone) there is none, and the screen draws what it drew before.

import { createContext, useContext, type ReactNode } from "react";

export type MenuShellValue = {
  /** R3728002: the live game to go back to, else this tab's queue search, else nothing. */
  status: ReactNode;
};

export const MenuShellContext = createContext<MenuShellValue | null>(null);

/** The shell around this screen, or null outside one. */
export function useMenuShell(): MenuShellValue | null {
  return useContext(MenuShellContext);
}
