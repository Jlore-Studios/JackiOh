//! IA #40 Hall of Mirrors (SPEC §8.13 row 40), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Whenever you set a card face-down, summon a Reflection."
//!   Radiant: "Whenever you set a card face-down, summon a Radiant Reflection."
//!
//! A card its controller sets face-down arrives face-down in its controller's backrow: a
//! face-down play and a summon landing face-down both emit `summoned` with the backrow row
//! (§10.5 step 4 places every arrival, R227), still face-down as they land. The one arrival
//! event answers once — a `cardPlayed` arm beside it would answer the same set twice, since a
//! face-down play emits both. Face-up arrivals never answer it (R61). Each answers with a
//! Reflection, Radiant on the Radiant face.
//!
//! Engine: `summon` on `summoned` (R4586003). Built by C2 (#4586).

use jackioh_engine::prelude::*;
use jackioh_engine::preview::is_face_down;

pub const ID: &str = "infiniteassault-040";

/// What each face-down set summons (§7).
const REFLECTION: &str = "infiniteassault-040-1";

/// One of its controller's cards arriving face-down in its controller's backrow: a face-down
/// play or a summon landing face-down.
fn set_face_down(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::Summoned {
        player,
        instance_id,
        row: Row::Backrow,
        ..
    } = event
    else {
        return false;
    };
    *player == ctx.controller
        && find_instance(&*ctx.state, instance_id)
            .is_some_and(|card| is_face_down(&*ctx.state, card))
}

fn mirrors(radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "hall-of-mirrors",
            &[GameEventType::Summoned],
            move |ctx, event| {
                if !set_face_down(ctx, event) {
                    return vec![];
                }
                vec![summon(json_as(json!({ "defId": REFLECTION, "radiant": radiant })))]
            },
        )
        .with_when(set_face_down)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: mirrors(false),
        radiant: mirrors(true),
    }
}

// IA #40 Hall of Mirrors — SPEC §8.13 row 40 (R4586003).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ID, REFLECTION};
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const TRAP: &str = "core-060"; // Bear Honeypot: a Trap.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true, "lane": 1 }],
                "hand": [TRAP, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
        }))
    }

    fn reflections(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| card.def_id == REFLECTION)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586003_a_trap_played_face_down_summons_a_reflection() {
            let mut s = standing(false, "mirrors-trap");
            s.play(TRAP, json!({ "zone": 2 }));
            let summoned = reflections(&s);
            assert_eq!(summoned.len(), 1);
            assert!(!summoned[0].radiant, "a plain Reflection");
        }

        #[test]
        fn r4586003_a_cloaked_set_summons_a_reflection() {
            let mut s = standing(false, "mirrors-cloaked");
            crate::set_cloaked(&mut s, P1, crate::CLOAKED_UNIT, 2);
            assert_eq!(reflections(&s).len(), 1);
        }

        #[test]
        fn r4586003_a_face_up_summon_summons_none() {
            let mut s = standing(false, "mirrors-faceup");
            s.play(STOCKPILE, json!({}));
            assert!(reflections(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586003_the_reflection_is_radiant() {
            let mut s = standing(true, "mirrors-radiant");
            s.play(TRAP, json!({ "zone": 2 }));
            let summoned = reflections(&s);
            assert_eq!(summoned.len(), 1);
            assert!(summoned[0].radiant, "a Radiant Reflection");
        }
    }
}
