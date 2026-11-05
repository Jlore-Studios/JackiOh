// Every control's and every read value's `data-testid`, which the e2e suite drives (README.md, Test ids).
// A function names one per ladder, clear or bar.

import type { Bar, ClearPart, Ladder } from "./types.ts";

export const testids = {
  loading: "admin-loading",

  // Sign-in (SignIn.tsx).
  signIn: "admin-sign-in",
  signInMethodPassword: "admin-sign-in-method-password",
  signInMethodCode: "admin-sign-in-method-code",
  signInEmail: "admin-sign-in-email",
  signInPassword: "admin-sign-in-password",
  signInSubmit: "admin-sign-in-submit",
  signInSendCode: "admin-sign-in-send-code",
  signInCode: "admin-sign-in-code",
  signInVerifyCode: "admin-sign-in-verify-code",
  signInTotp: "admin-sign-in-totp",
  signInTotpSubmit: "admin-sign-in-totp-submit",
  signInCancel: "admin-sign-in-cancel",
  signInNotice: "admin-sign-in-notice",
  signInError: "admin-sign-in-error",

  // After sign-in, before the dashboard (App.tsx).
  forbidden: "admin-forbidden",
  forbiddenMessage: "admin-forbidden-message",
  bootError: "admin-boot-error",
  retry: "admin-retry",
  signOut: "admin-sign-out",

  // The dashboard (Dashboard.tsx) and its one status line.
  dashboard: "admin-dashboard",
  status: "admin-status",
  loadError: "admin-load-error",

  // Account.
  account: "admin-account",
  accountUsername: "admin-account-username",
  accountProfileId: "admin-account-profile-id",
  accountStrict: "admin-account-strict",
  accountSecondFactor: "admin-account-second-factor",

  // Holdings and Dev tools.
  holdings: "admin-holdings",
  walletJlore: "admin-wallet-jlore",
  questRow: "admin-quest-row",
  set: "admin-set",
  xpInput: "admin-xp-input",
  xpGive: "admin-xp-give",
  jloreInput: "admin-jlore-input",
  jloreGive: "admin-jlore-give",
  fill: "admin-fill",
  bar: (bar: Bar) => `admin-bar-${bar}`,
  questGive: "admin-quest-give",
  questSelect: "admin-quest-select",
  questComplete: "admin-quest-complete",
  clear: (part: ClearPart) => `admin-clear-${part}`,

  // Rating, one block per ladder.
  rating: "admin-rating",
  ratingLadder: (ladder: Ladder) => `admin-rating-${ladder}`,
  ratingValue: (ladder: Ladder) => `admin-rating-value-${ladder}`,
  ratingDeviation: (ladder: Ladder) => `admin-rating-deviation-${ladder}`,
  ratingVolatility: (ladder: Ladder) => `admin-rating-volatility-${ladder}`,
  ratingHandSet: (ladder: Ladder) => `admin-rating-hand-set-${ladder}`,
  ratingEarned: (ladder: Ladder) => `admin-rating-earned-${ladder}`,
  ratingInput: (ladder: Ladder) => `admin-rating-input-${ladder}`,
  ratingSet: (ladder: Ladder) => `admin-rating-set-${ladder}`,
  ratingReset: (ladder: Ladder) => `admin-rating-reset-${ladder}`,

  // Leaderboard: every row carries `data-profile-id`, `data-kind`, `data-hand-set` and `data-you`.
  leaderboard: "admin-leaderboard",
  ladder: (ladder: Ladder) => `admin-ladder-${ladder}`,
  leaderboardSeason: "admin-leaderboard-season",
  leaderboardRow: "admin-leaderboard-row",

  // News posts: every row carries `data-post-id` and `data-taken-down`.
  news: "admin-news",
  newsTitle: "admin-news-title",
  newsSummary: "admin-news-summary",
  newsPost: "admin-news-post",
  newsRow: "admin-news-row",
  newsTakeDown: "admin-news-take-down",

  // Audit: every row carries `data-action-id`.
  audit: "admin-audit",
  auditRow: "admin-audit-row",
  auditRefresh: "admin-audit-refresh",
} as const;
