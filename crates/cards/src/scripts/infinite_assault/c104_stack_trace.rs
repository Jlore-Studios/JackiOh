//! SPEC §8.13 IA #104 Stack Trace ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Flashback\nDraw a card."
//!   Radiant: "Flashback, Echo 1\nDraw a card."
//!
//! M+ #5 Spirit Lash's shape without the damage: the Cry draws a card, and Flashback is the
//! catalog's keyword (§6.1). The Radiant face adds Echo 1 (`StaticFlags.echo`, §6.2), so it draws
//! twice. The Echo count is the keyword's own, not a param (R482's `KEYWORD` row).

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-104";

/// "Draw a card".
const DRAW: i32 = 1;
/// "Echo 1", the Radiant face's.
const RADIANT_ECHO: i32 = 1;

fn trace(echo: bool) -> Script {
    Script {
        static_flags: echo.then(|| StaticFlags {
            echo: Some(RADIANT_ECHO),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|_ctx| vec![draw(json_as(json!({ "count": DRAW })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: trace(false),
        radiant: trace(true),
    }
}

// IA #104 Stack Trace — SPEC §8.13 row 104.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn traced(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "library": crate::filler(2) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn cry_draws_a_card() {
            let s = traced(false);
            assert_eq!(s.hand(P1).len(), 1, "spent, then drew one");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn echo_1_draws_twice() {
            let s = traced(true);
            assert_eq!(s.hand(P1).len(), 2, "spent, then drew two");
        }
    }
}
