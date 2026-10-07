//! Port of `packages/engine/test/windfury.test.ts`.
//!
//! Windfury (SPEC §6.1, R636): a Unit may attack twice each turn. The second attack is a full one, the
//! first spends the exertion a switch needs, and `legalActions` and the reducer's refusal agree on it
//! because both ask `combat.hasExertion` (BUILD M2-T1).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{deft_duelist, plain, windfurier};
use crate::rules::fixtures::harness::{new_game, put, slot};

/// TS's module `let nonce = 0`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn attempt(state: &GameState, body: ActionInput) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    reduce(state, &body.with_nonce(format!("wf{nonce}")))
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    let result = attempt(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn input(body: ActionBody, player_id: PlayerId) -> ActionInput {
    ActionInput { body, player_id }
}

/// Past the mulligans, in the main phase of turn 1.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, input(ActionBody::Mulligan { keep }, PlayerId::P1));
    let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, input(ActionBody::Mulligan { keep }, PlayerId::P2));
    state
}

fn unit_at(state: &GameState, player: PlayerId, lane: i32) -> &CardInstance {
    state.players[player]
        .units
        .get((lane - 1) as usize)
        .and_then(|pile| pile.as_ref())
        .and_then(|pile| pile.first())
        .unwrap_or_else(|| panic!("no unit in {player} lane {lane}"))
}

/// `unitAt(...)` written through, as TS wrote through the live object.
fn unit_at_mut(state: &mut GameState, player: PlayerId, lane: i32) -> &mut CardInstance {
    state.players[player]
        .units
        .get_mut((lane - 1) as usize)
        .and_then(|pile| pile.as_mut())
        .and_then(|pile| pile.first_mut())
        .unwrap_or_else(|| panic!("no unit in {player} lane {lane}"))
}

fn attack_body(state: &GameState, lane: i32) -> ActionInput {
    input(
        ActionBody::Attack {
            attacker_id: unit_at(state, PlayerId::P1, lane).id.clone(),
            target_id: "hero-p2".to_string(),
        },
        PlayerId::P1,
    )
}

fn attack_hero(state: &GameState, lane: i32) -> GameState {
    act(state, attack_body(state, lane))
}

/// Whether `legalActions` offers an attack by this unit (and so the reducer must accept it).
fn offers_attack(state: &GameState, unit: &CardInstance) -> bool {
    legal_actions(state, PlayerId::P1)
        .iter()
        .any(|a| matches!(a, ActionBody::Attack { attacker_id, .. } if *attacker_id == unit.id))
}

/// TS's `sinkFor(state)` for one call: a sink whose rng starts at the state's cursor, as reduce does.
fn with_sink<T>(state: &mut GameState, f: impl FnOnce(&mut EngineSink<'_>) -> T) -> T {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    f(&mut sink)
}

/// `expect(error).toMatch(/text/)`: a refusal whose message contains the text.
fn expect_refusal(error: Option<String>, text: &str) {
    match error {
        Some(message) => assert!(message.contains(text), "expected {message:?} to match /{text}/"),
        None => panic!("expected a refusal matching /{text}/, got none"),
    }
}

mod r636_windfury {
    use super::*;

    #[test]
    fn r636_a_unit_with_windfury_attacks_twice_in_a_turn_each_a_full_attack_and_not_a_third_time() {
        assert_eq!(WINDFURY_ATTACKS, 2);
        let mut state = playing("windfury-twice");
        put(
            &mut state,
            &windfurier.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        assert_eq!(attacks_per_turn(&state, unit_at(&state, PlayerId::P1, 1)), 2);

        state = attack_hero(&state, 1);
        assert_eq!(state.players.p2.hero.health, 27);
        // The first attack leaves the unit able to declare another, in `legalActions` and in the reducer.
        assert!(offers_attack(&state, unit_at(&state, PlayerId::P1, 1)));

        state = attack_hero(&state, 1);
        assert_eq!(state.players.p2.hero.health, 24);
        assert_eq!(
            unit_at(&state, PlayerId::P1, 1).exertion,
            Exertion {
                attacked: true,
                switched: false,
                attacks: Some(2)
            }
        );

        // The third is refused the way a second attack by any unit is, and `legalActions` no longer lists it.
        assert!(!offers_attack(&state, unit_at(&state, PlayerId::P1, 1)));
        expect_refusal(attempt(&state, attack_body(&state, 1)).error, "already acted");
        assert_eq!(state.players.p2.hero.health, 24);
    }

    #[test]
    fn r636_r6_a_unit_without_windfury_still_attacks_once_and_the_first_windfury_attack_spends_the_switch() {
        let mut state = playing("windfury-switch");
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut state,
            &windfurier.id,
            slot(PlayerId::P1, Row::Units, 2),
            Default::default(),
        );

        state = attack_hero(&state, 1);
        assert!(!offers_attack(&state, unit_at(&state, PlayerId::P1, 1)));

        state = attack_hero(&state, 2);
        let wind = unit_at(&state, PlayerId::P1, 2).clone();
        // Attacked once, with an attack left: it cannot also switch, since a switch takes the exertion an attack spent.
        assert!(has_exertion(&state, &wind, ExertionKind::Attack));
        assert!(!has_exertion(&state, &wind, ExertionKind::Switch));
        assert!(
            !legal_actions(&state, PlayerId::P1)
                .iter()
                .any(|a| matches!(a, ActionBody::SwitchPosition { instance_id } if *instance_id == wind.id))
        );
        expect_refusal(
            attempt(
                &state,
                input(
                    ActionBody::SwitchPosition {
                        instance_id: wind.id.clone(),
                    },
                    PlayerId::P1,
                ),
            )
            .error,
            "already acted",
        );
    }

    #[test]
    fn r636_r6_a_unit_that_switched_cannot_attack_windfury_or_not() {
        let mut state = playing("windfury-switched");
        put(
            &mut state,
            &windfurier.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        let instance_id = unit_at(&state, PlayerId::P1, 1).id.clone();
        state = act(
            &state,
            input(ActionBody::SwitchPosition { instance_id }, PlayerId::P1),
        );
        // An effect flips it back to Attack Position (R20, no exertion): the switch it made already spent the turn's.
        let card = unit_at(&state, PlayerId::P1, 1).clone();
        let flipped = with_sink(&mut state, |sink| {
            switch_position(
                sink,
                &card,
                SwitchPositionOptions {
                    spend_exertion: Some(false),
                    to: Some(Position::Atk),
                },
            )
        });
        assert!(flipped.is_ok(), "{flipped:?}");
        let wind = unit_at(&state, PlayerId::P1, 1).clone();
        assert!(!has_exertion(&state, &wind, ExertionKind::Attack));
        assert!(!offers_attack(&state, &wind));
        expect_refusal(
            attempt(
                &state,
                input(
                    ActionBody::Attack {
                        attacker_id: wind.id.clone(),
                        target_id: "hero-p2".to_string(),
                    },
                    PlayerId::P1,
                ),
            )
            .error,
            "already acted",
        );
    }

    #[test]
    fn r636_the_second_attack_comes_back_next_turn_both_attacks_are_available_again_at_its_controllers_start()
    {
        let mut state = playing("windfury-reset");
        put(
            &mut state,
            &windfurier.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        state = attack_hero(&state, 1);
        state = attack_hero(&state, 1);
        state = act(&state, input(ActionBody::EndTurn, PlayerId::P1));
        state = act(&state, input(ActionBody::EndTurn, PlayerId::P2));
        assert_eq!(state.active, PlayerId::P1);
        assert_eq!(
            unit_at(&state, PlayerId::P1, 1).exertion,
            Exertion {
                attacked: false,
                switched: false,
                attacks: None
            }
        );
        state = attack_hero(&state, 1);
        state = attack_hero(&state, 1);
        assert_eq!(state.players.p2.hero.health, 18);
    }

    #[test]
    fn r636_the_count_is_read_when_the_second_attack_is_declared_windfury_gained_after_one_attack_gives_another_lost_it_takes_it()
     {
        let mut state = playing("windfury-gain-lose");
        put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        put(
            &mut state,
            &windfurier.id,
            slot(PlayerId::P1, Row::Units, 2),
            Default::default(),
        );

        state = attack_hero(&state, 1);
        state = attack_hero(&state, 2);
        assert!(!offers_attack(&state, unit_at(&state, PlayerId::P1, 1)));

        unit_at_mut(&mut state, PlayerId::P1, 1)
            .granted_keywords
            .push(Keyword::Windfury);
        assert!(offers_attack(&state, unit_at(&state, PlayerId::P1, 1)));
        state = attack_hero(&state, 1);
        assert!(!offers_attack(&state, unit_at(&state, PlayerId::P1, 1)));

        // The other unit loses its Windfury (here: it is Vanilla'd, which takes the printed keyword) after one attack.
        unit_at_mut(&mut state, PlayerId::P1, 2).vanilla = true;
        assert_eq!(attacks_per_turn(&state, unit_at(&state, PlayerId::P1, 2)), 1);
        assert!(!offers_attack(&state, unit_at(&state, PlayerId::P1, 2)));
        expect_refusal(attempt(&state, attack_body(&state, 2)).error, "already acted");
    }

    #[test]
    fn r636_deft_duelist_keeps_its_independent_switch_beside_windfury() {
        let mut state = playing("windfury-duelist");
        let duelist = put(
            &mut state,
            &deft_duelist.id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        find_instance_mut(&mut state, &duelist.id)
            .expect("the duelist on the field")
            .granted_keywords
            .push(Keyword::Windfury);
        let next = attack_hero(&state, 1);
        let after = unit_at(&next, PlayerId::P1, 1);
        // One attack made, one left, and the duelist's own switch still unspent.
        assert!(has_exertion(&next, after, ExertionKind::Attack));
        assert!(has_exertion(&next, after, ExertionKind::Switch));
    }
}
