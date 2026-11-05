//! #89 Corpse Eater (SPEC §8.5, §6.2, §10.3, §10.4, R11, R38, R68, R78, R89, R3139004).
//!
//! Base: "Rush, Brittle 1. While in your hand: whenever a unit on either side dies, this gains its attack and
//! max health. After this attacks a Unit, it may attack again". Radiant: "Rush, Divine Shield, Brittle 3;
//! gains double". §8's Conventions: both keyword lists are printed in `catalog.json` (§10.4 layer 1), so no
//! script grants them and only the gain doubles.
//!
//! "After this attacks a Unit, it may attack again" (since issue #3139, R3139004) is an `after_attack` hook,
//! Tmack Trav's (R2285001, R2521002): it runs only for an attacker that survived its combat, and when the
//! target was a Unit and no hero it is `may_attack_again` itself, a fresh exertion with `keepSick`, so on the
//! turn it arrives its Rush still allows a Unit and never the hero. There is no cap on the repeats: each
//! attack on a Unit it survives gives another. A printed Brittle starts on arrival and first ticks at its
//! controller's start of turn t + 2 or later, so Brittle 1 destroys it at its controller's next turn start
//! and Brittle 3 at the third. `script_v0_5_11o` keeps the Corpse Eater with neither, for replays (R767000).
//!
//! A hand-zone trigger: "while in your hand" and "stops once on the field" are the registry's doing
//! (§10.3, R68). It reads the event, not the instance (R78 resets an instance leaving the field): R89's
//! `destroyed` carries the attack and max health the layers computed at death (R38). A discard, burn or
//! bounce is no death (§4.5 step 1, §6.2). `def_of` includes a transient Fuse def (R77).

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

/// A hero target is `hero-<player>` in the combat's facts.
const HERO_TARGET: &str = "hero-";

/// "After this attacks a Unit, it may attack again": `keepSick`, as Tmack Trav's (R2521002). The hook runs
/// for a survivor only (R2285001); the check on `survived` and on the target is the card's own words.
fn attack_again() -> Hook {
    hook(|ctx| {
        let Some(combat) = after_attack_of(ctx) else {
            return Vec::new();
        };
        if !combat.survived || combat.target_id.starts_with(HERO_TARGET) {
            return Vec::new();
        }
        vec![may_attack_again(json_as(json!({ "target": { "of": "self" }, "keepSick": true })))]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        hand_triggers: vec![feed(1)],
        after_attack: Some(attack_again()),
        ..Script::default()
    };
    let radiant = Script {
        hand_triggers: vec![feed(2)],
        after_attack: Some(attack_again()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

/// R767000: the script v0.5.11o and the versions before it ran, the hand growth alone with no repeat
/// attack; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
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

    const EATER: &str = "core-089"; // 2/2 → 4/4 (6/6 before patch v0.3.10), cost 4

    const FENDER: &str = "core-001"; // 0/7: a Unit to attack that hits back for nothing.
    const MENACE: &str = "core-019"; // 9/9: a Unit that kills the Eater.

    // Inert fixtures: each unit's only text is a Cry, which `field` never fires.
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

        /// Issue #3139: both faces gain Brittle, 1 on the base and 3 on the Radiant face.
        #[test]
        fn the_keyword_cells_list_rush_and_brittle_and_the_radiant_one_divine_shield_and_brittle_3() {
            crate::register_all();
            let def = crate::card_def(ID);
            assert_eq!(kinds(&def.base.keywords), vec![KeywordKind::Rush, KeywordKind::Brittle]);
            assert_eq!(
                kinds(&def.radiant.keywords),
                vec![KeywordKind::Rush, KeywordKind::DivineShield, KeywordKind::Brittle]
            );
            let brittle = |keywords: &[Keyword]| keywords.iter().find(|keyword| keyword.kind() == KeywordKind::Brittle).and_then(Keyword::n);
            assert_eq!(brittle(&def.base.keywords), Some(1));
            assert_eq!(brittle(&def.radiant.keywords), Some(3));
        }
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

            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
            s.attack(FELINORS, GARY);

            s.expect_stats(EATER, json!({ "attack": 3, "maxHealth": 3 }));
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

            s.expect_stats(EATER, json!({ "attack": 3, "maxHealth": 3 }));
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

            // 2/2 + 4/6. Reading current health instead would have made it 6/3.
            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 8 }));
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

            // Both 1/1s die in the same check, so the Eater eats twice: 2/2 + 1/1 + 1/1.
            s.expect_stats(EATER, json!({ "attack": 4, "maxHealth": 4 }));
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
            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
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
            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
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
            s.expect_stats(EATER, json!({ "attack": 2, "maxHealth": 2 }));
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
        fn gains_double_the_radiant_4_4_takes_twice_the_dying_unit_s_attack_and_max_health() {
            let mut s = scenario(json!({
                "seed": SEED,
                "active": "p2",
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER], "field": [{ "def": GARY, "lane": 1 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": FELINORS, "lane": 1 }] },
            }));

            s.expect_stats(EATER, json!({ "attack": 4, "maxHealth": 4 }));
            s.attack(FELINORS, GARY);

            // 4/4 + 2 × 1/1.
            s.expect_stats(EATER, json!({ "attack": 6, "maxHealth": 6 }));
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

            // 4/4 + 2 × 4/6.
            s.expect_stats(EATER, json!({ "attack": 12, "maxHealth": 16 }));
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
            s.expect_stats(EATER, json!({ "attack": 4, "maxHealth": 4 }));
        }

        #[test]
        fn stops_once_on_the_field_and_its_printed_divine_shield_is_the_catalog_s_not_a_grant() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [{ "def": EATER, "radiant": true }, FILLER], "field": [{ "def": STRAAZA, "lane": 2 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": GARY, "lane": 1 }] },
            }));

            s.play(EATER, json!({}));
            assert_eq!(
                kinds(&s.stats(EATER).keywords),
                vec![KeywordKind::Rush, KeywordKind::DivineShield, KeywordKind::Brittle]
            );
            assert!(s.card(EATER).granted_keywords.is_empty());

            s.attack(STRAAZA, GARY);

            s.expect_stats(EATER, json!({ "attack": 4, "maxHealth": 4 }));
        }
    }

    /// An Eater on p1's field (`radiant`'s face) facing `enemy` Units in p2's lanes 1.., p2's turn waiting.
    fn fielded(radiant: bool, enemy: &[&str]) -> Scenario {
        let enemy: Vec<Value> = enemy.iter().enumerate().map(|(at, def)| json!({ "def": def, "lane": at + 1 })).collect();
        scenario(json!({
            "seed": SEED,
            "p1": { "hand": [FILLER], "field": [{ "def": EATER, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": enemy, "library": crate::filler(4) },
        }))
    }

    /// p1 holds an Eater to play this turn, p2 has `enemy` Units in lanes 1...
    fn in_hand(radiant: bool, enemy: &[&str]) -> Scenario {
        let enemy: Vec<Value> = enemy.iter().enumerate().map(|(at, def)| json!({ "def": def, "lane": at + 1 })).collect();
        scenario(json!({
            "seed": SEED,
            "p1": { "hand": [{ "def": EATER, "radiant": radiant }, FILLER], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": enemy, "library": crate::filler(4) },
        }))
    }

    /// Whether `legal_actions` offers the Eater an attack on `target`.
    fn offers(s: &Scenario, target: &str) -> bool {
        let eater = s.card(EATER).id.clone();
        legal_actions(s.state(), P1).iter().any(|action| {
            matches!(
                action,
                ActionBody::Attack { attacker_id, target_id, .. } if *attacker_id == eater && target_id == target
            )
        })
    }

    mod n89_corpse_eater_attacks_again_r3139004 {
        use super::*;

        /// Issue #3139, R3139004: an attack on a Unit it survives gives another attack, with no cap: the Eater
        /// hits the 0/7 fender, which hits back for nothing, as often as it likes while the fender lives.
        #[test]
        fn r3139004_an_attack_on_a_unit_it_survives_frees_it_to_attack_again_with_no_cap() {
            for radiant in [false, true] {
                let mut s = fielded(radiant, &[FENDER, FENDER, FENDER]);
                let first = s.unit_at(P2, 1);
                let second = s.unit_at(P2, 2);
                let third = s.unit_at(P2, 3);

                s.attack(EATER, &first);
                assert!(!s.card(EATER).exertion.attacked, "radiant {radiant}: a fresh exertion after the first");
                s.attack(EATER, &second);
                assert!(!s.card(EATER).exertion.attacked, "radiant {radiant}: and after the second");
                s.attack(EATER, &third);
                assert!(!s.card(EATER).exertion.attacked, "radiant {radiant}: and after the third");
                // The same Unit may be attacked again too, and nothing ever ends the chain but a death or the hero.
                s.attack(EATER, &first);
                assert!(!s.card(EATER).exertion.attacked, "radiant {radiant}");
            }
        }

        #[test]
        fn r3139004_a_kill_also_frees_it_to_attack_again() {
            let mut s = fielded(true, &[GARY, GARY]);
            let (first, second) = (s.unit_at(P2, 1), s.unit_at(P2, 2));

            s.attack(EATER, &first);
            s.expect_in_zone(&first, "graveyard");
            assert!(!s.card(EATER).exertion.attacked);
            s.attack(EATER, &second);

            s.expect_in_zone(&second, "graveyard");
            assert!(!s.card(EATER).exertion.attacked);
        }

        /// The hero is no Unit: an attack on it ends the Eater's attacks for the turn.
        #[test]
        fn r3139004_an_attack_on_the_hero_gives_no_other() {
            for radiant in [false, true] {
                let mut s = fielded(radiant, &[FENDER]);

                s.attack(EATER, "hero");

                assert!(s.card(EATER).exertion.attacked, "radiant {radiant}");
                let fender = s.unit_at(P2, 1);
                assert!(!offers(&s, &fender.id), "radiant {radiant}");
                s.expect_refused(|s| s.attack(EATER, &fender));
            }
        }

        /// A dead attacker runs no `after_attack` (R2285001): the Eater that dies in its combat attacks no more.
        #[test]
        fn r3139004_an_eater_that_dies_in_the_combat_attacks_no_more() {
            let mut s = fielded(false, &[MENACE, FENDER]);

            s.attack(EATER, s.unit_at(P2, 1));

            s.expect_in_zone(EATER, "graveyard");
        }

        /// The turn it arrives its Rush still allows a Unit only (§6.1): the repeat keeps its summoning sickness.
        #[test]
        fn r3139004_r2521002_on_its_summon_turn_the_extra_attack_hits_a_unit_and_never_the_hero() {
            for radiant in [false, true] {
                let mut s = in_hand(radiant, &[FENDER, FENDER]);
                s.play(EATER, json!({ "zone": 1 }));
                let (first, second) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
                assert!(!offers(&s, "hero-p2"), "radiant {radiant}: Rush, a Unit and not the hero");

                s.attack(EATER, &first);
                assert!(!s.card(EATER).exertion.attacked, "radiant {radiant}");
                assert!(!offers(&s, "hero-p2"), "radiant {radiant}: still summoning sick");
                s.expect_refused_with(|s| s.attack(EATER, "hero"), "Rush cannot hit the hero");
                s.attack(EATER, &second);
                assert!(!s.card(EATER).exertion.attacked, "radiant {radiant}");
            }
        }
    }

    mod n89_corpse_eater_brittle_r3139004 {
        use super::*;

        /// Issue #3139: a printed Brittle starts on arrival and first ticks at its controller's start of turn
        /// t + 2 or later, so Brittle 1 destroys it at its controller's next turn start (R638).
        #[test]
        fn r3139004_brittle_1_destroys_the_base_face_at_its_controllers_next_turn_start() {
            let mut s = in_hand(false, &[FENDER]);
            s.play(EATER, json!({ "zone": 1 }));
            s.expect_in_zone(EATER, "field");
            s.end_turn();
            s.expect_in_zone(EATER, "field");

            s.end_turn();

            assert_eq!(s.state().active, P1);
            s.expect_in_zone(EATER, "graveyard");
        }

        #[test]
        fn r3139004_brittle_3_destroys_the_radiant_face_at_its_controllers_third_turn_start() {
            let mut s = in_hand(true, &[FENDER]);
            s.play(EATER, json!({ "zone": 1 }));
            for _ in 0..2 {
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().active, P1);
                s.expect_in_zone(EATER, "field");
            }

            s.end_turn();
            s.end_turn();

            s.expect_in_zone(EATER, "graveyard");
        }
    }

    /// R767000: through v0.5.11o it had neither Brittle nor a repeat attack; the kept script is the hand growth
    /// alone, and the catalog it folds under (v0.5.11o's) prints no Brittle.
    #[test]
    fn r767000_the_script_kept_for_v0_5_11o_grows_in_hand_and_never_attacks_again() {
        let kept = script_v0_5_11o();
        assert!(kept.base.after_attack.is_none() && kept.radiant.after_attack.is_none());
        assert_eq!(kept.base.hand_triggers.len(), 1);
        assert_eq!(kept.radiant.hand_triggers.len(), 1);
        let now = script();
        assert!(now.base.after_attack.is_some() && now.radiant.after_attack.is_some());
        let old = |def: &mut CardDef| {
            def.base.keywords.retain(|keyword| keyword.kind() != KeywordKind::Brittle);
            def.radiant.keywords.retain(|keyword| keyword.kind() != KeywordKind::Brittle);
        };
        crate::under_kept_as(ID, kept, old, || {
            let mut s = fielded(false, &[FENDER]);
            s.attack(EATER, s.unit_at(P2, 1));
            assert!(s.card(EATER).exertion.attacked, "no second attack");
        });
    }

    mod n89_corpse_eater_r219 {
        use super::*;

        #[test]
        fn r219_corpse_eater_gains_nothing_from_a_unit_suppressive_aura_starved_below_0_max_health_r38_s10_4() {
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

            // The unit had no attack and no max health to give, so the Eater does not shrink by the −1 the
            // aura dragged the dead unit's max health to.
            assert_eq!(js(&g.card(&eater).buffs), json!({ "attack": 0, "health": 0 }));
        }
    }
}
