//! Fuse and Craft a Card: SPEC §6.3's Fuse row as R77 spells it out, and BUILD M3-T7's `fuse.ts`
//! bullet. One test per clause of R77, in the order the ruling writes them.
//!
//! R77, verbatim: "Fuse creates a transient definition. Its base form sums the ingredients' base
//! attack and health, unions their base keywords and tags, and concatenates their base scripts; its
//! radiant form does the same with their radiant forms. Its cost is min(sum of the printed costs per
//! R65, 4). Its type is the target's, or the ingredients' shared type when there is no target on the
//! field (Field Trap if any ingredient is one). Radiant Unlicensed Experimentation fuses the played
//! permanent onto each matching permanent separately, one fusion at a time. One ingredient may be a
//! target already on the field: the result then keeps that instance, with its zone, position, damage,
//! exertion, summonedTurn, counters, memory and radiant flag, and only the other ingredients cease to
//! exist, without a Death trigger and without counting as destroyed. The result's buffs are the sum
//! of every ingredient's buffs and its granted keywords their union; every other field of the kept
//! instance is unchanged, `statsOverride` and the Vanilla flag included. A fused trap has every
//! ingredient's trigger condition, runs only the script whose condition was met, and is consumed
//! unless it is a Field Trap. Craft a Card fuses two or three cards with no target on the field, and
//! its result is a fresh, non-Radiant hand card with `costOverride` 0."
//!
//! The fixtures are local (`fu-` ids, indexes from 1501, so nothing collides with another test
//! file's) because no shared fixture expresses what the two-face clause needs: an ingredient whose
//! radiant stats are deliberately *not* double its base, so "the radiant form does the same with
//! their radiant forms" is observable rather than a coincidence of doubling.
//!
//! TS's tests read the live objects `fuse` changed (`result`, `food.zone`); Rust reads the kept card
//! back from the state by its id, and an ingredient that ceased to exist (`{ z: "gone" }`, R86) is one
//! the state holds in no pile.
//!
//! Port of `packages/engine/test/fuse.test.ts`.

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::damage;
use jackioh_engine::layers::unit_view;
use jackioh_engine::mana::{effective_cost, printed_cost};
use jackioh_engine::resolve::{HookName, HookOptions, run_hook};
use jackioh_engine::scripts::script_of;
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;
use jackioh_engine::traps::fire_traps_for;
use jackioh_engine::wire::PlayerId::{P1, P2};
use jackioh_engine::zones::card_at;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixtures.
// ---------------------------------------------------------------------------

/// TS's `def`, its running `nextIndex` (from 1500) passed as `index`.
fn def(name: &str, index: i32, type_: &str, extra: Value) -> CardDef {
    let mut card = json!({
        "id": format!("fu-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (fuse)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": format!("{name} base") },
        "radiant": { "keywords": [], "text": format!("{name} radiant") },
    });
    if let (Some(card), Some(extra)) = (card.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            card.insert(key.clone(), value.clone());
        }
    }
    json_as(card)
}

/// Half of every fusion below. 2/3 Taunt on the base face and 3/9 Taunt + Divine Shield on the
/// radiant one: the radiant stats are neither double the base nor the same keywords, so a fused
/// radiant face built out of base forms would read differently from one built out of radiant forms.
fn ingredient_a() -> CardDef {
    def(
        "ingredient-a",
        1501,
        "Unit",
        json!({
            "tags": ["Human"],
            "rarity": "Rare",
            "cost": 2,
            "base": { "attack": 2, "health": 3, "keywords": [{ "kind": "Taunt" }], "text": "2/3 Taunt" },
            "radiant": {
                "attack": 3,
                "health": 9,
                "keywords": [{ "kind": "Taunt" }, { "kind": "Divine Shield" }],
                "text": "3/9 Taunt, Divine Shield",
            },
        }),
    )
}

/// The other half: 1/1 Rush, radiant 5/2 Rush + Cleave — again not a doubling.
fn ingredient_b() -> CardDef {
    def(
        "ingredient-b",
        1502,
        "Unit",
        json!({
            "tags": ["Felinor"],
            "cost": 1,
            "base": { "attack": 1, "health": 1, "keywords": [{ "kind": "Rush" }], "text": "1/1 Rush" },
            "radiant": { "attack": 5, "health": 2, "keywords": [{ "kind": "Rush" }, { "kind": "Cleave" }], "text": "5/2 Rush, Cleave" },
        }),
    )
}

/// A 4-cost body, so one fusion's printed costs sum past FUSE_COST_CAP.
fn pricey() -> CardDef {
    def(
        "pricey",
        1503,
        "Unit",
        json!({
            "cost": 4,
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "1/1" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "2/2" },
        }),
    )
}

/// R65: an X-cost card's printed cost is the X on the instance.
fn x_unit() -> CardDef {
    def(
        "x-unit",
        1504,
        "Unit",
        json!({
            "cost": "X",
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "1/1 X" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "2/2 X" },
        }),
    )
}

/// R65: an embiggen card's printed cost is the price it was played for.
fn embiggen_unit() -> CardDef {
    def(
        "embiggen-unit",
        1505,
        "Unit",
        json!({
            "cost": { "base": 2, "embiggen": 4 },
            "base": { "attack": 1, "health": 1, "keywords": [], "text": "1/1 embiggen" },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": "2/2 embiggen" },
        }),
    )
}

/// A Field Spell ingredient, so "its type is the target's" has a type to override.
fn fieldy() -> CardDef {
    def("fieldy", 1506, "Field Spell", json!({ "cost": 1 }))
}

/// Craft a Card's ingredients: plain Spells, two or three of them (#99).
fn spell_a() -> CardDef {
    def("spell-a", 1507, "Spell", json!({}))
}
fn spell_b() -> CardDef {
    def("spell-b", 1508, "Spell", json!({}))
}
fn spell_c() -> CardDef {
    def("spell-c", 1509, "Spell", json!({}))
}

/// Two traps whose conditions are different events, and one Field Trap (§5.1).
fn trap_played() -> CardDef {
    def("trap-played", 1510, "Trap", json!({ "cost": 1 }))
}
fn trap_attack() -> CardDef {
    def("trap-attack", 1511, "Trap", json!({ "cost": 1 }))
}
fn field_trap_played() -> CardDef {
    def("field-trap-played", 1512, "Field Trap", json!({ "cost": 1 }))
}

fn defs() -> Vec<CardDef> {
    vec![
        ingredient_a(),
        ingredient_b(),
        pricey(),
        x_unit(),
        embiggen_unit(),
        fieldy(),
        spell_a(),
        spell_b(),
        spell_c(),
        trap_played(),
        trap_attack(),
        field_trap_played(),
    ]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

/// Deal `amount` to the enemy hero, the one visible thing a concatenated script list can do.
fn hit(amount: i32) -> Option<Hook> {
    Some(hook(move |_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]))
}

/// A trap whose one trigger, on `on`, deals `amount` to the enemy hero.
fn trap(id: &str, on: GameEventType, amount: i32) -> CardScripts {
    both(Script {
        triggers: vec![TriggerDef::new(id, &[on], move |_ctx, _event| {
            vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))]
        })],
        ..Script::default()
    })
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        // The base texts hit for 1 and 2 and the base Deaths for 3 and 4; the radiant texts hit for 10
        // and 20, so which pair of scripts a fusion concatenated is readable off the hero's health.
        (
            ingredient_a().id,
            CardScripts {
                base: Script { cry: hit(1), death: hit(3), ..Script::default() },
                radiant: Script { cry: hit(10), death: hit(30), ..Script::default() },
            },
        ),
        (
            ingredient_b().id,
            CardScripts {
                base: Script { cry: hit(2), death: hit(4), ..Script::default() },
                radiant: Script { cry: hit(20), death: hit(40), ..Script::default() },
            },
        ),
        (trap_played().id, trap("on-play", GameEventType::CardPlayed, 1)),
        (trap_attack().id, trap("on-attack", GameEventType::AttackDeclared, 2)),
        (field_trap_played().id, trap("on-play", GameEventType::CardPlayed, 5)),
    ]
}

/// A game whose catalog and script registry also carry this file's fixtures (BUILD §0).
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for entry in defs() {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    let mut registered = registered_scripts().clone();
    registered.extend(scripts());
    register_scripts(registered);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn must(card: Option<CardInstance>, what: &str) -> CardInstance {
    card.unwrap_or_else(|| panic!("expected {what}"))
}

fn keyword_kinds(keywords: &[Keyword]) -> Vec<String> {
    let mut kinds: Vec<String> = keywords
        .iter()
        .filter_map(|keyword| serde_json::to_value(keyword).ok())
        .filter_map(|keyword| keyword["kind"].as_str().map(str::to_string))
        .collect();
    kinds.sort();
    kinds
}

fn json_of<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// vitest's `toMatchObject`: every key the expected object names matches, recursively.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(a, e)| matches_object(a, e))
        }
        _ => actual == expected,
    }
}

/// TS `sinkFor(state)`: a sink whose rng starts at the state's cursor, as reduce does. The events and
/// the rng are kept beside the state, so the test can change the state between calls as TS's shared
/// objects let it.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Sink {
    fn for_state(state: &GameState) -> Sink {
        Sink {
            events: Vec::new(),
            rng: Rng::new(&state.seed, state.rng_cursor),
        }
    }

    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }
}

/// `fuse(sink, args)`, `args` the TS object literal (its instances as the state holds them now).
fn fuse_in(state: &mut GameState, sink: &mut Sink, args: Value) -> Option<CardInstance> {
    let args: FuseArgs = json_as(args);
    fuse(&mut sink.on(state), args)
}

/// `fuse(sinkFor(state), args)`.
fn fuse_fresh(state: &mut GameState, args: Value) -> Option<CardInstance> {
    let mut sink = Sink::for_state(state);
    fuse_in(state, &mut sink, args)
}

/// The card under `id` as the state holds it now (TS's live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("no card {id}"))
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// TS `put(state, defId, ref, { radiant: true })`: the card placed, then made Radiant.
fn put_radiant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let card = put(state, def_id, at, json!({}));
    live_mut(state, &card.id).radiant = true;
    live(state, &card.id)
}

fn first_in_hand(state: &mut GameState, def_id: &str, what: &str) -> CardInstance {
    in_hand(state, def_id, P1, 1)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("expected {what}"))
}

fn sorted_keys(defs: &IndexMap<String, CardDef>) -> Vec<String> {
    let mut keys: Vec<String> = defs.keys().cloned().collect();
    keys.sort();
    keys
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

/// `(attack, maxHealth)` as §10.4's layers read the card.
fn stats(state: &GameState, card: &CardInstance) -> (i32, i32) {
    let view = unit_view(state, card);
    (view.attack, view.max_health)
}

fn played_by_p2() -> GameEvent {
    json_as(json!({
        "type": "cardPlayed",
        "player": "p2",
        "instanceId": "c999",
        "defId": ingredient_a().id,
        "costPaid": 1,
    }))
}

// ---------------------------------------------------------------------------
// The transient definition.
// ---------------------------------------------------------------------------

mod fuse_the_transient_definition_r77_m3_t7 {
    use super::*;

    #[test]
    fn r77_fuse_creates_a_transient_definition_and_registers_its_new_def_id_in_state_transient_defs() {
        let mut state = game("fuse-transient");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({}));
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({}));

        assert!(state.transient_defs.is_empty());
        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );

        // A def id that is not a catalog id: it lives in the state, where `defOf` finds it first (§10.1).
        assert_ne!(result.def_id, ingredient_a().id);
        assert_eq!(sorted_keys(&state.transient_defs), vec![result.def_id.clone()]);
        assert_eq!(
            state.transient_defs.get(&result.def_id).map(|def| def.id.clone()),
            Some(result.def_id.clone())
        );
        assert_eq!(
            json_of(def_of(Some(&state), &result.def_id)),
            json_of(&state.transient_defs[&result.def_id])
        );

        // A second fusion is its own definition, so the first one is never edited (§10.1).
        let other = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 3), json!({}));
        let other_food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 4), json!({}));
        let second = must(
            fuse_fresh(&mut state, json!({ "ingredients": [other, other_food], "target": other })),
            "a second fusion",
        );
        assert_ne!(second.def_id, result.def_id);
        assert_eq!(
            sorted_keys(&state.transient_defs),
            sorted(vec![result.def_id.clone(), second.def_id.clone()])
        );
    }

    #[test]
    fn r77_sums_the_ingredients_base_attack_and_health_and_does_the_same_with_the_radiant_forms() {
        let mut state = game("fuse-both-faces");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({})); // base 2/3, radiant 3/9
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({})); // base 1/1, radiant 5/2

        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );
        let fused = def_of(Some(&state), &result.def_id).clone();

        // Each face is built from that face of every ingredient, never from the base forms twice.
        assert_eq!((fused.base.attack, fused.base.health), (Some(3), Some(4)));
        assert_eq!((fused.radiant.attack, fused.radiant.health), (Some(8), Some(11)));

        // And the layers read them (§10.4 layer 1): the running face is the instance's.
        assert!(!result.radiant);
        assert_eq!(stats(&state, &live(&state, &result.id)), (3, 4));
        live_mut(&mut state, &result.id).radiant = true;
        assert_eq!(stats(&state, &live(&state, &result.id)), (8, 11));
    }

    #[test]
    fn r77_unions_the_base_keywords_and_the_tags_and_takes_the_radiant_keywords_from_the_radiant_forms() {
        let mut state = game("fuse-keywords");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({})); // Taunt / Taunt + Divine Shield
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({})); // Rush / Rush + Cleave

        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );
        let fused = def_of(Some(&state), &result.def_id).clone();

        assert_eq!(keyword_kinds(&fused.base.keywords), vec!["Rush", "Taunt"]);
        assert_eq!(
            keyword_kinds(&fused.radiant.keywords),
            vec!["Cleave", "Divine Shield", "Rush", "Taunt"]
        );
        // A union, so the keyword both ingredients share appears once.
        assert_eq!(keyword_kinds(&fused.base.keywords).iter().filter(|kind| *kind == "Taunt").count(), 1);
        let mut tags: Vec<String> = json_of(&fused.tags)
            .as_array()
            .map(|tags| tags.iter().filter_map(|tag| tag.as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        tags.sort();
        assert_eq!(tags, vec!["Felinor", "Human"]);
    }

    #[test]
    fn r77_concatenates_the_base_scripts_so_both_ingredients_cry_and_death_lists_run() {
        let mut state = game("fuse-scripts");
        let mut sink = Sink::for_state(&state);
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({})); // Cry 1, Death 3
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({})); // Cry 2, Death 4

        let result = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );
        let fused_scripts = script_of(&state, result.def_id.as_str());
        assert!(fused_scripts.base.cry.is_some());
        assert!(fused_scripts.base.death.is_some());

        let card = live(&state, &result.id);
        run_hook(&mut sink.on(&mut state), &card, HookName::Cry, HookOptions::default());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3); // 1 + 2

        let card = live(&state, &result.id);
        run_hook(&mut sink.on(&mut state), &card, HookName::Death, HookOptions::default());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 3 - 7); // 3 + 4
    }

    #[test]
    fn r77_concatenates_the_radiant_scripts_separately_so_a_radiant_fusion_runs_the_radiant_texts() {
        let mut state = game("fuse-radiant-scripts");
        let mut sink = Sink::for_state(&state);
        let target = put_radiant(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1)); // radiant Cry 10
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({})); // radiant Cry 20

        let result = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );
        assert!(result.radiant);

        // The running face is the radiant one (§5.2), and it is the radiant texts that were fused.
        let card = live(&state, &result.id);
        run_hook(&mut sink.on(&mut state), &card, HookName::Cry, HookOptions::default());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 30); // 10 + 20

        // The base face of the same definition still holds the base pair.
        live_mut(&mut state, &result.id).radiant = false;
        let card = live(&state, &result.id);
        run_hook(&mut sink.on(&mut state), &card, HookName::Cry, HookOptions::default());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 30 - 3); // 1 + 2
    }

    #[test]
    fn r77_costs_min_sum_of_the_printed_costs_fuse_cost_cap() {
        let mut under = game("fuse-cost-under");
        let target_a = put(&mut under, &ingredient_a().id, slot(P1, Row::Units, 1), json!({})); // cost 2
        let food_a = put(&mut under, &ingredient_b().id, slot(P1, Row::Units, 2), json!({})); // cost 1
        let cheap = must(
            fuse_fresh(&mut under, json!({ "ingredients": [target_a, food_a], "target": target_a })),
            "a fusion",
        );
        assert_eq!(json_of(&def_of(Some(&under), &cheap.def_id).cost), json!(3));
        assert!(3 < FUSE_COST_CAP);

        let mut over = game("fuse-cost-capped");
        let target_b = put(&mut over, &ingredient_a().id, slot(P1, Row::Units, 1), json!({})); // cost 2
        let food_b = put(&mut over, &pricey().id, slot(P1, Row::Units, 2), json!({})); // cost 4
        let capped = must(
            fuse_fresh(&mut over, json!({ "ingredients": [target_b, food_b], "target": target_b })),
            "a fusion",
        );
        assert_eq!(json_of(&def_of(Some(&over), &capped.def_id).cost), json!(std::cmp::min(2 + 4, FUSE_COST_CAP)));
        assert_eq!(json_of(&def_of(Some(&over), &capped.def_id).cost), json!(FUSE_COST_CAP));
    }

    #[test]
    fn r77_sums_the_printed_costs_per_r65_an_x_card_counts_its_x_and_an_embiggen_card_its_price() {
        let mut state = game("fuse-cost-r65");
        let target = put(&mut state, &x_unit().id, slot(P1, Row::Units, 1), json!({}));
        live_mut(&mut state, &target.id).x = Some(1);
        let target = live(&state, &target.id);
        let embiggen = first_in_hand(&mut state, &embiggen_unit().id, "an embiggen ingredient");

        assert_eq!(printed_cost(&state, &target), 1);
        assert_eq!(printed_cost(&state, &embiggen), 2);
        let base = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, embiggen], "target": target })),
            "a fusion",
        );
        assert_eq!(json_of(&def_of(Some(&state), &base.def_id).cost), json!(3));

        // The chosen embiggen price is the printed cost of the card that was played for it (R65).
        let mut other = game("fuse-cost-r65-embiggened");
        let target2 = put(&mut other, &x_unit().id, slot(P1, Row::Units, 1), json!({}));
        live_mut(&mut other, &target2.id).x = Some(1);
        let target2 = live(&other, &target2.id);
        let bigger = first_in_hand(&mut other, &embiggen_unit().id, "an embiggen ingredient");
        live_mut(&mut other, &bigger.id).embiggened = Some(true);
        let bigger = live(&other, &bigger.id);
        assert_eq!(printed_cost(&other, &bigger), 4);
        let capped = must(
            fuse_fresh(&mut other, json!({ "ingredients": [target2, bigger], "target": target2 })),
            "a fusion",
        );
        assert_eq!(json_of(&def_of(Some(&other), &capped.def_id).cost), json!(std::cmp::min(1 + 4, FUSE_COST_CAP)));
    }

    #[test]
    fn r77_takes_the_targets_type_when_an_ingredient_is_a_target_on_the_field() {
        let mut state = game("fuse-type-target");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({})); // Unit
        let food = first_in_hand(&mut state, &fieldy().id, "a Field Spell ingredient"); // Field Spell

        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );
        assert_eq!(json_of(&def_of(Some(&state), &result.def_id).type_), json!("Unit"));
    }

    #[test]
    fn r77_takes_the_ingredients_shared_type_when_there_is_no_target_on_the_field() {
        let mut state = game("fuse-type-shared");
        let a = first_in_hand(&mut state, &spell_a().id, "a Spell");
        let b = first_in_hand(&mut state, &spell_b().id, "another Spell");

        let crafted = must(fuse_fresh(&mut state, json!({ "ingredients": [a, b], "toHand": "p1" })), "a crafted card");
        assert_eq!(json_of(&def_of(Some(&state), &crafted.def_id).type_), json!("Spell"));
    }

    #[test]
    fn r77_makes_the_result_a_field_trap_when_an_ingredient_is_one_and_no_target_is_on_the_field() {
        let mut state = game("fuse-type-field-trap");
        let plain_trap = first_in_hand(&mut state, &trap_played().id, "a Trap");
        let field = first_in_hand(&mut state, &field_trap_played().id, "a Field Trap");

        let crafted = must(
            fuse_fresh(&mut state, json!({ "ingredients": [plain_trap, field], "toHand": "p1" })),
            "a crafted card",
        );
        assert_eq!(json_of(&def_of(Some(&state), &crafted.def_id).type_), json!("Field Trap"));

        // Two plain Traps share their type and stay one (§5.1).
        let mut two = game("fuse-type-two-traps");
        let first = first_in_hand(&mut two, &trap_played().id, "a Trap");
        let second = first_in_hand(&mut two, &trap_attack().id, "another Trap");
        let plain_result = must(
            fuse_fresh(&mut two, json!({ "ingredients": [first, second], "toHand": "p1" })),
            "a crafted card",
        );
        assert_eq!(json_of(&def_of(Some(&two), &plain_result.def_id).type_), json!("Trap"));
    }
}

// ---------------------------------------------------------------------------
// The kept instance, and the ingredients that cease to exist.
// ---------------------------------------------------------------------------

mod fuse_the_instance_the_result_keeps_r77_m3_t7 {
    use super::*;

    #[test]
    fn r77_keeps_the_target_instance_with_its_zone_position_damage_exertion_summoned_turn_counters_memory_and_radiant_flag()
     {
        let mut state = game("fuse-keeps-instance");
        let target = put_radiant(&mut state, &ingredient_a().id, slot(P1, Row::Units, 3));
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 1), json!({}));

        {
            let card = live_mut(&mut state, &target.id);
            card.position = Some(Position::Def);
            card.damage = 2;
            card.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
            card.summoned_turn = Some(2);
            card.counters = json_as(json!({ "plague": 3, "grade": 1 }));
            card.memory = json_as(json!({ "meal": "felinor" }));
        }
        let target = live(&state, &target.id);

        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );

        // The result *is* that card: same instance id, still in its own zone.
        assert_eq!(result.id, target.id);
        assert_eq!(
            result.zone,
            Zone::Field {
                player: P1,
                row: Row::Units,
                lane: 3,
            }
        );
        assert_eq!(
            card_at(&state, slot(P1, Row::Units, 3)).map(|card| card.id.clone()),
            Some(target.id.clone())
        );
        assert_eq!(result.position, Some(Position::Def));
        assert_eq!(result.damage, 2);
        assert_eq!(
            result.exertion,
            Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            }
        );
        assert_eq!(result.summoned_turn, Some(2));
        assert_eq!(json_of(&result.counters), json!({ "plague": 3, "grade": 1 }));
        assert_eq!(json_of(&result.memory), json!({ "meal": "felinor" }));
        assert!(result.radiant);
    }

    #[test]
    fn r77_sums_every_ingredients_buffs_and_unions_their_granted_keywords() {
        let mut state = game("fuse-buffs");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({}));
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({}));

        {
            let card = live_mut(&mut state, &target.id);
            card.buffs = AttackHealth { attack: 1, health: 2 };
            card.granted_keywords = json_as(json!([{ "kind": "Lifesteal" }, { "kind": "Taunt" }]));
        }
        {
            let card = live_mut(&mut state, &food.id);
            card.buffs = AttackHealth { attack: 3, health: -1 };
            card.granted_keywords = json_as(json!([{ "kind": "Taunt" }, { "kind": "Charge" }]));
        }
        let target = live(&state, &target.id);
        let food = live(&state, &food.id);

        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );

        assert_eq!(result.buffs, AttackHealth { attack: 4, health: 1 });
        assert_eq!(keyword_kinds(&result.granted_keywords), vec!["Charge", "Lifesteal", "Taunt"]);
        // Layer 4 sits on top of the fused printed stats (§10.4): 3/4 printed plus +4/+1.
        assert_eq!(stats(&state, &live(&state, &result.id)), (7, 5));
    }

    #[test]
    fn r77_leaves_every_other_field_of_the_kept_instance_unchanged_the_vanilla_flag_included_and_sums_its_stats_override_into_the_fused_face()
     {
        let mut state = game("fuse-other-fields");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({}));
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({}));

        let turn = state.turn;
        {
            let card = live_mut(&mut state, &target.id);
            card.stats_override = Some(AttackHealth { attack: 7, health: 7 });
            card.vanilla = true;
            card.cost_mod = 2;
            card.cost_override = Some(1);
            card.divine_shield_spent = Some(true);
            card.last_damaged_by = Some("c99".to_string());
            card.face_up = Some(true);
            card.taunt_suppressed_turn = Some(turn);
        }
        let target = live(&state, &target.id);

        let result = must(
            fuse_fresh(&mut state, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );

        // §7, R175: the token's X/X is its printed face, so the fusion sums it (7/7 + 1/1) into the
        // fused definition, and the override leaves the instance rather than hiding that sum.
        assert!(result.stats_override.is_none());
        let fused_def = state.transient_defs.get(&result.def_id);
        assert!(matches_object(
            &json_of(fused_def.map(|def| &def.base)),
            &json!({ "attack": 8, "health": 8 })
        ));
        assert!(matches_object(
            &json_of(fused_def.map(|def| &def.radiant)),
            &json!({ "attack": 12, "health": 9 })
        ));
        assert!(result.vanilla);
        assert_eq!(result.cost_mod, 2);
        assert_eq!(result.cost_override, Some(1));
        assert_eq!(result.divine_shield_spent, Some(true));
        assert_eq!(result.last_damaged_by.as_deref(), Some("c99"));
        assert_eq!(result.face_up, Some(true));
        assert_eq!(result.taunt_suppressed_turn, Some(state.turn));
        assert_eq!(result.owner, P1);
        assert_eq!(result.controller, P1);

        // Layer 1 reads the fused face, and Vanilla still clears the fused printed keywords while the
        // granted ones stay.
        let view = unit_view(&state, &live(&state, &result.id));
        assert_eq!((view.attack, view.max_health), (8, 8));
        assert!(view.keywords.is_empty());
    }

    #[test]
    fn r77_the_other_ingredients_cease_to_exist_without_a_death_trigger_and_without_counting_as_destroyed() {
        let mut state = game("fuse-ingredients-gone");
        let target = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({}));
        let food = put(&mut state, &ingredient_b().id, slot(P1, Row::Units, 2), json!({})); // Death: 4 to the enemy hero

        let before = state.counters.destroyed;
        let mut sink = Sink::for_state(&state);
        let result = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [target, food], "target": target })),
            "a fusion",
        );
        let events = sink.events;
        assert_eq!(result.id, target.id);

        // Off the field and in no pile at all: `{ z: "gone" }` (§10.1, R86).
        assert!(card_at(&state, slot(P1, Row::Units, 2)).is_none());
        assert!(find_instance(&state, &food.id).is_none());
        assert!(state.players.p1.graveyard.is_empty());
        assert!(state.players.p1.exile.is_empty());

        // Not a death and not a destruction: no events, no counter, and the Death script never ran.
        assert!(events_of_type(&events, GameEventType::Destroyed).is_empty());
        assert!(events_of_type(&events, GameEventType::EnteredGraveyard).is_empty());
        assert_eq!(state.counters.destroyed, before);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);

        // What it does emit is one `fused` event naming every ingredient and the result (§10.3).
        assert_eq!(
            json_of(events_of_type(&events, GameEventType::Fused)),
            json!([{
                "type": "fused",
                "instanceIds": [target.id, food.id],
                "resultInstanceId": target.id,
                "defId": result.def_id,
            }])
        );
    }

    #[test]
    fn r77_radiant_unlicensed_experimentation_fuses_onto_each_matching_permanent_separately_one_fusion_at_a_time() {
        // #85r's loop is the card's (M4); what the engine promises is that each fusion is its own,
        // with its own transient definition and its own kept instance, rather than one fusion of
        // everything at once.
        let mut state = game("fuse-one-at-a-time");
        let mut sink = Sink::for_state(&state);
        let first = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 1), json!({}));
        let second = put(&mut state, &ingredient_a().id, slot(P1, Row::Units, 2), json!({}));
        let played = put(&mut state, &ingredient_b().id, slot(P2, Row::Units, 1), json!({}));

        let one = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [played], "target": first })),
            "the first fusion",
        );
        assert_eq!(one.id, first.id);
        assert!(find_instance(&state, &played.id).is_none());

        // The second fusion is a separate call with a separate copy of the played permanent, and it
        // leaves the first fusion's card and definition alone.
        let again = put(&mut state, &ingredient_b().id, slot(P2, Row::Units, 2), json!({}));
        let two = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [again], "target": second })),
            "the second fusion",
        );
        assert_eq!(two.id, second.id);
        assert_ne!(two.def_id, one.def_id);
        assert_eq!(
            sorted_keys(&state.transient_defs),
            sorted(vec![one.def_id.clone(), two.def_id.clone()])
        );
        assert_eq!(stats(&state, &live(&state, &one.id)), (3, 4));
        assert_eq!(stats(&state, &live(&state, &two.id)), (3, 4));
    }
}

// ---------------------------------------------------------------------------
// Fused traps and Craft a Card.
// ---------------------------------------------------------------------------

mod fuse_traps_and_craft_a_card_r77_m3_t7 {
    use super::*;

    #[test]
    fn r77_a_fused_trap_has_every_ingredients_trigger_condition_and_runs_only_the_script_whose_condition_was_met() {
        let mut state = game("fuse-trap-conditions");
        let mut sink = Sink::for_state(&state);
        let target = put(&mut state, &trap_played().id, slot(P1, Row::Backrow, 1), json!({})); // fires on cardPlayed, for 1
        let food = first_in_hand(&mut state, &trap_attack().id, "a second trap"); // fires on attackDeclared, for 2

        let result = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [target, food], "target": target })),
            "a fused trap",
        );
        let triggers = script_of(&state, result.def_id.as_str()).base.triggers.clone();
        let mut on: Vec<String> = triggers
            .iter()
            .flat_map(|trigger| trigger.on.iter().map(|event| event.as_str().to_string()))
            .collect();
        on.sort();
        assert_eq!(on, vec!["attackDeclared", "cardPlayed"]);
        // Two conditions, so two distinct trigger ids survive the fusion.
        let ids: IndexSet<String> = triggers.iter().map(|trigger| trigger.id.clone()).collect();
        assert_eq!(ids.len(), 2);

        // Only the script whose condition was met runs: the `cardPlayed` half hits for 1, not for 3.
        fire_traps_for(&mut sink.on(&mut state), &played_by_p2());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH - 1);
    }

    #[test]
    fn r77_a_fused_trap_is_consumed_unless_it_is_a_field_trap() {
        let mut consumed = game("fuse-trap-consumed");
        let mut consumed_sink = Sink::for_state(&consumed);
        let plain_target = put(&mut consumed, &trap_played().id, slot(P1, Row::Backrow, 1), json!({}));
        let extra = first_in_hand(&mut consumed, &trap_attack().id, "a second trap");
        let plain_result = must(
            fuse_in(
                &mut consumed,
                &mut consumed_sink,
                json!({ "ingredients": [plain_target, extra], "target": plain_target }),
            ),
            "a fused Trap",
        );
        assert_eq!(json_of(&def_of(Some(&consumed), &plain_result.def_id).type_), json!("Trap"));

        fire_traps_for(&mut consumed_sink.on(&mut consumed), &played_by_p2());
        assert!(card_at(&consumed, slot(P1, Row::Backrow, 1)).is_none());
        assert_eq!(
            consumed.players.p1.graveyard.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            vec![plain_result.id.clone()]
        );

        // A Field Trap target keeps the type, so the fused trap stays and can fire again (§5.1).
        let mut stays = game("fuse-field-trap-stays");
        let mut stays_sink = Sink::for_state(&stays);
        let field_target = put(&mut stays, &field_trap_played().id, slot(P1, Row::Backrow, 1), json!({}));
        let other = first_in_hand(&mut stays, &trap_attack().id, "a second trap");
        let field_result = must(
            fuse_in(
                &mut stays,
                &mut stays_sink,
                json!({ "ingredients": [field_target, other], "target": field_target }),
            ),
            "a fused Field Trap",
        );
        assert_eq!(json_of(&def_of(Some(&stays), &field_result.def_id).type_), json!("Field Trap"));

        let played = played_by_p2();
        fire_traps_for(&mut stays_sink.on(&mut stays), &played);
        fire_traps_for(&mut stays_sink.on(&mut stays), &played);
        assert_eq!(
            card_at(&stays, slot(P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(field_result.id.clone())
        );
        assert!(stays.players.p1.graveyard.is_empty());
        assert_eq!(stays.players.p2.hero.health, HERO_HEALTH - 10); // 5 twice
    }

    #[test]
    fn r77_craft_a_card_fuses_two_or_three_cards_with_no_target_giving_a_fresh_non_radiant_hand_card_with_cost_override_0()
     {
        let mut state = game("fuse-craft");
        let mut sink = Sink::for_state(&state);
        let a = first_in_hand(&mut state, &spell_a().id, "a Spell");
        let b = first_in_hand(&mut state, &spell_b().id, "another Spell");
        live_mut(&mut state, &a.id).radiant = true; // a Radiant ingredient still crafts a non-Radiant result
        let a = live(&state, &a.id);

        let two = must(
            fuse_in(&mut state, &mut sink, json!({ "ingredients": [a, b], "toHand": "p1" })),
            "a crafted card",
        );
        assert_ne!(two.id, a.id);
        assert_ne!(two.id, b.id);
        assert!(!two.radiant);
        assert_eq!(two.cost_override, Some(0));
        assert_eq!(effective_cost(&state, &two, Default::default()), 0);
        assert_eq!(two.zone, Zone::Hand { player: P1 });
        assert_eq!(
            state.players.p1.hand.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            vec![two.id.clone()]
        );
        assert!(find_instance(&state, &a.id).is_none());
        assert!(find_instance(&state, &b.id).is_none());

        // Three cards craft the same way, and the cost is still min(sum, cap).
        let mut three = game("fuse-craft-three");
        let x = first_in_hand(&mut three, &spell_a().id, &format!("{} in hand", spell_a().id));
        let y = first_in_hand(&mut three, &spell_b().id, &format!("{} in hand", spell_b().id));
        let z = first_in_hand(&mut three, &spell_c().id, &format!("{} in hand", spell_c().id));
        let crafted = must(
            fuse_fresh(&mut three, json!({ "ingredients": [x, y, z], "toHand": "p1" })),
            "a crafted card",
        );
        assert_eq!(json_of(&def_of(Some(&three), &crafted.def_id).cost), json!(std::cmp::min(3, FUSE_COST_CAP)));
        assert_eq!(crafted.cost_override, Some(0));
        assert_eq!(
            three.players.p1.hand.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            vec![crafted.id.clone()]
        );

        // Fewer than two cards is not a fusion, and neither is one with nowhere to put the result.
        let mut lone = game("fuse-craft-one");
        let only = first_in_hand(&mut lone, &spell_a().id, "a Spell");
        let pair = first_in_hand(&mut lone, &spell_b().id, "another Spell");
        assert!(fuse_fresh(&mut lone, json!({ "ingredients": [only], "toHand": "p1" })).is_none());
        assert!(fuse_fresh(&mut lone, json!({ "ingredients": [only, pair] })).is_none());
        assert!(lone.transient_defs.is_empty());
    }
}
