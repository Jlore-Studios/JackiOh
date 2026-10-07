//! C+ #60 Doctors Orders (SPEC §8.7 row 60). (1) Field Spell, Rare.
//!   Base:    "Activate: Add {apples|All Purpose Apple|All Purpose Apples} to your hand." — 1
//!            (balance patch 1: the Cry and the start-of-turn trigger became one Activate)
//!   Radiant: "Activate: Add {apples|Radiant All Purpose Apple|…} to your hand."
//!   Engine:  "A Field Spell (R421). The Apple is C+ #59, named, so no pool; the hand cap burns it
//!            (§2.4). No Fruit tag. Tunes: apples 1 ↑."

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-060";

/// TS `cardDef("classicplus-059").id`.
const APPLE: &str = "classicplus-059";

fn doctors_orders(radiant: bool) -> Script {
    let order = ActivationDecl {
        id: "order".to_string(),
        label: "Add an All Purpose Apple to your hand".to_string(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: Vec::new(),
        modes: Vec::new(),
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            (0..param(&*ctx, "apples"))
                .map(|_| add_to_hand(json_as(json!({ "defId": APPLE, "radiant": radiant }))))
                .collect()
        }),
    };
    Script {
        activations: vec![order],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: doctors_orders(false),
        radiant: doctors_orders(true),
    }
}

// C+ #60 Doctors Orders — SPEC §8.7 row 60, BUILD M9 Classic+ row C+ 60: "A Field Spell with no stats
// (R421): one Activate (balance patch 1: the Cry and the start-of-turn trigger became one Activate,
// once) adds an All Purpose Apple (C+ #59) to your hand, burned when the hand is full; no Cry, nothing
// at either start of turn; it needs an open backrow zone to play; the count reads through `param()`;
// radiant the Apple is Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const ORDERS: &str = "classicplus-060";
    const APPLE: &str = "classicplus-059";
    const WELL: &str = "core-006"; // Mana Well, a Field Spell to fill a backrow
    const FILLER: &str = "core-005";

    use crate::scenario;

    /// The Apples that reached a hand in the last step (a draw reports `addedToHand` too), as
    /// `(instanceId, defId)`.
    fn added(s: &Scenario, player: PlayerId) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: to, instance_id, def_id } if *to == player && def_id == APPLE => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    /// TS `inHand({ radiant?, fillers?, backrow? })`.
    fn in_hand(radiant: bool, fillers: Option<usize>, backrow: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ORDERS, "radiant": radiant })];
        hand.extend((0..fillers.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "p1": { "hand": hand, "backrow": backrow, "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    /// Doctors Orders already in p1's backrow, p2 about to end their turn.
    fn standing(radiant: bool, fillers: Option<usize>) -> Scenario {
        let hand: Vec<&str> = (0..fillers.unwrap_or(1)).map(|_| FILLER).collect();
        scenario(json!({
            "active": "p2",
            "turn": 10,
            "p1": {
                "backrow": [{ "def": ORDERS, "radiant": radiant }],
                "hand": hand,
                "library": [FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    #[test]
    fn r421_is_a_1_field_spell_with_no_stats_and_no_fruit_tag_naming_the_apple() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.type_, CardType::FieldSpell);
        assert_eq!(def.base.attack, None);
        assert_eq!(def.tags, Vec::<Tag>::new());
        assert_eq!(def.refs, Some(vec![APPLE.to_string()]));
    }

    mod base {
        use super::*;

        #[test]
        fn playing_adds_nothing_the_cry_is_gone() {
            let mut s = in_hand(false, None, json!([]));
            s.play(ORDERS, json!({}));
            s.expect_in_zone(ORDERS, "field");
            assert_eq!(added(&s, PlayerId::P1), Vec::<(String, String)>::new());
        }

        #[test]
        fn its_activate_adds_an_all_purpose_apple_to_your_hand() {
            let mut s = in_hand(false, None, json!([]));
            s.play(ORDERS, json!({}));
            s.activate(ORDERS, json!({}));
            let apples = added(&s, PlayerId::P1);
            let defs: Vec<String> = apples.iter().map(|(_, def_id)| def_id.clone()).collect();
            assert_eq!(defs, vec![APPLE.to_string()]);
            let id = apples.first().map(|(id, _)| id.clone()).unwrap_or_default();
            assert!(!s.card(&id).radiant);
        }

        #[test]
        fn the_activate_is_once_a_second_activation_is_refused() {
            let mut s = in_hand(false, Some(2), json!([]));
            s.play(ORDERS, json!({}));
            s.activate(ORDERS, json!({}));
            assert_eq!(added(&s, PlayerId::P1).len(), 1);
            s.expect_refused(|s| s.activate(ORDERS, json!({})));
            assert_eq!(s.hand(PlayerId::P1).iter().filter(|card| card.def_id == APPLE).count(), 1);
        }

        #[test]
        fn r62_neither_start_of_turn_adds_anything() {
            let mut s = standing(false, None);
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P1);
            assert_eq!(added(&s, PlayerId::P1), Vec::<(String, String)>::new());
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(added(&s, PlayerId::P1), Vec::<(String, String)>::new());
        }

        #[test]
        fn r62_nothing_at_the_opponents_start_of_turn() {
            let mut s = in_hand(false, None, json!([]));
            s.play(ORDERS, json!({}));
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P2);
            assert_eq!(added(&s, PlayerId::P1), Vec::<(String, String)>::new());
        }

        #[test]
        fn s3_2_it_needs_an_open_backrow_zone_to_play() {
            let mut s = in_hand(false, None, json!([WELL, WELL, WELL, WELL, WELL]));
            s.expect_refused(|s| s.play(ORDERS, json!({})));
            s.expect_in_zone(ORDERS, "hand");
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_apple() {
            let mut s = standing(false, Some(9));
            s.end_turn();
            assert_eq!(s.state().active, PlayerId::P1);
            assert_eq!(s.hand(PlayerId::P1).len(), 10);
            s.activate(ORDERS, json!({}));
            let burned = s.last_events().iter().find_map(|event| match event {
                GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            });
            assert_eq!(burned.unwrap_or_default(), APPLE);
        }

        #[test]
        fn r97_the_opponent_sees_the_add_under_the_sentinel() {
            let mut s = in_hand(false, None, json!([]));
            s.play(ORDERS, json!({}));
            s.activate(ORDERS, json!({}));
            let view = s.view(PlayerId::P2);
            let theirs: Vec<&GameEvent> = view
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert!(!theirs.is_empty());
            for event in theirs {
                assert!(matches!(
                    event,
                    GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden"
                ));
            }
        }

        #[test]
        fn r386_an_upgrade_makes_the_activation_add_2_apples() {
            let mut s = in_hand(false, None, json!([]));
            step_param(s.card_mut(ORDERS), "apples", 1);
            s.play(ORDERS, json!({}));
            s.activate(ORDERS, json!({}));
            assert_eq!(added(&s, PlayerId::P1).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_apple_of_its_activation_is_radiant() {
            let mut s = in_hand(true, None, json!([]));
            s.play(ORDERS, json!({}));
            assert_eq!(added(&s, PlayerId::P1), Vec::<(String, String)>::new());
            s.activate(ORDERS, json!({}));
            let radiant: Vec<bool> = added(&s, PlayerId::P1).iter().map(|(id, _)| s.card(id).radiant).collect();
            assert_eq!(radiant, vec![true]);
        }

        #[test]
        fn the_radiant_apple_it_adds_plays_as_a_radiant_apple_a_radiant_rush_token_heal_4_2_damage() {
            let mut s = in_hand(true, None, json!([]));
            s.play(ORDERS, json!({}));
            s.activate(ORDERS, json!({}));
            let apple = added(&s, PlayerId::P1).first().map(|(id, _)| id.clone()).unwrap_or_default();
            s.play(&apple, json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(s.unit(PlayerId::P1, 1).map(|card| card.radiant), Some(true));
            s.expect_health(PlayerId::P1, 34).expect_health(PlayerId::P2, 28);
        }
    }
}
