//! The library-and-hand family of §6.3: Add to hand (create OR move), the catalog add, the two
//! library exiles, and Discover with the library as the pool.
//! SPEC §6.3 (Add to hand, Exile, Discover, Cost), §2.4, §3.2, §5.1, §10.6, §10.7, §10.8;
//! R4, R11, R50, R57, R60, R65, R78.
//!
//! The fixture defs these tests need are registered here, on top of the shared fixture catalog, so
//! no shared fixture has to grow for them (CLAUDE.md, BUILD §0).
//!
//! Port of `packages/engine/test/effects-library.test.ts`.

use jackioh_engine::effects::add_to_hand::{add_random_from_catalog, add_to_hand};
use jackioh_engine::effects::choose::{discover_from_catalog, discover_from_library};
use jackioh_engine::effects::library::{exile_bottom_of_library, exile_random_from_library};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, set_library};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS `makeDef(name, overrides)`: `nextIndex` starts at 1800 and is bumped once per call, in the order
/// the TS file declares its defs; `overrides` is spread over the literal (a shallow merge).
fn make_def(name: &str, index: u32, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("lib-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (library)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    });
    if let Value::Object(fields) = overrides {
        for (key, value) in fields {
            def[key.as_str()] = value;
        }
    }
    json_as(def)
}

/// Three plain cards, so "which instance moved" is answerable by def as well as by id.
fn alpha() -> CardDef {
    make_def("alpha", 1801, json!({}))
}
fn beta() -> CardDef {
    make_def("beta", 1802, json!({}))
}
fn gamma() -> CardDef {
    make_def("gamma", 1803, json!({}))
}

/// Printed cost 4, so R65's "start from the override in place of the printed cost" is visible.
fn pricey() -> CardDef {
    make_def("pricey", 1804, json!({ "cost": 4 }))
}

/// The card whose script is running in the catalog-pool tests: §5.1 must never offer it back.
fn generator() -> CardDef {
    make_def(
        "generator",
        1805,
        json!({
            "type": "Spell",
            "base": { "keywords": [], "text": "generator" },
            "radiant": { "keywords": [], "text": "generator" },
        }),
    )
}

/// R11: a unit-token card can sit in a library (#75, and the copies of §3.2).
fn unit_token() -> CardDef {
    make_def(
        "token",
        1806,
        json!({ "id": "lib-token", "name": "Fixture Library Token", "rarity": "Token", "token": true }),
    )
}

// A pool of its own set, so `query({ set: "Boss" })` is exactly these three and nothing the shared
// fixture catalog happens to contain. Making `ctx.self` one OF the pool is what makes §5.1's "a
// random pool never offers the card that generated it" observable rather than vacuous.
fn pool_a() -> CardDef {
    make_def("pool-a", 1807, json!({ "set": "Boss" }))
}
fn pool_b() -> CardDef {
    make_def("pool-b", 1808, json!({ "set": "Boss" }))
}
fn pool_c() -> CardDef {
    make_def("pool-c", 1809, json!({ "set": "Boss" }))
}

/// A pool of exactly one card: three draws from it prove repeats are allowed (R60).
fn solo() -> CardDef {
    make_def("solo", 1810, json!({ "set": "Boss-X" }))
}

// The filter fixtures for `discoverFromLibrary` (#51's type options and cost brackets).
fn non_unit(name: &str, index: u32, type_: &str, cost: Value) -> CardDef {
    make_def(
        name,
        index,
        json!({
            "type": type_,
            "cost": cost,
            "base": { "keywords": [], "text": name },
            "radiant": { "keywords": [], "text": name },
        }),
    )
}

fn spell_two() -> CardDef {
    non_unit("spell-2", 1811, "Spell", json!(2))
}
fn spell_five() -> CardDef {
    non_unit("spell-5", 1812, "Spell", json!(5))
}
fn spell_x() -> CardDef {
    non_unit("spell-x", 1813, "Spell", json!("X"))
}
fn trap_two() -> CardDef {
    non_unit("trap-2", 1814, "Trap", json!(2))
}
fn field_trap_two() -> CardDef {
    non_unit("field-trap-2", 1815, "Field Trap", json!(2))
}
fn field_spell_two() -> CardDef {
    non_unit("field-spell-2", 1816, "Field Spell", json!(2))
}

fn defs() -> Vec<CardDef> {
    vec![
        alpha(),
        beta(),
        gamma(),
        pricey(),
        generator(),
        unit_token(),
        pool_a(),
        pool_b(),
        pool_c(),
        solo(),
        spell_two(),
        spell_five(),
        spell_x(),
        trap_two(),
        field_trap_two(),
        field_spell_two(),
    ]
}

/// A fresh game whose catalog also carries this file's fixtures.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.turn = 3;
    state
}

/// An off-board instance to be `ctx.self`: a resolving Spell, which is what #51 and #57 are.
fn resolving_self(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Resolving { player });
    state.players[player].resolving.push(card.clone());
    card
}

/// TS `run(state, effects, { controller, self })`: the effects applied in order through one context on
/// a fresh sink over `state`; its events. Like TS's, the rng is not written back.
fn run(
    state: &mut GameState,
    effects: Vec<Effect>,
    controller: Option<PlayerId>,
    self_: Option<&CardInstance>,
) -> Vec<GameEvent> {
    // TS passed the live object; here the card as it stands now.
    let self_: Option<CardInstance> = self_.map(|card| {
        find_instance(&*state, &card.id)
            .cloned()
            .unwrap_or_else(|| card.clone())
    });
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            self_.as_ref(),
            HookOptions {
                controller,
                ..Default::default()
            },
        );
        for effect in &effects {
            (effect.apply)(&mut ctx);
        }
    }
    events
}

fn must<T>(value: Option<T>, what: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected {what}"),
    }
}

fn hand_defs(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player]
        .hand
        .iter()
        .map(|card| card.def_id.clone())
        .collect()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn defs_of(defs: &[CardDef]) -> Vec<String> {
    defs.iter().map(|def| def.id.clone()).collect()
}

fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match serde_json::to_value(events).expect("events serialise") {
        Value::Array(items) => items
            .into_iter()
            .map(|item| item.get(key).cloned().unwrap_or(Value::Null))
            .collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

fn instance(id: &str) -> Value {
    json!({ "of": "instance", "instanceId": id })
}

// ---------------------------------------------------------------------------
// §6.3 Add to hand: "Creates OR MOVES the card"
// ---------------------------------------------------------------------------

mod add_to_hand_moves_an_existing_card_51_72 {
    use super::*;

    /// TS: "§6.3 moves a library card and a graveyard card into the hand, leaving no copy behind".
    #[test]
    fn moves_a_library_card_and_a_graveyard_card_into_the_hand_leaving_no_copy_behind() {
        let mut state = game("move-to-hand");
        let from_library = set_library(&mut state, PlayerId::P1, &[alpha().id, beta().id])
            .into_iter()
            .next();
        let library = must(from_library, "the library card");

        let mut buried = must(
            in_hand(&mut state, &gamma().id, PlayerId::P1, 1)
                .into_iter()
                .next(),
            "the graveyard card",
        );
        move_to_zone(
            &mut state,
            &mut buried,
            OffFieldZone::Graveyard,
            Default::default(),
        );

        // §10.6: a Discover's pick arrives in `ctx.targets`, which is what `{ of: "chosen" }` reads.
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);
        let first = run(
            &mut state,
            vec![add_to_hand(json_as(json!({ "instance": { "of": "chosen" } })))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(events_of_type(&first, GameEventType::AddedToHand).len(), 0);

        let moved = run(
            &mut state,
            vec![
                add_to_hand(json_as(json!({ "instance": instance(&library.id) }))),
                add_to_hand(json_as(json!({ "instance": instance(&buried.id) }))),
            ],
            Some(PlayerId::P1),
            None,
        );

        // One instance each, in the hand and nowhere else: a move, not a copy.
        assert_eq!(
            ids(&state.players[PlayerId::P1].hand),
            vec![library.id.clone(), buried.id.clone()]
        );
        assert_eq!(
            ids(&state.players[PlayerId::P1].library),
            vec![
                must(
                    state.players[PlayerId::P1].library.first(),
                    "the untouched library card"
                )
                .id
                .clone()
            ]
        );
        assert_eq!(state.players[PlayerId::P1].library.len(), 1);
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 0);
        assert_eq!(
            live(&state, &library.id).zone,
            Zone::Hand { player: PlayerId::P1 }
        );
        assert_eq!(live(&state, &buried.id).zone, Zone::Hand { player: PlayerId::P1 });
        assert_eq!(
            pluck(&events_of_type(&moved, GameEventType::AddedToHand), "instanceId"),
            vec![json!(library.id), json!(buried.id)]
        );
    }

    /// TS: "§6.3 does nothing when the instance it names is already in a hand".
    #[test]
    fn does_nothing_when_the_instance_it_names_is_already_in_a_hand() {
        let mut state = game("already-in-hand");
        let held = must(
            in_hand(&mut state, &alpha().id, PlayerId::P1, 1)
                .into_iter()
                .next(),
            "the held card",
        );

        let events = run(
            &mut state,
            vec![add_to_hand(json_as(json!({ "instance": instance(&held.id) })))],
            Some(PlayerId::P1),
            None,
        );

        assert_eq!(ids(&state.players[PlayerId::P1].hand), vec![held.id.clone()]);
        assert_eq!(events_of_type(&events, GameEventType::AddedToHand).len(), 0);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 0);
    }

    #[test]
    fn r65_cost_mod_adds_to_the_instance_s_cost_mod_while_cost_override_replaces_the_printed_cost() {
        let mut state = game("cost-riders");
        let library_card = set_library(&mut state, PlayerId::P1, &[pricey().id])
            .into_iter()
            .next();
        let carried = must(library_card, "the library card");
        // Something already discounted this card; the verb must stack with it, not clobber it.
        find_instance_mut(&mut state, &carried.id)
            .expect("the card is in the state")
            .cost_mod = -2;

        run(
            &mut state,
            vec![
                add_to_hand(json_as(
                    json!({ "instance": instance(&carried.id), "costMod": -1 }),
                )),
                add_to_hand(json_as(json!({ "defId": pricey().id, "costMod": -1 }))),
                add_to_hand(json_as(json!({ "defId": pricey().id, "costOverride": 1 }))),
            ],
            Some(PlayerId::P1),
            None,
        );

        let hand = state.players[PlayerId::P1].hand.clone();
        let stacked = must(hand.first(), "the moved card");
        assert_eq!(stacked.id, carried.id);
        assert_eq!(stacked.cost_mod, -3);
        assert_eq!(stacked.cost_override, None);
        assert_eq!(effective_cost(&state, stacked, Default::default()), 1); // printed 4, then -3.

        // A costMod starts from the printed cost; an override stands IN PLACE of it. Both exist because
        // only the first can stack with another discount.
        let discounted = must(hand.get(1), "the discounted card");
        assert_eq!(discounted.cost_mod, -1);
        assert_eq!(discounted.cost_override, None);
        assert_eq!(effective_cost(&state, discounted, Default::default()), 3);

        let priced = must(hand.get(2), "the overridden card");
        assert_eq!(priced.cost_mod, 0);
        assert_eq!(priced.cost_override, Some(1));
        assert_eq!(effective_cost(&state, priced, Default::default()), 1);
    }

    /// TS: "§2.4 a moved card added to a full hand is burned instead (R4)".
    #[test]
    fn r4_a_moved_card_added_to_a_full_hand_is_burned_instead() {
        let mut state = game("move-into-full-hand");
        let library_card = set_library(&mut state, PlayerId::P1, &[alpha().id])
            .into_iter()
            .next();
        let card = must(library_card, "the library card");
        in_hand(&mut state, &beta().id, PlayerId::P1, HAND_CAP);

        let events = run(
            &mut state,
            vec![add_to_hand(json_as(json!({ "instance": instance(&card.id) })))],
            Some(PlayerId::P1),
            None,
        );

        assert_eq!(state.players[PlayerId::P1].hand.len(), HAND_CAP as usize);
        assert_eq!(state.players[PlayerId::P1].library.len(), 0);
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![card.id.clone()]);
        assert_eq!(
            pluck(&events_of_type(&events, GameEventType::Burned), "instanceId"),
            vec![json!(card.id)]
        );
        assert_eq!(events_of_type(&events, GameEventType::AddedToHand).len(), 0);
    }
}

// ---------------------------------------------------------------------------
// §5.1 / §10.7 the catalog add (#54, #57, #59)
// ---------------------------------------------------------------------------

mod add_random_from_catalog_54_57_59 {
    use super::*;

    /// TS: "§5.1 never offers the card that generated the pool".
    #[test]
    fn never_offers_the_card_that_generated_the_pool() {
        let mut state = game("pool-excludes-self");

        // Four draws from a three-card pool minus the generator: only the other two can ever come up.
        let self_ = resolving_self(&mut state, &pool_a().id, PlayerId::P1);
        run(
            &mut state,
            vec![add_random_from_catalog(json_as(
                json!({ "query": { "set": "Boss" }, "count": 4 }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(hand_defs(&state, PlayerId::P1).len(), 4);
        assert!(!hand_defs(&state, PlayerId::P1).contains(&pool_a().id));
        for def_id in hand_defs(&state, PlayerId::P1) {
            assert!(defs_of(&[pool_b(), pool_c()]).contains(&def_id));
        }

        // The exclusion follows `ctx.self`, not a constant: a different generator is the excluded one.
        let mut other = game("pool-excludes-self-b");
        let other_self = resolving_self(&mut other, &pool_b().id, PlayerId::P1);
        run(
            &mut other,
            vec![add_random_from_catalog(json_as(
                json!({ "query": { "set": "Boss" }, "count": 4 }),
            ))],
            Some(PlayerId::P1),
            Some(&other_self),
        );
        assert_eq!(hand_defs(&other, PlayerId::P1).len(), 4);
        assert!(!hand_defs(&other, PlayerId::P1).contains(&pool_b().id));
    }

    #[test]
    fn r60_allows_repeats_unlike_discover_which_draws_without_replacement() {
        let mut state = game("pool-repeats");
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);

        // A one-card pool drawn three times gives three cards: with replacement, per #57's engine cell.
        run(
            &mut state,
            vec![add_random_from_catalog(json_as(
                json!({ "query": { "set": "Boss-X" }, "count": 3 }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(
            hand_defs(&state, PlayerId::P1),
            vec![solo().id, solo().id, solo().id]
        );

        // The same pool through §6.3's Discover row offers one option, not three: without replacement.
        let events = run(
            &mut state,
            vec![discover_from_catalog(json_as(
                json!({ "step": "pick", "query": { "set": "Boss-X" }, "count": 3 }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(
            must(state.pending.as_ref(), "the discover prompt").options.len(),
            1
        );
        assert_eq!(events_of_type(&events, GameEventType::PromptOpened).len(), 1);
    }

    /// TS: "§6.3 carries the radiant flag and the cost riders onto every card it creates (#54, #59)".
    #[test]
    fn carries_the_radiant_flag_and_the_cost_riders_onto_every_card_it_creates_54_59() {
        let mut state = game("pool-riders");
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);
        run(
            &mut state,
            vec![add_random_from_catalog(json_as(json!({
                "query": { "set": "Boss-X" }, "count": 2, "radiant": true, "costOverride": 0,
            })))],
            Some(PlayerId::P1),
            Some(&self_),
        );

        assert_eq!(state.players[PlayerId::P1].hand.len(), 2);
        for card in &state.players[PlayerId::P1].hand {
            assert_eq!(card.def_id, solo().id);
            assert!(card.radiant);
            assert_eq!(card.cost_override, Some(0));
        }
    }

    /// TS: "§6.3 an empty pool fizzles and the card still resolves".
    #[test]
    fn an_empty_pool_fizzles_and_the_card_still_resolves() {
        let mut state = game("pool-empty");
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);
        let events = run(
            &mut state,
            vec![add_random_from_catalog(json_as(
                json!({ "query": { "set": "Classic" }, "count": 3 }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(state.players[PlayerId::P1].hand.len(), 0);
        assert_eq!(events, Vec::<GameEvent>::new());
    }
}

// ---------------------------------------------------------------------------
// §6.3 Exile out of a library (#34, #40, #42, #65)
// ---------------------------------------------------------------------------

mod exile_random_from_library_34_42 {
    use super::*;

    #[test]
    fn r60_exiles_exactly_count_distinct_library_cards() {
        let mut state = game("exile-random");
        // `setLibrary` hands back the live pile, so the original ids are snapshotted before the move.
        let before = ids(&set_library(
            &mut state,
            PlayerId::P1,
            &[alpha().id, alpha().id, beta().id, beta().id, gamma().id],
        ));

        let events = run(
            &mut state,
            vec![exile_random_from_library(json_as(json!({ "count": 3 })))],
            Some(PlayerId::P1),
            None,
        );

        let exiled = ids(&state.players[PlayerId::P1].exile);
        assert_eq!(exiled.len(), 3);
        assert_eq!(exiled.iter().collect::<IndexSet<_>>().len(), 3);
        assert_eq!(state.players[PlayerId::P1].library.len(), 2);
        // Nothing was exiled twice and nothing is in both piles.
        for id in ids(&state.players[PlayerId::P1].library) {
            assert!(!exiled.contains(&id));
        }
        for id in &exiled {
            assert!(before.contains(id));
        }
        assert_eq!(state.counters.exiled, 3);
        let mut reported: Vec<String> = pluck(&events_of_type(&events, GameEventType::Exiled), "instanceId")
            .into_iter()
            .map(|id| id.as_str().unwrap_or_default().to_string())
            .collect();
        reported.sort();
        let mut sorted = exiled.clone();
        sorted.sort();
        assert_eq!(reported, sorted);
    }

    /// TS: "#34 exiles from the library the player argument names".
    #[test]
    fn exiles_from_the_library_the_player_argument_names_34() {
        let mut state = game("exile-random-enemy");
        set_library(&mut state, PlayerId::P1, &[alpha().id, beta().id]);
        set_library(&mut state, PlayerId::P2, &[gamma().id, gamma().id]);

        run(
            &mut state,
            vec![exile_random_from_library(json_as(
                json!({ "count": 1, "player": "enemy" }),
            ))],
            Some(PlayerId::P1),
            None,
        );

        assert_eq!(state.players[PlayerId::P1].exile.len(), 0);
        assert_eq!(state.players[PlayerId::P1].library.len(), 2);
        assert_eq!(state.players[PlayerId::P2].exile.len(), 1);
        assert_eq!(state.players[PlayerId::P2].library.len(), 1);
    }

    /// TS: "#42 fewer than count in the library exiles all of them".
    #[test]
    fn fewer_than_count_in_the_library_exiles_all_of_them_42() {
        let mut state = game("exile-random-short");
        set_library(&mut state, PlayerId::P1, &[alpha().id, beta().id]);

        run(
            &mut state,
            vec![exile_random_from_library(json_as(json!({ "count": 8 })))],
            Some(PlayerId::P1),
            None,
        );

        assert_eq!(state.players[PlayerId::P1].library.len(), 0);
        assert_eq!(state.players[PlayerId::P1].exile.len(), 2);
        assert_eq!(state.counters.exiled, 2);

        // An empty library fizzles rather than throwing, and the card still resolves (§6.3).
        let again = run(
            &mut state,
            vec![exile_random_from_library(json_as(json!({ "count": 8 })))],
            Some(PlayerId::P1),
            None,
        );
        assert_eq!(again, Vec::<GameEvent>::new());
        assert_eq!(state.counters.exiled, 2);
    }

    #[test]
    fn r11_a_unit_token_library_card_ceases_to_exist_instead_of_reaching_the_exile_pile() {
        let mut state = game("exile-random-token");
        let library = set_library(
            &mut state,
            PlayerId::P1,
            &[unit_token().id, alpha().id, beta().id],
        );
        let token = must(library.first().cloned(), "the token card");

        let events = run(
            &mut state,
            vec![exile_random_from_library(json_as(json!({ "count": 3 })))],
            Some(PlayerId::P1),
            None,
        );

        assert_eq!(state.players[PlayerId::P1].library.len(), 0);
        // R11/R86: it is in no pile and its zone says so. (The zone half, `{ z: "gone" }` on TS's live
        // object, has no Rust counterpart: a card in no pile is not reachable from the state.
        // `78f131c^:.fullsend/notes/spec-gaps-part-24-3.md`.)
        assert!(find_instance(&state, &token.id).is_none());
        assert!(!ids(&state.players[PlayerId::P1].exile).contains(&token.id));
        assert_eq!(state.players[PlayerId::P1].exile.len(), 2);
        // The counter counts only the cards that actually got there.
        assert_eq!(state.counters.exiled, 2);
        // The event still reports every card leaving, so §10.10 can animate all three.
        assert_eq!(events_of_type(&events, GameEventType::Exiled).len(), 3);
    }
}

mod exile_bottom_of_library_40_65 {
    use super::*;

    /// TS: "#40 takes the BOTTOM card of the library, not the top".
    #[test]
    fn takes_the_bottom_card_of_the_library_not_the_top_40() {
        let mut state = game("exile-bottom");
        let library = set_library(&mut state, PlayerId::P1, &[alpha().id, beta().id, gamma().id]);
        let (top, middle, bottom) = (library.first(), library.get(1), library.get(2));
        let top = must(top, "the top card").id.clone();
        let middle = must(middle, "the middle card").id.clone();
        let bottom = must(bottom, "the bottom card").id.clone();

        // `drawOne` takes library[0], so index 0 is the top and the last element is the bottom.
        let events = run(
            &mut state,
            vec![exile_bottom_of_library(json_as(json!({ "player": "self" })))],
            Some(PlayerId::P1),
            None,
        );
        assert_eq!(ids(&state.players[PlayerId::P1].exile), vec![bottom.clone()]);
        assert_eq!(
            ids(&state.players[PlayerId::P1].library),
            vec![top.clone(), middle.clone()]
        );
        assert_eq!(
            pluck(&events_of_type(&events, GameEventType::Exiled), "instanceId"),
            vec![json!(bottom)]
        );

        // Again: bottom-upward, so the new bottom goes next and the top is still untouched.
        run(
            &mut state,
            vec![exile_bottom_of_library(json_as(json!({ "player": "self" })))],
            Some(PlayerId::P1),
            None,
        );
        assert_eq!(ids(&state.players[PlayerId::P1].exile), vec![bottom, middle]);
        assert_eq!(ids(&state.players[PlayerId::P1].library), vec![top]);
        assert_eq!(state.counters.exiled, 2);
    }

    /// TS: "#40 an empty library exiles nothing and causes NO fatigue".
    #[test]
    fn an_empty_library_exiles_nothing_and_causes_no_fatigue_40() {
        let mut state = game("exile-bottom-empty");
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);
        let before = state.players[PlayerId::P1].fatigue_count;
        let health = state.players[PlayerId::P1].hero.health;

        let events = run(
            &mut state,
            vec![exile_bottom_of_library(json_as(json!({ "player": "self" })))],
            Some(PlayerId::P1),
            None,
        );

        assert_eq!(events, Vec::<GameEvent>::new());
        assert_eq!(state.players[PlayerId::P1].exile.len(), 0);
        assert_eq!(state.counters.exiled, 0);
        // Fatigue is the price of a DRAW from an empty library; this verb never reaches `draw.ts`.
        assert_eq!(state.players[PlayerId::P1].fatigue_count, before);
        assert_eq!(state.players[PlayerId::P1].hero.health, health);
        assert_eq!(events_of_type(&events, GameEventType::Damage).len(), 0);
    }

    #[test]
    fn r11_a_unit_token_card_at_the_bottom_ceases_to_exist_rather_than_being_exiled() {
        let mut state = game("exile-bottom-token");
        let library = set_library(&mut state, PlayerId::P1, &[alpha().id, unit_token().id]);
        let token = must(library.get(1).cloned(), "the token at the bottom");

        run(
            &mut state,
            vec![exile_bottom_of_library(json_as(json!({ "player": "self" })))],
            Some(PlayerId::P1),
            None,
        );

        // TS: `token.zone` is `{ z: "gone", player: "p1" }`. A card that is gone is in no pile, so it is
        // no longer reachable from the state (`78f131c^:.fullsend/notes/spec-gaps-part-24-3.md`).
        assert!(find_instance(&state, &token.id).is_none());
        assert_eq!(state.players[PlayerId::P1].exile.len(), 0);
        assert_eq!(state.counters.exiled, 0);
    }
}

// ---------------------------------------------------------------------------
// §6.3 Discover with the library as the pool (#51)
// ---------------------------------------------------------------------------

mod discover_from_library_51_ky_s_private_tutor {
    use super::*;

    /// #51's library: one match per bracket, plus the cards the filter must leave alone.
    fn tutor_library(state: &mut GameState) -> Vec<CardInstance> {
        set_library(
            state,
            PlayerId::P1,
            &[
                spell_two().id,
                spell_five().id,
                spell_x().id,
                trap_two().id,
                field_trap_two().id,
                field_spell_two().id,
                alpha().id,
            ],
        )
    }

    fn offered_ids(state: &GameState) -> Vec<String> {
        must(state.pending.as_ref(), "the discover prompt")
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Instance { instance_id } => instance_id.clone(),
                other => serde_json::to_value(other).expect("a selection serialises")["pick"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            })
            .collect()
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    /// TS: "§10.6 offers exactly the matching library instances and resumes at the step it was given".
    #[test]
    fn offers_exactly_the_matching_library_instances_and_resumes_at_the_step_it_was_given() {
        let mut state = game("discover-library");
        let library = tutor_library(&mut state);
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);
        let before = ids(&state.players[PlayerId::P1].library);

        let events = run(
            &mut state,
            vec![discover_from_library(json_as(json!({
                "step": "take",
                "count": 3,
                "player": "self",
                "filter": { "type": ["Spell"], "costRange": { "min": 2, "max": 2 } },
                "prompt": "Reveal 3 cards from your library; choose one to add to your hand",
            })))],
            Some(PlayerId::P1),
            Some(&self_),
        );

        let pending = must(state.pending.clone(), "the discover prompt");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.player_id, PlayerId::P1);
        assert_eq!(
            pending.prompt,
            "Reveal 3 cards from your library; choose one to add to your hand"
        );
        // Only the cost-2 Spell matches: the cost-5 Spell is out of the bracket, the X Spell reads 0
        // (R65), and no Trap, Field Trap, Field Spell or Unit is a Spell.
        assert_eq!(
            offered_ids(&state),
            vec![must(library.first(), "the cost-2 spell").id.clone()]
        );
        assert_eq!(pending.min, 1);
        assert_eq!(pending.max, 1);
        assert_eq!(pending.resume.step, "take");
        assert_eq!(pending.resume.def_id, generator().id);
        assert_eq!(pending.resume.instance_id, Some(self_.id.clone()));
        assert_eq!(events_of_type(&events, GameEventType::PromptOpened).len(), 1);
        // §10.8: revealing is the prompt, not a move — the library is untouched until the pick resolves.
        assert_eq!(ids(&state.players[PlayerId::P1].library), before);
    }

    #[test]
    fn r65_the_cost_bracket_reads_an_x_cost_library_card_as_0_and_honours_an_open_ended_4_plus() {
        let mut zero_bracket = game("discover-library-x");
        let library = tutor_library(&mut zero_bracket);
        let self_ = resolving_self(&mut zero_bracket, &generator().id, PlayerId::P1);
        run(
            &mut zero_bracket,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "filter": { "type": "Spell", "costRange": { "min": 0, "max": 1 } } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(
            offered_ids(&zero_bracket),
            vec![must(library.get(2), "the X-cost spell").id.clone()]
        );

        let mut high_bracket = game("discover-library-4plus");
        let other = tutor_library(&mut high_bracket);
        let self_ = resolving_self(&mut high_bracket, &generator().id, PlayerId::P1);
        run(
            &mut high_bracket,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "filter": { "type": "Spell", "costRange": { "min": 4 } } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(
            offered_ids(&high_bracket),
            vec![must(other.get(1), "the cost-5 spell").id.clone()]
        );
    }

    /// TS: "#51 a Field Trap counts as a Trap for type matching".
    #[test]
    fn a_field_trap_counts_as_a_trap_for_type_matching_51() {
        let mut state = game("discover-library-field-trap");
        let library = tutor_library(&mut state);
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);

        run(
            &mut state,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "count": 3, "filter": { "type": "Trap" } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );

        let trap = must(library.get(3), "the trap").id.clone();
        let field_trap = must(library.get(4), "the field trap").id.clone();
        assert_eq!(sorted(offered_ids(&state)), sorted(vec![trap, field_trap]));
        // The reverse does not hold: "Field Trap" names Field Traps only.
        let mut narrow = game("discover-library-field-trap-only");
        let narrow_library = tutor_library(&mut narrow);
        let self_ = resolving_self(&mut narrow, &generator().id, PlayerId::P1);
        run(
            &mut narrow,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "count": 3, "filter": { "type": "Field Trap" } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );
        assert_eq!(
            offered_ids(&narrow),
            vec![must(narrow_library.get(4), "the field trap").id.clone()]
        );
    }

    #[test]
    fn r60_reveals_count_different_cards_drawn_without_replacement() {
        let mut state = game("discover-library-distinct");
        set_library(
            &mut state,
            PlayerId::P1,
            &[alpha().id, alpha().id, beta().id, beta().id, gamma().id],
        );
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);

        run(
            &mut state,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "count": 3, "filter": { "type": "Unit" } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );

        let offered = offered_ids(&state);
        assert_eq!(offered.len(), 3);
        assert_eq!(offered.iter().collect::<IndexSet<_>>().len(), 3);
        for id in &offered {
            assert!(ids(&state.players[PlayerId::P1].library).contains(id));
        }
    }

    /// TS: "§6.3 opens no prompt when nothing matches: the effect fizzles and the card still resolves".
    #[test]
    fn opens_no_prompt_when_nothing_matches_the_effect_fizzles_and_the_card_still_resolves() {
        let mut state = game("discover-library-no-match");
        set_library(&mut state, PlayerId::P1, &[spell_two().id, trap_two().id]);
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);

        let events = run(
            &mut state,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "filter": { "type": "Field Spell" } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );

        assert!(state.pending.is_none());
        assert_eq!(events_of_type(&events, GameEventType::PromptOpened).len(), 0);
        assert_eq!(events, Vec::<GameEvent>::new());

        // An empty library is the same fizzle, which is the branch #51 answers with its Notebook.
        set_library(&mut state, PlayerId::P1, &[] as &[&str]);
        assert_eq!(
            run(
                &mut state,
                vec![discover_from_library(json_as(json!({ "step": "take" })))],
                Some(PlayerId::P1),
                None
            ),
            Vec::<GameEvent>::new()
        );
        assert!(state.pending.is_none());
    }

    /// TS: "§10.8 the prompt belongs to the chooser even when the pool is the other player's library".
    #[test]
    fn the_prompt_belongs_to_the_chooser_even_when_the_pool_is_the_other_player_s_library() {
        let mut state = game("discover-library-enemy-pool");
        let theirs = set_library(&mut state, PlayerId::P2, &[spell_two().id]);
        set_library(&mut state, PlayerId::P1, &[alpha().id]);
        let self_ = resolving_self(&mut state, &generator().id, PlayerId::P1);

        run(
            &mut state,
            vec![discover_from_library(json_as(
                json!({ "step": "take", "player": "enemy", "filter": { "type": "Spell" } }),
            ))],
            Some(PlayerId::P1),
            Some(&self_),
        );

        let pending = must(state.pending.as_ref(), "the discover prompt");
        assert_eq!(pending.player_id, PlayerId::P1);
        assert_eq!(
            offered_ids(&state),
            vec![must(theirs.first(), "their spell").id.clone()]
        );
    }
}
