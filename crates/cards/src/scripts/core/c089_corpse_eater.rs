//! #89 Corpse Eater (SPEC §8.5, §6.2, §10.3, §10.4, R11, R38, R68, R78, R89).
//!
//! Base: "Rush. While in your hand: whenever a unit on either side dies, this gains its attack and
//! max health". Radiant: "Rush, Divine Shield; gains double". §8's Conventions: both keyword lists
//! are printed in `catalog.json` (§10.4 layer 1), so no script grants them and only the gain doubles.
//!
//! A hand-zone trigger: "while in your hand" and "stops once on the field" are the registry's
//! doing (§10.3, R68). It reads the event, not the instance: R78 resets an instance leaving the
//! field, and R89's `destroyed` event carries the attack and max health the layers computed at death
//! (R38's "current" numbers). A death is a unit leaving the field (§4.5 step 1, §6.2); a discard or
//! burn emits another event. Units only, tokens excluded (R11); `def_of` is the layers' own catalog
//! read, a transient Fuse def included (R77). The gain is a layer-4 `buff`, kept on entering the field.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-089";

/// `factor` is the whole of the radiant text: 1 gains its stats, 2 gains double (§8 Conventions).
fn feed(factor: i32) -> TriggerDef {
    TriggerDef::new("corpse-eater-feed", &[GameEventType::Destroyed], move |ctx, event| {
        // `on` already narrows this, and the check is what gives the payload its type.
        let GameEvent::Destroyed {
            def_id,
            attack,
            max_health,
            ..
        } = event
        else {
            return vec![];
        };

        let dead = def_of(Some(&*ctx.state), def_id);
        // A destroyed Field Spell or Trap emits the same event; only units feed it.
        if dead.type_ != CardType::Unit {
            return vec![];
        }
        // R11: a unit token ceases to exist and never reaches a graveyard, so it never died for this.
        if dead.token {
            return vec![];
        }

        // R38, R89: the attack and max health the layers computed at the moment it died. R219: a gain
        // is never a loss, so a unit #46 starved below 0 max health gives nothing rather than shrinking
        // the Eater (§10.4 floors attack at 0 already; max health below 0 is only the check's signal).
        let gained_attack = (*attack).max(0) * factor;
        let gained_health = (*max_health).max(0) * factor;
        vec![buff(json_as(json!({
            "target": { "of": "self" },
            "attack": gained_attack,
            "health": gained_health,
        })))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        hand_triggers: vec![feed(1)],
        ..Script::default()
    };
    let radiant = Script {
        hand_triggers: vec![feed(2)],
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #89 Corpse Eater (SPEC §8.5, BUILD M4-T4 row 89): "In hand it gains the dying unit's current
// attack and max health from either side, tokens excluded (R11); stats per R38; stops once on the
// field; radiant double". R219: a gain is never a loss, so a unit #46 starved below 0 max health
// feeds it nothing.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EATER: &str = "core-089"; // 2/2 → 6/6, cost 4

    // Inert fixtures: every unit below has a Cry and nothing else, and the harness's `field` setup
    // never fires a Cry.
    const GARY: &str = "core-004"; // 1/1
    const FELINORS: &str = "core-012"; // 3/4
    const RENO: &str = "core-053"; // 4/6
    const STRAAZA: &str = "core-054"; // 8/8
    const FELINOR_TOKEN: &str = "core-t-felinor"; // 1/1 unit token (§7)
    const SUPPRESSIVE_AURA: &str = "core-046"; // −2/−2 to enemy units

    // §2.5: one always-playable card per hand keeps a scenario on the turn it started on.
    const FILLER: &str = "core-005";

    const SEED: &str = "eater-89";

    use crate::js;

    use crate::matches_object;

    fn count_of(s: &Scenario, type_: GameEventType) -> usize {
        s.events().iter().filter(|event| event.event_type() == type_).count()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(|keyword| keyword.kind()).collect()
    }

    mod n89_corpse_eater_printed_faces_s8_conventions {
        use super::*;

        #[test]
        fn the_radiant_keyword_cell_lists_no_plus_so_rush_and_divine_shield_are_the_complete_list() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(kinds(&def.base.keywords), vec![KeywordKind::Rush]);
            assert_eq!(
                kinds(&def.radiant.keywords),
                vec![KeywordKind::Rush, KeywordKind::DivineShield]
            );
        }
    }

    mod n89_corpse_eater_base {
        use super::*;

        #[test]
        fn r89_in_hand_it_gains_an_ally_unit_s_attack_and_max_health_when_it_dies() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
            s.attack(FELINORS, GARY);

            // Gary was 1/1, so the Eater is 3/3 while still sitting in hand.
            s.expect_stats(EATER, json!({ "attack": 3, "maxHealth": 3 }));
            assert_eq!(s.card(EATER).zone.z(), ZoneName::Hand);
            s.expect_events(json!(["destroyed", "buffed"]));
        }

        #[test]
        fn on_either_side_an_enemy_unit_dying_feeds_it_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.attack(FELINORS, GARY);

            s.expect_stats(EATER, json!({ "attack": 3, "maxHealth": 3 }));
        }

        #[test]
        fn r38_counts_the_dying_unit_s_current_attack_and_max_health_not_its_remaining_health() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                // Reno is 4/6 sitting on 5 damage, so its current health is 1 and its max health is 6.
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": RENO, "damage": 5, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": STRAAZA, "lane": 1 }] },
            }));
            s.expect_stats(RENO, json!({ "attack": 4, "health": 1, "maxHealth": 6 }));

            s.attack(STRAAZA, RENO);

            // 2/2 + 4/6. Reading current health instead would have made it 6/3.
            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 8 }));
        }

        #[test]
        fn r89_counts_the_layers_numbers_two_deaths_in_one_state_check_both_feed_it() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                // Gary (1/1) trades with Felinors' 3 attack and deals 1 back to a 1/1 attacker.
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            let attacker = s.unit(P2, 1).expect("p2's Gary");
            let defender = s.unit(P1, 1).expect("p1's Gary");
            s.attack(&attacker, &defender);

            // Both 1/1s die in the same check, so the Eater eats twice: 2/2 + 1/1 + 1/1.
            s.expect_stats(EATER, json!({ "attack": 4, "maxHealth": 4 }));
        }

        #[test]
        fn r11_a_unit_token_never_reaches_a_graveyard_so_it_never_feeds_the_eater() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": FELINOR_TOKEN, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));
            let token = s.card(FELINOR_TOKEN).clone();

            s.attack(FELINORS, &token);

            // R11: the token ceased to exist — in no pile at all — and the Eater is untouched.
            s.expect_in_zone(&token, "gone");
            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
            assert_eq!(count_of(&s, GameEventType::Buffed), 0);
        }

        #[test]
        fn s8_5_a_card_reaching_a_graveyard_from_a_hand_never_died_so_a_burn_does_not_count_r4() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                // HAND_CAP is 10: the hand is already full, so the draw below is burned (§2.4, R4).
                "p1": {
                    "hand": [EATER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER, FILLER],
                    "library": [GARY],
                },
                "p2": { "hand": [FILLER] },
            }));

            s.start_turn();

            // The unit went from hand to graveyard: `burned`, never `destroyed`.
            s.expect_events(json!(["drawn", "burned"]));
            assert_eq!(count_of(&s, GameEventType::Destroyed), 0);
            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
        }

        #[test]
        fn stops_once_it_is_on_the_field_the_hand_trigger_is_not_registered_from_a_unit_zone_s10_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": STRAAZA, "lane": 2 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));
            s.expect_in_zone(EATER, "field");

            s.attack(STRAAZA, GARY);

            // §10.3: a card on the field registers `triggers`, and this card declares none.
            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
        }

        #[test]
        fn r78_the_gain_travels_from_hand_onto_the_field_buffs_are_layer_4_on_the_instance() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 2 }] },
            }));

            s.attack(FELINORS, GARY);
            s.expect_stats(EATER, json!({ "attack": 3, "maxHealth": 3 }));

            // Back to p1's turn, then play the fattened Eater: entering the field resets nothing (R78
            // resets a card LEAVING the field), so it arrives at the stats it ate its way to.
            s.end_turn();
            s.play(EATER, json!({}));

            s.expect_in_zone(EATER, "field");
            s.expect_stats(EATER, json!({ "attack": 3, "maxHealth": 3 }));
        }
    }

    mod n89_corpse_eater_radiant {
        use super::*;

        #[test]
        fn gains_double_the_radiant_6_6_takes_twice_the_dying_unit_s_attack_and_max_health() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
            s.attack(FELINORS, GARY);

            // 6/6 + 2 × 1/1.
            s.expect_stats(EATER, json!({ "attack": 8, "maxHealth": 8 }));
        }

        #[test]
        fn r38_doubles_the_current_attack_and_max_health_from_either_side() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": {
                    "hand": [{ "def": EATER, "radiant": true }, FILLER],
                    "field": [{ "def": RENO, "damage": 5, "lane": 1 }],
                },
                "p2": { "hand": [FILLER], "field": [{ "def": STRAAZA, "lane": 1 }] },
            }));

            s.attack(STRAAZA, RENO);

            // 6/6 + 2 × 4/6.
            s.expect_stats(EATER, json!({ "attack": 14, "maxHealth": 18 }));
        }

        #[test]
        fn r11_the_radiant_face_excludes_tokens_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": {
                    "hand": [{ "def": EATER, "radiant": true }, FILLER],
                    "field": [{ "def": FELINOR_TOKEN, "lane": 1 }],
                },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));
            let token = s.card(FELINOR_TOKEN).clone();

            s.attack(FELINORS, &token);

            s.expect_in_zone(&token, "gone");
            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
        }

        #[test]
        fn stops_once_on_the_field_and_its_printed_divine_shield_is_the_catalog_s_not_a_grant() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER], "field": [{ "def": STRAAZA, "lane": 2 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));
            assert_eq!(
                kinds(&s.stats(EATER).keywords),
                vec![KeywordKind::Rush, KeywordKind::DivineShield]
            );
            assert!(s.card(EATER).granted_keywords.is_empty());

            s.attack(STRAAZA, GARY);

            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
        }
    }

    mod n89_corpse_eater_r219 {
        use super::*;

        #[test]
        fn r219_corpse_eater_gains_nothing_from_a_unit_suppressive_aura_starved_below_0_max_health_r38_s10_4() {
            crate::register_all();
            let mut g = scenario(json!({
                "p1": { "hand": [EATER, SUPPRESSIVE_AURA, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] }, // 1/1
            }));
            let eater = g
                .hand(Some(P1))
                .into_iter()
                .find(|card| card.def_id == EATER)
                .expect("Corpse Eater in hand");

            g.play(SUPPRESSIVE_AURA, json!({ "embiggen": true })); // paid 4, −2/−2: Gary is −1/−1 and dies at the state check (§4.5)
            g.expect_in_zone(GARY, "graveyard");
            let died = g
                .events()
                .iter()
                .find(|event| matches!(event, GameEvent::Destroyed { .. }))
                .cloned();
            assert!(matches_object(&js(&died), &json!({ "attack": 0, "maxHealth": -1 })));

            // "Whenever a unit dies, this GAINS its attack and max health": the unit had no attack and no
            // max health to give, so the Eater in hand is still its printed 2/2 — it does not shrink by the
            // −1 the aura dragged the dead unit's max health to.
            assert_eq!(js(&g.card(&eater).buffs), json!({ "attack": 0, "health": 0 }));
        }
    }
}
