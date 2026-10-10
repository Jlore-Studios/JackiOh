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

    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "temporal-rift",
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
    }
}
