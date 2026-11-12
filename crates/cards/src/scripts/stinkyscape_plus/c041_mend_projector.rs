//! SPEC §8.12 S+ #41 Mend Projector (R3556003; R62, R1420):
//! (2) Field Spell, Mindustrial, Rare, a card of the Stinkyscape+ set.
//!   Base:    "End of turn: Heal all units to full."
//!   Radiant: "End of turn: Heal all units to full & give them Divine Shield."
//!
//! At the end of its controller's turn (R62) it heals every Unit on both sides to full (`heal` with
//! `toFull`), its controller's first (R3556003); the Radiant face then grants each of them Divine
//! Shield — enemies too (as S+ #13 Shield Repairer's).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-041";

fn mend_projector(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let controller = ctx.controller;
            let mut effects = Vec::new();
            // R3556003: both sides, the controller's Units first.
            for player in [controller, opponent_of(controller)] {
                for unit in active_units_of(ctx.state, player) {
                    let id = unit.id.clone();
                    effects.push(heal(json_as(
                        json!({ "target": { "of": "instance", "instanceId": id }, "toFull": true }),
                    )));
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
        base: mend_projector(false),
        radiant: mend_projector(true),
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
        }
    }
}
