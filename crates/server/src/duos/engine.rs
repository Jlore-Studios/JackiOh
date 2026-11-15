//! The seam between the co-op server and the engine's Duos wrapper (fullsend SPEC S4,
//! CLAUDE.md rule 7): the server folds logs and relays views, registering cards first.
//!
//! Every function that reads the catalog runs under the match's `Pinned` catalog (R767000).
//! Decks: a person's saved deck or All Random deal (R258, R1372); an AI seat's deck from its
//! random deck-builder at its tier's handicap (§9.9) on `{seed}:{seat}-deck`.

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::config::{AI_DIFFICULTY, DUOS_HUMAN_TEAM, Difficulty, HUMAN_HANDICAP, duos_ai_team};
use jackioh_engine::duos::{
    DuosReduceResult, DuosReplayInput, DuosReplayResult, DuosState, duos_begin, duos_create,
    duos_create_recorded, duos_fold, duos_hash, duos_legal_actions, duos_reduce, duos_view_for,
};
use jackioh_engine::wire::{
    DuosAction, DuosActionBody, DuosCreateOptions, DuosDecks, DuosHandicaps, DuosResult, DuosSeat, DuosView,
    PerTeam,
};
use jackioh_engine::{Handicap, ReplayRefusal, Rng};
use jackioh_shared::host::DuosTeamB;
use jackioh_shared::host::duos::slot;

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

/// S6: an AI seat's deck: shadow ban and soft gate, its tier's deck size and mana cap.
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

/// R3731002: who team B is when a match is made: the AI team at its tier, or two more people with their
/// picks.
#[derive(Clone, Copy, Debug)]
pub enum TeamBChoices<'a> {
    Ai(Difficulty),
    People { b1: &'a DuosChoice, b2: &'a DuosChoice },
}

/// S6, B30: match options created with `DUOS_HUMAN_TEAM` (no handicap) and `duos_ai_team(difficulty)`,
/// or, for a 2v2's people, `DUOS_HUMAN_TEAM` on both teams (R3731002). A person's random deck is dealt
/// (R433), an AI's never (R185: redaction hides its own library). Pinned to `catalog_version` (R767000).
pub fn match_options(
    pinned: Pinned,
    catalog_version: &str,
    seed: &str,
    team_b: TeamBChoices<'_>,
    host: &DuosChoice,
    guest: &DuosChoice,
) -> DuosMatchOptions {
    pinned.run(|| dealt_options(catalog_version, seed, team_b, host, guest))
}

fn dealt_options(
    catalog_version: &str,
    seed: &str,
    team_b: TeamBChoices<'_>,
    host: &DuosChoice,
    guest: &DuosChoice,
) -> DuosMatchOptions {
    let mut people = vec![(DuosSeat::A1, host), (DuosSeat::A2, guest)];
    if let TeamBChoices::People { b1, b2 } = team_b {
        people.extend([(DuosSeat::B1, b1), (DuosSeat::B2, b2)]);
    }
    let dealt = people
        .iter()
        .filter(|(_, choice)| matches!(choice, DuosChoice::Random { .. }))
        .map(|(seat, _)| *seat)
        .collect();
    let a1 = person_deck(host, seed, DuosSeat::A1);
    let a2 = person_deck(guest, seed, DuosSeat::A2);
    let (b1, b2, b, handicap_b, who) = match team_b {
        TeamBChoices::Ai(difficulty) => {
            let handicap = AI_DIFFICULTY[difficulty];
            // Easy is a person's resources exactly, and the engine stores none for it.
            let ai_handicap = (handicap != HUMAN_HANDICAP).then_some(handicap);
            (
                ai_deck(seed, DuosSeat::B1, &handicap),
                ai_deck(seed, DuosSeat::B2, &handicap),
                duos_ai_team(difficulty),
                ai_handicap,
                DuosTeamB::Ai,
            )
        }
        TeamBChoices::People { b1, b2 } => (
            person_deck(b1, seed, DuosSeat::B1),
            person_deck(b2, seed, DuosSeat::B2),
            DUOS_HUMAN_TEAM,
            None,
            DuosTeamB::People,
        ),
    };
    DuosMatchOptions {
        decks: SeatDecks { a1, a2, b1, b2 },
        dealt,
        teams: TeamRules {
            a: DUOS_HUMAN_TEAM,
            b,
        },
        handicaps: SeatHandicaps {
            a1: None,
            a2: None,
            b1: handicap_b,
            b2: handicap_b,
        },
        catalog_version: catalog_version.to_string(),
        team_b: who,
    }
}

/// B41: every person's steps of a match, each seat's view as the game began (all four mulligans open)
/// and after each logged action, exactly what that seat's live view showed.
pub struct DuosSteps {
    /// By `slot`; an AI team's seats have none.
    pub steps: [Vec<DuosView>; 4],
    /// Where the game ended up, for the caller to hold to the result the match recorded.
    pub result: Option<DuosResult>,
}

impl DuosSteps {
    /// A person's own steps (the AI team's seats have none).
    pub fn of(&self, seat: DuosSeat) -> &[DuosView] {
        &self.steps[slot(seat)]
    }
}

/// B41: the match folded once under its catalog, its people's views after every step (team A's, and team
/// B's in a 2v2, R3731003). A log the engine no longer folds to its end, its setup or any action refused,
/// is `rules_changed` (B24, R768): the replay is refused, never cut short. The match is built as one
/// already made (R1705003). Panics, as `duos_create` does, on a deck the catalog refuses; the caller
/// catches it.
pub fn steps_for(
    pinned: Pinned,
    options: &DuosCreateOptions,
    log: &[DuosAction],
    team_b: DuosTeamB,
) -> Result<DuosSteps, ReplayRefusal> {
    registered();
    pinned.run(|| {
        let begun = duos_begin(&duos_create_recorded(options));
        if begun.error.is_some() {
            return Err(ReplayRefusal::RulesChanged);
        }
        let mut state = begun.state;
        let mut steps = DuosSteps {
            steps: Default::default(),
            result: None,
        };
        let record = |steps: &mut DuosSteps, state: &DuosState| {
            for seat in team_b.people() {
                steps.steps[slot(*seat)].push(duos_view_for(state, *seat));
            }
        };
        record(&mut steps, &state);
        for action in log {
            let result = duos_reduce(&state, action);
            if result.error.is_some() {
                return Err(ReplayRefusal::RulesChanged);
            }
            state = result.state;
            record(&mut steps, &state);
        }
        steps.result = state.result;
        Ok(steps)
    })
}
