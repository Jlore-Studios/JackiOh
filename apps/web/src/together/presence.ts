// The presence heartbeat (#936, B47, R3210000): the server counts every signed-in request as presence,
// so friends see the player online for `PRESENCE_TTL_MS` after the last one. While the client holds a
// session, `POST /api/presence` goes only once `PRESENCE_HEARTBEAT_MS` passes with no signed-in request
// (`onSignedInRequest`): a busy tab sends no beat, and a hidden tab whose polls pause (R3205000) sends
// one a heartbeat. None goes at mount, as the shell's social poll reads at once.
// Mounted once in the shell (`main.tsx`), so every screen keeps it. What "online" means is the
// server's (CLAUDE.md rule 7). A session the server refuses outright is not asked again until the tab
// holds another one; any other failure is the next beat's to retry.

import { useEffect } from "react";

import { PRESENCE_HEARTBEAT_MS } from "@jackioh/server-config";

import { onSignedInRequest } from "../net/api.ts";
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
    let timer: number | null = null;
    // R3210000: every signed-in request, the beat's own among them, puts the next beat a full heartbeat later.
    const rearm = (): void => {
      if (timer !== null) window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        timer = null;
        beat();
        rearm();
      }, PRESENCE_HEARTBEAT_MS);
    };
    const stopHearing = onSignedInRequest(rearm);
    rearm();
    return () => {
      stopHearing();
      if (timer !== null) window.clearTimeout(timer);
    };
  }, [active]);
}
