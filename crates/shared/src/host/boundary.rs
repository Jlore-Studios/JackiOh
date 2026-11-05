//! The host machine's JSON boundary: one tagged call in, one tagged answer out (the host contract,
//! `crates/shared/README.md`). `jackioh-wasm-core`'s `host(call)` parses a [`HostCall`], runs [`apply`] and
//! writes the [`HostAnswer`]; it decides nothing. A host holds the [`DuelHost`] (and the
//! [`RematchBook`]) as JSON and hands it back on every call.

use serde::{Deserialize, Serialize};

use jackioh_engine::{LeanPick, PlayerId, PlayerView};

use super::duel::{DuelHost, DuelSetup};
use super::rematch::{
    OfferFacts, OfferOutcome, RematchBook, RematchRefusal, RematchSeen, RematchStakes, check_offer,
    in_rematch_window,
};
use super::types::{DecideRequest, DuelEvent, DuelOutput};
use crate::bots::decide::BotChoice;

/// One call of the host machine.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "call", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostCall {
    /// A 1v1 host for a new or rebuilt match, and the timers it arms at once.
    NewDuel {
        setup: Box<DuelSetup>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// One event fed to a 1v1 host: the next host and its outputs.
    StepDuel {
        host: Box<DuelHost>,
        event: DuelEvent,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// `seat`'s view of the match, with its clock filled in (§10.8, R79).
    DuelView {
        host: Box<DuelHost>,
        seat: PlayerId,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R672: which accounts are here, by home seat.
    DuelPresence {
        host: Box<DuelHost>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// Whether an offer is refused before it is recorded (R672, R1638000).
    CheckOffer { facts: OfferFacts },
    /// R2685000: whether a finished match's rematch window is open.
    RematchWindow {
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        finished_at: Option<i64>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// A seat's rematch offer.
    Offer {
        book: RematchBook,
        match_id: String,
        #[cfg_attr(feature = "ts", ts(type = "number | null"))]
        finished_at: Option<i64>,
        seat: PlayerId,
        stakes: RematchStakes,
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        lean: Option<LeanPick>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
        new_match_id: String,
    },
    /// What a seat is told of its match's rematch.
    Seen {
        book: RematchBook,
        match_id: String,
        seat: PlayerId,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// A failed create clears the claim.
    Release {
        book: RematchBook,
        match_id: String,
        generation: u32,
    },
    /// Sweeps the entries older than the window with no live host.
    Sweep {
        book: RematchBook,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
        live: Vec<String>,
    },
}

/// What a [`HostCall`] answers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "answer", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HostAnswer {
    /// The host to hold from now on, and what to carry out.
    Stepped {
        host: Box<DuelHost>,
        outputs: Vec<DuelOutput>,
    },
    View {
        view: Box<PlayerView>,
    },
    Presence {
        #[cfg_attr(feature = "ts", ts(type = "{ p1: boolean, p2: boolean }"))]
        presence: jackioh_engine::PerPlayer<bool>,
    },
    Checked {
        refusal: Option<RematchRefusal>,
    },
    Window {
        open: bool,
    },
    Offered {
        book: RematchBook,
        outcome: OfferOutcome,
    },
    Seen {
        book: RematchBook,
        seen: RematchSeen,
    },
    Book {
        book: RematchBook,
    },
    Swept {
        book: RematchBook,
        removed: usize,
    },
}

/// The one function behind the boundary.
pub fn apply(call: HostCall) -> HostAnswer {
    match call {
        HostCall::NewDuel { setup, now } => {
            let (host, outputs) = DuelHost::new(*setup, now);
            HostAnswer::Stepped {
                host: Box::new(host),
                outputs,
            }
        }
        HostCall::StepDuel { host, event, now } => {
            let (host, outputs) = super::duel::step(*host, event, now);
            HostAnswer::Stepped {
                host: Box::new(host),
                outputs,
            }
        }
        HostCall::DuelView { host, seat, now } => HostAnswer::View {
            view: Box::new(host.view_for(seat, now)),
        },
        HostCall::DuelPresence { host, now } => HostAnswer::Presence {
            presence: host.presence(now),
        },
        HostCall::CheckOffer { facts } => HostAnswer::Checked {
            refusal: check_offer(&facts).err(),
        },
        HostCall::RematchWindow { finished_at, now } => HostAnswer::Window {
            open: in_rematch_window(finished_at, now),
        },
        HostCall::Offer {
            mut book,
            match_id,
            finished_at,
            seat,
            stakes,
            lean,
            now,
            new_match_id,
        } => {
            let outcome = book.offer(&match_id, finished_at, seat, stakes, lean, now, &new_match_id);
            HostAnswer::Offered { book, outcome }
        }
        HostCall::Seen {
            mut book,
            match_id,
            seat,
            now,
        } => {
            let seen = book.seen(&match_id, seat, now);
            HostAnswer::Seen { book, seen }
        }
        HostCall::Release {
            mut book,
            match_id,
            generation,
        } => {
            book.release(&match_id, generation);
            HostAnswer::Book { book }
        }
        HostCall::Sweep { mut book, now, live } => {
            let removed = book.sweep(now, |id| live.iter().any(|live| live == id));
            HostAnswer::Swept { book, removed }
        }
    }
}

/// R1452: the decision a `DuelOutput::Decide` asks for, as the host that holds the machine as JSON runs it
/// (`jackioh-wasm-core`'s `bot_decide`): the machine, which holds the game and who the bot is, and the
/// request it made. The answer is the [`BotChoice`] the host feeds back as `DuelEvent::Decided`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct DecideCall {
    pub host: Box<DuelHost>,
    pub request: DecideRequest,
}

/// Runs the decision of `call` under the catalog the caller registered; `should_stop` is the host's
/// wall-clock cap from `request.deadline_ms` (none: the search ends at the strength's budget). `None` when
/// no bot plays at the request's home.
pub fn decide(call: &DecideCall, should_stop: Option<&dyn Fn() -> bool>) -> Option<BotChoice> {
    call.host.job(&call.request).map(|job| job.run(should_stop))
}
