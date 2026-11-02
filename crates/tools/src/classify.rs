//! `cargo jackioh classify --base <rev> [--head <rev>] [--parent-bin <path>] [--json]` (#1365,
//! R1365000): what a change does to the game, read from its effect rather than its paths, so a patch
//! number moves exactly when the cards do (#1360). It names a **kind** (the highest of Cards, AI, App
//! and Internal), for Cards a **tier** and the families that play differently; no flag, title or label
//! overrides it. A patch takes `PATCH_MIN_REPRINTS` reprinted families, `PATCH_MIN_CARDS` that play
//! differently or a changed rule every game meets (R2455000, R2499000); else a micro. R2529000: a
//! change to how cards look (`CARD_VISUALS`) is a micro, a system the owner names (`SYSTEM_RELEASES`)
//! a patch.
//!
//! Stage 1 reads the diff: every catalog field but `WORDING_FIELDS` plays; a script's code is what
//! `catalog loc` counts; a set that ships is a major unless the owner wraps it into an earlier set's
//! line (`WRAPPED_SETS`, R2204000), and a card of an unshipped set is no change (R1420); other paths
//! keep their `use` lines, since an import can change which code runs (R2050000). R2173001: moving a
//! card across a BUSTED or complicated threshold (R1846000) plays every shipped reader differently.
//!
//! Stage 2 (`--parent-bin`, the base's build) keeps a script-only claim only when its forced games
//! diverge, so a refactor claims nothing; replays `UNFORCED_GAMES` games for play code, divergences
//! touching no card being a rule at `RULE_MIN_GAMES` (R2455000); and compares self-play logs for an AI
//! change, identical logs meaning Internal (`--selfplay-side`/`--selfplay-dir` split it across CI jobs,
//! #2651). It stops at `PATCH_MIN_CARDS` families; without `--parent-bin`, stage 1's answer stands.

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as Process, Stdio};

use anyhow::{Context, Result, bail};
use jackioh_cards::register_all;
use jackioh_engine::config::CRAFT_ID_PREFIX;
use jackioh_engine::wire::{SINGLE_PLAYER_SETS, SetName};
use jackioh_engine::{GameState, find_instance};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::card_lists::{self, CardLists, Scripts};
use crate::catalog::{code_lines, code_lines_with_uses};
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

/// R2455000: the unforced games that must hold a divergence touching no card for it to be a rule
/// every game meets, a quarter of them. Fewer is a rule few games meet, which moves no card's
/// statistics enough to restart them all, and counts as no card.
const RULE_MIN_GAMES: usize = UNFORCED_GAMES as usize / 4;

/// R2499000: the families a change must reprint for a patch, however few others it reaches. A
/// reprinted family's catalog entry changed and it plays differently: a balance pass, which moves
/// those cards' stats on purpose, as an engine change that only reaches cards does not
/// (`PATCH_MIN_CARDS`).
pub(crate) const PATCH_MIN_REPRINTS: usize = 15;

/// Stage 2: the self-play games a change to the AI plays on both builds, and their seed base.
const SELFPLAY_GAMES: u32 = 8;
const SELFPLAY_SEED: &str = "classify";

/// R2204000: the sets the owner wraps into an earlier set's release line, each with the set whose
/// line it joins. Such a set's arrival is no major: it earns the tier its cards do (a patch for any
/// set of `PATCH_MIN_CARDS` families or more), so `patches ship` names it within the line. Training
/// Grounds joins Stinkyscape's v0.5 (issue #2204, the owner's decision of 2026-10-28).
pub(crate) const WRAPPED_SETS: &[(SetName, SetName)] = &[(SetName::TrainingGrounds, SetName::Stinkyscape)];

/// R2204000: whether `set`, as its name is written, is wrapped into an earlier set's line.
pub(crate) fn wrapped(set: &str) -> bool {
    WRAPPED_SETS.iter().any(|(wrapped, _)| wrapped.as_str() == set)
}

/// R2529000: where the cards' look lives in the client: their faces, art, frames, foils and marks,
/// and the CSS that draws them. A change there (its tests aside) changes how cards look and not how
/// they play, so it is a micro of no family, claimed by a fragment's `visual`.
const CARD_VISUALS: &str = "apps/web/src/cards/";

/// R2529000: the systems the owner names a patch, each by its tracker issue and a name. The
/// classifier cannot tell a huge system from a big refactor, so a fragment claims one
/// (`patches --system <issue>`) and `patches ship` names it a patch whatever its cards earn. Naming
/// one is the owner's call, never a builder's, as wrapping a set is (R2204000).
pub(crate) const SYSTEM_RELEASES: &[(u32, &str)] = &[];

/// R2529000: whether the owner names `issue` a system (`SYSTEM_RELEASES`).
pub(crate) fn system_named(issue: u32) -> bool {
    SYSTEM_RELEASES.iter().any(|(named, _)| *named == issue)
}

const CATALOG_PATH: &str = "crates/cards/catalog.json";
pub(crate) const SETS_PATH: &str = "crates/engine/src/wire/catalog_types.rs";

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
    /// #2651: play one side of stage 2's AI check, `SELFPLAY_GAMES` self-play games on
    /// `SELFPLAY_SEED` (`head` on this binary, `base` on `--parent-bin`), into
    /// `<--selfplay-dir>/<side>` and stop, so each side plays in a CI job of its own. Nothing plays
    /// unless stage 1 finds a change to the AI and none to the cards (`plays_selfplay`).
    #[arg(long, value_enum, requires = "selfplay_dir")]
    selfplay_side: Option<SelfplaySide>,
    /// #2651: with `--selfplay-side`, where that side's games go; beside `--parent-bin` alone, where
    /// stage 2's AI check reads both sides' games (`<dir>/head`, `<dir>/base`) instead of playing them.
    #[arg(long, value_name = "DIR")]
    selfplay_dir: Option<PathBuf>,
}

/// One side of stage 2's AI check (`--selfplay-side`, #2651).
#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum SelfplaySide {
    /// The head's games, on this binary.
    Head,
    /// The base's games, on `--parent-bin`.
    Base,
}

impl SelfplaySide {
    /// The side's directory under `--selfplay-dir`, and under stage 2's own temporary one.
    fn dir_name(self) -> &'static str {
        match self {
            SelfplaySide::Head => "head",
            SelfplaySide::Base => "base",
        }
    }
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
    /// R2529000: the cards' look in the client (`CARD_VISUALS`).
    CardVisual,
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
    if path.starts_with(CARD_VISUALS) {
        return Area::CardVisual;
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

/// Whether a file's code changed, all white space removed (so a reflow is no change): for a card
/// script the lines `catalog loc` counts, so its `use` lines are no code (R1365000); for any other
/// Rust or TypeScript file the same lines with its `use` declarations kept, since moving an import
/// can change which code runs (R2050000); any other file byte for byte.
fn code_changed(path: &str, before: Option<&str>, after: Option<&str>) -> bool {
    let lines: fn(&str) -> Vec<String> = if area(path) == Area::Script {
        code_lines
    } else {
        code_lines_with_uses
    };
    let code =
        |text: Option<&str>| text.map(|text| lines(text).concat().split_whitespace().collect::<String>());
    if path.ends_with(".rs") || path.ends_with(".ts") || path.ends_with(".tsx") {
        code(before) != code(after)
    } else {
        before != after
    }
}

/// R1365000: a card's family, the card a token counts with (`meditative-097-6` is `meditative-097`).
/// A set's own token (`stinkyscape-t-dormant-starship`) is no one card's, so it is its own family.
pub(crate) fn family(id: &str) -> String {
    let mut parts = id.splitn(3, '-');
    match (parts.next(), parts.next()) {
        (Some(_), Some("t")) => id.to_string(),
        _ => id.splitn(3, '-').take(2).collect::<Vec<_>>().join("-"),
    }
}

/// The sets `SHIPPED_SETS` names in one side's `catalog_types.rs`.
pub(crate) fn shipped_sets(source: &str) -> Result<Vec<SetName>> {
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

/// One side of the change: its catalog, every set's cards, the sets that ship, and its card scripts
/// as the BUSTED and complicated lists see them (R2173001; none where only the catalog is read).
pub(crate) struct Side {
    pub catalog: Catalog,
    pub sets: Vec<SetName>,
    pub scripts: Scripts,
}

impl Side {
    pub(crate) fn of_catalog(catalog: Catalog, sets: Vec<SetName>) -> Side {
        Side {
            catalog,
            sets,
            scripts: Scripts::default(),
        }
    }

    /// R2173001: whether a list's card can join a pool on this side: a card of a set that ships, or
    /// of a single-player set, which an `everySet` pool takes (R1846001). A held set's is in none.
    fn pooled(&self, id: &str) -> bool {
        let single_player = self
            .catalog
            .get(id)
            .and_then(|entry| entry.get("set"))
            .and_then(|set| set.as_str())
            .is_some_and(|set| SINGLE_PLAYER_SETS.iter().any(|single| single.as_str() == set));
        self.ships(id) || single_player
    }

    /// R2173001: this side's BUSTED and complicated lists, of the cards that can join a pool.
    fn pooled_lists(&self) -> CardLists {
        let mut lists = self.scripts.lists();
        lists.busted.retain(|id| self.pooled(id));
        lists.complicated.retain(|id| self.pooled(id));
        lists
    }

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
    /// R2529000: the cards' look changed (`CARD_VISUALS`).
    pub visual: bool,
    pub app: bool,
}

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
    // R2204000: a set that ships brings every card of it into play, whether or not its entries moved
    // while it was held (R1420), so a wrapped set's arrival earns the tier its cards do.
    for (id, entry) in &after.catalog {
        let set = entry.get("set").and_then(|set| set.as_str());
        if set.is_some_and(|set| diff.sets.iter().any(|shipped| shipped == set)) {
            diff.play.insert(family(id));
        }
    }
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
            Area::CardVisual => diff.visual = true,
            Area::App => diff.app = true,
            Area::Catalog | Area::Internal => {}
        }
    }
    // R2173001: while a card that reads the BUSTED or complicated lists ships on both sides, a change
    // that moves a card across a threshold (a script edit, a card added or removed, a threshold
    // moved) plays every such reader differently: its pool is not what it was.
    let readers: Vec<&String> = after
        .scripts
        .readers
        .iter()
        .filter(|id| after.ships(id) && before.ships(id) && before.scripts.readers.contains(*id))
        .collect();
    if !readers.is_empty() && before.pooled_lists() != after.pooled_lists() {
        diff.play.extend(readers.into_iter().map(|id| family(id)));
    }
    diff.wording.retain(|family| !diff.play.contains(family));
    diff
}

/// A card script's `pub const ID`.
pub(crate) fn script_id(source: &str) -> Option<String> {
    let after = source.split("pub const ID: &str = \"").nth(1)?;
    after.split('"').next().map(str::to_string)
}

#[derive(Default, Debug, PartialEq)]
pub(crate) struct Checked {
    /// Families whose script change made their forced games diverge.
    pub scripts: BTreeSet<String>,
    /// Families an unforced divergence touched, after a change to play code.
    pub touched: BTreeSet<String>,
    /// Divergences that touch no card held `RULE_MIN_GAMES` of the unforced games, or the base
    /// build refused the head's decks: a rule every game meets (R2455000).
    pub rule: bool,
    /// Whether the AI's self-play logs differ; `None` when the check did not run.
    pub ai: Option<bool>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Classification {
    pub kind: Kind,
    pub tier: Option<Tier>,
    pub sets: Vec<String>,
    /// The families that play differently.
    pub cards: Vec<String>,
    /// R2499000: the families of `cards` whose catalog entry changed, a field that plays or their
    /// wording beside a change to their play.
    pub reprinted: Vec<String>,
    /// The families whose wording alone changed.
    pub wording: Vec<String>,
    /// A rule every game meets (`Checked::rule`) made the tier a patch.
    pub rule: bool,
    /// R2529000: the cards' look changed, a micro of no family at least.
    pub visual: bool,
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
    let reprinted: Vec<String> = cards
        .iter()
        .filter(|family| diff.play.contains(*family) || diff.wording.contains(*family))
        .cloned()
        .collect();
    let tier = if diff.sets.iter().any(|set| !wrapped(set)) {
        Some(Tier::Major)
    } else if rule || cards.len() >= PATCH_MIN_CARDS || reprinted.len() >= PATCH_MIN_REPRINTS {
        Some(Tier::Patch)
    } else if !cards.is_empty() || !diff.wording.is_empty() || diff.visual {
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
        reprinted,
        rule,
        visual: diff.visual,
        stage2: if checked.is_some() { "ran" } else { "skipped" },
        unchecked,
    }
}

// Stage 2

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

/// The shipped families the strings name, as catalog ids or as the instances of `state`. A crafted
/// definition (Meditative #17, `craft:#…`) is made in the game and ships in no patch, so it names no
/// family, and a fragment could not claim it (`patches check`).
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
        .filter(|id| !id.starts_with(CRAFT_ID_PREFIX) && head.ships(id))
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

/// Starts `bin`'s self-play run into `out`. The head's and the base's run side by side (#2353), since
/// each waits on its slowest game and in turn they took `rust (classify)` past seven minutes; or each
/// in a CI job of its own (`--selfplay-side`, #2651).
fn start_selfplay(bin: &Path, out: &Path) -> Result<Child> {
    fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let games = SELFPLAY_GAMES.to_string();
    Process::new(bin)
        .args(["selfplay", "--games", &games, "--seed", SELFPLAY_SEED, "--out"])
        .arg(out)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("running {} selfplay", bin.display()))
}

/// Waits for a self-play run `start_selfplay` started, and fails when the run did.
fn finish_selfplay(bin: &Path, run: Child) -> Result<()> {
    let output = run
        .wait_with_output()
        .with_context(|| format!("running {} selfplay", bin.display()))?;
    if !output.status.success() {
        bail!(
            "{} selfplay failed: {}",
            bin.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// The file a self-play run of `SELFPLAY_GAMES` on `SELFPLAY_SEED` writes into `out`.
fn selfplay_file(out: &Path) -> PathBuf {
    out.join(format!("selfplay-{SELFPLAY_SEED}-1-{SELFPLAY_GAMES}.jsonl"))
}

/// A self-play run's games, in `out`, as the comparison reads them: seed, log, final hash and result
/// (not the catalog version or the commit, which name the build).
fn selfplay_games(out: &Path) -> Result<Vec<Value>> {
    let file = selfplay_file(out);
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

/// R1705004: the text of the parent's refusal of a deck of the size the head deals (§2.6 L2), which
/// `validate_deck_with` words as "deck must hold exactly N cards (§2.6 L2), got M".
const PARENT_REFUSED_DECKS: &str = "cards (§2.6 L2), got ";

/// R1705004: whether the parent's `golden check` failed because it refuses the decks the head's
/// games are dealt, as a build from before #1705 refuses every 25-card deck. Its `create_game`
/// panics on them (it catches no unwind), so `parent_check` fails with the panic's text in its
/// stderr: the change moved what every game is dealt, a rule.
fn refuses_decks(error: &anyhow::Error) -> bool {
    let text = format!("{error:#}");
    text.contains(PARENT_REFUSED_DECKS) || text.contains(PARENT_REFUSED_PAIR)
}

/// R2511001: the text of a parent's refusal of a deck holding a Core card beside its Training Grounds
/// snapshot, which builds from before issue #2511 word as "… a deck holds one of the two (§2.6 L3,
/// R2195000)". The head lifted that part of §2.6 L3, so it deals decks the parent refuses: a deck
/// rule, read as R1705004 reads a refused deck size.
const PARENT_REFUSED_PAIR: &str = "a deck holds one of the two (§2.6 L3, R2195000)";

/// R1884002: the text of the parent's refusal of a card of a set it does not ship (§2.6 L3, R1420),
/// which `validate_setup` words as "\"<id>\" is a card of <set>, which has not shipped yet".
const PARENT_REFUSED_HELD: &str = "which has not shipped yet (§2.6 L3";

/// R1884002: whether the parent's `golden check` failed because the head's games deal cards of a set
/// the parent holds back, as main refuses every Stinkyscape card before the set's release: the
/// change ships that set, a major already (`diff.sets`), and the replay can say nothing more.
fn refuses_held_cards(error: &anyhow::Error) -> bool {
    format!("{error:#}").contains(PARENT_REFUSED_HELD)
}

/// What the parent's failure to replay the head's unforced games says: a rule every game meets when it
/// refuses the head's decks (R1705004); nothing more when the head ships a set whose cards it refuses,
/// the release being the major it reads (R1884002); and an error otherwise, a refusal of held cards
/// with no set shipping among them.
fn read_refusal(error: anyhow::Error, ships_a_set: bool) -> Result<bool> {
    if refuses_decks(&error) {
        Ok(true)
    } else if ships_a_set && refuses_held_cards(&error) {
        Ok(false)
    } else {
        Err(error)
    }
}

/// R2455000: whether the unforced games holding a divergence that touches no card, by seed, are
/// enough for a rule every game meets.
fn every_game(cardless: &BTreeSet<String>) -> bool {
    cardless.len() >= RULE_MIN_GAMES
}

/// Whether stage 2's AI check can run on stage 1's reading: the AI changed and, by the diff alone,
/// no card did (a catalog field, wording, the cards' look, a set that ships). `check` also skips it
/// when its own checks find a card change; `--selfplay-side` plays nothing when this is false.
fn plays_selfplay(diff: &Diff) -> bool {
    diff.ai && diff.play.is_empty() && diff.wording.is_empty() && !diff.visual && diff.sets.is_empty()
}

/// R1365000's stage 2 against the parent build, this binary being the head's.
fn check(diff: &Diff, head: &Side, parent: &Path, selfplay_dir: Option<&Path>) -> Result<Checked> {
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
        match parent_check(parent, &traces) {
            Ok((_, report)) => {
                let games = games_in(&traces)?;
                let mut cardless = BTreeSet::new();
                for divergence in divergences(&report) {
                    let Some(game) = games.get(&divergence.seed) else {
                        continue;
                    };
                    let touched = touched_by(&divergence, game, head)?;
                    if touched.is_empty() {
                        cardless.insert(divergence.seed.clone());
                    }
                    checked.touched.extend(touched);
                }
                if !cardless.is_empty() {
                    eprintln!(
                        "classify: a divergence no card explains in {} of {UNFORCED_GAMES} unforced games ({RULE_MIN_GAMES} make a rule every game meets)",
                        cardless.len()
                    );
                }
                checked.rule = every_game(&cardless);
            }
            Err(error) => checked.rule |= read_refusal(error, !diff.sets.is_empty())?,
        }
        let _ = fs::remove_file(&traces);
    }
    let found = !checked.scripts.is_empty() || !checked.touched.is_empty() || checked.rule;
    if plays_selfplay(diff) && !found {
        checked.ai = Some(match selfplay_dir {
            Some(dir) => {
                selfplay_games(&dir.join(SelfplaySide::Head.dir_name()))?
                    != selfplay_games(&dir.join(SelfplaySide::Base.dir_name()))?
            }
            None => {
                let dir = temp_traces().with_extension("selfplay");
                let current = std::env::current_exe().context("this binary's path")?;
                let (head_out, base_out) = (
                    dir.join(SelfplaySide::Head.dir_name()),
                    dir.join(SelfplaySide::Base.dir_name()),
                );
                // Both start before either is waited on, so they play side by side (#2353).
                let ours = start_selfplay(&current, &head_out);
                let theirs = start_selfplay(parent, &base_out);
                let ours = ours
                    .and_then(|run| finish_selfplay(&current, run))
                    .and_then(|()| selfplay_games(&head_out));
                let theirs = theirs
                    .and_then(|run| finish_selfplay(parent, run))
                    .and_then(|()| selfplay_games(&base_out));
                let _ = fs::remove_dir_all(&dir);
                ours? != theirs?
            }
        });
    }
    Ok(checked)
}

/// `--selfplay-side`: one side of stage 2's AI check, played into `<dir>/<side>` for another run's
/// `--selfplay-dir` to compare (#2651): `head` on this binary, `base` on `parent`. It plays only
/// when `plays_selfplay` holds, and prints one JSON line saying whether it played.
fn play_side(side: SelfplaySide, diff: &Diff, parent: Option<&Path>, dir: &Path) -> Result<()> {
    let out = dir.join(side.dir_name());
    let played = plays_selfplay(diff);
    if played {
        let bin = match side {
            SelfplaySide::Head => std::env::current_exe().context("this binary's path")?,
            SelfplaySide::Base => parent
                .context("--selfplay-side base needs --parent-bin, the base's build")?
                .to_path_buf(),
        };
        let run = start_selfplay(&bin, &out)?;
        finish_selfplay(&bin, run)?;
    }
    println!(
        "{}",
        serde_json::json!({ "selfplay": side.dir_name(), "played": played, "out": out.display().to_string() })
    );
    Ok(())
}

// The command

/// `rev:path`'s text, or `None` where the commit has no such file.
pub(crate) fn show(root: &Path, rev: &str, path: &str) -> Option<String> {
    git(root, &["cat-file", "-p", &format!("{rev}:{path}")], &[]).ok()
}

pub(crate) fn side_at(root: &Path, rev: &str) -> Result<Side> {
    let catalog = show(root, rev, CATALOG_PATH).with_context(|| format!("{rev} has no {CATALOG_PATH}"))?;
    let sets = show(root, rev, SETS_PATH).with_context(|| format!("{rev} has no {SETS_PATH}"))?;
    let catalog: Catalog = serde_json::from_str(&catalog).with_context(|| format!("{rev}'s catalog"))?;
    Ok(Side {
        scripts: card_lists::at_rev(root, rev, &catalog).with_context(|| format!("{rev}'s card scripts"))?,
        catalog,
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

    if let Some(side) = args.selfplay_side {
        if side == SelfplaySide::Head && rev("HEAD")? != head {
            bail!("--selfplay-side head plays this build as the head's, but the checkout is not at {head}");
        }
        let dir = args
            .selfplay_dir
            .as_deref()
            .context("--selfplay-side needs --selfplay-dir")?;
        return play_side(side, &diff, args.parent_bin.as_deref(), dir);
    }
    let checked = match &args.parent_bin {
        Some(parent) => {
            if rev("HEAD")? != head {
                bail!("stage 2 runs this build as the head's, but the checkout is not at {head}");
            }
            Some(check(&diff, &after, parent, args.selfplay_dir.as_deref())?)
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
        ("of them reprinted", &classification.reprinted),
        ("wording only", &classification.wording),
        ("not checked (no --parent-bin)", &classification.unchecked),
    ] {
        if !list.is_empty() {
            println!("  {label}: {}", list.join(", "));
        }
    }
    if classification.rule {
        println!("  divergences no card explains: a rule every game meets");
    }
    if classification.visual {
        println!("  how cards look changed");
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
        Side::of_catalog(cards.into_iter().collect(), sets.to_vec())
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

    // R2499000: a balance pass is a patch from `PATCH_MIN_REPRINTS` reprinted families.
    #[test]
    fn r2499000_fourteen_reprinted_families_are_a_micro_and_fifteen_a_patch() {
        for (n, tier) in [
            (PATCH_MIN_REPRINTS - 1, Tier::Micro),
            (PATCH_MIN_REPRINTS, Tier::Patch),
        ] {
            let diff = read_diff(
                &side(cards(n, 1, "T."), SHIPPED),
                &side(cards(n, 2, "T."), SHIPPED),
                &[],
            );
            let classified = classify(&diff, None);
            assert_eq!(
                (classified.tier, classified.reprinted.len()),
                (Some(tier), n),
                "{n} cards"
            );
        }
    }

    // R2455000, R2499000: families an engine change only reaches reprint nothing, so they need the
    // wider bar, `PATCH_MIN_CARDS`.
    #[test]
    fn r2455000_r2499000_families_the_engine_only_reaches_need_the_wider_bar() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let diff = Diff {
            play_code: true,
            ..read_diff(&both, &both, &[])
        };
        for (n, tier) in [
            (PATCH_MIN_REPRINTS, Tier::Micro),
            (PATCH_MIN_CARDS - 1, Tier::Micro),
            (PATCH_MIN_CARDS, Tier::Patch),
        ] {
            let reached = Checked {
                touched: (1..=n).map(|k| format!("core-{k:03}")).collect(),
                ..Checked::default()
            };
            let classified = classify(&diff, Some(&reached));
            assert_eq!(
                (classified.tier, classified.reprinted.len()),
                (Some(tier), 0),
                "{n} cards"
            );
        }
    }

    // R2499000: a rework rewrites a card's text and its script, so its wording beside a script that
    // plays reprints it; its wording alone does not.
    #[test]
    fn r2499000_a_rework_reprints_its_card_through_its_wording_and_its_script() {
        let diff = read_diff(
            &side(cards(PATCH_MIN_REPRINTS, 1, "Old."), SHIPPED),
            &side(cards(PATCH_MIN_REPRINTS, 1, "New."), SHIPPED),
            &[],
        );
        let reworked = Checked {
            scripts: diff.wording.clone(),
            ..Checked::default()
        };
        let classified = classify(&diff, Some(&reworked));
        assert_eq!(classified.reprinted.len(), PATCH_MIN_REPRINTS);
        assert_eq!(classified.tier, Some(Tier::Patch));
        assert_eq!(classify(&diff, Some(&Checked::default())).tier, Some(Tier::Micro));
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

    // R2204000: a set the owner wraps into an earlier line earns the tier its cards do, every card of
    // it playing differently once it ships though no entry moved; an unwrapped set is still a major.
    #[test]
    fn r2204000_a_wrapped_set_earns_its_cards_tier_and_an_unwrapped_one_is_a_major() {
        let set_cards = |set: &str, prefix: &str, n: usize| -> Vec<(String, crate::patches::js::Object)> {
            let mut all = cards(1, 1, "T.");
            all.extend((1..=n).map(|k| card(&format!("{prefix}-{k:03}"), set, 1, "T.")));
            all
        };
        let ships = |extra: SetName| -> Vec<SetName> { [SHIPPED, &[extra]].concat() };
        let tg = "Training Grounds";
        let wrapped_many = read_diff(
            &side(set_cards(tg, "traininggrounds", PATCH_MIN_CARDS), SHIPPED),
            &side(
                set_cards(tg, "traininggrounds", PATCH_MIN_CARDS),
                &ships(SetName::TrainingGrounds),
            ),
            &[],
        );
        let classified = classify(&wrapped_many, None);
        assert_eq!(
            (classified.kind, classified.tier),
            (Kind::Cards, Some(Tier::Patch))
        );
        assert_eq!(classified.sets, vec![tg.to_string()]);
        assert_eq!(classified.cards.len(), PATCH_MIN_CARDS);
        assert!(
            classified
                .cards
                .iter()
                .all(|family| family.starts_with("traininggrounds-"))
        );
        let wrapped_few = read_diff(
            &side(set_cards(tg, "traininggrounds", 2), SHIPPED),
            &side(
                set_cards(tg, "traininggrounds", 2),
                &ships(SetName::TrainingGrounds),
            ),
            &[],
        );
        assert_eq!(classify(&wrapped_few, None).tier, Some(Tier::Micro));
        let unwrapped = read_diff(
            &side(set_cards("Classic", "classic", PATCH_MIN_CARDS), SHIPPED),
            &side(
                set_cards("Classic", "classic", PATCH_MIN_CARDS),
                &ships(SetName::Classic),
            ),
            &[],
        );
        let classified = classify(&unwrapped, None);
        assert_eq!(classified.tier, Some(Tier::Major));
        assert_eq!(classified.cards.len(), PATCH_MIN_CARDS);
        assert!(!wrapped("Classic") && wrapped(tg));
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

    /// R2173001: a side whose catalog holds `ids`, its scripts counted from `scripts`.
    fn side_with_scripts(ids: &[(&str, &str)], sets: &[SetName], scripts: &[String]) -> Side {
        let mut side = side(ids.iter().map(|(id, set)| card(id, set, 1, "x")).collect(), sets);
        let config =
            "pub const BUSTED_CODE_LINES: usize = 100;\npub const COMPLICATED_CODE_LINES: usize = 60;\n";
        side.scripts = Scripts::read(scripts, &side.catalog, config).expect("the scripts");
        side
    }

    #[test]
    fn r2173001_a_script_edit_across_a_threshold_plays_the_readers_differently() {
        use crate::card_lists::tests::script;
        let ids = [
            ("core-001", "Core"),
            ("core-002", "Core"),
            ("stinkyscape-109", "Stinkyscape"),
            ("stinkyscape-050", "Stinkyscape"),
        ];
        let reader = script("stinkyscape-109", 30, true);
        let with = |lines: usize, held: usize, sets: &[SetName]| {
            side_with_scripts(
                &ids,
                sets,
                &[
                    script("core-001", lines, false),
                    script("core-002", 10, false),
                    reader.clone(),
                    script("stinkyscape-050", held, false),
                ],
            )
        };
        let shipping = [SetName::Core, SetName::Stinkyscape];
        let edit = |before: &Side, after: &Side| {
            let files = [file(
                "crates/cards/src/scripts/core/c001_x.rs",
                &script("core-001", 1, false),
                &script("core-001", 2, false),
            )];
            read_diff(before, after, &files)
        };

        // core-001 grows from 60 lines to 61: complicated now, so xxFreshJack's family plays differently.
        let diff = edit(&with(60, 10, &shipping), &with(61, 10, &shipping));
        assert_eq!(diff.play, BTreeSet::from(["stinkyscape-109".to_string()]));
        assert_eq!(diff.scripts, BTreeSet::from(["core-001".to_string()]));
        // Across BUSTED's 100 too; an edit that stays on one side of both moves no reader.
        let diff = edit(&with(100, 10, &shipping), &with(101, 10, &shipping));
        assert!(diff.play.contains("stinkyscape-109"));
        let diff = edit(&with(62, 10, &shipping), &with(90, 10, &shipping));
        assert!(diff.play.is_empty(), "{diff:?}");
        // A card no pool can draw (a held set's) moves no reader, nor does any move while no reader ships
        // on both sides: before the release (#1884) the rule is inert.
        let core_only = [SetName::Core];
        let held = side_with_scripts(
            &[
                ("core-001", "Core"),
                ("stinkyscape-109", "Core"),
                ("stinkyscape-050", "Stinkyscape"),
            ],
            &core_only,
            &[
                script("core-001", 10, false),
                reader.clone(),
                script("stinkyscape-050", 10, false),
            ],
        );
        let held_long = side_with_scripts(
            &[
                ("core-001", "Core"),
                ("stinkyscape-109", "Core"),
                ("stinkyscape-050", "Stinkyscape"),
            ],
            &core_only,
            &[
                script("core-001", 10, false),
                reader.clone(),
                script("stinkyscape-050", 200, false),
            ],
        );
        assert!(read_diff(&held, &held_long, &[]).play.is_empty());
        let diff = edit(&with(60, 10, &core_only), &with(61, 10, &core_only));
        assert!(diff.play.is_empty(), "no reader ships: {diff:?}");
        // The reader's own set shipping is the release, not a threshold move: every card of the set
        // plays differently for that alone (R2204000), and the threshold adds nothing to it.
        let diff = edit(&with(60, 10, &core_only), &with(61, 10, &shipping));
        assert_eq!(diff.sets, vec!["Stinkyscape".to_string()]);
        assert!(diff.play.contains("stinkyscape-109"));
        let level = edit(&with(60, 10, &core_only), &with(60, 10, &shipping));
        assert_eq!(diff.play, level.play);
        // The classification names the reader, so `--claims` asks a fragment for it (R1370000).
        let classification = classify(&edit(&with(60, 10, &shipping), &with(61, 10, &shipping)), None);
        assert!(classification.cards.contains(&"stinkyscape-109".to_string()));
    }

    // R2529000: how cards look is a micro of no family; its tests and the rest of the client are not.
    #[test]
    fn r2529000_a_change_to_how_cards_look_is_a_micro_of_no_family() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let face = |path: &str| {
            read_diff(
                &both,
                &both,
                &[file(path, "export const gold = 1;\n", "export const gold = 2;\n")],
            )
        };
        let looks = classify(&face("apps/web/src/cards/CardFace.tsx"), None);
        assert_eq!(
            (looks.kind, looks.tier, looks.visual),
            (Kind::Cards, Some(Tier::Micro), true)
        );
        assert!(looks.cards.is_empty());
        assert_eq!(labels(&looks), vec!["kind:cards", "tier:micro"]);
        let test = classify(&face("apps/web/src/cards/CardFace.test.tsx"), None);
        assert_eq!((test.kind, test.tier, test.visual), (Kind::Internal, None, false));
        let board = classify(&face("apps/web/src/game/Game.tsx"), None);
        assert_eq!((board.kind, board.tier, board.visual), (Kind::App, None, false));
        let comment = read_diff(
            &both,
            &both,
            &[file(
                "apps/web/src/cards/CardFace.tsx",
                "export const gold = 1;\n",
                "// The gold.\nexport const gold = 1;\n",
            )],
        );
        assert_eq!(classify(&comment, None).kind, Kind::Internal);
    }

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
            ("apps/web/src/cards/CardFace.tsx", Area::CardVisual),
            ("apps/web/src/cards/cards.css", Area::CardVisual),
            ("apps/web/src/cards/CardFace.test.tsx", Area::Internal),
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
    fn r1365000_a_crafted_card_is_no_shipped_family() {
        register_all();
        let mut game = jackioh_engine::testkit::scenario(json!({
            "seed": "classify-crafted",
            "p1": { "backrow": [{ "def": "core-079" }] },
        }));
        let id = game
            .backrow(jackioh_engine::PlayerId::P1, 1)
            .expect("the card is set")
            .id;
        jackioh_engine::find_instance_mut(game.state_mut(), &id)
            .expect("the card is set")
            .def_id = format!("{CRAFT_ID_PREFIX}#08aa72a2f2df2633");
        let head = side(vec![], &[SetName::Core]);
        assert_eq!(
            families_named(&BTreeSet::from([id]), &head, game.state()),
            BTreeSet::new()
        );
    }

    #[test]
    fn r1365000_families_and_code_lines() {
        assert_eq!(family("meditative-097-6"), "meditative-097");
        assert_eq!(family("core-002"), "core-002");
        assert_eq!(family("core-t-rush"), "core-t-rush");
        assert_eq!(
            family("stinkyscape-t-dormant-starship"),
            "stinkyscape-t-dormant-starship"
        );
        assert!(!code_changed(
            "a.rs",
            Some("fn a() {\n    1\n}\n"),
            Some("fn a() { 1 } // one\n")
        ));
        assert!(code_changed("a.json", Some("{}"), Some("{ }")));
    }

    // R2050000: outside a card script a `use` line is code, so a moved import is checked.
    const MODULE: &str = "use crate::a::run;\n\npub fn go() {\n    run()\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n";

    #[test]
    fn r2050000_a_use_only_change_outside_a_card_script_is_code() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let moved = MODULE.replace("use crate::a::run;", "use crate::b::run;");
        for path in ["crates/engine/src/combat.rs", "crates/cards/src/query.rs"] {
            let diff = read_diff(&both, &both, &[file(path, MODULE, &moved)]);
            assert!(diff.play_code, "{path}");
            assert_eq!(
                classify(&diff, None).unchecked,
                vec!["play-code".to_string()],
                "{path}"
            );
        }
        let diff = read_diff(&both, &both, &[file("crates/ai/src/search.rs", MODULE, &moved)]);
        let classified = classify(&diff, None);
        assert_eq!(classified.kind, Kind::Ai);
        assert_eq!(classified.unchecked, vec!["ai".to_string()]);
    }

    #[test]
    fn r2050000_a_use_only_change_in_a_card_script_is_still_no_code() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let edited = SCRIPT.replace(
            "use jackioh_engine::prelude::*;",
            "use jackioh_engine::prelude::{CardScripts, *};",
        );
        let diff = read_diff(
            &both,
            &both,
            &[file("crates/cards/src/scripts/core/c001.rs", SCRIPT, &edited)],
        );
        assert_eq!(diff, Diff::default());
        let classified = classify(&diff, None);
        assert_eq!(classified.kind, Kind::Internal);
        assert!(classified.unchecked.is_empty(), "{:?}", classified.unchecked);
        assert_eq!(
            crate::catalog::count_loc(SCRIPT),
            crate::catalog::count_loc(&edited)
        );
    }

    #[test]
    fn r2050000_comment_layout_and_test_changes_outside_a_card_script_are_no_code() {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let edited = "use crate::a::run; // the runner\n\n/// Goes.\npub fn go() { run() }\n\n#[cfg(test)]\nmod tests {\n    use super::go;\n\n    #[test]\n    fn goes() {\n        go()\n    }\n}\n";
        for path in [
            "crates/engine/src/combat.rs",
            "crates/cards/src/query.rs",
            "crates/ai/src/search.rs",
        ] {
            let diff = read_diff(&both, &both, &[file(path, MODULE, edited)]);
            assert_eq!(diff, Diff::default(), "{path}");
        }
    }

    #[test]
    fn r1365000_every_ai_tooling_entry_names_a_file() {
        let src = repo_root().join("crates/ai/src");
        for file in AI_TOOLING {
            assert!(
                src.join(file).is_file(),
                "AI_TOOLING names crates/ai/src/{file}, which is no file"
            );
        }
    }

    #[test]
    fn r1705004_a_base_build_refusing_the_heads_deck_size_is_a_rule() {
        // `parent_check`'s failure when main's `golden check` panics in `create_game` on a 25-card deck.
        let refused = anyhow::anyhow!(
            "/tmp/main-jackioh could not check /tmp/traces.jsonl: thread 'main' panicked at \
             crates/engine/src/state.rs:2066:13:\np1: deck must hold exactly 20 cards (§2.6 L2), got 25"
        );
        assert!(refuses_decks(&refused));
        assert!(refuses_decks(&refused.context("checking the unforced games")));
        let other = anyhow::anyhow!("/tmp/main-jackioh could not check /tmp/traces.jsonl: No such file");
        assert!(!refuses_decks(&other));
        let handicap = anyhow::anyhow!("p2: handicap.manaCap must be a non-negative integer (R180)");
        assert!(!refuses_decks(&handicap));
    }

    #[test]
    fn r2511001_a_base_build_refusing_a_core_card_beside_its_legacy_card_is_a_rule() {
        // `parent_check`'s failure when a build from before #2511 panics in `create_game` on a deck
        // holding core-026 beside traininggrounds-050.
        let refused = anyhow::anyhow!(
            "/tmp/main-jackioh could not check /tmp/traces.jsonl: thread '<unnamed>' panicked at \
             crates/engine/src/state.rs:2585:9:\np2: \"traininggrounds-050\" and \"core-026\" are a Core \
             card and its Training Grounds snapshot; a deck holds one of the two (§2.6 L3, R2195000)"
        );
        assert!(refuses_decks(&refused));
        assert!(read_refusal(refused, false).expect("a rule"));
    }

    // R2455000: divergences that touch no card are a rule every game meets only once they hold a
    // quarter of the unforced games; fewer leave the change to the families its divergences name, so
    // a narrow engine fix (v0.5.10's six families) is a micro.
    #[test]
    fn r2455000_a_rule_needs_a_quarter_of_the_unforced_games() {
        let seeds = |n: usize| -> BTreeSet<String> { (1..=n).map(|seed| seed.to_string()).collect() };
        assert_eq!(RULE_MIN_GAMES, 50);
        assert!(!every_game(&seeds(1)));
        assert!(!every_game(&seeds(RULE_MIN_GAMES - 1)));
        assert!(every_game(&seeds(RULE_MIN_GAMES)));
        let both = side(cards(1, 1, "T."), SHIPPED);
        let diff = Diff {
            play_code: true,
            ..read_diff(&both, &both, &[])
        };
        let narrow = Checked {
            touched: (1..=6).map(|k| format!("core-{k:03}")).collect(),
            ..Checked::default()
        };
        let classified = classify(&diff, Some(&narrow));
        assert_eq!((classified.tier, classified.rule), (Some(Tier::Micro), false));
        let wide = Checked { rule: true, ..narrow };
        assert_eq!(classify(&diff, Some(&wide)).tier, Some(Tier::Patch));
    }

    #[test]
    fn r1884002_a_base_build_refusing_the_set_the_head_ships_reads_as_that_release() {
        // `parent_check`'s failure when main's `golden check` panics in `create_game` on a deck
        // holding a card of the held Stinkyscape set, as it did on the set's release (#1884).
        let refused = anyhow::anyhow!(
            "/tmp/main-jackioh could not check /tmp/traces.jsonl: thread '<unnamed>' panicked at \
             crates/engine/src/state.rs:2475:9:\np1: \"stinkyscape-156\" is a card of Stinkyscape, which \
             has not shipped yet (§2.6 L3, R1420)"
        );
        assert!(refuses_held_cards(&refused));
        assert!(!refuses_decks(&refused), "no deck size moved");
        assert!(refuses_held_cards(
            &refused.context("checking the unforced games")
        ));
        let other = anyhow::anyhow!("/tmp/main-jackioh could not check /tmp/traces.jsonl: No such file");
        assert!(!refuses_held_cards(&other));
    }

    #[test]
    fn r1884002_the_refusal_reads_as_the_release_only_when_a_set_ships() {
        let held = || {
            anyhow::anyhow!(
                "/tmp/main-jackioh could not check /tmp/traces.jsonl: p1: \"stinkyscape-156\" is a card of \
                 Stinkyscape, which has not shipped yet (§2.6 L3, R1420)"
            )
        };
        // A set ships: the replay ends there, no rule.
        assert!(!read_refusal(held(), true).expect("the release"));
        // No set ships: the same refusal is still an error.
        assert!(read_refusal(held(), false).is_err());
        // R1705004's refused deck size is a rule either way.
        let decks = || anyhow::anyhow!("p1: deck must hold exactly 20 cards (§2.6 L2), got 25");
        assert!(read_refusal(decks(), false).expect("a rule"));
        assert!(read_refusal(decks(), true).expect("a rule"));
    }

    // #2651: each side of stage 2's AI check can play in a job of its own.
    #[derive(clap::Parser, Debug)]
    struct Cli {
        #[command(flatten)]
        args: Args,
    }

    fn parse(argv: &[&str]) -> Result<Args, clap::Error> {
        use clap::Parser;
        Cli::try_parse_from(std::iter::once("classify").chain(argv.iter().copied())).map(|cli| cli.args)
    }

    #[test]
    fn the_selfplay_flags_parse_and_a_side_needs_its_dir() {
        let args = parse(&["--base", "b", "--selfplay-side", "base", "--selfplay-dir", "d"]).unwrap();
        assert_eq!(args.selfplay_side, Some(SelfplaySide::Base));
        assert_eq!(args.selfplay_dir, Some(PathBuf::from("d")));
        assert!(parse(&["--base", "b", "--selfplay-side", "head"]).is_err());
        assert!(parse(&["--selfplay-side", "left", "--selfplay-dir", "d", "--base", "b"]).is_err());
        let args = parse(&["--base", "b", "--parent-bin", "p", "--selfplay-dir", "d"]).unwrap();
        assert_eq!(
            (args.selfplay_side, args.selfplay_dir),
            (None, Some(PathBuf::from("d")))
        );
    }

    /// The AI's file changed and nothing else: stage 1 finds an AI change and no card.
    fn ai_only() -> (Side, Diff) {
        let both = side(cards(1, 1, "T."), SHIPPED);
        let diff = read_diff(
            &both,
            &both,
            &[file("crates/ai/src/evaluate.rs", "fn a() { 1 }", "fn a() { 2 }")],
        );
        (both, diff)
    }

    #[test]
    fn a_selfplay_dir_is_compared_by_seed_log_hash_and_result() {
        let (both, diff) = ai_only();
        let dir = temp_traces().with_extension("selfplay-test");
        // A side's one game, its log and the build's name as given.
        let play = |side: SelfplaySide, log: Value, build: &str| {
            let out = dir.join(side.dir_name());
            fs::create_dir_all(&out).unwrap();
            let game = json!({
                "seed": "classify:1", "decks": [[], []], "log": log, "hash": "h", "result": null,
                "turns": 1, "catalog": build, "commit": build
            });
            fs::write(selfplay_file(&out), format!("{game}\n")).unwrap();
        };
        let log = || json!([{ "type": "endTurn" }]);
        let checked = || check(&diff, &both, Path::new("no-parent-is-run"), Some(&dir));
        // The builds' names differ and nothing else does: the AI decides as it did.
        play(SelfplaySide::Head, log(), "head-build");
        play(SelfplaySide::Base, log(), "base-build");
        assert_eq!(checked().unwrap().ai, Some(false));
        // One log differs: the AI decides differently.
        play(SelfplaySide::Base, json!([]), "base-build");
        assert_eq!(checked().unwrap().ai, Some(true));
        // A side that never played is an error, not "the same".
        fs::remove_dir_all(&dir).unwrap();
        assert!(checked().is_err());
    }

    #[test]
    fn a_side_plays_only_when_the_ai_check_can_run() {
        let (both, ai_diff) = ai_only();
        assert!(plays_selfplay(&ai_diff));
        // The AI changed, and so did a card's wording: stage 2 skips the AI check.
        let reworded = read_diff(
            &side(cards(1, 1, "Old."), SHIPPED),
            &side(cards(1, 1, "New."), SHIPPED),
            &[file("crates/ai/src/evaluate.rs", "fn a() { 1 }", "fn a() { 2 }")],
        );
        assert!(reworded.ai && !plays_selfplay(&reworded));
        let server = read_diff(
            &both,
            &both,
            &[file("crates/server/src/main.rs", "fn a() { 1 }", "fn a() { 2 }")],
        );
        assert!(!plays_selfplay(&server));

        let dir = temp_traces().with_extension("selfplay-side-test");
        // Nothing to play: it says so and writes nothing.
        play_side(SelfplaySide::Base, &server, None, &dir).unwrap();
        assert!(!dir.exists());
        // The base's side without the base's build is an error. (The head's side would run this
        // test binary as `selfplay`, so it is not tried.)
        assert!(play_side(SelfplaySide::Base, &ai_diff, None, &dir).is_err());
        assert!(!dir.exists());
    }
}
