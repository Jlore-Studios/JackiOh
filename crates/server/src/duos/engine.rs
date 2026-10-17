//! The one seam between the co-op server and the engine's Duos wrapper (fullsend SPEC S4), as
//! `actor/engine.rs` is the 1v1's: `jackioh_cards::register_all()` before anything reads the
//! catalog, the engine's functions under the names the actor reads, and the decks a co-op match is
//! dealt. Nothing here decides a rule (CLAUDE.md rule 7): the wrapper owns turns, passes, the pool
//! and the result, and the server only folds its log and relays its views.
//!
//! Every function that reads the catalog runs under the match's `Pinned` catalog (R767000): a co-op
//! match records the catalog version it began on (`DuosMatchOptions::catalog_version`), and its
//! play, its rebuilds and its replay all read that version's catalog, the build's own or a shipped
//! one this build carries, so a patch deployed under a match never changes what its log folds to.
//!
//! The decks: a person's saved deck as frozen at the room, or All Random's deal (R258) from the match
//! seed and their seat, leaning on the newest set when they asked (R1372); an AI seat's deck from the
//! AI's own random deck-builder at its tier's handicap, as practice deals the AI (§9.9: shadow ban
//! and soft gate on, the handicap's deck size and mana cap), on `{seed}:{seat}-deck`.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::config::{AI_DIFFICULTY, DUOS_HUMAN_TEAM, Difficulty, HUMAN_HANDICAP, duos_ai_team};
use jackioh_engine::duos::{
    DuosReduceResult, DuosReplayInput, DuosReplayResult, DuosState, duos_begin, duos_create, duos_fold,
    duos_hash, duos_legal_actions, duos_reduce, duos_seats_to_act, duos_view_for,
};
use jackioh_engine::wire::{
    DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks, DuosHandicaps, DuosResult, DuosSeat, DuosView,
    PerTeam,
};
use jackioh_engine::{Handicap, ReplayRefusal, Rng};

pub use crate::actor::engine::Pinned;
use crate::actor::engine::{deal_random_deck, lean_of};
use crate::duos::store::{DuosChoice, DuosMatch, DuosMatchOptions, SeatDecks, SeatHandicaps, TeamRules};

fn registered() {
    jackioh_cards::register_all();
}

/// S4: a begun co-op game (four mulligans open), or the engine's refusal of the setup. Panics, as
/// `duos_create` does, on a deck the catalog refuses; the caller catches it.
pub fn create_and_begin(pinned: Pinned, options: &DuosCreateOptions) -> Result<DuosState, String> {
    registered();
    let begun = pinned.run(|| duos_begin(&duos_create(options)));
    match begun.error {
        Some(error) => Err(error.to_string()),
        None => Ok(begun.state),
    }
}

pub fn reduce(pinned: Pinned, state: &DuosState, action: &DuosAction) -> DuosReduceResult {
    registered();
    pinned.run(|| duos_reduce(state, action))
}

/// S4: what `seat` may send now, the `legal` beside its view.
pub fn legal(pinned: Pinned, state: &DuosState, seat: DuosSeat) -> Vec<DuosActionBody> {
    registered();
    pinned.run(|| duos_legal_actions(state, seat))
}

/// S4: the seats whose legal actions hold more than a concession.
pub fn seats_to_act(pinned: Pinned, state: &DuosState) -> Vec<DuosSeat> {
    registered();
    pinned.run(|| duos_seats_to_act(state))
}

/// S2, B23: the only window a seat gets onto the game.
pub fn view(pinned: Pinned, state: &DuosState, seat: DuosSeat) -> DuosView {
    registered();
    pinned.run(|| duos_view_for(state, seat))
}

/// B24, B40: a match rebuilt from its seed, options and log.
pub fn fold(pinned: Pinned, input: &DuosReplayInput) -> DuosReplayResult {
    registered();
    pinned.run(|| duos_fold(input))
}

pub fn hash(state: &DuosState) -> String {
    duos_hash(state)
}

/// The engine's options for a stored match: its seed and what it was created with (no catalog
/// override: the registered one, as every server game).
pub fn create_options(row: &DuosMatch) -> DuosCreateOptions {
    let options = &row.options;
    DuosCreateOptions {
        seed: row.seed.clone(),
        decks: DuosDecks {
            a1: options.decks.a1.clone(),
            a2: options.decks.a2.clone(),
            b1: options.decks.b1.clone(),
            b2: options.decks.b2.clone(),
        },
        catalog: None,
        teams: Some(PerTeam {
            a: options.teams.a,
            b: options.teams.b,
        }),
        handicaps: Some(DuosHandicaps {
            a1: options.handicaps.a1,
            a2: options.handicaps.a2,
            b1: options.handicaps.b1,
            b2: options.handicaps.b2,
        }),
        dealt: if options.dealt.is_empty() {
            None
        } else {
            Some(options.dealt.clone())
        },
    }
}

/// A person's deck: their frozen saved deck, or All Random's deal for their seat (R258, R1372).
fn person_deck(choice: &DuosChoice, seed: &str, seat: DuosSeat) -> Vec<String> {
    match choice {
        DuosChoice::Deck { deck, .. } => deck.cards.clone(),
        DuosChoice::Random { lean } => deal_random_deck(&format!("{seed}:{seat}-deck"), lean_of(*lean)),
    }
}

/// S6: an AI seat's deck, as practice deals the AI its own (`buildAiDeck(rng, handicap.deckSize,
/// { manaCap })`): the AI's shadow ban and soft gate, its tier's deck size and mana cap.
fn ai_deck(seed: &str, seat: DuosSeat, handicap: &Handicap) -> Vec<String> {
    registered();
    build_ai_deck(
        &mut Rng::new(&format!("{seed}:{seat}-deck"), 0),
        handicap.deck_size,
        &AiDeckOptions {
            mana_cap: Some(handicap.mana_cap),
            ..AiDeckOptions::default()
        },
    )
}

/// S6, B30: everything a co-op match between the room's two choices is created with. The people are
/// team A under `DUOS_HUMAN_TEAM` with no handicap; the AI is team B under `duos_ai_team(difficulty)`
/// with `AI_DIFFICULTY[difficulty]` on each of its seats. A person's random deck is dealt (R433), an
/// AI's never (practice's rule: R185's redaction would hide the AI's own library from itself). The
/// match is pinned to `catalog_version`, the catalog it is dealt from (R767000); `pinned` is it.
pub fn match_options(
    pinned: Pinned,
    catalog_version: &str,
    seed: &str,
    difficulty: Difficulty,
    host: &DuosChoice,
    guest: &DuosChoice,
) -> DuosMatchOptions {
    pinned.run(|| dealt_options(catalog_version, seed, difficulty, host, guest))
}

fn dealt_options(
    catalog_version: &str,
    seed: &str,
    difficulty: Difficulty,
    host: &DuosChoice,
    guest: &DuosChoice,
) -> DuosMatchOptions {
    let handicap = AI_DIFFICULTY[difficulty];
    // Easy is a person's resources exactly, and the engine stores none for it.
    let ai_handicap = (handicap != HUMAN_HANDICAP).then_some(handicap);
    let dealt = [(DuosSeat::A1, host), (DuosSeat::A2, guest)]
        .into_iter()
        .filter(|(_, choice)| matches!(choice, DuosChoice::Random { .. }))
        .map(|(seat, _)| seat)
        .collect();
    DuosMatchOptions {
        decks: SeatDecks {
            a1: person_deck(host, seed, DuosSeat::A1),
            a2: person_deck(guest, seed, DuosSeat::A2),
            b1: ai_deck(seed, DuosSeat::B1, &handicap),
            b2: ai_deck(seed, DuosSeat::B2, &handicap),
        },
        dealt,
        teams: TeamRules {
            a: DUOS_HUMAN_TEAM,
            b: duos_ai_team(difficulty),
        },
        handicaps: SeatHandicaps {
            a1: None,
            a2: None,
            b1: ai_handicap,
            b2: ai_handicap,
        },
        catalog_version: catalog_version.to_string(),
    }
}

/// B41: both people's steps of a match, each seat's view as the game began (all four mulligans open)
/// and after each logged action, exactly what that seat's live view showed.
pub struct DuosSteps {
    pub a1: Vec<DuosView>,
    pub a2: Vec<DuosView>,
    /// Where the game ended up, for the caller to hold to the result the match recorded.
    pub result: Option<DuosResult>,
}

impl DuosSteps {
    /// A person's own steps (team B's seats are the AI's and have none).
    pub fn of(&self, seat: DuosSeat) -> &[DuosView] {
        match seat {
            DuosSeat::A2 => &self.a2,
            _ => &self.a1,
        }
    }
}

/// B41: the match folded once under its catalog, both people's views after every step. A log the
/// engine no longer folds to its end, its setup or any action refused, is `rules_changed` (B24, R768):
/// the replay is refused, never cut short. Panics, as `duos_create` does, on a deck the catalog
/// refuses; the caller catches it.
pub fn steps_for(
    pinned: Pinned,
    options: &DuosCreateOptions,
    log: &[DuosAction],
) -> Result<DuosSteps, ReplayRefusal> {
    registered();
    pinned.run(|| {
        let begun = duos_begin(&duos_create(options));
        if begun.error.is_some() {
            return Err(ReplayRefusal::RulesChanged);
        }
        let mut state = begun.state;
        let mut steps = DuosSteps {
            a1: Vec::with_capacity(log.len() + 1),
            a2: Vec::with_capacity(log.len() + 1),
            result: None,
        };
        steps.a1.push(duos_view_for(&state, DuosSeat::A1));
        steps.a2.push(duos_view_for(&state, DuosSeat::A2));
        for action in log {
            let result = duos_reduce(&state, action);
            if result.error.is_some() {
                return Err(ReplayRefusal::RulesChanged);
            }
            state = result.state;
            steps.a1.push(duos_view_for(&state, DuosSeat::A1));
            steps.a2.push(duos_view_for(&state, DuosSeat::A2));
        }
        steps.result = state.result;
        Ok(steps)
    })
}
