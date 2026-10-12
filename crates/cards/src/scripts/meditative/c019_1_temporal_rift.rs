//! Meditative #19.1 Temporal Rift (SPEC §8.8 row 19.1, SPEC §7). (2) Spell, Token (printed Epic).
//!   Base:    "Take an extra turn after this one. You can take only one extra turn from Temporal Rift
//!             each game."
//!   Radiant: "Take an extra turn after this one. Gain {mana} mana and {nextMana} mana next turn. Draw
//!             {draw} cards. You can take only one extra turn from Temporal Rift each game."
//!
//! ME-TURN's extra turns (R845–R847): `take_extra_turn` owes the caster one unless their
//! once-a-game Rift flag is set, and sets it. A later Rift resolves its other effects and grants no
//! turn. The Radiant face's riders are temporary mana now, #24's next-turn rider (landing on the
//! extra turn) and a draw.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-019-1";

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let mut effects = vec![take_extra_turn(json_as(json!({ "rift": true })))];
    if ctx.radiant {
        effects.push(gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))));
        effects.push(next_turn_mana(json_as(json!({ "amount": param(&*ctx, "nextMana") }))));
        effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
    }
    effects
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(cry)),
        ..Script::default()
    };
    // The Radiant face's riders are its declared numbers; the turn is both faces' one verb.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #19.1 Temporal Rift — SPEC §8.8 row 19.1, SPEC §7, BUILD M10 row M 19.1: after the
// turn's cleanup the turn starts again — a whole turn, the turn count and max mana up, its refresh,
// Brittle tick, start-of-turn triggers and draw, Units no longer summoning sick; the 60-turn cap
// counts it; cast on the opponent's turn the extra turn follows the next one; a second Rift that
// game, either face or a copy, grants no turn and resolves the rest; each player has their own flag;
// the opponent's next-turn effects wait for their own turn; radiant also 2 temporary mana now, 2 next
// turn (on the extra turn) and draw 2, read through `param()`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const RIFT: &str = "meditative-019-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const WELL: &str = "core-006"; // (3) Field Spell: start of turn, gain 1 mana.

    use jackioh_engine::resolve::{CastOptions, cast_card};
    use jackioh_engine::triggers::{SettleOptions, settle};

    /// p1's first turn (the scenario's default is turn 9), so the extra turn is turn 2 and its max
    /// mana 2.
    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "temporal-rift",
            "turn": 1,
            "p1": {
                "hand": hand,
                "field": [VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r845_your_turn_starts_again_units_not_sick() {
            crate::register_all();
            let mut s = board(json!([RIFT]));
            let library = s.state().players.p1.library.len();
            s.play(RIFT, json!({}));
            s.end_turn();
            // P1 again: a whole turn — the count grew, max mana with it, and a card was drawn.
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().turn, 2);
            assert_eq!(s.state().players.p1.turns_started, 2);
            assert_eq!(s.state().players.p1.mana.max, 2);
            assert_eq!(s.state().players.p1.library.len(), library - 1);
            // The extra turn's banner turn carries the flag.
            let extras: Vec<Option<bool>> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::TurnStarted { extra, .. } => Some(*extra),
                    _ => None,
                })
                .collect();
            assert_eq!(extras.last(), Some(&Some(true)));
            // Units are no longer summoning sick on the extra turn.
            let unit = s.unit(P1, 1).expect("the unit stands");
            assert!(!is_sick(s.state(), &unit), "no longer sick on the extra turn");
            // Afterwards the game goes on: p2's turn follows the extra one.
            s.end_turn();
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r847_a_second_rift_resolves_but_grants_no_turn() {
            crate::register_all();
            let mut s = board(json!([RIFT, RIFT]));
            s.play(RIFT, json!({}));
            assert_eq!(s.state().players.p1.extra_turns, Some(1));
            s.play(RIFT, json!({}));
            assert_eq!(s.state().players.p1.extra_turns, Some(1), "no second grant");
            s.end_turn();
            assert_eq!(s.state().active, P1, "one extra turn");
            s.end_turn();
            assert_eq!(s.state().active, P2, "then the game goes on");
        }

        #[test]
        fn r845_the_extra_turn_ticks_brittle_and_runs_start_of_turn_triggers() {
            crate::register_all();
            let ticked = std::rc::Rc::new(std::cell::Cell::new(0));
            let seen = ticked.clone();
            mock_brittle_tick(move |sink: &mut EngineSink<'_>, player: PlayerId| {
                if player == PlayerId::P1 {
                    seen.set(sink.state.turn);
                }
            });
            let mut s = scenario(json!({
                "seed": "temporal-rift-start",
                "turn": 1,
                "p1": {
                    "hand": [RIFT],
                    "field": [VANILLA],
                    "backrow": [{ "def": WELL, "faceUp": true, "lane": 1 }],
                    "library": [FILLER, FILLER, FILLER, FILLER],
                    "mana": 10,
                },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
            }));
            s.play(RIFT, json!({}));
            s.end_turn();
            clear_seams();
            assert_eq!(s.state().active, P1);
            assert_eq!(ticked.get(), 2, "the Brittle stage ran on the extra turn");
            // Max mana 2, plus the Mana Well's start-of-turn 1.
            assert_eq!(s.state().players.p1.mana.current, 3);
        }

        #[test]
        fn r2_r845_the_turn_cap_counts_the_extra_turn() {
            crate::register_all();
            let mut s = board(json!([RIFT]));
            s.state_mut().turn = TURN_CAP_PLAYER_TURNS - 1;
            s.play(RIFT, json!({}));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().turn, TURN_CAP_PLAYER_TURNS);
            assert!(s.state().result.is_none());
            s.end_turn();
            let result = s.state().result.expect("the cap ends the game");
            assert_eq!(result.reason, GameOverReason::TurnCap);
        }

        #[test]
        fn r846_cast_on_the_opponents_turn_the_extra_turn_follows_your_next() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "temporal-rift-theirs",
                "turn": 2,
                "active": "p2",
                "p1": { "hand": [RIFT, FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER, FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER] },
            }));
            let rift = s.card(RIFT).clone();
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                cast_card(&mut sink, &rift, CastOptions { controller: Some(P1), ..Default::default() });
                settle(&mut sink, SettleOptions::default());
            }
            s.state_mut().rng_cursor = rng.cursor();
            assert_eq!(s.state().players.p1.extra_turns, Some(1));
            // p2's turn ends into p1's normal turn, which the extra turn follows.
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.extra_turns, Some(1));
            s.end_turn();
            assert_eq!(s.state().active, P1, "the extra turn");
            assert_eq!(s.state().players.p1.extra_turns, None);
            s.end_turn();
            assert_eq!(s.state().active, P2);
        }

        #[test]
        fn r847_a_radiant_second_rift_grants_no_turn_and_resolves_the_rest() {
            crate::register_all();
            let mut s = board(json!([RIFT, { "def": RIFT, "radiant": true }, FILLER]));
            let base = s.hand(P1).into_iter().find(|card| card.def_id == RIFT && !card.radiant).expect("base");
            let shiny = s.hand(P1).into_iter().find(|card| card.radiant).expect("radiant");
            s.play(&base.id, json!({}));
            let library = s.state().players.p1.library.len();
            s.play(&shiny.id, json!({}));
            assert_eq!(s.state().players.p1.extra_turns, Some(1), "no second grant");
            // Its riders still resolve: 2 drawn, 2 mana gained (10 - 2 - 2 + 2).
            assert_eq!(s.state().players.p1.library.len(), library - 2);
            assert_eq!(s.state().players.p1.mana.current, 8);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.end_turn();
            assert_eq!(s.state().active, P2, "only one extra turn");
        }

        #[test]
        fn r845_the_opponents_next_turn_effects_wait_for_their_own_turn() {
            crate::register_all();
            let mut s = board(json!([RIFT]));
            s.state_mut().players.p2.mana.next_turn_mod = 2;
            s.play(RIFT, json!({}));
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p2.mana.next_turn_mod, 2, "still waiting on the extra turn");
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let side = &s.state().players.p2;
            assert_eq!(side.mana.current, side.mana.max + 2, "it lands on p2's own turn");
            assert_eq!(side.mana.next_turn_mod, 0);
        }

        #[test]
        fn r847_each_player_has_a_flag() {
            crate::register_all();
            let mut s = board(json!([RIFT]));
            s.play(RIFT, json!({}));
            assert_eq!(s.state().players.p1.rift_extra_turn, Some(true));
            assert_eq!(s.state().players.p2.rift_extra_turn, None);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_gains_2_now_2_on_the_extra_turn_and_draws_2() {
            crate::register_all();
            let def = crate::card_def(RIFT);
            for (key, value) in [("mana", 2), ("nextMana", 2), ("draw", 2)] {
                let param = def.params.as_ref().expect("params").iter().find(|entry| entry.key == key).expect("the param");
                assert_eq!((param.base, param.radiant), (value, value), "for {key}");
            }
            let mut s = board(json!([{ "def": RIFT, "radiant": true }]));
            s.state_mut().players.p1.mana.current = 2;
            let hand = s.hand(P1).len();
            s.play(RIFT, json!({}));
            // 2 now and 2 drawn, the Rift itself leaving the hand.
            assert_eq!(s.state().players.p1.mana.current, 2);
            assert_eq!(s.hand(P1).len(), hand + 2 - 1);
            // The next-turn 2 lands on the extra turn's refresh.
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.mana.current, 4);
        }

        #[test]
        fn r386_radiant_numbers_read_through_param() {
            crate::register_all();
            let mut s = board(json!([{ "def": RIFT, "radiant": true }]));
            s.state_mut().players.p1.mana.current = 2;
            for key in ["mana", "nextMana", "draw"] {
                step_param(s.card_mut(RIFT), key, 1);
            }
            let hand = s.hand(P1).len();
            s.play(RIFT, json!({}));
            assert_eq!(s.state().players.p1.mana.current, 3);
            assert_eq!(s.hand(P1).len(), hand + 3 - 1);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            // Turn 2's max (2) plus the tuned 3.
            assert_eq!(s.state().players.p1.mana.current, 5);
        }
    }
}
