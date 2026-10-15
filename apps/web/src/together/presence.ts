// The presence heartbeat (#936, B47): while the client holds a session, `POST /api/presence` now and
// every `PRESENCE_HEARTBEAT_MS`, so friends see the player online for `PRESENCE_TTL_MS` after each.
// Mounted once in the shell (`main.tsx`), so every screen keeps it, a match and practice included.
//
// What "online" means is the server's (CLAUDE.md rule 7); this only says the client is open. A session
// the server refuses outright (signed out, or an account that is not active) is not asked again until
// the tab holds another one; any other failure is the next beat's to retry.

import { useEffect } from "react";

import { PRESENCE_HEARTBEAT_MS } from "@jackioh/server-config";

import { heartbeat } from "../net/social.ts";
import { readSession } from "../net/session.ts";
import { isRefusedSession } from "./session.ts";

export function usePresenceHeartbeat(active: boolean): void {
  useEffect(() => {
    if (!active) return;
    let refused: string | null = null;
    const beat = (): void => {
      const token = readSession()?.accessToken ?? null;
      if (token === null || token === refused) return;
      heartbeat().catch((cause: unknown) => {
        if (isRefusedSession(cause)) refused = token;
      });
    };
    beat();
    const handle = window.setInterval(beat, PRESENCE_HEARTBEAT_MS);
    return () => {
      window.clearInterval(handle);
    };
  }, [active]);
}
