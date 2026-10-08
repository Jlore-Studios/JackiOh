//! Nothing happens after the game is over (SPEC §2.5, §4.5 step 2, R96, R216). Found by the polish-4
//! edge-case hunt, round 4 (docs/polish/4-edge-cases.md, lens "engine invariants"), which checked
//! seeded random games for a finished game that stays finished; every case here failed before its
//! fix, and the fuzz monitor's I5 now checks the same thing in every random game.
//!
//! Round 5 (lens L1) found /fullsend's Combo draws drawing on, one per rider, after a cast inside the
//! first had ended the game. Round 7 (lens "engine invariants") found a game conceded under an open
//! Discover still holding the prompt, which nothing could answer.
//!
//! The check that finds a hero at 0 or less ends the game at once. Whatever was still to resolve then
//! does not: the rest of the effect list the check ran inside, owed work, queued triggers, or a trap's
//! consumption after the AI turn it handed over has ended the game.
//!
//! Port of `packages/cards/test/game-over.test.ts` (SURFACE §4.1, §8).

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

const STOCKPILE: &str = "core-005";
const SHREDDER: &str = "core-013";
const MENACE: &str = "core-019";
const RENO: &str = "core-053";
const CN_VIRUS: &str = "core-090-1";
const MY_PAWN: &str = "core-096";
const VANILLA: &str = "core-008";
const HINDER: &str = "core-021";
const FULLSEND: &str = "core-078";
const SCARAB: &str = "core-007";
const POINTMASTER: &str = "core-020";
const PUNISH: &str = "classic-020";
const DOOM: &str = "destroy a Unit at the start of your next turn";

use super::scenario;

/// TS `lastEvents.map((event) => event.type)`.
fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

/// TS `types.slice(types.indexOf("gameOver") + 1)`: what follows the first `gameOver` (the whole list
/// when there is none, as `indexOf`'s -1 + 1 = 0 gives).
fn after_game_over(types: &[GameEventType]) -> Vec<GameEventType> {
    let from = types
        .iter()
        .position(|kind| *kind == GameEventType::GameOver)
        .map_or(0, |at| at + 1);
    types[from..].to_vec()
}

fn joined(types: &[GameEventType]) -> String {
    types
        .iter()
        .map(|kind| kind.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

mod r216_nothing_happens_after_the_game_is_over {
    use super::*;

    #[test]
    fn r216_5_stockpiles_heal_does_not_follow_the_cn_virus_cast_that_killed_its_hero_2_5_4_5_2_4() {
        // p1 at 1 health draws a CN-Virus off Stockpile's "draw 2": the cast deals p1 1 damage, and
        // §2.4's chain runs §4.5's check after that cast, which ends the game (§2.5). Stockpile's
        // "heal your hero 2" is the rest of a script whose game is already decided.
        let mut s = scenario(json!({
            "seed": "inv-r4-after-game-over",
            "p1": { "hand": [STOCKPILE], "health": 1, "library": [CN_VIRUS, RENO, MENACE] },
            "p2": { "hand": [RENO], "field": [MENACE] },
        }));
        s.play(STOCKPILE, json!({}));

        assert_eq!(
            s.state().result,
            Some(GameResult {
                winner: Winner::P2,
                reason: GameOverReason::HeroDeath
            })
        );
        let types = types_of(s.last_events());
        // gameOver is the last thing that happens: no heal, draw or anything else after it.
        assert_eq!(after_game_over(&types), Vec::<GameEventType>::new());
        // The state that says p1 lost by hero death still shows p1's hero dead.
        assert!(s.state().players.p1.hero.health <= 0);
    }

    #[test]
    fn r216_r152_96_my_pawn_is_not_consumed_after_the_ai_turn_it_handed_over_ended_the_game_2_5() {
        // p2's Jlockeed Shredder swings for lethal at p1 (2 health); My Pawn cancels it and hands the
        // rest of p2's turn to the AI (R44), which has nothing left but to end it. Shredder's
        // end-of-turn 2 damage kills p1 and the game is over (§2.5) — and nothing then consumes the trap.
        let mut s = scenario(json!({
            "seed": "inv-r4-pawn-after-game-over",
            "active": "p2",
            "p1": { "health": 2, "backrow": [{ "def": MY_PAWN, "faceUp": false }], "hand": [RENO] },
            "p2": { "field": [SHREDDER] },
        }));
        s.attack(SHREDDER, "hero");

        assert_eq!(
            s.state().result,
            Some(GameResult {
                winner: Winner::P2,
                reason: GameOverReason::HeroDeath
            })
        );
        let types = types_of(s.last_events());
        assert!(types.contains(&GameEventType::TrapFired));
        assert_eq!(after_game_over(&types), Vec::<GameEventType>::new());
    }

    #[test]
    fn r216_fullsends_combo_draws_stop_once_a_cast_on_draw_draw_inside_them_has_ended_the_game_2_5_2_4_10_5_step_5()
     {
        // Two Radiant /fullsends (the face with the Combo rider since patch v0.1.1) make two "Combo:
        // draw 1" riders. The Vanilla played after them owes two draws;
        // the first draws Hinder, which is cast (R70) and owes the same two draws of its own, both from an
        // empty library (fatigue 1, then 2), and the check after that cast finds p1 at 0 or less: the
        // game is over. The Vanilla's second Combo draw must not happen.
        let mut g = scenario(json!({
            "p1": {
                "hand": [{ "def": FULLSEND, "radiant": true }, { "def": FULLSEND, "radiant": true }, VANILLA],
                "mana": 10,
                "health": 2,
                // The Radiant Hinder discards nothing (R431), so its cast asks no question.
                "library": [VANILLA, { "def": HINDER, "radiant": true }],
            },
            "p2": { "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let fullsends: Vec<CardInstance> = g
            .hand(P1)
            .iter()
            .filter(|card| card.def_id == FULLSEND)
            .cloned()
            .collect();
        let (Some(first), Some(second)) = (fullsends.first(), fullsends.get(1)) else {
            panic!("setup: two /fullsends in hand");
        };
        g.play(&first.id, json!({}));
        g.play(&second.id, json!({}));
        g.play(VANILLA, json!({}));

        let types = types_of(g.last_events());
        let over = types.iter().position(|kind| *kind == GameEventType::GameOver);
        assert!(over.is_some(), "{}", joined(&types));
        assert_eq!(
            after_game_over(&types),
            Vec::<GameEventType>::new(),
            "{}",
            joined(&types)
        );
        assert_eq!(g.state().players.p1.fatigue_count, 2);
    }

    #[test]
    fn r216_r437_a_marked_unit_that_dies_with_its_hero_says_nothing_of_its_mark_after_game_over_2_5() {
        // Fuzz seed 329 (#562), cut down: p1's The Power to Punish marks p2's Pointmaster for a destroy at
        // the start of p1's next turn (R437's red mark). Shredder's end-of-turn 2 damage kills the
        // Pointmaster and p2's hero in one state check, which ends the game (§2.5). The destroy went
        // with its Unit, but the mark's `marked` (added: false) may not follow `gameOver`.
        let mut s = scenario(json!({
            "seed": "r216-marked-after-game-over",
            "p1": { "field": [SHREDDER], "backrow": [PUNISH] },
            "p2": { "health": 2, "field": [POINTMASTER] },
        }));
        let pointmaster = s.card(POINTMASTER).clone();
        s.activate(
            PUNISH,
            json!({ "modes": [DOOM], "targets": [{ "pick": "instance", "instanceId": pointmaster.id }] }),
        );
        assert!(
            s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::Marked { added: true, .. }))
        );
        s.end_turn();

        assert_eq!(
            s.state().result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath
            })
        );
        let types = types_of(s.last_events());
        assert!(types.contains(&GameEventType::Destroyed), "{}", joined(&types));
        assert_eq!(
            after_game_over(&types),
            Vec::<GameEventType>::new(),
            "{}",
            joined(&types)
        );
    }
}

mod r216_a_question_still_open_when_the_game_ends_is_closed_with_it {
    use super::*;

    #[test]
    fn r216_r211_a_game_that_ends_while_a_prompt_is_open_leaves_no_prompt_open_2_5() {
        let mut s = scenario(json!({
            "seed": "r7-concede-at-prompt",
            "p1": { "hand": [SCARAB], "mana": 4 },
            "p2": { "field": [POINTMASTER] },
        }));
        s.play(SCARAB, json!({}));
        assert!(matches!(s.view(P1).pending, Some(PendingView::ForYou(_))));

        // R211: the other seat may concede while p1's Discover is open.
        let over = reduce(s.state(), &Action::new(ActionBody::Concede, P2, "r7-concede"));
        assert_eq!(over.error, None);
        assert_eq!(
            over.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::Concede
            })
        );
        // Nothing can answer the Discover any more: legalActions offers nothing once the game is over...
        assert_eq!(legal_actions(&over.state, P1), Vec::<ActionBody>::new());
        // ...so the finished game does not still hold it open, nor show it to either seat, and
        // `gameOver` is the action's last event.
        assert!(over.state.pending.is_none());
        assert!(view_for(&over.state, P1).pending.is_none());
        assert!(view_for(&over.state, P2).pending.is_none());
        assert_eq!(
            over.events.last().map(GameEvent::event_type),
            Some(GameEventType::GameOver)
        );
    }
}
