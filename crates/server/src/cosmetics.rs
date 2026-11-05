//! The cosmetics registry (#1295, #1297, SPEC §11 R1295000): every cosmetic a player can pick, in
//! `crates/server/cosmetics.json`. The pack roll ([`pack_cosmetics`], R1301002) and ownership read it here;
//! `tests/export_cosmetics.rs` writes it into `apps/web/src/wire/generated/cosmetics.ts`. [`problems`] is
//! its check: unique lower-case slugs short enough for a settings text (R633), one default per kind a
//! player picks, a chase exactly when Legendary or Mythic (R1333000). A cosmetic never enters `PlayerView`,
//! an action, the log or a replay hash (CLAUDE.md rule 7).
//!
//! R2655000: a full-art card has an Epic (still), a Legendary (animated) and a Mythic (signed) full art, the
//! last two chases; the check refuses two of one rarity. A card's own pick names one ([`full_art_picked`]):
//! `false`, `true` for its Epic, or a full art's id. R2655001: #2655 appends 57, none a default or a
//! starter (§14.12).
//!
//! R2715000: each full-art card has a fourth, its Animated art (`animated-<slug>`, [`ANIMATED_ART_ID_PREFIX`]),
//! an Epic and no chase, which the check takes beside the still Epic, refusing a second or a rarer one. `true`
//! still names the still Epic ([`full_art_of`]). #2715 appends the 21, none a default or a starter (§14.12).
//!
//! R1827000: the hero portraits (R641) are account cosmetics, one `portrait-<id>` per `PortraitId`,
//! `portrait-vanilla` the default and a starter. R1833000: #1833 appends 36, none a default or a starter; a
//! deck's save refuses one its account does not own ([`owns_account_cosmetic`], [`owns_portrait`]). A
//! Legendary token reaches a Legendary chase, and no token a Mythic (R2728000).
//! R1834001: every write of a pick is held to [`check_pick`], and a loadout freezes nothing else
//! (`loadout::freeze_loadout`, R1834002).
//!
//! R2725000: a card wears one version (`version-…`, [`CosmeticKind::Version`]) and the augments that layer on it
//! (`augment-…`, [`CosmeticKind::Augment`]), each one registry entry owned card by card like a foil, on the cards its
//! `offer` takes ([`offered`], R2725001). #2725 appends three: Full art (`version-full-art`, Legendary), Animated
//! (Epic) and Signature (Mythic). The 84 full arts before it are retired: kept with their owners, dropped by nothing,
//! and each counts as owning the version and augments it combines ([`implied_by`], [`row_owns`], R2725002).
//!
//! R2949000: Signature was worn only beside the Full art version and Animated, now Full art's signed level
//! (R3015005). R2949001: an
//! offer clause may take a seeded share of its cards ([`OfferSample`]), so Full art lands on some cards and not
//! others, and Signature's offer lies inside Full art's and Animated's.
//!
//! R2940000: #2940 appends three versions, Alt art ([`VERSION_ALT_ART_ID`], Rare), Sketch ([`VERSION_SKETCH_ID`],
//! Rare) and Alt full art ([`VERSION_ALT_FULL_ART_ID`], Legendary), each offered on a seeded share of its own, so no
//! two versions take the same cards.
//!
//! R2942000: #2942 appends three Epic versions, Extended art ([`VERSION_EXTENDED_ART_ID`]), Pixel
//! ([`VERSION_PIXEL_ID`]) and Comic ([`VERSION_COMIC_ID`]), each offered on a seeded share of its own, so no two
//! versions take the same cards.
//!
//! R2943000: #2943 appends two Legendary chase versions, Set showcase ([`VERSION_SET_SHOWCASE_ID`]) and Out of
//! frame ([`VERSION_OUT_OF_FRAME_ID`]), each offered on a seeded share of its own, so no two versions take the same
//! cards.
//!
//! R2944000: #2944 appends two Mythic chase versions, Immersive ([`VERSION_IMMERSIVE_ID`]) and Prismatic
//! ([`VERSION_PRISMATIC_ID`]), each offered on cards of its own: Immersive on a seeded share of the Legendary and
//! Mythic cards, Prismatic on every Mythic card and a seeded share of the Legendary ones. No token buys either, as
//! no token buys a Mythic (R2728000).
//!
//! R3015000: every version is a **level** of a **variant** ([`VariantLevel`]): a variant is one picture of a card
//! and its levels a ladder, its art levels first (its normal-frame form, then its full-art form where it has one),
//! then animated, then signed above a full art alone ([`problems`]). The Default variant's first level is the
//! normal face, which no entry names. Every level of a variant is offered where its first is, but an animated or
//! signed one never on a Legacy card, whose picture never moves ([`offered`], R2724000). #3015 appends fifteen
//! levels; the Animated and Signature augments are retired with the full arts ([`Cosmetic::is_retired`]), each row
//! of one still owning the levels it maps to ([`implied_by`], R3015006). R3015003: owning a level means owning
//! every level below it ([`levels_below`], [`is_next`]). R3015005: a card's pick names one level
//! ([`level_of_pick`]), every older shape mapped to the level it drew.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use jackioh_engine::replay::fnv1a32_utf16_u32;
use jackioh_engine::wire::emotes::{DEFAULT_PORTRAIT, PORTRAIT_IDS, PortraitId};
use jackioh_engine::{CardDef, CardDefs, CardType, Rarity, SetName, Tag};

use crate::config::{
    GLOBAL_COSMETIC_FLOOR_RARITY, OFFER_SAMPLE_PERCENT_SCALE, PLAYER_SETTINGS_TEXT_MAX_LENGTH,
};
use crate::economy::store::{FullArtPick, OwnedCosmetics};

/// The registry's text, compiled in.
pub const COSMETICS_JSON: &str = include_str!("../cosmetics.json");

/// The slot a cosmetic fills: on a card, a foil, a finish or a border (R1295000); on a deck, the
/// playmat (`Table`), the backdrop (`Field`) or the back of its face-down cards and deck
/// (`CardBack`) (R1297000); a card's full-art face (`FullArt`), picked only by the card's own pick
/// (R1313000, R2655000); the deck's hero portrait (`Portrait`), an account cosmetic (R641, R1827000); a card's
/// version (`Version`), the one look it wears, and an augment (`Augment`) layered on whichever version it wears,
/// each per card and picked by the card's own pick alone (R2725000).
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
    Version,
    Augment,
}

impl CosmeticKind {
    /// The kinds a player picks one of in the settings, in list order, one default each; `FullArt`
    /// is none (R1313000), and `Portrait` is a deck's pick alone (R1827000).
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

    /// A per-card kind, owned for one card (§14.2): a version and an augment too (R2725000). The deck slots'
    /// kinds and the hero portrait are account cosmetics (R1827000).
    pub fn is_per_card(self) -> bool {
        matches!(
            self,
            Self::Foil | Self::Finish | Self::Border | Self::FullArt | Self::Version | Self::Augment
        )
    }

    /// R2725001: a kind whose entries carry an `offer`, the cards that may own and wear one.
    pub fn is_offered(self) -> bool {
        matches!(self, Self::Version | Self::Augment)
    }

    /// R2725000: what an entry of the kind's id starts with, for the kinds that fix one.
    fn id_prefix(self) -> Option<&'static str> {
        match self {
            Self::Version => Some(VERSION_ID_PREFIX),
            Self::Augment => Some(AUGMENT_ID_PREFIX),
            _ => None,
        }
    }
}

/// How rare a cosmetic is (R1295000): its own scale, commonest first, never tied to its card's
/// rarity. It spells its rarities as `PrintedRarity` does, but nothing converts one into the other
/// (§14.2, R1333000).
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
    /// (R1306001). A cosmetic pack slot and a duplicate's global roll (R1661001) read it; no floor
    /// passes `Common`.
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
    /// §14.2: owned by every account on every card with no ledger row, so no pack drops it
    /// (R1301002). Later cosmetics (#1827's portraits but vanilla, #1833's 36) leave it false (§14.12,
    /// R1833000).
    #[serde(default)]
    pub starter: bool,
    /// §14.2: a chase cosmetic, true exactly for a Legendary or Mythic one (R1333000). A Legendary token
    /// buys a Legendary one, and no token a Mythic (R2728000); the Chase Shard bar buys one, and a
    /// duplicate's roll or a pack slot landing on its tier drops one (§14.6).
    #[serde(default)]
    pub chase: bool,
    /// R2755000: owned only by reaching Jlorious (§9, R608). No roll, pack, token, Chase Shard bar,
    /// dev fill or ladder bot's wardrobe offers it, whatever its rarity, so it is never a chase; a
    /// rated game grants it to each person it leaves Jlorious ([`JLORIOUS_CARD_BACK`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub jlorious: bool,
    /// The card a full art belongs to, by catalog id; no other kind names one (R1313000). A card
    /// has at most one full art of each rarity (R2655000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card: Option<String>,
    /// R2725001: a version's or an augment's offer, the cards that may own and wear it: a card is offered it
    /// when one clause takes it ([`offered`]). No other kind has one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offer: Vec<OfferClause>,
    /// R3015000: a version's place on its variant's ladder; no other kind has one. An opening's log written before
    /// #3015 holds none ([`crate::economy::roll::OpeningLog`]), so it is read as absent there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<VariantLevel>,
}

impl Cosmetic {
    /// R3015000: a retired entry, kept with its owners and read for what it maps to ([`implied_by`]), but dropped,
    /// sold and picked by nothing: the full arts (R2725000) and the Animated and Signature augments.
    pub fn is_retired(&self) -> bool {
        matches!(self.kind, CosmeticKind::FullArt | CosmeticKind::Augment)
    }
}

/// R3015000: what a level adds to the one below it: the variant's picture in its normal frame (`Art`), across the
/// whole face (`FullArt`), its picture's own layers moving (`Animated`, R2715001) or a signature (`Signed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LevelAdds {
    Art,
    FullArt,
    Animated,
    Signed,
}

impl LevelAdds {
    /// Whether the level moves: an animated one and the signed one above it, never on a Legacy card (R2724000).
    pub fn moves(self) -> bool {
        matches!(self, Self::Animated | Self::Signed)
    }
}

/// R3015000: one level of a variant: its variant's slug, its rank on the ladder from 1 (2 for the Default
/// variant, whose first level is the normal face) and what it adds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantLevel {
    pub variant: String,
    pub rank: u8,
    pub adds: LevelAdds,
}

/// R2725001: one clause of an offer. It takes a non-token card of the catalog when each field it fills lists
/// the card's type, rarity, set and id, a field left empty taking any, and `except` does not name it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferClause {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<CardType>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rarities: Vec<Rarity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sets: Vec<SetName>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cards: Vec<String>,
    /// R2725003: the cards the clause never takes, whatever its other fields take: where the look cannot
    /// show, as the Animated augment on a Legacy card, whose picture never moves (R2724000).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub except: Vec<String>,
    /// R2949001: when set, the clause takes only a seeded share of the cards its other fields take, so two
    /// entries with the same fields and different seeds land on different cards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<OfferSample>,
}

impl OfferClause {
    /// Whether the clause takes the card `id`, defined as `def`.
    pub fn takes(&self, id: &str, def: &CardDef) -> bool {
        (self.types.is_empty() || self.types.contains(&def.type_))
            && (self.rarities.is_empty() || self.rarities.contains(&def.rarity))
            && (self.sets.is_empty() || self.sets.contains(&def.set))
            && (self.cards.is_empty() || self.cards.iter().any(|card| card == id))
            && !self.except.iter().any(|card| card == id)
            && self.sample.as_ref().is_none_or(|sample| sample.takes(id))
    }
}

/// R2949001: a seeded share of the cards a clause's other fields take: a card is in it when the FNV-1a hash of
/// `<seed>/<card id>`, modulo `OFFER_SAMPLE_PERCENT_SCALE`, falls below `percent`. The registry's check holds
/// `percent` to some and not all, and `seed` to a slug.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferSample {
    pub seed: String,
    pub percent: u32,
}

impl OfferSample {
    /// Whether the sample takes the card `card`.
    pub fn takes(&self, card: &str) -> bool {
        fnv1a32_utf16_u32(&format!("{}/{card}", self.seed)) % OFFER_SAMPLE_PERCENT_SCALE < self.percent
    }
}

/// The shipped catalog's definitions, read once: what an offer is judged against (R2725001).
pub fn shipped_defs() -> &'static CardDefs {
    static DEFS: OnceLock<CardDefs> = OnceLock::new();
    DEFS.get_or_init(|| {
        serde_json::from_str(jackioh_cards::catalog_json()).expect("the compiled catalog parses as CardDefs")
    })
}

/// R2725001: whether `cosmetic` may be owned and worn on `card` (on the account, or every card, when `None`).
/// A full art on its own card alone (R1313000); a version or an augment on a non-token card of the shipped
/// catalog that a clause of its offer takes; any other cosmetic anywhere. R3015000: a level that moves never on a
/// Legacy card, whose picture never moves (R2724000), so its ladders stop before animated.
pub fn offered(cosmetic: &Cosmetic, card: Option<&str>) -> bool {
    match cosmetic.kind {
        CosmeticKind::FullArt => cosmetic.card.as_deref() == card,
        CosmeticKind::Version | CosmeticKind::Augment => card.is_some_and(|card| {
            shipped_defs().get(card).is_some_and(|def| {
                !def.is_token()
                    && !(moves(cosmetic) && def.tags.contains(&Tag::Legacy))
                    && cosmetic.offer.iter().any(|clause| clause.takes(card, def))
            })
        }),
        _ => true,
    }
}

/// R3015000: whether `cosmetic` is a level that moves ([`LevelAdds::moves`]).
fn moves(cosmetic: &Cosmetic) -> bool {
    cosmetic.level.as_ref().is_some_and(|level| level.adds.moves())
}

/// R3015000: the levels of `variant` in `registry`, lowest first.
pub fn ladder<'r>(registry: &'r [Cosmetic], variant: &str) -> Vec<&'r Cosmetic> {
    let mut levels: Vec<&Cosmetic> = registry
        .iter()
        .filter(|cosmetic| cosmetic.level.as_ref().is_some_and(|level| level.variant == variant))
        .collect();
    levels.sort_by_key(|cosmetic| cosmetic.level.as_ref().map(|level| level.rank));
    levels
}

/// R3015003: the levels of `cosmetic`'s variant below it, lowest first: none for anything but a level.
pub fn levels_below<'r>(registry: &'r [Cosmetic], cosmetic: &Cosmetic) -> Vec<&'r Cosmetic> {
    let Some(level) = &cosmetic.level else {
        return Vec::new();
    };
    ladder(registry, &level.variant)
        .into_iter()
        .filter(|below| below.level.as_ref().is_some_and(|at| at.rank < level.rank))
        .collect()
}

/// R3015003, R3015004: whether `cosmetic` is a next level where `held` says what is owned: every level below it
/// is. Anything but a level always is.
pub fn is_next(registry: &[Cosmetic], cosmetic: &Cosmetic, held: impl Fn(&Cosmetic) -> bool) -> bool {
    levels_below(registry, cosmetic).into_iter().all(held)
}

/// R2725002, R3015006: the levels owning the retired entry `cosmetic` counts as owning on its card, beside itself:
/// an Animated art (`animated-…`) and the Animated augment the Default variant's animated level; a still Epic full
/// art Full art's first level; a Legendary that, its animated level and the Default one; a Mythic those and its
/// signed level. Signature alone, and any other entry, nothing.
pub fn implied_by(cosmetic: &Cosmetic) -> &'static [&'static str] {
    match cosmetic.kind {
        CosmeticKind::Augment if cosmetic.id == AUGMENT_ANIMATED_ID => &[VERSION_ANIMATED_ID],
        CosmeticKind::FullArt if cosmetic.id.starts_with(ANIMATED_ART_ID_PREFIX) => &[VERSION_ANIMATED_ID],
        CosmeticKind::FullArt => match cosmetic.rarity {
            CosmeticRarity::Epic => &[VERSION_FULL_ART_ID],
            CosmeticRarity::Legendary => &[
                VERSION_FULL_ART_ID,
                VERSION_FULL_ART_ANIMATED_ID,
                VERSION_ANIMATED_ID,
            ],
            CosmeticRarity::Mythic => &[
                VERSION_FULL_ART_ID,
                VERSION_FULL_ART_ANIMATED_ID,
                VERSION_FULL_ART_SIGNED_ID,
                VERSION_ANIMATED_ID,
            ],
            _ => &[],
        },
        _ => &[],
    }
}

/// R2725002, R3015006: whether a ledger row of `held` owns `id` on the row's card: it is `id`, or a shipped retired
/// entry that maps to the level `id` ([`implied_by`]). The mapping is read where it is worn; migration 0083 wrote
/// each combination's levels as rows of their own.
pub fn row_owns(held: &str, id: &str) -> bool {
    held == id
        || (id.starts_with(VERSION_ID_PREFIX)
            && registry()
                .iter()
                .find(|cosmetic| cosmetic.id == held)
                .is_some_and(|cosmetic| implied_by(cosmetic).contains(&id)))
}

/// R2725002, R3015006: the shipped retired entries a row of which owns the level `id` beside itself
/// ([`implied_by`]): none unless `id` is a level's.
pub fn implying(id: &str) -> impl Iterator<Item = &'static str> + '_ {
    let level = id.starts_with(VERSION_ID_PREFIX);
    registry()
        .iter()
        .filter(move |cosmetic| level && implied_by(cosmetic).contains(&id))
        .map(|cosmetic| cosmetic.id.as_str())
}

/// R3015005: the level a card's pick of `full_art` and `augments` names on `card`, owned or not: none for no pick
/// or `false`; Full art's first level for `true` on a full-art card (its still Epic, R2655000); a level by its id;
/// a retired full art of the card the highest level it maps to ([`implied_by`]), a non-Default one first. Then
/// each old augment steps up one rank where the next adds it: Animated (from no level, the Default variant's
/// animated level), then Signature. So R2949000's Signature look is Full art's signed level, and Alt art or Sketch
/// with Animated, which has a full-art level above it, stays at its first.
pub fn level_of_pick<'r>(
    registry: &'r [Cosmetic],
    card: &str,
    full_art: Option<&FullArtPick>,
    augments: &[String],
) -> Option<&'r Cosmetic> {
    let level = |id: &str| registry.iter().find(|cosmetic| cosmetic.id == id && cosmetic.level.is_some());
    let mut worn = match full_art {
        None | Some(FullArtPick::On(false)) => None,
        Some(FullArtPick::On(true)) => full_art_of(registry, card).and(level(VERSION_FULL_ART_ID)),
        Some(FullArtPick::Id(id)) => match registry.iter().find(|cosmetic| cosmetic.id == *id) {
            Some(cosmetic) if cosmetic.level.is_some() => Some(cosmetic),
            Some(cosmetic) if cosmetic.kind == CosmeticKind::FullArt && cosmetic.card.as_deref() == Some(card) => {
                implied_by(cosmetic)
                    .iter()
                    .filter_map(|id| level(id))
                    .max_by_key(|cosmetic| {
                        cosmetic
                            .level
                            .as_ref()
                            .map(|level| (level.variant != DEFAULT_VARIANT, level.rank))
                    })
            }
            _ => None,
        },
    };
    let step = |worn: Option<&'r Cosmetic>, adds: LevelAdds| -> Option<&'r Cosmetic> {
        let Some(at) = worn.and_then(|cosmetic| cosmetic.level.as_ref()) else {
            return (adds == LevelAdds::Animated)
                .then(|| level(VERSION_ANIMATED_ID))
                .flatten()
                .or(worn);
        };
        ladder(registry, &at.variant)
            .into_iter()
            .find(|above| above.level.as_ref().is_some_and(|up| up.rank == at.rank + 1 && up.adds == adds))
            .or(worn)
    };
    if augments.iter().any(|id| id == AUGMENT_ANIMATED_ID) {
        worn = step(worn, LevelAdds::Animated);
    }
    if augments.iter().any(|id| id == AUGMENT_SIGNATURE_ID) {
        worn = step(worn, LevelAdds::Signed);
    }
    worn
}

/// R3015005: what a card wears of the level `level` on `card`: it, where `wears` takes it (owned and offered
/// there), else the highest level below it that `wears` takes, so a pick of an older shape whose augment was not
/// owned wears what it wore; none when no level does.
pub fn worn_level<'r>(
    registry: &'r [Cosmetic],
    level: Option<&'r Cosmetic>,
    wears: impl Fn(&Cosmetic) -> bool,
) -> Option<&'r Cosmetic> {
    let level = level?;
    if wears(level) {
        return Some(level);
    }
    levels_below(registry, level).into_iter().rev().find(|below| wears(below))
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
        // R1313000: a full art names its card and is no default; nothing else names a card.
        // R2655000: a card has at most one full art of each rarity. R2715000: and at most one animated
        // art, an Epic beside the still one.
        match (cosmetic.kind, cosmetic.card.as_deref()) {
            (CosmeticKind::FullArt, None) => out.push(format!("the full art {id:?} names no card")),
            (CosmeticKind::FullArt, Some(card)) => {
                let animated = id.starts_with(ANIMATED_ART_ID_PREFIX);
                if !animated && !id.starts_with(FULL_ART_ID_PREFIX) {
                    out.push(format!(
                        "the full art {id:?} does not start with {FULL_ART_ID_PREFIX:?} or {ANIMATED_ART_ID_PREFIX:?}"
                    ));
                }
                let rarity = cosmetic.rarity;
                if animated && rarity != CosmeticRarity::Epic {
                    out.push(format!(
                        "the animated art {id:?} is {rarity}, but an animated art is Epic"
                    ));
                }
                if !full_art_cards.insert((card, rarity, animated)) {
                    out.push(if animated {
                        format!("the card {card:?} has two animated arts")
                    } else {
                        format!("the card {card:?} has two {rarity} full arts")
                    });
                }
            }
            (kind, Some(card)) => out.push(format!(
                "the {kind:?} {id:?} names the card {card:?}, which only a full art does"
            )),
            (_, None) => {}
        }
        // R2725000, R2725001: a version or an augment is its kind's prefix and an offer, and earned, never a
        // default or a starter; no other kind has an offer, and an offer's card is a card of the catalog.
        if let Some(prefix) = cosmetic.kind.id_prefix() {
            if !id.starts_with(prefix) {
                out.push(format!(
                    "the {:?} {id:?} does not start with {prefix:?}",
                    cosmetic.kind
                ));
            }
            if cosmetic.offer.is_empty() {
                out.push(format!("the {:?} {id:?} has no offer", cosmetic.kind));
            }
            if cosmetic.default || cosmetic.starter {
                out.push(format!(
                    "the {:?} {id:?} is a default or a starter, which no version or augment is",
                    cosmetic.kind
                ));
            }
        } else if !cosmetic.offer.is_empty() {
            out.push(format!(
                "the {:?} {id:?} has an offer, which only a version or an augment has",
                cosmetic.kind
            ));
        }
        for card in cosmetic
            .offer
            .iter()
            .flat_map(|clause| clause.cards.iter().chain(&clause.except))
        {
            if !shipped_defs().get(card).is_some_and(|def| !def.is_token()) {
                out.push(format!(
                    "the offer of {id:?} names {card:?}, which is no card of the catalog"
                ));
            }
        }
        // R2949001: a sample takes some of its clause's cards and not all, by a slug's seed.
        for sample in cosmetic.offer.iter().filter_map(|clause| clause.sample.as_ref()) {
            if sample.percent == 0 || sample.percent >= OFFER_SAMPLE_PERCENT_SCALE {
                out.push(format!(
                    "the offer of {id:?} samples {} in {OFFER_SAMPLE_PERCENT_SCALE} of its cards, which must be some and not all",
                    sample.percent
                ));
            }
            if !is_slug(&sample.seed) {
                out.push(format!(
                    "the offer of {id:?} samples by the seed {:?}, which is not a lower-case slug",
                    sample.seed
                ));
            }
        }
        // R1333000: a chase is exactly a Legendary or Mythic cosmetic, but a Jlorious one, which
        // nothing drops (R2755000).
        if cosmetic.chase != (cosmetic.rarity.is_chase() && !cosmetic.jlorious) {
            out.push(format!(
                "the cosmetic {id:?} is {} and its chase mark is {}, but a cosmetic is a chase exactly when it is Legendary or Mythic and not Jlorious",
                cosmetic.rarity, cosmetic.chase
            ));
        }
        // R2755000: a Jlorious cosmetic is an account cosmetic every account must earn.
        if cosmetic.jlorious && (cosmetic.kind.is_per_card() || cosmetic.default || cosmetic.starter) {
            out.push(format!(
                "the Jlorious cosmetic {id:?} is a per-card cosmetic, a default or a starter, which only reaching Jlorious may grant"
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
    level_problems(cosmetics, &mut out);
    out
}

/// R3015000, R3015001: what is wrong with the registry's ladders. A version has a level and nothing else does, its
/// variant a slug; each variant's ranks run gapless from 1 (from 2 for the Default variant, whose first level is
/// the normal face); its levels add, in rank order, an art level, then a full-art one (at least one of the two, and
/// neither in the Default variant), then animated, then signed only above a full art; each is rarer than the one
/// below it but at Mythic, and a signed one is Mythic; each is offered as its variant's first level is; and the
/// Default variant's offer takes every non-token card, so every card has at least the Default ladder.
fn level_problems(cosmetics: &[Cosmetic], out: &mut Vec<String>) {
    let mut variants: Vec<&str> = Vec::new();
    for cosmetic in cosmetics {
        let id = cosmetic.id.as_str();
        match (&cosmetic.level, cosmetic.kind == CosmeticKind::Version) {
            (None, true) => out.push(format!("the version {id:?} has no level")),
            (Some(_), false) => out.push(format!(
                "the {:?} {id:?} has a level, which only a version has",
                cosmetic.kind
            )),
            (Some(level), true) => {
                if !is_slug(&level.variant) {
                    out.push(format!(
                        "the level {id:?} names the variant {:?}, which is not a lower-case slug",
                        level.variant
                    ));
                }
                if !variants.contains(&level.variant.as_str()) {
                    variants.push(&level.variant);
                }
            }
            (None, false) => {}
        }
    }
    for variant in variants {
        let levels = ladder(cosmetics, variant);
        let first: u8 = if variant == DEFAULT_VARIANT { 2 } else { 1 };
        let ranks: Vec<u8> = levels.iter().filter_map(|c| c.level.as_ref().map(|l| l.rank)).collect();
        if !ranks.iter().zip(first..).all(|(rank, expected)| *rank == expected) {
            out.push(format!(
                "the variant {variant:?} has the ranks {ranks:?}, which must run gapless from {first}"
            ));
        }
        let adds: Vec<LevelAdds> = levels.iter().filter_map(|c| c.level.as_ref().map(|l| l.adds)).collect();
        if !ladder_shape_holds(&adds, variant == DEFAULT_VARIANT) {
            out.push(format!(
                "the variant {variant:?} adds {adds:?}, but a ladder adds its art (none in the Default variant), then \
                 its full art, then animated, then signed above a full art alone"
            ));
        }
        for pair in levels.windows(2) {
            let (below, above) = (pair[0], pair[1]);
            if above.rarity <= below.rarity && above.rarity != CosmeticRarity::Mythic {
                out.push(format!(
                    "the level {:?} is {}, not rarer than {:?} below it, {}",
                    above.id, above.rarity, below.id, below.rarity
                ));
            }
        }
        for level in &levels {
            if level.level.as_ref().is_some_and(|l| l.adds == LevelAdds::Signed)
                && level.rarity != CosmeticRarity::Mythic
            {
                out.push(format!("the signed level {:?} is {}, not Mythic", level.id, level.rarity));
            }
            if let Some(lowest) = levels.first()
                && level.offer != lowest.offer
            {
                out.push(format!(
                    "the level {:?} is offered otherwise than {:?}, its variant's first",
                    level.id, lowest.id
                ));
            }
        }
        if variant == DEFAULT_VARIANT
            && let Some(lowest) = levels.first()
        {
            for (card, def) in shipped_defs().iter().filter(|(_, def)| !def.is_token()) {
                if !lowest.offer.iter().any(|clause| clause.takes(card, def)) {
                    out.push(format!(
                        "the Default variant's offer does not take {card:?}, but every card has the Default ladder"
                    ));
                }
            }
        }
    }
}

/// R3015000: whether a ladder's `adds`, in rank order, read `Art? FullArt? Animated Signed?`, at least one art
/// level outside the Default variant and none in it, and `Signed` only above a `FullArt`.
fn ladder_shape_holds(adds: &[LevelAdds], default: bool) -> bool {
    let mut rest = adds;
    let mut take = |want: LevelAdds| {
        let found = rest.first() == Some(&want);
        if found {
            rest = &rest[1..];
        }
        found
    };
    let art = take(LevelAdds::Art);
    let full_art = take(LevelAdds::FullArt);
    let animated = take(LevelAdds::Animated);
    let signed = take(LevelAdds::Signed);
    rest.is_empty() && animated && (art || full_art) != default && (!signed || full_art)
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
/// the card `card`, or on the account when `card` is `None`. A kind's default and a starter are
/// owned everywhere (§14.2); any other by a ledger row, on that card for a per-card kind (no
/// account row names one) and on the account for an account kind. R2725002: a row of a retired full
/// art owns what it implies there ([`row_owns`]).
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
            .any(|row| row.card_id == card && row_owns(&row.cosmetic_id, &cosmetic.id))
    })
}

/// Why [`check_pick`] refused a pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickRefusal {
    /// The registry has no such id of the slot's kind (an unknown id, or one of another kind).
    WrongKind,
    /// The id is of the slot's kind, and the account does not own it there.
    NotOwned,
    /// R2725001: the id is a version or an augment the card is not offered.
    NotOffered,
}

impl PickRefusal {
    /// The words a write's 400 `bad_request` answers with, naming the field the pick came in.
    pub fn message(self, key: &str) -> String {
        match self {
            PickRefusal::WrongKind => format!("\"{key}\" is not a cosmetic id of its kind"),
            PickRefusal::NotOwned => format!("\"{key}\" is a cosmetic this account does not own"),
            PickRefusal::NotOffered => format!("\"{key}\" is not offered on this card"),
        }
    }
}

/// R1834001: a pick of `id` for a `kind` slot, worn on `card` (or on every card, or the account,
/// when `None`), as every write of a pick checks it. A full art of another card is the wrong kind; a version
/// or an augment the card is not offered is refused before its ownership is read (R2725001).
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
    if !offered(cosmetic, card) {
        return Err(PickRefusal::NotOffered);
    }
    if owns(cosmetic, card, owned) {
        Ok(())
    } else {
        Err(PickRefusal::NotOwned)
    }
}

/// The Epic full art of `card`, if it has one (R1313000): what a pick of `true` names (R2655000). It is the
/// still one, never the card's Animated art, also an Epic (R2715000).
pub fn full_art_of<'r>(registry: &'r [Cosmetic], card: &str) -> Option<&'r Cosmetic> {
    registry.iter().find(|cosmetic| {
        cosmetic.kind == CosmeticKind::FullArt
            && cosmetic.card.as_deref() == Some(card)
            && cosmetic.rarity == CosmeticRarity::Epic
            && cosmetic.id.starts_with(FULL_ART_ID_PREFIX)
    })
}

/// The cosmetics a duplicate may drop (R1301002): the per-card ones neither default nor starter, in
/// registry order, each on a card it is offered (`economy::roll::duplicate_grant`, R2725001). R2725000: never
/// a retired full art, nor (R3015000) a retired augment.
pub fn pack_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| {
            cosmetic.kind.is_per_card() && !cosmetic.is_retired() && !cosmetic.default && !cosmetic.starter
        })
        .cloned()
        .collect()
}

/// The account cosmetics a duplicate's global roll may drop (R1661001): the playmats, backdrops,
/// card backs and hero portraits that are neither a default nor a starter, in the registry's order.
/// Every one is `GLOBAL_COSMETIC_FLOOR_RARITY` or better ([`problems`], R1661000); on the shipped
/// registry they are the five portraits but vanilla (R1827000), then #1833's six playmats, six
/// backdrops and six card backs (R1833000), its first Legendary and Mythic account cosmetics among
/// them. A Jlorious one is never among them (R2755000).
pub fn global_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| {
            !cosmetic.kind.is_per_card() && !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious
        })
        .cloned()
        .collect()
}

/// The cosmetics a cosmetic pack may drop (R1306000): every entry, per-card or account, that is
/// not a default or a starter, in the registry's order. A chase drops when a slot lands on its
/// tier (R1333000), and a per-card one on a card it is offered (R2725001). A Jlorious one never drops
/// (R2755000), nor a retired full art (R2725000) or augment (R3015000).
pub fn cosmetic_pack_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious && !cosmetic.is_retired())
        .cloned()
        .collect()
}

/// Whether the shipped registry lists `id` under `kind`: a deck's pick is kept only then (R1297000).
pub fn is_id_of(kind: CosmeticKind, id: &str) -> bool {
    shipped().is_ok_and(|cosmetics| {
        cosmetics
            .iter()
            .any(|cosmetic| cosmetic.kind == kind && cosmetic.id == id)
    })
}

/// What a full art's registry id starts with (R2655000), so a card pick's `fullArt` text names one.
pub const FULL_ART_ID_PREFIX: &str = "full-art-";

/// What a card's Animated art's registry id starts with (R2715000): the other id a card pick's
/// `fullArt` text may name.
pub const ANIMATED_ART_ID_PREFIX: &str = "animated-";

/// R2725000: what a version's registry id starts with: the third id a card pick's `fullArt` text may name.
pub const VERSION_ID_PREFIX: &str = "version-";

/// R2725000: what an augment's registry id starts with, each one a card pick's `augments` may list.
pub const AUGMENT_ID_PREFIX: &str = "augment-";

/// R2725000: the Full art version, Legendary, the card's art across its whole face: R3015000's Full art variant's
/// first level.
pub const VERSION_FULL_ART_ID: &str = "version-full-art";

/// R3015000: the Default variant, whose first level is the normal face, which no entry names.
pub const DEFAULT_VARIANT: &str = "default";

/// R3015000: the Default variant's second level, the normal face with its picture's own layers moving (R2715001).
pub const VERSION_ANIMATED_ID: &str = "version-animated";

/// R3015000: Full art's second level, its full art animated.
pub const VERSION_FULL_ART_ANIMATED_ID: &str = "version-full-art-animated";

/// R3015000: Full art's third level, its full art animated and signed: R2949000's Signature look (R3015005).
pub const VERSION_FULL_ART_SIGNED_ID: &str = "version-full-art-signed";

/// R3015000, R3015002: Sketch's second level, the sketch's own picture across the whole face.
pub const VERSION_SKETCH_FULL_ART_ID: &str = "version-sketch-full-art";

/// R2725000: the Animated augment, Epic: the art's own elements move (R2715001) on whichever version is worn.
/// R3015000: retired; a row of it owns the Default variant's animated level ([`implied_by`]).
pub const AUGMENT_ANIMATED_ID: &str = "augment-animated";

/// R2725000: the Signature augment, Mythic: the card's name signed across its art. R3015000: retired; a row of it
/// alone owns no level.
pub const AUGMENT_SIGNATURE_ID: &str = "augment-signature";

/// R2940000: the Alt art version, Rare: a new picture of the card's subject in the normal frame.
pub const VERSION_ALT_ART_ID: &str = "version-alt-art";

/// R2940000: the Sketch version, Rare: the card's picture in pencil and ink on paper, its frame a pencilled outline.
pub const VERSION_SKETCH_ID: &str = "version-sketch";

/// R2940000: the Alt full art version, Legendary: Alt art's picture across the whole face.
pub const VERSION_ALT_FULL_ART_ID: &str = "version-alt-full-art";

/// R2942000: the Extended art version, Epic: the card's own art past its window, behind a translucent rules box and
/// frame.
pub const VERSION_EXTENDED_ART_ID: &str = "version-extended-art";

/// R2942000: the Pixel version, Epic: the card's picture as 8-bit pixel art in a retro frame, its name in a pixel face.
pub const VERSION_PIXEL_ID: &str = "version-pixel";

/// R2942000: the Comic version, Epic: the card's picture as pop art, halftone dots and ink outlines, its name in a burst.
pub const VERSION_COMIC_ID: &str = "version-comic";

/// R2943000: the Set showcase version, Legendary: the whole card restyled in its set's own look, frame, art
/// treatment and lettering.
pub const VERSION_SET_SHOWCASE_ID: &str = "version-set-showcase";

/// R2943000: the Out of frame version, Legendary: the card's subject steps out of a smaller window over its frame,
/// under its gem, crest, name, stats and rules text.
pub const VERSION_OUT_OF_FRAME_ID: &str = "version-out-of-frame";

/// R2944000: the Immersive version, Mythic: a layered scene with parallax depth that shifts with the pointer or the
/// device's tilt and breathes slowly; still under reduced motion.
pub const VERSION_IMMERSIVE_ID: &str = "version-immersive";

/// R2944000: the Prismatic version, Mythic: mirror chrome, rainbow-edged and iridescent, never gold.
pub const VERSION_PRISMATIC_ID: &str = "version-prismatic";

/// R2755000: the card back a rated game grants each person it leaves Jlorious, and nothing else
/// grants (`Cosmetic::jlorious`).
pub const JLORIOUS_CARD_BACK: &str = "card-back-jlorious";

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

/// R1827000: whether an account owning `account` cosmetics (its ledger's rows) owns `portrait`.
pub fn owns_portrait(portrait: PortraitId, account: &[String]) -> bool {
    owns_account_cosmetic(&portrait_cosmetic_id(portrait), account)
}

/// R1833000: whether an account owning `account` cosmetics owns the account cosmetic `id`: it holds
/// the id, or the registry makes it a default or a starter (§14.2).
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
            jlorious: false,
            card: None,
            offer: Vec::new(),
            level: None,
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

    /// R2655000: how many full arts #2655 appends after #1833's 36.
    const R2655000_NEW: usize = 6 * 2 + 15 * 3;

    /// R2755000: the one entry #2755 appends after #2655's, the Jlorious card back.
    const R2755000_NEW: usize = 1;

    /// R2715000: how many Animated arts #2715 appends after #2755's one, one on each full-art card.
    const R2715000_NEW: usize = 21;

    /// R2725000: the version and the two augments #2725 appends after #2715's 21, last in the registry.
    /// R2725003: Legacy: Duplicating Felinors, a Rare Unit whose Legacy picture never moves (R2724000).
    const LEGACY_DUPLICATING_FELINORS: &str = "traininggrounds-044";

    const R2725000_NEW: [(&str, CosmeticKind, CosmeticRarity); 3] = [
        (
            VERSION_FULL_ART_ID,
            CosmeticKind::Version,
            CosmeticRarity::Legendary,
        ),
        (AUGMENT_ANIMATED_ID, CosmeticKind::Augment, CosmeticRarity::Epic),
        (
            AUGMENT_SIGNATURE_ID,
            CosmeticKind::Augment,
            CosmeticRarity::Mythic,
        ),
    ];

    /// R2940000: the three versions #2940 appends after #2725's three, last in the registry.
    const R2940000_NEW: [(&str, CosmeticKind, CosmeticRarity); 3] = [
        (VERSION_ALT_ART_ID, CosmeticKind::Version, CosmeticRarity::Rare),
        (VERSION_SKETCH_ID, CosmeticKind::Version, CosmeticRarity::Rare),
        (
            VERSION_ALT_FULL_ART_ID,
            CosmeticKind::Version,
            CosmeticRarity::Legendary,
        ),
    ];

    /// R2942000: the three versions #2942 appends after #2940's three, last in the registry but for #2943's two
    /// and #2944's two.
    const R2942000_NEW: [(&str, CosmeticKind, CosmeticRarity); 3] = [
        (
            VERSION_EXTENDED_ART_ID,
            CosmeticKind::Version,
            CosmeticRarity::Epic,
        ),
        (VERSION_PIXEL_ID, CosmeticKind::Version, CosmeticRarity::Epic),
        (VERSION_COMIC_ID, CosmeticKind::Version, CosmeticRarity::Epic),
    ];

    /// R2943000: the two Legendary versions #2943 appends after #2942's three, last in the registry but for
    /// #2944's two.
    const R2943000_NEW: [(&str, CosmeticKind, CosmeticRarity); 2] = [
        (
            VERSION_SET_SHOWCASE_ID,
            CosmeticKind::Version,
            CosmeticRarity::Legendary,
        ),
        (
            VERSION_OUT_OF_FRAME_ID,
            CosmeticKind::Version,
            CosmeticRarity::Legendary,
        ),
    ];

    /// R2944000: the two Mythic versions #2944 appends after #2943's two, last in the registry.
    const R2944000_NEW: [(&str, CosmeticKind, CosmeticRarity); 2] = [
        (
            VERSION_IMMERSIVE_ID,
            CosmeticKind::Version,
            CosmeticRarity::Mythic,
        ),
        (
            VERSION_PRISMATIC_ID,
            CosmeticKind::Version,
            CosmeticRarity::Mythic,
        ),
    ];

    /// The shipped registry before #1833: every entry but #2944's two, #2943's two before them, #2942's three
    /// before those, #2940's three before those, #2725's three before those, #2715's 21 before those, #2755's one
    /// before those, #2655's 57 before it and #1833's 36 before those (R1833000).
    fn before_r1833000(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        &cosmetics[..cosmetics.len()
            - R2944000_NEW.len()
            - R2943000_NEW.len()
            - R2942000_NEW.len()
            - R2940000_NEW.len()
            - R2725000_NEW.len()
            - R2715000_NEW
            - R2755000_NEW
            - R2655000_NEW
            - R1833000_NEW.len()]
    }

    /// The 57 #2655 appends, last in the registry but for #2755's one, #2715's 21, #2725's three, #2940's
    /// three, #2942's three, #2943's two and #2944's two (R2655000).
    fn r2655000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let end = cosmetics.len()
            - R2944000_NEW.len()
            - R2943000_NEW.len()
            - R2942000_NEW.len()
            - R2940000_NEW.len()
            - R2725000_NEW.len()
            - R2715000_NEW
            - R2755000_NEW;
        &cosmetics[end - R2655000_NEW..end]
    }

    /// The 21 #2715 appends, last in the registry but for #2725's three, #2940's three, #2942's three, #2943's
    /// two and #2944's two (R2715000).
    fn r2715000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let end = cosmetics.len()
            - R2944000_NEW.len()
            - R2943000_NEW.len()
            - R2942000_NEW.len()
            - R2940000_NEW.len()
            - R2725000_NEW.len();
        &cosmetics[end - R2715000_NEW..end]
    }

    /// The three #2725 appends, last in the registry but for #2940's three, #2942's three, #2943's two and
    /// #2944's two (R2725000).
    fn r2725000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let end = cosmetics.len()
            - R2944000_NEW.len()
            - R2943000_NEW.len()
            - R2942000_NEW.len()
            - R2940000_NEW.len();
        &cosmetics[end - R2725000_NEW.len()..end]
    }

    /// The three #2940 appends, last in the registry but for #2942's three, #2943's two and #2944's two
    /// (R2940000).
    fn r2940000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let end = cosmetics.len() - R2944000_NEW.len() - R2943000_NEW.len() - R2942000_NEW.len();
        &cosmetics[end - R2940000_NEW.len()..end]
    }

    /// The three #2942 appends, last in the registry but for #2943's two and #2944's two (R2942000).
    fn r2942000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let end = cosmetics.len() - R2944000_NEW.len() - R2943000_NEW.len();
        &cosmetics[end - R2942000_NEW.len()..end]
    }

    /// The two #2943 appends, last in the registry but for #2944's two (R2943000).
    fn r2943000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let end = cosmetics.len() - R2944000_NEW.len();
        &cosmetics[end - R2943000_NEW.len()..end]
    }

    /// The two #2944 appends, last in the registry (R2944000).
    fn r2944000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        &cosmetics[cosmetics.len() - R2944000_NEW.len()..]
    }

    /// R2655001: the 21 full-art cards (six portrait cards, R1017001, and #2655's fifteen) and their slugs.
    const R2655001_CARDS: [(&str, &str); 21] = [
        ("core-001", "big-d-fender"),
        ("core-004", "gary-the-gambler"),
        ("core-008", "mr-vanilla"),
        ("core-011", "tempo-timmy"),
        ("core-012", "duplicating-felinors"),
        ("core-013", "jlockeed-shredder-10"),
        ("classicplus-019", "league-of-losers"),
        ("core-018", "bread-and-butter"),
        ("classicplus-012", "the-mother-pancake"),
        ("core-086", "miss-mrow"),
        ("meditative-021", "api-key-fishing"),
        ("classicplus-046", "felinor-flagbearer"),
        ("stinkyscape-154", "dice-of-doom"),
        ("classicplus-062", "kys-papaya"),
        ("classic-090", "in-too-deep"),
        ("classicplus-048", "jlockeeds-lobbyist"),
        ("meditative-087", "tatches-the-totem"),
        ("classicplus-069", "buff-billy"),
        ("traininggrounds-040", "goetzmann-game-design"),
        ("classicplus-026", "tommy-tempo"),
        ("classicplus-075", "j-jungle-explorer"),
    ];

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
            // Three since #1295 and #1297, and six more since #1833 (R1833000), the Jlorious card
            // back aside (R2755000).
            assert_eq!(
                of_kind
                    .iter()
                    .filter(|cosmetic| !cosmetic.default && !cosmetic.jlorious)
                    .count(),
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
                of_kind
                    .iter()
                    .filter(|cosmetic| !cosmetic.default && !cosmetic.jlorious)
                    .count(),
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
    fn r1313000_r2655001_every_full_art_card_has_an_epic_a_legendary_and_a_mythic_and_none_is_a_pick() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let full_arts: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .collect();
        let cards: BTreeSet<&str> = full_arts
            .iter()
            .filter_map(|cosmetic| cosmetic.card.as_deref())
            .collect();
        // R2655001: R1017001's six portrait cards and #2655's fifteen.
        assert_eq!(
            cards,
            R2655001_CARDS
                .iter()
                .map(|(card, _)| *card)
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(full_arts.len(), 21 * 4);
        for (card, slug) in R2655001_CARDS {
            let of_card: Vec<(&str, CosmeticRarity)> = full_arts
                .iter()
                .filter(|cosmetic| cosmetic.card.as_deref() == Some(card))
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.rarity))
                .collect();
            // One of each rarity on the cosmetics' own scale, whatever the card's rarity, by id,
            // and the Animated Epic last (R2715000).
            assert_eq!(
                of_card,
                [
                    (format!("full-art-{slug}").as_str(), CosmeticRarity::Epic),
                    (
                        format!("full-art-{slug}-animated").as_str(),
                        CosmeticRarity::Legendary
                    ),
                    (format!("full-art-{slug}-signed").as_str(), CosmeticRarity::Mythic),
                    (format!("animated-{slug}").as_str(), CosmeticRarity::Epic),
                ],
                "{card}"
            );
            assert_eq!(
                full_art_of(&cosmetics, card).map(|cosmetic| cosmetic.id.as_str()),
                Some(format!("full-art-{slug}").as_str()),
                "{card}"
            );
        }
        for cosmetic in &full_arts {
            // R2655000: the chase mark exactly on the Legendary and the Mythic; none a default or a
            // starter, so every one is earned.
            assert_eq!(
                cosmetic.chase,
                cosmetic.rarity != CosmeticRarity::Epic,
                "{}",
                cosmetic.id
            );
            assert!(!cosmetic.default && !cosmetic.starter, "{}", cosmetic.id);
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
    fn r2655000_the_check_takes_one_full_art_of_each_rarity_on_a_card_and_refuses_two_of_one() {
        let full_art = |id: &str, card: &str, rarity: CosmeticRarity| Cosmetic {
            card: Some(card.to_owned()),
            rarity,
            chase: rarity.is_chase(),
            ..entry(id, CosmeticKind::FullArt, false)
        };
        let mut tiers = defaults();
        tiers.extend([
            full_art("full-art-a", "core-001", CosmeticRarity::Epic),
            full_art("full-art-a-animated", "core-001", CosmeticRarity::Legendary),
            full_art("full-art-a-signed", "core-001", CosmeticRarity::Mythic),
            full_art("full-art-b", "core-004", CosmeticRarity::Epic),
        ]);
        assert!(problems(&tiers).is_empty(), "{:?}", problems(&tiers));
        for rarity in [
            CosmeticRarity::Epic,
            CosmeticRarity::Legendary,
            CosmeticRarity::Mythic,
        ] {
            let mut twice = tiers.clone();
            twice.push(full_art("full-art-a-again", "core-001", rarity));
            assert_eq!(
                problems(&twice),
                vec![format!("the card \"core-001\" has two {rarity} full arts")]
            );
        }
        let mut unprefixed = defaults();
        unprefixed.push(full_art("art-a", "core-001", CosmeticRarity::Epic));
        assert_eq!(
            problems(&unprefixed),
            vec!["the full art \"art-a\" does not start with \"full-art-\" or \"animated-\"".to_owned()]
        );
        let mut unmarked = defaults();
        unmarked.push(Cosmetic {
            chase: false,
            ..full_art("full-art-a-signed", "core-001", CosmeticRarity::Mythic)
        });
        assert_eq!(problems(&unmarked).len(), 1, "{:?}", problems(&unmarked));
        let picked = |card: &str, pick: FullArtPick| {
            full_art_picked(&tiers, card, &pick).map(|cosmetic| cosmetic.id.as_str())
        };
        assert_eq!(picked("core-001", FullArtPick::On(true)), Some("full-art-a"));
        assert_eq!(picked("core-001", FullArtPick::On(false)), None);
        assert_eq!(
            picked("core-001", FullArtPick::Id("full-art-a-signed".to_owned())),
            Some("full-art-a-signed")
        );
        assert_eq!(
            picked("core-004", FullArtPick::Id("full-art-a-signed".to_owned())),
            None
        );
        assert_eq!(picked("core-001", FullArtPick::Id("foil-none".to_owned())), None);
        assert_eq!(picked("core-002", FullArtPick::On(true)), None);
    }

    #[test]
    fn r2715000_every_full_art_card_has_one_animated_epic_appended_last() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let new = r2715000_new(&cosmetics);
        // One per full-art card, in R2655001's order, after every earlier entry and before #2725's three.
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.card.as_deref(), cosmetic.id.clone()))
                .collect::<Vec<_>>(),
            R2655001_CARDS
                .iter()
                .map(|(card, slug)| (Some(*card), format!("{ANIMATED_ART_ID_PREFIX}{slug}")))
                .collect::<Vec<_>>()
        );
        for cosmetic in new {
            // A full art, Epic, no chase, no default and no starter, so every one is earned.
            assert_eq!(cosmetic.kind, CosmeticKind::FullArt, "{}", cosmetic.id);
            assert_eq!(cosmetic.rarity, CosmeticRarity::Epic, "{}", cosmetic.id);
            assert!(
                !cosmetic.chase && !cosmetic.default && !cosmetic.starter,
                "{}",
                cosmetic.id
            );
            // Named as its card's still Epic, an animated art for a full art.
            let card = cosmetic.card.as_deref().unwrap_or_default();
            let still = full_art_of(&cosmetics, card).unwrap_or_else(|| panic!("{card} has an Epic"));
            assert_eq!(
                cosmetic.name,
                still.name.replace(" full art", " animated art"),
                "{}",
                cosmetic.id
            );
            // `true` still names the still Epic, and the id names the Animated art.
            assert!(still.id.starts_with(FULL_ART_ID_PREFIX), "{card}");
            assert_eq!(
                full_art_picked(&cosmetics, card, &FullArtPick::Id(cosmetic.id.clone())),
                Some(cosmetic),
                "{card}"
            );
        }
        // No other entry starts with the prefix.
        assert_eq!(
            cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.id.starts_with(ANIMATED_ART_ID_PREFIX))
                .count(),
            R2715000_NEW
        );
        // R2725000: retired, so neither a duplicate's per-card roll nor a cosmetic pack drops one.
        let ids = |list: Vec<Cosmetic>| list.into_iter().map(|cosmetic| cosmetic.id).collect::<Vec<_>>();
        let (per_card, in_packs) = (
            ids(pack_cosmetics(&cosmetics)),
            ids(cosmetic_pack_cosmetics(&cosmetics)),
        );
        assert!(
            new.iter()
                .all(|cosmetic| !per_card.contains(&cosmetic.id) && !in_packs.contains(&cosmetic.id))
        );
    }

    #[test]
    fn r2715000_the_check_takes_an_animated_epic_beside_the_still_one_and_refuses_a_second_or_a_rarer_one() {
        let full_art = |id: &str, card: &str, rarity: CosmeticRarity| Cosmetic {
            card: Some(card.to_owned()),
            rarity,
            chase: rarity.is_chase(),
            ..entry(id, CosmeticKind::FullArt, false)
        };
        // The Animated Epic beside the still one and the two chases passes, listed first.
        let mut tiers = defaults();
        tiers.extend([
            full_art("animated-a", "core-001", CosmeticRarity::Epic),
            full_art("full-art-a", "core-001", CosmeticRarity::Epic),
            full_art("full-art-a-animated", "core-001", CosmeticRarity::Legendary),
            full_art("full-art-a-signed", "core-001", CosmeticRarity::Mythic),
            full_art("full-art-b", "core-004", CosmeticRarity::Epic),
        ]);
        assert!(problems(&tiers).is_empty(), "{:?}", problems(&tiers));
        // A second animated art on one card is refused, and so is a second still Epic.
        let mut twice = tiers.clone();
        twice.push(full_art("animated-a-again", "core-001", CosmeticRarity::Epic));
        assert_eq!(
            problems(&twice),
            vec!["the card \"core-001\" has two animated arts".to_owned()]
        );
        let mut still_twice = tiers.clone();
        still_twice.push(full_art("full-art-a-again", "core-001", CosmeticRarity::Epic));
        assert_eq!(
            problems(&still_twice),
            vec!["the card \"core-001\" has two Epic full arts".to_owned()]
        );
        // An animated art of any rarity but Epic is refused.
        for rarity in CosmeticRarity::ALL
            .into_iter()
            .filter(|rarity| *rarity != CosmeticRarity::Epic)
        {
            let mut rarer = tiers.clone();
            rarer.push(full_art("animated-b", "core-004", rarity));
            assert_eq!(
                problems(&rarer),
                vec![format!(
                    "the animated art \"animated-b\" is {rarity}, but an animated art is Epic"
                )]
            );
        }
        // `true` names the still Epic even with the Animated one listed first; its id names it.
        let picked = |card: &str, pick: FullArtPick| {
            full_art_picked(&tiers, card, &pick).map(|cosmetic| cosmetic.id.as_str())
        };
        assert_eq!(picked("core-001", FullArtPick::On(true)), Some("full-art-a"));
        assert_eq!(
            full_art_of(&tiers, "core-001").map(|cosmetic| cosmetic.id.as_str()),
            Some("full-art-a")
        );
        assert_eq!(
            picked("core-001", FullArtPick::Id("animated-a".to_owned())),
            Some("animated-a")
        );
        assert_eq!(picked("core-004", FullArtPick::Id("animated-a".to_owned())), None);
    }

    #[test]
    fn r2725000_registry_holds_versions_and_augments() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let new = r2725000_new(&cosmetics);
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.kind, cosmetic.rarity))
                .collect::<Vec<_>>(),
            R2725000_NEW
        );
        for cosmetic in new {
            // Earned, per card, offered, naming no card; a chase exactly at Legendary and Mythic.
            assert!(
                !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious,
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.kind.is_per_card() && cosmetic.kind.is_offered(),
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.card.is_none() && !cosmetic.offer.is_empty(),
                "{}",
                cosmetic.id
            );
            assert_eq!(cosmetic.chase, cosmetic.rarity.is_chase(), "{}", cosmetic.id);
        }
        // Nothing before them is a version or an augment, or has an offer; only #2940's three, #2942's three,
        // #2943's two and #2944's two come after them.
        assert!(
            cosmetics[..cosmetics.len()
                - R2944000_NEW.len()
                - R2943000_NEW.len()
                - R2942000_NEW.len()
                - R2940000_NEW.len()
                - R2725000_NEW.len()]
                .iter()
                .all(|cosmetic| !cosmetic.kind.is_offered() && cosmetic.offer.is_empty())
        );
        assert!(!CosmeticKind::PICKED.contains(&CosmeticKind::Version));
        assert!(!CosmeticKind::DEFAULTED.contains(&CosmeticKind::Augment));
        assert_eq!(
            serde_json::to_string(&CosmeticKind::Version).ok().as_deref(),
            Some("\"version\"")
        );
        assert_eq!(
            serde_json::to_string(&CosmeticKind::Augment).ok().as_deref(),
            Some("\"augment\"")
        );

        // The check refuses a missing offer, a bad prefix, an offer on a foil, a card, a starter and an
        // offer naming no card of the catalog.
        let offer = vec![OfferClause {
            rarities: vec![Rarity::Legendary],
            ..OfferClause::default()
        }];
        let version = |id: &str| Cosmetic {
            starter: false,
            offer: offer.clone(),
            ..entry(id, CosmeticKind::Version, false)
        };
        let mut fine = defaults();
        fine.extend([
            version("version-a"),
            Cosmetic {
                kind: CosmeticKind::Augment,
                ..version("augment-a")
            },
        ]);
        assert!(problems(&fine).is_empty(), "{:?}", problems(&fine));
        let refused = |wrong: Cosmetic, problem: &str| {
            let mut cosmetics = defaults();
            cosmetics.push(wrong);
            assert_eq!(problems(&cosmetics), vec![problem.to_owned()]);
        };
        refused(
            Cosmetic {
                offer: Vec::new(),
                ..version("version-a")
            },
            "the Version \"version-a\" has no offer",
        );
        refused(
            version("full-art-a"),
            "the Version \"full-art-a\" does not start with \"version-\"",
        );
        refused(
            Cosmetic {
                kind: CosmeticKind::Augment,
                ..version("version-a")
            },
            "the Augment \"version-a\" does not start with \"augment-\"",
        );
        refused(
            Cosmetic {
                offer: offer.clone(),
                ..entry("foil-a", CosmeticKind::Foil, false)
            },
            "the Foil \"foil-a\" has an offer, which only a version or an augment has",
        );
        refused(
            Cosmetic {
                card: Some("core-001".to_owned()),
                ..version("version-a")
            },
            "the Version \"version-a\" names the card \"core-001\", which only a full art does",
        );
        refused(
            Cosmetic {
                starter: true,
                ..version("version-a")
            },
            "the Version \"version-a\" is a default or a starter, which no version or augment is",
        );
        refused(
            Cosmetic {
                offer: vec![OfferClause {
                    cards: vec!["core-999".to_owned()],
                    ..OfferClause::default()
                }],
                ..version("version-a")
            },
            "the offer of \"version-a\" names \"core-999\", which is no card of the catalog",
        );
        refused(
            Cosmetic {
                offer: vec![OfferClause {
                    rarities: vec![Rarity::Legendary],
                    except: vec!["core-998".to_owned()],
                    ..OfferClause::default()
                }],
                ..version("version-a")
            },
            "the offer of \"version-a\" names \"core-998\", which is no card of the catalog",
        );
        let unknown = r#"{"cosmetics":[{"id":"version-a","kind":"version","name":"A","rarity":"Rare","default":false,"offer":[{"colours":["Red"]}]}]}"#;
        assert!(parse(unknown).is_err_and(|error| error.contains("unknown field")));
    }

    #[test]
    fn r2725001_every_version_reaches_its_floor_in_every_set() {
        use crate::config::{VERSION_OFFER_MIN_CARDS, VERSION_OFFER_MIN_PER_SET};
        use jackioh_engine::SHIPPED_SETS;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let defs = shipped_defs();
        let offered_kinds: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.kind.is_offered())
            .collect();
        // R2940000, R2942000, R2943000, R2944000: #2725's three, #2940's three, #2942's three, #2943's two and
        // #2944's two.
        assert_eq!(
            offered_kinds.len(),
            R2725000_NEW.len()
                + R2940000_NEW.len()
                + R2942000_NEW.len()
                + R2943000_NEW.len()
                + R2944000_NEW.len()
        );
        for cosmetic in offered_kinds {
            let on: Vec<&str> = defs
                .keys()
                .map(String::as_str)
                .filter(|card| offered(cosmetic, Some(card)))
                .collect();
            assert!(
                on.len() >= VERSION_OFFER_MIN_CARDS,
                "{}: {} cards",
                cosmetic.id,
                on.len()
            );
            for set in SHIPPED_SETS {
                let in_set = on.iter().filter(|card| defs[**card].set == set).count();
                assert!(
                    in_set >= VERSION_OFFER_MIN_PER_SET,
                    "{} on {set}: {in_set}",
                    cosmetic.id
                );
            }
            // Never a token, never every card, and every one of the 21 with retired full arts.
            assert!(on.iter().all(|card| !defs[*card].is_token()), "{}", cosmetic.id);
            assert!(
                on.len() < defs.values().filter(|def| !def.is_token()).count(),
                "{}",
                cosmetic.id
            );
            // Nowhere for no card or an id the catalog does not hold.
            assert!(!offered(cosmetic, None) && !offered(cosmetic, Some("core-999")));
            // What follows is #2725's three's alone: #2940's and #2942's three and #2943's and #2944's two imply
            // nothing a retired full art held, and their own offers are R2940000's, R2942000's, R2943000's and
            // R2944000's tests'.
            if !R2725000_NEW.iter().any(|(id, _, _)| *id == cosmetic.id) {
                continue;
            }
            for (card, _) in R2655001_CARDS {
                assert!(on.contains(&card), "{} on {card}", cosmetic.id);
            }
            // R2725003: Animated never on a Legacy card, whose picture never moves (R2724000); the version there
            // as on any Rare Unit its sample takes, and R2949000: Signature, which needs Animated, not.
            assert_eq!(
                offered(cosmetic, Some(LEGACY_DUPLICATING_FELINORS)),
                cosmetic.id == VERSION_FULL_ART_ID,
                "{}",
                cosmetic.id
            );
        }
        // A full art on its own card alone; any other kind anywhere.
        let full_art =
            full_art_of(&cosmetics, "core-008").unwrap_or_else(|| panic!("Mr. Vanilla's full art"));
        assert!(offered(full_art, Some("core-008")) && !offered(full_art, Some("core-022")));
        let foil = cosmetics
            .iter()
            .find(|cosmetic| cosmetic.kind == CosmeticKind::Foil)
            .unwrap_or_else(|| panic!("a foil"));
        assert!(offered(foil, Some("core-002")) && offered(foil, None));

        // check_pick refuses a version or an augment the card is not offered before it reads ownership, and
        // a card's pick may name the version where it is offered.
        let owned = OwnedCosmetics {
            cards: [
                ("core-002", VERSION_FULL_ART_ID),
                ("core-022", AUGMENT_ANIMATED_ID),
            ]
            .into_iter()
            .map(|(card, id)| crate::economy::store::CardCosmetic {
                card_id: card.to_owned(),
                cosmetic_id: id.to_owned(),
            })
            .collect(),
            account: Vec::new(),
        };
        let check = |kind, id: &str, card: &str| check_pick(&cosmetics, &owned, kind, id, Some(card));
        assert_eq!(
            check(CosmeticKind::Version, VERSION_FULL_ART_ID, "core-002"),
            Err(PickRefusal::NotOffered)
        );
        assert_eq!(
            check(CosmeticKind::Version, VERSION_FULL_ART_ID, "core-022"),
            Err(PickRefusal::NotOwned)
        );
        assert_eq!(
            check(CosmeticKind::Augment, AUGMENT_ANIMATED_ID, "core-022"),
            Ok(())
        );
        assert_eq!(
            check(CosmeticKind::Version, AUGMENT_ANIMATED_ID, "core-022"),
            Err(PickRefusal::WrongKind)
        );
        assert_eq!(
            PickRefusal::NotOffered.message("augments"),
            "\"augments\" is not offered on this card"
        );
        let named = |card: &str| {
            full_art_picked(&cosmetics, card, &FullArtPick::Id(VERSION_FULL_ART_ID.to_owned()))
                .map(|cosmetic| cosmetic.id.as_str())
        };
        assert_eq!(named("core-022"), Some(VERSION_FULL_ART_ID));
        assert_eq!(named("core-002"), None);
        // `true` still names a full-art card's retired still Epic alone.
        assert_eq!(
            full_art_picked(&cosmetics, "core-022", &FullArtPick::On(true)),
            None
        );
    }

    #[test]
    fn r2725002_a_retired_full_art_owns_what_it_implies() {
        use crate::economy::store::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        for (card, slug) in R2655001_CARDS {
            for (held, implied) in [
                (format!("full-art-{slug}"), vec![VERSION_FULL_ART_ID]),
                (
                    format!("full-art-{slug}-animated"),
                    vec![VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID],
                ),
                (
                    format!("full-art-{slug}-signed"),
                    vec![VERSION_FULL_ART_ID, AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID],
                ),
                (format!("animated-{slug}"), vec![AUGMENT_ANIMATED_ID]),
            ] {
                assert_eq!(implied_by(by_id(&held)), implied.as_slice(), "{held}");
                let owned = OwnedCosmetics {
                    cards: vec![CardCosmetic {
                        card_id: card.to_owned(),
                        cosmetic_id: held.clone(),
                    }],
                    account: Vec::new(),
                };
                for (id, _, _) in R2725000_NEW {
                    let entry = by_id(id);
                    // Owned on its own card exactly where implied, and never on another card.
                    assert_eq!(
                        owns(entry, Some(card), &owned),
                        implied.contains(&id),
                        "{held} → {id}"
                    );
                    assert_eq!(row_owns(&held, id), implied.contains(&id), "{held} → {id}");
                    assert!(!owns(entry, Some("core-022"), &owned), "{held} → {id} elsewhere");
                    assert_eq!(
                        implying(card, id).any(|retired| retired == held),
                        implied.contains(&id),
                        "{held} → {id}"
                    );
                }
                // It still owns itself, and no other retired full art.
                assert!(owns(by_id(&held), Some(card), &owned));
                assert!(row_owns(&held, &held));
            }
        }
        // Nothing else implies anything, and an implication is never of another kind.
        assert!(
            cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.kind != CosmeticKind::FullArt)
                .all(|cosmetic| implied_by(cosmetic).is_empty())
        );
        assert!(!row_owns("full-art-mr-vanilla", "foil-galaxy"));
        assert!(!row_owns("foil-galaxy", VERSION_FULL_ART_ID));
        assert!(implying("core-008", "foil-galaxy").next().is_none());
    }

    #[test]
    fn r2725000_retired_full_arts_never_drop() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for list in [pack_cosmetics(&cosmetics), cosmetic_pack_cosmetics(&cosmetics)] {
            assert!(list.iter().all(|cosmetic| cosmetic.kind != CosmeticKind::FullArt));
            // The version and both augments drop in their place.
            for (id, _, _) in R2725000_NEW {
                assert!(list.iter().any(|cosmetic| cosmetic.id == id), "{id}");
            }
        }
        // They stay in the registry, so every owner keeps them by id (§14.12).
        assert_eq!(
            cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
                .count(),
            21 * 4
        );
    }

    #[test]
    fn r2949000_signature_fits_only_beside_the_full_art_version_and_animated() {
        let pick = |id: &str| Some(FullArtPick::Id(id.to_owned()));
        let list = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<String>>();
        let both = list(&[AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
        let animated = list(&[AUGMENT_ANIMATED_ID]);
        // The version, `true` (a full-art card's still Epic) or a retired full art implying it, with Animated
        // listed or implied.
        for (full_art, augments) in [
            (pick(VERSION_FULL_ART_ID), both.clone()),
            (Some(FullArtPick::On(true)), animated.clone()),
            (pick("full-art-mr-vanilla-animated"), Vec::new()),
            (pick("full-art-mr-vanilla-signed"), Vec::new()),
            (pick("full-art-mr-vanilla"), animated.clone()),
        ] {
            assert!(
                signature_fits(full_art.as_ref(), &augments),
                "{full_art:?} {augments:?}"
            );
        }
        // Without Animated, without the version, or with a version this build does not know.
        for (full_art, augments) in [
            (pick(VERSION_FULL_ART_ID), list(&[AUGMENT_SIGNATURE_ID])),
            (pick("full-art-mr-vanilla"), Vec::new()),
            (pick("animated-mr-vanilla"), animated.clone()),
            (Some(FullArtPick::On(false)), both.clone()),
            (None, both.clone()),
            (pick("version-from-a-newer-client"), both.clone()),
        ] {
            assert!(
                !signature_fits(full_art.as_ref(), &augments),
                "{full_art:?} {augments:?}"
            );
        }
    }

    #[test]
    fn r2949001_a_sample_takes_a_card_by_its_seed_and_percent_and_the_check_holds_it_to_some_not_all() {
        let defs = shipped_defs();
        let sample = |seed: &str, percent: u32| OfferSample {
            seed: seed.to_owned(),
            percent,
        };
        let taken = |sample: &OfferSample| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| sample.takes(card))
                .collect()
        };
        let half = sample("version-full-art", 50);
        for card in defs.keys() {
            assert_eq!(
                half.takes(card),
                fnv1a32_utf16_u32(&format!("version-full-art/{card}")) % OFFER_SAMPLE_PERCENT_SCALE < 50,
                "{card}"
            );
        }
        // The same seed and percent take the same cards, a larger percent more of them, another seed others.
        assert_eq!(taken(&half), taken(&sample("version-full-art", 50)));
        assert!(taken(&half).is_subset(&taken(&sample("version-full-art", 60))));
        assert!(taken(&half).len() < taken(&sample("version-full-art", 60)).len());
        assert_ne!(taken(&half), taken(&sample("version-alt-full-art", 50)));
        assert!(!taken(&half).is_empty() && taken(&half).len() < defs.len());
        // A clause with a sample takes what its fields take and its sample does.
        let rares = OfferClause {
            rarities: vec![Rarity::Rare],
            ..OfferClause::default()
        };
        let sampled = OfferClause {
            sample: Some(half.clone()),
            ..rares.clone()
        };
        for (card, def) in defs {
            assert_eq!(
                sampled.takes(card, def),
                rares.takes(card, def) && half.takes(card),
                "{card}"
            );
        }

        // The check refuses none or all of the cards, and a seed that is not a slug.
        let entry_with = |sample: OfferSample| Cosmetic {
            starter: false,
            offer: vec![OfferClause {
                sample: Some(sample),
                ..rares.clone()
            }],
            ..entry("version-a", CosmeticKind::Version, false)
        };
        let checked = |sample: OfferSample| {
            let mut cosmetics = defaults();
            cosmetics.push(entry_with(sample));
            problems(&cosmetics)
        };
        assert!(checked(half.clone()).is_empty());
        assert!(checked(sample("version-full-art", OFFER_SAMPLE_PERCENT_SCALE - 1)).is_empty());
        for percent in [0, OFFER_SAMPLE_PERCENT_SCALE] {
            assert_eq!(
                checked(sample("version-full-art", percent)),
                vec![format!(
                    "the offer of \"version-a\" samples {percent} in {OFFER_SAMPLE_PERCENT_SCALE} of its cards, which must be some and not all"
                )]
            );
        }
        assert_eq!(
            checked(sample("Bad Seed", 50)),
            vec![
                "the offer of \"version-a\" samples by the seed \"Bad Seed\", which is not a lower-case slug"
                    .to_owned()
            ]
        );
        let unknown = r#"{"cosmetics":[{"id":"version-a","kind":"version","name":"A","rarity":"Rare","default":false,"offer":[{"sample":{"seed":"a","percent":5,"salt":1}}]}]}"#;
        assert!(parse(unknown).is_err_and(|error| error.contains("unknown field")));
    }

    #[test]
    fn r2949001_full_art_is_a_seeded_share_holding_the_21_and_signature_lies_inside_full_art_and_animated() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let on = |cosmetic: &Cosmetic| -> BTreeSet<&str> {
            shipped_defs()
                .keys()
                .map(String::as_str)
                .filter(|card| offered(cosmetic, Some(card)))
                .collect()
        };
        let full_art = by_id(VERSION_FULL_ART_ID);
        let animated = by_id(AUGMENT_ANIMATED_ID);
        let signature = by_id(AUGMENT_SIGNATURE_ID);
        // Full art samples its clauses: strictly fewer cards than they take whole, and not Animated's cards.
        let unsampled = Cosmetic {
            offer: full_art
                .offer
                .iter()
                .map(|clause| OfferClause {
                    sample: None,
                    ..clause.clone()
                })
                .collect(),
            ..full_art.clone()
        };
        assert!(full_art.offer.iter().any(|clause| clause.sample.is_some()));
        assert!(on(full_art).is_subset(&on(&unsampled)));
        assert!(on(full_art).len() < on(&unsampled).len());
        assert_ne!(on(full_art), on(animated));
        // The 21 with retired full arts always among them, so every pick worn today stays worn.
        for (card, _) in R2655001_CARDS {
            assert!(on(full_art).contains(card), "{card}");
        }
        // Signature exactly where both are offered.
        let both: BTreeSet<&str> = on(full_art).intersection(&on(animated)).copied().collect();
        assert_eq!(on(signature), both);
    }

    #[test]
    fn r2940000_part_two_appends_alt_art_sketch_and_alt_full_art() {
        use crate::economy::store::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let new = r2940000_new(&cosmetics);
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.kind, cosmetic.rarity))
                .collect::<Vec<_>>(),
            R2940000_NEW
        );
        for cosmetic in new {
            // Earned, per card, offered, naming no card; a chase exactly at Legendary.
            assert!(
                !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious,
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.kind.is_per_card() && cosmetic.kind.is_offered(),
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.card.is_none() && !cosmetic.offer.is_empty(),
                "{}",
                cosmetic.id
            );
            assert_eq!(cosmetic.chase, cosmetic.rarity.is_chase(), "{}", cosmetic.id);
            // R2949001: every clause samples by the version's own id, so it lands on cards of its own.
            assert!(
                cosmetic.offer.iter().all(|clause| clause
                    .sample
                    .as_ref()
                    .is_some_and(|sample| sample.seed == cosmetic.id)),
                "{}",
                cosmetic.id
            );
        }

        let defs = shipped_defs();
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let on = |id: &str| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| offered(by_id(id), Some(card)))
                .collect()
        };
        let non_tokens: BTreeSet<&str> = defs
            .iter()
            .filter(|(_, def)| !def.is_token())
            .map(|(id, _)| id.as_str())
            .collect();
        // Each version on a set of cards of its own: no two of the four alike, none on every card.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
            VERSION_ALT_FULL_ART_ID,
        ];
        for (at, a) in versions.iter().enumerate() {
            assert!(on(a).len() < non_tokens.len(), "{a}");
            for b in &versions[at + 1..] {
                assert_ne!(on(a), on(b), "{a} and {b}");
            }
        }
        // Alt art the widest of the three.
        assert!(on(VERSION_ALT_ART_ID).len() > on(VERSION_SKETCH_ID).len());
        assert!(on(VERSION_ALT_ART_ID).len() > on(VERSION_ALT_FULL_ART_ID).len());
        // R2940001: never Alt art or Alt full art on a Legacy card (R2511000), whose one picture is its Core card's.
        let legacy: Vec<&str> = defs
            .iter()
            .filter(|(_, def)| def.name.starts_with("Legacy: "))
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(legacy.len(), 20);
        for card in &legacy {
            assert!(!offered(by_id(VERSION_ALT_ART_ID), Some(card)), "{card}");
            assert!(!offered(by_id(VERSION_ALT_FULL_ART_ID), Some(card)), "{card}");
        }

        // R3015005: a pick of one with the old Animated and Signature steps up only where its variant's next level
        // adds them: Signature never, since no level of it is signed above a normal-frame art.
        for (id, _, _) in R2940000_NEW {
            let worn = picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &[AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
            let above = ladder(&cosmetics, &by_id(id).level.as_ref().map_or(String::new(), |level| level.variant.clone()))
                .into_iter()
                .find(|level| level.level.as_ref().is_some_and(|at| at.rank == by_id(id).level.as_ref().map_or(0, |l| l.rank) + 1))
                .filter(|level| level.level.as_ref().is_some_and(|at| at.adds == LevelAdds::Animated))
                .map_or(id, |level| level.id.as_str());
            assert_eq!(worn, Some(above), "{id}");
            assert!(worn.is_some_and(|worn| by_id(worn).level.as_ref().is_some_and(|at| at.adds != LevelAdds::Signed)), "{id}");
        }

        // check_pick refuses each where it is not offered, owned or not, and takes it where it is offered and owned.
        const BARE: &str = "core-002";
        const ALL_SIX: &str = "core-083";
        let owned = OwnedCosmetics {
            cards: R2940000_NEW
                .iter()
                .flat_map(|(id, _, _)| [BARE, ALL_SIX].map(|card| (card, *id)))
                .map(|(card, id)| CardCosmetic {
                    card_id: card.to_owned(),
                    cosmetic_id: id.to_owned(),
                })
                .collect(),
            account: Vec::new(),
        };
        for (id, kind, _) in R2940000_NEW {
            assert!(on(id).contains(ALL_SIX) && !on(id).contains(BARE), "{id}");
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(BARE)),
                Err(PickRefusal::NotOffered),
                "{id}"
            );
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(ALL_SIX)),
                Ok(()),
                "{id}"
            );
            // R3015005: a card's pick names the level, and it is worn only where it is offered.
            let worn = |card: &str| {
                worn_level(&cosmetics, level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]), |level| {
                    offered(level, Some(card))
                })
                .map(|level| level.id.as_str())
            };
            assert_eq!(worn(ALL_SIX), Some(id));
            assert_eq!(worn(BARE), None);
            // No retired entry owns one: owning a part 1 look never owns a part 2 one (R3015006).
            assert!(implying(id).next().is_none(), "{id}");
        }
    }

    #[test]
    fn r2942000_part_three_appends_extended_art_pixel_and_comic() {
        use crate::economy::store::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let new = r2942000_new(&cosmetics);
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.kind, cosmetic.rarity))
                .collect::<Vec<_>>(),
            R2942000_NEW
        );
        for cosmetic in new {
            // Earned, per card, offered, naming no card, and Epic, so no chase.
            assert!(
                !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious && !cosmetic.chase,
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.kind.is_per_card() && cosmetic.kind.is_offered(),
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.card.is_none() && !cosmetic.offer.is_empty(),
                "{}",
                cosmetic.id
            );
            // R2949001: every clause samples by the version's own id, so it lands on cards of its own.
            assert!(
                cosmetic.offer.iter().all(|clause| clause
                    .sample
                    .as_ref()
                    .is_some_and(|sample| sample.seed == cosmetic.id)),
                "{}",
                cosmetic.id
            );
        }

        let defs = shipped_defs();
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let on = |id: &str| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| offered(by_id(id), Some(card)))
                .collect()
        };
        let non_tokens: BTreeSet<&str> = defs
            .iter()
            .filter(|(_, def)| !def.is_token())
            .map(|(id, _)| id.as_str())
            .collect();
        // Each version on a set of cards of its own: no two of the seven alike, none on every card, and each of
        // part 3's with a card no other version takes.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
            VERSION_ALT_FULL_ART_ID,
            VERSION_EXTENDED_ART_ID,
            VERSION_PIXEL_ID,
            VERSION_COMIC_ID,
        ];
        for (at, a) in versions.iter().enumerate() {
            assert!(on(a).len() < non_tokens.len(), "{a}");
            for b in &versions[at + 1..] {
                assert_ne!(on(a), on(b), "{a} and {b}");
            }
        }
        for (id, _, _) in R2942000_NEW {
            let others: BTreeSet<&str> = versions
                .iter()
                .filter(|other| **other != id)
                .flat_map(|other| on(other))
                .collect();
            assert!(on(id).difference(&others).next().is_some(), "{id}");
        }
        // Extended art on Spells, Pixel on Units and Traps, Comic on any of them, each with Legendary and Mythic
        // cards beside: every card each takes is of the types or rarities its clauses name.
        let legendary_or_mythic =
            |card: &str| matches!(defs[card].rarity, Rarity::Legendary | Rarity::Mythic);
        for card in on(VERSION_EXTENDED_ART_ID) {
            assert!(
                matches!(defs[card].type_, CardType::Spell | CardType::FieldSpell)
                    || legendary_or_mythic(card),
                "{card}"
            );
        }
        for card in on(VERSION_PIXEL_ID) {
            assert!(
                (matches!(
                    defs[card].type_,
                    CardType::Unit | CardType::Trap | CardType::FieldTrap
                ) && matches!(defs[card].rarity, Rarity::Rare | Rarity::Epic))
                    || legendary_or_mythic(card),
                "{card}"
            );
        }
        for card in on(VERSION_COMIC_ID) {
            assert!(
                (matches!(
                    defs[card].type_,
                    CardType::Unit | CardType::Spell | CardType::Trap
                ) && matches!(defs[card].rarity, Rarity::Rare | Rarity::Epic))
                    || legendary_or_mythic(card),
                "{card}"
            );
        }

        // R3015005: a pick of one with the old Animated and Signature steps up only where its variant's next level
        // adds them: Signature never, since no level of it is signed above a normal-frame art.
        for (id, _, _) in R2942000_NEW {
            let worn = picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &[AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
            let above = ladder(&cosmetics, &by_id(id).level.as_ref().map_or(String::new(), |level| level.variant.clone()))
                .into_iter()
                .find(|level| level.level.as_ref().is_some_and(|at| at.rank == by_id(id).level.as_ref().map_or(0, |l| l.rank) + 1))
                .filter(|level| level.level.as_ref().is_some_and(|at| at.adds == LevelAdds::Animated))
                .map_or(id, |level| level.id.as_str());
            assert_eq!(worn, Some(above), "{id}");
            assert!(worn.is_some_and(|worn| by_id(worn).level.as_ref().is_some_and(|at| at.adds != LevelAdds::Signed)), "{id}");
        }

        // check_pick refuses each where it is not offered, owned or not, and takes it where it is offered and owned.
        const BARE: &str = "core-002";
        const ALL_THREE: &str = "classic-056";
        let owned = OwnedCosmetics {
            cards: R2942000_NEW
                .iter()
                .flat_map(|(id, _, _)| [BARE, ALL_THREE].map(|card| (card, *id)))
                .map(|(card, id)| CardCosmetic {
                    card_id: card.to_owned(),
                    cosmetic_id: id.to_owned(),
                })
                .collect(),
            account: Vec::new(),
        };
        for (id, kind, _) in R2942000_NEW {
            assert!(on(id).contains(ALL_THREE) && !on(id).contains(BARE), "{id}");
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(BARE)),
                Err(PickRefusal::NotOffered),
                "{id}"
            );
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(ALL_THREE)),
                Ok(()),
                "{id}"
            );
            // R3015005: a card's pick names the level, and it is worn only where it is offered.
            let worn = |card: &str| {
                worn_level(&cosmetics, level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]), |level| {
                    offered(level, Some(card))
                })
                .map(|level| level.id.as_str())
            };
            assert_eq!(worn(ALL_THREE), Some(id));
            assert_eq!(worn(BARE), None);
            // No retired entry owns one: owning a part 1 look never owns a part 3 one (R3015006).
            assert!(implying(id).next().is_none(), "{id}");
        }
    }

    #[test]
    fn r2943000_part_four_appends_set_showcase_and_out_of_frame() {
        use crate::economy::store::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let new = r2943000_new(&cosmetics);
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.kind, cosmetic.rarity))
                .collect::<Vec<_>>(),
            R2943000_NEW
        );
        for cosmetic in new {
            // Earned, per card, offered, naming no card, and Legendary, so a chase.
            assert!(
                !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious && cosmetic.chase,
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.kind.is_per_card() && cosmetic.kind.is_offered(),
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.card.is_none() && !cosmetic.offer.is_empty(),
                "{}",
                cosmetic.id
            );
            // R2949001: every clause samples by the version's own id, so it lands on cards of its own.
            assert!(
                cosmetic.offer.iter().all(|clause| clause
                    .sample
                    .as_ref()
                    .is_some_and(|sample| sample.seed == cosmetic.id)),
                "{}",
                cosmetic.id
            );
        }

        let defs = shipped_defs();
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let on = |id: &str| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| offered(by_id(id), Some(card)))
                .collect()
        };
        let non_tokens: BTreeSet<&str> = defs
            .iter()
            .filter(|(_, def)| !def.is_token())
            .map(|(id, _)| id.as_str())
            .collect();
        // Each version on a set of cards of its own: no two of the nine alike, none on every card.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
            VERSION_ALT_FULL_ART_ID,
            VERSION_EXTENDED_ART_ID,
            VERSION_PIXEL_ID,
            VERSION_COMIC_ID,
            VERSION_SET_SHOWCASE_ID,
            VERSION_OUT_OF_FRAME_ID,
        ];
        for (at, a) in versions.iter().enumerate() {
            assert!(on(a).len() < non_tokens.len(), "{a}");
            for b in &versions[at + 1..] {
                assert_ne!(on(a), on(b), "{a} and {b}");
            }
        }
        // Set showcase on Legendary and Mythic cards, Out of frame on Units of Rare and up but the Legacy cards
        // and Dormant Starship: every card each takes is of the types and rarities its clause names.
        for card in on(VERSION_SET_SHOWCASE_ID) {
            assert!(
                matches!(defs[card].rarity, Rarity::Legendary | Rarity::Mythic),
                "{card}"
            );
        }
        let out_of_frame = on(VERSION_OUT_OF_FRAME_ID);
        for card in &out_of_frame {
            assert!(
                defs[*card].type_ == CardType::Unit
                    && matches!(
                        defs[*card].rarity,
                        Rarity::Rare | Rarity::Epic | Rarity::Legendary | Rarity::Mythic
                    ),
                "{card}"
            );
        }
        let legacy = (41..=60).map(|number| format!("traininggrounds-{number:03}"));
        for card in legacy.chain(["stinkyscape-t-dormant-starship".to_owned()]) {
            assert!(defs.contains_key(&card), "{card}");
            assert!(!out_of_frame.contains(card.as_str()), "{card}");
        }

        // R3015005: a pick of one with the old Animated and Signature steps up only where its variant's next level
        // adds them: Signature never, since no level of it is signed above a normal-frame art.
        for (id, _, _) in R2943000_NEW {
            let worn = picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &[AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
            let above = ladder(&cosmetics, &by_id(id).level.as_ref().map_or(String::new(), |level| level.variant.clone()))
                .into_iter()
                .find(|level| level.level.as_ref().is_some_and(|at| at.rank == by_id(id).level.as_ref().map_or(0, |l| l.rank) + 1))
                .filter(|level| level.level.as_ref().is_some_and(|at| at.adds == LevelAdds::Animated))
                .map_or(id, |level| level.id.as_str());
            assert_eq!(worn, Some(above), "{id}");
            assert!(worn.is_some_and(|worn| by_id(worn).level.as_ref().is_some_and(|at| at.adds != LevelAdds::Signed)), "{id}");
        }

        // check_pick refuses each where it is not offered, owned or not, and takes it where it is offered and owned.
        const BARE: &str = "core-002";
        const PART_FOUR: &str = "classic-080";
        let owned = OwnedCosmetics {
            cards: R2943000_NEW
                .iter()
                .flat_map(|(id, _, _)| [BARE, PART_FOUR].map(|card| (card, *id)))
                .map(|(card, id)| CardCosmetic {
                    card_id: card.to_owned(),
                    cosmetic_id: id.to_owned(),
                })
                .collect(),
            account: Vec::new(),
        };
        for (id, kind, _) in R2943000_NEW {
            assert!(on(id).contains(PART_FOUR) && !on(id).contains(BARE), "{id}");
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(BARE)),
                Err(PickRefusal::NotOffered),
                "{id}"
            );
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(PART_FOUR)),
                Ok(()),
                "{id}"
            );
            // R3015005: a card's pick names the level, and it is worn only where it is offered.
            let worn = |card: &str| {
                worn_level(&cosmetics, level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]), |level| {
                    offered(level, Some(card))
                })
                .map(|level| level.id.as_str())
            };
            assert_eq!(worn(PART_FOUR), Some(id));
            assert_eq!(worn(BARE), None);
            // No retired entry owns one: owning a part 1 look never owns a part 4 one (R3015006).
            assert!(implying(id).next().is_none(), "{id}");
        }
    }

    #[test]
    fn r2944000_part_five_appends_immersive_and_prismatic() {
        use crate::economy::store::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let new = r2944000_new(&cosmetics);
        assert_eq!(
            new.iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.kind, cosmetic.rarity))
                .collect::<Vec<_>>(),
            R2944000_NEW
        );
        for cosmetic in new {
            // Earned, per card, offered, naming no card, and Mythic, so a chase.
            assert!(
                !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious && cosmetic.chase,
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.kind.is_per_card() && cosmetic.kind.is_offered(),
                "{}",
                cosmetic.id
            );
            assert!(
                cosmetic.card.is_none() && !cosmetic.offer.is_empty(),
                "{}",
                cosmetic.id
            );
            // R2949001: every clause that samples does so by the version's own id, so it lands on cards of its
            // own, and at least one samples (Prismatic's Mythic clause takes every Mythic card).
            let samples: Vec<&OfferSample> = cosmetic
                .offer
                .iter()
                .filter_map(|clause| clause.sample.as_ref())
                .collect();
            assert!(
                !samples.is_empty() && samples.iter().all(|sample| sample.seed == cosmetic.id),
                "{}",
                cosmetic.id
            );
        }

        let defs = shipped_defs();
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let on = |id: &str| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| offered(by_id(id), Some(card)))
                .collect()
        };
        let non_tokens: BTreeSet<&str> = defs
            .iter()
            .filter(|(_, def)| !def.is_token())
            .map(|(id, _)| id.as_str())
            .collect();
        // Each version on a set of cards of its own: no two of the eleven alike, none on every card.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
            VERSION_ALT_FULL_ART_ID,
            VERSION_EXTENDED_ART_ID,
            VERSION_PIXEL_ID,
            VERSION_COMIC_ID,
            VERSION_SET_SHOWCASE_ID,
            VERSION_OUT_OF_FRAME_ID,
            VERSION_IMMERSIVE_ID,
            VERSION_PRISMATIC_ID,
        ];
        for (at, a) in versions.iter().enumerate() {
            assert!(on(a).len() < non_tokens.len(), "{a}");
            for b in &versions[at + 1..] {
                assert_ne!(on(a), on(b), "{a} and {b}");
            }
        }
        // Immersive and Prismatic on Legendary and Mythic cards alone; Immersive never on Dormant Starship, and
        // Prismatic on every Mythic card.
        let immersive = on(VERSION_IMMERSIVE_ID);
        let prismatic = on(VERSION_PRISMATIC_ID);
        for card in immersive.iter().chain(&prismatic) {
            assert!(
                matches!(defs[*card].rarity, Rarity::Legendary | Rarity::Mythic),
                "{card}"
            );
        }
        const STARSHIP: &str = "stinkyscape-t-dormant-starship";
        assert!(non_tokens.contains(STARSHIP) && !immersive.contains(STARSHIP));
        for card in non_tokens
            .iter()
            .filter(|card| defs[**card].rarity == Rarity::Mythic)
        {
            assert!(prismatic.contains(card), "{card}");
        }

        // R3015005: a pick of one with the old Animated and Signature steps up only where its variant's next level
        // adds them: Signature never, since no level of it is signed above a normal-frame art.
        for (id, _, _) in R2944000_NEW {
            let worn = picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &[AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]);
            let above = ladder(&cosmetics, &by_id(id).level.as_ref().map_or(String::new(), |level| level.variant.clone()))
                .into_iter()
                .find(|level| level.level.as_ref().is_some_and(|at| at.rank == by_id(id).level.as_ref().map_or(0, |l| l.rank) + 1))
                .filter(|level| level.level.as_ref().is_some_and(|at| at.adds == LevelAdds::Animated))
                .map_or(id, |level| level.id.as_str());
            assert_eq!(worn, Some(above), "{id}");
            assert!(worn.is_some_and(|worn| by_id(worn).level.as_ref().is_some_and(|at| at.adds != LevelAdds::Signed)), "{id}");
        }

        // check_pick refuses each where it is not offered, owned or not, and takes it where it is offered and owned.
        const BARE: &str = "core-002";
        const PART_FIVE: &str = "core-100";
        let owned = OwnedCosmetics {
            cards: R2944000_NEW
                .iter()
                .flat_map(|(id, _, _)| [BARE, PART_FIVE].map(|card| (card, *id)))
                .map(|(card, id)| CardCosmetic {
                    card_id: card.to_owned(),
                    cosmetic_id: id.to_owned(),
                })
                .collect(),
            account: Vec::new(),
        };
        for (id, kind, _) in R2944000_NEW {
            assert!(on(id).contains(PART_FIVE) && !on(id).contains(BARE), "{id}");
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(BARE)),
                Err(PickRefusal::NotOffered),
                "{id}"
            );
            assert_eq!(
                check_pick(&cosmetics, &owned, kind, id, Some(PART_FIVE)),
                Ok(()),
                "{id}"
            );
            // R3015005: a card's pick names the level, and it is worn only where it is offered.
            let worn = |card: &str| {
                worn_level(&cosmetics, level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]), |level| {
                    offered(level, Some(card))
                })
                .map(|level| level.id.as_str())
            };
            assert_eq!(worn(PART_FIVE), Some(id));
            assert_eq!(worn(BARE), None);
            // No retired entry owns one: owning a part 1 look never owns a part 5 one (R3015006).
            assert!(implying(id).next().is_none(), "{id}");
        }
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
            vec!["the card \"core-001\" has two Common full arts".to_owned()]
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
        cosmetics[0].starter = false;
        let ids: Vec<String> = pack_cosmetics(&cosmetics)
            .into_iter()
            .map(|cosmetic| cosmetic.id)
            .collect();
        // R2725000: a full art is retired, so no duplicate drops one.
        assert_eq!(ids, vec!["foil-new", "finish-new", "border-new"]);
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
                    "the cosmetic \"foil-wrong\" is {rarity} and its chase mark is {chase}, but a cosmetic is a chase exactly when it is Legendary or Mythic and not Jlorious"
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

        // The shipped chases: two Legendary starters every account owns, #1833's fifteen (R1833000),
        // then #2655's 42 full arts, a Legendary and a Mythic on each of 21 cards (R2655000), then #2725's
        // Full art version and Signature augment (R2725000), then #2940's Alt full art version (R2940000), then
        // #2943's Set showcase and Out of frame versions (R2943000), then #2944's Immersive and Prismatic versions
        // (R2944000).
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let chases: Vec<&Cosmetic> = cosmetics.iter().filter(|cosmetic| cosmetic.chase).collect();
        assert_eq!(chases.len(), 17 + 42 + 7);
        assert_eq!(
            chases[17 + 42..]
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            [
                VERSION_FULL_ART_ID,
                AUGMENT_SIGNATURE_ID,
                VERSION_ALT_FULL_ART_ID,
                VERSION_SET_SHOWCASE_ID,
                VERSION_OUT_OF_FRAME_ID,
                VERSION_IMMERSIVE_ID,
                VERSION_PRISMATIC_ID,
            ]
        );
        let full_art_chases: Vec<&str> = r2655000_new(&cosmetics)
            .iter()
            .filter(|cosmetic| cosmetic.chase)
            .map(|cosmetic| cosmetic.id.as_str())
            .collect();
        assert_eq!(full_art_chases.len(), 42);
        assert_eq!(
            chases[17..17 + 42]
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            full_art_chases
        );
        assert!(
            chases[17..17 + 42]
                .iter()
                .all(|cosmetic| cosmetic.kind == CosmeticKind::FullArt && cosmetic.rarity.is_chase())
        );
        assert_eq!(
            chases[..17]
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
        // R1295000 and R1297000 unlock every item they shipped (§14.2), so no pack drops one; the six
        // full arts and the portraits but vanilla are owned instead (R1313000, R1827000). #1833's 36
        // are no starters (R1833000).
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let earlier = before_r1833000(&cosmetics);
        assert!(
            earlier.iter().all(|cosmetic| cosmetic.starter
                == !(cosmetic.kind == CosmeticKind::FullArt
                    || (cosmetic.kind == CosmeticKind::Portrait && !cosmetic.default))),
            "{earlier:?}"
        );
        // The six earlier Epic full arts.
        let full_arts: Vec<&Cosmetic> = earlier
            .iter()
            .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
            .collect();
        assert_eq!(full_arts.len(), 6);
        assert!(
            full_arts
                .iter()
                .all(|cosmetic| cosmetic.rarity == CosmeticRarity::Epic)
        );
        // A duplicate drops #1833's 18 per-card ones, then #2725's version and augments (R2725000), which
        // retire every full art: those six, #2655's 57 and #2715's 21; then #2940's three versions (R2940000),
        // #2942's three (R2942000), #2943's two (R2943000) and #2944's two (R2944000).
        let mut dropped: Vec<Cosmetic> = cosmetics[earlier.len()..earlier.len() + R1833000_NEW.len()]
            .iter()
            .filter(|cosmetic| cosmetic.kind.is_per_card())
            .cloned()
            .collect();
        assert_eq!(dropped.len(), 18);
        dropped.extend(r2725000_new(&cosmetics).iter().cloned());
        dropped.extend(r2940000_new(&cosmetics).iter().cloned());
        dropped.extend(r2942000_new(&cosmetics).iter().cloned());
        dropped.extend(r2943000_new(&cosmetics).iter().cloned());
        dropped.extend(r2944000_new(&cosmetics).iter().cloned());
        assert_eq!(
            dropped.len(),
            18 + R2725000_NEW.len()
                + R2940000_NEW.len()
                + R2942000_NEW.len()
                + R2943000_NEW.len()
                + R2944000_NEW.len()
        );
        assert_eq!(pack_cosmetics(&cosmetics), dropped);
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
        // The global roll drops the five in registry order, then #1833's account cosmetics
        // (R1833000); a cosmetic pack may too, a duplicate's per-card roll never.
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
        // #1833's 72, #2655's 57 after them (R2655000), #2755's one (R2755000), #2715's 21 (R2715000),
        // #2725's three (R2725000), #2940's three (R2940000), #2942's three (R2942000), #2943's two (R2943000)
        // and #2944's two last (R2944000).
        assert_eq!(
            cosmetics.len(),
            72 + R2655000_NEW
                + R2755000_NEW
                + R2715000_NEW
                + R2725000_NEW.len()
                + R2940000_NEW.len()
                + R2942000_NEW.len()
                + R2943000_NEW.len()
                + R2944000_NEW.len()
        );
        // Appended after every earlier entry, the portraits last among those, so none moves.
        let earlier = before_r1833000(&cosmetics);
        assert_eq!(
            earlier.last().map(|cosmetic| cosmetic.id.as_str()),
            Some("portrait-shredder")
        );
        let new = &cosmetics[earlier.len()..earlier.len() + R1833000_NEW.len()];
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
        assert!(owns_portrait(PortraitId::Gary, &["portrait-gary".to_owned()]));
        assert!(!owns_portrait(PortraitId::Gary, &["table-astral".to_owned()]));
    }

    #[test]
    fn r1827000_owns_portrait_counts_the_starter_and_the_ledger_s_rows() {
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

        assert_eq!(check(&none, CosmeticKind::Table, "table-tavern", None), Ok(()));
        assert_eq!(check(&none, CosmeticKind::Table, "table-felt", None), Ok(()));
        assert_eq!(
            check(&none, CosmeticKind::Foil, "foil-none", Some("core-002")),
            Ok(())
        );
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
        assert_eq!(
            registry().len(),
            shipped().map(|cosmetics| cosmetics.len()).unwrap_or(0)
        );
    }

    #[test]
    fn r2755000_the_jlorious_card_back_ships_after_the_full_arts_mythic_and_no_pool_offers_it() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        // Appended after #2655's full arts, with only #2715's Animated arts (R2715000), #2725's three
        // (R2725000), #2940's three (R2940000), #2942's three (R2942000), #2943's two (R2943000) and #2944's two
        // (R2944000) after it.
        let back = &cosmetics[cosmetics.len()
            - R2944000_NEW.len()
            - R2943000_NEW.len()
            - R2942000_NEW.len()
            - R2940000_NEW.len()
            - R2725000_NEW.len()
            - R2715000_NEW
            - R2755000_NEW];
        assert_eq!(back.id, JLORIOUS_CARD_BACK);
        assert_eq!(back.kind, CosmeticKind::CardBack);
        assert_eq!(back.rarity, CosmeticRarity::Mythic);
        assert!(back.jlorious && !back.chase && !back.starter && !back.default);
        // The only Jlorious cosmetic, and neither a duplicate's global roll nor a cosmetic pack
        // offers it.
        assert_eq!(cosmetics.iter().filter(|cosmetic| cosmetic.jlorious).count(), 1);
        assert!(
            global_cosmetics(&cosmetics)
                .iter()
                .all(|cosmetic| cosmetic.id != JLORIOUS_CARD_BACK)
        );
        assert!(
            cosmetic_pack_cosmetics(&cosmetics)
                .iter()
                .all(|cosmetic| cosmetic.id != JLORIOUS_CARD_BACK)
        );
    }

    #[test]
    fn r2755000_a_jlorious_cosmetic_is_never_a_chase_a_default_a_starter_or_per_card() {
        let jlorious = Cosmetic {
            rarity: CosmeticRarity::Mythic,
            starter: false,
            jlorious: true,
            ..entry("card-back-earned", CosmeticKind::CardBack, false)
        };
        let mut cosmetics = defaults();
        cosmetics.push(jlorious.clone());
        assert!(problems(&cosmetics).is_empty(), "{:?}", problems(&cosmetics));
        // Its rarity is the chase tier's, but nothing drops it, so a chase mark is refused.
        let mut chased = defaults();
        chased.push(Cosmetic {
            chase: true,
            ..jlorious.clone()
        });
        assert_eq!(problems(&chased).len(), 1, "{:?}", problems(&chased));
        for wrong in [
            Cosmetic {
                starter: true,
                ..jlorious.clone()
            },
            Cosmetic {
                kind: CosmeticKind::Foil,
                id: "foil-earned".to_owned(),
                ..jlorious.clone()
            },
        ] {
            let mut cosmetics = defaults();
            cosmetics.push(wrong.clone());
            assert!(
                problems(&cosmetics)
                    .iter()
                    .any(|problem| problem.contains("only reaching Jlorious may grant")),
                "{wrong:?}: {:?}",
                problems(&cosmetics)
            );
        }
    }
}
