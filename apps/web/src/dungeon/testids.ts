// The Dungeon Run page's and its start screen's `data-testid` vocabulary (DR14 part D, #2705): every
// id is prefixed `dungeon-`, as each part of the dungeon run keeps its own in its directory.

export const dungeonTestid = {
  /** The page's frame, around whichever screen the run is on. */
  screen: "dungeon-screen",
  /** The start screen. */
  start: "dungeon-start",
  /** B14: the mode choices; the Sandbox is chosen, the Campaign disabled. */
  modeSandbox: "dungeon-mode-sandbox",
  modeCampaign: "dungeon-mode-campaign",
  /** B14: why the Campaign is off. */
  campaignNote: "dungeon-campaign-note",
  /** B15: the seed's three ways, the typed seed's input and the seed the run will use. */
  seedDaily: "dungeon-seed-daily",
  seedRandom: "dungeon-seed-random",
  seedTyped: "dungeon-seed-typed",
  seedInput: "dungeon-seed-input",
  seedValue: "dungeon-seed-value",
  /** B15: a typed seed that is not capital letters of the right length; role="alert". */
  seedError: "dungeon-seed-error",
  /** B16: a slot of the level's ladder, the cards offered, one card, and how many are chosen. */
  slot: (name: string): string => `dungeon-slot-${name}`,
  card: (id: string): string => `dungeon-card-${id}`,
  cards: "dungeon-cards",
  chosenCount: "dungeon-chosen-count",
  /** B16: a relic offered, how many are taken and their points out of the most. */
  relic: (id: string): string => `dungeon-relic-${id}`,
  relicCount: "dungeon-relic-count",
  relicPoints: "dungeon-relic-points",
  /** B17: a curse tier and the points the chosen ones are worth. */
  curse: (id: string): string => `dungeon-curse-${id}`,
  cursePoints: "dungeon-curse-points",
  /** B16: the run's own refusal of the setup, word for word; role="alert". */
  setupRefusal: "dungeon-setup-refusal",
  startRun: "dungeon-start-run",
  /** B18: the Embers balance, the starting deck's level, the next level's button and its refusal. */
  embers: "dungeon-embers",
  level: "dungeon-level",
  buyLevel: "dungeon-buy-level",
  buyRefusal: "dungeon-buy-refusal",
  /** B19: a run in progress is offered back, or given up. */
  saved: "dungeon-saved",
  continueRun: "dungeon-continue",
  abandonRun: "dungeon-abandon",
} as const;
