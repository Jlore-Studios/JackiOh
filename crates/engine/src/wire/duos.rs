//! Duos co-op's wire types (#936, fullsend SPEC S2): two ordinary 1v1 games ("duels") side by side,
//! A1 vs B1 and A2 vs B2, whose teams share a life pool, pass cards to each other and end their turns
//! together. Team A is `p1` in both duels (goes first, draws 3) and team B is `p2` (draws 4, takes The
//! Coin), so every card still sees a 1v1 game.
//!
//! What a seat sends (`DuosAction`), what it sees (`DuosView`) and what a step reports (`DuosEvent`).
//! The state and the reducer that use them are `crate::duos`; the life rules are `crate::duos_life`.

use serde::{Deserialize, Serialize};

use crate::config::Handicap;
use crate::wire::actions::ActionBody;
use crate::wire::catalog_types::{CardDefs, PlayerId};
use crate::wire::events::GameEvent;
use crate::wire::string_union;
use crate::wire::view::PlayerView;

string_union! {
    /// A Duos team: the humans' (`a`, `p1` in both duels) or the AI's (`b`, `p2`).
    pub enum Team {
        A = "a",
        B = "b",
    }
}

string_union! {
    /// One of a Duos game's four seats: a team and the duel it plays in.
    pub enum DuosSeat {
        A1 = "a1",
        A2 = "a2",
        B1 = "b1",
        B2 = "b2",
    }
}

/// Every seat, team A's first, each team in duel order.
pub const DUOS_SEATS: [DuosSeat; 4] = [DuosSeat::A1, DuosSeat::A2, DuosSeat::B1, DuosSeat::B2];

/// How many duels a Duos game holds.
pub const DUOS_DUELS: usize = 2;

/// `DuosResult.reason` (S2): a team's pool reached 0 (B16).
pub const DUOS_REASON_POOL_EMPTIED: &str = "poolEmptied";
/// `DuosResult.reason`: both seats of a team conceded (B11).
pub const DUOS_REASON_CONCEDE: &str = "concede";
/// `DuosResult.reason`: a duel reached the R2 turn cap (B16).
pub const DUOS_REASON_TURN_CAP: &str = "turnCap";
/// `DuosResult.reason`: a duel ended by its own rules otherwise — an alternative win, a hero dead with
/// its pool still standing (B16).
pub const DUOS_REASON_DUEL_RESULT: &str = "duelResult";
/// `DuosResult.reason`: the server's wall-clock ceiling (R79) ended the game as a draw.
pub const DUOS_REASON_CEILING: &str = "ceiling";

impl Team {
    /// The team's `PlayerId` in both duels: team A is `p1`, team B is `p2`.
    pub fn player(self) -> PlayerId {
        match self {
            Team::A => PlayerId::P1,
            Team::B => PlayerId::P2,
        }
    }

    /// The team across the table.
    pub fn other(self) -> Team {
        match self {
            Team::A => Team::B,
            Team::B => Team::A,
        }
    }

    /// The team whose seats are `player` in their duels.
    pub fn of(player: PlayerId) -> Team {
        match player {
            PlayerId::P1 => Team::A,
            PlayerId::P2 => Team::B,
        }
    }

    /// The team's two seats, in duel order.
    pub fn seats(self) -> [DuosSeat; DUOS_DUELS] {
        match self {
            Team::A => [DuosSeat::A1, DuosSeat::A2],
            Team::B => [DuosSeat::B1, DuosSeat::B2],
        }
    }
}

impl DuosSeat {
    /// A1, A2 → A; B1, B2 → B.
    pub fn team(self) -> Team {
        match self {
            DuosSeat::A1 | DuosSeat::A2 => Team::A,
            DuosSeat::B1 | DuosSeat::B2 => Team::B,
        }
    }

    /// A1, B1 → duel 0; A2, B2 → duel 1.
    pub fn duel(self) -> usize {
        match self {
            DuosSeat::A1 | DuosSeat::B1 => 0,
            DuosSeat::A2 | DuosSeat::B2 => 1,
        }
    }

    /// The seat's player in its duel: team A is `p1`, team B is `p2`.
    pub fn player(self) -> PlayerId {
        self.team().player()
    }

    /// A1 ↔ A2, B1 ↔ B2: the seat across the portal, in the other duel.
    pub fn teammate(self) -> DuosSeat {
        match self {
            DuosSeat::A1 => DuosSeat::A2,
            DuosSeat::A2 => DuosSeat::A1,
            DuosSeat::B1 => DuosSeat::B2,
            DuosSeat::B2 => DuosSeat::B1,
        }
    }

    /// A1 ↔ B1, A2 ↔ B2: the seat across the board, in the same duel.
    pub fn across(self) -> DuosSeat {
        match self {
            DuosSeat::A1 => DuosSeat::B1,
            DuosSeat::B1 => DuosSeat::A1,
            DuosSeat::A2 => DuosSeat::B2,
            DuosSeat::B2 => DuosSeat::A2,
        }
    }

    /// The seat playing `player` in duel `duel` (0 or 1; anything past 0 is duel 1).
    pub fn of(duel: usize, player: PlayerId) -> DuosSeat {
        match (duel, player) {
            (0, PlayerId::P1) => DuosSeat::A1,
            (0, PlayerId::P2) => DuosSeat::B1,
            (_, PlayerId::P1) => DuosSeat::A2,
            (_, PlayerId::P2) => DuosSeat::B2,
        }
    }
}

/// `Record<Team, T>`: one value per team, serialised `{ "a": …, "b": … }`, as `PerPlayer` is per seat.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
pub struct PerTeam<T> {
    pub a: T,
    pub b: T,
}

impl<T> PerTeam<T> {
    pub fn new(a: T, b: T) -> PerTeam<T> {
        PerTeam { a, b }
    }
}

impl<T> std::ops::Index<Team> for PerTeam<T> {
    type Output = T;

    fn index(&self, team: Team) -> &T {
        match team {
            Team::A => &self.a,
            Team::B => &self.b,
        }
    }
}

impl<T> std::ops::IndexMut<Team> for PerTeam<T> {
    fn index_mut(&mut self, team: Team) -> &mut T {
        match team {
            Team::A => &mut self.a,
            Team::B => &mut self.b,
        }
    }
}

/// A team's Duos rules (S1, B12, B19): each seat's own HP, the shared pool, and what a pass costs and
/// how many a seat may make in a round.
#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Debug)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosTeamRules {
    pub own_health: i32,
    pub pool_health: i32,
    pub pass_cost: i32,
    pub pass_cap: i32,
}

/// The four decks, one per seat.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosDecks {
    pub a1: Vec<String>,
    pub a2: Vec<String>,
    pub b1: Vec<String>,
    pub b2: Vec<String>,
}

impl DuosDecks {
    /// The deck `seat` plays.
    pub fn of(&self, seat: DuosSeat) -> &Vec<String> {
        match seat {
            DuosSeat::A1 => &self.a1,
            DuosSeat::A2 => &self.a2,
            DuosSeat::B1 => &self.b1,
            DuosSeat::B2 => &self.b2,
        }
    }
}

/// §9.9's per-seat handicaps (R180); an omitted seat plays with HUMAN_HANDICAP.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosHandicaps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub a1: Option<Handicap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub a2: Option<Handicap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub b1: Option<Handicap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub b2: Option<Handicap>,
}

impl DuosHandicaps {
    /// The handicap `seat` was given, if any.
    pub fn get(&self, seat: DuosSeat) -> Option<&Handicap> {
        match seat {
            DuosSeat::A1 => self.a1.as_ref(),
            DuosSeat::A2 => self.a2.as_ref(),
            DuosSeat::B1 => self.b1.as_ref(),
            DuosSeat::B2 => self.b2.as_ref(),
        }
    }
}

/// `duos_create`'s options: the setup a fold needs again, as `CreateGameOptions` is a 1v1 game's.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosCreateOptions {
    pub seed: String,
    pub decks: DuosDecks,
    /// As `CreateGameOptions.catalog`: the catalog both duels deal from, the registered one when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub catalog: Option<CardDefs>,
    /// Each team's rules; absent, both teams play DUOS_HUMAN_TEAM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub teams: Option<PerTeam<DuosTeamRules>>,
    /// §9.9's per-seat handicaps (R180); absent, every seat plays HUMAN_HANDICAP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub handicaps: Option<DuosHandicaps>,
    /// As `CreateGameOptions.dealt` (R433): the seats whose deck was dealt rather than built.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dealt: Option<Vec<DuosSeat>>,
}

/// What a seat sends: a 1v1 action in its own duel, a pass to its teammate, or a concede (B11).
/// JSON: `{"kind":"duel","body":{…ActionBody…}}` | `{"kind":"pass","instanceId":"…"}` |
/// `{"kind":"concede"}`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosActionBody {
    Duel { body: ActionBody },
    Pass { instance_id: String },
    Concede,
}

/// One action of a Duos log: who sent it, its nonce, and what it is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosAction {
    pub seat: DuosSeat,
    pub nonce: String,
    pub body: DuosActionBody,
}

string_union! {
    /// How a Duos game ended: a team won, or nobody did.
    pub enum DuosOutcome {
        Win = "win",
        Draw = "draw",
    }
}

/// How a Duos game ended (B11, B16).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosResult {
    pub outcome: DuosOutcome,
    /// The winning team: `Some` exactly when `outcome` is `win`.
    pub winner: Option<Team>,
    /// `poolEmptied` | `concede` | `turnCap` | `duelResult` | `ceiling` (the `DUOS_REASON_*` constants).
    pub reason: String,
}

/// The life one side shows (B23), public to both teams.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosSideLife {
    /// The seat's own HP, 0 once it is spent (B13).
    pub own: i32,
    pub spent: bool,
}

/// One team as a seat sees it (B22, B23): its pool and both seats' own HP for either team, and for
/// its own team alone who has ended this round and how many passes each seat has left.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosTeamView {
    pub team: Team,
    pub pool: i32,
    pub pool_cap: i32,
    /// [seat 1, seat 2] of that team, duel order.
    pub sides: Vec<DuosSideLife>,
    /// [seat 1, seat 2]: has ended its turn this round (own team only; the enemy's are all false).
    pub ended: Vec<bool>,
    /// [seat 1, seat 2]: passes left this round (own team only; the enemy's are zeros).
    pub passes_left: Vec<i32>,
    pub pass_cost: i32,
}

/// Everything one seat may see (B23): its own duel as a 1v1 seat, its teammate's duel from the
/// teammate's seat, both teams' life, the round, its own team's concede offer and the result.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DuosView {
    pub seat: DuosSeat,
    /// `view_for(duels[seat.duel()], seat.player())`.
    pub own: PlayerView,
    /// `view_for(duels[teammate.duel()], teammate.player())`: the teammate's own view.
    pub teammate: PlayerView,
    pub team: DuosTeamView,
    pub enemy: DuosTeamView,
    /// The 1-based team-round counter, 0 while the mulligans are open (B2).
    pub round: i32,
    /// The own team's open concede offer (B11).
    pub concede_offered_by: Option<DuosSeat>,
    pub result: Option<DuosResult>,
}

/// What one Duos step reports, in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosEvent {
    /// An event of one duel's own reduction (or of the pass's mana payment), unredacted: a server
    /// sends a seat its views, never these raw.
    Duel { duel: u8, event: GameEvent },
    /// B20: a card went from `from`'s hand to the right end of `to`'s, under a new instance id.
    Passed {
        from: DuosSeat,
        to: DuosSeat,
        instance_id: String,
        new_instance_id: String,
    },
    /// B2, B5: a team-round began.
    RoundStarted { round: i32 },
    /// B11, B16: the game ended.
    Ended { result: DuosResult },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seats_know_their_team_duel_player_and_neighbours() {
        for seat in DUOS_SEATS {
            assert_eq!(DuosSeat::of(seat.duel(), seat.player()), seat);
            assert_eq!(seat.teammate().team(), seat.team());
            assert_ne!(seat.teammate().duel(), seat.duel());
            assert_eq!(seat.across().duel(), seat.duel());
            assert_ne!(seat.across().team(), seat.team());
            assert_eq!(seat.teammate().teammate(), seat);
            assert_eq!(seat.across().across(), seat);
            assert!(seat.team().seats().contains(&seat));
        }
        assert_eq!(DuosSeat::A1.player(), PlayerId::P1);
        assert_eq!(DuosSeat::B2.player(), PlayerId::P2);
        assert_eq!(DuosSeat::B2.duel(), 1);
    }

    #[test]
    fn actions_and_events_serialise_as_s2_writes_them() {
        let pass = DuosActionBody::Pass {
            instance_id: "c12".into(),
        };
        assert_eq!(
            serde_json::to_value(&pass).unwrap(),
            serde_json::json!({ "kind": "pass", "instanceId": "c12" })
        );
        let duel = DuosActionBody::Duel {
            body: ActionBody::EndTurn,
        };
        assert_eq!(
            serde_json::to_value(&duel).unwrap(),
            serde_json::json!({ "kind": "duel", "body": { "type": "endTurn" } })
        );
        assert_eq!(
            serde_json::to_value(DuosActionBody::Concede).unwrap(),
            serde_json::json!({ "kind": "concede" })
        );
        let started = DuosEvent::RoundStarted { round: 2 };
        assert_eq!(
            serde_json::to_value(&started).unwrap(),
            serde_json::json!({ "type": "roundStarted", "round": 2 })
        );
        let passed = DuosEvent::Passed {
            from: DuosSeat::A1,
            to: DuosSeat::A2,
            instance_id: "c1".into(),
            new_instance_id: "c90".into(),
        };
        assert_eq!(
            serde_json::to_value(&passed).unwrap(),
            serde_json::json!({
                "type": "passed", "from": "a1", "to": "a2", "instanceId": "c1", "newInstanceId": "c90"
            })
        );
        let rules = DuosTeamRules {
            own_health: 20,
            pool_health: 20,
            pass_cost: 1,
            pass_cap: 10,
        };
        assert_eq!(
            serde_json::to_value(rules).unwrap(),
            serde_json::json!({ "ownHealth": 20, "poolHealth": 20, "passCost": 1, "passCap": 10 })
        );
    }
}
