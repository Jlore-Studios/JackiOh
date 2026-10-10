//! R290: the tutorial's opponent is easier than Easy, by play (SPEC §9.9, R180).
//!
//! The tutorial deals its AI seat AI_TUTORIAL: a 12-card deck, at most 3 mana crystals and a hero at
//! 20. The AI itself is the one every tier plays, so the tier is only easier if those resources make
//! it lose more. The human's stand-in, the greedy baseline at a human's resources, has to beat the AI
//! at AI_TUTORIAL in about half its games, and on the same seeds more often than the AI at Easy.
//!
//! Game n plays seed `${TUTORIAL_TIER.series}:greedy:${n}` at both tiers, so only the AI seat's
//! handicap and deck differ. Both seats are dealt by the gates' one rule, with the shadow ban (R186)
//! on both sides, and every game has to be clean and fold back to its hash (B31).
//! §10.7's random policy is not asserted: a run small enough for `pnpm test` holds too few wins.

use std::io::Write as _;
use std::sync::OnceLock;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

/// Every number this file states (CLAUDE.md rule 9). The runs are frozen; the thresholds sit below
/// what was measured so that a change which re-deals every game (as The Coin did, R244) still passes
/// when the tier is as much easier as measured. A re-dealt run fails them about 22 and 9 times in 100
/// (the gates allow 5, SPEC §9.9); a wider margin would take a weaker AI_TUTORIAL (R290).
struct TutorialTier {
    /// The frozen seed series; no gate or tuning run plays it.
    series: &'static str,
    /// Games 1..tutorialGames against the AI at AI_TUTORIAL.
    tutorial_games: i32,
    /// Greedy's wins against AI_TUTORIAL the run needs: about half of its 13. Measured: 6 of these 13
    /// (53 of 100 on seeds 1–100, where a re-dealt run of 13 reaches 6 about 78 times in 100, and an AI
    /// as strong as Easy, 17 of 100, lets it about once in 70).
    greedy_wins_vs_tutorial: i32,
    /// Games 1..easyGames are played at Easy too, for the comparison on the same seeds. An Easy game
    /// costs the AI more than twice the search of a tutorial one (4 crystals give it more to try), so
    /// this is kept smaller than tutorialGames.
    easy_games: i32,
    /// How many more of seeds 1..easyGames greedy wins against AI_TUTORIAL than against Easy. Measured:
    /// 4 against 3 (53 against 17 on seeds 1–100, where a re-dealt run of 8 keeps AI_TUTORIAL ahead
    /// about 91 times in 100).
    greedy_margin_over_easy: i32,
}

const TUTORIAL_TIER: TutorialTier = TutorialTier {
    series: "tutorial-tier",
    tutorial_games: 13,
    greedy_wins_vs_tutorial: 6,
    easy_games: 8,
    greedy_margin_over_easy: 1,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tier {
    Tutorial,
    Easy,
}

impl Tier {
    fn name(self) -> &'static str {
        match self {
            Tier::Tutorial => "tutorial",
            Tier::Easy => "easy",
        }
    }
}

fn tier_handicap(tier: Tier) -> Handicap {
    match tier {
        Tier::Tutorial => AI_TUTORIAL,
        Tier::Easy => AI_DIFFICULTY.easy,
    }
}

fn tier_games(tier: Tier) -> i32 {
    match tier {
        Tier::Tutorial => TUTORIAL_TIER.tutorial_games,
        Tier::Easy => TUTORIAL_TIER.easy_games,
    }
}

/// Game n (1-based): the greedy seat, standing in for the human, sits p1 when n is odd.
fn human_seat_of(n: i32) -> PlayerId {
    if n % 2 == 1 { PlayerId::P1 } else { PlayerId::P2 }
}

/// The deck a config deals `seat`.
fn deck_at(config: &MatchConfig, seat: PlayerId) -> &Vec<String> {
    match seat {
        PlayerId::P1 => &config.decks.0,
        PlayerId::P2 => &config.decks.1,
    }
}

/// Game n of the greedy baseline against the AI at `tier`.
fn tier_game(tier: Tier, n: i32) -> MatchConfig {
    jackioh_cards::register_all();
    let seed = format!("{}:greedy:{n}", TUTORIAL_TIER.series);
    let human_seat = human_seat_of(n);
    let ai_seat = human_seat.opponent();
    let ai_handicap = tier_handicap(tier);

    let deck_for = |seat: PlayerId, handicap: &Handicap| -> Vec<String> {
        build_ai_deck(
            &mut create_rng(&format!("{seed}:deck:{seat}"), 0),
            handicap.deck_size,
            &AiDeckOptions {
                mana_cap: Some(handicap.mana_cap),
                ..Default::default()
            },
        )
    };
    let human_deck = deck_for(human_seat, &HUMAN_HANDICAP);
    let ai_deck = deck_for(ai_seat, &ai_handicap);

    // The greedy seat plays with no handicap (R180); at Easy the AI seat stores none either.
    let mut handicaps: PerPlayerOpt<Handicap> = PerPlayerOpt::default();
    *handicaps.slot(ai_seat) = Some(ai_handicap);
    let ai = SeatController::Ai {
        budget: Some(AI_GATE_BUDGET),
    };
    MatchConfig {
        seed,
        decks: if human_seat == PlayerId::P1 {
            (human_deck, ai_deck)
        } else {
            (ai_deck, human_deck)
        },
        handicaps: Some(handicaps),
        controllers: if human_seat == PlayerId::P1 {
            PerPlayer {
                p1: SeatController::Greedy,
                p2: ai,
            }
        } else {
            PerPlayer {
                p1: ai,
                p2: SeatController::Greedy,
            }
        },
        max_actions: None,
    }
}

struct TierGame {
    n: i32,
    config: MatchConfig,
    record: MatchRecord,
    human_won: bool,
    replay_hash: String,
    replay_errors: usize,
}

static TUTORIAL_PLAYED: OnceLock<Vec<TierGame>> = OnceLock::new();
static EASY_PLAYED: OnceLock<Vec<TierGame>> = OnceLock::new();

/// Plays games 1..TIER_GAMES[tier] at `tier`, each folded back from its log.
fn play_tier(tier: Tier) -> Vec<TierGame> {
    let mut games = Vec::new();
    for n in 1..=tier_games(tier) {
        let config = tier_game(tier, n);
        let record = play_match(&config, &mut MatchHooks::default());
        let replayed = fold(&json_as(json!({
            "seed": config.seed,
            "decks": [config.decks.0, config.decks.1],
            "log": record.log,
            "handicaps": config.handicaps,
        })));
        let human_won = record
            .result
            .as_ref()
            .is_some_and(|result| result.winner == Winner::from(human_seat_of(n)));
        games.push(TierGame {
            n,
            replay_hash: hash_state(&replayed.state),
            replay_errors: replayed.errors.len(),
            human_won,
            config,
            record,
        });
    }
    games
}

/// Plays games 1..TIER_GAMES[tier] at `tier`, once per test binary.
fn run(tier: Tier) -> &'static [TierGame] {
    let cache = match tier {
        Tier::Tutorial => &TUTORIAL_PLAYED,
        Tier::Easy => &EASY_PLAYED,
    };
    cache.get_or_init(|| play_tier(tier))
}

/// Greedy's wins among games 1..upTo (all of them when `up_to` is `None`).
fn wins_of(games: &[TierGame], up_to: Option<i32>) -> i32 {
    let up_to = up_to.unwrap_or(games.len() as i32);
    games
        .iter()
        .filter(|game| game.n <= up_to && game.human_won)
        .count() as i32
}

fn not_won(games: &[TierGame]) -> String {
    games
        .iter()
        .filter(|game| !game.human_won)
        .map(|game| {
            format!(
                "{} (greedy {}, {})",
                game.config.seed,
                human_seat_of(game.n),
                serde_json::to_string(&game.record.result).unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

mod r290_the_tutorial_tier_by_play {
    use super::*;

    /// R290 tierGame seats greedy at a human's resources against the AI at AI_TUTORIAL or Easy, on the same seed, seats alternating
    #[test]
    fn r290_tier_game_seats_greedy_at_a_humans_resources_against_the_ai_at_ai_tutorial_or_easy_on_the_same_seed_seats_alternating()
     {
        const { assert!(TUTORIAL_TIER.easy_games <= TUTORIAL_TIER.tutorial_games) };
        for n in [1, 2, 3] {
            let human_seat = human_seat_of(n);
            let ai_seat = human_seat.opponent();
            let tutorial = tier_game(Tier::Tutorial, n);
            let easy = tier_game(Tier::Easy, n);

            assert_eq!(tutorial.seed, format!("{}:greedy:{n}", TUTORIAL_TIER.series));
            assert_eq!(easy.seed, tutorial.seed);
            for config in [&tutorial, &easy] {
                assert_eq!(config.controllers[human_seat], SeatController::Greedy);
                assert_eq!(
                    config.controllers[ai_seat],
                    SeatController::Ai {
                        budget: Some(AI_GATE_BUDGET)
                    }
                );
                let human_handicap = config
                    .handicaps
                    .as_ref()
                    .and_then(|handicaps| handicaps.get(human_seat))
                    .copied()
                    .unwrap_or(HUMAN_HANDICAP);
                assert_eq!(human_handicap, HUMAN_HANDICAP);
                assert_eq!(deck_at(config, human_seat).len(), DECK_SIZE as usize);
            }
            // The greedy seat's deck is the same at both tiers; only the AI seat's differs.
            assert_eq!(deck_at(&easy, human_seat), deck_at(&tutorial, human_seat));

            assert_eq!(
                tutorial
                    .handicaps
                    .as_ref()
                    .and_then(|handicaps| handicaps.get(ai_seat))
                    .copied(),
                Some(AI_TUTORIAL)
            );
            assert_eq!(deck_at(&tutorial, ai_seat).len(), AI_TUTORIAL.deck_size as usize);
            assert_eq!(
                *deck_at(&tutorial, ai_seat),
                build_ai_deck(
                    &mut create_rng(&format!("{}:deck:{ai_seat}", tutorial.seed), 0),
                    AI_TUTORIAL.deck_size,
                    &AiDeckOptions {
                        mana_cap: Some(AI_TUTORIAL.mana_cap),
                        ..Default::default()
                    },
                )
            );
            let easy_ai_handicap = easy
                .handicaps
                .as_ref()
                .and_then(|handicaps| handicaps.get(ai_seat))
                .copied()
                .unwrap_or(HUMAN_HANDICAP);
            assert_eq!(easy_ai_handicap, AI_DIFFICULTY.easy);
            assert_eq!(deck_at(&easy, ai_seat).len(), DECK_SIZE as usize);
        }
    }

    /// R290 the greedy baseline at a human's resources beats the AI at AI_TUTORIAL in at least 6 of 13 games
    #[test]
    fn r290_the_greedy_baseline_at_a_humans_resources_beats_the_ai_at_ai_tutorial_in_at_least_6_of_13_games()
    {
        let games = run(Tier::Tutorial);
        assert_eq!(games.len() as i32, TUTORIAL_TIER.tutorial_games);
        let wins = wins_of(games, None);
        // Written to stdout past the test harness's capture, as the gates write theirs.
        let _ = writeln!(
            std::io::stdout(),
            "[R290 tutorial tier] greedy won {wins} of {} against AI_TUTORIAL; {} needed",
            games.len(),
            TUTORIAL_TIER.greedy_wins_vs_tutorial
        );
        assert!(
            wins >= TUTORIAL_TIER.greedy_wins_vs_tutorial,
            "not won: {}",
            not_won(games)
        );
    }

    /// R290 on seeds 1–8 the same greedy wins more games against AI_TUTORIAL than against Easy, by at least 1
    #[test]
    fn r290_on_seeds_1_8_the_same_greedy_wins_more_games_against_ai_tutorial_than_against_easy_by_at_least_1()
    {
        let easy_games = run(Tier::Easy);
        assert_eq!(easy_games.len() as i32, TUTORIAL_TIER.easy_games);
        let tutorial = wins_of(run(Tier::Tutorial), Some(TUTORIAL_TIER.easy_games));
        let easy = wins_of(easy_games, None);
        let _ = writeln!(
            std::io::stdout(),
            "[R290 tutorial tier] on seeds 1-{} greedy won {tutorial} against AI_TUTORIAL and {easy} against Easy",
            TUTORIAL_TIER.easy_games
        );
        assert!(
            tutorial - easy >= TUTORIAL_TIER.greedy_margin_over_easy,
            "AI_TUTORIAL {tutorial}, Easy {easy}; Easy not won: {}",
            not_won(easy_games)
        );
    }

    /// R290 every game at either tier is clean: nothing rejected or thrown, no fallback, a result, and a replay that matches
    #[test]
    fn r290_every_game_at_either_tier_is_clean_nothing_rejected_or_thrown_no_fallback_a_result_and_a_replay_that_matches()
     {
        for tier in [Tier::Tutorial, Tier::Easy] {
            let games = run(tier);
            assert_eq!(games.len() as i32, tier_games(tier), "{}", tier.name());
            for game in games {
                let label = format!("{} {}", tier.name(), game.config.seed);
                assert!(game.record.rejected.is_empty(), "{label}");
                assert!(game.record.thrown.is_empty(), "{label}");
                assert_eq!(game.record.fallbacks, 0, "{label}");
                assert!(game.record.result.is_some(), "{label}");
                assert_eq!(game.replay_errors, 0, "{label}");
                assert_eq!(game.replay_hash, game.record.hash, "{label}");
            }
        }
    }
}
