//! SPEC §8.13 IA #98 Echo Chamber ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Echo 1\nDeal {damage} damage to an enemy Unit." (damage 2)
//!   Radiant: "Echo 2\nDeal {damage} damage to an enemy Unit." (damage 2)
//!
//! S T-axnit's shape, aimed at enemy Units alone and with no Lifesteal: one hit of the declared
//! `damage` per resolution (`damage`, R386), the Radiant face resolving three times instead of twice
//! (`StaticFlags.echo`, §6.2; the repeat asks its target afresh). The Echo counts are the keyword's
//! own, not params (R482's `KEYWORD` row).

use jackioh_engine::effects::damage;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-098";

/// "Echo 1", the base face's.
const ECHO: i32 = 1;
/// "Echo 2", the Radiant face's.
const RADIANT_ECHO: i32 = 2;

fn face(echo: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            echo: Some(echo),
            ..StaticFlags::default()
        }),
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        cry: Some(hook(|ctx| {
            vec![damage(json_as(json!({
                "to": { "of": "chosen" },
                "amount": param(&*ctx, "damage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(ECHO),
        radiant: face(RADIANT_ECHO),
    }
}

// IA #98 Echo Chamber — SPEC §8.13 row 98.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P2;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const BODY: &str = "stand-in (2) 5/5";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "field": [BODY] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn echo_1_hits_twice() {
            let mut s = board(false);
            let body = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&body)));
            s.expect_stats(&body, json!({ "health": 3 }));
            assert!(s.state().pending.is_some(), "the Echo repeat asks its target");
            s.answer(pick::instance(&body));
            s.expect_stats(body.id.as_str(), json!({ "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn echo_2_hits_three_times() {
            let mut s = board(true);
            let body = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&body)));
            s.answer(pick::instance(&body));
            s.answer(pick::instance(&body));
            s.expect_in_zone(body.id.as_str(), "graveyard");
        }
    }
}
