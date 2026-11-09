//! The fixtures the economy's unit tests share (#2160): a printed rarity's place in the tier
//! order, and the foils the cosmetic rolls draw from (`roll`, `cosmetic_packs`).

use jackioh_engine::wire::PrintedRarity;

use crate::cosmetics::{Cosmetic, CosmeticKind, CosmeticRarity, LevelAdds, OfferClause, VariantLevel};

/// A printed rarity's place in `PrintedRarity::ALL`, commonest first.
pub(crate) fn card_place(rarity: PrintedRarity) -> usize {
    PrintedRarity::ALL
        .iter()
        .position(|each| *each == rarity)
        .expect("a printed rarity")
}

/// A per-card cosmetic a pack may drop.
pub(crate) fn foil(id: &str, rarity: CosmeticRarity) -> Cosmetic {
    Cosmetic {
        id: id.to_owned(),
        kind: CosmeticKind::Foil,
        name: id.to_owned(),
        rarity,
        default: false,
        starter: false,
        chase: rarity.is_chase(),
        jlorious: false,
        track: None,
        track_level: None,
        card: None,
        offer: Vec::new(),
        level: None,
    }
}

/// R3015000: a level of `variant` at `rank`, adding `adds`, offered on every non-token card, as a version is.
pub(crate) fn level(id: &str, variant: &str, rank: u8, adds: LevelAdds, rarity: CosmeticRarity) -> Cosmetic {
    Cosmetic {
        kind: CosmeticKind::Version,
        offer: vec![OfferClause::default()],
        level: Some(VariantLevel {
            variant: variant.to_owned(),
            rank,
            adds,
            default_on: Vec::new(),
        }),
        ..foil(id, rarity)
    }
}

/// One droppable foil of each cosmetic rarity the registry has.
pub(crate) fn four_foils() -> Vec<Cosmetic> {
    vec![
        foil("foil-common", CosmeticRarity::Common),
        foil("foil-rare", CosmeticRarity::Rare),
        foil("foil-epic", CosmeticRarity::Epic),
        foil("foil-legendary", CosmeticRarity::Legendary),
    ]
}
