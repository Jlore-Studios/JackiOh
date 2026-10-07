//! R738 — the opponent's aim through the actor (§9.5): a top-level `aim` frame, never an
//! `ActionBody`, shape-checked by `protocol.ts`, relayed to the opponent alone, coalesced to one
//! relay per `AIM_RELAY_INTERVAL_MS` per seat, and dropped when an end names anything the opponent
//! may not see.
//!
//! Runs on the scripted engine port (`test/fakes/engine.ts`), whose `viewFor` shows the opponent's
//! hand as a count — the one fact the hidden-information check reads besides the zones.
//!
//! Port of `apps/server/test/match/aim.test.ts`. The Rust server has no engine port to script:
//! `support::engine`'s test cards run as real engine scripts under the testkit's thread-local
//! override (SURFACE §8, §11.2), so the views here are the real `view_for`'s, which shows the
//! opponent's hand as a count just as the fake did. The registry, the clock and the results writer
//! are the test app's own (`support::deps::test_app`), tokio's paused clock stands in for the manual
//! timers, and every frame is read as the JSON the wire carries (SURFACE §5.1).

use std::sync::Arc;
use std::time::Duration;

use jackioh_engine::{Aim, PlayerId, PlayerView};
use jackioh_server::actor::match_actor::{MatchActor, aim_is_public};
use jackioh_server::actor::protocol::{ClientMessage, parse_client_message};
use jackioh_server::actor::ws_server::Socket;
use jackioh_server::app::App;
use jackioh_server::config::AIM_RELAY_INTERVAL_MS;
use jackioh_server::db::store::StartMatchInput;
use serde_json::{Value, json};

use crate::support::deps::test_app;
use crate::support::engine::{fake_deck, install_test_cards};
use crate::support::socket::{FakeSocket, create_fake_socket};

const MATCH_ID: &str = "match-aim";

/// TS `TEST_CATALOG_VERSION` (`test/fakes/deps.ts`).
const TEST_CATALOG_VERSION: &str = "test-1";

/// How many times `settle` yields: enough for every task woken at this instant to run.
const SETTLE_YIELDS: usize = 8;

/// A config number as milliseconds' arithmetic wants it, whatever integer type `config.rs` gives it.
fn int(value: impl Into<i64>) -> i64 {
    value.into()
}

/// `AIM_RELAY_INTERVAL_MS`, in the milliseconds the tests count in.
fn aim_interval() -> i64 {
    int(AIM_RELAY_INTERVAL_MS)
}

/// Lets every task woken at this instant run (the actor, its aim timers, the sockets' channels).
async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// TS `deps.timers.advance(ms)`: sleeps on the paused clock, so every timer due by then fires in
/// deadline order, then lets what it woke run.
async fn advance(ms: i64) {
    tokio::time::sleep(Duration::from_millis(u64::try_from(ms.max(0)).unwrap_or(0))).await;
    settle().await;
}

/// One client's end of a fake socket (`support::socket`), read as the JSON frames the server sent.
/// Every call this file makes on the fake goes through here.
struct Client(FakeSocket);

impl Client {
    fn new() -> Client {
        Client(create_fake_socket())
    }

    /// The half the actor holds. TS handed the fake itself to `attach`; Rust's `Socket` is a struct.
    fn socket(&self) -> Socket {
        self.0.socket()
    }

    /// Every frame the server sent, parsed, in order.
    fn messages(&self) -> Vec<Value> {
        self.0.sent().iter().map(|text| serde_json::from_str(text).expect("every frame is JSON")).collect()
    }

    /// Frames of one `type`, parsed.
    fn of_type(&self, kind: &str) -> Vec<Value> {
        self.messages().into_iter().filter(|message| message["type"] == kind).collect()
    }

    /// Simulate the client sending JSON.
    fn receive_json(&self, value: Value) {
        self.0.receive(&value.to_string());
    }

    /// Simulate the transport dropping.
    fn drop_transport(&self) {
        self.0.drop();
    }

    fn clear(&self) {
        self.0.clear();
    }
}

struct Harness {
    app: Arc<App>,
    actor: MatchActor,
    p1: Client,
    p2: Client,
}

async fn harness() -> Harness {
    // The scripted cards, as real engine scripts under this thread's testkit override.
    install_test_cards();
    let app = test_app().await;
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": MATCH_ID,
        "seed": "seed-aim",
        "catalogVersion": TEST_CATALOG_VERSION,
        "ranked": false,
        "seats": [
            { "profileId": "profile-1", "player": "p1", "deck": fake_deck(&["test-prompt-self"]) },
            { "profileId": "profile-2", "player": "p2", "deck": fake_deck(&["test-prompt-enemy"]) },
        ],
    }))
    .expect("a StartMatchInput");
    app.matches.start(&app, input).await.expect("registry.start");
    let actor = app.matches.actor_for(&app, MATCH_ID).await.expect("the actor");
    let p1 = Client::new();
    let p2 = Client::new();
    actor.attach(PlayerId::P1, p1.socket());
    actor.attach(PlayerId::P2, p2.socket());
    actor.idle().await;
    settle().await;
    p1.clear();
    p2.clear();
    Harness { app, actor, p1, p2 }
}

impl Harness {
    /// TS `actor.idle()`: the frames sent so far have been handled and what they pushed has arrived.
    async fn idle(&self) {
        settle().await;
        self.actor.idle().await;
        settle().await;
    }
}

/// TS `deps.store.tables.matchActions`, for this match: the rows `match_actions` holds, as JSON.
async fn match_actions(app: &App, match_id: &str) -> Vec<Value> {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let rows = tx.matches_actions(match_id).await.expect("matches.actions");
    tx.commit().await.expect("commit");
    rows.iter().map(|row| serde_json::to_value(row).expect("MatchActionRow serialises")).collect()
}

fn relays(socket: &Client) -> Vec<Value> {
    socket.of_type("aim")
}

fn relayed_aims(socket: &Client) -> Vec<Value> {
    relays(socket).into_iter().map(|frame| frame["aim"].clone()).collect()
}

fn errors(socket: &Client) -> Vec<Value> {
    socket.of_type("error")
}

fn hand_count(view: &Value, side: &str) -> i64 {
    let hand = &view[side]["hand"];
    match hand.as_array() {
        Some(cards) => i64::try_from(cards.len()).expect("a hand fits an i64"),
        None => hand["count"].as_i64().expect("a hand is a list or a count"),
    }
}

fn at_hero() -> Value {
    json!({
        "source": { "at": "zone", "player": "p1", "row": "units", "lane": 2 },
        "target": { "at": "hero", "player": "p2" },
    })
}

mod r738_the_opponents_aim_through_the_actor_9_5 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r738_relays_an_aim_to_the_opponent_alone_stamped_with_the_senders_seat_and_writes_nothing() {
        let h = harness().await;
        h.p1.receive_json(json!({ "type": "aim", "aim": at_hero() }));
        h.idle().await;

        assert_eq!(relays(&h.p2), vec![json!({ "type": "aim", "from": "p1", "aim": at_hero() })]);
        assert_eq!(relays(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(h.p1.of_type("ack"), Vec::<Value>::new());
        assert_eq!(match_actions(&h.app, MATCH_ID).await, Vec::<Value>::new());
        // Never part of the view: no view frame went to either seat for it.
        assert_eq!(h.p1.of_type("view"), Vec::<Value>::new());
        assert_eq!(h.p2.of_type("view"), Vec::<Value>::new());

        advance(aim_interval()).await;
        h.p1.receive_json(json!({ "type": "aim", "aim": null }));
        h.idle().await;
        assert_eq!(relays(&h.p2).last(), Some(&json!({ "type": "aim", "from": "p1", "aim": null })));
    }

    #[tokio::test(start_paused = true)]
    async fn r738_shape_checks_the_frame_anything_but_public_handles_is_malformed_and_relays_nothing() {
        let h = harness().await;
        for aim in [
            json!({ "source": { "at": "card", "instanceId": "p1-h0" }, "target": null }),
            json!({ "source": { "at": "zone", "player": "p1", "row": "units", "lane": 0 }, "target": null }),
            json!({ "source": { "at": "hand", "player": "p1", "index": -1 }, "target": null }),
            json!({ "source": { "at": "hero", "player": "p3" }, "target": null }),
            json!({ "source": { "at": "hero", "player": "p1" }, "target": { "at": "hand", "player": "p2", "index": 0 } }),
            json!({ "source": { "at": "hero", "player": "p1" } }),
            json!("aim"),
        ] {
            h.p1.receive_json(json!({ "type": "aim", "aim": aim }));
        }
        h.p1.receive_json(json!({ "type": "aim" }));
        h.idle().await;
        let codes: Vec<Value> = errors(&h.p1).into_iter().map(|error| error["code"].clone()).collect();
        assert_eq!(codes, vec![json!("malformed"); 8]);
        assert_eq!(relays(&h.p2), Vec::<Value>::new());
    }

    #[test]
    fn r738_keeps_only_the_handles_fields_a_smuggled_instance_id_or_def_id_never_reaches_the_opponent() {
        let parsed = parse_client_message(
            &json!({
                "type": "aim",
                "aim": {
                    "source": { "at": "hand", "player": "p1", "index": 0, "instanceId": "p1-h0", "defId": "core-001" },
                    "target": { "at": "zone", "player": "p2", "row": "backrow", "lane": 1, "instanceId": "p2-b1" },
                    "extra": true,
                },
            })
            .to_string(),
        );
        let ClientMessage::Aim { aim } = parsed else { panic!("the frame did not parse as an aim") };
        assert_eq!(
            serde_json::to_value(&aim).expect("Aim serialises"),
            json!({
                "source": { "at": "hand", "player": "p1", "index": 0 },
                "target": { "at": "zone", "player": "p2", "row": "backrow", "lane": 1 },
            })
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r738_coalesces_aims_inside_aim_relay_interval_ms_the_newest_goes_alone_once_the_interval_ends() {
        let h = harness().await;
        let mut second = at_hero();
        second["target"] = json!({ "at": "zone", "player": "p2", "row": "units", "lane": 3 });
        let mut third = at_hero();
        third["target"] = json!({ "at": "zone", "player": "p2", "row": "backrow", "lane": 4 });

        h.p1.receive_json(json!({ "type": "aim", "aim": at_hero() }));
        h.p1.receive_json(json!({ "type": "aim", "aim": second }));
        h.p1.receive_json(json!({ "type": "aim", "aim": third }));
        h.idle().await;
        assert_eq!(relayed_aims(&h.p2), vec![at_hero()]);

        advance(aim_interval() - 1).await;
        assert_eq!(relays(&h.p2).len(), 1);
        advance(1).await;
        assert_eq!(relayed_aims(&h.p2), vec![at_hero(), third.clone()]);

        // A repeat of what the opponent was last told is not sent again.
        advance(aim_interval()).await;
        h.p1.receive_json(json!({ "type": "aim", "aim": third }));
        h.idle().await;
        assert_eq!(relays(&h.p2).len(), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn r738_keeps_a_window_per_seat_one_seats_aims_never_spend_the_others() {
        let h = harness().await;
        let from_p2 = json!({ "source": { "at": "hero", "player": "p2" }, "target": { "at": "hero", "player": "p1" } });
        h.p1.receive_json(json!({ "type": "aim", "aim": at_hero() }));
        h.p2.receive_json(json!({ "type": "aim", "aim": from_p2 }));
        h.idle().await;
        assert_eq!(relays(&h.p2).len(), 1);
        assert_eq!(relays(&h.p1), vec![json!({ "type": "aim", "from": "p2", "aim": from_p2 })]);
    }

    #[tokio::test(start_paused = true)]
    async fn r738_drops_an_aim_that_names_a_hand_card_past_the_senders_hand_or_the_opponents_hand_at_all() {
        let h = harness().await;
        let p2_view = serde_json::to_value(h.actor.view_for(PlayerId::P2)).expect("PlayerView serialises");
        let count = hand_count(&p2_view, "opponent");
        assert!(count > 0);

        let from_hand = |player: &str, index: i64| {
            json!({ "source": { "at": "hand", "player": player, "index": index }, "target": { "at": "hero", "player": "p2" } })
        };
        h.p1.receive_json(json!({ "type": "aim", "aim": from_hand("p1", count) }));
        h.idle().await;
        advance(aim_interval()).await;
        h.p1.receive_json(json!({ "type": "aim", "aim": from_hand("p2", 0) }));
        h.idle().await;
        // Silently: no relay, no error.
        assert_eq!(relays(&h.p2), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());

        advance(aim_interval()).await;
        h.p1.receive_json(json!({ "type": "aim", "aim": from_hand("p1", count - 1) }));
        h.idle().await;
        assert_eq!(relayed_aims(&h.p2), vec![from_hand("p1", count - 1)]);

        // An aim dropped while an arrow is up clears that arrow instead of leaving it standing.
        advance(aim_interval()).await;
        h.p1.receive_json(json!({ "type": "aim", "aim": from_hand("p1", count) }));
        h.idle().await;
        assert_eq!(relayed_aims(&h.p2), vec![from_hand("p1", count - 1), Value::Null]);
    }

    /// A real `PlayerView` for `viewer`, its two hands and its lanes cut down to the ones the TS
    /// test's literal named (`{ viewer, you: { hand, locks }, opponent: { hand, locks } }`): Rust has no
    /// `as unknown as PlayerView`, so the rest of the view is a real deal's, from the real catalog.
    fn cut_down_view(viewer: PlayerId) -> PlayerView {
        jackioh_cards::register_all();
        let pool: Vec<String> = jackioh_cards::CATALOG
            .iter()
            .filter(|(_, def)| !(def.token || def.tags.iter().any(|tag| tag.as_str() == "Token")))
            .map(|(id, _)| id.clone())
            .take(40)
            .collect();
        let decks = (pool[..20].to_vec(), pool[20..40].to_vec());
        let state = jackioh_engine::create_game(&jackioh_engine::CreateGameArgs {
            seed: "r738-lanes".to_string(),
            decks,
            ..jackioh_engine::CreateGameArgs::default()
        });
        let begun = jackioh_engine::begin_game(&state).state;
        let mut view = serde_json::to_value(jackioh_engine::view_for(&begun, viewer)).expect("PlayerView serialises");
        let locks = json!({ "units": [false, false], "backrow": [false] });
        view["viewer"] = json!(viewer.as_str());
        view["you"]["hand"] = json!([]);
        view["you"]["locks"] = locks.clone();
        view["opponent"]["hand"] = json!({ "count": 2 });
        view["opponent"]["locks"] = locks;
        serde_json::from_value(view).expect("the cut-down view is still a PlayerView")
    }

    #[test]
    fn r738_drops_an_aim_at_a_zone_past_the_boards_lanes() {
        let view = cut_down_view(PlayerId::P2);
        let aim = |value: Value| -> Aim { serde_json::from_value(value).expect("an Aim") };
        let at = |lane: i64| {
            aim(json!({ "source": { "at": "hero", "player": "p1" }, "target": { "at": "zone", "player": "p2", "row": "backrow", "lane": lane } }))
        };
        assert!(aim_is_public(&at(1), PlayerId::P1, &view));
        assert!(!aim_is_public(&at(2), PlayerId::P1, &view));
        assert!(aim_is_public(
            &aim(json!({ "source": { "at": "hand", "player": "p1", "index": 1 }, "target": null })),
            PlayerId::P1,
            &view
        ));
        assert!(!aim_is_public(
            &aim(json!({ "source": { "at": "hand", "player": "p1", "index": 2 }, "target": null })),
            PlayerId::P1,
            &view
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn r738_clears_the_senders_arrow_when_its_socket_goes() {
        let h = harness().await;
        h.p1.receive_json(json!({ "type": "aim", "aim": at_hero() }));
        h.idle().await;
        h.p1.drop_transport();
        h.idle().await;
        assert_eq!(relayed_aims(&h.p2), vec![at_hero(), Value::Null]);
    }
}
