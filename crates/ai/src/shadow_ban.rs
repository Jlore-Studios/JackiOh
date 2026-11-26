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
// Report of record: training/history/sweep-2026-11-13.md
// Sweep of record: 2026-11-13 (UTC), `cargo jackioh sweep`, pass 1 over 166 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 70 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":1200,"lethalNodes":300,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// Report of record: training/history/sweep-2026-11-13-2.md
// Sweep of record: 2026-11-13 (UTC), `cargo jackioh sweep`, pass 1 over 4 card(s) at easy and hard, 8 seeds each (`sweep:<tier>:<id>:<n>`), pass 2 over 4 at-risk card(s), 24 seeds each (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET {"nodes":1200,"lethalNodes":300,"determinizations":3,"beamWidth":4,"rootBranching":20,"branching":6,"maxDepth":8,"finalists":3}.
// END `cargo jackioh sweep --write`

/// R186: defId → why the AI never deals it to itself. Each reason starts "<SweepFlag>: <tier>: ".
/// Sorted by id.
///
/// Empty from #2521, which lifted its last two rows, S #149 Wildar (#2358) and TG #25 Trigor (#2397),
/// both `neverPlayed`, through the unban lane (`SHADOW_UNBANNED`, R2521010, #2452): a banned card the
/// beam never casts is a dead slot costing a fraction of a draw. Fullsend 2's sweep of the Stinkyscape+
/// set (#3574, R3574000) wrote four rows; fullsend 3 (#3575, R3575000) cleared S+ #47 and #163 by a sweep
/// of record and lifted #81 and #157 through the unban lane. Each generation's lifts and their evidence
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
/// itself every card of every set, #2452; #3575: `stinkyscapeplus-081` Doomsayer and
/// `stinkyscapeplus-157` Phantom Shipyard, `neverPlayed` at Hard after fullsend 3's fixes, lifted at the
/// owner's call, R3575000). `cargo jackioh sweep --write` leaves their rows out of
/// `SHADOW_BAN`, and refuses an id no report of record bans. The lane unbans a card by adding its id
/// here and running `cargo jackioh sweep --write`; a new sweep of a set drops its own ids from this
/// list. An id may be any card of a set the ban takes: one whose row of the set table gives it a
/// shadow-ban share (`shadow_ban_share`, `crates/engine/src/wire/catalog_types.rs`, R3114001).
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
    "stinkyscapeplus-081",
    "stinkyscapeplus-157",
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
        "stinkyscapeplus-001",
        "at risk: hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 5.8",
    ),
    (
        "stinkyscapeplus-004",
        "at risk: hard: pass 1 affordable on 7 turns and playable on 6 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-005",
        "at risk: hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-008",
        "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change -0.1",
    ),
    (
        "stinkyscapeplus-009",
        "at risk: hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-013",
        "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 2.8",
    ),
    (
        "stinkyscapeplus-015",
        "at risk: hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-023",
        "at risk: hard: pass 1 affordable on 22 turns and playable on 22 over 8 games, played 1 time(s), mean evaluate change 0.1",
    ),
    (
        "stinkyscapeplus-024",
        "at risk: hard: pass 1 affordable on 16 turns and playable on 16 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-026",
        "at risk: easy: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 4.7",
    ),
    (
        "stinkyscapeplus-030",
        "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "stinkyscapeplus-032",
        "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played; pass 2 affordable on 62 turns and playable on 62 over 87 games, played 12 time(s), mean evaluate change 4.0, lost the game on 1 play(s)",
    ),
    (
        "stinkyscapeplus-033",
        "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 12.9; hard: pass 1 affordable on 8 turns and playable on 5 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-038",
        "at risk: easy: pass 1 affordable on 6 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 3.4",
    ),
    (
        "stinkyscapeplus-040",
        "at risk: easy: pass 1 affordable on 7 turns and playable on 7 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-041",
        "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-043",
        "at risk: easy: pass 1 affordable on 9 turns and playable on 9 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-046",
        "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 0.0",
    ),
    (
        "stinkyscapeplus-049",
        "at risk: easy: pass 1 affordable on 24 turns and playable on 24 over 8 games, played 1 time(s), mean evaluate change 1.0",
    ),
    (
        "stinkyscapeplus-050",
        "at risk: hard: pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change -0.8",
    ),
    (
        "stinkyscapeplus-052",
        "at risk: easy: pass 1 affordable on 16 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change -6.4; pass 2 affordable on 44 turns and playable on 32 over 57 games, played 1 time(s), mean evaluate change 5.7; hard: pass 1 affordable on 21 turns and playable on 5 over 8 games, never played; pass 2 affordable on 122 turns and playable on 91 over 118 games, played 1 time(s), mean evaluate change -7.5",
    ),
    (
        "stinkyscapeplus-053",
        "at risk: hard: pass 1 affordable on 10 turns and playable on 6 over 8 games, never played; pass 2 affordable on 146 turns and playable on 69 over 120 games, played 1 time(s), mean evaluate change 2.5",
    ),
    (
        "stinkyscapeplus-054",
        "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "stinkyscapeplus-056",
        "at risk: easy: pass 1 affordable on 17 turns and playable on 12 over 8 games, never played; hard: pass 1 affordable on 23 turns and playable on 14 over 8 games, never played; pass 2 affordable on 201 turns and playable on 108 over 147 games, played 1 time(s)",
    ),
    (
        "stinkyscapeplus-057",
        "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-060",
        "at risk: hard: pass 1 affordable on 6 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.2",
    ),
    (
        "stinkyscapeplus-063",
        "at risk: easy: pass 1 affordable on 6 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 1.5; hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "stinkyscapeplus-068",
        "at risk: easy: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change -1.3; hard: pass 1 affordable on 13 turns and playable on 13 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-069",
        "at risk: easy: pass 1 affordable on 16 turns and playable on 17 over 8 games, played 1 time(s), mean evaluate change 7.2; hard: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change -0.6",
    ),
    (
        "stinkyscapeplus-072",
        "at risk: hard: pass 1 affordable on 11 turns and playable on 5 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-073",
        "at risk: hard: pass 1 affordable on 15 turns and playable on 14 over 8 games, played 1 time(s), mean evaluate change 14.6",
    ),
    (
        "stinkyscapeplus-077",
        "at risk: easy: pass 1 affordable on 8 turns and playable on 8 over 8 games, never played; hard: pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 14.8",
    ),
    (
        "stinkyscapeplus-079",
        "at risk: easy: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change -3.2",
    ),
    (
        "stinkyscapeplus-085",
        "at risk: hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 3.4",
    ),
    (
        "stinkyscapeplus-087",
        "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-098",
        "at risk: easy: pass 1 affordable on 8 turns and playable on 8 over 8 games, played 1 time(s), mean evaluate change 1.6; hard: pass 1 affordable on 13 turns and playable on 13 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-103",
        "at risk: easy: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 33.6",
    ),
    (
        "stinkyscapeplus-106",
        "at risk: easy: pass 1 affordable on 6 turns and playable on 4 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-109",
        "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 2 time(s), mean evaluate change -69.8; hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 15.9",
    ),
    (
        "stinkyscapeplus-111",
        "at risk: easy: pass 1 affordable on 7 turns and playable on 6 over 8 games, played 1 time(s), mean evaluate change 5.6",
    ),
    (
        "stinkyscapeplus-113",
        "at risk: hard: pass 1 affordable on 11 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 23.6",
    ),
    (
        "stinkyscapeplus-119",
        "at risk: hard: pass 1 affordable on 9 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 29.1",
    ),
    (
        "stinkyscapeplus-123",
        "at risk: easy: pass 1 affordable on 13 turns and playable on 7 over 8 games, never played; hard: pass 1 affordable on 8 turns and playable on 4 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-125",
        "at risk: hard: pass 1 affordable on 7 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 17.2",
    ),
    (
        "stinkyscapeplus-126",
        "at risk: hard: pass 1 affordable on 18 turns and playable on 13 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-129",
        "at risk: easy: pass 1 affordable on 11 turns and playable on 11 over 8 games, played 1 time(s), mean evaluate change 2.0; hard: pass 1 affordable on 15 turns and playable on 15 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-130",
        "at risk: easy: pass 1 affordable on 5 turns and playable on 4 over 8 games, never played; pass 2 affordable on 70 turns and playable on 63 over 67 games, played 1 time(s), mean evaluate change 4.7; hard: pass 1 affordable on 6 turns and playable on 6 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-131",
        "at risk: easy: pass 2 affordable on 126 turns and playable on 30 over 72 games, played 1 time(s), mean evaluate change -177.1; hard: pass 1 affordable on 7 turns and playable on 3 over 8 games, never played; pass 2 affordable on 128 turns and playable on 45 over 112 games, played 1 time(s)",
    ),
    (
        "stinkyscapeplus-133",
        "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-136",
        "at risk: hard: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.5",
    ),
    (
        "stinkyscapeplus-138",
        "at risk: hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-140",
        "at risk: easy: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change -1.3",
    ),
    (
        "stinkyscapeplus-142",
        "at risk: hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change -2.2",
    ),
    (
        "stinkyscapeplus-143",
        "at risk: hard: pass 1 affordable on 10 turns and playable on 10 over 8 games, played 1 time(s), mean evaluate change 0.4",
    ),
    (
        "stinkyscapeplus-144",
        "at risk: easy: pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 3.1; hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-145",
        "at risk: easy: pass 1 affordable on 10 turns and playable on 10 over 8 games, never played; hard: pass 1 affordable on 3 turns and playable on 3 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-146",
        "at risk: easy: pass 1 affordable on 20 turns and playable on 20 over 8 games, played 1 time(s), mean evaluate change 0.4; hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-147",
        "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 5.9; hard: pass 1 affordable on 7 turns and playable on 7 over 8 games, played 1 time(s), mean evaluate change 1.1",
    ),
    (
        "stinkyscapeplus-148",
        "at risk: easy: pass 1 affordable on 4 turns and playable on 4 over 8 games, played 1 time(s), mean evaluate change 1.8; hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-150",
        "at risk: hard: pass 1 affordable on 12 turns and playable on 12 over 8 games, played 1 time(s), mean evaluate change 4.5",
    ),
    (
        "stinkyscapeplus-151",
        "at risk: hard: pass 1 affordable on 11 turns and playable on 11 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-153",
        "at risk: easy: pass 1 affordable on 15 turns and playable on 15 over 8 games, played 1 time(s), mean evaluate change 5.4; hard: pass 1 affordable on 16 turns and playable on 16 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-155",
        "at risk: hard: pass 1 affordable on 5 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 9.3",
    ),
    (
        "stinkyscapeplus-156",
        "at risk: easy: pass 1 affordable on 3 turns and playable on 3 over 8 games, played 1 time(s), mean evaluate change 8.9",
    ),
    (
        "stinkyscapeplus-160",
        "at risk: hard: pass 1 affordable on 16 turns and playable on 15 over 8 games, never played",
    ),
    (
        "stinkyscapeplus-165",
        "at risk: easy: pass 1 affordable on 6 turns and playable on 5 over 8 games, played 1 time(s), mean evaluate change 16.2",
    ),
    (
        "stinkyscapeplus-166",
        "at risk: hard: pass 1 affordable on 9 turns and playable on 9 over 8 games, played 1 time(s), mean evaluate change 1.4",
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
