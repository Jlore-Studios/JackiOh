// The server's answers the dashboard's tests read: one dev account, its ratings, a leaderboard of people
// and bots, news posts (one taken down), an audit trail, a wallet and two held quests.

import type { Route } from "./fakes.ts";
import type {
  AdminMe,
  AuditResponse,
  LeaderboardResponse,
  NewsResponse,
  QuestsResponse,
  RatingResponse,
  WalletResponse,
} from "../types.ts";

export const AT = Date.UTC(2026, 10, 1, 9, 30);

export const ME: AdminMe = { profileId: "p-dev", username: "devname", strict: true, secondFactor: true };

export const RATING: RatingResponse = {
  ladders: [
    {
      ladder: "constructed",
      rating: 1800,
      deviation: 50.123,
      volatility: 0.0599987,
      handSet: true,
      earned: { rating: 1523.456, deviation: 80, volatility: 0.06 },
    },
    { ladder: "random", rating: 1500, deviation: 350, volatility: 0.06, handSet: false, earned: null },
  ],
};

export const CONSTRUCTED: LeaderboardResponse = {
  season: "2026-11",
  ladder: "constructed",
  rows: [
    {
      profileId: "p-dev",
      username: "devname",
      kind: "person",
      rating: 1800,
      games: 0,
      wins: 0,
      losses: 0,
      draws: 0,
      rank: { tier: "golden", division: 2, pips: 3, pipsPerDivision: 5 },
      handSet: true,
      you: true,
      bot: null,
    },
    {
      profileId: "p-bot",
      username: "Gristle",
      kind: "bot",
      rating: 1650.5,
      games: 40,
      wins: 25,
      losses: 14,
      draws: 1,
      rank: { tier: "large", division: 1 },
      handSet: false,
      you: false,
      bot: { strength: 0.75, persona: "aggro", retired: false },
    },
    {
      profileId: "p-old",
      username: "Old Bot",
      kind: "bot",
      rating: 1400,
      games: 12,
      wins: 4,
      losses: 8,
      draws: 0,
      rank: { tier: "jlorious", position: 4 },
      handSet: false,
      you: false,
      bot: { strength: 0.2, persona: "control", retired: true },
    },
    {
      profileId: "p-new",
      username: "newbie",
      kind: "person",
      rating: 1500,
      games: 3,
      wins: 1,
      losses: 2,
      draws: 0,
      rank: { tier: "normal", placementsPlayed: 3, placementGames: 10 },
      handSet: false,
      you: false,
      bot: null,
    },
  ],
};

export const RANDOM: LeaderboardResponse = {
  season: "2026-11",
  ladder: "random",
  rows: [
    {
      profileId: "p-rand",
      username: "shuffler",
      kind: "person",
      rating: 1700,
      games: 9,
      wins: 6,
      losses: 3,
      draws: 0,
      rank: { tier: "raisin" },
      handSet: false,
      you: false,
      bot: null,
    },
  ],
};

export const NEWS: NewsResponse = {
  posts: [
    { id: "n-2", title: "Patch day", summary: "New cards are out.", postedAt: AT, postedBy: "devname", takenDownAt: null },
    {
      id: "n-1",
      title: "<b>Old news</b>",
      summary: "<img src=x onerror=alert(1)> Gone now.",
      postedAt: AT - 1,
      postedBy: "",
      takenDownAt: AT,
    },
  ],
};

export const AUDIT: AuditResponse = {
  actions: [
    {
      id: 7,
      profileId: "p-dev",
      username: "devname",
      route: "PUT /api/admin/rating",
      request: { ladder: "constructed", rating: 1800 },
      before: { rating: 1523.456, handSet: false },
      after: { rating: 1800, handSet: true },
      at: AT,
    },
    { id: 6, profileId: "p-gone", username: "", route: "POST /api/admin/news", request: { title: "x" }, before: null, after: { id: "n-2" }, at: AT - 1 },
  ],
};

export const WALLET: WalletResponse = { jlore: 1234 };

export const QUESTS: QuestsResponse = {
  quests: [
    { questId: "q-1", title: "Win 3 games", progress: 1, target: 3, jlore: 50, xp: 100 },
    { questId: "q-2", title: "Play a trap", progress: 0, target: 1, jlore: 20, xp: 40 },
  ],
};

/** Every read the dashboard makes, answered. */
export function dashboardRoutes(): Record<string, Route> {
  return {
    "GET /api/admin/me": { body: ME },
    "GET /api/wallet": { body: WALLET },
    "GET /api/quests": { body: QUESTS },
    "GET /api/admin/rating": { body: RATING },
    "GET /api/admin/leaderboard?ladder=constructed": { body: CONSTRUCTED },
    "GET /api/admin/leaderboard?ladder=random": { body: RANDOM },
    "GET /api/admin/news": { body: NEWS },
    "GET /api/admin/audit": { body: AUDIT },
  };
}
