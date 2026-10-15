//! The Duos state (#936 S4) and `duos_create`: two 1v1 games, each team's rules and pool, and the
//! round's bookkeeping. Everything here is JSON, as a `GameState` is, so a Duos game survives a JSON
//! round trip and folds exactly (§9.3).

use serde::{Deserialize, Serialize};

use crate::config::{DUOS_HUMAN_TEAM, Handicap, SETUP_TURN};
use crate::state::{CreateGameOptions, DuosLife, EngineError, GameState, PlayerState, create_game};
use crate::wire::{
    DUOS_DUELS, DuosCreateOptions, DuosEvent, DuosResult, DuosSeat, DuosTeamRules, PLAYER_IDS, PerPlayerOpt,
    PerTeam, PlayerId,
};

/// A Duos game (S4).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosState {
    pub seed: String,
    /// Exactly two: duel 0 is A1 (`p1`) against B1 (`p2`), duel 1 is A2 (`p1`) against B2 (`p2`).
    pub duels: Vec<GameState>,
    pub teams: PerTeam<DuosTeam>,
    /// The 1-based team-round counter; 0 until all four mulligans have resolved (B2).
    pub round: i32,
    /// [duel 0, duel 1]: whose turn the round began with — the seat that may act there this round (B4).
    pub round_seats: Vec<DuosSeat>,
    /// [duel 0, duel 1]: each duel's `turn` when the round began, so a turn that moved without the
    /// round closing is seen to have ended by itself (B7).
    pub round_turns: Vec<i32>,
    /// [duel 0, duel 1]: the round seat has ended its turn this round (B5).
    pub ended: Vec<bool>,
    /// [duel 0, duel 1]: End turn has reached the duel itself — at the round's close, or by the turn
    /// ending on its own (B5, B7).
    pub end_applied: Vec<bool>,
    /// [duel 0, duel 1]: the passes the round seat has made this round (B19, B22).
    pub passes: Vec<i32>,
    /// Each team's open concede offer (B11): the seat that conceded first, waiting for its teammate.
    /// One per team, so one team's concede never touches the other's offer.
    pub concede_offer: PerTeam<Option<DuosSeat>>,
    pub result: Option<DuosResult>,
}

/// One team: its rules and its shared pool (B12), the pool every duel's copy is kept equal to (B15).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosTeam {
    pub rules: DuosTeamRules,
    pub pool: i32,
}

/// What a Duos step answers, as `ReduceResult` does a 1v1 one: `error` is `Some` exactly when the
/// action was refused, and then `state` is the input, unchanged, and `events` is empty. On the wire
/// the error is its message, as a 1v1 refusal is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosReduceResult {
    pub state: DuosState,
    pub events: Vec<DuosEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "refusal_text")]
    pub error: Option<EngineError>,
}

impl DuosReduceResult {
    /// A refusal: the input state, no events, the reason.
    pub(super) fn refused(state: &DuosState, error: EngineError) -> DuosReduceResult {
        DuosReduceResult {
            state: state.clone(),
            events: Vec::new(),
            error: Some(error),
        }
    }
}

/// `DuosReduceResult.error` as the message alone.
mod refusal_text {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::state::EngineError;

    pub fn serialize<S: Serializer>(error: &Option<EngineError>, serializer: S) -> Result<S::Ok, S::Error> {
        match error {
            Some(error) => serializer.serialize_some(&error.message),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<EngineError>, D::Error> {
        Ok(Option::<String>::deserialize(deserializer)?.map(EngineError::new))
    }
}

/// S4: duel `duel`'s seed, so the two duels deal and draw apart.
pub fn duos_duel_seed(seed: &str, duel: usize) -> String {
    format!("{seed}:duel{duel}")
}

/// A Duos game at setup (B1, B12): two `create_game`s with the seats' decks and handicaps (each as
/// given, B30), each player's R82 automatic turn end off (the wrapper ends turns, B9), each hero at
/// its team's own HP and each seat holding its team's pool. `duos_begin` deals.
///
/// A Glitch reset (R676) deals a duel again without any of this; the wrapper puts it back on the
/// next step (`reduce::sync_pools`, `duos_seat`). Panics as `create_game` does on a deck or handicap
/// it refuses.
pub fn duos_create(options: &DuosCreateOptions) -> DuosState {
    let rules = options
        .teams
        .clone()
        .unwrap_or_else(|| PerTeam::new(DUOS_HUMAN_TEAM, DUOS_HUMAN_TEAM));
    let duels: Vec<GameState> = (0..DUOS_DUELS)
        .map(|duel| create_duel(options, &rules, duel))
        .collect();
    DuosState {
        seed: options.seed.clone(),
        duels,
        teams: PerTeam::new(
            DuosTeam {
                rules: rules.a,
                pool: rules.a.pool_health,
            },
            DuosTeam {
                rules: rules.b,
                pool: rules.b.pool_health,
            },
        ),
        round: 0,
        round_seats: (0..DUOS_DUELS)
            .map(|duel| DuosSeat::of(duel, PlayerId::P1))
            .collect(),
        round_turns: vec![SETUP_TURN; DUOS_DUELS],
        ended: vec![false; DUOS_DUELS],
        end_applied: vec![false; DUOS_DUELS],
        passes: vec![0; DUOS_DUELS],
        concede_offer: PerTeam::new(None, None),
        result: None,
    }
}

/// One duel at setup: team A's seat as `p1`, team B's as `p2`.
fn create_duel(options: &DuosCreateOptions, rules: &PerTeam<DuosTeamRules>, duel: usize) -> GameState {
    let seats = PLAYER_IDS.map(|player| DuosSeat::of(duel, player));
    let mut handicaps: PerPlayerOpt<Handicap> = PerPlayerOpt::default();
    for seat in seats {
        *handicaps.slot(seat.player()) = options
            .handicaps
            .as_ref()
            .and_then(|handicaps| handicaps.get(seat))
            .copied();
    }
    let dealt: Option<Vec<PlayerId>> = options.dealt.as_ref().and_then(|dealt| {
        let here: Vec<PlayerId> = dealt
            .iter()
            .filter(|seat| seat.duel() == duel)
            .map(|seat| seat.player())
            .collect();
        (!here.is_empty()).then_some(here)
    });
    let mut game = create_game(&CreateGameOptions {
        seed: duos_duel_seed(&options.seed, duel),
        decks: (
            options.decks.of(seats[0]).clone(),
            options.decks.of(seats[1]).clone(),
        ),
        catalog: options.catalog.clone(),
        handicaps: Some(handicaps),
        dealt,
        last_boards: None,
        glitch_boards: None,
    });
    for seat in seats {
        let team = rules[seat.team()];
        duos_seat(&mut game.players[seat.player()], &team, team.pool_health);
    }
    game
}

/// B9, B12: what makes a duel's side a Duos seat — R82's automatic turn end off, the hero at its
/// team's own HP, unspent, and the team's pool, `pool` of `rules.pool_health`.
pub(super) fn duos_seat(side: &mut PlayerState, rules: &DuosTeamRules, pool: i32) {
    side.auto_end_turn = Some(false);
    side.hero.health = rules.own_health;
    side.duos = Some(DuosLife {
        pool,
        pool_cap: rules.pool_health,
        spent: false,
    });
}
