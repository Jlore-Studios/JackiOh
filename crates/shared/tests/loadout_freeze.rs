//! A seat's loadout as its match freezes it (`src/loadout.rs`, SPEC §14.2; R1834002, R2655000, R2715000, R2725000,
//! R2949000, R3013002), moved from the server's `loadout.rs` with the functions: every id that leaves
//! `freeze_loadout` is one of the registry, of its slot's kind and owned where it is worn, and anything else is the
//! kind's default; the stamps a seat wears; and the plain records the server builds its picks from. These tests
//! run on a small registry of their own, so what they hold is the rule and not the shipped data.

use jackioh_shared::cosmetics::{
    CardCosmetic, Cosmetic, CosmeticKind, CosmeticRarity, LevelAdds, OfferClause, OwnedCosmetics,
    VERSION_FULL_ART_ID, VariantLevel, registry,
};
use jackioh_shared::deck::DeckSlots;
use jackioh_shared::loadout::{
    AccountPicks, AccountSettings, CardPicks, CardProficiency, CardStamp, FullArtPick, Loadout,
    LoadoutAnswer, LoadoutCall, SeatLoadout, SeatPicks, apply, freeze_loadout, setting_key, worn_stamps,
};
use serde_json::{Value, json};

fn entry(id: &str, kind: CosmeticKind, default: bool, starter: bool) -> Cosmetic {
    Cosmetic {
        id: id.to_owned(),
        kind,
        name: id.to_owned(),
        rarity: if default || kind.is_per_card() {
            CosmeticRarity::Common
        } else {
            CosmeticRarity::Epic
        },
        default,
        starter,
        chase: false,
        jlorious: false,
        card: None,
        offer: Vec::new(),
        level: None,
    }
}

/// Each kind's default, one starter and one earned item per slot kind, a full art, and (R3015000) Full art's
/// first level, offered on that full art's card alone.
fn test_registry() -> Vec<Cosmetic> {
    let mut cosmetics = Vec::new();
    for (kind, prefix) in [
        (CosmeticKind::Foil, "foil"),
        (CosmeticKind::Finish, "finish"),
        (CosmeticKind::Border, "border"),
        (CosmeticKind::Table, "table"),
        (CosmeticKind::Field, "field"),
        (CosmeticKind::CardBack, "card-back"),
    ] {
        cosmetics.push(entry(&format!("{prefix}-default"), kind, true, true));
        cosmetics.push(entry(&format!("{prefix}-starter"), kind, false, true));
        cosmetics.push(entry(&format!("{prefix}-earned"), kind, false, false));
    }
    cosmetics.push(Cosmetic {
        card: Some("core-001".to_owned()),
        rarity: CosmeticRarity::Epic,
        ..entry("full-art-core-001", CosmeticKind::FullArt, false, false)
    });
    cosmetics.push(Cosmetic {
        rarity: CosmeticRarity::Legendary,
        offer: vec![OfferClause {
            cards: vec!["core-001".to_owned()],
            ..OfferClause::default()
        }],
        level: Some(VariantLevel {
            variant: "full-art".to_owned(),
            rank: 1,
            adds: LevelAdds::FullArt,
            default_on: Vec::new(),
        }),
        ..entry(VERSION_FULL_ART_ID, CosmeticKind::Version, false, false)
    });
    cosmetics
}

fn account(pairs: &[(CosmeticKind, &str)], full_art: bool) -> AccountPicks {
    AccountPicks {
        picks: pairs
            .iter()
            .map(|(kind, id)| (setting_key(*kind).to_owned(), (*id).to_owned()))
            .collect(),
        full_art,
    }
}

fn cards(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| (*id).to_owned()).collect()
}

#[test]
fn r1834002_deck_then_account_then_default() {
    let registry = test_registry();
    let owned = OwnedCosmetics::default();
    let picks = account(
        &[
            (CosmeticKind::Table, "table-starter"),
            (CosmeticKind::Field, "field-starter"),
            (CosmeticKind::Foil, "foil-starter"),
        ],
        true,
    );
    let deck = DeckSlots {
        table: Some("table-default".to_owned()),
        field: None,
        card_back: None,
    };
    let loadout = freeze_loadout(&registry, &owned, &picks, Some(&deck), &[], &cards(&["core-002"]));
    assert_eq!(loadout.seat.table, "table-default");
    assert_eq!(loadout.seat.field, "field-starter");
    assert_eq!(loadout.seat.card_back, "card-back-default");
    // R1953001: a card slot is its default, whatever the account's settings still hold.
    assert_eq!(loadout.seat.foil, "foil-default");
    assert_eq!(loadout.seat.finish, "finish-default");
    assert!(loadout.cards.is_empty());
    // No deck (All Random): the account's, then the default.
    let random = freeze_loadout(&registry, &owned, &picks, None, &[], &[]);
    assert_eq!(random.seat.table, "table-starter");
    // No settings at all: every default, the full-art switch on.
    let bare = AccountPicks::of_settings(None);
    assert!(bare.full_art);
    assert_eq!(
        freeze_loadout(&registry, &owned, &bare, None, &[], &[]).seat,
        SeatLoadout::defaults_of(&registry)
    );
}

#[test]
fn r1834002_an_unknown_or_unowned_pick_freezes_as_the_default() {
    let registry = test_registry();
    let none = OwnedCosmetics::default();
    let picks = account(
        &[
            (CosmeticKind::Table, "table-earned"),
            (CosmeticKind::Field, "field-unheard-of"),
            (CosmeticKind::CardBack, "table-starter"),
            (CosmeticKind::Border, "border-earned"),
        ],
        true,
    );
    let loadout = freeze_loadout(&registry, &none, &picks, None, &[], &[]);
    assert_eq!(loadout.seat, SeatLoadout::defaults_of(&registry));
    // An unowned deck pick is the default too, never the account's pick under it.
    let deck = DeckSlots {
        table: Some("table-earned".to_owned()),
        ..DeckSlots::default()
    };
    let starter = account(&[(CosmeticKind::Table, "table-starter")], true);
    assert_eq!(
        freeze_loadout(&registry, &none, &starter, Some(&deck), &[], &[])
            .seat
            .table,
        "table-default"
    );
    // Owned on the account, the same picks are worn.
    let owned = OwnedCosmetics {
        cards: Vec::new(),
        account: vec!["table-earned".to_owned()],
    };
    assert_eq!(
        freeze_loadout(&registry, &owned, &picks, None, &[], &[])
            .seat
            .table,
        "table-earned"
    );
    // A per-card kind is never owned on the account, so an earned border picked for every card
    // is the default on the seat.
    assert_eq!(
        freeze_loadout(&registry, &owned, &picks, None, &[], &[])
            .seat
            .border,
        "border-default"
    );
}

#[test]
fn r1834002_per_card_only_for_owned_deck_cards() {
    let registry = test_registry();
    let owned = OwnedCosmetics {
        cards: vec![
            // R3015005: the full-art card owns Full art's first level, what its full art prints as.
            CardCosmetic {
                card_id: "core-001".to_owned(),
                cosmetic_id: VERSION_FULL_ART_ID.to_owned(),
            },
            CardCosmetic {
                card_id: "core-002".to_owned(),
                cosmetic_id: "foil-earned".to_owned(),
            },
        ],
        account: Vec::new(),
    };
    let picks = account(&[(CosmeticKind::Finish, "finish-starter")], true);
    let card_picks = vec![
        // Owned on its card: worn.
        CardPicks {
            card_id: "core-002".to_owned(),
            foil: Some("foil-earned".to_owned()),
            ..CardPicks::default()
        },
        // Not owned on this card: the default, which the seat already wears.
        CardPicks {
            card_id: "core-003".to_owned(),
            foil: Some("foil-earned".to_owned()),
            ..CardPicks::default()
        },
        // The account's own finish: the card's pick, since the seat wears the default (R1953001).
        CardPicks {
            card_id: "core-004".to_owned(),
            finish: Some("finish-starter".to_owned()),
            ..CardPicks::default()
        },
        // An id the registry does not know: the default, the card's word final.
        CardPicks {
            card_id: "core-005".to_owned(),
            finish: Some("finish-from-a-newer-client".to_owned()),
            ..CardPicks::default()
        },
        // Not in the deck: never frozen.
        CardPicks {
            card_id: "core-009".to_owned(),
            border: Some("border-starter".to_owned()),
            ..CardPicks::default()
        },
    ];
    let deck = cards(&[
        "core-001", "core-002", "core-002", "core-003", "core-004", "core-005",
    ]);
    let loadout = freeze_loadout(&registry, &owned, &picks, None, &card_picks, &deck);
    assert_eq!(loadout.seat.finish, "finish-default");
    assert_eq!(
        loadout.cards,
        vec![
            CardPicks {
                card_id: "core-001".to_owned(),
                full_art: Some(FullArtPick::Id(VERSION_FULL_ART_ID.to_owned())),
                ..CardPicks::default()
            },
            CardPicks {
                card_id: "core-002".to_owned(),
                foil: Some("foil-earned".to_owned()),
                ..CardPicks::default()
            },
            CardPicks {
                card_id: "core-004".to_owned(),
                finish: Some("finish-starter".to_owned()),
                ..CardPicks::default()
            },
        ]
    );
    // The full art follows the card's own switch first, then the account's setting, and needs
    // the cosmetic owned.
    let off = account(&[], false);
    assert!(
        freeze_loadout(&registry, &owned, &off, None, &[], &deck)
            .card("core-001")
            .is_none()
    );
    let on_for_the_card = vec![CardPicks {
        card_id: "core-001".to_owned(),
        full_art: Some(FullArtPick::On(true)),
        ..CardPicks::default()
    }];
    assert_eq!(
        freeze_loadout(&registry, &owned, &off, None, &on_for_the_card, &deck)
            .card("core-001")
            .and_then(|card| card.full_art.clone()),
        Some(FullArtPick::Id(VERSION_FULL_ART_ID.to_owned()))
    );
    assert!(
        freeze_loadout(
            &registry,
            &OwnedCosmetics::default(),
            &picks,
            None,
            &on_for_the_card,
            &deck
        )
        .card("core-001")
        .is_none()
    );
}

/// Rows of `ids`, each on `card`.
fn rows_on(card: &str, ids: &[&str]) -> OwnedCosmetics {
    OwnedCosmetics {
        cards: ids
            .iter()
            .map(|id| CardCosmetic {
                card_id: card.to_owned(),
                cosmetic_id: (*id).to_owned(),
            })
            .collect(),
        account: Vec::new(),
    }
}

/// What the shipped registry freezes for Mr. Vanilla, a full-art card, owning `owned`, its own pick `pick`
/// (none when `None`) and the Full-art cards setting `account_on`.
fn vanilla_frozen(
    owned: &OwnedCosmetics,
    account_on: bool,
    pick: Option<FullArtPick>,
) -> Option<FullArtPick> {
    let picks: Vec<CardPicks> = pick
        .map(|pick| CardPicks {
            card_id: "core-008".to_owned(),
            full_art: Some(pick),
            ..CardPicks::default()
        })
        .into_iter()
        .collect();
    let deck = cards(&["core-008", "core-002"]);
    freeze_loadout(registry(), owned, &account(&[], account_on), None, &picks, &deck)
        .card("core-008")
        .and_then(|card| card.full_art.clone())
}

#[test]
fn r2655000_r3015005_a_card_freezes_the_level_its_picked_tier_maps_to_when_owned() {
    use jackioh_shared::cosmetics::{VERSION_FULL_ART_ANIMATED_ID, VERSION_FULL_ART_SIGNED_ID};
    let frozen = vanilla_frozen;
    let owning = |ids: &[&str]| rows_on("core-008", ids);
    let id = |id: &str| FullArtPick::Id(id.to_owned());
    let every = owning(&[
        "full-art-mr-vanilla",
        "full-art-mr-vanilla-animated",
        "full-art-mr-vanilla-signed",
    ]);
    // An owned tier freezes as the level it maps to (R3015005): the Mythic Full art's signed level, the
    // Legendary its animated one, the Epic, by its id or by `true`, its first; with the setting off the card's
    // own pick still prints it.
    assert_eq!(
        frozen(&every, false, Some(id("full-art-mr-vanilla-signed"))),
        Some(id(VERSION_FULL_ART_SIGNED_ID))
    );
    assert_eq!(
        frozen(&every, true, Some(id("full-art-mr-vanilla-animated"))),
        Some(id(VERSION_FULL_ART_ANIMATED_ID))
    );
    assert_eq!(
        frozen(&every, true, Some(id("full-art-mr-vanilla"))),
        Some(id(VERSION_FULL_ART_ID))
    );
    assert_eq!(
        frozen(&every, false, Some(FullArtPick::On(true))),
        Some(id(VERSION_FULL_ART_ID))
    );
    // With no pick of its own the card wears Full art's first level when the setting is on, never a higher one;
    // a retired Mythic alone owns that level too (R3015006).
    assert_eq!(frozen(&every, true, None), Some(id(VERSION_FULL_ART_ID)));
    assert_eq!(frozen(&every, false, None), None);
    assert_eq!(
        frozen(&owning(&["full-art-mr-vanilla-signed"]), true, None),
        Some(id(VERSION_FULL_ART_ID))
    );
    // A tier not owned wears the highest level below it that is; another card's, an unknown id or `false`
    // freeze nothing.
    let epic_only = owning(&["full-art-mr-vanilla"]);
    assert_eq!(
        frozen(&epic_only, true, Some(id("full-art-mr-vanilla-signed"))),
        Some(id(VERSION_FULL_ART_ID))
    );
    assert_eq!(
        frozen(&every, true, Some(id("full-art-gary-the-gambler-signed"))),
        None
    );
    assert_eq!(
        frozen(&every, true, Some(id("full-art-from-a-newer-client"))),
        None
    );
    assert_eq!(frozen(&every, true, Some(FullArtPick::On(false))), None);
    assert_eq!(
        frozen(&OwnedCosmetics::default(), true, Some(FullArtPick::On(true))),
        None
    );
    // The frozen pick travels as the level's id, as the client reads `fullArt`; `true` still reads as `true`.
    assert_eq!(
        serde_json::to_value(FullArtPick::On(true)).ok(),
        Some(serde_json::json!(true))
    );
    assert_eq!(
        serde_json::to_value(id(VERSION_FULL_ART_SIGNED_ID)).ok(),
        Some(serde_json::json!(VERSION_FULL_ART_SIGNED_ID))
    );
}

#[test]
fn r2715000_r3015005_an_owned_animated_art_freezes_as_the_default_animated_level() {
    use jackioh_shared::cosmetics::VERSION_ANIMATED_ID;
    let frozen = vanilla_frozen;
    let owning = |ids: &[&str]| rows_on("core-008", ids);
    let animated = || Some(FullArtPick::Id("animated-mr-vanilla".to_owned()));
    let level = |id: &str| Some(FullArtPick::Id(id.to_owned()));
    let both = owning(&["full-art-mr-vanilla", "animated-mr-vanilla"]);
    // An owned Animated pick freezes as the Default variant's animated level it maps to, whatever the setting.
    assert_eq!(frozen(&both, false, animated()), level(VERSION_ANIMATED_ID));
    assert_eq!(frozen(&both, true, animated()), level(VERSION_ANIMATED_ID));
    // `true`, or the setting with no pick, freezes Full art's first level, the still Epic's look.
    assert_eq!(
        frozen(&both, false, Some(FullArtPick::On(true))),
        level(VERSION_FULL_ART_ID)
    );
    assert_eq!(frozen(&both, true, None), level(VERSION_FULL_ART_ID));
    // The Animated art alone owns no full-art level, so `true` and the setting print nothing.
    let animated_only = owning(&["animated-mr-vanilla"]);
    assert_eq!(
        frozen(&animated_only, true, animated()),
        level(VERSION_ANIMATED_ID)
    );
    assert_eq!(frozen(&animated_only, true, Some(FullArtPick::On(true))), None);
    assert_eq!(frozen(&animated_only, true, None), None);
    // Not owned, it freezes nothing.
    assert_eq!(frozen(&owning(&["full-art-mr-vanilla"]), true, animated()), None);
}

#[test]
fn r2725000_r3015005_a_loadout_freezes_one_level_and_no_augment() {
    use jackioh_shared::cosmetics::{
        AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, VERSION_ANIMATED_ID, VERSION_FULL_ART_ANIMATED_ID,
        VERSION_FULL_ART_SIGNED_ID,
    };
    // The shipped registry: Carnivorous Cube (an Epic Unit) is offered Full art's ladder, Speciesist (a Common
    // Unit) is not, and Mr. Vanilla is one of the 21 with retired full arts.
    let registry = registry();
    let (cube, speciesist, vanilla) = ("core-022", "core-002", "core-008");
    let row = |card: &str, id: &str| CardCosmetic {
        card_id: card.to_owned(),
        cosmetic_id: id.to_owned(),
    };
    let pick = |card: &str, full_art: Option<FullArtPick>, augments: &[&str]| CardPicks {
        card_id: card.to_owned(),
        full_art,
        augments: augments.iter().map(|id| (*id).to_owned()).collect(),
        ..CardPicks::default()
    };
    let level = |id: &str| Some(FullArtPick::Id(id.to_owned()));
    let version = || level(VERSION_FULL_ART_ID);
    let every_augment = [
        AUGMENT_SIGNATURE_ID,
        "augment-from-a-newer-client",
        AUGMENT_ANIMATED_ID,
        AUGMENT_ANIMATED_ID,
    ];
    let owned = OwnedCosmetics {
        cards: vec![
            // The version and Animated, and the levels migration 0084 wrote for them (R3015006).
            row(cube, VERSION_FULL_ART_ID),
            row(cube, AUGMENT_ANIMATED_ID),
            row(cube, VERSION_FULL_ART_ANIMATED_ID),
            row(cube, VERSION_ANIMATED_ID),
            // Owned on Speciesist by a grant no shop makes, Full art's ladder is not offered there.
            row(speciesist, VERSION_FULL_ART_ID),
            row(speciesist, VERSION_FULL_ART_ANIMATED_ID),
            // A retired signed full art owns Full art's three levels (R3015006).
            row(vanilla, "full-art-mr-vanilla-signed"),
        ],
        account: Vec::new(),
    };
    let picks = [
        pick(cube, version(), &every_augment),
        pick(speciesist, version(), &every_augment),
        pick(vanilla, version(), &[AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID]),
    ];
    let loadout = freeze_loadout(
        registry,
        &owned,
        &AccountPicks::default(),
        None,
        &picks,
        &cards(&[cube, speciesist, vanilla]),
    );
    // The pick names Full art's signed level; the highest owned at or below it is frozen, with no augment.
    assert_eq!(
        loadout.card(cube),
        Some(&pick(cube, level(VERSION_FULL_ART_ANIMATED_ID), &[]))
    );
    // Not offered: nothing of its own, so the card is left out.
    assert_eq!(loadout.card(speciesist), None);
    assert_eq!(
        loadout.card(vanilla),
        Some(&pick(vanilla, level(VERSION_FULL_ART_SIGNED_ID), &[]))
    );
    // A pick from before #2725 freezes as the level it maps to, and the setting alone prints Full art's first
    // level on a full-art card alone.
    let legacy = OwnedCosmetics {
        cards: vec![
            row(vanilla, "full-art-mr-vanilla"),
            row(vanilla, "full-art-mr-vanilla-signed"),
            row(cube, VERSION_FULL_ART_ID),
        ],
        account: Vec::new(),
    };
    let frozen = |picks: &[CardPicks]| {
        freeze_loadout(
            registry,
            &legacy,
            &AccountPicks::default(),
            None,
            picks,
            &cards(&[cube, vanilla]),
        )
    };
    let signed = Some(FullArtPick::Id("full-art-mr-vanilla-signed".to_owned()));
    assert_eq!(
        frozen(&[pick(vanilla, signed, &[])]).card(vanilla),
        Some(&pick(vanilla, level(VERSION_FULL_ART_SIGNED_ID), &[]))
    );
    let by_setting = frozen(&[]);
    assert_eq!(by_setting.card(vanilla), Some(&pick(vanilla, version(), &[])));
    assert_eq!(by_setting.card(cube), None);
}

#[test]
fn r2949000_r3015005_every_old_pick_shape_freezes_the_level_it_maps_to() {
    use jackioh_shared::cosmetics::{
        AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID, VERSION_ANIMATED_ID, VERSION_FULL_ART_ANIMATED_ID,
        VERSION_FULL_ART_SIGNED_ID,
    };
    let registry = registry();
    let (cube, vanilla) = ("core-022", "core-008");
    let row = |card: &str, id: &str| CardCosmetic {
        card_id: card.to_owned(),
        cosmetic_id: id.to_owned(),
    };
    let pick = |card: &str, full_art: Option<FullArtPick>, augments: &[&str]| CardPicks {
        card_id: card.to_owned(),
        full_art,
        augments: augments.iter().map(|id| (*id).to_owned()).collect(),
        ..CardPicks::default()
    };
    let level = |id: &str| Some(FullArtPick::Id(id.to_owned()));
    let version = || level(VERSION_FULL_ART_ID);
    let both = [AUGMENT_ANIMATED_ID, AUGMENT_SIGNATURE_ID];
    // The version and both augments on Carnivorous Cube and on Mr. Vanilla, whose still Epic is owned too, and
    // the levels migration 0084 wrote for them (R3015006).
    let owned = OwnedCosmetics {
        cards: [cube, vanilla]
            .into_iter()
            .flat_map(|card| {
                [
                    VERSION_FULL_ART_ID,
                    AUGMENT_ANIMATED_ID,
                    AUGMENT_SIGNATURE_ID,
                    VERSION_FULL_ART_ANIMATED_ID,
                    VERSION_FULL_ART_SIGNED_ID,
                    VERSION_ANIMATED_ID,
                ]
                .map(|id| row(card, id))
            })
            .chain([row(vanilla, "full-art-mr-vanilla")])
            .collect(),
        account: Vec::new(),
    };
    let frozen = |account: &AccountPicks, picks: CardPicks| {
        let card = picks.card_id.clone();
        freeze_loadout(registry, &owned, account, None, &[picks], &cards(&[&card]))
            .card(&card)
            .cloned()
    };
    let on = AccountPicks::default();
    // R2949000's Signature look, the version with both augments: Full art's signed level.
    assert_eq!(
        frozen(&on, pick(cube, version(), &both)),
        Some(pick(cube, level(VERSION_FULL_ART_SIGNED_ID), &[]))
    );
    // The version with Signature and no Animated: Full art's first level, as it printed.
    assert_eq!(
        frozen(&on, pick(cube, version(), &[AUGMENT_SIGNATURE_ID])),
        Some(pick(cube, version(), &[]))
    );
    // Animated on the normal face: the Default variant's animated level.
    assert_eq!(
        frozen(&on, pick(cube, Some(FullArtPick::On(false)), &both)),
        Some(pick(cube, level(VERSION_ANIMATED_ID), &[]))
    );
    // A full-art card with no pick of a look wears Full art's first level by the setting, so with both
    // augments its signed level; with the setting off the normal face, so the Default variant's animated level.
    assert_eq!(
        frozen(&on, pick(vanilla, None, &both)),
        Some(pick(vanilla, level(VERSION_FULL_ART_SIGNED_ID), &[]))
    );
    let off = AccountPicks {
        full_art: false,
        ..AccountPicks::default()
    };
    assert_eq!(
        frozen(&off, pick(vanilla, None, &both)),
        Some(pick(vanilla, level(VERSION_ANIMATED_ID), &[]))
    );
    // A level picked since #3015 freezes as itself, and an unowned one, with nothing owned below it, as nothing.
    assert_eq!(
        frozen(&on, pick(cube, level(VERSION_FULL_ART_ANIMATED_ID), &[])),
        Some(pick(cube, level(VERSION_FULL_ART_ANIMATED_ID), &[]))
    );
    assert_eq!(
        frozen(&on, pick(cube, level("version-sketch-full-art-signed"), &[])),
        None
    );
}

fn stamp(card: &str, stamp: i64) -> CardStamp {
    CardStamp {
        card_id: card.to_owned(),
        stamp,
    }
}

#[test]
fn r3013002_a_stamp_is_frozen_for_a_dealt_card() {
    // R3013001: only a stamp the row's level reached is worn.
    let rows = [
        CardProficiency {
            card_id: "core-001".to_owned(),
            plays: 20,
            level: 2,
            stamp: Some(2),
        },
        CardProficiency {
            card_id: "core-002".to_owned(),
            plays: 3,
            level: 1,
            stamp: None,
        },
        CardProficiency {
            card_id: "core-003".to_owned(),
            plays: 3,
            level: 1,
            stamp: Some(4),
        },
        CardProficiency {
            card_id: "core-004".to_owned(),
            plays: 200,
            level: 5,
            stamp: Some(5),
        },
    ];
    let worn = worn_stamps(&rows);
    assert_eq!(worn, vec![stamp("core-001", 2), stamp("core-004", 5)]);
    // A card the deck did not deal is never frozen, and each dealt card is listed once.
    let picks = SeatPicks {
        stamps: worn,
        ..SeatPicks::default()
    };
    let deck: Vec<String> = ["core-001", "core-002", "core-001"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let frozen = picks.freeze(None, &deck);
    assert_eq!(frozen.stamps, vec![stamp("core-001", 2)]);
    assert_eq!(frozen.stamp("core-001"), Some(&stamp("core-001", 2)));
    assert_eq!(frozen.stamp("core-004"), None);
    // That it travels alone, once, and only once its card is shown is `jackioh-shared`'s rule
    // (`tests/loadout.rs`).
    // A bot's seat wears none (R1454): its picks are read off its row, with no stamps.
    assert!(SeatPicks::default().freeze(None, &deck).stamps.is_empty());
}

#[test]
fn r1834002_the_settings_hold_the_cosmetics_group_texts_and_the_full_art_switch() {
    // The server builds `AccountSettings` from a settings row (its own test reads the row); what the picks
    // are of it is this crate's: a key of a kind, its text, and the switch on unless it is off.
    let settings = AccountSettings {
        cosmetics: [
            ("foil", "foil-starter"),
            ("cardBack", "card-back-starter"),
            ("somethingElse", "not-a-kind"),
        ]
        .into_iter()
        .map(|(key, id)| (key.to_owned(), id.to_owned()))
        .collect(),
        full_art: Some(false),
    };
    let picks = AccountPicks::of_settings(Some(&settings));
    assert_eq!(picks.get(CosmeticKind::Foil), Some("foil-starter"));
    assert_eq!(picks.get(CosmeticKind::CardBack), Some("card-back-starter"));
    assert_eq!(picks.get(CosmeticKind::Border), None);
    assert_eq!(picks.get(CosmeticKind::Table), None);
    assert!(!picks.full_art);
    assert_eq!(picks.picks.len(), 2, "a key no kind has is left out");
    // No settings, or settings with no switch: nothing picked, the switch on.
    assert_eq!(AccountPicks::of_settings(None), AccountPicks::default());
    assert!(AccountPicks::of_settings(Some(&AccountSettings::default())).full_art);
    let on = AccountSettings {
        full_art: Some(true),
        ..AccountSettings::default()
    };
    assert!(AccountPicks::of_settings(Some(&on)).full_art);
}

/// A call as a host sends it: through JSON and back.
fn over_json(call: &LoadoutCall) -> LoadoutCall {
    serde_json::from_str(&serde_json::to_string(call).expect("a call serializes")).expect("a call reads back")
}

#[test]
fn r1834002_the_freeze_calls_answer_what_the_functions_they_name_answer() {
    // The shipped registry judges a call's picks, as it judges the server's: a pick it owns is worn.
    let table = registry()
        .iter()
        .find(|cosmetic| cosmetic.kind == CosmeticKind::Table && !cosmetic.default && !cosmetic.starter)
        .expect("an earned playmat");
    let owned = OwnedCosmetics {
        cards: Vec::new(),
        account: vec![table.id.clone()],
    };
    let picks = SeatPicks {
        owned,
        account: AccountPicks {
            picks: [(setting_key(CosmeticKind::Table).to_owned(), table.id.clone())]
                .into_iter()
                .collect(),
            full_art: true,
        },
        cards: Vec::new(),
        stamps: vec![CardStamp {
            card_id: "core-001".to_owned(),
            stamp: 3,
        }],
    };
    let deck: Vec<String> = ["core-001", "core-002"].into_iter().map(str::to_owned).collect();
    let call = over_json(&LoadoutCall::FreezeSeat {
        picks: Box::new(picks.clone()),
        deck: None,
        deck_cards: deck.clone(),
    });
    let LoadoutAnswer::Frozen(frozen) = apply(&call) else {
        panic!("freezeSeat answers a loadout");
    };
    assert_eq!(*frozen, picks.freeze(None, &deck));
    assert_eq!(frozen.seat.table, table.id, "the owned account pick is worn");
    assert_eq!(frozen.stamps.len(), 1);

    assert_eq!(
        apply(&over_json(&LoadoutCall::DefaultLoadout)),
        LoadoutAnswer::Frozen(Box::new(Loadout::defaults()))
    );
    assert_eq!(Loadout::defaults().seat, SeatLoadout::defaults_of(registry()));

    let settings = AccountSettings {
        full_art: Some(false),
        ..AccountSettings::default()
    };
    assert_eq!(
        apply(&over_json(&LoadoutCall::AccountPicks {
            settings: Some(settings.clone())
        })),
        LoadoutAnswer::Account(AccountPicks::of_settings(Some(&settings)))
    );

    let rows = vec![CardProficiency {
        card_id: "core-001".to_owned(),
        plays: 20,
        level: 2,
        stamp: Some(2),
    }];
    assert_eq!(
        apply(&over_json(&LoadoutCall::WornStamps { rows: rows.clone() })),
        LoadoutAnswer::Stamps(worn_stamps(&rows))
    );
}

#[test]
fn r1834002_a_freeze_call_reads_from_bare_json() {
    // A host that has nothing but a deck's cards sends that and gets the defaults, the switch on.
    let call: LoadoutCall = serde_json::from_value(json!({
        "call": "freezeSeat", "picks": {}, "deckCards": ["core-001"],
    }))
    .expect("a bare freezeSeat reads");
    let LoadoutAnswer::Frozen(frozen) = apply(&call) else {
        panic!("freezeSeat answers a loadout");
    };
    assert_eq!(frozen.seat, Loadout::defaults().seat);
    let text: Value = serde_json::to_value(LoadoutAnswer::Frozen(frozen)).expect("serializes");
    assert!(
        text.get("frozen")
            .is_some_and(|loadout| loadout.get("seat").is_some())
    );
}
