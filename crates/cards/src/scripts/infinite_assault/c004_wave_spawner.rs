//! SPEC §8.13 IA #4 Wave Spawner ((2) Unit, Common, 3/4 → 6/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "End of turn: If you played a Created card this turn, summon a Drone."
//!   Radiant: "End of turn: If you played a Created card this turn, summon two Drones."
//!
//! S T-AI-8 Drone Uplink's end-of-turn summon of IA #4.1, behind a read of this turn's plays
//! (`played_ids_this_turn`): one the game still holds that is Created (R943) meets the condition,
//! this Wave Spawner among them; a token that has ceased to exist (R11) is no longer read (R4577005).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-004";

/// IA #4.1 Drone.
const DRONE: &str = "infiniteassault-004-1";

/// "a Drone", Radiant "two Drones".
const DRONES: usize = 1;
const RADIANT_DRONES: usize = 2;

/// R4577005: whether any card its controller played this turn is Created.
fn played_a_created_card(ctx: &EffectContext<'_>) -> bool {
    played_ids_this_turn(ctx.state, ctx.controller)
        .iter()
        .filter_map(|id| find_instance(ctx.state, id))
        .any(is_created)
}

fn spawner(drones: usize) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            if !played_a_created_card(ctx) {
                return vec![];
            }
            (0..drones)
                .map(|_| summon(json_as(json!({ "defId": DRONE, "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spawner(DRONES),
        radiant: spawner(RADIANT_DRONES),
    }
}

// IA #4 Wave Spawner — SPEC §8.13 row 4 (R4577005).
#[cfg(test)]
mod tests {
    use super::{DRONE, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SPELL: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": SPELL, "created": true }, SPELL],
                "field": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(2),
            },
            "p2": { "hand": [SPELL], "library": crate::filler(2) },
        }))
    }

    fn drones(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == DRONE)
            .cloned()
            .collect()
    }

    fn play_the_created_one(s: &mut Scenario) {
        let made = s.hand(P1).into_iter().find(|card| card.created == Some(true)).expect("the Created card");
        s.play(&made, json!({}));
    }

    fn play_the_dealt_one(s: &mut Scenario) {
        let dealt = s.hand(P1).into_iter().find(|card| card.created != Some(true)).expect("the dealt card");
        s.play(&dealt, json!({}));
    }

    mod base {
        use super::*;

        #[test]
        fn a_created_card_played_this_turn_summons_a_drone_at_its_end() {
            let mut s = board(false);
            play_the_created_one(&mut s);
            assert!(drones(&s).is_empty(), "not before the end of turn");
            s.end_turn();
            let drones = drones(&s);
            assert_eq!(drones.len(), 1);
            for drone in nonempty(drones, "the Drone") {
                assert!(!drone.radiant);
            }
        }

        #[test]
        fn a_dealt_card_or_no_play_summons_nothing() {
            let mut s = board(false);
            play_the_dealt_one(&mut s);
            s.end_turn();
            assert!(drones(&s).is_empty());
            let mut quiet = board(false);
            quiet.end_turn();
            assert!(drones(&quiet).is_empty());
        }

        #[test]
        fn r4577005_a_created_wave_spawner_reads_its_own_play() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "created": true }], "library": crate::filler(2) },
                "p2": { "hand": [SPELL], "library": crate::filler(2) },
            }));
            s.play(ID, json!({}));
            s.end_turn();
            assert_eq!(drones(&s).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_created_card_played_this_turn_summons_two_drones() {
            let mut s = board(true);
            play_the_created_one(&mut s);
            play_the_dealt_one(&mut s);
            s.end_turn();
            assert_eq!(drones(&s).len(), 2);
        }

        #[test]
        fn a_dealt_card_summons_nothing() {
            let mut s = board(true);
            play_the_dealt_one(&mut s);
            s.end_turn();
            assert!(drones(&s).is_empty());
        }
    }
}
