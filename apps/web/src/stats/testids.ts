// Test IDs for public statistics screen (R654).

export const statsTestid = {
  screen: "stats-screen",
  tabCards: "stats-tab-cards",
  tabPlayers: "stats-tab-players",
  /** R1966000: the Sets tab. */
  tabSets: "stats-tab-sets",
  setsLeaders: "stats-sets-leaders",
  setsTable: "stats-sets-table",
  setsOutliers: "stats-sets-outliers",
  setsRetry: "stats-sets-retry",
  summaryTiles: "stats-summary-tiles",
  /** R1994000: the Games panel. */
  gamesPanel: "stats-games-panel",
  gamesFew: "stats-games-few",
  gamesRetry: "stats-games-retry",
  /** R1973002: the patch bar; R3324000: its patch tabs (a Segmented's options) and a collecting patch's message. */
  patchBar: "stats-patch-bar",
  patchTab: "stats-patch-tab",
  patchPrevious: "stats-patch-previous",
  patchNext: "stats-patch-next",
  patchCollecting: "stats-patch-collecting",
  patchCollectingBack: "stats-patch-collecting-back",
  /** R3324001: the header's current patch. */
  currentPatch: "stats-current-patch",
  summaryTotalGames: "stats-summary-total-games",
  searchInput: "stats-search-input",
  cardsTable: "stats-cards-table",
  cardsGrid: "stats-cards-grid",
  /** R2089000, R3324002: the Created cards that are no token, and the Tokens, two sections that share no row. */
  createdPanel: "stats-created-panel",
  createdTable: "stats-created-table",
  tokensPanel: "stats-tokens-panel",
  tokensTable: "stats-tokens-table",
  /** R3324003: the Columns toggles. */
  columnGames: "stats-column-games",
  columnPlayRate: "stats-column-play-rate",
  viewTable: "stats-view-table",
  viewGrid: "stats-view-grid",
  sortSelect: "stats-sort-select",
  sortDir: "stats-sort-dir",
  modeSelect: "stats-mode-select",
  /** R2777000: a card row's adjusted win rate. */
  adjustedCell: "stats-adjusted-cell",
  /** R3185000: the card detail's Statistics panel's tier chip and adjusted win rate (beside `INSPECT_STATS`). */
  cardStatsTier: "inspect-stats-tier",
  cardStatsAdjusted: "inspect-stats-adjusted",
  /** R3022000: a card's tier (a table row's or a grid item's) and the line under the cards that says what tiers mean. */
  tierCell: "stats-tier-cell",
  tiersNote: "stats-tiers-note",
  /** R3184001: under a tier, with the ranges shown, the share of the card's Adjusted range inside it. */
  tierFit: "stats-tier-fit",
  /** R2895000: the Cards tab's Show 95% ranges toggle, a row's two ranges and the line that says what they mean. */
  rangesToggle: "stats-ranges-toggle",
  adjustedRange: "stats-adjusted-range",
  winRateRange: "stats-win-rate-range",
  rangesNote: "stats-ranges-note",
  playersTable: "stats-players-table",
  playersPager: "stats-players-pager",
  playersPrev: "stats-players-prev",
  playersNext: "stats-players-next",
  drillDownModal: "stats-drill-down-modal",
  drillDownClose: "stats-drill-down-close",
  drillDownRadiantToggle: "stats-drill-down-radiant-toggle",
  drillDownRadiantFace: "stats-drill-down-radiant-face",
  drillDownAlmanac: "stats-drill-down-almanac",
  drillDownTokens: "stats-drill-down-tokens",
} as const;
