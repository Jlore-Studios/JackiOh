//! R186: the AI's shadow ban. The cards the AI never deals into its own decks, each with the reason
//! the sweep flagged. It governs AI deck-building (`build_ai_deck`'s default `banned`) and nothing
//! else: a banned card stays legal for every player, a human may play it against the AI, and the AI
//! still has to answer it. It is not §9.4 L6's ban, which is server state (R164).
//!
//! How an entry gets here: `pnpm ai:sweep` (scripts/sweep.ts; `cargo jackioh sweep` once v0.3.0
//! ships) forces each non-token card of every set into AI decks at every tier in AI_SWEEP.tiers (Easy
//! and Hard) against the greedy baseline, in two passes (R390): the second sweeps the cards at risk
//! again with more games and deals them more often as filler, and a `neverPlayed` or `selfHarm` ban
//! needs its numbers. It prints one row per flagged card and tier, together with ready-made entries
//! for both tables below. A flag at either tier bans the card at every tier, and the reason names the
//! tier after the flags ("neverPlayed: hard: …"). No card is listed without a flag, and neither table
//! is tuned by hand.
//!
//! Sweep of record: 2026-09-27 (UTC), `pnpm ai:sweep` over 100 non-token Core cards at easy and hard,
//! 8 seeds per card and tier (`sweep:<tier>:<id>:<n>`), budget AI_GATE_BUDGET {"nodes":600,
//! "lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,
//! "maxDepth":8,"finalists":3}, re-run on the whole of patch v0.1.1 (issue #27) once its engine,
//! card and client halves were merged: the two sweeps its halves ran apart each saw only part of the
//! patch. It ran as five parallel slices and flagged no card `timeout`, so nothing needed a second
//! run; every entry is `neverPlayed`. Against the table before it (the card half's sweep): Eugenics,
//! KY's Private Tutor, Lava Golem, KY's Trial, Fed Fauci and Combo-Index are new, and /fullsend is
//! now flagged at easy and Craft a Card at both tiers; Flood, Glowy Jelly Bean, GIGA Glowy Jelly Bean,
//! Unstable Clone Machine, Lunar Eclipse, Professor Curvature, Twinspell and My Pawn were played this
//! time and come off, and so do Zao Gao and CN-Viral Injection, which the engine half's sweep had
//! added. Hinder, Blood Ridden Glowy Jelly Bean and Ceaseless Void were never affordable at either
//! tier, so they are unswept and not listed (R186: no evidence either way). No card was flagged
//! `error` or `selfHarm`.
//!
//! Sweep of record for the Rust AI (generation 0, #306 part 40): 2026-10-07 (UTC), `cargo jackioh
//! sweep`, pass 1 over 268 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2
//! over 91 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget
//! AI_GATE_BUDGET as above, in four parallel slices per pass. It filled `SHADOW_WATCH` below (41
//! cards). `SHADOW_BAN` above stays generation 0's, TypeScript's eleven (#306): that sweep would ban
//! 64 cards, four of the eleven among them and 59 of the 64 for `timeout` alone, and clear the other
//! seven.
//! training/history/sweep-2026-10-07.md lists them, with what the timeouts measured, as the unban
//! lane's starting notes.
//!
//! Port of `packages/ai/src/shadowBan.ts` (SURFACE §9): TS's `Record<string, string>` is a slice of
//! `(defId, reason)` pairs sorted by id, the order `Object.keys` gave it.

/// R186: defId → why the AI never deals it to itself. Each reason starts "<SweepFlag>: <tier>: ".
/// Sorted by id.
pub const SHADOW_BAN: &[(&str, &str)] = &[
    (
        "core-042",
        "neverPlayed: hard: affordable in hand on 21 turns, never played",
    ),
    (
        "core-051",
        "neverPlayed: hard: affordable in hand on 20 turns, never played",
    ),
    (
        "core-055",
        "neverPlayed: hard: affordable in hand on 22 turns, never played",
    ),
    (
        "core-057",
        "neverPlayed: hard: affordable in hand on 4 turns, never played",
    ),
    (
        "core-076",
        "neverPlayed: hard: affordable in hand on 15 turns, never played",
    ),
    (
        "core-078",
        "neverPlayed: easy: affordable in hand on 31 turns, never played",
    ),
    (
        "core-082",
        "neverPlayed: hard: affordable in hand on 6 turns, never played",
    ),
    (
        "core-091",
        "neverPlayed: hard: affordable in hand on 19 turns, never played",
    ),
    (
        "core-093",
        "neverPlayed: hard: affordable in hand on 25 turns, never played",
    ),
    (
        "core-094",
        "neverPlayed: hard: affordable in hand on 13 turns, never played",
    ),
    (
        "core-099",
        "neverPlayed: easy: affordable in hand on 21 turns, never played; hard: affordable in hand on 13 turns, never played",
    ),
];

/// `Object.keys(SHADOW_BAN)`, sorted. `SHADOW_BAN` is kept sorted by id, so its ids in order are it.
pub const SHADOW_BAN_IDS: &[&str] = &{
    let mut ids = [""; SHADOW_BAN.len()];
    let mut at = 0;
    while at < SHADOW_BAN.len() {
        ids[at] = SHADOW_BAN[at].0;
        at += 1;
    }
    ids
};

/// R390, R600: the cards the last sweep of record found at risk by their own numbers and did not ban,
/// each with those numbers. The next sweep counts them at risk from the start (`at_risk_ids`), so a
/// card on track to be banned stays watched from one sweep to the next. It changes no deck.
pub const SHADOW_WATCH: &[(&str, &str)] = &[
    (
        "classic-014",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classic-015",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 4.8",
    ),
    (
        "classic-026",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change -0.4",
    ),
    (
        "classic-030",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 0.0",
    ),
    (
        "classic-037",
        "at risk: hard: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change -2.3",
    ),
    (
        "classic-038",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 3.5",
    ),
    (
        "classic-044",
        "at risk: easy: pass 1 affordable on 13 turns over 8 games, played 1 time(s), mean evaluate change -2.2; hard: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change -2.2",
    ),
    (
        "classic-055",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 4.9",
    ),
    (
        "classic-056",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 21.3",
    ),
    (
        "classic-062",
        "at risk: easy: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change -0.6",
    ),
    (
        "classic-065",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classic-070",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "classic-078",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "classic-085",
        "at risk: easy: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change 0.4; hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 33.2",
    ),
    (
        "classicplus-002",
        "at risk: hard: pass 1 affordable on 13 turns over 8 games, played 1 time(s), mean evaluate change 2.0",
    ),
    (
        "classicplus-003",
        "at risk: hard: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change 5.6",
    ),
    (
        "classicplus-011",
        "at risk: hard: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change 9.2",
    ),
    (
        "classicplus-017",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, never played",
    ),
    (
        "classicplus-031",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classicplus-034",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, never played",
    ),
    (
        "classicplus-035",
        "at risk: easy: pass 1 affordable on 16 turns over 8 games, played 2 time(s), mean evaluate change -499990.1",
    ),
    (
        "classicplus-036",
        "at risk: hard: pass 1 affordable on 24 turns over 8 games, played 1 time(s), mean evaluate change -1.6",
    ),
    (
        "classicplus-037",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 11.0",
    ),
    (
        "classicplus-041",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change -0.8",
    ),
    (
        "classicplus-043",
        "at risk: easy: pass 1 affordable on 8 turns over 8 games, never played; hard: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change -0.6",
    ),
    (
        "classicplus-044",
        "at risk: hard: pass 1 affordable on 22 turns over 8 games, never played",
    ),
    (
        "classicplus-048",
        "at risk: hard: pass 1 affordable on 14 turns over 8 games, played 1 time(s), mean evaluate change 0.1",
    ),
    (
        "classicplus-062",
        "at risk: easy: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change -1.3; hard: pass 1 affordable on 16 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "classicplus-065",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 1.9",
    ),
    (
        "classicplus-066",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, never played",
    ),
    (
        "classicplus-078",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "core-007",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, never played",
    ),
    (
        "core-040",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "core-042",
        "at risk: hard: pass 1 affordable on 14 turns over 8 games, never played",
    ),
    (
        "core-055",
        "at risk: easy: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 9.7; hard: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change 28.5",
    ),
    (
        "core-057",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, never played",
    ),
    (
        "core-065",
        "at risk: hard: pass 1 affordable on 45 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "core-079",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "core-087",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, never played",
    ),
    (
        "core-093",
        "at risk: hard: pass 1 affordable on 21 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "core-099",
        "at risk: easy: pass 1 affordable on 16 turns over 8 games, played 1 time(s), mean evaluate change -2.7; hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change -2.2",
    ),
];
