//! SPEC §8.12 S+ #47 Plastanium (§6.1 Divine Shield; §6.3 Buff; R81, R1420, R3535000, R3547000): (1)
//! Spell, Epic, Mindustrial, Mineral, a card of the Stinkyscape+ set.
//!   Base:    "Give a Unit +{health} health and Divine Shield." — 2.
//!   Radiant: "Give a Unit +{health} health and Divine Shield." — 6.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then in the text's order a
//! `buff` of health alone (current and maximum, as Training Grounds' Fortify's) and `grant_keyword`
//! Divine Shield, which brings a spent shield back as Fortify's Radiant face does. The two faces
//! differ by the number alone, `health`, 2 on the base face and 6 on the Radiant one.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-047";

/// R81, R3547000: any Unit on either side, picked with the play.
/// R656: Health and Divine Shield help, so a random cast that targets enemies aims this at friends.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl { aim: Some(TargetAim::Help), ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] })) }]
}

/// One script for both faces: `health` is declared per face.
fn plastanium() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "health": param(&*ctx, "health") }))),
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Divine Shield" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = plastanium();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: through v0.6.13e the target declared no aim (issue #4844); `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_13e() -> CardScripts {
    let old = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))];
    let now = script();
    CardScripts {
        base: Script { targets: old.clone(), ..now.base },
        radiant: Script { targets: old, ..now.radiant },
    }
}

// S+ #47 Plastanium — SPEC §8.12 row S+ 47 (R81, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PLASTANIUM: &str = "stinkyscapeplus-047";
    const PALANTIR: &str = "classic-004"; // Palantir: steals a (2) Cost Spell and casts it at enemies (R831001).
    const MINE: &str = "stand-in (1) 3/3";
    const THEIRS: &str = "stand-in (1) 4/4";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const SHIELDED: &str = "stand-in (1) 4/4 Divine Shield";
    const WALL: &str = "stand-in (1) 1/30";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": PLASTANIUM, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = s.play_targets(PLASTANIUM);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn max_health_up_2_keeping_the_damage_and_a_divine_shield_and_no_attack_change() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1, "damage": 1 }]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(PLASTANIUM, pick::targets(pick::instance(&unit)));
            s.expect_stats(&unit, json!({ "attack": 4, "health": 5 }));
            assert_eq!(s.stats(&unit).max_health, 6);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
        }

        #[test]
        fn an_enemy_unit_takes_both() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(PLASTANIUM, pick::targets(pick::instance(&theirs)));
            assert_eq!(s.stats(&theirs).max_health, 6);
            assert_eq!(keyword_count(&s, &theirs, KeywordKind::DivineShield), 1);
        }

        #[test]
        fn a_spent_shield_comes_back() {
            let mut s = board(false, json!([SHIELDED]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.card_mut(&unit).divine_shield_spent = Some(true);
            s.play(PLASTANIUM, pick::targets(pick::instance(&unit)));
            assert_ne!(s.card(&unit).divine_shield_spent, Some(true));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
        }

        #[test]
        fn health_reads_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, PLASTANIUM, "health"), 3);
            let unit = s.unit_at(P1, 1);
            s.play(PLASTANIUM, pick::targets(pick::instance(&unit)));
            assert_eq!(s.stats(&unit).max_health, 7);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn max_health_up_6_and_a_divine_shield() {
            let mut s = board(true, json!([{ "def": VANILLA, "lane": 1, "damage": 1 }]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(PLASTANIUM, pick::targets(pick::instance(&unit)));
            s.expect_stats(&unit, json!({ "attack": 4, "health": 9 }));
            assert_eq!(s.stats(&unit).max_health, 10);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
        }

        #[test]
        fn the_shield_eats_the_first_hit() {
            let mut s = board(true, json!([VANILLA]), json!([WALL]));
            let (unit, wall) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(PLASTANIUM, pick::targets(pick::instance(&unit)));
            s.attack(&unit, &wall);
            assert_eq!(s.card(&unit).damage, 0, "the wall's 1 attack is taken by the shield");
            assert_eq!(s.card(&unit).divine_shield_spent, Some(true));
            assert_eq!(s.card(&wall).damage, 4);
        }

        #[test]
        fn health_reads_through_param_on_its_own_face() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, PLASTANIUM, "health"), 7);
            let unit = s.unit_at(P1, 1);
            s.play(PLASTANIUM, pick::targets(pick::instance(&unit)));
            assert_eq!(s.stats(&unit).max_health, 11);
        }
    }

    /// A (1) Cost Spell made a (2) Cost one by a `costMod` of 1: Palantir steals and casts it (R831001).
    /// P1 holds the Spell and a Unit, P2 a Unit and a face-up Palantir: P1 plays it on their own
    /// Unit, Palantir steals it and casts it at enemies.
    fn stolen_rig() -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            "seed": "audit-aim",
            "p1": { "hand": [{ "def": PLASTANIUM, "costMod": 1 }], "field": [{ "def": MINE, "lane": 1 }],
                    "library": [FILLER, FILLER, FILLER], "mana": 10 },
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
    fn r656_the_target_aims_help() {
        let scripts = super::script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                serde_json::to_value(&face.targets).unwrap(),
                json!([{
                    "kind": "target",
                    "min": 1,
                    "max": 1,
                    "aim": "help",
                    "filter": { "side": "any", "of": ["unit"] },
                }]),
            );
        }
    }

    mod v0_6_13e {
        use super::*;

        use crate::scripts::stinkyscape_plus::c047_plastanium::{ID, script_v0_6_13e};

        /// R767000: a game on v0.6.13e, before the aim, plastaniums the caster's enemy with a stolen cast.
        #[test]
        fn r767000_a_game_on_v0_6_13e_plastaniums_the_casters_enemy_with_a_stolen_cast() {
            crate::under_kept(ID, script_v0_6_13e(), || {
                let (s, mine, theirs) = stolen_rig();
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == PLASTANIUM), "P2 stole and cast it");
                assert_eq!(s.stats(&mine).max_health, 5, "the caster's enemy takes the +2 Health");
                assert_eq!(keyword_count(&s, &mine, KeywordKind::DivineShield), 1);
                assert_eq!(s.stats(&theirs).max_health, 4, "the caster's own Unit takes no buff");
                assert_eq!(keyword_count(&s, &theirs, KeywordKind::DivineShield), 0);
            });
        }
    }
}
