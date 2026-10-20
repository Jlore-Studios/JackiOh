//! The cosmetics registry (#1295, SPEC §11 R1295000): every cosmetic a player can pick, in
//! `crates/server/cosmetics.json`, with its id, kind, name, rarity, whether it is its kind's
//! default and whether it is a starter (§14.2). The server reads it here, so the pack roll
//! ([`pack_cosmetics`], R1301002) and ownership name the same ids, and
//! `tests/export_cosmetics.rs` writes it into `apps/web/src/wire/generated/cosmetics.ts` for the
//! client, which draws them. No I/O: the file is compiled in.
//!
//! [`problems`] is the registry's check: no id twice, every id a lower-case slug short enough to
//! travel as a settings text (R633, the client keeps a player's picks in a settings group), and
//! exactly one default per kind a player picks, and a chase exactly when Legendary or Mythic
//! (R1333000). A cosmetic never enters `PlayerView`, an action, the log or a replay hash: none of
//! it is a rule (CLAUDE.md rule 7).
//!
//! R1333000: a cosmetic's rarity is [`CosmeticRarity`], its own scale, never its card's: nothing
//! here converts it to or from the catalog's `PrintedRarity`, so no code can tie one to the other.
//!
//! R1313000: the full-art faces of the hero portraits' cards (R1017001) are cosmetics too, one per
//! card, each naming its card. A player owns one on its card and never picks one, so the kind has
//! no default, and the check refuses a full art with no card, a card with two, a default one and a
//! card named by any other kind.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::config::PLAYER_SETTINGS_TEXT_MAX_LENGTH;

/// The registry's text, compiled in.
pub const COSMETICS_JSON: &str = include_str!("../cosmetics.json");

/// The slot a cosmetic fills: on a card, a foil over the art, a finish on the print and a border
/// round the frame (R1295000); on a deck, the playmat (`Table`), the ambient backdrop behind the
/// board (`Field`) and the back of its face-down cards and deck (`CardBack`) (R1297000); and a
/// hero portrait's card's full-art face (`FullArt`), owned on its card and never picked (R1313000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CosmeticKind {
    Foil,
    Finish,
    Border,
    Table,
    Field,
    CardBack,
    FullArt,
}

impl CosmeticKind {
    /// The kinds a player picks one of, in the order the settings list them, each with exactly one
    /// default; `FullArt` is none (R1313000).
    pub const PICKED: [Self; 6] = [
        Self::Foil,
        Self::Finish,
        Self::Border,
        Self::Table,
        Self::Field,
        Self::CardBack,
    ];

    /// A per-card kind, owned for one card (§14.2): a foil, a finish, a border or a full art
    /// (R1313000). The deck slots' kinds are account cosmetics.
    pub fn is_per_card(self) -> bool {
        matches!(self, Self::Foil | Self::Finish | Self::Border | Self::FullArt)
    }
}

/// How rare a cosmetic is (R1295000): its own scale, commonest first, never tied to the rarity of
/// the card it goes on. It names its rarities as the catalog's `PrintedRarity` does, so both travel
/// as the same words, but nothing converts one into the other (§14.2, R1333000).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CosmeticRarity {
    Common,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl CosmeticRarity {
    /// Every rarity, commonest first.
    pub const ALL: [Self; 5] = [
        Self::Common,
        Self::Rare,
        Self::Epic,
        Self::Legendary,
        Self::Mythic,
    ];

    /// The rarity's name, as the registry, the wire and a token's inventory key spell it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Common => "Common",
            Self::Rare => "Rare",
            Self::Epic => "Epic",
            Self::Legendary => "Legendary",
            Self::Mythic => "Mythic",
        }
    }

    /// The rarity `text` names, as [`Self::as_str`] spells it.
    pub fn named(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|rarity| rarity.as_str() == text)
    }

    /// The chase tier, Legendary and Mythic (§14.2, R1333000).
    pub fn is_chase(self) -> bool {
        self >= Self::Legendary
    }

    /// The rarities a cosmetic pack slot that rolled `self` reads, in order (R1306001): its own,
    /// then each lower one from the nearest down, then each higher one from the nearest up.
    pub fn fallback_order(self) -> Vec<Self> {
        let mut order = vec![self];
        order.extend(Self::ALL.iter().rev().copied().filter(|rarity| *rarity < self));
        order.extend(Self::ALL.iter().copied().filter(|rarity| *rarity > self));
        order
    }

    /// The rarity's place on the cosmetics' scale, commonest first (R1305003): a cosmetic token
    /// buys a cosmetic at its tier or below.
    pub fn tier(self) -> u8 {
        match self {
            Self::Common => 0,
            Self::Rare => 1,
            Self::Epic => 2,
            Self::Legendary => 3,
            Self::Mythic => 4,
        }
    }
}

impl fmt::Display for CosmeticRarity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
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
    /// does not know draws as. Exactly one per kind a player picks, and never a full art.
    pub default: bool,
    /// §14.2: owned by every account on every card without a ledger row, so no pack drops it
    /// (R1301002). Every entry #1295 and #1297 shipped is one; a cosmetic added later leaves it
    /// false and must be owned (§14.12).
    #[serde(default)]
    pub starter: bool,
    /// §14.2: a chase cosmetic. The mark agrees with the rarity, true exactly for a Legendary or
    /// Mythic one, and [`problems`] refuses a registry where it does not (R1333000). No token pays
    /// for one; the Chase Shard bar buys one, and one drops when a duplicate's roll or a cosmetic
    /// pack slot lands on its tier (§14.6).
    #[serde(default)]
    pub chase: bool,
    /// The card a full art belongs to, by catalog id; no other kind names one (R1313000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    cosmetics: Vec<Cosmetic>,
}

/// An id is a lower-case letter, then lower-case letters, digits and hyphens (the quest pool's too).
pub(crate) fn is_slug(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|first| first.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Everything wrong with `cosmetics`, one sentence each; empty when the registry holds (R1295000).
pub fn problems(cosmetics: &[Cosmetic]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut full_art_cards = BTreeSet::new();
    for cosmetic in cosmetics {
        let id = cosmetic.id.as_str();
        if !seen.insert(id) {
            out.push(format!("the id {id:?} is listed twice"));
        }
        // R1313000: a full art names its card, one per card, and is no default; nothing else names one.
        match (cosmetic.kind, cosmetic.card.as_deref()) {
            (CosmeticKind::FullArt, None) => out.push(format!("the full art {id:?} names no card")),
            (CosmeticKind::FullArt, Some(card)) => {
                if !full_art_cards.insert(card) {
                    out.push(format!("the card {card:?} has two full arts"));
                }
            }
            (kind, Some(card)) => out.push(format!(
                "the {kind:?} {id:?} names the card {card:?}, which only a full art does"
            )),
            (_, None) => {}
        }
        // R1333000: a chase is exactly a Legendary or Mythic cosmetic.
        if cosmetic.chase != cosmetic.rarity.is_chase() {
            out.push(format!(
                "the cosmetic {id:?} is {} and its chase mark is {}, but a cosmetic is a chase exactly when it is Legendary or Mythic",
                cosmetic.rarity, cosmetic.chase
            ));
        }
        if cosmetic.kind == CosmeticKind::FullArt && cosmetic.default {
            out.push(format!("the full art {id:?} is a default, which no full art is"));
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
    for kind in CosmeticKind::PICKED {
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
/// nor a starter, in the registry's order. One that names a card, a full art, drops on that card
/// alone (`economy::roll::duplicate_grant`, R1313000).
pub fn pack_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| cosmetic.kind.is_per_card() && !cosmetic.default && !cosmetic.starter)
        .cloned()
        .collect()
}

/// The cosmetics a cosmetic pack may drop (R1306000): every entry, per-card or account, that is
/// not a default or a starter, in the registry's order. A chase drops when a slot lands on its
/// tier (R1333000), and one that names a card, a full art, drops on that card alone (R1313000).
pub fn cosmetic_pack_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| !cosmetic.default && !cosmetic.starter)
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
            chase: false,
            card: None,
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
        for kind in CosmeticKind::PICKED {
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
    fn r1313000_the_six_full_arts_are_epic_one_per_hero_card_and_no_pick() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let full_arts: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .collect();
        let mut cards: Vec<&str> = full_arts
            .iter()
            .filter_map(|cosmetic| cosmetic.card.as_deref())
            .collect();
        cards.sort_unstable();
        // R1017001's six: Big D-fender, Gary the Gambler, Mr. Vanilla, Tempo Timmy, Duplicating
        // Felinors and Jlockeed Shredder-10.
        assert_eq!(
            cards,
            [
                "core-001", "core-004", "core-008", "core-011", "core-012", "core-013"
            ]
        );
        for cosmetic in &full_arts {
            // Epic on the cosmetics' own scale, whatever its card's rarity.
            assert_eq!(cosmetic.rarity, CosmeticRarity::Epic, "{}", cosmetic.id);
            assert!(!cosmetic.default, "{}", cosmetic.id);
        }
        // No other kind names a card.
        assert!(
            cosmetics
                .iter()
                .all(|cosmetic| (cosmetic.kind == CosmeticKind::FullArt) == cosmetic.card.is_some())
        );
        assert_eq!(
            serde_json::to_string(&CosmeticKind::FullArt).unwrap(),
            "\"fullArt\""
        );
        assert!(!CosmeticKind::PICKED.contains(&CosmeticKind::FullArt));
    }

    #[test]
    fn r1313000_a_full_art_needs_its_card_and_only_a_full_art_names_one() {
        let full_art = |id: &str, card: Option<&str>| Cosmetic {
            card: card.map(str::to_owned),
            ..entry(id, CosmeticKind::FullArt, false)
        };
        let mut fine = defaults();
        fine.push(full_art("full-art-a", Some("core-001")));
        fine.push(full_art("full-art-b", Some("core-004")));
        assert!(problems(&fine).is_empty(), "{:?}", problems(&fine));

        let mut no_card = defaults();
        no_card.push(full_art("full-art-a", None));
        assert_eq!(
            problems(&no_card),
            vec!["the full art \"full-art-a\" names no card".to_owned()]
        );

        let mut twice = defaults();
        twice.push(full_art("full-art-a", Some("core-001")));
        twice.push(full_art("full-art-b", Some("core-001")));
        assert_eq!(
            problems(&twice),
            vec!["the card \"core-001\" has two full arts".to_owned()]
        );

        let mut foil_with_card = defaults();
        foil_with_card.push(Cosmetic {
            card: Some("core-001".to_owned()),
            ..entry("foil-gold", CosmeticKind::Foil, false)
        });
        assert_eq!(
            problems(&foil_with_card),
            vec!["the Foil \"foil-gold\" names the card \"core-001\", which only a full art does".to_owned()]
        );

        let mut a_default = defaults();
        a_default.push(Cosmetic {
            default: true,
            ..full_art("full-art-a", Some("core-001"))
        });
        assert_eq!(
            problems(&a_default),
            vec!["the full art \"full-art-a\" is a default, which no full art is".to_owned()]
        );

        // The card travels in the file as `card`, and a kind that names none leaves it out.
        let json = r#"{"cosmetics":[{"id":"full-art-a","kind":"fullArt","name":"A","rarity":"Epic","default":false,"card":"core-001"}]}"#;
        let parsed: RegistryFile = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.cosmetics[0].card.as_deref(), Some("core-001"));
        assert!(
            !serde_json::to_string(&defaults()[0])
                .unwrap()
                .contains("\"card\"")
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
            Cosmetic {
                card: Some("core-001".to_owned()),
                ..dropping("full-art-new", CosmeticKind::FullArt)
            },
        ]);
        // A default that is not marked a starter is still a default.
        cosmetics[0].starter = false;
        let ids: Vec<String> = pack_cosmetics(&cosmetics)
            .into_iter()
            .map(|cosmetic| cosmetic.id)
            .collect();
        assert_eq!(ids, vec!["foil-new", "finish-new", "border-new", "full-art-new"]);
    }

    #[test]
    fn r1333000_a_chase_is_a_legendary_or_mythic_cosmetic_and_nothing_else() {
        assert_eq!(
            CosmeticRarity::ALL.map(CosmeticRarity::is_chase),
            [false, false, false, true, true]
        );
        let marked = |id: &str, rarity: CosmeticRarity, chase: bool| Cosmetic {
            rarity,
            chase,
            ..entry(id, CosmeticKind::Foil, false)
        };
        for (rarity, chase) in [
            (CosmeticRarity::Common, true),
            (CosmeticRarity::Epic, true),
            (CosmeticRarity::Legendary, false),
            (CosmeticRarity::Mythic, false),
        ] {
            let mut cosmetics = defaults();
            cosmetics.push(marked("foil-wrong", rarity, chase));
            assert_eq!(
                problems(&cosmetics),
                vec![format!(
                    "the cosmetic \"foil-wrong\" is {rarity} and its chase mark is {chase}, but a cosmetic is a chase exactly when it is Legendary or Mythic"
                )]
            );
        }
        let mut fine = defaults();
        fine.extend([
            marked("foil-rare", CosmeticRarity::Rare, false),
            marked("foil-legendary", CosmeticRarity::Legendary, true),
            marked("foil-mythic", CosmeticRarity::Mythic, true),
        ]);
        assert!(problems(&fine).is_empty(), "{:?}", problems(&fine));

        // The shipped chases are its two Legendary starters, which every account owns already.
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let chases: Vec<&Cosmetic> = cosmetics.iter().filter(|cosmetic| cosmetic.chase).collect();
        assert_eq!(
            chases
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            ["foil-cracked-ice", "finish-gilded-edge"]
        );
        assert!(chases.iter().all(|cosmetic| cosmetic.starter));
    }

    #[test]
    fn r1333000_a_cosmetic_rarity_travels_as_its_own_name() {
        for rarity in CosmeticRarity::ALL {
            assert_eq!(CosmeticRarity::named(rarity.as_str()), Some(rarity));
            assert_eq!(rarity.to_string(), rarity.as_str());
            assert_eq!(
                serde_json::to_string(&rarity).unwrap(),
                format!("\"{}\"", rarity.as_str())
            );
        }
        assert_eq!(CosmeticRarity::named("common"), None);
        assert_eq!(CosmeticRarity::named(""), None);
        // Commonest first, as the tiers count.
        assert!(CosmeticRarity::ALL.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            CosmeticRarity::ALL
                .windows(2)
                .all(|pair| pair[0].tier() < pair[1].tier())
        );
        assert_eq!(
            CosmeticRarity::Epic.fallback_order(),
            [
                CosmeticRarity::Epic,
                CosmeticRarity::Rare,
                CosmeticRarity::Common,
                CosmeticRarity::Legendary,
                CosmeticRarity::Mythic
            ]
        );
    }

    #[test]
    fn r1301002_every_shipped_cosmetic_but_the_full_arts_is_a_starter() {
        // R1295000 and R1297000 unlock every item they shipped for every account (§14.2), so no
        // pack drops one; the six full arts are owned (R1313000), so they are all a pack drops.
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        assert!(
            cosmetics
                .iter()
                .all(|cosmetic| cosmetic.starter == (cosmetic.kind != CosmeticKind::FullArt)),
            "{cosmetics:?}"
        );
        let full_arts: Vec<Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .cloned()
            .collect();
        assert_eq!(full_arts.len(), 6);
        assert_eq!(pack_cosmetics(&cosmetics), full_arts);
        // An entry that leaves the mark out is no starter.
        let unmarked = r#"{"cosmetics":[{"id":"foil-none","kind":"foil","name":"No foil","rarity":"Common","default":true}]}"#;
        let file: RegistryFile = serde_json::from_str(unmarked).unwrap_or_else(|error| panic!("{error}"));
        assert!(!file.cosmetics[0].starter);
    }
}
