//! BUILD M4-T1 and M9-T1 acceptance: catalog.json is the diff against SPEC §8 and the brief.
//!
//! PROVENANCE OF THE FIXTURE TABLES BELOW. Each table is a transcription of a document, not of
//! catalog.json, so that a card whose data drifts from its source fails here. Each was produced
//! mechanically from its document's own lines and checked in as a literal; the test never reads a
//! document at run time (a test that re-parsed one would pass while it and the catalog drifted
//! together, and would put file I/O in a crate that must stay pure — CLAUDE.md rule 4).
//!
//!   CORE           — SPEC §8.1–§8.5, the 105 rows of the catalog table, columns "#", "Name",
//!                    "Rarity", "Cost", "Type, tags" and "Stats" (`A/B → C/D` is base → radiant;
//!                    `A/B` alone means the radiant form keeps those stats; an empty cell means the
//!                    card has no stats), and SPEC §7's six named tokens (`T-rush`, `T-sheep`,
//!                    `T-felinor`, `T-bread`, `T-coin`, the one §2.1's setup deals, R244, and
//!                    `T-ghoul`, a card of its own in patch v0.1.1, R353), whose `rarity: "Token"` and
//!                    Token tag follow §8's in-catalog token rows and §7's token rules. The costs of
//!                    #16 Hit Job (3), #17 Flood, #34 Collateral Damage, #43 Big Felinor, #49 Snom
//!                    Bunny Mind Control, #88 Twisting Nether (4) and #65 Masochism Mask (1) are
//!                    patch v0.2.0's (issue #40).
//!   CLASSIC        — docs/classic-sets.md B6, each entry's header line (`classic-NNN` · (cost)
//!                    type, tags · rarity · stats base → Radiant), with the decisions B9 and the
//!                    v0.2.0 plan adopted: #55 is Book of Wildfire, #72 Grand Counterspell (R381),
//!                    #86 Genn's Radiant 42/42.
//!   CLASSIC_PLUS   — docs/classic-sets.md B7's header lines and token bullets (B2.3's numbering;
//!                    the Losers typed Unit, Otherworldly Removal's three and the Grapes typed Spell
//!                    by their sections' own sentences, the Grapes tagged Fruit, every token tagged
//!                    Token and rated "Token", B2.5) and B8's table of the ten AI generated cards
//!                    (tagged AI). Buff Billy's "3X/3X → 6X/6X" prints 0/0 (its `xStats` carry X).
//!   MEDITATIVE     — docs/meditative-set.md's card headers (M6), with §8.8's later patches.
//!   MEDITATIVE_PLUS — docs/meditative-plus-set.md's card headers (MP6, issue #1178), held (R1420,
//!                    R1178000) until it shipped in v0.3.15.
//!   STINKYSCAPE    — docs/stinkyscape-set.md's card headers (ST13, issue #1857): the set is held
//!                    (R1420, R1857000), so a row waits for its card until the set ships (#1884).
//!   patch v0.3.Y    — issue #1067 renamed Jlockheed to Jlockeed in every name (the briefs keep
//!                    the old spelling as the record).
//!   patch v0.3.10   — issue #831's costs, stats, types and tags, in every set's rows: the briefs'
//!                    lines are what the designer first wrote, the patch what the cards are now.
//!   issue #2278      — Megabase (Stinkyscape 116) is a Unit, without Animated.
//!   patch v0.3.12   — issue #1149's costs, stats and the CN in a Bottle rename, and issue #1219's
//!                    12 Attack for the Radiant Jlockeed Shredder-10.
//!   patch v0.3.13   — issue #1194's costs: #76 Field of Dreams (2), C+ #64 Mulch Muncher (5) and
//!                    Meditative #19.1 Temporal Rift (0).
//!   patch v0.3.16   — issue #1151 tags C #40 MC Tech and Meditative #57, #82, #83 and #85 Human
//!                    (#57's tag ships under patch v0.3.10's claim of that card).
//!   issue #2280    — C+ #74 Twice Forward One Step Backwards and S #16 Rod of Ages print no stats on
//!                    either face (R2280000); the briefs keep the stats they first printed.
//!   issue #2521    — the Stinkyscape balance patch, in §8's rows: Core #84 Going Long and
//!                    Meditative #80 Aluneth lose Quickdraw; C+ #34 Memory Leak (2); S #11 (2), #39
//!                    (3), #163 (0) and #164 (1); S #68 Stratos 3/1 → 6/2; the five Faction cards
//!                    (S #95, #159 to #162) renamed "Faction: …", (2) and tagged Faction; S #137
//!                    Starfleet Commander (was Firststrike Capability) a Legendary Human; the six
//!                    Resources tagged Resource; and the new S #176 Preemptive Strike, S #177
//!                    Cloaking Module and M+ #1.5 Star Chart (§8.9, §8.10, §7).
//!   radiant faces  — every entry has a Radiant face of its own (§5.2, R276), the Ghoul Token's
//!                    being R349's fallback.
//!   census         — each set's cards, tokens and rarities (SPEC §8's rarity paragraph for Core,
//!                    B2.5's table for Classic and Classic+, each later set's brief), read from
//!                    `jackioh_cards::census`, where they are written once (#2581); the totals are
//!                    its sums.
//!   tag vocabulary — SPEC §5/§6 tags as BUILD M4-T1 lists them, R278's Jlockeed, B2.4's Book,
//!                    Pancake and AI, the mechanics patch's Plague (every card that uses
//!                    Plague Counters), and patch v0.2.Y's Catalyst, Prime and Acclaimed.
//!
//! Not asserted here: `keywords`, `text`, `params`, `refs` and `loc` (the per-card tests prove the
//! behavior `keywords`, `text` and `params` describe, references.rs proves `refs`, and `loc` is
//! frozen data since v0.3.0, SURFACE §7.5), and id shape (`cargo jackioh catalog check` owns the
//! schema).
//!
//! Port of `packages/cards/test/catalog.test.ts`. TS's one `it` per fixture row is one test per set
//! here, which checks every row and names every row that fails.

use std::collections::BTreeMap;

use indexmap::IndexMap;
use jackioh_cards::census::{CENSUS, SetCensus};
use jackioh_cards::{CATALOG, chinese_json};
use jackioh_engine::{
    CardCost, CardDef, CardFace, CardType, FaceKind, GLITCH_DEF_ID, KeywordKind, Rarity, SetName, Tag,
    fill_params, set_ships,
};
use serde_json::Value;

use super::sidecar;
use CardType as T;
use Rarity as R;
use Tag as G;

/// `[attack, health]`, or `None` for a §8 Stats cell that is empty (a card with no stats).
type StatPair = (Option<i32>, Option<i32>);

struct SpecRow {
    index: &'static str,
    name: &'static str,
    cost: CardCost,
    type_: CardType,
    tags: &'static [Tag],
    rarity: Rarity,
    base: StatPair,
    radiant: StatPair,
}

#[allow(clippy::too_many_arguments)]
const fn row(
    index: &'static str,
    name: &'static str,
    cost: CardCost,
    type_: CardType,
    tags: &'static [Tag],
    rarity: Rarity,
    base: StatPair,
    radiant: StatPair,
) -> SpecRow {
    SpecRow {
        index,
        name,
        cost,
        type_,
        tags,
        rarity,
        base,
        radiant,
    }
}

const fn c(n: i32) -> CardCost {
    CardCost::Fixed(n)
}

const X: CardCost = CardCost::X;

const fn emb(base: i32, embiggen: i32) -> CardCost {
    CardCost::Embiggen { base, embiggen }
}

const fn st(attack: i32, health: i32) -> StatPair {
    (Some(attack), Some(health))
}

const NO_STATS: StatPair = (None, None);

const CORE: &[SpecRow] = &[
    row(
        "1",
        "Big D-fender",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(0, 7),
        st(0, 14),
    ),
    row(
        "2",
        "Bigot",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 1),
        st(8, 2),
    ),
    row(
        "3",
        "Right-house defender",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "4",
        "Gary the Gambler",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "5",
        "Stockpile",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "6",
        "Mana Well",
        c(3),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "7",
        "Jewelosco Scarab",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "8",
        "Mr. Vanilla",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 4),
        st(12, 12),
    ),
    row(
        "9",
        "Moths to the Flame",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(1, 14),
        st(2, 28),
    ),
    row(
        "10",
        "Rapid Replenish",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Tempo Timmy",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "12",
        "Duplicating Felinors",
        c(2),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "13",
        "Jlockeed Shredder-10",
        c(3),
        T::Unit,
        &[G::Jlockeed],
        R::Common,
        st(6, 10),
        st(12, 20),
    ),
    row(
        "14",
        "Jlockeed's Weapons",
        c(4),
        T::FieldSpell,
        &[G::Jlockeed],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Me and Mr Token",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "16",
        "Hit Job",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row("17", "Flood", c(4), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "18",
        "Bread and Butter",
        c(1),
        T::FieldTrap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Midrange Menace",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "20",
        "Pointmaster",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(7, 1),
        st(14, 2),
    ),
    row("21", "Hinder", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "22",
        "Carnivorous Cube",
        c(3),
        T::Unit,
        &[],
        R::Epic,
        st(4, 6),
        st(8, 12),
    ),
    row(
        "23",
        "Reoccurring Dream",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Efficiency Dividend",
        X,
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "4-mana 7/7",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(7, 7),
        st(14, 14),
    ),
    row(
        "26",
        "Glowy Jelly Bean",
        c(3),
        T::Spell,
        &[G::JellyBean],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Blood Ridden Glowy Jelly Bean",
        c(1),
        T::Spell,
        &[G::JellyBean],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "28",
        "Knockoff Temu Glowy Jelly Bean",
        c(1),
        T::Spell,
        &[G::JellyBean],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "GIGA Glowy Jelly Bean",
        c(6),
        T::Spell,
        &[G::JellyBean],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Archivist",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "31",
        "KY's Math Equation",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32",
        "Prem Panther",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "33",
        "Unstable Clone Machine",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "Collateral Damage",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Lunar Eclipse",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "Magic Jammed",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "Gravedigger",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "38",
        "Quickstriker",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39",
        "Recycling Initiative",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "40",
        "Echoes of the Forgotten",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("41", "Sheepish", c(1), T::Trap, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "42",
        "Eugenics",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "Big Felinor",
        c(4),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 10),
        st(6, 20),
    ),
    row(
        "44",
        "True Strike",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Deft Duelist",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 3),
        st(8, 6),
    ),
    row(
        "46",
        "Suppressive Aura",
        emb(2, 4),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "Fig of Life",
        c(3),
        T::Spell,
        &[G::Fruit],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "48",
        "5pek Controller",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "49",
        "Snom Bunny Mind Control",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "50",
        "K-Pop Fanatic",
        c(2),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "51",
        "KY's Private Tutor",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51.1",
        "KY's Empty Notebook",
        c(1),
        T::Spell,
        &[G::Ky, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "52",
        "Silly Silas",
        c(4),
        T::Unit,
        &[G::Human],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "53",
        "Reno",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 6),
        st(8, 12),
    ),
    row(
        "54",
        "Straaza",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "55",
        "Lava Golem",
        c(3),
        T::Unit,
        &[],
        R::Rare,
        st(10, 5),
        st(20, 10),
    ),
    row(
        "56",
        "Jilliax",
        emb(2, 3),
        T::Unit,
        &[],
        R::Common,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "57",
        "Conjure KY",
        c(2),
        T::Spell,
        &[G::Ky],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "58",
        "Rush Token Farm",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "Unbiased Immigration",
        emb(2, 4),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "60",
        "Bear Honeypot",
        c(1),
        T::Trap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "61",
        "Prejudiced Postdoc",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "62",
        "Friend of Felinors",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Plastic Surgery",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Gifted Program",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65",
        "Masochism Mask",
        c(1),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.1",
        "Spikey Pillow",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 2),
        st(0, 4),
    ),
    row(
        "66",
        "The Rock",
        c(4),
        T::Unit,
        &[G::Human],
        R::Common,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "67",
        "Zoomerbin Oomen",
        c(1),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(1, 2),
        st(2, 4),
    ),
    row(
        "68",
        "Twisted Sorcerer",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "69",
        "Call to Arms",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "70",
        "Spiteful Stab",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71",
        "Intern Stimmy",
        c(1),
        T::FieldTrap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "72",
        "Reminisce",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73",
        "Anti-oneshot Armor",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("74", "Adaptive UI", X, T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "75",
        "Infinite Reserves",
        c(0),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Field of Dreams",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "77",
        "Professor Curvature",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "78",
        "/fullsend",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "79",
        "Twinspell",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "80",
        "Zao Gao",
        c(2),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "81",
        "Radiant Saintess",
        c(2),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "82",
        "KY's Trial",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "83",
        "Transmogulate",
        emb(2, 4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "84",
        "Going Long",
        emb(2, 4),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "85",
        "Unlicensed Experimentation",
        c(2),
        T::Trap,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "86",
        "\"Miss\" Mrow",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "87",
        "Pocket Chaos",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "88",
        "Twisting Nether",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "89",
        "Corpse Eater",
        c(4),
        T::Unit,
        &[],
        R::Epic,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "90",
        "CN-Viral Injection",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "90.1",
        "CN-Virus",
        c(1),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "91",
        "Fed Fauci",
        c(2),
        T::Unit,
        &[G::Human, G::Plague],
        R::Rare,
        st(1, 6),
        st(2, 12),
    ),
    row(
        "92",
        "Felinor Fiender",
        c(2),
        T::Unit,
        &[G::Human],
        R::Legendary,
        st(5, 7),
        st(10, 14),
    ),
    row(
        "93",
        "Combo-Index",
        c(3),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "93.1",
        "Combo-Fodder",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "94",
        "Genn's Greed",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "95",
        "Call to Chaos (Core Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "95.1",
        "Chaos Golem",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row("96", "My Pawn", c(1), T::Trap, &[], R::Mythic, NO_STATS, NO_STATS),
    row(
        "97",
        "Zephrys",
        c(1),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "98",
        "Heroic Power",
        c(0),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "99",
        "Craft a Card",
        c(4),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "100",
        "Ceaseless Void",
        c(100),
        T::Unit,
        &[G::Terminus],
        R::Mythic,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "T-rush",
        "Rush Token",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "T-sheep",
        "Sheep Token",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "T-felinor",
        "Felinor Token",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "T-bread",
        "Bread Token",
        c(0),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 0),
        st(0, 0),
    ),
    row(
        "T-coin",
        "The Coin",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-ghoul",
        "Ghoul Token",
        c(0),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 0),
        st(0, 0),
    ),
];

const CLASSIC: &[SpecRow] = &[
    row(
        "1",
        "Curse of the Forgotten Classic",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "The Trickster",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "3",
        "Book of Heal",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4",
        "Palantir",
        c(2),
        T::FieldSpell,
        &[G::Jlockeed],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row("5", "Tesla", c(2), T::FieldTrap, &[], R::Epic, st(1, 4), st(2, 8)),
    row(
        "6",
        "Cloaked Toe Cracker",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "7",
        "InfiniScepter",
        c(1),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row("8", "Pickle", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "9",
        "Income Tax",
        c(3),
        T::Trap,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row("10", "Exile", c(2), T::Trap, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "11",
        "Mind Melt",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12",
        "Book of Blood",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "13",
        "Boots on the Ground",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "14",
        "Shadowstep",
        c(2),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Nose Hunter",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 1),
        st(6, 2),
    ),
    row(
        "16",
        "Book of Flame",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "17",
        "Counterspell",
        c(2),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18",
        "Glitch in the System",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Lizard's Breath",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20",
        "The Power to Punish",
        c(3),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "21",
        "Turtinator",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "22",
        "Mid Runner",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "23",
        "Devil's Pact",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Book of Knowledge",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Lag in the System",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "26",
        "Rapid Draw",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Pestilent Slime",
        c(0),
        T::Unit,
        &[G::Plague],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "28",
        "Second Wind",
        c(0),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "Book of Vital Kill",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row("30", "Recycle", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "31",
        "Cookie Guild",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "32",
        "Felinor Feelings",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("33", "Joro", c(0), T::Unit, &[], R::Legendary, st(1, 1), st(2, 2)),
    row(
        "34",
        "Ancient Acquisition",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Prep",
        emb(0, 2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row("36", "Burn", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "37",
        "Last Hurrah",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "38",
        "Jackiestan Auctioneer",
        c(2),
        T::FieldTrap,
        &[G::Human],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "39",
        "Outbreak",
        c(1),
        T::Spell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "40",
        "MC Tech",
        c(1),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "41",
        "State of the Game",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "42",
        "Transmutable Toxins",
        c(2),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "Plague Nuke",
        c(4),
        T::Spell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "44",
        "Back from the GY",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Nature Titan",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "46",
        "Divine Favor",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "Recurring Felinor",
        c(2),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "48",
        "Hired Shrimp",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(4, 3),
        st(8, 6),
    ),
    row(
        "49",
        "Anti-Greed Machine",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "50",
        "Voidwalker",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(6, 3),
        st(12, 6),
    ),
    row(
        "51",
        "Back Breaker",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "52",
        "Final Gambit",
        c(2),
        T::Trap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Plague Crawler",
        c(1),
        T::Unit,
        &[G::Plague],
        R::Common,
        st(2, 2),
        st(4, 4),
    ),
    row("54", "Rewind", c(1), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "55",
        "Book of Wildfire",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "Spell Tyrant",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row("57", "Echo", c(1), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "58",
        "Common Resources",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "Plague Doctor",
        c(1),
        T::Unit,
        &[G::Human, G::Plague],
        R::Common,
        st(2, 3),
        st(4, 6),
    ),
    row("60", "Pile On", c(5), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "61",
        "Plague Bringer Goliath",
        c(3),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(7, 7),
        st(14, 14),
    ),
    row(
        "62",
        "Living Bomb",
        c(1),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Crop Dusting",
        c(1),
        T::Trap,
        &[G::Plague],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Malzahar's Recycler",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65",
        "Ace in the Hole",
        c(2),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "66",
        "EU Striker",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "67",
        "Felinor Feeler",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "68",
        "Small Card Lobbyist",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(11, 13),
        st(22, 26),
    ),
    row(
        "69",
        "Plague Charger",
        c(2),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(4, 2),
        st(8, 4),
    ),
    row(
        "70",
        "Book of Plague",
        c(1),
        T::Spell,
        &[G::Book, G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71",
        "Lane Eater",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "72",
        "Grand Counterspell",
        c(2),
        T::Trap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73",
        "Nurse Cleaver",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(3, 6),
        st(6, 12),
    ),
    row(
        "74",
        "Corpse Plantation",
        c(2),
        T::FieldSpell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "75",
        "Argusland",
        c(3),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Plague Bringer",
        c(2),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "77",
        "Anti-Magic Monkey",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "78",
        "Mutate Spell",
        c(1),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "79",
        "Risky Die",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "80",
        "BOOM! Big Max",
        c(4),
        T::Unit,
        &[G::Acclaimed],
        R::Legendary,
        st(13, 8),
        st(26, 16),
    ),
    row(
        "81",
        "The Power to Thrive",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "82",
        "Sheeople",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "83",
        "Flame Lance",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "84",
        "Lockdown",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "85",
        "King Wagtoggle",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "86",
        "Genn",
        c(4),
        T::Unit,
        &[],
        R::Common,
        st(14, 14),
        st(42, 42),
    ),
    row(
        "87",
        "Plague Chalice",
        X,
        T::FieldSpell,
        &[G::Plague],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "88",
        "Siphon Squad",
        c(3),
        T::FieldTrap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "89",
        "Paul Allen's Ghost",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(5, 6),
        st(10, 12),
    ),
    row(
        "90",
        "In Too Deep",
        c(1),
        T::FieldSpell,
        &[G::Quickdraw, G::Quest],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    // Issue #170, R674: Glitch, the hidden token R673's roll makes, blank on both faces.
    row(
        "T-glitch",
        "Glitch",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
];

const CLASSIC_PLUS: &[SpecRow] = &[
    row(
        "1",
        "Doom Shroom",
        c(3),
        T::Trap,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "Groom Shroom",
        c(3),
        T::Trap,
        &[G::Felinor],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "3",
        "Second Amendment Snake",
        c(2),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(1, 6),
        st(2, 12),
    ),
    row(
        "4",
        "Juhan Biggest Bat",
        c(3),
        T::Unit,
        &[G::Cn],
        R::Common,
        st(9, 6),
        st(18, 12),
    ),
    row(
        "5",
        "Guy Att",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 8),
        st(12, 16),
    ),
    row(
        "6",
        "Wrong-House Attacker",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "7",
        "The House",
        c(3),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "8",
        "Withering Storm",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("9", "Silence", c(0), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "10",
        "New Wraps",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Anime Armor",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "12",
        "The Mother Pancake",
        c(3),
        T::Unit,
        &[G::Pancake],
        R::Legendary,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "12.1",
        "Devour",
        c(0),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.2",
        "Death Boil",
        c(1),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.3",
        "Fluffy Grip",
        c(1),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.4",
        "Powder Spray",
        c(1),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.5",
        "Anti-Waffle Shell",
        c(1),
        T::FieldSpell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.6",
        "Frozen Wastes",
        c(2),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.7",
        "Legion of the Hungry",
        c(2),
        T::Spell,
        &[G::Pancake, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.8",
        "Frostspatula",
        c(2),
        T::FieldSpell,
        &[G::Pancake, G::Token],
        R::Token,
        st(7, 3),
        st(14, 6),
    ),
    row(
        "13",
        "Mommy Barker",
        c(1),
        T::Unit,
        &[G::Human, G::Pancake],
        R::Legendary,
        st(2, 2),
        st(4, 4),
    ),
    row("14", "Forever&", c(1), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "15",
        "Conjure Rush Token",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "16",
        "Conjure Rush Token+",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "17",
        "Conjure Rush Token++",
        c(4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18",
        "Gullible Treatler",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(8, 9),
        st(16, 18),
    ),
    row(
        "19",
        "League of Losers",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19.1",
        "Top Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "19.2",
        "Jungle Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "19.3",
        "Mid Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "19.4",
        "Support Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "19.5",
        "Bot Loser",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "20",
        "Mushroom Power",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "21",
        "Whirlwind",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "22",
        "Blood Moon",
        c(2),
        T::Trap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "23",
        "Dropshipping",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Crushing Walls",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Soul Shot",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "26",
        "Tommy Tempo",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "27",
        "Zephrys Zealotism",
        c(4),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "28",
        "Nuestro hogar, nuestras tumbas",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "29",
        "Portal to the Past",
        c(3),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Felinor Fuser",
        c(4),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "31",
        "Fusion Lab",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32",
        "Otherworldly Removal",
        c(3),
        T::Spell,
        &[G::Otherworldly],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32.1",
        "Execute",
        c(1),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32.2",
        "Brawl",
        c(3),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32.3",
        "Blade Storm",
        c(1),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "33",
        "Ivory Tower",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "Memory Leak",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Rollback",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "Conjure Bones",
        emb(2, 4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36.1",
        "Bone Storm",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "Wardrum",
        c(5),
        T::Unit,
        &[G::Quickdraw, G::Acclaimed],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "38",
        "Solarius",
        c(2),
        T::Unit,
        &[G::Catalyst],
        R::Epic,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "38.1",
        "Solarius Prime",
        c(4),
        T::Unit,
        &[G::Prime, G::Token],
        R::Token,
        st(9, 5),
        st(18, 10),
    ),
    row(
        "39",
        "Book Worm",
        c(1),
        T::Unit,
        &[G::Plague],
        R::Common,
        st(1, 4),
        st(2, 8),
    ),
    row(
        "40",
        "Appropriations",
        X,
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "41",
        "KY's Constant",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "42",
        "KY's Test",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "42.1",
        "KY's Gift",
        c(4),
        T::FieldSpell,
        &[G::Ky, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "AI Slop",
        c(3),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "44",
        "Simplicity Audit",
        c(3),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Complexity Audit",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "46",
        "Felinor Flagbearer",
        c(2),
        T::Unit,
        &[G::Felinor, G::Catalyst],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "46.1",
        "Felinor Flagbearer Prime",
        c(2),
        T::Unit,
        &[G::Felinor, G::Prime, G::Token],
        R::Token,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "47",
        "Jogg's Box",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "48",
        "Jlockeed's Lobbyist",
        c(1),
        T::Unit,
        &[G::Jlockeed],
        R::Legendary,
        st(0, 3),
        st(0, 6),
    ),
    row(
        "49",
        "Jay Fungus",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(3, 6),
        st(6, 12),
    ),
    row(
        "50",
        "Adaptive Growth",
        c(1),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51",
        "Jlockeed's J15 Fighter",
        c(3),
        T::Unit,
        &[G::Jlockeed],
        R::Epic,
        st(7, 2),
        st(14, 4),
    ),
    row(
        "52",
        "Jlockeed's Permanent Defense Contract",
        c(2),
        T::Spell,
        &[G::Jlockeed],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Book of Tokens",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "54",
        "Book of Books",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "55",
        "Book of Greed",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "Book of Pain",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "57",
        "Book of Stats",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "58",
        "Fruit Basket",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "All Purpose Apple",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "60",
        "Doctors Orders",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "61",
        "Bauble Bubble",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "62",
        "KY's Papaya",
        c(2),
        T::Spell,
        &[G::Fruit, G::Ky],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Fruit Tree",
        c(3),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Mulch Muncher",
        c(5),
        T::Unit,
        &[],
        R::Rare,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "65",
        "Two Grapes",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.1",
        "Rotten Grape",
        c(1),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.2",
        "Normal Grape",
        c(1),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.3",
        "Large Grape",
        c(3),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.4",
        "Golden Grape",
        c(1),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65.5",
        "Mythic Grape",
        c(0),
        T::Spell,
        &[G::Fruit, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "66",
        "Vine of Grapes",
        c(3),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "67",
        "Pear",
        c(2),
        T::Spell,
        &[G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "68",
        "Organic Produce",
        c(4),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "69",
        "Buff Billy",
        X,
        T::Unit,
        &[G::Human],
        R::Rare,
        st(0, 0),
        st(0, 0),
    ),
    row(
        "70",
        "Chaos Machine",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71",
        "Book of Buff",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "72",
        "Book of Nerf",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73",
        "Call to Chaos (Classic+ Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "73.1",
        "Classic Golem",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "74",
        "Twice Forward One Step Backwards",
        c(2),
        T::FieldTrap,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "75",
        "J-lease J-Jungle EX-plorer",
        c(3),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "75.1",
        "J-lease J-Jungle EX-plorer Pack",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Brother Lar",
        c(1),
        T::Unit,
        &[G::Cn, G::Human],
        R::Rare,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "76.1",
        "Brother Ping",
        c(2),
        T::Unit,
        &[G::Cn, G::Human, G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "77",
        "Anti-Softlock",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "78",
        "Claude's Datacenter",
        c(3),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-1",
        "Helpful Assistant",
        c(1),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(1, 3),
        st(2, 6),
    ),
    row(
        "T-AI-2",
        "Scaling Law",
        c(2),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "T-AI-3",
        "Hallucination",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-4",
        "Chain of Thought",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-5",
        "Autocomplete",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-6",
        "Datacenter Fire",
        c(2),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-7",
        "Alignment Tax",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-8",
        "Rate Limit",
        c(1),
        T::Trap,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-9",
        "Refusal",
        c(1),
        T::Trap,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-10",
        "Fine-Tuning",
        c(2),
        T::FieldSpell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
];

/// Each set's fixture, in catalog order (B2.2).
/// R1420: the Meditative set's rows, transcribed from docs/meditative-set.md's card headers (M6), in
const MEDITATIVE: &[SpecRow] = &[
    row(
        "1",
        "Disruptive Disruptor",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "Rampaging Rhino",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 9),
        st(10, 18),
    ),
    row(
        "3",
        "Jlockwork Machine",
        c(3),
        T::Unit,
        &[],
        R::Rare,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "4",
        "Juicy Kumquat Melon",
        c(2),
        T::Spell,
        &[G::Fruit],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "5",
        "Death by 1000 cuts",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "6",
        "Me no Likey",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "7",
        "Introspection",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "8",
        "Reach the Summit",
        c(0),
        T::Spell,
        &[G::Quickdraw, G::Wincon],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "9",
        "Joint Filing",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10",
        "Double Counting",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Double Header",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12",
        "Fear Mongerer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(5, 7),
        st(10, 14),
    ),
    row(
        "13",
        "Gatling Pea",
        c(2),
        T::Unit,
        &[],
        R::Epic,
        st(2, 6),
        st(4, 12),
    ),
    row(
        "14",
        "Prime Time",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Smelly Steven",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "16",
        "Trenful Trickster",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(1, 4),
        st(2, 8),
    ),
    row(
        "17",
        "True Craft a Card",
        c(0),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18",
        "Expedition12",
        c(1),
        T::Spell,
        &[G::Quickdraw],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Expedition123",
        c(1),
        T::Spell,
        &[G::Quickdraw],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19.1",
        "Temporal Rift",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20",
        "Aestheticize the Game",
        c(0),
        T::Spell,
        &[G::Quickdraw, G::Wincon],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "21",
        "API Key Fishing",
        emb(0, 1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "22",
        "Mind Games",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "22.1",
        "Fortify Mind",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "23",
        "Golly Bob Howdy",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 5),
        st(6, 10),
    ),
    row(
        "24",
        "Polymorph",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Blue-Eyes White Felinor",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(12, 9),
        st(24, 18),
    ),
    row(
        "26",
        "Alternate Fate",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Clip-Farming Lawyer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 5),
        st(12, 10),
    ),
    row(
        "28",
        "Shade-iris",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(7, 4),
        st(14, 8),
    ),
    row(
        "28.1",
        "Ancient Curse",
        c(2),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "Forbiddenous Factory",
        c(3),
        T::FieldSpell,
        &[G::Plague],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Fickle E-Kitten",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "30.1",
        "Love Bomb",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "31",
        "The Conductor",
        c(3),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(7, 9),
        st(14, 18),
    ),
    row(
        "32",
        "Spiritually 中国",
        c(4),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "33",
        "First Day of 学校",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "高考",
        c(4),
        T::Spell,
        &[G::Cn, G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "RCTA (CN)",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "CN Peptides",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "CN in a Bottle",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "38",
        "H1B Printer",
        c(3),
        T::FieldSpell,
        &[G::Cn, G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39",
        "赌石 Addict",
        c(2),
        T::Unit,
        &[G::Cn],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "39.1",
        "Auspicious Rock",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.2",
        "Jade",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.3",
        "Dud",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.4",
        "Red Jade",
        c(0),
        T::Spell,
        &[G::Cn, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.5",
        "Jade Beauty",
        c(10),
        T::Unit,
        &[G::Cn, G::Token],
        R::Token,
        st(20, 20),
        st(40, 40),
    ),
    row(
        "40",
        "Feng Shui",
        c(2),
        T::FieldSpell,
        &[G::Cn],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "41",
        "CN Smuggler",
        c(2),
        T::Unit,
        &[G::Cn],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "42",
        "CN Flea Market",
        c(1),
        T::Spell,
        &[G::Cn],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "CN Jade Market",
        X,
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "44",
        "CN Jade Well",
        c(3),
        T::FieldSpell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Knowledge Breaker",
        c(1),
        T::Unit,
        &[G::Cn, G::Ky, G::Catalyst],
        R::Legendary,
        st(1, 3),
        st(2, 6),
    ),
    row(
        "45.1",
        "Knowledge Breaker Prime",
        c(4),
        T::Unit,
        &[G::Cn, G::Ky, G::Prime, G::Token],
        R::Token,
        st(6, 18),
        st(12, 36),
    ),
    row(
        "46",
        "Conjure Intellect",
        emb(2, 4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "饕餮",
        c(4),
        T::Unit,
        &[G::Cn],
        R::Legendary,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "48",
        "Tranquility",
        c(2),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "49",
        "YileGPT",
        c(2),
        T::Unit,
        &[G::Cn, G::Acclaimed, G::Catalyst],
        R::Mythic,
        st(2, 6),
        st(4, 12),
    ),
    row(
        "49.1",
        "YileGPT Prime",
        c(8),
        T::Unit,
        &[G::Cn, G::Acclaimed, G::Prime, G::Token],
        R::Token,
        st(8, 20),
        st(16, 40),
    ),
    row(
        "49.2",
        "Yile's Virus",
        c(2),
        T::Unit,
        &[G::Cn, G::Token],
        R::Token,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "49.3",
        "AI Girlfriend",
        c(4),
        T::Unit,
        &[G::Cn, G::Token],
        R::Token,
        st(2, 30),
        st(4, 60),
    ),
    row(
        "50",
        "CN Tech",
        c(0),
        T::Spell,
        &[G::Cn],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51",
        "Devin Bot",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "52",
        "Economic Anxiety",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Prestige",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "54",
        "Money Machine",
        c(1),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "55",
        "Dragon Fruit",
        c(2),
        T::Spell,
        &[G::Cn, G::Fruit],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "House Party",
        c(4),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "57",
        "Clip-Farming Critikal",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 1),
        st(8, 2),
    ),
    row(
        "58",
        "Permanent Underclassman",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "59",
        "Permanent Upperclassman",
        c(4),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "60",
        "Eschews",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "61",
        "Joon Jorker",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(7, 5),
        st(14, 10),
    ),
    row(
        "62",
        "Small Time Recruits",
        c(0),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "63",
        "Skull of J'Nari",
        c(3),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "64",
        "Traitorous Blood",
        c(1),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "65",
        "Keymaster Keenus",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "66",
        "Fiery Waraxe",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "67",
        "Sentient Cat Ears",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "68",
        "Catnip",
        c(1),
        T::FieldTrap,
        &[G::Felinor],
        R::Rare,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "69",
        "The Maestro",
        c(4),
        T::Unit,
        &[G::Human, G::Plague],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "70",
        "I'M WILL BE YOUR DOOM",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(5, 8),
        st(10, 16),
    ),
    row(
        "70.1",
        "Ready… I'm",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "71",
        "Pareto Optimality",
        c(3),
        T::FieldSpell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "71.1",
        "The Cane",
        c(2),
        T::FieldSpell,
        &[G::Cn, G::Token],
        R::Token,
        st(3, 1),
        st(6, 2),
    ),
    row(
        "72",
        "The Banisher",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "73",
        "Plate Packer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "74",
        "Montaña Giant",
        c(8),
        T::Unit,
        &[],
        R::Epic,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "75",
        "Ever Growing Tree",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "76",
        "Do or Die",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "77",
        "Bulk Booster",
        c(3),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "78",
        "Occidentless Mandate",
        c(4),
        T::Spell,
        &[G::Cn],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "79",
        "Touched by KY",
        c(3),
        T::Unit,
        &[G::Ky],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "80",
        "Aluneth",
        c(2),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "81",
        "Deadman's Hand",
        emb(0, 2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "82",
        "Medina Outfitter",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "83",
        "Medina Enforcer",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "84",
        "Volatility",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "85",
        "Playtester",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "86",
        "Mayor Medinamogger",
        c(2),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 4),
        st(10, 8),
    ),
    row(
        "87",
        "Tatches the Totem",
        c(1),
        T::Unit,
        &[G::Human, G::Felinor, G::Ky, G::Cn, G::Jlockeed],
        R::Legendary,
        st(0, 3),
        st(0, 6),
    ),
    row(
        "88",
        "The True Sheep",
        c(4),
        T::Unit,
        &[],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "89",
        "Jlarna",
        c(4),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "90",
        "Spell Basket",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "91",
        "Windfast",
        c(1),
        T::Unit,
        &[G::Catalyst],
        R::Epic,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "91.1",
        "Windfurious Prime",
        c(3),
        T::Unit,
        &[G::Prime, G::Token],
        R::Token,
        st(5, 10),
        st(10, 20),
    ),
    row("92", "Unan", c(3), T::Unit, &[], R::Rare, st(3, 13), st(6, 26)),
    row(
        "93",
        "Growing Felinor",
        c(1),
        T::Unit,
        &[G::Felinor],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "93.1",
        "Growing Felinor Sr",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "93.2",
        "Growing Felinor Sr Sr",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "93.3",
        "Growing Felinor Super Senior",
        c(1),
        T::Unit,
        &[G::Felinor, G::Token],
        R::Token,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "94",
        "Shrinking Felinor",
        c(4),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "95",
        "Call to Chaos (Meditative Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "95.1",
        "CN Golem",
        c(4),
        T::Unit,
        &[G::Cn, G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "96",
        "Meditative Journey",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "96.1",
        "Journey Complete",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "97",
        "Jlockeed's Evil Blueprints",
        c(0),
        T::Spell,
        &[G::Jlockeed],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "97.1",
        "Empty Plot",
        c(0),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 3),
        st(0, 6),
    ),
    row(
        "97.2",
        "Wishing Well",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "97.3",
        "School",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "97.4",
        "Mega Church",
        c(3),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 5),
        st(0, 10),
    ),
    row(
        "97.5",
        "Bunker",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(5, 10),
        st(10, 20),
    ),
    row(
        "97.6",
        "University",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 12),
        st(0, 24),
    ),
    row(
        "97.7",
        "The Great Wall",
        c(2),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 50),
        st(0, 100),
    ),
    row(
        "97.8",
        "Prison",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 16),
        st(0, 32),
    ),
    row(
        "97.9",
        "Jlockeed's Headquarters",
        c(4),
        T::Unit,
        &[G::Jlockeed, G::Token],
        R::Token,
        st(0, 20),
        st(0, 40),
    ),
    row("98", "Showdown", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "99",
        "Paranoia",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "100",
        "Greaser",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(7, 7),
        st(21, 21),
    ),
    row(
        "101",
        "Gachaholic",
        c(1),
        T::Unit,
        &[G::Cn, G::Human],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "102",
        "Catboy Maid SSR+",
        c(1),
        T::Unit,
        &[G::Cn, G::Felinor],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
];

/// R1420, R1178000: the Meditative+ set's rows, transcribed from docs/meditative-plus-set.md's card
/// headers (MP6), in number order with each token after its card. The set does not ship yet: an
/// entry it holds must equal its row, and a row with no entry is a card still being built, until it
/// ships (issue #1187). #17 is Dogpile, Classic #60 holding the name Pile On (R381, R1178002); #20.1
/// Lil' Kenny prints 0/0, its X/X its summon's (`xStats`, as C+ #69 Buff Billy's 3X/3X).
const MEDITATIVE_PLUS: &[SpecRow] = &[
    row(
        "1",
        "Bob's Plan",
        c(2),
        T::Spell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "1.1",
        "Base Camp",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "1.2",
        "Spa Day",
        c(3),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "1.3",
        "Garage Sale",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "1.4",
        "Leg Day",
        c(3),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "1.5",
        "Star Chart",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row("2", "NA Tech", c(1), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "3",
        "Wondrous Wand",
        emb(2, 4),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4",
        "Jlockeed's Supply Drop",
        emb(2, 4),
        T::Spell,
        &[G::Jlockeed],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "5",
        "Spirit Lash",
        emb(0, 1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "6",
        "Bloodbloom",
        c(0),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "7",
        "Life Mage",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(1, 2),
        st(2, 4),
    ),
    row(
        "8",
        "Arch Mage",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 8),
        st(8, 16),
    ),
    row(
        "9",
        "Magic Amplifier",
        emb(1, 2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10",
        "Medina Grifter",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 1),
        st(6, 2),
    ),
    row(
        "11",
        "Living Mana Potion",
        c(1),
        T::Unit,
        &[G::Potion],
        R::Rare,
        st(5, 1),
        st(10, 2),
    ),
    row(
        "12",
        "Living Armor Potion",
        c(1),
        T::Unit,
        &[G::Potion],
        R::Rare,
        st(1, 5),
        st(2, 10),
    ),
    row(
        "13",
        "Living Health Potion",
        c(1),
        T::Unit,
        &[G::Potion],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "14",
        "Potion Pouch",
        c(1),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "Book of Potions",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "16",
        "Jazakus",
        c(1),
        T::Unit,
        &[G::Human, G::Catalyst],
        R::Epic,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "16.1",
        "Jazakus Prime",
        c(1),
        T::Unit,
        &[G::Human, G::Prime, G::Token],
        R::Token,
        st(8, 8),
        st(16, 16),
    ),
    row("17", "Dogpile", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "18",
        "Cognitive Offloading",
        c(1),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Grubby Gloves",
        emb(1, 2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20",
        "KY's Competition",
        c(2),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20.1",
        "Lil' Kenny",
        c(1),
        T::Unit,
        &[G::Ky, G::Token],
        R::Token,
        st(0, 0),
        st(0, 0),
    ),
    row(
        "21",
        "Harmony Spirit",
        c(2),
        T::Unit,
        &[G::Cn],
        R::Rare,
        st(3, 7),
        st(6, 14),
    ),
    row(
        "22",
        "Mountaintop Garden",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "23",
        "Enormous Elderberry",
        emb(2, 4),
        T::Spell,
        &[G::Fruit],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Bob, Wisconsin's Strongest Soldier",
        c(3),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "25",
        "KY's Intuition",
        c(1),
        T::Spell,
        &[G::Ky],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "26",
        "Scratch Pad",
        c(0),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
];

/// R1420, R1857000, R1857001: the Stinkyscape set's rows, transcribed from docs/stinkyscape-set.md
/// ST13's header lines (issue #1857), in catalog order: each card's tokens after it, then the Dormant
/// Starship (a card at a `T-` index), the shared tokens and the ten AI generated cards. The set is
/// held, so a row waits for its card until the set ships (#1884). The designer's second #21 and #122
/// are #174 Aerelret and #175 Frost Module.
const STINKYSCAPE: &[SpecRow] = &[
    row(
        "1",
        "Freeze Ray",
        c(0),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "2",
        "Absolute Zero",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row("3", "Meteor", c(3), T::Spell, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "4",
        "Olivian Expanse",
        c(4),
        T::Spell,
        &[],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4.1",
        "Farewell",
        c(4),
        T::Spell,
        &[G::Terminus, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4.2",
        "KY's Last Exam",
        c(4),
        T::Spell,
        &[G::Terminus, G::Ky, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4.3",
        "Event Horizon",
        c(4),
        T::Spell,
        &[G::Terminus, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4.4",
        "Last Word",
        c(4),
        T::Trap,
        &[G::Terminus, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4.5",
        "1v1 Final Destination",
        c(4),
        T::Trap,
        &[G::Terminus, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "4.6",
        "The Big Crunch",
        c(4),
        T::Spell,
        &[G::Terminus, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "5",
        "Masochism Unmasked",
        c(1),
        T::FieldTrap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "6",
        "Tylannt",
        c(2),
        T::Unit,
        &[G::Acclaimed, G::Human],
        R::Mythic,
        st(6, 4),
        st(12, 8),
    ),
    row(
        "7",
        "The Greatest Heist",
        c(3),
        T::FieldSpell,
        &[G::Quest],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "8",
        "Conjure Rosemary Triscuit",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "8.1",
        "Rosemary Triscuit",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "9",
        "Ticketed",
        c(2),
        T::FieldSpell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10",
        "Otherworldly Endurance",
        c(2),
        T::Spell,
        &[G::Otherworldly],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10.1",
        "Inner Fire",
        c(1),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10.2",
        "Divine Spirit",
        c(2),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "10.3",
        "Power Word Shield",
        c(0),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11",
        "Otherworldly Magic",
        c(2),
        T::Spell,
        &[G::Otherworldly],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11.1",
        "Frostbolt",
        c(1),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11.2",
        "Arcane Missiles",
        c(0),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "11.3",
        "Flame Strike",
        c(2),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12",
        "Otherworldly Nightmare",
        c(2),
        T::Spell,
        &[G::Otherworldly],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.1",
        "Shadow Word Death",
        c(1),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.2",
        "Shadow Word Pain",
        c(0),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "12.3",
        "Psychic Scream",
        c(3),
        T::Spell,
        &[G::OtherOrigins, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "13",
        "Asphyxiants Cloud",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "14",
        "Arm of the Jempire",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "15",
        "Arg Arg Defender",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 3),
        st(4, 6),
    ),
    row(
        "16",
        "Rod of Ages",
        c(1),
        T::FieldSpell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "17",
        "Imprisoned Ship Smith",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "18",
        "Imitated Instrumentality",
        c(4),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18.1",
        "MELCHIOR",
        c(1),
        T::FieldSpell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18.2",
        "BALTHASAR",
        c(2),
        T::FieldSpell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "18.3",
        "CASPER",
        c(3),
        T::FieldSpell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "19",
        "Imprisoned Cosmality",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(10, 6),
        st(20, 12),
    ),
    row(
        "20",
        "Tnack Trav",
        c(2),
        T::Unit,
        &[],
        R::Epic,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "21",
        "Moonberry",
        c(1),
        T::Spell,
        &[G::Fruit],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row("22", "Gatinsir", c(4), T::Trap, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "23",
        "Warp Disruptor",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "24",
        "Warp Disruptor II",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "KY's Tricknology",
        c(2),
        T::Spell,
        &[G::Ky],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "26",
        "ZolarKeth",
        c(4),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Legendary,
        st(2, 9),
        st(4, 18),
    ),
    row(
        "27",
        "Drone Core I",
        c(0),
        T::Spell,
        &[G::DroneCore],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "28",
        "Drone Core II",
        c(1),
        T::Spell,
        &[G::DroneCore],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "29",
        "Drone Core III",
        c(2),
        T::Spell,
        &[G::DroneCore],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "30",
        "Drone Core IV",
        c(3),
        T::Spell,
        &[G::DroneCore],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "31",
        "Drone Core V",
        c(3),
        T::Spell,
        &[G::DroneCore],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32",
        "Drone Core VI",
        c(4),
        T::Spell,
        &[G::DroneCore],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "33",
        "Drone Core VII",
        c(4),
        T::Spell,
        &[G::DroneCore],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "34",
        "Drone Core VIII",
        c(6),
        T::Spell,
        &[G::DroneCore],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Drone Core IX",
        c(6),
        T::Spell,
        &[G::DroneCore],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "Drone Core X",
        c(8),
        T::Spell,
        &[G::DroneCore],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "37",
        "Kwzka",
        c(3),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Legendary,
        st(6, 9),
        st(12, 18),
    ),
    row(
        "38",
        "Shield Amplifier",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "39",
        "The Narcor Project",
        c(3),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "40",
        "Poison Capsule",
        c(3),
        T::Unit,
        &[G::Plague],
        R::Rare,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "41",
        "Drone Sympathizer",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "42",
        "Core Researcher",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(4, 5),
        st(8, 10),
    ),
    row(
        "43",
        "Healing Capsule",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "44",
        "Great Replacement Theory",
        c(2),
        T::Spell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "45",
        "Drone Commander",
        c(4),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(8, 12),
        st(16, 24),
    ),
    row(
        "46",
        "Circadian Halfcycler",
        c(2),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "Circadian Fullcycler",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "48",
        "Ultralight Chassis",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(4, 1),
        st(8, 2),
    ),
    row(
        "49",
        "Spacemine",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(5, 1),
        st(10, 2),
    ),
    row(
        "50",
        "Beam v3",
        c(2),
        T::Unit,
        &[G::Weapon],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "51",
        "Repeater v3",
        c(2),
        T::Unit,
        &[G::Weapon],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "52",
        "Cannon v3",
        c(2),
        T::Unit,
        &[G::Weapon],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "53",
        "Flak v3",
        c(2),
        T::Unit,
        &[G::Weapon],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "54",
        "Beam v4",
        c(3),
        T::Unit,
        &[G::Weapon],
        R::Epic,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "55",
        "Repeater v4",
        c(3),
        T::Unit,
        &[G::Weapon],
        R::Epic,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "56",
        "Cannon v4",
        c(3),
        T::Unit,
        &[G::Weapon],
        R::Epic,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "57",
        "Flak v4",
        c(3),
        T::Unit,
        &[G::Weapon],
        R::Epic,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "58",
        "Beam v5",
        c(4),
        T::Unit,
        &[G::Weapon],
        R::Legendary,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "59",
        "Repeater v5",
        c(4),
        T::Unit,
        &[G::Weapon],
        R::Legendary,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "60",
        "Cannon v5",
        c(4),
        T::Unit,
        &[G::Weapon],
        R::Legendary,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "61",
        "Flak v5",
        c(4),
        T::Unit,
        &[G::Weapon],
        R::Legendary,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "62",
        "Miner v1",
        c(0),
        T::Unit,
        &[],
        R::Common,
        st(0, 1),
        st(0, 2),
    ),
    row(
        "63",
        "Miner v2",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(0, 2),
        st(0, 4),
    ),
    row("64", "Miner v3", c(2), T::Unit, &[], R::Rare, st(0, 4), st(0, 8)),
    row("65", "Miner v4", c(3), T::Unit, &[], R::Epic, st(0, 6), st(0, 12)),
    row(
        "66",
        "Miner v5",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "67",
        "Tractor Beam",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(0, 2),
        st(0, 4),
    ),
    row("68", "Stratos", c(1), T::Unit, &[], R::Common, st(3, 1), st(6, 2)),
    row("69", "Falcon", c(2), T::Unit, &[], R::Rare, st(4, 1), st(8, 2)),
    row(
        "70",
        "Z'Zoth",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(5, 7),
        st(10, 14),
    ),
    row(
        "71",
        "A'Zoth",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(7, 5),
        st(14, 10),
    ),
    row(
        "72",
        "Paragon Mk1",
        c(3),
        T::Unit,
        &[],
        R::Epic,
        st(6, 1),
        st(12, 2),
    ),
    row(
        "73",
        "Paragon Mk2",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(7, 1),
        st(14, 2),
    ),
    row(
        "74",
        "Paragon-X",
        c(4),
        T::Unit,
        &[],
        R::Mythic,
        st(10, 1),
        st(20, 2),
    ),
    row("75", "Badger", c(1), T::Unit, &[], R::Common, st(2, 3), st(4, 6)),
    row(
        "76",
        "Honey Badger",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(4, 6),
        st(8, 12),
    ),
    row("77", "Yuma", c(2), T::Unit, &[], R::Epic, st(1, 8), st(2, 16)),
    row("78", "Ozark", c(3), T::Unit, &[], R::Epic, st(4, 4), st(8, 8)),
    row(
        "79",
        "Orca",
        emb(4, 5),
        T::Unit,
        &[],
        R::Epic,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "80",
        "Galaxy",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(10, 10),
        st(20, 20),
    ),
    row("81", "Radix", c(3), T::Unit, &[], R::Rare, st(1, 1), st(2, 2)),
    row("82", "Phalanx", c(4), T::Unit, &[], R::Epic, st(3, 3), st(6, 6)),
    row("83", "Halo", c(4), T::Unit, &[], R::Epic, st(2, 2), st(4, 4)),
    row(
        "84",
        "Power Cell",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "84.1",
        "Metal Scraps",
        c(1),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "85",
        "Prototype-X1",
        c(4),
        T::Unit,
        &[],
        R::Legendary,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "86",
        "Prototype-X2",
        c(4),
        T::Unit,
        &[],
        R::Mythic,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "87",
        "Siva 1",
        c(2),
        T::Unit,
        &[G::Felinor, G::Acclaimed],
        R::Legendary,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "88",
        "Siva 2",
        c(3),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Legendary,
        st(4, 4),
        st(8, 8),
    ),
    row("89", "Spice", c(1), T::Spell, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "90",
        "After Burner",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(4, 8),
        st(8, 16),
    ),
    row(
        "91",
        "The Propagation's Wasp",
        c(1),
        T::Unit,
        &[G::Swarm],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "92",
        "The Propagation's Hornet",
        c(2),
        T::Unit,
        &[G::Swarm],
        R::Rare,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "93",
        "Patcher Nanobots",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "94",
        "Tmack Trav",
        c(3),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "95",
        "Faction: Vanguard Coalition",
        c(2),
        T::Spell,
        &[G::Faction],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "96",
        "Ocean Tsunami",
        c(3),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "97",
        "Asteroid Field",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "98",
        "Hull Booster",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(1, 1),
        st(2, 2),
    ),
    row(
        "99",
        "No-No Words",
        X,
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "100",
        "AP Rounds",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 1),
        st(6, 2),
    ),
    row(
        "101",
        "Overdrive Module",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(5, 5),
        st(10, 10),
    ),
    row(
        "102",
        "Book of Nono-Words",
        c(1),
        T::Spell,
        &[G::Book],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "103",
        "Vxyl",
        c(2),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(1, 4),
        st(2, 8),
    ),
    row(
        "104",
        "Reflective Plating",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "105",
        "Disintegrex",
        c(2),
        T::Unit,
        &[],
        R::Epic,
        st(6, 3),
        st(12, 6),
    ),
    row(
        "106",
        "Warpfield Amplifier",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(2, 3),
        st(4, 6),
    ),
    row("107", "RR-5000", c(2), T::Unit, &[], R::Rare, st(4, 4), st(8, 8)),
    row(
        "108",
        "Trila 3",
        c(3),
        T::Unit,
        &[],
        R::Common,
        st(7, 7),
        st(14, 14),
    ),
    row(
        "109",
        "xxFreshJack",
        c(4),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Mythic,
        st(6, 11),
        st(12, 22),
    ),
    row(
        "110",
        "iKwzki",
        c(2),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Mythic,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "111",
        "Shockwave447",
        c(4),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Mythic,
        st(4, 7),
        st(8, 14),
    ),
    row(
        "112",
        "Job Application",
        c(1),
        T::FieldSpell,
        &[G::Quest],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "112.1",
        "Paycheck",
        c(0),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "113",
        "A Very Dandy Lad",
        c(4),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(9, 15),
        st(18, 30),
    ),
    row(
        "114",
        "Arctic Citadel",
        c(4),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(4, 4),
        st(8, 8),
    ),
    row(
        "115",
        "Scornboi",
        c(4),
        T::Unit,
        &[G::Human, G::Acclaimed],
        R::Legendary,
        st(12, 9),
        st(24, 18),
    ),
    row(
        "116",
        "Megabase",
        c(1),
        T::Unit,
        &[G::Quickdraw],
        R::Legendary,
        st(0, 20),
        st(0, 40),
    ),
    row(
        "117",
        "Core System",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "118",
        "Secure System",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "119",
        "Unsecure System",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "120",
        "Wild System",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "121",
        "System Schematic",
        c(1),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "122",
        "Vamp Rounds",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "123",
        "Gokugains",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "124",
        "Jlockeed's Shipbuilding Grant",
        c(2),
        T::Spell,
        &[G::Jlockeed],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "125",
        "Jlockeed's Spacerace Propaganda",
        c(2),
        T::FieldSpell,
        &[G::Jlockeed],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "126",
        "Jlockeed's Permanent Procurement Lobbying",
        c(2),
        T::Spell,
        &[G::Jlockeed],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "127",
        "Jlockeed's Siege Tank",
        emb(3, 4),
        T::Unit,
        &[G::Jlockeed],
        R::Rare,
        st(8, 5),
        st(16, 10),
    ),
    row(
        "128",
        "Jlockeed's Yamato Cannon",
        emb(2, 3),
        T::Unit,
        &[G::Jlockeed],
        R::Rare,
        st(4, 1),
        st(8, 2),
    ),
    row(
        "129",
        "Jlockeed's Missile Pod",
        emb(1, 2),
        T::Unit,
        &[G::Jlockeed],
        R::Epic,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "130",
        "Jlockeed's Starship Retrofitting Program",
        c(1),
        T::Spell,
        &[G::Jlockeed],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "131",
        "Spacerace Stimulus",
        c(1),
        T::FieldTrap,
        &[],
        R::Common,
        st(6, 6),
        st(12, 12),
    ),
    row(
        "132",
        "Defensive Crystal",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(3, 6),
        st(6, 12),
    ),
    row(
        "133",
        "Ultra Capacitor",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "134",
        "Felinor Containment Chamber",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row("135", "Sacked", c(1), T::Trap, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "135.1",
        "Sack",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "136",
        "Magic Module",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(4, 3),
        st(8, 6),
    ),
    row(
        "137",
        "Starfleet Commander",
        c(4),
        T::Unit,
        &[G::Human],
        R::Legendary,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "137.1",
        "Land Doctrine",
        c(4),
        T::Spell,
        &[G::WarDoctrine, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "137.2",
        "Sea Doctrine",
        c(4),
        T::Spell,
        &[G::WarDoctrine, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "137.3",
        "Air Doctrine",
        c(4),
        T::Spell,
        &[G::WarDoctrine, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "137.4",
        "Space Doctrine",
        c(4),
        T::Spell,
        &[G::WarDoctrine, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "138",
        "Pocket Peptides",
        c(1),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row("139", "Wormhole", c(2), T::Trap, &[], R::Rare, NO_STATS, NO_STATS),
    row(
        "140",
        "Rush Token Launcher",
        c(2),
        T::Unit,
        &[G::Weapon],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "141",
        "Fruit Launcher",
        c(2),
        T::Unit,
        &[G::Weapon],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "142",
        "Jade Launcher",
        c(3),
        T::Unit,
        &[G::Cn, G::Weapon],
        R::Epic,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "143",
        "Book Launcher",
        c(3),
        T::Unit,
        &[G::Weapon],
        R::Epic,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "144",
        "Prepare For Liftoff",
        c(1),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "145",
        "Fortify Flesh",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "146",
        "Flash Freeze",
        c(0),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "147",
        "IN33DH3",
        c(0),
        T::Unit,
        &[G::Catalyst, G::Human],
        R::Legendary,
        st(1, 2),
        st(2, 4),
    ),
    row(
        "147.1",
        "IN33DH3 Prime",
        c(4),
        T::Unit,
        &[G::Prime, G::Human, G::Token],
        R::Token,
        st(9, 5),
        st(18, 10),
    ),
    row(
        "147.2",
        "Wardrum Blob",
        c(2),
        T::FieldSpell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "148",
        "Alteran Reach",
        c(0),
        T::FieldSpell,
        &[G::Wincon, G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "148.1",
        "Edge of the Universe",
        c(0),
        T::Spell,
        &[G::Wincon, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "149",
        "Wildar",
        c(3),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "150",
        "Call to Chaos (Stinky Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "150.1",
        "Stinkyscape Golem",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "151",
        "Call to Chaos (Scape Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "152",
        "Sanctum",
        c(4),
        T::FieldSpell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "153",
        "Stimmycheck",
        c(0),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "154",
        "Dice of Doom",
        emb(0, 1),
        T::Spell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "155",
        "Comet Crash",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "156",
        "Catalyst Comet",
        c(1),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "157",
        "The Gauntlet",
        c(1),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "158",
        "Stratos Swarm",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "159",
        "Faction: ETHICS",
        c(2),
        T::Spell,
        &[G::Faction],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "160",
        "Faction: Golden Accord",
        c(2),
        T::Spell,
        &[G::Faction],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "161",
        "Faction: Pegasus",
        c(2),
        T::Spell,
        &[G::Faction],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "162",
        "Faction: Star-Jumper Core",
        c(2),
        T::Spell,
        &[G::Faction],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "163",
        "Swear Loyalities",
        c(0),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "164",
        "The Syndicate",
        c(0),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "165",
        "The Trade Union",
        c(0),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "166",
        "Scanning Probe Mk1",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "167",
        "Scanning Probe Mk2",
        c(2),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "168",
        "Drone Hanger",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "169",
        "Tiny Asteroid",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "170",
        "Medium Asteroid",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(0, 6),
        st(0, 12),
    ),
    row(
        "171",
        "Large Asteroid",
        c(3),
        T::Unit,
        &[],
        R::Rare,
        st(0, 8),
        st(0, 16),
    ),
    row(
        "172",
        "Huge Asteroid",
        c(4),
        T::Unit,
        &[],
        R::Epic,
        st(0, 10),
        st(0, 20),
    ),
    row(
        "173",
        "Abandoned Spice Tower",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(0, 8),
        st(0, 16),
    ),
    row("174", "Aerelret", c(4), T::Trap, &[], R::Epic, NO_STATS, NO_STATS),
    row(
        "175",
        "Frost Module",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "176",
        "Preemptive Strike",
        c(2),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "177",
        "Cloaking Module",
        c(2),
        T::Unit,
        &[],
        R::Rare,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "T-dormant-starship",
        "Dormant Starship",
        c(3),
        T::Unit,
        &[],
        R::Legendary,
        st(0, 0),
        st(0, 0),
    ),
    row(
        "T-drone-interceptor",
        "Drone Interceptor",
        c(1),
        T::Unit,
        &[G::Token],
        R::Token,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "T-drone-corvette",
        "Drone Corvette",
        c(3),
        T::Unit,
        &[G::Token],
        R::Token,
        st(8, 8),
        st(16, 16),
    ),
    row(
        "T-shield-layer",
        "Drone Battleship (Shield Layer)",
        c(5),
        T::Unit,
        &[G::Token],
        R::Token,
        st(12, 12),
        st(24, 24),
    ),
    row(
        "T-propagation-layer",
        "Drone Battleship (Propagation Layer)",
        c(5),
        T::Unit,
        &[G::Token],
        R::Token,
        st(12, 12),
        st(24, 24),
    ),
    row(
        "T-meltdown-layer",
        "Drone Battleship (Meltdown Layer)",
        c(5),
        T::Unit,
        &[G::Token],
        R::Token,
        st(12, 12),
        st(24, 24),
    ),
    row(
        "T-reknite",
        "Reknite",
        c(0),
        T::Spell,
        &[G::Resource, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-korrelite",
        "Korrelite",
        c(0),
        T::Spell,
        &[G::Resource, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-gellium",
        "Gellium",
        c(0),
        T::Spell,
        &[G::Resource, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-axnit",
        "Axnit",
        c(0),
        T::Spell,
        &[G::Resource, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-narcor",
        "Narcor",
        c(0),
        T::Spell,
        &[G::Resource, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-red-narcor",
        "Red Narcor",
        c(0),
        T::Spell,
        &[G::Resource, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-anomaly-found",
        "Anomaly Found!",
        c(4),
        T::Spell,
        &[G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-1",
        "Escort Protocol",
        c(1),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(1, 3),
        st(2, 6),
    ),
    row(
        "T-AI-2",
        "Mining Subroutine",
        c(0),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-3",
        "Route Optimizer",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-4",
        "Predictive Shield",
        c(1),
        T::Trap,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-5",
        "Launch Sequencer",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-6",
        "Probe Relay",
        c(1),
        T::Spell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-7",
        "Combat Subroutine",
        c(2),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "T-AI-8",
        "Drone Uplink",
        c(2),
        T::FieldSpell,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-9",
        "Honeypot",
        c(1),
        T::Trap,
        &[G::Ai, G::Token],
        R::Token,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "T-AI-10",
        "Salvage Protocol",
        c(1),
        T::Unit,
        &[G::Ai, G::Token],
        R::Token,
        st(1, 2),
        st(2, 4),
    ),
];

/// R1420, R2194000: the Training Grounds set's rows, transcribed from docs/training-grounds-set.md
/// TG5's header lines (issue #2194), in catalog order: #1 to #40 with the token #39.1 after #39, then
/// the Core snapshots #41 to #60 (part 2, #2195), each with its Core card's name, cost, type, tags,
/// rarity and stats. The set is held, so a row waits for its card until the set ships (#2204).
const TRAINING_GROUNDS: &[SpecRow] = &[
    row(
        "1",
        "Spikey Steven",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(5, 2),
        st(15, 6),
    ),
    row(
        "2",
        "Personal Trainer",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "3",
        "Group leader",
        c(1),
        T::Unit,
        &[G::Human],
        R::Common,
        st(4, 1),
        st(8, 2),
    ),
    row(
        "4",
        "Boulder Fist Ogre",
        c(2),
        T::Unit,
        &[],
        R::Common,
        st(6, 7),
        st(18, 21),
    ),
    row(
        "5",
        "Midlane Menace",
        c(2),
        T::Unit,
        &[G::Human],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "6",
        "Chosen by the Sun",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 3),
        st(9, 9),
    ),
    row(
        "7",
        "Chosen by the Moon",
        c(1),
        T::Unit,
        &[],
        R::Common,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "8",
        "Clobberer",
        c(3),
        T::Unit,
        &[G::Human],
        R::Common,
        st(14, 7),
        st(42, 14),
    ),
    row(
        "9",
        "Ravager",
        c(4),
        T::Unit,
        &[G::Human],
        R::Common,
        st(7, 7),
        st(14, 17),
    ),
    row(
        "10",
        "Fan of Knives",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row("11", "Thwomp", c(2), T::Spell, &[], R::Common, NO_STATS, NO_STATS),
    row(
        "12",
        "Fortify",
        c(1),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "13",
        "Explosive Runes",
        c(1),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "14",
        "Explosive Trap",
        c(1),
        T::Trap,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "15",
        "In a hurry",
        c(2),
        T::Spell,
        &[],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "16",
        "Celestial Caller",
        c(4),
        T::Unit,
        &[],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "17",
        "Flaming Felinor",
        c(2),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 6),
        st(6, 12),
    ),
    row(
        "18",
        "Knife Juggler",
        c(1),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 2),
        st(6, 4),
    ),
    row(
        "19",
        "Extra Rations",
        c(1),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "20",
        "Recruitment Posters",
        c(2),
        T::FieldSpell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "21",
        "Flame Tongue Totem",
        c(1),
        T::Unit,
        &[],
        R::Rare,
        st(0, 4),
        st(0, 8),
    ),
    row(
        "22",
        "Right-Hand Felinor",
        c(2),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "23",
        "Quadrupling Felinors",
        c(3),
        T::Unit,
        &[G::Felinor],
        R::Rare,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "24",
        "Goliath Situation",
        c(1),
        T::FieldTrap,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "25",
        "Trigor",
        c(3),
        T::Unit,
        &[G::Human],
        R::Rare,
        st(3, 3),
        st(6, 6),
    ),
    row(
        "26",
        "Path of Exile",
        c(4),
        T::Spell,
        &[],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "27",
        "Hopeful Jelly Bean",
        c(4),
        T::Spell,
        &[G::JellyBean],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "28",
        "Occidental",
        c(3),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(9, 5),
        st(18, 10),
    ),
    row(
        "29",
        "Oriental",
        c(3),
        T::Unit,
        &[G::Cn],
        R::Epic,
        st(5, 9),
        st(10, 18),
    ),
    row(
        "30",
        "Floodgates",
        c(3),
        T::Unit,
        &[],
        R::Epic,
        st(0, 12),
        st(0, 24),
    ),
    row(
        "31",
        "Level Up!",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "32",
        "Selfless Protector",
        c(1),
        T::Unit,
        &[G::Human],
        R::Epic,
        st(2, 1),
        st(4, 2),
    ),
    row(
        "33",
        "Guerilla Felinor",
        c(3),
        T::Unit,
        &[G::Felinor],
        R::Epic,
        st(4, 6),
        st(8, 12),
    ),
    row(
        "34",
        "Dino-size",
        c(3),
        T::Spell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "35",
        "Corpse Recycler",
        c(3),
        T::FieldSpell,
        &[],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "36",
        "Training Dummy",
        c(1),
        T::Unit,
        &[],
        R::Epic,
        st(0, 3),
        st(0, 6),
    ),
    row(
        "37",
        "Seance",
        c(2),
        T::Spell,
        &[],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "38",
        "Tutorial Master",
        c(4),
        T::Unit,
        &[G::Human],
        R::Legendary,
        st(4, 16),
        st(8, 32),
    ),
    row(
        "39",
        "Call to Chaos (Training-Grounds Edition)",
        c(4),
        T::Spell,
        &[G::CallToChaos],
        R::Legendary,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "39.1",
        "Training-Grounds Golem",
        c(4),
        T::Unit,
        &[G::Token],
        R::Token,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "40",
        "Goetzmann Game Design",
        c(0),
        T::Spell,
        &[G::Quickdraw],
        R::Mythic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "41",
        "Legacy: Bigot",
        c(2),
        T::Unit,
        &[G::Human, G::Legacy],
        R::Common,
        st(4, 1),
        st(8, 2),
    ),
    row(
        "42",
        "Legacy: Stockpile",
        c(1),
        T::Spell,
        &[G::Legacy],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "43",
        "Legacy: Mr. Vanilla",
        c(1),
        T::Unit,
        &[G::Human, G::Legacy],
        R::Common,
        st(4, 4),
        st(12, 12),
    ),
    row(
        "44",
        "Legacy: Duplicating Felinors",
        c(2),
        T::Unit,
        &[G::Felinor, G::Legacy],
        R::Rare,
        st(3, 4),
        st(6, 8),
    ),
    row(
        "45",
        "Legacy: Jlockeed's Weapons",
        c(4),
        T::FieldSpell,
        &[G::Jlockeed, G::Legacy],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "46",
        "Legacy: Hit Job",
        c(3),
        T::Spell,
        &[G::Legacy],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "47",
        "Legacy: Midrange Menace",
        c(3),
        T::Unit,
        &[G::Legacy],
        R::Common,
        st(9, 9),
        st(18, 18),
    ),
    row(
        "48",
        "Legacy: Jlockeed Shredder-10",
        c(3),
        T::Unit,
        &[G::Jlockeed, G::Legacy],
        R::Common,
        st(6, 10),
        st(12, 20),
    ),
    row(
        "49",
        "Legacy: Reoccurring Dream",
        c(1),
        T::Spell,
        &[G::Legacy],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "50",
        "Legacy: Glowy Jelly Bean",
        c(3),
        T::Spell,
        &[G::JellyBean, G::Legacy],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "51",
        "Legacy: Big Felinor",
        c(4),
        T::Unit,
        &[G::Felinor, G::Legacy],
        R::Rare,
        st(3, 10),
        st(6, 20),
    ),
    row(
        "52",
        "Legacy: Snom Bunny Mind Control",
        c(4),
        T::Spell,
        &[G::Legacy],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "53",
        "Legacy: Prejudiced Postdoc",
        c(2),
        T::Unit,
        &[G::Human, G::Legacy],
        R::Rare,
        st(2, 4),
        st(4, 8),
    ),
    row(
        "54",
        "Legacy: The Rock",
        c(4),
        T::Unit,
        &[G::Human, G::Legacy],
        R::Common,
        st(10, 10),
        st(20, 20),
    ),
    row(
        "55",
        "Legacy: Reminisce",
        c(1),
        T::Spell,
        &[G::Legacy],
        R::Rare,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "56",
        "Legacy: Corpse Eater",
        c(4),
        T::Unit,
        &[G::Legacy],
        R::Epic,
        st(2, 2),
        st(4, 4),
    ),
    row(
        "57",
        "Legacy: Recycling Initiative",
        c(0),
        T::Spell,
        &[G::Legacy],
        R::Epic,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "58",
        "Legacy: True Strike",
        c(1),
        T::Spell,
        &[G::Legacy],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "59",
        "Legacy: Fig of Life",
        c(3),
        T::Spell,
        &[G::Fruit, G::Legacy],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
    row(
        "60",
        "Legacy: Rush Token Farm",
        c(2),
        T::FieldSpell,
        &[G::Legacy],
        R::Common,
        NO_STATS,
        NO_STATS,
    ),
];

const FIXTURES: &[(SetName, &[SpecRow])] = &[
    (SetName::Core, CORE),
    (SetName::Classic, CLASSIC),
    (SetName::ClassicPlus, CLASSIC_PLUS),
    (SetName::Meditative, MEDITATIVE),
    (SetName::MeditativePlus, MEDITATIVE_PLUS),
    (SetName::Stinkyscape, STINKYSCAPE),
    (SetName::TrainingGrounds, TRAINING_GROUNDS),
];

/// BUILD M4-T1 and B2.4, plus the mechanics patch's Plague, patch v0.2.Y's Catalyst, Prime and
/// Acclaimed, the Meditative set's Wincon, the Meditative+ set's Potion, #1837's Terminus, Quest,
/// Otherworldly, Other-Origins and Jelly Bean (R1837000), the Stinkyscape set's Drone Core,
/// Weapon, War Doctrine and Swarm (R1857000), and #2521's Resource and Faction: the only tags any
/// entry may carry.
const ALLOWED_TAGS: &[&str] = &[
    "Human",
    "Felinor",
    "KY",
    "CN",
    "Fruit",
    "Call to Chaos",
    "Quickdraw",
    "Jlockeed",
    "Book",
    "Pancake",
    "AI",
    "Plague",
    "Catalyst",
    "Prime",
    "Acclaimed",
    "Wincon",
    "Potion",
    "Terminus",
    "Quest",
    "Otherworldly",
    "Other-Origins",
    "Jelly Bean",
    "Drone Core",
    "Weapon",
    "War Doctrine",
    "Swarm",
    "Legacy",
    "Resource",
    "Faction",
    "Token",
];

/// A set's census (`jackioh_cards::census`, where each set's counts are written once, #2581).
fn census_of(set: &str) -> &'static SetCensus {
    CENSUS
        .iter()
        .find(|census| census.set == set)
        .unwrap_or_else(|| panic!("the census has no {set}"))
}

/// A set's tokens in the census's order: those its cards define, then those they share.
fn census_tokens(census: &SetCensus) -> Vec<&'static str> {
    census
        .card_defined_tokens
        .iter()
        .chain(census.shared_tokens)
        .copied()
        .collect()
}

/// R1420: whether a set ships. A set that does not may hold fewer entries than its fixture and its
/// counts, never more, and none of them counts toward the catalog's totals.
fn ships(set: &str) -> bool {
    SetName::ALL
        .iter()
        .find(|known| known.as_str() == set)
        .is_some_and(|known| set_ships(*known))
}

/// B2.1's totals, as the census sums them over the sets that ship: the only count of the whole
/// catalog this file keeps.
fn total_cards() -> usize {
    CENSUS
        .iter()
        .filter(|census| ships(census.set))
        .map(SetCensus::card_count)
        .sum()
}

fn total_tokens() -> usize {
    CENSUS
        .iter()
        .filter(|census| ships(census.set))
        .map(SetCensus::token_count)
        .sum()
}

/// The entries of the sets that ship (R1420).
fn shipped_entries() -> Vec<&'static CardDef> {
    entries()
        .into_iter()
        .filter(|entry| set_ships(entry.set))
        .collect()
}

/// A set's census as one number (cards, named cards and tokens), for the fixture-row counts below.
fn set_size(set: &str) -> usize {
    CENSUS
        .iter()
        .find(|census| census.set == set)
        .map_or(0, |census| census.card_count() + census.token_count())
}

fn entries() -> Vec<&'static CardDef> {
    CATALOG.values().collect()
}

fn key_of(set: &str, index: &str) -> String {
    format!("{set} #{index}")
}

/// An index is unique only within its set (B2.2), so entries are found by `set` and `index`.
fn by_key() -> IndexMap<String, &'static CardDef> {
    let mut by_key: IndexMap<String, &'static CardDef> = IndexMap::new();
    for entry in entries() {
        let key = key_of(entry.set.as_str(), &entry.index);
        if by_key.contains_key(&key) {
            panic!("catalog has two entries with {key}");
        }
        by_key.insert(key, entry);
    }
    by_key
}

fn entry_for(by_key: &IndexMap<String, &'static CardDef>, set: SetName, index: &str) -> &'static CardDef {
    match by_key.get(&key_of(set.as_str(), index)) {
        Some(entry) => entry,
        None => panic!(
            "{} is in the fixture but missing from catalog.json",
            key_of(set.as_str(), index)
        ),
    }
}

fn stat_of(face: &CardFace, field: &str) -> Option<i32> {
    match field {
        "attack" => face.attack,
        _ => face.health,
    }
}

fn show(value: &Value) -> String {
    match value.as_str() {
        Some(text) => text.to_string(),
        None => value.to_string(),
    }
}

fn label(entry: &CardDef, field: &str, expected: &Value, actual: &Value) -> String {
    format!(
        "{} (#{}) {field}: expected {}, got {}",
        entry.id,
        entry.index,
        show(expected),
        show(actual)
    )
}

fn json<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("catalog values serialize")
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn row_problems(set: SetName, row: &SpecRow, by_key: &IndexMap<String, &'static CardDef>) -> Vec<String> {
    let entry = entry_for(by_key, set, row.index);
    let mut problems: Vec<String> = Vec::new();

    if entry.name != row.name {
        problems.push(label(entry, "name", &json(row.name), &json(&entry.name)));
    }
    if entry.cost != row.cost {
        problems.push(label(entry, "cost", &json(row.cost), &json(entry.cost)));
    }
    if entry.type_ != row.type_ {
        problems.push(label(entry, "type", &json(row.type_), &json(entry.type_)));
    }
    if entry.rarity != row.rarity {
        problems.push(label(entry, "rarity", &json(row.rarity), &json(entry.rarity)));
    }

    let mut expected_tags: Vec<&str> = row.tags.iter().map(|tag| tag.as_str()).collect();
    expected_tags.sort();
    let mut actual_tags: Vec<&str> = entry.tags.iter().map(|tag| tag.as_str()).collect();
    actual_tags.sort();
    if expected_tags != actual_tags {
        problems.push(label(
            entry,
            "tags (as a set)",
            &json(&expected_tags),
            &json(&actual_tags),
        ));
    }

    let stats: [(&str, &CardFace, StatPair); 2] = [
        ("base", &entry.base, row.base),
        ("radiant", &entry.radiant, row.radiant),
    ];
    for (face, actual, expected) in stats {
        if stat_of(actual, "attack") != expected.0 {
            problems.push(label(
                entry,
                &format!("{face}.attack"),
                &json(expected.0),
                &json(stat_of(actual, "attack")),
            ));
        }
        if stat_of(actual, "health") != expected.1 {
            problems.push(label(
                entry,
                &format!("{face}.health"),
                &json(expected.1),
                &json(stat_of(actual, "health")),
            ));
        }
    }
    problems
}

fn fixture_report(set: SetName) -> String {
    let by_key = by_key();
    let rows = FIXTURES
        .iter()
        .find(|(fixture_set, _)| *fixture_set == set)
        .map(|(_, rows)| *rows)
        .unwrap_or_default();
    rows.iter()
        // R1420: a row of a set that has not shipped, with no entry yet, is a card still being built.
        .filter(|row| set_ships(set) || by_key.contains_key(&key_of(set.as_str(), row.index)))
        .filter_map(|row| {
            let problems = row_problems(set, row, &by_key);
            if problems.is_empty() {
                None
            } else {
                Some(format!(
                    "{set} #{} {} vs its fixture:\n{}",
                    row.index,
                    row.name,
                    problems.join("\n")
                ))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// R1420, R1178000, R1857000: the ids of a held set's entries among `ids` that the catalog holds yet
/// (a card still being built has none), or all of them once its set ships, when a missing one fails
/// the list it is in. The ids of a shipped set pass unchanged. An id's set is the one its segment
/// spells (B2.2: `meditativeplus-…`, `stinkyscape-…`).
fn landed(ids: &[&str]) -> Vec<String> {
    ids.iter()
        .filter(|id| CATALOG.contains_key(**id) || set_of_id(id).is_none_or(set_ships))
        .map(|id| (*id).to_string())
        .collect()
}

/// B2.2: the set an id's segment spells, its name lowered with "+" written "plus" and "-" and " "
/// Training Grounds', the first set name with a space, R2194000).
fn set_of_id(id: &str) -> Option<SetName> {
    let segment = id.split('-').next()?;
    SetName::ALL.iter().copied().find(|set| {
        set.as_str()
            .to_lowercase()
            .replace('+', "plus")
            .replace(['-', ' '], "")
            == segment
    })
}

fn is_plain_index(index: &str) -> bool {
    !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit())
}

mod catalog_membership_build_m4_t1_b2_1 {
    use super::*;

    #[test]
    fn holds_b2_1_s_cards_and_tokens_every_entry_across_core_classic_and_classic_plus() {
        let cards = shipped_entries().into_iter().filter(|entry| !entry.token).count();
        let tokens = shipped_entries().into_iter().filter(|entry| entry.token).count();
        assert_eq!(cards, total_cards(), "entries with token: false");
        assert_eq!(tokens, total_tokens(), "entries with token: true");
        assert_eq!(
            shipped_entries().len(),
            total_cards() + total_tokens(),
            "shipped catalog entries"
        );
        for census in CENSUS {
            let in_set: Vec<&CardDef> = entries()
                .into_iter()
                .filter(|entry| entry.set.as_str() == census.set)
                .collect();
            let (cards, tokens) = (
                in_set.iter().filter(|entry| !entry.token).count(),
                in_set.iter().filter(|entry| entry.token).count(),
            );
            // R1857001: a set's cards are its numbered ones and those outside its numbers.
            let most_cards = census.card_count();
            if ships(census.set) {
                assert_eq!(cards, most_cards, "{} cards", census.set);
                assert_eq!(tokens, census.token_count(), "{} tokens", census.set);
            } else {
                // R1420: a set being built holds no more than its brief lists.
                assert!(
                    cards <= most_cards,
                    "{} cards: {cards} of at most {most_cards}",
                    census.set
                );
                assert!(
                    tokens <= census.token_count(),
                    "{} tokens: {tokens} of at most {}",
                    census.set,
                    census.token_count()
                );
            }
        }
    }

    #[test]
    fn has_each_set_s_indices_1_n_present_exactly_once_none_of_them_a_token() {
        let by_key = by_key();
        let mut missing: Vec<String> = Vec::new();
        for census in CENSUS {
            for n in 1..=census.cards {
                match by_key.get(&key_of(census.set, &n.to_string())) {
                    // R1420: a set that has not shipped need not hold every card yet.
                    None if !ships(census.set) => {}
                    None => missing.push(format!("{} #{n} missing", census.set)),
                    Some(entry) if entry.token => {
                        missing.push(format!(
                            "{} #{n} ({}) is flagged token: true",
                            census.set, entry.id
                        ));
                    }
                    Some(_) => {}
                }
            }
            let plain = entries()
                .into_iter()
                .filter(|entry| entry.set.as_str() == census.set && is_plain_index(&entry.index))
                .count();
            if ships(census.set) {
                assert_eq!(plain, census.cards, "{} plain numeric indices", census.set);
            } else {
                assert!(plain <= census.cards, "{} plain numeric indices", census.set);
            }
        }
        assert_eq!(missing, Vec::<String>::new(), "indices");
    }

    #[test]
    fn has_core_s_five_card_defined_tokens_the_four_shared_tokens_the_coin_and_the_ghoul_token() {
        let by_key = by_key();
        let expected = census_tokens(census_of("Core"));
        for index in expected.iter().copied() {
            let entry = by_key.get(&key_of("Core", index));
            assert_eq!(
                entry.map(|entry| entry.index.as_str()),
                Some(index),
                "token index {index}"
            );
            assert_eq!(
                entry.map(|entry| entry.token),
                Some(true),
                "token index {index} token flag"
            );
        }
        let mut flagged: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::Core && entry.token)
            .map(|entry| entry.index.clone())
            .collect();
        flagged.sort();
        let mut expected_sorted = strings(&expected);
        expected_sorted.sort();
        assert_eq!(flagged, expected_sorted, "Core entries flagged token: true");
    }

    #[test]
    fn has_classic_plus_s_card_defined_tokens_and_the_ten_ai_generated_cards_b2_1_b2_3_b8() {
        let tokens: Vec<String> = CLASSIC_PLUS
            .iter()
            .filter(|row| row.rarity == Rarity::Token)
            .map(|row| row.index.to_string())
            .collect();
        assert_eq!(tokens.len(), census_of("Classic+").token_count());
        assert_eq!(
            tokens.iter().filter(|index| index.starts_with("T-AI-")).count(),
            10
        );
        let mut flagged: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::ClassicPlus && entry.token)
            .map(|entry| entry.index.clone())
            .collect();
        flagged.sort();
        let mut tokens_sorted = tokens.clone();
        tokens_sorted.sort();
        assert_eq!(flagged, tokens_sorted);
    }

    #[test]
    fn holds_no_entry_its_set_s_fixture_does_not_list() {
        let known: Vec<String> = FIXTURES
            .iter()
            .flat_map(|(set, rows)| rows.iter().map(|row| key_of(set.as_str(), row.index)))
            .collect();
        let extra: Vec<String> = entries()
            .into_iter()
            .filter(|entry| !known.contains(&key_of(entry.set.as_str(), &entry.index)))
            .map(|entry| format!("{} ({})", entry.id, key_of(entry.set.as_str(), &entry.index)))
            .collect();
        assert_eq!(extra, Vec::<String>::new(), "catalog entries with no fixture row");
        for (set, rows) in FIXTURES {
            assert_eq!(rows.len(), set_size(set.as_str()), "{set} fixture rows");
        }
    }
}

mod every_core_entry_equals_its_fixture_row_build_m4_t1_m9_t1 {
    use super::*;

    #[test]
    fn every_core_entry_equals_its_fixture_row() {
        assert_eq!(fixture_report(SetName::Core), "");
    }
}

mod every_classic_entry_equals_its_fixture_row_build_m4_t1_m9_t1 {
    use super::*;

    #[test]
    fn every_classic_entry_equals_its_fixture_row() {
        assert_eq!(fixture_report(SetName::Classic), "");
    }
}

mod every_classic_plus_entry_equals_its_fixture_row_build_m4_t1_m9_t1 {
    use super::*;

    #[test]
    fn every_classic_plus_entry_equals_its_fixture_row() {
        assert_eq!(fixture_report(SetName::ClassicPlus), "");
    }
}

mod r1420_every_meditative_entry_equals_its_fixture_row_while_the_set_is_built {
    use super::*;

    #[test]
    fn r1420_every_meditative_entry_equals_its_fixture_row_and_none_is_missing_once_it_ships() {
        assert_eq!(fixture_report(SetName::Meditative), "");
        if set_ships(SetName::Meditative) {
            let by_key = by_key();
            let missing: Vec<&str> = MEDITATIVE
                .iter()
                .filter(|row| !by_key.contains_key(&key_of("Meditative", row.index)))
                .map(|row| row.name)
                .collect();
            assert_eq!(
                missing,
                Vec::<&str>::new(),
                "a shipped set holds every card it lists"
            );
        }
    }
}

mod r1178000_every_meditative_plus_entry_equals_its_fixture_row_while_the_set_is_built {
    use super::*;

    #[test]
    fn r1178000_every_meditative_plus_entry_equals_its_fixture_row_and_none_is_missing_once_it_ships() {
        assert_eq!(fixture_report(SetName::MeditativePlus), "");
        let by_key = by_key();
        let missing: Vec<&str> = MEDITATIVE_PLUS
            .iter()
            .filter(|row| !by_key.contains_key(&key_of("Meditative+", row.index)))
            .map(|row| row.name)
            .collect();
        if set_ships(SetName::MeditativePlus) {
            assert_eq!(
                missing,
                Vec::<&str>::new(),
                "a shipped set holds every card it lists"
            );
        }
        // Every entry of the set is `meditativeplus-NNN` or `meditativeplus-NNN-k` (B2.2's segment,
        // no hyphen), and none of them ships before the release (#1187).
        for entry in entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::MeditativePlus)
        {
            assert!(entry.id.starts_with("meditativeplus-"), "{}", entry.id);
        }
    }

    #[test]
    fn r1178000_the_set_s_census_is_26_cards_and_7_tokens_5_9_9_1_2() {
        let tokens: Vec<&str> = MEDITATIVE_PLUS
            .iter()
            .filter(|row| row.rarity == Rarity::Token)
            .map(|row| row.index)
            .collect();
        // #1563 added Bob's Plan's #1.3 Garage Sale and #1.4 Leg Day (R1563003), and #2521 its #1.5
        // Star Chart, Alteran Reach's plan (R2521008).
        assert_eq!(tokens, census_tokens(census_of("Meditative+")));
        let mut rarities: BTreeMap<&str, usize> = BTreeMap::new();
        for row in MEDITATIVE_PLUS.iter().filter(|row| row.rarity != Rarity::Token) {
            *rarities.entry(row.rarity.as_str()).or_insert(0) += 1;
        }
        let expected: BTreeMap<&str, usize> = census_of("Meditative+").rarities.iter().copied().collect();
        assert_eq!(rarities, expected);
        assert_eq!(MEDITATIVE_PLUS.len(), set_size("Meditative+"));
        // The Potion tag is the set's, on #11, #12 and #13 (#16's crafted potions are transient).
        let potions: Vec<&str> = MEDITATIVE_PLUS
            .iter()
            .filter(|row| row.tags.contains(&Tag::Potion))
            .map(|row| row.index)
            .collect();
        assert_eq!(potions, ["11", "12", "13"]);
    }

    #[test]
    fn r1178002_names_its_17_dogpile_since_classic_60_is_pile_on_and_shares_no_name() {
        assert_eq!(
            CATALOG.get("classic-060").map(|entry| entry.name.as_str()),
            Some("Pile On")
        );
        let names: Vec<&str> = MEDITATIVE_PLUS
            .iter()
            .filter(|row| row.rarity != Rarity::Token)
            .map(|row| row.name)
            .collect();
        assert!(names.contains(&"Dogpile"));
        let others: Vec<&str> = entries()
            .into_iter()
            .filter(|entry| entry.set != SetName::MeditativePlus && !entry.token)
            .map(|entry| entry.name.as_str())
            .collect();
        let clashes: Vec<&&str> = names.iter().filter(|name| others.contains(name)).collect();
        assert_eq!(
            clashes,
            Vec::<&&str>::new(),
            "R381, R739: one name names one card"
        );
    }
}

mod r1857000_every_stinkyscape_entry_equals_its_fixture_row_while_the_set_is_built {
    use super::*;

    fn row_at(index: &str) -> &'static SpecRow {
        STINKYSCAPE
            .iter()
            .find(|row| row.index == index)
            .unwrap_or_else(|| panic!("STINKYSCAPE has no row {index}"))
    }

    #[test]
    fn r1857000_every_stinkyscape_entry_equals_its_fixture_row_and_none_is_missing_once_it_ships() {
        assert_eq!(fixture_report(SetName::Stinkyscape), "");
        let by_key = by_key();
        let missing: Vec<&str> = STINKYSCAPE
            .iter()
            .filter(|row| !by_key.contains_key(&key_of("Stinkyscape", row.index)))
            .map(|row| row.name)
            .collect();
        if set_ships(SetName::Stinkyscape) {
            assert_eq!(
                missing,
                Vec::<&str>::new(),
                "a shipped set holds every card it lists"
            );
        }
        // Every entry of the set is `stinkyscape-…` (B2.2's segment), and since the release (#1884)
        for entry in entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::Stinkyscape)
        {
            assert!(entry.id.starts_with("stinkyscape-"), "{}", entry.id);
            assert_eq!(set_of_id(&entry.id), Some(SetName::Stinkyscape), "{}", entry.id);
        }
        assert!(
            set_ships(SetName::Stinkyscape),
            "the set ships from its release, #1884"
        );
    }

    #[test]
    fn r1857001_numbers_174_and_175_and_the_t_ids() {
        // The designer's second #21 and second #122 take the numbers after the list's last, and
        // #2521's two new cards the two after them.
        assert_eq!(row_at("174").name, "Aerelret");
        assert_eq!(row_at("175").name, "Frost Module");
        assert_eq!(row_at("176").name, "Preemptive Strike");
        assert_eq!(row_at("177").name, "Cloaking Module");
        // The plain indices are 1 to the census's count, each once; a token is N.k after its card N,
        // or T-name; the AI generated cards are T-AI-1 to T-AI-10.
        let plain: Vec<usize> = STINKYSCAPE
            .iter()
            .filter(|row| is_plain_index(row.index))
            .filter_map(|row| row.index.parse().ok())
            .collect();
        assert_eq!(
            plain,
            (1..=census_of("Stinkyscape").cards).collect::<Vec<usize>>()
        );
        for row in STINKYSCAPE.iter().filter(|row| !is_plain_index(row.index)) {
            let numbered = row
                .index
                .split_once('.')
                .is_some_and(|(card, k)| is_plain_index(card) && is_plain_index(k));
            assert!(
                numbered || row.index.starts_with("T-"),
                "{} {}",
                row.index,
                row.name
            );
        }
        let ai: Vec<&str> = STINKYSCAPE
            .iter()
            .filter(|row| row.index.starts_with("T-AI-"))
            .map(|row| row.index)
            .collect();
        assert_eq!(ai, (1..=10).map(|n| format!("T-AI-{n}")).collect::<Vec<_>>());
        for row in STINKYSCAPE.iter().filter(|row| row.index.starts_with("T-AI-")) {
            assert_eq!(row.tags, [Tag::Ai, Tag::Token], "{}", row.index);
            assert_eq!(row.rarity, Rarity::Token, "{}", row.index);
        }
    }

    #[test]
    fn r1857001_the_census_is_177_cards_the_dormant_starship_and_52_tokens() {
        let cards: Vec<&SpecRow> = STINKYSCAPE
            .iter()
            .filter(|row| row.rarity != Rarity::Token)
            .collect();
        let named: Vec<&str> = cards
            .iter()
            .filter(|row| !is_plain_index(row.index))
            .map(|row| row.index)
            .collect();
        let census = census_of("Stinkyscape");
        assert_eq!(cards.len(), census.card_count());
        assert_eq!(named, census.shared_cards);
        let tokens: Vec<&str> = STINKYSCAPE
            .iter()
            .filter(|row| row.rarity == Rarity::Token)
            .map(|row| row.index)
            .collect();
        assert_eq!(tokens.len(), census.token_count());
        assert_eq!(
            tokens.iter().filter(|index| index.starts_with("T-")).count(),
            census.shared_tokens.len(),
            "shared tokens"
        );
        assert_eq!(STINKYSCAPE.len(), set_size("Stinkyscape"));
        let mut rarities: BTreeMap<&str, usize> = BTreeMap::new();
        for row in &cards {
            *rarities.entry(row.rarity.as_str()).or_insert(0) += 1;
        }
        let expected: BTreeMap<&str, usize> = census.rarities.iter().copied().collect();
        assert_eq!(rarities, expected);
        // Its four tags are its own (R1857000): #27 to #36 Drone Core, #50 to #61 and #140 to #143
        // Weapon, the four #137 tokens War Doctrine, and the Swarm Units #91 and #92.
        let carrying = |tag: Tag| -> Vec<&str> {
            STINKYSCAPE
                .iter()
                .filter(|row| row.tags.contains(&tag))
                .map(|row| row.index)
                .collect()
        };
        let indices =
            |from: usize, to: usize| -> Vec<String> { (from..=to).map(|n| n.to_string()).collect() };
        assert_eq!(carrying(Tag::DroneCore), indices(27, 36));
        assert_eq!(
            carrying(Tag::Weapon),
            [indices(50, 61), indices(140, 143)].concat()
        );
        assert_eq!(carrying(Tag::WarDoctrine), ["137.1", "137.2", "137.3", "137.4"]);
        assert_eq!(carrying(Tag::Swarm), ["91", "92"]);
        for index in carrying(Tag::Swarm) {
            assert_eq!(row_at(index).type_, CardType::Unit, "a Swarm entry is a Unit");
        }
        // And #2521's two: the six Resources (Resource) and the five Faction cards (Faction).
        assert_eq!(
            carrying(Tag::Resource),
            [
                "T-reknite",
                "T-korrelite",
                "T-gellium",
                "T-axnit",
                "T-narcor",
                "T-red-narcor"
            ]
        );
        for index in carrying(Tag::Resource) {
            assert_eq!(row_at(index).rarity, Rarity::Token, "a Resource is a token");
        }
        assert_eq!(carrying(Tag::Faction), ["95", "159", "160", "161", "162"]);
        for index in carrying(Tag::Faction) {
            assert!(row_at(index).name.starts_with("Faction: "), "{index}");
        }
    }

    #[test]
    fn r1857001_the_dormant_starship_is_a_non_token_with_no_radiant_form_of_its_own() {
        let hull = row_at("T-dormant-starship");
        assert_eq!(
            (hull.name, hull.cost, hull.type_, hull.rarity),
            ("Dormant Starship", c(3), CardType::Unit, Rarity::Legendary)
        );
        assert_eq!((hull.base, hull.radiant), (st(0, 0), st(0, 0)));
        assert!(!hull.tags.contains(&Tag::Token));
        // Its id is the one the engine summons (R1852000).
        assert_eq!(
            format!("stinkyscape-{}", hull.index.to_lowercase()),
            jackioh_engine::DORMANT_STARSHIP_DEF_ID
        );
        if let Some(entry) = CATALOG.get(jackioh_engine::DORMANT_STARSHIP_DEF_ID) {
            assert!(!entry.token, "no token (R1852000)");
            assert_eq!(entry.radiant_fallback, Some(true), "R349's fallback");
            assert_eq!(entry.set, SetName::Stinkyscape);
            // In no deck (R1857001), even with its set open.
            assert!(!jackioh_engine::deckable(entry));
            assert!(jackioh_engine::made_by_a_rule(entry));
        }
    }
}

mod r2194000_every_training_grounds_entry_equals_its_fixture_row_while_the_set_is_built {
    use super::*;

    #[test]
    fn r2194000_every_training_grounds_entry_equals_its_fixture_row_and_none_is_missing_once_it_ships() {
        assert_eq!(fixture_report(SetName::TrainingGrounds), "");
        let by_key = by_key();
        let missing: Vec<&str> = TRAINING_GROUNDS
            .iter()
            .filter(|row| !by_key.contains_key(&key_of("Training Grounds", row.index)))
            .map(|row| row.name)
            .collect();
        if set_ships(SetName::TrainingGrounds) {
            assert_eq!(
                missing,
                Vec::<&str>::new(),
                "a shipped set holds every card it lists"
            );
        }
        // Every entry of the set is `traininggrounds-…` (B2.2's segment, its name's space dropped),
        // and since its release (#2204) the set ships.
        for entry in entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::TrainingGrounds)
        {
            assert!(entry.id.starts_with("traininggrounds-"), "{}", entry.id);
            assert_eq!(
                set_of_id(&entry.id),
                Some(SetName::TrainingGrounds),
                "{}",
                entry.id
            );
        }
        assert!(
            set_ships(SetName::TrainingGrounds),
            "the set ships from its release, #2204"
        );
    }

    #[test]
    fn r2194002_its_rows_are_1_to_60_and_the_one_token_39_1_with_the_designer_s_rarity_bands() {
        let plain: Vec<usize> = TRAINING_GROUNDS
            .iter()
            .filter(|row| is_plain_index(row.index))
            .filter_map(|row| row.index.parse().ok())
            .collect();
        assert_eq!(
            plain,
            (1..=census_of("Training Grounds").cards).collect::<Vec<usize>>()
        );
        let tokens: Vec<&str> = TRAINING_GROUNDS
            .iter()
            .filter(|row| !is_plain_index(row.index))
            .map(|row| row.index)
            .collect();
        assert_eq!(tokens, census_tokens(census_of("Training Grounds")));
        // #1-#15 Common, #16-#27 Rare, #28-#36 Epic, #37-#39 Legendary, #40 Mythic; the snapshots
        // keep their Core rarities, none Legendary or Mythic.
        for row in TRAINING_GROUNDS.iter().filter(|row| is_plain_index(row.index)) {
            let n: usize = row.index.parse().expect("a plain index");
            let band = match n {
                1..=15 => Some(Rarity::Common),
                16..=27 => Some(Rarity::Rare),
                28..=36 => Some(Rarity::Epic),
                37..=39 => Some(Rarity::Legendary),
                40 => Some(Rarity::Mythic),
                _ => None,
            };
            match band {
                Some(rarity) => assert_eq!(row.rarity, rarity, "#{n} {}", row.name),
                None => assert!(
                    !matches!(row.rarity, Rarity::Legendary | Rarity::Mythic),
                    "#{n} {}",
                    row.name
                ),
            }
        }
        let golem = TRAINING_GROUNDS
            .iter()
            .find(|row| row.index == "39.1")
            .expect("#39.1");
        assert_eq!(golem.rarity, Rarity::Token);
        if let Some(entry) = CATALOG.get("traininggrounds-039-1") {
            assert_eq!(
                entry.printed_rarity.map(|rarity| rarity.as_str()),
                Some("Legendary")
            );
        }
    }
}

mod rarity_distribution_spec_8_b2_5_build_m4_t1 {
    use super::*;

    #[test]
    fn is_core_32_40_16_7_5_classic_35_26_18_10_1_and_classic_plus_13_24_25_13_3_common_rare_epic_legendary_mythic()
     {
        for SetCensus {
            set,
            rarities: expected,
            ..
        } in CENSUS
        {
            let mut counted: BTreeMap<String, usize> = BTreeMap::new();
            for entry in entries()
                .into_iter()
                .filter(|entry| entry.set.as_str() == *set && !entry.token)
            {
                *counted.entry(entry.rarity.as_str().to_string()).or_insert(0) += 1;
            }
            let expected: BTreeMap<String, usize> = expected
                .iter()
                .map(|(rarity, n)| ((*rarity).to_string(), *n))
                .collect();
            if ships(set) {
                assert_eq!(counted, expected, "{set} rarity counts over its non-token cards");
            } else {
                // R1420: a set being built holds no more of a rarity than its brief lists.
                for (rarity, n) in &counted {
                    let most = expected.get(rarity).copied().unwrap_or(0);
                    assert!(*n <= most, "{set}: {n} {rarity} non-token cards, at most {most}");
                }
            }
        }
    }

    #[test]
    fn gives_every_token_rarity_token_and_no_card_rarity_token() {
        let wrong: Vec<String> = entries()
            .into_iter()
            .filter(|entry| (entry.rarity == Rarity::Token) != entry.token)
            .map(|entry| {
                label(
                    entry,
                    "rarity",
                    &json(if entry.token { "Token" } else { "not Token" }),
                    &json(entry.rarity),
                )
            })
            .collect();
        assert_eq!(wrong, Vec::<String>::new(), "token rarity");
    }

    #[test]
    fn r739_every_family_shares_one_rarity_issue_44() {
        let rarity_of = |id: &str| -> Rarity {
            match CATALOG.get(id) {
                Some(entry) => entry.rarity,
                None => panic!("R739 family member {id} is not in the catalog"),
            }
        };
        // cards spelled it differently before patch v0.3.Y (#1067) and differ in type and
        // effect shape, so the template does not reach them (R739). The House is no
        let families: &[(&str, Rarity, &[&str])] = &[
            (
                "Book of ___",
                Rarity::Epic,
                &[
                    "classic-003",
                    "classic-012",
                    "classic-016",
                    "classic-024",
                    "classic-029",
                    "classic-055",
                    "classic-070",
                    "classicplus-053",
                    "classicplus-054",
                    "classicplus-055",
                    "classicplus-056",
                    "classicplus-057",
                    "classicplus-071",
                    "classicplus-072",
                ],
            ),
            (
                "Call to Chaos",
                Rarity::Legendary,
                &["core-095", "classicplus-073", "meditative-095"],
            ),
            (
                "___ Glowy Jelly Bean",
                Rarity::Rare,
                &["core-026", "core-027", "core-028", "core-029"],
            ),
            ("The Power to ___", Rarity::Rare, &["classic-020", "classic-081"]),
            ("___ in the System", Rarity::Epic, &["classic-018", "classic-025"]),
            ("___ of the Forgotten", Rarity::Rare, &["core-040", "classic-001"]),
            ("Rapid ___", Rarity::Common, &["core-010", "classic-026"]),
            (
                "___ Shroom",
                Rarity::Epic,
                &["classicplus-001", "classicplus-002"],
            ),
            ("Plague Bringer", Rarity::Rare, &["classic-061", "classic-076"]),
            ("Jlockeed ___", Rarity::Common, &["core-013", "core-014"]),
            (
                "Right-house defender / Wrong-House Attacker",
                Rarity::Common,
                &["core-003", "classicplus-006"],
            ),
        ];
        for (family, rarity, members) in families {
            for id in *members {
                assert_eq!(rarity_of(id), *rarity, "R739 {family} member {id}");
            }
        }
    }

    #[test]
    fn r739_a_bigger_version_of_an_effect_is_never_a_lower_rarity_pile_on_over_call_to_arms() {
        let rank = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
        let position = |id: &str| -> Option<usize> {
            CATALOG
                .get(id)
                .and_then(|entry| rank.iter().position(|r| *r == entry.rarity.as_str()))
        };
        assert!(position("classic-060") >= position("core-069"));
    }

    /// R739: the names two non-token cards share, as "name (first, second)".
    fn shared_names(cards: &[&CardDef]) -> Vec<String> {
        let mut seen: IndexMap<&str, &CardDef> = IndexMap::new();
        let mut dupes: Vec<String> = Vec::new();
        for card in cards.iter().copied().filter(|card| !card.token) {
            if let Some(first) = seen.get(card.name.as_str()) {
                dupes.push(format!("{} ({}, {})", card.name, first.id, card.id));
            } else {
                seen.insert(card.name.as_str(), card);
            }
        }
        dupes
    }

    #[test]
    fn r739_one_name_names_one_card() {
        assert_eq!(
            shared_names(&entries()),
            Vec::<String>::new(),
            "cards sharing a name"
        );
    }

    /// R2195001, R2511000: a Legacy card (Training Grounds #41–#60) is named `Legacy: <its Core card's
    /// name>` and carries the Legacy tag beside its Core card's tags, so it shares no name with its
    /// Core card or any other; it keeps everything else.
    #[test]
    fn r2195001_r2511000_a_legacy_card_is_its_core_card_s_name_after_legacy_and_tagged_legacy() {
        let legacy: Vec<&CardDef> = entries()
            .into_iter()
            .filter(|card| card.tags.contains(&Tag::Legacy))
            .collect();
        assert_eq!(
            legacy.iter().map(|card| card.id.as_str()).collect::<Vec<_>>(),
            (41..=60)
                .map(|n| format!("traininggrounds-{n:03}"))
                .collect::<Vec<_>>()
        );
        for card in &legacy {
            assert_eq!(card.set, SetName::TrainingGrounds, "{}", card.id);
            assert_eq!(card.tags.last(), Some(&Tag::Legacy), "{}", card.id);
            assert!(card.name.starts_with("Legacy: "), "{}", card.name);
        }
        let core = &CATALOG["core-002"];
        let bigot = &CATALOG["traininggrounds-041"];
        assert_eq!(bigot.name, format!("Legacy: {}", core.name));
        assert_eq!(bigot.tags, [Tag::Human, Tag::Legacy]);
        assert_eq!(
            (bigot.cost, bigot.rarity, &bigot.base, &bigot.radiant),
            (core.cost, core.rarity, &core.base, &core.radiant)
        );
        assert_eq!(CATALOG["traininggrounds-060"].name, "Legacy: Rush Token Farm");
        // No name is shared, the Core card's and its Legacy card's included.
        let twin = CardDef {
            id: "traininggrounds-041".to_string(),
            set: SetName::TrainingGrounds,
            ..core.clone()
        };
        assert_eq!(
            shared_names(&[core, &twin]),
            ["Bigot (core-002, traininggrounds-041)"]
        );
        assert_eq!(shared_names(&[core, bigot]), Vec::<String>::new());
    }

    #[test]
    fn b2_5_prints_the_designer_s_rarity_on_the_classic_plus_tokens_he_rated_never_on_a_card_or_an_ai_generated_card()
     {
        let printed: BTreeMap<String, String> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::ClassicPlus)
            .filter_map(|entry| {
                entry
                    .printed_rarity
                    .map(|rarity| (entry.index.clone(), rarity.as_str().to_string()))
            })
            .collect();
        let mut legendary: Vec<String> = CLASSIC_PLUS
            .iter()
            .filter(|row| row.index.starts_with("12.") || row.index.starts_with("19."))
            .map(|row| row.index.to_string())
            .collect();
        legendary.extend(strings(&["42.1", "46.1", "73.1", "75.1"]));
        let mut expected: BTreeMap<String, String> = legendary
            .into_iter()
            .map(|index| (index, "Legendary".to_string()))
            .collect();
        for (index, rarity) in [
            ("32.1", "Epic"),
            ("32.2", "Epic"),
            ("32.3", "Epic"),
            ("38.1", "Epic"),
            ("36.1", "Rare"),
            ("76.1", "Rare"),
            ("65.1", "Common"),
            ("65.2", "Common"),
            ("65.3", "Rare"),
            ("65.4", "Legendary"),
            ("65.5", "Mythic"),
        ] {
            expected.insert(index.to_string(), rarity.to_string());
        }
        assert_eq!(printed, expected);
        // §7 prints a designer rarity on the Meditative tokens too (the next test, keyed apart:
        // The Stinkyscape set prints its tokens' too (R1857000, its §7 rows), and the held Training
        // Grounds set its #39.1 Training-Grounds Golem's, the designer's Legendary (R2194000).
        assert!(
            entries()
                .into_iter()
                .filter(|entry| entry.printed_rarity.is_some())
                .all(|entry| entry.token
                    && (entry.set == SetName::ClassicPlus
                        || entry.set == SetName::Meditative
                        || entry.set == SetName::MeditativePlus
                        || entry.set == SetName::Stinkyscape
                        || entry.set == SetName::TrainingGrounds))
        );
    }

    #[test]
    fn r1178000_prints_the_designer_s_rarity_on_meditative_plus_16_1_and_20_1_and_none_on_its_ai_helpers() {
        // §7: Jazakus Prime the designer's Epic, Lil' Kenny his Rare; the plan's AI generated
        // helpers, #1.1 Base Camp, #1.2 Spa Day, #1.3 Garage Sale, #1.4 Leg Day and #1.5 Star Chart
        // (#2521), none, as the T-AI cards (B2.5).
        const MEDITATIVE_PLUS_PRINTED: &[(&str, Option<&str>)] = &[
            ("1.1", None),
            ("1.2", None),
            ("1.3", None),
            ("1.4", None),
            ("1.5", None),
            ("16.1", Some("Epic")),
            ("20.1", Some("Rare")),
        ];
        let mut wrong: Vec<String> = Vec::new();
        for entry in entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::MeditativePlus)
        {
            let printed = entry.printed_rarity.map(|rarity| rarity.as_str());
            match MEDITATIVE_PLUS_PRINTED
                .iter()
                .find(|(index, _)| *index == entry.index)
            {
                None if printed.is_some() => wrong.push(format!("{} prints {printed:?}", entry.id)),
                Some((_, rarity)) if printed != *rarity => {
                    wrong.push(format!("{} prints {printed:?}, not {rarity:?}", entry.id));
                }
                _ => {}
            }
        }
        assert_eq!(wrong, Vec::<String>::new());
    }

    #[test]
    fn b2_5_prints_the_designer_s_rarity_on_every_meditative_token_the_catalog_holds() {
        // The rarity §7 and §8.8 print on each of the set's thirty tokens, by index.
        const MEDITATIVE_PRINTED: &[(&str, &str)] = &[
            ("19.1", "Epic"),
            ("22.1", "Epic"),
            ("28.1", "Common"),
            ("30.1", "Epic"),
            ("39.1", "Rare"),
            ("39.2", "Rare"),
            ("39.3", "Common"),
            ("39.4", "Mythic"),
            ("39.5", "Mythic"),
            ("45.1", "Legendary"),
            ("49.1", "Mythic"),
            ("49.2", "Mythic"),
            ("49.3", "Mythic"),
            ("70.1", "Common"),
            ("71.1", "Mythic"),
            ("91.1", "Epic"),
            ("93.1", "Common"),
            ("93.2", "Common"),
            ("93.3", "Common"),
            ("95.1", "Legendary"),
            ("96.1", "Rare"),
            ("97.1", "Common"),
            ("97.2", "Common"),
            ("97.3", "Rare"),
            ("97.4", "Rare"),
            ("97.5", "Epic"),
            ("97.6", "Epic"),
            ("97.7", "Epic"),
            ("97.8", "Legendary"),
            ("97.9", "Mythic"),
        ];
        // Every Meditative token the catalog holds yet (R1420: the set joins part by part), with
        let present: BTreeMap<String, Option<String>> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::Meditative && entry.token)
            .map(|entry| {
                (
                    entry.index.clone(),
                    entry.printed_rarity.map(|rarity| rarity.as_str().to_string()),
                )
            })
            .collect();
        assert!(!present.is_empty(), "the catalog holds Meditative tokens");
        let mut wrong: Vec<String> = Vec::new();
        for (index, printed) in &present {
            match MEDITATIVE_PRINTED.iter().find(|(row, _)| row == index) {
                None => wrong.push(format!("Meditative token {index} has no row")),
                Some((_, rarity)) if printed.as_deref() != Some(*rarity) => {
                    wrong.push(format!(
                        "Meditative token {index} prints {printed:?}, not {rarity}"
                    ));
                }
                Some(_) => {}
            }
        }
        assert_eq!(
            wrong,
            Vec::<String>::new(),
            "every present token prints its row's rarity"
        );
        let cards: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.set == SetName::Meditative && !entry.token)
            .filter(|entry| entry.printed_rarity.is_some())
            .map(|entry| entry.id.clone())
            .collect();
        assert_eq!(cards, Vec::<String>::new());
    }
}

/// SPEC §7, read at compile time (`include_str!`, no I/O at run time: CLAUDE.md rule 4). Unlike the
/// transcriptions above this is the note itself on purpose: the test below asks that every token
/// the catalog holds, of any set, has a row in §7's table, so a token added without one fails
const TOKENS_NOTE: &str = include_str!("../../../../spec/07-tokens.md");

/// The `(Token, Index, held)` cells of §7's token tables, row by row: the lines after each table's
/// header row and its rule, up to the first line that is not a row. The first table holds the
/// shipped sets' tokens; a later one (`held`) a set the catalog holds before it ships (R1420,
/// R1178000), whose rows may wait for their cards as its census does.
fn section_7_rows() -> Vec<(String, String, bool)> {
    let mut rows: Vec<(String, String, bool)> = Vec::new();
    let mut tables = 0;
    let mut lines = TOKENS_NOTE.lines();
    while let Some(line) = lines.next() {
        if !line.starts_with("| Token | Index |") {
            continue;
        }
        tables += 1;
        lines.next();
        for line in lines.by_ref() {
            if !line.starts_with("| ") {
                break;
            }
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            if let (Some(name), Some(index)) = (cells.get(1), cells.get(2)) {
                rows.push(((*name).to_string(), (*index).to_string(), tables > 1));
            }
        }
    }
    rows
}

mod r1177001_a_token_is_one_by_all_four_signals_spec_5_7 {
    use super::*;

    #[test]
    fn r1177001_every_token_has_token_true_rarity_token_the_token_tag_and_a_section_7_row_and_no_card_has_any()
     {
        let rows = section_7_rows();
        assert!(rows.len() >= 80, "§7's table reads as {} rows", rows.len());
        let mut wrong: Vec<String> = Vec::new();
        for entry in entries() {
            let signals = [
                ("token: true", entry.token),
                ("rarity Token", entry.rarity == Rarity::Token),
                ("the Token tag", entry.tags.contains(&Tag::Token)),
                (
                    "a §7 row",
                    rows.iter()
                        .any(|(name, index, _)| *name == entry.name && *index == entry.index),
                ),
            ];
            // `token` is the one the rules read (R11, R102); the label (R1177001) and every other
            for (signal, has) in signals {
                if has != entry.token {
                    let verdict = if has { "has" } else { "lacks" };
                    wrong.push(format!(
                        "{} ({} #{}) {verdict} {signal}",
                        entry.id, entry.name, entry.index
                    ));
                }
            }
        }
        assert_eq!(wrong, Vec::<String>::new(), "the four token signals disagree");
        let orphans: Vec<String> = rows
            .iter()
            .filter(|(name, index, held)| {
                !held
                    && !entries()
                        .into_iter()
                        .any(|entry| entry.token && entry.name == *name && entry.index == *index)
            })
            .map(|(name, index, _)| format!("{name} ({index})"))
            .collect();
        assert_eq!(orphans, Vec::<String>::new(), "§7 rows with no token");
    }
}

mod faces_that_change_more_than_text_b2_7_e40 {
    use super::*;

    #[test]
    fn gives_blood_moon_s_radiant_face_a_type_of_its_own_a_field_trap_and_no_other_face_one() {
        let typed: Vec<String> = entries()
            .into_iter()
            .flat_map(|entry| {
                [(FaceKind::Base, &entry.base), (FaceKind::Radiant, &entry.radiant)]
                    .into_iter()
                    .filter_map(move |(face, printed)| {
                        printed.type_.map(|kind| format!("{} {face} {kind}", entry.id))
                    })
            })
            .collect();
        assert_eq!(typed, strings(&["classicplus-022 radiant Field Trap"]));
        assert_eq!(
            CATALOG.get("classicplus-022").map(|entry| entry.type_),
            Some(CardType::Trap)
        );
    }

    #[test]
    fn prints_buff_billy_s_3x_3x_and_6x_6x_and_lil_kenny_s_x_x_and_2x_2x_as_x_multiples_and_no_other_face_has_them()
     {
        // Buff Billy's X is the one it is played for; Meditative+ #20.1 Lil' Kenny's the one M+ #20
        // summons it with (R1184003).
        let with_x: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.base.x_stats.is_some() || entry.radiant.x_stats.is_some())
            .map(|entry| entry.id.clone())
            .collect();
        assert_eq!(with_x, strings(&["classicplus-069", "meditativeplus-020-1"]));
        let billy = CATALOG.get("classicplus-069");
        assert_eq!(billy.map(|entry| entry.cost), Some(CardCost::X));
        assert_eq!(
            billy.and_then(|entry| entry.base.x_stats).map(json),
            Some(serde_json::json!({ "attack": 3, "health": 3 }))
        );
        assert_eq!(
            billy.and_then(|entry| entry.radiant.x_stats).map(json),
            Some(serde_json::json!({ "attack": 6, "health": 6 }))
        );
        let kenny = CATALOG.get("meditativeplus-020-1");
        assert_eq!(
            kenny.and_then(|entry| entry.base.x_stats).map(json),
            Some(serde_json::json!({ "attack": 1, "health": 1 }))
        );
        assert_eq!(
            kenny.and_then(|entry| entry.radiant.x_stats).map(json),
            Some(serde_json::json!({ "attack": 2, "health": 2 }))
        );
    }
}

mod names_r381_b2_8 {
    use super::*;

    fn name_of(id: &str) -> Option<String> {
        CATALOG.get(id).map(|entry| entry.name.clone())
    }

    #[test]
    fn r381_gives_no_two_cards_one_name_classic_55_is_book_of_wildfire_and_72_grand_counterspell() {
        let mut names: IndexMap<String, Vec<String>> = IndexMap::new();
        for entry in entries() {
            names
                .entry(entry.name.clone())
                .or_default()
                .push(entry.id.clone());
        }
        let shared: Vec<(String, Vec<String>)> = names.into_iter().filter(|(_, ids)| ids.len() > 1).collect();
        assert_eq!(shared, Vec::<(String, Vec<String>)>::new());
        assert_eq!(name_of("classic-016").as_deref(), Some("Book of Flame"));
        assert_eq!(name_of("classic-055").as_deref(), Some("Book of Wildfire"));
        assert_eq!(name_of("classic-017").as_deref(), Some("Counterspell"));
        assert_eq!(name_of("classic-072").as_deref(), Some("Grand Counterspell"));
        assert_eq!(
            CATALOG.get("classic-055").map(|entry| entry.rarity),
            Some(Rarity::Epic)
        );
        assert_eq!(
            CATALOG.get("classic-055").map(|entry| entry.tags.clone()),
            Some(vec![Tag::Book])
        );
    }

    #[test]
    fn r381_keeps_the_names_that_are_rules_words_exile_burn_echo_recycle_as_the_designer_named_them() {
        let names: Vec<Option<String>> = ["classic-010", "classic-036", "classic-057", "classic-030"]
            .into_iter()
            .map(name_of)
            .collect();
        assert_eq!(
            names,
            ["Exile", "Burn", "Echo", "Recycle"]
                .map(|name| Some(name.to_string()))
                .to_vec()
        );
    }
}

mod tag_vocabulary_build_m4_t1_b2_4 {
    use super::*;

    #[test]
    fn uses_only_human_felinor_ky_cn_fruit_call_to_chaos_quickdraw_jlockeed_book_pancake_ai_plague_catalyst_prime_acclaimed_and_token()
     {
        let mut wrong: Vec<String> = Vec::new();
        for entry in entries() {
            for tag in &entry.tags {
                if !ALLOWED_TAGS.contains(&tag.as_str()) {
                    wrong.push(label(
                        entry,
                        "tags",
                        &json(format!("one of {}", ALLOWED_TAGS.join(", "))),
                        &json(tag),
                    ));
                }
            }
        }
        assert_eq!(wrong, Vec::<String>::new(), "tags outside the allowed vocabulary");
    }
}

mod the_jlockeed_tag_spec_5_8_r278_b2_4 {
    use super::*;

    #[test]
    fn r278_tags_core_13_and_14_the_three_classic_plus_jlockeed_cards_48_51_and_52_classic_4_palantir_meditative_87_97_and_97_9_and_meditative_plus_4_and_no_other_entry()
     {
        let mut tagged: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.tags.contains(&Tag::Jlockeed))
            .map(|entry| entry.id.clone())
            .collect();
        tagged.sort();
        assert_eq!(
            tagged,
            strings(&[
                "classic-004",
                "classicplus-048",
                "classicplus-051",
                "classicplus-052",
                "core-013",
                "core-014",
                "meditative-087",
                "meditative-097",
                "meditative-097-9",
            ])
            .into_iter()
            .chain(landed(&["meditativeplus-004"]))
            // R1857000: the Stinkyscape set's seven, #124 to #130, each named for the faction.
            .chain(landed(&[
                "stinkyscape-124",
                "stinkyscape-125",
                "stinkyscape-126",
                "stinkyscape-127",
                "stinkyscape-128",
                "stinkyscape-129",
                "stinkyscape-130",
            ]))
            // R2195003: the held Training Grounds set's snapshots of Core #14 and #13 (TG #45, #48).
            .chain(landed(&["traininggrounds-045", "traininggrounds-048"]))
            .collect::<Vec<_>>(),
            "entries tagged Jlockeed"
        );
        // faction under one spelling since patch v0.3.Y (#1067) respelled the five names the
        // designer spelled with an h (B2.4) — plus Classic #4 Palantir, which the balance patch
        // inducted without renaming it, and M #87 Tatches the Totem, which carries every tribal
        // tag ("All Tribes", §8.8 row 87, R1424). M #97.9 Jlockeed's Headquarters is the
        // Meditative set's, a token that fills the board with the faction's Units (§8.8).
        let mut named: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.name.contains("Jlockeed"))
            .map(|entry| entry.id.clone())
            .collect();
        named.sort();
        // Meditative+ #4, the designer's "Jlockheed" Spell, names the faction since issue #1828 renamed
        let expected: Vec<String> = tagged
            .into_iter()
            .filter(|id| id != "classic-004" && id != "meditative-087")
            .collect();
        assert_eq!(named, expected, "entries whose name names Jlockeed");
    }

    #[test]
    fn r278_no_name_or_face_spells_the_faction_with_an_h() {
        let old = |text: &str| text.to_lowercase().contains("jlockh");
        for entry in entries() {
            assert!(!old(&entry.name), "{} names the faction with an h", entry.id);
            assert!(
                !old(&entry.face(FaceKind::Base).text),
                "{} base face spells the faction with an h",
                entry.id
            );
            assert!(
                !old(&entry.face(FaceKind::Radiant).text),
                "{} radiant face spells the faction with an h",
                entry.id
            );
        }
        let table = sidecar(chinese_json(), "crates/cards/chinese.json");
        for (id, entry) in &table {
            for field in ["name", "base", "radiant"] {
                let text = entry.get(field).and_then(Value::as_str).unwrap_or("");
                assert!(!old(text), "{id} chinese {field} spells the faction with an h");
            }
        }
    }
}

mod the_catalyst_prime_and_acclaimed_tags_spec_5_7_8_6_8_7_patch_v0_2_y {
    use super::*;

    fn tagged(tag: Tag) -> Vec<String> {
        let mut ids: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .map(|entry| entry.id.clone())
            .collect();
        ids.sort();
        ids
    }

    fn hyphenated_prime(text: &str) -> bool {
        text.match_indices("-Prime").any(|(at, found)| {
            !text[at + found.len()..]
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        })
    }

    #[test]
    fn tags_classic_plus_38_solarius_and_46_felinor_flagbearer_meditative_45_49_and_91_and_meditative_plus_16_catalyst_and_no_other_entry()
     {
        // Meditative #49 YileGPT since patch v0.3.10 (#831).
        assert_eq!(
            tagged(Tag::Catalyst),
            [
                strings(&[
                    "classicplus-038",
                    "classicplus-046",
                    "meditative-045",
                    "meditative-049",
                    "meditative-091"
                ]),
                landed(&["meditativeplus-016"]),
                // R1857000: Stinkyscape #147 IN33DH3.
                landed(&["stinkyscape-147"])
            ]
            .concat()
        );
    }

    #[test]
    fn tags_the_prime_tokens_prime_each_named_its_catalyst_s_name_and_a_spaced_prime_and_no_other_entry() {
        assert_eq!(
            tagged(Tag::Prime),
            [
                strings(&[
                    "classicplus-038-1",
                    "classicplus-046-1",
                    "meditative-045-1",
                    "meditative-049-1",
                    "meditative-091-1"
                ]),
                landed(&["meditativeplus-016-1"]),
                // R1857000: Stinkyscape #147.1 IN33DH3 Prime.
                landed(&["stinkyscape-147-1"])
            ]
            .concat()
        );
        // defines. M #49.1, the token M #49's Death puts in the deck, took the tag in patch v0.3.10
        // (#831) as YileGPT Unleashed, and its name in #1563: YileGPT Prime, of M #49 YileGPT.
        let mut named: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.name.ends_with(" Prime"))
            .map(|entry| entry.id.clone())
            .collect();
        named.sort();
        assert_eq!(named, tagged(Tag::Prime), "entries named <card> Prime");
        for id in tagged(Tag::Prime) {
            let prime = CATALOG.get(id.as_str());
            let catalyst = CATALOG.get(id.strip_suffix("-1").unwrap_or(id.as_str()));
            assert_eq!(prime.map(|entry| entry.token), Some(true), "{id}");
            assert!(
                catalyst.is_some_and(|entry| entry.tags.contains(&Tag::Catalyst)),
                "{id}"
            );
            // The designer named M #91.1 Windfurious Prime, not "Windfast Prime".
            if let Some(name) = match id.as_str() {
                "meditative-091-1" => Some("Windfurious Prime"),
                _ => None,
            } {
                assert_eq!(
                    prime.map(|entry| entry.name.clone()),
                    Some(name.to_string()),
                    "{id}"
                );
                continue;
            }
            assert_eq!(
                prime.map(|entry| entry.name.clone()),
                Some(format!(
                    "{} Prime",
                    catalyst.map_or("?", |entry| entry.name.as_str())
                )),
                "{id}"
            );
        }
        // #322 spaces both: no name or face text writes a hyphenated Prime.
        let hyphenated: Vec<String> = entries()
            .into_iter()
            .filter(|entry| {
                [&entry.name, &entry.base.text, &entry.radiant.text]
                    .iter()
                    .any(|text| hyphenated_prime(text))
            })
            .map(|entry| entry.id.clone())
            .collect();
        assert_eq!(hyphenated, Vec::<String>::new());
    }

    #[test]
    fn tags_classic_80_boom_big_max_classic_plus_37_wardrum_and_meditative_49_yilegpt_and_yilegpt_prime_acclaimed_and_no_other_entry()
     {
        // §8.8 rows 49 and 49.1 print the tag on YileGPT and YileGPT Prime (the designer's
        // "Acclaimed"); Yile's Virus and AI Girlfriend lost it in patch v0.3.10 (#831).
        assert_eq!(
            tagged(Tag::Acclaimed),
            [
                strings(&[
                    "classic-080",
                    "classicplus-037",
                    "meditative-049",
                    "meditative-049-1",
                ]),
                // R1857000: the Stinkyscape set's nine.
                landed(&[
                    "stinkyscape-006",
                    "stinkyscape-026",
                    "stinkyscape-037",
                    "stinkyscape-087",
                    "stinkyscape-088",
                    "stinkyscape-109",
                    "stinkyscape-110",
                    "stinkyscape-111",
                    "stinkyscape-115",
                ])
            ]
            .concat()
        );
    }
}

mod the_tags_issue_1837_moves_r1837000 {
    use super::*;

    fn tagged(tag: Tag) -> Vec<String> {
        let mut ids: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .map(|entry| entry.id.clone())
            .collect();
        ids.sort();
        ids
    }

    #[test]
    fn r1837000_tags_core_100_terminus_classic_90_quest_classic_plus_32_otherworldly_its_three_tokens_other_origins_and_core_26_to_29_jelly_bean_and_no_other_entry()
     {
        // R1857000: and the Stinkyscape set's, once each has landed: its #4's six Terminus
        // tokens, #7 and #112 (Quest), #10 to #12 (Otherworldly) and their nine Other-Origins tokens.
        assert_eq!(
            tagged(Tag::Terminus),
            [
                strings(&["core-100"]),
                landed(&[
                    "stinkyscape-004-1",
                    "stinkyscape-004-2",
                    "stinkyscape-004-3",
                    "stinkyscape-004-4",
                    "stinkyscape-004-5",
                    "stinkyscape-004-6",
                ])
            ]
            .concat()
        );
        assert_eq!(
            tagged(Tag::Quest),
            [
                strings(&["classic-090"]),
                landed(&["stinkyscape-007", "stinkyscape-112"])
            ]
            .concat()
        );
        assert_eq!(
            tagged(Tag::Otherworldly),
            [
                strings(&["classicplus-032"]),
                landed(&["stinkyscape-010", "stinkyscape-011", "stinkyscape-012"])
            ]
            .concat()
        );
        assert_eq!(
            tagged(Tag::OtherOrigins),
            [
                strings(&["classicplus-032-1", "classicplus-032-2", "classicplus-032-3"]),
                landed(&[
                    "stinkyscape-010-1",
                    "stinkyscape-010-2",
                    "stinkyscape-010-3",
                    "stinkyscape-011-1",
                    "stinkyscape-011-2",
                    "stinkyscape-011-3",
                    "stinkyscape-012-1",
                    "stinkyscape-012-2",
                    "stinkyscape-012-3",
                ])
            ]
            .concat()
        );
        // The held Training Grounds set's #27 Hopeful Jelly Bean and its snapshot #50 Glowy Jelly Bean
        // (part 2, #2195) join it as they land (R2194000).
        assert_eq!(
            tagged(Tag::JellyBean),
            [
                strings(&["core-026", "core-027", "core-028", "core-029"]),
                landed(&["traininggrounds-027", "traininggrounds-050"]),
            ]
            .concat()
        );
        // Stinkyscape #10 to #12's three (its maker the card whose number the token's carries).
        let mut beans: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.name.ends_with("Jelly Bean"))
            .map(|entry| entry.id.clone())
            .collect();
        beans.sort();
        assert_eq!(beans, tagged(Tag::JellyBean));
        for id in tagged(Tag::OtherOrigins) {
            assert_eq!(
                CATALOG.get(id.as_str()).map(|entry| entry.token),
                Some(true),
                "{id}"
            );
            let maker_id = id.rsplit_once('-').map_or(id.as_str(), |(maker, _)| maker);
            let maker = CATALOG.get(maker_id).expect("an Other-Origins token's maker");
            assert!(maker.tags.contains(&Tag::Otherworldly), "{maker_id}");
            assert!(maker.refs.as_ref().is_some_and(|refs| refs.contains(&id)), "{id}");
        }
        assert_eq!(
            CATALOG.get("classic-090").map(|entry| entry.tags.clone()),
            Some(vec![Tag::Quickdraw, Tag::Quest])
        );
    }

    #[test]
    fn r1837000_felinor_feelings_and_friend_of_felinors_are_no_felinors_and_no_field_spell_is_a_fruit() {
        for id in ["classic-032", "core-062", "classicplus-063", "classicplus-068"] {
            assert_eq!(
                CATALOG.get(id).map(|entry| entry.tags.clone()),
                Some(Vec::new()),
                "{id}"
            );
        }
        let fruit_field_spells: Vec<String> = tagged(Tag::Fruit)
            .into_iter()
            .filter(|id| {
                CATALOG
                    .get(id.as_str())
                    .is_some_and(|entry| entry.type_ == T::FieldSpell)
            })
            .collect();
        assert_eq!(fruit_field_spells, Vec::<String>::new());
        assert!(
            CATALOG
                .get("core-t-felinor")
                .is_some_and(|entry| entry.tags.contains(&Tag::Felinor))
        );
    }
}

mod every_entry_has_a_radiant_face_of_its_own_spec_5_2_r276 {
    use super::*;

    #[test]
    fn gives_no_entry_a_radiant_face_identical_to_its_base_face_the_five_that_had_none_included() {
        // R349: a card that prints no Radiant form has the fallback as its Radiant face, which doubles
        // its stats — for an X/X token, the X it is summoned with (`layers::worn_stats_override`) — so
        // it is the one entry whose printed faces may read alike. It is marked as such, and only a Unit.
        // A face's text is read with its own `params` values filled in (B3.4 rule 5): "Heal a target
        let printed = |entry: &CardDef, face: FaceKind| -> Value {
            let mut printed = json(entry.face(face));
            if let Some(object) = printed.as_object_mut() {
                object.insert("text".to_string(), Value::from(fill_params(entry, face, None)));
            }
            printed
        };
        // R674: Glitch is blank on both faces, the one entry exempt; its client face corrupts whatever it shows.
        let same: Vec<String> = entries()
            .into_iter()
            .filter(|entry| {
                entry.radiant_fallback != Some(true)
                    && entry.id != GLITCH_DEF_ID
                    && printed(entry, FaceKind::Radiant) == printed(entry, FaceKind::Base)
            })
            .map(|entry| format!("{} (#{}) radiant is a copy of base", entry.id, entry.index))
            .collect();
        assert_eq!(
            same,
            Vec::<String>::new(),
            "entries whose radiant face is identical to base"
        );
        let fallbacks: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.radiant_fallback == Some(true))
            .map(|entry| entry.id.clone())
            .collect();
        // R1857001: and the Stinkyscape set's Dormant Starship, which prints no Radiant form.
        assert_eq!(
            fallbacks,
            [
                strings(&["core-t-ghoul"]),
                landed(&["stinkyscape-t-dormant-starship"])
            ]
            .concat()
        );
    }
}

mod no_face_prints_taunt_beside_indestructible_spec_6_1_r347 {
    use super::*;

    // R347 keeps Taunt off an Indestructible unit whatever prints it, so a face printing both would
    // print a Taunt it never has. Patch v0.1.1 dropped Indestructible from the two faces that did, #55
    // and #56 radiant (issue #27), and none may print both again. Keymaster Keenus (M #65) printed every
    // keyword until patch v0.3.7 (R1084); it prints Equip and an Aura now (R781004), so no face does.
    #[test]
    fn r347_r1084_no_unit_face_prints_both() {
        let both: Vec<String> = entries()
            .into_iter()
            .filter(|entry| entry.type_ == CardType::Unit)
            .flat_map(|entry| {
                [(FaceKind::Base, &entry.base), (FaceKind::Radiant, &entry.radiant)]
                    .into_iter()
                    .filter_map(move |(face, printed)| {
                        let kinds: Vec<KeywordKind> =
                            printed.keywords.iter().map(|keyword| keyword.kind()).collect();
                        (kinds.contains(&KeywordKind::Taunt) && kinds.contains(&KeywordKind::Indestructible))
                            .then(|| format!("{} {face}", entry.id))
                    })
            })
            .collect();
        assert!(both.is_empty(), "{both:?}");
    }
}

mod every_entry_printed_in_chinese_is_cn_r1176000 {
    use super::*;

    fn is_han(c: char) -> bool {
        matches!(
            c,
            '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{3134f}'
        )
    }

    // R1176000: every card in Chinese is CN. A card printed in Chinese (a Han character in its name or
    #[test]
    fn r1176000_every_entry_with_a_han_character_in_its_name_or_text_has_the_cn_tag() {
        let printed: Vec<&CardDef> = entries()
            .into_iter()
            .filter(|entry| {
                [&entry.name, &entry.base.text, &entry.radiant.text]
                    .iter()
                    .any(|text| text.chars().any(is_han))
            })
            .collect();
        assert!(printed.iter().any(|entry| entry.id == "meditative-032"));
        let missing: Vec<&str> = printed
            .iter()
            .filter(|entry| !entry.tags.contains(&Tag::Cn))
            .map(|entry| entry.id.as_str())
            .collect();
        assert!(
            missing.is_empty(),
            "printed in Chinese without the CN tag: {missing:?}"
        );
    }
}
