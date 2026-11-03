//! R2739001: the opponent's choice hover through the actor (§9.5), R738's aim relay copied onto the
//! option of an open card picker the other seat is shown a count for (R2739000).
//! Surface contract: docs/v0.3.0/SURFACE.md §5.1, §8, §11.2.

use std::sync::Arc;

use jackioh_engine::{ActionBody, PendingElsewhereView, PendingView, PlayerId, PlayerView};
use jackioh_server::actor::match_actor::{MatchActor, hover_is_shown};
use jackioh_server::actor::ws_server::Socket;
use jackioh_server::app::App;
use jackioh_server::config::AIM_RELAY_INTERVAL_MS;
use jackioh_server::db::store::StartMatchInput;
use serde_json::{Value, json};

use crate::support::deps::{TEST_CATALOG_VERSION, test_app};
use crate::support::engine::{TEST_PROMPT_SELF, begun_game, fake_deck, install_test_cards};
use crate::support::socket::{FakeSocket, create_fake_socket};
use crate::support::store::sleep_settled;

const MATCH_ID: &str = "match-choice-hover";

/// How many times `settle` yields: enough for every task woken at this instant to run.
const SETTLE_YIELDS: usize = 8;

/// How many options `TEST_PROMPT_SELF`'s Choose One offers (`support::engine::PROMPT_OPTIONS`), each
/// a mode, asked by a Spell resolving face up: a count the other seat is shown (R2739000).
const OPTION_COUNT: i64 = 2;

/// `AIM_RELAY_INTERVAL_MS`, in the milliseconds the tests count in.
fn hover_interval() -> i64 {
    AIM_RELAY_INTERVAL_MS
}

/// Lets every task woken at this instant run (the actor, its relay timers, the sockets' channels).
async fn settle() {
    for _ in 0..SETTLE_YIELDS {
        tokio::task::yield_now().await;
    }
}

/// One client's end of a fake socket (`support::socket`), read as the JSON frames the server sent.
struct Client(FakeSocket);

impl Client {
    fn new() -> Client {
        Client(create_fake_socket())
    }

    /// The half the actor holds.
    fn socket(&self) -> Socket {
        self.0.socket()
    }

    /// Frames of one `type`, parsed, in order.
    fn of_type(&self, kind: &str) -> Vec<Value> {
        self.0
            .sent()
            .iter()
            .map(|text| serde_json::from_str::<Value>(text).expect("every frame is JSON"))
            .filter(|message| message["type"] == kind)
            .collect()
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

/// One action through `submit`, which must be acked.
async fn submit_ok(actor: &MatchActor, player: PlayerId, nonce: &str, body: Value) {
    let body: ActionBody = serde_json::from_value(body).expect("an ActionBody");
    let reply = serde_json::to_value(actor.submit(player, nonce.to_string(), body).await)
        .expect("ServerMessage serializes");
    assert_eq!(reply["type"], json!("ack"), "{nonce} was refused: {reply}");
}

fn view_json(actor: &MatchActor, player: PlayerId) -> Value {
    serde_json::to_value(actor.view_for(player)).expect("PlayerView serializes")
}

/// The instance ids of the viewer's own hand, in hand order.
fn hand_ids(view: &Value) -> Vec<Value> {
    view["you"]["hand"]
        .as_array()
        .expect("the viewer's own hand is a list")
        .iter()
        .map(|card| card["instanceId"].clone())
        .collect()
}

/// p1's turn 1, both sockets attached and their frames cleared, as `walk_setup` walks it: no auto end
/// of turn, and both mulligans keeping the whole hand.
async fn harness() -> Harness {
    install_test_cards();
    let app = test_app().await;
    let input: StartMatchInput = serde_json::from_value(json!({
        "matchId": MATCH_ID,
        "seed": "seed-choice-hover",
        "catalogVersion": TEST_CATALOG_VERSION,
        "ranked": false,
        "seats": [
            { "profileId": "profile-1", "player": "p1", "deck": fake_deck(&[TEST_PROMPT_SELF]) },
            { "profileId": "profile-2", "player": "p2", "deck": fake_deck(&[]) },
        ],
    }))
    .expect("a StartMatchInput");
    app.matches.start(&app, input).await.expect("registry.start");
    let actor = app.matches.actor_for(&app, MATCH_ID).await.expect("the actor");
    actor.idle().await;
    for player in [PlayerId::P1, PlayerId::P2] {
        submit_ok(
            &actor,
            player,
            &format!("setup-auto-{player}"),
            json!({ "type": "setAutoEndTurn", "enabled": false }),
        )
        .await;
    }
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = hand_ids(&view_json(&actor, player));
        submit_ok(
            &actor,
            player,
            &format!("setup-mulligan-{player}"),
            json!({ "type": "mulligan", "keep": keep }),
        )
        .await;
    }
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
    /// The frames sent so far have been handled and what they pushed has arrived.
    async fn idle(&self) {
        settle().await;
        self.actor.idle().await;
        settle().await;
    }

    /// p1 plays `TEST_PROMPT_SELF`, opening its Choose One, which p2 is shown `OPTION_COUNT` backs of;
    /// the frames the play pushed are cleared.
    async fn open_the_prompt(&self) {
        let card = view_json(&self.actor, PlayerId::P1)["you"]["hand"]
            .as_array()
            .expect("p1's own hand is a list")
            .iter()
            .find(|card| card["defId"] == TEST_PROMPT_SELF)
            .map(|card| card["instanceId"].clone())
            .expect("the prompt card is in p1's opening hand");
        submit_ok(
            &self.actor,
            PlayerId::P1,
            "play-prompt",
            json!({ "type": "play", "instanceId": card }),
        )
        .await;
        self.idle().await;
        assert_eq!(
            view_json(&self.actor, PlayerId::P2)["pending"],
            json!({ "forYou": false, "pendingFor": "p1", "optionCount": OPTION_COUNT })
        );
        self.p1.clear();
        self.p2.clear();
    }

    fn hover(&self, socket: &Client, index: Value) {
        socket.receive_json(json!({ "type": "choiceHover", "index": index }));
    }
}

/// The rows `match_actions` holds for this match.
async fn match_actions(app: &App) -> usize {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let rows = tx.matches_actions(MATCH_ID).await.expect("matches.actions");
    tx.commit().await.expect("commit");
    rows.len()
}

fn relays(socket: &Client) -> Vec<Value> {
    socket.of_type("choiceHover")
}

fn relayed_indexes(socket: &Client) -> Vec<Value> {
    relays(socket)
        .into_iter()
        .map(|frame| frame["index"].clone())
        .collect()
}

fn errors(socket: &Client) -> Vec<Value> {
    socket.of_type("error")
}

mod r2739001_the_opponents_choice_hover_9_5 {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r2739001_relays_a_hover_to_the_opponent_alone() {
        let h = harness().await;
        h.open_the_prompt().await;
        let rows = match_actions(&h.app).await;

        h.hover(&h.p1, json!(1));
        h.idle().await;

        assert_eq!(
            relays(&h.p2),
            vec![json!({ "type": "choiceHover", "from": "p1", "index": 1 })]
        );
        assert_eq!(relays(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(h.p1.of_type("ack"), Vec::<Value>::new());
        assert_eq!(h.p1.of_type("view"), Vec::<Value>::new());
        assert_eq!(h.p2.of_type("view"), Vec::<Value>::new());
        assert_eq!(match_actions(&h.app).await, rows);
    }

    #[tokio::test(start_paused = true)]
    async fn r2739001_coalesces_hovers_inside_aim_relay_interval_ms() {
        let h = harness().await;
        h.open_the_prompt().await;

        // The first goes at once; the newest waits out the interval and goes alone.
        h.hover(&h.p1, json!(0));
        h.hover(&h.p1, Value::Null);
        h.hover(&h.p1, json!(1));
        h.idle().await;
        assert_eq!(relayed_indexes(&h.p2), vec![json!(0)]);
        sleep_settled(hover_interval() - 1).await;
        assert_eq!(relays(&h.p2).len(), 1);
        sleep_settled(1).await;
        assert_eq!(relayed_indexes(&h.p2), vec![json!(0), json!(1)]);

        // 0, 1, 0: the 1 is coalesced away, and the newest is what p2 was last told, so nothing more.
        sleep_settled(hover_interval()).await;
        h.hover(&h.p1, json!(0));
        h.hover(&h.p1, json!(1));
        h.hover(&h.p1, json!(0));
        h.idle().await;
        sleep_settled(hover_interval()).await;
        assert_eq!(relayed_indexes(&h.p2), vec![json!(0), json!(1), json!(0)]);
    }

    #[tokio::test(start_paused = true)]
    async fn r2739001_drops_a_hover_with_no_prompt_or_out_of_range() {
        let h = harness().await;
        h.hover(&h.p1, json!(0));
        h.idle().await;
        assert_eq!(relays(&h.p2), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        h.open_the_prompt().await;

        h.hover(&h.p1, json!(OPTION_COUNT));
        // p2 holds no prompt: p1's is open.
        h.hover(&h.p2, json!(0));
        h.idle().await;
        assert_eq!(relays(&h.p2), Vec::<Value>::new());
        assert_eq!(relays(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p1), Vec::<Value>::new());
        assert_eq!(errors(&h.p2), Vec::<Value>::new());

        // In range it goes, and one out of range after it clears the outline instead.
        sleep_settled(hover_interval()).await;
        h.hover(&h.p1, json!(OPTION_COUNT - 1));
        h.idle().await;
        sleep_settled(hover_interval()).await;
        h.hover(&h.p1, json!(OPTION_COUNT));
        h.idle().await;
        assert_eq!(relayed_indexes(&h.p2), vec![json!(OPTION_COUNT - 1), Value::Null]);
    }

    /// A real p2 `PlayerView`, its `pending` replaced by the notice of a prompt `pending_for` holds.
    fn p2_view_of(pending_for: PlayerId, option_count: Option<i32>) -> PlayerView {
        install_test_cards();
        let state = begun_game("r2739001-shown", (fake_deck(&[TEST_PROMPT_SELF]), fake_deck(&[])));
        PlayerView {
            pending: Some(PendingView::Elsewhere(PendingElsewhereView {
                for_you: false,
                pending_for,
                option_count,
            })),
            ..jackioh_engine::view_for(&state, PlayerId::P2)
        }
    }

    #[test]
    fn r2739001_hover_is_shown_only_for_a_shown_count() {
        let bare = p2_view_of(PlayerId::P1, None);
        assert!(!hover_is_shown(0, PlayerId::P1, &bare));

        let counted = p2_view_of(PlayerId::P1, Some(2));
        assert!(hover_is_shown(0, PlayerId::P1, &counted));
        assert!(hover_is_shown(1, PlayerId::P1, &counted));
        assert!(!hover_is_shown(2, PlayerId::P1, &counted));

        let own = p2_view_of(PlayerId::P2, Some(2));
        assert!(!hover_is_shown(1, PlayerId::P1, &own));

        let none = PlayerView {
            pending: None,
            ..counted
        };
        assert!(!hover_is_shown(0, PlayerId::P1, &none));
    }

    #[tokio::test(start_paused = true)]
    async fn r2739001_a_malformed_frame_is_malformed() {
        let h = harness().await;
        h.open_the_prompt().await;
        h.p1.receive_json(json!({ "type": "choiceHover" }));
        for index in [json!(-1), json!(1.5), json!("1")] {
            h.hover(&h.p1, index);
        }
        h.idle().await;
        let codes: Vec<Value> = errors(&h.p1)
            .into_iter()
            .map(|error| error["code"].clone())
            .collect();
        assert_eq!(codes, vec![json!("malformed"); 4]);
        assert_eq!(relays(&h.p2), Vec::<Value>::new());

        h.hover(&h.p1, json!(1));
        h.idle().await;
        assert_eq!(relayed_indexes(&h.p2), vec![json!(1)]);
    }

    /// How a test ends the hover's prompt.
    #[derive(Clone, Copy, Debug)]
    enum End {
        Answer,
        SocketClose,
        Concession,
    }

    #[tokio::test(start_paused = true)]
    async fn r2739001_clears_with_null_on_answer_socket_close_and_game_end() {
        for end in [End::Answer, End::SocketClose, End::Concession] {
            let h = harness().await;
            h.open_the_prompt().await;
            h.hover(&h.p1, json!(1));
            h.idle().await;
            match end {
                End::Answer => {
                    let answer = jackioh_engine::legal_actions(&h.actor.engine_state(), PlayerId::P1)
                        .into_iter()
                        .find(|body| matches!(body, ActionBody::Answer { .. }))
                        .expect("p1 may answer its prompt");
                    let answer = serde_json::to_value(answer).expect("ActionBody serializes");
                    submit_ok(&h.actor, PlayerId::P1, "answer", answer).await;
                }
                End::SocketClose => h.p1.drop_transport(),
                End::Concession => {
                    submit_ok(&h.actor, PlayerId::P2, "concede", json!({ "type": "concede" })).await;
                }
            }
            h.idle().await;
            assert_eq!(
                relays(&h.p2),
                vec![
                    json!({ "type": "choiceHover", "from": "p1", "index": 1 }),
                    json!({ "type": "choiceHover", "from": "p1", "index": null }),
                ],
                "{end:?}"
            );
            assert_eq!(relays(&h.p1), Vec::<Value>::new(), "{end:?}");
        }
    }
}
