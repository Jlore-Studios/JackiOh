//! C+ #68 Organic Produce (SPEC §8.7 row 68; BUILD M9 row C+ 68). (4) Field Spell, Fruit, Epic.
//!   Base:    "Cry: Add {fruits|random Fruit|random Fruits} to your hand. / Aura: Fruits you play are Radiant." — fruits 1
//!   Radiant: "Cry: Add {fruits|…} to your hand. Each costs (0). / (the same aura)" — fruits 2
//!
//! The Cry's pool is the Fruit pool (R382: the non-token Fruits and the five Grapes), never Organic
//! Produce itself (R387: `addRandomFromCatalog` leaves out the running card's id), repeats allowed
//! (R60). The aura is Core #64 Gifted Program's static flag at §10.5 step 3 by tag (R213, R214):
//! every Fruit its controller plays, casts included, resolves on its Radiant face.

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-068";

/// The Radiant face's "Each costs (0)."
const FREE: i32 = 0;

fn static_flags() -> StaticFlags {
    StaticFlags {
        radiant_plays_tagged: Some(vec![Tag::Fruit]),
        ..StaticFlags::default()
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(static_flags()),
        cry: Some(hook(|ctx| {
            vec![add_random_from_catalog(json_as(json!({ "query": { "tags": ["Fruit"] }, "count": param(ctx, "fruits") })))]
        })),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(static_flags()),
        cry: Some(hook(|ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Fruit"] },
                "count": param(ctx, "fruits"),
                "costOverride": FREE
            })))]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C+ #68 Organic Produce — SPEC §8.7 row 68, BUILD M9 row C+ 68: a Field Spell whose Cry adds a random
// card of the Fruit pool (R382), never Organic Produce (R387); its aura makes every Fruit-tagged card its
// controller plays Radiant at §10.5 step 3 (R213, R214), so it resolves on its Radiant face; the
// opponent's Fruits are untouched; a Fruit Trap set face-down is made Radiant unread by the opponent
// (R177); the count reads through `param()`; radiant its Cry adds 2 Fruits that cost (0).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PRODUCE: &str = "classicplus-068";
    const FIG: &str = "core-047"; // Fig of Life: (3) Spell, Fruit. "Heal a target 20." / Radiant 50.
    const TRAP: &str = "core-041"; // Sheepish, a Trap with no Fruit tag.
    const FILLER: &str = "core-005";

    /// An engine value as the JSON TS compares it with.
    fn js<T: serde::Serialize>(value: &T) -> Value {
        serde_json::to_value(value).expect("serialisable")
    }

    /// TS `FRUIT_POOL`: the Fruit query's ids, Organic Produce left out.
    fn fruit_pool() -> Vec<String> {
        jackioh_engine::catalog::query(&json_as(json!({ "tags": ["Fruit"] })))
            .iter()
            .map(|card| card.id.clone())
            .filter(|id| id != PRODUCE)
            .collect()
    }

    fn added(s: &Scenario, before: &[String]) -> Vec<String> {
        s.hand(P1)
            .into_iter()
            .filter(|card| !before.contains(&card.id))
            .map(|card| card.def_id)
            .collect()
    }

    fn hand_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).into_iter().map(|card| card.id).collect()
    }

    fn play_produce(s: &mut Scenario) -> Vec<String> {
        let before = hand_ids(s);
        s.play(PRODUCE, json!({ "zone": 1 }));
        added(s, &before)
    }

    fn heal_hero(s: &mut Scenario, player: PlayerId) {
        let Some(fig) = s.hand(player).into_iter().find(|card| card.def_id == FIG) else {
            panic!("no Fig of Life");
        };
        s.play(&fig, json!({ "targets": [{ "pick": "hero", "player": player }] }));
    }

    #[test]
    fn is_a_4_field_spell_fruit_both_faces_flag_fruit_plays_radiant() {
        crate::register_all();
        let def = js(&crate::card_def(ID));
        assert_eq!(def["cost"], json!(4));
        assert_eq!(def["type"], json!("Field Spell"));
        assert_eq!(def["tags"], json!(["Fruit"]));
        let CardScripts { base, radiant } = script();
        assert_eq!(base.static_flags.and_then(|flags| flags.radiant_plays_tagged), Some(vec![Tag::Fruit]));
        assert_eq!(radiant.static_flags.and_then(|flags| flags.radiant_plays_tagged), Some(vec![Tag::Fruit]));
    }

    mod base {
        use super::*;

        #[test]
        fn r382_its_cry_adds_one_random_card_of_the_fruit_pool_which_holds_the_five_grapes() {
            crate::register_all();
            let pool = fruit_pool();
            for id in ["classicplus-065-1", "classicplus-065-5", FIG] {
                assert!(pool.iter().any(|entry| entry == id));
            }
            let mut s = scenario(json!({ "p1": { "hand": [PRODUCE, FILLER] }, "p2": { "hand": [FILLER] } }));
            let fruit = play_produce(&mut s);
            assert_eq!(fruit.len(), 1);
            assert!(pool.contains(&fruit[0]));
        }

        #[test]
        fn r387_never_organic_produce_itself_whatever_the_seed_and_every_pick_is_from_the_fruit_pool() {
            crate::register_all();
            let pool = fruit_pool();
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..40 {
                let mut s = scenario(json!({
                    "seed": format!("produce-{n}"),
                    "p1": { "hand": [PRODUCE, FILLER] },
                    "p2": { "hand": [FILLER] }
                }));
                for id in play_produce(&mut s) {
                    seen.insert(id);
                }
            }
            assert!(!seen.contains(PRODUCE));
            assert!(seen.iter().all(|id| pool.contains(id)));
            assert!(seen.len() > 3);
        }

        #[test]
        fn s2_4_a_full_hand_burns_the_fruit_it_adds() {
            crate::register_all();
            let mut hand = vec![json!(PRODUCE)];
            hand.extend((0..HAND_CAP).map(|_| json!(FILLER)));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [FILLER] } }));
            let graveyard = s.pile(P1, "graveyard").len();
            play_produce(&mut s);
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(s.pile(P1, "graveyard").len(), graveyard + 1);
            s.expect_events(json!(["burned"]));
        }

        #[test]
        fn r386_the_count_reads_through_param_an_upgrade_of_fruits_adds_two() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [PRODUCE, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(PRODUCE), "fruits", 1);
            assert_eq!(play_produce(&mut s).len(), 2);
        }

        #[test]
        fn r213_r214_a_fruit_you_play_while_it_stands_is_made_radiant_as_it_is_played_and_resolves_on_its_radiant_face() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FIG, FILLER], "backrow": [PRODUCE], "health": 10, "mana": 10 },
                "p2": { "hand": [FILLER] }
            }));
            heal_hero(&mut s, P1);
            // Fig of Life's Radiant face heals 50, not 20.
            s.expect_health(P1, 60);
            assert!(s.card(FIG).radiant);
            s.expect_events(json!(["radiantSet", "cardPlayed"]));
        }

        #[test]
        fn r70_r213_a_fruit_you_cast_is_made_radiant_too() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [FILLER], "backrow": [PRODUCE] }, "p2": { "hand": [FILLER] } }));
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let state = s.state_mut();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                {
                    let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
                    {
                        let mut ctx = jackioh_engine::resolve::make_context(
                            &mut sink,
                            None,
                            jackioh_engine::resolve::HookOptions { controller: Some(P1), ..Default::default() },
                        );
                        jackioh_engine::resolve::apply_effects(
                            &[jackioh_engine::effects::cast_new(json_as(json!({ "def": FIG, "random": true })))],
                            &mut ctx,
                        );
                    }
                    jackioh_engine::triggers::settle(&mut sink, jackioh_engine::triggers::SettleOptions::default());
                }
                state.rng_cursor = rng.cursor();
            }
            let fig = s.pile(P1, "graveyard").into_iter().find(|card| card.def_id == FIG);
            assert_eq!(fig.as_ref().map(|card| card.radiant), Some(true));
            let fig_id = fig.map(|card| card.id);
            assert!(events.iter().any(|event| matches!(
                event,
                GameEvent::RadiantSet { instance_id, .. } if Some(instance_id) == fig_id.as_ref()
            )));
        }

        #[test]
        fn r213_a_non_fruit_you_play_is_untouched_and_so_are_the_opponent_s_fruits() {
            crate::register_all();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "backrow": [PRODUCE] },
                "p2": { "hand": [FIG, FILLER], "health": 10, "mana": 10 }
            }));
            heal_hero(&mut s, P2);
            s.expect_health(P2, 30);
            assert!(!s.card(FIG).radiant);
            let mut own = scenario(json!({ "p1": { "hand": [FILLER, FILLER], "backrow": [PRODUCE] }, "p2": { "hand": [FILLER] } }));
            let first = own.hand(P1)[0].clone();
            own.play(&first, json!({}));
            assert!(!own.events().iter().any(|event| matches!(event, GameEvent::RadiantSet { .. })));
        }

        #[test]
        fn r119_it_never_catches_its_own_play_but_a_second_organic_produce_is_a_fruit_radiant_its_cry_adds_2_that_cost_0() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [PRODUCE, PRODUCE, FILLER], "mana": 8 }, "p2": { "hand": [FILLER] } }));
            let hand = s.hand(P1);
            let (first, second) = (hand[0].id.clone(), hand[1].id.clone());
            s.play(&first, json!({ "zone": 1 }));
            assert!(!s.card(&first).radiant);
            let before = hand_ids(&s);
            s.play(&second, json!({ "zone": 2 }));
            assert!(s.card(&second).radiant);
            let fruits: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
            assert_eq!(fruits.len(), 2);
            let costs: Vec<i32> = fruits
                .iter()
                .map(|card| jackioh_engine::mana::effective_cost(s.state(), card, Default::default()))
                .collect();
            assert_eq!(costs, vec![0, 0]);
        }

        #[test]
        fn r177_a_fruit_trap_set_face_down_is_made_radiant_and_the_opponent_reads_only_that_some_card_changed() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [TRAP, FIG, FILLER], "backrow": [PRODUCE], "mana": 10 },
                "p2": { "hand": [FILLER] }
            }));
            // A Trap that carries the Fruit tag: Sheepish with a Fig of Life fused into it, kept in hand (R470).
            let fig = s.card(FIG).clone();
            let trap = s.card(TRAP).clone();
            let fused = {
                let mut events: Vec<GameEvent> = Vec::new();
                let state = s.state_mut();
                let mut rng = Rng::new(&state.seed, state.rng_cursor);
                let fused = {
                    let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
                    subsystems::fuse(
                        &mut sink,
                        json_as(json!({ "ingredients": [fig], "into": trap, "handPrice": "fused" })),
                    )
                };
                state.rng_cursor = rng.cursor();
                fused
            };
            let Some(fused) = fused else {
                panic!("the fusion");
            };
            assert_eq!(s.card(&fused.id).zone.z(), ZoneName::Hand);
            // The fused card carries Fig of Life's declared target too (R102).
            s.play(&fused.id, json!({ "zone": 3, "targets": [{ "pick": "hero", "player": "p1" }] }));
            let set = s.backrow(P1, 3);
            // R227: a card set face-down takes a fresh id.
            assert_eq!(set.as_ref().map(|card| card.def_id.clone()), Some(fused.def_id.clone()));
            assert_eq!(set.as_ref().map(|card| card.radiant), Some(true));
            assert_ne!(set.as_ref().and_then(|card| card.face_up), Some(true));
            let theirs: Vec<GameEvent> = s
                .view(P2)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::RadiantSet { .. }))
                .collect();
            assert_eq!(theirs.len(), 1);
            let hidden = jackioh_engine::view_for::HIDDEN_ID;
            assert!(matches!(
                &theirs[0],
                GameEvent::RadiantSet { instance_id, def_id, .. } if instance_id == hidden && def_id == hidden
            ));
            let mine: Vec<GameEvent> = s
                .view(P1)
                .events
                .into_iter()
                .filter(|event| matches!(event, GameEvent::RadiantSet { .. }))
                .collect();
            assert!(matches!(&mine[0], GameEvent::RadiantSet { def_id, .. } if *def_id == fused.def_id));
            assert!(!matches!(&mine[0], GameEvent::RadiantSet { instance_id, .. } if instance_id == hidden));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_adds_2_random_fruits_that_cost_0() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": PRODUCE, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
            let before = hand_ids(&s);
            s.play(PRODUCE, json!({ "zone": 1 }));
            let fruits: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).collect();
            assert_eq!(fruits.len(), 2);
            let pool = fruit_pool();
            assert!(fruits.iter().all(|card| pool.contains(&card.def_id)));
            let costs: Vec<i32> = fruits
                .iter()
                .map(|card| jackioh_engine::mana::effective_cost(s.state(), card, Default::default()))
                .collect();
            assert_eq!(costs, vec![0, 0]);
        }

        #[test]
        fn r213_its_aura_is_the_same_a_fruit_you_play_resolves_radiant() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FIG, FILLER], "backrow": [{ "def": PRODUCE, "radiant": true }], "health": 10, "mana": 10 },
                "p2": { "hand": [FILLER] }
            }));
            heal_hero(&mut s, P1);
            s.expect_health(P1, 60);
        }

        #[test]
        fn r386_an_upgrade_of_fruits_adds_three() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": PRODUCE, "radiant": true }, FILLER] }, "p2": { "hand": [FILLER] } }));
            step_param(s.card_mut(PRODUCE), "fruits", 1);
            let before = hand_ids(&s);
            s.play(PRODUCE, json!({ "zone": 1 }));
            assert_eq!(s.hand(P1).into_iter().filter(|card| !before.contains(&card.id)).count(), 3);
        }
    }
}
