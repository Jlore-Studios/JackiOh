//! Duos with the real cards (#1168). Every card sees a 1v1 game (§13.1), so a card in a duel plays as
//! it does in a 1v1 game but for what §13 changes: the round, the shared life and passing. The
//! wrapper's own rules are proven with fixture scripts in the engine's `tests/rules/duos.rs`; these
//! are the catalog's cards inside it.
//!
//! Two setups: `duos_scenario`, the real wrapper, where the round, the pools or a pass matter; and a
//! 1v1 `scenario()` with a side's `duos` life set, which is what `duos_create` puts on a duel's side
//! (`duos_seat`), for a rule that lives inside one duel.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::config::DUOS_HUMAN_TEAM;
use jackioh_engine::duos::*;
use jackioh_engine::random_targets::draw_attack_target;
use jackioh_engine::subsystems::{
    ScorePriority, ScorerOptions, dry_run_base, judge_play, rank, score_instance,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::DuosSeat::{A1, A2, B1, B2};

const VANILLA: &str = "core-008"; // (1) Unit 4/4.
const FILLER: &str = "core-005"; // (1) Spell Stockpile: draw 1, heal your hero 3.
const COIN: &str = "core-t-coin";
const TEMPO: &str = "classicplus-026"; // Tommy Tempo: cast on draw, ends its caster's turn (R456).

// ---------------------------------------------------------------------------------------------
// The wrapper
// ---------------------------------------------------------------------------------------------

/// Twenty distinct Core cards with no cast on draw, no Trap and no Quickdraw.
fn deck() -> Vec<String> {
    [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 19, 20, 22,
    ]
    .iter()
    .map(|n| format!("core-{n:03}"))
    .collect()
}

fn begun_with(seed: &str, teams: Option<PerTeam<DuosTeamRules>>) -> DuosState {
    jackioh_cards::register_all();
    let decks = DuosDecks {
        a1: deck(),
        a2: deck(),
        b1: deck(),
        b2: deck(),
    };
    duos_scenario(seed, decks, teams)
}

fn begun(seed: &str) -> DuosState {
    begun_with(seed, None)
}

/// Every seat holds `hand` over a library of Vanillas, so no draw does anything.
fn quiet(seed: &str, hand: &[&str]) -> DuosState {
    let mut state = begun(seed);
    for seat in DUOS_SEATS {
        rig(&mut state, seat, hand, &[VANILLA; 14]);
    }
    state
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn nonce() -> String {
    format!("cross-duos-{}", NONCE.fetch_add(1, Ordering::SeqCst) + 1)
}

fn act(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> DuosReduceResult {
    duos_reduce(
        state,
        &DuosAction {
            seat,
            nonce: nonce(),
            body,
        },
    )
}

/// The action, which must be accepted.
fn ok(state: &DuosState, seat: DuosSeat, body: DuosActionBody) -> DuosState {
    let what = serde_json::to_string(&body).unwrap_or_default();
    let result = act(state, seat, body);
    if let Some(error) = &result.error {
        panic!("{seat} {what} was refused: {error}");
    }
    result.state
}

fn end_turn() -> DuosActionBody {
    DuosActionBody::Duel {
        body: ActionBody::EndTurn,
    }
}

fn pass(instance_id: &str) -> DuosActionBody {
    DuosActionBody::Pass {
        instance_id: instance_id.to_string(),
    }
}

fn side(state: &DuosState, seat: DuosSeat) -> &PlayerState {
    &state.duels[seat.duel()].players[seat.player()]
}

fn side_mut(state: &mut DuosState, seat: DuosSeat) -> &mut PlayerState {
    &mut state.duels[seat.duel()].players[seat.player()]
}

/// A new card at the right end of the seat's hand; its id.
fn give(state: &mut DuosState, seat: DuosSeat, def_id: &str) -> String {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let card = new_instance(game, def_id, player, Zone::Hand { player });
    let id = card.id.clone();
    game.players[player].hand.push(card);
    id
}

/// Replaces the seat's hand and library.
fn rig(state: &mut DuosState, seat: DuosSeat, hand: &[&str], library: &[&str]) {
    let player = seat.player();
    side_mut(state, seat).hand.clear();
    side_mut(state, seat).library.clear();
    for def_id in hand {
        give(state, seat, def_id);
    }
    for def_id in library {
        let game = &mut state.duels[seat.duel()];
        let card = new_instance(game, def_id, player, Zone::Library { player });
        game.players[player].library.push(card);
    }
}

/// Puts `def_id` on top of the seat's library: its next draw.
fn draws_next(state: &mut DuosState, seat: DuosSeat, def_id: &str) {
    let player = seat.player();
    let game = &mut state.duels[seat.duel()];
    let card = new_instance(game, def_id, player, Zone::Library { player });
    game.players[player].library.insert(0, card);
}

fn held(state: &DuosState, seat: DuosSeat, def_id: &str) -> Option<String> {
    side(state, seat)
        .hand
        .iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id.clone())
}

/// The play of `id` the wrapper offers (its first).
fn legal_play(state: &DuosState, seat: DuosSeat, id: &str) -> DuosActionBody {
    duos_legal_actions(state, seat)
        .into_iter()
        .find(|offered| {
            matches!(offered, DuosActionBody::Duel { body: ActionBody::Play { instance_id, .. } } if instance_id == id)
        })
        .unwrap_or_else(|| panic!("{seat} is offered no play of {id}"))
}

fn play_def(state: &DuosState, seat: DuosSeat, def_id: &str) -> DuosState {
    let id = held(state, seat, def_id).unwrap_or_else(|| panic!("{seat} holds no {def_id}"));
    let body = legal_play(state, seat, &id);
    ok(state, seat, body)
}

/// A play of `id` with these declared modes.
fn play_with_modes(id: &str, modes: &[&str]) -> DuosActionBody {
    let mut body = json!({ "type": "play", "instanceId": id });
    if !modes.is_empty() {
        body["modes"] = json!(modes);
    }
    DuosActionBody::Duel { body: json_as(body) }
}

/// Every round seat that has not ended ends its turn; the round must close.
fn close_round(state: DuosState) -> DuosState {
    let round = state.round;
    let mut state = state;
    for duel in 0..DUOS_DUELS {
        if state.round == round && !state.ended[duel] {
            let seat = state.round_seats[duel];
            state = ok(&state, seat, end_turn());
        }
    }
    assert_eq!(state.round, round + 1, "round {round} closed");
    state
}

/// The id the last pass gave its card in the teammate's duel.
fn last_passed(events: &[DuosEvent]) -> String {
    events
        .iter()
        .rev()
        .find_map(|event| match event {
            DuosEvent::Passed { new_instance_id, .. } => Some(new_instance_id.clone()),
            _ => None,
        })
        .expect("a pass")
}

fn keep_all(state: &DuosState, seat: DuosSeat) -> DuosState {
    let prompt = jackioh_engine::setup::mulligan_prompt_for(&state.duels[seat.duel()], seat.player())
        .unwrap_or_else(|| panic!("{seat} owes no mulligan"));
    let keep: Vec<String> = prompt.options.iter().map(|option| option.key.clone()).collect();
    ok(
        state,
        seat,
        DuosActionBody::Duel {
            body: ActionBody::Mulligan { keep },
        },
    )
}

// ---------------------------------------------------------------------------------------------
// One duel with Duos life
// ---------------------------------------------------------------------------------------------

/// Makes `player`'s side a Duos seat: own HP `own`, its team's pool `pool` of cap `cap`, spent or not.
/// A spent seat's health reads the pool (§13.4).
fn duos_side(s: &mut Scenario, player: PlayerId, own: i32, pool: i32, cap: i32, spent: bool) {
    let side = &mut s.state_mut().players[player];
    side.hero.health = if spent { pool } else { own };
    side.duos = Some(DuosLife {
        pool,
        pool_cap: cap,
        spent,
    });
}

// =============================================================================================
// Meditative #19 Expedition123 and #19.1 Temporal Rift (R844, R846)
// =============================================================================================

mod expedition123 {
    use super::*;

    const EXPEDITION: &str = "meditative-019";
    const RIFT: &str = "meditative-019-1";

    /// A1 casts Expedition123, then its Rift off two Coins, in round 1. The lost refreshes land on
    /// A1's extra turn and on its next own turn, in duel 0 alone; the third is normal, as in the 1v1
    /// game below. The extra turn keeps duel 0's round on A1 (§13.3), so the rounds run [A1, B2],
    /// [B1, A2] from then on, and no seat acts out of its round.
    #[test]
    fn r844_r846_expedition123_and_its_rift_lose_the_casters_own_turns_in_its_own_duel() {
        let mut s = begun("cross-duos-expedition");
        rig(&mut s, A1, &[EXPEDITION, COIN, COIN], &[VANILLA; 12]);
        for seat in [A2, B1, B2] {
            rig(&mut s, seat, &[VANILLA, COIN], &[VANILLA; 12]);
        }
        let s = play_def(&s, A1, EXPEDITION);
        assert!(held(&s, A1, RIFT).is_some(), "the Rift is in hand");
        assert_eq!(
            side(&s, A2).lost_refresh_through,
            None,
            "the teammate loses nothing"
        );
        let s = play_def(&s, A1, COIN);
        let s = play_def(&s, A1, COIN);
        let s = play_def(&s, A1, RIFT);
        assert_eq!(side(&s, A1).extra_turns, Some(1));
        assert_eq!(side(&s, A2).rift_extra_turn, None, "each player's own Rift flag");

        let mut mana = Vec::new();
        let mut s = s;
        for round in 2..=6 {
            s = close_round(s);
            let expected = if round % 2 == 0 { [A1, B2] } else { [B1, A2] };
            assert_eq!(s.round_seats, expected, "round {round}");
            for duel in 0..DUOS_DUELS {
                assert_eq!(
                    s.round_seats[duel],
                    DuosSeat::of(duel, s.duels[duel].active),
                    "round {round}"
                );
                assert_eq!(
                    s.duels[duel].turn, round,
                    "round {round}: each duel's turn moves once a round"
                );
            }
            if round % 2 == 0 {
                mana.push(side(&s, A1).mana.current);
            } else {
                assert_eq!(
                    side(&s, A2).mana.current,
                    side(&s, A2).mana.max,
                    "A2's refreshes are whole"
                );
            }
        }
        assert_eq!(
            mana,
            vec![0, 0, 4],
            "the extra turn and A1's next turn are lost, the third is not"
        );

        // The same plays in a 1v1 game: the same mana on the same turns.
        jackioh_cards::register_all();
        let library = [VANILLA; 6];
        let mut one = scenario(json!({
            "seed": "cross-duos-expedition-1v1",
            "turn": 1,
            "p1": { "hand": [EXPEDITION, COIN, COIN], "library": library, "mana": 1 },
            "p2": { "hand": [VANILLA], "library": library },
        }));
        // A duel's players have R82's automatic turn end off (B9), so a lost turn is ended by hand.
        one.state_mut().players.p1.auto_end_turn = Some(false);
        one.state_mut().players.p2.auto_end_turn = Some(false);
        one.play(EXPEDITION, json!({}));
        one.play(COIN, json!({}));
        one.play(COIN, json!({}));
        one.play(RIFT, json!({}));
        let mut solo = Vec::new();
        one.end_turn();
        solo.push(one.state().players.p1.mana.current);
        for _ in 0..2 {
            one.end_turn();
            one.end_turn();
            solo.push(one.state().players.p1.mana.current);
        }
        assert_eq!(solo, mana);
    }
}

// =============================================================================================
// R1168000: a passed card carries no link into the duel it left
// =============================================================================================

mod pass_links {
    use super::*;

    const MIND_GAMES: &str = "meditative-022";
    const FORTIFY_MIND: &str = "meditative-022-1";
    const JOURNEY_COMPLETE: &str = "meditative-096-1";
    const DO_OR_DIE: &str = "meditative-076";
    const HEROIC_POWER: &str = "core-098";

    fn free_passes() -> Option<PerTeam<DuosTeamRules>> {
        let free = DuosTeamRules {
            pass_cost: 0,
            ..DUOS_HUMAN_TEAM
        };
        Some(PerTeam::new(free, free))
    }

    fn top_up(state: &mut DuosState) {
        for seat in DUOS_SEATS {
            side_mut(state, seat).mana.current = 10;
            side_mut(state, seat).mana.max = 10;
        }
    }

    /// B1 and B2 each keep a Mind Games secret under the same id, as two duels numbering from the same
    /// start do. A1 passes B1's Fortify Mind to A2, whose guess would beat B2's choice: it has no link
    /// in duel 1 (R862), so it judges, reveals and cancels nothing there.
    #[test]
    fn r1168000_a_passed_fortify_mind_judges_no_secret_of_the_other_duel() {
        let mut state = close_round(begun_with("cross-duos-fortify", free_passes()));
        top_up(&mut state);
        let b1 = give(&mut state, B1, MIND_GAMES);
        let b2 = give(&mut state, B2, MIND_GAMES);
        let state = ok(&state, B1, play_with_modes(&b1, &["defend"]));
        let state = ok(&state, B2, play_with_modes(&b2, &["greed"]));
        let secret_in = |state: &DuosState, duel: usize| {
            state.duels[duel].secrets.clone().expect("a secret")[0].id.clone()
        };
        let b2_secret = secret_in(&state, 1);
        assert_eq!(
            secret_in(&state, 0),
            b2_secret,
            "the same secret id in both duels"
        );

        let mut state = close_round(state);
        top_up(&mut state);
        let fortify = held(&state, A1, FORTIFY_MIND).expect("A1 holds B1's Fortify Mind");
        let passed = act(&state, A1, pass(&fortify));
        assert!(passed.error.is_none(), "{:?}", passed.error);
        let moved = last_passed(&passed.events);
        let card = side(&passed.state, A2)
            .hand
            .iter()
            .find(|card| card.id == moved)
            .expect("passed");
        assert!(card.memory.is_empty(), "no link crosses: {:?}", card.memory);

        let played = act(&passed.state, A2, play_with_modes(&moved, &["attack"]));
        assert!(played.error.is_none(), "{:?}", played.error);
        let judged = played.events.iter().any(|event| {
            matches!(
                event,
                DuosEvent::Duel {
                    duel: 1,
                    event: GameEvent::Predicted { .. } | GameEvent::SecretRevealed { .. }
                }
            )
        });
        assert!(!judged, "the passed card judged a secret of duel 1");
        let kept = played.state.duels[1].secrets.as_ref();
        assert!(
            kept.is_some_and(|secrets| secrets.iter().any(|secret| secret.id == b2_secret)),
            "B2's secret stands"
        );
    }

    /// A1's Journey Complete remembers a card of A1's own sent on a journey in duel 0 (R1246). Duel 1
    /// numbers its cards from the same start, so that id is one of A2's own cards, exiled there:
    /// passed, the card remembers nothing, and A2's play returns nothing.
    #[test]
    fn r1168000_a_passed_journey_complete_returns_nothing_of_the_other_duel() {
        let mut state = begun_with("cross-duos-journey", free_passes());
        top_up(&mut state);
        let journeyed = side(&state, A1).library[0].id.clone();
        for seat in [A1, A2] {
            let side = side_mut(&mut state, seat);
            let at = side
                .library
                .iter()
                .position(|card| card.id == journeyed)
                .expect("the same id in each duel");
            let mut card = side.library.remove(at);
            card.zone = Zone::Exile {
                player: seat.player(),
            };
            side.exile.push(card);
        }
        let complete = give(&mut state, A1, JOURNEY_COMPLETE);
        let card = side_mut(&mut state, A1)
            .hand
            .iter_mut()
            .find(|card| card.id == complete)
            .expect("given");
        card.memory
            .insert("journey".to_string(), json!([journeyed.clone()]));

        let passed = act(&state, A1, pass(&complete));
        assert!(passed.error.is_none(), "{:?}", passed.error);
        let moved = last_passed(&passed.events);
        let state = ok(&passed.state, A2, play_with_modes(&moved, &[]));
        assert!(
            side(&state, A2).exile.iter().any(|card| card.id == journeyed),
            "A2's own card stays in exile: the journey was duel 0's"
        );
    }

    /// What a card is travels with it: KY's Math Equation's climbing price stays a price, and Heroic
    /// Power keeps the power it rolled (R43).
    #[test]
    fn r1168000_a_passed_card_keeps_its_return_price_and_its_rolled_power() {
        jackioh_cards::register_all();
        let mut a1 = deck();
        a1[19] = HEROIC_POWER.to_string();
        let decks = DuosDecks {
            a1,
            a2: deck(),
            b1: deck(),
            b2: deck(),
        };
        let mut state = duos_scenario("cross-duos-power", decks, free_passes());
        let power = held(&state, A1, HEROIC_POWER).expect("Quickdraw deals Heroic Power");
        let rolled = side(&state, A1)
            .hand
            .iter()
            .find(|card| card.id == power)
            .expect("held")
            .memory
            .clone();
        assert!(rolled.contains_key("power"));
        let price = give(&mut state, A1, "core-031");
        let climb = json!(2);
        let card = side_mut(&mut state, A1)
            .hand
            .iter_mut()
            .find(|card| card.id == price)
            .expect("given");
        card.memory.insert("@returnPrice".to_string(), climb.clone());

        let state = ok(&state, A1, pass(&power));
        let passed = act(&state, A1, pass(&price));
        let price_moved = last_passed(&passed.events);
        let a2 = side(&passed.state, A2);
        let power_moved = a2
            .hand
            .iter()
            .find(|card| card.def_id == HEROIC_POWER)
            .expect("passed");
        assert_eq!(power_moved.memory, rolled);
        let price_card = a2
            .hand
            .iter()
            .find(|card| card.id == price_moved)
            .expect("passed");
        assert_eq!(price_card.memory.get("@returnPrice"), Some(&climb));
    }

    /// A Radiant Do or Die watches every card of A1's hand (R1140). A pass ends each card's stay in
    /// that hand as any other way out of it does, so once A1 has passed them all, nothing is watched.
    #[test]
    fn r1168000_a_pass_ends_the_cards_stay_under_a_hand_watch() {
        let mut state = close_round(begun_with("cross-duos-watch", free_passes()));
        top_up(&mut state);
        let watcher = give(&mut state, B1, DO_OR_DIE);
        let card = side_mut(&mut state, B1)
            .hand
            .iter_mut()
            .find(|card| card.id == watcher)
            .expect("given");
        card.radiant = true;
        let state = ok(&state, B1, play_with_modes(&watcher, &[]));
        let watched = |state: &DuosState| -> Vec<String> {
            state.duels[0]
                .delayed
                .iter()
                .flat_map(|entry| entry.hand_watch.clone().unwrap_or_default())
                .collect()
        };
        let ids = watched(&state);
        assert!(!ids.is_empty(), "A1's hand is watched");
        let mut state = close_round(state);
        top_up(&mut state);
        for id in &ids {
            if side(&state, A1).hand.iter().any(|card| &card.id == id) {
                state = ok(&state, A1, pass(id));
            }
        }
        assert_eq!(watched(&state), Vec::<String>::new());
    }
}

// =============================================================================================
// R1168001: each duel's turn moves once a round
// =============================================================================================

mod round_turns {
    use super::*;

    /// Round 2 closes into A1's turn 3, which a Tommy Tempo drawn at its start ends at once, so the
    /// close runs on into B1's turn 4. A1's turn was round 3's in duel 0, already over: round 3 is
    /// team A's with A1 counted as ended, and round 4 team B's, both duels on turn 4.
    #[test]
    fn r1168001_a_turn_that_ends_as_the_close_starts_it_is_the_next_rounds_turn() {
        let mut state = close_round(quiet("cross-duos-tempo-close", &[VANILLA]));
        assert_eq!(state.round_seats, [B1, B2]);
        draws_next(&mut state, A1, TEMPO);
        let state = close_round(state);
        assert_eq!(state.duels[0].turn, 4, "A1's turn 3 ended as it began");
        assert_eq!(state.round_seats, [A1, A2], "round 3 is team A's");
        assert_eq!(state.ended, [true, false], "A1's round turn is over");
        assert_eq!(duos_seats_to_act(&state), vec![A2], "B1 waits for its round");
        let state = close_round(state);
        assert_eq!(state.round_seats, [B1, B2]);
        assert_eq!(state.ended, [false, false]);
        assert_eq!(
            state.duels[0].turn, state.duels[1].turn,
            "the duels are in step again"
        );
    }

    /// In round 1, A1's Stockpile draws a Tommy Tempo (A1's turn ends) and B1's turn, begun at once,
    /// draws another (B1's ends too): duel 0 is on turn 3 before round 1 closes. Each of those turns
    /// takes a round of its own: round 2 is B1's, already over, beside B2's, and round 3 is team A's.
    #[test]
    fn r1168001_two_turns_ended_in_one_round_each_take_a_round() {
        let mut state = quiet("cross-duos-tempo-twice", &[VANILLA]);
        rig(&mut state, A1, &[FILLER], &[TEMPO, VANILLA, VANILLA, VANILLA]);
        rig(&mut state, B1, &[VANILLA], &[TEMPO, VANILLA, VANILLA, VANILLA]);
        let state = play_def(&state, A1, FILLER);
        assert_eq!(
            state.duels[0].turn, 3,
            "A1's and B1's turns both ended by themselves"
        );
        let state = close_round(state);
        assert_eq!(state.round_seats, [B1, B2]);
        assert_eq!(state.ended, [true, false], "B1's turn 2 is over");
        let state = close_round(state);
        assert_eq!(state.round_seats, [A1, A2]);
        assert_eq!(state.duels[0].turn, 3);
        assert_eq!(state.duels[1].turn, 3);
    }
}

// =============================================================================================
// R1168002: Glitch in a co-op game
// =============================================================================================

mod glitch {
    use super::*;

    /// Team A's pool at `pool` in both duels, A1 spent on it.
    fn spend_a1(state: &mut DuosState, pool: i32) {
        state.teams[Team::A].pool = pool;
        for seat in [A1, A2] {
            let side = side_mut(state, seat);
            if let Some(life) = side.duos.as_mut() {
                life.pool = pool;
                life.spent = seat == A1;
            }
        }
        side_mut(state, A1).hero.health = pool;
    }

    fn outcome(events: &[DuosEvent]) -> Option<GlitchOutcome> {
        events.iter().find_map(|event| match event {
            DuosEvent::Duel {
                event: GameEvent::Glitched { outcome, .. },
                ..
            } => Some(*outcome),
            _ => None,
        })
    }

    /// The first seed at which A1's Glitch, played with A1 spent on a pool of 15, draws `want`: the
    /// state before and the step.
    fn glitched(want: GlitchOutcome) -> (DuosState, DuosReduceResult) {
        for n in 0..300 {
            let mut state = begun(&format!("cross-duos-glitch-{n}"));
            spend_a1(&mut state, 15);
            let id = give(&mut state, A1, GLITCH_DEF_ID);
            let body = legal_play(&state, A1, &id);
            let result = act(&state, A1, body);
            assert!(
                result.error.is_none(),
                "the Glitch was refused: {:?}",
                result.error
            );
            if outcome(&result.events) == Some(want) {
                return (state, result);
            }
        }
        panic!("no seed draws {want:?}");
    }

    /// A reset deals duel 0 again (R676), and its life totals start over: A1's own HP is whole and
    /// unspent again, on its team's pool as it stands.
    #[test]
    fn r1168002_a_reset_deals_a_spent_seat_its_own_hp_again_on_the_pool_as_it_stands() {
        let (_, reset) = glitched(GlitchOutcome::Reset);
        let a1 = side(&reset.state, A1);
        let life = a1.duos.expect("a Duos seat again");
        assert_eq!(
            (a1.hero.health, life.spent, life.pool),
            (DUOS_HUMAN_TEAM.own_health, false, 15)
        );
        assert_eq!(reset.state.teams[Team::A].pool, 15);
    }

    /// A reset in round 1 whose mulligans are answered before the round closes starts duel 0 on turn
    /// 1 again, the round's own turn number. The round still closes, and duel 0's new first turn is
    /// round 2's there.
    #[test]
    fn r1168002_a_reset_whose_new_turn_reaches_the_rounds_number_never_freezes_the_round() {
        let mut state = quiet("cross-duos-reset-r1", &[VANILLA, VANILLA]);
        state.duels[0].reset_owed = Some(true);
        let state = play_def(&state, A1, VANILLA);
        assert_eq!(state.ended, [true, false], "the round's turn in duel 0 is over");
        let state = keep_all(&state, A1);
        let state = keep_all(&state, B1);
        assert_eq!(state.duels[0].turn, 1, "duel 0's new first turn");
        let state = ok(&state, A2, end_turn());
        assert_eq!(state.round, 2, "round 1 closed");
        assert_eq!(state.round_seats, [A1, B2]);
        assert_eq!(state.ended, [false, false]);
    }

    /// The same in round 2, with the new first turn cut short by a Tommy Tempo: duel 0 reaches turn
    /// 2, round 2's own number, and the round still closes.
    #[test]
    fn r1168002_a_reset_then_a_turn_ended_as_it_began_never_freezes_the_round() {
        let mut state = close_round(quiet("cross-duos-reset-r2", &[VANILLA, VANILLA]));
        state.duels[0].reset_owed = Some(true);
        let state = play_def(&state, B1, VANILLA);
        let mut state = keep_all(&state, A1);
        let count = side(&state, A1).library.len();
        rig(&mut state, A1, &[], &vec![TEMPO; count]);
        let held_before = side(&state, A1).hand.len();
        assert_eq!(held_before, 0);
        let state = keep_all(&state, B1);
        assert_eq!(state.duels[0].turn, 2, "A1's new first turn ended as it began");
        let state = ok(&state, B2, end_turn());
        assert_eq!(state.round, 3, "round 2 closed");
    }

    /// A void ends duel 0 with no winner (R679), and with it the co-op game: a draw, `voided`.
    #[test]
    fn r1168002_a_void_ends_the_coop_game_drawn_as_voided() {
        let (_, void) = glitched(GlitchOutcome::Void);
        let result = void.state.result.expect("the co-op game ended");
        assert_eq!(result.outcome, DuosOutcome::Draw);
        assert_eq!(result.winner, None);
        assert_eq!(result.reason, DUOS_REASON_VOIDED);
    }

    /// A swap counts on duel 0 (R677), but no seat changes hands in a co-op game: A1 still plays duel
    /// 0's Player 1 for team A, its view and its actions as before.
    #[test]
    fn r1168002_a_swap_moves_no_seat() {
        let (before, swap) = glitched(GlitchOutcome::Swap);
        assert_eq!(swap.state.duels[0].seat_swaps, Some(1));
        let view = duos_view_for(&swap.state, A1);
        assert_eq!(view.own.viewer, P1);
        assert_eq!(view.team.team, Team::A);
        assert_eq!(
            side(&swap.state, A1).duos.map(|life| life.spent),
            side(&before, A1).duos.map(|life| life.spent)
        );
        let a1_moves = duos_legal_actions(&swap.state, A1);
        assert!(
            a1_moves.iter().any(|body| *body == end_turn()),
            "A1 still acts in duel 0"
        );
        assert!(
            !duos_legal_actions(&swap.state, B1)
                .iter()
                .any(|body| *body == end_turn())
        );
    }
}

// =============================================================================================
// R1168003: a judge of lethal counts own HP and the pool
// =============================================================================================

mod lethal {
    use super::*;

    const BOARD: &str = "core-011"; // a 3/3
    const CHARGER: &str = "core-045"; // (2) 4-attack Charge

    fn lethal_ids(state: &GameState) -> Vec<String> {
        rank(state, P1, &ScorerOptions { radiant: None })
            .into_iter()
            .filter(|scored| scored.priority == ScorePriority::Lethal)
            .map(|scored| scored.def.id)
            .collect()
    }

    fn scorer_board(hand: Value, duos: bool) -> Scenario {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "cross-duos-scorer",
            "p1": { "field": [BOARD], "hand": hand },
            "p2": { "health": 27 },
        }));
        if duos {
            duos_side(&mut s, P2, 7, 20, 20, false);
        }
        s
    }

    /// §10.7's scorer (Zephyrs, Zephrys Zealotism) against 27 life: a 1v1 hero at 27 health, or a Duos
    /// hero at own HP 7 with a pool of 20. Nothing is lethal against either.
    #[test]
    fn r1168003_the_scorer_judges_lethal_against_own_hp_and_the_pool() {
        let one = scorer_board(json!([]), false);
        let duos = scorer_board(json!([]), true);
        assert_eq!(effective_life(duos.state(), P2), 27);
        assert_eq!(lethal_ids(one.state()), Vec::<String>::new());
        assert_eq!(lethal_ids(duos.state()), Vec::<String>::new());
    }

    /// Pareto Optimality's judge (MD-D28) agrees: the play it calls optimal in the 1v1 game is
    /// optimal against the same life in Duos.
    #[test]
    fn r1168003_paretos_judge_calls_the_same_play_optimal() {
        let hand = json!([CHARGER, "core-019", "core-008", "core-020"]);
        let one = scorer_board(hand.clone(), false);
        let state = one.state();
        let mut base = dry_run_base(state, P1).expect("p1 can play");
        let best = state.players[P1]
            .hand
            .iter()
            .filter_map(|card| {
                score_instance(state, P1, &card.id, Some(&mut base)).map(|scored| (card.id.clone(), scored))
            })
            .max_by(|a, b| a.1.score.total_cmp(&b.1.score))
            .expect("a scored card");
        assert!(judge_play(state, P1, &best.0));
        assert!(judge_play(scorer_board(hand, true).state(), P1, &best.0));
    }

    /// The Radiant Mayor Medinamogger's Lucky (R1201) ranks a hero a 4-attack cannot destroy alike
    /// whether its 23 life is health or own HP 3 and a pool of 20.
    #[test]
    fn r1168003_the_radiant_mayors_lucky_ranks_a_hero_by_its_whole_life() {
        const MAYOR: &str = "meditative-086";
        const WALL: &str = "core-001"; // 0/7
        let board = |health: i32| -> Scenario {
            jackioh_cards::register_all();
            scenario(json!({
                "seed": "cross-duos-mayor",
                "p1": { "field": [{ "def": MAYOR, "radiant": true }, VANILLA] },
                "p2": { "field": [WALL], "health": health },
            }))
        };
        let hero_picks = |s: &Scenario| {
            let attacker = s.card(VANILLA).clone();
            (0..200)
                .filter(|n| {
                    let mut rng = Rng::new(&format!("cross-duos-mayor-{n}"), 0);
                    matches!(
                        draw_attack_target(s.state(), &mut rng, &attacker),
                        Some(DamageTarget::Hero { .. })
                    )
                })
                .count()
        };
        let one = board(23);
        let mut duos = board(30);
        duos_side(&mut duos, P2, 3, 20, 20, false);
        assert_eq!(hero_picks(&one), hero_picks(&duos));
    }

    const CHAOS_GOLEM: &str = "core-095-1"; // 10/10 Lifesteal, Divine Shield, First Strike
    const GOLIATH: &str = "classic-061"; // Trample
    const PAWN: &str = "core-096";

    /// p2's 30/30 Trample Goliath attacks p1's First Strike Lifesteal Golem (its Divine Shield gone),
    /// which heals p1 for 10 before the 20 excess lands. Spent on a pool of 15 (cap 20), p1 heals 5,
    /// not 10, so the swing empties the pool and My Pawn cancels it; in a 1v1 game at 15 health the
    /// hero ends at 5 and My Pawn rightly stays set.
    #[test]
    fn r1168003_my_pawn_counts_a_spent_heros_capped_heal_before_the_hit() {
        let swing = |duos: bool| -> Scenario {
            jackioh_cards::register_all();
            let library = [FILLER; 5];
            let mut s = scenario(json!({
                "seed": "cross-duos-pawn-cap",
                "p1": {
                    "hand": [FILLER], "library": library, "health": 15,
                    "field": [CHAOS_GOLEM],
                    "backrow": [{ "def": PAWN, "faceUp": false }],
                },
                "p2": {
                    "hand": [FILLER], "library": library,
                    "field": [{ "def": GOLIATH, "statsOverride": { "attack": 30, "health": 30 } }],
                },
                "active": "p2",
            }));
            let golem = s.card(CHAOS_GOLEM).id.clone();
            if let Some(card) = find_instance_mut(s.state_mut(), &golem) {
                card.divine_shield_spent = Some(true);
            }
            if duos {
                duos_side(&mut s, P1, 0, 15, 20, true);
                duos_side(&mut s, P2, 20, 20, 20, false);
            }
            let goliath = s.card(GOLIATH).clone();
            let target = s.card(CHAOS_GOLEM).clone();
            s.attack(&goliath, &target);
            s
        };
        let fired = |s: &Scenario| {
            s.events()
                .iter()
                .filter(|event| event.event_type() == GameEventType::TrapFired)
                .count()
        };
        let one = swing(false);
        assert_eq!((fired(&one), one.state().players.p1.hero.health), (0, 5));
        let duos = swing(true);
        assert_eq!(fired(&duos), 1, "the swing would empty the pool");
        assert!(duos.state().result.is_none());
    }
}
