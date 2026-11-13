// The Play screen's browser-test contract: no imports, so `routes/play.tsx`, its panels and `e2e/support/testids.ts`
// read one list. `e2e/support/testids.ts` mirrors these strings.

export const playTestid = {
  queue: "play-queue",
  leaveQueue: "play-leave-queue",
  status: "play-status",
  unreachable: "play-unreachable",
  error: "play-error",
  practice: "play-practice",
  /** #2705: the Dungeon run beside the other ways to play. */
  dungeon: "play-dungeon",
  screen: "play-screen",
  online: "play-online",
  /** #3729, R3729000: the hub, the four groups of cards inside it. */
  hub: "play-hub",
  /** R3729000: Replays and Play together, under the groups. */
  more: "play-more",
  together: "play-together",
  /** #3727: daily games, badged with how many wait on the player. */
  daily: "play-daily",
  dailyBadge: "play-daily-badge",
  /** R3729000: the cards of With friends. */
  lobby: "play-lobby",
  challenge: "play-challenge",
  tutorial: "play-tutorial",
  signIn: "play-sign-in",
  modeBo1: "play-mode-bo1",
  modeBo3: "play-mode-bo3",
  modeRandom: "play-mode-random",
  deckSelect: "play-deck-select",
  trioSelect: "play-trio-select",
  verdict: "play-choice-verdict",
  recentGames: "play-recent-games",
  decksLink: "play-decks-link",
  seriesLink: "play-series-link",
  /** R1566000: a refusal naming the player's live co-op match links to it. */
  duosLink: "play-duos-link",
  searching: "play-searching",
  rank: "play-rank",
  leaderboard: "play-leaderboard",
  /** R768000: the recent games and their replays. */
  replays: "play-replays",
  leanNewest: "play-lean-newest",
  /** #1568: a failed decks read offers Try again beside the alert. */
  decksRetry: "play-decks-retry",
  /** R3729000: an Online card opens this panel: the deck or trio, the verdict, the rank and Find a match. */
  queuePanel: "play-queue-panel",
  /** R3729000: the Private lobby card opens this dialog around the lobby panels. */
  lobbyDialog: "play-lobby-dialog",
  /** R3729000: the mark on the card of the mode the player used last. */
  lastUsed: "play-last-used",
} as const;

/** R3729000: a group's section of the hub. */
export function playGroupTestid(group: string): string {
  return `play-group-${group}`;
}

/** R3729000: the rank an Online card shows for its ladder. */
export function playCardRankTestid(mode: string): string {
  return `play-card-rank-${mode}`;
}

/** R640002: an Online card's expected wait. */
export function playWaitTestid(mode: string): string {
  return `play-wait-${mode}`;
}
