//! R186: the AI's shadow ban. The cards the AI never deals into its own decks, each with the reason
//! the sweep flagged. It governs AI deck-building (`build_ai_deck`'s default `banned`) and nothing
//! else: a banned card stays legal for every player, a human may play it against the AI, and the AI
//! still has to answer it. It is not §9.4 L6's ban, which is server state (R164).
//!
//! How an entry gets here: `cargo jackioh sweep` (crates/tools/src/sweep.rs) forces each non-token
//! card of every set into AI decks at every tier in AI_SWEEP.tiers against the greedy baseline, in two
//! passes (R390), and `cargo jackioh sweep --write` writes both tables in from the reports of record
//! the header block names (`--check` fails on anything else). A flag at either tier bans the card at
//! every tier, the reason naming the tier ("neverPlayed: hard: …"); no card is listed without a flag,
//! and neither table is tuned by hand. From #1678 on (R1678000), `.github/workflows/sweep.yml` runs a
//! sweep of record after every push to `main` that moves the AI's generation or ships a set. Each
//! report under training/history/ says what its sweep ran and found, the earlier ones' included (#27,
//! #306's generation 0, #1186's for #1173 with every set previewed by R1420, #1675's on #1477's
//! learned evaluation). The table is `(defId, reason)` pairs sorted by id (SURFACE §9).

// BEGIN `cargo jackioh sweep --write`: the reports of record (R186), oldest first. Never edit by hand.
// Report of record: training/history/sweep-2026-10-07.md
// Sweep of record: 2026-10-07 (UTC), `cargo jackioh sweep`, pass 1 over 268 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 91 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-12.md
// Sweep of record: 2026-10-12 (UTC), `cargo jackioh sweep`, pass 1 over 102 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 22 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-18.md
// Sweep of record: 2026-10-18 (UTC), `cargo jackioh sweep`, pass 1 over 26 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 11 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-22.md
// Sweep of record: 2026-10-22 (UTC), `cargo jackioh sweep`, pass 1 over 17 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 17 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-25.md
// Sweep of record: 2026-10-25 (UTC), `cargo jackioh sweep`, pass 1 over 1 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 1 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-25-2.md
// Sweep of record: 2026-10-25 (UTC), `cargo jackioh sweep`, pass 1 over 27 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 27 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-28.md
// Sweep of record: 2026-10-28 (UTC), `cargo jackioh sweep`, pass 1 over 175 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 86 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-29.md
// Sweep of record: 2026-10-29 (UTC), `cargo jackioh sweep`, pass 1 over 60 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 23 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-29-2.md
// Sweep of record: 2026-10-29 (UTC), `cargo jackioh sweep`, pass 1 over 10 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 10 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-29-3.md
// Sweep of record: 2026-10-29 (UTC), `cargo jackioh sweep`, pass 1 over 60 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 33 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-10-29-4.md
// Sweep of record: 2026-10-29 (UTC), `cargo jackioh sweep`, pass 1 over 60 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 30 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":600,"lethalNodes":150,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// END `cargo jackioh sweep --write`

/// R186: defId → why the AI never deals it to itself. Each reason starts "<SweepFlag>: <tier>: ".
/// Sorted by id.
///
/// Empty since #2521 lifted its last two rows, S #149 Wildar (#2358) and TG #25 Trigor (#2397), both
/// `neverPlayed`, through the unban lane (`SHADOW_UNBANNED`, R2521010, #2452): a banned card the beam
/// never casts is a dead slot costing a fraction of a draw. Each generation's lifts and their evidence
/// (1–8: beam blind spots and R29 scorers' slow games, #628's game-ending-play artifact, R101, R360;
/// 9: #732's `timeout` bans as R1390's gate lifted; 10–12: wall-clock bookkeeping, each table held to
/// the seeded fixtures, R265, R677, B34; then #1186, #1675's R977000 and R1675000, and #1889) are in
/// the reports under training/history/ and in git log.
pub const SHADOW_BAN: &[(&str, &str)] = &[];

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

/// R732000, R1186000, R2521010: the ids the unban lane has removed from the reports of record's
/// `SHADOW_BAN` entries, sorted by id (generation 10: 021, 033, 047, 051, 063, 073, 092; 11: 041, 067,
/// 080; 12: 006, 016, 045, 068, as training/history/ tells; #2521: `stinkyscape-149` Wildar and
/// `traininggrounds-025` Trigor, the two `neverPlayed` rows #2358 and #2397 wrote, so the AI deals
/// itself every card of every set, #2452). `cargo jackioh sweep --write` leaves their rows out of
/// `SHADOW_BAN`, and refuses an id no report of record bans. The lane unbans a card by adding its id
/// here and running `cargo jackioh sweep --write`; a new sweep of a set drops its own ids from this
/// list. An id may be any card of a set the ban takes (`BAN_SETS` in `crates/tools/src/sweep.rs`).
pub const SHADOW_UNBANNED: &[&str] = &[
    "meditative-006",
    "meditative-016",
    "meditative-021",
    "meditative-033",
    "meditative-041",
    "meditative-045",
    "meditative-047",
    "meditative-051",
    "meditative-063",
    "meditative-067",
    "meditative-068",
    "meditative-073",
    "meditative-080",
    "meditative-092",
    "stinkyscape-149",
    "traininggrounds-025",
];

/// Issue #1113: frozen fixture pool (R290's tier games, gate smokes), isolating seeded fixtures
/// from shadow-ban table changes.
pub const FIXTURE_BANNED_IDS: &[&str] = &[
    "meditative-011",
    "meditative-012",
    "meditative-017",
    "meditative-025",
    "meditative-029",
    "meditative-030",
    "meditative-031",
    "meditative-049",
    "meditative-058",
    "meditative-069",
    "meditative-075",
    "meditative-078",
    "meditative-085",
    "meditative-087",
    "meditative-091",
    "meditative-096",
    "meditative-099",
];

/// R390, R600: the cards the last sweep of record found at risk by their own numbers and did not ban,
/// each with those numbers. The next sweep counts them at risk from the start (`at_risk_ids`), so a
/// card on track to be banned stays watched from one sweep to the next. It changes no deck.
pub const SHADOW_WATCH: &[(&str, &str)] = &[
    (
        "classic-006",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, never played",
    ),
    (
        "classic-015",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 4.8",
    ),
    (
        "classic-018",
        "at risk: hard: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change 11.0",
    ),
    (
        "classic-026",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change -0.4",
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
        "classic-042",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classic-043",
        "at risk: easy: pass 1 affordable on 12 turns over 8 games, never played; hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change -3.1",
    ),
    (
        "classic-044",
        "at risk: easy: pass 1 affordable on 13 turns over 8 games, played 1 time(s), mean evaluate change -2.2; hard: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change -2.2",
    ),
    (
        "classic-050",
        "at risk: hard: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change 10.1",
    ),
    (
        "classic-055",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 4.9",
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
        "classic-066",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 10.9",
    ),
    (
        "classic-070",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "classic-074",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classic-078",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "classic-080",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classic-089",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 14.9",
    ),
    (
        "classicplus-003",
        "at risk: hard: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change 5.6",
    ),
    (
        "classicplus-009",
        "at risk: hard: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change -1.0",
    ),
    (
        "classicplus-014",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, never played; hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "classicplus-017",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, never played",
    ),
    (
        "classicplus-022",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "classicplus-029",
        "at risk: hard: pass 1 affordable on 20 turns over 8 games, never played",
    ),
    (
        "classicplus-030",
        "at risk: easy: pass 1 affordable on 11 turns over 8 games, never played",
    ),
    (
        "classicplus-031",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "classicplus-033",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 14.0",
    ),
    (
        "classicplus-034",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, never played",
    ),
    (
        "classicplus-036",
        "at risk: hard: pass 1 affordable on 24 turns over 8 games, played 1 time(s), mean evaluate change -1.6",
    ),
    (
        "classicplus-041",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change -0.8",
    ),
    (
        "classicplus-042",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, never played",
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
        "classicplus-045",
        "at risk: easy: pass 1 affordable on 23 turns over 8 games, never played; hard: pass 1 affordable on 15 turns over 8 games, never played",
    ),
    (
        "classicplus-048",
        "at risk: hard: pass 1 affordable on 14 turns over 8 games, played 1 time(s), mean evaluate change 0.1",
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
        "classicplus-073",
        "at risk: hard: pass 1 affordable on 15 turns over 8 games, played 1 time(s), mean evaluate change -2.2",
    ),
    (
        "classicplus-078",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "core-001",
        "at risk: easy: pass 1 affordable on 23 turns over 8 games, never played",
    ),
    (
        "core-005",
        "at risk: hard: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change 2.0",
    ),
    (
        "core-006",
        "at risk: easy: pass 1 affordable on 17 turns over 8 games, never played",
    ),
    (
        "core-007",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, never played",
    ),
    (
        "core-016",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 999883.7",
    ),
    (
        "core-017",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, played 1 time(s), mean evaluate change 34.1",
    ),
    (
        "core-022",
        "at risk: hard: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change 2.0",
    ),
    (
        "core-026",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, never played",
    ),
    (
        "core-034",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 19.3",
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
        "core-048",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 4.4",
    ),
    (
        "core-057",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, never played",
    ),
    (
        "core-058",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "core-059",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, played 1 time(s), mean evaluate change 1.4",
    ),
    (
        "core-065",
        "at risk: hard: pass 1 affordable on 45 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "core-069",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, never played",
    ),
    (
        "core-072",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "core-079",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "core-083",
        "at risk: hard: pass 1 affordable on 21 turns over 8 games, never played",
    ),
    (
        "core-086",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "core-087",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, never played",
    ),
    (
        "core-090",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change -1.6",
    ),
    (
        "core-092",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 13.9",
    ),
    (
        "core-093",
        "at risk: hard: pass 1 affordable on 21 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "core-094",
        "at risk: easy: pass 1 affordable on 21 turns over 8 games, played 1 time(s), mean evaluate change 0.4; hard: pass 1 affordable on 10 turns over 8 games, never played",
    ),
    (
        "core-099",
        "at risk: easy: pass 1 affordable on 16 turns over 8 games, played 1 time(s), mean evaluate change -2.7; hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change -2.2",
    ),
    (
        "meditative-011",
        "at risk: easy: pass 1 affordable on 13 turns over 8 games, played 1 time(s), mean evaluate change 2.5",
    ),
    (
        "meditative-022",
        "at risk: easy: pass 1 affordable on 21 turns over 8 games, never played; hard: pass 1 affordable on 17 turns over 8 games, never played",
    ),
    (
        "meditative-026",
        "at risk: easy: pass 1 affordable on 19 turns over 8 games, never played; hard: pass 1 affordable on 14 turns over 8 games, never played",
    ),
    (
        "meditative-053",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, never played",
    ),
    (
        "meditative-055",
        "at risk: hard: pass 1 affordable on 14 turns over 8 games, never played",
    ),
    (
        "meditative-060",
        "at risk: easy: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 2.0; hard: pass 1 affordable on 19 turns over 8 games, never played",
    ),
    (
        "meditative-069",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 8.6; hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 6.6",
    ),
    (
        "meditative-075",
        "at risk: easy: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 1.5; hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 2.5",
    ),
    (
        "meditative-098",
        "at risk: easy: pass 1 affordable on 21 turns over 8 games, never played; hard: pass 1 affordable on 5 turns over 8 games, never played",
    ),
    (
        "meditative-099",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 2.8",
    ),
    (
        "meditativeplus-002",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 0.0",
    ),
    (
        "meditativeplus-003",
        "at risk: easy: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change 1.6; hard: pass 1 affordable on 14 turns over 8 games, played 1 time(s), mean evaluate change 1.7",
    ),
    (
        "meditativeplus-004",
        "at risk: easy: pass 1 affordable on 3 turns over 8 games, never played; hard: pass 1 affordable on 11 turns over 8 games, never played",
    ),
    (
        "meditativeplus-005",
        "at risk: easy: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 2.0",
    ),
    (
        "meditativeplus-013",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 6.8",
    ),
    (
        "meditativeplus-014",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 1.3",
    ),
    (
        "meditativeplus-020",
        "at risk: easy: pass 1 affordable on 33 turns over 8 games, never played; hard: pass 1 affordable on 15 turns over 8 games, never played",
    ),
    (
        "meditativeplus-022",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "meditativeplus-023",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "stinkyscape-002",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 148.4",
    ),
    (
        "stinkyscape-004",
        "at risk: easy: pass 1 affordable on 8 turns over 8 games, never played; hard: pass 1 affordable on 7 turns over 8 games, never played; pass 2 affordable on 46 turns over 40 games, played 1 time(s), mean evaluate change -2.2",
    ),
    (
        "stinkyscape-007",
        "at risk: easy: pass 1 affordable on 14 turns over 8 games, played 1 time(s), mean evaluate change 2.0",
    ),
    (
        "stinkyscape-008",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, never played",
    ),
    (
        "stinkyscape-009",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 10.8",
    ),
    (
        "stinkyscape-010",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 2.3",
    ),
    (
        "stinkyscape-011",
        "at risk: easy: pass 1 affordable on 3 turns over 8 games, played 2 time(s), mean evaluate change -71.6",
    ),
    (
        "stinkyscape-017",
        "at risk: easy: pass 1 affordable on 5 turns over 8 games, never played; hard: pass 1 affordable on 12 turns over 8 games, never played",
    ),
    (
        "stinkyscape-019",
        "at risk: easy: pass 1 affordable on 24 turns over 8 games, played 1 time(s), mean evaluate change -2.6; hard: pass 1 affordable on 7 turns over 8 games, never played; pass 2 affordable on 38 turns over 24 games, played 1 time(s), mean evaluate change 0.8",
    ),
    (
        "stinkyscape-021",
        "at risk: hard: pass 1 affordable on 17 turns over 8 games, never played",
    ),
    (
        "stinkyscape-022",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, never played",
    ),
    (
        "stinkyscape-024",
        "at risk: easy: pass 1 affordable on 18 turns over 8 games, played 4 time(s), mean evaluate change -61.8",
    ),
    (
        "stinkyscape-032",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 100.3",
    ),
    (
        "stinkyscape-036",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, never played",
    ),
    (
        "stinkyscape-039",
        "at risk: hard: pass 1 affordable on 4 turns over 8 games, never played; pass 2 affordable on 28 turns over 24 games, played 1 time(s), mean evaluate change 3.9",
    ),
    (
        "stinkyscape-040",
        "at risk: hard: pass 1 affordable on 24 turns over 8 games, played 1 time(s), mean evaluate change 2.5; pass 2 affordable on 29 turns over 24 games, played 1 time(s), mean evaluate change 3.5",
    ),
    (
        "stinkyscape-046",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 7.3",
    ),
    (
        "stinkyscape-047",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 24.2; hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 0.2",
    ),
    (
        "stinkyscape-049",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 17.8",
    ),
    (
        "stinkyscape-052",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 2.4",
    ),
    (
        "stinkyscape-055",
        "at risk: easy: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 4.1; hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 4.1",
    ),
    (
        "stinkyscape-058",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 7.4",
    ),
    (
        "stinkyscape-059",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 5.8",
    ),
    (
        "stinkyscape-061",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 5.8",
    ),
    (
        "stinkyscape-063",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 0.7",
    ),
    (
        "stinkyscape-064",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 2.4",
    ),
    (
        "stinkyscape-065",
        "at risk: easy: pass 1 affordable on 21 turns over 8 games, played 1 time(s), mean evaluate change 10.1; hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 4.1",
    ),
    (
        "stinkyscape-066",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 13.3; hard: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 5.8",
    ),
    (
        "stinkyscape-071",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 21.3",
    ),
    (
        "stinkyscape-074",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 0.8",
    ),
    (
        "stinkyscape-076",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 11.5",
    ),
    (
        "stinkyscape-080",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 5.7",
    ),
    (
        "stinkyscape-082",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change 29.0",
    ),
    (
        "stinkyscape-083",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, never played",
    ),
    (
        "stinkyscape-084",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, never played",
    ),
    (
        "stinkyscape-085",
        "at risk: easy: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change 5.5",
    ),
    (
        "stinkyscape-086",
        "at risk: easy: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 3.7; hard: pass 1 affordable on 8 turns over 8 games, played 2 time(s), mean evaluate change -41.9",
    ),
    (
        "stinkyscape-087",
        "at risk: easy: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 10.2",
    ),
    (
        "stinkyscape-088",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 17.4",
    ),
    (
        "stinkyscape-089",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "stinkyscape-093",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change 19.9",
    ),
    (
        "stinkyscape-095",
        "at risk: hard: pass 1 affordable on 16 turns over 8 games, never played",
    ),
    (
        "stinkyscape-096",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change -19.8; hard: pass 1 affordable on 16 turns over 8 games, never played",
    ),
    (
        "stinkyscape-104",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 4.4",
    ),
    (
        "stinkyscape-109",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 26.7",
    ),
    (
        "stinkyscape-114",
        "at risk: hard: pass 1 affordable on 11 turns over 8 games, never played",
    ),
    (
        "stinkyscape-115",
        "at risk: easy: pass 1 affordable on 12 turns over 8 games, never played; hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 21.9",
    ),
    (
        "stinkyscape-117",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, never played; hard: pass 1 affordable on 12 turns over 8 games, never played",
    ),
    (
        "stinkyscape-118",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 149.7; hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "stinkyscape-119",
        "at risk: easy: pass 1 affordable on 21 turns over 8 games, played 3 time(s), mean evaluate change -33.1; hard: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change 28.2",
    ),
    (
        "stinkyscape-120",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, never played; pass 2 affordable on 42 turns over 24 games, played 1 time(s), mean evaluate change 10.0",
    ),
    (
        "stinkyscape-121",
        "at risk: easy: pass 1 affordable on 19 turns over 8 games, played 1 time(s), mean evaluate change 0.6",
    ),
    (
        "stinkyscape-124",
        "at risk: easy: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change 2.9",
    ),
    (
        "stinkyscape-125",
        "at risk: hard: pass 1 affordable on 12 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "stinkyscape-126",
        "at risk: easy: pass 1 affordable on 15 turns over 8 games, played 1 time(s), mean evaluate change 1.4; hard: pass 1 affordable on 20 turns over 8 games, never played",
    ),
    (
        "stinkyscape-127",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 16.7",
    ),
    (
        "stinkyscape-128",
        "at risk: hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 14.1",
    ),
    (
        "stinkyscape-130",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "stinkyscape-131",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 7.6",
    ),
    (
        "stinkyscape-132",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, never played",
    ),
    (
        "stinkyscape-133",
        "at risk: hard: pass 1 affordable on 15 turns over 8 games, never played",
    ),
    (
        "stinkyscape-134",
        "at risk: hard: pass 1 affordable on 10 turns over 8 games, played 1 time(s), mean evaluate change 3.1",
    ),
    (
        "stinkyscape-141",
        "at risk: easy: pass 1 affordable on 20 turns over 8 games, played 1 time(s), mean evaluate change 2.4",
    ),
    (
        "stinkyscape-144",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "stinkyscape-145",
        "at risk: easy: pass 1 affordable on 19 turns over 8 games, never played",
    ),
    (
        "stinkyscape-150",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "stinkyscape-151",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 44.7; hard: pass 1 affordable on 7 turns over 8 games, played 2 time(s), mean evaluate change -24.7",
    ),
    (
        "stinkyscape-152",
        "at risk: easy: pass 1 affordable on 14 turns over 8 games, played 1 time(s), mean evaluate change 2.5; hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change -2.9",
    ),
    (
        "stinkyscape-155",
        "at risk: hard: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change -0.1; pass 2 affordable on 23 turns over 24 games, played 1 time(s), mean evaluate change -1.1",
    ),
    (
        "stinkyscape-156",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 0.3",
    ),
    (
        "stinkyscape-157",
        "at risk: easy: pass 1 affordable on 13 turns over 8 games, played 1 time(s), mean evaluate change 2.1",
    ),
    (
        "stinkyscape-158",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 23.4",
    ),
    (
        "stinkyscape-159",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 5.7; hard: pass 1 affordable on 4 turns over 8 games, never played",
    ),
    (
        "stinkyscape-161",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, never played",
    ),
    (
        "stinkyscape-162",
        "at risk: hard: pass 1 affordable on 10 turns over 8 games, never played",
    ),
    (
        "stinkyscape-163",
        "at risk: easy: pass 2 affordable on 46 turns over 24 games, played 7 time(s), mean evaluate change -21.1; hard: pass 1 affordable on 15 turns over 8 games, played 1 time(s), mean evaluate change 0.6",
    ),
    (
        "stinkyscape-164",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "stinkyscape-165",
        "at risk: easy: pass 1 affordable on 6 turns over 8 games, never played; hard: pass 1 affordable on 3 turns over 8 games, never played; pass 2 affordable on 21 turns over 24 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "stinkyscape-166",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, never played; hard: pass 1 affordable on 20 turns over 8 games, never played",
    ),
    (
        "stinkyscape-167",
        "at risk: hard: pass 1 affordable on 3 turns over 8 games, never played; pass 2 affordable on 59 turns over 41 games, played 1 time(s), mean evaluate change -1.4",
    ),
    (
        "stinkyscape-171",
        "at risk: hard: pass 1 affordable on 29 turns over 8 games, never played",
    ),
    (
        "stinkyscape-172",
        "at risk: hard: pass 1 affordable on 13 turns over 8 games, played 1 time(s), mean evaluate change 7.8; pass 2 affordable on 37 turns over 24 games, played 1 time(s), mean evaluate change 7.8",
    ),
    (
        "stinkyscape-173",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, never played",
    ),
    (
        "stinkyscape-174",
        "at risk: easy: pass 1 affordable on 7 turns over 8 games, never played; hard: pass 2 affordable on 36 turns over 41 games, played 1 time(s), mean evaluate change 2.5",
    ),
    (
        "stinkyscape-175",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 3.1",
    ),
    (
        "traininggrounds-007",
        "at risk: hard: pass 1 affordable on 5 turns over 8 games, played 1 time(s), mean evaluate change 10.7",
    ),
    (
        "traininggrounds-012",
        "at risk: hard: pass 1 affordable on 14 turns over 8 games, played 1 time(s), mean evaluate change 4.0",
    ),
    (
        "traininggrounds-013",
        "at risk: hard: pass 1 affordable on 13 turns over 8 games, never played",
    ),
    (
        "traininggrounds-016",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, never played",
    ),
    (
        "traininggrounds-017",
        "at risk: hard: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change 8.8",
    ),
    (
        "traininggrounds-019",
        "at risk: easy: pass 1 affordable on 3 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "traininggrounds-020",
        "at risk: easy: pass 1 affordable on 21 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "traininggrounds-031",
        "at risk: hard: pass 1 affordable on 17 turns over 8 games, never played",
    ),
    (
        "traininggrounds-035",
        "at risk: easy: pass 1 affordable on 15 turns over 8 games, never played; hard: pass 1 affordable on 6 turns over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "traininggrounds-036",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, never played; hard: pass 1 affordable on 7 turns over 8 games, never played",
    ),
    (
        "traininggrounds-037",
        "at risk: easy: pass 1 affordable on 8 turns over 8 games, never played; hard: pass 1 affordable on 6 turns over 8 games, never played",
    ),
    (
        "traininggrounds-038",
        "at risk: easy: pass 1 affordable on 4 turns over 8 games, played 1 time(s), mean evaluate change 24.6",
    ),
    (
        "traininggrounds-046",
        "at risk: hard: pass 1 affordable on 7 turns over 8 games, played 1 time(s), mean evaluate change 40.8",
    ),
    (
        "traininggrounds-049",
        "at risk: easy: pass 1 affordable on 11 turns over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "traininggrounds-050",
        "at risk: easy: pass 1 affordable on 17 turns over 8 games, played 1 time(s), mean evaluate change 4.8; hard: pass 1 affordable on 10 turns over 8 games, never played",
    ),
    (
        "traininggrounds-053",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 17.7",
    ),
    (
        "traininggrounds-055",
        "at risk: easy: pass 1 affordable on 9 turns over 8 games, played 1 time(s), mean evaluate change -1.8; hard: pass 1 affordable on 7 turns over 8 games, never played",
    ),
    (
        "traininggrounds-060",
        "at risk: hard: pass 1 affordable on 8 turns over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
];

// The unban lane's dealt-pool model (training/README.md).

/// The lane's unbans dealt into the AI's own decks prefer this hard, on top of
/// `DEALT_Q`'s shaped weight: the whole point of the lane is that the AI meets
/// these cards, so it meets them often.
pub const UNBANNED_PREFER: &[&str] = &[
    "core-051", "core-055", "core-057", "core-082", "core-093", "core-094",
];
pub const UNBANNED_PREFER_BY: f64 = 3.0;

/// `DEALT_Q` is measured against this: the candidate's share of the records the
/// table was fit on (3,583 of 5,360 `self-vs-bin` games). It is also the weight
/// a card the table does not name (a banned id, which the eligible pool never
/// holds anyway) is treated as having.
pub const DEALT_Q_PRIOR: f64 = 0.6685;
/// `(q / DEALT_Q_PRIOR) ^ DEAL_SHAPE_K`, capped at `DEAL_SHAPE_CAP`: the deck
/// builder's per-card weight multiplier when a deal asks for it
/// (`AiDeckOptions::shape`, R976000), which no deal does today.
pub const DEAL_SHAPE_K: f64 = 20.0;
pub const DEAL_SHAPE_CAP: f64 = 12.0;
/// The mulligan keeps at most one card costing `keep_max_cost + 1`, and only
/// when its shaped weight reaches this (mulligan.rs).
pub const DEAL_MULLIGAN_KEEP: f64 = 2.0;

/// Per defId, win share with card in deck shrunk toward `DEALT_Q_PRIOR`. Absent ids price as `DEALT_Q_PRIOR`.
pub const DEALT_Q: &[(&str, f64)] = &[
    ("classic-001", 0.5742),
    ("classic-002", 0.5418),
    ("classic-003", 0.5765),
    ("classic-004", 0.6852),
    ("classic-005", 0.6457),
    ("classic-006", 0.5784),
    ("classic-007", 0.5870),
    ("classic-008", 0.5091),
    ("classic-009", 0.5555),
    ("classic-010", 0.7508),
    ("classic-011", 0.6824),
    ("classic-012", 0.7491),
    ("classic-013", 0.7155),
    ("classic-014", 0.6427),
    ("classic-015", 0.5589),
    ("classic-016", 0.5405),
    ("classic-017", 0.6377),
    ("classic-018", 0.5832),
    ("classic-019", 0.6591),
    ("classic-020", 0.5342),
    ("classic-021", 0.6838),
    ("classic-022", 0.5618),
    ("classic-023", 0.7202),
    ("classic-024", 0.7363),
    ("classic-025", 0.6024),
    ("classic-026", 0.5532),
    ("classic-027", 0.5217),
    ("classic-028", 0.5289),
    ("classic-029", 0.6450),
    ("classic-030", 0.5692),
    ("classic-031", 0.6509),
    ("classic-032", 0.5929),
    ("classic-033", 0.5356),
    ("classic-034", 0.7417),
    ("classic-035", 0.5910),
    ("classic-036", 0.7086),
    ("classic-037", 0.5701),
    ("classic-038", 0.6146),
    ("classic-039", 0.6388),
    ("classic-040", 0.6227),
    ("classic-041", 0.7400),
    ("classic-042", 0.6424),
    ("classic-043", 0.5990),
    ("classic-044", 0.6024),
    ("classic-045", 0.6028),
    ("classic-046", 0.6249),
    ("classic-047", 0.7138),
    ("classic-048", 0.6641),
    ("classic-049", 0.6279),
    ("classic-050", 0.5701),
    ("classic-051", 0.6413),
    ("classic-052", 0.7221),
    ("classic-053", 0.6176),
    ("classic-054", 0.5760),
    ("classic-055", 0.5344),
    ("classic-056", 0.6272),
    ("classic-057", 0.7033),
    ("classic-058", 0.6855),
    ("classic-059", 0.5444),
    ("classic-060", 0.5728),
    ("classic-061", 0.6783),
    ("classic-062", 0.6703),
    ("classic-063", 0.5585),
    ("classic-064", 0.5409),
    ("classic-065", 0.6769),
    ("classic-066", 0.5417),
    ("classic-067", 0.6714),
    ("classic-068", 0.7245),
    ("classic-069", 0.5477),
    ("classic-070", 0.5878),
    ("classic-071", 0.5777),
    ("classic-072", 0.6183),
    ("classic-073", 0.7252),
    ("classic-074", 0.7376),
    ("classic-075", 0.5622),
    ("classic-076", 0.7476),
    ("classic-077", 0.6858),
    ("classic-078", 0.5304),
    ("classic-079", 0.5393),
    ("classic-080", 0.6412),
    ("classic-081", 0.7454),
    ("classic-082", 0.5960),
    ("classic-083", 0.6963),
    ("classic-084", 0.5905),
    ("classic-085", 0.5333),
    ("classic-086", 0.5772),
    ("classic-087", 0.5239),
    ("classic-088", 0.7978),
    ("classic-089", 0.5779),
    ("classic-090", 0.6430),
    ("classicplus-001", 0.5397),
    ("classicplus-002", 0.5939),
    ("classicplus-003", 0.5385),
    ("classicplus-004", 0.5389),
    ("classicplus-005", 0.6746),
    ("classicplus-006", 0.5203),
    ("classicplus-007", 0.6425),
    ("classicplus-008", 0.6810),
    ("classicplus-009", 0.5624),
    ("classicplus-010", 0.7480),
    ("classicplus-011", 0.5960),
    ("classicplus-012", 0.6876),
    ("classicplus-013", 0.6929),
    ("classicplus-014", 0.6662),
    ("classicplus-015", 0.5536),
    ("classicplus-016", 0.6316),
    ("classicplus-017", 0.5882),
    ("classicplus-018", 0.5538),
    ("classicplus-019", 0.7776),
    ("classicplus-020", 0.7421),
    ("classicplus-021", 0.5923),
    ("classicplus-022", 0.5995),
    ("classicplus-023", 0.7148),
    ("classicplus-024", 0.4813),
    ("classicplus-025", 0.7629),
    ("classicplus-026", 0.5873),
    ("classicplus-027", 0.5667),
    ("classicplus-028", 0.6911),
    ("classicplus-029", 0.6590),
    ("classicplus-030", 0.6878),
    ("classicplus-031", 0.5830),
    ("classicplus-032", 0.6707),
    ("classicplus-033", 0.6099),
    ("classicplus-034", 0.6470),
    ("classicplus-035", 0.5146),
    ("classicplus-036", 0.6650),
    ("classicplus-037", 0.6256),
    ("classicplus-038", 0.6073),
    ("classicplus-039", 0.6778),
    ("classicplus-040", 0.7646),
    ("classicplus-041", 0.5764),
    ("classicplus-042", 0.5639),
    ("classicplus-043", 0.5442),
    ("classicplus-044", 0.7270),
    ("classicplus-045", 0.6099),
    ("classicplus-046", 0.6281),
    ("classicplus-047", 0.6553),
    ("classicplus-048", 0.6890),
    ("classicplus-049", 0.5049),
    ("classicplus-050", 0.7353),
    ("classicplus-051", 0.7131),
    ("classicplus-052", 0.6830),
    ("classicplus-053", 0.6694),
    ("classicplus-054", 0.6943),
    ("classicplus-055", 0.6398),
    ("classicplus-056", 0.5959),
    ("classicplus-057", 0.5828),
    ("classicplus-058", 0.6319),
    ("classicplus-059", 0.6798),
    ("classicplus-060", 0.7663),
    ("classicplus-061", 0.6777),
    ("classicplus-062", 0.6399),
    ("classicplus-063", 0.6654),
    ("classicplus-064", 0.6809),
    ("classicplus-065", 0.7168),
    ("classicplus-066", 0.7168),
    ("classicplus-067", 0.7415),
    ("classicplus-068", 0.6733),
    ("classicplus-069", 0.6687),
    ("classicplus-070", 0.6966),
    ("classicplus-071", 0.7459),
    ("classicplus-072", 0.5405),
    ("classicplus-073", 0.6601),
    ("classicplus-074", 0.5202),
    ("classicplus-075", 0.7637),
    ("classicplus-076", 0.7001),
    ("classicplus-077", 0.5038),
    ("classicplus-078", 0.6819),
    ("core-001", 0.5418),
    ("core-002", 0.5847),
    ("core-003", 0.6111),
    ("core-004", 0.5837),
    ("core-005", 0.6608),
    ("core-006", 0.6432),
    ("core-007", 0.5767),
    ("core-008", 0.5484),
    ("core-009", 0.6507),
    ("core-010", 0.6491),
    ("core-011", 0.6801),
    ("core-012", 0.6664),
    ("core-013", 0.7489),
    ("core-014", 0.6265),
    ("core-015", 0.7181),
    ("core-016", 0.6345),
    ("core-017", 0.6114),
    ("core-018", 0.6543),
    ("core-019", 0.6201),
    ("core-020", 0.5783),
    ("core-021", 0.6602),
    ("core-022", 0.5018),
    ("core-023", 0.6221),
    ("core-024", 0.6143),
    ("core-025", 0.6755),
    ("core-026", 0.6239),
    ("core-027", 0.6708),
    ("core-028", 0.6694),
    ("core-029", 0.6487),
    ("core-030", 0.5731),
    ("core-031", 0.6032),
    ("core-032", 0.7376),
    ("core-033", 0.7052),
    ("core-034", 0.5644),
    ("core-035", 0.5365),
    ("core-036", 0.6140),
    ("core-037", 0.5743),
    ("core-038", 0.6925),
    ("core-039", 0.6544),
    ("core-040", 0.6740),
    ("core-041", 0.6321),
    ("core-043", 0.6376),
    ("core-044", 0.6118),
    ("core-045", 0.5809),
    ("core-046", 0.5094),
    ("core-047", 0.6961),
    ("core-048", 0.4890),
    ("core-049", 0.7730),
    ("core-050", 0.6910),
    ("core-051", 0.5604),
    ("core-052", 0.7608),
    ("core-053", 0.6706),
    ("core-054", 0.7093),
    ("core-056", 0.7094),
    ("core-058", 0.7335),
    ("core-059", 0.5570),
    ("core-060", 0.7147),
    ("core-061", 0.6074),
    ("core-062", 0.5476),
    ("core-063", 0.7484),
    ("core-064", 0.5908),
    ("core-065", 0.5793),
    ("core-066", 0.5907),
    ("core-067", 0.7130),
    ("core-068", 0.5265),
    ("core-069", 0.5985),
    ("core-070", 0.5435),
    ("core-071", 0.5790),
    ("core-072", 0.6214),
    ("core-073", 0.7278),
    ("core-074", 0.5815),
    ("core-075", 0.5961),
    ("core-077", 0.6093),
    ("core-079", 0.7159),
    ("core-080", 0.6966),
    ("core-081", 0.5582),
    ("core-082", 0.5967),
    ("core-083", 0.6023),
    ("core-084", 0.6107),
    ("core-085", 0.5268),
    ("core-086", 0.7257),
    ("core-087", 0.5918),
    ("core-088", 0.6719),
    ("core-089", 0.6795),
    ("core-090", 0.5456),
    ("core-091", 0.5465),
    ("core-092", 0.5943),
    ("core-093", 0.6713),
    ("core-095", 0.6333),
    ("core-096", 0.5611),
    ("core-097", 0.6871),
    ("core-098", 0.6957),
    ("core-100", 0.6139),
];

fn lookup(table: &[(&str, f64)], id: &str) -> Option<f64> {
    table
        .binary_search_by(|(entry, _)| (*entry).cmp(id))
        .ok()
        .map(|at| table[at].1)
}

/// `DEALT_Q[id]`, `DEALT_Q_PRIOR` when the table does not name the id.
pub fn dealt_quality(id: &str) -> f64 {
    lookup(DEALT_Q, id).unwrap_or(DEALT_Q_PRIOR)
}

/// The weight `build_ai_deck` deals `id` at when a deal asks for shaping
/// (`AiDeckOptions::shape`, R976000), and the one `mulligan_keep_shaped` reads:
/// the measured quality ratio sharpened and capped, with the lane's unbans
/// preferred another `UNBANNED_PREFER_BY` on top.
pub fn shaped_weight(id: &str) -> f64 {
    let mut weight = (dealt_quality(id) / DEALT_Q_PRIOR)
        .powf(DEAL_SHAPE_K)
        .min(DEAL_SHAPE_CAP);
    if UNBANNED_PREFER.contains(&id) {
        weight *= UNBANNED_PREFER_BY;
    }
    weight
}
