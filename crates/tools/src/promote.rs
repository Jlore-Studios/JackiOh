//! `cargo jackioh promote`: the promotion gate of the AI training lanes (SURFACE §14.2;
//! docs/v0.3.0/README.md §8; training/README.md).
//!
//! ```text
//! cargo jackioh promote --lane improve|unban --parent-bin <path to main's jackioh> [--dry-run] [--verify]
//! ```
//!
//! The candidate is this build's AI (`self`). It plays `TRAINING_GAMES` games against SPEC §10.7's
//! random policy (`jackioh_ai::random_action`) and `TRAINING_GAMES` against the parent, the AI on
//! `main` (`bin:<parent-bin>`, SURFACE §14.1), seats alternating (the candidate sits p1 on odd
//! games), on the seeds `"<lane>:<tree>:<k>"` where `<tree>` is `git rev-parse HEAD:crates/ai/src`, so
//! committing `generation.json` (or the history line) does not change them, and the same seeds
//! against both opponents. Each seat's deck is `build_ai_deck` over all three sets minus its own AI's
//! shadow ban (R186), with no handicap (Easy both, R180). The games run on rayon's pool
//! (`RAYON_NUM_THREADS` is respected); no result depends on the thread count.
//!
//! It promotes when:
//!
//! | Lane      | vs random                              | vs parent                              | shadow bans                      |
//! |-----------|----------------------------------------|----------------------------------------|----------------------------------|
//! | `improve` | ≥ `TRAINING_IMPROVE.vs_random` (90)    | ≥ `TRAINING_IMPROVE.vs_parent` (85)    | —                                |
//! | `unban`   | ≥ `TRAINING_UNBAN.vs_random` (90)      | ≥ `TRAINING_UNBAN.vs_parent` (75)      | strictly fewer than the parent's |
//!
//! A draw is not a win, and neither is a game without a result. Without `--dry-run` or `--verify`, a
//! pass writes `crates/ai/generation.json` and appends the same object to
//! `training/history/<lane>.jsonl`. `--verify` (CI) recomputes and compares with the branch's
//! `generation.json`. The report (Markdown, the lane's pull request body) goes to stdout. Exit 0 on a
//! pass, 1 on a failed gate.
//!
//! Because the seeds come from the committed AI, a promotion (and a verification) refuses to run on
//! a `crates/ai/src` with uncommitted changes: they would be measured on another AI's seeds. A dry
//! run measures them anyway, on HEAD's seeds, and says so.

use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use jackioh_engine::{GameResult, PlayerId, TRAINING_GAMES, TRAINING_IMPROVE, TRAINING_UNBAN, TrainingGate};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::arena::{self, AgentSpec, ArenaOutcome, Entrant};
use crate::patches::{git, repo_root};

/// The AI's source, whose git tree names the seeds.
pub(crate) const AI_SOURCE: &str = "crates/ai/src";

/// The main AI's generation record (SURFACE §14.3), relative to the repository root.
pub(crate) const GENERATION_PATH: &str = "crates/ai/generation.json";

/// The fields `--verify` holds a branch's `generation.json` to. The date is when the lane ran, and
/// the parent commit is the `main` it ran against, which may have moved on since without its AI
/// moving; every number and every name of what was measured must match.
const VERIFIED_FIELDS: &[&str] = &[
    "generation",
    "lane",
    "tree",
    "vsRandom",
    "vsParent",
    "shadowBan",
    "parentShadowBan",
];

/// The two training lanes (docs/v0.3.0/README.md §8).
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    /// Beat the parent: ≥ 85 of 100 against it and ≥ 90 of 100 against random.
    Improve,
    /// Play well with fewer shadow bans: strictly fewer than the parent's, ≥ 75 of 100 against it and
    /// ≥ 90 of 100 against random.
    Unban,
}

impl Lane {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Lane::Improve => "improve",
            Lane::Unban => "unban",
        }
    }

    /// The lane's gate (CLAUDE.md rule 9: the numbers live in `crates/engine/src/config.rs`).
    pub(crate) fn gate(self) -> TrainingGate {
        match self {
            Lane::Improve => TRAINING_IMPROVE,
            Lane::Unban => TRAINING_UNBAN,
        }
    }
}

#[derive(clap::Args)]
pub struct Args {
    /// The lane whose gate to apply.
    #[arg(long, value_enum)]
    pub lane: Lane,
    /// The parent: a `jackioh` binary built from `main` (training/loop.sh keeps it at ~/parent-jackioh).
    #[arg(long)]
    pub parent_bin: PathBuf,
    /// Measure and report; write nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// Recompute and compare with the branch's `crates/ai/generation.json`; write nothing (CI).
    #[arg(long)]
    pub verify: bool,
}

/// What a promotion run counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GateCounts {
    pub games: i32,
    pub vs_random: i32,
    pub vs_parent: i32,
    pub shadow_ban: usize,
    pub parent_shadow_ban: usize,
}

/// The rules of the lane's gate that `counts` fails, in the table's order; empty when it promotes.
pub(crate) fn gate_failures(lane: Lane, counts: &GateCounts) -> Vec<String> {
    let gate = lane.gate();
    let mut failures = Vec::new();
    if counts.vs_random < gate.vs_random {
        failures.push(format!(
            "vs random: {}/{} wins, {} needed",
            counts.vs_random, counts.games, gate.vs_random
        ));
    }
    if counts.vs_parent < gate.vs_parent {
        failures.push(format!(
            "vs parent: {}/{} wins, {} needed",
            counts.vs_parent, counts.games, gate.vs_parent
        ));
    }
    if lane == Lane::Unban && counts.shadow_ban >= counts.parent_shadow_ban {
        failures.push(format!(
            "shadow bans: {}, which is not strictly fewer than the parent's {}",
            counts.shadow_ban, counts.parent_shadow_ban
        ));
    }
    failures
}

/// The candidate's wins over games given as (result, the candidate's seat). A draw is not a win, and
/// a game without a result (the action ceiling, a controller that threw) is nobody's.
pub(crate) fn wins_of(games: &[(Option<GameResult>, PlayerId)]) -> i32 {
    games
        .iter()
        .filter(|(result, seat)| arena::won_by(*result, *seat))
        .count() as i32
}

/// The (result, candidate seat) pairs of a series in which the candidate is agent `a`.
fn results_of(outcomes: &[ArenaOutcome]) -> Vec<(Option<GameResult>, PlayerId)> {
    outcomes
        .iter()
        .map(|outcome| (outcome.result, outcome.game.a_seat))
        .collect()
}

/// Game `k`'s seed: `"<lane>:<tree>:<k>"`.
pub(crate) fn game_seed(lane: Lane, tree: &str, k: i32) -> String {
    format!("{}:{tree}:{k}", lane.as_str())
}

/// SURFACE §14.2's `<tree>`: `git rev-parse HEAD:crates/ai/src`.
pub(crate) fn ai_tree(repo: &Path) -> anyhow::Result<String> {
    Ok(git(repo, &["rev-parse", &format!("HEAD:{AI_SOURCE}")], &[])?
        .trim()
        .to_string())
}

/// Whether `crates/ai/src` differs from HEAD (staged, unstaged or untracked).
fn ai_source_dirty(repo: &Path) -> anyhow::Result<bool> {
    Ok(!git(repo, &["status", "--porcelain", "--", AI_SOURCE], &[])?
        .trim()
        .is_empty())
}

/// The `main` commit the parent is: the branch's merge base with `origin/main` (a lane's branch is
/// cut from it and rebased on it), else with `main`; `None` when neither is known.
fn parent_commit(repo: &Path) -> Option<String> {
    ["origin/main", "main"].into_iter().find_map(|base| {
        git(repo, &["merge-base", "HEAD", base], &[])
            .ok()
            .map(|commit| commit.trim().to_string())
    })
}

/// A promotion's record: `crates/ai/generation.json` and its history line, in SURFACE §14.2's key order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenerationRecord {
    pub generation: i64,
    pub lane: String,
    /// The `main` commit the parent was built from.
    pub parent: Option<String>,
    /// `git rev-parse HEAD:crates/ai/src`, the seeds' name.
    pub tree: String,
    /// `"<wins>/<games>"`.
    pub vs_random: String,
    pub vs_parent: String,
    pub shadow_ban: usize,
    pub parent_shadow_ban: usize,
    /// The UTC date of the run, `YYYY-MM-DD`.
    pub date: String,
}

/// The fields of `VERIFIED_FIELDS` on which a branch's `generation.json` and a recomputed record
/// disagree, each as one line.
pub(crate) fn verify_mismatches(claimed: &Value, measured: &GenerationRecord) -> Vec<String> {
    let measured = serde_json::to_value(measured).unwrap_or(Value::Null);
    VERIFIED_FIELDS
        .iter()
        .filter_map(|key| {
            let said = claimed.get(*key).unwrap_or(&Value::Null);
            let found = measured.get(*key).unwrap_or(&Value::Null);
            (said != found).then(|| format!("{key}: generation.json says {said}, the gate measured {found}"))
        })
        .collect()
}

/// Appends the record to the lane's history. A last line of the same generation and lane (the
/// promotion run again, say after a rebase) is replaced, so one generation is one line.
fn write_history(path: &Path, record: &GenerationRecord) -> anyhow::Result<()> {
    let line = serde_json::to_string(record)?;
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<&str> = existing.lines().filter(|line| !line.trim().is_empty()).collect();
    if let Some(last) = lines.last()
        && let Ok(previous) = serde_json::from_str::<Value>(last)
        && previous.get("generation") == Some(&json!(record.generation))
        && previous.get("lane") == Some(&json!(record.lane))
    {
        lines.pop();
    }
    lines.push(&line);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("promote: cannot create {}", dir.display()))?;
    }
    std::fs::write(path, format!("{}\n", lines.join("\n")))
        .with_context(|| format!("promote: cannot write {}", path.display()))
}

/// Writes `crates/ai/generation.json`, pretty-printed as generation 0's was.
fn write_generation(path: &Path, record: &GenerationRecord) -> anyhow::Result<()> {
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(record)?))
        .with_context(|| format!("promote: cannot write {}", path.display()))
}

/// Draws and games without a result in a series, for the report.
fn draws_and_aborted(outcomes: &[ArenaOutcome]) -> (usize, usize) {
    (
        outcomes.iter().filter(|outcome| outcome.drawn()).count(),
        outcomes.iter().filter(|outcome| outcome.result.is_none()).count(),
    )
}

/// The run's report, in Markdown: the lane's pull request body (training/loop.sh).
#[allow(clippy::too_many_arguments)]
fn report(
    lane: Lane,
    mode: &str,
    record: &GenerationRecord,
    counts: &GateCounts,
    parent: &Entrant,
    vs_random: &[ArenaOutcome],
    vs_parent: &[ArenaOutcome],
    failures: &[String],
) -> anyhow::Result<String> {
    let gate = lane.gate();
    let verdict = match (failures.is_empty(), mode) {
        (true, "promotion") => "promoted",
        (true, _) => "passes",
        (false, _) => "does not pass",
    };
    let (random_draws, random_aborted) = draws_and_aborted(vs_random);
    let (parent_draws, parent_aborted) = draws_and_aborted(vs_parent);
    let parent_name = match (&parent.lane, &record.parent) {
        (Some(parent_lane), Some(commit)) => format!(
            "gen {} ({parent_lane}), `main` at `{commit}`",
            parent.generation.unwrap_or_default()
        ),
        (Some(parent_lane), None) => format!("gen {} ({parent_lane})", parent.generation.unwrap_or_default()),
        _ => parent.label.clone(),
    };
    let first = game_seed(lane, &record.tree, 1);
    let last = game_seed(lane, &record.tree, counts.games);
    let mut out = String::new();
    out.push_str(&format!(
        "## AI gen {} ({}): {verdict} ({mode})\n\n",
        record.generation,
        lane.as_str()
    ));
    out.push_str("| Opponent | Wins | Needs | Draws | No result |\n|---|---|---|---|---|\n");
    out.push_str(&format!(
        "| random (SPEC §10.7's policy) | {} | {} | {random_draws} | {random_aborted} |\n",
        record.vs_random, gate.vs_random
    ));
    out.push_str(&format!(
        "| parent: {parent_name} | {} | {} | {parent_draws} | {parent_aborted} |\n\n",
        record.vs_parent, gate.vs_parent
    ));
    let ban_rule = match lane {
        Lane::Unban => "; the unban lane needs strictly fewer than the parent's",
        Lane::Improve => "",
    };
    out.push_str(&format!(
        "Shadow bans: {} (parent {}){ban_rule}.\n\n",
        record.shadow_ban, record.parent_shadow_ban
    ));
    out.push_str(&format!(
        "Seeds `{first}` to `{last}`, the same against both opponents; the candidate sits p1 on odd \
         games; each seat's deck is built from all three sets minus its own AI's shadow ban, with no \
         handicap. A draw is not a win, nor is a game without a result.\n\n"
    ));
    if !failures.is_empty() {
        out.push_str("Fails:\n\n");
        for failure in failures {
            out.push_str(&format!("- {failure}\n"));
        }
        out.push('\n');
    }
    out.push_str(&format!("```json\n{}\n```\n", serde_json::to_string(record)?));
    Ok(out)
}

pub fn run(args: Args) -> anyhow::Result<()> {
    jackioh_cards::register_all();
    let repo = repo_root();
    let mode = if args.verify {
        "verify"
    } else if args.dry_run {
        "dry run"
    } else {
        "promotion"
    };

    if ai_source_dirty(&repo)? {
        if mode != "dry run" {
            bail!(
                "promote: {AI_SOURCE} has uncommitted changes. The gate's seeds come from `git rev-parse \
                 HEAD:{AI_SOURCE}`, so commit the change first, then run the {mode} again (training/README.md)."
            );
        }
        eprintln!(
            "promote: {AI_SOURCE} has uncommitted changes; this dry run measures them on HEAD's seeds, and \
             the promotion will play other seeds once they are committed"
        );
    }

    let tree = ai_tree(&repo)?;
    let candidate = arena::entrant(&AgentSpec::SelfAi)?;
    let random = arena::entrant(&AgentSpec::Random)?;
    let parent = arena::entrant(&AgentSpec::Bin(args.parent_bin.clone()))?;
    let parent_generation = parent
        .generation
        .with_context(|| format!("promote: {} reported no generation", args.parent_bin.display()))?;

    let seed_of = |k: i32| game_seed(args.lane, &tree, k);
    let mut games = arena::setups(&candidate, &random, TRAINING_GAMES, seed_of)?;
    games.extend(arena::setups(&candidate, &parent, TRAINING_GAMES, seed_of)?);
    eprintln!(
        "promote: {} lane, {TRAINING_GAMES} games against random and {TRAINING_GAMES} against gen \
         {parent_generation}, on seeds {} to {}",
        args.lane.as_str(),
        game_seed(args.lane, &tree, 1),
        game_seed(args.lane, &tree, TRAINING_GAMES)
    );
    let outcomes = arena::play_games(&games)?;
    if let Some(path) = arena::records_path(None) {
        arena::append_records(&path, &arena::records_of(&outcomes)?)?;
    }
    let (vs_random, vs_parent) = outcomes.split_at(TRAINING_GAMES as usize);

    let counts = GateCounts {
        games: TRAINING_GAMES,
        vs_random: wins_of(&results_of(vs_random)),
        vs_parent: wins_of(&results_of(vs_parent)),
        shadow_ban: candidate.shadow_ban.len(),
        parent_shadow_ban: parent.shadow_ban.len(),
    };
    let record = GenerationRecord {
        generation: parent_generation + 1,
        lane: args.lane.as_str().to_string(),
        parent: parent_commit(&repo),
        tree: tree.clone(),
        vs_random: format!("{}/{}", counts.vs_random, counts.games),
        vs_parent: format!("{}/{}", counts.vs_parent, counts.games),
        shadow_ban: counts.shadow_ban,
        parent_shadow_ban: counts.parent_shadow_ban,
        date: arena::utc_date(),
    };
    let failures = gate_failures(args.lane, &counts);
    print!(
        "{}",
        report(
            args.lane, mode, &record, &counts, &parent, vs_random, vs_parent, &failures
        )?
    );

    if args.verify {
        let path = repo.join(GENERATION_PATH);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("promote: cannot read {}", path.display()))?;
        let claimed: Value = serde_json::from_str(&text)
            .with_context(|| format!("promote: {} is not JSON", path.display()))?;
        let mismatches = verify_mismatches(&claimed, &record);
        if !mismatches.is_empty() {
            println!("\nVerification failed:\n");
            for mismatch in &mismatches {
                println!("- {mismatch}");
            }
            bail!("promote: {GENERATION_PATH} does not match the gate's measurement");
        }
        if claimed.get("parent") != Some(&json!(record.parent)) {
            eprintln!(
                "promote: note: {GENERATION_PATH} names parent {}, and the parent is now {}; its AI measured the same",
                claimed.get("parent").unwrap_or(&Value::Null),
                json!(record.parent)
            );
        }
    }
    if !failures.is_empty() {
        bail!(
            "promote: the {} gate is not passed: {}",
            args.lane.as_str(),
            failures.join("; ")
        );
    }
    if mode == "promotion" {
        write_generation(&repo.join(GENERATION_PATH), &record)?;
        let history = repo.join(format!("training/history/{}.jsonl", args.lane.as_str()));
        write_history(&history, &record)?;
        eprintln!(
            "promote: wrote {GENERATION_PATH} and {}; commit them with your change as `AI gen {} ({}): <what changed>`",
            history.display(),
            record.generation,
            args.lane.as_str()
        );
    }
    Ok(())
}

/// docs/v0.3.0/README.md V26: `promote` promotes exactly by §8's table and writes `generation.json`
/// and the history line.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::{GameOverReason, Winner};
    use std::process::Command;

    fn counts(vs_random: i32, vs_parent: i32, shadow_ban: usize, parent_shadow_ban: usize) -> GateCounts {
        GateCounts {
            games: TRAINING_GAMES,
            vs_random,
            vs_parent,
            shadow_ban,
            parent_shadow_ban,
        }
    }

    #[test]
    fn the_improve_gate_needs_90_against_random_and_85_against_the_parent() {
        assert!(gate_failures(Lane::Improve, &counts(90, 85, 11, 11)).is_empty());
        assert!(gate_failures(Lane::Improve, &counts(100, 100, 11, 11)).is_empty());
        assert_eq!(gate_failures(Lane::Improve, &counts(89, 85, 11, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Improve, &counts(90, 84, 11, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Improve, &counts(89, 84, 11, 11)).len(), 2);
        // The improve lane has no shadow-ban rule: more bans than the parent's still promote.
        assert!(gate_failures(Lane::Improve, &counts(90, 85, 12, 11)).is_empty());
        assert_eq!(TRAINING_IMPROVE.vs_random, 90);
        assert_eq!(TRAINING_IMPROVE.vs_parent, 85);
    }

    #[test]
    fn the_unban_gate_needs_strictly_fewer_bans_90_against_random_and_75_against_the_parent() {
        assert!(gate_failures(Lane::Unban, &counts(90, 75, 10, 11)).is_empty());
        assert!(gate_failures(Lane::Unban, &counts(95, 80, 0, 11)).is_empty());
        // Strictly fewer: as many bans as the parent is a failure, more is too.
        assert_eq!(gate_failures(Lane::Unban, &counts(90, 75, 11, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &counts(90, 75, 12, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &counts(90, 74, 10, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &counts(89, 75, 10, 11)).len(), 1);
        assert_eq!(gate_failures(Lane::Unban, &counts(89, 74, 11, 11)).len(), 3);
        // 75 against the parent passes the unban lane and fails the improve lane.
        assert!(!gate_failures(Lane::Improve, &counts(90, 75, 10, 11)).is_empty());
        assert_eq!(TRAINING_UNBAN.vs_random, 90);
        assert_eq!(TRAINING_UNBAN.vs_parent, 75);
    }

    #[test]
    fn draws_and_games_without_a_result_count_as_losses() {
        let result = |winner| {
            Some(GameResult {
                winner,
                reason: GameOverReason::HeroDeath,
            })
        };
        let draw = Some(GameResult {
            winner: Winner::Draw,
            reason: GameOverReason::TurnCap,
        });
        let games = vec![
            (result(Winner::P1), PlayerId::P1),
            (draw, PlayerId::P1),
            (draw, PlayerId::P2),
            (None, PlayerId::P2),
            (result(Winner::P2), PlayerId::P1),
            (result(Winner::P2), PlayerId::P2),
        ];
        assert_eq!(wins_of(&games), 2);

        // 89 wins and 11 draws against random is a failed gate: the draws are not wins.
        let mut series: Vec<(Option<GameResult>, PlayerId)> =
            (0..89).map(|_| (result(Winner::P1), PlayerId::P1)).collect();
        series.extend((0..11).map(|_| (draw, PlayerId::P1)));
        let measured = counts(wins_of(&series), 85, 11, 11);
        assert_eq!(measured.vs_random, 89);
        assert_eq!(gate_failures(Lane::Improve, &measured).len(), 1);
    }

    #[test]
    fn seeds_name_the_lane_the_tree_and_the_game() {
        assert_eq!(game_seed(Lane::Improve, "4b825dc", 7), "improve:4b825dc:7");
        assert_eq!(game_seed(Lane::Unban, "4b825dc", 100), "unban:4b825dc:100");
    }

    /// Runs git in `dir` for a test, with an identity and no signing, panicking on failure.
    fn git_in(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.name=JackiOh tests",
                "-c",
                "user.email=tests@jackioh.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    #[test]
    fn the_seeds_are_stable_across_a_generation_json_only_commit() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("jackioh-promote-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(dir.join(AI_SOURCE)).unwrap();
        git_in(&dir, &["init", "-q"]);
        std::fs::write(dir.join(AI_SOURCE).join("lib.rs"), "pub fn decide() {}\n").unwrap();
        std::fs::write(dir.join(GENERATION_PATH), "{\"generation\": 0}\n").unwrap();
        git_in(&dir, &["add", "-A"]);
        git_in(&dir, &["commit", "-q", "-m", "AI source"]);
        let tree = ai_tree(&dir).unwrap();
        assert!(!ai_source_dirty(&dir).unwrap());

        // A promotion commits generation.json and the history line: the seeds do not move.
        std::fs::write(dir.join(GENERATION_PATH), "{\"generation\": 1}\n").unwrap();
        std::fs::create_dir_all(dir.join("training/history")).unwrap();
        std::fs::write(dir.join("training/history/improve.jsonl"), "{\"generation\":1}\n").unwrap();
        git_in(&dir, &["add", "-A"]);
        git_in(&dir, &["commit", "-q", "-m", "AI gen 1 (improve): record"]);
        assert_eq!(ai_tree(&dir).unwrap(), tree);
        assert_eq!(
            game_seed(Lane::Improve, &ai_tree(&dir).unwrap(), 1),
            game_seed(Lane::Improve, &tree, 1)
        );

        // A change to the AI's source is another AI: other seeds, and dirty until it is committed.
        std::fs::write(
            dir.join(AI_SOURCE).join("lib.rs"),
            "pub fn decide() { let _ = 1; }\n",
        )
        .unwrap();
        assert!(ai_source_dirty(&dir).unwrap());
        assert_eq!(ai_tree(&dir).unwrap(), tree);
        git_in(&dir, &["commit", "-q", "-am", "AI change"]);
        assert_ne!(ai_tree(&dir).unwrap(), tree);

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn sample_record() -> GenerationRecord {
        GenerationRecord {
            generation: 8,
            lane: "improve".to_string(),
            parent: Some("0123abc".to_string()),
            tree: "4b825dc".to_string(),
            vs_random: "93/100".to_string(),
            vs_parent: "87/100".to_string(),
            shadow_ban: 11,
            parent_shadow_ban: 11,
            date: "2026-11-02".to_string(),
        }
    }

    #[test]
    fn the_record_serialises_in_surface_order() {
        assert_eq!(
            serde_json::to_string(&sample_record()).unwrap(),
            r#"{"generation":8,"lane":"improve","parent":"0123abc","tree":"4b825dc","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}"#
        );
    }

    #[test]
    fn verify_ignores_the_date_and_the_parent_commit_and_nothing_else() {
        let measured = sample_record();
        let mut claimed = serde_json::to_value(&measured).unwrap();
        claimed["date"] = json!("2026-10-30");
        claimed["parent"] = json!("fedcba9");
        assert!(verify_mismatches(&claimed, &measured).is_empty());

        claimed["vsParent"] = json!("88/100");
        claimed["generation"] = json!(9);
        let mismatches = verify_mismatches(&claimed, &measured);
        assert_eq!(mismatches.len(), 2);
        assert!(mismatches[0].starts_with("generation:"));
        assert!(mismatches[1].starts_with("vsParent:"));

        // Generation 0's record (the port) never verifies as a promotion.
        let port: Value = serde_json::from_str(include_str!("../../ai/generation.json")).unwrap();
        assert_eq!(port["generation"], json!(0));
        assert_eq!(port["lane"], json!("port"));
        assert!(!verify_mismatches(&port, &measured).is_empty());
    }

    #[test]
    fn the_history_keeps_one_line_per_generation() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("jackioh-history-{}-{stamp}", std::process::id()));
        let path = dir.join("training/history/improve.jsonl");
        let mut record = sample_record();
        write_history(&path, &record).unwrap();
        record.vs_parent = "86/100".to_string();
        write_history(&path, &record).unwrap();
        record.generation = 9;
        write_history(&path, &record).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("\"vsParent\":\"86/100\""));
        assert!(lines[1].starts_with("{\"generation\":9,"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
