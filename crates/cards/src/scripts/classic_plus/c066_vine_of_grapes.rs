//! C+ #66 Vine of Grapes (SPEC §8.7 row 66). (3) Spell, Fruit, Rare.
//!   Base:    "Add {grapes|Grape|Grapes} to your hand, each rolled: Rotten Grape 12%, Normal Grape 60%,
//!            Large Grape 20%, Golden Grape 7%, Mythic Grape 1%."
//!   Radiant: "Lucky 1 / Add {grapes|Radiant Grape|Radiant Grapes} to your hand, each rolled: …"
//!   Engine:  "As C+ #65: five independent rolls (`GRAPE_ODDS`), the Radiant's with Lucky 1; the hand cap
//!            burns extras. Tunes: grapes 5 ↑."
//!
//! C+ #65 Two Grapes with five: the same engine verb (`addRolledGrapes`, effects/fruit.ts), the count
//! its own declared number. The designer's Radiant said 3, read as copied from Two Grapes; it is 5 so
//! the Radiant face never loses two Grapes (SPEC §8.7 row 66).

use jackioh_engine::effects::add_rolled_grapes;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-066";

fn vine_of_grapes(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut args = json!({ "count": param(ctx, "grapes") });
            if radiant {
                args["radiant"] = json!(true);
            }
            vec![add_rolled_grapes(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = vine_of_grapes(false);

    let radiant = vine_of_grapes(true);

    CardScripts { base, radiant }
}

// C+ #66 Vine of Grapes — SPEC §8.7 row 66, BUILD M9 Classic+ row C+ 66: "As Two Grapes with 5 Grapes;
// the count reads through `param()`; radiant 5 Radiant Grapes, each rolled with Lucky 1".
//
// The roll is C+ #65's (`addRolledGrapes`), whose odds are proved in packages/engine/test/
// effects-fruit.test.ts and through Two Grapes in 065-two-grapes.test.ts.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VINE: &str = "classicplus-066";
    const FILLER: &str = "core-005";

    fn grape_ids() -> Vec<&'static str> {
        GRAPE_ODDS.iter().map(|grape| grape.def_id).collect()
    }

    /// TS `GRAPE_IDS.indexOf(id)`: −1 for an id that is no Grape.
    fn index_of(id: &str) -> i64 {
        grape_ids().iter().position(|grape| *grape == id).map_or(-1, |at| at as i64)
    }

    /// An engine value as the JSON TS compares it with.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    /// One `addedToHand` event of p1's, as TS's `Extract<GameEvent, { type: "addedToHand" }>` reads it.
    struct Added {
        instance_id: String,
        def_id: String,
    }

    fn added(s: &Scenario) -> Vec<Added> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id } => Some(Added {
                    instance_id: instance_id.clone(),
                    def_id: def_id.clone(),
                }),
                _ => None,
            })
            .collect()
    }

    /// TS `played({ radiant?, seed?, hand? })`.
    fn played(radiant: bool, seed: Option<&str>, hand: Option<usize>) -> Scenario {
        let mut vine = json!({ "def": VINE });
        if radiant {
            vine["radiant"] = json!(true);
        }
        let mut cards = vec![vine];
        cards.extend((0..hand.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed.unwrap_or("vine"),
            "p1": { "hand": cards },
            "p2": { "hand": [FILLER] }
        }))
    }

    #[test]
    fn is_a_3_fruit_spell_with_no_refs_grape_names_the_five_grapes_r480_the_radiant_face_prints_lucky_1() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.id, VINE);
        assert_eq!(js(&def.cost), json!(3));
        assert!(def.tags.contains(&Tag::Fruit));
        assert!(def.refs.is_none());
        assert_eq!(js(&def.radiant.keywords), json!([{ "kind": "Lucky", "n": 1 }]));
        let CardScripts { base, radiant } = script();
        assert!(base.cry.is_some());
        assert!(radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r382_adds_5_grapes_each_its_own_roll_of_grape_odds_from_the_match_rng() {
            crate::register_all();
            let mut s = played(false, Some("five"), None);
            let cursor = s.state().rng_cursor;
            s.play(VINE, json!({}));

            let mut rng = Rng::new("five", cursor);
            let expected: Vec<String> =
                [0, 1, 2, 3, 4].iter().map(|_| jackioh_engine::catalog::roll_grape(&mut rng, 0)).collect();
            let got: Vec<String> = added(&s).into_iter().map(|event| event.def_id).collect();
            assert_eq!(got, expected);
            assert!(added(&s).iter().all(|event| !s.card(&event.instance_id).radiant));
            assert_eq!(s.state().rng_cursor - cursor, 5);
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_grapes_that_don_t_fit() {
            crate::register_all();
            let mut s = played(false, None, Some(7));
            s.play(VINE, json!({}));
            assert_eq!(added(&s).len(), 3);
            let burned = s.last_events().iter().filter(|event| event.event_type().as_str() == "burned").count();
            assert_eq!(burned, 2);
            assert_eq!(s.hand(P1).len(), 10);
        }

        #[test]
        fn r97_the_opponent_learns_only_that_five_cards_reached_the_hand() {
            crate::register_all();
            let mut s = played(false, None, None);
            s.play(VINE, json!({}));
            let theirs: Vec<GameEvent> = s
                .view(P2)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert_eq!(theirs.len(), 5);
            assert!(
                theirs
                    .iter()
                    .all(|event| matches!(event, GameEvent::AddedToHand { def_id, .. } if def_id == "hidden"))
            );
        }

        #[test]
        fn r386_an_upgrade_of_its_count_adds_6_a_degrade_4() {
            crate::register_all();
            let mut up = played(false, None, None);
            step_param(up.card_mut(VINE), "grapes", 1);
            up.play(VINE, json!({}));
            assert_eq!(added(&up).len(), 6);

            let mut down = played(false, None, None);
            step_param(down.card_mut(VINE), "grapes", -1);
            down.play(VINE, json!({}));
            assert_eq!(added(&down).len(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_s6_1_adds_5_radiant_grapes_each_two_rolls_with_the_better_kept() {
            crate::register_all();
            let mut s = played(true, Some("lucky-vine"), None);
            let cursor = s.state().rng_cursor;
            s.play(VINE, json!({}));

            let mut rng = Rng::new("lucky-vine", cursor);
            let ids = grape_ids();
            let expected: Vec<String> = [0, 1, 2, 3, 4]
                .iter()
                .map(|_| {
                    let first = index_of(&jackioh_engine::catalog::roll_grape(&mut rng, 0));
                    let second = index_of(&jackioh_engine::catalog::roll_grape(&mut rng, 0));
                    ids[first.max(second) as usize].to_string()
                })
                .collect();
            let got: Vec<String> = added(&s).into_iter().map(|event| event.def_id).collect();
            assert_eq!(got, expected);
            assert!(added(&s).iter().all(|event| s.card(&event.instance_id).radiant));
            assert_eq!(s.state().rng_cursor - cursor, 10);
        }

        #[test]
        fn r276_the_radiant_face_keeps_five_grapes_not_the_designer_s_three() {
            crate::register_all();
            let params = crate::CATALOG.get(VINE).map(|def| js(&def.params)).unwrap_or(Value::Null);
            let grapes = params
                .as_array()
                .and_then(|entries| entries.iter().find(|entry| entry["key"] == json!("grapes")))
                .cloned()
                .unwrap_or(Value::Null);
            assert_eq!(grapes["base"], json!(5));
            assert_eq!(grapes["radiant"], json!(5));
            let mut s = played(true, None, None);
            s.play(VINE, json!({}));
            assert_eq!(added(&s).len(), 5);
        }

        #[test]
        fn r386_the_radiant_count_steps_the_same_way() {
            crate::register_all();
            let mut s = played(true, None, None);
            step_param(s.card_mut(VINE), "grapes", -1);
            s.play(VINE, json!({}));
            assert_eq!(added(&s).len(), 4);
        }
    }
}
