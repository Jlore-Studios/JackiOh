//! `cargo jackioh classify --base <rev> [--head <rev>] [--parent-bin <path>] [--json]` (#1365,
//! R1365000): what a change between two commits does to the game, read from its effect rather than
//! its paths, so a patch number moves exactly when the cards do (#1360). It names a **kind**, the
//! highest the change reaches of Cards, AI, App and Internal, and for Cards a **tier** (major, patch
//! or micro) and the card families that play differently. Nothing overrides it: no flag, title or
//! label.
//!
//! Stage 1 reads the diff. `catalog.json` card by card: the wording fields (`WORDING_FIELDS`) a
//! player reads and no game does, every other field plays. Each card script's code, the lines
//! `catalog loc` counts, so its tests, comments, `use` lines and layout are not code. The sets
//! `SHIPPED_SETS` names on each side: a set that ships is a major, and a card of a set that has not
//! shipped is no change (R1420). And the paths of everything else (`area`).
//!
//! Stage 2 (`--parent-bin`, the base's build, with this binary built from the head) asks the base
//! build what the diff cannot say:
//! - a card claimed only through its script keeps the claim only when its forced games diverge
//!   between the builds (`luau diff`'s check), so a refactor claims nothing;
//! - a change to code a card's play can rest on (`Area::PlayCode`) replays `UNFORCED_GAMES` unforced
//!   games, and the cards each divergence touches play differently; a divergence that touches no
//!   card is a rule every game meets, and makes a patch;
//! - a change to the AI plays the same self-play games on both builds: identical logs mean the AI
//!   decides as it did, and the change is Internal.
//!
//! It stops once `PATCH_MIN_CARDS` families play differently, the tier being a patch already.
//! Without `--parent-bin`, stage 1's answer stands, and the output names what stage 2 would have
//! checked: script claims count unconfirmed, a change to play code counts as none.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

use anyhow::{Context, Result, bail};
use jackioh_cards::register_all;
use jackioh_engine::wire::SetName;
use jackioh_engine::{GameState, find_instance};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::code_lines;
use crate::golden::{record_seeds, text_at};
use crate::luau::{parent_check, temp_traces};
use crate::patches::patches_io::{Catalog, ChangeKind, diff_catalogs};
use crate::patches::{git, repo_root};

/// R1365000: the card families that must play differently for a patch; fewer is a micro. The
/// engine's, since a card's stats window reads it too (R1367000).
pub(crate) use jackioh_engine::config::PATCH_MIN_CARDS;

/// R1365000: the catalog fields a player reads and no game does. Every other field plays. The cards
/// crate's, since a card's stats window reads it too (R1367000). A `flavor` (R1437000) never
/// reaches this list, since `changed_fields` leaves it out.
pub(crate) use jackioh_cards::WORDING_FIELDS;

/// The AI crate's own tooling, which plays no decision (gates, the sweep, development runs).
const AI_TOOLING: &[&str] = &["gate.rs", "sweep.rs", "dev_run.rs", "baselines.rs"];

/// Stage 2: forced games per card a script change claims. A refactor diverges in none of them.
const FORCED_GAMES: u32 = 40;

/// Stage 2: the unforced games a change to play code replays, `luau diff`'s count.
const UNFORCED_GAMES: u32 = 200;

/// Stage 2: the self-play games a change to the AI plays on both builds, and their seed base.
const SELFPLAY_GAMES: u32 = 8;
const SELFPLAY_SEED: &str = "classify";

const CATALOG_PATH: &str = "crates/cards/catalog.json";
const SETS_PATH: &str = "crates/engine/src/wire/catalog_types.rs";

#[derive(clap::Args, Debug)]
pub struct Args {
    /// The commit the change starts from (a pull request's base).
    #[arg(long)]
    base: String,
    /// The commit the change ends at; stage 2 needs this binary built from it.
    #[arg(long, default_value = "HEAD")]
    head: String,
    /// The base's own `jackioh`, for stage 2. Without it, stage 1's answer stands.
    #[arg(long, value_name = "PATH")]
    parent_bin: Option<PathBuf>,
    /// Print the classification as one JSON object.
    #[arg(long)]
    json: bool,
    /// Hold the head's pending fragments to what the change does (R1370000): fail when a card plays
    /// differently and no fragment claims it, or a fragment claims a play or a rule the base build
    /// does not show. Needs `--parent-bin` whenever stage 1 leaves something unchecked.
    #[arg(long)]
    claims: bool,
}

/// `--json --claims`: the classification, the labels CI puts on the pull request, and what the
/// fragments' claims get wrong.
#[derive(Serialize)]
struct Report<'a> {
    #[serde(flatten)]
    classification: &'a Classification,
    labels: Vec<String>,
    problems: Vec<String>,
}

/// R1370000: the labels a pull request carries for its kind and its tier (`kind:cards`,
/// `tier:micro`), as `bot/harness/config.py`'s `LABELS` lists them.
pub(crate) fn labels(classification: &Classification) -> Vec<String> {
    let mut labels = vec![format!("kind:{}", kind_name(classification.kind))];
    if let Some(tier) = classification.tier {
        labels.push(format!("tier:{}", format!("{tier:?}").to_lowercase()));
    }
    labels
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Internal => "internal",
        Kind::App => "app",
        Kind::Ai => "ai",
        Kind::Cards => "cards",
    }
}

/// What a change reaches, lowest first, so the highest of several is their `max`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Internal,
    App,
    Ai,
    Cards,
}

/// A card change's tier, lowest first. `patches ship` names a version from it and records it on the
/// patch (R1366000), so it reads back too.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Tier {
    Micro,
    Patch,
    Major,
}

/// Where a path's change lands when no card names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Area {
    /// `catalog.json`, read card by card.
    Catalog,
    /// One card's script, claimed by its `pub const ID`.
    Script,
    /// Code a card's play can rest on: the engine outside its wire types and view, and the cards
    /// crate outside the scripts (shared helpers, the compiled data, the build script).
    PlayCode,
    Ai,
    App,
    Internal,
}

fn is_test(path: &str) -> bool {
    path.contains("/tests/")
        || path.contains("/testkit/")
        || path.starts_with("e2e/")
        || path.ends_with(".test.ts")
        || path.ends_with(".test.tsx")
}

/// R1365000: where a changed path lands.
fn area(path: &str) -> Area {
    if path == CATALOG_PATH {
        return Area::Catalog;
    }
    if is_test(path) || path.ends_with(".md") {
        return Area::Internal;
    }
    if path.starts_with("crates/cards/src/scripts/") && path.ends_with(".rs") {
        return Area::Script;
    }
    if path.starts_with("crates/engine/src/wire/") || path.starts_with("crates/engine/src/view") {
        return Area::App;
    }
    if path.starts_with("crates/engine/src/")
        || path.starts_with("crates/cards/src/")
        || path.starts_with("crates/cards/data/")
        || path == "crates/cards/build.rs"
    {
        return Area::PlayCode;
    }
    if path == "crates/ai/generation.json" {
        return Area::Ai;
    }
    if let Some(file) = path.strip_prefix("crates/ai/src/") {
        return if AI_TOOLING.contains(&file) {
            Area::Internal
        } else {
            Area::Ai
        };
    }
    let app = [
        "apps/web/",
        "crates/wasm/",
        "crates/server/",
        "crates/luau/",
        "crates/cards/patches/",
        "crates/cards/luau/",
    ];
    if app.iter().any(|prefix| path.starts_with(prefix)) {
        return Area::App;
    }
    Area::Internal
}

/// Whether a file's code changed: for Rust and TypeScript the lines `catalog loc` counts, all white
/// space removed (so a reflow is no change); any other file byte for byte.
fn code_changed(path: &str, before: Option<&str>, after: Option<&str>) -> bool {
    let code = |text: Option<&str>| {
        text.map(|text| code_lines(text).concat().split_whitespace().collect::<String>())
    };
    if path.ends_with(".rs") || path.ends_with(".ts") || path.ends_with(".tsx") {
        code(before) != code(after)
    } else {
        before != after
    }
}

/// R1365000: a card's family, the card a token counts with (`meditative-097-6` is `meditative-097`).
pub(crate) fn family(id: &str) -> String {
    id.splitn(3, '-').take(2).collect::<Vec<_>>().join("-")
}

/// The sets `SHIPPED_SETS` names in one side's `catalog_types.rs`.
fn shipped_sets(source: &str) -> Result<Vec<SetName>> {
    let start = source
        .find("pub const SHIPPED_SETS")
        .context("no `pub const SHIPPED_SETS`")?;
    let list = &source[start..];
    let open = list.find("= [").context("`SHIPPED_SETS` has no list")?;
    let close = list[open..]
        .find("];")
        .context("`SHIPPED_SETS`'s list does not close")?;
    list[open..open + close]
        .split("SetName::")
        .skip(1)
        .map(|rest| {
            let name: String = rest.chars().take_while(char::is_ascii_alphanumeric).collect();
            SetName::ALL
                .iter()
                .find(|set| format!("{set:?}") == name)
                .copied()
                .with_context(|| format!("`SetName::{name}` is no set"))
        })
        .collect()
}

/// One side of the change: its catalog, every set's cards, and the sets that ship.
pub(crate) struct Side {
    pub catalog: Catalog,
    pub sets: Vec<SetName>,
}

impl Side {
    fn ships(&self, id: &str) -> bool {
        let set = self
            .catalog
            .get(id)
            .and_then(|entry| entry.get("set"))
            .and_then(|set| set.as_str());
        set.is_none_or(|set| self.sets.iter().any(|shipped| shipped.as_str() == set))
    }
}

/// One changed path, with its text on each side (`None` where it does not exist).
pub(crate) struct FileChange {
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// Stage 1's reading of a diff.
#[derive(Default, Debug, PartialEq)]
pub(crate) struct Diff {
    /// The sets the head ships and the base does not.
    pub sets: Vec<String>,
    /// Shipped families whose catalog entries changed a field that plays (added and removed ones too).
    pub play: BTreeSet<String>,
    /// Shipped families whose catalog entries changed wording only.
    pub wording: BTreeSet<String>,
    /// Shipped cards (ids, not families) whose script's code changed.
    pub scripts: BTreeSet<String>,
    pub play_code: bool,
    pub ai: bool,
    pub app: bool,
}

/// R1365000: stage 1, from both sides and the changed files.
pub(crate) fn read_diff(before: &Side, after: &Side, files: &[FileChange]) -> Diff {
    let mut diff = Diff {
        sets: after
            .sets
            .iter()
            .filter(|set| !before.sets.contains(set))
            .map(|set| set.as_str().to_string())
            .collect(),
        ..Diff::default()
    };
    for change in diff_catalogs(Some(&before.catalog), &after.catalog) {
        let side = if change.kind == ChangeKind::Removed {
            before
        } else {
            after
        };
        if !side.ships(&change.id) {
            continue;
        }
        let plays = match &change.fields {
            Some(fields) => fields
                .iter()
                .any(|field| !WORDING_FIELDS.contains(&field.as_str())),
            None => true,
        };
        if plays {
            diff.play.insert(family(&change.id));
        } else {
            diff.wording.insert(family(&change.id));
        }
    }
    for file in files {
        let (before_text, after_text) = (file.before.as_deref(), file.after.as_deref());
        let area = area(&file.path);
        if area == Area::Catalog || !code_changed(&file.path, before_text, after_text) {
            continue;
        }
        match area {
            Area::Script => {
                let id = after_text.or(before_text).and_then(script_id);
                match id {
                    Some(id) if after.ships(&id) && after.catalog.contains_key(&id) => {
                        diff.scripts.insert(id);
                    }
                    // A script whose card left the catalog is the catalog's removal; one of a set not
                    // shipped is no change (R1420).
                    Some(_) => {}
                    None => diff.play_code = true,
                }
            }
            Area::PlayCode => diff.play_code = true,
            Area::Ai => diff.ai = true,
            Area::App => diff.app = true,
            Area::Catalog | Area::Internal => {}
        }
    }
    diff.wording.retain(|family| !diff.play.contains(family));
    diff
}

/// A card script's `pub const ID`.
pub(crate) fn script_id(source: &str) -> Option<String> {
    let after = source.split("pub const ID: &str = \"").nth(1)?;
    after.split('"').next().map(str::to_string)
}

/// What stage 2 found against the base build.
#[derive(Default, Debug, PartialEq)]
pub(crate) struct Checked {
    /// Families whose script change made their forced games diverge.
    pub scripts: BTreeSet<String>,
    /// Families an unforced divergence touched, after a change to play code.
    pub touched: BTreeSet<String>,
    /// An unforced divergence touched no card: a rule every game meets.
    pub rule: bool,
    /// Whether the AI's self-play logs differ; `None` when the check did not run.
    pub ai: Option<bool>,
}

/// R1365000's answer.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Classification {
    pub kind: Kind,
    pub tier: Option<Tier>,
    pub sets: Vec<String>,
    /// The families that play differently.
    pub cards: Vec<String>,
    /// The families whose wording alone changed.
    pub wording: Vec<String>,
    /// A divergence no card explains made the tier a patch.
    pub rule: bool,
    pub stage2: &'static str,
    /// What stage 2 would have checked, when it did not run.
    pub unchecked: Vec<String>,
}

/// R1365000: the kind and tier, from stage 1 and, when it ran, stage 2.
pub(crate) fn classify(diff: &Diff, checked: Option<&Checked>) -> Classification {
    let mut cards = diff.play.clone();
    let mut unchecked = Vec::new();
    match checked {
        Some(checked) => {
            cards.extend(checked.scripts.iter().cloned());
            cards.extend(checked.touched.iter().cloned());
        }
        None => {
            cards.extend(diff.scripts.iter().map(|id| family(id)));
            unchecked.extend(diff.scripts.iter().map(|id| format!("script:{id}")));
            if diff.play_code {
                unchecked.push("play-code".to_string());
            }
            if diff.ai {
                unchecked.push("ai".to_string());
            }
        }
    }
    let rule = checked.is_some_and(|checked| checked.rule);
    let tier = if !diff.sets.is_empty() {
        Some(Tier::Major)
    } else if rule || cards.len() >= PATCH_MIN_CARDS {
        Some(Tier::Patch)
    } else if !cards.is_empty() || !diff.wording.is_empty() {
        Some(Tier::Micro)
    } else {
        None
    };
    let ai = match checked.and_then(|checked| checked.ai) {
        Some(changed) => changed,
        None => diff.ai,
    };
    let kind = if tier.is_some() {
        Kind::Cards
    } else if ai {
        Kind::Ai
    } else if diff.app {
        Kind::App
    } else {
        Kind::Internal
    };
    Classification {
        kind,
        tier,
        sets: diff.sets.clone(),
        wording: diff
            .wording
            .iter()
            .filter(|family| !cards.contains(*family))
            .cloned()
            .collect(),
        cards: cards.into_iter().collect(),
        rule,
        stage2: if checked.is_some() { "ran" } else { "skipped" },
        unchecked,
    }
}

// ---------------------------------------------------------------------------------------------
// Stage 2
// ---------------------------------------------------------------------------------------------

/// One divergence in the parent's `golden check` report.
#[derive(Debug, PartialEq)]
pub(crate) struct Divergence {
    pub seed: String,
    pub step: String,
    pub which: String,
    pub action: Option<String>,
    /// Where the parent left its side's canonical text.
    pub text: Option<String>,
}

/// The divergences in a `golden check` report: `seed S step N: \`W\` expected …`, then its indented
/// `action:` and `Rust side:` lines.
pub(crate) fn divergences(report: &str) -> Vec<Divergence> {
    let mut out: Vec<Divergence> = Vec::new();
    for line in report.lines() {
        if let Some(rest) = line.strip_prefix("seed ") {
            let Some((seed, rest)) = rest.split_once(" step ") else {
                continue;
            };
            let Some((step, rest)) = rest.split_once(": `") else {
                continue;
            };
            let Some((which, _)) = rest.split_once('`') else {
                continue;
            };
            out.push(Divergence {
                seed: seed.to_string(),
                step: step.to_string(),
                which: which.to_string(),
                action: None,
                text: None,
            });
        } else if let Some(last) = out.last_mut() {
            let line = line.trim_start();
            if let Some(action) = line.strip_prefix("action: ") {
                last.action = Some(action.to_string());
            } else if let Some(path) = line.strip_prefix("Rust side: ") {
                last.text = Some(path.to_string());
            }
        }
    }
    out
}

/// Every string inside a JSON value.
fn strings_in(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            out.insert(text.clone());
        }
        Value::Array(items) => items.iter().for_each(|item| strings_in(item, out)),
        Value::Object(map) => map.values().for_each(|item| strings_in(item, out)),
        _ => {}
    }
}

/// The strings that name what differs between two canonical texts: the `defId` of every card
/// instance that changed, the elements one side's arrays hold and the other's do not (instances
/// paired by `id`), and the strings where two leaves differ.
pub(crate) fn differing_strings(a: &Value, b: &Value, out: &mut BTreeSet<String>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for side in [x, y] {
                if let Some(Value::String(def)) = side.get("defId") {
                    out.insert(def.clone());
                }
            }
            let keys: BTreeSet<&String> = x.keys().chain(y.keys()).collect();
            for key in keys {
                differing_strings(
                    x.get(key).unwrap_or(&Value::Null),
                    y.get(key).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            let id = |item: &Value| item.get("id").and_then(Value::as_str).map(str::to_string);
            for item in x {
                match id(item)
                    .and_then(|key| y.iter().find(|other| id(other).as_deref() == Some(key.as_str())))
                {
                    Some(other) => differing_strings(item, other, out),
                    None if !y.contains(item) => strings_in(item, out),
                    None => {}
                }
            }
            for item in y {
                let paired = id(item)
                    .is_some_and(|key| x.iter().any(|other| id(other).as_deref() == Some(key.as_str())));
                if !paired && !x.contains(item) {
                    strings_in(item, out);
                }
            }
        }
        _ => {
            strings_in(a, out);
            strings_in(b, out);
        }
    }
}

/// The shipped families the strings name, as catalog ids or as the instances of `state`.
fn families_named(strings: &BTreeSet<String>, head: &Side, state: &GameState) -> BTreeSet<String> {
    strings
        .iter()
        .filter_map(|text| {
            if head.catalog.contains_key(text) {
                Some(text.clone())
            } else {
                find_instance(state, text).map(|card| card.def_id.clone())
            }
        })
        .flat_map(|id| fused_parts(state, &id))
        .filter(|id| head.ships(id))
        .map(|id| family(&id))
        .collect()
}

/// R1487001: the cards a definition stands for: itself, or for a fusion every card it was fused from,
/// a fusion among them read the same way (R77, R468). A fused definition is no catalog card.
fn fused_parts(state: &GameState, def_id: &str) -> Vec<String> {
    match jackioh_engine::subsystems::fuse::fused_ingredients(state, def_id) {
        Some(parts) => parts.iter().flat_map(|part| fused_parts(state, part)).collect(),
        None => vec![def_id.to_string()],
    }
}

/// The instances an action or a legal action acts with: the card played, activated or switched,
/// and the attacker, never the targets it names.
fn actors(action: &Value) -> BTreeSet<String> {
    ["instanceId", "attackerId"]
        .iter()
        .filter_map(|key| action.get(*key).and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

/// The families one divergence touches, most direct first: the cards whose own legal actions differ,
/// else the card the diverging action acts with, else the cards whose instances differ between the
/// two builds' texts. Empty when it names none (a deal, a turn's end, a rule every game meets).
fn touched_by(divergence: &Divergence, game: &Value, head: &Side) -> Result<BTreeSet<String>> {
    let step_before = || match divergence.step.parse::<usize>() {
        Ok(0) => Some("begin".to_string()),
        Ok(n) => Some((n - 1).to_string()),
        Err(_) => None,
    };
    let parent = divergence
        .text
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok());
    let (mut options, mut differing) = (BTreeSet::new(), BTreeSet::new());
    let mut state = None;
    if let Some((text, at)) = text_at(game, &divergence.step, &divergence.which)? {
        if let Some(parent) = &parent {
            if divergence.which == "l" {
                let (ours, theirs): (BTreeSet<&str>, BTreeSet<&str>) =
                    (text.lines().collect(), parent.trim_end().lines().collect());
                for line in ours.symmetric_difference(&theirs) {
                    options.extend(actors(&serde_json::from_str(line).unwrap_or(Value::Null)));
                }
            } else if let (Ok(ours), Ok(theirs)) = (
                serde_json::from_str::<Value>(&text),
                serde_json::from_str::<Value>(parent.trim_end()),
            ) {
                differing_strings(&ours, &theirs, &mut differing);
            }
        }
        state = Some(at);
    }
    if state.is_none()
        && let Some(step) = step_before()
    {
        state = text_at(game, &step, "s")?.map(|(_, at)| at);
    }
    let Some(state) = state else {
        return Ok(BTreeSet::new());
    };
    let acting = divergence
        .action
        .as_deref()
        .and_then(|action| serde_json::from_str::<Value>(action).ok())
        .map(|action| actors(&action))
        .unwrap_or_default();
    for named in [options, acting, differing] {
        let touched = families_named(&named, head, &state);
        if !touched.is_empty() {
            return Ok(touched);
        }
    }
    Ok(BTreeSet::new())
}

/// A recorded traces file's games, by seed.
fn games_in(traces: &Path) -> Result<HashMap<String, Value>> {
    Ok(fs::read_to_string(traces)
        .with_context(|| format!("reading {}", traces.display()))?
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|game| Some((game["seed"].as_str()?.to_string(), game)))
        .collect())
}

/// Whether `id`'s own play diverges from the parent's in `FORCED_GAMES` games with it forced in: a
/// divergence that touches its family, or one that touches no card at all. The other cards of those
/// games are random, so a divergence one of them explains says nothing of `id`.
fn forced_diverge(id: &str, head: &Side, parent: &Path) -> Result<bool> {
    let traces = temp_traces();
    let seeds: Vec<u32> = (1..=FORCED_GAMES).collect();
    record_seeds(&seeds, &[id.to_string()], &traces)?;
    let (_, report) = parent_check(parent, &traces)?;
    let games = games_in(&traces)?;
    let own = family(id);
    let mut diverged = false;
    for divergence in divergences(&report) {
        let Some(game) = games.get(&divergence.seed) else {
            continue;
        };
        let touched = touched_by(&divergence, game, head)?;
        if touched.is_empty() || touched.contains(&own) {
            diverged = true;
            break;
        }
    }
    let _ = fs::remove_file(&traces);
    Ok(diverged)
}

/// A self-play run's games as the comparison reads them: seed, log, final hash and result (not the
/// catalog version or the commit, which name the build).
fn selfplay_games(bin: &Path, out: &Path) -> Result<Vec<Value>> {
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let games = SELFPLAY_GAMES.to_string();
    let output = Process::new(bin)
        .args(["selfplay", "--games", &games, "--seed", SELFPLAY_SEED, "--out"])
        .arg(out)
        .output()
        .with_context(|| format!("running {} selfplay", bin.display()))?;
    if !output.status.success() {
        bail!(
            "{} selfplay failed: {}",
            bin.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let file = out.join(format!("selfplay-{SELFPLAY_SEED}-1-{SELFPLAY_GAMES}.jsonl"));
    let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
    text.lines()
        .map(|line| {
            let game: Value = serde_json::from_str(line).context("a self-play line")?;
            Ok(serde_json::json!([
                game["seed"],
                game["log"],
                game["hash"],
                game["result"]
            ]))
        })
        .collect()
}

/// R1365000's stage 2 against the parent build, this binary being the head's.
fn check(diff: &Diff, head: &Side, parent: &Path) -> Result<Checked> {
    let mut checked = Checked::default();
    let enough = |checked: &Checked| {
        diff.play.len() + checked.scripts.len() + checked.touched.len() >= PATCH_MIN_CARDS || checked.rule
    };
    for id in &diff.scripts {
        if enough(&checked) {
            break;
        }
        let family = family(id);
        if diff.play.contains(&family) || checked.scripts.contains(&family) {
            continue;
        }
        let diverged = match forced_diverge(id, head, parent) {
            Ok(diverged) => diverged,
            Err(error) => {
                // A card the recorder cannot force counts as changed, never as a refactor.
                eprintln!("classify: {id} could not be forced ({error:#}); counted as changed");
                true
            }
        };
        if diverged {
            checked.scripts.insert(family);
        }
    }
    if diff.play_code && !enough(&checked) {
        let traces = temp_traces();
        let seeds: Vec<u32> = (1..=UNFORCED_GAMES).collect();
        record_seeds(&seeds, &[], &traces)?;
        let (_, report) = parent_check(parent, &traces)?;
        let games = games_in(&traces)?;
        for divergence in divergences(&report) {
            let Some(game) = games.get(&divergence.seed) else {
                continue;
            };
            let touched = touched_by(&divergence, game, head)?;
            if touched.is_empty() {
                checked.rule = true;
            }
            checked.touched.extend(touched);
        }
        let _ = fs::remove_file(&traces);
    }
    let cards_changed = !diff.play.is_empty()
        || !diff.wording.is_empty()
        || !diff.sets.is_empty()
        || !checked.scripts.is_empty()
        || !checked.touched.is_empty()
        || checked.rule;
    if diff.ai && !cards_changed {
        let dir = temp_traces().with_extension("selfplay");
        let current = std::env::current_exe().context("this binary's path")?;
        let ours = selfplay_games(&current, &dir.join("head"))?;
        let theirs = selfplay_games(parent, &dir.join("base"))?;
        let _ = fs::remove_dir_all(&dir);
        checked.ai = Some(ours != theirs);
    }
    Ok(checked)
}

// ---------------------------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------------------------

/// `rev:path`'s text, or `None` where the commit has no such file.
pub(crate) fn show(root: &Path, rev: &str, path: &str) -> Option<String> {
    git(root, &["cat-file", "-p", &format!("{rev}:{path}")], &[]).ok()
}

pub(crate) fn side_at(root: &Path, rev: &str) -> Result<Side> {
    let catalog = show(root, rev, CATALOG_PATH).with_context(|| format!("{rev} has no {CATALOG_PATH}"))?;
    let sets = show(root, rev, SETS_PATH).with_context(|| format!("{rev} has no {SETS_PATH}"))?;
    Ok(Side {
        catalog: serde_json::from_str(&catalog).with_context(|| format!("{rev}'s catalog"))?,
        sets: shipped_sets(&sets).with_context(|| format!("{rev}'s SHIPPED_SETS"))?,
    })
}

pub fn run(args: Args) -> Result<()> {
    register_all();
    let root = repo_root();
    let rev = |name: &str| -> Result<String> {
        Ok(git(
            &root,
            &["rev-parse", "--verify", &format!("{name}^{{commit}}")],
            &[],
        )?
        .trim()
        .to_string())
    };
    let (base, head) = (rev(&args.base)?, rev(&args.head)?);
    let names = git(&root, &["diff", "--no-renames", "--name-only", &base, &head], &[])?;
    let files: Vec<FileChange> = names
        .lines()
        .filter(|path| !path.is_empty())
        .map(|path| FileChange {
            path: path.to_string(),
            before: show(&root, &base, path),
            after: show(&root, &head, path),
        })
        .collect();
    let (before, after) = (side_at(&root, &base)?, side_at(&root, &head)?);
    let diff = read_diff(&before, &after, &files);

    let checked = match &args.parent_bin {
        Some(parent) => {
            if rev("HEAD")? != head {
                bail!("stage 2 runs this build as the head's, but the checkout is not at {head}");
            }
            Some(check(&diff, &after, parent)?)
        }
        None => None,
    };
    let classification = classify(&diff, checked.as_ref());
    let problems = if args.claims {
        if checked.is_none() && !classification.unchecked.is_empty() {
            bail!(
                "--claims needs --parent-bin: stage 1 left {} unchecked",
                classification.unchecked.join(", ")
            );
        }
        crate::claims::problems(
            &classification,
            &crate::claims::fragments_at(&root, &head)?,
            &crate::claims::fragments_at(&root, &base)?,
        )
    } else {
        Vec::new()
    };
    if args.json {
        let report = Report {
            classification: &classification,
            labels: labels(&classification),
            problems: problems.clone(),
        };
        println!("{}", serde_json::to_string(&report)?);
    } else {
        print_classification(&classification);
        for problem in &problems {
            println!("classify: {problem}");
        }
    }
    if !problems.is_empty() {
        bail!(
            "the pending fragments' claims do not match the change ({} problem(s))",
            problems.len()
        );
    }
    Ok(())
}

fn print_classification(classification: &Classification) {
    let tier = classification
        .tier
        .map_or(String::new(), |tier| format!(", {tier:?}").to_lowercase());
    println!("classify: {:?}{tier}", classification.kind);
    for (label, list) in [
        ("sets that ship", &classification.sets),
        ("cards that play differently", &classification.cards),
        ("wording only", &classification.wording),
        ("not checked (no --parent-bin)", &classification.unchecked),
    ] {
        if !list.is_empty() {
            println!("  {label}: {}", list.join(", "));
        }
    }
    if classification.rule {
        println!("  a divergence no card explains: a rule every game meets");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn card(id: &str, set: &str, cost: i64, text: &str) -> (String, crate::patches::js::Object) {
        let entry = json!({ "id": id, "name": id, "set": set, "cost": cost, "base": { "text": text } });
        (id.to_string(), serde_json::from_value(entry).expect("an entry"))
    }

    fn side(cards: Vec<(String, crate::patches::js::Object)>, sets: &[SetName]) -> Side {
        Side {
            catalog: cards.into_iter().collect(),
            sets: sets.to_vec(),
        }
    }

    const SHIPPED: &[SetName] = &[SetName::Core, SetName::Meditative];

    fn cards(n: usize, cost: i64, text: &str) -> Vec<(String, crate::patches::js::Object)> {
        (1..=n)
            .map(|k| card(&format!("core-{k:03}"), "Core", cost, text))
            .collect()
    }

    #[test]
    fn r1365000_wording_alone_is_a_micro_however_many_cards() {
        let diff = read_diff(
            &side(cards(40, 1, "Old."), SHIPPED),
            &side(cards(40, 1, "New."), SHIPPED),
            &[],
        );
        let classified = classify(&diff, None);
        assert_eq!(
            (classified.kind, classified.tier),
            (Kind::Cards, Some(Tier::Micro))
        );
        assert_eq!(classified.wording.len(), 40);
        assert!(classified.cards.is_empty());
    }

    #[test]
    fn r1365000_nine_families_are_a_micro_and_ten_a_patch() {
        for (n, tier) in [(9, Tier::Micro), (PATCH_MIN_CARDS, Tier::Patch)] {
            let diff = read_diff(
                &side(cards(n, 1, "T."), SHIPPED),
                &side(cards(n, 2, "T."), SHIPPED),
                &[],
            );
            assert_eq!(classify(&diff, None).tier, Some(tier), "{n} cards");
        }
    }

    #[test]
    fn r1365000_a_token_counts_with_its_card_and_an_unshipped_set_not_at_all() {
        let before = side(
            vec![
                card("core-001", "Core", 1, "T."),
                card("core-001-1", "Core", 1, "T."),
                card("classic-001", "Classic", 1, "T."),
            ],
            SHIPPED,
        );
        let after = side(
            vec![
                card("core-001", "Core", 2, "T."),
                card("core-001-1", "Core", 2, "T."),
                card("classic-001", "Classic", 2, "T."),
            ],
            SHIPPED,
        );
        let classified = classify(&read_diff(&before, &after, &[]), None);
        assert_eq!(classified.cards, vec!["core-001".to_string()]);
    }

    #[test]
    fn r1365000_a_set_that_ships_is_a_major_and_nothing_else_is() {
        let before = side(cards(1, 1, "T."), &[SetName::Core]);
        let after = side(cards(1, 1, "T."), SHIPPED);
        let classified = classify(&read_diff(&before, &after, &[]), None);
        assert_eq!(
            (classified.kind, classified.tier),
            (Kind::Cards, Some(Tier::Major))
        );
        assert_eq!(classified.sets, vec!["Meditative".to_string()]);
        let many = read_diff(
            &side(cards(60, 1, "T."), SHIPPED),
            &side(cards(60, 9, "T."), SHIPPED),
            &[],
        );
        assert_eq!(classify(&many, None).tier, Some(Tier::Patch));
    }

    // R1437000: a flavor is display only, so a change to it alone is no card change.
    #[test]
    fn r1437000_a_flavor_alone_is_no_card_change() {
        let flavored = |flavor: &str| -> Vec<(String, crate::patches::js::Object)> {
            let mut cards = cards(3, 1, "T.");
            for (_, entry) in &mut cards {
                entry.insert("flavor".to_string(), crate::patches::js::Json::from(flavor));
            }
            cards
        };
        for (before, after) in [
            (cards(3, 1, "T."), flavored("Fire")),
            (flavored("Fire"), flavored("Dark")),
        ] {
            let classified = classify(
                &read_diff(&side(before, SHIPPED), &side(after, SHIPPED), &[]),
                None,
            );
            assert_eq!((classified.kind, classified.tier), (Kind::Internal, None));
            assert!(classified.cards.is_empty(), "{:?}", classified.cards);
            assert!(classified.wording.is_empty(), "{:?}", classified.wording);
        }
        let mut costed = flavored("Fire");
        costed[0]
            .1
            .insert("cost".to_string(), crate::patches::js::Json::from(2));
        let classified = classify(
            &read_diff(&side(cards(3, 1, "T."), SHIPPED), &side(costed, SHIPPED), &[]),
            None,
        );
        assert_eq!(classified.cards, vec!["core-001".to_string()]);
    }

    fn file(path: &str, before: &str, after: &str) -> FileChange {
        FileChange {
            path: path.to_string(),
            before: Some(before.to_string()),
            after: Some(after.to_string()),
        }
    }

    const SCRIPT: &str = "use jackioh_engine::prelude::*;\n\npub const ID: &str = \"core-001\";\n\npub fn script() -> CardScripts {\n    CardScripts::default()\n}\n\n#[cfg(test)]\nmod tests {}\n";

    #[test]
    fn r1365000_a_script_edit_to_comments_tests_or_layout_is_internal() {
        let edited = SCRIPT
            .replace("pub fn script()", "/// Its script.\npub fn script()")
            .replace(
                "    CardScripts::default()",
                "    CardScripts::default() // nothing",
            )
            .replace("mod tests {}", "mod tests { fn more() {} }")
            .replace(
                "use jackioh_engine::prelude::*;",
                "use jackioh_engine::prelude::{self, *};",
            );
        let both = side(cards(1, 1, "T."), SHIPPED);
        let diff = read_diff(
            &both,
            &both,
            &[file("crates/cards/src/scripts/core/c001.rs", SCRIPT, &edited)],
        );
        assert_eq!(classify(&diff, None).kind, Kind::Internal);

        let changed = SCRIPT.replace("CardScripts::default()", "CardScripts { ..Default::default() }");
        let diff = read_diff(
            &both,
            &both,
            &[file("crates/cards/src/scripts/core/c001.rs", SCRIPT, &changed)],
        );
        let classified = classify(&diff, None);
        assert_eq!(
            (classified.kind, classified.tier),
            (Kind::Cards, Some(Tier::Micro))
        );
        assert_eq!(classified.unchecked, vec!["script:core-001".to_string()]);
    }

    #[test]
    fn r1365000_stage_2_drops_a_refactor_and_keeps_a_divergence() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let changed = SCRIPT.replace("CardScripts::default()", "CardScripts { ..Default::default() }");
        let diff = read_diff(
            &both,
            &both,
            &[file("crates/cards/src/scripts/core/c001.rs", SCRIPT, &changed)],
        );
        assert_eq!(classify(&diff, Some(&Checked::default())).kind, Kind::Internal);
        let diverged = Checked {
            scripts: BTreeSet::from(["core-001".to_string()]),
            ..Checked::default()
        };
        assert_eq!(classify(&diff, Some(&diverged)).tier, Some(Tier::Micro));
        let rule = Checked {
            rule: true,
            ..Checked::default()
        };
        assert_eq!(classify(&diff, Some(&rule)).tier, Some(Tier::Patch));
    }

    #[test]
    fn r1365000_ai_with_identical_self_play_is_internal() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let diff = read_diff(
            &both,
            &both,
            &[file("crates/ai/src/evaluate.rs", "fn a() { 1 }", "fn a() { 2 }")],
        );
        assert_eq!(classify(&diff, None).kind, Kind::Ai);
        let same = Checked {
            ai: Some(false),
            ..Checked::default()
        };
        assert_eq!(classify(&diff, Some(&same)).kind, Kind::Internal);
        let tooling = read_diff(
            &both,
            &both,
            &[file("crates/ai/src/gate.rs", "fn a() { 1 }", "fn a() { 2 }")],
        );
        assert_eq!(classify(&tooling, None).kind, Kind::Internal);
    }

    #[test]
    fn r1365000_paths_land_in_their_area() {
        for (path, expected) in [
            ("apps/web/src/game/Game.tsx", Area::App),
            ("apps/web/src/game/Game.test.tsx", Area::Internal),
            ("crates/server/src/api/stats.rs", Area::App),
            ("crates/engine/src/wire/stats.rs", Area::App),
            ("crates/engine/src/combat.rs", Area::PlayCode),
            ("crates/engine/tests/rules/mb22.rs", Area::Internal),
            ("crates/cards/src/query.rs", Area::PlayCode),
            ("crates/cards/data/win_rates.json", Area::PlayCode),
            ("crates/cards/patches/patches.json", Area::App),
            ("crates/ai/src/search.rs", Area::Ai),
            ("crates/ai/src/sweep.rs", Area::Internal),
            (".github/workflows/ci.yml", Area::Internal),
            ("docs/issues-and-patches.md", Area::Internal),
        ] {
            assert_eq!(area(path), expected, "{path}");
        }
    }

    #[test]
    fn r1365000_shipped_sets_read_from_the_source() {
        let source =
            "pub const SHIPPED_SETS: [SetName; 2] = [\n    SetName::Core,\n    SetName::ClassicPlus,\n];";
        assert_eq!(
            shipped_sets(source).expect("sets"),
            vec![SetName::Core, SetName::ClassicPlus]
        );
        let here = fs::read_to_string(repo_root().join(SETS_PATH)).expect("catalog_types.rs");
        assert_eq!(
            shipped_sets(&here).expect("sets"),
            jackioh_engine::wire::SHIPPED_SETS.to_vec()
        );
    }

    #[test]
    fn r1365000_a_report_names_its_divergences() {
        let report = "seed jackioh-fuzz-3 step 12: `s` expected 1a, Rust has 2b\n    action: {\"type\":\"endTurn\"}\n    Rust side: /x/y.json\nseed jackioh-fuzz-9 step begin: `refused` expected a, Rust has b\ngolden check: 1 of 3 games replayed identically (40 steps), 2 diverged\n";
        assert_eq!(
            divergences(report),
            vec![
                Divergence {
                    seed: "jackioh-fuzz-3".into(),
                    step: "12".into(),
                    which: "s".into(),
                    action: Some("{\"type\":\"endTurn\"}".into()),
                    text: Some("/x/y.json".into()),
                },
                Divergence {
                    seed: "jackioh-fuzz-9".into(),
                    step: "begin".into(),
                    which: "refused".into(),
                    action: None,
                    text: None
                },
            ]
        );
    }

    #[test]
    fn r1365000_a_divergence_names_the_instances_that_changed() {
        let ours = json!({ "players": { "p1": { "life": 20, "board": [
            { "id": "c1", "defId": "core-001", "attack": 2 },
            { "id": "c2", "defId": "core-002", "attack": 3 } ] } } });
        let theirs = json!({ "players": { "p1": { "life": 20, "board": [
            { "id": "c2", "defId": "core-002", "attack": 3 },
            { "id": "c1", "defId": "core-001", "attack": 1 } ] } } });
        let mut strings = BTreeSet::new();
        differing_strings(&ours, &theirs, &mut strings);
        assert_eq!(strings, BTreeSet::from(["core-001".to_string()]));
    }

    #[test]
    fn r1487001_a_fused_card_touches_the_cards_it_was_fused_from() {
        register_all();
        let mut game = jackioh_engine::testkit::scenario(json!({
            "seed": "classify-fused",
            "p1": { "backrow": [{ "def": "core-079" }] },
        }));
        let id = game
            .backrow(jackioh_engine::PlayerId::P1, 1)
            .expect("the card is set")
            .id;
        jackioh_engine::find_instance_mut(game.state_mut(), &id)
            .expect("the card is set")
            .def_id = "t-2:core-067+(t-1:core-079+classicplus-074)".to_string();
        let head = side(vec![], &[SetName::Core, SetName::ClassicPlus]);
        assert_eq!(
            families_named(&BTreeSet::from([id]), &head, game.state()),
            BTreeSet::from([
                "classicplus-074".to_string(),
                "core-067".to_string(),
                "core-079".to_string(),
            ])
        );
    }

    #[test]
    fn r1365000_families_and_code_lines() {
        assert_eq!(family("meditative-097-6"), "meditative-097");
        assert_eq!(family("core-002"), "core-002");
        assert!(!code_changed(
            "a.rs",
            Some("fn a() {\n    1\n}\n"),
            Some("fn a() { 1 } // one\n")
        ));
        assert!(code_changed("a.json", Some("{}"), Some("{ }")));
    }
}
