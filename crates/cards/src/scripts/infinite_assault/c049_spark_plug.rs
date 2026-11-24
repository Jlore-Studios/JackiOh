//! SPEC §8.13 IA #49 Spark Plug ((0) Unit, Common, 2/1 → 4/2), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Rush. Death: Gain {mana} mana this turn." (mana 1)
//!   Radiant: "Rush. Death: Gain {mana} mana this turn." (mana 2)
//!
//! Rush is the catalog's. Its Death gains the declared `mana` for this turn (`gain_mana`, §6.3),
//! which may pass the max (§2.3).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-049";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 2 is its declared `mana`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #49 Spark Plug — SPEC §8.13 row 49.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// A 3/3 wall: the Plug's Rush attack kills the Plug and nothing else.
    const WALL: &str = "stand-in (1) 3/3";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
            "p2": { "field": [WALL] },
        }))
    }

    /// Play the Plug and Rush it into the wall; it dies and its Death resolves.
    fn ram(s: &mut Scenario) {
        s.play(ID, json!({ "zone": 1 }));
        let plug = s.unit_at(P1, 1);
        s.attack(&plug, s.unit_at(P2, 1));
        s.expect_in_zone(&plug, "graveyard");
    }

    mod base {
        use super::*;

        #[test]
        fn its_death_gains_1_mana() {
            let mut s = board(false);
            s.expect_mana(P1, 4);
            ram(&mut s);
            s.expect_mana(P1, 5);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_death_gains_2_mana() {
            let mut s = board(true);
            ram(&mut s);
            s.expect_mana(P1, 6);
        }
    }
}
