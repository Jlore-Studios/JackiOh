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
// "maxDepth":8,"finalists":3}, re-run on the whole of patch v0.1.1 (issue #27) once its engine,
// card and client halves were merged: the two sweeps its halves ran apart each saw only part of the
// patch. It ran as five parallel slices and flagged no card `timeout`, so nothing needed a second
// run; every entry is `neverPlayed`. Against the table before it (the card half's sweep): Eugenics,
// KY's Private Tutor, Lava Golem, KY's Trial, Fed Fauci and Combo-Index are new, and /fullsend is
// now flagged at easy and Craft a Card at both tiers; Flood, Glowy Jelly Bean, GIGA Glowy Jelly Bean,
// Unstable Clone Machine, Lunar Eclipse, Professor Curvature, Twinspell and My Pawn were played this
// time and come off, and so do Zao Gao and CN-Viral Injection, which the engine half's sweep had
// added. Hinder, Blood Ridden Glowy Jelly Bean and Ceaseless Void were never affordable at either
// tier, so they are unswept and not listed (R186: no evidence either way). No card was flagged
// `error` or `selfHarm`.

/** R186: defId → why the AI never deals it to itself. Each reason starts "<SweepFlag>: <tier>: ". */
export const SHADOW_BAN: Readonly<Record<string, string>> = {
  "core-042": "neverPlayed: hard: affordable in hand on 21 turns, never played",
  "core-051": "neverPlayed: hard: affordable in hand on 20 turns, never played",
  "core-055": "neverPlayed: hard: affordable in hand on 22 turns, never played",
  "core-057": "neverPlayed: hard: affordable in hand on 4 turns, never played",
  "core-076": "neverPlayed: hard: affordable in hand on 15 turns, never played",
  "core-078": "neverPlayed: easy: affordable in hand on 31 turns, never played",
  "core-082": "neverPlayed: hard: affordable in hand on 6 turns, never played",
  "core-091": "neverPlayed: hard: affordable in hand on 19 turns, never played",
  "core-093": "neverPlayed: hard: affordable in hand on 25 turns, never played",
  "core-094": "neverPlayed: hard: affordable in hand on 13 turns, never played",
  "core-099":
    "neverPlayed: easy: affordable in hand on 21 turns, never played; hard: affordable in hand on 13 turns, never played",
};

/** Object.keys(SHADOW_BAN), sorted. */
export const SHADOW_BAN_IDS: readonly string[] = Object.keys(SHADOW_BAN).sort();
