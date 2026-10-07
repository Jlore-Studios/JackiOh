//! Two verbs of the Classic+ #1–#39 workstream: `damageRoundsUntilDeath` (effects/rounds.ts; SPEC
//! §8.7 C+ #32.3 Blade Storm, §4.5, R59, R113, R283) and `chooseFromHand`'s `where` (effects/choose.ts;
//! C+ #31 Fusion Lab's end of turn, R23). The real cards' tests (packages/cards/test/classic-plus/
//! 032-3-blade-storm.test.ts, 031-fusion-lab.test.ts) cover the cards again.
//!
//! Fixtures are this file's own: defs are `pcv-`, indexed from 5900.
//!
//! Port of `packages/engine/test/effects-plus-c.test.ts`.

use std::sync::Arc;

use jackioh_engine::testkit::*;

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::config::BLADE_STORM_ROUNDS;
use jackioh_engine::effects::{ChooseFromHandArgs, choose_from_hand, choose_mode, damage_rounds_until_death};
use jackioh_engine::layers::unit_has;
use jackioh_engine::reduce::{ReduceResult, begin_game, reduce};
use jackioh_engine::replay::hash_state;
use jackioh_engine::script::{CardScripts, EffectContext, Hook, Script, hook};
use jackioh_engine::scripts::registered_scripts;
use jackioh_engine::state::{CardInstance, GameState, find_instance_mut};
use jackioh_engine::view_for::view_for;

use super::fixtures::harness::{in_hand, new_game, put, slot};

/// TS `def(name, type, extra)`, numbered from a module counter starting at 5900 in declaration order;
/// the number is passed here.
fn def(name: &str, type_: &str, index: i32, extra: Value) -> CardDef {
    let face = json!({ "keywords": [], "text": name });
    let mut def = json!({
        "id": format!("pcv-{name}"),
        "index": index.to_string(),
        "name": format!("PCV {name}"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face,
        "radiant": face,
    });
    if let (Some(base), Some(extra)) = (def.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            base.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

fn unit(name: &str, index: i32, attack: i32, health: i32, keywords: Value) -> CardDef {
    let face = json!({ "attack": attack, "health": health, "keywords": keywords, "text": name });
    def(name, "Unit", index, json!({ "base": face, "radiant": face }))
}

fn storm() -> CardDef {
    def("storm", "Spell", 5901, json!({}))
}
fn short_storm() -> CardDef {
    def("short-storm", "Spell", 5902, json!({}))
}
const SHORT_ROUNDS: i32 = 3;
fn body3() -> CardDef {
    unit("body-3", 5903, 3, 3, json!([]))
}
fn body5() -> CardDef {
    unit("body-5", 5904, 5, 5, json!([]))
}
fn shielded() -> CardDef {
    unit("shielded", 5905, 1, 1, json!([{ "kind": "Divine Shield" }]))
}
fn armored() -> CardDef {
    unit("armored", 5906, 1, 1, json!([{ "kind": "Armor", "n": 1 }]))
}
fn unbreakable() -> CardDef {
    unit("unbreakable", 5907, 1, 2, json!([{ "kind": "Indestructible" }]))
}
/// Survives every round of a 30-round storm, so its hits count the rounds.
fn tank() -> CardDef {
    unit("tank", 5908, 1, BLADE_STORM_ROUNDS + 1, json!([]))
}
fn reborn() -> CardDef {
    unit("reborn", 5909, 1, 2, json!([{ "kind": "Reborn" }]))
}
fn spellproof() -> CardDef {
    unit("spellproof", 5910, 1, 1, json!([{ "kind": "Immune to Spells" }]))
}
fn spell_damage() -> CardDef {
    unit("spell-damage", 5911, 1, 9, json!([{ "kind": "Spell Damage", "n": 1 }]))
}
/// Death: its controller chooses "left" or "right" — a Death hook that asks inside a round's check.
fn ask_on_death() -> CardDef {
    unit("ask-on-death", 5912, 1, 1, json!([]))
}
/// End of turn: choose a hand card that is not Immutable; remembers the pick.
fn picker() -> CardDef {
    def("picker", "Field Spell", 5913, json!({}))
}
fn immutable() -> CardDef {
    unit("immutable", 5914, 1, 1, json!([{ "kind": "Immutable" }]))
}
fn plain() -> CardDef {
    unit("plain", 5915, 1, 1, json!([]))
}

fn defs() -> Vec<CardDef> {
    vec![
        storm(),
        short_storm(),
        body3(),
        body5(),
        shielded(),
        armored(),
        unbreakable(),
        tank(),
        reborn(),
        spellproof(),
        spell_damage(),
        ask_on_death(),
        picker(),
        immutable(),
        plain(),
    ]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn storm_cry(rounds: i32, side: &'static str) -> Option<Hook> {
    Some(hook(move |_ctx| {
        vec![damage_rounds_until_death(json_as(json!({ "amount": 1, "rounds": rounds, "side": side })))]
    }))
}

fn nothing_more() -> Hook {
    hook(|_ctx| vec![])
}

/// `chooseFromHand`'s `where`: `(ctx, card) => !unitHas(ctx.state, card, "Immutable")`.
fn not_immutable(ctx: &EffectContext<'_>, card: &CardInstance) -> bool {
    !unit_has(&*ctx.state, card, KeywordKind::Immutable)
}

fn local_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        storm().id,
        CardScripts {
            base: Script {
                cry: storm_cry(BLADE_STORM_ROUNDS, "any"),
                ..Script::default()
            },
            radiant: Script {
                cry: storm_cry(BLADE_STORM_ROUNDS, "enemy"),
                ..Script::default()
            },
        },
    );
    scripts.insert(
        short_storm().id,
        both(Script {
            cry: storm_cry(SHORT_ROUNDS, "any"),
            ..Script::default()
        }),
    );
    let mut asked = IndexMap::new();
    asked.insert("asked", nothing_more());
    scripts.insert(
        ask_on_death().id,
        both(Script {
            death: Some(hook(|_ctx| {
                vec![choose_mode(json_as(json!({ "options": ["left", "right"], "step": "asked" })))]
            })),
            resume: asked,
            ..Script::default()
        }),
    );
    let mut picked = IndexMap::new();
    picked.insert("picked", nothing_more());
    scripts.insert(
        picker().id,
        both(Script {
            end_of_turn: Some(hook(|_ctx| {
                vec![choose_from_hand(ChooseFromHandArgs {
                    where_: Some(Arc::new(not_immutable)),
                    ..json_as(json!({ "step": "picked" }))
                })]
            })),
            resume: picked,
            ..Script::default()
        }),
    );
    scripts
}

/// `reduce` with a fresh nonce; panics with the refusal (TS threw). TS numbered its nonces from a
/// module counter (`pcv<n>`); here the game's own count of applied actions numbers them, which is
/// unique within a game as the counter was (the state hash leaves `applied` out, §5.2).
fn act(state: &GameState, body: Value) -> ReduceResult {
    let mut action = body;
    action["nonce"] = json!(format!("pcv{}", state.applied.len() + 1));
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// p1's main phase, hands empty, this file's cards registered.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    for player_id in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player_id].hand.iter().map(|c| c.id.clone()).collect();
        state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player_id })).state;
    }
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(local_scripts());
    register_scripts(scripts);
    for player_id in [PlayerId::P1, PlayerId::P2] {
        state.players[player_id].hand = Vec::new();
    }
    state
}

/// TS `stormed`'s `options: { defId?, radiant? }`.
#[derive(Default)]
struct StormOptions {
    def_id: Option<String>,
    radiant: bool,
}

/// p1 casts a storm onto the board `setup` builds (a spare hand card keeps the turn going). What
/// `setup` hands back is handed back beside the play's result (TS's closures assigned outer `let`s).
fn stormed<T>(seed: &str, setup: impl FnOnce(&mut GameState) -> T, options: StormOptions) -> (T, ReduceResult) {
    let mut state = playing(seed);
    let built = setup(&mut state);
    let def_id = options.def_id.unwrap_or_else(|| storm().id);
    let card = in_hand(&mut state, &def_id, PlayerId::P1, 1)
        .into_iter()
        .next()
        .expect("the storm");
    find_instance_mut(&mut state, &card.id).expect("the storm").radiant = options.radiant;
    in_hand(&mut state, &plain().id, PlayerId::P1, 1);
    let played = act(&state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
    (built, played)
}

fn event_json(events: &[GameEvent]) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .collect()
}

fn hits_to(events: &[GameEvent], card: &CardInstance) -> Vec<i32> {
    event_json(events)
        .iter()
        .filter(|e| e["type"] == "damage" && e["targetId"] == card.id.as_str())
        .map(|e| e["amount"].as_i64().unwrap_or_default() as i32)
        .collect()
}

fn deaths(events: &[GameEvent]) -> Vec<String> {
    event_json(events)
        .iter()
        .filter(|e| e["type"] == "destroyed")
        .map(|e| e["defId"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn rounds_run(events: &[GameEvent], card: &CardInstance) -> usize {
    hits_to(events, card).len()
}

fn kinds(events: &[GameEvent]) -> Vec<String> {
    event_json(events)
        .iter()
        .map(|e| e["type"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn count_of(events: &[GameEvent], kind: &str) -> usize {
    kinds(events).iter().filter(|k| k.as_str() == kind).count()
}

/// The top card of `player`'s unit lane `lane` (TS `state.players[player].units[lane - 1]?.[0]`).
fn top_of(state: &GameState, player: PlayerId, lane: usize) -> Option<&CardInstance> {
    state.players[player].units[lane - 1].as_ref().and_then(|pile| pile.first())
}

fn thaw(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state serialises")).expect("a state parses")
}

mod r59_damage_rounds_until_death_each_round_checks_state_and_the_storm_stops_after_a_round_in_which_a_unit_died {
    use super::*;

    #[test]
    fn r59_1_damage_to_every_unit_a_round_until_one_dies_a_3_3_dies_in_round_3_and_the_5_5_keeps_3_damage() {
        let ((small, big), played) = stormed(
            "pcv-basic",
            |s| {
                let small = put(s, &body3().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                let big = put(s, &body5().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
                (small, big)
            },
            StormOptions::default(),
        );
        assert_eq!(hits_to(&played.events, &small), vec![1, 1, 1]);
        assert_eq!(hits_to(&played.events, &big), vec![1, 1, 1]);
        assert_eq!(deaths(&played.events), vec![body3().id]);
        assert_eq!(top_of(&played.state, PlayerId::P1, 1).map(|c| c.damage), Some(3));
    }

    #[test]
    fn r59_each_rounds_check_comes_before_the_next_rounds_hits_the_death_sits_between_round_3_and_nothing_after() {
        let (small, played) = stormed(
            "pcv-order",
            |s| {
                let small = put(s, &body3().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                put(s, &body5().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
                small
            },
            StormOptions::default(),
        );
        let kinds = kinds(&played.events);
        let last_damage = kinds.iter().rposition(|k| k == "damage").map_or(-1, |at| at as i64);
        let first_death = kinds.iter().position(|k| k == "destroyed").map_or(-1, |at| at as i64);
        assert!(last_damage < first_death);
        assert!(!small.id.is_empty());
    }

    #[test]
    fn r59_divine_shield_pops_in_the_first_round_and_the_storm_goes_on() {
        let (shield, played) = stormed(
            "pcv-shield",
            |s| {
                let shield = put(s, &shielded().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                put(s, &body5().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
                shield
            },
            StormOptions::default(),
        );
        assert_eq!(count_of(&played.events, "divineShieldLost"), 1);
        assert_eq!(deaths(&played.events), vec![shielded().id]);
        assert_eq!(hits_to(&played.events, &shield), vec![1]);
    }

    #[test]
    fn r59_a_board_no_round_can_kill_armor_1_indestructible_runs_out_its_rounds_and_stops_no_hit_lands_nothing_dies() {
        let ((), played) = stormed(
            "pcv-walls",
            |s| {
                put(s, &armored().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                put(s, &unbreakable().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
            },
            StormOptions::default(),
        );
        assert_eq!(count_of(&played.events, "damage"), 0);
        assert_eq!(deaths(&played.events), Vec::<String>::new());
        assert!(played.state.pending.is_none());
    }

    #[test]
    fn r59_with_no_death_the_storm_runs_exactly_the_round_cap_blade_storm_rounds() {
        let (counter, played) = stormed(
            "pcv-cap",
            |s| {
                put(s, &unbreakable().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                put(s, &tank().id, slot(PlayerId::P1, Row::Units, 1), json!({}))
            },
            StormOptions::default(),
        );
        assert_eq!(rounds_run(&played.events, &counter), BLADE_STORM_ROUNDS as usize);
        assert_eq!(top_of(&played.state, PlayerId::P1, 1).map(|c| c.damage), Some(BLADE_STORM_ROUNDS));
        assert_eq!(deaths(&played.events), Vec::<String>::new());
    }

    #[test]
    fn r59_the_cap_is_the_callers_a_three_round_storm_stops_after_three() {
        let (counter, played) = stormed(
            "pcv-short",
            |s| put(s, &tank().id, slot(PlayerId::P1, Row::Units, 1), json!({})),
            StormOptions {
                def_id: Some(short_storm().id),
                ..Default::default()
            },
        );
        assert_eq!(rounds_run(&played.events, &counter), SHORT_ROUNDS as usize);
    }

    #[test]
    fn r59_a_reborn_death_counts_the_storm_stops_and_the_unit_comes_back() {
        let ((back, wall), played) = stormed(
            "pcv-reborn",
            |s| {
                let back = put(s, &reborn().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                let wall = put(s, &tank().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
                (back, wall)
            },
            StormOptions::default(),
        );
        assert!(!back.id.is_empty());
        assert_eq!(deaths(&played.events), vec![reborn().id]);
        assert_eq!(rounds_run(&played.events, &wall), 2);
        assert_eq!(top_of(&played.state, PlayerId::P2, 1).map(|c| c.def_id.clone()), Some(reborn().id));
    }

    #[test]
    fn r59_with_no_unit_to_hit_nothing_happens_an_immune_to_spells_unit_is_never_hit_and_never_counts() {
        let ((), empty) = stormed("pcv-empty", |_s| (), StormOptions::default());
        assert_eq!(count_of(&empty.events, "damage"), 0);
        let (proof, played) = stormed(
            "pcv-proof",
            |s| put(s, &spellproof().id, slot(PlayerId::P2, Row::Units, 1), json!({})),
            StormOptions::default(),
        );
        assert_eq!(hits_to(&played.events, &proof), Vec::<i32>::new());
        assert_eq!(count_of(&played.events, "damage"), 0);
    }

    #[test]
    fn s4_4_spell_damage_raises_every_rounds_hit() {
        let (target, played) = stormed(
            "pcv-spell-damage",
            |s| {
                put(s, &spell_damage().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
                put(s, &body5().id, slot(PlayerId::P2, Row::Units, 1), json!({}))
            },
            StormOptions::default(),
        );
        assert_eq!(hits_to(&played.events, &target), vec![2, 2, 2]);
    }

    #[test]
    fn r59_the_radiant_storm_hits_enemy_units_only_and_a_death_there_stops_it() {
        let (mine, played) = stormed(
            "pcv-radiant",
            |s| {
                let mine = put(s, &body5().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
                put(s, &body3().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                mine
            },
            StormOptions {
                radiant: true,
                ..Default::default()
            },
        );
        assert_eq!(hits_to(&played.events, &mine), Vec::<i32>::new());
        assert_eq!(deaths(&played.events), vec![body3().id]);
    }

    #[test]
    fn r113_a_death_hook_asking_inside_a_rounds_check_pauses_the_storm_the_pause_survives_json_and_nothing_more_hits_after_the_answer() {
        let (wall, played) = stormed(
            "pcv-ask",
            |s| {
                put(s, &ask_on_death().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
                put(s, &tank().id, slot(PlayerId::P1, Row::Units, 1), json!({}))
            },
            StormOptions::default(),
        );
        let state = played.state;
        assert_eq!(state.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Mode));
        assert_eq!(rounds_run(&played.events, &wall), 1);
        let thawed = thaw(&state);
        assert_eq!(thawed, state);
        let pending = thawed.pending.clone().expect("a prompt");
        let answer = |s: &GameState| {
            act(
                s,
                json!({
                    "type": "answer",
                    "choiceId": pending.id,
                    "selection": [{ "pick": "mode", "option": "left" }],
                    "playerId": pending.player_id,
                }),
            )
        };
        let live = answer(&state);
        let again = answer(&thawed);
        assert_eq!(hash_state(&again.state), hash_state(&live.state));
        assert!(live.state.pending.is_none());
        assert_eq!(rounds_run(&live.events, &wall), 0);
    }

    #[test]
    fn s9_3_a_storm_replays_to_the_same_hash_from_a_json_copy() {
        let mut state = playing("pcv-replay");
        put(&mut state, &body3().id, slot(PlayerId::P2, Row::Units, 1), json!({}));
        put(&mut state, &armored().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let card = in_hand(&mut state, &storm().id, PlayerId::P1, 1).into_iter().next();
        let action: Action = json_as(json!({
            "type": "play",
            "instanceId": card.map(|c| c.id).unwrap_or_default(),
            "playerId": "p1",
            "nonce": "pcv-replay",
        }));
        let thawed = thaw(&state);
        assert_eq!(hash_state(&reduce(&thawed, &action).state), hash_state(&reduce(&state, &action).state));
    }
}

mod r23_choose_from_hands_where_only_the_cards_it_admits_are_offered {
    use super::*;

    fn at_end_of_turn(seed: &str, hand: &[String]) -> (Vec<CardInstance>, ReduceResult) {
        let mut state = playing(seed);
        put(&mut state, &picker().id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let held: Vec<CardInstance> = hand
            .iter()
            .flat_map(|def_id| in_hand(&mut state, def_id, PlayerId::P1, 1))
            .collect();
        let ended = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        (held, ended)
    }

    #[test]
    fn r23_offers_the_hand_cards_the_filter_admits_to_the_chooser_alone() {
        let (held, ended) = at_end_of_turn("pcv-pick", &[immutable().id, plain().id, plain().id]);
        let state = ended.state;
        let pending = state.pending.as_ref();
        assert_eq!(pending.map(|p| p.kind), Some(PromptKind::Hand));
        assert_eq!(
            pending
                .map(|p| p.options.iter().map(|o| o.key.clone()).collect::<Vec<_>>())
                .unwrap_or_default(),
            held[1..]
                .iter()
                .map(|card| format!("instance:{}", card.id))
                .collect::<Vec<_>>()
        );
        let theirs = serde_json::to_string(&view_for(&state, PlayerId::P2).pending).expect("a view serialises");
        assert!(!theirs.contains(held.get(1).map_or("none", |card| card.id.as_str())));
    }

    #[test]
    fn r23_with_no_admitted_card_it_asks_nothing() {
        assert!(at_end_of_turn("pcv-none", &[immutable().id]).1.state.pending.is_none());
    }
}
