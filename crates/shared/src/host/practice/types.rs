//! What crosses the practice session's boundary (the host contract, `crates/shared/README.md`): the
//! setup a game is dealt from, what a save and a kept game hold, and what the page is told. Every type
//! is serde and, behind `ts`, ts-rs, so the web reads the same shapes (`apps/web/src/wire/generated/`).
//!
//! The config's JSON is the web's `PracticeStartConfig` (`apps/web/src/practice/protocol.ts`), field for
//! field, so a save or a kept game written before this module read exactly as it did.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use jackioh_engine::{
    Action, ActionBody, CombatForecast, DeckMark, Difficulty, GameResult, GameState, Handicap,
    LastBoardEntry, LeanPick, PerPlayer, PerPlayerOpt, PlayerId, PlayerView, PortraitId, ReplayRefusal,
};
use jackioh_run::{RunAction, RunFightSetup};

/// The human's deck, as the setup asked for it (§9.9). A preset's cards are the web's list (`decks.ts`),
/// which the host puts in `cards` before it hands a config over: the machine knows no preset by name.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PracticeDeckChoice {
    /// R1373, R924000: dealt from the seed, leaning on `lean_set` ("newest" when it deals, R1370, R1371).
    /// A game kept before the picker (R668, R768) carries R1373's `lean_newest` instead, so it deals the
    /// deck it always dealt.
    Random {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "string"))]
        lean_set: Option<LeanPick>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        lean_newest: Option<bool>,
    },
    /// A named preset: `cards` is its list, filled in by the host.
    Preset {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        cards: Option<Vec<String>>,
    },
    /// The 1-based deck number the setup's `saved:<n>` value names, and its cards as given.
    Saved {
        index: i32,
        cards: Vec<String>,
        #[serde(
            default,
            skip_serializing_if = "Option::is_none",
            with = "crate::absent_or_null"
        )]
        #[cfg_attr(feature = "ts", ts(optional, type = "string | null"))]
        portrait: Option<Option<String>>,
        /// The deck's cosmetics, which only the page reads.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "unknown"))]
        cosmetics: Option<Value>,
    },
}

/// SPEC §9.10, R291: a lesson's fixed decks, which the host reads from the web's lesson (`tutorial/lessons.ts`)
/// and puts in the config. The handicaps are the machine's (R290, R1705005, R2201001).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LessonDeal {
    pub human_deck: Vec<String>,
    pub ai_deck: Vec<String>,
    /// R2201001: whether every random pool leaves out Legendary and Mythic cards (lessons 1 to 4).
    #[serde(default)]
    pub pool_hold: bool,
}

/// What a practice game is dealt from (SPEC §9.9): the web's `PracticeStartConfig`, and what the server
/// issues a seed for (`crates/server/src/economy/store.rs`'s `PracticeSetup`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeConfig {
    pub seed: String,
    pub difficulty: Difficulty,
    pub human_seat: PlayerId,
    pub deck: PracticeDeckChoice,
    /// SPEC §9.10, R291: a tutorial lesson, whose decks (`lesson_deal`) and handicaps replace `deck` and
    /// `difficulty`'s; the seed and the seat are still this config's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lesson: Option<String>,
    /// The lesson's decks, put there by the host (the page never sends one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lesson_deal: Option<LessonDeal>,
    /// R417, R508: the human's last board, from the device; a lesson ignores it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_board: Option<Vec<LastBoardEntry>>,
    /// R768: both seats' portraits as the page dealt them (R642), for the replay summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub portraits: Option<PerPlayer<PortraitId>>,
    /// R923000: a sandbox game, whose dev tools only it accepts. It brings no last board and leaves none
    /// (R508). Never a lesson.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub sandbox: Option<bool>,
    /// Issue #1113, tests only: the ids the AI deck is dealt without; absent, the live shadow ban.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub ai_banned: Option<Vec<String>>,
    /// R1303000: the game is on a seed the server issued for this setup, so its end may be claimed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub earning: Option<bool>,
    /// R2882000: today's deal, the UTC day whose seed and seat it plays; the page's, kept with the game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub daily_deal: Option<String>,
    /// DR14, R2706000: a dungeon run's fight, whose decks, handicaps, marks and seed replace the free
    /// game's deal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub run: Option<RunFightSetup>,
}

impl PracticeConfig {
    /// A free game on `seed`: the human's seat, deck and tier, and nothing more.
    pub fn free(
        seed: &str,
        difficulty: Difficulty,
        human_seat: PlayerId,
        deck: PracticeDeckChoice,
    ) -> PracticeConfig {
        PracticeConfig {
            seed: seed.to_string(),
            difficulty,
            human_seat,
            deck,
            lesson: None,
            lesson_deal: None,
            last_board: None,
            portraits: None,
            sandbox: None,
            ai_banned: None,
            earning: None,
            daily_deal: None,
            run: None,
        }
    }

    pub fn is_sandbox(&self) -> bool {
        self.sandbox == Some(true)
    }

    /// Whether this is a tutorial lesson (§9.10), which keeps no save, no replay and no last board.
    pub fn is_lesson(&self) -> bool {
        self.lesson.is_some()
    }

    /// Whether this is a dungeon run's fight (DR14), a game of its own kind.
    pub fn is_run(&self) -> bool {
        self.run.is_some()
    }

    /// Whether the game is on a server seed (R1303000), so its end may be claimed.
    pub fn is_earning(&self) -> bool {
        self.earning == Some(true)
    }
}

/// How a finished game went for the human (R1843004), as the page and the claim read it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "lowercase")]
pub enum PracticeResult {
    Win,
    Loss,
    Draw,
}

impl From<jackioh_engine::SeatOutcome> for PracticeResult {
    fn from(outcome: jackioh_engine::SeatOutcome) -> Self {
        match outcome {
            jackioh_engine::SeatOutcome::Win => PracticeResult::Win,
            jackioh_engine::SeatOutcome::Loss => PracticeResult::Loss,
            jackioh_engine::SeatOutcome::Draw => PracticeResult::Draw,
        }
    }
}

/// The number and step count a kept game was kept under (R768000), which "Watch the replay" opens.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct KeptAs {
    pub game: u32,
    pub steps: u32,
}

/// Rule 7: everything the page ever gets about the game.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSnapshot {
    /// `view_for(state, human)`
    pub view: PlayerView,
    /// `legal_actions(state, human)`
    pub legal: Vec<ActionBody>,
    /// `ai_to_act(state, ai)`
    pub ai_to_act: bool,
    /// The engine's refusal of the last human action.
    pub error: Option<String>,
    /// R508: once a free game is over, the field as the human saw it, the AI's face-down cards left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_board: Option<Vec<LastBoardEntry>>,
    /// R768000: once a free game is over and kept, its number and step count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub replay: Option<KeptAs>,
    /// R2102000: set by the host when its lethal check (`PracticeOutput::Lethal`) says the human has a
    /// sure win this turn. It never says how.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub lethal: Option<bool>,
    /// R3100000, R3477000: what each of the human's legal attacks would do, in every snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub forecast: Option<CombatForecast>,
}

/// R668: the worker-only state a game in progress resumes from.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeSave {
    pub catalog: String,
    /// R1371000: the scheme `catalog` is named by; a save kept before the version retcon has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub versions: Option<u32>,
    pub config: PracticeConfig,
    pub log: Vec<Action>,
    pub ai_cursor: u32,
    pub hash: String,
}

/// R767000: the setup a kept game was dealt, as `create_game` had it, so a game played on an older
/// catalog version folds under that version without dealing it again from this build's catalog.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeReplaySetup {
    pub decks: (Vec<String>, Vec<String>),
    pub handicaps: PerPlayerOpt<Handicap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_boards: Option<(Vec<LastBoardEntry>, Vec<LastBoardEntry>)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dealt: Option<Vec<PlayerId>>,
}

/// R768: one kept practice game as the replay list shows it: never its log, seed or decks.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeReplaySummary {
    /// The number a replay request names: 1, 2, 3… on this device.
    pub game: u32,
    /// When it ended, as the host's clock read it.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub ended_at: i64,
    /// The result for the human, read from the seat it ended in (R677).
    pub result: PracticeResult,
    /// The state's turn when it ended.
    pub turns: i32,
    /// The replay's steps: step 0 and one per accepted action.
    pub steps: u32,
    pub portraits: PerPlayer<PortraitId>,
}

/// R768: a finished free game, kept on the device.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeReplay {
    pub catalog: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub versions: Option<u32>,
    pub config: PracticeConfig,
    /// R767000: absent on a game kept before setups were (it folds only on its own catalog version).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub setup: Option<PracticeReplaySetup>,
    pub log: Vec<Action>,
    pub hash: String,
    /// R677: the seat the human played when the game ended.
    pub end_seat: PlayerId,
    pub summary: PracticeReplaySummary,
}

/// R768: one kept game in the list: its summary, and why it cannot be replayed when it cannot.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeReplayListing {
    #[serde(flatten)]
    pub summary: PracticeReplaySummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub unavailable: Option<ReplayRefusal>,
}

/// R1303000: a finished game on a server seed, as the server checks it; the human's result is read from
/// the seat it ended in (R677).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeClaim {
    pub seed: String,
    pub result: PracticeResult,
    pub log: Vec<Action>,
}

/// Dev builds only: the raw game, for the e2e replay check.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeDebug {
    pub seed: String,
    pub decks: (Vec<String>, Vec<String>),
    pub handicaps: PerPlayerOpt<Handicap>,
    pub log: Vec<Action>,
    #[cfg_attr(feature = "ts", ts(type = "unknown"))]
    pub state: GameState,
    pub hash: String,
    pub difficulty: Difficulty,
    pub human_seat: PlayerId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub lesson: Option<String>,
    /// R417: the last boards `create_game` had, seat ordered, when the human brought one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub last_boards: Option<(Vec<LastBoardEntry>, Vec<LastBoardEntry>)>,
    /// R433: the seats `create_game` was told were dealt; absent when none was.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub dealt: Option<Vec<PlayerId>>,
    /// R2706000: a run fight's deck marks, which its fold must be given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub deck_marks: Option<PerPlayerOpt<Vec<DeckMark>>>,
    /// R923000: present when the game is a sandbox, which its fold must be told.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
    pub sandbox: Option<bool>,
}

/// What the store must record when the game ends (SPEC §9.5): the result for the human, and what the
/// claim and the economy read off the log. The host knows who the account is.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct PracticeRecord {
    pub outcome: GameResult,
    /// The seat the human ended in (R677).
    pub human_seat: PlayerId,
    /// The result for the human, from that seat (R1843004: the AI's `fled` is the human's draw).
    pub result: PracticeResult,
    pub turns: i32,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
    /// The human's accepted actions in order (`h<n>`), which the economy counts (R1302004, §14.9).
    pub human_actions: Vec<ActionBody>,
    /// How many of the log's actions were the AI's (`a<n>`).
    pub ai_moves: u32,
    /// R508: the board the human takes from the game, from the seat they ended in.
    pub last_board: Vec<LastBoardEntry>,
    /// R768: the final state's hash, which the game's replay is held to.
    pub final_hash: String,
}

/// The AI's decision, asked of the host (one separate step, bounded by `clock_ms`): decide a move for
/// `seat` on the machine's game and answer it as [`super::PracticeEvent::Decided`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AiAsk {
    /// The engine seat the AI plays now (R677).
    pub seat: PlayerId,
    /// The AI's own stream, `"{seed}:ai"`, and where it stands.
    pub rng_seed: String,
    pub rng_cursor: u32,
    /// The wall-clock cap for the search (`PRACTICE_AI_CLOCK_MS`), for a host with a clock.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub clock_ms: i64,
}

/// The AI's move as the host decided it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AiMove {
    /// The decision's action, or none when the search gave nothing usable (or threw).
    pub body: Option<ActionBody>,
    /// Where the AI's stream stopped; unchanged when the search gave nothing.
    pub rng_cursor: u32,
}

/// R2102000: the lethal check asked of the host, bounded by `clock_ms`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct LethalAsk {
    /// The engine seat the human plays now (R677).
    pub seat: PlayerId,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub clock_ms: i64,
}

/// A finished fight as the run reads it (DR14, R2706000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum RunReport {
    Action { action: Box<RunAction> },
    Refused { error: String },
}
