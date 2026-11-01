//! S #163 Swear Loyalities (SPEC §8.10 row S 163; §2.4, §5.2, §6.3, R81, R2521004): (0) Spell, Rare.
//!   Base:    "Swear Allegiance to a faction: add its Faction card to your hand, then exile every card in
//!            your hand and deck that belongs to another faction."
//!   Radiant: "Swear Allegiance to a faction: add its Radiant Faction card to your hand, then exile every
//!            card in your hand and deck that belongs to another faction."
//!
//! Engine: five modes, one per faction, chosen with the play (R81, S #121 System Schematic's pattern), all
//! five offered (R1880002); the Cry is one `add_to_hand` of the chosen faction's Faction card (S #95, #159
//! to #162), a full hand burning it (§2.4), Radiant on the Radiant face (§5.2). A faction is its Faction
//! card and the cards that card adds (#2521's table, the consts below). Then one `exile` per card in its
//! caster's library, then hand (#94 Genn's Greed's order, R135), whose definition is of another faction
//! and not of the chosen one (R2521004); the piles are read once the Faction card is added, so it stays.
//! The field, its caster's graveyard and the opponent's piles are not read (the opponent's untouched as
//! in R1880001, its "wherever it is" narrowed to the hand and deck by R2521004). "Golden" is Golden
//! Accord (ST8). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-163";

/// ETHICS: S #159 Faction: ETHICS and the six cards it adds.
const ETHICS: [&str; 7] = [
    "stinkyscape-159", // Faction: ETHICS (S #159)
    "stinkyscape-110", // iKwzki (S #110)
    "stinkyscape-111", // Shockwave447 (S #111)
    "stinkyscape-109", // xxFreshJack (S #109)
    "stinkyscape-020", // Tnack Trav (S #20)
    "stinkyscape-037", // Kwzka (S #37)
    "stinkyscape-006", // Tylannt (S #6)
];

/// Vanguard Coalition: S #95 Faction: Vanguard Coalition and the six cards it adds.
const VANGUARD_COALITION: [&str; 7] = [
    "stinkyscape-095", // Faction: Vanguard Coalition (S #95)
    "stinkyscape-115", // Scornboi (S #115)
    "stinkyscape-096", // Ocean Tsunami (S #96)
    "stinkyscape-006", // Tylannt (S #6)
    "stinkyscape-103", // Vxyl (S #103)
    "stinkyscape-087", // Siva 1 (S #87)
    "stinkyscape-088", // Siva 2 (S #88)
];

/// Golden Accord: S #160 Faction: Golden Accord and the four cards it adds.
const GOLDEN_ACCORD: [&str; 5] = [
    "stinkyscape-160", // Faction: Golden Accord (S #160)
    "stinkyscape-072", // Paragon Mk1 (S #72)
    "stinkyscape-073", // Paragon Mk2 (S #73)
    "stinkyscape-074", // Paragon-X (S #74)
    "stinkyscape-069", // Falcon (S #69)
];

/// Pegasus: S #161 Faction: Pegasus and the four cards it adds.
const PEGASUS: [&str; 5] = [
    "stinkyscape-161", // Faction: Pegasus (S #161)
    "stinkyscape-113", // A Very Dandy Lad (S #113)
    "stinkyscape-094", // Tmack Trav (S #94)
    "stinkyscape-123", // Gokugains (S #123)
    "stinkyscape-114", // Arctic Citadel (S #114)
];

/// Star-Jumper Core: S #162 Faction: Star-Jumper Core and the five cards it adds.
const STAR_JUMPER_CORE: [&str; 6] = [
    "stinkyscape-162", // Faction: Star-Jumper Core (S #162)
    "stinkyscape-075", // Badger (S #75)
    "stinkyscape-076", // Honey Badger (S #76)
    "stinkyscape-078", // Ozark (S #78)
    "stinkyscape-079", // Orca (S #79)
    "stinkyscape-080", // Galaxy (S #80)
];

/// Each mode's word and its faction, the Faction card first.
const FACTIONS: [(&str, &[&str]); 5] = [
    ("Vanguard Coalition", &VANGUARD_COALITION),
    ("ETHICS", &ETHICS),
    ("Golden Accord", &GOLDEN_ACCORD),
    ("Pegasus", &PEGASUS),
    ("Star-jumper Core", &STAR_JUMPER_CORE),
];

/// "Your hand and deck": its caster's library, then hand, in Genn's Greed's order (R135, R2521004).
const ZONES: [OffFieldZone; 2] = [OffFieldZone::Library, OffFieldZone::Hand];

fn swear_loyalities(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: FACTIONS.iter().map(|(mode, _)| mode.to_string()).collect(),
        }],
        cry: Some(hook(move |ctx| {
            let chosen = chosen_options(ctx);
            let picked = |(mode, _): &&(&str, &[&str])| chosen.first().is_some_and(|option| option == mode);
            let Some((_, sworn)) = FACTIONS.iter().find(picked) else {
                return vec![];
            };
            let mut effects = vec![add_to_hand(json_as(json!({ "defId": sworn[0], "radiant": radiant })))];
            for zone in ZONES {
                for card in zone_cards(&*ctx.state, ctx.controller, zone) {
                    let def_id = card.def_id.as_str();
                    let factioned = FACTIONS.iter().any(|(_, faction)| faction.contains(&def_id));
                    if factioned && !sworn.contains(&def_id) {
                        let target = json!({ "of": "instance", "instanceId": card.id });
                        effects.push(exile(json_as(json!({ "target": target }))));
                    }
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: swear_loyalities(false),
        radiant: swear_loyalities(true),
    }
}

/// R767000: each mode's word and the Faction card it adds, the only cards v0.5.11's script exiled.
const BUNDLES_V0_5_11: [(&str, &str); 5] = [
    ("Vanguard Coalition", "stinkyscape-095"),
    ("ETHICS", "stinkyscape-159"),
    ("Golden Accord", "stinkyscape-160"),
    ("Pegasus", "stinkyscape-161"),
    ("Star-jumper Core", "stinkyscape-162"),
];

/// R767000: "Wherever it is" as v0.5.11 read it: its caster's piles, in Genn's Greed's order (R135).
const ZONES_V0_5_11: [OffFieldZone; 3] = [OffFieldZone::Library, OffFieldZone::Hand, OffFieldZone::Graveyard];

/// R767000: Swear Loyalities as v0.5.11 ran it: the chosen bundle added, then every card its caster
/// owns of the four others exiled from its library, hand and graveyard (R1880001).
fn swear_loyalities_v0_5_11(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: BUNDLES_V0_5_11.iter().map(|(mode, _)| mode.to_string()).collect(),
        }],
        cry: Some(hook(move |ctx| {
            let chosen = chosen_options(ctx);
            let picked = |(mode, _): &&(&str, &str)| chosen.first().is_some_and(|option| option == mode);
            let Some((_, kept)) = BUNDLES_V0_5_11.iter().find(picked) else {
                return vec![];
            };
            let mut effects = vec![add_to_hand(json_as(json!({ "defId": kept, "radiant": radiant })))];
            for zone in ZONES_V0_5_11 {
                for card in zone_cards(&*ctx.state, ctx.controller, zone) {
                    let bundle = BUNDLES_V0_5_11.iter().any(|(_, def_id)| card.def_id == *def_id);
                    if card.def_id != *kept && bundle {
                        let target = json!({ "of": "instance", "instanceId": card.id });
                        effects.push(exile(json_as(json!({ "target": target }))));
                    }
                }
            }
            effects
        })),
        ..Script::default()
    }
}

/// R767000: through v0.5.11 it exiled the other four Faction cards alone, its caster's graveyard's
/// too; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    CardScripts {
        base: swear_loyalities_v0_5_11(false),
        radiant: swear_loyalities_v0_5_11(true),
    }
}

// S #163 Swear Loyalities — SPEC §8.10 row S 163, BUILD M12 row S 163, R2521004: the chosen faction's
// Faction card is added to the hand, then every card in its caster's hand and library of another faction
// and not of the chosen one is exiled; radiant the Faction card Radiant.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SWEAR: &str = "stinkyscape-163";
    /// Core #5 Stockpile: a card of no faction.
    const FILLER: &str = "core-005";
    const MODES: [&str; 5] = [
        "Vanguard Coalition",
        "ETHICS",
        "Golden Accord",
        "Pegasus",
        "Star-jumper Core",
    ];

    /// The Faction cards sworn to below.
    const VANGUARD: &str = "stinkyscape-095";
    const ETHICS: &str = "stinkyscape-159";
    const GOLDEN_ACCORD: &str = "stinkyscape-160";
    const PEGASUS: &str = "stinkyscape-161";
    /// S #115 Scornboi: Vanguard Coalition's alone.
    const SCORNBOI: &str = "stinkyscape-115";
    /// S #6 Tylannt: ETHICS' and Vanguard Coalition's.
    const TYLANNT: &str = "stinkyscape-006";
    /// S #109 xxFreshJack: ETHICS' alone.
    const FRESH_JACK: &str = "stinkyscape-109";
    /// S #72 Paragon Mk1 and S #69 Falcon: Golden Accord's.
    const PARAGON_MK1: &str = "stinkyscape-072";
    const FALCON: &str = "stinkyscape-069";

    /// Swear Loyalities on `radiant`'s face in P1's hand, before `hand`; `library` is P1's library.
    fn board(radiant: bool, hand: Value, library: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": SWEAR, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": held, "library": library },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// R2521004: sworn to Vanguard Coalition, the other factions' cards leave the hand and the library;
    /// Tylannt, ETHICS' too but of the faction sworn to, stays.
    fn swears_to_vanguard_coalition(radiant: bool) {
        let mut s = board(
            radiant,
            json!([FILLER, FRESH_JACK, SCORNBOI, TYLANNT]),
            json!([PARAGON_MK1, ETHICS, FILLER, SCORNBOI, TYLANNT, VANGUARD]),
        );

        s.play(SWEAR, json!({ "modes": ["Vanguard Coalition"] }));

        let hand = s.pile(P1, "hand");
        assert_eq!(def_ids(&hand), [FILLER, SCORNBOI, TYLANNT, VANGUARD]);
        assert_eq!(hand.last().map(|card| card.radiant), Some(radiant));
        assert!(hand[..3].iter().all(|card| !card.radiant));
        assert_eq!(def_ids(&s.pile(P1, "library")), [FILLER, SCORNBOI, TYLANNT, VANGUARD]);
        // R135's order: the library's, then the hand's.
        assert_eq!(def_ids(&s.pile(P1, "exile")), [PARAGON_MK1, ETHICS, FRESH_JACK]);
    }

    /// R2521004: sworn to Golden Accord, Tylannt is of neither faction kept and leaves the library and
    /// the hand.
    fn swears_to_golden_accord(radiant: bool) {
        let mut s = board(
            radiant,
            json!([TYLANNT, FILLER, PARAGON_MK1]),
            json!([TYLANNT, FALCON, SCORNBOI, FRESH_JACK]),
        );

        s.play(SWEAR, json!({ "modes": ["Golden Accord"] }));

        let hand = s.pile(P1, "hand");
        assert_eq!(def_ids(&hand), [FILLER, PARAGON_MK1, GOLDEN_ACCORD]);
        assert_eq!(hand.last().map(|card| card.radiant), Some(radiant));
        assert_eq!(def_ids(&s.pile(P1, "library")), [FALCON]);
        assert_eq!(def_ids(&s.pile(P1, "exile")), [TYLANNT, SCORNBOI, FRESH_JACK, TYLANNT]);
    }

    /// R2521004: only its caster's hand and library are read (the opponent's untouched, R1880001).
    fn leaves_the_field_the_graveyard_and_the_opponent(radiant: bool) {
        crate::register_all();
        let others = json!([SCORNBOI, PARAGON_MK1, ETHICS]);
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": SWEAR, "radiant": radiant }, FILLER],
                "field": [PARAGON_MK1],
                "graveyard": [FRESH_JACK],
            },
            "p2": { "hand": others, "library": others, "graveyard": others },
        }));

        s.play(SWEAR, json!({ "modes": ["Pegasus"] }));

        assert_eq!(def_ids(&s.pile(P1, "hand")), [FILLER, PEGASUS]);
        assert_eq!(s.unit_at(P1, 1).def_id, PARAGON_MK1);
        assert!(def_ids(&s.pile(P1, "graveyard")).contains(&FRESH_JACK.to_string()));
        assert!(s.pile(P1, "exile").is_empty());
        for zone in ["hand", "library", "graveyard"] {
            assert_eq!(def_ids(&s.pile(P2, zone)), [SCORNBOI, PARAGON_MK1, ETHICS], "{zone}");
        }
        assert!(s.pile(P2, "exile").is_empty());
    }

    mod base {
        use super::*;

        #[test]
        fn r1880002_offers_all_five_factions_to_its_caster() {
            let s = board(false, json!([FILLER]), filler(3));
            let held = s.card(SWEAR).id.clone();
            let offered: Vec<Vec<String>> = legal_actions(s.state(), P1)
                .iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, modes, .. } if *instance_id == held => modes.clone(),
                    _ => None,
                })
                .collect();
            let words: Vec<Vec<String>> = MODES.iter().map(|mode| vec![mode.to_string()]).collect();
            assert_eq!(offered, words);
        }

        #[test]
        fn r2521004_sworn_to_vanguard_coalition_other_factions_leave_the_hand_and_library_tylannt_stays() {
            swears_to_vanguard_coalition(false);
        }

        #[test]
        fn r2521004_sworn_to_golden_accord_tylannt_is_exiled() {
            swears_to_golden_accord(false);
        }

        #[test]
        fn r1880001_r2521004_the_field_the_graveyard_and_the_opponents_cards_stay() {
            leaves_the_field_the_graveyard_and_the_opponent(false);
        }

        /// R767000: the script kept for the replays of v0.5.11 and before exiled the other four Faction
        /// cards alone, from its caster's library, hand and graveyard in that order.
        #[test]
        fn r767000_the_script_kept_for_v0_5_11_exiles_the_other_faction_cards_alone_the_graveyard_too() {
            crate::under_kept(SWEAR, super::super::script_v0_5_11(), || {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [SWEAR, FILLER, SCORNBOI, VANGUARD],
                        "library": [ETHICS, FRESH_JACK],
                        "graveyard": [PEGASUS],
                    },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(SWEAR, json!({ "modes": ["Golden Accord"] }));

                assert_eq!(def_ids(&s.pile(P1, "hand")), [FILLER, SCORNBOI, GOLDEN_ACCORD]);
                assert_eq!(def_ids(&s.pile(P1, "library")), [FRESH_JACK]);
                assert_eq!(def_ids(&s.pile(P1, "exile")), [ETHICS, VANGUARD, PEGASUS]);
            });
        }

        /// §2.4: a full hand burns the Faction card, which stays in the graveyard, and the library is
        /// still sworn (R2521004).
        #[test]
        fn r4_r2521004_a_full_hand_burns_the_faction_card_and_the_library_is_still_exiled() {
            let mut s = board(false, filler(HAND_CAP as usize), json!([SCORNBOI, FRESH_JACK, TYLANNT]));

            s.play(SWEAR, json!({ "modes": ["ETHICS"] }));

            assert!(!def_ids(&s.pile(P1, "hand")).contains(&ETHICS.to_string()));
            assert!(def_ids(&s.pile(P1, "graveyard")).contains(&ETHICS.to_string()));
            assert_eq!(def_ids(&s.pile(P1, "library")), [FRESH_JACK, TYLANNT]);
            assert_eq!(def_ids(&s.pile(P1, "exile")), [SCORNBOI]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2521004_sworn_to_vanguard_coalition_the_radiant_faction_card_is_added_and_tylannt_stays() {
            swears_to_vanguard_coalition(true);
        }

        #[test]
        fn r2521004_sworn_to_golden_accord_the_radiant_faction_card_is_added_and_tylannt_is_exiled() {
            swears_to_golden_accord(true);
        }

        #[test]
        fn r2521004_the_field_the_graveyard_and_the_opponents_cards_stay() {
            leaves_the_field_the_graveyard_and_the_opponent(true);
        }
    }
}
