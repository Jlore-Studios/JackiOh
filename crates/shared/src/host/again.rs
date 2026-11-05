//! Play again after a finished co-op match (fullsend SPEC §13.9, R2092000, R1398000), modeled on the 1v1
//! rematch (`rematch.rs`, R672), pure: both people offer, and the game is made when the pair is complete.
//! The book is the rendezvous state for two sockets, not rows, kept by the host in memory (a restart drops
//! it and both clients re-offer).
//!
//! An offer counts only within `REMATCH_OFFER_TTL_MS` of the finished match's end (R1398000, R2685000).
//! Unlike a 1v1 rematch, only one create may be in flight: an offer that lands meanwhile waits on it
//! (`AgainClaim::Wait`) instead of minting a second game, and the create it waits on is named by a
//! number the host holds its own wake-up under.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use jackioh_engine::LeanPick;
use jackioh_engine::wire::DuosSeat;

use crate::config::REMATCH_OFFER_TTL_MS;

/// R1398000: whether a finished co-op match's Play again window is open at `now`: at most
/// `REMATCH_OFFER_TTL_MS` since its end.
pub fn in_again_window(ended_at: i64, now: i64) -> bool {
    now - ended_at <= REMATCH_OFFER_TTL_MS
}

/// One finished match's offers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AgainEntry {
    /// The seats that offered, in the order they did.
    pub offers: Vec<DuosSeat>,
    /// R924000, R2031000: each seat's latest offer's "More cards from" pick.
    #[cfg_attr(feature = "ts", ts(type = "{ [seat in string]?: string | null }"))]
    pub leans: IndexMap<DuosSeat, Option<LeanPick>>,
    /// `now` of the latest offer; the sweep reads this.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
    /// The game both seats' offers made, once made — and only once made, so no offer ever reads an id for a
    /// game whose create then fails.
    pub match_id: Option<String>,
    /// The create in flight, while one offer makes the game: its number. An offer landing meanwhile waits
    /// on it, and a swept and remade entry holds none.
    pub creating: Option<u32>,
}

/// What an offer does next.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "claim", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AgainClaim {
    /// Answer now: the pair is incomplete (`answered` null), or the game is already made.
    Done { answered: Option<String> },
    /// This offer makes the game, holding the claim `creating` while it does. The host gives the claim
    /// back ([`AgainBook::release`]) however the create ends, and publishes the game ([`AgainBook::made`]).
    Make {
        creating: u32,
        #[cfg_attr(feature = "ts", ts(type = "[string | null, string | null]"))]
        leans: (Option<LeanPick>, Option<LeanPick>),
    },
    /// Another offer's create `creating` is in flight: wait until it lands or is let go, then claim again.
    Wait { creating: u32 },
}

/// What a person is told of its finished match's Play again.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AgainSeen {
    pub you_offered: bool,
    pub teammate_offered: bool,
    pub match_id: Option<String>,
}

/// R2092000's offers, by the finished match's id.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct AgainBook {
    entries: IndexMap<String, AgainEntry>,
    next_creating: u32,
}

impl AgainBook {
    /// How many finished matches hold an entry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops the entries whose latest offer is older than `REMATCH_OFFER_TTL_MS` with no live host
    /// (`live`), and answers how many went.
    pub fn sweep(&mut self, now: i64, live: impl Fn(&str) -> bool) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|match_id, entry| now - entry.at <= REMATCH_OFFER_TTL_MS || live(match_id));
        before - self.entries.len()
    }

    /// The live entry for a match, sweeping unanswered offers past `REMATCH_OFFER_TTL_MS`.
    fn live_entry(&mut self, match_id: &str, now: i64) -> Option<&mut AgainEntry> {
        let stale = {
            let entry = self.entries.get(match_id)?;
            entry.match_id.is_none() && now - entry.at > REMATCH_OFFER_TTL_MS
        };
        if stale {
            self.entries.shift_remove(match_id);
            return None;
        }
        self.entries.get_mut(match_id)
    }

    /// R1398000: an offer past the finished match's window lapses silently: it records nothing and makes no
    /// game. An entry that already made a game inside the window keeps its id, so a seat that offers or
    /// polls late still learns where to go (the answer); any other entry is dropped.
    pub fn lapse(&mut self, match_id: &str) -> Option<String> {
        match self.entries.get(match_id) {
            Some(entry) if entry.match_id.is_some() => entry.match_id.clone(),
            _ => {
                self.entries.shift_remove(match_id);
                None
            }
        }
    }

    /// Records `seat`'s offer for the finished match `match_id` and decides what happens next; the check and
    /// the claim are one step.
    pub fn claim(&mut self, match_id: &str, seat: DuosSeat, lean: Option<LeanPick>, now: i64) -> AgainClaim {
        // Sweeps a stale entry, so the offer starts a fresh one.
        self.live_entry(match_id, now);
        let entry = self
            .entries
            .entry(match_id.to_string())
            .or_insert_with(|| AgainEntry {
                offers: Vec::new(),
                leans: IndexMap::new(),
                at: now,
                match_id: None,
                creating: None,
            });
        entry.at = now;
        if !entry.offers.contains(&seat) {
            entry.offers.push(seat);
        }
        entry.leans.insert(seat, lean);
        if let Some(created) = &entry.match_id {
            return AgainClaim::Done {
                answered: Some(created.clone()),
            };
        }
        if !entry.offers.contains(&seat.teammate()) {
            return AgainClaim::Done { answered: None };
        }
        if let Some(inflight) = entry.creating {
            return AgainClaim::Wait { creating: inflight };
        }
        self.next_creating = self.next_creating.wrapping_add(1);
        let creating = self.next_creating;
        let Some(entry) = self.entries.get_mut(match_id) else {
            return AgainClaim::Done { answered: None };
        };
        entry.creating = Some(creating);
        let lean_at = |at: DuosSeat| entry.leans.get(&at).copied().flatten();
        let leans = (lean_at(DuosSeat::A1), lean_at(DuosSeat::A2));
        AgainClaim::Make { creating, leans }
    }

    /// Gives the claim `creating` back, however its create ended: only the entry that holds it.
    pub fn release(&mut self, match_id: &str, creating: u32) {
        if let Some(entry) = self.entries.get_mut(match_id)
            && entry.creating == Some(creating)
        {
            entry.creating = None;
        }
    }

    /// Publishes the created game's id; remakes the entry if it was swept meanwhile.
    pub fn made(&mut self, match_id: &str, creating: u32, id: &str, now: i64) {
        match self.entries.get_mut(match_id) {
            Some(entry) if entry.creating == Some(creating) => {
                entry.match_id = Some(id.to_string());
            }
            _ => {
                self.entries.insert(
                    match_id.to_string(),
                    AgainEntry {
                        offers: vec![DuosSeat::A1, DuosSeat::A2],
                        leans: IndexMap::new(),
                        at: now,
                        match_id: Some(id.to_string()),
                        creating: None,
                    },
                );
            }
        }
    }

    /// What `seat` is told of its match's Play again at `now`.
    pub fn seen(&mut self, match_id: &str, seat: DuosSeat, now: i64) -> AgainSeen {
        match self.live_entry(match_id, now) {
            None => AgainSeen {
                you_offered: false,
                teammate_offered: false,
                match_id: None,
            },
            Some(entry) => AgainSeen {
                you_offered: entry.offers.contains(&seat),
                teammate_offered: entry.offers.contains(&seat.teammate()),
                match_id: entry.match_id.clone(),
            },
        }
    }
}
