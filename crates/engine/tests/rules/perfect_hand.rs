//! B5 E34, the perfect-hand scorer (`subsystems/perfectHand.ts`; SPEC §8.7 C+ #27, §10.7, R29, R364,
//! R387, R416), through a fixture card of Classic+ #27's shape: "replace your hand with the perfect
//! hand, then refresh your mana". The real card's test (packages/cards/test/classic-plus/
//! 027-zephrys-zealotism.test.ts) covers the card again over the real catalog.
//!
//! The pool is pinned: this file registers seven Classic and Classic+ cards on top of the (Core)
//! fixture catalog and nothing else of those sets, so each ranking is a statement about a fixed state
//! and pool, never about the scorer's weights. Core #97's scorer is reused unchanged. Defs are `ph-`,
//! indexed from 4901.
//!
//! Port of `packages/engine/test/perfectHand.test.ts`.

use std::cmp::Ordering;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use serde::Serialize;

use jackioh_engine::effects::mana::refresh_mana;
use jackioh_engine::subsystems::perfect_hand::{rank_perfect_hand, replace_hand_with_perfect};
use jackioh_engine::subsystems::scorer::compare_scored;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::harness::{in_hand, new_game};

/// TS `{ ...base, ...extra }`.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Value::Object(base), Value::Object(extra)) = (&mut base, extra) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    base
}

/// TS `face(text, extra)`.
fn face(text: &str, extra: Value) -> Value {
    spread(json!({ "keywords": [], "text": text }), extra)
}

/// TS `def(name, type, set, extra)`; `index` is the value TS's running `nextIndex` (from 4900) gives it.
fn def(name: &str, type_: &str, set: &str, index: u32, extra: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": format!("ph-{name}"),
            "index": index.to_string(),
            "name": format!("PH {name}"),
            "set": set,
            "type": type_,
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": face(name, json!({})),
            "radiant": face(name, json!({})),
        }),
        extra,
    ))
}

/// TS `unit(name, set, attack, health, extra)`: `extra.keywords` goes on both faces, the rest on the def.
fn unit(name: &str, set: &str, index: u32, attack: i32, health: i32, keywords: Value, rest: Value) -> CardDef {
    def(
        name,
        "Unit",
        set,
        index,
        spread(
            json!({
                "base": face(name, json!({ "attack": attack, "health": health, "keywords": keywords })),
                "radiant": face(name, json!({ "attack": attack * 2, "health": health * 2, "keywords": keywords })),
            }),
            rest,
        ),
    )
}

/// C+ #27's shape. Classic+, so it would be in its own pool but for R387.
fn zealot() -> CardDef {
    def("zealot", "Spell", "Classic+", 4901, json!({ "cost": 4 }))
}

/// A 4/4 Charge for 1: the one card that enables lethal.
fn charger() -> CardDef {
    unit("charger", "Classic+", 4902, 4, 4, json!([{ "kind": "Charge" }]), json!({}))
}

/// The best stats per mana in the pool, 9/9 for 2.
fn big_body() -> CardDef {
    unit("big-body", "Classic", 4903, 9, 9, json!([]), json!({ "cost": 2 }))
}

/// Always tied; "ph-tie-a" sorts first by id but sits at a later index than "ph-tie-b" (index 1).
fn tie_a() -> CardDef {
    unit("tie-a", "Classic+", 4904, 3, 3, json!([]), json!({ "index": "4999" }))
}

fn tie_b() -> CardDef {
    unit("tie-b", "Classic", 4905, 3, 3, json!([]), json!({ "index": "1" }))
}

/// A 0/1 whose Radiant face is a 30/30: low on the base face, first on the Radiant face.
fn sleeper() -> CardDef {
    def(
        "sleeper",
        "Unit",
        "Classic",
        4906,
        json!({
            "base": face("sleeper", json!({ "attack": 0, "health": 1 })),
            "radiant": face("sleeper radiant", json!({ "attack": 30, "health": 30 })),
        }),
    )
}

fn quiet_spell() -> CardDef {
    def("quiet-spell", "Spell", "Classic", 4907, json!({ "cost": 0 }))
}

fn quiet_trap() -> CardDef {
    def("quiet-trap", "Trap", "Classic+", 4908, json!({}))
}

/// Never in the pool: a Core card and a Classic+ token, each better than anything above.
fn core_giant() -> CardDef {
    unit("core-giant", "Core", 4909, 20, 20, json!([]), json!({}))
}

fn plus_token() -> CardDef {
    unit(
        "plus-token",
        "Classic+",
        4910,
        20,
        20,
        json!([]),
        json!({ "token": true, "tags": ["Token"], "rarity": "Token" }),
    )
}

fn pool() -> Vec<CardDef> {
    vec![charger(), big_body(), tie_a(), tie_b(), sleeper(), quiet_spell(), quiet_trap()]
}

fn defs() -> Vec<CardDef> {
    let mut all = vec![zealot()];
    all.extend(pool());
    all.push(core_giant());
    all.push(plus_token());
    all
}

/// TS `Number.POSITIVE_INFINITY`: as much mana as any refresh could give back. An `i32` holds no
/// infinity, and the effect adds it to the current mana, so it is a half of `i32::MAX`, far past any
/// mana pool and short of overflowing the sum.
const ALL_MANA: i32 = i32::MAX / 2;

fn zealot_scripts() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    replace_hand_with_perfect(json_as(json!({}))),
                    refresh_mana(json_as(json!({ "amount": ALL_MANA }))),
                ]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![
                    replace_hand_with_perfect(json_as(json!({ "radiant": true }))),
                    refresh_mana(json_as(json!({ "amount": ALL_MANA }))),
                ]
            })),
            ..Script::default()
        },
    }
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> ReduceResult {
    let n = NONCE.fetch_add(1, AtomicOrdering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("ph{n}"));
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// p1's main phase, both hands empty, 4 of 4 mana, this file's cards registered.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player in [P1, P2] {
        let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
        state = act(
            &state,
            json!({ "type": "mulligan", "keep": keep, "playerId": player }),
        )
        .state;
    }
    let mut catalog = registered_catalog().clone();
    for entry in defs() {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts();
    scripts.insert(zealot().id, zealot_scripts());
    register_scripts(scripts);
    for player in [P1, P2] {
        state.players[player].hand.clear();
    }
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        ..state.players.p1.mana
    };
    state
}

struct Holding {
    state: GameState,
    zealot_id: String,
    held: Vec<CardInstance>,
}

/// p1 holds the zealot and `others`; returns the state and the zealot's id.
fn holding(seed: &str, others: &[String], radiant: bool, mana: Option<i32>) -> Holding {
    let mut state = playing(seed);
    let card = in_hand(&mut state, &zealot().id, P1, 1)
        .into_iter()
        .next()
        .expect("the zealot");
    find_instance_mut(&mut state, &card.id).expect("the zealot in hand").radiant = radiant;
    let held: Vec<CardInstance> = others
        .iter()
        .flat_map(|def_id| in_hand(&mut state, def_id, P1, 1))
        .collect();
    if let Some(mana) = mana {
        state.players.p1.mana.current = mana;
    }
    Holding {
        state,
        zealot_id: card.id,
        held,
    }
}

fn cast(state: &GameState, instance_id: &str) -> ReduceResult {
    act(
        state,
        json!({ "type": "play", "instanceId": instance_id, "playerId": "p1" }),
    )
}

fn hand_ids(state: &GameState) -> Vec<String> {
    state.players.p1.hand.iter().map(|card| card.def_id.clone()).collect()
}

fn rank_ids(state: &GameState, radiant: bool) -> Vec<String> {
    rank_perfect_hand(
        state,
        P1,
        json_as(json!({ "selfDefId": zealot().id, "radiant": radiant })),
    )
    .iter()
    .map(|scored| scored.def.id.clone())
    .collect()
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// TS `JSON.parse(JSON.stringify(state))`.
fn revive(state: &GameState) -> GameState {
    serde_json::from_value(json_of(state)).expect("a state revives from its JSON")
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

/// TS `list.indexOf(id)`: -1 when absent.
fn index_of(list: &[String], id: &str) -> i64 {
    list.iter().position(|entry| entry == id).map_or(-1, |at| at as i64)
}

/// `expect(list).toEqual(expect.arrayContaining(expected))`.
fn contains_all(list: &[String], expected: &[String]) -> bool {
    expected.iter().all(|entry| list.contains(entry))
}

/// The `defId` of each event of this type (TS `"defId" in e ? e.defId : ""`).
fn def_ids_of(events: &[GameEvent], kind: GameEventType) -> Vec<String> {
    events
        .iter()
        .filter(|event| event.event_type() == kind)
        .map(|event| json_of(event)["defId"].as_str().unwrap_or("").to_string())
        .collect()
}

mod e34_the_perfect_hand_ranking_r29_r387_r416 {
    use super::*;

    #[test]
    fn r416_ranks_every_non_token_classic_and_classic_card_but_the_asking_card_no_core_card_no_token() {
        let Holding { state, .. } = holding("ph-pool", &[], false, None);
        assert_eq!(
            sorted(rank_ids(&state, false)),
            sorted(pool().into_iter().map(|entry| entry.id).collect())
        );
        // Asked by nobody in particular, the zealot is a Classic+ card like any other.
        assert!(rank_perfect_hand(&state, P1, json_as(json!({})))
            .iter()
            .any(|scored| scored.def.id == zealot().id));
    }

    #[test]
    fn r387_a_fused_asking_card_leaves_out_every_one_of_its_ingredients() {
        let Holding { state, .. } = holding("ph-fused", &[], false, None);
        let ranked: Vec<String> = rank_perfect_hand(
            &state,
            P1,
            json_as(json!({ "selfDefId": format!("t-1:{}+{}", big_body().id, zealot().id) })),
        )
        .iter()
        .map(|scored| scored.def.id.clone())
        .collect();
        assert!(!ranked.contains(&zealot().id));
        assert!(!ranked.contains(&big_body().id));
        assert!(ranked.contains(&charger().id));
    }

    #[test]
    fn r416_ties_go_by_card_id_never_by_index_core_97s_order_would_put_tie_b_first() {
        let Holding { state, .. } = holding("ph-ties", &[], false, None);
        let ranked = rank_perfect_hand(&state, P1, json_as(json!({ "selfDefId": zealot().id })));
        let a = ranked.iter().find(|scored| scored.def.id == tie_a().id);
        let b = ranked.iter().find(|scored| scored.def.id == tie_b().id);
        let (Some(a), Some(b)) = (a, b) else {
            panic!("both tied cards rank");
        };
        assert_eq!(a.score, b.score);
        let order: Vec<String> = ranked.iter().map(|scored| scored.def.id.clone()).collect();
        assert_eq!(index_of(&order, &tie_a().id), index_of(&order, &tie_b().id) - 1);
        assert_eq!(compare_scored(b, a), Ordering::Less);
    }

    #[test]
    fn r29_the_card_that_enables_lethal_ranks_first_when_lethal_exists_and_only_then() {
        let Holding { mut state, .. } = holding("ph-lethal", &[], false, None);
        state.players.p2.hero.health = 4;
        let lethal = rank_perfect_hand(&state, P1, json_as(json!({ "selfDefId": zealot().id })));
        assert_eq!(lethal.first().map(|scored| scored.def.id.clone()), Some(charger().id));
        assert_eq!(lethal.first().map(|scored| json_of(scored.priority)), Some(json!("lethal")));
        state.players.p2.hero.health = 30;
        let value = rank_perfect_hand(&state, P1, json_as(json!({ "selfDefId": zealot().id })));
        assert_eq!(value.first().map(|scored| scored.def.id.clone()), Some(big_body().id));
        assert!(value.iter().all(|scored| json_of(scored.priority) != json!("lethal")));
    }

    #[test]
    fn r416_the_radiant_face_scores_each_candidate_on_its_radiant_face() {
        let Holding { state, .. } = holding("ph-radiant-rank", &[], false, None);
        assert!(index_of(&rank_ids(&state, false), &sleeper().id) > 2);
        assert_eq!(rank_ids(&state, true).first(), Some(&sleeper().id));
    }

    #[test]
    fn s10_7_the_same_state_ranks_the_same_a_json_copy_included_and_ranking_draws_nothing() {
        let Holding { state, .. } = holding("ph-determinism", &[], false, None);
        let cursor = state.rng_cursor;
        let first = rank_ids(&state, false);
        assert_eq!(rank_ids(&revive(&state), false), first);
        assert_eq!(state.rng_cursor, cursor);
    }
}

mod e34_replace_hand_with_perfect_then_a_refresh_c_27s_shape {
    use super::*;

    #[test]
    fn r416_the_hand_keeps_its_size_each_card_goes_to_the_graveyard_not_a_discard_and_the_top_n_arrive_in_rank_order() {
        let Holding { state, zealot_id, held } = holding(
            "ph-replace",
            &[core_giant().id, core_giant().id, quiet_spell().id],
            false,
            None,
        );
        let expected: Vec<String> = rank_ids(&state, false).into_iter().take(3).collect();
        let after = cast(&state, &zealot_id);
        assert_eq!(hand_ids(&after.state), expected);
        for old in &held {
            assert_eq!(
                find_instance(&after.state, &old.id).map(|card| card.zone.clone()),
                Some(Zone::Graveyard { player: P1 })
            );
        }
        assert!(!after
            .events
            .iter()
            .any(|event| event.event_type() == GameEventType::Discarded));
        assert!(contains_all(
            &def_ids_of(&after.events, GameEventType::EnteredGraveyard),
            &[core_giant().id, quiet_spell().id]
        ));
        for fresh in &after.state.players.p1.hand {
            assert_eq!(fresh.owner, P1);
            assert!(!fresh.radiant);
            assert!(fresh.cost_override.is_none());
            assert_eq!(fresh.cost_mod, 0);
        }
    }

    #[test]
    fn r416_with_no_other_card_in_hand_nothing_arrives_and_the_refresh_still_gives_back_the_4() {
        let Holding { state, zealot_id, .. } = holding("ph-alone", &[], false, None);
        let after = cast(&state, &zealot_id).state;
        assert!(after.players.p1.hand.is_empty());
        assert_eq!(after.players.p1.mana.current, 4);
    }

    #[test]
    fn r11_a_unit_token_card_in_the_replaced_hand_ceases_to_exist_instead_of_reaching_the_graveyard() {
        let Holding { state, zealot_id, held } =
            holding("ph-token", &[plus_token().id, quiet_spell().id], false, None);
        let after = cast(&state, &zealot_id).state;
        let token = held.first();
        let spell = held.get(1);
        assert!(find_instance(&after, token.map(|card| card.id.as_str()).unwrap_or("")).is_none());
        assert!(spell.is_some_and(|spell| after.players.p1.graveyard.iter().any(|card| card.id == spell.id)));
        assert_eq!(after.players.p1.hand.len(), 2);
    }

    #[test]
    fn r29_with_lethal_on_the_board_the_lethal_enabling_card_arrives_first() {
        let Holding { mut state, zealot_id, .. } = holding(
            "ph-lethal-hand",
            &[quiet_spell().id, quiet_spell().id],
            false,
            Some(8),
        );
        state.players.p2.hero.health = 4;
        assert_eq!(
            hand_ids(&cast(&state, &zealot_id).state).first(),
            Some(&charger().id)
        );
    }

    #[test]
    fn r416_the_radiant_face_the_perfect_radiant_hand_arrives_radiant() {
        let Holding { state, zealot_id, .. } =
            holding("ph-radiant", &[quiet_spell().id, quiet_trap().id], true, None);
        let after = cast(&state, &zealot_id).state;
        assert_eq!(hand_ids(&after).first(), Some(&sleeper().id));
        assert!(after.players.p1.hand.iter().all(|card| card.radiant));
    }

    #[test]
    fn r364_the_refresh_gives_back_up_to_max_mana_and_never_past_it() {
        let spent = holding("ph-refresh", &[quiet_spell().id], false, None);
        assert_eq!(
            cast(&spent.state, &spent.zealot_id).state.players.p1.mana.current,
            4
        );
        let above = holding("ph-refresh-above", &[quiet_spell().id], false, Some(9));
        assert_eq!(
            cast(&above.state, &above.zealot_id).state.players.p1.mana.current,
            5
        );
    }

    #[test]
    fn s9_3_the_play_draws_nothing_from_the_match_rng_and_a_json_copy_replays_to_the_same_hash_and_events() {
        let Holding { state, zealot_id, .. } =
            holding("ph-replay", &[quiet_spell().id, core_giant().id], false, None);
        let thawed = revive(&state);
        let action: Action = json_as(json!({
            "type": "play",
            "instanceId": zealot_id,
            "playerId": "p1",
            "nonce": "ph-replay",
        }));
        let live = reduce(&state, &action);
        let again = reduce(&thawed, &action);
        assert_eq!(live.state.rng_cursor, state.rng_cursor);
        assert_eq!(hash_state(&again.state), hash_state(&live.state));
        assert_eq!(again.events, live.events);
    }

    #[test]
    fn r97_the_opponent_sees_the_replaced_cards_reach_the_graveyard_and_never_the_new_cards() {
        let Holding { state, zealot_id, .. } =
            holding("ph-hidden", &[quiet_spell().id, core_giant().id], false, None);
        let after = cast(&state, &zealot_id).state;
        let theirs = view_for(&after, P2);
        let added: Vec<Value> = theirs
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::AddedToHand)
            .map(json_of)
            .collect();
        assert!(added.len() >= 2);
        // `/ph-(charger|big-body|tie|sleeper|quiet)/`, alternative by alternative.
        for event in &added {
            let text = serde_json::to_string(event).expect("an event serialises");
            for name in ["ph-charger", "ph-big-body", "ph-tie", "ph-sleeper", "ph-quiet"] {
                assert!(!text.contains(name));
            }
        }
        assert!(contains_all(
            &def_ids_of(&theirs.events, GameEventType::EnteredGraveyard),
            &[quiet_spell().id, core_giant().id]
        ));
    }
}
