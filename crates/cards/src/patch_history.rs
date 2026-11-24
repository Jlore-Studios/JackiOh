//! R1367000: the card patch history (`patches/patches.json`, compiled in) as a card's stats window
//! reads it: each shipped version in the file's order, with its tier and the card families that play
//! differently in it (`jackioh_engine::wire::stats::window`).
//!
//! `patches ship` records both (`tier`, `plays`) from #1366 on. A version shipped before that recorded
//! neither, and is read from its `changes`: a family plays differently when a line adds or removes
//! one of its cards, or changes a field of one that is not wording (`WORDING_FIELDS`). With no tier,
//! the first version of each release line (`PRE_TIER_MAJORS`) is a major, a version in which
//! `PATCH_MIN_CARDS` families or more play is a patch, and any other a micro. #1371's retcon writes
//! every old entry's real tier, so this reading only has to be sensible.
//!
//! R2529000: a patch that a system the owner names alone made one (`system: true`) moves no card but
//! the families it plays, so a window reads it as a micro over them. Its version keeps its tier
//! everywhere else.
//!
//! R4675000: the same history read as each card's play versions (`play_versions`), the runs of shipped
//! versions over which it plays the same, which no major or patch ends: only a change line that moves a
//! field that plays, or a claim in a version's own `plays`.

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::config::PATCH_MIN_CARDS;
use jackioh_engine::wire::stats::{PatchStep, PatchTier, PlayVersion, card_family, play_version_id};
use serde::Deserialize;

/// R1365000: the catalog fields a player reads and no game does. Every other field plays.
pub const WORDING_FIELDS: &[&str] = &[
    "name",
    "base.text",
    "radiant.text",
    "refs",
    "printedRarity",
    "index",
    "flavor",
];

/// R1367000: first version of each release line shipped before tiers were recorded.
pub const PRE_TIER_MAJORS: &[&str] = &["v0.1.0", "v0.2.0", "v0.2.5f", "v0.4.0"];

/// R4675000: the fields beyond `WORDING_FIELDS` that start no play version: `loc`, a card's line count,
/// which other cards read but which never moves its own play.
pub const PLAY_VERSION_QUIET_FIELDS: &[&str] = &["loc"];

/// The fields of a `patches.json` entry the window reads.
#[derive(Deserialize)]
struct Entry {
    version: String,
    #[serde(default)]
    tier: Option<PatchTier>,
    #[serde(default)]
    plays: Option<Vec<String>>,
    #[serde(default)]
    system: bool,
    /// R3944000: the micros a major or a patch rolls up.
    #[serde(default)]
    includes: Vec<String>,
    #[serde(default)]
    changes: Vec<Change>,
}

#[derive(Deserialize)]
struct Change {
    id: String,
    kind: String,
    #[serde(default)]
    fields: Vec<String>,
    /// The card's name in the version: its new name when the line renames it.
    #[serde(default)]
    name: Option<String>,
}

/// The families an entry's `changes` play differently, sorted.
fn plays_of(changes: &[Change]) -> Vec<String> {
    let mut families: Vec<String> = changes
        .iter()
        .filter(|change| {
            change.kind != "changed"
                || change
                    .fields
                    .iter()
                    .any(|field| !WORDING_FIELDS.contains(&field.as_str()))
        })
        .map(|change| card_family(&change.id).to_string())
        .collect();
    families.sort();
    families.dedup();
    families
}

fn step_of(entry: Entry) -> PatchStep {
    let plays = entry.plays.unwrap_or_else(|| plays_of(&entry.changes));
    let tier = if entry.system {
        PatchTier::Micro
    } else {
        entry
            .tier
            .unwrap_or(if PRE_TIER_MAJORS.contains(&entry.version.as_str()) {
                PatchTier::Major
            } else if plays.len() >= PATCH_MIN_CARDS {
                PatchTier::Patch
            } else {
                PatchTier::Micro
            })
    };
    PatchStep {
        version: entry.version,
        tier,
        plays,
    }
}

/// R1367000: a `patches.json` list as a card's window reads it, in the list's order.
pub fn patch_history_of(patches_json: &str) -> Result<Vec<PatchStep>, serde_json::Error> {
    let entries: Vec<Entry> = serde_json::from_str(patches_json)?;
    Ok(entries.into_iter().map(step_of).collect())
}

/// R1367000: the shipped patch history this build compiles in, oldest first.
pub fn patch_history() -> Vec<PatchStep> {
    patch_history_of(include_str!("../patches/patches.json")).expect("patches.json reads as a patch history")
}

/// R4675000: whether a change line moves how its card plays: it adds the card, or changes a field
/// that is neither wording (`WORDING_FIELDS`) nor quiet (`PLAY_VERSION_QUIET_FIELDS`).
fn starts_a_play_version(change: &Change) -> bool {
    change.kind == "added"
        || (change.kind == "changed"
            && change.fields.iter().any(|field| {
                !WORDING_FIELDS.contains(&field.as_str())
                    && !PLAY_VERSION_QUIET_FIELDS.contains(&field.as_str())
            }))
}

/// R4675000: whether a change line moves only quiet fields (`loc` alone).
fn quiet(change: &Change) -> bool {
    change.kind == "changed"
        && !change.fields.is_empty()
        && change
            .fields
            .iter()
            .all(|field| PLAY_VERSION_QUIET_FIELDS.contains(&field.as_str()))
}

/// R4675000: the families and ids an entry's own `plays` claim. A cut (a patch with `includes`, R3944000)
/// restates its micros' plays, which they claimed when they shipped, so it claims none; a major's
/// `includes` names the micros it rolls up, but its `plays` are its own. An entry with no `plays` is read
/// from its `changes`, as its window reads it. A claim whose every change line there moves `loc` alone
/// is none: #1371's retcon read the plays of the versions before #1366 from their `changes`, counting a
/// `loc` change as play (R1365000).
fn own_claims(entry: &Entry) -> IndexSet<String> {
    if entry.tier == Some(PatchTier::Patch) && !entry.includes.is_empty() {
        return IndexSet::new();
    }
    let plays = entry.plays.clone().unwrap_or_else(|| plays_of(&entry.changes));
    plays
        .into_iter()
        .filter(|claim| {
            let mut lines = entry
                .changes
                .iter()
                .filter(|change| change.id == *claim || card_family(&change.id) == claim.as_str())
                .peekable();
            lines.peek().is_none() || !lines.all(quiet)
        })
        .collect()
}

/// R4675000: every card's play versions over a `patches.json` list, by id in the order the cards were
/// added, each card's oldest first. A card's first play version starts where a line adds it, and a new
/// one where a line moves a field that plays (`starts_a_play_version`) or the version's own `plays`
/// claim it (`own_claims`): its family (`card_family`), or its id for a set's own token, which the
/// classifier reads as a family of its own (R1365000). A tier ends none: a rule every game meets claims
/// no family, so it starts none. The newest version of each card the last entry still holds is live.
pub fn play_versions_of(patches_json: &str) -> Result<IndexMap<String, Vec<PlayVersion>>, serde_json::Error> {
    let entries: Vec<Entry> = serde_json::from_str(patches_json)?;
    let mut present: IndexSet<String> = IndexSet::new();
    let mut names: IndexMap<String, String> = IndexMap::new();
    let mut out: IndexMap<String, Vec<PlayVersion>> = IndexMap::new();
    for entry in &entries {
        let claims = own_claims(entry);
        let mut starts: IndexSet<&str> = IndexSet::new();
        for change in &entry.changes {
            match change.kind.as_str() {
                "added" => {
                    present.insert(change.id.clone());
                }
                "removed" => {
                    present.shift_remove(&change.id);
                }
                _ => {}
            }
            if starts_a_play_version(change) {
                starts.insert(&change.id);
            }
            if let Some(name) = &change.name {
                names.insert(change.id.clone(), name.clone());
            }
        }
        for id in &present {
            let started = starts.contains(id.as_str())
                || claims.contains(card_family(id))
                || claims.contains(id.as_str());
            let name = names.get(id).cloned().unwrap_or_else(|| id.clone());
            let versions = out.entry(id.clone()).or_default();
            match versions.last_mut() {
                Some(open) if !started => {
                    open.versions.push(entry.version.clone());
                    open.name = name;
                }
                _ => versions.push(PlayVersion {
                    id: play_version_id(id, &entry.version),
                    card: id.clone(),
                    name,
                    versions: vec![entry.version.clone()],
                    live: false,
                }),
            }
        }
    }
    for id in &present {
        if let Some(newest) = out.get_mut(id).and_then(|versions| versions.last_mut()) {
            newest.live = true;
        }
    }
    Ok(out)
}

/// R4675000: the play versions of the shipped history this build compiles in, by card id.
pub fn play_versions() -> IndexMap<String, Vec<PlayVersion>> {
    play_versions_of(include_str!("../patches/patches.json")).expect("patches.json reads as play versions")
}

/// R4675000: the play versions of every card a deck can hold, in catalog order: the catalog's cards that
/// are no token, of a set that ships (R1420), each card's oldest first.
pub fn deck_play_versions() -> Vec<PlayVersion> {
    let mut by_card = play_versions();
    crate::CATALOG
        .values()
        .filter(|def| !def.token && jackioh_engine::set_ships(def.set))
        .flat_map(|def| by_card.shift_remove(&def.id).unwrap_or_default())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(version: &str, tier: PatchTier, plays: &[&str]) -> PatchStep {
        PatchStep {
            version: version.to_string(),
            tier,
            plays: plays.iter().map(|family| family.to_string()).collect(),
        }
    }

    #[test]
    fn r1367000_an_entry_with_no_tier_is_read_from_its_changes() {
        let many: Vec<String> = (1..=PATCH_MIN_CARDS)
            .map(|n| format!(r#"{{ "id": "core-{n:03}", "kind": "changed", "fields": ["cost"] }}"#))
            .collect();
        let json = format!(
            r#"[
              {{ "version": "v0.1.0", "changes": [{{ "id": "core-001", "kind": "added" }}] }},
              {{ "version": "v0.1.0b", "changes": [
                {{ "id": "core-002", "kind": "changed", "fields": ["name", "base.text"] }},
                {{ "id": "core-003-1", "kind": "changed", "fields": ["base.text", "loc"] }},
                {{ "id": "core-004", "kind": "removed" }}
              ] }},
              {{ "version": "v0.1.1", "changes": [{}] }},
              {{ "version": "v0.1.1b", "tier": "patch", "plays": ["core-009"], "changes": [] }}
            ]"#,
            many.join(",")
        );
        let history = patch_history_of(&json).expect("reads");
        assert_eq!(history[0], step("v0.1.0", PatchTier::Major, &["core-001"]));
        // Wording alone plays nothing; a token's `loc` plays as its card's family.
        assert_eq!(
            history[1],
            step("v0.1.0b", PatchTier::Micro, &["core-003", "core-004"])
        );
        assert_eq!(history[2].tier, PatchTier::Patch);
        assert_eq!(history[2].plays.len(), PATCH_MIN_CARDS);
        // A recorded tier and play set stand as recorded.
        assert_eq!(history[3], step("v0.1.1b", PatchTier::Patch, &["core-009"]));
    }

    #[test]
    fn r1367000_the_compiled_in_history_reads_every_shipped_version_in_order() {
        let history = patch_history();
        let versions: Vec<&str> = history.iter().map(|step| step.version.as_str()).collect();
        assert_eq!(versions, crate::shipped_versions());
        assert_eq!(history[0].tier, PatchTier::Major);
        for major in PRE_TIER_MAJORS {
            assert!(versions.contains(major), "{major} is a shipped version");
        }
    }

    /// Each card's play versions over `json`, as the versions each spans.
    fn spans(json: &str) -> IndexMap<String, Vec<Vec<String>>> {
        play_versions_of(json)
            .expect("reads")
            .into_iter()
            .map(|(id, versions)| (id, versions.into_iter().map(|version| version.versions).collect()))
            .collect()
    }

    fn runs(runs: &[&[&str]]) -> Vec<Vec<String>> {
        runs.iter()
            .map(|run| run.iter().map(|version| version.to_string()).collect())
            .collect()
    }

    #[test]
    fn r4675000_two_changes_in_play_and_one_in_wording_make_two_breaks() {
        let json = r#"[
          { "version": "v1", "tier": "major", "plays": ["core-001"], "changes": [
            { "id": "core-001", "name": "Big D", "kind": "added" }
          ] },
          { "version": "v2", "tier": "micro", "plays": ["core-001"], "changes": [
            { "id": "core-001", "name": "Big D", "kind": "changed", "fields": ["cost"] }
          ] },
          { "version": "v3", "tier": "micro", "plays": [], "changes": [
            { "id": "core-001", "name": "Big D-fender", "kind": "changed", "fields": ["name", "base.text"] }
          ] },
          { "version": "v4", "tier": "patch", "plays": ["core-001"], "changes": [
            { "id": "core-001", "name": "Big D-fender", "kind": "changed", "fields": ["base.attack"] }
          ] },
          { "version": "v5", "tier": "micro", "plays": ["core-001"], "changes": [
            { "id": "core-001", "name": "Big D-fender", "kind": "changed", "fields": ["loc"] }
          ] }
        ]"#;
        let versions = play_versions_of(json).expect("reads");
        let card = &versions["core-001"];
        // The cost and the attack each start one; the rename and a `loc` claimed alone start none.
        assert_eq!(
            card.iter()
                .map(|version| version.versions.clone())
                .collect::<Vec<_>>(),
            runs(&[&["v1"], &["v2", "v3"], &["v4", "v5"]])
        );
        assert_eq!(
            card.iter().map(|version| version.id.as_str()).collect::<Vec<_>>(),
            ["core-001~v1", "core-001~v2", "core-001~v4"]
        );
        // A play version is named as its card was at its last version.
        assert_eq!(
            card.iter()
                .map(|version| version.name.as_str())
                .collect::<Vec<_>>(),
            ["Big D", "Big D-fender", "Big D-fender"]
        );
        assert_eq!(
            card.iter().map(|version| version.live).collect::<Vec<_>>(),
            [false, false, true]
        );
        assert_eq!((card[2].first(), card[2].last()), ("v4", "v5"));
    }

    #[test]
    fn r4675000_a_script_claimed_through_plays_starts_one_and_a_cut_restates_none() {
        let json = r#"[
          { "version": "v1", "tier": "major", "plays": [], "changes": [
            { "id": "core-001", "kind": "added" },
            { "id": "core-001-2", "kind": "added" },
            { "id": "core-002", "kind": "added" },
            { "id": "core-t-rush", "kind": "added" },
            { "id": "core-t-sheep", "kind": "added" }
          ] },
          { "version": "v2", "tier": "micro", "plays": ["core-001", "core-t-rush"], "changes": [] },
          { "version": "v3", "tier": "patch", "includes": ["v2"], "plays": ["core-001", "core-t-rush"], "changes": [] },
          { "version": "v4", "tier": "micro", "plays": ["core-t"], "changes": [] },
          { "version": "v5", "tier": "major", "includes": ["v4"], "plays": ["core-002"], "changes": [] }
        ]"#;
        let spans = spans(json);
        // A script claimed through its family starts a play version for the card and its token alike;
        // the cut that restates the claim starts none.
        assert_eq!(spans["core-001"], runs(&[&["v1"], &["v2", "v3", "v4", "v5"]]));
        assert_eq!(spans["core-001-2"], spans["core-001"]);
        // A set's own token is claimed by its id, or by the family an older version's plays name it by.
        assert_eq!(
            spans["core-t-rush"],
            runs(&[&["v1"], &["v2", "v3"], &["v4", "v5"]])
        );
        assert_eq!(spans["core-t-sheep"], runs(&[&["v1", "v2", "v3"], &["v4", "v5"]]));
        // A major's `includes` rolls its micros up, but its plays are its own.
        assert_eq!(spans["core-002"], runs(&[&["v1", "v2", "v3", "v4"], &["v5"]]));
    }

    #[test]
    fn r4675000_a_card_added_late_starts_where_it_was_added() {
        let json = r#"[
          { "version": "v1", "tier": "major", "plays": [], "changes": [
            { "id": "core-001", "kind": "added" },
            { "id": "core-003", "kind": "added" }
          ] },
          { "version": "v2", "tier": "micro", "plays": ["core-002"], "changes": [
            { "id": "core-002", "name": "Speciesist", "kind": "added" }
          ] },
          { "version": "v3", "tier": "micro", "plays": ["core-003"], "changes": [
            { "id": "core-003", "kind": "removed" }
          ] }
        ]"#;
        let versions = play_versions_of(json).expect("reads");
        let late = &versions["core-002"];
        assert_eq!(late.len(), 1);
        assert_eq!(late[0].versions, ["v2", "v3"]);
        assert_eq!(
            (late[0].id.as_str(), late[0].name.as_str()),
            ("core-002~v2", "Speciesist")
        );
        assert!(late[0].live);
        assert_eq!(versions["core-001"][0].versions, ["v1", "v2", "v3"]);
        // A card a version removes ends at the version before it, and is no longer live.
        assert_eq!(versions["core-003"][0].versions, ["v1", "v2"]);
        assert!(!versions["core-003"][0].live);
    }

    #[test]
    fn r4675000_a_rule_version_breaks_no_card() {
        let json = r#"[
          { "version": "v1", "tier": "major", "plays": [], "changes": [
            { "id": "core-001", "kind": "added" },
            { "id": "core-002", "kind": "added" }
          ] },
          { "version": "v2", "tier": "patch", "plays": [], "changes": [] },
          { "version": "v3", "tier": "major", "plays": [], "changes": [] }
        ]"#;
        let spans = spans(json);
        for id in ["core-001", "core-002"] {
            assert_eq!(spans[id], runs(&[&["v1", "v2", "v3"]]), "{id}");
        }
    }

    #[test]
    fn r4675000_the_compiled_in_history_gives_every_shipped_card_contiguous_versions() {
        let shipped = crate::shipped_versions();
        let versions = deck_play_versions();
        let mut ids: IndexSet<&str> = IndexSet::new();
        let mut by_card: IndexMap<&str, Vec<&PlayVersion>> = IndexMap::new();
        for version in &versions {
            assert!(ids.insert(&version.id), "{} repeats", version.id);
            assert!(!version.id.contains('@'), "{}", version.id);
            assert_eq!(version.id, play_version_id(&version.card, version.first()));
            by_card.entry(&version.card).or_default().push(version);
        }
        let deckable = crate::CATALOG
            .values()
            .filter(|def| !def.token && jackioh_engine::set_ships(def.set))
            .count();
        assert_eq!(by_card.len(), deckable);
        for (card, versions) in &by_card {
            let spanned: Vec<&str> = versions
                .iter()
                .flat_map(|version| version.versions.iter().map(String::as_str))
                .collect();
            let from = shipped
                .iter()
                .position(|version| *version == spanned[0])
                .expect("a shipped version");
            assert_eq!(spanned, shipped[from..], "{card}");
            let live: Vec<&&PlayVersion> = versions.iter().filter(|version| version.live).collect();
            assert_eq!(live.len(), 1, "{card}");
            assert_eq!(live[0].last(), crate::catalog_version(), "{card}");
            assert!(versions.last().is_some_and(|version| version.live), "{card}");
        }
    }

    // R2529000: a patch a named system alone made one restarts no card's window but its own families'.
    #[test]
    fn r2529000_a_system_patch_is_read_as_a_micro_over_its_plays() {
        let json = r#"[
          { "version": "v0.5.0", "tier": "major", "plays": [], "changes": [] },
          { "version": "v0.5.0b", "tier": "patch", "plays": ["core-001"], "system": true, "changes": [] },
          { "version": "v0.5.1", "tier": "patch", "plays": [], "changes": [] }
        ]"#;
        let history = patch_history_of(json).expect("reads");
        assert_eq!(history[1], step("v0.5.0b", PatchTier::Micro, &["core-001"]));
        assert_eq!(history[2].tier, PatchTier::Patch);
    }
}
