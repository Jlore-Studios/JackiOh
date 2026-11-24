//! SPEC §8.13 IA #117 Core Dump ((2) Field Trap, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when a friendly Unit dies: Shuffle a copy of it into your deck."
//!   Radiant: "Reveals when a friendly Unit dies: Shuffle a Radiant copy of it into your deck."
//!
//! IA #111's trigger shape on a Field Trap: it answers its controller's Unit deaths (`is_unit_death`,
//! R4423000) and shuffles a fresh copy of the card that died on the face it died with (R4577002)
//! into its controller's deck, up to the library cap (R80, R316). A Field Trap stays after it fires
//! and fires again, so every death shuffles another copy (R2657001); the Radiant face always
//! shuffles a Radiant copy.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-117";

fn core_dump(face_radiant: bool) -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "core-dump",
            &[GameEventType::Destroyed],
            move |_ctx, event| {
                let GameEvent::Destroyed {
                    def_id, radiant, ..
                } = event
                else {
                    return vec![];
                };
                vec![shuffle_into(json_as(json!({
                    "defId": def_id.clone(),
                    "count": 1,
                    "player": "self",
                    "radiant": face_radiant || *radiant == Some(true),
                })))]
            },
        )
        .with_when(|ctx, event| {
            matches!(event, GameEvent::Destroyed { controller, .. } if *controller == ctx.controller)
                && is_unit_death(ctx.state, event)
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: core_dump(false),
        radiant: core_dump(true),
    }
}

// IA #117 Core Dump — SPEC §8.13 row 117.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const FILLER: &str = "stand-in (0) Spell";

    /// The Dump set face-down behind p1's two Fodder; p1 kills both.
    fn dumped(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB],
                "field": [FODDER, FODDER],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": false, "lane": 1 }],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        let first = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&first)));
        let second = s.unit_at(P1, 2);
        s.play(HIT_JOB, pick::targets(pick::instance(&second)));
        s
    }

    fn dumped_copies(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == FODDER)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn every_death_shuffles_a_copy_on_the_face_it_died_with() {
            let s = dumped(false);
            let copies = dumped_copies(&s);
            assert_eq!(copies.len(), 2, "a Field Trap fires on every death");
            for copy in nonempty(copies, "the copies") {
                assert!(!copy.radiant);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn every_death_shuffles_a_radiant_copy() {
            let s = dumped(true);
            let copies = dumped_copies(&s);
            assert_eq!(copies.len(), 2, "a Field Trap fires on every death");
            for copy in nonempty(copies, "the copies") {
                assert!(copy.radiant);
            }
        }
    }
}
