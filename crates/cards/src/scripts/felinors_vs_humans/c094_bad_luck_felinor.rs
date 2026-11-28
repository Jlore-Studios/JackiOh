//! SPEC §8.14 FvH #94 Bad Luck Felinor ((2) Unit, Felinor, Common, 4/3 → 8/6), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Death: Shuffle {tokens|Felinor Token|Felinor Tokens} into your opponent's deck." (tokens 2)
//!   Radiant: "Reborn\nDeath: Shuffle {tokens|Felinor Token|Felinor Tokens} into your opponent's deck."
//!
//! The Death is `shuffle_into` of the declared `tokens` Felinor Tokens (`core-t-felinor`) into the enemy
//! deck, each at a random place; the tokens are base face on both faces, as the text names no Radiant one,
//! and a full deck takes what fits (R80, R316). The Radiant face's Reborn is catalog data, its order with
//! the Death R8's. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-094";

/// The token the Death shuffles in.
pub const FELINOR_TOKEN: &str = "core-t-felinor";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": FELINOR_TOKEN,
                "count": param(ctx, "tokens"),
                "player": "enemy",
            })))]
        })),
        ..Script::default()
    };
    // The same Death: the Radiant face's Reborn is catalog data and its tokens are not Radiant.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #94 Bad Luck Felinor — SPEC §8.14 row 94.
#[cfg(test)]
mod tests {
    use super::{FELINOR_TOKEN, ID};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [HIT_JOB, HIT_JOB, FILLER],
                "field": [{ "def": ID, "radiant": radiant }],
                "mana": 10,
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn kill(s: &mut Scenario) {
        let me = must(
            active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the Bad Luck Felinor on p1's field",
        );
        s.play(HIT_JOB, pick::targets(pick::instance(&me)));
    }

    fn tokens_in(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        s.pile(player, "library")
            .into_iter()
            .filter(|card| card.def_id == FELINOR_TOKEN)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn its_death_shuffles_2_felinor_tokens_into_the_opponents_deck() {
            let mut s = board(false);
            kill(&mut s);
            let shuffled = nonempty(tokens_in(&s, P2), "Felinor Tokens in p2's deck");
            assert_eq!(shuffled.len(), 2);
            for token in &shuffled {
                assert!(!token.radiant, "base face tokens");
            }
            assert!(tokens_in(&s, P1).is_empty(), "its own deck takes none");
            assert_eq!(s.pile(P2, "library").len(), 4 + 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_death_shuffles_base_face_tokens_and_the_reborn_bodys_death_shuffles_more() {
            let mut s = board(true);
            kill(&mut s);
            assert_eq!(tokens_in(&s, P2).len(), 2);
            // Reborn returned it; its second death shuffles two more.
            kill(&mut s);
            let shuffled = nonempty(tokens_in(&s, P2), "Felinor Tokens in p2's deck");
            assert_eq!(shuffled.len(), 4);
            for token in &shuffled {
                assert!(!token.radiant, "the text names no Radiant token");
            }
        }
    }
}
