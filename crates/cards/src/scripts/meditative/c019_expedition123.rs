//! Meditative #19 Expedition123 (SPEC §8.8 row 19). (1) Spell, Quickdraw, Epic. Expedition1234 until
//! patch v0.3.10 (#831), which renamed it and took a turn off its loss.
//!   Base:    "Lose all mana for your next {turns} turns. Add a Temporal Rift to your hand."
//!   Radiant: "Lose all mana for your next {turns} turns. Add a Radiant Temporal Rift to your hand."
//!
//! ME-TURN's lost refreshes through the next 2 turns (R844), then the Rift (M #19.1) into hand,
//! Radiant on the Radiant face. A full hand burns it (§2.4). An extra turn from the Rift counts as
//! one of the two, as Hearthstone's Overload lands on Time Warp's extra turn.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-019";

/// M #19.1 Temporal Rift, made by this card.
pub const RIFT_ID: &str = "meditative-019-1";

fn cry(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![
        lose_refreshes(json_as(json!({ "turns": param(&*ctx, "turns") }))),
        add_to_hand(json_as(json!({ "defId": RIFT_ID, "radiant": ctx.radiant }))),
    ]
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(cry)),
        ..Script::default()
    };
    // The Rift's face is the running face; everything else is the same script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative #19 Expedition123 — SPEC §8.8 row 19, BUILD M10 row M 19: Quickdraw; your next two
// refreshes give 0 mana and spend their riders, the third is normal; an extra turn counts among the
// two; mana gained in those turns still adds; adds a base Temporal Rift to hand, burned at a full
// hand; turns reads through `param()`; radiant the Rift is Radiant.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const EXPEDITION: &str = "meditative-019";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const COIN: &str = "core-t-coin"; // (0) Spell: gain 1 mana.

    /// p1's first turn (the scenario's default is turn 9), so its later turns are 3, 5, 7 and 9, and
    /// max mana reaches 4 on turn 9.
    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "expedition123",
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

    fn rifts(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|held| held.def_id == RIFT_ID).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r844_two_lost_then_normal() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            assert_eq!(rifts(&s).len(), 1);

            // Turns 3 and 5 are p1's lost refreshes; turn 7, its fourth, is normal.
            for turn in [3, 5] {
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().turn, turn, "on turn {turn}");
                assert_eq!(s.state().players.p1.mana.current, 0, "lost on turn {turn}");
            }
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 7);
            assert_eq!(s.state().players.p1.mana.current, 4);
            assert_eq!(s.state().players.p1.lost_refresh_through, None);
        }

        #[test]
        fn r844_an_extra_turn_counts_among_the_two() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            // The extra turn's refresh is the first of the two lost ones.
            s.state_mut().players.p1.extra_turns = Some(1);
            s.end_turn();
            // P1 again: the extra turn, still lost.
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.mana.current, 0);
            // The other lost refresh follows on p1's next turn, then normal.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 4);
            assert_eq!(s.state().players.p1.mana.current, 0, "lost on turn 4");
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 6);
            assert_eq!(s.state().players.p1.mana.current, 4);
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
        fn r844_each_lost_refresh_spends_its_rider() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            for turn in [3, 5] {
                s.state_mut().players.p1.mana.next_turn_mod = 2;
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().turn, turn);
                assert_eq!(s.state().players.p1.mana.current, 0, "lost on turn {turn}");
                assert_eq!(s.state().players.p1.mana.next_turn_mod, 0, "the rider spent on turn {turn}");
            }
        }

        #[test]
        fn r844_mana_gained_in_those_turns_still_adds() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, COIN, COIN]));
            s.play(EXPEDITION, json!({}));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
            s.play(COIN, json!({}));
            assert_eq!(s.state().players.p1.mana.current, 1, "the Coin adds on a lost turn");
        }

        #[test]
        fn r386_turns_tuned_reads_through_param() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            step_param(s.card_mut(EXPEDITION), "turns", -1);
            s.play(EXPEDITION, json!({}));
            // One lost refresh, turn 3; turn 5 is normal.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 3);
            assert_eq!(s.state().players.p1.mana.current, 0, "lost on turn 3");
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 5);
            let side = &s.state().players.p1;
            assert!(side.mana.current > 0);
            assert_eq!(side.mana.current, side.mana.max);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_adds_a_radiant_rift() {
            crate::register_all();
            let mut s = board(json!([{ "def": EXPEDITION, "radiant": true }]));
            s.play(EXPEDITION, json!({}));
            let added = rifts(&s);
            assert_eq!(added.len(), 1);
            assert!(added[0].radiant, "a Radiant Rift");
            // The loss is the same two turns.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
        }
    }
}
