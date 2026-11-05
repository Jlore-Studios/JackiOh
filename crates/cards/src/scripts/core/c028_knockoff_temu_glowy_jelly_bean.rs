//! #28 Knockoff Temu Glowy Jelly Bean (SPEC §8.2): "2 random cards among your library, hand and
//! field become Radiant", radiant "4" — a Radiant cell that changes only a number changes only that
//! number (§8 Conventions).
//! Pooled uniform random draw across library, hand, and field (R60, §3.2, R13 top of Stack only; §5.2, R74).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-028";

/// The union the pick is uniform over, in the pool's canonical order (hand, then library top down,
/// then lane order) so the draw depends only on (seed, cursor) and replays exactly (§9.3).
const ZONES: [&str; 3] = ["hand", "library", "field"];

/// The count is the declared number `cards` (R386): 2, 4 on the Radiant face.
fn knockoff_temu() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            vec![set_radiant_random(json_as(json!({ "zones": ZONES, "count": param(&*ctx, "cards") })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = knockoff_temu();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #28 Knockoff Temu Glowy Jelly Bean (SPEC §8.2, BUILD M4-T4 row 28) (R60, R22).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SEED: &str = "core-028";

    /// Exactly the union #28 draws from: hand, library and field — never the graveyard.
    fn pool_cards(state: &GameState) -> Vec<CardInstance> {
        let side = &state.players.p1;
        let mut cards: Vec<CardInstance> = Vec::new();
        cards.extend(side.hand.iter().cloned());
        cards.extend(side.library.iter().cloned());
        cards.extend(side.units.iter().flatten().filter_map(|pile| pile.first().cloned()));
        cards.extend(side.backrow.iter().flatten().cloned());
        cards
    }

    fn radiant_in_pool(state: &GameState) -> Vec<String> {
        pool_cards(state)
            .into_iter()
            .filter(|card| card.radiant)
            .map(|card| card.id)
            .collect()
    }

    /// `side` is `{ hand, library?, field? }` as JSON; p1 gets 2 mana.
    fn knockoff(mut side: Value) -> Scenario {
        side["mana"] = json!(2);
        scenario(json!({ "seed": SEED, "p1": side }))
    }


    mod n28_knockoff_temu_glowy_jelly_bean_base {
        use super::*;

        #[test]
        fn r60_flags_2_different_non_radiant_cards_drawn_from_library_hand_and_field_as_one_pool() {
            crate::register_all();
            let mut s = knockoff(json!({
                "hand": ["core-028", "core-005", "core-016"],
                "library": ["core-010", "core-013"],
                "field": ["core-025"],
            }));

            s.play("core-028", json!({}));

            let flagged = radiant_in_pool(s.state());
            assert_eq!(flagged.len(), 2);
            assert_eq!(flagged.iter().collect::<IndexSet<_>>().len(), 2);
            assert!(s.state().pending.is_none());
            s.expect_in_zone("core-028", "graveyard");
        }

        #[test]
        fn r22_a_field_unit_swaps_its_base_stats_in_place_and_keeps_the_damage_it_has_taken() {
            crate::register_all();
            // The pool holds exactly one card — the unit on the field: #28 itself is in the `resolving`
            // zone while its script runs and the library is empty, so the pick is not a coin flip.
            let mut s = knockoff(json!({ "hand": ["core-028"], "library": [], "field": [{ "def": "core-013", "damage": 2 }] }));
            let unit_id = s.unit(P1, 1).map(|card| card.id.clone()).unwrap_or_default();
            s.expect_stats(&unit_id, json!({ "attack": 6, "maxHealth": 8, "health": 6 }));

            s.play("core-028", json!({}));

            // #13 Jlockeed Shredder-10 is 6/8 → 12/16. R22: the base layer swaps, the 2 damage stays (so
            // health is 14, not a fresh 16), and setting a flag is no entry to the field, so no Cry re-fires.
            assert!(s.card(&unit_id).radiant);
            s.expect_stats(&unit_id, json!({ "attack": 12, "maxHealth": 16, "health": 14 }));
            assert_eq!(s.card(&unit_id).damage, 2);
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Summoned { .. }))
                    .count(),
                0
            );
        }

        #[test]
        fn r60_does_nothing_at_all_when_every_card_in_the_union_is_already_radiant() {
            crate::register_all();
            let mut s = knockoff(json!({
                "hand": ["core-028", { "def": "core-005", "radiant": true }],
                "library": [{ "def": "core-010", "radiant": true }],
                "field": [{ "def": "core-025", "radiant": true }],
            }));
            let mut stays: Vec<String> = pool_cards(s.state())
                .into_iter()
                .filter(|card| card.def_id != "core-028")
                .map(|card| card.id)
                .collect();
            stays.sort();

            s.play("core-028", json!({}));

            let mut flagged = radiant_in_pool(s.state());
            flagged.sort();
            assert_eq!(flagged, stays);
            // Nothing changed. R177: the picks R60 could not make are cued on the hidden cards of the union
            // (the hand's, then the library's), never on the public 7/7, so the other seat's stream does not
            // count how many hidden cards were Radiant already.
            let cued: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::RadiantSet { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            let p1 = &s.state().players.p1;
            let hidden: Vec<String> = p1.hand.iter().chain(p1.library.iter()).map(|card| card.id.clone()).collect();
            assert!(!cued.is_empty());
            for id in &cued {
                assert!(hidden.contains(id));
            }
        }
    }

    mod n28_knockoff_temu_glowy_jelly_bean_radiant {
        use super::*;

        #[test]
        fn flags_4_different_cards_across_the_same_three_zones() {
            crate::register_all();
            let mut s = knockoff(json!({
                "hand": [{ "def": "core-028", "radiant": true }, "core-005", "core-016", "core-010"],
                "library": ["core-013", "core-043", "core-047"],
                "field": ["core-025"],
            }));

            s.play("core-028", json!({}));

            let flagged = radiant_in_pool(s.state());
            assert_eq!(flagged.len(), 4);
            assert_eq!(flagged.iter().collect::<IndexSet<_>>().len(), 4);
        }
    }

    #[test]
    fn r386_an_upgrade_flags_3_cards_and_a_degrade_1() {
        for (upgrade, cards) in [(true, 3), (false, 1)] {
            crate::register_all();
            let mut s = knockoff(json!({
                "hand": ["core-028", "core-005", "core-016", "core-010"],
                "library": ["core-013", "core-043", "core-047"],
                "field": ["core-025"],
            }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "core-028", "cards")
            } else {
                crate::degrade_number(&mut s, "core-028", "cards")
            };
            assert_eq!(moved, cards);
            s.play("core-028", json!({}));
            assert_eq!(radiant_in_pool(s.state()).len(), cards as usize);
        }
    }
}
