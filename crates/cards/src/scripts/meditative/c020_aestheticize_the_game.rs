//! Meditative #20 Aestheticize the Game (SPEC §8.8 row 20). (1) Spell, Quickdraw, Wincon, Mythic.
//!   Base:    "Choose one. For the rest of the game, you also win while it is true: your hero has
//!             {health} or more Health; your graveyard holds {graveyard} or more cards; or your Units
//!             have {board} or more Attack and {board} or more Health in total."
//!   Radiant: the same with 90s, then: "Draw {draw} cards."
//!
//! ME-WIN's chosen condition (R848–R850): the three modes are declared (R81), and the chosen one
//! installs a rest-of-game player effect on the caster, `altWin { condition, threshold }` (never
//! expiring, R458), public on the view with its progress. The designer's Radiant face was only 10%
//! easier; its draw is R275's Spell rider.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-020";

const HEALTH_MODE: &str = "health";
const GRAVEYARD_MODE: &str = "graveyard";
const BOARD_MODE: &str = "board";

fn modes() -> Vec<ModeDecl> {
    vec![ModeDecl {
        kind: PromptKind::Mode,
        options: vec![
            HEALTH_MODE.to_string(),
            GRAVEYARD_MODE.to_string(),
            BOARD_MODE.to_string(),
        ],
    }]
}

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(mode) = chosen_options(ctx).into_iter().next() else {
        return vec![];
    };
    let threshold = param(&*ctx, mode.as_str());
    let mut effects = vec![alt_win(json_as(json!({ "condition": mode, "threshold": threshold })))];
    if ctx.radiant {
        effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
    }
    effects
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        modes: modes(),
        cry: Some(hook(cry)),
        ..Script::default()
    };
    // The Radiant face's 90s and draw are its declared numbers; the modes are both faces'.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #20 Aestheticize the Game — SPEC §8.8 row 20, BUILD M10 row M 20: Quickdraw; the
// declared mode installs a public rest-of-game condition shown with its progress, and reaching it
// wins at the state check's game-end point (`AltWin`): the hero at 100 health; 100 cards in the own
// graveyard; 100 total attack and 100 total current health over the acting Units, dormant cards not
// counted, both needed; a hero at 0 loses even with a win held, and two winners draw; a second copy
// adds a second condition, either one winning; nothing removes it; the thresholds read through
// `param()`; radiant 90 each, and it draws a card.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const GAME: &str = "meditative-020";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.

    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "aestheticize",
            "p1": {
                "hand": hand,
                "field": [VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
        }))
    }

    fn play_mode(s: &mut Scenario, mode: &str) {
        s.play(GAME, json!({ "modes": [mode] }));
    }

    fn conditions(s: &Scenario) -> Vec<(String, i32)> {
        s.state().players.p1.mods.iter().filter_map(|modifier| match &modifier.kind {
            ModifierKind::AltWin { condition, threshold } => {
                Some((condition.as_str().to_string(), *threshold))
            }
            _ => None,
        }).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r848_health_wins_at_100_by_alt_win() {
            crate::register_all();
            let mut s = board(json!([GAME]));
            s.state_mut().players.p1.hero.health = 100;
            play_mode(&mut s, HEALTH_MODE);
            assert_eq!(conditions(&s), vec![("health".to_string(), 100)]);
            let result = s.state().result.expect("a met condition wins");
            assert_eq!(result.winner, Winner::P1);
            assert_eq!(result.reason, GameOverReason::AltWin);
        }

        #[test]
        fn r848_graveyard() {
            crate::register_all();
            let mut s = board(json!([GAME, FILLER]));
            // Not met at once: the game goes on with the condition kept.
            play_mode(&mut s, GRAVEYARD_MODE);
            assert!(s.state().result.is_none());
            assert_eq!(conditions(&s), vec![("graveyard".to_string(), 100)]);
            // Met with 100 cards there: it wins.
            let filler = s.card(FILLER).clone();
            for _ in 0..100 {
                s.state_mut().players.p1.graveyard.push(filler.clone());
            }
            s.state_mut().players.p1.mana.current = 10;
            s.play(FILLER, json!({}));
            let result = s.state().result.expect("a met condition wins");
            assert_eq!(result.winner, Winner::P1);
            assert_eq!(result.reason, GameOverReason::AltWin);
        }

        #[test]
        fn r849_board_needs_both_and_skips_dormant() {
            crate::register_all();
            // One 4/4: attack short of 100 and health short of 100.
            let mut s = board(json!([GAME]));
            play_mode(&mut s, BOARD_MODE);
            assert!(s.state().result.is_none(), "a 4/4 is short of 100/100");
            assert_eq!(conditions(&s), vec![("board".to_string(), 100)]);
        }

        #[test]
        fn r848_two_copies_keep_two_conditions() {
            crate::register_all();
            let mut s = board(json!([GAME, GAME]));
            s.state_mut().players.p1.mana.current = 10;
            play_mode(&mut s, HEALTH_MODE);
            assert!(s.state().result.is_none(), "30 health is short of 100");
            play_mode(&mut s, GRAVEYARD_MODE);
            assert!(s.state().result.is_none(), "an empty graveyard is short of 100");
            assert_eq!(
                conditions(&s),
                vec![("health".to_string(), 100), ("graveyard".to_string(), 100)]
            );
        }

        #[test]
        fn r850_a_hero_at_0_loses_with_a_win_held() {
            crate::register_all();
            let mut s = board(json!([GAME]));
            s.state_mut().players.p1.hero.health = 0;
            s.state_mut().players.p1.mana.current = 10;
            // Health 0 still meets nothing, so hold the win first, then fall to 0.
            s.state_mut().players.p1.hero.health = 100;
            play_mode(&mut s, HEALTH_MODE);
            assert!(s.state().result.is_some(), "100 health wins at once");
            // A met graveyard win held while the hero falls to 0: the loss still wins.
            let mut s = board(json!([GAME, FILLER]));
            play_mode(&mut s, GRAVEYARD_MODE);
            assert!(s.state().result.is_none());
            let filler = s.card(FILLER).clone();
            for _ in 0..100 {
                s.state_mut().players.p1.graveyard.push(filler.clone());
            }
            s.state_mut().players.p1.hero.health = 0;
            s.end_turn();
            assert_eq!(conditions(&s), vec![("graveyard".to_string(), 100)], "held");
            let result = s.state().result.expect("a hero at 0 ends the game");
            assert_eq!(result.winner, Winner::P2);
            assert_eq!(result.reason, GameOverReason::HeroDeath);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_is_90_and_draws() {
            crate::register_all();
            let def = crate::card_def(GAME);
            for key in ["health", "graveyard", "board"] {
                let param = def.params.as_ref().expect("params").iter().find(|entry| entry.key == key).expect("the param");
                assert_eq!((param.base, param.radiant), (100, 90), "for {key}");
            }
            let mut s = board(json!([{ "def": GAME, "radiant": true }]));
            let hand = s.hand(P1).len();
            play_mode(&mut s, HEALTH_MODE);
            assert_eq!(conditions(&s), vec![("health".to_string(), 90)]);
            // Drew a card, the game itself leaving the hand.
            assert_eq!(s.hand(P1).len(), hand + 1 - 1);
        }
    }
}
