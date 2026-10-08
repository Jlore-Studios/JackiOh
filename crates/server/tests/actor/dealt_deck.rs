//! R433 on the server: an All Random match (R258) deals both seats' decks, so neither player is shown
//! theirs going in, and each seat's own library lists every card as unknown until it leaves (R312).
//! The registry reads the match's mode off what made it (`matches.modeOf`) and tells `createGame` and
//! every rebuild's `fold` both seats were dealt; a Best-of-1 match's decks were built and list in full
//! (R310). The real engine port under the real registry, actor and results writer, over the in-memory
//! store.
//!
//! Port of `apps/server/test/match/dealt-deck.test.ts`. In Rust there is no engine port to inject
//! and no actor deps to assemble: the registry is the test app's own (`App.matches`), over the fake
//! store, and it reaches the engine through `actor::engine`'s functions, which call
//! `jackioh_engine` directly (SURFACE §11.3).

use std::sync::Arc;

use jackioh_engine::{PLAYER_IDS, PlayerId};
use jackioh_server::actor::engine::deal_random_deck;
use jackioh_server::app::App;
use jackioh_server::db::store::{Db, StartMatchInput, Ticket};
use serde_json::{Value, json};

use crate::support::deps::test_app;

const P1: &str = "profile-1";
const P2: &str = "profile-2";

/// TS `TEST_CATALOG_VERSION` (`test/fakes/deps.ts`): the catalog version the test matches carry.
const TEST_CATALOG_VERSION: &str = "test-1";

/// The test app over the fake store, both profiles seeded, the real catalog registered.
async fn world() -> Arc<App> {
    jackioh_cards::register_all();
    let app = test_app().await;
    seed_profile(&app, json!({ "id": P1, "rating": 1000 })).await;
    seed_profile(&app, json!({ "id": P2, "rating": 1000 })).await;
    app
}

/// TS `deps.store.seedProfile(...)`: a profile row written straight into the fake store.
async fn seed_profile(app: &App, profile: Value) {
    let Db::Fake(data) = &app.db else {
        panic!("the test app runs on the fake store")
    };
    data.lock().await.seed_profile(profile);
}

/// The two queue tickets a match in `mode` was paired from (§9.5): what tells the registry its mode.
async fn paired_tickets(app: &App, match_id: &str, mode: &str) {
    let Db::Fake(data) = &app.db else {
        panic!("the test app runs on the fake store")
    };
    let mut data = data.lock().await;
    for profile_id in [P1, P2] {
        let ticket: Ticket = serde_json::from_value(json!({
            "id": format!("ticket-{match_id}-{profile_id}"),
            "profileId": profile_id,
            "rating": 1000,
            "mode": mode,
            "deck": [],
            "trio": null,
            "catalogVersion": TEST_CATALOG_VERSION,
            "enqueuedAt": 0,
            "status": "matched",
            "matchId": match_id,
        }))
        .expect("a Ticket");
        data.tables.tickets.push(ticket);
    }
}

/// Starts a match on two decks R258's deal gives for `seed`, as the queue and the rooms deal them.
async fn start(app: &Arc<App>, match_id: &str, seed: &str) {
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": match_id,
        "seed": seed,
        "catalogVersion": TEST_CATALOG_VERSION,
        "ranked": false,
        "seats": [
            { "profileId": P1, "player": "p1", "deck": deal_random_deck(&format!("{seed}:p1-deck"), None) },
            { "profileId": P2, "player": "p2", "deck": deal_random_deck(&format!("{seed}:p2-deck"), None) },
        ],
    }))
    .expect("a StartMatchInput");
    app.matches.start(app, input).await.expect("registry.start");
}

/// A seat's view of the live actor, as its JSON (what a socket would carry, §10.8).
async fn view_of(app: &Arc<App>, match_id: &str, player: PlayerId) -> Value {
    let actor = app.matches.actor_for(app, match_id).await.expect("the actor");
    serde_json::to_value(actor.view_for(player)).expect("PlayerView serialises")
}

/// How many cards the seat's own library list names (R310), the unknown ones aside.
fn listed(view: &Value) -> i64 {
    view["you"]["ownLibrary"]["cards"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .map(|entry| entry["count"].as_i64().unwrap_or(0))
                .sum()
        })
        .unwrap_or(0)
}

fn library_count(view: &Value) -> i64 {
    view["you"]["libraryCount"]
        .as_i64()
        .expect("libraryCount is a number")
}

mod r433_a_dealt_deck_lists_only_the_cards_its_owner_has_been_shown {
    use super::*;

    #[tokio::test]
    async fn r433_an_all_random_match_lists_neither_seats_starting_library_and_a_rebuilt_actor_folds_the_same_game()
     {
        let app = world().await;
        paired_tickets(&app, "m-random", "random").await;
        start(&app, "m-random", "r433-random").await;
        let actor = app.matches.actor_for(&app, "m-random").await.expect("the actor");

        for player in PLAYER_IDS {
            let view = serde_json::to_value(actor.view_for(player)).expect("PlayerView serialises");
            assert!(library_count(&view) > 0);
            assert_eq!(
                view["you"]["ownLibrary"],
                json!({ "cards": [], "unknown": library_count(&view) })
            );
        }

        let live = jackioh_engine::hash_state(&actor.engine_state());
        app.matches.stop("m-random").await;
        let rebuilt = app
            .matches
            .actor_for(&app, "m-random")
            .await
            .expect("the rebuilt actor");
        assert_eq!(jackioh_engine::hash_state(&rebuilt.engine_state()), live);
        for player in PLAYER_IDS {
            let view = serde_json::to_value(rebuilt.view_for(player)).expect("PlayerView serialises");
            assert_eq!(
                view["you"]["ownLibrary"],
                json!({ "cards": [], "unknown": library_count(&view) })
            );
        }
    }

    #[tokio::test]
    async fn r433_r310_a_best_of_1_matchs_decks_were_built_each_seats_own_library_is_listed_in_full() {
        let app = world().await;
        paired_tickets(&app, "m-bo1", "bo1").await;
        start(&app, "m-bo1", "r433-bo1").await;

        for player in PLAYER_IDS {
            let view = view_of(&app, "m-bo1", player).await;
            assert!(library_count(&view) > 0);
            assert_eq!(view["you"]["ownLibrary"]["unknown"].as_i64(), Some(0));
            assert_eq!(listed(&view), library_count(&view));
        }
    }

    #[tokio::test]
    async fn r433_a_match_nothing_a_mode_can_be_read_off_made_is_not_dealt() {
        let app = world().await;
        start(&app, "m-none", "r433-none").await;

        for player in PLAYER_IDS {
            let view = view_of(&app, "m-none", player).await;
            assert!(library_count(&view) > 0);
            assert_eq!(view["you"]["ownLibrary"]["unknown"].as_i64(), Some(0));
            assert_eq!(listed(&view), library_count(&view));
        }
    }
}
