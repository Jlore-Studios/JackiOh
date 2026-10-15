// What a route hangs on the Duos board beyond S8's frozen `DuosBoardProps`: its way back, its
// connection line, its clock and the teammate's socket (B37). The networked route (`DuosRoute.tsx`)
// and the dev hotseat (`DevDuosRoute.tsx`) provide different chrome around the same board, so it
// travels by context and the props stay S8's. Presentation only (CLAUDE.md rule 7).

import { createContext, type ReactNode } from "react";

export type DuosChrome = {
  /** The way off the board, first on its rail. */
  back?: ReactNode;
  /** The route's connection line, on the rail (Game's `matchStatus`). */
  status?: ReactNode;
  /** The round clock, in the End turn housing (Game's `turnClock`). */
  clock?: ReactNode;
  /** B37: the teammate's socket as the server last reported it; null before it has. */
  teammate?: { connected: boolean; aiPlaying: boolean } | null;
  /** Where the result panel's "back" goes. */
  onLeave?: () => void;
};

export const DuosChromeContext = createContext<DuosChrome | null>(null);
