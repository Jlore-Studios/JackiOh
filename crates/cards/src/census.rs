//! The census of every set the catalog holds (#2581): each set's cards, the cards outside its
//! numbers, its tokens, its rarities, and how many entries carry each tag. It is written once, here,
//! and read by `tests/cross/catalog.rs` and `cargo jackioh catalog check` (`crates/tools/src/catalog.rs`);
//! every total either of them holds the catalog to is a sum of it.
//!
//! Every number is a literal transcribed from SPEC §1, §7 and §8 and the set briefs, never computed
//! from `catalog.json` (#108): the census is the independent proof the catalog is held to, so a card
//! that drifts onto, off or across a count in the catalog alone fails both.

use jackioh_engine::{SetName, Tag, set_ships};

/// One set's census.
pub struct SetCensus {
    /// The set's name, as `SetName::as_str` writes it.
    pub set: &'static str,
    /// B2.2: its id segment (`classicplus` holds no hyphen).
    pub segment: &'static str,
    /// Non-token cards, indexed 1..=cards.
    pub cards: usize,
    /// Non-token cards outside the set's numbers, indexed T-name (R1857001: Stinkyscape's Dormant
    /// Starship). Each counts as a card, beside `cards`.
    pub shared_cards: &'static [&'static str],
    /// Tokens a card defines, indexed N.k after card N.
    pub card_defined_tokens: &'static [&'static str],
    /// Tokens no one card defines, indexed T-name.
    pub shared_tokens: &'static [&'static str],
    /// The rarity distribution over its non-token cards.
    pub rarities: &'static [(&'static str, usize)],
}

impl SetCensus {
    /// Its cards, numbered and not.
    pub const fn card_count(&self) -> usize {
        self.cards + self.shared_cards.len()
    }

    /// Its tokens.
    pub const fn token_count(&self) -> usize {
        self.card_defined_tokens.len() + self.shared_tokens.len()
    }

    /// R1420: whether the set ships. One that does not is held to its shape alone and counts toward
    /// no total and no tag.
    pub fn ships(&self) -> bool {
        SetName::ALL
            .iter()
            .find(|set| set.as_str() == self.set)
            .is_some_and(|set| set_ships(*set))
    }
}

/// Every set the catalog holds, in the order `catalog check` prints them.
pub const CENSUS: &[SetCensus] = &[
    // §8 + §7: the Core indices, the tokens its cards define, and the named ones: the tokens several
    // cards share, The Coin, which §2.1's setup deals (R244), and the Ghoul Token, a card of its own
    // in patch v0.1.1 (R353). §8's distribution, as patch v0.2.9 (issue #44, which made #27, #28 and
    // #40 Rare) left it.
    SetCensus {
        set: "Core",
        segment: "core",
        cards: 100,
        shared_cards: &[],
        card_defined_tokens: &["51.1", "65.1", "90.1", "93.1", "95.1"],
        shared_tokens: &["T-rush", "T-sheep", "T-felinor", "T-bread", "T-coin", "T-ghoul"],
        rarities: &[
            ("Common", 32),
            ("Rare", 40),
            ("Epic", 16),
            ("Legendary", 7),
            ("Mythic", 5),
        ],
    },
    // B2.1, B2.5: its cards and the designer's rarities, as patch v0.2.9 (issue #44) left them, and
    // the shared token issue #170 adds, Glitch, which only R673's roll ever makes (R674).
    SetCensus {
        set: "Classic",
        segment: "classic",
        cards: 90,
        shared_cards: &[],
        card_defined_tokens: &[],
        shared_tokens: &["T-glitch"],
        rarities: &[
            ("Common", 35),
            ("Rare", 26),
            ("Epic", 18),
            ("Legendary", 10),
            ("Mythic", 1),
        ],
    },
    // B2.1, B2.3, B2.5, B8: its cards, the tokens they define and the AI generated cards, with the
    // rarities patch v0.2.9 (issue #44) left them.
    SetCensus {
        set: "Classic+",
        segment: "classicplus",
        cards: 78,
        shared_cards: &[],
        card_defined_tokens: &[
            "12.1", "12.2", "12.3", "12.4", "12.5", "12.6", "12.7", "12.8", "19.1", "19.2", "19.3", "19.4",
            "19.5", "32.1", "32.2", "32.3", "36.1", "38.1", "42.1", "46.1", "65.1", "65.2", "65.3", "65.4",
            "65.5", "73.1", "75.1", "76.1",
        ],
        shared_tokens: &[
            "T-AI-1", "T-AI-2", "T-AI-3", "T-AI-4", "T-AI-5", "T-AI-6", "T-AI-7", "T-AI-8", "T-AI-9",
            "T-AI-10",
        ],
        rarities: &[
            ("Common", 13),
            ("Rare", 24),
            ("Epic", 25),
            ("Legendary", 13),
            ("Mythic", 3),
        ],
    },
    // R1420: the Meditative set (issue #496, docs/meditative-set.md M2): its cards and the tokens they
    // define, and the designer's rarities, those it left out filled in by §8's rubric. It ships since
    // issue #553.
    SetCensus {
        set: "Meditative",
        segment: "meditative",
        cards: 102,
        shared_cards: &[],
        card_defined_tokens: &[
            "19.1", "22.1", "28.1", "30.1", "39.1", "39.2", "39.3", "39.4", "39.5", "45.1", "49.1", "49.2",
            "49.3", "70.1", "71.1", "91.1", "93.1", "93.2", "93.3", "95.1", "96.1", "97.1", "97.2", "97.3",
            "97.4", "97.5", "97.6", "97.7", "97.8", "97.9",
        ],
        shared_tokens: &[],
        rarities: &[
            ("Common", 29),
            ("Rare", 29),
            ("Epic", 22),
            ("Legendary", 16),
            ("Mythic", 6),
        ],
    },
    // R1420, R1178000: the Meditative+ set (issue #1173, docs/meditative-plus-set.md MP2): its cards
    // and the tokens they define (#1.1 Base Camp, #1.2 Spa Day, since #1563 #1.3 Garage Sale and #1.4
    // Leg Day, and since #2521 #1.5 Star Chart, the AI generated helpers of #1 Bob's Plan; #16.1
    // Jazakus Prime; #20.1 Lil' Kenny), the rarities the designer printed and #26 Scratch Pad's
    // Mythic. It ships since its release (#1187).
    SetCensus {
        set: "Meditative+",
        segment: "meditativeplus",
        cards: 26,
        shared_cards: &[],
        card_defined_tokens: &["1.1", "1.2", "1.3", "1.4", "1.5", "16.1", "20.1"],
        shared_tokens: &[],
        rarities: &[
            ("Common", 5),
            ("Rare", 9),
            ("Epic", 9),
            ("Legendary", 1),
            ("Mythic", 2),
        ],
    },
    // R1420, R1857000, R1857001: the Stinkyscape set (issue #1836, docs/stinkyscape-set.md ST12): its
    // cards (the designer's second #21 and #122 numbered #174 and #175, and #2521's #176 Preemptive
    // Strike and #177 Cloaking Module), the tokens they define, the Dormant Starship (a card outside
    // the numbers, the set's shared card), and its shared tokens: the drones, the Drone Battleship's
    // layers, the Resources, Anomaly Found! and the AI generated cards. Its rarities count the Dormant
    // Starship among the Legendaries (#2521: #137 Starfleet Commander, Common until then, and #176 are
    // Legendary, #177 Rare). It ships since its release (#1884).
    SetCensus {
        set: "Stinkyscape",
        segment: "stinkyscape",
        cards: 177,
        shared_cards: &["T-dormant-starship"],
        card_defined_tokens: &[
            "4.1", "4.2", "4.3", "4.4", "4.5", "4.6", "8.1", "10.1", "10.2", "10.3", "11.1", "11.2", "11.3",
            "12.1", "12.2", "12.3", "18.1", "18.2", "18.3", "84.1", "112.1", "135.1", "137.1", "137.2",
            "137.3", "137.4", "147.1", "147.2", "148.1", "150.1",
        ],
        shared_tokens: &[
            "T-drone-interceptor",
            "T-drone-corvette",
            "T-shield-layer",
            "T-propagation-layer",
            "T-meltdown-layer",
            "T-reknite",
            "T-korrelite",
            "T-gellium",
            "T-axnit",
            "T-narcor",
            "T-red-narcor",
            "T-anomaly-found",
            "T-AI-1",
            "T-AI-2",
            "T-AI-3",
            "T-AI-4",
            "T-AI-5",
            "T-AI-6",
            "T-AI-7",
            "T-AI-8",
            "T-AI-9",
            "T-AI-10",
        ],
        rarities: &[
            ("Common", 41),
            ("Rare", 51),
            ("Epic", 43),
            ("Legendary", 31),
            ("Mythic", 12),
        ],
    },
    // R1420, R2194000: the Training Grounds set (issue #2193, docs/training-grounds-set.md TG2): its
    // cards (the designer's, then part 2's Legacy cards) and the one token #39 defines, #39.1, with
    // the designer's rarity bands and the Legacy cards' Core rarities. It ships since its release
    // (#2204).
    SetCensus {
        set: "Training Grounds",
        segment: "traininggrounds",
        cards: 60,
        shared_cards: &[],
        card_defined_tokens: &["39.1"],
        shared_tokens: &[],
        rarities: &[
            ("Common", 26),
            ("Rare", 19),
            ("Epic", 11),
            ("Legendary", 3),
            ("Mythic", 1),
        ],
    },
];

/// §5, §7, §8, B2.4: how many catalog entries carry each tag, tokens included, so a tag that drifts
/// onto or off a card fails catalog check (R278: Jlockeed is Core #13 and #14's and the three
/// Classic+ Jlockeed cards'). The match is exhaustive over the union. Each count is Core's,
/// Classic's and Classic+'s plus each later set's from its brief, counted once its set ships (R1420,
/// R1178000, R1857000, R2194000); the Token count is the census's token total.
pub fn tag_count(tag: Tag) -> usize {
    match tag {
        // Meditative+ #7 Life Mage, #8 Arch Mage, #10 Medina Grifter, #16 Jazakus and #16.1 Jazakus Prime.
        // Stinkyscape #6, #14, #15, #17, #26, #37, #42, #45, #88, #94, #96, #103, #109, #110, #111, #113,
        // #114, #115, #147 and #147.1, and since #2521 #137 Starfleet Commander.
        // Training Grounds #1, #2, #3, #5, #8, #9, #18, #25, #28, #32 and #38, and the Legacy cards #41, #43,
        // #53 and #54.
        Tag::Human => 39 + meditative(16) + meditative_plus(5) + stinkyscape(21) + training_grounds(15),
        // Core #62 Friend of Felinors and Classic #32 Felinor Feelings lost it in #1837 (R1837000).
        // Stinkyscape #87.
        // Training Grounds #17, #22, #23 and #33, and the Legacy cards #44 and #51.
        Tag::Felinor => 9 + meditative(11) + stinkyscape(1) + training_grounds(6),
        // Meditative+ #20 KY's Competition, #20.1 Lil' Kenny and #25 KY's Intuition.
        // Stinkyscape #4.2 and #25.
        Tag::Ky => 9 + meditative(6) + meditative_plus(3) + stinkyscape(2),
        // Meditative+ #21 Harmony Spirit.
        // Stinkyscape #142.
        // Training Grounds #29 Oriental.
        Tag::Cn => 7 + meditative(33) + meditative_plus(1) + stinkyscape(1) + training_grounds(1),
        // Meditative #4 Juicy Kumquat Melon and #55 Dragon Fruit; C+ #61 Bauble Bubble lost it in
        // patch v0.3.10 (#831), and Classic+ #63 Fruit Tree and #68 Organic Produce, the two Field
        // Spells that make Fruit, in #1837 (R1837000). Meditative+ #23 Enormous Elderberry.
        // Stinkyscape #21.
        // Training Grounds' Legacy card #59 Legacy: Fig of Life.
        Tag::Fruit => 12 + meditative(2) + meditative_plus(1) + stinkyscape(1) + training_grounds(1),
        // Meditative #95 Call to Chaos (Meditative Edition).
        // Stinkyscape #150 and #151.
        // Training Grounds #39.
        Tag::CallToChaos => 2 + meditative(1) + stinkyscape(2) + training_grounds(1),
        // Core #84 Going Long and Meditative #80 Aluneth lost it in #2521.
        // Meditative+ #1 Bob's Plan.
        // Stinkyscape #16, #44, #116, #148 and #154.
        // Training Grounds #40 Goetzmann Game Design.
        Tag::Quickdraw => 4 + meditative(4) + meditative_plus(1) + stinkyscape(5) + training_grounds(1),
        // Meditative #87 Tatches the Totem (every tribal tag, R1424), #97 and #97.9; Meditative+ #4
        // Jlockeed's Supply Drop (the designer's "Jlockheed").
        // Stinkyscape #124 to #130.
        // Training Grounds' Legacy cards #45 and #48.
        Tag::Jlockeed => 6 + meditative(3) + meditative_plus(1) + stinkyscape(7) + training_grounds(2),
        // Meditative+ #15 Book of Potions.
        // Stinkyscape #102.
        Tag::Book => 14 + meditative_plus(1) + stinkyscape(1),
        Tag::Pancake => 10,
        // Meditative+ #1.1 Base Camp, #1.2 Spa Day, #1.3 Garage Sale, #1.4 Leg Day and, since #2521,
        // #1.5 Star Chart, #1 Bob's Plan's AI generated helpers.
        // Stinkyscape's ten AI generated cards, T-AI-1 to T-AI-10.
        Tag::Ai => 10 + meditative_plus(5) + stinkyscape(10),
        // Classic+ #7 The House since #1704, and #39 Book Worm since #1828.
        // Stinkyscape #40 Poison Capsule.
        Tag::Plague => 19 + meditative(2) + stinkyscape(1),
        // Meditative #45, #49 (patch v0.3.10, #831) and #91; their Primes #45.1, #49.1 and #91.1.
        // Meditative+ #16 Jazakus and its #16.1 Jazakus Prime.
        // Stinkyscape #147 (Catalyst) and its #147.1 (Prime).
        Tag::Catalyst => 2 + meditative(3) + meditative_plus(1) + stinkyscape(1),
        Tag::Prime => 2 + meditative(3) + meditative_plus(1) + stinkyscape(1),
        // Meditative #49 and #49.1; #49.2 and #49.3 lost it in patch v0.3.10 (#831).
        // Stinkyscape #6, #26, #37, #87, #88, #109, #110, #111 and #115.
        Tag::Acclaimed => 2 + meditative(2) + stinkyscape(9),
        // Meditative #8 Reach the Summit and #20 Aestheticize the Game, the tag's only cards.
        // Stinkyscape #148 and #148.1.
        Tag::Wincon => meditative(2) + stinkyscape(2),
        // Meditative+ #11, #12 and #13, the Living Potions, the tag's only catalog cards (R1178000).
        Tag::Potion => meditative_plus(3),
        // #1837, ahead of the Stinkyscape set (R1837000): Core #100 Ceaseless Void; Classic #90 In Too
        // Deep; Classic+ #32 Otherworldly Removal; its three tokens, #32.1 Execute, #32.2 Brawl and
        // #32.3 Blade Storm; and Core #26 to #29, the four Glowy Jelly Beans.
        // Stinkyscape: #4.1 to #4.6 (Terminus); #7 and #112 (Quest); #10 to #12 (Otherworldly) and
        // their nine tokens, #10.1 to #12.3 (Other-Origins).
        Tag::Terminus => 1 + stinkyscape(6),
        Tag::Quest => 1 + stinkyscape(2),
        Tag::Otherworldly => 1 + stinkyscape(3),
        Tag::OtherOrigins => 3 + stinkyscape(9),
        // Training Grounds #27 Hopeful Jelly Bean and the Legacy card #50 Legacy: Glowy Jelly Bean.
        Tag::JellyBean => 4 + training_grounds(2),
        // R1857000, the Stinkyscape set's four: #27 to #36 (Drone Core); #50 to #61 and #140 to #143
        // (Weapon); the four #137 tokens (War Doctrine); #91 and #92, its Swarm Units (Swarm).
        Tag::DroneCore => stinkyscape(10),
        Tag::Weapon => stinkyscape(16),
        Tag::WarDoctrine => stinkyscape(4),
        Tag::Swarm => stinkyscape(2),
        // R2511000: Training Grounds #41 to #60, its twenty Legacy cards.
        Tag::Legacy => training_grounds(20),
        // #2521: the Stinkyscape set's six Resources, T-reknite, T-korrelite, T-gellium, T-axnit,
        // T-narcor and T-red-narcor (Resource), and its five Faction cards, #95 and #159 to #162
        // (Faction).
        Tag::Resource => stinkyscape(6),
        Tag::Faction => stinkyscape(5),
        // Every token of the sets that ship carries it, so its count is their tokens' sum (R1420).
        Tag::Token => CENSUS
            .iter()
            .filter(|census| census.ships())
            .map(SetCensus::token_count)
            .sum(),
    }
}

/// R1420: a Meditative entry counts toward the totals and the tag census only once its set ships,
/// so the census holds on either side of the patch that lists it in `SHIPPED_SETS`.
fn meditative(count: usize) -> usize {
    if set_ships(SetName::Meditative) { count } else { 0 }
}

/// R1420, R1178000: the same for a Meditative+ entry, held until its release (#1187).
fn meditative_plus(count: usize) -> usize {
    if set_ships(SetName::MeditativePlus) {
        count
    } else {
        0
    }
}

/// R1420, R1857000: the same for a Stinkyscape entry, held until its release (#1884).
fn stinkyscape(count: usize) -> usize {
    if set_ships(SetName::Stinkyscape) { count } else { 0 }
}

/// R2194000: `count` once the Training Grounds set ships, else 0.
fn training_grounds(count: usize) -> usize {
    if set_ships(SetName::TrainingGrounds) {
        count
    } else {
        0
    }
}
