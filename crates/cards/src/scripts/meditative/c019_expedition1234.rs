//! Meditative #19 Expedition1234 (SPEC §8.8 row 19). (1) Spell, Quickdraw, Epic.
//!   Base:    "Lose all mana for your next {turns} turns. Add a Temporal Rift to your hand."
//!   Radiant: "Lose all mana for your next {turns} turns. Add a Radiant Temporal Rift to your hand."
//!
//! ME-TURN's lost refreshes through the next 3 turns (MD-A17), then the Rift (M #19.1) into hand,
//! Radiant on the Radiant face. A full hand burns it (§2.4). An extra turn from the Rift counts as
//! one of the three, as Hearthstone's Overload lands on Time Warp's extra turn.

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

// Meditative #19 Expedition1234 — SPEC §8.8 row 19, BUILD M10 row M 19: Quickdraw; your next three
// refreshes give 0 mana and spend their riders, the fourth is normal; an extra turn counts among the
// three; mana gained in those turns still adds; adds a base Temporal Rift to hand, burned at a full
// hand; turns reads through `param()`; radiant the Rift is Radiant.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EXPEDITION: &str = "meditative-019";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.

    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "expedition1234",
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
        fn r844_three_lost_then_normal() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            assert_eq!(rifts(&s).len(), 1);

            // Turns 3, 5 and 7 are p1's lost refreshes; turn 9 is normal.
            for turn in [3, 5, 7] {
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().turn, turn, "on turn {turn}");
                assert_eq!(s.state().players.p1.mana.current, 0, "lost on turn {turn}");
            }
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 9);
            assert_eq!(s.state().players.p1.mana.current, 4);
            assert_eq!(s.state().players.p1.lost_refresh_through, None);
        }

        #[test]
        fn r844_an_extra_turn_counts_among_the_three() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            // The extra turn's refresh is the first of the three lost ones.
            s.state_mut().players.p1.extra_turns = Some(1);
            s.end_turn();
            // P1 again: the extra turn, still lost.
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.mana.current, 0);
            // The other two lost refreshes follow on p1's next turns, then normal.
            for turn in [4, 6] {
                s.end_turn();
                s.end_turn();
                assert_eq!(s.state().turn, turn, "on turn {turn}");
                assert_eq!(s.state().players.p1.mana.current, 0, "lost on turn {turn}");
            }
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().turn, 8);
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
        fn turns_reads_through_param() {
            crate::register_all();
            let def = crate::card_def(EXPEDITION);
            let param = def.params.as_ref().expect("params").iter().find(|entry| entry.key == "turns").expect("turns");
            assert_eq!((param.base, param.radiant), (3, 3));
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
            // The loss is the same three turns.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
        }
    }
}
