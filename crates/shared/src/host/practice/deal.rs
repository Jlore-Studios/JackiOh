//! How a practice game is dealt (SPEC §9.9, §9.10): the decks, the handicaps and the setup the engine
//! creates the game from. The one place both the page's worker and the server's claim check read, so a
//! claim folds from exactly the game the worker dealt (R1303000, R2560000).

use jackioh_ai::{AiDeckOptions, build_ai_deck};
use jackioh_engine::{
    AI_DIFFICULTY, AI_TUTORIAL, CreateGameOptions, DECK_SIZE, DeckMark, Handicap, LastBoardEntry, LeanPick,
    PerPlayerOpt, PlayerId, Rng, SetupKind, TUTORIAL_HUMAN,
};

use super::types::{PracticeConfig, PracticeDeckChoice};

/// A game dealt: what `create_game` takes, and the config it is played under.
#[derive(Clone, Debug, PartialEq)]
pub struct Opening {
    /// The game as the engine creates it.
    pub options: CreateGameOptions,
    /// The config the game is played under: a run fight's seed replaces the asked one (R2706000).
    pub config: PracticeConfig,
}

/// R433: the seats whose deck was dealt rather than built, so the human is shown none of it going in.
/// The AI's deck is never counted dealt: no view shows its seat, and R185's redaction would hide its own
/// library.
fn dealt_seats(config: &PracticeConfig) -> Option<Vec<PlayerId>> {
    // R2706000: a run deck is built over the run, not dealt, whatever `deck` says.
    if config.is_run() || config.is_lesson() {
        return None;
    }
    matches!(config.deck, PracticeDeckChoice::Random { .. }).then(|| vec![config.human_seat])
}

/// R1373, R924000: the pick a random deck leans on, R1373's `lean_newest` read as "newest".
fn lean_of(lean_set: Option<LeanPick>, lean_newest: Option<bool>) -> Option<LeanPick> {
    lean_set.or((lean_newest == Some(true)).then_some(LeanPick::Newest))
}

/// `create_game` checks a preset or a saved deck like any other (§2.6). R1373, R924000: a random deck
/// leans as asked, "newest" (R1371) resolved here; the AI's never leans. A player's random deck is dealt
/// from every set: neither the AI's shadow ban nor its soft gate (R1390) applies.
fn human_deck(seed: &str, choice: &PracticeDeckChoice) -> Result<Vec<String>, String> {
    match choice {
        PracticeDeckChoice::Random {
            lean_set,
            lean_newest,
        } => Ok(build_ai_deck(
            &mut Rng::new(&format!("{seed}:human-deck"), 0),
            DECK_SIZE,
            &AiDeckOptions {
                banned: Some(Vec::new()),
                gated_sets: Some(Vec::new()),
                lean_set: lean_of(*lean_set, *lean_newest).map(LeanPick::resolve),
                ..AiDeckOptions::default()
            },
        )),
        PracticeDeckChoice::Preset {
            cards: Some(cards), ..
        }
        | PracticeDeckChoice::Saved { cards, .. } => Ok(cards.clone()),
        PracticeDeckChoice::Preset { id, cards: None } => Err(format!("unknown preset deck \"{id}\"")),
    }
}

/// The decks and handicaps a game is dealt: the human's deck, the AI's, the AI's handicap and the
/// human's when it has one.
struct Deal {
    human_deck: Vec<String>,
    ai_deck: Vec<String>,
    handicap: Handicap,
    human_handicap: Option<Handicap>,
}

/// §9.10, R291: a lesson's fixed decks, the tutorial handicap on the AI (R290) and `TUTORIAL_HUMAN` on
/// the human, whose fixed deck is smaller than `DECK_SIZE` (R1705005); or a free game's (§9.9), none on
/// the human. R2706000: a dungeon run's fight plays its setup's two decks and two handicaps, whatever
/// `deck` and `difficulty` say.
fn deal_decks(config: &PracticeConfig) -> Result<Deal, String> {
    if let Some(run) = &config.run {
        return Ok(Deal {
            human_deck: run.human_deck.clone(),
            ai_deck: run.enemy_deck.clone(),
            handicap: run.enemy_handicap,
            human_handicap: Some(run.human_handicap),
        });
    }
    if let Some(lesson) = &config.lesson {
        let Some(deal) = &config.lesson_deal else {
            return Err(format!("unknown tutorial lesson \"{lesson}\""));
        };
        // R2201001: the lesson's pool hold rides on both seats' handicaps (lessons 1–4).
        let hold = deal.pool_hold.then_some(true);
        return Ok(Deal {
            human_deck: deal.human_deck.clone(),
            ai_deck: deal.ai_deck.clone(),
            handicap: Handicap {
                pool_hold: hold.or(AI_TUTORIAL.pool_hold),
                ..AI_TUTORIAL
            },
            human_handicap: Some(Handicap {
                pool_hold: hold.or(TUTORIAL_HUMAN.pool_hold),
                ..TUTORIAL_HUMAN
            }),
        });
    }
    let handicap = AI_DIFFICULTY[config.difficulty];
    Ok(Deal {
        human_deck: human_deck(&config.seed, &config.deck)?,
        ai_deck: build_ai_deck(
            &mut Rng::new(&format!("{}:ai-deck", config.seed), 0),
            handicap.deck_size,
            &AiDeckOptions {
                mana_cap: Some(handicap.mana_cap),
                banned: config.ai_banned.clone(),
                ..AiDeckOptions::default()
            },
        ),
        handicap,
        human_handicap: None,
    })
}

/// `body` and `rest` in seat order, as `home` and its opponent.
fn seat_ordered<T>(home: PlayerId, body: T, rest: T) -> (T, T) {
    match home {
        PlayerId::P1 => (body, rest),
        PlayerId::P2 => (rest, body),
    }
}

/// `value` at `seat`, nothing at the other.
fn only_at<T>(seat: PlayerId, value: T) -> PerPlayerOpt<T> {
    let mut at = PerPlayerOpt::default();
    match seat {
        PlayerId::P1 => at.p1 = Some(value),
        PlayerId::P2 => at.p2 = Some(value),
    }
    at
}

/// Deals the game `config` asks for. `Err` is the engine's own reason when a deck is refused (a wrong
/// size, a duplicate, an unknown card, a handicap it will not take), or the config's (an unknown preset,
/// a lesson with no decks): no game, and the page is told why.
pub fn deal(asked: &PracticeConfig) -> Result<Opening, String> {
    // R2706000: a run fight plays on its setup's seed (the run's, row and column: DR14 B9), whatever the
    // config's own says, so everything below, the AI's stream included, reads that one.
    let mut config = asked.clone();
    if let Some(run) = &asked.run {
        config.seed = run.seed.clone();
    }
    let human = config.human_seat;
    let Deal {
        human_deck,
        ai_deck,
        handicap,
        human_handicap,
    } = deal_decks(&config)?;
    let mut handicaps = only_at(human.opponent(), handicap);
    if let Some(own) = human_handicap {
        match human {
            PlayerId::P1 => handicaps.p1 = Some(own),
            PlayerId::P2 => handicaps.p2 = Some(own),
        }
    }
    // R508: the human's last board, on a free game only; the AI's seat never has one. R923000: a sandbox
    // game brings none, and R2706000: nor does a run fight, a game of its own kind.
    let board: Vec<LastBoardEntry> = if config.is_lesson() || config.is_sandbox() || config.is_run() {
        Vec::new()
    } else {
        config.last_board.clone().unwrap_or_default()
    };
    let last_boards = (!board.is_empty()).then(|| seat_ordered(human, board, Vec::new()));
    // R2706000: the run deck's Radiant and buffed cards, at the human's seat; none, no key, so a game
    // without marks is created and hashes as before (DR14 B3).
    let deck_marks = config
        .run
        .as_ref()
        .filter(|run| !run.human_marks.is_empty())
        .map(|run| only_at::<Vec<DeckMark>>(human, run.human_marks.clone()));
    let options = CreateGameOptions {
        seed: config.seed.clone(),
        decks: seat_ordered(human, human_deck, ai_deck),
        handicaps: Some(handicaps),
        dealt: dealt_seats(&config),
        last_boards,
        deck_marks,
        // R923000: the one game whose dev tools the engine takes.
        sandbox: config.is_sandbox().then_some(true),
        ..CreateGameOptions::default()
    };
    // `create_game` panics on an illegal deck, and a panic in WebAssembly is a trap the page cannot
    // catch, so the engine's own check goes first (R184).
    jackioh_engine::validate_setup(&options, jackioh_engine::registered_catalog(), SetupKind::New)
        .map_err(|error| error.to_string())?;
    Ok(Opening { options, config })
}
