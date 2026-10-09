//! Meditative #95 Call to Chaos (Meditative Edition)'s table (`subsystems/call_to_chaos_meditative.rs`),
//! rolled by Core #95's subsystem (R28, R87, R423, R436): each of the ten entries against fixture pools,
//! the third edition in the Call to Chaos pool only while its set is previewed (R1240, R1420), the
//! rest-of-game entry and its cap (R1241), the hand fusion and `add_to_hand { copyOf }` (R1242), the
//! Acclaimed pool (R1243), Bounce then Nerf (R1244), `shuffle_into { memory }` through JSON (R1246) and
//! the Radiant's three different entries in list order (R423). The real card's test covers the same
//! cases against the real catalog.

use jackioh_engine::catalog::query;
use jackioh_engine::effects::add_to_hand::{AddToHandArgs, add_to_hand};
use jackioh_engine::effects::shuffle_into::shuffle_into;
use jackioh_engine::subsystems::call_to_chaos::{
    CHAOS_CHAIN_KEY, CallToChaosArgs, ChaosEffectDef, call_to_chaos, cast_random_call_to_chaos,
    roll_chaos_effects,
};
use jackioh_engine::subsystems::call_to_chaos_meditative::{
    CHAOS_ETERNAL_HOOK, CHAOS_ETERNAL_LABEL, CHAOS_MED_EFFECTS, eternal_effects_held,
};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::work::script_step_for;

use crate::rules::fixtures::call_to_chaos_meditative::{
    ACCLAIMED, ACCLAIMED_SPELL, ACCLAIMED_TOKEN, CN, GOLEM, JADE, MED, PRIME, chaos_med_catalog,
};
use crate::rules::fixtures::call_to_chaos_plus::{chaos_plus_catalog, core95, immutable, plus, trap};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

fn scripts(cry: Hook) -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(cry.clone()),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(cry),
            ..Script::default()
        },
    }
}

/// The three editions' cries: this edition's table on MED, and what C+ #73's and Core #95's
/// stand-ins do when the recursion casts them (nothing, unless a test says so).
fn register(med_cry: Option<Hook>, other_cry: Option<Hook>) {
    register_catalog(chaos_med_catalog(chaos_plus_catalog(
        registered_catalog().clone(),
    )));
    let mut registered = registered_scripts().clone();
    registered.insert(
        MED.id.clone(),
        scripts(med_cry.unwrap_or_else(|| hook(|_ctx| vec![call_to_chaos(med_table())]))),
    );
    let other = other_cry.unwrap_or_else(|| hook(|_ctx| vec![]));
    registered.insert(plus.id.clone(), scripts(other.clone()));
    registered.insert(core95.id.clone(), scripts(other));
    register_scripts(registered);
}

/// A main phase on p1's turn, empty hands and an empty p1 deck.
fn game(seed: &str, med_cry: Option<Hook>, other_cry: Option<Hook>) -> GameState {
    let mut state = new_game(seed, None);
    register(med_cry, other_cry);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state.players.p1.hand = Vec::new();
    state.players.p2.hand = Vec::new();
    set_library(&mut state, P1, &[] as &[&str]);
    state
}

/// `{ table: CHAOS_MED_EFFECTS }`.
fn med_table() -> CallToChaosArgs {
    CallToChaosArgs {
        table: Some(CHAOS_MED_EFFECTS),
        ..CallToChaosArgs::default()
    }
}

/// A Meditative #95 mid-resolution, as `self` is while its Spell script runs (§10.5).
fn med_card(state: &mut GameState, radiant: bool, chain: Option<i32>) -> CardInstance {
    let mut card = new_instance(&mut *state, &MED.id, P1, Zone::Resolving { player: P1 });
    if radiant {
        card.radiant = true;
    }
    if let Some(chain) = chain {
        card.memory.insert(CHAOS_CHAIN_KEY.to_string(), json!(chain));
    }
    card
}

fn entry(name: &str) -> Effect {
    let found = CHAOS_MED_EFFECTS
        .iter()
        .find(|effect| effect.name == name)
        .unwrap_or_else(|| panic!("no entry {name}"));
    (found.build)()
}

/// Run `effect` as p1 with `self_` (a fresh Meditative #95 when `None`), then the state check.
fn run(state: &mut GameState, effect: Effect, self_: Option<CardInstance>) -> Vec<GameEvent> {
    let self_ = self_.unwrap_or_else(|| med_card(state, false, None));
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        let options = HookOptions {
            controller: Some(P1),
            ..HookOptions::default()
        };
        {
            let mut ctx = make_context(&mut sink, Some(&self_), options);
            apply_effects(&[effect], &mut ctx);
        }
        state_check(&mut sink);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn names_of<D: std::borrow::Borrow<ChaosEffectDef>>(effects: &[D]) -> Vec<String> {
    effects
        .iter()
        .map(|effect| effect.borrow().name.to_string())
        .collect()
}

fn labels_of<D: std::borrow::Borrow<ChaosEffectDef>>(effects: &[D]) -> Vec<String> {
    effects
        .iter()
        .map(|effect| effect.borrow().label.to_string())
        .collect()
}

/// A seed whose roll (base or Radiant) the predicate accepts.
fn seed_where(radiant: bool, accept: impl Fn(&[String]) -> bool, tag: &str) -> String {
    for i in 0..4000 {
        let seed = format!("{tag}-{i}");
        if accept(&names_of(&roll_chaos_effects(
            &mut Rng::new(&seed, 0),
            radiant,
            Some(CHAOS_MED_EFFECTS),
        ))) {
            return seed;
        }
    }
    panic!("no seed for {tag}");
}

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).unwrap()
}

/// One field of every event of a type, as JSON.
fn field_of(events: &[GameEvent], kind: GameEventType, field: &str) -> Vec<Value> {
    events_of_type(events, kind)
        .iter()
        .map(|event| json_of(event)[field].clone())
        .collect()
}

fn strings_of(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect()
}

/// TS `Array.prototype.indexOf`: -1 when absent.
fn index_of(types: &[GameEventType], kind: GameEventType) -> i64 {
    types
        .iter()
        .position(|each| *each == kind)
        .map(|at| at as i64)
        .unwrap_or(-1)
}

/// The live card.
fn card<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids.dedup();
    ids
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// The Call to Chaos pool as a pool that names no set reads it (R28, R380).
fn chaos_pool() -> Vec<String> {
    sorted(
        query(&json_as(json!({ "tags": ["Call to Chaos"] })))
            .iter()
            .map(|def| def.id.clone())
            .collect(),
    )
}

/// The defs a list of events played, in order.
fn played(events: &[GameEvent]) -> Vec<String> {
    strings_of(&field_of(events, GameEventType::CardPlayed, "defId"))
}

mod r1240_meditative_95s_table {
    use super::*;

    #[test]
    fn is_the_ten_entries_of_s8_8_row_95_in_the_cards_order_each_with_its_printed_clause() {
        assert_eq!(
            names_of(CHAOS_MED_EFFECTS),
            vec![
                "fuse",
                "cn",
                "prime",
                "armor",
                "jade",
                "acclaimed",
                "bounce",
                "golem",
                "eternal",
                "recast"
            ]
        );
        assert_eq!(
            labels_of(CHAOS_MED_EFFECTS),
            vec![
                "Fuse your hand into one card and add 2 copies of it to your hand, all three of which cost (0)",
                "Add 3 random CN cards to your hand, which cost (0)",
                "Add 2 random Prime cards to your hand, which cost (0)",
                "Your hero gains 8 Armor and you heal it 8",
                "Summon a Jade Beauty",
                "Summon 3 random Acclaimed cards",
                "Bounce every enemy permanent, then Nerf each card bounced",
                "Summon a CN Golem",
                "For the rest of the game, at the start of each of your turns, cast a random Call to Chaos",
                "Cast a random Call to Chaos",
            ]
        );
    }

    #[test]
    fn r1240_the_call_to_chaos_pool_holds_the_meditative_edition_only_while_its_set_is_previewed() {
        game("pool", None, None);
        assert_eq!(chaos_pool(), sorted(vec![core95.id.clone(), plus.id.clone()]));
        {
            let _preview = preview_sets(&[SetName::Meditative]);
            assert_eq!(
                chaos_pool(),
                sorted(vec![core95.id.clone(), plus.id.clone(), MED.id.clone()])
            );
        }
        assert_eq!(chaos_pool(), sorted(vec![core95.id.clone(), plus.id.clone()]));
    }

    #[test]
    fn r1240_r28_the_chain_counts_casts_of_every_edition_against_the_cap() {
        let _preview = preview_sets(&[SetName::Meditative]);
        let recast = || hook(|_ctx| vec![cast_random_call_to_chaos()]);
        let mut seen: Vec<String> = Vec::new();
        for i in 0..3 {
            let mut state = game(&format!("chain-{i}"), Some(recast()), Some(recast()));
            let events = run(&mut state, entry("recast"), None);
            let cast = played(&events);
            assert_eq!(cast.len() as i32, CALL_TO_CHAOS_CHAIN_CAP);
            seen.extend(cast);
        }
        assert_eq!(
            sorted(seen),
            sorted(vec![core95.id.clone(), plus.id.clone(), MED.id.clone()])
        );
    }

    #[test]
    fn r1240_r87_a_recursion_rolled_at_the_cap_resolves_into_nothing() {
        let seed = seed_where(
            true,
            |names| names.iter().any(|name| name == "recast") && names.iter().any(|name| name == "golem"),
            "cap",
        );
        let mut state = game(&seed, None, None);
        let me = med_card(&mut state, true, Some(CALL_TO_CHAOS_CHAIN_CAP));
        let events = run(&mut state, call_to_chaos(med_table()), Some(me));
        assert!(events_of_type(&events, GameEventType::CardPlayed).is_empty());
        assert_eq!(
            field_of(&events, GameEventType::Summoned, "defId"),
            vec![json!(GOLEM.id.clone())]
        );
    }

    #[test]
    fn r423_the_radiant_face_resolves_three_different_entries_in_list_order() {
        for i in 0..200 {
            let names = names_of(&roll_chaos_effects(
                &mut Rng::new(&format!("med-radiant-{i}"), 0),
                true,
                Some(CHAOS_MED_EFFECTS),
            ));
            assert_eq!(
                names.iter().collect::<IndexSet<_>>().len() as i32,
                CALL_TO_CHAOS_RADIANT_EFFECTS
            );
            let order: Vec<usize> = names
                .iter()
                .filter_map(|name| {
                    CHAOS_MED_EFFECTS
                        .iter()
                        .position(|effect| effect.name == name.as_str())
                })
                .collect();
            let mut in_order = order.clone();
            in_order.sort();
            assert_eq!(order, in_order);
        }
        let seed = seed_where(
            true,
            |names| names.join(",") == ["cn", "golem", "recast"].join(","),
            "order",
        );
        let mut state = game(&seed, None, None);
        let me = med_card(&mut state, true, None);
        let events = run(&mut state, call_to_chaos(med_table()), Some(me));
        let types: Vec<GameEventType> = events.iter().map(|event| event.event_type()).collect();
        assert!(index_of(&types, GameEventType::AddedToHand) >= 0);
        assert!(index_of(&types, GameEventType::AddedToHand) < index_of(&types, GameEventType::Summoned));
        assert!(index_of(&types, GameEventType::Summoned) < index_of(&types, GameEventType::CardPlayed));
    }

    #[test]
    fn r436_both_players_are_told_the_rolled_clauses_by_this_card_before_any_of_it_resolves() {
        let seed = seed_where(
            true,
            |names| !names.iter().any(|name| name == "recast"),
            "announce",
        );
        let mut state = game(&seed, None, None);
        let me = med_card(&mut state, true, None);
        let events = run(&mut state, call_to_chaos(med_table()), Some(me.clone()));
        let expected = labels_of(&roll_chaos_effects(
            &mut Rng::new(&seed, 0),
            true,
            Some(CHAOS_MED_EFFECTS),
        ));
        assert_eq!(
            events.first().map(json_of),
            Some(
                json!({ "type": "chaosRolled", "player": "p1", "instanceId": me.id, "defId": MED.id, "effects": expected })
            )
        );
    }
}

mod r1241_the_rest_of_game_entry {
    use super::*;

    /// p1's rest-of-game effects, as `(hook, instance, label)`.
    fn held(state: &GameState) -> Vec<(String, Option<String>, String)> {
        state
            .players
            .p1
            .mods
            .iter()
            .filter_map(|held| match &held.kind {
                ModifierKind::StartOfTurnEffect { resume, label, .. } => {
                    Some((resume.hook.clone(), resume.instance_id.clone(), label.clone()))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1241_the_rest_of_game_entry_gives_its_caster_one_start_of_turn_effect_with_no_self() {
        let mut state = game("eternal", None, None);
        run(&mut state, entry("eternal"), None);
        assert_eq!(
            held(&state),
            vec![(
                CHAOS_ETERNAL_HOOK.to_string(),
                None,
                CHAOS_ETERNAL_LABEL.to_string()
            )]
        );
        assert_eq!(eternal_effects_held(&state, P1), 1);
        assert_eq!(eternal_effects_held(&state, P2), 0);
        // R169: the badge both players read.
        assert!(
            json_of(view_for(&state, P2))["opponent"]["modifiers"]
                .as_array()
                .is_some_and(|mods| mods
                    .iter()
                    .any(|entry| entry["label"] == json!(CHAOS_ETERNAL_LABEL)))
        );
    }

    #[test]
    fn r1241_at_the_cap_a_roll_adds_nothing_and_draws_no_rng() {
        let mut state = game("eternal-cap", None, None);
        for _ in 0..CALL_TO_CHAOS_ETERNAL_CAP {
            run(&mut state, entry("eternal"), None);
        }
        assert_eq!(eternal_effects_held(&state, P1), CALL_TO_CHAOS_ETERNAL_CAP);
        let cursor = state.rng_cursor;
        let mods = state.players.p1.mods.len();
        let events = run(&mut state, entry("eternal"), None);
        assert!(events.is_empty());
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(state.players.p1.mods.len(), mods);
        assert_eq!(eternal_effects_held(&state, P1), CALL_TO_CHAOS_ETERNAL_CAP);
    }

    #[test]
    fn r1241_the_engine_resolves_the_eternal_hook_for_any_script() {
        let resume = Resume {
            def_id: "fx-1".to_string(),
            hook: CHAOS_ETERNAL_HOOK.to_string(),
            step: "chaosEternal".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
        };
        assert!(script_step_for(&Script::default(), &resume).is_some());
        // Any other `@` hook a script does not hold stays unanswered.
        let other = Resume {
            hook: "@elsewhere".to_string(),
            ..resume
        };
        assert!(script_step_for(&Script::default(), &other).is_none());
    }

    /// `reduce` with a fresh nonce.
    fn act(state: &GameState, body: Value, nonce: &mut u32) -> ReduceResult {
        *nonce += 1;
        let mut action = body;
        action["nonce"] = json!(format!("med{nonce}"));
        let result = reduce(state, &json_as::<Action>(action));
        if let Some(error) = &result.error {
            panic!("{error}");
        }
        result
    }

    fn events_of(result: &ReduceResult) -> Vec<GameEvent> {
        result.events.clone()
    }

    #[test]
    fn r1241_from_the_casters_next_turn_each_start_of_turn_casts_a_random_call_to_chaos_as_a_new_chain() {
        let recast = || hook(|_ctx| vec![cast_random_call_to_chaos()]);
        let mut state = begin_game(&new_game("eternal-turns", None)).state;
        register(None, Some(recast()));
        let mut nonce = 0;
        for player in [P1, P2] {
            let keep = ids_of(&state.players[player].hand);
            state = act(
                &state,
                json!({ "type": "mulligan", "keep": keep, "playerId": player }),
                &mut nonce,
            )
            .state;
        }
        state.players.p1.auto_end_turn = Some(false);
        state.players.p2.auto_end_turn = Some(false);
        let caster = state.active;
        // The entry resolves on the caster's turn: nothing is cast yet.
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let options = HookOptions {
                controller: Some(caster),
                ..HookOptions::default()
            };
            let mut ctx = make_context(&mut sink, None, options);
            apply_effects(&[entry("eternal")], &mut ctx);
        }
        state.rng_cursor = rng.cursor();
        assert!(played(&events).is_empty());
        // The opponent's turn casts nothing; the caster's next start of turn casts one chain, its first
        // cast at depth 1, so the stand-ins' recursion runs it to the cap (R28).
        let theirs = act(
            &state,
            json!({ "type": "endTurn", "playerId": caster }),
            &mut nonce,
        );
        assert!(played(&events_of(&theirs)).is_empty());
        let back = act(
            &theirs.state,
            json!({ "type": "endTurn", "playerId": theirs.state.active }),
            &mut nonce,
        );
        assert_eq!(back.state.active, caster);
        let cast = played(&events_of(&back));
        assert_eq!(cast.len() as i32, CALL_TO_CHAOS_CHAIN_CAP);
        assert!(
            cast.iter()
                .all(|id| [core95.id.clone(), plus.id.clone()].contains(id))
        );
    }
}

mod r1242_the_hand_fusion {
    use super::*;

    #[test]
    fn r1242_fuses_every_hand_card_but_immutable_ones_into_one_fresh_card_and_adds_two_copies_all_at_zero() {
        let mut state = game("fuse", None, None);
        let first = in_hand(&mut state, "fx-1", P1, 1);
        let kept = in_hand(&mut state, &immutable.id, P1, 1);
        let spell = in_hand(&mut state, &CN.id, P1, 1);
        let theirs = in_hand(&mut state, "fx-2", P2, 1);
        let events = run(&mut state, entry("fuse"), None);
        let fused = events_of_type(&events, GameEventType::Fused);
        assert_eq!(fused.len(), 1);
        let fused = json_of(&fused[0]);
        assert_eq!(
            fused["instanceIds"],
            json!([first[0].id.clone(), spell[0].id.clone()])
        );
        let result = fused["resultInstanceId"].as_str().unwrap_or_default().to_string();
        let hand = state.players.p1.hand.clone();
        assert_eq!(hand.len(), 1 + 1 + CHAOS_MED_FUSE_COPIES as usize);
        assert_eq!(hand[0].id, kept[0].id);
        assert_eq!(hand[1].id, result);
        let fusion = &hand[1];
        assert!(state.transient_defs.contains_key(&fusion.def_id));
        for copy in &hand[2..] {
            assert_eq!(copy.def_id, fusion.def_id);
            assert_ne!(copy.id, fusion.id);
        }
        for each in &hand[1..] {
            assert_eq!(
                effective_cost(&state, each, CostOptions::default()),
                CHAOS_MED_COST
            );
        }
        // The Immutable card stays, at its own cost (R23); the ingredients ceased to exist.
        assert_eq!(effective_cost(&state, &hand[0], CostOptions::default()), 1);
        assert!(find_instance(&state, &first[0].id).is_none());
        assert!(find_instance(&state, &spell[0].id).is_none());
        assert_eq!(state.players.p2.hand[0].id, theirs[0].id);
        // R470: a hand's fusion is hidden as the hand is; the opponent reads no card of it.
        state.applied = vec![AppliedAction {
            nonce: "fuse".into(),
            events,
        }];
        let seen: Vec<Value> = view_for(&state, P2)
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::AddedToHand)
            .map(json_of)
            .collect();
        assert!(seen.iter().all(|event| event["instanceId"] == json!(HIDDEN_ID)));
        assert!(
            !json_of(view_for(&state, P2).events)
                .to_string()
                .contains(&fusion.def_id)
        );
    }

    #[test]
    fn r1242_one_fusable_card_takes_the_zero_and_two_copies() {
        let mut state = game("fuse-one", None, None);
        let kept = in_hand(&mut state, &immutable.id, P1, 1);
        let only = put(
            &mut state,
            "fx-3",
            slot(P1, Row::Units, 1),
            json!({ "radiant": true }),
        );
        // A Radiant card in hand: moved off the field into the hand by hand.
        bounce_to_hand(&mut state, &only.id);
        let events = run(&mut state, entry("fuse"), None);
        assert!(events_of_type(&events, GameEventType::Fused).is_empty());
        let hand = state.players.p1.hand.clone();
        assert_eq!(ids_of(&hand[..2]), vec![kept[0].id.clone(), only.id.clone()]);
        assert_eq!(hand.len(), 2 + CHAOS_MED_FUSE_COPIES as usize);
        assert_eq!(hand[1].cost_override, Some(CHAOS_MED_COST));
        for copy in &hand[2..] {
            assert_eq!(copy.def_id, "fx-3");
            assert!(copy.radiant);
            assert_eq!(copy.cost_override, Some(CHAOS_MED_COST));
        }
        assert_eq!(hand[0].cost_override, None);
    }

    #[test]
    fn r1242_no_fusable_card_does_nothing() {
        let mut state = game("fuse-none", None, None);
        let kept = in_hand(&mut state, &immutable.id, P1, 1);
        let events = run(&mut state, entry("fuse"), None);
        assert!(events.is_empty());
        assert_eq!(ids_of(&state.players.p1.hand), ids_of(&kept));
        assert_eq!(state.players.p1.hand[0].cost_override, None);
        let mut empty = game("fuse-empty", None, None);
        assert!(run(&mut empty, entry("fuse"), None).is_empty());
        assert!(empty.players.p1.hand.is_empty());
    }

    #[test]
    fn r1242_copy_of_carries_r57s_riders_and_leaves_the_source() {
        let mut state = game("copy-of", None, None);
        let source = in_hand(&mut state, "fx-4", P1, 1);
        if let Some(live) = find_instance_mut(&mut state, &source[0].id) {
            live.radiant = true;
            live.chinese = Some(true);
            live.granted_tags = Some(vec![Tag::Cn]);
            live.stats_override = Some(AttackHealth { attack: 7, health: 9 });
            live.tuning = Some(Tuning {
                attack: Some(2),
                ..Tuning::default()
            });
            live.cost_mod = -1;
        }
        let before = card(&state, &source[0].id).clone();
        let copy = add_to_hand(AddToHandArgs {
            copy_of: Some(source[0].id.clone()),
            cost_override: Some(CHAOS_MED_COST),
            ..AddToHandArgs::default()
        });
        run(&mut state, copy, None);
        let hand = state.players.p1.hand.clone();
        assert_eq!(hand.len(), 2);
        assert_eq!(&hand[0], &before);
        let made = &hand[1];
        assert_ne!(made.id, before.id);
        assert_eq!(made.def_id, before.def_id);
        assert!(made.radiant);
        assert_eq!(made.chinese, Some(true));
        assert_eq!(made.granted_tags, Some(vec![Tag::Cn]));
        assert_eq!(made.stats_override, before.stats_override);
        assert_eq!(made.tuning, before.tuning);
        // The price is the copy's own rider, not the source's.
        assert_eq!(made.cost_mod, 0);
        assert_eq!(made.cost_override, Some(CHAOS_MED_COST));
        // A copy of a card on the field leaves it there; a copy of no card is nothing.
        let unit = put(&mut state, "fx-5", slot(P1, Row::Units, 2), json!({}));
        run(
            &mut state,
            add_to_hand(AddToHandArgs {
                copy_of: Some(unit.id.clone()),
                ..AddToHandArgs::default()
            }),
            None,
        );
        assert_eq!(card(&state, &unit.id).zone.z(), ZoneName::Field);
        assert_eq!(state.players.p1.hand.len(), 3);
        assert_eq!(state.players.p1.hand[2].def_id, "fx-5");
        let events = run(
            &mut state,
            add_to_hand(AddToHandArgs {
                copy_of: Some("c-none".to_string()),
                ..AddToHandArgs::default()
            }),
            None,
        );
        assert!(events.is_empty());
        assert_eq!(state.players.p1.hand.len(), 3);
    }

    /// Move a field card into its owner's hand directly, keeping its radiant flag (a test's shortcut).
    fn bounce_to_hand(state: &mut GameState, id: &str) {
        let mut sink = sink_for_state(state);
        let found = find_instance(sink.state, id).cloned();
        if let Some(found) = found {
            bounce_card(&mut sink, &found);
        }
    }

    fn sink_for_state(state: &mut GameState) -> EngineSink<'_> {
        crate::rules::fixtures::harness::sink_for(state)
    }
}

mod the_other_entries {
    use super::*;

    #[test]
    fn entry_cn_and_prime_add_cards_of_their_tag_at_zero() {
        let mut state = game("cn", None, None);
        run(&mut state, entry("cn"), None);
        let hand = state.players.p1.hand.clone();
        assert_eq!(hand.len() as i32, CHAOS_MED_CN_CARDS);
        assert!(hand.iter().all(|each| each.def_id == CN.id));
        assert!(hand.iter().all(|each| each.cost_override == Some(CHAOS_MED_COST)));

        // R1421: only tokens carry Prime, and the Prime pool holds them.
        let mut state = game("prime", None, None);
        run(&mut state, entry("prime"), None);
        let hand = state.players.p1.hand.clone();
        assert_eq!(hand.len() as i32, CHAOS_MED_PRIME_CARDS);
        assert!(hand.iter().all(|each| each.def_id == PRIME.id));
        assert!(hand.iter().all(|each| each.cost_override == Some(CHAOS_MED_COST)));
    }

    #[test]
    fn entry_armor_gives_8_hero_armor_and_heals_8_past_30() {
        let mut state = game("armor", None, None);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
        run(&mut state, entry("armor"), None);
        assert_eq!(state.players.p1.hero.armor, CHAOS_MED_HERO_ARMOR);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH + CHAOS_MED_HEAL);
        assert_eq!(state.players.p2.hero.armor, 0);
    }

    #[test]
    fn entries_jade_and_golem_summon_their_tokens_by_index() {
        let mut state = game("tokens", None, None);
        put(&mut state, "fx-1", slot(P1, Row::Units, 1), json!({}));
        let jade = run(&mut state, entry("jade"), None);
        let golem = run(&mut state, entry("golem"), None);
        assert_eq!(
            field_of(&jade, GameEventType::Summoned, "defId"),
            vec![json!(JADE.id.clone())]
        );
        assert_eq!(
            field_of(&golem, GameEventType::Summoned, "defId"),
            vec![json!(GOLEM.id.clone())]
        );
        // The Jade Beauty is its base face (the entry touches no Jade Counter).
        let summoned = field_of(&jade, GameEventType::Summoned, "instanceId");
        assert!(!card(&state, summoned[0].as_str().unwrap_or_default()).radiant);
    }

    #[test]
    fn r1243_three_acclaimed_permanents_never_an_acclaimed_token() {
        for i in 0..4 {
            let mut state = game(&format!("acclaimed-{i}"), None, None);
            let events = run(&mut state, entry("acclaimed"), None);
            let summoned = strings_of(&field_of(&events, GameEventType::Summoned, "defId"));
            assert_eq!(summoned.len() as i32, CHAOS_MED_ACCLAIMED);
            assert!(summoned.iter().all(|id| *id == ACCLAIMED.id));
            assert!(
                !summoned
                    .iter()
                    .any(|id| *id == ACCLAIMED_TOKEN.id || *id == ACCLAIMED_SPELL.id)
            );
        }
    }

    #[test]
    fn r1244_every_enemy_permanent_bounced_and_each_that_reached_a_hand_nerfed_once() {
        let mut state = game("bounce", None, None);
        let unit = put(&mut state, "fx-2", slot(P2, Row::Units, 1), json!({}));
        let face_down = put(&mut state, &trap.id, slot(P2, Row::Backrow, 1), json!({}));
        let token = put(&mut state, &GOLEM.id, slot(P2, Row::Units, 2), json!({}));
        let mine = put(&mut state, "fx-3", slot(P1, Row::Units, 1), json!({}));
        let events = run(&mut state, entry("bounce"), None);
        assert_eq!(
            sorted(strings_of(&field_of(
                &events,
                GameEventType::Bounced,
                "instanceId"
            ))),
            sorted(vec![unit.id.clone(), face_down.id.clone(), token.id.clone()])
        );
        assert_eq!(
            sorted(ids_of(&state.players.p2.hand)),
            sorted(vec![unit.id.clone(), face_down.id.clone()])
        );
        // R11: the token ceased to exist, so it is no card to Nerf.
        assert!(find_instance(&state, &token.id).is_none());
        assert_eq!(card(&state, &mine.id).zone.z(), ZoneName::Field);
        let nerfed = strings_of(&field_of(&events, GameEventType::Degraded, "instanceId"));
        assert_eq!(
            sorted(nerfed.clone()),
            sorted(vec![unit.id.clone(), face_down.id.clone()])
        );
        assert_eq!(nerfed.len() as i32, 2 * CHAOS_MED_NERFS);
        // Hidden in their hand (R177, R440): the Nerfs read to the caster as no card.
        state.applied = vec![AppliedAction {
            nonce: "bounce".into(),
            events,
        }];
        let read: Vec<Value> = view_for(&state, P1)
            .events
            .iter()
            .filter(|event| event.event_type() == GameEventType::Degraded)
            .map(json_of)
            .collect();
        assert_eq!(read.len(), 2);
        assert!(read.iter().all(|event| event["instanceId"] == json!(HIDDEN_ID)));
    }

    #[test]
    fn r1246_shuffle_into_writes_its_memory_on_each_new_card_and_it_survives_json() {
        let mut state = game("memory", None, None);
        let remembered = json!(["c-a", "c-b"]);
        run(
            &mut state,
            shuffle_into(json_as(json!({
                "defId": "fx-6",
                "count": 2,
                "memory": { "journey": remembered },
            }))),
            None,
        );
        let library = state.players.p1.library.clone();
        assert_eq!(library.len(), 2);
        assert!(
            library
                .iter()
                .all(|each| each.memory.get("journey") == Some(&remembered))
        );
        let round: GameState = serde_json::from_value(json_of(&state)).unwrap();
        assert_eq!(round, state);
        assert_eq!(hash_state(&round), hash_state(&state));
        // Without it, a card shuffled in remembers nothing.
        run(
            &mut state,
            shuffle_into(json_as(json!({ "defId": "fx-6", "count": 1 }))),
            None,
        );
        assert_eq!(
            state
                .players
                .p1
                .library
                .iter()
                .filter(|each| each.memory.is_empty())
                .count(),
            1
        );
    }
}
