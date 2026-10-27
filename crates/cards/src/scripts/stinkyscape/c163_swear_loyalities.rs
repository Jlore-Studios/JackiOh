//! S #163 Swear Loyalities (SPEC §8.10 row S 163; §2.4, §5.2, §6.3, R81): (1) Spell, Rare.
//!   Base:    "Choose a Vanguard Coalition, ETHICS, Golden Accord, Pegasus or Star-jumper Core and add
//!            it to your hand. Exile every card you own of the four you didn't choose, wherever it is."
//!   Radiant: "Choose a Radiant Vanguard Coalition, ETHICS, Golden Accord, Pegasus or Star-jumper Core
//!            and add it to your hand. Exile every card you own of the four you didn't choose, wherever
//!            it is."
//!
//! Engine: five modes, one per bundle (S #95, #159 to #162), chosen with the play (R81, S #121 System
//! Schematic's pattern), all five offered (R1880002); the Cry is one `add_to_hand` of the chosen
//! bundle, a full hand burning it (§2.4), Radiant on the Radiant face (§5.2). Then one `exile` per
//! card its caster owns of the four others, in its library, hand and graveyard, walked in that order
//! (#94 Genn's Greed's zones and order, R135): "wherever it is" is every zone its caster's own cards
//! can be in, a Spell never standing on the field, and the opponent's copies stay (R1880001, ST13 E9).
//! "Golden" is Golden Accord (ST8). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-163";

/// Each mode's word and the bundle it adds.
const BUNDLES: [(&str, &str); 5] = [
    ("Vanguard Coalition", "stinkyscape-095"),
    ("ETHICS", "stinkyscape-159"),
    ("Golden Accord", "stinkyscape-160"),
    ("Pegasus", "stinkyscape-161"),
    ("Star-jumper Core", "stinkyscape-162"),
];

/// "Wherever it is": its caster's piles, in Genn's Greed's order (R135).
const ZONES: [OffFieldZone; 3] = [OffFieldZone::Library, OffFieldZone::Hand, OffFieldZone::Graveyard];

fn swear_loyalities(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: BUNDLES.iter().map(|(mode, _)| mode.to_string()).collect(),
        }],
        cry: Some(hook(move |ctx| {
            let chosen = chosen_options(ctx);
            let picked = |(mode, _): &&(&str, &str)| chosen.first().is_some_and(|option| option == mode);
            let Some((_, kept)) = BUNDLES.iter().find(picked) else {
                return vec![];
            };
            let mut effects = vec![add_to_hand(json_as(json!({ "defId": kept, "radiant": radiant })))];
            for zone in ZONES {
                for card in zone_cards(&*ctx.state, ctx.controller, zone) {
                    if card.def_id != *kept && BUNDLES.iter().any(|(_, def_id)| card.def_id == *def_id) {
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

// S #163 Swear Loyalities — SPEC §8.10 row S 163, BUILD M12 row S 163: "offers all five bundles to its
// caster alone; the chosen one is added to the hand; every Vanguard Coalition, ETHICS, Golden Accord,
// Pegasus or Star-jumper Core its caster owns that was not chosen is exiled from hand, deck and
// graveyard, the opponent's untouched; radiant the chosen one Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{def_ids, filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SWEAR: &str = "stinkyscape-163";
    const FILLER: &str = "core-005";

    const BUNDLES: [(&str, &str); 5] = [
        ("Vanguard Coalition", "stinkyscape-095"),
        ("ETHICS", "stinkyscape-159"),
        ("Golden Accord", "stinkyscape-160"),
        ("Pegasus", "stinkyscape-161"),
        ("Star-jumper Core", "stinkyscape-162"),
    ];

    fn bundle_ids() -> Vec<Value> {
        BUNDLES.iter().map(|(_, id)| json!(id)).collect()
    }

    /// Swear Loyalities in P1's hand beside `hand`; a copy of every bundle in each side's library and
    /// graveyard, and in P2's hand.
    fn board(radiant: bool, hand: Value) -> Scenario {
        crate::register_all();
        let mut held = vec![json!({ "def": SWEAR, "radiant": radiant })];
        held.extend(hand.as_array().cloned().unwrap_or_default());
        let mut library = bundle_ids();
        library.extend(filler(3).as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": held, "library": library, "graveyard": bundle_ids() },
            "p2": { "hand": bundle_ids(), "library": library, "graveyard": bundle_ids() },
        }))
    }

    fn bundles_in(cards: &[CardInstance]) -> Vec<String> {
        def_ids(cards).into_iter().filter(|id| BUNDLES.iter().any(|(_, bundle)| id == bundle)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1880002_offers_all_five_bundles_to_its_caster() {
            let s = board(false, json!([FILLER]));
            let held = s.card(SWEAR).id.clone();
            let offered: Vec<Vec<String>> = legal_actions(s.state(), P1)
                .iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, modes, .. } if *instance_id == held => modes.clone(),
                    _ => None,
                })
                .collect();
            let words: Vec<Vec<String>> = BUNDLES.iter().map(|(mode, _)| vec![mode.to_string()]).collect();
            assert_eq!(offered, words);
        }

        #[test]
        fn r1880001_each_mode_adds_its_bundle_and_exiles_its_casters_other_four_from_every_pile() {
            for (mode, kept) in BUNDLES {
                let others: Vec<&str> = BUNDLES.iter().map(|(_, id)| *id).filter(|id| *id != kept).collect();
                let mut hand = vec![json!(FILLER)];
                hand.extend(others.iter().map(|id| json!(id)));
                let mut s = board(false, json!(hand));

                s.play(SWEAR, json!({ "modes": [mode] }));

                assert_eq!(bundles_in(&s.pile(P1, "hand")), [kept], "{mode}");
                assert!(s.pile(P1, "hand").iter().all(|card| !card.radiant), "{mode}");
                assert_eq!(bundles_in(&s.pile(P1, "library")), [kept], "{mode}");
                assert_eq!(bundles_in(&s.pile(P1, "graveyard")), [kept], "{mode}");
                // R135's order: the library's four, then the hand's, then the graveyard's.
                let mut exiled = others.clone();
                exiled.extend(others.iter());
                exiled.extend(others.iter());
                assert_eq!(bundles_in(&s.pile(P1, "exile")), exiled, "{mode}");
            }
        }

        #[test]
        fn r1880001_the_opponents_copies_stay() {
            let mut s = board(false, json!([FILLER]));

            s.play(SWEAR, json!({ "modes": ["Pegasus"] }));

            for zone in ["hand", "library", "graveyard"] {
                assert_eq!(bundles_in(&s.pile(P2, zone)).len(), BUNDLES.len(), "{zone}");
            }
            assert!(s.pile(P2, "exile").is_empty());
        }

        #[test]
        fn r4_r1880001_a_full_hand_burns_the_chosen_one_and_the_others_are_still_exiled() {
            let mut s = board(false, filler(HAND_CAP as usize));

            s.play(SWEAR, json!({ "modes": ["ETHICS"] }));

            assert!(bundles_in(&s.pile(P1, "hand")).is_empty());
            assert_eq!(bundles_in(&s.pile(P1, "graveyard")), ["stinkyscape-159", "stinkyscape-159"]);
            assert_eq!(bundles_in(&s.pile(P1, "library")), ["stinkyscape-159"]);
            assert_eq!(bundles_in(&s.pile(P1, "exile")).len(), 8);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_chosen_one_is_radiant_and_the_others_are_still_exiled() {
            let mut s = board(true, json!([FILLER, "stinkyscape-095"]));

            s.play(SWEAR, json!({ "modes": ["Star-jumper Core"] }));

            let made: Vec<CardInstance> =
                s.pile(P1, "hand").into_iter().filter(|card| card.def_id == "stinkyscape-162").collect();
            assert_eq!(made.len(), 1);
            assert!(made[0].radiant);
            assert!(bundles_in(&s.pile(P1, "hand")).iter().all(|id| id == "stinkyscape-162"));
            assert_eq!(bundles_in(&s.pile(P1, "exile")).len(), 9);
        }
    }
}
