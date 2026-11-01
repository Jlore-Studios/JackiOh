//! C #65 Ace in the Hole (SPEC §8.6 row 65; §2.2, §6.3 Recruit; R33, R62, R78, R99, R386). Trap, cost 2,
//! Common.
//!   Base:    "End of turn: Flip a coin. On heads, this reveals: Recruit {recruits|card|cards}." (1)
//!   Radiant: "Revealed. End of turn: Flip a coin. On tails, Recruit a card. On heads, this reveals:
//!            Recruit {recruits|card|cards}." (3)
//!   Engine:  one seeded coin at each end of its controller's turn. Heads fires it in the end-of-turn trap
//!            window (R62; consumed); Recruit scans the deck top down for a permanent, "3 cards" three
//!            scans. The Radiant's tails recruits one without firing: the trap stays set, face-down.
//!
//! The coin is the `endOfTurn` hook's: a backrow card answers it at its controller's end of turn alone,
//! in the end-of-turn triggers, which R62 runs before the trap window. The hook flips once, remembers a
//! heads on the instance under the turn's number, and on the Radiant's tails recruits there and then —
//! no `trapFired`, so the trap stays face-down. The window's check (R99) only reads that memory, so no
//! predicate draws from the rng, and a memory from another turn or another stay (R78) arms nothing.
//!
//! The coin is a luck-based roll whose better side is heads on both faces (R1440): the trap's own
//! Lucky (`lucky_on`: printed plus given, R1438) flips that many more coins, heads kept. It prints
//! none, so with nothing given the flip is the one draw it always was.
//!
//! The coin is reported as it lands (`report_rolls`, R2652001), every coin a Lucky flipped with it.
//! The base face flips face-down, so the opponent sees that flip only on heads, when the trap fires in
//! the same action, as a Secret's is seen; the Radiant face is Revealed before it flips, so both see
//! every flip (R2652000).

use jackioh_engine::effects::{recruit, remember, reveal};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-065";

const HEADS: &str = "headsOnTurn";

/// TS `const fire: TriggerDef` (a function here: a closure cannot be a `const`).
fn fire() -> TriggerDef {
    TriggerDef::new("ace-in-the-hole", &[GameEventType::TurnEnded], |ctx, _event| {
        vec![recruit(json_as(json!({ "count": param(&*ctx, "recruits") })))]
    })
    .with_when(|ctx, event| {
        matches!(event, GameEvent::TurnEnded { player, .. } if *player == ctx.controller)
            && recalled(ctx, HEADS).and_then(|value| value.as_i64()) == Some(i64::from(ctx.state.turn))
    })
}

fn flip(on_tails: bool) -> Hook {
    hook(move |ctx| {
        // R987: the controller's Luck flips extra times beside the card's own Lucky.
        let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me)) + luck_of(&*ctx.state, ctx.controller);
        let coin = flip_coin(ctx, lucky);
        let heads = coin.heads();
        // R2652000, R2652001: the coin is shown first; to the opponent only if the trap fires on it.
        let mut effects = vec![report_rolls(RollKind::Coin, ROLL_COIN_SIDES, vec![coin])];
        if heads {
            effects.push(remember(json_as(
                json!({ "key": HEADS, "value": ctx.state.turn }),
            )));
        } else if on_tails {
            effects.push(recruit(json_as(json!({}))));
        }
        effects
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(flip(false)),
        triggers: vec![fire()],
        ..Script::default()
    };

    // The Radiant face is Revealed regardless of the coin flip (balance patch 1, R686): the end of turn
    // shows its face to both players first, then flips as usual. Revealed is not face-up, so a tails
    // that recruits without firing leaves the trap armed and still answering.
    let radiant = Script {
        end_of_turn: Some(hook(|ctx| {
            let tails = flip(true)(ctx);
            let mut effects = vec![reveal(json_as(json!({})))];
            effects.extend(tails);
            effects
        })),
        triggers: vec![fire()],
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #65 Ace in the Hole — SPEC §8.6 row 65, BUILD M9 Classic row C 65: "Face-down (R33); at the end of
// its controller's turn, in the end-of-turn trap window (R62), flip a coin from the match rng: heads
// fires it (to the graveyard) and Recruits the first permanent from the top of your deck (none, or its
// row full, → nothing); tails does nothing and it stays set; the opponent's turns flip nothing;
// radiant: tails Recruits 1 without firing, the trap staying face-down and never named in the
// opponent's view, and heads fires it and Recruits 3; a recruited trap lands face-down (R33); its tuned
// number (recruits) reads through `param()` (R386)".
//
// The coin is the match rng's next draw as its controller's turn ends (the trap's end-of-turn hook,
// which R62 runs before the window the trap fires in), so each case picks a seed whose next coin is the
// face it needs (`withCoin`) and then checks the trap did what that coin says — which is what "one
// seeded coin each time" means. The deck holds Core cards: Mr. Vanilla and Gary the Gambler (Units),
// Sheepish (a Trap), Lunar Eclipse (a Spell, which a Recruit passes by).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;
    use crate::def_ids;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ACE: &str = "classic-065";
    const VANILLA: &str = "core-008";
    const GARY: &str = "core-004";
    const SHEEPISH: &str = "core-041";
    const LUNAR: &str = "core-035";
    const FILLER: &str = "core-005";

    /// TS `type Coin = "heads" | "tails"`.
    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Coin {
        Heads,
        Tails,
    }

    impl Coin {
        /// The TS literal.
        fn as_str(self) -> &'static str {
            match self {
                Coin::Heads => "heads",
                Coin::Tails => "tails",
            }
        }
    }

    use crate::js;

    /// The coin the match rng gives next: the one the trap flips as the active player's turn ends.
    fn next_coin(state: &GameState) -> Coin {
        if Rng::new(&state.seed, state.rng_cursor).coin() {
            Coin::Heads
        } else {
            Coin::Tails
        }
    }

    /// A board with an Ace in the Hole set in p1's backrow, on the first seed whose next coin is `coin`.
    /// `opts`: `{ radiant?, p1? (a partial side, spread over the default), active? }`, as the TS helper's.
    fn with_coin(coin: Coin, opts: Value) -> Scenario {
        for attempt in 0..64 {
            let mut ace = json!({ "def": ACE, "faceUp": false });
            if opts["radiant"] == true {
                ace["radiant"] = json!(true);
            }
            let mut p1 = json!({
                "hand": [FILLER],
                "backrow": [ace],
                "library": [LUNAR, VANILLA, GARY, VANILLA],
            });
            if let Some(overrides) = opts["p1"].as_object() {
                for (key, value) in overrides {
                    p1[key] = value.clone();
                }
            }
            let mut setup = json!({
                "seed": format!("ace-in-the-hole-{}-{attempt}", coin.as_str()),
                "p1": p1,
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
            });
            if !opts["active"].is_null() {
                setup["active"] = opts["active"].clone();
            }
            let s = scenario(setup);
            if next_coin(s.state()) == coin {
                return s;
            }
        }
        panic!("no seed gave {}", coin.as_str());
    }

    /// The `rolled` batches among `events`.
    fn rolled(events: &[GameEvent]) -> Vec<GameEvent> {
        events
            .iter()
            .filter(|event| event.event_type() == GameEventType::Rolled)
            .cloned()
            .collect()
    }

    /// The types of `events` from the first `rolled` on, in the order the view lists them.
    fn from_the_flip(events: &[GameEvent]) -> Vec<GameEventType> {
        events
            .iter()
            .map(GameEvent::event_type)
            .skip_while(|kind| *kind != GameEventType::Rolled)
            .collect()
    }

    fn fired(s: &Scenario) -> usize {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == "trapFired")
            .count()
    }

    /// p1's unit row, lanes 1–5: each lane's def id, or null.
    fn units(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| unit.def_id.clone()))
            .collect()
    }

    /// The expected `units(s)`: a def id per lane, `None` for an empty one.
    fn lanes(ids: [Option<&str>; 5]) -> Vec<Option<String>> {
        ids.iter().map(|id| id.map(str::to_string)).collect()
    }

    mod c_65_ace_in_the_hole {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r33_it_is_set_face_down_and_never_named_in_the_opponents_view() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [ACE, FILLER] }, "p2": { "hand": [FILLER] } }));
                s.play(ACE, json!({}));
                assert_ne!(s.card(ACE).face_up, Some(true));
                assert_eq!(js(&s.view(P2))["opponent"]["backrow"][0]["faceDown"], true);
                assert!(!js(&s.view(P2)).to_string().contains(ACE));
            }

            #[test]
            fn r62_heads_at_the_end_of_your_turn_it_fires_face_up_to_the_graveyard_and_recruits_the_first_permanent_from_the_top_of_your_deck()
             {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({}));
                let ace = s.card(ACE).clone();
                let cursor = s.state().rng_cursor;
                s.end_turn();
                assert!(s.state().rng_cursor > cursor);
                s.expect_events(json!(["turnEnded", "trapFired", "summoned"]));
                assert_eq!(units(&s), lanes([Some(VANILLA), None, None, None, None]));
                s.expect_in_zone(&ace, "graveyard");
                assert_eq!(
                    def_ids(&s.pile(P1, "library")),
                    vec![LUNAR, GARY, VANILLA],
                );
            }

            /// R1438, R1440: set from the hand with Lucky 1 given to it, it flips twice and keeps heads,
            /// so tails then heads fires it.
            #[test]
            fn r1438_r1440_given_lucky_1_tails_then_heads_fires_it() {
                crate::register_all();
                for attempt in 0..256 {
                    let mut s = scenario(json!({
                        "seed": format!("ace-in-the-hole-lucky-{attempt}"),
                        "p1": { "hand": [ACE, FILLER], "library": [LUNAR, VANILLA, GARY, VANILLA] },
                        "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                    }));
                    crate::give_lucky(&mut s, ACE, 1);
                    s.play(ACE, json!({}));
                    let mut coins = Rng::new(&s.state().seed, s.state().rng_cursor);
                    if coins.coin() || !coins.coin() {
                        continue;
                    }
                    let ace = s.card(ACE).clone();
                    assert_eq!(lucky_on(s.state(), &ace), 1);
                    let cursor = s.state().rng_cursor;
                    s.end_turn();
                    assert!(s.state().rng_cursor >= cursor + 2);
                    assert_eq!(fired(&s), 1);
                    assert_eq!(units(&s), lanes([Some(VANILLA), None, None, None, None]));
                    s.expect_in_zone(&ace, "graveyard");
                    return;
                }
                panic!("no seed flipped tails then heads");
            }

            /// R1440: given Lucky 1 it fires wherever it fires with none (its first coin is the plain
            /// flip), and at more seeds besides.
            #[test]
            fn r1440_given_lucky_1_it_fires_more_often_over_100_seeds() {
                crate::register_all();
                let fires = |seed: usize, lucky: bool| -> bool {
                    let mut s = scenario(json!({
                        "seed": format!("ace-in-the-hole-seeds-{seed}"),
                        "p1": {
                            "hand": [FILLER],
                            "backrow": [{ "def": ACE, "faceUp": false }],
                            "library": [LUNAR, VANILLA, GARY, VANILLA],
                        },
                        "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                    }));
                    if lucky {
                        crate::give_lucky(&mut s, ACE, 1);
                    }
                    s.end_turn();
                    fired(&s) == 1
                };
                let plain: Vec<usize> = (0..100).filter(|seed| fires(*seed, false)).collect();
                let lucky: Vec<usize> = (0..100).filter(|seed| fires(*seed, true)).collect();
                for seed in &plain {
                    assert!(lucky.contains(seed));
                }
                assert!(lucky.len() > plain.len());
            }

            #[test]
            fn tails_nothing_happens_and_it_stays_set_face_down_never_named_to_the_opponent() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({}));
                let ace = s.card(ACE).clone();
                s.end_turn();
                assert_eq!(fired(&s), 0);
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned"));
                s.expect_in_zone(&ace, "field");
                assert_ne!(s.card(&ace).face_up, Some(true));
                assert!(!js(&s.view(P2)).to_string().contains(ACE));
            }

            #[test]
            fn r177_tails_leaves_the_opponent_nothing_to_read_their_events_and_the_shared_counter_match_a_face_down_sheepishs()
             {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({}));
                let mut plain = scenario(json!({
                    "seed": s.state().seed.clone(),
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": SHEEPISH, "faceUp": false }],
                        "library": [LUNAR, VANILLA, GARY, VANILLA],
                    },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                s.end_turn();
                plain.end_turn();
                assert_eq!(s.state().next_seq, plain.state().next_seq);
                assert_eq!(s.view(P2).events, plain.view(P2).events);
            }

            /// R2652000: a tails flipped face-down is one coin reported to its controller alone: the
            /// raw event marks it face-down, p1's view shows the coin and p2's has no `rolled` at all.
            #[test]
            fn r2652000_base_tails_the_flip_reaches_its_controller_alone() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({}));
                let ace = s.card(ACE).clone();
                s.end_turn();
                let raw = rolled(s.events());
                assert_eq!(raw.len(), 1);
                let GameEvent::Rolled {
                    player,
                    instance_id,
                    kind,
                    rolls,
                    face_down,
                    ..
                } = &raw[0]
                else {
                    panic!("a rolled event");
                };
                assert_eq!((*player, instance_id.as_str(), *kind), (P1, ace.id.as_str(), RollKind::Coin));
                assert_eq!(rolls, &vec![RollResult { faces: vec![ROLL_TAILS], kept: 0 }]);
                assert_eq!(*face_down, Some(true));

                let mine = rolled(&s.view(P1).events);
                assert_eq!(mine.len(), 1);
                // `faceDown` is the engine's: no view forwards it.
                assert!(matches!(&mine[0], GameEvent::Rolled { face_down: None, def_id, .. } if def_id == ACE));
                assert!(rolled(&s.view(P2).events).is_empty());
                assert!(!js(&s.view(P2)).to_string().contains(ACE));
            }

            /// R2652000: on heads the trap fires in the same action, so the opponent sees the flip too,
            /// naming the card as its graveyard does, before the `trapFired` it caused.
            #[test]
            fn r2652000_base_heads_the_opponent_sees_the_flip_with_the_trap_it_fired() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({}));
                s.end_turn();
                for viewer in [P1, P2] {
                    let view = s.view(viewer);
                    let seen = rolled(&view.events);
                    assert_eq!(seen.len(), 1, "{viewer} sees the flip");
                    assert!(matches!(
                        &seen[0],
                        GameEvent::Rolled { def_id, rolls, face_down: None, .. }
                            if def_id == ACE && rolls[0].heads()
                    ));
                    // The flip is the end-of-turn hook's, before `turnEnded`; the trap fires after it.
                    let order = from_the_flip(&view.events);
                    assert_eq!(order[0], GameEventType::Rolled);
                    assert!(order.contains(&GameEventType::TrapFired));
                }
            }

            #[test]
            fn a_trap_left_set_flips_again_at_your_next_end_of_turn_one_coin_each_time() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({}));
                s.end_turn(); // p1's end: tails.
                s.end_turn(); // p2's end: no coin.
                let second = next_coin(s.state());
                s.end_turn(); // p1's end again: a fresh coin.
                assert_eq!(fired(&s), if second == Coin::Heads { 1 } else { 0 });
                s.expect_in_zone(
                    ACE,
                    if second == Coin::Heads {
                        "graveyard"
                    } else {
                        "field"
                    },
                );
            }

            #[test]
            fn the_opponents_turns_flip_nothing_their_end_of_turn_draws_no_coin_and_leaves_it_set() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({ "active": "p2" }));
                let cursor = s.state().rng_cursor;
                s.end_turn();
                assert_eq!(s.state().rng_cursor, cursor);
                assert_eq!(fired(&s), 0);
                s.expect_in_zone(ACE, "field");
            }

            #[test]
            fn heads_with_no_permanent_in_the_deck_it_fires_and_recruits_nothing() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({ "p1": { "library": [LUNAR, FILLER] } }));
                s.end_turn();
                assert_eq!(fired(&s), 1);
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned"));
                s.expect_in_zone(ACE, "graveyard");
            }

            #[test]
            fn heads_with_the_unit_row_full_it_fires_and_the_unit_stays_on_top_of_the_deck() {
                crate::register_all();
                let mut s = with_coin(
                    Coin::Heads,
                    json!({ "p1": { "field": [GARY, GARY, GARY, GARY, GARY], "library": [VANILLA, LUNAR] } }),
                );
                s.end_turn();
                assert_eq!(fired(&s), 1);
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned"));
                assert_eq!(
                    s.pile(P1, "library").first().map(|card| card.def_id.clone()),
                    Some(VANILLA.to_string())
                );
            }

            #[test]
            fn r33_a_recruited_trap_lands_face_down_unnamed_in_the_opponents_view() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({ "p1": { "library": [SHEEPISH, VANILLA] } }));
                s.end_turn();
                let sheepish = s.card(SHEEPISH).clone();
                assert_eq!(js(&sheepish.zone)["z"], "field");
                assert_ne!(sheepish.face_up, Some(true));
                assert!(!js(&s.view(P2)).to_string().contains(SHEEPISH));
            }

            #[test]
            fn r386_an_upgrade_recruits_2_on_heads() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({}));
                step_param(s.card_mut(ACE), "recruits", 1);
                s.end_turn();
                assert_eq!(units(&s), lanes([Some(VANILLA), Some(GARY), None, None, None]));
            }

            #[test]
            fn the_coin_is_seeded_a_round_tripped_state_flips_the_same_coin_and_replays_the_same_turn_end() {
                crate::register_all();
                let s = with_coin(Coin::Heads, json!({}));
                let thawed: GameState =
                    serde_json::from_value(js(s.state())).expect("the state round-trips through JSON");
                let end: Action =
                    json_as(json!({ "type": "endTurn", "playerId": "p1", "nonce": "ace-roundtrip" }));
                let live = reduce(s.state(), &end);
                let frozen = reduce(&thawed, &end);
                assert!(live.error.is_none());
                assert_eq!(frozen.state, live.state);
                assert_eq!(frozen.events, live.events);
                assert!(
                    live.events
                        .iter()
                        .map(js)
                        .any(|event| event["type"] == "trapFired")
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn heads_it_fires_and_recruits_3() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({ "radiant": true }));
                s.end_turn();
                assert_eq!(fired(&s), 1);
                assert_eq!(
                    units(&s),
                    lanes([Some(VANILLA), Some(GARY), Some(VANILLA), None, None])
                );
                s.expect_in_zone(ACE, "graveyard");
            }

            #[test]
            fn r686_tails_it_recruits_1_without_firing_and_stays_set_revealed_readable_still_armed() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({ "radiant": true }));
                let ace = s.card(ACE).clone();
                s.end_turn();
                assert_eq!(fired(&s), 0);
                assert_eq!(units(&s), lanes([Some(VANILLA), None, None, None, None]));
                s.expect_in_zone(&ace, "field");
                assert_ne!(s.card(&ace).face_up, Some(true));
                // R686: Revealed regardless of the coin flip — the opponent reads its face, but it never fired.
                assert_eq!(s.card(&ace).revealed, Some(true));
                let theirs = js(&s.view(P2));
                assert!(theirs.to_string().contains(ACE));
                assert_eq!(theirs["opponent"]["backrow"][0]["faceDown"], false);
            }

            #[test]
            fn tails_a_recruited_trap_lands_face_down_r33_and_the_set_trap_flips_again_at_your_next_end_of_turn()
             {
                crate::register_all();
                let mut s = with_coin(
                    Coin::Tails,
                    json!({ "radiant": true, "p1": { "library": [SHEEPISH, VANILLA, GARY, VANILLA] } }),
                );
                s.end_turn(); // p1's end: tails, Sheepish recruited.
                assert_eq!(js(&s.card(SHEEPISH).zone)["z"], "field");
                assert_ne!(s.card(SHEEPISH).face_up, Some(true));
                assert!(!js(&s.view(P2)).to_string().contains(SHEEPISH));
                s.end_turn(); // p2's end: no coin.
                let second = next_coin(s.state());
                s.end_turn(); // p1's end again: a fresh coin — heads recruits 3, tails 1.
                assert_eq!(fired(&s), if second == Coin::Heads { 1 } else { 0 });
                assert_eq!(
                    units(&s).iter().filter(|unit| unit.is_some()).count(),
                    if second == Coin::Heads { 3 } else { 1 },
                );
                s.expect_in_zone(
                    ACE,
                    if second == Coin::Heads {
                        "graveyard"
                    } else {
                        "field"
                    },
                );
            }

            #[test]
            fn tails_replays_the_same_from_a_round_tripped_state_one_coin_one_recruit_no_firing() {
                crate::register_all();
                let s = with_coin(Coin::Tails, json!({ "radiant": true }));
                let thawed: GameState =
                    serde_json::from_value(js(s.state())).expect("the state round-trips through JSON");
                let end: Action =
                    json_as(json!({ "type": "endTurn", "playerId": "p1", "nonce": "ace-radiant-roundtrip" }));
                let live = reduce(s.state(), &end);
                let frozen = reduce(&thawed, &end);
                assert!(live.error.is_none());
                assert_eq!(frozen.state, live.state);
                assert_eq!(frozen.events, live.events);
                assert_eq!(live.state.rng_cursor, s.state().rng_cursor + 1);
                assert_eq!(
                    live.events
                        .iter()
                        .map(js)
                        .filter(|event| event["type"] == "summoned")
                        .count(),
                    1
                );
                assert!(
                    !live
                        .events
                        .iter()
                        .map(js)
                        .any(|event| event["type"] == "trapFired")
                );
            }

            /// R2652000: the Radiant face is Revealed before it flips, so its tails, which fires nothing,
            /// reaches both players, the coin after the reveal and before the recruit.
            #[test]
            fn r2652000_radiant_the_revealed_trap_flips_in_both_views() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({ "radiant": true }));
                s.end_turn();
                assert_eq!(fired(&s), 0);
                let raw = rolled(s.events());
                assert_eq!(raw.len(), 1);
                assert!(matches!(&raw[0], GameEvent::Rolled { face_down: None, .. }));
                for viewer in [P1, P2] {
                    let seen = rolled(&s.view(viewer).events);
                    assert_eq!(seen.len(), 1, "{viewer} sees the flip");
                    assert!(matches!(
                        &seen[0],
                        GameEvent::Rolled { def_id, rolls, .. } if def_id == ACE && !rolls[0].heads()
                    ));
                }
            }

            /// R1440, R2652001: given Lucky 1 its two coins are one batch, both faces in draw order
            /// and the heads kept when either lands it.
            #[test]
            fn r2652001_given_lucky_1_its_two_coins_are_one_report() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({ "radiant": true }));
                crate::give_lucky(&mut s, ACE, 1);
                let mut coins = Rng::new(&s.state().seed, s.state().rng_cursor);
                let face = |heads: bool| if heads { ROLL_HEADS } else { ROLL_TAILS };
                let faces = vec![face(coins.coin()), face(coins.coin())];
                s.end_turn();
                let raw = rolled(s.events());
                assert_eq!(raw.len(), 1);
                let GameEvent::Rolled { rolls, .. } = &raw[0] else {
                    panic!("a rolled event");
                };
                assert_eq!(rolls.len(), 1);
                assert_eq!(rolls[0].faces, faces);
                assert_eq!(rolls[0].heads(), faces.contains(&ROLL_HEADS));
                assert_eq!(fired(&s), usize::from(rolls[0].heads()));
            }

            #[test]
            fn tails_with_nothing_to_recruit_nothing_happens_and_it_stays_set() {
                crate::register_all();
                let mut s = with_coin(
                    Coin::Tails,
                    json!({ "radiant": true, "p1": { "library": [LUNAR] } }),
                );
                s.end_turn();
                assert_eq!(fired(&s), 0);
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned"));
                s.expect_in_zone(ACE, "field");
            }

            #[test]
            fn the_opponents_turns_flip_nothing_and_recruit_nothing() {
                crate::register_all();
                let mut s = with_coin(Coin::Tails, json!({ "radiant": true, "active": "p2" }));
                let cursor = s.state().rng_cursor;
                s.end_turn();
                assert_eq!(s.state().rng_cursor, cursor);
                assert!(!s.events().iter().map(js).any(|event| event["type"] == "summoned"));
            }

            #[test]
            fn r33_a_recruited_trap_lands_face_down() {
                crate::register_all();
                let mut s = with_coin(
                    Coin::Heads,
                    json!({ "radiant": true, "p1": { "library": [SHEEPISH] } }),
                );
                s.end_turn();
                assert_ne!(s.card(SHEEPISH).face_up, Some(true));
                assert!(!js(&s.view(P2)).to_string().contains(SHEEPISH));
            }

            #[test]
            fn r386_a_degrade_recruits_2_on_heads() {
                crate::register_all();
                let mut s = with_coin(Coin::Heads, json!({ "radiant": true }));
                step_param(s.card_mut(ACE), "recruits", -1);
                s.end_turn();
                assert_eq!(units(&s), lanes([Some(VANILLA), Some(GARY), None, None, None]));
            }
        }
    }
}
