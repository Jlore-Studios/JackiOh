//! Meditative #20 Aestheticize the Game (SPEC §8.8 row 20). (1) Spell, Quickdraw, Wincon, Mythic.
//!   Base:    "Choose one. You also win while it's true: your hero has {health}+ Health; your graveyard
//!             has {graveyard}+ cards; or your Units total {board}+ Attack and {board}+ Health."
//!   Radiant: the same with 90s, then: "Draw {draw}."
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
    const FIENDER: &str = "core-092"; // (2) Unit with Stack.

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

    mod more {
        use super::*;

        fn board_of(field: Value) -> Scenario {
            crate::register_all();
            scenario(json!({
                "seed": "aestheticize-board",
                "p1": { "hand": [GAME, FILLER], "field": field, "library": [FILLER, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            }))
        }

        #[test]
        fn r640_it_is_quickdraw_on_both_faces() {
            crate::register_all();
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(face.static_flags.as_ref().and_then(|flags| flags.quickdraw), Some(true));
            }
            assert!(crate::card_def(GAME).tags.contains(&Tag::Quickdraw));
        }

        #[test]
        fn r848_the_condition_is_public_with_its_progress() {
            crate::register_all();
            let mut s = board(json!([GAME]));
            play_mode(&mut s, HEALTH_MODE);
            for viewer in [P1, PlayerId::P2] {
                let view = s.view(viewer);
                let side = if viewer == P1 { &view.you } else { &view.opponent };
                let labels: Vec<String> = side.modifiers.iter().map(|badge| badge.label.clone()).collect();
                assert!(
                    labels.contains(&"You win at 100 hero Health (30/100)".to_string()),
                    "{viewer:?} sees it: {labels:?}"
                );
            }
        }

        #[test]
        fn r850_it_wins_only_once_its_effect_list_closes() {
            crate::register_all();
            // The Radiant face: the win is met at once, and the draw after it still resolves.
            let mut s = board(json!([{ "def": GAME, "radiant": true }]));
            s.state_mut().players.p1.hero.health = 90;
            let library = s.state().players.p1.library.len();
            play_mode(&mut s, HEALTH_MODE);
            assert_eq!(s.state().players.p1.library.len(), library - 1, "the draw resolved first");
            let result = s.state().result.expect("then the win");
            assert_eq!(result.reason, GameOverReason::AltWin);
            assert_eq!(result.winner, Winner::P1);
        }

        #[test]
        fn r849_100_attack_and_100_health_win() {
            let mut s = board_of(json!([{ "def": VANILLA, "statsOverride": { "attack": 100, "health": 100 } }]));
            s.play(GAME, json!({ "modes": [BOARD_MODE] }));
            let result = s.state().result.expect("100/100 wins");
            assert_eq!(result.winner, Winner::P1);
            assert_eq!(result.reason, GameOverReason::AltWin);
        }

        #[test]
        fn r849_both_totals_are_needed() {
            let mut s = board_of(json!([{ "def": VANILLA, "statsOverride": { "attack": 100, "health": 99 } }]));
            s.play(GAME, json!({ "modes": [BOARD_MODE] }));
            assert!(s.state().result.is_none(), "99 health is short");
            let mut s = board_of(json!([{ "def": VANILLA, "statsOverride": { "attack": 99, "health": 100 } }]));
            s.play(GAME, json!({ "modes": [BOARD_MODE] }));
            assert!(s.state().result.is_none(), "99 attack is short");
        }

        #[test]
        fn r849_current_health_and_dormant_cards_not_counted() {
            // A damaged 100/100 counts its current health.
            let mut s = board_of(json!([{ "def": VANILLA, "statsOverride": { "attack": 100, "health": 100 }, "damage": 1 }]));
            s.play(GAME, json!({ "modes": [BOARD_MODE] }));
            assert!(s.state().result.is_none(), "99 current health is short");
            // A 100/100 dormant under a Stack pile counts nothing.
            let mut s = board_of(json!([
                { "def": VANILLA, "statsOverride": { "attack": 100, "health": 100 } },
                { "def": FIENDER, "stack": true },
            ]));
            s.play(GAME, json!({ "modes": [BOARD_MODE] }));
            assert!(s.state().result.is_none(), "the dormant card is not counted");
        }

        #[test]
        fn r850_two_winners_draw() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "aestheticize-draw",
                "active": "p2",
                "p1": { "hand": [GAME, FILLER], "library": [FILLER, FILLER], "mana": 10 },
                "p2": { "hand": [GAME, FILLER], "library": [FILLER, FILLER], "mana": 10 },
            }));
            s.play(GAME, json!({ "modes": [GRAVEYARD_MODE] }));
            assert!(s.state().result.is_none());
            s.end_turn();
            assert_eq!(s.state().active, P1);
            // p2's graveyard reaches 100 and p1's hero 100 with no check between.
            let filler = s.pile(P1, "library")[0].clone();
            for _ in 0..100 {
                s.state_mut().players.p2.graveyard.push(filler.clone());
            }
            s.state_mut().players.p1.hero.health = 100;
            s.play(GAME, json!({ "modes": [HEALTH_MODE] }));
            let result = s.state().result.expect("two winners");
            assert_eq!(result.winner, Winner::Draw);
            assert_eq!(result.reason, GameOverReason::AltWin);
        }

        #[test]
        fn r848_either_of_two_conditions_wins() {
            crate::register_all();
            let mut s = board(json!([GAME, GAME, FILLER]));
            s.state_mut().players.p1.mana.current = 10;
            play_mode(&mut s, HEALTH_MODE);
            play_mode(&mut s, GRAVEYARD_MODE);
            assert!(s.state().result.is_none());
            // The second one met, the first still unmet: it wins.
            let filler = s.card(FILLER).clone();
            for _ in 0..100 {
                s.state_mut().players.p1.graveyard.push(filler.clone());
            }
            s.play(FILLER, json!({}));
            let result = s.state().result.expect("either one wins");
            assert_eq!(result.winner, Winner::P1);
            assert_eq!(result.reason, GameOverReason::AltWin);
        }

        #[test]
        fn r848_nothing_removes_it_its_card_leaving_included() {
            crate::register_all();
            let mut s = board(json!([GAME]));
            play_mode(&mut s, HEALTH_MODE);
            let card = s.card(GAME).id.clone();
            s.expect_in_zone(&card, "graveyard");
            // The card leaves its graveyard and turns go by: the condition stays.
            s.state_mut().players.p1.graveyard.retain(|held| held.id != card);
            for _ in 0..4 {
                s.end_turn();
            }
            assert_eq!(conditions(&s), vec![("health".to_string(), 100)]);
        }

        #[test]
        fn r386_the_thresholds_read_through_param() {
            for (mode, key) in [(HEALTH_MODE, "health"), (GRAVEYARD_MODE, "graveyard"), (BOARD_MODE, "board")] {
                crate::register_all();
                let mut s = board(json!([GAME]));
                step_param(s.card_mut(GAME), key, -1);
                play_mode(&mut s, mode);
                assert_eq!(conditions(&s), vec![(key.to_string(), 90)], "for {key}");
            }
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
