//! The cosmetics registry (#1295, #1297, SPEC §11 R1295000): every cosmetic a player can pick, in
//! `crates/shared/cosmetics.json`. The pack roll ([`pack_cosmetics`], R1301002) and ownership read it here;
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
//!
//! R3017000: #3017 appends six two-level variants, each an art level and its animated one: Night
//! ([`VERSION_NIGHT_ID`]), Watercolor ([`VERSION_WATERCOLOR_ID`]) and Woodcut ([`VERSION_WOODCUT_ID`]), Rare, and
//! Neon sign ([`VERSION_NEON_ID`]), Stained-glass window ([`VERSION_STAINED_GLASS_ID`]) and Tarot card
//! ([`VERSION_TAROT_ID`]), Epic. R3017001: they make three pairs, each splitting every card between its two by one
//! seed, a sample's `from` ([`OfferSample`]) taking the cards the other's percent leaves, so every card, Legacy ones
//! included, is offered at least `VARIANT_MIN_PER_CARD` variants besides the Default.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use jackioh_engine::replay::fnv1a32_utf16_u32;
use jackioh_engine::wire::emotes::{DEFAULT_PORTRAIT, PORTRAIT_IDS, PortraitId};
use jackioh_engine::{CardDef, CardDefs, CardType, Rarity, SetName, Tag};

use crate::config::{
    FLEET_TRACK_LEVELS, GLOBAL_COSMETIC_FLOOR_RARITY, OFFER_SAMPLE_PERCENT_SCALE,
    PLAYER_SETTINGS_TEXT_MAX_LENGTH,
};

/// The registry's text, compiled in.
pub const COSMETICS_JSON: &str = include_str!("../cosmetics.json");

/// One row of `public.card_cosmetics` (§14.2): a per-card cosmetic owned on one card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct CardCosmetic {
    pub card_id: String,
    pub cosmetic_id: String,
}

/// An account's cosmetics (§14.2): per card, and the account cosmetics by id. The starter
/// cosmetics, owned by every account, have no row.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct OwnedCosmetics {
    pub cards: Vec<CardCosmetic>,
    pub account: Vec<String>,
}

/// R2655000: a card's own full-art pick: `false` (no full art), `true` (its Epic full art, as every
/// pick before #2655 named it) or a full art's registry id, which travels as the id's text. An id
/// is kept as written and judged where it is worn, as a foil's is (`cosmetics::full_art_picked`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(untagged)]
pub enum FullArtPick {
    On(bool),
    Id(String),
}

impl From<bool> for FullArtPick {
    fn from(on: bool) -> FullArtPick {
        FullArtPick::On(on)
    }
}

impl FullArtPick {
    /// Whether the pick prints a full art at all: `true` or any id.
    pub fn prints(&self) -> bool {
        !matches!(self, FullArtPick::On(false))
    }

    /// The id the pick names, if it names one rather than a switch.
    pub fn id(&self) -> Option<&str> {
        match self {
            FullArtPick::Id(id) => Some(id),
            FullArtPick::On(_) => None,
        }
    }

    /// The pick a row of `public.card_cosmetic_picks` holds: its `full_art` switch, and the id
    /// migration 0072 keeps beside a switch that is on.
    pub fn of_row(full_art: Option<bool>, id: Option<String>) -> Option<FullArtPick> {
        match (full_art, id) {
            (Some(true), Some(id)) => Some(FullArtPick::Id(id)),
            (Some(on), _) => Some(FullArtPick::On(on)),
            (None, _) => None,
        }
    }
}

/// The slot a cosmetic fills: on a card, a foil, a finish or a border (R1295000); on a deck, the
/// playmat (`Table`), the backdrop (`Field`) or the back of its face-down cards and deck
/// (`CardBack`) (R1297000); a card's full-art face (`FullArt`), picked only by the card's own pick
/// (R1313000, R2655000); the deck's hero portrait (`Portrait`), an account cosmetic (R641, R1827000); a card's
/// version (`Version`), the one look it wears, and an augment (`Augment`) layered on whichever version it wears,
/// each per card and picked by the card's own pick alone (R2725000). R3335000: four account kinds dress every
/// Starship the account launches: its hull design (`StarshipHull`), its colours and emblem (`StarshipLivery`), its
/// engine glow and launch trail (`StarshipTrail`) and its modules' recolour (`StarshipPieceSkin`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
    StarshipHull,
    StarshipLivery,
    StarshipTrail,
    StarshipPieceSkin,
}

impl CosmeticKind {
    /// The kinds a player picks one of in the settings, in list order, one default each; `FullArt`
    /// is none (R1313000), and `Portrait` is a deck's pick alone (R1827000). R3335000: the four Starship kinds
    /// last, picked on the account alone.
    pub const PICKED: [Self; 10] = [
        Self::Foil,
        Self::Finish,
        Self::Border,
        Self::Table,
        Self::Field,
        Self::CardBack,
        Self::StarshipHull,
        Self::StarshipLivery,
        Self::StarshipTrail,
        Self::StarshipPieceSkin,
    ];

    /// The kinds with exactly one default: those of [`Self::PICKED`], then `Portrait`, whose
    /// default is `portrait-vanilla` (R641, R1827000).
    pub const DEFAULTED: [Self; 11] = [
        Self::Foil,
        Self::Finish,
        Self::Border,
        Self::Table,
        Self::Field,
        Self::CardBack,
        Self::StarshipHull,
        Self::StarshipLivery,
        Self::StarshipTrail,
        Self::StarshipPieceSkin,
        Self::Portrait,
    ];

    /// R3335000: the four kinds that dress a launched Starship, in list order.
    pub const STARSHIP: [Self; 4] = [
        Self::StarshipHull,
        Self::StarshipLivery,
        Self::StarshipTrail,
        Self::StarshipPieceSkin,
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
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
    /// R3335002: the earned track that alone pays it, the Fleet track (§14.15). Gated as a Jlorious one is: no roll,
    /// pack, token, Chase Shard bar or ladder bot's wardrobe offers it, so it is never a chase, a default, a starter
    /// or per card ([`Cosmetic::earned_only`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<CosmeticTrack>,
    /// R3335001: the level of its `track` that pays it, from 1; nothing else has one.
    #[serde(default, rename = "trackLevel", skip_serializing_if = "Option::is_none")]
    pub track_level: Option<u32>,
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

/// R3335002: an earned track that alone pays a cosmetic (`Cosmetic::track`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub enum CosmeticTrack {
    /// §14.15: the Fleet track, earned by playing Stinkyscape in recorded games.
    Fleet,
}

impl Cosmetic {
    /// R3335002: owned only by earning it, by reaching Jlorious (R2755000) or by a track's level (R3335001): no
    /// roll, pack, token, Chase Shard bar or ladder bot's wardrobe offers it.
    pub fn earned_only(&self) -> bool {
        self.jlorious || self.track.is_some()
    }

    /// R3015000: a retired entry, kept with its owners and read for what it maps to ([`implied_by`]), but dropped,
    /// sold and picked by nothing: the full arts (R2725000) and the Animated and Signature augments.
    pub fn is_retired(&self) -> bool {
        matches!(self.kind, CosmeticKind::FullArt | CosmeticKind::Augment)
    }
}

/// R3015000: what a level adds to the one below it: the variant's picture in its normal frame (`Art`), across the
/// whole face (`FullArt`), its picture's own layers moving (`Animated`, R2715001) or a signature (`Signed`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(deny_unknown_fields)]
pub struct VariantLevel {
    pub variant: String,
    pub rank: u8,
    pub adds: LevelAdds,
}

/// R2725001: one clause of an offer. It takes a non-token card of the catalog when each field it fills lists
/// the card's type, rarity, set and id, a field left empty taking any, and `except` does not name it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
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
/// `<seed>/<card id>`, modulo `OFFER_SAMPLE_PERCENT_SCALE`, falls below `percent`. R3017001: and at or above
/// `from`, 0 when absent, so two samples of one seed, one below a cut and one from it, split their cards between
/// them. The registry's check holds the share to some and not all, `percent` to the scale, and `seed` to a slug.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(deny_unknown_fields)]
pub struct OfferSample {
    pub seed: String,
    /// R3017001: the lowest hash the sample takes; 0, and left out of the registry's text, unless set.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub from: u32,
    pub percent: u32,
}

/// Whether `n` is 0: a sample's `from` left out of its text.
fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl OfferSample {
    /// Whether the sample takes the card `card`.
    pub fn takes(&self, card: &str) -> bool {
        let at = fnv1a32_utf16_u32(&format!("{}/{card}", self.seed)) % OFFER_SAMPLE_PERCENT_SCALE;
        self.from <= at && at < self.percent
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
        .filter(|cosmetic| {
            cosmetic
                .level
                .as_ref()
                .is_some_and(|level| level.variant == variant)
        })
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
/// entry that maps to the level `id` ([`implied_by`]). The mapping is read where it is worn; migration 0084 wrote
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
    let level = |id: &str| {
        registry
            .iter()
            .find(|cosmetic| cosmetic.id == id && cosmetic.level.is_some())
    };
    let mut worn = match full_art {
        None | Some(FullArtPick::On(false)) => None,
        Some(FullArtPick::On(true)) => full_art_of(registry, card).and(level(VERSION_FULL_ART_ID)),
        Some(FullArtPick::Id(id)) => match registry.iter().find(|cosmetic| cosmetic.id == *id) {
            Some(cosmetic) if cosmetic.level.is_some() => Some(cosmetic),
            Some(cosmetic)
                if cosmetic.kind == CosmeticKind::FullArt && cosmetic.card.as_deref() == Some(card) =>
            {
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
            .find(|above| {
                above
                    .level
                    .as_ref()
                    .is_some_and(|up| up.rank == at.rank + 1 && up.adds == adds)
            })
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
    levels_below(registry, level)
        .into_iter()
        .rev()
        .find(|below| wears(below))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    cosmetics: Vec<Cosmetic>,
}

/// An id is a lower-case letter, then lower-case letters, digits and hyphens (the quest pool's too).
pub fn is_slug(id: &str) -> bool {
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
        // R2949001: a sample takes some of its clause's cards and not all, by a slug's seed. R3017001: its share runs
        // from `from` to below `percent`, which stays within the scale.
        for sample in cosmetic.offer.iter().filter_map(|clause| clause.sample.as_ref()) {
            let share = sample.percent.saturating_sub(sample.from);
            if share == 0 || share >= OFFER_SAMPLE_PERCENT_SCALE {
                out.push(format!(
                    "the offer of {id:?} samples {share} in {OFFER_SAMPLE_PERCENT_SCALE} of its cards, which must be some and not all"
                ));
            }
            if sample.percent > OFFER_SAMPLE_PERCENT_SCALE {
                out.push(format!(
                    "the offer of {id:?} samples below {}, past {OFFER_SAMPLE_PERCENT_SCALE}",
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
        // R1333000: a chase is exactly a Legendary or Mythic cosmetic, but a Jlorious one or a track's, which
        // nothing drops (R2755000, R3335002).
        if cosmetic.chase != (cosmetic.rarity.is_chase() && !cosmetic.earned_only()) {
            out.push(format!(
                "the cosmetic {id:?} is {} and its chase mark is {}, but a cosmetic is a chase exactly when it is Legendary or Mythic and not earned only",
                cosmetic.rarity, cosmetic.chase
            ));
        }
        // R2755000, R3335002: a Jlorious cosmetic or a track's is an account cosmetic every account must earn.
        if cosmetic.earned_only() && (cosmetic.kind.is_per_card() || cosmetic.default || cosmetic.starter) {
            out.push(format!(
                "the earned-only cosmetic {id:?} is a per-card cosmetic, a default or a starter, which only reaching Jlorious or a track's level may grant"
            ));
        }
        // R3335002: one way to earn it; R3335001: a track's cosmetic names its level, and nothing else does.
        if cosmetic.jlorious && cosmetic.track.is_some() {
            out.push(format!(
                "the cosmetic {id:?} is both Jlorious and a track's, but one way earns it"
            ));
        }
        if cosmetic.track.is_some() != cosmetic.track_level.is_some() {
            out.push(format!(
                "the cosmetic {id:?} has a track without a level or a level without a track"
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
    fleet_problems(cosmetics, &mut out);
    out
}

/// R3335001: what is wrong with the Fleet track's rewards: a registry that lists one lists the whole track, one per
/// level from 1 to `FLEET_TRACK_LEVELS`, each level once, the last [`FLEET_CAPSTONE`], a Mythic hull design. The
/// shipped registry lists it (`r3335002_the_fleet_track_has_one_reward_per_level_the_stinkyscape_hull_last`).
fn fleet_problems(cosmetics: &[Cosmetic], out: &mut Vec<String>) {
    if cosmetics.iter().all(|cosmetic| cosmetic.track.is_none()) {
        return;
    }
    let mut levels: Vec<u32> = cosmetics
        .iter()
        .filter(|cosmetic| cosmetic.track == Some(CosmeticTrack::Fleet))
        .filter_map(|cosmetic| cosmetic.track_level)
        .collect();
    levels.sort_unstable();
    if !levels.iter().copied().eq(1..=FLEET_TRACK_LEVELS) {
        out.push(format!(
            "the Fleet track's rewards are at the levels {levels:?}, but it pays one at each level from 1 to {FLEET_TRACK_LEVELS}"
        ));
    }
    let last = cosmetics.iter().find(|cosmetic| {
        cosmetic.track == Some(CosmeticTrack::Fleet) && cosmetic.track_level == Some(FLEET_TRACK_LEVELS)
    });
    if last.is_none_or(|last| last.id != FLEET_CAPSTONE) {
        out.push(format!(
            "the Fleet track's level {FLEET_TRACK_LEVELS} does not pay {FLEET_CAPSTONE:?}, its capstone"
        ));
    }
    if let Some(capstone) = cosmetics.iter().find(|cosmetic| cosmetic.id == FLEET_CAPSTONE)
        && (capstone.kind != CosmeticKind::StarshipHull || capstone.rarity != CosmeticRarity::Mythic)
    {
        out.push(format!(
            "the Fleet capstone {FLEET_CAPSTONE:?} is a {} {:?}, but it is a Mythic hull design",
            capstone.rarity, capstone.kind
        ));
    }
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
        let ranks: Vec<u8> = levels
            .iter()
            .filter_map(|c| c.level.as_ref().map(|l| l.rank))
            .collect();
        if !ranks
            .iter()
            .zip(first..)
            .all(|(rank, expected)| *rank == expected)
        {
            out.push(format!(
                "the variant {variant:?} has the ranks {ranks:?}, which must run gapless from {first}"
            ));
        }
        let adds: Vec<LevelAdds> = levels
            .iter()
            .filter_map(|c| c.level.as_ref().map(|l| l.adds))
            .collect();
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
                out.push(format!(
                    "the signed level {:?} is {}, not Mythic",
                    level.id, level.rarity
                ));
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
/// them. A Jlorious one is never among them (R2755000), nor a track's (R3335002).
pub fn global_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| {
            !cosmetic.kind.is_per_card()
                && !cosmetic.default
                && !cosmetic.starter
                && !cosmetic.earned_only()
        })
        .cloned()
        .collect()
}

/// The cosmetics a cosmetic pack may drop (R1306000): every entry, per-card or account, that is
/// not a default or a starter, in the registry's order. A chase drops when a slot lands on its
/// tier (R1333000), and a per-card one on a card it is offered (R2725001). A Jlorious one never drops
/// (R2755000), nor a track's (R3335002), nor a retired full art (R2725000) or augment (R3015000).
pub fn cosmetic_pack_cosmetics(cosmetics: &[Cosmetic]) -> Vec<Cosmetic> {
    cosmetics
        .iter()
        .filter(|cosmetic| {
            !cosmetic.default && !cosmetic.starter && !cosmetic.earned_only() && !cosmetic.is_retired()
        })
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

/// R3017000: the Night variant's first level, Rare: a new picture of the card's subject under a night sky, a moon
/// and stars. It pairs with Neon sign: every card is offered one of the two (R3017001).
pub const VERSION_NIGHT_ID: &str = "version-night";

/// R3017000: the Neon sign variant's first level, Epic: the card's own picture as glowing tubes on a dark wall.
pub const VERSION_NEON_ID: &str = "version-neon";

/// R3017000: the Watercolor variant's first level, Rare: a new picture of the card's subject in washes on paper.
/// It pairs with Stained-glass window (R3017001).
pub const VERSION_WATERCOLOR_ID: &str = "version-watercolor";

/// R3017000: the Stained-glass window variant's first level, Epic: a new picture of the card's subject in leaded
/// panes of flat colour.
pub const VERSION_STAINED_GLASS_ID: &str = "version-stained-glass";

/// R3017000: the Woodcut variant's first level, Rare: the card's own picture cut as a three-tone print. It pairs
/// with Tarot card (R3017001).
pub const VERSION_WOODCUT_ID: &str = "version-woodcut";

/// R3017000: the Tarot card variant's first level, Epic: a new picture of the card's subject as a fortune card's,
/// under an arch with a sun, a crescent and stars.
pub const VERSION_TAROT_ID: &str = "version-tarot";

/// R2755000: the card back a rated game grants each person it leaves Jlorious, and nothing else
/// grants (`Cosmetic::jlorious`).
pub const JLORIOUS_CARD_BACK: &str = "card-back-jlorious";

/// R3335003: the Jlorious Starship, a hull design in its own livery, granted beside [`JLORIOUS_CARD_BACK`].
pub const JLORIOUS_STARSHIP: &str = "starship-hull-jlorious";

/// R2755000, R3335003: every cosmetic a rated game grants each person it leaves Jlorious, the card back first.
pub const JLORIOUS_COSMETICS: [&str; 2] = [JLORIOUS_CARD_BACK, JLORIOUS_STARSHIP];

/// R3335001: the Fleet track's last level's reward, the Stinkyscape hull: a Mythic flagship.
pub const FLEET_CAPSTONE: &str = "starship-hull-stinkyscape";

/// R3335001: the cosmetic the Fleet track's `level` pays, from 1 to `FLEET_TRACK_LEVELS`; none past it.
pub fn fleet_reward(level: u32) -> Option<&'static Cosmetic> {
    registry().iter().find(|cosmetic| {
        cosmetic.track == Some(CosmeticTrack::Fleet) && cosmetic.track_level == Some(level)
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
            track: None,
            track_level: None,
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
            entry("starship-hull-standard", CosmeticKind::StarshipHull, true),
            entry("starship-livery-standard", CosmeticKind::StarshipLivery, true),
            entry("starship-trail-standard", CosmeticKind::StarshipTrail, true),
            entry("starship-piece-skin-standard", CosmeticKind::StarshipPieceSkin, true),
        ];
        cosmetics.extend(portraits());
        cosmetics
    }

    /// R3015005: the id of the level a card's pick of `full_art` and `augments` names on `card`, owned or not.
    fn picked<'r>(
        registry: &'r [Cosmetic],
        card: &str,
        full_art: FullArtPick,
        augments: &[&str],
    ) -> Option<&'r str> {
        let augments: Vec<String> = augments.iter().map(|id| (*id).to_owned()).collect();
        level_of_pick(registry, card, Some(&full_art), &augments).map(|level| level.id.as_str())
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

    /// R3015000: the fifteen levels #3015 appends after #2944's two, last in the registry but for #3017's twelve.
    const R3015000_NEW: usize = 15;

    /// R3017000: the twelve levels #3017 appends after #3015's fifteen, last in the registry but for #3335's.
    const R3017000_NEW: usize = 12;

    /// R3335000: the 35 #3335 appends after #3017's twelve, last in the registry: the four Starship defaults, the
    /// Jlorious Starship (R3335003) and the Fleet track's thirty rewards (R3335001).
    const R3335000_NEW: usize = 4 + 1 + 30;

    /// The shipped registry before #3335's 35 (R3335000).
    fn before_r3335000(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        &cosmetics[..cosmetics.len() - R3335000_NEW]
    }

    /// The 35 #3335 appends, last in the registry (R3335000).
    fn r3335000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        &cosmetics[cosmetics.len() - R3335000_NEW..]
    }

    /// The shipped registry before #3017's twelve levels (R3017000).
    fn before_r3017000(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3335000(cosmetics);
        &cosmetics[..cosmetics.len() - R3017000_NEW]
    }

    /// The twelve #3017 appends, last in the registry but for #3335's (R3017000).
    fn r3017000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3335000(cosmetics);
        &cosmetics[cosmetics.len() - R3017000_NEW..]
    }

    /// The shipped registry before #3015's fifteen levels (R3015000), which every slice below is taken from.
    fn before_r3015000(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3017000(cosmetics);
        &cosmetics[..cosmetics.len() - R3015000_NEW]
    }

    /// The fifteen #3015 appends, last in the registry but for #3017's twelve (R3015000).
    fn r3015000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3017000(cosmetics);
        &cosmetics[cosmetics.len() - R3015000_NEW..]
    }

    /// The shipped registry before #1833: every entry but #2944's two, #2943's two before them, #2942's three
    /// before those, #2940's three before those, #2725's three before those, #2715's 21 before those, #2755's one
    /// before those, #2655's 57 before it and #1833's 36 before those (R1833000).
    fn before_r1833000(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3015000(cosmetics);
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
        let cosmetics = before_r3015000(cosmetics);
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
        let cosmetics = before_r3015000(cosmetics);
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
        let cosmetics = before_r3015000(cosmetics);
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
        let cosmetics = before_r3015000(cosmetics);
        let end = cosmetics.len() - R2944000_NEW.len() - R2943000_NEW.len() - R2942000_NEW.len();
        &cosmetics[end - R2940000_NEW.len()..end]
    }

    /// The three #2942 appends, last in the registry but for #2943's two and #2944's two (R2942000).
    fn r2942000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3015000(cosmetics);
        let end = cosmetics.len() - R2944000_NEW.len() - R2943000_NEW.len();
        &cosmetics[end - R2942000_NEW.len()..end]
    }

    /// The two #2943 appends, last in the registry but for #2944's two (R2943000).
    fn r2943000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3015000(cosmetics);
        let end = cosmetics.len() - R2944000_NEW.len();
        &cosmetics[end - R2943000_NEW.len()..end]
    }

    /// The two #2944 appends, last in the registry but for #3015's fifteen (R2944000).
    fn r2944000_new(cosmetics: &[Cosmetic]) -> &[Cosmetic] {
        let cosmetics = before_r3015000(cosmetics);
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

    /// The kinds a player picked before #3335 added the four Starship kinds (R3335000), which only the Fleet track
    /// and reaching Jlorious fill.
    fn picked_before_r3335000() -> impl Iterator<Item = CosmeticKind> {
        CosmeticKind::PICKED
            .into_iter()
            .filter(|kind| !CosmeticKind::STARSHIP.contains(kind))
    }

    #[test]
    fn r1295000_the_shipped_registry_passes_with_one_default_and_nine_items_per_kind() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for kind in picked_before_r3335000() {
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
            // back (R2755000) and the Fleet track's (R3335002) aside.
            assert_eq!(
                of_kind
                    .iter()
                    .filter(|cosmetic| !cosmetic.default && !cosmetic.earned_only())
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
                    .filter(|cosmetic| !cosmetic.default && !cosmetic.earned_only())
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
        // R3015005: a card's pick names a level, so `true` on a full-art card names Full art's first level (its still
        // Epic's look), and a full art's id the highest level it maps to, on its own card alone.
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let worn = |card: &str, pick: FullArtPick| picked(&cosmetics, card, pick, &[]);
        let id = |id: &str| FullArtPick::Id(id.to_owned());
        assert_eq!(worn("core-008", FullArtPick::On(true)), Some(VERSION_FULL_ART_ID));
        assert_eq!(worn("core-008", FullArtPick::On(false)), None);
        assert_eq!(
            worn("core-008", id("full-art-mr-vanilla")),
            Some(VERSION_FULL_ART_ID)
        );
        assert_eq!(
            worn("core-008", id("full-art-mr-vanilla-animated")),
            Some(VERSION_FULL_ART_ANIMATED_ID)
        );
        assert_eq!(
            worn("core-008", id("full-art-mr-vanilla-signed")),
            Some(VERSION_FULL_ART_SIGNED_ID)
        );
        assert_eq!(worn("core-004", id("full-art-mr-vanilla-signed")), None);
        assert_eq!(worn("core-008", id("foil-none")), None);
        assert_eq!(worn("core-002", FullArtPick::On(true)), None);
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
            // `true` still names the still Epic, and the id (R3015005) the Default variant's animated level it maps to.
            assert!(still.id.starts_with(FULL_ART_ID_PREFIX), "{card}");
            assert_eq!(
                picked(&cosmetics, card, FullArtPick::Id(cosmetic.id.clone()), &[]),
                Some(VERSION_ANIMATED_ID),
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
        // `true` names the still Epic even with the Animated one listed first; R3015005: an Animated art's id names
        // the Default variant's animated level it maps to, on its own card alone.
        assert_eq!(
            full_art_of(&tiers, "core-001").map(|cosmetic| cosmetic.id.as_str()),
            Some("full-art-a")
        );
        let animated = tiers
            .iter()
            .find(|cosmetic| cosmetic.id == "animated-a")
            .unwrap_or_else(|| panic!("animated-a is listed"));
        assert_eq!(implied_by(animated), [VERSION_ANIMATED_ID]);
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let animated = || FullArtPick::Id("animated-mr-vanilla".to_owned());
        assert_eq!(
            picked(&cosmetics, "core-008", animated(), &[]),
            Some(VERSION_ANIMATED_ID)
        );
        assert_eq!(picked(&cosmetics, "core-004", animated(), &[]), None);
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
        // #2943's two, #2944's two, #3015's fifteen (R3015000), #3017's twelve (R3017000) and #3335's 35 (R3335000)
        // come after them.
        assert!(
            cosmetics[..cosmetics.len()
                - R3335000_NEW
                - R3017000_NEW
                - R3015000_NEW
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
        // R3015000: a version is a level of a ladder, which its own check holds
        // (`r3015000_the_check_refuses_a_broken_ladder`); these stand alone, so the one problem that check finds
        // with each, its missing level, is set aside here.
        let found = |cosmetics: &[Cosmetic]| -> Vec<String> {
            problems(cosmetics)
                .into_iter()
                .filter(|problem| !problem.ends_with("\" has no level"))
                .collect()
        };
        let mut fine = defaults();
        fine.extend([
            version("version-a"),
            Cosmetic {
                kind: CosmeticKind::Augment,
                ..version("augment-a")
            },
        ]);
        assert!(found(&fine).is_empty(), "{:?}", found(&fine));
        let refused = |wrong: Cosmetic, problem: &str| {
            let mut cosmetics = defaults();
            cosmetics.push(wrong);
            assert_eq!(found(&cosmetics), vec![problem.to_owned()]);
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
        // R2940000, R2942000, R2943000, R2944000, R3015000, R3017000: #2725's three, #2940's three, #2942's three,
        // #2943's two, #2944's two, #3015's fifteen and #3017's twelve levels, each reaching the floor as its
        // variant's first does.
        assert_eq!(
            offered_kinds.len(),
            R3017000_NEW
                + R3015000_NEW
                + R2725000_NEW.len()
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
            .map(|(card, id)| crate::cosmetics::CardCosmetic {
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
        // R3015005: a pick names the level, worn only where it is offered.
        let named = |card: &str| {
            let level = level_of_pick(
                &cosmetics,
                card,
                Some(&FullArtPick::Id(VERSION_FULL_ART_ID.to_owned())),
                &[],
            );
            worn_level(&cosmetics, level, |level| offered(level, Some(card))).map(|level| level.id.as_str())
        };
        assert_eq!(named("core-022"), Some(VERSION_FULL_ART_ID));
        assert_eq!(named("core-002"), None);
        // `true` names Full art's first level on a full-art card alone (its retired still Epic's, R2655000).
        assert_eq!(picked(&cosmetics, "core-022", FullArtPick::On(true), &[]), None);
    }

    #[test]
    fn r2725002_r3015006_a_retired_row_owns_the_levels_it_maps_to() {
        use crate::cosmetics::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let levels: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.level.is_some())
            .collect();
        // Each retired full art on its card, as R3015006 maps it: the still Epic Full art's first level, the
        // Legendary that, its animated level and the Default one, the Mythic those and its signed level, the
        // Animated art the Default variant's animated level.
        let mut retired: Vec<(&str, String, Vec<&str>)> = Vec::new();
        for (card, slug) in R2655001_CARDS {
            retired.extend([
                (card, format!("full-art-{slug}"), vec![VERSION_FULL_ART_ID]),
                (
                    card,
                    format!("full-art-{slug}-animated"),
                    vec![
                        VERSION_FULL_ART_ID,
                        VERSION_FULL_ART_ANIMATED_ID,
                        VERSION_ANIMATED_ID,
                    ],
                ),
                (
                    card,
                    format!("full-art-{slug}-signed"),
                    vec![
                        VERSION_FULL_ART_ID,
                        VERSION_FULL_ART_ANIMATED_ID,
                        VERSION_FULL_ART_SIGNED_ID,
                        VERSION_ANIMATED_ID,
                    ],
                ),
                (card, format!("animated-{slug}"), vec![VERSION_ANIMATED_ID]),
            ]);
        }
        // The retired augments, on any card: Animated the Default variant's animated level, Signature alone nothing.
        retired.push((
            "core-022",
            AUGMENT_ANIMATED_ID.to_owned(),
            vec![VERSION_ANIMATED_ID],
        ));
        retired.push(("core-022", AUGMENT_SIGNATURE_ID.to_owned(), Vec::new()));
        for (card, held, implied) in &retired {
            assert_eq!(implied_by(by_id(held)), implied.as_slice(), "{held}");
            let owned = OwnedCosmetics {
                cards: vec![CardCosmetic {
                    card_id: (*card).to_owned(),
                    cosmetic_id: held.clone(),
                }],
                account: Vec::new(),
            };
            let elsewhere = if *card == "core-022" {
                "core-008"
            } else {
                "core-022"
            };
            for level in &levels {
                let id = level.id.as_str();
                // Owned on its own card exactly where it maps, and never on another card.
                assert_eq!(
                    owns(level, Some(card), &owned),
                    implied.contains(&id),
                    "{held} → {id}"
                );
                assert_eq!(row_owns(held, id), implied.contains(&id), "{held} → {id}");
                assert!(!owns(level, Some(elsewhere), &owned), "{held} → {id} elsewhere");
                assert_eq!(
                    implying(id).any(|retired| retired == held),
                    implied.contains(&id),
                    "{held} → {id}"
                );
            }
            // It still owns itself, and no other retired entry.
            assert!(owns(by_id(held), Some(card), &owned));
            assert!(row_owns(held, held));
        }
        // Nothing but a retired entry maps to anything, and a mapping is only ever to a level.
        assert!(
            cosmetics
                .iter()
                .filter(|cosmetic| !cosmetic.is_retired())
                .all(|cosmetic| implied_by(cosmetic).is_empty())
        );
        assert!(!row_owns("full-art-mr-vanilla", "foil-galaxy"));
        assert!(!row_owns("foil-galaxy", VERSION_FULL_ART_ID));
        assert!(implying("foil-galaxy").next().is_none());
    }

    #[test]
    fn r2725000_retired_full_arts_never_drop() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for list in [pack_cosmetics(&cosmetics), cosmetic_pack_cosmetics(&cosmetics)] {
            assert!(list.iter().all(|cosmetic| cosmetic.kind != CosmeticKind::FullArt));
            // The Full art version drops in their place; R3015000: the augments are retired beside them.
            for (id, kind, _) in R2725000_NEW {
                assert_eq!(
                    list.iter().any(|cosmetic| cosmetic.id == id),
                    kind == CosmeticKind::Version,
                    "{id}"
                );
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
    fn r2949000_r3015005_level_of_pick_maps_every_old_shape_to_the_level_it_drew() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let id = |id: &str| Some(FullArtPick::Id(id.to_owned()));
        let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
        let animated = [AUGMENT_ANIMATED_ID];
        let signature = [AUGMENT_SIGNATURE_ID];
        // On Mr. Vanilla, a full-art card; the mapping reads no offer and no ownership, which the callers hold.
        for (full_art, augments, level) in [
            // R2949000's Signature look, the Full art version with both augments, is Full art's signed level.
            (
                id(VERSION_FULL_ART_ID),
                &both[..],
                Some(VERSION_FULL_ART_SIGNED_ID),
            ),
            (
                Some(FullArtPick::On(true)),
                &animated[..],
                Some(VERSION_FULL_ART_ANIMATED_ID),
            ),
            (id(VERSION_FULL_ART_ID), &signature[..], Some(VERSION_FULL_ART_ID)),
            // A retired full art the highest level it maps to, a non-Default one first.
            (id("full-art-mr-vanilla"), &[][..], Some(VERSION_FULL_ART_ID)),
            (
                id("full-art-mr-vanilla"),
                &animated[..],
                Some(VERSION_FULL_ART_ANIMATED_ID),
            ),
            (
                id("full-art-mr-vanilla-animated"),
                &[][..],
                Some(VERSION_FULL_ART_ANIMATED_ID),
            ),
            (
                id("full-art-mr-vanilla-signed"),
                &[][..],
                Some(VERSION_FULL_ART_SIGNED_ID),
            ),
            (
                id("animated-mr-vanilla"),
                &animated[..],
                Some(VERSION_ANIMATED_ID),
            ),
            // Animated on the normal face, or on an id this build does not know, the Default variant's animated
            // level; Signature on no full art steps nowhere.
            (Some(FullArtPick::On(false)), &both[..], Some(VERSION_ANIMATED_ID)),
            (None, &both[..], Some(VERSION_ANIMATED_ID)),
            (None, &signature[..], None),
            (None, &[][..], None),
            (
                id("version-from-a-newer-client"),
                &both[..],
                Some(VERSION_ANIMATED_ID),
            ),
            (id("version-from-a-newer-client"), &[][..], None),
            // A normal-frame art with Animated stays at its first level where the next is its full art.
            (id(VERSION_ALT_ART_ID), &both[..], Some(VERSION_ALT_ART_ID)),
            (id(VERSION_SKETCH_ID), &animated[..], Some(VERSION_SKETCH_ID)),
            (
                id(VERSION_ALT_FULL_ART_ID),
                &both[..],
                Some("version-alt-full-art-signed"),
            ),
            (
                id(VERSION_ALT_FULL_ART_ID),
                &animated[..],
                Some("version-alt-full-art-animated"),
            ),
            (id(VERSION_PIXEL_ID), &both[..], Some("version-pixel-animated")),
            // A level's id is that level.
            (
                id("version-sketch-full-art-animated"),
                &[][..],
                Some("version-sketch-full-art-animated"),
            ),
        ] {
            let listed: Vec<String> = augments.iter().map(|id| (*id).to_owned()).collect();
            assert_eq!(
                level_of_pick(&cosmetics, "core-008", full_art.as_ref(), &listed)
                    .map(|level| level.id.as_str()),
                level,
                "{full_art:?} {augments:?}"
            );
        }
        // Another card's retired full art names nothing on this one.
        assert_eq!(
            picked(
                &cosmetics,
                "core-004",
                FullArtPick::Id("full-art-mr-vanilla".to_owned()),
                &[]
            ),
            None
        );
    }

    #[test]
    fn r2949001_a_sample_takes_a_card_by_its_seed_and_percent_and_the_check_holds_it_to_some_not_all() {
        let defs = shipped_defs();
        let sample = |seed: &str, percent: u32| OfferSample {
            seed: seed.to_owned(),
            from: 0,
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
        // R3015000: this version stands alone, so the ladders' check's one problem with it, its missing level, is set
        // aside (`r3015000_the_check_refuses_a_broken_ladder` holds that check).
        let checked = |sample: OfferSample| {
            let mut cosmetics = defaults();
            cosmetics.push(entry_with(sample));
            problems(&cosmetics)
                .into_iter()
                .filter(|problem| !problem.ends_with("\" has no level"))
                .collect::<Vec<_>>()
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
        use crate::cosmetics::CardCosmetic;
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
            // R2949001: every clause samples by its version's own id, so it lands on cards of its own. R3015000: Alt
            // art's levels share Alt art's clause and Alt full art's, each sampled by one of the two.
            let seeds: &[&str] = if cosmetic.id == VERSION_SKETCH_ID {
                &[VERSION_SKETCH_ID]
            } else {
                &[VERSION_ALT_ART_ID, VERSION_ALT_FULL_ART_ID]
            };
            assert!(
                cosmetic.offer.iter().all(|clause| clause
                    .sample
                    .as_ref()
                    .is_some_and(|sample| seeds.contains(&sample.seed.as_str()))),
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
        // Each version on a set of cards of its own: no two of the three alike (R3015000: Alt full art, Alt art's second level, offered as Alt art is), none on every card.
        let versions = [VERSION_FULL_ART_ID, VERSION_ALT_ART_ID, VERSION_SKETCH_ID];
        for (at, a) in versions.iter().enumerate() {
            assert!(on(a).len() < non_tokens.len(), "{a}");
            for b in &versions[at + 1..] {
                assert_ne!(on(a), on(b), "{a} and {b}");
            }
        }
        // Alt art the widest of them, and Alt full art on every card Alt art is (R3015000), each card it took before
        // among them.
        assert!(on(VERSION_ALT_ART_ID).len() > on(VERSION_SKETCH_ID).len());
        assert_eq!(on(VERSION_ALT_ART_ID), on(VERSION_ALT_FULL_ART_ID));
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

        // R3015005: a pick of one with the old Animated and Signature steps up where its variant's next levels add
        // them: Alt art and the Sketch stay at their first level, whose next is a full art, and Alt full art, now Alt
        // art's second level, becomes its signed level.
        for (id, level) in [
            (VERSION_ALT_ART_ID, VERSION_ALT_ART_ID),
            (VERSION_SKETCH_ID, VERSION_SKETCH_ID),
            (VERSION_ALT_FULL_ART_ID, "version-alt-full-art-signed"),
        ] {
            let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
            assert_eq!(
                picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &both),
                Some(level),
                "{id}"
            );
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
                worn_level(
                    &cosmetics,
                    level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]),
                    |level| offered(level, Some(card)),
                )
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
        use crate::cosmetics::CardCosmetic;
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
        // Each version on a set of cards of its own: no two of the six alike (R3015000: Alt full art, Alt art's second level, offered as Alt art is), none on every card, and each of
        // part 3's with a card no other version takes.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
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

        // R3015005: a pick of one with the old Animated and Signature is its animated level: Signature steps nowhere,
        // no level of a one-art variant being signed.
        for (id, _, _) in R2942000_NEW {
            let animated = format!("{id}-animated");
            let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
            assert_eq!(
                picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &both),
                Some(animated.as_str()),
                "{id}"
            );
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
                worn_level(
                    &cosmetics,
                    level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]),
                    |level| offered(level, Some(card)),
                )
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
        use crate::cosmetics::CardCosmetic;
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
        // Each version on a set of cards of its own: no two of the eight alike (R3015000: Alt full art, Alt art's second level, offered as Alt art is), none on every card.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
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

        // R3015005: a pick of one with the old Animated and Signature is its animated level: Signature steps nowhere,
        // no level of a one-art variant being signed.
        for (id, _, _) in R2943000_NEW {
            let animated = format!("{id}-animated");
            let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
            assert_eq!(
                picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &both),
                Some(animated.as_str()),
                "{id}"
            );
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
                worn_level(
                    &cosmetics,
                    level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]),
                    |level| offered(level, Some(card)),
                )
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
        use crate::cosmetics::CardCosmetic;
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
        // Each version on a set of cards of its own: no two of the ten alike (R3015000: Alt full art, Alt art's second level, offered as Alt art is), none on every card.
        let versions = [
            VERSION_FULL_ART_ID,
            VERSION_ALT_ART_ID,
            VERSION_SKETCH_ID,
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

        // R3015005: a pick of one with the old Animated and Signature is its animated level: Signature steps nowhere,
        // no level of a one-art variant being signed.
        for (id, _, _) in R2944000_NEW {
            let animated = format!("{id}-animated");
            let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
            assert_eq!(
                picked(&cosmetics, "core-022", FullArtPick::Id(id.to_owned()), &both),
                Some(animated.as_str()),
                "{id}"
            );
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
                worn_level(
                    &cosmetics,
                    level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]),
                    |level| offered(level, Some(card)),
                )
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
            {"id":"starship-hull-standard","kind":"starshipHull","name":"Hull","rarity":"Common","default":true},
            {"id":"starship-livery-standard","kind":"starshipLivery","name":"Livery","rarity":"Common","default":true},
            {"id":"starship-trail-standard","kind":"starshipTrail","name":"Trail","rarity":"Common","default":true},
            {"id":"starship-piece-skin-standard","kind":"starshipPieceSkin","name":"Skin","rarity":"Common","default":true},
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
                    "the cosmetic \"foil-wrong\" is {rarity} and its chase mark is {chase}, but a cosmetic is a chase exactly when it is Legendary or Mythic and not earned only"
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
        // (R2944000), then #3015's thirteen Legendary and Mythic levels (R3015000, R3015001), then #3017's three
        // Legendary animated levels (R3017000).
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let chases: Vec<&Cosmetic> = cosmetics.iter().filter(|cosmetic| cosmetic.chase).collect();
        assert_eq!(chases.len(), 17 + 42 + 7 + 13 + 3);
        assert_eq!(
            chases[17 + 42 + 7 + 13..]
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            [
                "version-neon-animated",
                "version-stained-glass-animated",
                "version-tarot-animated",
            ]
        );
        assert_eq!(
            chases[17 + 42 + 7..17 + 42 + 7 + 13]
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            R3015000_NEW_IDS
                .into_iter()
                .filter(|id| ![VERSION_ANIMATED_ID, VERSION_SKETCH_FULL_ART_ID].contains(id))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            chases[17 + 42..17 + 42 + 7]
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
        // #2942's three (R2942000), #2943's two (R2943000), #2944's two (R2944000) and #3015's fifteen levels
        // (R3015000), which retire the two augments, then #3017's twelve (R3017000).
        let mut dropped: Vec<Cosmetic> = cosmetics[earlier.len()..earlier.len() + R1833000_NEW.len()]
            .iter()
            .filter(|cosmetic| cosmetic.kind.is_per_card())
            .cloned()
            .collect();
        assert_eq!(dropped.len(), 18);
        dropped.extend(
            r2725000_new(&cosmetics)
                .iter()
                .filter(|cosmetic| !cosmetic.is_retired())
                .cloned(),
        );
        dropped.extend(r2940000_new(&cosmetics).iter().cloned());
        dropped.extend(r2942000_new(&cosmetics).iter().cloned());
        dropped.extend(r2943000_new(&cosmetics).iter().cloned());
        dropped.extend(r2944000_new(&cosmetics).iter().cloned());
        dropped.extend(r3015000_new(&cosmetics).iter().cloned());
        dropped.extend(r3017000_new(&cosmetics).iter().cloned());
        assert_eq!(
            dropped.len(),
            18 + R2725000_NEW.len() - 2
                + R2940000_NEW.len()
                + R2942000_NEW.len()
                + R2943000_NEW.len()
                + R2944000_NEW.len()
                + R3015000_NEW
                + R3017000_NEW
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
        // #2725's three (R2725000), #2940's three (R2940000), #2942's three (R2942000), #2943's two (R2943000),
        // #2944's two (R2944000), #3015's fifteen (R3015000), #3017's twelve (R3017000) and #3335's 35 last
        // (R3335000).
        assert_eq!(
            cosmetics.len(),
            72 + R3335000_NEW
                + R3017000_NEW
                + R3015000_NEW
                + R2655000_NEW
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
        for kind in picked_before_r3335000() {
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
        use crate::cosmetics::CardCosmetic;
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
        // (R2725000), #2940's three (R2940000), #2942's three (R2942000), #2943's two (R2943000), #2944's two
        // (R2944000), #3015's fifteen (R3015000), #3017's twelve (R3017000) and #3335's 35 (R3335000) after it.
        let back = &cosmetics[cosmetics.len()
            - R3335000_NEW
            - R3017000_NEW
            - R3015000_NEW
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
        // R3335003: the Jlorious cosmetics are it and the Jlorious Starship, and neither a duplicate's global roll
        // nor a cosmetic pack offers it.
        let jlorious: Vec<&str> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.jlorious)
            .map(|cosmetic| cosmetic.id.as_str())
            .collect();
        assert_eq!(jlorious, JLORIOUS_COSMETICS);
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
                    .any(|problem| problem.contains("only reaching Jlorious or a track's level may grant")),
                "{wrong:?}: {:?}",
                problems(&cosmetics)
            );
        }
    }

    #[test]
    fn r3335000_each_starship_kind_has_one_owned_default() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        // #3335's 35, last in the registry: the four defaults first, then the Jlorious Starship, then the Fleet
        // track's thirty in level order.
        let new = r3335000_new(&cosmetics);
        assert!(
            before_r3335000(&cosmetics)
                .iter()
                .all(|cosmetic| !CosmeticKind::STARSHIP.contains(&cosmetic.kind) && cosmetic.track.is_none())
        );
        for (kind, cosmetic) in CosmeticKind::STARSHIP.into_iter().zip(new) {
            assert_eq!(cosmetic.kind, kind);
            assert!(cosmetic.default && cosmetic.starter, "{}", cosmetic.id);
            assert_eq!(default_of(&cosmetics, kind), cosmetic.id);
            assert!(owns(cosmetic, None, &OwnedCosmetics::default()), "{}", cosmetic.id);
            assert!(!kind.is_per_card() && !kind.is_offered(), "{kind:?}");
            assert!(CosmeticKind::PICKED.contains(&kind) && CosmeticKind::DEFAULTED.contains(&kind));
        }
        assert_eq!(new[4].id, JLORIOUS_STARSHIP);
        let levels: Vec<Option<u32>> = new[5..].iter().map(|cosmetic| cosmetic.track_level).collect();
        assert_eq!(levels, (1..=FLEET_TRACK_LEVELS).map(Some).collect::<Vec<_>>());
        // Only the defaults are Common; nothing else of the four kinds is owned without a grant.
        for cosmetic in cosmetics
            .iter()
            .filter(|cosmetic| CosmeticKind::STARSHIP.contains(&cosmetic.kind) && !cosmetic.default)
        {
            assert!(cosmetic.earned_only(), "{}", cosmetic.id);
            assert!(!owns(cosmetic, None, &OwnedCosmetics::default()), "{}", cosmetic.id);
        }
        // A kind's name travels as the camel-case word the settings group keys on.
        assert_eq!(
            serde_json::to_string(&CosmeticKind::StarshipPieceSkin).unwrap(),
            "\"starshipPieceSkin\""
        );
    }

    #[test]
    fn r3335002_no_pool_offers_a_fleet_reward_or_the_jlorious_starship() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let earned: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.earned_only())
            .collect();
        // The two Jlorious cosmetics and the Fleet track's thirty.
        assert_eq!(earned.len(), JLORIOUS_COSMETICS.len() + FLEET_TRACK_LEVELS as usize);
        assert!(earned.iter().any(|cosmetic| cosmetic.id == JLORIOUS_STARSHIP));
        for pool in [
            global_cosmetics(&cosmetics),
            cosmetic_pack_cosmetics(&cosmetics),
            pack_cosmetics(&cosmetics),
        ] {
            assert!(
                pool.iter().all(|cosmetic| !cosmetic.earned_only()),
                "{:?}",
                pool.iter()
                    .filter(|cosmetic| cosmetic.earned_only())
                    .map(|cosmetic| &cosmetic.id)
                    .collect::<Vec<_>>()
            );
        }
        // Each is a chase's rarity or below it, and never marked one, so the Chase Shard's list skips it.
        assert!(earned.iter().all(|cosmetic| !cosmetic.chase));
    }

    #[test]
    fn r3335002_an_earned_only_cosmetic_is_never_a_chase_default_starter_or_per_card() {
        let fleet = Cosmetic {
            rarity: CosmeticRarity::Legendary,
            starter: false,
            track: Some(CosmeticTrack::Fleet),
            track_level: Some(1),
            ..entry("starship-livery-earned", CosmeticKind::StarshipLivery, false)
        };
        let mut cosmetics = defaults();
        cosmetics.push(fleet.clone());
        // One Fleet reward alone is no whole track, which the check refuses; a registry with none passes.
        assert!(
            problems(&cosmetics)
                .iter()
                .any(|problem| problem.contains("Fleet track's rewards are at the levels [1]")),
            "{:?}",
            problems(&cosmetics)
        );
        assert!(problems(&defaults()).is_empty(), "{:?}", problems(&defaults()));
        let shape = |cosmetics: &[Cosmetic]| -> Vec<String> {
            problems(cosmetics)
                .into_iter()
                .filter(|problem| !problem.contains("Fleet track's") && !problem.contains("Fleet capstone"))
                .collect()
        };
        assert!(shape(&cosmetics).is_empty(), "{:?}", shape(&cosmetics));
        for (wrong, says) in [
            (
                Cosmetic {
                    chase: true,
                    ..fleet.clone()
                },
                "not earned only",
            ),
            (
                Cosmetic {
                    starter: true,
                    ..fleet.clone()
                },
                "only reaching Jlorious or a track's level may grant",
            ),
            (
                Cosmetic {
                    default: true,
                    ..fleet.clone()
                },
                "only reaching Jlorious or a track's level may grant",
            ),
            (
                Cosmetic {
                    kind: CosmeticKind::Foil,
                    id: "foil-earned".to_owned(),
                    ..fleet.clone()
                },
                "only reaching Jlorious or a track's level may grant",
            ),
            (
                Cosmetic {
                    jlorious: true,
                    ..fleet.clone()
                },
                "both Jlorious and a track's",
            ),
            (
                Cosmetic {
                    track_level: None,
                    ..fleet.clone()
                },
                "a track without a level",
            ),
            (
                Cosmetic {
                    track: None,
                    ..fleet.clone()
                },
                "a level without a track",
            ),
        ] {
            let mut cosmetics = defaults();
            cosmetics.push(wrong.clone());
            assert!(
                shape(&cosmetics).iter().any(|problem| problem.contains(says)),
                "{wrong:?}: {:?}",
                problems(&cosmetics)
            );
        }
    }

    #[test]
    fn r3335002_the_fleet_track_has_one_reward_per_level_the_stinkyscape_hull_last() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for level in 1..=FLEET_TRACK_LEVELS {
            let reward = fleet_reward(level).unwrap_or_else(|| panic!("level {level} pays nothing"));
            assert_eq!(reward.track, Some(CosmeticTrack::Fleet));
            assert!(!reward.kind.is_per_card(), "{}", reward.id);
            assert!(reward.rarity >= GLOBAL_COSMETIC_FLOOR_RARITY, "{}", reward.id);
        }
        assert!(fleet_reward(0).is_none() && fleet_reward(FLEET_TRACK_LEVELS + 1).is_none());
        let capstone = fleet_reward(FLEET_TRACK_LEVELS).map(|cosmetic| cosmetic.id.as_str());
        assert_eq!(capstone, Some(FLEET_CAPSTONE));
        // Every Starship kind is paid, and a Stinkyscape card back, playmat and backdrop.
        for kind in CosmeticKind::STARSHIP.into_iter().chain(crate::loadout::DECK_SLOT_KINDS) {
            assert!(
                (1..=FLEET_TRACK_LEVELS).any(|level| fleet_reward(level).is_some_and(|c| c.kind == kind)),
                "{kind:?}"
            );
        }
        // Moving the capstone, or dropping a level, is refused.
        let mut moved = cosmetics.clone();
        for cosmetic in &mut moved {
            if cosmetic.track_level == Some(FLEET_TRACK_LEVELS) {
                cosmetic.track_level = Some(FLEET_TRACK_LEVELS - 1);
            } else if cosmetic.track_level == Some(FLEET_TRACK_LEVELS - 1) {
                cosmetic.track_level = Some(FLEET_TRACK_LEVELS);
            }
        }
        assert!(
            problems(&moved).iter().any(|problem| problem.contains("its capstone")),
            "{:?}",
            problems(&moved)
        );
        let dropped: Vec<Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| cosmetic.track_level != Some(1))
            .cloned()
            .collect();
        assert!(
            problems(&dropped)
                .iter()
                .any(|problem| problem.contains("Fleet track's rewards are at the levels")),
            "{:?}",
            problems(&dropped)
        );
    }

    use CosmeticRarity::{Epic, Legendary, Mythic, Rare};
    use LevelAdds::{Animated, Art, FullArt, Signed};

    /// One level as R3015000's table lists it: its id, what it adds and its rarity.
    type TableLevel = (&'static str, LevelAdds, CosmeticRarity);

    /// R3015000, R3015001: every variant's ladder as the registry ships it, lowest level first: each level's id,
    /// what it adds and its rarity. The Default variant's first level, the normal face, names no entry.
    const R3015000_LADDERS: [(&str, &[TableLevel]); 11] = [
        (DEFAULT_VARIANT, &[(VERSION_ANIMATED_ID, Animated, Epic)]),
        (
            "full-art",
            &[
                (VERSION_FULL_ART_ID, FullArt, Legendary),
                (VERSION_FULL_ART_ANIMATED_ID, Animated, Mythic),
                (VERSION_FULL_ART_SIGNED_ID, Signed, Mythic),
            ],
        ),
        (
            "alt-art",
            &[
                (VERSION_ALT_ART_ID, Art, Rare),
                (VERSION_ALT_FULL_ART_ID, FullArt, Legendary),
                ("version-alt-full-art-animated", Animated, Mythic),
                ("version-alt-full-art-signed", Signed, Mythic),
            ],
        ),
        (
            "sketch",
            &[
                (VERSION_SKETCH_ID, Art, Rare),
                (VERSION_SKETCH_FULL_ART_ID, FullArt, Epic),
                ("version-sketch-full-art-animated", Animated, Legendary),
                ("version-sketch-full-art-signed", Signed, Mythic),
            ],
        ),
        (
            "extended-art",
            &[
                (VERSION_EXTENDED_ART_ID, Art, Epic),
                ("version-extended-art-animated", Animated, Legendary),
            ],
        ),
        (
            "pixel",
            &[
                (VERSION_PIXEL_ID, Art, Epic),
                ("version-pixel-animated", Animated, Legendary),
            ],
        ),
        (
            "comic",
            &[
                (VERSION_COMIC_ID, Art, Epic),
                ("version-comic-animated", Animated, Legendary),
            ],
        ),
        (
            "set-showcase",
            &[
                (VERSION_SET_SHOWCASE_ID, Art, Legendary),
                ("version-set-showcase-animated", Animated, Mythic),
            ],
        ),
        (
            "out-of-frame",
            &[
                (VERSION_OUT_OF_FRAME_ID, Art, Legendary),
                ("version-out-of-frame-animated", Animated, Mythic),
            ],
        ),
        (
            "immersive",
            &[
                (VERSION_IMMERSIVE_ID, Art, Mythic),
                ("version-immersive-animated", Animated, Mythic),
            ],
        ),
        (
            "prismatic",
            &[
                (VERSION_PRISMATIC_ID, Art, Mythic),
                ("version-prismatic-animated", Animated, Mythic),
            ],
        ),
    ];

    /// R3015000: the fifteen levels #3015 appends, in the registry's order.
    const R3015000_NEW_IDS: [&str; R3015000_NEW] = [
        VERSION_ANIMATED_ID,
        VERSION_FULL_ART_ANIMATED_ID,
        VERSION_FULL_ART_SIGNED_ID,
        "version-alt-full-art-animated",
        "version-alt-full-art-signed",
        VERSION_SKETCH_FULL_ART_ID,
        "version-sketch-full-art-animated",
        "version-sketch-full-art-signed",
        "version-extended-art-animated",
        "version-pixel-animated",
        "version-comic-animated",
        "version-set-showcase-animated",
        "version-out-of-frame-animated",
        "version-immersive-animated",
        "version-prismatic-animated",
    ];

    /// R3017000: the six two-level ladders #3017 appends, in the registry's order: each an art level, then its
    /// animated one a tier rarer.
    const R3017000_LADDERS: [(&str, &[TableLevel]); 6] = [
        (
            "night",
            &[
                (VERSION_NIGHT_ID, Art, Rare),
                ("version-night-animated", Animated, Epic),
            ],
        ),
        (
            "neon",
            &[
                (VERSION_NEON_ID, Art, Epic),
                ("version-neon-animated", Animated, Legendary),
            ],
        ),
        (
            "watercolor",
            &[
                (VERSION_WATERCOLOR_ID, Art, Rare),
                ("version-watercolor-animated", Animated, Epic),
            ],
        ),
        (
            "stained-glass",
            &[
                (VERSION_STAINED_GLASS_ID, Art, Epic),
                ("version-stained-glass-animated", Animated, Legendary),
            ],
        ),
        (
            "woodcut",
            &[
                (VERSION_WOODCUT_ID, Art, Rare),
                ("version-woodcut-animated", Animated, Epic),
            ],
        ),
        (
            "tarot",
            &[
                (VERSION_TAROT_ID, Art, Epic),
                ("version-tarot-animated", Animated, Legendary),
            ],
        ),
    ];

    /// R3017001: #3017's three pairs, each splitting every card between its two variants' first levels.
    const R3017001_PAIRS: [(&str, &str); 3] = [
        (VERSION_NIGHT_ID, VERSION_NEON_ID),
        (VERSION_WATERCOLOR_ID, VERSION_STAINED_GLASS_ID),
        (VERSION_WOODCUT_ID, VERSION_TAROT_ID),
    ];

    #[test]
    fn r3015000_every_variant_is_a_ladder_of_levels() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        assert!(problems(&cosmetics).is_empty(), "{:?}", problems(&cosmetics));
        // #3015's fifteen are appended last, none a default or a starter.
        assert_eq!(
            r3015000_new(&cosmetics)
                .iter()
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            R3015000_NEW_IDS
        );
        assert!(
            r3015000_new(&cosmetics)
                .iter()
                .all(|cosmetic| !cosmetic.default && !cosmetic.starter)
        );
        // Every version is a level of one of the eleven ladders, or #3017's six (R3017000), ranked gaplessly from its
        // first, which is 2 for the Default variant, and nothing else has a level.
        let mut listed = 0;
        for (variant, levels) in R3015000_LADDERS.into_iter().chain(R3017000_LADDERS) {
            let first: u8 = if variant == DEFAULT_VARIANT { 2 } else { 1 };
            assert_eq!(
                ladder(&cosmetics, variant)
                    .iter()
                    .map(|cosmetic| cosmetic.level.as_ref().map(|level| (
                        cosmetic.id.as_str(),
                        level.rank,
                        level.adds
                    )))
                    .collect::<Vec<_>>(),
                levels
                    .iter()
                    .zip(first..)
                    .map(|((id, adds, _), rank)| Some((*id, rank, *adds)))
                    .collect::<Vec<_>>(),
                "{variant}"
            );
            // The art first, then animated, then signed above an animated full art alone.
            let adds: Vec<LevelAdds> = levels.iter().map(|(_, adds, _)| *adds).collect();
            let animated = adds.iter().position(|adds| *adds == Animated);
            assert!(
                animated.is_some_and(|at| adds[..at].iter().all(|adds| matches!(adds, Art | FullArt))),
                "{variant}"
            );
            if let Some(signed) = adds.iter().position(|adds| *adds == Signed) {
                assert_eq!(Some(signed - 1), animated, "{variant}");
                assert_eq!(adds.get(signed.wrapping_sub(2)), Some(&FullArt), "{variant}");
            }
            listed += levels.len();
        }
        assert_eq!(
            cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.kind == CosmeticKind::Version)
                .count(),
            listed
        );
        assert!(
            cosmetics
                .iter()
                .all(|cosmetic| cosmetic.level.is_some() == (cosmetic.kind == CosmeticKind::Version))
        );
        // Every card has at least the Default ladder, and each level is offered where its variant's first is, but
        // a level that moves never on a Legacy card.
        let defs = shipped_defs();
        let default_first = cosmetics
            .iter()
            .find(|cosmetic| cosmetic.id == VERSION_ANIMATED_ID)
            .unwrap_or_else(|| panic!("the Default variant's animated level ships"));
        for (card, def) in defs.iter().filter(|(_, def)| !def.is_token()) {
            assert!(
                default_first.offer.iter().any(|clause| clause.takes(card, def)),
                "{card}"
            );
            let legacy = def.tags.contains(&Tag::Legacy);
            for (variant, _) in R3015000_LADDERS.into_iter().chain(R3017000_LADDERS) {
                let levels = ladder(&cosmetics, variant);
                let first = levels
                    .first()
                    .is_some_and(|first| first.offer.iter().any(|clause| clause.takes(card, def)));
                for level in levels {
                    let moves = level.level.as_ref().is_some_and(|at| at.adds.moves());
                    assert_eq!(
                        offered(level, Some(card)),
                        first && !(legacy && moves),
                        "{} on {card}",
                        level.id
                    );
                }
            }
        }
        // The retired entries keep their ids and drop, sell and wear nowhere (R3015006 reads them).
        assert!(
            cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.is_retired())
                .all(|cosmetic| matches!(cosmetic.kind, CosmeticKind::FullArt | CosmeticKind::Augment))
        );
        assert_eq!(
            cosmetics
                .iter()
                .filter(|cosmetic| cosmetic.kind == CosmeticKind::Augment)
                .map(|cosmetic| cosmetic.id.as_str())
                .collect::<Vec<_>>(),
            [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]
        );
    }

    #[test]
    fn r3015000_no_animated_or_signed_level_on_a_legacy_card() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let legacy: Vec<&str> = shipped_defs()
            .iter()
            .filter(|(_, def)| !def.is_token() && def.tags.contains(&Tag::Legacy))
            .map(|(card, _)| card.as_str())
            .collect();
        assert!(legacy.contains(&LEGACY_DUPLICATING_FELINORS), "{legacy:?}");
        let mut stopped = 0;
        for card in &legacy {
            for level in cosmetics.iter().filter(|cosmetic| cosmetic.level.is_some()) {
                let adds = level.level.as_ref().map(|at| at.adds);
                if adds.is_some_and(LevelAdds::moves) {
                    assert!(!offered(level, Some(card)), "{} on {card}", level.id);
                } else if offered(level, Some(card)) {
                    stopped += 1;
                }
            }
            // The Default ladder is the normal face alone there.
            assert!(
                ladder(&cosmetics, DEFAULT_VARIANT)
                    .iter()
                    .all(|level| !offered(level, Some(card)))
            );
        }
        // Some Legacy card is offered an art level, its ladder stopping before animated.
        assert!(stopped > 0);
        // A level that moves is refused a Legacy card even where its clauses take every card.
        let mut tiers = defaults();
        tiers.push(Cosmetic {
            starter: false,
            offer: vec![OfferClause::default()],
            level: Some(VariantLevel {
                variant: DEFAULT_VARIANT.to_owned(),
                rank: 2,
                adds: Animated,
            }),
            ..entry(VERSION_ANIMATED_ID, CosmeticKind::Version, false)
        });
        assert!(offered(&tiers[tiers.len() - 1], Some("core-022")));
        assert!(!offered(
            &tiers[tiers.len() - 1],
            Some(LEGACY_DUPLICATING_FELINORS)
        ));
    }

    #[test]
    fn r3015000_the_check_refuses_a_broken_ladder() {
        let level = |id: &str, variant: &str, rank: u8, adds: LevelAdds, rarity: CosmeticRarity| Cosmetic {
            rarity,
            chase: rarity.is_chase(),
            starter: false,
            offer: vec![OfferClause::default()],
            level: Some(VariantLevel {
                variant: variant.to_owned(),
                rank,
                adds,
            }),
            ..entry(id, CosmeticKind::Version, false)
        };
        let ladders = || {
            let mut cosmetics = defaults();
            cosmetics.extend([
                level("version-animated", DEFAULT_VARIANT, 2, Animated, Epic),
                level("version-a", "a", 1, Art, Rare),
                level("version-a-full-art", "a", 2, FullArt, Epic),
                level("version-a-animated", "a", 3, Animated, Legendary),
                level("version-a-signed", "a", 4, Signed, Mythic),
                level("version-b", "b", 1, Art, Epic),
                level("version-b-animated", "b", 2, Animated, Legendary),
            ]);
            cosmetics
        };
        assert!(problems(&ladders()).is_empty(), "{:?}", problems(&ladders()));
        let refused = |change: &dyn Fn(&mut Vec<Cosmetic>), problem: &str| {
            let mut cosmetics = ladders();
            change(&mut cosmetics);
            let found = problems(&cosmetics);
            assert!(
                found.iter().any(|found| found.starts_with(problem)),
                "{problem:?} not in {found:?}"
            );
        };
        let at = |cosmetics: &mut Vec<Cosmetic>, id: &str| -> usize {
            cosmetics
                .iter()
                .position(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} is listed"))
        };
        let set_level = |cosmetics: &mut Vec<Cosmetic>, id: &str, rank: u8, adds: LevelAdds| {
            let i = at(cosmetics, id);
            if let Some(level) = cosmetics[i].level.as_mut() {
                level.rank = rank;
                level.adds = adds;
            }
        };
        // A version without a level, and a level on another kind.
        refused(
            &|c| {
                c.push(Cosmetic {
                    level: None,
                    ..level("version-c", "c", 1, Art, Rare)
                })
            },
            "the version \"version-c\" has no level",
        );
        refused(
            &|c| {
                c.push(Cosmetic {
                    kind: CosmeticKind::Foil,
                    offer: Vec::new(),
                    ..level("foil-c", "c", 1, Art, Rare)
                })
            },
            "the Foil \"foil-c\" has a level, which only a version has",
        );
        // A variant that is no slug.
        refused(
            &|c| {
                c.push(level("version-c", "C", 1, Art, Rare));
                c.push(level("version-c-animated", "C", 2, Animated, Epic));
            },
            "the level \"version-c\" names the variant \"C\", which is not a lower-case slug",
        );
        // Ranks with a gap, or the Default variant's from 1.
        refused(
            &|c| set_level(c, "version-a-signed", 5, Signed),
            "the variant \"a\" has the ranks [1, 2, 3, 5]",
        );
        refused(
            &|c| set_level(c, "version-animated", 1, Animated),
            "the variant \"default\" has the ranks [1]",
        );
        // Signed with no animated level below it, signed above a normal-frame art, no animated level, two art
        // levels, an art level in the Default variant, and a level after signed.
        refused(
            &|c| {
                let i = at(c, "version-a-animated");
                c.remove(i);
                set_level(c, "version-a-signed", 3, Signed);
            },
            "the variant \"a\" adds [Art, FullArt, Signed]",
        );
        refused(
            &|c| c.push(level("version-b-signed", "b", 3, Signed, Mythic)),
            "the variant \"b\" adds [Art, Animated, Signed]",
        );
        refused(
            &|c| {
                let i = at(c, "version-b-animated");
                c.remove(i);
            },
            "the variant \"b\" adds [Art]",
        );
        refused(
            &|c| set_level(c, "version-a-full-art", 2, Art),
            "the variant \"a\" adds [Art, Art, Animated, Signed]",
        );
        refused(
            &|c| {
                set_level(c, "version-animated", 3, Animated);
                c.push(level("version-default-art", DEFAULT_VARIANT, 2, Art, Rare));
            },
            "the variant \"default\" adds [Art, Animated]",
        );
        refused(
            &|c| c.push(level("version-a-more", "a", 5, Animated, Mythic)),
            "the variant \"a\" adds [Art, FullArt, Animated, Signed, Animated]",
        );
        // R3015001: a level no rarer than the one below it but at Mythic, and a signed level below Mythic.
        refused(
            &|c| {
                let i = at(c, "version-a-full-art");
                c[i] = level("version-a-full-art", "a", 2, FullArt, Rare);
            },
            "the level \"version-a-full-art\" is Rare, not rarer than \"version-a\" below it, Rare",
        );
        refused(
            &|c| {
                for (id, rarity) in [
                    ("version-a", CosmeticRarity::Common),
                    ("version-a-full-art", Rare),
                    ("version-a-animated", Epic),
                    ("version-a-signed", Legendary),
                ] {
                    let i = at(c, id);
                    let (rank, adds) = c[i]
                        .level
                        .as_ref()
                        .map_or((0, Art), |level| (level.rank, level.adds));
                    c[i] = level(id, "a", rank, adds, rarity);
                }
            },
            "the signed level \"version-a-signed\" is Legendary, not Mythic",
        );
        // A level offered otherwise than its variant's first.
        refused(
            &|c| {
                let i = at(c, "version-a-animated");
                c[i].offer = vec![OfferClause {
                    rarities: vec![Rarity::Legendary],
                    ..OfferClause::default()
                }];
            },
            "the level \"version-a-animated\" is offered otherwise than \"version-a\", its variant's first",
        );
        // A Default ladder that leaves a card out.
        refused(
            &|c| {
                let i = at(c, "version-animated");
                c[i].offer = vec![OfferClause {
                    rarities: vec![Rarity::Mythic],
                    ..OfferClause::default()
                }];
            },
            "the Default variant's offer does not take",
        );
    }

    #[test]
    fn r3015001_each_level_has_the_table_s_rarity() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        for (variant, levels) in R3015000_LADDERS {
            let shipped: Vec<(&str, CosmeticRarity, bool)> = ladder(&cosmetics, variant)
                .iter()
                .map(|cosmetic| (cosmetic.id.as_str(), cosmetic.rarity, cosmetic.chase))
                .collect();
            // Each level its table's rarity, a chase exactly at Legendary and up (R1333000).
            assert_eq!(
                shipped,
                levels
                    .iter()
                    .map(|(id, _, rarity)| (*id, *rarity, rarity.is_chase()))
                    .collect::<Vec<_>>(),
                "{variant}"
            );
            // Rising up the ladder until Mythic, and a signed level Mythic.
            for pair in levels.windows(2) {
                assert!(pair[1].2 > pair[0].2 || pair[1].2 == Mythic, "{pair:?}");
            }
            for (id, adds, rarity) in levels.iter() {
                assert!(*adds != Signed || *rarity == Mythic, "{id}");
            }
            // A variant's first level keeps the rarity its version had before #3015.
            if let Some((first, _, rarity)) = levels.first()
                && variant != DEFAULT_VARIANT
            {
                assert_eq!(
                    before_r3015000(&cosmetics)
                        .iter()
                        .find(|cosmetic| cosmetic.id == *first)
                        .map(|cosmetic| cosmetic.rarity),
                    Some(*rarity),
                    "{first}"
                );
            }
        }
        // The Default variant's animated level is the retired Animated augment's Epic.
        assert_eq!(
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == AUGMENT_ANIMATED_ID)
                .map(|cosmetic| cosmetic.rarity),
            Some(Epic)
        );
    }

    #[test]
    fn r3017000_part_six_appends_six_two_level_variants() {
        use crate::cosmetics::CardCosmetic;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        assert!(problems(&cosmetics).is_empty(), "{:?}", problems(&cosmetics));
        let new = r3017000_new(&cosmetics);
        // Appended last, in the table's order: each variant's art level, then its animated one.
        assert_eq!(
            new.iter()
                .map(|cosmetic| (
                    cosmetic.id.as_str(),
                    cosmetic.level.as_ref().map(|level| level.adds),
                    cosmetic.rarity
                ))
                .collect::<Vec<_>>(),
            R3017000_LADDERS
                .iter()
                .flat_map(|(_, levels)| levels
                    .iter()
                    .map(|(id, adds, rarity)| (*id, Some(*adds), *rarity)))
                .collect::<Vec<_>>()
        );
        for cosmetic in new {
            // Earned, per card, offered, naming no card; a chase exactly at Legendary (R1333000).
            assert!(
                !cosmetic.default && !cosmetic.starter && !cosmetic.jlorious,
                "{}",
                cosmetic.id
            );
            assert_eq!(cosmetic.kind, CosmeticKind::Version, "{}", cosmetic.id);
            assert!(
                cosmetic.card.is_none() && !cosmetic.offer.is_empty(),
                "{}",
                cosmetic.id
            );
            assert_eq!(cosmetic.chase, cosmetic.rarity.is_chase(), "{}", cosmetic.id);
            // No retired entry owns one: owning an older look never owns a part 6 one (R3015006).
            assert!(implying(&cosmetic.id).next().is_none(), "{}", cosmetic.id);
        }
        // Each ladder is an art level and its animated one, offered alike, named for its variant.
        for (variant, levels) in R3017000_LADDERS {
            let shipped = ladder(&cosmetics, variant);
            assert_eq!(shipped.len(), 2, "{variant}");
            assert_eq!(shipped[0].id, format!("{VERSION_ID_PREFIX}{variant}"));
            assert_eq!(shipped[1].id, format!("{VERSION_ID_PREFIX}{variant}-animated"));
            assert_eq!(shipped[1].name, format!("{}, animated", shipped[0].name));
            assert_eq!(shipped[0].offer, shipped[1].offer, "{variant}");
            assert_eq!(
                shipped
                    .iter()
                    .map(|level| (level.id.as_str(), level.rarity))
                    .collect::<Vec<_>>(),
                levels
                    .iter()
                    .map(|(id, _, rarity)| (*id, *rarity))
                    .collect::<Vec<_>>()
            );
        }

        // check_pick refuses each level where it is not offered, owned or not, and takes it where it is offered and
        // owned: night, stained-glass and tarot on one card, neon, watercolor and woodcut on the other.
        const ONE: &str = "meditativeplus-026";
        const OTHER: &str = "core-002";
        let owned = OwnedCosmetics {
            cards: new
                .iter()
                .flat_map(|cosmetic| [ONE, OTHER].map(|card| (card, cosmetic.id.as_str())))
                .map(|(card, id)| CardCosmetic {
                    card_id: card.to_owned(),
                    cosmetic_id: id.to_owned(),
                })
                .collect(),
            account: Vec::new(),
        };
        for cosmetic in new {
            let id = cosmetic.id.as_str();
            let variant = cosmetic.level.as_ref().map_or("", |level| level.variant.as_str());
            let (on, off) = if ["night", "stained-glass", "tarot"].contains(&variant) {
                (ONE, OTHER)
            } else {
                (OTHER, ONE)
            };
            assert_eq!(
                check_pick(&cosmetics, &owned, CosmeticKind::Version, id, Some(off)),
                Err(PickRefusal::NotOffered),
                "{id} on {off}"
            );
            assert_eq!(
                check_pick(&cosmetics, &owned, CosmeticKind::Version, id, Some(on)),
                Ok(()),
                "{id} on {on}"
            );
            // R3015005: a card's pick names the level, and it is worn only where it is offered.
            let worn = |card: &str| {
                worn_level(
                    &cosmetics,
                    level_of_pick(&cosmetics, card, Some(&FullArtPick::Id(id.to_owned())), &[]),
                    |level| offered(level, Some(card)),
                )
                .map(|level| level.id.as_str())
            };
            assert_eq!(worn(on), Some(id));
            assert_eq!(worn(off), None);
        }
    }

    #[test]
    fn r3017001_a_sample_from_a_start_takes_the_cards_from_it_below_its_percent() {
        let defs = shipped_defs();
        let sample = |from: u32, percent: u32| OfferSample {
            seed: "version-night-neon".to_owned(),
            from,
            percent,
        };
        let taken = |sample: &OfferSample| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| sample.takes(card))
                .collect()
        };
        // A card is taken when its hash falls at or above `from` and below `percent`.
        let upper = sample(60, OFFER_SAMPLE_PERCENT_SCALE);
        let middle = sample(20, 60);
        for card in defs.keys() {
            let at = fnv1a32_utf16_u32(&format!("version-night-neon/{card}")) % OFFER_SAMPLE_PERCENT_SCALE;
            assert_eq!(upper.takes(card), at >= 60, "{card}");
            assert_eq!(middle.takes(card), (20..60).contains(&at), "{card}");
        }
        // From 60 to the scale takes exactly what below 60 leaves, and from 0 is the sample without one.
        let lower = sample(0, 60);
        let all: BTreeSet<&str> = defs.keys().map(String::as_str).collect();
        assert!(taken(&lower).is_disjoint(&taken(&upper)));
        assert_eq!(
            taken(&lower)
                .union(&taken(&upper))
                .copied()
                .collect::<BTreeSet<_>>(),
            all
        );
        assert!(!taken(&upper).is_empty() && !taken(&lower).is_empty());
        // A `from` of 0 is left out of the text, and read as 0 where it is absent.
        assert_eq!(
            serde_json::to_string(&lower).ok().as_deref(),
            Some(r#"{"seed":"version-night-neon","percent":60}"#)
        );
        assert_eq!(
            serde_json::to_string(&upper).ok().as_deref(),
            Some(r#"{"seed":"version-night-neon","from":60,"percent":100}"#)
        );
        assert_eq!(
            serde_json::from_str::<OfferSample>(r#"{"seed":"version-night-neon","percent":60}"#).ok(),
            Some(lower.clone())
        );

        // The check refuses a share of none or all of the scale, and a percent past it.
        let checked = |sample: OfferSample| {
            let mut cosmetics = defaults();
            cosmetics.push(Cosmetic {
                starter: false,
                offer: vec![OfferClause {
                    sample: Some(sample),
                    ..OfferClause::default()
                }],
                ..entry("version-a", CosmeticKind::Version, false)
            });
            problems(&cosmetics)
                .into_iter()
                .filter(|problem| !problem.ends_with("\" has no level"))
                .collect::<Vec<_>>()
        };
        assert!(checked(upper.clone()).is_empty());
        assert!(checked(middle.clone()).is_empty());
        for (from, percent) in [(60, 60), (70, 60)] {
            assert_eq!(
                checked(sample(from, percent)),
                vec![format!(
                    "the offer of \"version-a\" samples 0 in {OFFER_SAMPLE_PERCENT_SCALE} of its cards, which must be some and not all"
                )],
                "{from} to {percent}"
            );
        }
        assert_eq!(
            checked(sample(10, OFFER_SAMPLE_PERCENT_SCALE + 10)),
            vec![
                format!(
                    "the offer of \"version-a\" samples {OFFER_SAMPLE_PERCENT_SCALE} in {OFFER_SAMPLE_PERCENT_SCALE} of its cards, which must be some and not all"
                ),
                format!(
                    "the offer of \"version-a\" samples below {}, past {OFFER_SAMPLE_PERCENT_SCALE}",
                    OFFER_SAMPLE_PERCENT_SCALE + 10
                ),
            ]
        );
        assert_eq!(
            checked(sample(50, OFFER_SAMPLE_PERCENT_SCALE + 10)),
            vec![format!(
                "the offer of \"version-a\" samples below {}, past {OFFER_SAMPLE_PERCENT_SCALE}",
                OFFER_SAMPLE_PERCENT_SCALE + 10
            )]
        );
    }

    #[test]
    fn r3017001_each_pair_splits_every_card_between_its_two() {
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let by_id = |id: &str| {
            cosmetics
                .iter()
                .find(|cosmetic| cosmetic.id == id)
                .unwrap_or_else(|| panic!("{id} ships"))
        };
        let defs = shipped_defs();
        let mut legacy = 0;
        for (card, def) in defs.iter().filter(|(_, def)| !def.is_token()) {
            legacy += usize::from(def.tags.contains(&Tag::Legacy));
            for (a, b) in R3017001_PAIRS {
                assert!(
                    offered(by_id(a), Some(card)) != offered(by_id(b), Some(card)),
                    "{card}: {a} and {b}"
                );
            }
        }
        // Legacy cards are among them, offered the pairs' art levels and none of their animated ones.
        assert!(legacy > 0);
        for (variant, _) in R3017000_LADDERS {
            let animated = format!("{VERSION_ID_PREFIX}{variant}-animated");
            assert!(
                !offered(by_id(&animated), Some(LEGACY_DUPLICATING_FELINORS)),
                "{animated}"
            );
        }
        // Each pair shares one seed, each clause of the first ending where the second's begins.
        for (a, b) in R3017001_PAIRS {
            let (a, b) = (by_id(a), by_id(b));
            assert_eq!(a.offer.len(), b.offer.len());
            for (first, second) in a.offer.iter().zip(&b.offer) {
                assert_eq!(first.types, second.types);
                let (Some(low), Some(high)) = (&first.sample, &second.sample) else {
                    panic!("{} and {} sample", a.id, b.id);
                };
                assert_eq!(low.seed, high.seed);
                assert_eq!((low.from, low.percent), (0, high.from));
                assert_eq!(high.percent, OFFER_SAMPLE_PERCENT_SCALE);
            }
        }
    }

    #[test]
    fn r3017001_every_card_is_offered_at_least_variant_min_per_card_variants() {
        use crate::config::{VARIANT_MIN_PER_CARD, VERSION_OFFER_MIN_CARDS, VERSION_OFFER_MIN_PER_SET};
        use jackioh_engine::SHIPPED_SETS;
        let cosmetics = shipped().unwrap_or_else(|error| panic!("cosmetics.json: {error}"));
        let defs = shipped_defs();
        // Each variant but the Default by its first level, the one its other levels are offered as.
        let firsts: Vec<&Cosmetic> = cosmetics
            .iter()
            .filter(|cosmetic| {
                cosmetic
                    .level
                    .as_ref()
                    .is_some_and(|level| level.variant != DEFAULT_VARIANT && level.rank == 1)
            })
            .collect();
        assert_eq!(firsts.len(), R3015000_LADDERS.len() - 1 + R3017000_LADDERS.len());
        let on = |cosmetic: &Cosmetic| -> BTreeSet<&str> {
            defs.keys()
                .map(String::as_str)
                .filter(|card| offered(cosmetic, Some(card)))
                .collect()
        };
        let non_tokens: BTreeSet<&str> = defs
            .iter()
            .filter(|(_, def)| !def.is_token())
            .map(|(id, _)| id.as_str())
            .collect();
        // The floor, on every card, Legacy ones among them.
        let mut legacy = 0;
        for card in &non_tokens {
            let variants = firsts.iter().filter(|first| offered(first, Some(card))).count();
            assert!(variants >= VARIANT_MIN_PER_CARD, "{card}: {variants}");
            legacy += usize::from(defs[*card].tags.contains(&Tag::Legacy));
        }
        assert!(legacy > 0 && non_tokens.contains(LEGACY_DUPLICATING_FELINORS));
        // The mix: no two variants on the same cards, and none on every card.
        for (at, a) in firsts.iter().enumerate() {
            assert!(on(a).len() < non_tokens.len(), "{}", a.id);
            for b in &firsts[at + 1..] {
                assert_ne!(on(a), on(b), "{} and {}", a.id, b.id);
            }
        }
        // The spread: each part 6 level reaches its floor in all and in every set that ships.
        for cosmetic in r3017000_new(&cosmetics) {
            let cards = on(cosmetic);
            assert!(cards.len() >= VERSION_OFFER_MIN_CARDS, "{}", cosmetic.id);
            for set in SHIPPED_SETS {
                let in_set = cards.iter().filter(|card| defs[**card].set == set).count();
                assert!(
                    in_set >= VERSION_OFFER_MIN_PER_SET,
                    "{} in {set}: {in_set}",
                    cosmetic.id
                );
            }
        }
    }
}
