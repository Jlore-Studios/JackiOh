//! SPEC §8.12 S+ #T-the-enigmata The Enigmata (4) Spell, Terminus, Mythic, a Terminus token of the held
//! Stinkyscape+ set (R1420, R3535000; R3569004).
//!   Base:    "Players take turns casting Jogg's box until the game ends."
//!   Radiant: "Your hero gains Armor {count}, then players take turns casting Jogg's box until the game
//!            ends."
//!
//! Engine: a `forRestOfGame` for each player, at the start of each of their own turns (R458), casts
//! a fresh Classic+ #47 Jogg's Box as that player: every choice of the cast is random (R452), so nothing
//! pauses for a prompt. Its "End the turn" (R456) ends that turn at once, so the turns play themselves,
//! the players alternating, until a hero dies or the turn cap draws the game (R2); it is not a loop of
//! casts inside one turn, which could never end within the match's action cap (R3569004). The Box is
//! the base card, and the Radiant face's Armor is given first.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-t-the-enigmata";

/// Classic+ #47 Jogg's Box: "Cast 10 random Spells. End the turn."
const JOGGS_BOX: &str = "classicplus-047";
/// The rest-of-game effects' step (§10.6).
const STEP: &str = "jogg";
/// The badge each side carries for the rest of the game.
const BADGE: &str = "The Enigmata: cast a Jogg's Box at the start of your turn.";

fn enigmata(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = Vec::new();
            if radiant {
                effects.push(gain_hero_armor(json_as(json!({ "amount": param(ctx, "count") }))));
            }
            effects.push(for_rest_of_game(json_as(json!({ "step": STEP, "label": BADGE }))));
            effects.push(for_rest_of_game(json_as(json!({
                "step": STEP,
                "label": BADGE,
                "player": "enemy",
            }))));
            effects
        })),
        delayed: Some(hook(|_ctx| {
            vec![cast_new(json_as(json!({ "def": JOGGS_BOX, "random": true })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: enigmata(false),
        radiant: enigmata(true),
    }
}

// S+ #T-the-enigmata — SPEC §8.12 (R3569004; R452, R456, R458, R2).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn game(seed: &str, radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": filler(40),
                "mana": 4,
                "health": 30,
            },
            "p2": { "library": filler(40), "health": 30 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// Plays the turns out: the random casts may reach a card that asks a question of its own
    /// (Meditative+ #20's quiz), which any prompt's policy answers; here its first option.
    fn play_out(s: &mut Scenario) {
        for _ in 0..200 {
            if s.state().result.is_some() {
                return;
            }
            let Some(pending) = s.state().pending.clone() else {
                return;
            };
            s.answer(json!([crate::js(&pending.options[0].selection)]));
        }
        panic!("the game did not end");
    }

    /// Who cast each Jogg's Box in `events`, in order.
    fn boxes(s: &Scenario) -> Vec<PlayerId> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed { player, def_id, .. } if def_id == JOGGS_BOX => Some(*player),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3569004_each_player_casts_a_joggs_box_at_their_turn_start() {
            let mut s = game("enigmata-base", false);
            s.play(ID, json!({}));
            assert!(boxes(&s).is_empty(), "nothing is cast until a turn starts");
            assert_eq!(
                s.state().players[P1].hero.armor,
                0,
                "the base face gives no Armor"
            );
            s.end_turn();
            play_out(&mut s);
            let casters = boxes(&s);
            assert!(casters.len() >= 2, "{casters:?}");
            assert_eq!(casters[0], P2, "the opponent's turn starts first");
            for pair in casters.windows(2) {
                assert_ne!(pair[0], pair[1], "the players take turns: {casters:?}");
            }
            assert!(
                s.state().result.is_some(),
                "the turns play themselves until the game ends"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569004_radiant_enigmata_gives_armor_first() {
            let mut s = game("enigmata-radiant", true);
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].hero.armor, 5);
            assert_eq!(s.state().players[P2].hero.armor, 0, "only the caster's hero");
            assert!(boxes(&s).is_empty());
            s.end_turn();
            play_out(&mut s);
            assert_eq!(boxes(&s).first(), Some(&P2));
            assert!(s.state().result.is_some());
        }
    }
}
