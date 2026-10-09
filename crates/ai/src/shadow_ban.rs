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
//! sweep` on `7e91089`, pass 1 over 268 card(s) at easy and hard, 8 seeds each
//! (`sweep:<tier>:<id>:<n>`), pass 2 over 91 at-risk card(s), 24 seeds each
//! (`sweep2:<tier>:<id>:<n>`, at-risk filler ×4), budget AI_GATE_BUDGET as above, in four parallel
//! slices per pass. It filled `SHADOW_WATCH` below (67 cards). `SHADOW_BAN` above stays generation 0's,
//! TypeScript's eleven (#306): that sweep would ban 25 cards, three of the eleven among them and 22 of
//! the 25 for `timeout` (decisions that resolve R29's dry-run scorers, and one turn that never ends),
//! and clear the other eight. It replaces a first run of the same day (`fa03797`), whose 63 timeouts
//! were fused scripts composed on every lookup. training/history/sweep-2026-10-07.md lists them, with
//! what the timeouts measured, as the unban lane's starting notes.
//!
//! Port of `packages/ai/src/shadowBan.ts` (SURFACE §9): TS's `Record<string, string>` is a slice of
//! `(defId, reason)` pairs sorted by id, the order `Object.keys` gave it.

/// R186: defId → why the AI never deals it to itself. Each reason starts "<SweepFlag>: <tier>: ".
/// Sorted by id.
///
/// Generation 1 (the unban lane) removed four entries, all generation-0 `neverPlayed` bans the Rust
/// sweep of record cleared or explained:
/// - `core-051` (KY's Private Tutor) and `core-082` (KY's Trial): cheap units the 2026-10-07 sweep
///   cleared at both tiers and both passes; the never-played flag measured a deal the sweep's
///   forced-filler bias never made worth casting, not a card the AI misplays.
/// - `core-091` (Fed Fauci): its stored-mana engine lives in Plague Counters, which `evaluate` now
///   prices (`AI_EVAL.plague_counter`); it was invisible to the eval that kept passing it over.
/// - `core-093` (Combo-Index): its grade cascade is now priced too (`AI_EVAL.grade_counter` per
///   banked step, plus the step an armed rise is about to bank on the controller's own turn), so the
///   beam sees the snowball it would otherwise give away.
pub const SHADOW_BAN: &[(&str, &str)] = &[
    (
        "core-042",
        "neverPlayed: hard: affordable in hand on 21 turns, never played",
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
];

// ---------------------------------------------------------------------------
// The unban lane's dealt-pool model (training/README.md): what each seat's deck
// can hold, learned from the lane's own game records. Generation 1's.
// ---------------------------------------------------------------------------

/// The lane's unbans dealt into the AI's own decks prefer this hard, on top of
/// `DEALT_Q`'s shaped weight: the whole point of the lane is that the AI meets
/// these cards, so it meets them often. `core-091` is not preferred on purpose:
/// the records show the AI losing the games it casts Fauci in, so it deals at
/// its measured weight like any other card.
pub const UNBANNED_PREFER: &[&str] = &["core-051", "core-082", "core-093"];
pub const UNBANNED_PREFER_BY: f64 = 3.0;

/// `DEALT_Q` is measured against this: the candidate's share of the records the
/// table was fit on (2,656 of 4,220 `self-vs-bin` games). It is also the weight
/// a card the table does not name (a banned id, which the eligible pool never
/// holds anyway) is treated as having.
pub const DEALT_Q_PRIOR: f64 = 0.6294;
/// `(q / DEALT_Q_PRIOR) ^ DEAL_SHAPE_K`, capped at `DEAL_SHAPE_CAP`: the deck
/// builder's per-card weight multiplier when it deals a seat this AI's own
/// shadow ban applies to (deck.rs's `shape` gate).
pub const DEAL_SHAPE_K: f64 = 20.0;
pub const DEAL_SHAPE_CAP: f64 = 12.0;
/// The mulligan keeps at most one card costing `keep_max_cost + 1`, and only
/// when its shaped weight reaches this (mulligan.rs).
pub const DEAL_MULLIGAN_KEEP: f64 = 2.0;

/// Per defId, the share of games the candidate won with it dealt into its deck,
/// shrunk toward `DEALT_Q_PRIOR` (`(w + 60 x prior) / (n + 60)` over the lane's
/// `self-vs-bin` records). The seven banned ids are absent: a deck dealt under
/// this AI's ban list never holds one. Sorted by id.
pub const DEALT_Q: &[(&str, f64)] = &[
    ("classic-001", 0.5578),
    ("classic-002", 0.5312),
    ("classic-003", 0.5642),
    ("classic-004", 0.6484),
    ("classic-005", 0.6186),
    ("classic-006", 0.5485),
    ("classic-007", 0.5617),
    ("classic-008", 0.4899),
    ("classic-009", 0.5311),
    ("classic-010", 0.7106),
    ("classic-011", 0.6434),
    ("classic-012", 0.6883),
    ("classic-013", 0.6950),
    ("classic-014", 0.6189),
    ("classic-015", 0.5503),
    ("classic-016", 0.5262),
    ("classic-017", 0.6325),
    ("classic-018", 0.5796),
    ("classic-019", 0.6319),
    ("classic-020", 0.5229),
    ("classic-021", 0.6466),
    ("classic-022", 0.5480),
    ("classic-023", 0.6669),
    ("classic-024", 0.6885),
    ("classic-025", 0.6130),
    ("classic-026", 0.5450),
    ("classic-027", 0.4973),
    ("classic-028", 0.5101),
    ("classic-029", 0.6186),
    ("classic-030", 0.5532),
    ("classic-031", 0.6190),
    ("classic-032", 0.5729),
    ("classic-033", 0.5253),
    ("classic-034", 0.6954),
    ("classic-035", 0.5911),
    ("classic-036", 0.6835),
    ("classic-037", 0.5520),
    ("classic-038", 0.6261),
    ("classic-039", 0.6140),
    ("classic-040", 0.5999),
    ("classic-041", 0.7054),
    ("classic-042", 0.6390),
    ("classic-043", 0.5651),
    ("classic-044", 0.5822),
    ("classic-045", 0.5960),
    ("classic-046", 0.5718),
    ("classic-047", 0.6668),
    ("classic-048", 0.6135),
    ("classic-049", 0.6144),
    ("classic-050", 0.5645),
    ("classic-051", 0.6102),
    ("classic-052", 0.6750),
    ("classic-053", 0.5665),
    ("classic-054", 0.5629),
    ("classic-055", 0.5260),
    ("classic-056", 0.5930),
    ("classic-057", 0.6712),
    ("classic-058", 0.6561),
    ("classic-059", 0.5217),
    ("classic-060", 0.5629),
    ("classic-061", 0.6385),
    ("classic-062", 0.6317),
    ("classic-063", 0.5364),
    ("classic-064", 0.5238),
    ("classic-065", 0.6398),
    ("classic-066", 0.5320),
    ("classic-067", 0.6264),
    ("classic-068", 0.6801),
    ("classic-069", 0.5310),
    ("classic-070", 0.5613),
    ("classic-071", 0.5611),
    ("classic-072", 0.6077),
    ("classic-073", 0.6751),
    ("classic-074", 0.6823),
    ("classic-075", 0.5534),
    ("classic-076", 0.7036),
    ("classic-077", 0.6619),
    ("classic-078", 0.5164),
    ("classic-079", 0.5213),
    ("classic-080", 0.6219),
    ("classic-081", 0.6966),
    ("classic-082", 0.5857),
    ("classic-083", 0.6574),
    ("classic-084", 0.5702),
    ("classic-085", 0.5259),
    ("classic-086", 0.5611),
    ("classic-087", 0.5103),
    ("classic-088", 0.7485),
    ("classic-089", 0.5669),
    ("classic-090", 0.6115),
    ("classicplus-001", 0.5283),
    ("classicplus-002", 0.5742),
    ("classicplus-003", 0.5250),
    ("classicplus-004", 0.5321),
    ("classicplus-005", 0.6675),
    ("classicplus-006", 0.5110),
    ("classicplus-007", 0.6050),
    ("classicplus-008", 0.6365),
    ("classicplus-009", 0.5443),
    ("classicplus-010", 0.7044),
    ("classicplus-011", 0.5784),
    ("classicplus-012", 0.6484),
    ("classicplus-013", 0.6621),
    ("classicplus-014", 0.6438),
    ("classicplus-015", 0.5330),
    ("classicplus-016", 0.6056),
    ("classicplus-017", 0.5735),
    ("classicplus-018", 0.5378),
    ("classicplus-019", 0.7288),
    ("classicplus-020", 0.6837),
    ("classicplus-021", 0.5827),
    ("classicplus-022", 0.5735),
    ("classicplus-023", 0.6695),
    ("classicplus-024", 0.4700),
    ("classicplus-025", 0.7014),
    ("classicplus-026", 0.5742),
    ("classicplus-027", 0.5573),
    ("classicplus-028", 0.6727),
    ("classicplus-029", 0.5989),
    ("classicplus-030", 0.6392),
    ("classicplus-031", 0.5717),
    ("classicplus-032", 0.6500),
    ("classicplus-033", 0.5896),
    ("classicplus-034", 0.6006),
    ("classicplus-035", 0.5015),
    ("classicplus-036", 0.6246),
    ("classicplus-037", 0.6048),
    ("classicplus-038", 0.5881),
    ("classicplus-039", 0.6407),
    ("classicplus-040", 0.7098),
    ("classicplus-041", 0.5754),
    ("classicplus-042", 0.5538),
    ("classicplus-043", 0.5257),
    ("classicplus-044", 0.6694),
    ("classicplus-045", 0.5879),
    ("classicplus-046", 0.6023),
    ("classicplus-047", 0.6247),
    ("classicplus-048", 0.6573),
    ("classicplus-049", 0.4945),
    ("classicplus-050", 0.6927),
    ("classicplus-051", 0.6788),
    ("classicplus-052", 0.6526),
    ("classicplus-053", 0.6359),
    ("classicplus-054", 0.6525),
    ("classicplus-055", 0.5999),
    ("classicplus-056", 0.5752),
    ("classicplus-057", 0.5603),
    ("classicplus-058", 0.5982),
    ("classicplus-059", 0.6349),
    ("classicplus-060", 0.7209),
    ("classicplus-061", 0.6501),
    ("classicplus-062", 0.6527),
    ("classicplus-063", 0.6261),
    ("classicplus-064", 0.6319),
    ("classicplus-065", 0.6817),
    ("classicplus-066", 0.6873),
    ("classicplus-067", 0.6875),
    ("classicplus-068", 0.6446),
    ("classicplus-069", 0.6323),
    ("classicplus-070", 0.6641),
    ("classicplus-071", 0.7065),
    ("classicplus-072", 0.5216),
    ("classicplus-073", 0.6243),
    ("classicplus-074", 0.5050),
    ("classicplus-075", 0.7240),
    ("classicplus-076", 0.6783),
    ("classicplus-077", 0.4892),
    ("classicplus-078", 0.6384),
    ("core-001", 0.5409),
    ("core-002", 0.5662),
    ("core-003", 0.5891),
    ("core-004", 0.5625),
    ("core-005", 0.6141),
    ("core-006", 0.6324),
    ("core-007", 0.5682),
    ("core-008", 0.5375),
    ("core-009", 0.6288),
    ("core-010", 0.6414),
    ("core-011", 0.6398),
    ("core-012", 0.6429),
    ("core-013", 0.7046),
    ("core-014", 0.5999),
    ("core-015", 0.6918),
    ("core-016", 0.6175),
    ("core-017", 0.6095),
    ("core-018", 0.6157),
    ("core-019", 0.6079),
    ("core-020", 0.5615),
    ("core-021", 0.6410),
    ("core-022", 0.4889),
    ("core-023", 0.5986),
    ("core-024", 0.5850),
    ("core-025", 0.6539),
    ("core-026", 0.5982),
    ("core-027", 0.6550),
    ("core-028", 0.6427),
    ("core-029", 0.6142),
    ("core-030", 0.5645),
    ("core-031", 0.5954),
    ("core-032", 0.6915),
    ("core-033", 0.6563),
    ("core-034", 0.5474),
    ("core-035", 0.5197),
    ("core-036", 0.5913),
    ("core-037", 0.5637),
    ("core-038", 0.6483),
    ("core-039", 0.6271),
    ("core-040", 0.6419),
    ("core-041", 0.5919),
    ("core-043", 0.6070),
    ("core-044", 0.5791),
    ("core-045", 0.5551),
    ("core-046", 0.4957),
    ("core-047", 0.6504),
    ("core-048", 0.4782),
    ("core-049", 0.7115),
    ("core-050", 0.6440),
    ("core-051", 0.5491),
    ("core-052", 0.7217),
    ("core-053", 0.6322),
    ("core-054", 0.6805),
    ("core-056", 0.6541),
    ("core-058", 0.6766),
    ("core-059", 0.5415),
    ("core-060", 0.6506),
    ("core-061", 0.5913),
    ("core-062", 0.5371),
    ("core-063", 0.7225),
    ("core-064", 0.5586),
    ("core-065", 0.5626),
    ("core-066", 0.5841),
    ("core-067", 0.6613),
    ("core-068", 0.5171),
    ("core-069", 0.5901),
    ("core-070", 0.5243),
    ("core-071", 0.5648),
    ("core-072", 0.5941),
    ("core-073", 0.6658),
    ("core-074", 0.5648),
    ("core-075", 0.5775),
    ("core-077", 0.5884),
    ("core-079", 0.6877),
    ("core-080", 0.6597),
    ("core-081", 0.5469),
    ("core-082", 0.5785),
    ("core-083", 0.5472),
    ("core-084", 0.5822),
    ("core-085", 0.5066),
    ("core-086", 0.6663),
    ("core-087", 0.5860),
    ("core-088", 0.6339),
    ("core-089", 0.6397),
    ("core-090", 0.5294),
    ("core-091", 0.5403),
    ("core-092", 0.5782),
    ("core-093", 0.6348),
    ("core-095", 0.6043),
    ("core-096", 0.5444),
    ("core-097", 0.6555),
    ("core-098", 0.6564),
    ("core-100", 0.5943),
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

/// The weight `build_ai_deck` deals `id` at in a seat under this AI's own
/// shadow ban: the measured quality ratio sharpened and capped, with the lane's
/// unbans preferred another `UNBANNED_PREFER_BY` on top.
pub fn shaped_weight(id: &str) -> f64 {
    let mut weight = (dealt_quality(id) / DEALT_Q_PRIOR)
        .powf(DEAL_SHAPE_K)
        .min(DEAL_SHAPE_CAP);
    if UNBANNED_PREFER.contains(&id) {
        weight *= UNBANNED_PREFER_BY;
    }
    weight
}
