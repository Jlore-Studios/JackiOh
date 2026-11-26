// R3496000: the stats of a run that never settled, apart from the store so that the page's tests'
// fixtures can import it inside their `vi.mock("../wasm/index.ts")` factories without the WebAssembly
// bindings `store.ts` loads.

import type { RunStats } from "@jackioh/run";

/** R3496000: a run that never settled has no rows, no score and no stats. */
export const EMPTY_RUN_STATS: RunStats = {
  nodesVisited: 0,
  fightsWon: 0,
  elitesWon: 0,
  ghostsWon: 0,
  miniBossesWon: 0,
  bossesWon: 0,
  damageDealt: 0,
  healingDone: 0,
  cardsDrawn: 0,
  turns: 0,
  damageTaken: 0,
  healed: 0,
  coinsEarned: 0,
  coinsSpent: 0,
  ticketsEarned: 0,
  ticketsSpent: 0,
  cardsKept: 0,
  cardsBought: 0,
  cardsRemoved: 0,
  cardsMadeRadiant: 0,
  cardsBuffed: 0,
  packsSkipped: 0,
  artifactsTaken: 0,
  artifactsLeveled: 0,
  blessingsTaken: 0,
  eventsSeen: 0,
  chestsOpened: 0,
};
