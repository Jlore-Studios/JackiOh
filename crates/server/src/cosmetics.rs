//! The cosmetics registry (#1295, SPEC §11 R1295000): every cosmetic a player can pick, in
//! `crates/server/cosmetics.json`. The server reads it here, so the pack roll ([`pack_cosmetics`],
//! R1301002) and ownership name the same ids, and `tests/export_cosmetics.rs` writes it into
//! `apps/web/src/wire/generated/cosmetics.ts` for the client, which draws them.
//!
//! [`problems`] is the registry's check: no id twice, every id a lower-case slug short enough to
//! travel as a settings text (R633, the client keeps a player's picks in a settings group), exactly
//! one default per kind a player picks, and a chase exactly when Legendary or Mythic (R1333000). A
//! cosmetic never enters `PlayerView`, an action, the log or a replay hash (CLAUDE.md rule 7).
//!
//! R1827000: the hero portraits (R641) are account cosmetics, one `portrait-<id>` entry per
//! `PortraitId`, `portrait-vanilla` the kind's default and a starter and the other five owned, so
//! they are what the global roll drops.
//!
//! R1833000: #1833 appends 36 cosmetics, six of each kind a player picks, none a default or a
//! starter, so every account earns them (§14.12) and none is granted to an account that exists. A
//! deck's save refuses a playmat, backdrop or card back its account does not own
//! ([`owns_account_cosmetic`]), as it does a portrait ([`owns_portrait`]).
//!
//! R1834001: every write of a pick (a deck's slots, the `cosmetics` settings group, a card's own
//! picks) is held to [`check_pick`], and a loadout freezes nothing else (`loadout::freeze_loadout`,
//! R1834002).

use std::collections::BTreeSet;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use jackioh_engine::wire::emotes::{DEFAULT_PORTRAIT, PORTRAIT_IDS, PortraitId};

use crate::config::{GLOBAL_COSMETIC_FLOOR_RARITY, PLAYER_SETTINGS_TEXT_MAX_LENGTH};
use crate::economy::store::OwnedCosmetics;

/// The registry's text, compiled in.
pub const COSMETICS_JSON: &str = include_str!("../cosmetics.json");

/// The slot a cosmetic fills: on a card, a foil, a finish and a border (R1295000); on a deck, the
/// playmat (`Table`), the ambient backdrop behind the board (`Field`) and the back of its face-down
/// cards and deck (`CardBack`) (R1297000); a hero portrait's card's full-art face (`FullArt`), owned
/// on its card and never picked (R1313000); and the deck's hero portrait (`Portrait`), an account
/// cosmetic (R641, R1827000).
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
    Portrait,
}

impl CosmeticKind {
    /// The kinds a player picks one of in the settings, in the order they list them, each with
    /// exactly one default; `FullArt` is none (R1313000), and `Portrait` is a deck's pick alone
    /// (R1827000).
    pub const PICKED: [Self; 6] = [
        Self::Foil,
        Self::Finish,
        Self::Border,
        Self::Table,
        Self::Field,
        Self::CardBack,
    ];

    /// The kinds with exactly one default: those of [`Self::PICKED`], then `Portrait`, whose
    /// default is `portrait-vanilla` (R641, R1827000).
    pub const DEFAULTED: [Self; 7] = [
        Self::Foil,
        Self::Finish,
        Self::Border,
        Self::Table,
        Self::Field,
        Self::CardBack,
        Self::Portrait,
    ];

    /// A per-card kind, owned for one card (§14.2). The deck slots' kinds and the hero portrait are
    /// account cosmetics (R1827000).
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

    /// [`Self::fallback_order`] with every rarity at or above `floor` ahead of those below it
    /// (R1306001). A cosmetic pack slot reads it, and so does a duplicate's global roll (R1661001);
    /// one with no floor passes `Common`.
    pub fn floored_fallback_order(self, floor: Self) -> Vec<Self> {
        let mut order = self.fallback_order();
        // Stable, so each side keeps `fallback_order`'s order.
        order.sort_by_key(|rarity| *rarity < floor);
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
    /// does not know draws as.
    pub default: bool,
    /// §14.2: owned by every account on every card without a ledger row, so no pack drops it
    /// (R1301002). A cosmetic added later leaves it false and must be owned (§14.12), as #1827's
    /// portraits but vanilla and #1833's 36 do (R1833000).
    #[serde(default)]
    pub starter: bool,
    /// §14.2: a chase cosmetic. The mark agrees with the rarity, true exactly for a Legendary or
    /// Mythic one (R1333000). No token pays for one; the Chase Shard bar buys one, and one drops
    /// when a duplicate's roll or a cosmetic pack slot lands on its tier (§14.6).
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
        // R1661000: an account cosmetic but its kind's default is at the global roll's floor or above.
        if !cosmetic.kind.is_per_card() && !cosmetic.default && cosmetic.rarity < GLOBAL_COSMETIC_FLOOR_RARITY
        {
            out.push(format!(
                "the account cosmetic {id:?} is {}, below GLOBAL_COSMETIC_FLOOR_RARITY's {GLOBAL_COSMETIC_FLOOR_RARITY}, which only its kind's default may be",
                cosmetic.rarity
            ));
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
        // R1827000: a portrait entry names a portrait id, and is the default exactly for vanilla.
        if cosmetic.kind == CosmeticKind::Portrait {
            if portrait_of(id).is_none() {
                out.push(format!("the portrait {id:?} names no portrait id (R641)"));
            } else if cosmetic.default != (id == portrait_cosmetic_id(DEFAULT_PORTRAIT)) {
                out.push(format!(
                    "the portrait {id:?} is a default exactly when it is \"portrait-vanilla\""
                ));
            }
        }
    }
    // R1827000: every portrait id has its entry.
    for portrait in PORTRAIT_IDS {
        let id = portrait_cosmetic_id(*portrait);
        if !cosmetics
            .iter()
            .any(|cosmetic| cosmetic.kind == CosmeticKind::Portrait && cosmetic.id == id)
        {
            out.push(format!("the portrait {portrait} has no {id:?} entry (R641)"));
        }
    }
    for kind in CosmeticKind::DEFAULTED {
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

pub fn shipped() -> Result<Vec<Cosmetic>, String> {
    parse(COSMETICS_JSON)
}

/// The shipped registry, read once; its unit tests hold it to [`problems`].
pub fn registry() -> &'static [Cosmetic] {
    static REGISTRY: OnceLock<Vec<Cosmetic>> = OnceLock::new();
    REGISTRY.get_or_init(|| shipped().expect("cosmetics.json passes cosmetics::problems"))
}

/// The id of `kind`'s default in `registry`: what a slot wears when nothing else is picked or
/// owned. `FullArt` has none (R1313000), so it answers `""`.
pub fn default_of(registry: &[Cosmetic], kind: CosmeticKind) -> &str {
    registry
        .iter()
        .find(|cosmetic| cosmetic.kind == kind && cosmetic.default)
        .map_or("", |cosmetic| cosmetic.id.as_str())
}

/// R1834001: whether an account whose ledger holds `owned` owns `cosmetic` where it is worn: on
/// the card `card`, or on the whole account when `card` is `None`. A kind's default and a starter
/// are owned by every account everywhere (§14.2); any other is owned by a ledger row, on that card
/// for a per-card kind (which no account row ever names, so a per-card cosmetic picked for every
/// card is owned only when it is a starter) and on the account for an account kind.
pub fn owns(cosmetic: &Cosmetic, card: Option<&str>, owned: &OwnedCosmetics) -> bool {
    if cosmetic.default || cosmetic.starter {
        return true;
    }
    if !cosmetic.kind.is_per_card() {
        return owned.account.contains(&cosmetic.id);
    }
    card.is_some_and(|card| {
        owned
            .cards
            .iter()
            .any(|row| row.card_id == card && row.cosmetic_id == cosmetic.id)
    })
}

/// Why [`check_pick`] refused a pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickRefusal {
    /// The registry has no such id of the slot's kind (an unknown id, or one of another kind).
    WrongKind,
    /// The id is of the slot's kind, and the account does not own it there.
    NotOwned,
}

impl PickRefusal {
    /// The words a write's 400 `bad_request` answers with, naming the field the pick came in.
    pub fn message(self, key: &str) -> String {
        match self {
            PickRefusal::WrongKind => format!("\"{key}\" is not a cosmetic id of its kind"),
            PickRefusal::NotOwned => format!("\"{key}\" is a cosmetic this account does not own"),
        }
    }
}

/// R1834001: a pick of `id` for a `kind` slot, worn on `card` (or on every card, or the account,
/// when `None`), as every write of a pick checks it. A full art names its card, so a full art of
/// another card is the wrong kind for this one.
pub fn check_pick(
    registry: &[Cosmetic],
    owned: &OwnedCosmetics,
    kind: CosmeticKind,
    id: &str,
    card: Option<&str>,
) -> Result<(), PickRefusal> {
    let Some(cosmetic) = registry.iter().find(|cosmetic| {
        cosmetic.kind == kind
            && cosmetic.id == id
            && (cosmetic.kind != CosmeticKind::FullArt || cosmetic.card.as_deref() == card)
    }) else {
        return Err(PickRefusal::WrongKind);
    };
    if owns(cosmetic, card, owned) {
        Ok(())
    } else {
        Err(PickRefusal::NotOwned)
    }
}

/// The full art of `card` in `registry`, if the card has one (R1313000).
pub fn full_art_of<'r>(registry: &'r [Cosmetic], card: &str) -> Option<&'r Cosmetic> {
    registry
        .iter()
        .find(|cosmetic| cosmetic.kind == CosmeticKind::FullArt && cosmetic.card.as_deref() == Some(card))
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

/// The account cosmetics a duplicate's global roll may drop (R1661001): the playmats, backdrops,
/// card backs and hero portraits that are neither a default nor a starter, in the registry's order.
/// Every one is `GLOBAL_COSMETIC_FLOOR_RARITY` or better ([`problems`], R1661000).
pub fn global_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| !cosmetic.kind.is_per_card() && !cosmetic.default && !cosmetic.starter)
        .cloned()
        .collect()
}

/// The cosmetics a cosmetic pack may drop (R1306000): every entry, per-card or account, that is
/// not a default or a starter, in the registry's order. One that names a card, a full art, drops
/// on that card alone (R1313000).
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

/// What a portrait's registry id starts with: `portrait-` and its `PortraitId` (R1827000).
pub const PORTRAIT_ID_PREFIX: &str = "portrait-";

/// The registry id of `portrait`'s cosmetic (`portrait-gary`, R1827000).
pub fn portrait_cosmetic_id(portrait: PortraitId) -> String {
    format!("{PORTRAIT_ID_PREFIX}{}", portrait.as_str())
}

/// The portrait a registry id names, or none when it is no `portrait-<id>` of a `PortraitId`.
pub fn portrait_of(id: &str) -> Option<PortraitId> {
    let rest = id.strip_prefix(PORTRAIT_ID_PREFIX)?;
    PORTRAIT_IDS
        .iter()
        .copied()
        .find(|portrait| portrait.as_str() == rest)
}

/// R1827000: whether an account whose owned account cosmetics are `account` (its ledger's rows)
/// owns `portrait`.
pub fn owns_portrait(portrait: PortraitId, account: &[String]) -> bool {
    owns_account_cosmetic(&portrait_cosmetic_id(portrait), account)
}

/// R1833000: whether an account whose owned account cosmetics are `account` (its ledger's rows)
/// owns the account cosmetic `id`: it holds the id, or the shipped registry makes it its kind's
/// default or a starter, which every account owns with no row (§14.2).
pub fn owns_account_cosmetic(id: &str, account: &[String]) -> bool {
    account.iter().any(|owned| owned == id)
        || shipped().is_ok_and(|cosmetics| {
            cosmetics
                .iter()
                .any(|cosmetic| cosmetic.id == id && (cosmetic.default || cosmetic.starter))
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

    /// R1827000: one entry per portrait id, vanilla the default and the other five at the global
    /// roll's floor, each a starter as [`entry`] makes it, so they drop nowhere.
    fn portraits() -> Vec<Cosmetic> {
        PORTRAIT_IDS
            .iter()
            .map(|portrait| {
                let default = *portrait == DEFAULT_PORTRAIT;
                Cosmetic {
                    rarity: if default {
                        CosmeticRarity::Common
                    } else {
                        GLOBAL_COSMETIC_FLOOR_RARITY
                    },
                    ..entry(&portrait_cosmetic_id(*portrait), CosmeticKind::Portrait, default)
                }
            })
            .collect()
    }

    /// One default per kind, and the portraits the check asks for, and nothing else.
    fn defaults() -> Vec<Cosmetic> {
        let mut cosmetics = vec![
            entry("foil-none", CosmeticKind::Foil, true),
            entry("finish-plain", CosmeticKind::Finish, true),
            entry("border-standard", CosmeticKind::Border, true),
            entry("table-tavern", CosmeticKind::Table, true),
            entry("field-none", CosmeticKind::Field, true),
            entry("card-back-standard", CosmeticKind::CardBack, true),
        ];
        cosmetics.extend(portraits());
        cosmetics
    }

    /// R1833000: the 36 #1833 appends, in the registry's order: six of each kind a player picks.
    const R1833000_NEW: [(&str, CosmeticKind, CosmeticRarity); 36] = [
        ("foil-glitter", CosmeticKind::Foil, CosmeticRarity::Common),
        ("foil-sunburst", CosmeticKind::Foil, CosmeticRarity::Rare),
        ("foil-aurora", CosmeticKind::Foil, CosmeticRarity::Rare),
        ("foil-galaxy", CosmeticKind::Foil, CosmeticRarity::Epic),
        ("foil-molten-gold", CosmeticKind::Foil, CosmeticRarity::Legendary),
        ("foil-void-shimmer", CosmeticKind::Foil, CosmeticRarity::Mythic),
        ("finish-parchment", CosmeticKind::Finish, CosmeticRarity::Common),
        ("finish-brushed-steel", CosmeticKind::Finish, CosmeticRarity::Rare),
        ("finish-velvet", CosmeticKind::Finish, CosmeticRarity::Rare),
        ("finish-stained-glass", CosmeticKind::Finish, CosmeticRarity::Epic),
        ("finish-obsidian", CosmeticKind::Finish, CosmeticRarity::Legendary),
        ("finish-starlit", CosmeticKind::Finish, CosmeticRarity::Mythic),
        ("border-stone", CosmeticKind::Border, CosmeticRarity::Common),
        ("border-copper", CosmeticKind::Border, CosmeticRarity::Rare),
        ("border-vines", CosmeticKind::Border, CosmeticRarity::Rare),
        ("border-runes", CosmeticKind::Border, CosmeticRarity::Epic),
        ("border-filigree", CosmeticKind::Border, CosmeticRarity::Legendary),
        ("border-celestial", CosmeticKind::Border, CosmeticRarity::Mythic),
        ("table-marble", CosmeticKind::Table, CosmeticRarity::Epic),
        ("table-chessboard", CosmeticKind::Table, CosmeticRarity::Epic),
        ("table-desert-sand", CosmeticKind::Table, CosmeticRarity::Epic),
        (
            "table-obsidian-slab",
            CosmeticKind::Table,
            CosmeticRarity::Legendary,
        ),
        (
            "table-ancient-map",
            CosmeticKind::Table,
            CosmeticRarity::Legendary,
        ),
        ("table-astral", CosmeticKind::Table, CosmeticRarity::Mythic),
        ("field-snow", CosmeticKind::Field, CosmeticRarity::Epic),
        ("field-cherry-blossoms", CosmeticKind::Field, CosmeticRarity::Epic),
        ("field-autumn-leaves", CosmeticKind::Field, CosmeticRarity::Epic),
        ("field-aurora", CosmeticKind::Field, CosmeticRarity::Legendary),
        (
            "field-lightning-storm",
            CosmeticKind::Field,
            CosmeticRarity::Legendary,
        ),
        ("field-cosmic-drift", CosmeticKind::Field, CosmeticRarity::Mythic),
        (
            "card-back-crimson-damask",
            CosmeticKind::CardBack,
            CosmeticRarity::Epic,
        ),
        ("card-back-tidal", CosmeticKind::CardBack, CosmeticRarity::Epic),
        ("card-back-tarot", CosmeticKind::CardBack, CosmeticRarity::Epic),
        (
            "card-back-dragon-scale",
            CosmeticKind::CardBack,
            CosmeticRarity::Legendary,
        ),
        (
            "card-back-clockwork",
            CosmeticKind::CardBack,
            CosmeticRarity::Legendary,
        ),
        ("card-back-nebula", CosmeticKind::CardBack, CosmeticRarity::Mythic),
    ];

    /// The shipped registry before #1833: every entry but its last 36 (R1833000).
    fn before_r1833000(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        &cosmetics[..cosmetics.len() - R1833000_NEW.len()]
    }

    #[test]
    fn r1295000_the_shipped_registry_passes_with_one_default_and_nine_items_per_kind() {
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
            // Three since #1295 and #1297, and six more since #1833 (R1833000).
            assert_eq!(
                of_kind.iter().filter(|cosmetic| !cosmetic.default).count(),
                9,
                "{kind:?}'s items"
            );
        }
        assert!(problems(&defaults()).is_empty(), "{:?}", problems(&defaults()));
    }

    #[test]
    fn r1297000_the_deck_slots_have_one_default_and_nine_items_each() {
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
                9,
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
        // R1017001's six hero cards.
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
            {"id":"card-back-standard","kind":"cardBack","name":"Standard back","rarity":"Common","default":true},
            {"id":"portrait-vanilla","kind":"portrait","name":"Vanilla","rarity":"Common","default":true},
            {"id":"portrait-gary","kind":"portrait","name":"Gary","rarity":"Epic","default":false},
            {"id":"portrait-timmy","kind":"portrait","name":"Timmy","rarity":"Epic","default":false},
            {"id":"portrait-dfender","kind":"portrait","name":"D-fender","rarity":"Epic","default":false},
            {"id":"portrait-felinors","kind":"portrait","name":"Felinors","rarity":"Epic","default":false},
            {"id":"portrait-shredder","kind":"portrait","name":"Shredder","rarity":"Epic","default":false}
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
    fn r1661000_an_account_cosmetic_below_epic_is_refused_and_the_shipped_file_passes() {
        let rated = |id: &str, kind: CosmeticKind, rarity: CosmeticRarity| Cosmetic {
            rarity,
            chase: rarity.is_chase(),
            ..entry(id, kind, false)
        };
        for (id, kind, rarity) in [
            ("table-rare", CosmeticKind::Table, CosmeticRarity::Rare),
            ("field-common", CosmeticKind::Field, CosmeticRarity::Common),
            ("card-back-rare", CosmeticKind::CardBack, CosmeticRarity::Rare),
        ] {
            let mut cosmetics = defaults();
            cosmetics.push(rated(id, kind, rarity));
            assert_eq!(
                problems(&cosmetics),
                vec![format!(
                    "the account cosmetic {id:?} is {rarity}, below GLOBAL_COSMETIC_FLOOR_RARITY's {GLOBAL_COSMETIC_FLOOR_RARITY}, which only its kind's default may be"
                )]
            );
        }
        // At the floor or above passes, each kind's default stays Common, and a per-card cosmetic
        // keeps any rarity.
        let mut fine = defaults();
        fine.extend([
            rated("table-epic", CosmeticKind::Table, GLOBAL_COSMETIC_FLOOR_RARITY),
            rated("field-legendary", CosmeticKind::Field, CosmeticRarity::Legendary),
            rated("card-back-mythic", CosmeticKind::CardBack, CosmeticRarity::Mythic),
            rated("foil-common", CosmeticKind::Foil, CosmeticRarity::Common),
            rated("border-rare", CosmeticKind::Border, CosmeticRarity::Rare),
        ]);
        assert!(problems(&fine).is_empty(), "{:?}", problems(&fine));

        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for kind in [CosmeticKind::Table, CosmeticKind::Field, CosmeticKind::CardBack] {
            for cosmetic in cosmetics.iter().filter(|cosmetic| cosmetic.kind == kind) {
                if cosmetic.default {
                    assert_eq!(cosmetic.rarity, CosmeticRarity::Common, "{}", cosmetic.id);
                } else {
                    assert!(cosmetic.rarity >= GLOBAL_COSMETIC_FLOOR_RARITY, "{}", cosmetic.id);
                }
            }
        }
        let global = global_cosmetics(&cosmetics);
        assert!(!global.is_empty());
        assert!(
            global
                .iter()
                .all(|cosmetic| !cosmetic.starter && !cosmetic.kind.is_per_card())
        );
    }

    #[test]
    fn r1661001_the_global_roll_drops_account_cosmetics_neither_default_nor_starter() {
        let dropping = |id: &str, kind: CosmeticKind| Cosmetic {
            rarity: GLOBAL_COSMETIC_FLOOR_RARITY,
            starter: false,
            ..entry(id, kind, false)
        };
        let mut cosmetics = defaults();
        // A default that is not marked a starter is still a default.
        cosmetics[3].starter = false;
        cosmetics.extend([
            Cosmetic {
                rarity: GLOBAL_COSMETIC_FLOOR_RARITY,
                ..entry("table-starter", CosmeticKind::Table, false)
            },
            dropping("foil-new", CosmeticKind::Foil),
            dropping("table-new", CosmeticKind::Table),
            dropping("field-new", CosmeticKind::Field),
            Cosmetic {
                card: Some("core-001".to_owned()),
                ..dropping("full-art-new", CosmeticKind::FullArt)
            },
            dropping("card-back-new", CosmeticKind::CardBack),
        ]);
        assert!(problems(&cosmetics).is_empty(), "{:?}", problems(&cosmetics));
        let ids: Vec<String> = global_cosmetics(&cosmetics)
            .into_iter()
            .map(|cosmetic| cosmetic.id)
            .collect();
        assert_eq!(ids, vec!["table-new", "field-new", "card-back-new"]);
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

        // The shipped chases are its two Legendary starters, which every account owns already, then
        // #1833's twelve, the first that can drop (R1833000).
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let chases: Vec<&Cosmetic> = cosmetics.iter().filter(|cosmetic| cosmetic.chase).collect();
        assert_eq!(
            chases
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            [
                "foil-cracked-ice",
                "finish-gilded-edge",
                "foil-molten-gold",
                "foil-void-shimmer",
                "finish-obsidian",
                "finish-starlit",
                "border-filigree",
                "border-celestial",
                "table-obsidian-slab",
                "table-ancient-map",
                "table-astral",
                "field-aurora",
                "field-lightning-storm",
                "field-cosmic-drift",
                "card-back-dragon-scale",
                "card-back-clockwork",
                "card-back-nebula",
            ]
        );
        assert!(chases[..2].iter().all(|cosmetic| cosmetic.starter));
        assert!(chases[2..].iter().all(|cosmetic| !cosmetic.starter));
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
    fn r1301002_r1833000_every_earlier_cosmetic_but_the_full_arts_and_five_portraits_is_a_starter() {
        // R1295000 and R1297000 unlock every item they shipped for every account (§14.2), so no
        // pack drops one; the six full arts are owned (R1313000), and the portraits but vanilla are
        // owned too, on the account (R1827000). #1833's 36 are none of them starters (R1833000).
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let earlier = before_r1833000(&cosmetics);
        assert!(
            earlier.iter().all(|cosmetic| cosmetic.starter
                == !(cosmetic.kind == CosmeticKind::FullArt
                    || (cosmetic.kind == CosmeticKind::Portrait && !cosmetic.default))),
            "{earlier:?}"
        );
        // A duplicate drops the six full arts, then #1833's 18 per-card ones.
        let mut dropped: Vec<Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .cloned()
            .collect();
        assert_eq!(dropped.len(), 6);
        dropped.extend(
            cosmetics[earlier.len()..]
                .iter()
                .filter(|cosmetic| cosmetic.kind.is_per_card())
                .cloned(),
        );
        assert_eq!(dropped.len(), 6 + 18);
        assert_eq!(pack_cosmetics(&cosmetics), dropped);
        // An entry that leaves the mark out is no starter.
        let unmarked = r#"{"cosmetics":[{"id":"foil-none","kind":"foil","name":"No foil","rarity":"Common","default":true}]}"#;
        let file: RegistryFile = serde_json::from_str(unmarked).unwrap_or_else(|error| panic!("{error}"));
        assert!(!file.cosmetics[0].starter);
    }

    #[test]
    fn r1827000_the_shipped_portraits_are_one_per_portrait_id_vanilla_the_starter_default_and_five_epic() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let portraits: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::Portrait)
            .collect();
        assert_eq!(
            portraits
                .iter()
                .map(|cosmetic| portrait_of(&cosmetic.id))
                .collect::<Vec<_>>(),
            PORTRAIT_IDS.iter().copied().map(Some).collect::<Vec<_>>()
        );
        for cosmetic in &portraits {
            let vanilla = cosmetic.id == "portrait-vanilla";
            assert_eq!(cosmetic.default, vanilla, "{}", cosmetic.id);
            assert_eq!(cosmetic.starter, vanilla, "{}", cosmetic.id);
            let rarity = if vanilla {
                CosmeticRarity::Common
            } else {
                CosmeticRarity::Epic
            };
            assert_eq!(cosmetic.rarity, rarity, "{}", cosmetic.id);
            assert!(!cosmetic.chase && cosmetic.card.is_none(), "{}", cosmetic.id);
        }
        // The global roll drops the five, in registry order, and #1833's account cosmetics after them
        // (R1833000); a cosmetic pack may drop them too, and a duplicate's per-card roll never does.
        let ids = |list: Vec<Cosmetic>| list.into_iter().map(|cosmetic| cosmetic.id).collect::<Vec<_>>();
        assert_eq!(
            ids(global_cosmetics(&cosmetics))
                .into_iter()
                .filter(|id| portrait_of(id).is_some())
                .collect::<Vec<_>>(),
            [
                "portrait-gary",
                "portrait-timmy",
                "portrait-dfender",
                "portrait-felinors",
                "portrait-shredder"
            ]
        );
        let in_packs = ids(cosmetic_pack_cosmetics(&cosmetics));
        assert!(
            ids(global_cosmetics(&cosmetics))
                .iter()
                .all(|id| in_packs.contains(id)),
            "{in_packs:?}"
        );
        assert!(
            pack_cosmetics(&cosmetics)
                .iter()
                .all(|cosmetic| cosmetic.kind != CosmeticKind::Portrait)
        );
        // An account kind with a default, picked on a deck and not in the settings.
        assert!(CosmeticKind::DEFAULTED.contains(&CosmeticKind::Portrait));
        assert!(!CosmeticKind::PICKED.contains(&CosmeticKind::Portrait));
        assert!(!CosmeticKind::Portrait.is_per_card());
        assert_eq!(
            serde_json::to_string(&CosmeticKind::Portrait).unwrap(),
            "\"portrait\""
        );
        assert_eq!(portrait_of("portrait-gary"), Some(PortraitId::Gary));
        assert_eq!(portrait_cosmetic_id(PortraitId::Gary), "portrait-gary");
        assert_eq!(portrait_of("portrait-ulfric"), None);
        assert_eq!(portrait_of("table-wood"), None);
        assert_eq!(portrait_of("gary"), None);
    }

    #[test]
    fn r1827000_the_check_holds_one_entry_per_portrait_id_and_no_other() {
        assert!(problems(&defaults()).is_empty(), "{:?}", problems(&defaults()));

        let mut missing = defaults();
        missing.retain(|cosmetic| cosmetic.id != "portrait-timmy");
        assert_eq!(
            problems(&missing),
            vec!["the portrait timmy has no \"portrait-timmy\" entry (R641)".to_owned()]
        );

        let mut extra = defaults();
        extra.push(Cosmetic {
            rarity: CosmeticRarity::Epic,
            ..entry("portrait-ulfric", CosmeticKind::Portrait, false)
        });
        assert_eq!(
            problems(&extra),
            vec!["the portrait \"portrait-ulfric\" names no portrait id (R641)".to_owned()]
        );

        // R1661000's floor holds a portrait too.
        let mut rare = defaults();
        for cosmetic in rare.iter_mut().filter(|cosmetic| cosmetic.id == "portrait-gary") {
            cosmetic.rarity = CosmeticRarity::Rare;
        }
        assert_eq!(
            problems(&rare),
            vec![format!(
                "the account cosmetic \"portrait-gary\" is Rare, below GLOBAL_COSMETIC_FLOOR_RARITY's {GLOBAL_COSMETIC_FLOOR_RARITY}, which only its kind's default may be"
            )]
        );

        // Vanilla alone is the default: another one is refused, and so is vanilla without it.
        let mut two = defaults();
        for cosmetic in two.iter_mut().filter(|cosmetic| cosmetic.id == "portrait-gary") {
            cosmetic.default = true;
        }
        assert_eq!(
            problems(&two),
            vec![
                "the portrait \"portrait-gary\" is a default exactly when it is \"portrait-vanilla\""
                    .to_owned(),
                "the kind Portrait has 2 defaults, not one".to_owned(),
            ]
        );
        let mut none = defaults();
        for cosmetic in none
            .iter_mut()
            .filter(|cosmetic| cosmetic.id == "portrait-vanilla")
        {
            cosmetic.default = false;
        }
        // A Common vanilla that is no default is below the floor as well.
        assert_eq!(
            problems(&none),
            vec![
                format!(
                    "the account cosmetic \"portrait-vanilla\" is Common, below GLOBAL_COSMETIC_FLOOR_RARITY's {GLOBAL_COSMETIC_FLOOR_RARITY}, which only its kind's default may be"
                ),
                "the portrait \"portrait-vanilla\" is a default exactly when it is \"portrait-vanilla\""
                    .to_owned(),
                "the kind Portrait has 0 defaults, not one".to_owned(),
            ]
        );

        // A portrait id under another kind is no portrait entry.
        let mut other_kind = defaults();
        other_kind.retain(|cosmetic| cosmetic.id != "portrait-shredder");
        other_kind.push(Cosmetic {
            rarity: GLOBAL_COSMETIC_FLOOR_RARITY,
            ..entry("portrait-shredder", CosmeticKind::Table, false)
        });
        assert_eq!(
            problems(&other_kind),
            vec!["the portrait shredder has no \"portrait-shredder\" entry (R641)".to_owned()]
        );
    }

    #[test]
    fn r1833000_the_shipped_registry_appends_the_36_none_default_or_starter_the_chases_exactly_legendary_and_mythic()
     {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        assert!(problems(&cosmetics).is_empty(), "{:?}", problems(&cosmetics));
        assert_eq!(cosmetics.len(), 72);
        // Appended after every earlier entry, the portraits last among those, so none moves.
        let earlier = before_r1833000(&cosmetics);
        assert_eq!(
            earlier.last().map(|cosmetic| cosmetic.id.as_str()),
            Some("portrait-shredder")
        );
        let new = &cosmetics[earlier.len()..];
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.kind, cosmetic.rarity))
                .collect::<Vec<_>>(),
            R1833000_NEW
        );
        for cosmetic in new {
            assert!(!cosmetic.default && !cosmetic.starter, "{}", cosmetic.id);
            assert_eq!(
                cosmetic.chase,
                cosmetic.rarity >= CosmeticRarity::Legendary,
                "{}",
                cosmetic.id
            );
            assert!(cosmetic.card.is_none(), "{}", cosmetic.id);
            // R1661000: an account kind sits at the global roll's floor or above; a per-card one has no floor.
            if !cosmetic.kind.is_per_card() {
                assert!(cosmetic.rarity >= GLOBAL_COSMETIC_FLOOR_RARITY, "{}", cosmetic.id);
            }
        }
        for kind in CosmeticKind::PICKED {
            let of_kind: Vec<&Cosmetic> = new.iter().filter(|cosmetic| cosmetic.kind == kind).collect();
            assert_eq!(of_kind.len(), 6, "{kind:?}");
            assert_eq!(
                of_kind
                    .iter()
                    .filter(|cosmetic| cosmetic.rarity == CosmeticRarity::Mythic)
                    .count(),
                1,
                "{kind:?}"
            );
        }
        // The first Mythics of the registry, and the first chases no account owns.
        assert!(earlier.iter().all(
            |cosmetic| cosmetic.rarity != CosmeticRarity::Mythic && (!cosmetic.chase || cosmetic.starter)
        ));
        assert_eq!(new.iter().filter(|cosmetic| cosmetic.chase).count(), 15);

        let ids = |list: Vec<Cosmetic>| list.into_iter().map(|cosmetic| cosmetic.id).collect::<Vec<_>>();
        let mut global = vec![
            "portrait-gary".to_owned(),
            "portrait-timmy".to_owned(),
            "portrait-dfender".to_owned(),
            "portrait-felinors".to_owned(),
            "portrait-shredder".to_owned(),
        ];
        global.extend(
            R1833000_NEW
                .iter()
                .filter(|(_, kind, _)| !kind.is_per_card())
                .map(|(id, _, _)| (*id).to_owned()),
        );
        assert_eq!(global.len(), 5 + 18);
        assert_eq!(ids(global_cosmetics(&cosmetics)), global);
        assert!(
            global_cosmetics(&cosmetics)
                .iter()
                .all(|cosmetic| cosmetic.rarity >= GLOBAL_COSMETIC_FLOOR_RARITY)
        );
        let in_packs = ids(cosmetic_pack_cosmetics(&cosmetics));
        assert!(
            R1833000_NEW
                .iter()
                .all(|(id, _, _)| in_packs.iter().any(|dropped| dropped == id)),
            "{in_packs:?}"
        );
    }

    #[test]
    fn r1833000_owns_account_cosmetic_counts_the_starters_defaults_and_rows() {
        // Each kind's default and every starter are owned with no row.
        for id in [
            "table-tavern",
            "field-none",
            "card-back-standard",
            "table-felt",
            "field-rain",
            "portrait-vanilla",
        ] {
            assert!(owns_account_cosmetic(id, &[]), "{id}");
        }
        // The new ones only by a row of their own.
        for (id, kind, _) in R1833000_NEW.iter().filter(|(_, kind, _)| !kind.is_per_card()) {
            assert!(!owns_account_cosmetic(id, &[]), "{id} ({kind:?})");
            assert!(owns_account_cosmetic(id, &[(*id).to_owned()]), "{id}");
            assert!(
                !owns_account_cosmetic(id, &["table-felt".to_owned(), "portrait-gary".to_owned()]),
                "{id}"
            );
        }
        // An id the registry does not know is owned only by its row, which no grant writes.
        assert!(!owns_account_cosmetic("table-unknown", &[]));
        // A portrait's ownership is the same question.
        assert!(owns_portrait(PortraitId::Gary, &["portrait-gary".to_owned()]));
        assert!(!owns_portrait(PortraitId::Gary, &["table-astral".to_owned()]));
    }

    #[test]
    fn r1827000_owns_portrait_counts_the_starter_and_the_ledger_s_rows() {
        // Vanilla is a starter: owned with no row.
        assert!(owns_portrait(PortraitId::Vanilla, &[]));
        for portrait in PORTRAIT_IDS
            .iter()
            .copied()
            .filter(|portrait| *portrait != DEFAULT_PORTRAIT)
        {
            assert!(!owns_portrait(portrait, &[]), "{portrait}");
            assert!(
                owns_portrait(portrait, &[portrait_cosmetic_id(portrait)]),
                "{portrait}"
            );
            // Another cosmetic's row, or the bare portrait id, owns nothing.
            assert!(
                !owns_portrait(portrait, &["table-wood".to_owned(), portrait.as_str().to_owned()]),
                "{portrait}"
            );
        }
        let gary = [portrait_cosmetic_id(PortraitId::Gary)];
        assert!(owns_portrait(PortraitId::Gary, &gary));
        assert!(!owns_portrait(PortraitId::Timmy, &gary));
    }

    #[test]
    fn r1834001_check_pick_refuses_another_kind_and_an_unowned_id() {
        use crate::economy::store::CardCosmetic;
        let earned = |id: &str, kind: CosmeticKind| Cosmetic {
            rarity: GLOBAL_COSMETIC_FLOOR_RARITY,
            starter: false,
            ..entry(id, kind, false)
        };
        let mut cosmetics = defaults();
        cosmetics.extend([
            Cosmetic {
                rarity: GLOBAL_COSMETIC_FLOOR_RARITY,
                ..entry("table-felt", CosmeticKind::Table, false)
            },
            earned("table-velvet", CosmeticKind::Table),
            earned("foil-prism", CosmeticKind::Foil),
            Cosmetic {
                card: Some("core-001".to_owned()),
                ..earned("full-art-dfender", CosmeticKind::FullArt)
            },
        ]);
        assert!(problems(&cosmetics).is_empty(), "{:?}", problems(&cosmetics));
        let none = OwnedCosmetics {
            cards: Vec::new(),
            account: Vec::new(),
        };
        let check = |owned: &OwnedCosmetics, kind, id: &str, card: Option<&str>| {
            check_pick(&cosmetics, owned, kind, id, card)
        };

        // A default and a starter are every account's, on every card and on the account.
        assert_eq!(check(&none, CosmeticKind::Table, "table-tavern", None), Ok(()));
        assert_eq!(check(&none, CosmeticKind::Table, "table-felt", None), Ok(()));
        assert_eq!(
            check(&none, CosmeticKind::Foil, "foil-none", Some("core-002")),
            Ok(())
        );
        // An id of another kind, an unknown id and a full art of another card are the wrong kind.
        assert_eq!(
            check(&none, CosmeticKind::Field, "table-felt", None),
            Err(PickRefusal::WrongKind)
        );
        assert_eq!(
            check(&none, CosmeticKind::Table, "table-unheard-of", None),
            Err(PickRefusal::WrongKind)
        );
        assert_eq!(
            check(&none, CosmeticKind::FullArt, "full-art-dfender", Some("core-004")),
            Err(PickRefusal::WrongKind)
        );
        // An earned one is owned only by its ledger row, on the account or on its card.
        assert_eq!(
            check(&none, CosmeticKind::Table, "table-velvet", None),
            Err(PickRefusal::NotOwned)
        );
        let owned = OwnedCosmetics {
            cards: vec![
                CardCosmetic {
                    card_id: "core-001".to_owned(),
                    cosmetic_id: "full-art-dfender".to_owned(),
                },
                CardCosmetic {
                    card_id: "core-002".to_owned(),
                    cosmetic_id: "foil-prism".to_owned(),
                },
            ],
            account: vec!["table-velvet".to_owned()],
        };
        assert_eq!(check(&owned, CosmeticKind::Table, "table-velvet", None), Ok(()));
        assert_eq!(
            check(
                &owned,
                CosmeticKind::FullArt,
                "full-art-dfender",
                Some("core-001")
            ),
            Ok(())
        );
        assert_eq!(
            check(&owned, CosmeticKind::Foil, "foil-prism", Some("core-002")),
            Ok(())
        );
        // A per-card cosmetic owned on one card is not owned on another, nor on every card.
        assert_eq!(
            check(&owned, CosmeticKind::Foil, "foil-prism", Some("core-003")),
            Err(PickRefusal::NotOwned)
        );
        assert_eq!(
            check(&owned, CosmeticKind::Foil, "foil-prism", None),
            Err(PickRefusal::NotOwned)
        );
        assert_eq!(default_of(&cosmetics, CosmeticKind::Border), "border-standard");
        assert_eq!(default_of(&cosmetics, CosmeticKind::FullArt), "");
        assert_eq!(
            full_art_of(&cosmetics, "core-001").map(|cosmetic| cosmetic.id.as_str()),
            Some("full-art-dfender")
        );
        assert!(full_art_of(&cosmetics, "core-002").is_none());
        // The shipped registry reads once, and passes.
        assert_eq!(
            registry().len(),
            shipped().map(|cosmetics| cosmetics.len()).unwrap_or(0)
        );
    }
}
