//! Port of `packages/engine/test/start-of-opponent-turn.test.ts`.
//!
//! Classic #62 Living Bomb's card-specific hook (R400, R68): `Script.startOfOpponentTurn`, "at the start
//! of your opponent's turn", queued at R62's start-of-turn trigger point right after the turn player's own
//! `startOfTurn` hooks — R68's order, the active player's cards and then the opponent's — and never on its
//! controller's own turn. Proved through fixture scripts that note what fired, in order, on the watcher.

use std::cell::Cell;

use jackioh_engine::testkit::*;

use super::fixtures::harness::{PutOptions, new_game, put as put_with, slot};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// The harness's `put(state, defId, ref)` with TS's default `options = {}`.
fn put(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    put_with(state, def_id, at, PutOptions::default())
}

fn def(id: &str, type_: CardType, index: i32) -> CardDef {
    let face = if type_ == CardType::Unit {
        json!({ "attack": 1, "health": 5, "keywords": [], "text": id })
    } else {
        json!({ "keywords": [], "text": id })
    };
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    }))
}

/// Living Bomb's shape: a Field Spell answering both its own turn's start and its opponent's.
const WATCHER: &str = "sot-watcher";
fn watcher() -> CardDef {
    def(WATCHER, CardType::FieldSpell, 9611)
}
/// A unit with an ordinary start-of-turn hook, for the other side.
const HOLDER: &str = "sot-holder";
fn holder() -> CardDef {
    def(HOLDER, CardType::Unit, 9612)
}

/// Append `name` to the watcher's log (p1's backrow lane 1), whoever's hook it is.
fn note(name: &'static str) -> Effect {
    Effect::new("sot:note", move |ctx| {
        let Some(log) = ctx.sink.state.players.p1.backrow.get_mut(0).and_then(|slot| slot.as_mut()) else {
            return;
        };
        let mut steps: Vec<Value> = log
            .memory
            .get("steps")
            .and_then(|steps| steps.as_array())
            .cloned()
            .unwrap_or_default();
        steps.push(json!(name));
        log.memory.insert("steps".to_string(), Value::Array(steps));
    })
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        WATCHER.to_string(),
        CardScripts {
            base: Script {
                start_of_turn: Some(hook(|_| vec![note("watcher:own")])),
                start_of_opponent_turn: Some(hook(|_| vec![note("watcher:opponent's")])),
                ..Script::default()
            },
            radiant: Script::default(),
        },
    );
    scripts.insert(
        HOLDER.to_string(),
        CardScripts {
            base: Script {
                start_of_turn: Some(hook(|_| vec![note("holder:own")])),
                ..Script::default()
            },
            radiant: Script::default(),
        },
    );
    scripts
}

thread_local! {
    /// TS's module-level `let nonce`.
    static NONCE: Cell<u32> = const { Cell::new(0) };
}

fn act(state: &GameState, player: PlayerId, body: ActionBody) -> GameState {
    let nonce = NONCE.with(|counter| {
        counter.set(counter.get() + 1);
        counter.get()
    });
    let result = reduce::reduce(state, &Action::new(body, player, format!("sot{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn steps(state: &GameState) -> Vec<String> {
    state.players.p1.backrow[0]
        .as_ref()
        .and_then(|log| log.memory.get("steps"))
        .and_then(|steps| steps.as_array())
        .map(|steps| steps.iter().filter_map(|step| step.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

mod at_the_start_of_your_opponents_turn_c_62_r400_r68 {
    use super::*;

    #[test]
    fn r68_it_fires_on_the_opponents_turn_after_their_own_start_of_turn_hooks_and_never_on_its_controllers_turn() {
        let fresh = new_game("start-of-opponent-turn", None);
        let mut catalog = catalog::registered_catalog().clone();
        catalog.insert(WATCHER.to_string(), watcher());
        catalog.insert(HOLDER.to_string(), holder());
        register_catalog(catalog);
        let mut all = scripts::registered_scripts();
        all.extend(scripts());
        register_scripts(all);
        let mut state = reduce::begin_game(&fresh).state;
        for player_id in [P1, P2] {
            let keep = state.players[player_id].hand.iter().map(|card| card.id.clone()).collect();
            state = act(&state, player_id, ActionBody::Mulligan { keep });
        }
        put(&mut state, WATCHER, slot(P1, Row::Backrow, 1));
        put(&mut state, HOLDER, slot(P2, Row::Units, 1));

        state = act(&state, P1, ActionBody::EndTurn);
        assert_eq!(state.active, P2);
        assert_eq!(steps(&state), vec!["holder:own", "watcher:opponent's"]);

        state = act(&state, P2, ActionBody::EndTurn);
        assert_eq!(state.active, P1);
        assert_eq!(steps(&state), vec!["holder:own", "watcher:opponent's", "watcher:own"]);
    }
}
