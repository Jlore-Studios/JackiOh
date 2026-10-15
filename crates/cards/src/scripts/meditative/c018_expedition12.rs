//! Meditative #18 Expedition12 (SPEC §8.8 row 18). (1) Spell, Quickdraw, Epic.
//!   Base:    "Lose all mana next turn. Make a card in your hand Radiant."
//!   Radiant: "Echo 1. Lose all mana next turn. Make a card in your hand Radiant."
//!
//! ME-TURN's lost refresh (R844) and a declared hand pick (R81, as C+ #41's): your other
//! non-Radiant hand cards, made Radiant (§5.2). With no other non-Radiant hand card the pick has
//! nothing and the Spell still resolves (§8 Conventions). The Radiant face's Echo repeat reopens
//! the pick as a `hand` prompt (§10.6); its mana loss is the same next turn, so it changes nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-018";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::hand(
        1,
        1,
        json!({ "of": ["hand"], "excludeSelf": true, "check": "notRadiant" }),
    )]
}

fn target_checks() -> IndexMap<&'static str, TargetCheck> {
    IndexMap::from([(
        "notRadiant",
        target_check(|a| a.candidate.is_some_and(|candidate| !candidate.radiant)),
    )])
}

fn cry() -> Vec<Effect> {
    vec![
        lose_refreshes(json_as(json!({ "turns": 1 }))),
        set_radiant(json_as(json!({ "target": { "of": "chosen" } }))),
    ]
}

fn face(echo: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            echo: echo.then_some(1),
            ..StaticFlags::default()
        }),
        targets: targets(),
        target_checks: target_checks(),
        cry: Some(hook(|_| cry())),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// Meditative #18 Expedition12 — SPEC §8.8 row 18, BUILD M10 row M 18: Quickdraw; your next turn's
// refresh gives 0 mana and spends any next-turn rider, while mana gained during that turn still adds;
// the turn after is normal; an extra turn taken next is the lost one; the declared pick is another
// non-Radiant card in your hand, made Radiant; with none it still resolves; radiant Echo 1: a second
// hand pick, also made Radiant, and still one lost turn.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const EXPEDITION: &str = "meditative-018";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile.
    const COIN: &str = "core-t-coin"; // (0) Spell: gain 1 mana.

    /// A board where p1 can play the expedition and neither side auto-ends its turn. It is p1's
    /// first turn (the scenario's default is turn 9), so its later turns are 3 and 5, with max mana
    /// 2 and 3.
    fn board(hand: Value) -> Scenario {
        scenario(json!({
            "seed": "expedition12",
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

    fn pick(s: &Scenario, card: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": s.card(card).id }] })
    }

    /// The hand cards `legalActions` offers as the expedition's pick.
    fn offered(s: &Scenario) -> Vec<String> {
        let expedition_id = s.card(EXPEDITION).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == expedition_id => {
                    targets
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|target| match target {
                            Selection::Instance { instance_id } => Some(instance_id),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                }
                _ => vec![],
            })
            .collect()
    }

    fn radiant_hand(state: &GameState) -> Vec<String> {
        state.players.p1.hand.iter().filter(|card| card.radiant).map(|card| card.id.clone()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r844_next_turn_gives_0_spends_the_rider_and_the_turn_after_is_normal() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER]));
            s.state_mut().players.p1.mana.next_turn_mod = 2;
            s.play(EXPEDITION, pick(&s, FILLER));
            assert_eq!(radiant_hand(s.state()), vec![s.card(FILLER).id.clone()]);

            // Turn 3 is p1's next turn: 0 mana, the rider spent with it.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
            assert_eq!(s.state().players.p1.mana.next_turn_mod, 0);

            // Turn 5 is normal again.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 3);
        }

        #[test]
        fn r844_mana_gained_still_adds() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER]));
            s.play(EXPEDITION, pick(&s, FILLER));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
            s.state_mut().players.p1.mana.current += 2;
            assert_eq!(s.state().players.p1.mana.current, 2);
        }

        #[test]
        fn the_pick_becomes_radiant_and_offers_no_radiant_card() {
            crate::register_all();
            let mut s = board(json!([
                EXPEDITION,
                FILLER,
                { "def": VANILLA, "radiant": true },
            ]));
            let offered_ids = offered(&s);
            assert!(!offered_ids.contains(&s.card(EXPEDITION).id.clone()));
            let shiny = s.hand(P1).into_iter().find(|held| held.radiant).expect("the Radiant card").id.clone();
            assert!(!offered_ids.contains(&shiny), "no Radiant card is offered");
            s.play(EXPEDITION, pick(&s, FILLER));
            assert!(s.card(FILLER).radiant, "the pick becomes Radiant");
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
        fn r844_mana_gained_during_the_lost_turn_still_adds() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER, COIN]));
            s.play(EXPEDITION, pick(&s, FILLER));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
            // The Coin's gain is an effect, not the refresh: it adds.
            s.play(COIN, json!({}));
            assert_eq!(s.state().players.p1.mana.current, 1);
        }

        #[test]
        fn r844_the_turn_after_the_lost_one_is_normal() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER]));
            s.play(EXPEDITION, pick(&s, FILLER));
            for _ in 0..4 {
                s.end_turn();
            }
            assert_eq!(s.state().turn, 5);
            let side = &s.state().players.p1;
            assert_eq!(side.mana.current, side.mana.max);
            assert_eq!(side.mana.current, 3);
            assert_eq!(side.lost_refresh_through, None);
        }

        #[test]
        fn r844_r845_an_extra_turn_taken_next_is_the_lost_one() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION, FILLER]));
            s.play(EXPEDITION, pick(&s, FILLER));
            s.state_mut().players.p1.extra_turns = Some(1);
            s.end_turn();
            // P1 again: the extra turn's refresh is the lost one.
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.mana.current, 0);
            // p2's turn, then p1's next is normal.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            assert_eq!(s.state().players.p1.mana.current, 3);
        }

        #[test]
        fn with_no_other_card_it_still_resolves() {
            crate::register_all();
            let mut s = board(json!([EXPEDITION]));
            s.play(EXPEDITION, json!({}));
            s.expect_in_zone(EXPEDITION, "graveyard");
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn radiant_echo_1_reopens_a_hand_pick_and_still_loses_one_turn() {
            crate::register_all();
            let mut s = board(json!([{ "def": EXPEDITION, "radiant": true }, FILLER, VANILLA]));
            let flags = jackioh_engine::scripts::flags_of(s.state(), s.card(EXPEDITION));
            assert_eq!(flags.echo, Some(1));
            s.play(EXPEDITION, pick(&s, FILLER));
            // The Echo repeat reopens the pick as a hand prompt.
            assert!(s.state().pending.is_some(), "a second hand pick opens");
            let vanilla = s.card(VANILLA).id.clone();
            s.answer(json!([{ "pick": "instance", "instanceId": vanilla }]));
            assert!(s.card(FILLER).radiant, "the first pick becomes Radiant");
            assert!(s.card(VANILLA).radiant, "the second pick becomes Radiant");

            // Still one lost turn: the turn after is normal.
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 0);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.mana.current, 3);
        }
    }
}
