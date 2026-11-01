//! TG #27 Hopeful Jelly Bean (SPEC §8.11 row TG 27; §6.3 Make Radiant, R22, R74, R81, R97, R177,
//! R2198003): (4) Spell, Jelly Bean, Rare, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Make a permanent Radiant."
//!   Radiant: "Cleave / Make a permanent Radiant."
//!
//! Engine: Core #34's target (any permanent on either side's field, R81) and Make Radiant (`set_radiant`,
//! R74): the flag only, so damage and buffs are kept, no Cry fires again (R22), and a face-down card's
//! change reaches its opponent as a change alone (R97, R177). A Radiant pick is allowed, so no pick check
//! leaks a face-down card's face. Radiant's Cleave (R2198003): the permanents beside the pick in its row
//! and side (`adjacent_to`, §3.1), read as the clause begins; a neighbour Immune to Spells is passed by.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-027";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))]
}

fn chosen() -> TargetSpec {
    json_as(json!({ "of": "chosen" }))
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
                let mut effects: Vec<Effect> = adjacent_to(&*ctx, &chosen(), &BoardScope::default())
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
    const FELINORS: &str = "core-012"; // (2) Unit 3/4, Cry: summon a copy of this; Radiant 6/8.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4; Radiant 12/12.
    const WEAPONS: &str = "core-014"; // Field Spell: your Units have +3 Attack (Radiant +6).
    const TRAP: &str = "core-041"; // Sheepish, a Trap.
    const MEGABASE: &str = "stinkyscape-116"; // Unit, Immune to Spells.

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

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] })
    }

    fn felinors(s: &Scenario, player: PlayerId) -> usize {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane))
            .filter(|card| card.def_id == FELINORS)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn a_base_unit_picked_becomes_radiant_its_damage_and_buffs_kept() {
            let mut s = board(false, json!([{ "def": FELINORS, "lane": 1, "damage": 1 }]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.card_mut(&unit).buffs = AttackHealth { attack: 1, health: 1 };
            s.play(BEAN, on(&unit));
            assert!(s.card(&unit).radiant);
            // 6/8 Radiant, +1/+1 kept, 1 damage kept.
            s.expect_stats(&unit, json!({ "attack": 7, "health": 8 }));
            assert_eq!(s.stats(&unit).max_health, 9);
        }

        #[test]
        fn its_cry_does_not_fire_again() {
            let mut s = board(false, json!([FELINORS]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(BEAN, on(&unit));
            assert!(s.card(&unit).radiant);
            assert_eq!(felinors(&s, P1), 1, "R22: no copy summoned");
        }

        #[test]
        fn a_field_spell_picked_runs_its_radiant_text_from_then_on() {
            let mut s = board(false, json!([VANILLA]), json!([WEAPONS]), json!([]));
            let unit = s.unit_at(P1, 1);
            let weapons = s.backrow(P1, 1).expect("the Field Spell");
            assert_eq!(s.stats(&unit).attack, 7);
            s.play(BEAN, on(&weapons));
            assert!(s.card(&weapons).radiant);
            assert_eq!(s.stats(&unit).attack, 10, "+6 now");
        }

        #[test]
        fn a_face_down_trap_of_yours_turns_radiant_its_opponent_seeing_only_a_change() {
            let mut s = board(false, json!([]), json!([{ "def": TRAP, "faceUp": false }]), json!([]));
            let trap = s.backrow(P1, 1).expect("the set Trap");
            s.play(BEAN, on(&trap));
            assert!(s.card(&trap).radiant);
            assert_ne!(s.card(&trap).face_up, Some(true), "still hidden");
            let seen = serde_json::to_string(&s.view(P2)).unwrap();
            assert!(!seen.contains(TRAP), "the opponent never sees what it is (R97, R177)");
        }

        #[test]
        fn an_enemy_permanent_may_be_picked() {
            let mut s = board(false, json!([]), json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(BEAN, on(&theirs));
            assert!(s.card(&theirs).radiant);
            s.expect_stats(&theirs, json!({ "attack": 12, "health": 12 }));
        }

        #[test]
        fn an_already_radiant_pick_nothing() {
            let mut s = board(false, json!([{ "def": FELINORS, "radiant": true, "lane": 1 }]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(BEAN, on(&unit));
            assert!(s.card(&unit).radiant);
            s.expect_stats(&unit, json!({ "attack": 6, "health": 8 }));
            assert_eq!(felinors(&s, P1), 1);
        }

        #[test]
        fn the_base_face_makes_the_pick_alone_radiant() {
            let mut s = board(false, json!([VANILLA, VANILLA, VANILLA]), json!([]), json!([]));
            let (left, pick, right) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P1, 3));
            s.play(BEAN, on(&pick));
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
            s.play(BEAN, on(&pick));
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
            s.play(BEAN, on(&pick));
            assert!(s.card(&pick).radiant && s.card(&right).radiant);
            assert!(!s.card(&immune).radiant);
        }
    }
}
