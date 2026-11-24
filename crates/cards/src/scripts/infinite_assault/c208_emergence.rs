//! IA #208 Emergence (SPEC §8.13 row 208), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when your opponent's turn starts, if you control {units} or more Units:
//!            Give your Units +{buff}/+{buff}."
//!   Radiant: "Reveals when your opponent's turn starts, if you control {units} or more Units:
//!            Give your Units +{buff}/+{buff}."
//!
//! Engine: a Trap set face-down (R33) answering its opponent's `TurnStarted` (§5.1, R99): when at
//! least the declared `units` of its controller's active Units stand as it resolves — Dormant
//! ones not counted (R1839000) — it reveals and gives them a permanent `buff_all_units` of the
//! declared `buff`; below that count it stays set (R4593006). Built by C9 (#4593).

use jackioh_engine::effects::buff_all_units;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-208";

fn emergence() -> TriggerDef {
    TriggerDef::new("emergence", &[GameEventType::TurnStarted], |ctx, _event| {
        let gain = param(ctx, "buff");
        vec![buff_all_units(json_as(json!({ "attack": gain, "health": gain })))]
    })
    .with_when(|ctx, event| {
        let GameEvent::TurnStarted { player, .. } = event else {
            return false;
        };
        if *player != opponent_of(ctx.controller) {
            return false;
        }
        active_units_of(ctx.state, ctx.controller).len() as i32 >= param(ctx, "units")
    })
}

pub fn script() -> CardScripts {
    // The same trigger: the Radiant face's +6/+6 is its declared `buff`.
    let radiant = Script {
        triggers: vec![emergence()],
        ..Script::default()
    };
    CardScripts {
        base: Script {
            triggers: vec![emergence()],
            ..Script::default()
        },
        radiant,
    }
}

// IA #208 Emergence — SPEC §8.13 row 208 (R4593006).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn set(radiant: bool, units: usize, seed: &str) -> Scenario {
        let mut field = vec![];
        for lane in 1..=units {
            field.push(json!({ "def": VANILLA, "lane": lane }));
        }
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "field": field,
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn stats(s: &Scenario, lane: i32) -> (i32, i32) {
        let unit = s.unit_at(P1, lane);
        let view = unit_view(s.state(), &unit);
        (view.attack, view.health)
    }

    mod base {
        use super::*;

        #[test]
        fn r4593006_fires_at_the_opponents_start_with_3_units() {
            let mut s = set(false, 3, "emergence-base");
            s.end_turn();
            assert_eq!(stats(&s, 1), (7, 7));
            assert_eq!(stats(&s, 2), (7, 7));
            assert_eq!(stats(&s, 3), (7, 7));
        }

        #[test]
        fn r4593006_stays_set_with_2_units() {
            let mut s = set(false, 2, "emergence-quiet");
            s.end_turn();
            assert_eq!(stats(&s, 1), (4, 4));
            assert!(is_face_down(s.state(), &s.card(ID)), "still set");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4593006_fires_at_the_opponents_start_with_3_units() {
            let mut s = set(true, 3, "emergence-radiant");
            s.end_turn();
            assert_eq!(stats(&s, 1), (10, 10));
        }
    }
}
