//! SPEC §8.12 S+ #41 Mend Projector (R3556003; R62, R1420):
//! (1) Field Spell, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "End of turn: Heal all Units to full and give them +{health} Health." (1)
//!   Radiant: "End of turn: Heal all Units to full and give them +{health} Health and Divine Shield." (2)
//!
//! At the end of its controller's turn (R62) it heals every Unit on both sides to full (`heal` with
//! `toFull`), its controller's first (R3556003); since issue #4353 it then gives each of them +`health`
//! Health, the declared number (R386), a permanent Buff of its max and current Health (`buff`, §10.4
//! layer 4), so each stands at full after it (R4353024); the Radiant face then grants each of them Divine
//! Shield — enemies too, as the heal and the Health reach them (as S+ #13 Shield Repairer's). Through
//! v0.6.8b it gave no Health, which `script_v0_6_8b` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-041";

/// `health`: whether each Unit healed also gets +`health` Health (R4353024, since issue #4353).
fn mend_projector(radiant: bool, health: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let controller = ctx.controller;
            let gain = if health { param(&*ctx, "health") } else { 0 };
            let mut effects = Vec::new();
            // R3556003: both sides, the controller's Units first.
            for player in [controller, opponent_of(controller)] {
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
        base: mend_projector(false, true),
        radiant: mend_projector(true, true),
    }
}

/// R767000: through v0.6.8b it healed and (Radiant) shielded, giving no Health (issue #4353);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_8b() -> CardScripts {
    CardScripts {
        base: mend_projector(false, false),
        radiant: mend_projector(true, false),
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

    /// R4353024: every Unit on both sides stands at full at `BODY_HEALTH` + `gain`.
    fn assert_all_at(s: &Scenario, gain: i32) {
        for player in [P1, P2] {
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
    }

    mod base {
        use super::*;

        #[test]
        fn r3556003_end_of_turn_heals_every_unit_on_both_sides_to_full() {
            let mut s = projected(false, "s41-heal");
            wound_all(&mut s);
            for player in [P1, P2] {
                for lane in 1..=2 {
                    assert!(!full(&s, player, lane), "wounded before the end of turn");
                }
            }
            s.end_turn();
            for player in [P1, P2] {
                for lane in 1..=2 {
                    assert!(full(&s, player, lane), "{player:?}'s lane {lane} healed to full");
                }
            }
        }

        /// R4353024: after the heal, each Unit on both sides gets +1 Health, so each stands at full a point
        /// higher, every end of its controller's turn.
        #[test]
        fn r4353024_end_of_turn_gives_every_unit_on_both_sides_plus_1_health_after_the_heal() {
            let mut s = projected(false, "s41-health");
            wound_all(&mut s);
            s.end_turn();
            assert_all_at(&s, 1);
            next_turn_of(&mut s, P1);
            wound_all(&mut s);
            s.end_turn();
            assert_all_at(&s, 2);
        }

        /// R386: the Health is the declared `health`, read through `param`.
        #[test]
        fn r386_the_health_reads_through_param() {
            let mut s = projected(false, "s41-tuned");
            let gain = crate::upgrade_number(&mut s, ID, "health");
            s.end_turn();
            assert_all_at(&s, gain);
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
        fn r3556003_radiant_gives_every_unit_divine_shield_enemies_too() {
            let mut s = projected(true, "s41-radiant");
            wound_all(&mut s);
            s.end_turn();
            for player in [P1, P2] {
                for lane in 1..=2 {
                    let id = s.unit(player, lane).expect("a Unit there").id;
                    assert!(full(&s, player, lane), "{player:?}'s lane {lane} healed to full");
                    assert!(shielded(&s, id.as_str()), "{player:?}'s lane {lane} has Divine Shield");
                }
            }
            // R4353024: +2 Health beside the Divine Shield.
            assert_all_at(&s, 2);
        }
    }
}
