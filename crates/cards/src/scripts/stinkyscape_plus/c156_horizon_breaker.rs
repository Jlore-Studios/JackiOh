//! SPEC §8.12 S+ #156 Horizon Breaker (Unit 3/6 → 6/12, Starship Piece, cost 4, Epic), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece
//!            When launched, cast a random Terminus Card"
//!   Radiant: "Starship Piece
//!            When launched, cast a random Radiant Terminus Card"
//!
//! Starship Piece is a catalog keyword the engine plays: its death fuses a copy into its owner's Dormant
//! Starship (R1852000), and "When launched" (`when_launched`, R1852003) runs only as part of that Starship,
//! after its fused Cries; on the field it does nothing. R3552001: the launch casts one card drawn from every
//! Terminus card of the shipped sets, tokens included (R380, R382, R1420), never this card (R387), through
//! `castRandom`: the card is the Radiant face's Radiant and every choice of the cast is random, so nothing
//! pauses for a prompt. A pick that is no Spell resolves as its own type: a Unit is summoned, a Trap set.

use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, cast_random};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-156";

/// "Cast a random [Radiant] Terminus Card": one cast.
const CASTS: i32 = 1;

/// R3552001: "When launched, cast a random [Radiant] Terminus Card."
fn breaker(radiant: bool) -> Script {
    Script {
        triggers: vec![when_launched("horizon-breaker", move |_ctx, _event| {
            vec![cast_random(CastRandomArgs {
                query: crate::query::terminus_cards().into(),
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
        base: breaker(false),
        radiant: breaker(true),
    }
}

// S+ #156 Horizon Breaker — SPEC §8.12 row 156 (R3552001; R380, R382, R387, R1852000, R1852003).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    /// The cards cast in `events`, in order, as (instance id, def id).
    fn casts(events: &[GameEvent]) -> Vec<(String, String)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { instance_id, def_id, .. } => Some((instance_id.clone(), def_id.clone())),
                _ => None,
            })
            .collect()
    }

    /// Launches a Starship built of the Breaker on `seed`: the cards its launch cast.
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

    /// The Terminus pool of the shipped sets, tokens included.
    fn pool() -> Vec<String> {
        crate::register_all();
        crate::query::query(&crate::query::terminus_cards())
            .into_iter()
            .map(|def| def.id.clone())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3552001_launch_casts_a_random_terminus_card() {
            let pool = pool();
            // The Core Terminus Unit and the Stinkyscape set's Terminus tokens are all in it.
            for id in ["core-100", "stinkyscape-004-1", "stinkyscape-004-4"] {
                assert!(pool.iter().any(|def| def == id), "{id} is missing from {pool:?}");
            }
            for i in 0..8 {
                let (s, made) = launched(&format!("r3552001-base-{i}"), false);
                let (picked, def) = made.first().expect("the launch cast a card");
                assert!(pool.contains(def), "{def} is no Terminus card");
                // R3569000: a cast Tears of the Kwzki Tree turns every card Radiant at the next check,
                // itself included, so only a game it did not make Radiant shows the face it was cast with.
                if s.state().radiant_world != Some(true) {
                    assert!(!s.card(picked.as_str()).radiant, "{def}");
                }
            }
        }

        #[test]
        fn r3552001_every_kind_of_pick_resolves_with_no_prompt_left_open() {
            let pool = pool();
            let mut seen: Vec<CardType> = Vec::new();
            for i in 0..60 {
                let (s, made) = launched(&format!("r3552001-kinds-{i}"), false);
                let Some((_, def)) = made.first() else { continue };
                assert!(pool.contains(def));
                let kind = catalog::def_of(None, def).type_;
                assert!(s.state().pending.is_none() || s.state().result.is_some(), "{def} left a prompt open");
                if !seen.contains(&kind) {
                    seen.push(kind);
                }
            }
            assert!(seen.len() > 1, "only {seen:?} came up");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3552001_radiant_launch_casts_it_radiant() {
            let pool = pool();
            for i in 0..8 {
                let (s, made) = launched(&format!("r3552001-radiant-{i}"), true);
                let (picked, def) = made.first().expect("the launch cast a card");
                assert!(pool.contains(def), "{def} is no Terminus card");
                assert!(s.card(picked.as_str()).radiant);
            }
        }
    }
}
