//! C+ #65.1 Rotten Grape (SPEC §8.7 row 65.1). (1) Spell, Fruit, Token (printed Common).
//!   Base:    "Your hero loses 5 health."
//!   Radiant: "Your hero loses 1 health."
//!   Engine:  "Lose health (§6.3, R18): no pipeline, no Armor, no cap. The Radiant's smaller loss is its
//!            upgrade (R275). A Fruit for every Fruit rule (C+ #64, C+ #68). Tunes: none."
//!
//! R18: losing health is not damage — `loseHealth` takes it straight off the hero, past Armor, under no
//! per-hit cap and through no replacement window, and a hero at 0 loses at the state check after the
//! list (§4.5). The entry declares no params (the Rotten Grape's numbers are printed), so the two
//! amounts are named here.

use jackioh_engine::effects::lose_health;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-065-1";

/// §8.7: the health the base face costs its own hero.
const BASE_LOSS: i32 = 5;
/// §8.7: the Radiant face's smaller loss, its upgrade (R275).
const RADIANT_LOSS: i32 = 1;

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| vec![lose_health(json_as(json!({ "player": "self", "amount": BASE_LOSS })))])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![lose_health(json_as(json!({ "player": "self", "amount": RADIANT_LOSS })))])),
            ..Script::default()
        },
    }
}

// C+ #65.1 Rotten Grape — SPEC §8.7 row 65.1, BUILD M9 Classic+ row C+ 65.1: "Your hero loses 5 health
// (R18): no Armor, no cap, no Blood Moon, no on-damage effect, and at 0 you lose at the state check;
// radiant loses 1".
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::convert_healing;
    use jackioh_engine::testkit::*;

    const ROTTEN: &str = "classicplus-065-1";
    const FILLER: &str = "core-005";
    const GOING_LONG: &str = "core-084"; // Field Spell: "Your hero has Armor 2."
    const ANTI_ONESHOT: &str = "core-073"; // Field Spell: "Your hero can't take more than 5 damage at once." (Radiant 3)
    const MENACE: &str = "core-019";

    /// `_harness.ts` registers every card on import; the engine's testkit cannot, so this does.
    fn scenario(opts: Value) -> Scenario {
        crate::register_all();
        jackioh_engine::testkit::scenario(opts)
    }

    fn health_lost(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::HealthLost { .. }))
            .cloned()
            .collect()
    }

    fn any_damage(s: &Scenario) -> bool {
        s.last_events().iter().any(|event| matches!(event, GameEvent::Damage { .. }))
    }

    fn winner(s: &Scenario) -> Option<Winner> {
        s.state().result.as_ref().map(|result| result.winner)
    }

    #[test]
    fn is_a_1_fruit_spell_token_with_the_printed_rarity_common() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, ROTTEN);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert_eq!(def.tags, vec![Tag::Fruit, Tag::Token]);
        assert_eq!(def.rarity, Rarity::Token);
        assert_eq!(def.printed_rarity, Some(PrintedRarity::Common));
        let scripts = super::script();
        assert!(scripts.base.cry.is_some());
        assert!(scripts.radiant.cry.is_some());
    }

    mod base {
        use super::*;

        #[test]
        fn r18_your_hero_loses_5_health_a_health_lost_never_a_damage_instance() {
            let mut s = scenario(json!({ "p1": { "hand": [ROTTEN, FILLER] }, "p2": { "hand": [FILLER] } }));
            s.play(ROTTEN, json!({}));

            s.expect_health(PlayerId::P1, 25).expect_health(PlayerId::P2, 30);
            assert_eq!(health_lost(&s), vec![GameEvent::HealthLost { player: PlayerId::P1, amount: 5 }]);
            assert!(!any_damage(&s));
            s.expect_in_zone(ROTTEN, "graveyard");
        }

        #[test]
        fn r18_armor_takes_none_of_it_a_hero_with_armor_2_still_loses_5() {
            let mut s = scenario(json!({
                "p1": { "hand": [ROTTEN, FILLER], "backrow": [{ "def": GOING_LONG, "faceUp": true }] },
                "p2": { "hand": [FILLER] },
            }));
            assert_eq!(s.view(PlayerId::P1).you.hero.armor, 2);
            s.play(ROTTEN, json!({}));
            s.expect_health(PlayerId::P1, 25);
        }

        #[test]
        fn r18_no_cap_a_radiant_anti_oneshot_armor_at_most_3_at_once_leaves_the_5_whole() {
            let mut s = scenario(json!({
                "p1": { "hand": [ROTTEN, FILLER], "backrow": [{ "def": ANTI_ONESHOT, "faceUp": true, "radiant": true }] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ROTTEN, json!({}));
            s.expect_health(PlayerId::P1, 25);
        }

        #[test]
        fn r18_no_on_damage_effect_no_damage_event_goes_out_so_nothing_that_answers_a_hit_fires() {
            let mut s = scenario(json!({ "p1": { "hand": [ROTTEN, FILLER] }, "p2": { "hand": [FILLER] } }));
            s.play(ROTTEN, json!({}));
            assert!(!any_damage(&s));
            assert_eq!(health_lost(&s).len(), 1);
        }

        #[test]
        fn r18_no_blood_moon_with_the_enemys_heal_to_damage_conversion_b5_e8_up_the_loss_is_still_5_health_and_no_hit() {
            let mut s = scenario(json!({
                "p1": { "hand": [ROTTEN, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            let menace = s.unit(PlayerId::P2, 1);
            let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                let mut ctx = make_context(
                    &mut sink,
                    menace.as_ref(),
                    HookOptions { controller: Some(PlayerId::P2), ..HookOptions::default() },
                );
                apply_effects(&[convert_healing(json_as(json!({})))], &mut ctx);
            }
            s.state_mut().rng_cursor = rng.cursor();
            assert!(
                s.state()
                    .players
                    .p2
                    .mods
                    .iter()
                    .any(|m| matches!(m.kind, ModifierKind::HealToDamage { .. }))
            );
            s.play(ROTTEN, json!({}));
            s.expect_health(PlayerId::P1, 25);
            assert!(!any_damage(&s));
        }

        #[test]
        fn s4_5_at_0_you_lose_at_the_state_check_after_it() {
            let mut s = scenario(json!({ "p1": { "hand": [ROTTEN, FILLER], "health": 5 }, "p2": { "hand": [FILLER] } }));
            s.play(ROTTEN, json!({}));
            assert_eq!(winner(&s), Some(Winner::P2));
        }

        #[test]
        fn r18_below_5_health_it_takes_you_below_0_all_the_same() {
            let mut s = scenario(json!({ "p1": { "hand": [ROTTEN, FILLER], "health": 3 }, "p2": { "hand": [FILLER] } }));
            s.play(ROTTEN, json!({}));
            assert_eq!(winner(&s), Some(Winner::P2));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r275_its_upgrade_is_the_smaller_loss_your_hero_loses_1() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ROTTEN, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ROTTEN, json!({}));
            s.expect_health(PlayerId::P1, 29);
            assert_eq!(health_lost(&s), vec![GameEvent::HealthLost { player: PlayerId::P1, amount: 1 }]);
        }

        #[test]
        fn r18_armor_takes_none_of_the_radiant_loss_either() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [{ "def": ROTTEN, "radiant": true }, FILLER],
                    "backrow": [{ "def": GOING_LONG, "faceUp": true }],
                },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ROTTEN, json!({}));
            s.expect_health(PlayerId::P1, 29);
        }

        #[test]
        fn s4_5_at_1_health_the_radiant_face_still_loses_you_the_game() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ROTTEN, "radiant": true }, FILLER], "health": 1 },
                "p2": { "hand": [FILLER] },
            }));
            s.play(ROTTEN, json!({}));
            assert_eq!(winner(&s), Some(Winner::P2));
        }
    }
}
