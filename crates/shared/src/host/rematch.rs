//! Rematch offers after a finished non-series match (SPEC §9.5, R672, R2685000), pure: either seat may
//! offer, normal or double-or-nothing, and a game is made only when both seats offer equal stakes. The
//! book is the rendezvous state for two sockets, not rows: one entry per finished match, kept by the
//! host in memory (a restart drops it and both clients re-offer). The host makes the game, tells the
//! store and the seats; this module decides whether an offer is taken, what it meets and who is
//! answered what.
//!
//! R2685000: an offer counts only within `REMATCH_OFFER_TTL_MS` of the finished match's `finished_at`,
//! as a co-op Play again's does (R1398000). Past it an offer lapses silently, recording nothing, and any
//! entry that has not made a game is dropped.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use jackioh_engine::{LeanPick, PlayerId};

use crate::config::REMATCH_OFFER_TTL_MS;
pub use crate::config::{STAKE_DOUBLE, STAKE_NORMAL};

/// `STAKE_NORMAL` or `STAKE_DOUBLE`, nothing else.
pub type RematchStakes = i32;

/// R1285000, R2685000: whether a finished match's rematch window is open at `now`: at most
/// `REMATCH_OFFER_TTL_MS` since its end. A finished row with no end is outside it.
pub fn in_rematch_window(finished_at: Option<i64>, now: i64) -> bool {
    finished_at.is_some_and(|finished_at| now - finished_at <= REMATCH_OFFER_TTL_MS)
}

/// R672: the stakes a request names, or `None` when it names neither a rematch nor a double-or-nothing.
pub fn stakes_of(requested: Option<f64>) -> Option<RematchStakes> {
    match requested {
        Some(stakes) if stakes == f64::from(STAKE_NORMAL) => Some(STAKE_NORMAL),
        Some(stakes) if stakes == f64::from(STAKE_DOUBLE) => Some(STAKE_DOUBLE),
        _ => None,
    }
}

/// Why an offer is refused, before it is recorded anywhere.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "snake_case")]
pub enum RematchRefusal {
    /// R672: only a ranked match can spawn a double-or-nothing.
    DoubleRequiresRanked,
    /// The match is not over.
    MatchNotFinished,
    /// A Conquest series game offers none: its continue flow owns what comes next.
    SeriesGame,
    /// R1638000: a card patch went live since the match began.
    UpdateRequired,
}

/// What the host knows of the finished match when an offer arrives.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct OfferFacts {
    pub stakes: RematchStakes,
    /// Whether the match was ranked; `None` for a row that never said.
    pub ranked: Option<bool>,
    pub finished: bool,
    /// Whether the match was a game of a Conquest series.
    pub series_game: bool,
    /// R1638000: whether the match was played on the catalog this build holds.
    pub catalog_current: bool,
}

/// The order the refusals are given in: stakes, then the match's state, then its kind, then its cards.
pub fn check_offer(facts: &OfferFacts) -> Result<(), RematchRefusal> {
    // R672: only a ranked match can spawn a double-or-nothing.
    if facts.stakes == STAKE_DOUBLE && facts.ranked != Some(true) {
        return Err(RematchRefusal::DoubleRequiresRanked);
    }
    if !facts.finished {
        return Err(RematchRefusal::MatchNotFinished);
    }
    if facts.series_game {
        return Err(RematchRefusal::SeriesGame);
    }
    // R1638000: refused before the offer is recorded, so nothing is kept and no rematch is made.
    if !facts.catalog_current {
        return Err(RematchRefusal::UpdateRequired);
    }
    Ok(())
}

/// One finished match's offers.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RematchEntry {
    /// Seat -> the stakes it offered.
    pub offers: IndexMap<PlayerId, RematchStakes>,
    /// R1372, R924000: seat -> its latest offer's "More cards from" pick.
    #[cfg_attr(feature = "ts", ts(type = "{ [seat in string]?: string | null }"))]
    pub leans: IndexMap<PlayerId, Option<LeanPick>>,
    /// `now` of the latest offer; the sweep reads this. It is not the offer window: R2685000 counts
    /// that from the finished match's `finished_at`, and an offer past it is not taken.
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub at: i64,
    /// The game equal stakes made, once made.
    pub match_id: Option<String>,
    /// Which entry this is: a swept-and-remade entry must not be mistaken for it after a failed create.
    pub generation: u32,
}

/// What an offer came to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "outcome", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OfferOutcome {
    /// R2685000: past the finished match's window. Nothing was recorded; `answered` is the game an
    /// entry already made inside the window, which a seat that offers or polls late still learns.
    Lapsed { answered: Option<String> },
    /// The offer is in and meets no equal one yet (or meets a game already made: `answered`).
    Waiting { answered: Option<String> },
    /// Both seats offered equal stakes: the host makes this game, from these offers' leans, and tells
    /// the book if it fails ([`RematchBook::release`]).
    Claimed {
        match_id: String,
        generation: u32,
        #[cfg_attr(feature = "ts", ts(type = "[string | null, string | null]"))]
        leans: (Option<LeanPick>, Option<LeanPick>),
    },
}

/// What a seat is told of its match's rematch.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RematchSeen {
    pub you_offered: Option<RematchStakes>,
    pub opponent_offer: Option<RematchStakes>,
    pub match_id: Option<String>,
}

/// R672's offers, by the finished match's id.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct RematchBook {
    entries: IndexMap<String, RematchEntry>,
    next_generation: u32,
}

impl RematchBook {
    /// How many finished matches hold an entry.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Sweeps the entries whose latest offer is older than `REMATCH_OFFER_TTL_MS` with no live host
    /// (`live`), and answers how many went.
    pub fn sweep(&mut self, now: i64, live: impl Fn(&str) -> bool) -> usize {
        let before = self.entries.len();
        self.entries
            .retain(|match_id, entry| now - entry.at <= REMATCH_OFFER_TTL_MS || live(match_id));
        before - self.entries.len()
    }

    /// The live entry for a match, sweeping one whose offers went stale. An entry that already made a
    /// game is never swept here: the id is how a late poller finds the game, until the sweep. This
    /// reads the latest offer's `at`; the window an offer is taken in is the finished match's, which
    /// [`RematchBook::offer`] checks first (R2685000).
    fn live_entry(&mut self, match_id: &str, now: i64) -> Option<&mut RematchEntry> {
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

    /// What `seat` is told of its match's rematch at `now`.
    pub fn seen(&mut self, match_id: &str, seat: PlayerId, now: i64) -> RematchSeen {
        match self.live_entry(match_id, now) {
            None => RematchSeen {
                you_offered: None,
                opponent_offer: None,
                match_id: None,
            },
            Some(entry) => RematchSeen {
                you_offered: entry.offers.get(&seat).copied(),
                opponent_offer: entry.offers.get(&seat.opponent()).copied(),
                match_id: entry.match_id.clone(),
            },
        }
    }

    /// `seat`'s offer of `stakes` (and its lean) for the finished match `match_id`, whose end was at
    /// `finished_at`, at `now`. `new_match_id` is the id the game gets if this offer meets an equal
    /// one: the host mints it, as it mints every id.
    #[allow(clippy::too_many_arguments)]
    pub fn offer(
        &mut self,
        match_id: &str,
        finished_at: Option<i64>,
        seat: PlayerId,
        stakes: RematchStakes,
        lean: Option<LeanPick>,
        now: i64,
        new_match_id: &str,
    ) -> OfferOutcome {
        // R2685000: the window is the finished match's, not an offer's. A finished row with no end is
        // outside it, as it is for its socket (R1285000). The offer lapses silently, recording nothing,
        // telling no host and making no game; an entry that already made one keeps its id, so a seat
        // that offers or polls late still learns where to go, and any other entry is dropped.
        if !in_rematch_window(finished_at, now) {
            let answered = match self.entries.get(match_id) {
                Some(entry) if entry.match_id.is_some() => entry.match_id.clone(),
                _ => {
                    self.entries.shift_remove(match_id);
                    None
                }
            };
            return OfferOutcome::Lapsed { answered };
        }
        // One new game per pair of equal offers: the check and the claim are one step.
        if self.live_entry(match_id, now).is_none() {
            self.next_generation = self.next_generation.wrapping_add(1);
            let generation = self.next_generation;
            self.entries.insert(
                match_id.to_string(),
                RematchEntry {
                    offers: IndexMap::new(),
                    leans: IndexMap::new(),
                    at: now,
                    match_id: None,
                    generation,
                },
            );
        }
        let Some(entry) = self.entries.get_mut(match_id) else {
            return OfferOutcome::Waiting { answered: None };
        };
        entry.at = now;
        entry.offers.insert(seat, stakes);
        entry.leans.insert(seat, lean);
        if entry.offers.get(&seat.opponent()) == Some(&stakes) && entry.match_id.is_none() {
            entry.match_id = Some(new_match_id.to_string());
            let lean_of = |at: PlayerId| entry.leans.get(&at).copied().flatten();
            let leans = (lean_of(PlayerId::P1), lean_of(PlayerId::P2));
            OfferOutcome::Claimed {
                match_id: new_match_id.to_string(),
                generation: entry.generation,
                leans,
            }
        } else {
            OfferOutcome::Waiting {
                answered: entry.match_id.clone(),
            }
        }
    }

    /// A failed create clears the claim, so the seats can re-offer: only the entry that made it.
    pub fn release(&mut self, match_id: &str, generation: u32) {
        if let Some(entry) = self.entries.get_mut(match_id)
            && entry.generation == generation
        {
            entry.match_id = None;
        }
    }
}

/// What a finished match tells of its seats, for the rematch that follows it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct FinishedSeats {
    /// The two accounts as the match began, by seat.
    pub players: (String, String),
    /// Their frozen decks.
    pub decks: (Vec<String>, Vec<String>),
    /// R642: their portraits, if the match froze any.
    pub portraits: Option<(String, String)>,
    /// R1834002: the slots their decks wore, if the match froze its loadouts.
    pub slots: Option<(crate::deck::DeckSlots, crate::deck::DeckSlots)>,
}

/// R1989000, R1372, R924000, R1834002: the seats of the rematch of a finished match. The seats swap — the
/// finished match's p2 takes p1 with that profile's deck, portrait, slots and lean, and its p1 takes p2 with
/// theirs. A Best-of-1 rematch (`random` false) replays the frozen decks under the slots their seats wore
/// and the portraits they had; an All Random one deals fresh decks on each seat's latest "More cards from"
/// pick and fresh portraits, and wears the accounts' own slots. `leans` is still keyed by the finished
/// match's seats, so the new p1 leans on the old p2's pick. Deals from the catalog the host registered.
pub fn next_seats(
    finished: &FinishedSeats,
    random: bool,
    seed: &str,
    leans: (Option<LeanPick>, Option<LeanPick>),
) -> (crate::deck::MatchSeat, crate::deck::MatchSeat) {
    use jackioh_ai::{AiDeckOptions, build_ai_deck};
    use jackioh_engine::{DECK_SIZE, Rng};

    let deal = |stream: &str, lean: Option<LeanPick>| {
        build_ai_deck(
            &mut Rng::new(&format!("{seed}:{stream}"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(vec![]),
                lean_set: lean.map(LeanPick::resolve),
                gated_sets: Some(vec![]),
                strength: Some(0.0),
                ..AiDeckOptions::default()
            },
        )
    };
    let dealt_portrait = |seat: &str| {
        Some(jackioh_engine::wire::pick_portrait_from_seed(&format!("{seed}:portrait:{seat}")).to_string())
    };
    let portrait_of = |at: usize| {
        finished.portraits.as_ref().map(|portraits| {
            if at == 0 {
                portraits.0.clone()
            } else {
                portraits.1.clone()
            }
        })
    };
    let slots_of = |at: usize| {
        if random {
            return None;
        }
        finished
            .slots
            .as_ref()
            .map(|slots| if at == 0 { slots.0.clone() } else { slots.1.clone() })
    };
    (
        crate::deck::MatchSeat {
            profile_id: finished.players.1.clone(),
            player: PlayerId::P1,
            deck: if random {
                deal("p1-deck", leans.1)
            } else {
                finished.decks.1.clone()
            },
            portrait: if random {
                dealt_portrait("p1")
            } else {
                portrait_of(1)
            },
            deck_slots: slots_of(1),
        },
        crate::deck::MatchSeat {
            profile_id: finished.players.0.clone(),
            player: PlayerId::P2,
            deck: if random {
                deal("p2-deck", leans.0)
            } else {
                finished.decks.0.clone()
            },
            portrait: if random {
                dealt_portrait("p2")
            } else {
                portrait_of(0)
            },
            deck_slots: slots_of(0),
        },
    )
}
