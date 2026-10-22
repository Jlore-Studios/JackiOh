//! Meditative #19 Expedition123 (SPEC §8.8 row 19). (1) Spell, Quickdraw, Epic. Expedition1234 until
//! patch v0.3.10 (#831), which renamed it and took a turn off its loss.
//!   Base:    "Lose ({loss}) mana next turn, then ({lossAfter}) mana the turn after. Add a Temporal Rift
//!             to your hand."
//!   Radiant: the same with a Radiant Temporal Rift.
//!
//! Two next-turn riders (§6.3 Mana, `next_turn_mana`), as Expedition12's (#1563; it was ME-TURN's lost
//! refreshes, R844, "Lose all mana for your next 2 turns", until then): the first lowers your next
//! refresh by `loss`; a delayed step at the start of your next turn (§10.1, R62: after that turn's
//! refresh) sets the second, `lossAfter`, on the refresh after it, carrying the number as it stood at
//! the cast. Each refresh floors at 0. Then the Rift (M #19.1) into hand, Radiant on the Radiant face; a
//! full hand burns it (§2.4). An extra turn taken next is "next turn", as Hearthstone's Overload lands
//! on Time Warp's extra turn.
//!
//! `script_v0_4_0f` keeps the card as it ran through v0.4.0f for its replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-019";

/// M #19.1 Temporal Rift, made by this card.
pub const RIFT_ID: &str = "meditative-019-1";

/// The delayed step that sets the second loss, and the number it carries.
const TURN_AFTER: &str = "turnAfter";
const LOSS_AFTER: &str = "lossAfter";

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![
        next_turn_mana(json_as(json!({ "amount": -param(&*ctx, "loss") }))),
        delay(json_as(json!({
            "at": { "phase": "start", "player": "self" },
            "step": TURN_AFTER,
            "hook": RESUME_HOOK,
            "data": { "lossAfter": param(&*ctx, LOSS_AFTER) },
            "next": true,
        }))),
        add_to_hand(json_as(json!({ "defId": RIFT_ID, "radiant": ctx.radiant }))),
    ]
}

/// The start of your next turn: the second loss rides on the refresh after it.
fn turn_after(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let loss = ctx.data.get(LOSS_AFTER).and_then(Value::as_i64).unwrap_or(0) as i32;
    if loss <= 0 {
        return vec![];
    }
    vec![next_turn_mana(json_as(json!({ "amount": -loss })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(cry)),
        resume: IndexMap::from([(TURN_AFTER, hook(turn_after))]),
        ..Script::default()
    };
    // The Rift's face is the running face; everything else is the same script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

/// R767000: Expedition123 through v0.4.0f: "Lose all mana for your next {turns} turns" (R844).
pub fn script_v0_4_0f() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            vec![
                lose_refreshes(json_as(json!({ "turns": param(&*ctx, "turns") }))),
                add_to_hand(json_as(json!({ "defId": RIFT_ID, "radiant": ctx.radiant }))),
            ]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #19 Expedition123 — SPEC §8.8 row 19, BUILD M10 row M 19: Quickdraw; your next refresh
// gives 2 less and the one after 3 less, each never below 0, the third normal; an extra turn taken
// next is the first; mana gained in those turns still adds; adds a base Temporal Rift to hand, burned
// at a full hand; both losses read through `param()`; radiant the Rift is Radiant.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const EXPEDITION: &str = "meditative-019";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const COIN: &str = "core-t-coin"; // (0) Spell: gain 1 mana.

    /// p1's turn 9 (the scenario's default), so its later turns have max mana 4.
    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "expedition123",
            "p1": {
                "hand": hand,
                "field": [VANILLA],
                "library": [FILLER, FILLER, FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER] },
        }))
    }

    fn rifts(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|held| held.def_id == RIFT_ID).collect()
    }

    /// p1's next turn: the opponent's passes, and p1's refresh and start of turn have run.
    fn next_turn(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
    }

    fn lost(s: &Scenario) -> i32 {
        let side = &s.state().players.p1;
        side.mana.max - side.mana.current
    }

    mod base {
        use super::*;

        #[test]
        fn loses_2_then_3_then_normal() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            assert_eq!(rifts(&s).len(), 1);
            next_turn(&mut s);
            assert_eq!(lost(&s), 2, "next turn");
            next_turn(&mut s);
            assert_eq!(lost(&s), 3, "the turn after");
            next_turn(&mut s);
            assert_eq!(lost(&s), 0, "then normal");
            assert_eq!(s.state().players.p1.lost_refresh_through, None);
        }

        #[test]
        fn each_refresh_floors_at_0() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "expedition123-floor",
                "turn": 1,
                "p1": { "hand": [EXPEDITION], "field": [VANILLA], "library": [FILLER, FILLER, FILLER], "mana": 10 },
                "p2": { "hand": [FILLER], "field": [VANILLA], "library": [FILLER, FILLER, FILLER] },
            }));
            s.play(EXPEDITION, json!({}));
            // Turns 3 and 5 have max mana 2 and 3: both losses take everything, never below 0.
            next_turn(&mut s);
            assert_eq!(s.state().players.p1.mana.current, 0);
            next_turn(&mut s);
            assert_eq!(s.state().players.p1.mana.current, 0);
        }

        #[test]
        fn an_extra_turn_taken_next_is_the_first() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            s.state_mut().players.p1.extra_turns = Some(1);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(lost(&s), 2, "the extra turn");
            next_turn(&mut s);
            assert_eq!(lost(&s), 3, "the turn after it");
            next_turn(&mut s);
            assert_eq!(lost(&s), 0);
        }

        #[test]
        fn adds_a_base_rift_and_burns_at_full_hand() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            let added = rifts(&s);
            assert_eq!(added.len(), 1);
            assert!(!added[0].radiant, "a base Rift");

            // A full hand burns the Rift: it lands in the graveyard instead.
            let mut s = board(json!([EXPEDITION]));
            let filler = s.card(FILLER).clone();
            for _ in 0..10 {
                s.state_mut().players.p1.hand.push(filler.clone());
            }
            s.play(EXPEDITION, json!({}));
            assert!(rifts(&s).is_empty(), "no room for the Rift");
            assert!(
                s.state().players.p1.graveyard.iter().any(|card| card.def_id == RIFT_ID),
                "the Rift burns to the graveyard"
            );
        }

        #[test]
        fn r640_it_is_quickdraw_on_both_faces() {
            crate::register_all();
            let scripts = script();
            for face in [&scripts.base, &scripts.radiant] {
                assert_eq!(face.static_flags.as_ref().and_then(|flags| flags.quickdraw), Some(true));
            }
            assert!(crate::card_def(EXPEDITION).tags.contains(&Tag::Quickdraw));
        }

        #[test]
        fn mana_gained_in_those_turns_still_adds() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, COIN, COIN]));
            s.play(EXPEDITION, json!({}));
            next_turn(&mut s);
            let before = s.state().players.p1.mana.current;
            s.play(COIN, json!({}));
            assert_eq!(s.state().players.p1.mana.current, before + 1, "the Coin adds");
        }

        #[test]
        fn s9_3_the_waiting_second_loss_survives_a_json_round_trip() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            let text = serde_json::to_string(s.state()).unwrap_or_default();
            let back: GameState = serde_json::from_str(&text).expect("the state round-trips");
            assert_eq!(&back, s.state());
            next_turn(&mut s);
            next_turn(&mut s);
            assert_eq!(lost(&s), 3);
        }

        #[test]
        fn r386_both_losses_read_through_param() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            // A step down is the Buff's direction for a loss (`better: down`), a step up the Nerf's.
            step_param(s.card_mut(EXPEDITION), "loss", -1);
            step_param(s.card_mut(EXPEDITION), "lossAfter", 1);
            s.play(EXPEDITION, json!({}));
            next_turn(&mut s);
            assert_eq!(lost(&s), 1);
            next_turn(&mut s);
            assert_eq!(lost(&s), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_adds_a_radiant_rift_and_loses_the_same() {
            crate::register_all();
            let mut s = board(json!([{ "def": EXPEDITION, "radiant": true }]));
            s.play(EXPEDITION, json!({}));
            let added = rifts(&s);
            assert_eq!(added.len(), 1);
            assert!(added[0].radiant, "a Radiant Rift");
            next_turn(&mut s);
            assert_eq!(lost(&s), 2);
            next_turn(&mut s);
            assert_eq!(lost(&s), 3);
        }
    }
}
