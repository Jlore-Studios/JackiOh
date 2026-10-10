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
///
/// Generation 2 removed `core-057` (Conjure KY): the Rust sweep of record cleared it at both tiers
/// and both passes — its generation-0 `neverPlayed` row measured 4 affordable turns, far under the
/// flag's bar — and pass 2 watched the AI cast it 145 times at easy for +2.2 mean eval delta. Two
/// mana for three cards is exactly the card advantage the mirror's tempo race runs on.
///
/// Generation 3 removed `core-094` (Genn's Greed): the sweep of record cleared it — its
/// `neverPlayed` row measured a card whose draw-and-exile read as pure loss to the eval that
/// priced it, not a misplay. The eval already counts what the cast moves: the drawn two-costs land
/// in hand (`hand_card`), the exiled odd-costs leave library and hand (`library_card`,
/// `hand_card`), so a determinized world shows the whole trade and the beam casts it only when the
/// dealt deck's two-cost density makes it pay.
///
/// Generation 4 removed `core-099` (Craft a Card): the sweep of record cleared it at both tiers,
/// and its `neverPlayed` flag names a structural blind spot rather than a misplay — the two
/// Discover answers sit between the cast and the fused card, so the beam's open-prompt states read
/// as a spent spell with no payoff and prune the line before the 0-cost fused unit lands (a sweep
/// on this build still shows 0 plays in 16 affordable turns). It is not in `UNBANNED_PREFER`: dealt
/// at stock weight it shows up in a few decks, the mulligan returns it like any other 4-cost, and
/// the never-played slot costs a fraction of a draw — the ban it came off measured a deal the
/// beam's shape makes dead, not a card the AI mishandles.
///
/// Generation 5 removed `core-055` (Lava Golem): the only entry the Rust sweep of record would have
/// kept for `timeout`, and the report itself says what that flag measured — a decision over two
/// seconds in a game that also held an R29 scorer (Zephrys Zealotism's perfect hand and Zephyrs'
/// Discover rank dry-run whole pools of cards, "from a card dealt or one a determinization sampled
/// into the opponent's hand, so neither card need be in the game"). The card it was charged to is
/// the one the sweep watched the AI play best: 222 casts at easy for +4545.7 mean eval delta and
/// 168 at hard for +23833.0, every tribute set enumerated by `legal_actions` (R101's own list keeps
/// the enemy units) and every one priced by the same eval that reads the Golem's `controller`
/// whether it lands at home or across the table (R360). Dealt ×3 by `UNBANNED_PREFER` on the same
/// evidence as the generation-2 unban: a card the AI demonstrably plays well belongs in its decks.
///
/// Generation 6 removed `core-078` (/fullsend): its `neverPlayed` row is the same beam-horizon
/// blind spot as generation-4's core-099 — the cast alone buys nothing (4 paid, 3 refreshed, a
/// discount that pays back only across the rest of the line, and a delayed hand-exile the eval
/// reads only once it fires), so no prefix of the line ever outbids the tempo plays beside it.
/// What makes the row a blind spot rather than a misplay is that a cast that did survive selection
/// would be honest: the exile is real engine state, every simulated EndTurn runs it, and an open
/// line is scored with its turn ended — a /fullsend that fails to cash its hand costs itself by
/// the same eval that would have to pick it. Dealt at stock weight and not in `UNBANNED_PREFER`,
/// the never-played slot costs a fraction of a draw where it lands at all.
///
/// Generation 7 removed `core-042` (Eugenics): the Rust sweep of record cleared it at both
/// tiers and both passes — pass 2 watched the AI cast it 34 times at easy for -2.3 mean
/// evaluate change and 30 at hard for -0.4, so the generation-0 `neverPlayed` row measured a
/// deal an older AI's beam never made worth casting, not a card this one mishandles. The casts
/// it does choose are honest on the terms the eval sees: the exiled seven leave the library
/// `library_card` counts, and the line is picked only where the cast outbids the tempo play
/// beside it; the Radiant flags themselves are the part no weight prices, the same blind spot
/// generation 4 and 6 named, so most deals still end with it unplayed. Not in
/// `UNBANNED_PREFER`, dealt at stock weight (`DEALT_Q` has no row for it, so it keeps
/// `DEALT_Q_PRIOR`): preferring it would deal a mostly-dead slot three times as often.
pub const SHADOW_BAN: &[(&str, &str)] = &[(
    "core-076",
    "neverPlayed: hard: affordable in hand on 15 turns, never played",
)];

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
/// its measured weight like any other card. `core-057` joins them in
/// generation 2 on the sweep's numbers (played 145 times at easy, +2.2 mean
/// eval delta) — card advantage is what the tempo mirror rewards. `core-094`
/// joins them in generation 3: the sweep cleared it, and the AI meets the card
/// the table now teaches. `core-099` is not preferred either: the beam prunes
/// its Discover chain before the fused payoff lands (the generation-4 note
/// above the ban table), so preferring it would only deal a dead card more.
/// `core-055` joins them in generation 5 on the strongest numbers the sweep of
/// record took (222 casts at easy for +4545.7, 168 at hard for +23833.0): the
/// AI already plays the Golem well, so it should meet it often. `core-078` is
/// not preferred on the generation-4 precedent: its refresh, discount and
/// hand-exile pay only across a whole dumped hand (the generation-6 note above
/// the ban table), so preferring it would deal the dead slot three times as
/// often. `core-042` is not preferred on the same precedent (the generation-7
/// note): the sweep watched the casts it makes land near neutral, so the extra
/// deals would buy nothing.
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
