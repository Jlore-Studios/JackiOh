//! `cargo jackioh stats fit-dealt <file>…` (SPEC §9.11, R378; `training/README.md`'s unban lane): a
//! refit of `crates/ai/src/shadow_ban.rs`'s `DEALT_Q` and `DEALT_Q_PRIOR` over arena record files,
//! printed to paste, as `sweep` prints `SHADOW_BAN`'s rows. Nothing is written back.
//!
//! Only `self-vs-bin-…` games count: agent `a`, this build's AI at full skill, against a parent
//! build. `a`'s seat is read off the record id (`arena::record_id`, `arena::game_setup`): the
//! `:p1`/`:p2` a paired game ends with, else the parity of the game number its seed ends with.
//! `DEALT_Q_PRIOR` is the candidate's share of the games it won, each card's `q` that share
//! shrunk toward it, `(w + 60 x prior) / (n + 60)`, over the candidate's decks that held it (a card
//! twice in one deck counts once). An id read twice counts once, as the store holds one row per id,
//! and a draw is not won.
//!
//! The output is the prior line and the rows by id, each as `shadow_ban.rs` writes it, plus an
//! advisory table for `UNBANNED_PREFER`: a person's choice on the sweep's numbers, never made here.
//!
//! Tooling, so it may read files and the console; `crates/ai` stays pure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use anyhow::{Result, bail};

use jackioh_ai::{DEALT_Q, UNBANNED_PREFER};
use jackioh_engine::{DEV_RECORD_ID_PREFIX, GameRecord, PlayerId};

use crate::stats::read_records;

/// shadow_ban.rs's shrinkage: each card's share pulled toward the prior as if it had this many more
/// games at the prior.
const SHRINK_GAMES: f64 = 60.0;
/// shadow_ban.rs writes each weight with this many decimals.
const DECIMALS: usize = 4;
/// The label of this build's AI at full skill in an arena record id (`arena::entrant`).
const CANDIDATE_LABEL: &str = "self";
/// How the label of a parent build begins in an arena record id (`bin-gen<generation>`).
const PARENT_LABEL_PREFIX: &str = "bin-";

/// `cargo jackioh stats fit-dealt <file>…`.
#[derive(clap::Args, Debug, Clone, PartialEq)]
pub struct Args {
    /// Arena record files, one GameRecord per line (the unban lane's `~/training-out/unban/*.jsonl`).
    #[arg(required = true, value_name = "FILE")]
    pub files: Vec<PathBuf>,
}

/// One card's counts over the candidate's decks that held it, and its shrunk share.
#[derive(Clone, Debug, PartialEq)]
pub struct DealtRow {
    pub card: String,
    pub games: i64,
    pub wins: i64,
    pub q: f64,
}

/// The refit: the games counted, the candidate's share of them and a row per card dealt, in id order.
#[derive(Clone, Debug, PartialEq)]
pub struct DealtFit {
    /// Records read, counted or not.
    pub read: usize,
    pub games: i64,
    pub wins: i64,
    pub prior: f64,
    pub rows: Vec<DealtRow>,
}

/// The seat agent `a` sat in, when the record is a `self-vs-bin-…` arena game of its own patch, else
/// `None`: another matchup, another skill, a gather game or a live match.
pub fn candidate_seat(record: &GameRecord) -> Option<PlayerId> {
    let rest = record
        .id
        .strip_prefix(&format!("{DEV_RECORD_ID_PREFIX}{}:arena:", record.patch))?;
    let (matchup, seed) = rest.split_once(':')?;
    let (a, b) = matchup.split_once("-vs-")?;
    if a != CANDIDATE_LABEL || !b.starts_with(PARENT_LABEL_PREFIX) {
        return None;
    }
    match seed.rsplit(':').next()? {
        "p1" => Some(PlayerId::P1),
        "p2" => Some(PlayerId::P2),
        n => Some(if n.parse::<i32>().ok()? % 2 == 1 {
            PlayerId::P1
        } else {
            PlayerId::P2
        }),
    }
}

/// The refit over `records`, or `None` when none is a `self-vs-bin` arena game. The records' order
/// changes nothing: counts are integers and the rows come out of a `BTreeMap`.
pub fn fit(records: &[GameRecord]) -> Option<DealtFit> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut by_card: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    let (mut games, mut wins) = (0i64, 0i64);
    for record in records {
        if seen.contains(record.id.as_str()) {
            continue;
        }
        let Some(seat) = candidate_seat(record) else {
            continue;
        };
        seen.insert(record.id.as_str());
        let won = record.game.winner.player() == Some(seat);
        games += 1;
        wins += i64::from(won);
        let held: BTreeSet<&str> = record.game.seats[seat].deck.iter().map(String::as_str).collect();
        for card in held {
            let tally = by_card.entry(card).or_default();
            tally.0 += 1;
            tally.1 += i64::from(won);
        }
    }
    if games == 0 {
        return None;
    }
    let prior = wins as f64 / games as f64;
    let rows = by_card
        .into_iter()
        .map(|(card, (n, w))| DealtRow {
            card: card.to_owned(),
            games: n,
            wins: w,
            q: (w as f64 + SHRINK_GAMES * prior) / (n as f64 + SHRINK_GAMES),
        })
        .collect();
    Some(DealtFit {
        read: records.len(),
        games,
        wins,
        prior,
        rows,
    })
}

/// A `DEALT_Q` row as `shadow_ban.rs` writes it.
pub fn dealt_row(card: &str, q: f64) -> String {
    format!("    ({card:?}, {q:.DECIMALS$}),")
}

/// The `DEALT_Q_PRIOR` line as `shadow_ban.rs` writes it.
pub fn prior_line(prior: f64) -> String {
    format!("pub const DEALT_Q_PRIOR: f64 = {prior:.DECIMALS$};")
}

/// For `UNBANNED_PREFER`: each entry of `prefer` in order with its refit row (`None` when the
/// candidate never held it), then the candidates: the cards `table` names no row for whose refit
/// `q` is above the prior and that `prefer` does not hold yet, best first.
pub fn prefer_rows(
    fit: &DealtFit,
    table: &[(&str, f64)],
    prefer: &[&str],
) -> (Vec<(String, Option<DealtRow>)>, Vec<DealtRow>) {
    let held = prefer
        .iter()
        .map(|card| {
            let row = fit.rows.iter().find(|row| row.card == *card).cloned();
            ((*card).to_owned(), row)
        })
        .collect();
    let mut candidates: Vec<DealtRow> = fit
        .rows
        .iter()
        .filter(|row| {
            row.q > fit.prior
                && !prefer.contains(&row.card.as_str())
                && !table.iter().any(|(card, _)| *card == row.card)
        })
        .cloned()
        .collect();
    candidates.sort_by(|a, b| b.q.total_cmp(&a.q).then_with(|| a.card.cmp(&b.card)));
    (held, candidates)
}

/// The refit as the command prints it: Markdown, its Rust blocks ready to paste.
pub fn render(fit: &DealtFit, table: &[(&str, f64)], prefer: &[&str]) -> String {
    let mut blocks: Vec<String> = Vec::new();
    blocks.push(format!(
        "# DEALT_Q refit\n{} self-vs-bin arena games of the {} records read (an id read twice counts \
         once); the candidate won {}.",
        fit.games, fit.read, fit.wins
    ));
    blocks.push(format!(
        "DEALT_Q_PRIOR, the candidate's share:\n```rust\n{}\n```",
        prior_line(fit.prior)
    ));
    let rows: Vec<String> = fit.rows.iter().map(|row| dealt_row(&row.card, row.q)).collect();
    blocks.push(format!(
        "DEALT_Q entries, {} cards by id, each (w + {SHRINK_GAMES} x prior) / (n + {SHRINK_GAMES}) over \
         the candidate's decks that held it:\n```rust\n{}\n```",
        rows.len(),
        rows.join("\n")
    ));

    let (held, candidates) = prefer_rows(fit, table, prefer);
    let mut advice = String::from(
        "## UNBANNED_PREFER\n\nA person's choice, on the sweep's numbers (the note above \
         UNBANNED_PREFER): its entries, then the candidates, cards DEALT_Q names no row for whose \
         refit q is above the prior. This picks nothing.\n\n| Card | Games | Won | q | |\n| --- | ---: | \
         ---: | ---: | --- |",
    );
    for (card, row) in &held {
        match row {
            Some(row) => advice.push_str(&format!(
                "\n| `{card}` | {} | {} | {:.DECIMALS$} | held |",
                row.games, row.wins, row.q
            )),
            None => advice.push_str(&format!("\n| `{card}` | 0 | 0 | — | held, never dealt |")),
        }
    }
    for row in &candidates {
        advice.push_str(&format!(
            "\n| `{}` | {} | {} | {:.DECIMALS$} | candidate |",
            row.card, row.games, row.wins, row.q
        ));
    }
    blocks.push(advice);
    blocks.join("\n\n") + "\n"
}

/// `cargo jackioh stats fit-dealt`.
pub fn run(args: &Args) -> Result<()> {
    let records = read_records(&args.files)?;
    let Some(fit) = fit(&records) else {
        bail!(
            "{} records read and none is a self-vs-bin arena game: DEALT_Q is fit on the candidate's \
             games against a parent build (`cargo jackioh promote`'s, or `arena --a self --b bin:<path>`)",
            records.len()
        );
    };
    print!("{}", render(&fit, DEALT_Q, UNBANNED_PREFER));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{self, AgentSpec, ArenaGame};
    use jackioh_ai::AI_SKILL_MAX;
    use jackioh_engine::PerPlayer;
    use serde_json::json;

    const BIN: &str = "dev:v0.4.0:arena:self-vs-bin-gen11:unban:abc";

    fn record(id: &str, winner: &str, p1_deck: &[&str], p2_deck: &[&str]) -> GameRecord {
        let seat = |deck: &[&str]| json!({ "deck": deck, "opening": [], "drawn": [], "played": [] });
        serde_json::from_value(json!({
            "id": id,
            "source": "dev",
            "mode": "random",
            "patch": "v0.4.0",
            "pilots": { "p1": "ai", "p2": "ai" },
            "game": {
                "first": "p1",
                "winner": winner,
                "reason": "hero-death",
                "turns": 9,
                "seats": { "p1": seat(p1_deck), "p2": seat(p2_deck) }
            }
        }))
        .expect("a well-formed game record")
    }

    fn arena_game(labels: (&str, &str), seed: &str, a_seat: PlayerId, paired: bool) -> ArenaGame {
        ArenaGame {
            n: 1,
            seed: seed.to_owned(),
            decks: (Vec::new(), Vec::new()),
            seats: PerPlayer::new(AgentSpec::SelfAi, AgentSpec::Bin("parent".into())),
            skills: PerPlayer::new(AI_SKILL_MAX, AI_SKILL_MAX),
            a_seat,
            labels: (labels.0.to_owned(), labels.1.to_owned()),
            paired,
            turn_actions: None,
        }
    }

    /// The fixture's records: four `self-vs-bin` games (the first counted twice), a gather game and an
    /// arena game against random.
    fn fixture() -> Vec<GameRecord> {
        let one = record(&format!("{BIN}:1"), "p1", &["card-a", "card-b"], &["card-c"]);
        vec![
            one.clone(),
            record(&format!("{BIN}:2"), "p2", &["card-a"], &["card-a", "card-c"]),
            record(&format!("{BIN}:3"), "p2", &["card-b", "card-b"], &["card-a"]),
            record(&format!("{BIN}:9:p2"), "p2", &["card-b"], &["card-c", "card-d"]),
            record("dev:v0.4.0:gather:1", "p1", &["card-a"], &[]),
            record("dev:v0.4.0:arena:self-vs-random:t:1", "p1", &["card-a"], &[]),
            one,
        ]
    }

    #[test]
    fn candidate_seat_is_agent_as_seat_in_a_self_vs_bin_record_id() {
        for game in [
            arena_game(("self", "bin-gen11"), "unban:abc:1", PlayerId::P1, false),
            arena_game(("self", "bin-gen11"), "unban:abc:2", PlayerId::P2, false),
            arena_game(("self", "bin-gen11"), "unban:abc:5", PlayerId::P2, true),
        ] {
            let id = arena::record_id("v0.4.0", &game);
            assert_eq!(
                candidate_seat(&record(&id, "p1", &[], &[])),
                Some(game.a_seat),
                "{id}"
            );
        }
        for (labels, seed) in [
            (("self", "random"), "t:1"),
            (("bin-gen11", "self"), "t:1"),
            (("self-s0.5", "bin-gen11"), "t:1"),
        ] {
            let id = arena::record_id("v0.4.0", &arena_game(labels, seed, PlayerId::P1, false));
            assert_eq!(candidate_seat(&record(&id, "p1", &[], &[])), None, "{id}");
        }
        for id in ["dev:v0.4.0:gather:7", "m1"] {
            assert_eq!(candidate_seat(&record(id, "p1", &[], &[])), None, "{id}");
        }
        // The id's patch is the record's own.
        let other = arena::record_id(
            "v0.3.9",
            &arena_game(("self", "bin-gen11"), "t:1", PlayerId::P1, false),
        );
        assert_eq!(candidate_seat(&record(&other, "p1", &[], &[])), None, "{other}");
    }

    #[test]
    fn fit_counts_the_candidates_decks_and_shrinks_toward_its_share() {
        let records = fixture();
        let fit_of = fit(&records).expect("self-vs-bin games");
        assert_eq!(
            (fit_of.read, fit_of.games, fit_of.wins, fit_of.prior),
            (7, 4, 3, 0.75)
        );
        let counts: Vec<(&str, i64, i64)> = fit_of
            .rows
            .iter()
            .map(|row| (row.card.as_str(), row.games, row.wins))
            .collect();
        assert_eq!(
            counts,
            [
                ("card-a", 2, 2),
                ("card-b", 2, 1),
                ("card-c", 2, 2),
                ("card-d", 1, 1)
            ]
        );
        let lines: Vec<String> = fit_of
            .rows
            .iter()
            .map(|row| dealt_row(&row.card, row.q))
            .collect();
        assert_eq!(
            lines,
            [
                "    (\"card-a\", 0.7581),",
                "    (\"card-b\", 0.7419),",
                "    (\"card-c\", 0.7581),",
                "    (\"card-d\", 0.7541),",
            ]
        );
        assert_eq!(prior_line(fit_of.prior), "pub const DEALT_Q_PRIOR: f64 = 0.7500;");

        let mut reversed = records.clone();
        reversed.reverse();
        assert_eq!(fit(&reversed).expect("self-vs-bin games"), fit_of);
        assert_eq!(fit(&records[4..6]), None);
    }

    #[test]
    fn rows_and_prior_print_as_shadow_ban_rs_writes_them() {
        let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../ai/src/shadow_ban.rs"));
        let start = "pub const DEALT_Q: &[(&str, f64)] = &[\n";
        let from = source.find(start).expect("the table opens") + start.len();
        let table = &source[from..from + source[from..].find("];").expect("the table closes")];
        let printed: String = jackioh_ai::DEALT_Q
            .iter()
            .map(|(card, q)| dealt_row(card, *q) + "\n")
            .collect();
        assert_eq!(table, printed);
        assert!(source.contains(&prior_line(jackioh_ai::DEALT_Q_PRIOR)));
    }

    #[test]
    fn unbanned_prefer_lists_held_entries_and_unpriced_cards_above_the_prior() {
        let fit_of = fit(&fixture()).expect("self-vs-bin games");
        let table = [("card-a", 0.7)];
        let (held, candidates) = prefer_rows(&fit_of, &table, &["card-b", "card-z"]);
        let held_cards: Vec<(&str, bool)> = held
            .iter()
            .map(|(card, row)| (card.as_str(), row.is_some()))
            .collect();
        assert_eq!(held_cards, [("card-b", true), ("card-z", false)]);
        assert_eq!(held[0].1.as_ref().map(|row| row.games), Some(2));
        let named: Vec<&str> = candidates.iter().map(|row| row.card.as_str()).collect();
        assert_eq!(named, ["card-c", "card-d"]);

        let text = render(&fit_of, &table, &["card-b", "card-z"]);
        assert!(
            text.contains("```rust\npub const DEALT_Q_PRIOR: f64 = 0.7500;\n```"),
            "{text}"
        );
        assert!(
            text.contains(
                "```rust\n    (\"card-a\", 0.7581),\n    (\"card-b\", 0.7419),\n    (\"card-c\", 0.7581),\n    \
                 (\"card-d\", 0.7541),\n```"
            ),
            "{text}"
        );
        assert!(text.contains("| `card-b` | 2 | 1 | 0.7419 | held |"), "{text}");
        assert!(
            text.contains("| `card-z` | 0 | 0 | — | held, never dealt |"),
            "{text}"
        );
        assert!(
            text.contains("| `card-c` | 2 | 2 | 0.7581 | candidate |"),
            "{text}"
        );
        assert!(!text.contains("| `card-a` |"), "{text}");
        assert!(text.ends_with("\n") && !text.ends_with("\n\n"), "{text:?}");
    }

    #[test]
    fn fit_over_files_is_the_same_whatever_order_they_are_named_in() {
        let dir = std::env::temp_dir().join(format!("jackioh-fit-dealt-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let lines = |records: &[GameRecord]| -> String {
            records
                .iter()
                .map(|record| serde_json::to_string(record).unwrap() + "\n")
                .collect()
        };
        let records = fixture();
        let (a, b) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
        std::fs::write(&a, lines(&records[..3])).unwrap();
        std::fs::write(&b, lines(&records[3..])).unwrap();

        let print = |paths: &[PathBuf]| {
            let fit_of = fit(&read_records(paths).unwrap()).expect("self-vs-bin games");
            render(&fit_of, DEALT_Q, UNBANNED_PREFER)
        };
        let forward = print(&[a.clone(), b.clone()]);
        assert_eq!(forward, print(&[b, a]));
        assert!(
            forward.contains("4 self-vs-bin arena games of the 7 records read"),
            "{forward}"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
