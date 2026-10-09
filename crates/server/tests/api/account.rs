//! `DELETE /api/account` (`src/api/auth.rs`): a player deletes their own account.
//!
//! The route is authenticated like every other (`user`, so a pending account can leave too), answers
//! 204 with no body when the account is gone, and the API's one error shape otherwise. What the
//! delete does to each table is `tests/sql/06_account_deletion.sql` and `tests/store/contract.rs`;
//! here: who may call it, what it refuses, and the order of its two deletes.
//!
//! Only the Supabase provider deletes users, so the route is driven through it against the scripted
//! GoTrue of `super::auth`; the "cannot delete" case goes through the fixture auth.

use std::sync::Arc;

use serde_json::{Value, json};

use jackioh_server::app::App;
use jackioh_server::auth::{Auth, AuthError, E2eDeletion};

use super::auth::{AdminReply, Captured, DeletionReply, Harness, confirmed_user, fake_of, raw, row};
use crate::support::deps::{call, test_app};

const PROFILE: &str = "p-leaving";
const OTHER: &str = "p-staying";

/// The suite's server on a scripted GoTrue, with two active accounts.
struct Setup {
    h: Harness,
    app: Arc<App>,
    token: String,
}

/// The GoTrue user id behind a profile: the admin client asks for a UUID, so the id is one,
/// derived from the profile id (FNV-1a) so each profile keeps its own.
fn user_of(id: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("00000000-0000-4000-8000-{:012x}", hash & 0xffff_ffff_ffff)
}

/// GoTrue's id for the first user.
const USER_A: &str = "aaaaaaaa-0000-4000-8000-000000000001";

/// A profile row seeded with the profile's user id and a token that verifies as it.
async fn profile_with(h: &Harness, app: &Arc<App>, id: &str, status: &str) -> String {
    let user_id = user_of(id);
    fake_of(app)
        .lock()
        .await
        .seed_profile(json!({ "id": id, "userId": user_id, "status": status }));
    h.token_for(&user_id)
}

async fn setup() -> Setup {
    let h = Harness::new().await;
    let app = h.app().await;
    let token = profile_with(&h, &app, PROFILE, "active").await;
    profile_with(&h, &app, OTHER, "active").await;
    Setup { h, app, token }
}

/// `DELETE /api/account` with `bearer`, answered with its status and its exact body.
async fn remove(app: &Arc<App>, bearer: Option<&str>) -> (u16, String) {
    raw(app, "DELETE", "/api/account", bearer, None).await
}

/// The `{ error: { code, message } }` of an answer.
fn error_of(body: &str) -> Value {
    serde_json::from_str::<Value>(body).expect("a JSON error")["error"].clone()
}

async fn profile_exists(app: &Arc<App>, profile_id: &str) -> bool {
    let mut tx = app.db.begin(None).await.expect("a transaction");
    let profile = tx.profiles_get_by_id(profile_id).await.expect("a read");
    tx.commit().await.expect("a commit");
    profile.is_some()
}

/// `DELETE /api/account`.
mod delete_api_account {
    use super::*;

    #[tokio::test]
    async fn is_declared_user_beside_the_other_account_routes() {
        // The declaration shows in what the route lets through: a pending account (which an
        // `active` route refuses with 403) is let in.
        let setup = setup().await;
        let pending = profile_with(&setup.h, &setup.app, "p-pending", "pending").await;
        let (status, _) = remove(&setup.app, Some(&pending)).await;
        assert_ne!(status, 403, "a `user` route lets a pending account in");
        assert_eq!(status, 204);
    }

    #[tokio::test]
    async fn answers_204_with_no_body_removes_the_profiles_own_rows_and_signs_the_token_out() {
        let setup = setup().await;
        let app = &setup.app;
        let log = Captured::default();
        let _guard = log.install();
        let at = setup.h.clock.now();
        let version = jackioh_cards::catalog_version();

        let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
        tx.decks_upsert(
            &row(json!({
                "id": "deck-1", "profileId": PROFILE, "name": "Mine", "cards": [], "portrait": null,
                "catalogVersion": version, "createdAt": at, "updatedAt": at,
            })),
            10,
        )
        .await
        .expect("a deck");
        tx.tutorial_merge(
            &row(json!({ "profileId": PROFILE, "completed": ["basics"], "hiddenChoice": null, "at": at })),
            32,
        )
        .await
        .expect("a lesson");
        tx.collection_upsert_quantities(PROFILE, &[row(json!({ "cardId": "core-001", "quantity": 1 }))])
            .await
            .expect("a card");
        tx.tickets_insert(&row(json!({
            "id": "ticket-1", "profileId": PROFILE, "rating": 1000, "mode": "bo1", "deck": [], "trio": null,
            "catalogVersion": version, "enqueuedAt": at, "status": "open", "matchId": null,
        })))
        .await
        .expect("a ticket");
        assert!(
            tx.rooms_create(&row(json!({
                "code": "ROOM22", "hostProfileId": PROFILE, "mode": "bo1", "hostDeck": [], "hostTrio": null,
                "catalogVersion": version, "createdAt": at, "expiresAt": at + 60_000,
                "guestProfileId": null, "matchId": null,
            })))
            .await
            .expect("a room"),
            "premise: the room code is free"
        );
        tx.commit().await.expect("a commit");

        let (status, body) = remove(app, Some(&setup.token)).await;
        assert_eq!(status, 204);
        assert_eq!(body, "");

        let mut tx = app.db.begin(None).await.expect("a transaction");
        assert!(tx.profiles_get_by_id(PROFILE).await.expect("a read").is_none());
        assert!(tx.decks_list(PROFILE).await.expect("a read").is_empty());
        assert!(tx.tutorial_get(PROFILE).await.expect("a read").is_none());
        assert!(tx.collection_get(PROFILE).await.expect("a read").is_empty());
        assert!(
            tx.tickets_open_for_profile(PROFILE)
                .await
                .expect("a read")
                .is_none()
        );
        assert!(tx.rooms_get("ROOM22").await.expect("a read").is_none());
        tx.commit().await.expect("a commit");
        assert!(app.auth.verify(&setup.token).await.is_err());

        // The same token is refused from now on, as any unknown token is.
        let (again, _) = remove(app, Some(&setup.token)).await;
        assert_eq!(again, 401);
        assert!(log.text().contains("account.deleted"));
    }

    #[tokio::test]
    async fn keeps_the_other_players_record_a_finished_match_they_lost_is_still_a_loss() {
        let setup = setup().await;
        let app = &setup.app;
        let mut tx = app.db.begin(None).await.expect("a transaction");
        tx.results_insert(&row(json!({
            "matchId": "match-1",
            "players": [PROFILE, OTHER],
            "winnerProfileId": PROFILE,
            "reason": "concede",
            "turns": 4,
            "endedAt": setup.h.clock.now(),
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1016, 984],
        })))
        .await
        .expect("a result");
        tx.commit().await.expect("a commit");

        assert_eq!(remove(app, Some(&setup.token)).await.0, 204);
        let mut tx = app.db.begin(None).await.expect("a transaction");
        let record = tx.results_record_for(OTHER).await.expect("a read");
        tx.commit().await.expect("a commit");
        assert_eq!((record.wins, record.losses, record.draws), (0, 1, 0));
    }

    #[tokio::test]
    async fn lets_a_pending_account_delete_itself_too() {
        let setup = setup().await;
        let pending = profile_with(&setup.h, &setup.app, "p-pending", "pending").await;
        assert_eq!(remove(&setup.app, Some(&pending)).await.0, 204);
        assert!(!profile_exists(&setup.app, "p-pending").await);
    }

    #[tokio::test]
    async fn refuses_a_caller_with_no_token_with_401_and_the_usual_error_shape() {
        let setup = setup().await;
        let (status, body) = remove(&setup.app, None).await;
        assert_eq!(status, 401);
        assert_eq!(error_of(&body)["code"], json!("unauthorized"));
    }

    #[tokio::test]
    async fn refuses_a_player_in_a_live_match_with_409_and_deletes_nothing() {
        let setup = setup().await;
        let app = &setup.app;
        let mut tx = app.db.begin(Some(PROFILE)).await.expect("a transaction");
        tx.profiles_set_in_match(PROFILE, Some("match-live"))
            .await
            .expect("a write");
        tx.commit().await.expect("a commit");

        let (status, body) = remove(app, Some(&setup.token)).await;
        assert_eq!(status, 409);
        let error = error_of(&body);
        assert_eq!(error["code"], json!("already_in_match"));
        assert!(
            error["message"]
                .as_str()
                .expect("a message")
                .to_lowercase()
                .contains("concede")
        );
        assert!(profile_exists(app, PROFILE).await);
        assert!(app.auth.verify(&setup.token).await.is_ok());
    }

    #[tokio::test]
    async fn refuses_a_player_in_a_conquest_series_that_is_not_over_with_409_and_deletes_nothing() {
        let setup = setup().await;
        let app = &setup.app;
        let trio = json!({
            "name": "t",
            "decks": [{ "name": "a", "cards": [] }, { "name": "b", "cards": [] }, { "name": "c", "cards": [] }],
        });
        let mut tx = app.db.begin(None).await.expect("a transaction");
        tx.series_create(&row(json!({
            "id": "series-1",
            "sides": [
                { "profileId": PROFILE, "trio": trio, "wins": 0, "pick": null },
                { "profileId": OTHER, "trio": trio, "wins": 0, "pick": null },
            ],
            "catalogVersion": jackioh_cards::catalog_version(),
            "ranked": false,
            "seedBase": "s",
            "status": "picking",
            "games": [],
            "nextMatchId": "reserved",
            "pickDeadline": null,
            "winner": null,
            "endReason": null,
            "ratingBefore": null,
            "ratingAfter": null,
            "createdAt": 0,
            "updatedAt": 0,
            "endedAt": null,
            "version": 1,
        })))
        .await
        .expect("a series");
        tx.commit().await.expect("a commit");

        let (status, body) = remove(app, Some(&setup.token)).await;
        assert_eq!(status, 409);
        assert_eq!(error_of(&body)["code"], json!("conflict"));
        assert!(profile_exists(app, PROFILE).await);
    }

    #[tokio::test]
    async fn answers_503_on_a_server_whose_auth_provider_cannot_delete_users_and_deletes_nothing() {
        // The E2E fixture auth is that server: it has no way to remove an account.
        // `support::deps::test_app()` switches deletion on, so it is switched back off here.
        let app = test_app().await;
        let Auth::E2e(fixtures) = &app.auth else {
            panic!("support::deps::test_app() runs on the fixture auth");
        };
        fixtures.set_deletion(E2eDeletion::Unsupported);
        let (status, _, body) = call(&app, "DELETE", "/api/account", Some("e2e-token-p1"), Value::Null).await;
        assert_eq!(status, 503);
        assert_eq!(body["error"]["code"], json!("unavailable"));
        let mut tx = app.db.begin(None).await.expect("a transaction");
        assert!(
            tx.profiles_get_by_user_id("e2e-p1")
                .await
                .expect("a read")
                .is_some()
        );
        tx.commit().await.expect("a commit");
    }

    #[tokio::test]
    async fn can_be_retried_when_the_provider_fails_after_the_profile_is_gone() {
        let setup = setup().await;
        let app = &setup.app;
        setup.h.gotrue.answer_deletion(DeletionReply::Unavailable);
        let (failed, _) = remove(app, Some(&setup.token)).await;
        assert_eq!(failed, 503);
        assert!(!profile_exists(app, PROFILE).await);

        // The sign-in still works, so the next call gets a fresh pending profile and removes that.
        setup.h.gotrue.answer_deletion(DeletionReply::Deleted);
        assert_eq!(remove(app, Some(&setup.token)).await.0, 204);
        let mut tx = app.db.begin(None).await.expect("a transaction");
        assert!(
            tx.profiles_get_by_user_id(&user_of(PROFILE))
                .await
                .expect("a read")
                .is_none()
        );
        tx.commit().await.expect("a commit");
        assert!(app.auth.verify(&setup.token).await.is_err());
    }
}

/// The Supabase provider's `delete_user`.
mod the_supabase_providers_delete_user {
    use super::*;

    /// The admin lookup answers `USER_A`, confirmed.
    async fn provider(deletion: DeletionReply) -> Harness {
        let h = Harness::new().await;
        h.gotrue
            .answer_admin(|user_id| AdminReply::Ok(confirmed_user(user_id)));
        h.gotrue.answer_deletion(deletion);
        h
    }

    #[tokio::test]
    async fn deletes_through_the_admin_api_and_stops_honouring_the_users_token_at_once() {
        let h = provider(DeletionReply::Deleted).await;
        let token = h.token_for(USER_A);
        // Verified once, so the confirmed email is remembered for a while.
        assert!(
            h.auth
                .verify(&token)
                .await
                .expect("user-a verifies")
                .email_verified
        );

        h.auth.delete_user(USER_A).await.expect("deleted");
        assert_eq!(h.gotrue.deleted(), vec![USER_A.to_string()]);
        // GoTrue now answers 404 for the user. Without forgetting the remembered email this would
        // still verify until the memory lapsed (the clock has not moved).
        assert!(h.auth.verify(&token).await.is_err());
    }

    #[tokio::test]
    async fn counts_a_user_that_is_already_gone_as_deleted() {
        let h = provider(DeletionReply::Missing).await;
        assert!(h.auth.delete_user(USER_A).await.is_ok());
    }

    #[tokio::test]
    async fn throws_unavailable_when_the_provider_does_not_answer() {
        let h = provider(DeletionReply::Unavailable).await;
        let refused = h.auth.delete_user(USER_A).await;
        assert!(matches!(refused, Err(AuthError::Unavailable { .. })));
    }
}
