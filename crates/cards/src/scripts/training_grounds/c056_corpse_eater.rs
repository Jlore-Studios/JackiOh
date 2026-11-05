//! SPEC §8.11 Training Grounds #56 Legacy: Corpse Eater,
//! the Legacy card of Core #89 Corpse Eater (`core-089`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). Since issue #3136
//! its data differs from the Core card's: 5/5 with no keyword, Radiant 10/10 with Rush (it was 2/2 with
//! Rush and 4/4 with Rush and Divine Shield); the hand-growth script is unchanged. The Core
//! file's notes, as they stood:
//!
//! #89 Corpse Eater (SPEC §8.5, §6.2, §10.3, §10.4, R11, R38, R68, R78, R89).
//!
//! Base: "Rush. While in your hand: whenever a unit on either side dies, this gains its attack and
//! max health". Radiant: "Rush, Divine Shield; gains double". §8's Conventions: both keyword lists
//! are printed in `catalog.json` (§10.4 layer 1), so no script grants them and only the gain doubles.
//!
//! A hand-zone trigger: "while in your hand" and "stops once on the field" are the registry's
//! doing (§10.3, R68). It reads the event, not the instance: R78 resets an instance leaving the
//! field, and R89's `destroyed` event carries the attack and max health the layers computed at death
//! (R38's "current" numbers). A death is a unit going from the field to a graveyard (§4.5 step 1, §6.2);
//! a discard, burn or bounce emits another event. Units only, tokens excluded (R11); `def_of` is the layers' own catalog
//! read, a transient Fuse def included (R77). The gain is a layer-4 `buff`, kept on entering the field.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-056";

/// `factor` is the whole of the radiant text: 1 gains its stats, 2 gains double (§8 Conventions).
fn feed(factor: i32) -> TriggerDef {
    TriggerDef::new(
        "corpse-eater-feed",
        &[GameEventType::Destroyed],
        move |ctx, event| {
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
        },
    )
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

// #89 Corpse Eater (SPEC §8.5, BUILD M4-T4 row 89) (R11, R38).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EATER: &str = "traininggrounds-056"; // 5/5 → 10/10, cost 4 (2/2 → 4/4 until issue #3136)

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
        s.events()
            .iter()
            .filter(|event| event.event_type() == type_)
            .count()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(|keyword| keyword.kind()).collect()
    }

    mod n89_corpse_eater_base {
        use super::*;

        #[test]
        fn r89_in_hand_it_gains_an_ally_unit_s_attack_and_max_health_when_it_dies() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.expect_stats(EATER, json!({ "attack": 5, "maxHealth": 5 }));
            s.attack(FELINORS, GARY);

            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
            assert_eq!(s.card(EATER).zone.z(), ZoneName::Hand);
            s.expect_events(json!(["destroyed", "buffed"]));
        }

        #[test]
        fn on_either_side_an_enemy_unit_dying_feeds_it_too() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.attack(FELINORS, GARY);

            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
        }

        #[test]
        fn r38_counts_the_dying_unit_s_current_attack_and_max_health_not_its_remaining_health() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                // Reno is 4/6 sitting on 5 damage, so its current health is 1 and its max health is 6.
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": RENO, "damage": 5, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": STRAAZA, "lane": 1 }] },
            }));
            s.expect_stats(RENO, json!({ "attack": 4, "health": 1, "maxHealth": 6 }));

            s.attack(STRAAZA, RENO);

            // 5/5 + 4/6. Reading current health instead would have made it 9/6.
            s.expect_stats(EATER, json!({ "attack": 9, "maxHealth": 11 }));
        }

        #[test]
        fn r89_counts_the_layers_numbers_two_deaths_in_one_state_check_both_feed_it() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            let attacker = s.unit(P2, 1).expect("p2's Gary");
            let defender = s.unit(P1, 1).expect("p1's Gary");
            s.attack(&attacker, &defender);

            // Both 1/1s die in the same check, so the Eater eats twice: 5/5 + 1/1 + 1/1.
            s.expect_stats(EATER, json!({ "attack": 7, "maxHealth": 7 }));
        }

        #[test]
        fn r11_a_unit_token_never_reaches_a_graveyard_so_it_never_feeds_the_eater() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": FELINOR_TOKEN, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));
            let token = s.card(FELINOR_TOKEN).clone();

            s.attack(FELINORS, &token);

            s.expect_in_zone(&token, "gone");
            s.expect_stats(EATER, json!({ "attack": 5, "maxHealth": 5 }));
            assert_eq!(count_of(&s, GameEventType::Buffed), 0);
        }

        #[test]
        fn s8_5_a_card_reaching_a_graveyard_from_a_hand_never_died_so_a_burn_does_not_count_r4() {
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

            s.expect_events(json!(["drawn", "burned"]));
            assert_eq!(count_of(&s, GameEventType::Destroyed), 0);
            s.expect_stats(EATER, json!({ "attack": 5, "maxHealth": 5 }));
        }

        #[test]
        fn stops_once_it_is_on_the_field_the_hand_trigger_is_not_registered_from_a_unit_zone_s10_3() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": STRAAZA, "lane": 2 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));
            s.expect_in_zone(EATER, "field");

            s.attack(STRAAZA, GARY);

            // §10.3: a card on the field registers `triggers`, and this card declares none.
            s.expect_stats(EATER, json!({ "attack": 5, "maxHealth": 5 }));
        }

        #[test]
        fn r78_the_gain_travels_from_hand_onto_the_field_buffs_are_layer_4_on_the_instance() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [EATER, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 2 }] },
            }));

            s.attack(FELINORS, GARY);
            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));

            // Back to p1's turn, then play the fattened Eater: entering the field resets nothing (R78
            // resets a card LEAVING the field), so it arrives at the stats it ate its way to.
            s.end_turn();
            s.play(EATER, json!({}));

            s.expect_in_zone(EATER, "field");
            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
        }

        #[test]
        fn issue_3136_it_is_a_5_5_with_no_keyword_so_it_cannot_attack_the_turn_it_enters() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [EATER, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));

            s.expect_stats(EATER, json!({ "attack": 5, "health": 5, "maxHealth": 5 }));
            assert!(kinds(&s.stats(EATER).keywords).is_empty());
            // No Rush any more: summoning sick against a Unit as against the hero.
            s.expect_refused(|s| s.attack(EATER, GARY));
            s.expect_refused(|s| s.attack(EATER, "hero"));
        }
    }

    mod n89_corpse_eater_radiant {
        use super::*;

        #[test]
        fn gains_double_the_radiant_10_10_takes_twice_the_dying_unit_s_attack_and_max_health() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.expect_stats(EATER, json!({ "attack": 10, "maxHealth": 10 }));
            s.attack(FELINORS, GARY);

            // 10/10 + 2 × 1/1.
            s.expect_stats(EATER, json!({ "attack": 12, "maxHealth": 12 }));
        }

        #[test]
        fn r38_doubles_the_current_attack_and_max_health_from_either_side() {
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

            // 10/10 + 2 × 4/6.
            s.expect_stats(EATER, json!({ "attack": 18, "maxHealth": 22 }));
        }

        #[test]
        fn r11_the_radiant_face_excludes_tokens_too() {
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
            s.expect_stats(EATER, json!({ "attack": 10, "maxHealth": 10 }));
        }

        #[test]
        fn issue_3136_it_is_a_10_10_with_rush_so_it_attacks_a_unit_the_turn_it_enters_never_the_hero() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));

            s.expect_stats(EATER, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
            assert_eq!(kinds(&s.stats(EATER).keywords), vec![KeywordKind::Rush]);
            s.expect_refused(|s| s.attack(EATER, "hero"));
            s.attack(EATER, GARY);
            s.expect_in_zone(GARY, "graveyard");
        }

        #[test]
        fn stops_once_on_the_field_and_its_printed_rush_is_the_catalog_s_not_a_grant() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER], "field": [{ "def": STRAAZA, "lane": 2 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));
            assert_eq!(kinds(&s.stats(EATER).keywords), vec![KeywordKind::Rush]);
            assert!(s.card(EATER).granted_keywords.is_empty());

            s.attack(STRAAZA, GARY);

            s.expect_stats(EATER, json!({ "attack": 10, "maxHealth": 10 }));
        }
    }

    mod n89_corpse_eater_r219 {
        use super::*;

        #[test]
        fn r219_corpse_eater_gains_nothing_from_a_unit_suppressive_aura_starved_below_0_max_health_r38_s10_4()
        {
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
            assert!(matches_object(
                &js(&died),
                &json!({ "attack": 0, "maxHealth": -1 })
            ));

            // The unit had no attack and no max health to give, so the Eater does not shrink by the −1 the
            // aura dragged the dead unit's max health to.
            assert_eq!(js(&g.card(&eater).buffs), json!({ "attack": 0, "health": 0 }));
        }
    }

    /// R2195003, R3139004: the snapshot is its own copy, no longer core-089's play: core-089 has Brittle
    /// since #3139, and the Legacy card has none on either face (#3136).
    #[test]
    fn r2195003_the_legacy_card_is_its_own_copy_and_no_longer_plays_as_core_089() {
        crate::register_all();
        for radiant in [false, true] {
            let keywords = |id: &str| {
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }] } }));
                s.play(id, json!({ "zone": 1 }));
                s.stats(id)
                    .keywords
                    .iter()
                    .map(|keyword| keyword.kind())
                    .collect::<Vec<_>>()
            };
            assert!(
                keywords("core-089").contains(&KeywordKind::Brittle),
                "radiant {radiant}"
            );
            assert!(
                !keywords(super::ID).contains(&KeywordKind::Brittle),
                "radiant {radiant}"
            );
        }
    }
}
