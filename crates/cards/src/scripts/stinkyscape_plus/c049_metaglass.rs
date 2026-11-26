//! SPEC §8.12 S+ #49 Metaglass (§4.3, §6.1 Ranged; §6.3 Buff; R81, R783000, R1420, R3535000, R3547000,
//! R4783024): (1) Spell, Rare, Mindustrial, Mineral, a card of the Stinkyscape+ set.
//!   Base:    "Give a Unit Ranged and +{attack} Attack." — 1.
//!   Radiant: "Give a Unit Ranged and +{attack} Attack." — 5.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then `grant_keyword` Ranged
//! (R783000: the Unit takes no strike back from a defender without it) and a `buff` of attack alone, the
//! declared `attack` of the face (R386, R4783024).
//!
//! Issue #4783 (Stinkyscape+ Balance Changes VIII) gave the base face the attack too, 1, and made the
//! Radiant face's 5, from 3. Before it (`script_v0_6_13h`, and `script_before_help_aim` before #4844's
//! help aim, R767000) the base face gave Ranged
//! alone and `attack` was declared for the Radiant face only (`tunedOn`, R2261001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-049";

/// R81, R3547000: any Unit on either side, picked with the play.
/// R656: Ranged always benefits its holder, so a random cast that targets enemies aims this at friends.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl { aim: Some(TargetAim::Help), ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] })) }]
}

/// Ranged, and, when `buffs`, the declared attack: both faces today; through v0.6.13h the Radiant face
/// alone.
fn metaglass(buffs: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut effects = vec![grant_keyword(json_as(json!({
                "target": { "of": "chosen" },
                "keyword": { "kind": "Ranged" },
            })))];
            if buffs {
                effects.push(buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "attack": param(&*ctx, "attack"),
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: metaglass(true),
        radiant: metaglass(true),
    }
}

/// R767000: what it ran before issue #4783, the base face giving Ranged alone, with the help aim
/// (`script_before_help_aim` the same without it, for the versions before #4844).
pub fn script_v0_6_13h() -> CardScripts {
    CardScripts {
        base: metaglass(false),
        radiant: metaglass(true),
    }
}

/// R767000: until its help aim shipped (issue #4844), the target declared no aim; `KEPT_SCRIPTS` folds
/// those replays under it. Those versions also ran the faces before issue #4783, so the script built is
/// `script_v0_6_13h`'s with the old targets.
pub fn script_before_help_aim() -> CardScripts {
    let old = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))];
    let then = script_v0_6_13h();
    CardScripts {
        base: Script { targets: old.clone(), ..then.base },
        radiant: Script { targets: old, ..then.radiant },
    }
}

// S+ #49 Metaglass — SPEC §8.12 row S+ 49 (R81, R783000, R3547000, R656, R4783024).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const METAGLASS: &str = "stinkyscapeplus-049";
    const PALANTIR: &str = "classic-004"; // Palantir: steals the play and casts it at enemies (R831001).
    const MINE: &str = "stand-in (1) 3/3";
    const THEIRS: &str = "stand-in (1) 4/4";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const STRIKER: &str = "stand-in (1) 3/10";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": METAGLASS, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    /// A 4/4 of p1's after Metaglass of `radiant`'s face: its Ranged count and its stats.
    fn glassed(radiant: bool) -> (usize, i32, i32) {
        let mut s = board(radiant, json!([VANILLA]), json!([]));
        let unit = s.unit_at(P1, 1);
        s.play(METAGLASS, pick::targets(pick::instance(&unit)));
        let stats = s.stats(&unit);
        (keyword_count(&s, &unit, KeywordKind::Ranged), stats.attack, stats.max_health)
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = s.play_targets(METAGLASS);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn r4783024_ranged_and_plus_1_attack_and_no_health_change() {
            assert_eq!(glassed(false), (1, 4 + 1, 4));
        }

        #[test]
        fn the_unit_takes_no_strike_back_from_a_defender_without_ranged() {
            let mut s = board(false, json!([VANILLA]), json!([STRIKER]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(METAGLASS, pick::targets(pick::instance(&mine)));
            s.attack(&mine, &theirs);
            assert_eq!(s.card(&mine).damage, 0, "R783000");
            assert_eq!(s.card(&theirs).damage, 4 + 1, "the new attack");
        }

        #[test]
        fn an_enemy_unit_takes_ranged_too() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(METAGLASS, pick::targets(pick::instance(&theirs)));
            assert_eq!(keyword_count(&s, &theirs, KeywordKind::Ranged), 1);
            s.expect_stats(&theirs, json!({ "attack": 4 + 1, "health": 4 }));
        }

        #[test]
        fn ranged_once_on_a_unit_that_has_it() {
            let mut s = board(false, json!(["stand-in (1) 4/4 Ranged"]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Ranged), 1);
        }

        #[test]
        fn r4783024_attack_reads_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, METAGLASS, "attack"), 2);
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, pick::targets(pick::instance(&unit)));
            s.expect_stats(&unit, json!({ "attack": 4 + 2, "health": 4 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4783024_ranged_and_plus_5_attack_and_no_health_change() {
            assert_eq!(glassed(true), (1, 4 + 5, 4));
        }

        #[test]
        fn the_unit_takes_no_strike_back_and_hits_for_the_new_attack() {
            let mut s = board(true, json!([VANILLA]), json!([STRIKER]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(METAGLASS, pick::targets(pick::instance(&mine)));
            s.attack(&mine, &theirs);
            assert_eq!(s.card(&mine).damage, 0, "R783000");
            assert_eq!(s.card(&theirs).damage, 4 + 5);
        }

        #[test]
        fn r4783024_attack_reads_through_param() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, METAGLASS, "attack"), 6);
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, pick::targets(pick::instance(&unit)));
            s.expect_stats(&unit, json!({ "attack": 4 + 6, "health": 4 }));
        }
    }

    /// R767000: the script kept for the replays of v0.6.13h and before (issue #4783), under its v0.6.13h
    /// `attack`: 3, the Radiant face's alone.
    mod kept_v0_6_13h {
        use super::super::script_v0_6_13h;
        use super::*;

        fn kept<T>(run: impl FnOnce() -> T) -> T {
            crate::under_kept_as(
                METAGLASS,
                script_v0_6_13h(),
                |def| {
                    def.params = serde_json::from_value(json!([
                        { "key": "attack", "base": 3, "radiant": 3, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }
                    ]))
                    .expect("the v0.6.13h params");
                },
                run,
            )
        }

        #[test]
        fn r767000_the_base_face_gave_ranged_alone_and_the_radiant_face_plus_3() {
            kept(|| {
                assert_eq!(glassed(false), (1, 4, 4), "base: Ranged alone");
                assert_eq!(glassed(true), (1, 4 + 3, 4), "radiant: Ranged and +3");
            });
        }
    }

    /// A (1) Cost Spell made a (2) Cost one by a `costMod` of 1: Palantir steals and casts it (R831001).
    /// P1 holds the Spell and a Unit, P2 a Unit and a face-up Palantir: P1 plays it on their own
    /// Unit, Palantir steals it and casts it at enemies.
    fn stolen_rig() -> (Scenario, CardInstance, CardInstance) {
        let mut s = scenario(json!({
            "seed": "audit-aim",
            "p1": { "hand": [{ "def": METAGLASS, "costMod": 1 }], "field": [{ "def": MINE, "lane": 1 }],
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

    mod before_help_aim {
        use super::*;

        use crate::scripts::stinkyscape_plus::c049_metaglass::{ID, script_before_help_aim};

        /// R767000: a game from before the help aim gives the caster's enemy Ranged with a stolen cast.
        #[test]
        fn r767000_a_game_before_the_help_aim_gives_the_casters_enemy_ranged_with_a_stolen_cast() {
            crate::under_kept(ID, script_before_help_aim(), || {
                let (s, mine, theirs) = stolen_rig();
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == METAGLASS), "P2 stole and cast it");
                assert_eq!(keyword_count(&s, &mine, KeywordKind::Ranged), 1, "the caster's enemy takes Ranged");
                assert_eq!(keyword_count(&s, &theirs, KeywordKind::Ranged), 0, "the caster's own Unit takes none");
            });
        }
    }
}
