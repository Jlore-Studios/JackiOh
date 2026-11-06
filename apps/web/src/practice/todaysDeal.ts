// Today's deal (SPEC §9.9, R2882000): one random practice deal a UTC day, the same on every device. Page-safe: it never imports driver.ts and decides no rule (CLAUDE.md rule 7).

import type { Difficulty } from "@jackioh/engine/config";
import type { LeanPick, PlayerId } from "@jackioh/shared";

import { DAILY_DEAL_DAY_MS, DAILY_DEAL_SEED_PREFIX } from "./config.ts";
import type { PracticeStartConfig } from "./protocol.ts";

/** A day's deal: its UTC day, the seed every device deals it on, and the seat the human plays. */
export type DailyDeal = { day: string; seed: string; humanSeat: PlayerId };

/** R2882000: the deal of the UTC day `nowMs` falls in, new at UTC midnight; the human is p1 on an even UTC day counted from 1970-01-01 and p2 on an odd one, so the seat alternates every day, month ends included. */
export function dailyDealOf(nowMs: number): DailyDeal {
  const iso = new Date(nowMs).toISOString();
  const day = iso.slice(0, iso.indexOf("T"));
  return { day, seed: `${DAILY_DEAL_SEED_PREFIX}${day}`, humanSeat: Math.floor(nowMs / DAILY_DEAL_DAY_MS) % 2 === 0 ? "p1" : "p2" };
}

/** R2882000: the game a day's deal starts: a random deck on the setup's "More cards from" pick, at `difficulty`. */
export function dailyDealConfig(nowMs: number, difficulty: Difficulty, lean: LeanPick | null): PracticeStartConfig {
  const deal = dailyDealOf(nowMs);
  return {
    seed: deal.seed,
    difficulty,
    humanSeat: deal.humanSeat,
    deck: lean === null ? { kind: "random" } : { kind: "random", leanSet: lean },
    dailyDeal: deal.day,
  };
}
