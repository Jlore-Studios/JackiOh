//! R1834005 (SPEC §14.2, R1454): ladder bots' cosmetics, on the server's store. The draw itself (the
//! wardrobe, the account's picks, the decks' slots, the cards' own picks, the aging and the floor) is
//! `jackioh-shared`'s, and its tests are `crates/shared/tests/bots_cosmetics.rs`; what is held here is that a
//! pool refresh reads the store, draws and writes a bot's cosmetics to its own row and nowhere else.

use jackioh_server::app::now_ms;
use jackioh_server::bots::cosmetics::{Holding, draw_wardrobe, eligible, fit_for};
use jackioh_server::bots::pool::refresh_pool;
use jackioh_server::config::{
    BOT_COSMETIC_FITS, BOT_COSMETIC_MIN_AGE_DAYS, BOT_COSMETIC_MIN_OWNERS, BOT_STRENGTHS, PersonaName,
};
use jackioh_server::cosmetics::{Cosmetic, CosmeticKind, registry};
use jackioh_server::db::store::{NewBot, ProfileCreateInput};
use jackioh_server::economy::store::{CosmeticGrant, EconomyLedger, EconomyReason, OwnedCosmetics};

use crate::support::deps::empty_test_app;
use crate::support::store::q;

/// Unit conversion, not configuration: a day in milliseconds.
const DAY: i64 = 86_400_000;
/// How many refresh seeds `r1834005_never_in_the_ledger` tries for one that gives its bot a
/// wardrobe; about one in four does.
const SEED_SEARCH: usize = 1_000;

/// The six full arts' holdings, each on its card.
fn full_arts() -> Vec<Holding> {
    registry()
        .iter()
        .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
        .map(|cosmetic| (cosmetic.id.clone(), cosmetic.card.clone()))
        .collect()
}

// Never in the ledger

/// The bot `r1834005_never_in_the_ledger` makes, by a known id so its refresh seed can be chosen.
const LEDGER_BOT: &str = "00000000-0000-4000-8000-000001834005";

#[tokio::test(start_paused = true)]
async fn r1834005_never_in_the_ledger() {
    let app = empty_test_app().await;
    let now = now_ms();

    // BOT_COSMETIC_MIN_OWNERS people own every full art on its card, granted a day past the floor.
    let granted_at = now - (BOT_COSMETIC_MIN_AGE_DAYS + 1) * DAY;
    let arts: Vec<&Cosmetic> = registry()
        .iter()
        .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
        .collect();
    for n in 0..BOT_COSMETIC_MIN_OWNERS {
        let input = ProfileCreateInput {
            user_id: format!("r1834005-person-{n}"),
            email: format!("r1834005-person-{n}@example.test"),
            rating: 1000.0,
            at: granted_at,
        };
        let person = q!(app, profiles_create(&input));
        for full_art in &arts {
            let grant = CosmeticGrant {
                profile_id: person.id.clone(),
                card_id: full_art.card.clone(),
                cosmetic_id: full_art.id.clone(),
                delta: 1,
                reason: EconomyReason::PackOpen,
                reference: None,
                dev: false,
                at: granted_at,
            };
            assert!(q!(app, economy_cosmetics_grant(&grant)));
        }
    }
    let owners = q!(app, economy_cosmetic_owners());
    let listed = eligible(registry(), &owners, now);
    assert_eq!(listed, {
        let mut all = full_arts();
        all.sort();
        all
    });

    // One bot of a known id, and a refresh seed that draws it a wardrobe of its own.
    let created_at = now - 40 * DAY;
    q!(
        app,
        bots_create(&NewBot {
            profile_id: LEDGER_BOT.to_string(),
            strength: 0,
            speed: 1.0,
            persona: PersonaName::Balanced,
            active_from_hour: 0,
            active_hours: 24,
            decks: Vec::new(),
            decks_catalog_version: None,
            created_at,
            seed_rating: BOT_STRENGTHS[0].seed_rating,
        })
    );
    let fit = fit_for(BOT_COSMETIC_FITS, (now - created_at) / DAY);
    let (seed, worn) = (0..SEED_SEARCH)
        .map(|n| format!("r1834005-ledger-{n}"))
        .find_map(|seed| {
            let drawn = draw_wardrobe(
                &format!("{seed}:{LEDGER_BOT}"),
                fit,
                &listed,
                &OwnedCosmetics::default(),
            );
            (!drawn.cards.is_empty()).then_some((seed, drawn))
        })
        .expect("a seed that draws the bot a wardrobe");

    let done = refresh_pool(&app, &seed, now).await.expect("a refresh");
    let bots = q!(app, bots_list());
    assert_eq!(done.dressed, bots.len(), "every active bot is dressed");
    let ours = bots
        .iter()
        .find(|bot| bot.profile_id == LEDGER_BOT)
        .expect("the bot");
    let cosmetics = ours.cosmetics.as_ref().expect("its cosmetics, on its row");
    assert_eq!(cosmetics.wardrobe, worn);
    assert_eq!(cosmetics.drawn_at, now);
    assert!(ours.decks.iter().all(|deck| deck.slots.is_some()));
    let (x, y, z) = &ours.trio.as_ref().expect("its trio").decks;
    assert!([x, y, z].into_iter().all(|deck| deck.slots.is_some()));

    // Its wardrobe and picks are on its row and nowhere else: no ledger row, no holding (what
    // `GET /api/cosmetics` reads), no card picks row, no collection, and no owner count of people.
    for bot in &bots {
        assert!(bot.cosmetics.is_some(), "{} is dressed", bot.profile_id);
        assert_eq!(
            q!(app, economy_cosmetics_get(&bot.profile_id)),
            OwnedCosmetics::default()
        );
        assert_eq!(
            q!(app, economy_ledger_get(&bot.profile_id)),
            EconomyLedger::default()
        );
        assert!(q!(app, economy_card_picks_get(&bot.profile_id)).is_empty());
        assert!(q!(app, collection_get(&bot.profile_id)).is_empty());
    }
    assert_eq!(q!(app, economy_cosmetic_owners()), owners);

    // A refresh with nothing due dresses nobody and changes nothing.
    let again = refresh_pool(&app, &format!("{seed}-again"), now)
        .await
        .expect("a refresh");
    assert_eq!(again.dressed, 0);
    assert_eq!(q!(app, bots_list()), bots);
}
