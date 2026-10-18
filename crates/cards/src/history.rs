//! R767000: the shipped catalog versions this build carries, so a replay folds under the version its
//! game was played on. Every shipped version but the newest, which is the build's own: its
//! definitions rebuilt from `build.rs`'s `history.json` (the snapshot's entries as they differ from
//! `catalog.json`'s), in catalog order, and its scripts the build's own but where one is kept in
//! `KEPT_SCRIPTS`. The browser's module carries only the newest of them (`build.rs`'s
//! `BROWSER_CARRIED_FROM`).
//!
//! A version's catalog is built the first time a replay asks for it and kept for the process: one of
//! the catalog statics a pure crate may hold (CLAUDE.md rule 4), never game state.

use std::sync::{LazyLock, OnceLock};

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::{CardDef, CardDefs, CardScripts, ShippedCatalog};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::scripts::classic::c088_siphon_squad;
use crate::scripts::classic_plus::c040_appropriations;
use crate::scripts::core::{c049_snom_bunny_mind_control, c050_k_pop_fanatic};
use crate::scripts::meditative::{
    c017_true_craft_a_card, c037_cn_in_a_bottle, c074_montana_giant, c097_2_wishing_well, c097_8_prison,
};
use crate::{CATALOG, REGISTRY, catalog_json};

/// R767000, R388: the scripts kept for shipped versions, `(ID, version, script)`: a patch that changes
/// what a card's script does, not only a number its definition holds, keeps the script it replaces
/// here, under the newest shipped version that ran it, as any `fn` that builds that card's scripts
/// (`docs/ADDING_CARDS.md` §3). A replay of a game on that version, or on one before it back to the
/// previous kept script, folds under the kept one; every other card runs the build's own. The first
/// kept are #1034's: Wishing Well and Prison rolled without their controller's Luck (R987) through
/// v0.3.7b, and #1039's: Montaña Giant read a full hand as `HAND_CAP` cards, not its player's hand cap
/// (R1143), through v0.3.7b. K-Pop Fanatic's plain steal is kept beside them: it marked its target
/// purple and left one it had no room for with the opponent through v0.3.7b, before it Allured (R1045000).
/// #1149's five are kept beside them, each as it ran through v0.3.7b: Snom Bunny Mind Control stole any
/// enemy permanent, not only a Unit; Siphon Squad's aura worked from the moment it was set, not once it
/// was revealed (R1149000); True Craft a Card went to the graveyard, not to exile; CN in a Bottle was
/// replaced on any arrival in a hand, not cast on draw (R1149001); Appropriations' Culture and the
/// Radiant's Military and Healthcare had their higher multiples.
/// A card patch before them that changed a script's behaviour kept none, and a game it alters is refused
/// by its final hash.
pub static KEPT_SCRIPTS: &[KeptScript] = &[
    (
        c097_2_wishing_well::ID,
        "v0.3.7b",
        c097_2_wishing_well::script_v0_3_7b,
    ),
    (c097_8_prison::ID, "v0.3.7b", c097_8_prison::script_v0_3_7b),
    (
        c074_montana_giant::ID,
        "v0.3.7b",
        c074_montana_giant::script_v0_3_7b,
    ),
    (
        c050_k_pop_fanatic::ID,
        "v0.3.7b",
        c050_k_pop_fanatic::script_v0_3_7b,
    ),
    (
        c049_snom_bunny_mind_control::ID,
        "v0.3.7b",
        c049_snom_bunny_mind_control::script_v0_3_7b,
    ),
    (
        c088_siphon_squad::ID,
        "v0.3.7b",
        c088_siphon_squad::script_v0_3_7b,
    ),
    (
        c017_true_craft_a_card::ID,
        "v0.3.7b",
        c017_true_craft_a_card::script_v0_3_7b,
    ),
    (
        c037_cn_in_a_bottle::ID,
        "v0.3.7b",
        c037_cn_in_a_bottle::script_v0_3_7b,
    ),
    (
        c040_appropriations::ID,
        "v0.3.7b",
        c040_appropriations::script_v0_3_7b,
    ),
];

/// One entry of `KEPT_SCRIPTS`: the card's `ID`, the shipped version it is kept under, and the `fn`
/// that builds the scripts that version ran.
pub type KeptScript = (&'static str, &'static str, fn() -> CardScripts);

/// `history.json`: every shipped version in order, the definition patches, and each carried version.
#[derive(Deserialize, Default)]
struct History {
    #[serde(default)]
    order: Vec<String>,
    #[serde(default)]
    patches: Vec<DefPatch>,
    #[serde(default)]
    versions: Vec<VersionDelta>,
}

/// What turns a catalog entry back into a snapshot's: top-level keys set to the snapshot's values,
/// and keys the snapshot's entry did not have.
#[derive(Deserialize, Default)]
struct DefPatch {
    #[serde(default)]
    set: Map<String, Value>,
    #[serde(default)]
    unset: Vec<String>,
}

/// One carried version: the catalog ids its snapshot lacks, and each id that differs, by patch.
#[derive(Deserialize, Default)]
struct VersionDelta {
    #[serde(default)]
    version: String,
    #[serde(default)]
    absent: IndexSet<String>,
    #[serde(default)]
    changed: IndexMap<String, usize>,
}

static HISTORY: LazyLock<History> = LazyLock::new(|| {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/history.json")))
        .unwrap_or_else(|error| panic!("crates/cards/build.rs's history.json: {error}"))
});

/// Each carried version's catalog, in `HISTORY.versions`' order, built on first use.
static BUILT: LazyLock<Vec<OnceLock<ShippedCatalog>>> =
    LazyLock::new(|| HISTORY.versions.iter().map(|_| OnceLock::new()).collect());

/// R767000: every shipped version this build carries, oldest first: all but the newest.
pub fn carried_versions() -> Vec<&'static str> {
    HISTORY
        .versions
        .iter()
        .filter(|delta| delta.version != crate::CATALOG_VERSION)
        .map(|delta| delta.version.as_str())
        .collect()
}

/// R767000: the catalog of a carried version, `None` for the build's own version or any it does not
/// carry. What `register_all` registers with the engine.
pub fn shipped_catalog(version: &str) -> Option<&'static ShippedCatalog> {
    if version == crate::CATALOG_VERSION {
        return None;
    }
    let at = HISTORY
        .versions
        .iter()
        .position(|delta| delta.version == version)?;
    Some(BUILT[at].get_or_init(|| build(&HISTORY.versions[at])))
}

/// R388: every shipped catalog version from `patches.json`, in release order.
pub fn shipped_versions() -> Vec<&'static str> {
    HISTORY.order.iter().map(String::as_str).collect()
}

/// R388: the catalog of any shipped version in `patches.json`, exact snapshot.
pub fn catalog_at(version: &str) -> Option<&'static ShippedCatalog> {
    let at = HISTORY
        .versions
        .iter()
        .position(|delta| delta.version == version)?;
    Some(BUILT[at].get_or_init(|| build(&HISTORY.versions[at])))
}

/// R388: the card definitions of a shipped catalog version: the carried snapshot for any version
/// in `patches.json`. `None` for any version `patches.json` does not hold.
pub fn catalog_defs_at(version: &str) -> Option<&'static CardDefs> {
    catalog_at(version).map(|catalog| catalog.defs())
}

/// A carried version's catalog: `CATALOG` in order without the ids it lacked, each changed id's raw
/// entry patched back and read as a card, and the scripts kept for it.
fn build(delta: &VersionDelta) -> ShippedCatalog {
    let raw: Map<String, Value> = serde_json::from_str(catalog_json())
        .unwrap_or_else(|error| panic!("crates/cards/catalog.json: {error}"));
    let mut defs = CardDefs::new();
    for (id, def) in CATALOG.iter() {
        if delta.absent.contains(id) {
            continue;
        }
        let def = match delta.changed.get(id) {
            None => def.clone(),
            Some(&at) => patched(&delta.version, id, raw.get(id), &HISTORY.patches[at]),
        };
        defs.insert(id.clone(), def);
    }
    let scripts = defs
        .keys()
        .filter_map(|id| {
            covering(&HISTORY.order, KEPT_SCRIPTS, &delta.version, id).map(|script| (id.clone(), script()))
        })
        .collect();
    ShippedCatalog::new(&delta.version, defs, scripts)
}

/// `version`'s definition of `id`: the catalog's raw entry with `patch` applied.
fn patched(version: &str, id: &str, raw: Option<&Value>, patch: &DefPatch) -> CardDef {
    let mut entry = raw.and_then(Value::as_object).cloned().unwrap_or_default();
    for key in &patch.unset {
        entry.remove(key);
    }
    for (key, value) in &patch.set {
        entry.insert(key.clone(), value.clone());
    }
    serde_json::from_value(Value::Object(entry)).unwrap_or_else(|error| {
        panic!("crates/cards/patches/{version}.json: \"{id}\" does not read as a card: {error}")
    })
}

/// R767000: the script kept for `id` that `version` ran: among `id`'s kept scripts, the one kept at
/// the earliest version at or after `version` in `order`, since a script is kept under the last
/// version that ran it. `None` when none is, or when `order` does not hold `version`.
fn covering<'a, T>(order: &[String], kept: &'a [(&str, &str, T)], version: &str, id: &str) -> Option<&'a T> {
    let from = order.iter().position(|listed| listed == version)?;
    kept.iter()
        .filter(|(kept_id, _, _)| *kept_id == id)
        .filter_map(|(_, at, script)| Some((order.iter().position(|listed| listed == at)?, script)))
        .filter(|(at, _)| *at >= from)
        .min_by_key(|(at, _)| *at)
        .map(|(_, script)| script)
}

/// R767000: whether `id` has a script on `version`: one kept for it, or the build's own.
pub fn has_script_at(version: &str, id: &str) -> bool {
    covering(&HISTORY.order, KEPT_SCRIPTS, version, id).is_some()
        || REGISTRY.iter().any(|(registered, _)| *registered == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r767000_a_kept_script_covers_its_version_and_those_before_back_to_the_previous() {
        let order: Vec<String> = ["v1", "v2", "v3", "v4", "v5"].map(String::from).to_vec();
        let kept = [("a", "v2", 1), ("a", "v4", 2), ("b", "v1", 3)];
        let at = |version: &str, id: &str| covering(&order, &kept, version, id).copied();
        assert_eq!(at("v1", "a"), Some(1));
        assert_eq!(at("v2", "a"), Some(1));
        assert_eq!(at("v3", "a"), Some(2));
        assert_eq!(at("v4", "a"), Some(2));
        assert_eq!(at("v5", "a"), None, "after the last kept, the build's own");
        assert_eq!(at("v1", "b"), Some(3));
        assert_eq!(at("v2", "b"), None);
        assert_eq!(at("v1", "c"), None);
        assert_eq!(at("v0", "a"), None, "a version the order does not hold");
    }

    #[test]
    fn r767000_the_build_carries_every_shipped_version_but_its_own() {
        let patches: Value =
            serde_json::from_str(include_str!("../patches/patches.json")).expect("patches.json is JSON");
        let shipped: Vec<&str> = patches
            .as_array()
            .expect("patches.json is a list")
            .iter()
            .filter_map(|patch| patch.get("version").and_then(Value::as_str))
            .collect();
        let (own, older) = shipped.split_last().expect("a shipped patch");
        assert_eq!(*own, crate::CATALOG_VERSION);
        assert_eq!(carried_versions(), older.to_vec());
        assert!(shipped_catalog(own).is_none());
        assert!(shipped_catalog("v0.0.0-unknown").is_none());

        // The oldest is rebuilt, entry for entry, as its snapshot holds it.
        let oldest = shipped_catalog("v0.1.0").expect("v0.1.0 is carried");
        assert_eq!(oldest.version(), "v0.1.0");
        let snapshot: CardDefs =
            serde_json::from_str(include_str!("../patches/v0.1.0.json")).expect("v0.1.0 reads as cards");
        assert_eq!(
            serde_json::to_value(oldest.defs()).expect("serialises"),
            serde_json::to_value(&snapshot).expect("serialises")
        );
        assert!(
            std::ptr::eq(oldest, shipped_catalog("v0.1.0").expect("carried")),
            "built once"
        );
        assert!(older.iter().all(|version| has_script_at(version, "core-001")));
    }

    #[test]
    fn r767000_a_kept_script_runs_only_in_its_fold() {
        crate::register_all();
        // Big D-fender kept as Anti-oneshot Armor's script, as a patch that changed what it does
        // would keep its old one (R388).
        let kept = jackioh_engine::scripts::registered_entry("core-073")
            .cloned()
            .expect("Anti-oneshot Armor has a script");
        let shipped: &'static ShippedCatalog = Box::leak(Box::new(ShippedCatalog::new(
            "v-test",
            CATALOG.clone(),
            IndexMap::from([("core-001".to_string(), kept)]),
        )));
        let anti_oneshot = || {
            jackioh_engine::scripts::registered_entry("core-001")
                .and_then(|entry| entry.base.static_flags.as_ref())
                .and_then(|flags| flags.anti_oneshot)
                == Some(true)
        };
        assert!(!anti_oneshot());
        assert!(jackioh_engine::with_shipped(Some(shipped), || {
            assert_eq!(jackioh_engine::catalog::catalog_version(), "v-test");
            anti_oneshot()
        }));
        assert!(!anti_oneshot());
        assert_eq!(jackioh_engine::catalog::catalog_version(), crate::CATALOG_VERSION);
    }

    #[test]
    fn r388_catalog_defs_at_rebuilds_every_shipped_snapshot_exact() {
        let patches: Value =
            serde_json::from_str(include_str!("../patches/patches.json")).expect("patches.json is JSON");
        let shipped: Vec<&str> = patches
            .as_array()
            .expect("patches.json is a list")
            .iter()
            .filter_map(|patch| patch.get("version").and_then(Value::as_str))
            .collect();
        assert_eq!(shipped_versions(), shipped);

        // The oldest snapshot is rebuilt, entry for entry.
        let oldest = catalog_defs_at("v0.1.0").expect("v0.1.0 is carried");
        let snapshot_v0_1_0: CardDefs =
            serde_json::from_str(include_str!("../patches/v0.1.0.json")).expect("v0.1.0 reads as cards");
        assert_eq!(
            serde_json::to_value(oldest).expect("serialises"),
            serde_json::to_value(&snapshot_v0_1_0).expect("serialises")
        );

        // The newest is carried too, and v0.3.7b, the newest when this was written, is rebuilt as its
        // snapshot holds it, without the pending patches' changes `catalog.json` already has.
        assert!(
            catalog_defs_at(crate::CATALOG_VERSION).is_some(),
            "the newest is carried"
        );
        let v0_3_7b = catalog_defs_at("v0.3.7b").expect("v0.3.7b is carried");
        let snapshot_v0_3_7b: CardDefs =
            serde_json::from_str(include_str!("../patches/v0.3.7b.json")).expect("v0.3.7b reads as cards");
        assert_eq!(
            serde_json::to_value(v0_3_7b).expect("serialises"),
            serde_json::to_value(&snapshot_v0_3_7b).expect("serialises")
        );

        assert!(catalog_defs_at("v0.0.0-unknown").is_none());
        assert!(catalog_at("v0.0.0-unknown").is_none());
    }
}
