//! C #67 Felinor Feeler (SPEC §8.6 row 67). (1) Unit, Human, Common, 2/4 → 4/8.
//!   Base:    "Pierce\nCry: Switch every enemy Unit to Defense Position."
//!   Radiant: "Pierce, Rush\nCry: Switch every enemy Unit to Defense Position."
//!   Engine:  "An effect's switch, which spends no exertion (R20); units already in Defense stay (R91);
//!            #65.1 Spikey Pillow never enters Defense. Tagged Human, not Felinor, as the designer
//!            tagged it. Tunes: none."
//!
//! Pierce and Rush are catalog keywords (§6.1, R346: its hits skip Armor). The Cry switches each enemy
//! Unit acting on the field — the top of each pile (R13), an animated card in the unit row included
//! (R383) — to Defense Position as an effect (`switchPositionOf` with `to: "DEF"`): no exertion is spent
//! (R20), a unit already in Defense is asked for the position it holds and nothing happens (R91), and a
//! unit that can't be in Defense Position (Spikey Pillow's `neverDefense`, §4.1) is refused and stays in
//! Attack. The set of units is read once, as the Cry reaches it (`forEachCard`, R113). The Radiant face
//! differs only in its catalog stats and Rush, so it runs the same script.
//!
//! Rulings: R20, R91, R13, R346. Its proof: `test/classic/067-felinor-feeler.test.ts`.

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, switch_position_of};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-067";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(|ctx: &mut EffectContext<'_>| -> Vec<String> {
                    active_units_of(&*ctx.state, opponent_of(ctx.controller))
                        .iter()
                        .map(|card| card.id.clone())
                        .collect()
                }),
                each: Arc::new(|instance_id: &str| {
                    switch_position_of(json_as(json!({
                        "to": "DEF",
                        "target": { "of": "instance", "instanceId": instance_id },
                    })))
                }),
            })]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face's 4/8 and its added Rush are catalog data.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #67 Felinor Feeler (SPEC §8.6 row 67; BUILD M9 row C 67). (1) Unit, Human, Common, 2/4 → 4/8:
// Pierce; Cry: switch every enemy Unit to Defense Position. Radiant: Pierce, Rush; the same Cry.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FEELER: &str = "classic-067";
    const VANILLA: &str = "core-008"; // 4/4
    const TIMMY: &str = "core-011"; // 3/3
    const PILLOW: &str = "core-065-1"; // Spikey Pillow: can't be in Defense Position
    const ARMORED: &str = "core-025"; // 4-mana 7/7, Armor 7
    const STOCKPILE: &str = "core-005";
    const FIENDER: &str = "core-092"; // Stack; has the stats of all your Felinors

    /// TS `const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] }`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": [VANILLA, VANILLA] })
    }

    /// TS `{ ...side, ...SPARE }`.
    fn with_spare(mut side: Value) -> Value {
        if let (Some(fields), Some(extra)) = (side.as_object_mut(), spare().as_object()) {
            for (key, value) in extra {
                fields.insert(key.clone(), value.clone());
            }
        }
        side
    }

    use crate::js;

    /// TS `s.unit(p, lane)!`.
    fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.unit(player, lane).unwrap_or_else(|| panic!("no unit in {player:?}'s lane {lane}"))
    }

    /// TS `s.stats(card).position`, as the JSON literal ("ATK" | "DEF").
    fn position(s: &Scenario, card: &CardInstance) -> Value {
        js(&s.stats(card).position)
    }

    fn switches(s: &Scenario) -> usize {
        s.last_events().iter().filter(|event| matches!(event, GameEvent::PositionSwitched { .. })).count()
    }

    mod c_67_felinor_feeler {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r20_cry_every_enemy_unit_switches_to_defense_position_as_an_effect_spending_no_exertion_yours_stay() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FEELER, STOCKPILE], "field": [TIMMY], "library": spare()["library"].clone() },
                    "p2": with_spare(json!({ "field": [VANILLA, TIMMY] })),
                }));
                s.play(FEELER, json!({}));
                for lane in [1, 2] {
                    let unit = unit_at(&s, P2, lane);
                    assert_eq!(position(&s, &unit), "DEF");
                    assert!(!s.card(&unit).exertion.switched);
                }
                assert_eq!(position(&s, &unit_at(&s, P1, 1)), "ATK");
                assert_eq!(switches(&s), 2);
            }

            #[test]
            fn r91_a_unit_already_in_defense_position_stays_with_no_switch_reported() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FEELER, STOCKPILE], "library": spare()["library"].clone() },
                    "p2": with_spare(json!({ "field": [{ "def": VANILLA, "position": "DEF" }, TIMMY] })),
                }));
                s.play(FEELER, json!({}));
                assert_eq!(position(&s, &unit_at(&s, P2, 1)), "DEF");
                assert_eq!(position(&s, &unit_at(&s, P2, 2)), "DEF");
                let switched: Vec<String> = s
                    .last_events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::PositionSwitched { instance_id, .. } => Some(instance_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(switched, vec![unit_at(&s, P2, 2).id]);
            }

            #[test]
            fn s4_1_spikey_pillow_which_cant_be_in_defense_position_stays_in_attack() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FEELER, STOCKPILE], "library": spare()["library"].clone() },
                    "p2": with_spare(json!({ "field": [PILLOW, VANILLA] })),
                }));
                s.play(FEELER, json!({}));
                assert_eq!(position(&s, &unit_at(&s, P2, 1)), "ATK");
                assert_eq!(position(&s, &unit_at(&s, P2, 2)), "DEF");
            }

            #[test]
            fn r346_pierce_its_hits_skip_armor() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": with_spare(json!({ "field": [FEELER] })),
                    "p2": with_spare(json!({ "field": [ARMORED] })),
                }));
                s.attack(FEELER, ARMORED);
                // 2 attack into Armor 7: Pierce lands all 2.
                assert_eq!(s.card(ARMORED).damage, 2);
            }

            #[test]
            fn s8_6_tagged_human_not_felinor_a_felinor_pool_never_finds_it() {
                crate::register_all();
                let tags = js(&crate::card_def(ID))["tags"].clone();
                let tags = tags.as_array().expect("the def's tags");
                assert!(tags.contains(&json!("Human")));
                assert!(!tags.contains(&json!("Felinor")));
                let ids_tagged = |tag: &str| -> Vec<String> {
                    crate::query::query(&json_as::<CatalogQueryArgs>(json!({ "tags": [tag] })))
                        .iter()
                        .map(|card| card.id.clone())
                        .collect()
                };
                assert!(!ids_tagged("Felinor").contains(&FEELER.to_string()));
                assert!(ids_tagged("Human").contains(&FEELER.to_string()));
            }

            #[test]
            fn s8_6_nor_a_felinor_count_felinor_fienders_stats_are_the_same_beside_it() {
                crate::register_all();
                let alone = scenario(json!({ "p1": { "field": [FIENDER] } }));
                let beside = scenario(json!({ "p1": { "field": [FIENDER, FEELER] } }));
                assert_eq!(beside.stats(FIENDER).attack, alone.stats(FIENDER).attack);
                assert_eq!(beside.stats(FIENDER).health, alone.stats(FIENDER).health);
            }

            #[test]
            fn r13_a_card_dormant_under_an_enemy_stack_pile_is_not_on_the_field_and_keeps_its_position() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FEELER, STOCKPILE], "library": spare()["library"].clone() },
                    "p2": with_spare(json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] })),
                }));
                let slot: ZoneRef = json_as(json!({ "player": "p2", "row": "units", "lane": 1 }));
                let dormant = beneath_at(s.state(), slot).first().cloned().expect("a card beneath the Fiender");
                assert_eq!(dormant.def_id, VANILLA);
                s.play(FEELER, json!({}));
                assert_eq!(position(&s, &unit_at(&s, P2, 1)), "DEF");
                assert_eq!(js(&s.card(&dormant).position), "ATK");
                assert_eq!(switches(&s), 1);
            }

            #[test]
            fn s8_6_a_2_4_with_pierce_and_no_rush_it_cannot_attack_the_turn_it_enters() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FEELER, STOCKPILE], "library": spare()["library"].clone() },
                    "p2": with_spare(json!({ "field": [VANILLA] })),
                }));
                s.play(FEELER, json!({}));
                s.expect_stats(FEELER, json!({ "attack": 2, "health": 4 }));
                s.expect_refused(|s| s.attack(FEELER, VANILLA));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s8_6_4_8_pierce_rush_the_same_cry_and_it_may_attack_a_unit_the_turn_it_enters() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FEELER, "radiant": true }, STOCKPILE], "library": spare()["library"].clone() },
                    "p2": with_spare(json!({ "field": [VANILLA, PILLOW] })),
                }));
                s.play(FEELER, json!({}));
                s.expect_stats(FEELER, json!({ "attack": 4, "health": 8 }));
                assert_eq!(position(&s, &unit_at(&s, P2, 1)), "DEF");
                assert_eq!(position(&s, &unit_at(&s, P2, 2)), "ATK");
                let foe = unit_at(&s, P2, 1);
                s.attack(FEELER, &foe);
                // 4 into the Vanilla's Defense Armor 1: Pierce lands all 4.
                s.expect_in_zone(&foe, "graveyard");
                s.expect_refused(|s| s.attack(FEELER, "hero"));
            }
        }
    }
}
