// A dungeon run's map, tiles and run-over screens' `data-testid` vocabulary (DR14 part F,
// #2707): every id is prefixed `dungeon-`, as each part of the dungeon run keeps its own in its
// directory. Indexed entries take the source array's own index, so an action's `index` is the
// test id's number.

export const dungeonMapTestid = {
  /** The run screen itself, in every phase but `fight`. */
  screen: "dungeon-run",
  /** The phase's body, under the HUD. */
  body: "dungeon-run-body",
  /** The HUD, on every screen. */
  hud: "dungeon-run-hud",
  /** B30: HP over the run's max. */
  hp: "dungeon-run-hp",
  /** B30: Coins. */
  coins: "dungeon-run-coins",
  /** B30: Tickets. */
  tickets: "dungeon-run-tickets",
  /** The deck's size. */
  deckSize: "dungeon-run-deck-size",
  /** The relics held. */
  artifacts: "dungeon-run-artifacts",
  /** One relic held, by its place in the run's list. */
  hudArtifact: (index: number): string => `dungeon-run-artifact-${String(index)}`,
  /** The score so far. */
  score: "dungeon-run-score",
  /** #3501: the HUD's Your run button, which opens the panel. */
  yourRun: "dungeon-run-your-run",
  /** #3501: the Your run panel. */
  panel: "dungeon-run-panel",
  /** #3501: the panel's ✕. */
  panelClose: "dungeon-run-panel-close",
  /** #3501: the run's seed, in the panel. */
  panelSeed: "dungeon-run-panel-seed",
  /** #3501: the panel's button that copies the seed. */
  panelSeedCopy: "dungeon-run-panel-seed-copy",
  /** #3501: the panel's deck grid. */
  panelDeck: "dungeon-run-panel-deck",
  /** #3501: one deck card in the panel, by its place in the deck. */
  panelCard: (index: number): string => `dungeon-run-panel-card-${String(index)}`,
  /** #3501: the relics the panel lists. */
  panelArtifacts: "dungeon-run-panel-artifacts",
  /** #3501: one relic in the panel, by its place in the run's list. */
  panelArtifact: (index: number): string => `dungeon-run-panel-artifact-${String(index)}`,
  /** #3501: the blessings the panel lists. */
  panelBlessings: "dungeon-run-panel-blessings",
  /** #3501: one blessing in the panel, by its place in the run's list. */
  panelBlessing: (index: number): string => `dungeon-run-panel-blessing-${String(index)}`,
  /** #3501: the curses the panel lists. */
  panelCurses: "dungeon-run-panel-curses",
  /** #3501: one curse in the panel, by its place in the run's list. */
  panelCurse: (index: number): string => `dungeon-run-panel-curse-${String(index)}`,
  /** The run's refusal of the last action; role="alert". */
  refusal: "dungeon-run-refusal",
  /** B27: the current act's map. */
  map: "dungeon-run-map",
  /** One map row, by its absolute row. */
  mapRow: (row: number): string => `dungeon-run-map-row-${String(row)}`,
  /** One map node, by its absolute row and column. */
  mapNode: (row: number, col: number): string => `dungeon-run-map-node-${String(row)}-${String(col)}`,
  /** B27: where the player stands, inside the current node. */
  mapCurrent: "dungeon-run-map-current",
  /** B27: a boss's preseeded relic, by its node's absolute row and column. */
  mapArtifact: (row: number, col: number): string => `dungeon-run-map-artifact-${String(row)}-${String(col)}`,
  /** B28: the reward screen. */
  reward: "dungeon-run-reward",
  /** B28: the artifact the fight dropped. */
  rewardArtifact: "dungeon-run-reward-artifact",
  /** B28: take the dropped relic. */
  artifactTake: "dungeon-run-artifact-take",
  /** B28: leave the dropped relic. */
  artifactLeave: "dungeon-run-artifact-leave",
  /** B28: the four packs. */
  packs: "dungeon-run-packs",
  /** B28: one pack, by its place in the reward's list. */
  pack: (index: number): string => `dungeon-run-pack-${String(index)}`,
  /** B28: the opened pack's cards. */
  packScreen: "dungeon-run-pack",
  /** B28: one of the pack's cards, by its catalog id. */
  packCard: (id: string): string => `dungeon-run-pack-card-${id}`,
  /** B28: keep the chosen cards. */
  packKeep: "dungeon-run-pack-keep",
  /** B28: keep none, with a Skip passive. */
  packSkip: "dungeon-run-pack-skip",
  /** B29: extraction after a mini-boss. */
  extractCard: "dungeon-run-extract-card",
  /** B29: one of the deck's cards, by its place in the deck. */
  extractCardChoice: (index: number): string => `dungeon-run-extract-card-${String(index)}`,
  /** B29: extraction after the final boss. */
  extractArtifact: "dungeon-run-extract-artifact",
  /** B29: one relic held, by its place in the run's list. */
  extractArtifactChoice: (index: number): string => `dungeon-run-extract-artifact-${String(index)}`,
  /** B29: extract nothing. */
  extractNone: "dungeon-run-extract-none",
  /** B30: the shop. */
  shop: "dungeon-run-shop",
  /** B30: one of the shop's stock, by its place in the stock. */
  shopStock: (index: number): string => `dungeon-run-shop-stock-${String(index)}`,
  /** B30: buy one of the stock. */
  shopBuy: (index: number): string => `dungeon-run-shop-buy-${String(index)}`,
  /** B30: remove one of the deck's cards for Coins, by its place in the deck. */
  shopRemove: (index: number): string => `dungeon-run-shop-remove-${String(index)}`,
  /** B30: make one of the deck's cards Radiant, by its place in the deck. */
  shopRadiant: (index: number): string => `dungeon-run-shop-radiant-${String(index)}`,
  /** B30: buff one of the deck's cards, by its place in the deck. */
  shopBuff: (index: number): string => `dungeon-run-shop-buff-${String(index)}`,
  /** B30: level one relic held, by its place in the run's list. */
  shopLevel: (index: number): string => `dungeon-run-shop-level-${String(index)}`,
  /** B30: the note a relic at its highest level shows where its Level up button would be. */
  shopLevelMax: (index: number): string => `dungeon-run-shop-level-max-${String(index)}`,
  /** B30: leave the shop. */
  shopLeave: "dungeon-run-shop-leave",
  /** B30: the campfire. */
  campfire: "dungeon-run-campfire",
  /** B30: heal at the campfire. */
  campfireHeal: "dungeon-run-campfire-heal",
  /** B30: make one of the deck's cards Radiant, by its place in the deck. */
  campfireRadiant: (index: number): string => `dungeon-run-campfire-radiant-${String(index)}`,
  /** B30: buff one of the deck's cards, by its place in the deck. */
  campfireBuff: (index: number): string => `dungeon-run-campfire-buff-${String(index)}`,
  /** B30: toggle one of the deck's cards for removal, by its place in the deck. */
  campfireRemoveCard: (index: number): string => `dungeon-run-campfire-remove-card-${String(index)}`,
  /** B30: remove the toggled cards. */
  campfireRemove: "dungeon-run-campfire-remove",
  /** B30: the event. */
  event: "dungeon-run-event",
  /** B30: one of the event's options, by its place in the list. */
  eventOption: (index: number): string => `dungeon-run-event-option-${String(index)}`,
  /** B30: the shrine. */
  shrine: "dungeon-run-shrine",
  /** B30: take one of the shrine's blessings, by its place in the offers. */
  shrineOffer: (index: number): string => `dungeon-run-shrine-offer-${String(index)}`,
  /** B30: leave the shrine. */
  shrineLeave: "dungeon-run-shrine-leave",
  /** B31: the run-over screen. */
  runOver: "dungeon-run-over",
  /** B31: the run's score. */
  endScore: "dungeon-run-end-score",
  /** B31: the run's time, beside the score. */
  endTime: "dungeon-run-end-time",
  /** B31: the turns the run's fights took, beside the score. */
  endTurns: "dungeon-run-end-turns",
  /** B31: the Embers the score pays. */
  endEmbers: "dungeon-run-end-embers",
  /** #3501: the run's seed, on the run-over screen. */
  endSeed: "dungeon-run-end-seed",
  /** B31: the cards the run extracted. */
  extractedCards: "dungeon-run-extracted-cards",
  /** B31: the relics the run extracted. */
  extractedArtifacts: "dungeon-run-extracted-artifacts",
  /** B31: one of the run's stats, by its `RunStats` key. */
  stat: (key: string): string => `dungeon-run-stat-${key}`,
  /** B31: start over; part D clears the finished run. */
  newRun: "dungeon-run-new",
  /** B31: the run's statistics page. */
  statsLink: "dungeon-run-stats-link",
} as const;
