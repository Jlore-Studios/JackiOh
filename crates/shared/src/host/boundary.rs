//! The host machine's JSON boundary: one tagged call in, one tagged answer out (the host contract,
//! `crates/shared/README.md`). `jackioh-wasm-core`'s `host(call)` parses a [`HostCall`], runs [`apply`] and
//! writes the [`HostAnswer`]; it decides nothing. A host holds the [`DuelHost`] (and the
//! [`RematchBook`]) as JSON and hands it back on every call.

use serde::{Deserialize, Serialize};

use jackioh_engine::wire::{DuosSeat, DuosView};
use jackioh_engine::{LeanPick, PlayerId, PlayerView};

use super::again::{AgainBook, AgainClaim, AgainSeen, in_again_window};
use super::duel::{DuelHost, DuelSetup};
use super::duos::{DuosHost, DuosHostEvent, DuosHostOutput, DuosSetup, step_duos};
use super::rematch::{
    OfferFacts, OfferOutcome, RematchBook, RematchRefusal, RematchSeen, RematchStakes, check_offer,
    in_rematch_window,
};
use super::types::{DuelEvent, DuelOutput};

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
    /// A Duos co-op host for a new or rebuilt match.
    NewDuos {
        setup: Box<DuosSetup>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// One event fed to a Duos host: the next host and its outputs.
    StepDuos {
        host: Box<DuosHost>,
        event: DuosHostEvent,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// `seat`'s view of the co-op match, with its clock filled in (B23, R79).
    DuosSeatView {
        host: Box<DuosHost>,
        seat: DuosSeat,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// R1398000: whether a finished co-op match's Play again window is open.
    AgainWindow {
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        ended_at: i64,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// A person's Play again offer.
    AgainClaim {
        book: AgainBook,
        match_id: String,
        seat: DuosSeat,
        #[cfg_attr(feature = "ts", ts(type = "string | null"))]
        lean: Option<LeanPick>,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// What a person is told of its match's Play again.
    AgainSeen {
        book: AgainBook,
        match_id: String,
        seat: DuosSeat,
        #[cfg_attr(feature = "ts", ts(type = "number"))]
        now: i64,
    },
    /// The create `creating` ended: the claim is given back.
    AgainRelease {
        book: AgainBook,
        match_id: String,
        creating: u32,
    },
    /// The create `creating` made the game `id`.
    AgainMade {
        book: AgainBook,
        match_id: String,
        creating: u32,
        id: String,
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
    /// The Duos host to hold from now on, and what to carry out.
    SteppedDuos {
        host: Box<DuosHost>,
        outputs: Vec<DuosHostOutput>,
    },
    View {
        view: Box<PlayerView>,
    },
    DuosView {
        view: Box<DuosView>,
    },
    Presence {
        #[cfg_attr(feature = "ts", ts(type = "{ p1: boolean, p2: boolean }"))]
        presence: jackioh_engine::PerPlayer<bool>,
    },
    Checked {
        refusal: Option<RematchRefusal>,
    },
    AgainClaimed {
        book: AgainBook,
        claim: AgainClaim,
    },
    AgainSeen {
        book: AgainBook,
        seen: AgainSeen,
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
    AgainBook {
        book: AgainBook,
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
        HostCall::NewDuos { setup, now } => {
            let (host, outputs) = DuosHost::new(*setup, now);
            HostAnswer::SteppedDuos {
                host: Box::new(host),
                outputs,
            }
        }
        HostCall::StepDuos { host, event, now } => {
            let (host, outputs) = step_duos(*host, event, now);
            HostAnswer::SteppedDuos {
                host: Box::new(host),
                outputs,
            }
        }
        HostCall::DuosSeatView { host, seat, now } => HostAnswer::DuosView {
            view: Box::new(host.view_for(seat, now)),
        },
        HostCall::AgainWindow { ended_at, now } => HostAnswer::Window {
            open: in_again_window(ended_at, now),
        },
        HostCall::AgainClaim {
            mut book,
            match_id,
            seat,
            lean,
            now,
        } => {
            let claim = book.claim(&match_id, seat, lean, now);
            HostAnswer::AgainClaimed { book, claim }
        }
        HostCall::AgainSeen {
            mut book,
            match_id,
            seat,
            now,
        } => {
            let seen = book.seen(&match_id, seat, now);
            HostAnswer::AgainSeen { book, seen }
        }
        HostCall::AgainRelease {
            mut book,
            match_id,
            creating,
        } => {
            book.release(&match_id, creating);
            HostAnswer::AgainBook { book }
        }
        HostCall::AgainMade {
            mut book,
            match_id,
            creating,
            id,
            now,
        } => {
            book.made(&match_id, creating, &id, now);
            HostAnswer::AgainBook { book }
        }
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
