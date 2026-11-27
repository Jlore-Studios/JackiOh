//! TG #12 Fortify (SPEC §8.11 row TG 12; §6.1 Armor X, Taunt, Divine Shield, R78, R90, R347): (1)
//! Spell, Common, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Give a Unit +{health} Health, +{armor} Armor and Taunt." — 5, 1 (3 until issue #3951).
//!   Radiant: "… +{health} Health, +{armor} Armor, Taunt and Divine Shield." — 10, 2 (6 until issue #3951).
//! Targets any Unit (R90; S #10.3); grants Armor n (§6.1, R78) and Taunt (R347 Indestructible ignores; S #15). Radiant grants Divine Shield.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-012";

fn fortify(radiant: bool) -> Script {
    Script {
        // R656: a buff helps, so a random cast that targets enemies aims this at friends.
        targets: vec![TargetDecl { aim: Some(TargetAim::Help), ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] })) }],
        cry: Some(hook(move |ctx| {
            let mut effects = vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "health": param(&*ctx, "health") }))),
                grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "Armor", "n": param(&*ctx, "armor") },
                }))),
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Taunt" } }))),
            ];
            if radiant {
                effects.push(grant_keyword(json_as(json!({
                    "target": { "of": "chosen" },
                    "keyword": { "kind": "Divine Shield" },
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fortify(false),
        radiant: fortify(true),
    }
}

/// R767000: until its help aim shipped, the target declared no aim, so a stolen or random cast aimed it at
/// enemies (issue #4844); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_before_help_aim() -> CardScripts {
    let old = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))];
    let now = script();
    CardScripts {
        base: Script { targets: old.clone(), ..now.base },
        radiant: Script { targets: old, ..now.radiant },
    }
}

// TG #12 Fortify — SPEC §8.11 row TG 12, BUILD M13 row TG 12.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FORTIFY: &str = "traininggrounds-012";
    const PALANTIR: &str = "classic-004"; // Palantir: steals the play and casts it at enemies (R831001).
    const MINE: &str = "stand-in (1) 3/3";
    const THEIRS: &str = "stand-in (1) 4/4";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const MOON: &str = "traininggrounds-007"; // Chosen by the Moon: Taunt, Armor 3 (once P3 builds it).
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7 R14/14 Armor 7, Reborn";
    const MENACE: &str = "core-019"; // Midrange Menace: Taunt
    const ROCK: &str = "core-066"; // The Rock: Indestructible
    const SHIELDED: &str = "core-056"; // Jilliax: Rush, Taunt, Lifesteal, Divine Shield

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FORTIFY, "radiant": radiant }], "field": mine, "mana": 10, "library": filler(4) },
            "p2": { "hand": filler(1), "field": theirs, "library": filler(4) },
        }))
    }

    /// A (1) Cost Spell made a (2) Cost one by a `costMod` of 1: Palantir steals and casts it (R831001).
    fn dear(def: &str) -> Value {
        json!({ "def": def, "costMod": 1 })
    }

    /// P1 holds the Spell and a Unit, P2 a Unit and a face-up Palantir: P1 plays it on their own
    /// Unit, Palantir steals it and casts it at enemies.
    fn stolen_rig(spell: Value) -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            "seed": "audit-aim",
            "p1": { "hand": [spell], "field": [{ "def": MINE, "lane": 1 }], "library": [FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "field": [{ "def": THEIRS, "lane": 1 }],
                    "backrow": [{ "def": PALANTIR, "faceUp": true }], "library": [FILLER, FILLER, FILLER] },
        }));
        let mine = s.unit_at(P1, 1);
        let theirs = s.unit_at(P2, 1);
        let cast = s.hand(P1)[0].id.clone();
        s.play(cast.as_str(), pick::targets(pick::instance(&mine)));
        (s, mine, theirs)
    }

    #[test]
    fn r656_stolen_fortify_lands_on_its_casters_unit() {
        let (s, mine, theirs) = stolen_rig(dear(FORTIFY));
        assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FORTIFY), "P2 stole and cast it");
        assert_eq!(s.stats(&theirs).max_health, 9, "the caster's own Unit takes the +5 Health");
        assert_eq!(s.stats(&mine).max_health, 3, "the caster's enemy takes no buff");
    }

    fn armor(s: &Scenario, card: &CardInstance) -> i32 {
        s.stats(card).armor
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    mod base {
        use super::*;

        #[test]
        fn it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = s.play_targets(FORTIFY);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn the_unit_gains_five_health_current_and_max() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1, "damage": 1 }]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            s.expect_stats(&unit, json!({ "attack": 4, "health": 8 }));
            assert_eq!(s.stats(&unit).max_health, 9);
        }

        #[test]
        fn its_armor_rises_by_one_on_top_of_its_printed_armor() {
            let mut s = board(false, json!([ARMORED, VANILLA]), json!([]));
            let (armored, vanilla) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            s.play(FORTIFY, pick::targets(pick::instance(&armored)));
            assert_eq!(armor(&s, &armored), 8);
            assert_eq!(armor(&s, &vanilla), 0);
        }

        #[test]
        fn it_gains_taunt() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Taunt), 1);
        }

        #[test]
        fn taunt_once_on_a_unit_that_has_it() {
            let mut s = board(false, json!([MENACE]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Taunt), 1);
        }

        /// R347, R5113000: an Indestructible Unit is Immutable (R4353000), so it may be aimed at and
        /// takes none of it: no Health, no Armor and, as ever, no Taunt.
        #[test]
        fn r5113000_never_taunt_on_an_indestructible_unit_and_none_of_the_rest() {
            let mut s = board(false, json!([ROCK]), json!([]));
            let rock = s.unit_at(P1, 1);
            let health = s.stats(&rock).max_health;
            s.play(FORTIFY, pick::targets(pick::instance(&rock)));
            assert_eq!(keyword_count(&s, &rock, KeywordKind::Taunt), 0, "R347");
            assert_eq!(armor(&s, &rock), 0);
            assert_eq!(s.stats(&rock).max_health, health);
        }

        #[test]
        fn an_enemy_unit_may_be_fortified() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&theirs)));
            assert_eq!(armor(&s, &theirs), 1);
            assert_eq!(s.stats(&theirs).max_health, 9);
        }

        #[test]
        fn both_numbers_read_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, FORTIFY, "health"), 6);
            assert_eq!(upgrade_number(&mut s, FORTIFY, "armor"), 2);
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            assert_eq!(s.stats(&unit).max_health, 10);
            assert_eq!(armor(&s, &unit), 2);
        }

        #[test]
        fn no_divine_shield_on_the_base_face() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 0);
        }

        #[test]
        fn a_moon_s_printed_armor_takes_the_one_on_top() {
            // Chosen by the Moon prints Armor 3 (§8.11 row TG 7); read off the catalog, so this holds
            // whether or not P3 has written its script yet.
            let mut s = board(false, json!([MOON]), json!([]));
            let moon = s.unit_at(P1, 1);
            let printed = armor(&s, &moon);
            s.play(FORTIFY, pick::targets(pick::instance(&moon)));
            assert_eq!(armor(&s, &moon), printed + 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn ten_health_two_armor_and_taunt() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            assert_eq!(s.stats(&unit).max_health, 14);
            assert_eq!(armor(&s, &unit), 2);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Taunt), 1);
        }

        #[test]
        fn divine_shield_a_spent_one_coming_back() {
            let mut s = board(true, json!([SHIELDED]), json!([]));
            let shielded = s.unit_at(P1, 1);
            s.card_mut(&shielded).divine_shield_spent = Some(true);
            s.play(FORTIFY, pick::targets(pick::instance(&shielded)));
            assert_ne!(s.card(&shielded).divine_shield_spent, Some(true));
            assert!(s.stats(&shielded).keywords.iter().any(|k| k.kind() == KeywordKind::DivineShield));
        }

        #[test]
        fn divine_shield_on_a_unit_without_one() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FORTIFY, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
        }
    }

    mod before_help_aim {
        use super::*;

        use crate::scripts::training_grounds::c012_fortify::{ID, script_before_help_aim};

        /// R767000: a game from before the help aim lands a stolen Fortify on the caster's enemy.
        #[test]
        fn r767000_a_game_before_the_help_aim_lands_a_stolen_fortify_on_the_casters_enemy() {
            crate::under_kept(ID, script_before_help_aim(), || {
                let (s, mine, theirs) = stolen_rig(dear(FORTIFY));
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == FORTIFY), "P2 stole and cast it");
                assert_eq!(s.stats(&mine).max_health, 8, "the caster's enemy takes the +5 Health");
                assert_eq!(s.stats(&theirs).max_health, 4, "the caster's own Unit takes no buff");
            });
        }
    }
}
