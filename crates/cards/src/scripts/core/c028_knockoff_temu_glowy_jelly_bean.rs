//! #28 Knockoff Temu Glowy Jelly Bean (SPEC §8.2): "2 random cards among your library, hand and
//! field become Radiant", radiant "5" — a Radiant cell that changes only a number changes only that
//! number (§8 Conventions).
//!
//! ONE `setRadiantRandom` call, not N calls and not one call per zone. The pick has to be uniform
//! over the union of the three zones and has to produce `count` DIFFERENT cards (R60), and both only
//! hold if the union is pooled once and drawn from once: three calls of one card each would weight
//! small zones and could repeat, and N calls of one card each would re-pool between picks.
//! `effects/radiant.ts` pools the non-Radiant cards of the named zones in a fixed order (hand order,
//! library top down, then lane order), shuffles that pool with the match rng and takes the first
//! `count` — so it is uniform, the cards are all different, it takes all of them when fewer exist,
//! and it does nothing at all when every card is already Radiant (R60).
//!
//! "Field" is the field as §3.2 and R13 define it, which `setRadiantRandom` already honours: both
//! rows, and only the top of a Stack pile, since cards under a Stack are not on the field.
//!
//! A field card converts in place (§5.2, R22): setting the flag is not an entry to the field, so the
//! base-stat layer swaps at once through `faceOf`/`unitView` while damage taken, buffs and granted
//! keywords stay and the Cry does not re-fire. A 4/5 that has taken 2 becomes an 8/10 that has taken
//! 2, i.e. 8 health. None of that is this card's code — it is what "set a flag" buys.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-028";

/// The union the pick is uniform over. Listed in `effects/radiant.ts`'s canonical pool order (hand,
/// then library top down, then lane order) so the draw depends only on (seed, cursor) and replays
/// exactly (§9.3); §8.2 writes the same three zones as "your library, hand and field".
const ZONES: [&str; 3] = ["hand", "library", "field"];

fn knockoff_temu(count: i32) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![set_radiant_random(json_as(json!({ "zones": ZONES, "count": count })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: knockoff_temu(2),
        radiant: knockoff_temu(5),
    }
}

// #28 Knockoff Temu Glowy Jelly Bean (SPEC §8.2, BUILD M4-T4 row 28): "2 different non-Radiant
// cards across library+hand+field (R60); a field unit swaps base stats in place keeping damage
// (R22); radiant 5".
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

    /// `side` is the TS `{ hand, library?, field? }`, given as the JSON it would be; p1 gets 2 mana.
    fn knockoff(mut side: Value) -> Scenario {
        side["mana"] = json!(2);
        scenario(json!({ "seed": SEED, "p1": side }))
    }

    /// §3.2: put a fresh `core-025` on top of p1's unit pile in lane 1, burying what stood there, and
    /// hand back the buried card's id and the new top's (`newInstance` + `pile.unshift`).
    fn stack_on_lane_1(s: &mut Scenario) -> Option<(String, CardInstance)> {
        let dormant_id = s.state().players.p1.units[0].as_ref()?.first().map(|card| card.id.clone()).unwrap_or_default();
        let top = new_instance(
            s.state_mut(),
            "core-025",
            P1,
            Zone::Field {
                player: P1,
                row: Row::Units,
                lane: 1,
            },
        );
        s.state_mut().players.p1.units[0].as_mut()?.insert(0, top.clone());
        Some((dormant_id, top))
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
        fn r60_draws_once_over_the_union_so_the_same_seed_flags_the_same_two_cards_s9_3() {
            crate::register_all();
            let build = || {
                let mut s = knockoff(json!({
                    "hand": ["core-028", "core-005", "core-016"],
                    "library": ["core-010", "core-013"],
                    "field": ["core-025"],
                }));
                s.play("core-028", json!({}));
                s
            };

            assert_eq!(radiant_in_pool(build().state()), radiant_in_pool(build().state()));
        }

        #[test]
        fn r22_a_field_unit_swaps_its_base_stats_in_place_and_keeps_the_damage_it_has_taken() {
            crate::register_all();
            // The pool holds exactly one card — the unit on the field: #28 itself is in the `resolving`
            // zone while its script runs and the library is empty, so the pick is not a coin flip.
            let mut s = knockoff(json!({ "hand": ["core-028"], "library": [], "field": [{ "def": "core-013", "damage": 2 }] }));
            let unit_id = s.unit(P1, 1).map(|card| card.id.clone()).unwrap_or_default();
            s.expect_stats(&unit_id, json!({ "attack": 8, "maxHealth": 10, "health": 8 }));

            s.play("core-028", json!({}));

            // #13 Jlockeed Shredder-10 is 8/10 → 16/20. R22: the base layer swaps, the 2 damage stays (so
            // health is 18, not a fresh 20), and setting a flag is no entry to the field, so no Cry re-fires.
            assert!(s.card(&unit_id).radiant);
            s.expect_stats(&unit_id, json!({ "attack": 16, "maxHealth": 20, "health": 18 }));
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
        fn r13_never_picks_a_card_under_a_stack_pile_which_is_not_on_the_field() {
            crate::register_all();
            let mut s = knockoff(json!({ "hand": ["core-028"], "library": [], "field": ["core-025"] }));
            assert!(s.state().players.p1.units[0].is_some());
            // §3.2: a Stack card enters an occupied unit zone and becomes the top of the pile; the card
            // beneath it is dormant, and R13 keeps a dormant card off the field and out of every pool.
            let Some((dormant_id, top)) = stack_on_lane_1(&mut s) else {
                return;
            };

            s.play("core-028", json!({}));

            assert!(s.card(&top.id).radiant);
            assert!(!s.card(&dormant_id).radiant);
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
        fn flags_5_different_cards_across_the_same_three_zones() {
            crate::register_all();
            let mut s = knockoff(json!({
                "hand": [{ "def": "core-028", "radiant": true }, "core-005", "core-016", "core-010"],
                "library": ["core-013", "core-043", "core-047"],
                "field": ["core-025"],
            }));

            s.play("core-028", json!({}));

            let flagged = radiant_in_pool(s.state());
            assert_eq!(flagged.len(), 5);
            assert_eq!(flagged.iter().collect::<IndexSet<_>>().len(), 5);
        }

        #[test]
        fn r60_takes_all_of_them_when_the_union_holds_fewer_than_5_non_radiant_cards() {
            crate::register_all();
            let mut s = knockoff(json!({
                "hand": [{ "def": "core-028", "radiant": true }, "core-005"],
                "library": ["core-013"],
                "field": ["core-025"],
            }));

            s.play("core-028", json!({}));

            // Three cards are left in the union once #28 has gone to the graveyard, and all three convert.
            assert_eq!(radiant_in_pool(s.state()).len(), 3);
        }
    }
}
