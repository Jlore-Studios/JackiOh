//! SPEC §8.13 IA #186 Identity Theft ((3) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Shuffle copies of the cards in your opponent's hand into your deck. Draw two cards."
//!   Radiant: "Shuffle copies of the cards in your opponent's hand into your deck. Draw three cards."
//!
//! Built by C8 (#4592). R4592000, R4592002.

use jackioh_engine::effects::{draw, shuffle_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-186";

fn identity_theft(radiant: bool) -> Script {
    let draw_count = if radiant { 3 } else { 2 };
    Script {
        cry: Some(hook(move |ctx| {
            let opp = opponent_of(ctx.controller);
            let cards: Vec<_> = ctx.state.players[opp]
                .hand
                .iter()
                .map(|c| (c.id.clone(), c.def_id.clone(), c.radiant))
                .collect();
            let mut effects = Vec::new();
            for (id, def_id, card_radiant) in cards {
                effects.push(shuffle_into(json_as(json!({
                    "defId": def_id,
                    "count": 1,
                    "radiant": card_radiant,
                    "copyOf": id,
                    "unseen": true,
                }))));
            }
            effects.push(draw(json_as(json!({ "count": draw_count }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: identity_theft(false),
        radiant: identity_theft(true),
    }
}

// IA #186 Identity Theft — SPEC §8.13 row 186.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TARGET: &str = "core-016";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(5), "mana": 10 },
            "p2": { "hand": [TARGET] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn shuffles_copies_and_draws_two() {
            let mut s = board(false);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 2);
            let p1_library = &s.state().players[P1].library;
            assert!(p1_library.iter().any(|c| c.def_id == TARGET));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn shuffles_copies_and_draws_three() {
            let mut s = board(true);
            s.play(ID, json!({}));
            assert_eq!(s.hand(P1).len(), 3);
        }
    }
}
