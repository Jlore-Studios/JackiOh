// Player statistics sync with the account (SPEC §9.11, R640).
//
// When an active session is present, syncs the device's tracked player statistics
// and privacy preference to PUT /api/stats/player.

import { putPlayerStats } from "../net/api.ts";
import { readSession } from "../net/session.ts";
import { readSettings } from "../settings/store.ts";
import { readPlayerStats } from "./store.ts";

/**
 * Sends the device's player stats and privacy preference to the server.
 * Safe to call frequently: if not signed in or network is unreachable, it silently completes.
 */
export async function syncPlayerStats(tokenOverride?: string): Promise<void> {
  const token = tokenOverride ?? readSession()?.accessToken;
  if (!token) return;

  const stats = readPlayerStats();
  const settings = readSettings();
  const isPrivate = !settings.publicStats;

  try {
    await putPlayerStats(token, {
      stats: stats as unknown as Record<string, unknown>,
      isPrivate,
    });
  } catch {
    // Network or server unreachable: device retains authoritative local stats
  }
}
