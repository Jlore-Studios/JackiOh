//! The cosmetics registry (#1295, SPEC §11 R1295000): every cosmetic a player can pick, in
//! `crates/server/cosmetics.json`, with its id, kind, name, rarity, whether it is its kind's
//! default and whether it is a starter (§14.2). The server reads it here, so the pack roll
//! ([`pack_cosmetics`], R1301002) and ownership name the same ids, and
//! `tests/export_cosmetics.rs` writes it into `apps/web/src/wire/generated/cosmetics.ts` for the
//! client, which draws them. No I/O: the file is compiled in.
//!
//! [`problems`] is the registry's check: no id twice, every id a lower-case slug short enough to
//! travel as a settings text (R633, the client keeps a player's picks in a settings group), and
//! exactly one default per kind. A cosmetic never enters `PlayerView`, an action, the log or a
//! replay hash: none of it is a rule (CLAUDE.md rule 7).

use std::collections::BTreeSet;

use jackioh_engine::wire::PrintedRarity;
use serde::{Deserialize, Serialize};

use crate::config::PLAYER_SETTINGS_TEXT_MAX_LENGTH;

/// The registry's text, compiled in.
pub const COSMETICS_JSON: &str = include_str!("../cosmetics.json");

/// The slot a cosmetic fills: on a card, a foil over the art, a finish on the print and a border
/// round the frame (R1295000); on a deck, the playmat (`Table`), the ambient backdrop behind the
/// board (`Field`) and the back of its face-down cards and deck (`CardBack`) (R1297000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CosmeticKind {
    Foil,
    Finish,
    Border,
    Table,
    Field,
    CardBack,
}

impl CosmeticKind {
    /// Every kind, in the order the settings list them.
    pub const ALL: [Self; 6] = [
        Self::Foil,
        Self::Finish,
        Self::Border,
        Self::Table,
        Self::Field,
        Self::CardBack,
    ];

    /// A per-card kind, owned for one card (§14.2): a foil, a finish or a border. The deck slots'
    /// kinds are account cosmetics.
    pub fn is_per_card(self) -> bool {
        matches!(self, Self::Foil | Self::Finish | Self::Border)
    }
}

/// How rare a cosmetic is, so a pack can later roll per-card cosmetics by rarity (R1295000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CosmeticRarity {
    Common,
    Rare,
    Epic,
    Legendary,
}

impl CosmeticRarity {
    /// Every rarity, commonest first.
    pub const ALL: [Self; 4] = [Self::Common, Self::Rare, Self::Epic, Self::Legendary];

    /// The same rarity on §5's scale, which a duplicate's cosmetic roll draws on (R1301002).
    pub fn printed(self) -> PrintedRarity {
        match self {
            Self::Common => PrintedRarity::Common,
            Self::Rare => PrintedRarity::Rare,
            Self::Epic => PrintedRarity::Epic,
            Self::Legendary => PrintedRarity::Legendary,
        }
    }
}

/// One entry of the registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cosmetic {
    /// A lower-case slug, unique across every kind (`foil-holo-sweep`).
    pub id: String,
    pub kind: CosmeticKind,
    /// The English name the settings list it by.
    pub name: String,
    pub rarity: CosmeticRarity,
    /// The kind's default: what a card wears when nothing else is picked, and what an id a client
    /// does not know draws as. Exactly one per kind.
    pub default: bool,
    /// §14.2: owned by every account on every card without a ledger row, so no pack drops it
    /// (R1301002). Every entry #1295 and #1297 shipped is one; a cosmetic added later leaves it
    /// false and must be owned (§14.12).
    #[serde(default)]
    pub starter: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    cosmetics: Vec<Cosmetic>,
}

/// An id is a lower-case letter, then lower-case letters, digits and hyphens.
fn is_slug(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Everything wrong with `cosmetics`, one sentence each; empty when the registry holds (R1295000).
pub fn problems(cosmetics: &[Cosmetic]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for cosmetic in cosmetics {
        let id = cosmetic.id.as_str();
        if !seen.insert(id) {
            out.push(format!("the id {id:?} is listed twice"));
        }
        if !is_slug(id) {
            out.push(format!(
                "the id {id:?} is not a lower-case slug (a letter, then letters, digits and hyphens)"
            ));
        }
        if id.chars().count() > PLAYER_SETTINGS_TEXT_MAX_LENGTH {
            out.push(format!(
                "the id {id:?} is longer than a setting's {PLAYER_SETTINGS_TEXT_MAX_LENGTH} characters (R633)"
            ));
        }
    }
    for kind in CosmeticKind::ALL {
        let defaults = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind == kind && cosmetic.default)
            .count();
        if defaults != 1 {
            out.push(format!("the kind {kind:?} has {defaults} defaults, not one"));
        }
    }
    out
}

/// The registry in `json`, or every problem with it joined by "; ".
pub fn parse(json: &str) -> Result<Vec<Cosmetic>, String> {
    let file: RegistryFile = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let found = problems(&file.cosmetics);
    if found.is_empty() {
        Ok(file.cosmetics)
    } else {
        Err(found.join("; "))
    }
}

/// The registry this build ships.
pub fn shipped() -> Result<Vec<Cosmetic>, String> {
    parse(COSMETICS_JSON)
}

/// The cosmetics a duplicate may drop (R1301002): the per-card ones that are neither a default
/// nor a starter, in the registry's order.
pub fn pack_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| cosmetic.kind.is_per_card() && !cosmetic.default && !cosmetic.starter)
        .cloned()
        .collect()
}

/// Whether the shipped registry lists `id` under `kind`: a deck's pick is kept only when it is
/// (R1297000).
pub fn is_id_of(kind: CosmeticKind, id: &str) -> bool {
    shipped().is_ok_and(|cosmetics| {
        cosmetics
            .iter()
            .any(|cosmetic| cosmetic.kind == kind && cosmetic.id == id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, kind: CosmeticKind, default: bool) -> Cosmetic {
        Cosmetic {
            id: id.to_owned(),
            kind,
            name: id.to_owned(),
            rarity: CosmeticRarity::Common,
            default,
            starter: true,
        }
    }

    /// One default per kind, and nothing else.
    fn defaults() -> Vec<Cosmetic> {
        vec![
            entry("foil-none", CosmeticKind::Foil, true),
            entry("finish-plain", CosmeticKind::Finish, true),
            entry("border-standard", CosmeticKind::Border, true),
            entry("table-tavern", CosmeticKind::Table, true),
            entry("field-none", CosmeticKind::Field, true),
            entry("card-back-standard", CosmeticKind::CardBack, true),
        ]
    }

    #[test]
    fn r1295000_the_shipped_registry_passes_with_one_default_and_three_items_per_kind() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for kind in CosmeticKind::ALL {
            let of_kind: Vec<&Cosmetic> = cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.kind == kind)
                .collect();
            assert_eq!(
                of_kind.iter().filter(|cosmetic| cosmetic.default).count(),
                1,
                "{kind:?}'s defaults"
            );
            assert_eq!(
                of_kind.iter().filter(|cosmetic| !cosmetic.default).count(),
                3,
                "{kind:?}'s items"
            );
        }
        assert!(problems(&defaults()).is_empty(), "{:?}", problems(&defaults()));
    }

    #[test]
    fn r1297000_the_deck_slots_have_one_default_and_three_items_each() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for kind in [CosmeticKind::Table, CosmeticKind::Field, CosmeticKind::CardBack] {
            let of_kind: Vec<&Cosmetic> = cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.kind == kind)
                .collect();
            assert_eq!(
                of_kind.iter().filter(|cosmetic| cosmetic.default).count(),
                1,
                "{kind:?}'s defaults"
            );
            assert_eq!(
                of_kind.iter().filter(|cosmetic| !cosmetic.default).count(),
                3,
                "{kind:?}'s items"
            );
        }
        // A kind's name travels as the camel-case word the web's settings group keys on.
        assert_eq!(
            serde_json::to_string(&CosmeticKind::CardBack).unwrap(),
            "\"cardBack\""
        );
        assert_eq!(
            serde_json::to_string(&CosmeticKind::Border).unwrap(),
            "\"border\""
        );
    }

    #[test]
    fn r1297000_is_id_of_knows_each_kind_s_ids_only() {
        assert!(is_id_of(CosmeticKind::Table, "table-felt"));
        assert!(is_id_of(CosmeticKind::Field, "field-none"));
        assert!(is_id_of(CosmeticKind::CardBack, "card-back-blueprint"));
        assert!(!is_id_of(CosmeticKind::Table, "field-rain"));
        assert!(!is_id_of(CosmeticKind::CardBack, "card-back-gold"));
        assert!(!is_id_of(CosmeticKind::Border, ""));
    }

    #[test]
    fn r1295000_a_doubled_id_is_refused() {
        let mut cosmetics = defaults();
        cosmetics.push(entry("foil-none", CosmeticKind::Foil, false));
        assert_eq!(
            problems(&cosmetics),
            vec!["the id \"foil-none\" is listed twice".to_owned()]
        );
        // An id is unique across kinds, not only within one.
        let mut across = defaults();
        across.push(entry("finish-plain", CosmeticKind::Border, false));
        assert_eq!(
            problems(&across),
            vec!["the id \"finish-plain\" is listed twice".to_owned()]
        );
    }

    #[test]
    fn r1295000_a_kind_with_no_default_is_refused() {
        let mut cosmetics = defaults();
        cosmetics[1].default = false;
        assert_eq!(
            problems(&cosmetics),
            vec!["the kind Finish has 0 defaults, not one".to_owned()]
        );
        let json = r#"{"cosmetics":[
            {"id":"foil-none","kind":"foil","name":"No foil","rarity":"Common","default":true},
            {"id":"finish-plain","kind":"finish","name":"Plain","rarity":"Common","default":true},
            {"id":"table-tavern","kind":"table","name":"Tavern table","rarity":"Common","default":true},
            {"id":"field-none","kind":"field","name":"No backdrop","rarity":"Common","default":true},
            {"id":"card-back-standard","kind":"cardBack","name":"Standard back","rarity":"Common","default":true}
        ]}"#;
        assert_eq!(
            parse(json),
            Err("the kind Border has 0 defaults, not one".to_owned())
        );
    }

    #[test]
    fn r1295000_a_kind_with_two_defaults_is_refused() {
        let mut cosmetics = defaults();
        cosmetics.push(entry("border-wood", CosmeticKind::Border, true));
        assert_eq!(
            problems(&cosmetics),
            vec!["the kind Border has 2 defaults, not one".to_owned()]
        );
    }

    #[test]
    fn r1295000_an_id_the_settings_cannot_carry_is_refused() {
        let mut upper = defaults();
        upper.push(entry("Foil-Gold", CosmeticKind::Foil, false));
        assert_eq!(problems(&upper).len(), 1, "{:?}", problems(&upper));
        assert!(problems(&upper)[0].contains("lower-case slug"));

        let long = format!(
            "foil-{}",
            "a".repeat(PLAYER_SETTINGS_TEXT_MAX_LENGTH + 1 - "foil-".len())
        );
        assert_eq!(long.len(), PLAYER_SETTINGS_TEXT_MAX_LENGTH + 1);
        let mut too_long = defaults();
        too_long.push(entry(&long, CosmeticKind::Foil, false));
        assert_eq!(problems(&too_long).len(), 1, "{:?}", problems(&too_long));
        assert!(problems(&too_long)[0].contains("(R633)"));

        // A field the registry does not have is refused rather than dropped.
        let extra = r#"{"cosmetics":[{"id":"foil-none","kind":"foil","name":"No foil","rarity":"Common","default":true,"owned":true}]}"#;
        assert!(parse(extra).is_err_and(|error| error.contains("unknown field")));
    }

    #[test]
    fn r1301002_pack_cosmetics_are_per_card_and_neither_default_nor_starter() {
        let dropping = |id: &str, kind: CosmeticKind| Cosmetic {
            starter: false,
            ..entry(id, kind, false)
        };
        let mut cosmetics = defaults();
        cosmetics.extend([
            entry("foil-starter", CosmeticKind::Foil, false),
            dropping("foil-new", CosmeticKind::Foil),
            dropping("finish-new", CosmeticKind::Finish),
            dropping("border-new", CosmeticKind::Border),
            dropping("table-new", CosmeticKind::Table),
            dropping("field-new", CosmeticKind::Field),
            dropping("card-back-new", CosmeticKind::CardBack),
        ]);
        // A default that is not marked a starter is still a default.
        cosmetics[0].starter = false;
        let ids: Vec<String> = pack_cosmetics(&cosmetics)
            .into_iter()
            .map(|cosmetic| cosmetic.id)
            .collect();
        assert_eq!(ids, vec!["foil-new", "finish-new", "border-new"]);
        assert_eq!(
            CosmeticRarity::ALL.map(CosmeticRarity::printed),
            [
                PrintedRarity::Common,
                PrintedRarity::Rare,
                PrintedRarity::Epic,
                PrintedRarity::Legendary
            ]
        );
    }

    #[test]
    fn r1301002_every_shipped_cosmetic_is_a_starter() {
        // R1295000 and R1297000 unlock every shipped item for every account (§14.2), so no pack
        // drops one yet and every duplicate goes to the bars.
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        assert!(cosmetics.iter().all(|cosmetic| cosmetic.starter));
        assert!(pack_cosmetics(&cosmetics).is_empty());
        // An entry that leaves the mark out is no starter.
        let unmarked = r#"{"cosmetics":[{"id":"foil-none","kind":"foil","name":"No foil","rarity":"Common","default":true}]}"#;
        let file: RegistryFile = serde_json::from_str(unmarked).unwrap_or_else(|error| panic!("{error}"));
        assert!(!file.cosmetics[0].starter);
    }
}
