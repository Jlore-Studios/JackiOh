//! SPEC §8.12 S+ #154 Chaos Accelerator (Unit 3/6 → 6/12, Starship Piece, cost 2, Epic), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece
//!            When launched, cast a random Call to Chaos"
//!   Radiant: "Starship Piece
//!            When launched, cast a random Radiant Call to Chaos"
//!
//! Starship Piece is a catalog keyword the engine plays: its death fuses a copy into its owner's Dormant
//! Starship (R1852000), and "When launched" (`when_launched`, R1852003) runs only as part of that Starship,
//! after its fused Cries; on the field it does nothing. R3552000: the launch casts one card drawn from every
//! Call to Chaos of the shipped sets, tokens included (R380, R382, R1420), never this card (R387), through
//! `castRandom`: the card is the Radiant face's Radiant, every choice of the cast is random so nothing
//! pauses for a prompt, and it counts as a link of the Call to Chaos chain (R28).

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-154";

/// "Cast a random [Radiant] Call to Chaos": one cast.
const CASTS: i32 = 1;

/// R3552000: "When launched, cast a random [Radiant] Call to Chaos."
fn accelerator(radiant: bool) -> Script {
    Script {
        triggers: vec![when_launched("chaos-accelerator", move |_ctx, _event| {
            vec![cast_random(CastRandomArgs {
                query: crate::query::call_to_chaos_editions().into(),
                count: Some(CastRandomCount::Fixed(CASTS)),
                radiant: Some(radiant),
                target_enemies: None,
                afterward: None,
            })]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: accelerator(false),
        radiant: accelerator(true),
    }
}

// S+ #154 Chaos Accelerator — SPEC §8.12 row 154 (R3552000; R28, R380, R382, R387, R1852000, R1852003).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// A Piece's own launch: the cards cast from it, in order, as (instance id, def id).
    fn casts(events: &[GameEvent]) -> Vec<(String, String)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed {
                    instance_id, def_id, ..
                } => Some((instance_id.clone(), def_id.clone())),
                _ => None,
            })
            .collect()
    }

    /// Launches a Starship built of the Accelerator on `seed`.
    fn launched(seed: &str, radiant: bool) -> (Scenario, Vec<(String, String)>) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": ID, "lane": 1, "radiant": radiant }]),
            json!({}),
            json!({}),
        );
        let from = s.events().len();
        launch_starship(&mut s, &ship);
        let made = casts(&s.events()[from..]);
        (s, made)
    }

    /// The Call to Chaos pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        crate::query::query(&crate::query::call_to_chaos_editions())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3552000_launch_casts_a_random_call_to_chaos() {
            let pool = pool();
            assert!(!pool.is_empty());
            for i in 0..5 {
                let (s, made) = launched(&format!("r3552000-base-{i}"), false);
                let (picked, def) = made.first().expect("the launch cast a card");
                assert!(pool.contains(def), "{def} is no Call to Chaos");
                assert!(
                    !s.card(picked.as_str()).radiant,
                    "the base face casts it as it is"
                );
                assert!(s.state().pending.is_none() || s.state().result.is_some());
            }
        }

        #[test]
        fn r3552000_the_cast_happens_only_at_the_launch() {
            let (mut s, ship) = built_starship(
                "r3552000-field",
                json!([{ "def": ID, "lane": 1 }]),
                json!({}),
                json!({}),
            );
            let from = s.events().len();
            s.end_turn();
            s.end_turn();
            assert!(
                casts(&s.events()[from..]).is_empty(),
                "nothing is cast before the launch"
            );
            assert!(s.state().active == P1 && !ship.is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3552000_radiant_launch_casts_it_radiant() {
            let pool = pool();
            for i in 0..5 {
                let (s, made) = launched(&format!("r3552000-radiant-{i}"), true);
                let (picked, def) = made.first().expect("the launch cast a card");
                assert!(pool.contains(def), "{def} is no Call to Chaos");
                assert!(
                    s.card(picked.as_str()).radiant,
                    "the Radiant face casts it Radiant"
                );
            }
        }
    }
}
