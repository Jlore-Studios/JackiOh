//! SPEC §8.12 S+ #41 Mend Projector (R3556003, R4665001; R62, R1420):
//! (3) Field Spell, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "End of turn: Heal all your Units to full and give them +{health} Health." (1)
//!   Radiant: "End of turn: Heal all your Units to full and give them +{health} Health and Divine
//!            Shield." (2)
//!
//! At the end of its controller's turn (R62) it heals every Unit of its controller's to full (`heal`
//! with `toFull`, R3556003); since issue #4353 it then gives each of them +`health` Health, the declared
//! number (R386), a permanent Buff of its max and current Health (`buff`, §10.4 layer 4), so each stands
//! at full after it (R4353024); the Radiant face then grants each of them Divine Shield (as S+ #13
//! Shield Repairer's). Issue #4665 made it (3), from (1), and its controller's Units alone (R4665001):
//! through v0.6.12b it mended the opponent's Units too, after its controller's, which `script_v0_6_12b`
//! keeps, and through v0.6.8b it gave no Health, which `script_v0_6_8b` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-041";

/// `health`: whether each Unit healed also gets +`health` Health (R4353024, since issue #4353);
/// `both`: whether it mends the opponent's Units too, as it did until issue #4665 (R4665001).
fn mend_projector(radiant: bool, health: bool, both: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let controller = ctx.controller;
            let gain = if health { param(&*ctx, "health") } else { 0 };
            let mut effects = Vec::new();
            // R3556003: the controller's Units, then (through v0.6.12b) the opponent's (R4665001).
            let sides = if both {
                vec![controller, opponent_of(controller)]
            } else {
                vec![controller]
            };
            for player in sides {
                for unit in active_units_of(ctx.state, player) {
                    let id = unit.id.clone();
                    effects.push(heal(json_as(
                        json!({ "target": { "of": "instance", "instanceId": id }, "toFull": true }),
                    )));
                    if health {
                        effects.push(buff(json_as(json!({
                            "target": { "of": "instance", "instanceId": id },
                            "health": gain,
                        }))));
                    }
                    if radiant {
                        effects.push(grant_keyword(json_as(json!({
                            "target": { "of": "instance", "instanceId": id },
                            "keyword": { "kind": "Divine Shield" },
                        }))));
                    }
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mend_projector(false, true, false),
        radiant: mend_projector(true, true, false),
    }
}

/// R767000: through v0.6.12b it mended both sides' Units (issue #4665, R4665001); `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_12b() -> CardScripts {
    CardScripts {
        base: mend_projector(false, true, true),
        radiant: mend_projector(true, true, true),
    }
}

/// R767000: through v0.6.8b it healed and (Radiant) shielded, giving no Health (issue #4353);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_8b() -> CardScripts {
    CardScripts {
        base: mend_projector(false, false, true),
        radiant: mend_projector(true, false, true),
    }
}

// S+ #41 Mend Projector — SPEC §8.12 row S+ 41 (R3556003; R62).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, next_turn_of, run_as, weapon_board};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A durable Unit to wound and heal.
    const BODY: &str = "stand-in (1) 4/4 R12/12";

    /// Both sides hold Units and p1 holds the Projector; `"mana": 10` plays it from hand.
    fn projected(radiant: bool, seed: &str) -> Scenario {
        let mut s = weapon_board(
            seed,
            json!({
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [BODY, BODY],
                "library": filler(6),
            }),
            json!({
                "field": [BODY, BODY],
                "library": filler(6),
            }),
        );
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// Wound every Unit on the board by 2, as p1 and outside any turn boundary.
    fn wound_all(s: &mut Scenario) {
        for player in [P1, P2] {
            for lane in 1..=2 {
                if let Some(unit) = s.unit(player, lane) {
                    let id = unit.id.clone();
                    run_as(
                        s,
                        jackioh_engine::effects::damage(json_as(json!({
                            "to": { "of": "instance", "instanceId": id },
                            "amount": 2,
                        }))),
                        P1,
                    );
                }
            }
        }
    }

    fn full(s: &Scenario, player: PlayerId, lane: i32) -> bool {
        let id = s.unit(player, lane).expect("a Unit there").id;
        let view = s.stats(id.as_str());
        view.health == view.max_health
    }

    fn shielded(s: &Scenario, id: &str) -> bool {
        s.stats(id).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::DivineShield)
    }

    /// The printed Health of `BODY`, which none of the boards plays Radiant.
    const BODY_HEALTH: i32 = 4;

    /// R4353024: every Unit of `player`'s stands at full at `BODY_HEALTH` + `gain`.
    fn assert_side_at(s: &Scenario, player: PlayerId, gain: i32) {
        for lane in 1..=2 {
            let id = s.unit(player, lane).expect("a Unit there").id;
            let view = s.stats(id.as_str());
            assert_eq!(
                (view.health, view.max_health),
                (BODY_HEALTH + gain, BODY_HEALTH + gain),
                "{player:?}'s lane {lane}"
            );
        }
    }

    /// The kept scripts' reach: every Unit on both sides at full at `BODY_HEALTH` + `gain`.
    fn assert_all_at(s: &Scenario, gain: i32) {
        for player in [P1, P2] {
            assert_side_at(s, player, gain);
        }
    }

    /// R4665001: the opponent's Units are left as `wound_all` left them, 2 below their printed Health,
    /// with no Divine Shield.
    fn assert_enemies_untouched(s: &Scenario) {
        for lane in 1..=2 {
            let id = s.unit(P2, lane).expect("a Unit there").id;
            let view = s.stats(id.as_str());
            assert_eq!(
                (view.health, view.max_health),
                (BODY_HEALTH - 2, BODY_HEALTH),
                "p2's lane {lane}"
            );
            assert!(!shielded(s, id.as_str()), "p2's lane {lane} has no Divine Shield");
        }
    }

    mod base {
        use super::*;

        /// R4665001: its controller's Units alone, since issue #4665.
        #[test]
        fn r3556003_r4665001_end_of_turn_heals_every_unit_of_its_controllers_to_full_and_no_enemy() {
            let mut s = projected(false, "s41-heal");
            wound_all(&mut s);
            for player in [P1, P2] {
                for lane in 1..=2 {
                    assert!(!full(&s, player, lane), "wounded before the end of turn");
                }
            }
            s.end_turn();
            for lane in 1..=2 {
                assert!(full(&s, P1, lane), "p1's lane {lane} healed to full");
            }
            assert_enemies_untouched(&s);
        }

        /// R4353024: after the heal, each Unit of its controller's gets +1 Health, so each stands at full
        /// a point higher, every end of its controller's turn.
        #[test]
        fn r4353024_end_of_turn_gives_every_friendly_unit_plus_1_health_after_the_heal() {
            let mut s = projected(false, "s41-health");
            wound_all(&mut s);
            s.end_turn();
            assert_side_at(&s, P1, 1);
            assert_enemies_untouched(&s);
            next_turn_of(&mut s, P1);
            wound_all(&mut s);
            s.end_turn();
            assert_side_at(&s, P1, 2);
        }

        /// R386: the Health is the declared `health`, read through `param`.
        #[test]
        fn r386_the_health_reads_through_param() {
            let mut s = projected(false, "s41-tuned");
            let gain = crate::upgrade_number(&mut s, ID, "health");
            s.end_turn();
            assert_side_at(&s, P1, gain);
        }

        /// R767000, R4665001: through v0.6.12b it mended both sides, its controller's Units first.
        #[test]
        fn r767000_r4665001_the_script_kept_for_v0_6_12b_mends_both_sides() {
            crate::under_kept(ID, script_v0_6_12b(), || {
                let mut s = projected(false, "s41-kept-both");
                wound_all(&mut s);
                s.end_turn();
                assert_all_at(&s, 1);
            });
        }

        /// R767000: through v0.6.8b it healed to full and gave no Health (issue #4353).
        #[test]
        fn r767000_the_script_kept_for_v0_6_8b_gives_no_health() {
            crate::under_kept(ID, script_v0_6_8b(), || {
                for radiant in [false, true] {
                    let mut s = projected(radiant, &format!("s41-kept-{radiant}"));
                    wound_all(&mut s);
                    s.end_turn();
                    assert_all_at(&s, 0);
                }
            });
        }

        #[test]
        fn r3556003_not_on_the_opponents_end_of_turn() {
            let mut s = projected(false, "s41-turns");
            // Past its controller's end of turn, so its heal already fired; wound afresh.
            next_turn_of(&mut s, P2);
            wound_all(&mut s);
            s.end_turn();
            assert!(!full(&s, P1, 1), "the opponent's end of turn heals nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3556003_r4665001_radiant_gives_every_friendly_unit_divine_shield_and_no_enemy() {
            let mut s = projected(true, "s41-radiant");
            wound_all(&mut s);
            s.end_turn();
            for lane in 1..=2 {
                let id = s.unit(P1, lane).expect("a Unit there").id;
                assert!(full(&s, P1, lane), "p1's lane {lane} healed to full");
                assert!(shielded(&s, id.as_str()), "p1's lane {lane} has Divine Shield");
            }
            // R4353024: +2 Health beside the Divine Shield.
            assert_side_at(&s, P1, 2);
            assert_enemies_untouched(&s);
        }

        /// R767000, R4665001: the Radiant face kept for v0.6.12b shields the enemies too.
        #[test]
        fn r767000_r4665001_the_radiant_script_kept_for_v0_6_12b_shields_both_sides() {
            crate::under_kept(ID, script_v0_6_12b(), || {
                let mut s = projected(true, "s41-kept-radiant");
                wound_all(&mut s);
                s.end_turn();
                for player in [P1, P2] {
                    for lane in 1..=2 {
                        let id = s.unit(player, lane).expect("a Unit there").id;
                        assert!(
                            shielded(&s, id.as_str()),
                            "{player:?}'s lane {lane} has Divine Shield"
                        );
                    }
                }
                assert_all_at(&s, 2);
            });
        }
    }
}
