//! SPEC §8.13 IA #157 Containment Field ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "An enemy Unit can't attack or be attacked until your next turn."
//!   Radiant: "Two enemy Units can't attack or be attacked until your next turn."
//!
//! Engine: each chosen enemy Unit (R81, aimed to harm) is Contained until the caster's next turn
//! begins (R4591001): it can neither attack nor be attacked, declared and forced alike, and the bar
//! ends as the caster's next turn begins or when the Unit leaves the field.

use jackioh_engine::effects::contain_until_next_turn;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-157";

fn containment(targets: i32) -> Script {
    Script {
        targets: vec![TargetDecl {
            aim: Some(TargetAim::Harm),
            ..TargetDecl::target(1, targets, json!({ "side": "enemy", "of": ["unit"] }))
        }],
        cry: Some(hook(move |_ctx| {
            (0..targets)
                .map(|index| {
                    contain_until_next_turn(json_as(json!({
                        "target": { "of": "chosen", "index": index },
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: containment(1),
        radiant: containment(2),
    }
}

// IA #157 Containment Field — SPEC §8.13 row 157 (R81, R4591001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT, UNIT],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "field": [UNIT, UNIT], "library": crate::filler(3) },
        }))
    }

    fn offers_attack(s: &Scenario, id: &str) -> bool {
        let unit = s.card(id).clone();
        legal_actions(s.state(), unit.controller).iter().any(|action| {
            matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == unit.id)
        })
    }

    mod base {
        use super::*;

        #[test]
        fn r4591001_a_contained_unit_neither_attacks_nor_is_attacked_until_your_next_turn() {
            let mut s = board(false);
            let target = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&target)));
            let held = s.unit_at(P2, 1).id.clone();
            assert!(!offers_attack(&s, &held));
            let mine = s.unit_at(P1, 1);
            let theirs = s.unit_at(P2, 1);
            s.expect_refused(|s| s.attack(&mine, &theirs));
            s.end_turn();
            assert!(!offers_attack(&s, &held), "still contained on the intervening turn");
            s.end_turn();
            assert!(offers_attack(&s, &held), "containment ends as your next turn begins");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn two_enemy_units_are_contained() {
            let mut s = board(true);
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);
            s.play(
                ID,
                pick::targets(json!([pick::instance(&first), pick::instance(&second)])),
            );
            let one = s.unit_at(P2, 1).id.clone();
            let two = s.unit_at(P2, 2).id.clone();
            assert!(!offers_attack(&s, &one));
            assert!(!offers_attack(&s, &two));
            s.end_turn();
            s.end_turn();
            assert!(offers_attack(&s, &one));
            assert!(offers_attack(&s, &two));
        }
    }
}
