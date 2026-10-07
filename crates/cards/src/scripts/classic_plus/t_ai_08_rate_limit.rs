//! T-AI-8 Rate Limit (SPEC §8.7 row T-AI-8, §7, B8). (1) Trap, AI, Token.
//!   Base:    "Activates when your opponent plays their 3rd card in a turn: After it resolves, their
//!            turn ends."
//!   Radiant: "… their 2nd card …"
//!   Engine:  "Counts the opponent's plays that turn (`turnLog.cardsPlayed`, casts included, R70; a
//!            countered card is never played) and fires as the 3rd (Radiant 2nd) is played
//!            (`cardPlayed`, §10.5 step 4), and on that play only, going to the graveyard as it fires
//!            (§3.2). The play that set it off resolves first (§10.5 step 7); then End the turn (§6.3,
//!            `turnCutShort`): the rest of that effect list resolves and the turn ends as if they had
//!            pressed End turn, every end-of-turn step running. Tunes: none."
//!
//! The condition is `when` (R99): any other play leaves it armed and face-down. §10.5 step 4 counts the
//! play before it emits `cardPlayed`, so `cards_played_this_turn` already counts the one that woke it, and
//! "that play only" is the count being exactly N — on their own turn, the only turn of theirs there is to
//! end (casts they make on yours leave it set). E10's `end_turn` on the opponent puts R456's rider on
//! their turn, which ends once everything their action set off has resolved, the play's Cry included.

use jackioh_engine::effects::end_turn;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-t-ai-08";

/// TS's `{ base, radiant } as const`: one number per face.
#[derive(Clone, Copy)]
struct ByFacePlay {
    base: i32,
    radiant: i32,
}

/// §8.7: "their 3rd card", Radiant "their 2nd". An AI card declares no params (B8).
const NTH_PLAY: ByFacePlay = ByFacePlay { base: 3, radiant: 2 };

/// TS `TrapTrigger`: part 1's `TriggerDef`, which carries the `when` (R99).
fn rate_limit(nth: i32) -> TriggerDef {
    TriggerDef::new("rate-limit", &[GameEventType::CardPlayed], |_ctx, _event| {
        vec![end_turn(json_as(json!({ "player": "enemy" })))]
    })
    .with_when(move |ctx, event| match event {
        GameEvent::CardPlayed { player, .. } => {
            *player != ctx.controller
                && ctx.state.active == *player
                && cards_played_this_turn(&*ctx.state, *player) == nth
        }
        _ => false,
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![rate_limit(NTH_PLAY.base)],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![rate_limit(NTH_PLAY.radiant)],
            ..Script::default()
        },
    }
}

// T-AI-8 Rate Limit — SPEC §8.7 row T-AI-8, BUILD M9 Classic+ row T-AI-8: "Face-down Trap: on the
// opponent's turn it fires as their 3rd play of the turn is played (`cardPlayed`; casts count, R70; a
// countered play is never played), going to your graveyard, and once that play has resolved their turn
// ends as if they had pressed End turn, every end-of-turn step running (`endTurnAfter`, §6.3); the
// opponent learns nothing of it until it fires (R33, R97); radiant after their 2nd play".
//
// Every case sets the trap face-down in p1's backrow and makes p2 the active player.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const RATE_LIMIT: &str = "classicplus-t-ai-08";
    const REFUSAL: &str = "classicplus-t-ai-09";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const STOCKPILE: &str = "core-005"; // (1) Spell: Draw 2. Heal your hero 2.
    const REPLENISH: &str = "core-010"; // (0) Spell
    const SHREDDER: &str = "core-013"; // (3) Unit: End of turn: deal 2 damage to each enemy Unit and the enemy hero.
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw
    const SCARAB: &str = "core-007"; // (1) Unit: Cry: Discover a (2) Cost card
    const HIT_JOB: &str = "core-016"; // Spell: destroy a target Unit

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

    /// TS's `{ ...base, ...extra }` on a side setup.
    fn spread(base: Value, extra: &Value) -> Value {
        let mut out = base;
        if let (Some(into), Some(from)) = (out.as_object_mut(), extra.as_object()) {
            for (key, value) in from {
                into.insert(key.clone(), value.clone());
            }
        }
        out
    }

    fn trap(radiant_face: bool, lane: i32) -> Value {
        json!({ "def": RATE_LIMIT, "radiant": radiant_face, "faceUp": false, "lane": lane })
    }

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "active": "p2",
            "p1": spread(
                json!({ "hand": [VANILLA], "backrow": [trap(radiant_face, 2)], "library": [VANILLA, VANILLA, VANILLA] }),
                &p1,
            ),
            "p2": spread(
                json!({
                    "hand": [VANILLA, REPLENISH, STOCKPILE, VANILLA],
                    "library": [VANILLA, VANILLA, VANILLA, VANILLA],
                    "mana": 8,
                }),
                &p2,
            ),
        }))
    }

    fn count(events: &[GameEvent], kind: &str) -> usize {
        events.iter().filter(|event| js(*event)["type"] == kind).count()
    }

    fn view_text(s: &Scenario, player: PlayerId) -> String {
        serde_json::to_string(&s.view(player)).expect("serialises")
    }

    mod t_ai_8_rate_limit {
        use super::*;

        #[test]
        fn is_a_1_ai_trap_token_whose_condition_lives_in_when_r99() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.type_, CardType::Trap);
            assert_eq!(js(&def.cost), json!(1));
            assert_eq!(js(&def.tags), json!(["AI", "Token"]));
            let scripts = super::super::script();
            assert!(scripts.base.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
            assert!(scripts.radiant.triggers.first().is_some_and(|trigger| trigger.when.is_some()));
        }

        mod base {
            use super::*;

            #[test]
            fn r99_their_1st_and_2nd_plays_leave_it_armed_and_face_down() {
                let mut s = setup(json!({}), json!({}), false);
                s.play(VANILLA, json!({ "zone": 1 })).play(REPLENISH, json!({}));
                assert_eq!(s.state().active, P2);
                assert_eq!(s.card(RATE_LIMIT).face_up, Some(false));
                assert_eq!(count(s.events(), "trapFired"), 0);
            }

            #[test]
            fn r456_fires_as_their_3rd_play_is_played_the_play_resolves_first_then_their_turn_ends() {
                let mut s = setup(json!({}), json!({}), false);
                s.play(VANILLA, json!({ "zone": 1 })).play(REPLENISH, json!({})).play(STOCKPILE, json!({}));

                s.expect_in_zone(RATE_LIMIT, "graveyard");
                // Stockpile resolved in full (draw 2, heal 2) before the turn ended.
                s.expect_health(P2, 32);
                s.expect_events(json!(["cardPlayed", "trapFired", "cardResolved", "turnCutShort", "turnEnded", "turnStarted"]));
                assert_eq!(s.state().active, P1);
            }

            #[test]
            fn r62_r456_every_end_of_turn_step_of_theirs_runs_as_if_they_had_pressed_end_turn() {
                let mut s = setup(json!({}), json!({ "field": [SHREDDER] }), false);
                s.play(VANILLA, json!({ "zone": 2 })).play(REPLENISH, json!({})).play(STOCKPILE, json!({}));
                assert_eq!(s.state().active, P1);
                // Shredder's end-of-turn hit on p1's hero.
                s.expect_health(P1, 28);
            }

            #[test]
            fn your_own_plays_never_set_it_off() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [VANILLA, REPLENISH, STOCKPILE, VANILLA],
                        "backrow": [trap(false, 2)],
                        "library": [VANILLA, VANILLA],
                        "mana": 8,
                    },
                    "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
                }));
                s.play(VANILLA, json!({ "zone": 1 })).play(REPLENISH, json!({})).play(STOCKPILE, json!({}));
                assert_eq!(s.state().active, P1);
                assert_eq!(s.card(RATE_LIMIT).face_up, Some(false));
            }

            #[test]
            fn r70_a_cast_is_a_play_a_cast_on_draw_card_their_2nd_play_draws_is_their_3rd_play() {
                let mut s = setup(
                    json!({}),
                    json!({ "library": [{ "def": HINDER, "radiant": true }, VANILLA, VANILLA, VANILLA] }),
                    false,
                );
                s.play(VANILLA, json!({ "zone": 1 })).play(STOCKPILE, json!({}));
                s.expect_in_zone(RATE_LIMIT, "graveyard");
                assert_eq!(s.state().active, P1);
                // Stockpile's whole list still resolved: both draws (the cast one and its replacement) and the heal.
                s.expect_health(P2, 32);
            }

            #[test]
            fn r158_r456_a_3rd_play_that_asks_pauses_the_end_answered_after_a_json_round_trip_the_play_finishes_and_then_the_turn_ends()
             {
                // The 3rd play is Scarab, whose Cry Discovers: the question pauses the turn the trap already
                // ended. (Base Hinder used to be the asker here; since R682 its discard is random, so a draw
                // that casts it asks nothing.)
                let mut s = setup(json!({}), json!({ "hand": [VANILLA, REPLENISH, SCARAB, VANILLA] }), false);
                s.play(VANILLA, json!({ "zone": 1 })).play(REPLENISH, json!({})).play(SCARAB, json!({ "zone": 2 }));
                assert_eq!(s.state().pending.as_ref().map(|pending| js(&pending.kind)), Some(json!("discover")));
                assert_eq!(s.state().active, P2);
                let thawed: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("serialises")).expect("parses");
                assert_eq!(hash_state(&thawed), hash_state(s.state()));
                // The offered def ids read out of the view (§10.8), like #7's own test does: the raw state
                // options carry only the key and the selection to send back.
                let seen = js(&s.view(P2))["pending"].clone();
                if seen.is_null() || seen["forYou"] != true {
                    panic!("no Discover open for p2");
                }
                let offered_id = match seen["options"][0]["defId"].as_str() {
                    Some(id) => id.to_string(),
                    None => panic!("no Discover option"),
                };
                let selection = match s.state().pending.as_ref().and_then(|pending| pending.options.first()) {
                    Some(option) => option.selection.clone(),
                    None => panic!("no selection to send back"),
                };
                let choice_id = s.state().pending.as_ref().map(|pending| pending.id.clone()).unwrap_or_default();
                let answer: Action = json_as(json!({
                    "type": "answer",
                    "choiceId": choice_id,
                    "selection": [selection],
                    "playerId": "p2",
                    "nonce": "rate-limit-pause",
                }));
                let live = reduce(s.state(), &answer);
                let frozen = reduce(&thawed, &answer);
                assert!(live.error.is_none());
                assert_eq!(hash_state(&frozen.state), hash_state(&live.state));
                s.answer(json!([selection]));
                s.expect_in_zone(RATE_LIMIT, "graveyard");
                // Scarab resolved after the answer: its Discover added the offered (2) Cost card to their hand.
                assert!(s.hand(P2).iter().any(|card| card.def_id == offered_id));
                assert_eq!(s.state().active, P1);
                s.expect_events(json!(["cardResolved", "turnCutShort", "turnEnded"]));
            }

            #[test]
            fn r448_a_countered_play_is_never_played_and_doesnt_count() {
                let mut s = setup(
                    json!({ "field": [VANILLA], "backrow": [trap(false, 2), { "def": REFUSAL, "faceUp": false, "lane": 3 }] }),
                    json!({ "hand": [VANILLA, HIT_JOB, REPLENISH, STOCKPILE] }),
                    false,
                );
                let target = s.unit(P1, 1).map(|unit| unit.id).unwrap_or_default();
                s.play(VANILLA, json!({ "zone": 1 }))
                    .play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": target }] }));
                assert_eq!(count(s.events(), "countered"), 1);
                s.play(REPLENISH, json!({}));
                // Two plays made: the Hit Job never was one.
                assert_eq!(s.state().active, P2);
                assert_eq!(s.card(RATE_LIMIT).face_up, Some(false));
                s.play(STOCKPILE, json!({}));
                assert_eq!(s.state().active, P1);
            }

            #[test]
            fn r33_r97_the_opponent_learns_nothing_of_it_until_it_fires() {
                let mut s = setup(json!({}), json!({}), false);
                s.play(VANILLA, json!({ "zone": 1 })).play(REPLENISH, json!({}));
                assert!(!view_text(&s, P2).contains(RATE_LIMIT));
                s.play(STOCKPILE, json!({}));
                assert!(view_text(&s, P2).contains(RATE_LIMIT));
            }

            #[test]
            fn r456_the_next_turn_is_theirs_again_as_usual_the_cut_short_turn_costs_them_only_the_rest_of_that_turn() {
                let mut s = setup(json!({}), json!({}), false);
                s.play(VANILLA, json!({ "zone": 1 })).play(REPLENISH, json!({})).play(STOCKPILE, json!({}));
                assert_eq!(s.state().active, P1);
                s.end_turn();
                assert_eq!(s.state().active, P2);
                s.play(VANILLA, json!({ "zone": 2 }))
                    .play(VANILLA, json!({ "zone": 3 }))
                    .play(VANILLA, json!({ "zone": 4 }));
                assert_eq!(s.state().active, P2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r456_fires_on_their_2nd_play_the_play_resolves_then_their_turn_ends() {
                let mut s = setup(json!({}), json!({}), true);
                s.play(VANILLA, json!({ "zone": 1 }));
                assert_eq!(s.card(RATE_LIMIT).face_up, Some(false));
                s.play(STOCKPILE, json!({}));
                s.expect_in_zone(RATE_LIMIT, "graveyard");
                s.expect_health(P2, 32);
                assert_eq!(s.state().active, P1);
            }
        }
    }
}
