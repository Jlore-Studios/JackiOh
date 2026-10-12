//! `cli/balance_watch.rs` (balance-watch), SPEC §9.11's advisory outlier watch for #699,
//! against the fake store: what the options read, that a planted 80% card flags high and a 50%
//! card is silent, that a 20% card flags low, that a large played delta flags whatever the in-deck
//! rate, that a card under the sample is never flagged, that the report says provisional below the
//! gate and counts live-only when forced, and that bot seats count like the public endpoint
//! (R1451). The arithmetic is the engine's `wire::stats`; the store's own filtering is
//! `tests/store/contract.rs`'s.

use serde_json::{Value, json};

use jackioh_engine::wire::GameRecord;
use jackioh_server::cli::balance_watch::{
    BalanceWatchOptions, balance_watch_report, is_provisional, parse_balance_watch_args,
    render_balance_watch, z_score,
};
use jackioh_server::config::{CARD_STATS_MIN_SAMPLE, PUBLIC_STATS_MIN_LIVE_GAMES};
use jackioh_server::db::store::Db;

const PATCH: &str = "v0.3.6";

/// `{ ...base, ...partial }` on two JSON objects.
fn spread(mut base: Value, partial: Value) -> Value {
    if let (Value::Object(into), Value::Object(from)) = (&mut base, partial) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
    base
}

fn seat(deck: &[&str], played: &[&str], hand: &[&str]) -> Value {
    json!({ "deck": deck, "opening": hand, "drawn": [], "played": played })
}

fn record_json(id: &str, partial: Value) -> Value {
    spread(
        json!({
            "id": id,
            "source": "live",
            "mode": "bo1",
            "patch": PATCH,
            "pilots": { "p1": "human", "p2": "human" },
            "game": {
                "first": "p1",
                "winner": "p1",
                "reason": "hero-death",
                "turns": 9,
                "seats": {
                    "p1": seat(&[], &[], &[]),
                    "p2": seat(&[], &[], &[]),
                }
            }
        }),
        partial,
    )
}

fn record_of(value: Value) -> GameRecord {
    serde_json::from_value(value).expect("a well-formed game record")
}

/// One game with fixed seats: p1 holds `p1_card` in its deck and played it, p2 holds `p2_card`
/// in its deck and never played it (its hand holds it, so it counts drawn-but-not-played). Only
/// the winner varies, so a card's win rate is the share of games its seat won.
fn fixed(
    id: &str,
    source: &str,
    pilots: Value,
    p1_card: &str,
    p2_card: &str,
    winner_is_p1: bool,
) -> GameRecord {
    let mut game = record_json(id, json!({ "source": source, "pilots": pilots }));
    game["game"]["winner"] = if winner_is_p1 { json!("p1") } else { json!("p2") };
    game["game"]["seats"]["p1"] = seat(&[p1_card], &[p1_card], &[]);
    game["game"]["seats"]["p2"] = seat(&[p2_card], &[], &[p2_card]);
    record_of(game)
}

fn dev_game(id: &str, p1_card: &str, p2_card: &str, winner_is_p1: bool) -> GameRecord {
    fixed(
        id,
        "dev",
        json!({ "p1": "ai", "p2": "ai" }),
        p1_card,
        p2_card,
        winner_is_p1,
    )
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

fn options(argv: &[&str]) -> BalanceWatchOptions {
    parse_balance_watch_args(&args(argv)).expect("the options parse")
}

async fn seed(records: &[GameRecord]) -> Db {
    let db = Db::fake();
    let mut tx = db.begin(None).await.expect("a transaction");
    for record in records {
        tx.game_records_insert(record)
            .await
            .expect("the record is written");
    }
    tx.commit().await.expect("the records commit");
    db
}

fn cards(rows: &[jackioh_server::cli::balance_watch::FlaggedCard]) -> Vec<&str> {
    rows.iter().map(|row| row.card.as_str()).collect()
}

fn nobody(_: &str) -> Option<String> {
    None
}

mod parse_balance_watch_args_cases {
    use super::*;
    use jackioh_engine::wire::SourceFilter;

    #[test]
    fn parse_requires_patch_and_defaults_source_to_all() {
        let error = parse_balance_watch_args(&args(&[]))
            .expect_err("no arguments are refused")
            .to_string();
        assert!(error.contains("--patch"), "{error}");
        assert!(error.contains("Usage:"), "{error}");
        let parsed = options(&["--patch=v0.3.6"]);
        assert_eq!(parsed.patch, "v0.3.6");
        assert_eq!(parsed.source, SourceFilter::All);
        let live = options(&["--patch=v0.3.6", "--source=live"]);
        assert_eq!(live.source, SourceFilter::Live);
    }

    #[test]
    fn parse_refuses_dev_source_and_unknown_flags() {
        let error = parse_balance_watch_args(&args(&["--patch=v0.3.6", "--source=dev"]))
            .expect_err("--source=dev is refused")
            .to_string();
        assert!(error.contains("--source must be one of live, all"), "{error}");
        assert!(error.contains("Usage:"), "{error}");
        let error = parse_balance_watch_args(&args(&["--patch=v0.3.6", "--bogus=1"]))
            .expect_err("an unknown flag is refused")
            .to_string();
        assert!(error.contains("Unrecognised option --bogus"), "{error}");
        assert!(error.contains("Usage:"), "{error}");
    }
}

mod z_score_cases {
    use super::*;

    #[test]
    fn z_scores_against_50_percent_and_is_none_without_games() {
        assert_eq!(z_score(0, 0), None);
        assert_eq!(z_score(20, 40), Some(0.0));
        let high = z_score(32, 40).expect("a z-score");
        assert!((high - 3.79).abs() < 0.01, "{high}");
        let low = z_score(8, 40).expect("a z-score");
        assert!((low + 3.79).abs() < 0.01, "{low}");
    }

    #[test]
    fn is_provisional_flips_at_the_gate() {
        assert!(is_provisional(PUBLIC_STATS_MIN_LIVE_GAMES - 1));
        assert!(!is_provisional(PUBLIC_STATS_MIN_LIVE_GAMES));
        assert!(is_provisional(0));
    }
}

mod balance_watch_report_cases {
    use super::*;

    /// Forty development games where p1's card wins 32 (80%) and forty where p1's card wins 20
    /// (50%). Each game's other seat holds a card seen once only, so no filler gathers games.
    async fn planted() -> Db {
        let mut records = Vec::new();
        for n in 0..40 {
            let strong_filler = format!("bw-strong-filler-{n}");
            records.push(dev_game(
                &format!("dev:strong:{n}"),
                "bw-strong",
                &strong_filler,
                n < 32,
            ));
            let fair_filler = format!("bw-fair-filler-{n}");
            records.push(dev_game(
                &format!("dev:fair:{n}"),
                "bw-fair",
                &fair_filler,
                n < 20,
            ));
        }
        seed(&records).await
    }

    #[tokio::test]
    async fn flags_a_planted_80_percent_card_and_leaves_a_50_percent_card_silent() {
        let db = planted().await;
        let report = balance_watch_report(&db, &options(&["--patch=v0.3.6"]))
            .await
            .expect("a report");
        assert!(report.provisional, "no live games: provisional");
        assert_eq!(cards(&report.high), vec!["bw-strong"]);
        let strong = &report.high[0];
        assert_eq!((strong.games, strong.win_rate), (40, 0.8));
        assert!(strong.z >= 2.5, "z is {}", strong.z);
        for list in [&report.low, &report.delta, &report.watched] {
            assert!(!cards(list).contains(&"bw-strong"), "bw-strong in {list:?}");
        }
        for list in [&report.high, &report.low, &report.delta, &report.watched] {
            assert!(!cards(list).contains(&"bw-fair"), "bw-fair flagged: {list:?}");
        }
        assert!(!report.not_enough.contains(&"bw-fair".to_owned()));
        let text = render_balance_watch(&report, &nobody);
        assert!(text.contains("provisional"), "{text}");
        assert!(text.contains("bw-strong"), "{text}");
        assert!(text.contains("Advisory only"), "{text}");
    }

    #[tokio::test]
    async fn flags_a_planted_20_percent_card_as_low() {
        let mut records = Vec::new();
        for n in 0..40 {
            let filler = format!("bw-weak-filler-{n}");
            records.push(dev_game(&format!("dev:weak:{n}"), "bw-weak", &filler, n < 8));
        }
        let db = seed(&records).await;
        let report = balance_watch_report(&db, &options(&["--patch=v0.3.6"]))
            .await
            .expect("a report");
        assert_eq!(cards(&report.low), vec!["bw-weak"]);
        assert!(report.high.is_empty());
        assert!(!report.not_enough.contains(&"bw-weak".to_owned()));
    }

    #[tokio::test]
    async fn flags_a_large_played_delta_at_a_flat_in_deck_rate() {
        // p1 always plays bw-tricky and wins 28 of 40 (played 70%); p2 always holds it unplayed
        // and wins 12 (drawn-but-not-played 30%). In-deck is 40 of 80 (50%, z 0): only the delta
        // flags.
        let mut records = Vec::new();
        for n in 0..40 {
            records.push(dev_game(
                &format!("dev:tricky:{n}"),
                "bw-tricky",
                "bw-tricky",
                n < 28,
            ));
        }
        let db = seed(&records).await;
        let report = balance_watch_report(&db, &options(&["--patch=v0.3.6"]))
            .await
            .expect("a report");
        assert!(report.high.is_empty(), "{:?}", report.high);
        assert!(report.low.is_empty(), "{:?}", report.low);
        assert_eq!(cards(&report.delta), vec!["bw-tricky"]);
        let gap = report.delta[0].played_delta.expect("a delta");
        assert!((gap - 0.4).abs() < 0.001, "{gap}");
    }

    #[tokio::test]
    async fn never_flags_a_card_under_min_sample() {
        let mut records = Vec::new();
        for n in 0..5 {
            let filler = format!("bw-rare-filler-{n}");
            records.push(dev_game(&format!("dev:rare:{n}"), "bw-rare", &filler, true));
        }
        let db = seed(&records).await;
        let report = balance_watch_report(&db, &options(&["--patch=v0.3.6"]))
            .await
            .expect("a report");
        assert!(report.not_enough.contains(&"bw-rare".to_owned()));
        for list in [&report.high, &report.low, &report.delta, &report.watched] {
            assert!(!cards(list).contains(&"bw-rare"), "bw-rare flagged: {list:?}");
        }
        let text = render_balance_watch(&report, &nobody);
        assert!(text.contains("bw-rare"), "{text}");
        assert!(text.contains(&format!("{CARD_STATS_MIN_SAMPLE}")), "{text}");
    }

    #[tokio::test]
    async fn reports_provisional_below_the_gate_and_live_only_when_forced() {
        let records = [
            fixed(
                "live-1",
                "live",
                json!({ "p1": "human", "p2": "human" }),
                "bw-live-card",
                "bw-live-filler",
                true,
            ),
            fixed(
                "live-2",
                "live",
                json!({ "p1": "human", "p2": "human" }),
                "bw-live-card",
                "bw-live-filler",
                false,
            ),
            dev_game("dev:one:0", "bw-dev-card", "bw-dev-filler", true),
            dev_game("dev:one:1", "bw-dev-card", "bw-dev-filler", false),
        ];
        let db = seed(&records).await;
        let blended = balance_watch_report(&db, &options(&["--patch=v0.3.6"]))
            .await
            .expect("a report");
        assert!(blended.provisional);
        assert_eq!(blended.live_games, 2);
        assert_eq!(blended.games, 4);
        let live_only = balance_watch_report(&db, &options(&["--patch=v0.3.6", "--source=live"]))
            .await
            .expect("a report");
        assert!(live_only.provisional, "two live games are still provisional");
        assert_eq!(live_only.games, 2);
        assert_eq!(live_only.live_games, 2);
        let text = render_balance_watch(&live_only, &nobody);
        assert!(text.contains("provisional"), "{text}");
    }

    #[tokio::test]
    async fn counts_seats_like_the_public_endpoint() {
        // R1451: a live human-vs-bot game counts the human seat only, a development game both.
        let mut records = Vec::new();
        for n in 0..20 {
            records.push(fixed(
                &format!("live-bot-{n}"),
                "live",
                json!({ "p1": "human", "p2": "ai" }),
                "bw-human",
                "bw-human",
                true,
            ));
        }
        for n in 0..10 {
            records.push(dev_game(&format!("dev:both:{n}"), "bw-dev", "bw-dev", true));
        }
        let db = seed(&records).await;
        let report = balance_watch_report(&db, &options(&["--patch=v0.3.6"]))
            .await
            .expect("a report");
        // Twenty human seats at 100%: watched (20–39 games), never flagged — both seats would be
        // forty games and flag high.
        let human = report
            .watched
            .iter()
            .find(|row| row.card == "bw-human")
            .expect("bw-human is watched");
        assert_eq!((human.games, human.win_rate), (20, 1.0));
        assert!(!cards(&report.high).contains(&"bw-human"));
        // Ten development games at 100% over both seats: twenty games, ten wins.
        let dev = report
            .watched
            .iter()
            .find(|row| row.card == "bw-dev")
            .expect("bw-dev is watched");
        assert_eq!((dev.games, dev.win_rate), (20, 0.5));
        assert!(!report.not_enough.contains(&"bw-dev".to_owned()));
    }
}
