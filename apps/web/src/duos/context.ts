// What a route hangs on the Duos board beyond S8's frozen `DuosBoardProps`. The networked route
// (`DuosRoute.tsx`) and the dev hotseat (`DevDuosRoute.tsx`) provide different chrome around the same
// board, so it travels by context and the props stay S8's. Presentation only (CLAUDE.md rule 7).

import { createContext, type ReactNode } from "react";

import type { DuosAgainStatus } from "../net/duos.ts";

export type DuosPlayAgain = {
  /** R2092000: the route's last status read, null before the first one lands. */
  status: DuosAgainStatus | null;
  offering: boolean;
  /** R2092000: the last offer's refusal, in the server's words, null when none. */
  refusal: string | null;
  offer: () => void;
};

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
  /** #2116: the route `onLeave` goes to, so the button names it for a prefetch (R2116000); absent, none. */
  leaveRoute?: string;
  /** #965: the result panel's "Watch the replay" (§13.9); absent, it offers none (/dev/duos has no server replay). */
  onWatchReplay?: () => void;
  /** R2092000: the result panel's Play again; absent, it offers none. */
  playAgain?: DuosPlayAgain;
};

export const DuosChromeContext = createContext<DuosChrome | null>(null);
