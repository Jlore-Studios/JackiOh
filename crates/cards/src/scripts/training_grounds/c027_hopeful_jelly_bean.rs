//! TG #27 Hopeful Jelly Bean (SPEC §8.11 row TG 27; §6.3 Make Radiant, R22, R74, R81, R97, R177,
//! R2198003): (4) Spell, Jelly Bean, Rare, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Make a permanent Radiant."
//!   Radiant: "Cleave / Make a permanent Radiant."
//!
//! Engine: Core #34's target (any permanent on either side's field, R81) and Make Radiant (`set_radiant`,
//! R74): the flag only, so damage and buffs are kept, no Cry fires again (R22), and a face-down card's
//! change reaches its opponent as a change alone (R97, R177). A Radiant pick is allowed, so no pick check
//! leaks a face-down card's face. Radiant's Cleave (R2198003): the permanents beside the pick in its row
//! and side (`adjacent_permanents_to`, §3.1), a Ticketed lying there among them (R4665000), read as the
//! clause begins; a neighbour Immune to Spells is passed by.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-027";

/// R656: making Radiant helps, so a random cast that targets enemies aims this at friends.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl { aim: Some(TargetAim::Help), ..TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] })) }]
}

fn chosen() -> TargetSpec {
    json_as(json!({ "of": "chosen" }))
}

/// R767000: until its help aim shipped, the target declared no aim, so a stolen or random cast aimed it at
/// enemies (issue #4844); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_before_help_aim() -> CardScripts {
    let old = vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))];
    let now = script();
    CardScripts {
        base: Script { targets: old.clone(), ..now.base },
        radiant: Script { targets: old, ..now.radiant },
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: targets(),
            cry: Some(hook(|_ctx| vec![set_radiant(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        },
        radiant: Script {
            targets: targets(),
            cry: Some(hook(|ctx| {
                // R2198003: the neighbours as the pick stands now, then the pick.
                let mut effects: Vec<Effect> = adjacent_permanents_to(&*ctx, &chosen(), &BoardScope::default())
                    .iter()
                    .map(|card| set_radiant(json_as(json!({ "instanceId": card.id }))))
                    .collect();
                effects.push(set_radiant(json_as(json!({ "target": { "of": "chosen" } }))));
                effects
            })),
            ..Script::default()
        },
    }
}

// TG #27 Hopeful Jelly Bean — SPEC §8.11 row TG 27, BUILD M13 row TG 27.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BEAN: &str = "traininggrounds-027";
    const PALANTIR: &str = "classic-004"; // Palantir: steals the play and casts it at enemies (R831001).
    const MINE: &str = "stand-in (1) 3/3";
    const THEIRS: &str = "stand-in (1) 4/4";
    const FILLER: &str = "core-005";
    const FELINORS: &str = "core-012"; // Duplicating Felinors: Cry: summon a copy of this
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const WEAPONS: &str = "core-014"; // Jlockeed's Weapons: your Units have +1 Attack (Radiant +3).
    const TRAP: &str = "core-041"; // Sheepish: a Trap
    const MEGABASE: &str = "stinkyscape-116"; // Megabase: Immune to Spells.

    fn board(radiant: bool, mine: Value, my_backrow: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": BEAN, "radiant": radiant }],
                "field": mine,
                "backrow": my_backrow,
                "mana": 10,
                "library": filler(4),
            },
            "p2": { "hand": filler(1), "field": theirs, "library": filler(4) },
        }))
    }

    fn felinors(s: &Scenario, player: PlayerId) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|card| card.def_id == FELINORS)
            .count()
    }

    /// A (4) Cost Spell made a (2) Cost one by a `costMod` of -2: Palantir steals and casts it (R831001).
    fn cheap(def: &str) -> Value {
        json!({ "def": def, "costMod": -2 })
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
    fn r656_stolen_bean_radiants_its_casters_permanent() {
        let (s, mine, theirs) = stolen_rig(cheap(BEAN));
        assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == BEAN), "P2 stole and cast it");
        assert!(s.card(&theirs).radiant, "the caster's own Unit becomes Radiant");
        assert!(!s.card(&mine).radiant, "the caster's enemy stays as it was");
    }

    mod base {
        use super::*;

        #[test]
        fn a_base_unit_picked_becomes_radiant_its_damage_and_buffs_kept() {
            let mut s = board(false, json!([{ "def": FELINORS, "lane": 1, "damage": 1 }]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.card_mut(&unit).buffs = AttackHealth { attack: 1, health: 1 };
            s.play(BEAN, pick::targets(pick::instance(&unit)));
            assert!(s.card(&unit).radiant);
            // 6/8 Radiant, +1/+1 kept, 1 damage kept.
            s.expect_stats(&unit, json!({ "attack": 7, "health": 8 }));
            assert_eq!(s.stats(&unit).max_health, 9);
        }

        #[test]
        fn its_cry_does_not_fire_again() {
            let mut s = board(false, json!([FELINORS]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(BEAN, pick::targets(pick::instance(&unit)));
            assert!(s.card(&unit).radiant);
            assert_eq!(felinors(&s, P1), 1, "R22: no copy summoned");
        }

        #[test]
        fn a_field_spell_picked_runs_its_radiant_text_from_then_on() {
            let mut s = board(false, json!([VANILLA]), json!([WEAPONS]), json!([]));
            let unit = s.unit_at(P1, 1);
            let weapons = s.backrow(P1, 1).expect("the Field Spell");
            assert_eq!(s.stats(&unit).attack, 5);
            s.play(BEAN, pick::targets(pick::instance(&weapons)));
            assert!(s.card(&weapons).radiant);
            assert_eq!(s.stats(&unit).attack, 7, "+3 now");
        }

        #[test]
        fn a_face_down_trap_of_yours_turns_radiant_its_opponent_seeing_only_a_change() {
            let mut s = board(false, json!([]), json!([{ "def": TRAP, "faceUp": false }]), json!([]));
            let trap = s.backrow(P1, 1).expect("the set Trap");
            s.play(BEAN, pick::targets(pick::instance(&trap)));
            assert!(s.card(&trap).radiant);
            assert_ne!(s.card(&trap).face_up, Some(true), "still hidden");
            let seen = serde_json::to_string(&s.view(P2)).unwrap();
            assert!(!seen.contains(TRAP), "the opponent never sees what it is (R97, R177)");
        }

        #[test]
        fn an_enemy_permanent_may_be_picked() {
            let mut s = board(false, json!([]), json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(BEAN, pick::targets(pick::instance(&theirs)));
            assert!(s.card(&theirs).radiant);
            s.expect_stats(&theirs, json!({ "attack": 12, "health": 12 }));
        }

        #[test]
        fn an_already_radiant_pick_nothing() {
            let mut s = board(false, json!([{ "def": FELINORS, "radiant": true, "lane": 1 }]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(BEAN, pick::targets(pick::instance(&unit)));
            assert!(s.card(&unit).radiant);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 8 }));
            assert_eq!(felinors(&s, P1), 1);
        }

        #[test]
        fn the_base_face_makes_the_pick_alone_radiant() {
            let mut s = board(false, json!([VANILLA, VANILLA, VANILLA]), json!([]), json!([]));
            let (left, pick, right) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P1, 3));
            s.play(BEAN, pick::targets(pick::instance(&pick)));
            assert!(s.card(&pick).radiant);
            assert!(!s.card(&left).radiant && !s.card(&right).radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2198003_the_permanents_beside_the_pick_in_its_row_and_side_become_radiant_too() {
            let mut s = board(
                true,
                json!([VANILLA, VANILLA, VANILLA, VANILLA]),
                json!([{ "def": WEAPONS, "lane": 2 }]),
                json!([{ "def": VANILLA, "lane": 2 }]),
            );
            let (left, pick, right, far) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P1, 3), s.unit_at(P1, 4));
            let behind = s.backrow(P1, 2).expect("the Field Spell");
            let across = s.unit_at(P2, 2);
            s.play(BEAN, pick::targets(pick::instance(&pick)));
            for made in [&left, &pick, &right] {
                assert!(s.card(made).radiant);
            }
            for kept in [&far, &behind, &across] {
                assert!(!s.card(kept).radiant, "nothing across sides or rows");
            }
        }

        #[test]
        fn r2198003_a_neighbour_immune_to_spells_is_passed_by() {
            let mut s = board(
                true,
                json!([
                    { "def": MEGABASE, "lane": 1 },
                    { "def": VANILLA, "lane": 2 },
                    { "def": VANILLA, "lane": 3 },
                ]),
                json!([]),
                json!([]),
            );
            let (immune, pick, right) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P1, 3));
            s.play(BEAN, pick::targets(pick::instance(&pick)));
            assert!(s.card(&pick).radiant && s.card(&right).radiant);
            assert!(!s.card(&immune).radiant);
        }
    }

    mod before_help_aim {
        use super::*;

        use crate::scripts::training_grounds::c027_hopeful_jelly_bean::{ID, script_before_help_aim};

        /// R767000: a game from before the help aim radiants the caster's enemy with a stolen Bean.
        #[test]
        fn r767000_a_game_before_the_help_aim_radiants_the_casters_enemy_with_a_stolen_bean() {
            crate::under_kept(ID, script_before_help_aim(), || {
                let (s, mine, theirs) = stolen_rig(cheap(BEAN));
                assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == BEAN), "P2 stole and cast it");
                assert!(s.card(&mine).radiant, "the caster's enemy becomes Radiant");
                assert!(!s.card(&theirs).radiant, "the caster's own Unit stays as it was");
            });
        }
    }
}
