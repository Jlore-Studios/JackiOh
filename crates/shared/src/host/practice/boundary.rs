//! The practice session's JSON boundary: one tagged call in, one tagged answer out (the host contract,
//! `crates/shared/README.md`). It rides `host(call)` as [`super::super::boundary::HostCall::Practice`], so
//! no export is added to the three modules. A host holds the [`PracticeHost`] as JSON and hands it back
//! on every call; the lists and the opens take what the store read and keep nothing.

use serde::{Deserialize, Serialize};

use jackioh_engine::ReplayOpen;

use super::host::{PracticeEvent, PracticeHost, PracticeOutput, step};
use super::replays::{ReplayAnswer, ReplayVerdict, list_replays, page_replay};
use super::types::{PracticeConfig, PracticeReplay, PracticeReplayListing, PracticeSave};

/// One call of the practice session.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PracticeCall {
    /// A game for `config` on `catalog` (this build's version), dealt and begun. `kept` is the numbers of
    /// the games the host keeps on its device (R768); absent, it keeps none.
    Start {
        config: Box<PracticeConfig>,
        catalog: String,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        kept: Option<Vec<u32>>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R668: the game `save` left, resumed. `config` is the game the page remembers.
    Resume {
        config: Box<PracticeConfig>,
        save: Option<Box<PracticeSave>>,
        catalog: String,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        kept: Option<Vec<u32>>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// One event fed to the session: the next host and its outputs.
    Step {
        host: Box<PracticeHost>,
        event: PracticeEvent,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R768: the kept games as the list shows them, newest first.
    Replays {
        replays: Vec<PracticeReplay>,
        verdicts: Vec<ReplayVerdict>,
        build: String,
    },
    /// R768: steps `from`, … of a kept game; `open` is the answer of an earlier call, if the host kept it.
    Replay {
        replay: Box<PracticeReplay>,
        #[serde(default)]
        #[cfg_attr(feature = "ts", ts(optional))]
        open: Option<Box<ReplayOpen>>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        from: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        count: i64,
        build: String,
    },
}

/// What a [`PracticeCall`] answers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "reply", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PracticeAnswer {
    /// The host to hold from now on, and what to carry out.
    Stepped {
        host: Box<PracticeHost>,
        outputs: Vec<PracticeOutput>,
    },
    /// There is no game, and why: the engine's reason for a deck or a setup it refuses, or a save that
    /// does not resume.
    Refused {
        message: String,
    },
    Listed {
        replays: Vec<PracticeReplayListing>,
    },
    Replayed {
        answer: Box<ReplayAnswer>,
    },
}

/// The one function behind the boundary.
pub fn apply(call: PracticeCall) -> PracticeAnswer {
    let stepped = |made: Result<(PracticeHost, Vec<PracticeOutput>), String>| match made {
        Ok((host, outputs)) => PracticeAnswer::Stepped {
            host: Box::new(host),
            outputs,
        },
        Err(message) => PracticeAnswer::Refused { message },
    };
    match call {
        PracticeCall::Start {
            config,
            catalog,
            kept,
            now,
        } => stepped(PracticeHost::start(&config, &catalog, kept, now)),
        PracticeCall::Resume {
            config,
            save,
            catalog,
            kept,
            now,
        } => stepped(PracticeHost::resume(
            &config,
            save.as_deref(),
            &catalog,
            kept,
            now,
        )),
        PracticeCall::Step { host, event, now } => {
            let (host, outputs) = step(*host, event, now);
            PracticeAnswer::Stepped {
                host: Box::new(host),
                outputs,
            }
        }
        PracticeCall::Replays {
            replays,
            verdicts,
            build,
        } => PracticeAnswer::Listed {
            replays: list_replays(&replays, &verdicts, &build),
        },
        PracticeCall::Replay {
            replay,
            open,
            from,
            count,
            build,
        } => PracticeAnswer::Replayed {
            answer: Box::new(page_replay(&replay, open.map(|open| *open), from, count, &build)),
        },
    }
}
