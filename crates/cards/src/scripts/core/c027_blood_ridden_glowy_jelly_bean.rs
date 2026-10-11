//! #27 Blood Ridden Glowy Jelly Bean (SPEC §8.2): "Cast on draw: a random card in your hand becomes
//! Radiant; you lose 5 health", radiant "2 random cards".
//!
//! The radiant cell restates only the number of random cards, so every clause it does not restate is
//! kept (§8 Conventions). `staticFlags` are read through `scriptOf`, which returns the radiant script
//! once the instance is Radiant, so the radiant face carries `castOnDraw` too or a Radiant copy would
//! go to hand uncast. `drawOne` casts the card the moment it is drawn, even with a full hand, and
//! repeats the draw up to CAST_ON_DRAW_CHAIN_CAP (R58); the cast is free and feeds Combo (R40, R70).
//!
//! R60's pick: `setRadiantRandom` pools only non-Radiant cards, takes `count` different ones (all when
//! fewer exist, none when none do); the resolving card never picks itself. "You lose 5 health" is
//! R18's `loseHealth`, not damage: it bypasses Armor, #73 Going Long's cap and Fed Fauci's tokens.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-027";

/// §8.2: the blood price is the same on both faces; only the number of cards changes. Both are
/// declared numbers (R386): `loss`, 5, less being better, and `cards`, 2 on the Radiant face and tuned
/// there alone (R749: the base face's "a random card" prints no number and is always 1).
fn blood_ridden() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![
                set_radiant_random(json_as(json!({ "zones": "hand", "count": param(&*ctx, "cards") }))),
                lose_health(json_as(json!({ "player": "self", "amount": param(&*ctx, "loss") }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = blood_ridden();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #27 Blood Ridden Glowy Jelly Bean (SPEC §8.2, BUILD M4-T4 row 27): "Cast on draw; a random
// non-Radiant hand card becomes Radiant (R60); 5 health lost ignoring Going Long (R18); radiant
// 2 cards".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const SEED: &str = "core-027";

    /// The hand cards the random pick reached. R60 narrows the pool to the non-Radiant ones.
    fn radiant_hand(state: &GameState) -> Vec<String> {
        state
            .players
            .p1
            .hand
            .iter()
            .filter(|card| card.radiant)
            .map(|card| card.id.clone())
            .collect()
    }

    /// A game whose next draw is the Blood Ridden Glowy Jelly Bean. `library[0]` is the top, so one
    /// `startTurn()` draws exactly this card, which casts itself (§2.4). #5 Stockpile sits under it as
    /// the card R58's repeated draw brings up.
    fn drawing(hand: Value, radiant: bool, armor: Option<i32>) -> Scenario {
        let mut p1 = json!({
            "hand": hand,
            "library": [{ "def": "core-027", "radiant": radiant }, "core-005"],
            "health": 30,
        });
        if let Some(armor) = armor {
            p1["armor"] = json!(armor);
        }
        scenario(json!({ "seed": SEED, "p1": p1 }))
    }

    use crate::matches_object;

    mod n27_blood_ridden_glowy_jelly_bean_base {
        use super::*;

        #[test]
        fn r40_r70_casts_itself_on_the_draw_free_counted_as_a_play_and_never_enters_the_hand() {
            crate::register_all();
            let mut s = drawing(json!(["core-005", "core-016"]), false, None);

            s.start_turn();

            s.expect_in_zone("core-027", "graveyard");
            assert!(!s.state().players.p1.hand.iter().any(|card| card.def_id == "core-027"));
            // R70: a cast is free and counts as a play — which is what feeds Combo (R40).
            let played: Vec<&GameEvent> = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CardPlayed { .. }))
                .collect();
            assert_eq!(played.len(), 1);
            assert!(matches_object(
                &serde_json::to_value(played[0]).unwrap(),
                &json!({ "defId": "core-027", "costPaid": 0 })
            ));
            assert_eq!(s.state().players.p1.turn_log.cards_played, 1);
            // R58: the draw repeats after the cast, so the card under it reaches the hand.
            assert_eq!(
                s.state().players.p1.hand.iter().filter(|card| card.def_id == "core-005").count(),
                2
            );
        }

        #[test]
        fn r60_flags_exactly_one_random_non_radiant_hand_card_the_same_one_on_the_same_seed() {
            crate::register_all();
            let mut first = drawing(json!(["core-005", "core-016", "core-010"]), false, None);
            first.start_turn();
            let flagged = radiant_hand(first.state());
            assert_eq!(flagged.len(), 1);

            // §9.3: (seed, cursor) reproduces the draw, so the same setup flags the same card.
            let mut second = drawing(json!(["core-005", "core-016", "core-010"]), false, None);
            second.start_turn();
            assert_eq!(radiant_hand(second.state()), flagged);
        }

        #[test]
        fn r60_never_picks_a_card_that_is_already_radiant_and_does_nothing_when_none_are_left() {
            crate::register_all();
            let mut s = drawing(
                json!([{ "def": "core-005", "radiant": true }, { "def": "core-016", "radiant": true }]),
                false,
                None,
            );
            let already: Vec<String> = s.state().players.p1.hand.iter().map(|card| card.id.clone()).collect();

            s.start_turn();

            // The pool was empty at resolution, so nothing was flagged; the card R58's repeated draw brings
            // up arrives afterwards and is untouched. R177: the hidden hand is still cued once, on a card that
            // was Radiant already, so p2 cannot tell this hand from one the pick changed.
            for id in &already {
                assert!(s.card(id).radiant);
            }
            let cues: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::RadiantSet { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(cues, vec![already[0].clone()]);
            let mut flagged = radiant_hand(s.state());
            flagged.sort();
            let mut wanted = already.clone();
            wanted.sort();
            assert_eq!(flagged, wanted);
        }

        #[test]
        fn r18_loses_5_health_not_damage_so_armor_pays_nothing_and_is_left_untouched() {
            crate::register_all();
            let mut s = drawing(json!(["core-005"]), false, Some(2));

            s.start_turn();

            s.expect_health(P1, 25);
            assert_eq!(s.state().players.p1.hero.armor, 2);
            let lost: Vec<Value> = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::HealthLost { .. }))
                .map(|event| serde_json::to_value(event).unwrap())
                .collect();
            assert!(matches_object(&Value::Array(lost), &json!([{ "player": "p1", "amount": 5 }])));
            // R18: "lose health" never goes through §4.4, so there is no damage instance to cap or absorb.
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Damage { .. }))
                    .count(),
                0
            );
        }
    }

    mod n27_blood_ridden_glowy_jelly_bean_radiant {
        use super::*;

        #[test]
        fn flags_2_different_random_cards_and_still_loses_5_health_s8_keeps_the_clause_it_does_not_restate() {
            crate::register_all();
            let mut s = drawing(json!(["core-005", "core-016", "core-010"]), true, None);

            s.start_turn();

            let flagged = radiant_hand(s.state());
            assert_eq!(flagged.len(), 2);
            assert_eq!(flagged.iter().collect::<IndexSet<_>>().len(), 2);
            s.expect_health(P1, 25);
        }

        #[test]
        fn r60_takes_all_of_them_when_the_hand_holds_fewer_than_2_non_radiant_cards() {
            crate::register_all();
            let mut s = drawing(json!(["core-016"]), true, None);

            s.start_turn();

            assert_eq!(radiant_hand(s.state()).len(), 1);
            s.expect_health(P1, 25);
        }

        #[test]
        fn keeps_cast_on_draw_on_the_radiant_face_so_a_radiant_copy_is_still_cast_from_the_library() {
            crate::register_all();
            let mut s = drawing(json!(["core-016"]), true, None);

            s.start_turn();

            s.expect_in_zone("core-027", "graveyard");
            assert!(!s.state().players.p1.hand.iter().any(|card| card.def_id == "core-027"));
        }

        #[test]
        fn r386_an_upgrade_flags_3_cards_and_a_degrade_1() {
            for (upgrade, cards) in [(true, 3), (false, 1)] {
                crate::register_all();
                let mut s = drawing(json!(["core-005", "core-016", "core-010", "core-011"]), true, None);
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-027", "cards")
                } else {
                    crate::degrade_number(&mut s, "core-027", "cards")
                };
                assert_eq!(moved, cards);
                s.start_turn();
                assert_eq!(radiant_hand(s.state()).len(), cards as usize);
            }
        }
    }

    #[test]
    fn r386_an_upgrade_loses_4_health_and_a_degrade_6() {
        for (upgrade, loss) in [(true, 4), (false, 6)] {
            crate::register_all();
            let mut s = drawing(json!(["core-016"]), false, None);
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "core-027", "loss")
            } else {
                crate::degrade_number(&mut s, "core-027", "loss")
            };
            assert_eq!(moved, loss);
            s.start_turn();
            s.expect_health(P1, 30 - loss);
        }
    }

    #[test]
    fn r749_the_base_face_s_one_card_is_never_tuned() {
        crate::register_all();
        let s = drawing(json!(["core-016"]), false, None);
        assert!(!crate::can_upgrade_number(&s, "core-027", "cards"));
        assert!(!crate::can_degrade_number(&s, "core-027", "cards"));
    }
}
