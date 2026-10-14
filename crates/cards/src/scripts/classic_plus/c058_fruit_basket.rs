//! C+ #58 Fruit Basket (SPEC §8.7 row 58). (1) Spell, Fruit, Rare.
//!   Base:    "Add {fruits|random Fruit|random Fruits} to your hand." — fruits 3
//!   Radiant: "Add {fruits|random Radiant Fruit|random Radiant Fruits} to your hand."
//!   Engine:  "The Fruit pool (R382): the non-token Fruit cards of every set plus the five Grapes (C+
//!            #65.1 to C+ #65.5), Fruit Basket excluded (R387); each pick uniform over the pool, a Grape
//!            one entry like any card; repeats allowed (R60); the hand cap burns extras. Tunes: fruits 3 ↑."
//!
//! A `tags: ["Fruit"]` query is the Fruit pool, Grapes included (R382); the verb excludes this card (R387).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-058";

fn fruit_basket(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Fruit"] },
                "count": param(&*ctx, "fruits"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fruit_basket(false),
        radiant: fruit_basket(true),
    }
}

// C+ #58 Fruit Basket — SPEC §8.7 row 58, BUILD M9 Classic+ row C+ 58: "Adds 3 random cards of the
// Fruit pool, the non-token Fruit cards of any set plus the five Grapes (R382), never Fruit Basket
// (R387), repeats allowed; a seeded game can hand out a Grape; a full hand burns; hidden from the
// opponent (R97); the count reads through `param()`; radiant they are Radiant".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BASKET: &str = "classicplus-058";
    const FILLER: &str = "core-005";
    const GRAPES: [&str; 5] = [
        "classicplus-065-1",
        "classicplus-065-2",
        "classicplus-065-3",
        "classicplus-065-4",
        "classicplus-065-5",
    ];

    use crate::scenario;

    /// TS `basket({ radiant?, seed?, fillers? })`: `None` is the TS default.
    fn basket(radiant: bool, seed: Option<&str>, fillers: Option<usize>) -> Scenario {
        let mut hand = vec![json!({ "def": BASKET, "radiant": radiant })];
        hand.extend((0..fillers.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed.unwrap_or("fruit-basket"),
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// The `addedToHand` events of p1 in the last step, as `(instanceId, defId)`.
    fn added(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id } => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn is_grape(id: &str) -> bool {
        GRAPES.contains(&id)
    }

    #[test]
    fn is_a_1_spell_fruit() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, BASKET);
        assert_eq!(def.tags, vec![Tag::Fruit]);
    }

    mod base {
        use super::*;

        #[test]
        fn adds_3_random_fruits_to_your_hand() {
            let mut s = basket(false, None, None);
            s.play(BASKET, json!({}));
            let fruits: Vec<CardInstance> = added(&s).iter().map(|(id, _)| s.card(id).clone()).collect();
            assert_eq!(fruits.len(), 3);
            for card in &fruits {
                assert_eq!(card.zone.z(), ZoneName::Hand);
                assert!(def_of(Some(s.state()), &card.def_id).tags.contains(&Tag::Fruit));
                assert!(!card.radiant);
            }
        }

        #[test]
        fn r382_r387_over_many_seeds_the_fruit_pool_grapes_included_never_fruit_basket() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..80 {
                let mut s = basket(false, Some(&format!("basket-{i}")), None);
                s.play(BASKET, json!({}));
                for (_, def_id) in added(&s) {
                    seen.insert(def_id);
                }
            }
            assert!(!seen.contains(BASKET));
            let fresh = basket(false, None, None);
            for id in &seen {
                assert!(def_of(Some(fresh.state()), id).tags.contains(&Tag::Fruit));
            }
            assert!(seen.iter().any(|id| is_grape(id)));
            assert!(seen.iter().any(|id| !is_grape(id)));
            let tokens: Vec<&String> = seen.iter().filter(|id| def_of(Some(fresh.state()), id).token).collect();
            assert!(tokens.iter().all(|id| is_grape(id)));
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_what_does_not_fit() {
            let mut s = basket(false, None, Some(8));
            s.play(BASKET, json!({}));
            assert_eq!(added(&s).len(), 2);
            assert_eq!(
                s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(),
                1
            );
        }

        #[test]
        fn r97_the_opponent_sees_the_adds_under_the_sentinel() {
            let mut s = basket(false, None, None);
            s.play(BASKET, json!({}));
            let ids: Vec<String> = added(&s).into_iter().map(|(id, _)| id).collect();
            let theirs = s.view(PlayerId::P2);
            let events: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert_eq!(events.len(), 3);
            for event in events {
                assert!(matches!(
                    event,
                    GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden"
                ));
            }
            let text = serde_json::to_string(&theirs).unwrap();
            for id in &ids {
                assert!(!text.contains(&format!("\"{id}\"")));
            }
        }

        #[test]
        fn r386_an_upgrade_adds_4_a_degrade_2() {
            let mut up = basket(false, None, None);
            step_param(up.card_mut(BASKET), "fruits", 1);
            up.play(BASKET, json!({}));
            assert_eq!(added(&up).len(), 4);

            let mut down = basket(false, None, None);
            step_param(down.card_mut(BASKET), "fruits", -1);
            down.play(BASKET, json!({}));
            assert_eq!(added(&down).len(), 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_fruits_are_radiant_a_grape_too_and_never_fruit_basket() {
            let mut seen: Vec<(String, bool)> = Vec::new();
            for i in 0..60 {
                let mut s = basket(true, Some(&format!("rbasket-{i}")), None);
                s.play(BASKET, json!({}));
                for (id, def_id) in added(&s) {
                    let radiant = s.card(&id).radiant;
                    seen.push((def_id, radiant));
                }
            }
            assert!(seen.iter().all(|entry| entry.1));
            assert!(!seen.iter().any(|entry| entry.0 == BASKET));
            assert!(seen.iter().any(|entry| is_grape(&entry.0)));
        }

        #[test]
        fn r60_s9_3_repeats_are_allowed_and_the_same_seed_adds_the_same_fruits() {
            let mut a = basket(true, Some("same"), None);
            let mut b = basket(true, Some("same"), None);
            a.play(BASKET, json!({}));
            b.play(BASKET, json!({}));
            let defs = |s: &Scenario| -> Vec<String> { added(s).into_iter().map(|(_, def_id)| def_id).collect() };
            assert_eq!(defs(&a), defs(&b));
            let mut repeated = false;
            let mut i = 0;
            while i < 100 && !repeated {
                let mut s = basket(true, Some(&format!("rep-{i}")), None);
                i += 1;
                s.play(BASKET, json!({}));
                let ids = defs(&s);
                repeated = ids.iter().collect::<IndexSet<_>>().len() < ids.len();
            }
            assert!(repeated);
        }
    }
}
