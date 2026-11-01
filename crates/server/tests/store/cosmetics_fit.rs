//! `cli/cosmetics_fit.rs` (cosmetics-fit), SPEC §14.2, R1834006: what people's seats wore, bucketed
//! by account age, as the ladder bots' fits to paste into config, a bucket under the minimum
//! keeping the prior and no person named in what it prints. The store's read (people's seats only)
//! is `tests/store/contract.rs`'s; what a fit draws is `tests/bots/cosmetics.rs`'s.

use std::collections::BTreeMap;

use jackioh_server::bots::cosmetics::fit_for;
use jackioh_server::cli::cosmetics_fit::{self, BucketCosmetics, fit_cosmetics, render_cosmetic_fits};
use jackioh_server::config::{
    BOT_COSMETIC_FIT_MIN_SAMPLES, BOT_COSMETIC_FITS, BOT_COSMETIC_PRIOR, CosmeticFit,
};
use jackioh_server::cosmetics::registry;
use jackioh_server::db::store::LoadoutSample;
use jackioh_server::economy::store::{CardCosmetic, CardPicks, OwnedCosmetics};
use jackioh_server::loadout::{Loadout, SeatLoadout};

/// Unit conversion, not configuration: a day in milliseconds.
const DAY: i64 = 86_400_000;
/// When the samples' first account was made, epoch ms: any fixed time does.
const T0: i64 = 1_000 * DAY;
/// How close two fitted numbers must be: well under the printed four decimals.
const EPSILON: f64 = 1e-9;

/// The people of the samples, by profile id. None of these may reach what the fit prints.
const ANNA: &str = "a1834006-0000-4000-8000-000000000001";
const BORIS: &str = "a1834006-0000-4000-8000-000000000002";
const CLEO: &str = "a1834006-0000-4000-8000-000000000003";

fn ids(cards: &[&str]) -> Vec<String> {
    cards.iter().map(|card| (*card).to_string()).collect()
}

/// A seat wearing every default but what `over` sets, by kind's camel-case key.
fn seat(over: &[(&str, &str)]) -> SeatLoadout {
    let mut seat = serde_json::to_value(SeatLoadout::defaults_of(registry())).expect("a seat loadout");
    for (key, id) in over {
        seat[*key] = serde_json::json!(id);
    }
    serde_json::from_value(seat).expect("a seat loadout")
}

fn sample(
    profile_id: &str,
    account_created_at: i64,
    match_created_at: i64,
    deck: &[&str],
    seat: SeatLoadout,
    cards: Vec<CardPicks>,
) -> LoadoutSample {
    LoadoutSample {
        profile_id: profile_id.to_string(),
        account_created_at,
        match_created_at,
        deck: ids(deck),
        loadout: Loadout { seat, cards },
    }
}

/// Four people's seats, oldest match first, as `matches_loadout_samples` reads them:
///
///  - Anna, two days old: the felt playmat, two cards of her own (a foil on core-002, a border on
///    core-005; core-001's printed full art is no pick of a slot), and Big D-fender's full art,
///    which she owns, printed;
///  - Boris, five days old: the default playmat, the rain backdrop and the rainbow foil, one card
///    of his own (the Glitter foil he earned on core-003), and Big D-fender's full art, which he
///    owns, not printed;
///  - Anna again at ten days old, every default, playing Gary's card without printing the full art
///    she owns: her later match puts her, for ownership, in the 30-day bucket;
///  - Cleo, 211 days old, every default, owning nothing.
fn samples() -> Vec<LoadoutSample> {
    vec![
        sample(
            ANNA,
            T0,
            T0 + 2 * DAY,
            &["core-001", "core-002", "core-005"],
            seat(&[("table", "table-felt")]),
            vec![
                CardPicks {
                    card_id: "core-001".to_string(),
                    full_art: Some(true.into()),
                    ..CardPicks::default()
                },
                CardPicks {
                    card_id: "core-002".to_string(),
                    foil: Some("foil-rainbow".to_string()),
                    ..CardPicks::default()
                },
                CardPicks {
                    card_id: "core-005".to_string(),
                    border: Some("border-wood".to_string()),
                    ..CardPicks::default()
                },
            ],
        ),
        sample(
            BORIS,
            T0 + DAY,
            T0 + 6 * DAY,
            &["core-001", "core-003"],
            seat(&[("field", "field-rain"), ("foil", "foil-rainbow")]),
            vec![CardPicks {
                card_id: "core-003".to_string(),
                foil: Some("foil-glitter".to_string()),
                ..CardPicks::default()
            }],
        ),
        sample(
            ANNA,
            T0,
            T0 + 10 * DAY,
            &["core-004", "core-006"],
            seat(&[]),
            Vec::new(),
        ),
        sample(
            CLEO,
            T0 - 200 * DAY,
            T0 + 11 * DAY,
            &["core-001"],
            seat(&[]),
            Vec::new(),
        ),
    ]
}

/// What each person owns now: Anna both full arts she plays, the Glitter foil on core-005 (worn on
/// no seat) and a portrait (an account cosmetic no loadout slot wears, so no ownership share),
/// Boris Big D-fender's and the Glitter foil on core-003; Cleo is missing, so owns none.
fn owned() -> BTreeMap<String, OwnedCosmetics> {
    let on = |cosmetic_id: &str, card_id: &str| CardCosmetic {
        card_id: card_id.to_string(),
        cosmetic_id: cosmetic_id.to_string(),
    };
    BTreeMap::from([
        (
            ANNA.to_string(),
            OwnedCosmetics {
                cards: vec![
                    on("full-art-big-d-fender", "core-001"),
                    on("full-art-gary-the-gambler", "core-004"),
                    on("foil-glitter", "core-005"),
                ],
                account: vec!["portrait-gary".to_string()],
            },
        ),
        (
            BORIS.to_string(),
            OwnedCosmetics {
                cards: vec![
                    on("full-art-big-d-fender", "core-001"),
                    on("foil-glitter", "core-003"),
                ],
                account: Vec::new(),
            },
        ),
    ])
}

fn pairs(list: &[(&str, f64)]) -> Vec<(String, f64)> {
    list.iter()
        .map(|(id, share)| ((*id).to_string(), *share))
        .collect()
}

/// Every kind's default at `share` of the seats, in the registry's order.
fn defaults_at(share: f64) -> Vec<(String, f64)> {
    pairs(&[
        ("foil-none", share),
        ("finish-plain", share),
        ("border-standard", share),
        ("table-tavern", share),
        ("field-none", share),
        ("card-back-standard", share),
    ])
}

/// The four seats' rows: the 7-day bucket (Anna's first seat and Boris's), the 30-day (Anna's
/// second), no 90-day row (no seat), and the one past every bound (Cleo's).
fn expected_rows() -> Vec<BucketCosmetics> {
    vec![
        BucketCosmetics {
            max_age_days: Some(7),
            samples: 2,
            slots: pairs(&[
                ("foil-none", 0.5),
                ("foil-rainbow", 0.5),
                ("finish-plain", 1.0),
                ("border-standard", 1.0),
                ("table-tavern", 0.5),
                ("table-felt", 0.5),
                ("field-none", 0.5),
                ("field-rain", 0.5),
                ("card-back-standard", 1.0),
            ]),
            per_card_share: 1.0,
            cards_per_deck_mean: 1.5,
            // Anna's two starters, core-002's foil and core-005's border; Boris's earned Glitter is
            // no share of them, but its wear on core-003, where he owns it.
            card_picks: pairs(&[("foil-rainbow", 0.5), ("border-wood", 0.5)]),
            earned_wear: vec![
                ("foil-glitter".to_string(), "core-003".to_string(), 1.0),
                ("foil-glitter".to_string(), "core-005".to_string(), 0.0),
            ],
            full_art_wear: pairs(&[("full-art-big-d-fender", 0.5)]),
            // Boris alone: Anna's latest match is in the next bucket.
            ownership: vec![
                ("foil-glitter".to_string(), Some("core-003".to_string()), 1.0),
                (
                    "full-art-big-d-fender".to_string(),
                    Some("core-001".to_string()),
                    1.0,
                ),
            ],
        },
        BucketCosmetics {
            max_age_days: Some(30),
            samples: 1,
            slots: defaults_at(1.0),
            per_card_share: 0.0,
            cards_per_deck_mean: 0.0,
            card_picks: Vec::new(),
            earned_wear: Vec::new(),
            full_art_wear: pairs(&[("full-art-gary-the-gambler", 0.0)]),
            ownership: vec![
                ("foil-glitter".to_string(), Some("core-005".to_string()), 1.0),
                (
                    "full-art-big-d-fender".to_string(),
                    Some("core-001".to_string()),
                    1.0,
                ),
                (
                    "full-art-gary-the-gambler".to_string(),
                    Some("core-004".to_string()),
                    1.0,
                ),
            ],
        },
        BucketCosmetics {
            max_age_days: None,
            samples: 1,
            slots: defaults_at(1.0),
            per_card_share: 0.0,
            cards_per_deck_mean: 0.0,
            card_picks: Vec::new(),
            earned_wear: Vec::new(),
            full_art_wear: Vec::new(),
            ownership: Vec::new(),
        },
    ]
}

#[track_caller]
fn assert_rows(actual: &[BucketCosmetics], expected: &[BucketCosmetics]) {
    let close = |a: f64, b: f64| (a - b).abs() < EPSILON;
    let same_pairs = |a: &[(String, f64)], b: &[(String, f64)]| {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.0 == y.0 && close(x.1, y.1))
    };
    assert_eq!(actual.len(), expected.len(), "{actual:#?}");
    for (row, want) in actual.iter().zip(expected) {
        assert_eq!((row.max_age_days, row.samples), (want.max_age_days, want.samples));
        assert!(
            same_pairs(&row.slots, &want.slots),
            "{:?}: {:?}",
            row.max_age_days,
            row.slots
        );
        assert!(close(row.per_card_share, want.per_card_share), "{row:?}");
        assert!(
            close(row.cards_per_deck_mean, want.cards_per_deck_mean),
            "{row:?}"
        );
        assert!(same_pairs(&row.card_picks, &want.card_picks), "{row:?}");
        assert_eq!(row.earned_wear.len(), want.earned_wear.len(), "{row:?}");
        for (got, wanted) in row.earned_wear.iter().zip(&want.earned_wear) {
            assert!(
                got.0 == wanted.0 && got.1 == wanted.1 && close(got.2, wanted.2),
                "{row:?}"
            );
        }
        assert!(same_pairs(&row.full_art_wear, &want.full_art_wear), "{row:?}");
        assert_eq!(row.ownership.len(), want.ownership.len(), "{row:?}");
        for (got, wanted) in row.ownership.iter().zip(&want.ownership) {
            assert!(
                got.0 == wanted.0 && got.1 == wanted.1 && close(got.2, wanted.2),
                "{row:?}"
            );
        }
    }
}

#[test]
fn r1834006_people_only_rows() {
    let rows = fit_cosmetics(registry(), &samples(), &owned());
    assert_rows(&rows, &expected_rows());

    let printed = render_cosmetic_fits(&rows, 1);
    assert_eq!(
        printed,
        "// jackioh-server cosmetics-fit: paste into BOT_COSMETIC_FITS, crates/server/src/config.rs\n\
         \x20   CosmeticFit { max_age_days: Some(7), samples: 2, slots: &[(\"foil-none\", 0.5000), (\"foil-rainbow\", 0.5000), (\"finish-plain\", 1.0000), (\"border-standard\", 1.0000), (\"table-tavern\", 0.5000), (\"table-felt\", 0.5000), (\"field-none\", 0.5000), (\"field-rain\", 0.5000), (\"card-back-standard\", 1.0000)], per_card_share: 1.0000, cards_per_deck_mean: 1.5000, card_picks: &[(\"foil-rainbow\", 0.5000), (\"border-wood\", 0.5000)], earned_wear: &[CosmeticWear { cosmetic: \"foil-glitter\", card: \"core-003\", share: 1.0000 }, CosmeticWear { cosmetic: \"foil-glitter\", card: \"core-005\", share: 0.0000 }], full_art_wear: &[(\"full-art-big-d-fender\", 0.5000)], ownership: &[CosmeticOwnership { cosmetic: \"foil-glitter\", card: Some(\"core-003\"), share: 1.0000 }, CosmeticOwnership { cosmetic: \"full-art-big-d-fender\", card: Some(\"core-001\"), share: 1.0000 }] },\n\
         \x20   CosmeticFit { max_age_days: Some(30), samples: 1, slots: &[(\"foil-none\", 1.0000), (\"finish-plain\", 1.0000), (\"border-standard\", 1.0000), (\"table-tavern\", 1.0000), (\"field-none\", 1.0000), (\"card-back-standard\", 1.0000)], per_card_share: 0.0000, cards_per_deck_mean: 0.0000, card_picks: &[], earned_wear: &[], full_art_wear: &[(\"full-art-gary-the-gambler\", 0.0000)], ownership: &[CosmeticOwnership { cosmetic: \"foil-glitter\", card: Some(\"core-005\"), share: 1.0000 }, CosmeticOwnership { cosmetic: \"full-art-big-d-fender\", card: Some(\"core-001\"), share: 1.0000 }, CosmeticOwnership { cosmetic: \"full-art-gary-the-gambler\", card: Some(\"core-004\"), share: 1.0000 }] },\n\
         \x20   CosmeticFit { max_age_days: None, samples: 1, slots: &[(\"foil-none\", 1.0000), (\"finish-plain\", 1.0000), (\"border-standard\", 1.0000), (\"table-tavern\", 1.0000), (\"field-none\", 1.0000), (\"card-back-standard\", 1.0000)], per_card_share: 0.0000, cards_per_deck_mean: 0.0000, card_picks: &[], earned_wear: &[], full_art_wear: &[], ownership: &[] },\n"
    );
    // Aggregates only: no person's id, whole or in part, and no username field.
    for person in [ANNA, BORIS, CLEO] {
        assert!(!printed.contains(person), "{person} is in the output");
        assert!(!printed.contains(&person[..8]), "{person} is in the output");
    }
    assert!(
        !printed.contains("profile") && !printed.contains("username"),
        "{printed}"
    );

    // Nothing sampled prints the header alone.
    assert!(fit_cosmetics(registry(), &[], &BTreeMap::new()).is_empty());
    assert_eq!(
        render_cosmetic_fits(&[], BOT_COSMETIC_FIT_MIN_SAMPLES),
        "// jackioh-server cosmetics-fit: paste into BOT_COSMETIC_FITS, crates/server/src/config.rs\n"
    );
}

/// A row for accounts up to a week old, one seat short of the minimum, and one for accounts up to
/// a month old at it, as they would be pasted.
static PASTED: [CosmeticFit; 2] = [
    CosmeticFit {
        max_age_days: Some(7),
        samples: BOT_COSMETIC_FIT_MIN_SAMPLES - 1,
        slots: &[("table-felt", 1.0)],
        per_card_share: 0.5,
        cards_per_deck_mean: 2.0,
        card_picks: &[],
        earned_wear: &[],
        full_art_wear: &[],
        ownership: &[],
    },
    CosmeticFit {
        max_age_days: Some(30),
        samples: BOT_COSMETIC_FIT_MIN_SAMPLES,
        slots: &[("table-wood", 1.0)],
        per_card_share: 0.4,
        cards_per_deck_mean: 1.0,
        card_picks: &[],
        earned_wear: &[],
        full_art_wear: &[],
        ownership: &[],
    },
];

#[test]
fn r1834006_under_the_minimum_keeps_the_prior() {
    let rows = fit_cosmetics(registry(), &samples(), &owned());
    // With a minimum of two seats, the 7-day row prints and the two one-seat buckets say why not.
    let printed = render_cosmetic_fits(&rows, 2);
    let lines: Vec<&str> = printed.lines().collect();
    assert_eq!(lines.len(), 4, "{printed}");
    assert!(
        lines[1].starts_with("    CosmeticFit { max_age_days: Some(7), samples: 2,"),
        "{printed}"
    );
    assert_eq!(
        &lines[2..],
        [
            "    // accounts up to 30 days: 1 seats, under BOT_COSMETIC_FIT_MIN_SAMPLES; keep the prior",
            "    // accounts older than 90 days: 1 seats, under BOT_COSMETIC_FIT_MIN_SAMPLES; keep the prior",
        ]
    );
    // Under the real minimum, every bucket keeps the prior and no row is printed.
    let printed = render_cosmetic_fits(&rows, BOT_COSMETIC_FIT_MIN_SAMPLES);
    assert!(!printed.contains("CosmeticFit {"), "{printed}");
    assert_eq!(
        printed
            .lines()
            .filter(|line| line.ends_with("keep the prior"))
            .count(),
        3
    );

    // A pasted row under the minimum is passed over for the prior; one at it is drawn from; an age
    // no row covers draws from the prior, as every age does while `BOT_COSMETIC_FITS` is empty.
    assert_eq!(fit_for(&PASTED, 3), &BOT_COSMETIC_PRIOR);
    assert_eq!(fit_for(&PASTED, 20), &PASTED[1]);
    assert_eq!(fit_for(&PASTED, 60), &BOT_COSMETIC_PRIOR);
    assert_eq!(fit_for(&PASTED, 400), &BOT_COSMETIC_PRIOR);
    assert_eq!(fit_for(BOT_COSMETIC_FITS, 20), &BOT_COSMETIC_PRIOR);
}

#[tokio::test]
async fn cosmetics_fit_refuses_an_unknown_argument() {
    let refusal = cosmetics_fit::run(vec!["--min=3".to_string()]).await;
    let message = refusal.expect_err("an option it does not take").to_string();
    assert!(message.contains("Unrecognized argument \"--min=3\""), "{message}");
    assert!(
        message.contains("Usage: jackioh-server cosmetics-fit"),
        "{message}"
    );
}
