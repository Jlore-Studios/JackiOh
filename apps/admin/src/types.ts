// What the server's admin routes answer (#3074), and the holdings the Dev tools read. Every number the
// dashboard shows is one of these, never one it works out.

/** The two rated ladders, in the order the server lists them. */
export const LADDERS = ["constructed", "random"] as const;
export type Ladder = (typeof LADDERS)[number];

/** `GET /api/admin/me`. `strict`: production rules (aal2 required, a hand-set rating off the public ladder). */
export type AdminMe = { profileId: string; username: string; strict: boolean; secondFactor: boolean };

export type Glicko = { rating: number; deviation: number; volatility: number };

/** One ladder's rating; `earned` is what the account's games had earned, kept while the rating is hand-set. */
export type AdminRating = Glicko & { ladder: Ladder; handSet: boolean; earned: Glicko | null };

/** `GET`, `PUT /api/admin/rating` and `POST /api/admin/rating/reset`. */
export type RatingResponse = { ladders: AdminRating[] };

export type RankTier = "raisin" | "rotten" | "normal" | "large" | "golden" | "mythic" | "jlorious";

export type VisibleRank = {
  tier: RankTier;
  division?: number;
  pips?: number;
  pipsPerDivision?: number;
  position?: number;
  placementsPlayed?: number;
  placementGames?: number;
};

export type AdminLeaderboardRow = {
  profileId: string;
  username: string;
  kind: "person" | "bot";
  rating: number;
  games: number;
  wins: number;
  losses: number;
  draws: number;
  rank: VisibleRank;
  handSet: boolean;
  you: boolean;
  bot: null | { strength: number; persona: string; retired: boolean };
};

/** `GET /api/admin/leaderboard?ladder=…`, highest rating first. */
export type LeaderboardResponse = { season: string; ladder: Ladder; rows: AdminLeaderboardRow[] };

/** A news post; `postedBy` is a username, or "" when the account is gone. Times are epoch ms. */
export type NewsPost = {
  id: string;
  title: string;
  summary: string;
  postedAt: number;
  postedBy: string;
  takenDownAt: number | null;
};

export type NewsResponse = { posts: NewsPost[] };

export type AuditAction = {
  id: number;
  profileId: string;
  username: string;
  route: string;
  request: unknown;
  before: unknown;
  after: unknown;
  at: number;
};

export type AuditResponse = { actions: AuditAction[] };

/** What `POST /api/dev/clear/:part` clears: one holding, or all five (SPEC §14.10), in the order §14.10 lists them. */
export const CLEAR_PARTS = ["cards", "cosmetics", "jlore", "counters", "tracks", "everything"] as const;
export type ClearPart = (typeof CLEAR_PARTS)[number];

/** The token bars `POST /api/dev/bar/:bar` fills (R1310003), in the order §14.5 lists them. */
export const BARS = ["card", "cosmetic"] as const;
export type Bar = (typeof BARS)[number];

/** One held quest (`GET /api/quests`, §14.9). */
export type QuestView = { questId: string; title: string; progress: number; target: number; jlore: number; xp: number };

export type QuestsResponse = { quests: QuestView[] };

export type WalletResponse = { jlore: number };
