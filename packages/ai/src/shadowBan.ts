// R186: the AI's shadow ban. The cards the AI never deals into its own decks, each with the reason
// the sweep flagged. It governs AI deck-building (`buildAiDeck`'s default `banned`) and nothing else:
// a banned card stays legal for every player, a human may play it against the AI, and the AI still
// has to answer it. It is not §9.4 L6's ban, which is server state (R164).
//
// How an entry gets here: `pnpm ai:sweep` (scripts/sweep.ts) forces each non-token Core card into
// AI decks at every tier in AI_SWEEP.tiers (Easy and Hard) against the greedy baseline and prints
// one row per flagged card and tier, together with a ready-made entry. A flag at either tier bans
// the card at every tier, and the reason names the tier after the flags ("neverPlayed: hard: …").
// No card is listed without a flag, and the ban is not tuned by hand.
//
// Sweep of record: 2026-09-27 (UTC), `pnpm ai:sweep` over 100 non-token Core cards at easy and hard,
// 8 seeds per card and tier (`sweep:<tier>:<id>:<n>`), budget AI_GATE_BUDGET {"nodes":600,
// "lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,
// "maxDepth":8,"finalists":3}, re-run after patch v0.1.1's card changes on the cards branch (the
// engine and Combo-Index changes of the same patch were not in it). It ran as five parallel slices
// on a machine shared with other test runs, which flagged 11 cards `timeout`; each was swept again
// (three slices, then the six whose flag held one after another, then Lunar Eclipse alone on a quiet
// machine), and only Lunar Eclipse's hard-tier timeout held, so it is the one `timeout` entry; each
// card's row is its latest run. The rest are `neverPlayed`. Against the table before it: Flood,
// Glowy Jelly Bean, GIGA Glowy Jelly Bean, Unstable Clone Machine, Lunar Eclipse, Conjure KY (now
// `neverPlayed`), Professor Curvature and My Pawn are new, and /fullsend and Craft a Card are now
// flagged at hard only; Right-house defender, Eugenics, Unbiased Immigration and Transmogulate were
// played this time and come off. Hinder, Blood Ridden Glowy Jelly Bean and Ceaseless Void were
// never affordable at either tier, so they are unswept and not listed (R186: no evidence either
// way). No card was flagged `error` or `selfHarm`.

/** R186: defId → why the AI never deals it to itself. Each reason starts "<SweepFlag>: <tier>: ". */
export const SHADOW_BAN: Readonly<Record<string, string>> = {
  "core-017": "neverPlayed: easy: affordable in hand on 16 turns, never played",
  "core-026": "neverPlayed: easy: affordable in hand on 17 turns, never played",
  "core-029": "neverPlayed: hard: affordable in hand on 15 turns, never played",
  "core-033": "neverPlayed: hard: affordable in hand on 9 turns, never played",
  "core-035": "timeout: hard: 1 decision(s) over 2000 ms or game(s) past 600 actions",
  "core-057": "neverPlayed: hard: affordable in hand on 6 turns, never played",
  "core-076": "neverPlayed: hard: affordable in hand on 7 turns, never played",
  "core-077": "neverPlayed: hard: affordable in hand on 14 turns, never played",
  "core-078": "neverPlayed: hard: affordable in hand on 9 turns, never played",
  "core-079": "neverPlayed: hard: affordable in hand on 7 turns, never played",
  "core-094": "neverPlayed: easy: affordable in hand on 22 turns, never played",
  "core-096": "neverPlayed: hard: affordable in hand on 12 turns, never played",
  "core-099": "neverPlayed: hard: affordable in hand on 13 turns, never played",
};

/** Object.keys(SHADOW_BAN), sorted. */
export const SHADOW_BAN_IDS: readonly string[] = Object.keys(SHADOW_BAN).sort();
