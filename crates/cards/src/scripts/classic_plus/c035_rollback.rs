//! C+ #35 Rollback (SPEC §8.7 row 35; R419, R562, R563, R566): N — 1, 2 or 3, declared with the play
//! (R81) — and the board goes back to how it was at the start of the player-turn N before this one; the
//! Radiant face also declares the part: your side, your opponent's side or both. The history and the
//! restore are E29's (`subsystems/boardHistory.ts`).

use jackioh_engine::effects::{chosen_number, roll_back};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-035";

/// TS `TURNS`: N, declared with the play, "1" to ROLLBACK_MAX_TURNS.
fn turns() -> ModeDecl {
    ModeDecl {
        kind: PromptKind::Number,
        options: (0..ROLLBACK_MAX_TURNS).map(|n| (n + 1).to_string()).collect(),
    }
}

/// The Radiant face's parts of the board, as its play names them.
const SIDES: [(&str, &str); 3] = [("your side", "self"), ("your opponent's side", "enemy"), ("both sides", "both")];

fn back(ctx: &EffectContext<'_>, sides: Option<&str>) -> Vec<Effect> {
    let turns_ago = chosen_number(ctx);
    match (turns_ago, sides) {
        (Some(turns_ago), Some(sides)) => {
            vec![roll_back(json_as(json!({ "turnsAgo": turns_ago, "sides": sides })))]
        }
        _ => vec![],
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        modes: vec![turns()],
        cry: Some(hook(|ctx| back(&*ctx, Some("both")))),
        ..Script::default()
    };
    let radiant = Script {
        modes: vec![
            turns(),
            ModeDecl {
                kind: PromptKind::Mode,
                options: SIDES.iter().map(|(label, _)| label.to_string()).collect(),
            },
        ],
        cry: Some(hook(|ctx| {
            let sides = ctx
                .modes
                .get(1)
                .and_then(|label| SIDES.iter().find(|(name, _)| name == label).map(|(_, side)| *side));
            back(&*ctx, sides)
        })),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// C+ #35 Rollback — SPEC §8.7 row 35, BUILD M9 "C+ 35", R419, and this card's rulings R562 (too little
// history), R563 (held zones) and R566 (what a put-back card keeps of the present). The history is
// E29's (`packages/engine/src/subsystems/boardHistory.ts`, engine-tested in `boardHistory.test.ts`).
//
// The harness starts mid-game on turn 9 without running a turn start, so a test's history begins with
// its first `endTurn()`: two of them reach p1's turn 11 holding the snapshots of turns 10 and 11.
// Libraries hold Mr. Vanilla fillers for the draws, and each side keeps a Unit, so no turn auto-ends
// (R82). Props: #8 Mr. Vanilla, #11 Tempo Timmy, #15 Me and Mr Token (a Cry that would show), #16 Hit
// Job, #17 Flood, #34 Collateral Damage, #36 Magic Jammed, #41 Sheepish (a face-down Trap), #43 Big
// Felinor under #92 Felinor Fiender (a Stack pile), #3 Right-house defender (Reborn), #49 Snom Bunny Mind
// Control, #63 Plastic Surgery (a buff and a keyword), #66 The Rock (Radiant: Indestructible, Immutable),
// #83 Transmogulate, #85 Unlicensed Experimentation, the Rush Token, C+ #12.8 Frostspatula ("Animated on
// your turn") and C+ #33 Ivory Tower (a Field Spell that fuses the first Unit stacked onto it).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ROLLBACK: &str = "classicplus-035";
    const VANILLA: &str = "core-008";
    const TIMMY: &str = "core-011";
    const TOKEN_MAKER: &str = "core-015";
    const HIT_JOB: &str = "core-016";
    const FLOOD: &str = "core-017";
    const COLLATERAL: &str = "core-034";
    const MAGIC_JAMMED: &str = "core-036";
    const SHEEPISH: &str = "core-041";
    const BIG_FELINOR: &str = "core-043";
    const FIENDER: &str = "core-092";
    const DEFENDER: &str = "core-003";
    const MIND_CONTROL: &str = "core-049";
    const SURGERY: &str = "core-063";
    const ROCK: &str = "core-066";
    const TRANSMOGULATE: &str = "core-083";
    const UNLICENSED: &str = "core-085";
    const RUSH_TOKEN: &str = "core-t-rush";
    const SPATULA: &str = "classicplus-012-8";
    const TOWER: &str = "classicplus-033";

    fn radiant_rollback() -> Value {
        json!({ "def": ROLLBACK, "radiant": true })
    }

    fn filler() -> Value {
        Value::Array((0..8).map(|_| json!(VANILLA)).collect())
    }

    /// The board at p1's turn 11, with the snapshots of turns 10 and 11 recorded.
    fn on_turn_11(opts: Value) -> Scenario {
        let mut options = opts.clone();
        for seat in ["p1", "p2"] {
            let mut side = json!({ "library": filler() });
            if let Some(Value::Object(fields)) = opts.get(seat) {
                for (key, value) in fields {
                    side[key.as_str()] = value.clone();
                }
            }
            options[seat] = side;
        }
        let mut s = scenario(options);
        s.end_turn().end_turn();
        s
    }

    /// Two more turns: p1's turn 13, the snapshots of turns 10 to 13 recorded.
    fn to_turn_13(s: &mut Scenario) {
        s.end_turn().end_turn();
        assert_eq!(s.state().turn, 13);
    }

    fn of_type(events: &[GameEvent], want: GameEventType) -> Vec<GameEvent> {
        events.iter().filter(|event| event.event_type() == want).cloned().collect()
    }

    fn target(instance_id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": instance_id }])
    }

    use crate::js;

    use crate::matches_object;

    fn unit(s: &Scenario, seat: PlayerId, lane: i32) -> CardInstance {
        s.unit(seat, lane).unwrap_or_else(|| panic!("no unit in {seat} lane {lane}"))
    }

    fn backrow(s: &Scenario, seat: PlayerId, lane: i32) -> CardInstance {
        s.backrow(seat, lane).unwrap_or_else(|| panic!("no backrow card in {seat} lane {lane}"))
    }

    fn unit_id(s: &Scenario, seat: PlayerId, lane: i32) -> Option<String> {
        s.unit(seat, lane).map(|card| card.id)
    }

    fn unit_def(s: &Scenario, seat: PlayerId, lane: i32) -> Option<String> {
        s.unit(seat, lane).map(|card| card.def_id)
    }

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    fn event_ids(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| js(event).get("instanceId").and_then(Value::as_str).map(str::to_string))
            .collect()
    }

    /// The ids of a unit pile, top first (`players[p].units[lane - 1]`).
    fn pile_ids(state: &GameState, seat: PlayerId, lane: usize) -> Option<Vec<String>> {
        state.players[seat].units.get(lane - 1).and_then(|pile| pile.as_ref()).map(|pile| ids(pile))
    }

    fn hand_ids(s: &Scenario, seat: PlayerId) -> Vec<String> {
        ids(&s.hand(seat))
    }

    /// TS `types.indexOf(type)`: -1 when absent.
    fn index_of(events: &[GameEvent], want: GameEventType) -> i64 {
        events.iter().position(|event| event.event_type() == want).map(|at| at as i64).unwrap_or(-1)
    }

    /// The `play` action on p1's Rollback with these modes, as JSON.
    fn play_rollback(s: &mut Scenario, modes: &[&str]) {
        s.play(ROLLBACK, json!({ "modes": modes }));
    }

    mod c_n35_rollback_the_script {
        use super::*;

        #[test]
        fn def_is_the_catalog_s_and_n_is_declared_with_the_play_r81_1_2_or_3_the_radiant_face_adds_the_side() {
            crate::register_all();
            assert_eq!(ID, ROLLBACK);
            assert_eq!(crate::card_def(ID).id, ROLLBACK);
            let scripts = script();
            assert_eq!(js(&scripts.base.modes), json!([{ "kind": "number", "options": ["1", "2", "3"] }]));
            assert_eq!(
                js(&scripts.radiant.modes),
                json!([
                    { "kind": "number", "options": ["1", "2", "3"] },
                    { "kind": "mode", "options": ["your side", "your opponent's side", "both sides"] },
                ])
            );
        }

        #[test]
        fn r81_legalactions_offers_one_play_per_n_on_the_base_face_and_per_n_and_side_on_the_radiant_face() {
            crate::register_all();
            let s = scenario(json!({ "p1": { "hand": [ROLLBACK, radiant_rollback()] } }));
            let hand = s.hand(P1);
            let (plain, shiny) = (hand.first().map(|card| card.id.clone()), hand.get(1).map(|card| card.id.clone()));
            let plays = |id: Option<String>| -> Vec<Value> {
                legal_actions(s.state(), P1)
                    .into_iter()
                    .filter_map(|action| match action {
                        ActionBody::Play { instance_id, modes, .. } if Some(&instance_id) == id.as_ref() => Some(js(&modes)),
                        _ => None,
                    })
                    .collect()
            };
            assert_eq!(plays(plain), vec![json!(["1"]), json!(["2"]), json!(["3"])]);
            let shiny_plays = plays(shiny);
            assert_eq!(shiny_plays.len(), 9);
            assert!(shiny_plays.contains(&json!(["2", "your opponent's side"])));
        }
    }

    mod c_n35_rollback_the_history_r419 {
        use super::*;

        #[test]
        fn r419_each_turn_s_start_records_both_sides_piles_dormant_cards_too_backrow_zones_and_locks_before_anything_else_and_keeps_board_history_depth() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [BIG_FELINOR, { "def": FIENDER, "stack": true }], "backrow": [SHEEPISH, SPATULA], "library": filler() },
                "p2": { "field": [TIMMY], "backrow": [SHEEPISH], "library": filler() },
            }));
            assert!(s.state().board_history.is_none());
            s.end_turn().end_turn();
            let history = |s: &Scenario| s.state().board_history.clone().unwrap_or_default();
            assert_eq!(history(&s).iter().map(|snapshot| snapshot.turn).collect::<Vec<_>>(), [10, 11]);
            let mine = history(&s)[1].sides.p1.clone();
            // The pile top first, its dormant Big Felinor included, as whole instances.
            let first_pile = mine.units[0].clone().unwrap_or_default();
            assert_eq!(first_pile.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(), [FIENDER, BIG_FELINOR]);
            let live_dormant = s.state().players.p1.units[0].as_ref().and_then(|pile| pile.get(1)).cloned();
            assert_eq!(first_pile.get(1).cloned(), live_dormant);
            assert_eq!(mine.backrow[0].as_ref().map(|card| card.def_id.as_str()), Some(SHEEPISH));
            assert_eq!(
                history(&s)[1].sides.p2.backrow[0].as_ref().map(|card| card.def_id.clone()).as_deref(),
                Some(SHEEPISH)
            );
            assert_eq!(
                js(&mine.locks),
                json!({ "units": [false, false, false, false, false], "backrow": [false, false, false, false, false] })
            );
            // Frostspatula animated at this turn's start, after the refresh: the snapshot came first.
            assert_eq!(mine.backrow[1].as_ref().map(|card| card.def_id.as_str()), Some(SPATULA));
            assert_eq!(unit_def(&s, P1, 2).as_deref(), Some(SPATULA));

            s.end_turn().end_turn().end_turn().end_turn();
            assert_eq!(BOARD_HISTORY_DEPTH, 4);
            assert_eq!(history(&s).iter().map(|snapshot| snapshot.turn).collect::<Vec<_>>(), [12, 13, 14, 15]);
        }

        #[test]
        fn r419_viewfor_never_carries_a_snapshot_nor_the_face_down_card_it_holds() {
            crate::register_all();
            let s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK], "field": [VANILLA], "backrow": [SHEEPISH] },
                "p2": { "field": [TIMMY] },
            }));
            let snapshot_backrow = s
                .state()
                .board_history
                .as_ref()
                .and_then(|history| history.first())
                .and_then(|snapshot| snapshot.sides.p1.backrow[0].clone());
            assert_eq!(snapshot_backrow.map(|card| card.def_id).as_deref(), Some(SHEEPISH));
            for viewer in [P1, P2] {
                assert!(!serde_json::to_string(&s.view(viewer)).unwrap().contains("boardHistory"));
            }
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(SHEEPISH));
        }

        #[test]
        fn r419_the_history_survives_json_a_round_tripped_state_hashes_the_same_and_rolls_back_the_same() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, HIT_JOB], "field": [VANILLA] }, "p2": { "field": [TIMMY] } }));
            let victim = unit_id(&s, P2, 1).unwrap_or_default();
            s.play(HIT_JOB, json!({ "targets": target(&victim) }));
            to_turn_13(&mut s);
            let copy: GameState = serde_json::from_str(&serde_json::to_string(s.state()).unwrap()).unwrap();
            assert_eq!(hash_state(&copy), hash_state(s.state()));
            let rollback = s.hand(P1).into_iter().find(|card| card.def_id == ROLLBACK).map(|card| card.id).unwrap_or_default();
            let action: Action = json_as(json!({
                "type": "play", "playerId": "p1", "instanceId": rollback, "modes": ["2"], "nonce": "json",
            }));
            let live = reduce(s.state(), &action);
            let again = reduce(&copy, &action);
            assert!(live.error.is_none());
            assert_eq!(hash_state(&again.state), hash_state(&live.state));
            assert_eq!(again.events, live.events);
        }
    }

    mod c_n35_rollback_base_both_sides {
        use super::*;

        fn run(n: &str) -> Scenario {
            let mut s = scenario(json!({
                "p1": { "hand": [ROLLBACK, TIMMY], "field": [VANILLA], "library": filler() },
                "p2": { "hand": [TIMMY, TOKEN_MAKER], "field": [VANILLA], "library": filler() },
            }));
            s.end_turn(); // turn 10, p2's: snapshot, then p2 plays a Tempo Timmy
            s.play(TIMMY, json!({ "zone": 2 }));
            s.end_turn(); // turn 11, p1's: snapshot, then p1 plays one
            s.play(TIMMY, json!({ "zone": 2 }));
            s.end_turn(); // turn 12, p2's: snapshot, then Me and Mr Token
            s.play(TOKEN_MAKER, json!({ "zone": 3 }));
            s.end_turn(); // turn 13
            play_rollback(&mut s, &[n]);
            s
        }

        fn lanes(s: &Scenario, player: PlayerId) -> Value {
            js(&(1..=4).map(|lane| unit_def(s, player, lane)).collect::<Vec<_>>())
        }

        #[test]
        fn r419_n_names_the_snapshot_of_the_start_of_the_player_turn_n_before_this_one() {
            crate::register_all();
            // 1: the start of turn 12 — both Tempo Timmys, no Me and Mr Token yet.
            let one = run("1");
            assert_eq!(lanes(&one, P1), json!([VANILLA, TIMMY, null, null]));
            assert_eq!(lanes(&one, P2), json!([VANILLA, TIMMY, null, null]));
            // 2: the start of turn 11 — p2's Timmy, not p1's.
            let two = run("2");
            assert_eq!(lanes(&two, P1), json!([VANILLA, null, null, null]));
            assert_eq!(lanes(&two, P2), json!([VANILLA, TIMMY, null, null]));
            // 3: the start of turn 10 — neither.
            let three = run("3");
            assert_eq!(lanes(&three, P1), json!([VANILLA, null, null, null]));
            assert_eq!(lanes(&three, P2), json!([VANILLA, null, null, null]));
            assert_eq!(
                js(&of_type(three.last_events(), GameEventType::RolledBack)),
                json!([{ "type": "rolledBack", "player": "p1", "turnsAgo": 3, "sides": ["p1", "p2"] }])
            );
        }

        #[test]
        fn r419_step_1_a_card_the_snapshot_lacks_goes_to_its_owner_s_hand_reset_r78_a_unit_token_ceases_to_exist_r11_no_death() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK, TOKEN_MAKER, { "def": DEFENDER, "radiant": true }], "field": [VANILLA] },
                "p2": { "field": [TIMMY] },
            }));
            s.play(TOKEN_MAKER, json!({ "zone": 2 }));
            let held = s.hand(P1).into_iter().find(|card| card.def_id == DEFENDER).expect("the defender in hand");
            s.play(&held, json!({ "zone": 4 }));
            let maker = unit(&s, P1, 2);
            let token = unit(&s, P1, 3);
            let defender = unit(&s, P1, 4);
            assert_eq!(token.def_id, RUSH_TOKEN);
            s.end_turn(); // p2's Timmy hits the defender's Divine Shield away
            let attacker = unit(&s, P2, 1);
            s.attack(&attacker, &defender);
            s.end_turn();

            play_rollback(&mut s, &["2"]);
            s.expect_in_zone(&maker, "hand").expect_in_zone(&defender, "hand").expect_in_zone(&token, "gone");
            assert!(s.card(&defender).divine_shield_spent.is_none());
            assert!(s.card(&defender).position.is_none());
            s.expect_events(json!(["rolledBack", "bounced", "bounced", "bounced"]));
            // A move to a hand is no death: the Radiant defender's Death summons nothing, and none dies.
            assert!(of_type(s.last_events(), GameEventType::Destroyed).is_empty());
            assert!(of_type(s.last_events(), GameEventType::Summoned).is_empty());
            assert_eq!(
                js(&(1..=5).map(|lane| unit_def(&s, P1, lane)).collect::<Vec<_>>()),
                json!([VANILLA, null, null, null, null])
            );
        }

        #[test]
        fn r419_step_1_a_full_hand_burns_the_card_s2_4_r4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ROLLBACK], "field": [VANILLA], "library": filler() },
                "p2": { "hand": (0..10).map(|_| TIMMY).collect::<Vec<_>>(), "field": [VANILLA], "library": filler() },
            }));
            s.end_turn(); // turn 10: p2's draw burns at 10; p2 plays two Timmys
            s.play(TIMMY, json!({ "zone": 2 })).play(TIMMY, json!({ "zone": 3 }));
            s.end_turn().end_turn(); // p2's draw on turn 12 brings it to 9
            s.end_turn();
            assert_eq!(s.hand(P2).len(), 9);
            let (first, second) = (unit(&s, P2, 2), unit(&s, P2, 3));

            play_rollback(&mut s, &["3"]);
            s.expect_in_zone(&first, "hand").expect_in_zone(&second, "graveyard");
            assert_eq!(event_ids(&of_type(s.last_events(), GameEventType::Burned)), [second.id.clone()]);
            assert_eq!(s.hand(P2).len(), 10);
        }

        #[test]
        fn r419_step_2_a_snapshot_card_goes_back_exactly_as_it_was_damage_buffs_position_counters_from_the_field_its_pile_rebuilt() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": {
                    "hand": [ROLLBACK, HIT_JOB],
                    "field": [
                        BIG_FELINOR,
                        { "def": FIENDER, "stack": true },
                        { "def": VANILLA, "damage": 1, "position": "DEF", "counters": { "plague": 2 } },
                    ],
                },
                "p2": { "field": [TIMMY] },
            }));
            let fiender = unit(&s, P1, 1);
            let felinor = s.state().players.p1.units[0].as_ref().and_then(|pile| pile.get(1)).cloned();
            let vanilla = unit(&s, P1, 2);
            s.play(HIT_JOB, json!({ "targets": target(&fiender.id) })); // the pile's top dies; Big Felinor resumes
            s.switch_position(&vanilla);
            assert_eq!(unit_id(&s, P1, 1), felinor.as_ref().map(|card| card.id.clone()));
            s.end_turn();
            let (attacker, defender) = (unit(&s, P2, 1), unit(&s, P1, 2));
            s.attack(&attacker, &defender);
            s.end_turn();

            play_rollback(&mut s, &["2"]);
            let mut want = vec![fiender.id.clone()];
            want.extend(felinor.map(|card| card.id));
            assert_eq!(pile_ids(s.state(), P1, 1), Some(want));
            assert!(matches_object(
                &js(s.card(&vanilla)),
                &json!({ "damage": 1, "position": "DEF", "counters": { "plague": 2 }, "buffs": { "attack": 0, "health": 0 } })
            ));
            assert!(!ids(&s.state().players.p1.graveyard).contains(&fiender.id));
        }

        #[test]
        fn r419_step_1_a_stack_card_played_on_top_since_goes_to_its_owner_s_hand_the_card_beneath_stands_alone_and_acts_again() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, FIENDER], "field": [BIG_FELINOR] }, "p2": { "field": [TIMMY] } }));
            let felinor = unit(&s, P1, 1);
            s.play(FIENDER, json!({ "zone": 1 }));
            let fiender = unit(&s, P1, 1);
            assert_eq!(pile_ids(s.state(), P1, 1), Some(vec![fiender.id.clone(), felinor.id.clone()]));
            s.end_turn().end_turn();

            play_rollback(&mut s, &["2"]); // the start of turn 11, before the Fiender
            s.expect_in_zone(&fiender, "hand");
            assert_eq!(pile_ids(s.state(), P1, 1), Some(vec![felinor.id.clone()]));
            assert_eq!(unit_id(&s, P1, 1), Some(felinor.id.clone()));
            assert_eq!(event_ids(&of_type(s.last_events(), GameEventType::Bounced)), [fiender.id.clone()]);
            // The Felinor never left its side: nothing but `rolledBack` reports it (R566).
            assert!(of_type(s.last_events(), GameEventType::ControlChanged).is_empty());
        }

        #[test]
        fn r419_r97_step_1_a_face_down_card_set_since_goes_back_to_its_owner_s_hand_and_the_opponent_reads_neither_it_nor_its_definition() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, SHEEPISH], "field": [VANILLA] }, "p2": { "field": [TIMMY] } }));
            s.play(SHEEPISH, json!({}));
            let trap = backrow(&s, P1, 1);
            s.end_turn().end_turn();

            play_rollback(&mut s, &["2"]);
            s.expect_in_zone(&trap, "hand");
            assert_eq!(
                js(&of_type(&s.view(P2).events, GameEventType::Bounced)),
                json!([{ "type": "bounced", "instanceId": "hidden", "defId": "hidden", "owner": "p1" }])
            );
            let seen = serde_json::to_string(&s.view(P2)).unwrap();
            assert!(!seen.contains(SHEEPISH));
            assert!(!seen.contains(&format!("\"{}\"", trap.id)));
            // Its owner reads it.
            assert_eq!(
                js(&of_type(&s.view(P1).events, GameEventType::Bounced)),
                json!([{ "type": "bounced", "instanceId": trap.id, "defId": SHEEPISH, "owner": "p1" }])
            );
        }

        #[test]
        fn r419_step_2_buffs_and_granted_keywords_are_the_snapshot_s_back_on_a_card_that_died_since_gone_from_one_buffed_since() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK, SURGERY, SURGERY], "field": [VANILLA, { "def": VANILLA, "lane": 2 }] },
                "p2": { "hand": [HIT_JOB], "field": [TIMMY] },
            }));
            let (early, late) = (unit(&s, P1, 1), unit(&s, P1, 2));
            s.play(SURGERY, json!({ "targets": target(&early.id) })); // turn 11: +3/+3 and a random keyword
            let keywords = s.card(&early).granted_keywords.clone();
            assert_eq!(keywords.len(), 1);
            s.end_turn(); // turn 12's snapshot holds it buffed
            s.play(HIT_JOB, json!({ "targets": target(&early.id) })); // it dies, and leaving the field strips both (R78)
            assert!(matches_object(
                &js(s.card(&early)),
                &json!({ "buffs": { "attack": 0, "health": 0 }, "grantedKeywords": [] })
            ));
            s.end_turn();
            s.play(SURGERY, json!({ "targets": target(&late.id) })); // turn 13, after that snapshot
            assert_eq!(js(&s.card(&late).buffs), json!({ "attack": 3, "health": 3 }));
            s.end_turn().end_turn();

            play_rollback(&mut s, &["3"]); // turn 15 → the start of turn 12
            assert_eq!(unit_id(&s, P1, 1), Some(early.id.clone()));
            assert!(matches_object(
                &js(s.card(&early)),
                &json!({ "buffs": { "attack": 3, "health": 3 }, "grantedKeywords": js(&keywords) })
            ));
            assert!(matches_object(
                &js(s.card(&late)),
                &json!({ "buffs": { "attack": 0, "health": 0 }, "grantedKeywords": [] })
            ));
            assert_eq!(s.stats(&late).attack, s.stats(&early).attack - 3);
        }

        #[test]
        fn r419_r566_an_ivory_tower_that_has_fused_a_unit_in_since_goes_back_as_it_stood_unfused_ready_to_take_a_unit_r653() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK, TOKEN_MAKER], "field": [VANILLA], "backrow": [{ "def": TOWER, "lane": 2 }] },
                "p2": { "field": [TIMMY] },
            }));
            let tower = backrow(&s, P1, 2);
            s.play(TOKEN_MAKER, json!({ "zone": 2, "row": "backrow" }));
            // R653: once its play resolved, Me and Mr Token was fused into the Tower.
            assert_ne!(s.card(&tower.id).def_id, TOWER);
            assert!(carried_at(s.state(), ZoneRef { player: P1, row: Row::Backrow, lane: 2 }).is_none());
            s.end_turn().end_turn();

            play_rollback(&mut s, &["2"]); // turn 13 → the start of turn 11, before the fusion
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(tower.id.clone()));
            assert_eq!(s.backrow(P1, 2).map(|card| card.def_id).as_deref(), Some(TOWER));
            assert!(stacked_onto(s.card(&tower.id)).is_none());
            // It stood on the same side and kept its id, so `rolledBack` alone reports it (R566).
            assert!(!event_ids(&of_type(s.last_events(), GameEventType::ControlChanged)).contains(&tower.id));
        }

        #[test]
        fn r419_step_2_from_a_hand_a_graveyard_and_exile_no_cry_for_a_card_that_comes_back_r1() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK, HIT_JOB], "field": [TOKEN_MAKER, { "def": VANILLA, "lane": 4 }] },
                "p2": { "hand": [COLLATERAL, FLOOD], "field": [TIMMY, VANILLA] },
            }));
            let maker = unit(&s, P1, 1);
            let vanilla = unit(&s, P1, 4);
            let timmy = unit(&s, P2, 1);
            let theirs = unit(&s, P2, 2);
            s.play(HIT_JOB, json!({ "targets": target(&timmy.id) })); // to p2's graveyard
            s.end_turn();
            s.play(COLLATERAL, json!({ "targets": target(&vanilla.id) })); // to p1's exile, nothing beside it
            s.end_turn().end_turn();
            s.play(FLOOD, json!({})); // p2's turn 14: every Unit to its owner's hand, and the turn has nothing left (R82)
            assert_eq!(s.state().turn, 15);
            s.expect_in_zone(&maker, "hand")
                .expect_in_zone(&timmy, "graveyard")
                .expect_in_zone(&vanilla, "exile")
                .expect_in_zone(&theirs, "hand");

            play_rollback(&mut s, &["3"]); // turn 15 → the start of turn 12, before the exile and the Flood
            assert_eq!(unit_id(&s, P1, 1), Some(maker.id.clone()));
            assert_eq!(unit_id(&s, P1, 4), Some(vanilla.id.clone()));
            assert_eq!(unit_id(&s, P2, 2), Some(theirs.id.clone()));
            // Timmy died on turn 11, before the snapshot of turn 12: it stays where it is.
            s.expect_in_zone(&timmy, "graveyard");
            assert!(of_type(s.last_events(), GameEventType::Summoned).is_empty());
            let tokens = s
                .state()
                .players
                .p1
                .units
                .iter()
                .flatten()
                .flatten()
                .filter(|card| card.def_id == RUSH_TOKEN)
                .count();
            assert_eq!(tokens, 0);
            assert_eq!(
                event_ids(&of_type(s.last_events(), GameEventType::ControlChanged)),
                [maker.id.clone(), vanilla.id.clone(), theirs.id.clone()]
            );
        }

        #[test]
        fn r419_step_2_a_stolen_card_goes_back_to_its_side_under_its_owner_s_control_r12_r171() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, MIND_CONTROL], "field": [VANILLA] }, "p2": { "field": [VANILLA, TIMMY] } }));
            let timmy = unit(&s, P2, 2);
            s.play(MIND_CONTROL, json!({ "targets": target(&timmy.id) }));
            assert_eq!(unit_id(&s, P1, 2), Some(timmy.id.clone()));
            to_turn_13(&mut s);

            play_rollback(&mut s, &["2"]);
            assert_eq!(unit_id(&s, P2, 2), Some(timmy.id.clone()));
            assert!(s.unit(P1, 2).is_none());
            assert!(matches_object(
                &js(s.card(&timmy)),
                &json!({ "controller": "p2", "owner": "p2", "summonedTurn": 13 })
            ));
            assert_eq!(
                js(&of_type(s.last_events(), GameEventType::ControlChanged)),
                json!([{ "type": "controlChanged", "instanceId": timmy.id, "controller": "p2", "row": "units", "lane": 2 }])
            );
        }

        #[test]
        fn r419_step_2_a_card_that_left_for_its_controller_s_hand_comes_back_out_of_it() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, MIND_CONTROL], "field": [VANILLA] }, "p2": { "hand": [FLOOD], "field": [TIMMY] } }));
            let vanilla = unit(&s, P1, 1);
            let timmy = unit(&s, P2, 1);
            s.play(MIND_CONTROL, json!({ "targets": target(&timmy.id) }));
            s.end_turn(); // turn 12's snapshot holds Timmy on p1's side
            s.play(FLOOD, json!({})); // to its controller, p1 (R747); the turn has nothing left (R82)
            s.expect_in_zone(&timmy, "hand");
            assert!(hand_ids(&s, P1).contains(&timmy.id));
            assert_eq!(s.state().turn, 13);

            play_rollback(&mut s, &["1"]);
            assert_eq!(unit_id(&s, P1, 1), Some(vanilla.id.clone()));
            assert_eq!(unit_id(&s, P1, 2), Some(timmy.id.clone()));
            assert!(!hand_ids(&s, P2).contains(&timmy.id));
            assert!(matches_object(&js(s.card(&timmy)), &json!({ "controller": "p1", "owner": "p2" })));
            // It is public on the field again, so both views name it.
            assert!(event_ids(&of_type(&s.view(P2).events, GameEventType::ControlChanged)).contains(&timmy.id));
        }

        #[test]
        fn r419_step_2_a_card_transformed_since_is_recreated_as_it_was_and_what_replaced_it_leaves_r35() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK], "field": [VANILLA] }, "p2": { "hand": [TRANSMOGULATE], "field": [TIMMY] } }));
            let timmy = unit(&s, P2, 1);
            s.end_turn();
            s.play(TRANSMOGULATE, json!({}));
            let legend = unit(&s, P2, 1);
            assert_ne!(legend.id, timmy.id);
            s.expect_in_zone(&timmy, "gone");
            s.end_turn();

            play_rollback(&mut s, &["1"]);
            assert!(matches_object(
                &js(&s.unit(P2, 1)),
                &json!({ "id": timmy.id, "defId": TIMMY, "controller": "p2" })
            ));
            assert!(hand_ids(&s, P2).contains(&legend.id));
        }

        #[test]
        fn r419_step_2_a_token_that_ceased_to_exist_is_recreated_from_the_snapshot_r11_r175() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, HIT_JOB], "field": [VANILLA, RUSH_TOKEN] }, "p2": { "field": [TIMMY] } }));
            let token = unit(&s, P1, 2);
            s.play(HIT_JOB, json!({ "targets": target(&token.id) }));
            s.expect_in_zone(&token, "gone");
            to_turn_13(&mut s);

            play_rollback(&mut s, &["2"]);
            assert!(matches_object(
                &js(&s.unit(P1, 2)),
                &json!({ "id": token.id, "defId": RUSH_TOKEN, "summonedTurn": 13 })
            ));
            s.expect_stats(&token, json!({ "attack": 3, "health": 3 }));
        }

        #[test]
        fn r419_step_2_a_card_fused_since_goes_back_to_its_own_definition_a_fired_trap_goes_back_face_down_r77() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK], "field": [TIMMY], "backrow": [{ "def": UNLICENSED, "lane": 3 }] },
                "p2": { "hand": [VANILLA], "field": [TIMMY] },
            }));
            let mine = unit(&s, P1, 1);
            let trap = backrow(&s, P1, 3);
            s.end_turn();
            s.play(VANILLA, json!({ "zone": 2 })); // the Trap fuses it onto p1's Timmy
            assert_ne!(s.card(&mine).def_id, TIMMY);
            s.expect_in_zone(&trap, "graveyard");
            s.end_turn();

            play_rollback(&mut s, &["1"]);
            assert_eq!(s.card(&mine).def_id, TIMMY);
            s.expect_stats(&mine, json!({ "attack": 3, "health": 3 }));
            let back = backrow(&s, P1, 3);
            assert_eq!(back.def_id, UNLICENSED);
            assert_ne!(back.face_up, Some(true));
            assert!(!s.state().players.p1.graveyard.iter().any(|card| card.def_id == UNLICENSED));
        }

        #[test]
        fn r419_step_3_the_locks_become_the_snapshot_s_a_card_put_back_face_down_shows_the_opponent_only_a_face_down_card_under_a_fresh_id_r33_r97_r227() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK, MAGIC_JAMMED], "field": [VANILLA] },
                "p2": { "field": [TIMMY], "backrow": [SHEEPISH] },
            }));
            let trap = backrow(&s, P2, 1);
            s.play(MAGIC_JAMMED, json!({ "targets": target(&trap.id) }));
            s.expect_in_zone(&trap, "graveyard");
            assert!(s.state().players.p2.locks.backrow[0]);
            to_turn_13(&mut s);

            play_rollback(&mut s, &["2"]);
            let back = backrow(&s, P2, 1);
            assert_eq!(back.def_id, SHEEPISH);
            assert_ne!(back.id, trap.id);
            assert!(!s.state().players.p2.locks.backrow[0]);
            assert_eq!(
                js(&of_type(s.last_events(), GameEventType::Unlocked)),
                json!([{ "type": "unlocked", "player": "p2", "row": "backrow", "lane": 1 }])
            );
            // p1 sees a face-down card in that zone and nothing that names it.
            let theirs = s.view(P1);
            assert!(matches_object(&js(&theirs.opponent.backrow[0]), &json!({ "faceDown": true })));
            assert!(!serde_json::to_string(&theirs.opponent).unwrap().contains(SHEEPISH));
            let seen = of_type(&theirs.events, GameEventType::ControlChanged)
                .into_iter()
                .find(|event| matches!(event, GameEvent::ControlChanged { row: Row::Backrow, .. }));
            assert_eq!(
                js(&seen),
                json!({ "type": "controlChanged", "instanceId": "hidden", "controller": "p2", "row": "backrow", "lane": 1 })
            );
            // Its controller reads it, and finds the card it was by `formerId` (R227).
            let own = of_type(&s.view(P2).events, GameEventType::ControlChanged)
                .into_iter()
                .find(|event| matches!(event, GameEvent::ControlChanged { row: Row::Backrow, .. }));
            assert_eq!(
                js(&own),
                json!({ "type": "controlChanged", "instanceId": back.id, "controller": "p2", "row": "backrow", "lane": 1, "formerId": trap.id })
            );
        }

        #[test]
        fn r419_health_mana_decks_and_graveyards_change_only_by_the_cards_that_moved_one_rolledback_then_the_moves() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, HIT_JOB], "field": [VANILLA] }, "p2": { "field": [TIMMY, VANILLA] } }));
            let timmy = unit(&s, P2, 1);
            s.play(HIT_JOB, json!({ "targets": target(&timmy.id) }));
            s.end_turn();
            let attacker = unit(&s, P2, 2);
            s.attack(&attacker, "hero");
            s.end_turn();
            let players = &s.state().players;
            let health = [players.p1.hero.health, players.p2.hero.health];
            let libraries = [players.p1.library.len(), players.p2.library.len()];
            let graveyards = [ids(&players.p1.graveyard), ids(&players.p2.graveyard)];

            play_rollback(&mut s, &["2"]);
            let players = &s.state().players;
            assert_eq!([players.p1.hero.health, players.p2.hero.health], health);
            assert_eq!([players.p1.library.len(), players.p2.library.len()], libraries);
            s.expect_mana(P1, 0);
            let mut mine = graveyards[0].clone();
            mine.push(s.card(ROLLBACK).id.clone());
            assert_eq!(ids(&s.state().players.p1.graveyard), mine);
            let theirs: Vec<String> = graveyards[1].iter().filter(|id| **id != timmy.id).cloned().collect();
            assert_eq!(ids(&s.state().players.p2.graveyard), theirs);
            assert_eq!(
                js(&of_type(s.last_events(), GameEventType::RolledBack)),
                json!([{ "type": "rolledBack", "player": "p1", "turnsAgo": 2, "sides": ["p1", "p2"] }])
            );
            assert!(
                index_of(s.last_events(), GameEventType::RolledBack) < index_of(s.last_events(), GameEventType::ControlChanged)
            );
            assert!(!s.last_events().iter().any(|event| event.event_type() == GameEventType::Destroyed));
        }

        #[test]
        fn r419_indestructible_and_immutable_change_nothing_a_move_is_no_destroy_and_no_transform() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK], "field": [VANILLA] },
                "p2": { "hand": [{ "def": ROCK, "radiant": true }], "field": [TIMMY] },
            }));
            let timmy = unit(&s, P2, 1);
            s.end_turn();
            s.play(ROCK, json!({ "zone": 2, "tributes": [timmy.id] }));
            let rock = unit(&s, P2, 2);
            s.end_turn();

            play_rollback(&mut s, &["1"]);
            s.expect_in_zone(&rock, "hand");
            assert_eq!(unit_id(&s, P2, 1), Some(timmy.id.clone()));
            assert!(of_type(s.last_events(), GameEventType::Destroyed).is_empty());
        }

        #[test]
        fn r419_reborn_a_unit_that_came_back_through_reborn_goes_back_to_the_snapshot_s_instance_reborn_and_divine_shield_again() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, HIT_JOB], "field": [VANILLA, DEFENDER] }, "p2": { "field": [TIMMY] } }));
            let defender = unit(&s, P1, 2);
            s.play(HIT_JOB, json!({ "targets": target(&defender.id) }));
            assert_eq!(s.card(&defender).reborn_spent, Some(true));
            to_turn_13(&mut s);

            play_rollback(&mut s, &["2"]);
            assert_eq!(unit_id(&s, P1, 2), Some(defender.id.clone()));
            assert!(s.card(&defender).reborn_spent.is_none());
            let kinds: Vec<KeywordKind> = s.stats(&defender).keywords.iter().map(|keyword| keyword.kind()).collect();
            assert!(kinds.contains(&KeywordKind::Reborn));
            assert!(kinds.contains(&KeywordKind::DivineShield));
            s.expect_stats(&defender, json!({ "health": 1, "maxHealth": 1 }));
        }
    }

    mod c_n35_rollback_what_a_put_back_card_keeps_r566 {
        use super::*;

        #[test]
        fn r566_a_card_that_stood_on_its_side_keeps_its_exertion_and_sickness_one_from_anywhere_else_entered_this_turn() {
            crate::register_all();
            let mut s = on_turn_11(json!({ "p1": { "hand": [ROLLBACK, HIT_JOB], "field": [VANILLA, TIMMY] }, "p2": { "field": [VANILLA] } }));
            let timmy = unit(&s, P1, 2);
            s.play(HIT_JOB, json!({ "targets": target(&timmy.id) }));
            to_turn_13(&mut s);
            let vanilla = unit(&s, P1, 1);
            s.attack(&vanilla, "hero");

            play_rollback(&mut s, &["2"]);
            assert!(s.card(&vanilla).exertion.attacked);
            assert!(s.card(&vanilla).summoned_turn.is_none());
            assert_eq!(s.card(&timmy).summoned_turn, Some(13));
            let attackers: Vec<String> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Attack { attacker_id, .. } => Some(attacker_id),
                    _ => None,
                })
                .collect();
            assert!(!attackers.contains(&vanilla.id));
            // Tempo Timmy has Rush: back on the field this turn, it may attack a Unit, never the hero (§6.1).
            assert!(!legal_actions(s.state(), P1).iter().any(|action| matches!(
                action,
                ActionBody::Attack { attacker_id, target_id } if *attacker_id == timmy.id && target_id == "hero-p2"
            )));
        }
    }

    mod c_n35_rollback_too_little_history_r562 {
        use super::*;

        #[test]
        fn r562_with_fewer_turns_recorded_than_n_it_goes_back_as_far_as_the_history_goes() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ROLLBACK], "field": [VANILLA], "library": filler() },
                "p2": { "hand": [TIMMY], "field": [VANILLA], "library": filler() },
            }));
            s.end_turn(); // turn 10, p2's: the first snapshot, then p2 plays a Tempo Timmy
            let timmy = s.play(TIMMY, json!({ "zone": 2 })).unit(P2, 2).expect("the Timmy");
            s.end_turn();
            play_rollback(&mut s, &["3"]);
            // Turn 8 was never recorded; the oldest snapshot, turn 10's, is where it goes.
            assert_eq!(
                js(&of_type(s.last_events(), GameEventType::RolledBack)),
                json!([{ "type": "rolledBack", "player": "p1", "turnsAgo": 1, "sides": ["p1", "p2"] }])
            );
            s.expect_in_zone(&timmy, "hand");
        }

        /// TS's `act`: one action, its nonce numbered by the actions so far.
        fn act(state: &mut GameState, n: &mut i32, body: Value) {
            *n += 1;
            let mut with_nonce = body;
            with_nonce["nonce"] = json!(format!("first-{n}"));
            let result = reduce(state, &json_as(with_nonce));
            if let Some(error) = result.error {
                panic!("{error}");
            }
            *state = result.state;
        }

        #[test]
        fn r562_a_game_s_first_turns_the_oldest_snapshot_is_the_start_of_turn_1_the_empty_board() {
            crate::register_all();
            let mut deck = vec![ROLLBACK];
            deck.extend([
                "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015", "core-016",
                "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053",
                "core-055",
            ]);
            let args: CreateGameArgs = json_as(json!({ "seed": "rollback-first-turn", "decks": [deck, deck] }));
            let mut state = begin_game(&create_game(&args)).state;
            let mut n = 0;
            for player in [P1, P2] {
                let keep: Vec<String> = ids(&state.players[player].hand);
                act(&mut state, &mut n, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
            }
            assert_eq!(
                state.board_history.as_ref().map(|history| history.iter().map(|snapshot| snapshot.turn).collect::<Vec<_>>()),
                Some(vec![1])
            );
            assert_eq!(subsystems::snapshot_for(&state, 3).map(|snapshot| snapshot.turn), Some(1));
            let first = state.board_history.as_ref().and_then(|history| history.first()).expect("a snapshot");
            assert!(first.sides.p1.units.iter().all(|pile| pile.is_none()));
        }

        #[test]
        fn r562_with_no_snapshot_at_all_nothing_is_restored_and_nothing_is_reported() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [ROLLBACK], "field": [VANILLA] }, "p2": { "field": [TIMMY] } }));
            play_rollback(&mut s, &["1"]);
            assert!(of_type(s.last_events(), GameEventType::RolledBack).is_empty());
            assert_eq!(unit_def(&s, P1, 1).as_deref(), Some(VANILLA));
            assert_eq!(unit_def(&s, P2, 1).as_deref(), Some(TIMMY));
        }
    }

    mod c_n35_rollback_held_zones_r563 {
        use super::*;

        #[test]
        fn r563_an_animated_card_s_home_is_let_go_and_the_card_stands_in_it_again_as_the_snapshot_had_it() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [ROLLBACK], "field": [VANILLA], "backrow": [{ "def": SPATULA, "lane": 2 }] },
                "p2": { "field": [TIMMY] },
            }));
            let spatula = unit(&s, P1, 2);
            assert_eq!(spatula.def_id, SPATULA);
            assert_eq!(
                js(&s.state().homes),
                json!([{ "instanceId": spatula.id, "zone": { "player": "p1", "row": "backrow", "lane": 2 } }])
            );

            play_rollback(&mut s, &["1"]);
            assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(spatula.id.clone()));
            assert!(s.unit(P1, 2).is_none());
            assert!(s.state().homes.is_none());
            // It moved along its own side: nothing but `rolledBack` reports it (R171, R566).
            assert!(of_type(s.last_events(), GameEventType::ControlChanged).is_empty());
        }
    }

    mod c_n35_rollback_radiant_your_side_your_opponent_s_or_both {
        use super::*;

        /// Answers `(s, mine, theirs)`: a Tempo Timmy each side played after the snapshot of turn 11.
        fn board() -> (Scenario, String, String) {
            let mut s = on_turn_11(json!({
                "p1": { "hand": [radiant_rollback(), TIMMY], "field": [VANILLA] },
                "p2": { "hand": [TIMMY], "field": [VANILLA] },
            }));
            s.play(TIMMY, json!({ "zone": 2 }));
            let mine = unit(&s, P1, 2).id;
            s.end_turn();
            s.play(TIMMY, json!({ "zone": 2 }));
            let theirs = unit(&s, P2, 2).id;
            s.end_turn();
            (s, mine, theirs)
        }

        #[test]
        fn your_side_only_your_side_goes_back() {
            crate::register_all();
            let (mut s, mine, theirs) = board();
            play_rollback(&mut s, &["2", "your side"]);
            s.expect_in_zone(&mine, "hand");
            assert_eq!(unit_id(&s, P2, 2), Some(theirs));
            assert_eq!(
                js(&of_type(s.last_events(), GameEventType::RolledBack)),
                json!([{ "type": "rolledBack", "player": "p1", "turnsAgo": 2, "sides": ["p1"] }])
            );
        }

        #[test]
        fn your_opponent_s_side_only_theirs_goes_back() {
            crate::register_all();
            let (mut s, mine, theirs) = board();
            play_rollback(&mut s, &["2", "your opponent's side"]);
            assert_eq!(unit_id(&s, P1, 2), Some(mine));
            s.expect_in_zone(&theirs, "hand");
            assert_eq!(
                js(&of_type(s.last_events(), GameEventType::RolledBack)),
                json!([{ "type": "rolledBack", "player": "p1", "turnsAgo": 2, "sides": ["p2"] }])
            );
        }

        #[test]
        fn both_sides_the_whole_board_as_the_base_face() {
            crate::register_all();
            let (mut s, mine, theirs) = board();
            play_rollback(&mut s, &["2", "both sides"]);
            s.expect_in_zone(&mine, "hand").expect_in_zone(&theirs, "hand");
        }

        #[test]
        fn r419_a_card_on_the_restored_side_that_the_other_side_s_snapshot_holds_still_leaves_only_that_part_goes_back() {
            crate::register_all();
            let mut s = on_turn_11(json!({
                "p1": { "hand": [radiant_rollback(), MIND_CONTROL], "field": [VANILLA] },
                "p2": { "field": [VANILLA, TIMMY] },
            }));
            let timmy = unit(&s, P2, 2);
            s.play(MIND_CONTROL, json!({ "targets": target(&timmy.id) }));
            to_turn_13(&mut s);
            play_rollback(&mut s, &["2", "your side"]);
            // p1's side as it was holds no Timmy, and p2's side is not restored: it goes to its controller's hand (R747).
            assert!(hand_ids(&s, P1).contains(&timmy.id));
        }
    }

    mod c_n35_rollback_a_whole_game_s9_3 {
        use super::*;

        /// TS's `act`: one action of the log, its nonce numbered by the log so far.
        fn act(state: &mut GameState, log: &mut Vec<Action>, body: Value, nonce: String) {
            let mut with_nonce = body;
            with_nonce["nonce"] = json!(nonce);
            let action: Action = json_as(with_nonce);
            let result = reduce(state, &action);
            if let Some(error) = result.error {
                panic!("{error}");
            }
            log.push(action);
            *state = result.state;
        }

        #[test]
        fn r419_a_game_using_it_replays_to_the_same_hash_and_the_same_history() {
            crate::register_all();
            let deck = [
                ROLLBACK, "core-008", "core-011", "core-015", "core-002", "core-005", "core-006", "core-012", "core-013",
                "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044",
                "core-053", "core-055",
            ];
            let other = [
                "core-008", "core-011", "core-015", "core-002", "core-005", "core-006", "core-012", "core-013", "core-016",
                "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043", "core-044", "core-053",
                "core-055", "core-004",
            ];
            let decks = json!([deck, other]);
            let mut found: Option<(String, GameState)> = None;
            let mut at = 0;
            while at < 300 && found.is_none() {
                let seed = format!("rollback-replay-{at}");
                let args: CreateGameArgs = json_as(json!({ "seed": seed, "decks": decks }));
                let begun = begin_game(&create_game(&args)).state;
                if begun.players.p1.hand.iter().any(|card| card.def_id == ROLLBACK) {
                    found = Some((seed, begun));
                }
                at += 1;
            }
            let (seed, mut state) = found.expect("no seed deals Rollback to p1");
            let mut log: Vec<Action> = Vec::new();
            for player in [P1, P2] {
                let keep = ids(&state.players[player].hand);
                let nonce = format!("rb-{}", log.len());
                act(&mut state, &mut log, json!({ "type": "mulligan", "keep": keep, "playerId": player }), nonce);
            }
            // Each side plays its cheapest Unit each turn until p1's fourth turn, when Rollback is affordable.
            while !(state.active == P1 && state.players.p1.mana.current >= 4) {
                let player = state.active;
                let play = legal_actions(&state, player).into_iter().find(|action| {
                    matches!(action, ActionBody::Play { zone: Some(zone), .. } if zone.row == Row::Units)
                });
                if let Some(play) = play {
                    let mut body = js(&play);
                    body["playerId"] = json!(player);
                    let nonce = format!("rb-{}", log.len());
                    act(&mut state, &mut log, body, nonce);
                }
                let nonce = format!("rb-{}", log.len());
                act(&mut state, &mut log, json!({ "type": "endTurn", "playerId": player }), nonce);
            }
            let rollback = state
                .players
                .p1
                .hand
                .iter()
                .find(|card| card.def_id == ROLLBACK)
                .map(|card| card.id.clone())
                .unwrap_or_default();
            let nonce = format!("rb-{}", log.len());
            act(
                &mut state,
                &mut log,
                json!({ "type": "play", "playerId": "p1", "instanceId": rollback, "modes": ["2"] }),
                nonce,
            );
            assert!(state.players.p1.graveyard.iter().any(|card| card.def_id == ROLLBACK));

            let replayed = fold(&json_as(json!({ "seed": seed, "decks": decks, "log": js(&log) })));
            assert!(replayed.errors.is_empty());
            assert_eq!(hash_state(&replayed.state), hash_state(&state));
            assert_eq!(replayed.state.board_history, state.board_history);
        }

        /// TS ran this with `{ timeout: 120_000 }`; cargo test has no per-test timeout.
        #[test]
        fn r419_random_policy_games_with_rollback_in_both_decks_keep_the_fuzz_invariants_and_replay_s9_3_r171() {
            crate::register_all();
            const POOL: [&str; 22] = [
                "core-008", "core-011", "core-015", "core-002", "core-012", "core-016", "core-017", "core-019", "core-020",
                "core-025", "core-026", "core-036", "core-041", "core-043", "core-049", "core-053", "core-055", "core-060",
                "core-092", "core-003", "core-018", "core-071",
            ];
            let deck = |offset: usize| -> Vec<&str> {
                let mut cards = vec![ROLLBACK];
                cards.extend(POOL.iter().skip(offset).take(19));
                cards
            };
            let mut rolled_back = 0;
            for seed in 1..=8 {
                let decks = json!([deck(0), deck(3)]);
                let game_seed = format!("rollback-fuzz-{seed}");
                let args: CreateGameArgs = json_as(json!({ "seed": game_seed, "decks": decks }));
                let mut state = begin_game(&create_game(&args)).state;
                let mut policy = Rng::new(&format!("rollback-policy-{seed}"), 0);
                let mut monitor = create_invariant_monitor(&state);
                let mut log: Vec<Action> = Vec::new();
                let mut step = 0;
                while state.result.is_none() && step < 5000 {
                    let seat = seat_to_act(&state).expect("a seat to act");
                    let chosen = match subsystems::choose_action(&state, seat, &mut policy) {
                        Some(chosen) => chosen,
                        None => panic!("no action for {seat}"),
                    };
                    assert_eq!(monitor.before(&state, seat, &chosen), Vec::<String>::new());
                    let mut body = js(&chosen);
                    body["playerId"] = json!(seat);
                    body["nonce"] = json!(format!("rf-{seed}-{step}"));
                    let action: Action = json_as(body);
                    let result = reduce(&state, &action);
                    if let Some(error) = &result.error {
                        panic!("{error}");
                    }
                    log.push(action);
                    state = result.state;
                    assert_eq!(monitor.after(&result.events, &state), Vec::<String>::new());
                    rolled_back += result
                        .events
                        .iter()
                        .filter(|event| event.event_type() == GameEventType::RolledBack)
                        .count();
                    step += 1;
                }
                assert!(state.result.is_some());
                let replayed = fold(&json_as(json!({ "seed": game_seed, "decks": decks, "log": js(&log) })));
                assert!(replayed.errors.is_empty());
                assert_eq!(hash_state(&replayed.state), hash_state(&state));
            }
            // The games did cast it: the property is about Rollback, not about games that never drew it.
            assert!(rolled_back > 0);
        }
    }
}
