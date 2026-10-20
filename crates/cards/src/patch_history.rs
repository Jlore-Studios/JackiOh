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

use jackioh_engine::config::PATCH_MIN_CARDS;
use jackioh_engine::wire::stats::{PatchStep, PatchTier, card_family};
use serde::Deserialize;

/// R1365000: the catalog fields a player reads and no game does. Every other field plays.
pub const WORDING_FIELDS: &[&str] = &[
    "name",
    "base.text",
    "radiant.text",
    "refs",
    "printedRarity",
    "index",
];

/// R1367000: the first version of each release line shipped before tiers were recorded, as the
/// Patch notes page's release lines name them (`apps/web/src/patches/releases.ts`'s `first`). Read
/// only for a version that records no tier.
pub const PRE_TIER_MAJORS: &[&str] = &["v0.1.0", "v0.2.0", "v0.2.5f", "v0.4.0"];

/// The fields of a `patches.json` entry the window reads.
#[derive(Deserialize)]
struct Entry {
    version: String,
    #[serde(default)]
    tier: Option<PatchTier>,
    #[serde(default)]
    plays: Option<Vec<String>>,
    #[serde(default)]
    changes: Vec<Change>,
}

#[derive(Deserialize)]
struct Change {
    id: String,
    kind: String,
    #[serde(default)]
    fields: Vec<String>,
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
    let tier = entry
        .tier
        .unwrap_or(if PRE_TIER_MAJORS.contains(&entry.version.as_str()) {
            PatchTier::Major
        } else if plays.len() >= PATCH_MIN_CARDS {
            PatchTier::Patch
        } else {
            PatchTier::Micro
        });
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
}
