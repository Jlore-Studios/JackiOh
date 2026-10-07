//! The effects library's cross-cutting acceptance items and the core verbs with no test file of
//! their own (BUILD M3-T1, SPEC §6.3's verb table).
//!
//! M3-T1's acceptance list, verbatim: "every effect has its own test file; `grep -r
//! \"state.players[\" packages/cards` returns nothing (scripts never touch state); `steal` places
//! into the same lane if free else first free and leaves excess (R15); `summon` with no zone takes
//! the leftmost free zone and skips zones reserved for Reborn (R64); `cast` counts as a play with
//! cost paid 0 (R70); `fuse` follows R77; leaving the field resets an instance per R78 while
//! `costMod`, `costOverride` and `radiant` persist; `bounce` returns to the owner's hand and drops
//! buffs (§6.3); `transform` and `vanilla` are refused on Immutable (R23); `recruit` scans top-down
//! and keeps library order."
//!
//! Who owns what. The per-effect files own their own verbs: `effects-steal` (R15), `effects-summon`
//! (R64, recruit, fill your board), `effects-move` (bounce, exile, discard, counter),
//! `effects-transform` (R23), and `fuse.test.ts` (R77). This file owns the two structural items, the
//! two items no single effect file owns — R70's cast and R78's reset — and the modules of
//! `src/effects` that have no test file of their own: draw, addToHand, shuffleInto, loseHealth,
//! mana, memory and position. `targets` used to be on that list; it has `effects-targets.test.ts`
//! of its own now that it carries the board scope the board-wide verbs are written in.
//!
//! Port of `packages/engine/test/effects-core.test.ts`. The TS file's three "M3-T1 structural
//! acceptance" tests read the source tree (`readdirSync` of `src/effects` and `test/`, `readFileSync`
//! of every card script), so they are not ported (part 24's brief, Risks: #133's rule): they are
//! listed in `.fullsend/notes/spec-gaps-part-24-2.md`, and the checks belong to the structural
//! spec checks (part 28), not to a rules test.

use jackioh_engine::effects::{
    add_random_from_graveyard, add_to_hand, damage, draw, gain_mana, lose_health, next_turn_mana, player_of,
    refresh_mana as refresh_effect, remember, remember_random, resolve_target, shuffle_copies_of_self,
    shuffle_into, switch_all_positions, switch_position_of, TargetSpec,
};
use jackioh_engine::mana::{max_mana_for, refresh_mana};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::{Backrow, Units},
};

use super::fixtures::combat::{plain, spikey_pillow, taunter};
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};
use super::fixtures::scripts::{anti_oneshot, stockpile};

/// TS `sinkFor(state)` (fixtures/harness.ts): the state, a fresh event list and an rng at the state's
/// cursor, as `reduce` starts one. A sink borrows all three, so they live here and `sink()` lends them
/// out, built from part 1's frozen `EngineSink::new` and `Rng::new`.
struct Bench<'a> {
    state: &'a mut GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench<'_> {
    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(self.state, &mut self.events, &mut self.rng)
    }
}

fn sink_for(state: &mut GameState) -> Bench<'_> {
    let rng = Rng::new(&state.seed, state.rng_cursor);
    Bench { state, events: Vec::new(), rng }
}

/// TS `put(state, defId, ref, { radiant: true })`: the card is made Radiant before it is placed.
fn put_radiant(state: &mut GameState, def_id: &str, at: ZoneSlot) -> CardInstance {
    let player = at.player;
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    card.radiant = true;
    let id = card.id.clone();
    assert!(place_on_field(state, card, at, Default::default()), "could not place {def_id}");
    find_instance(state, &id).cloned().expect("the placed card")
}

// ---------------------------------------------------------------------------
// Fixtures: a Spell with a Cry, for R70's cast.
// ---------------------------------------------------------------------------

/// TS `def(name, type, extra)`, whose module counter handed out 1701 to the first (and only) def; the
/// index is written out here, as the order of the TS definitions fixed it. `extra` is spread over the def.
fn def(name: &str, type_: &str, index: u32, extra: Value) -> CardDef {
    let mut def = json!({
        "id": format!("ec-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (effects-core)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": name },
        "radiant": { "keywords": [], "text": name },
    });
    if let (Value::Object(fields), Value::Object(extra)) = (&mut def, extra) {
        for (key, value) in extra {
            fields.insert(key, value);
        }
    }
    json_as(def)
}

/// A 3-cost Spell that deals 2 to the enemy hero, so a cast's cost and its script are both visible.
fn castable() -> CardDef {
    def("castable", "Spell", 1701, json!({ "cost": 3 }))
}

fn castable_script() -> Script {
    Script {
        cry: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 2 })))])),
        ..Script::default()
    }
}

fn ec_scripts() -> IndexMap<String, CardScripts> {
    [(castable().id, CardScripts { base: castable_script(), radiant: castable_script() })].into_iter().collect()
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    let castable = castable();
    catalog.insert(castable.id.clone(), castable);
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(ec_scripts());
    register_scripts(scripts);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// TS `HookOptions & { self?: CardInstance }`.
#[derive(Default)]
struct RunOptions {
    controller: Option<PlayerId>,
    self_: Option<CardInstance>,
    targets: Option<Vec<Selection>>,
}

impl RunOptions {
    fn as_p(controller: PlayerId) -> RunOptions {
        RunOptions { controller: Some(controller), ..Default::default() }
    }
}

/// Builds the context TS's `makeContext(sinkFor(state), options.self ?? null, options)` builds over a
/// fresh sink, and hands it to `f` (a context borrows the state, so it cannot be handed back). TS
/// handed over the live card object; the card is looked up again by id.
fn with_context<R>(state: &mut GameState, options: RunOptions, f: impl FnOnce(&mut EffectContext<'_>) -> R) -> R {
    let RunOptions { controller, self_, targets } = options;
    let self_ = self_.map(|card| find_instance(state, &card.id).cloned().unwrap_or(card));
    let mut sink = sink_for(state);
    let mut ctx = make_context(sink.sink(), self_, HookOptions { controller, targets, ..Default::default() });
    f(&mut ctx)
}

/// Apply effects outside any card, as the resolver does, and hand back the events (§10.3). As in TS,
/// the sink's rng is not written back to the state.
fn run(state: &mut GameState, effects: Vec<Effect>, options: RunOptions) -> Vec<GameEvent> {
    let RunOptions { controller, self_, targets } = options;
    let self_ = self_.map(|card| find_instance(state, &card.id).cloned().unwrap_or(card));
    let mut sink = sink_for(state);
    {
        let mut ctx = make_context(sink.sink(), self_, HookOptions { controller, targets, ..Default::default() });
        apply_effects(&effects, &mut ctx);
    }
    std::mem::take(&mut sink.events)
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` and matches, recursively;
/// arrays match element for element and in length.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, want)| actual.get(key).is_some_and(|got| matches_object(got, want))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len() && actual.iter().zip(expected).all(|(got, want)| matches_object(got, want))
        }
        _ => actual == expected,
    }
}

/// `resolveTarget`'s answer as TS's `toEqual` reads it: `{ kind: "unit", instance }`,
/// `{ kind: "hero", player }`, or `null`.
fn target_json(target: Option<DamageTarget>) -> Value {
    match target {
        Some(DamageTarget::Unit { instance }) => json!({ "kind": "unit", "instance": instance }),
        Some(DamageTarget::Hero { player }) => json!({ "kind": "hero", "player": player }),
        None => Value::Null,
    }
}

fn spec(value: Value) -> TargetSpec {
    json_as(value)
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn on_instance(card: &CardInstance) -> Option<Vec<Selection>> {
    Some(vec![Selection::Instance { instance_id: card.id.clone() }])
}

/// One field of every event of a type, in order.
fn field_of(events: &[GameEvent], of: GameEventType, key: &str) -> Vec<Value> {
    events_of_type(events, of).iter().map(|event| json_of(event)[key].clone()).collect()
}

// ---------------------------------------------------------------------------
// R70's cast and R78's reset: the two acceptance items no per-effect file owns.
// ---------------------------------------------------------------------------

mod r70_cast_r78_leaving_the_field_build_m3_t1 {
    use super::*;

    #[test]
    fn r70_a_cast_counts_as_a_play_with_cost_paid_0_and_fires_the_cards_script() {
        let mut state = game("r70-cast");
        let mut sink = sink_for(&mut state);
        let card = must(in_hand(sink.state, &castable().id, P1, 1).into_iter().next(), "a castable Spell"); // printed cost 3
        sink.state.players.p1.mana.current = 3;
        let played_before = sink.state.counters.played;

        cast_card(&mut sink.sink(), &card, Default::default());

        // "A cast is free": no mana moved, and the event the whole game reads says costPaid 0.
        assert_eq!(sink.state.players.p1.mana.current, 3);
        let played = events_of_type(&sink.events, GameEventType::CardPlayed);
        assert_eq!(played.len(), 1);
        let first = json_of(&played[0]);
        let expected = json!({ "player": "p1", "instanceId": card.id, "defId": castable().id, "costPaid": 0 });
        assert!(matches_object(&first, &expected), "{first} does not match {expected}");

        // "Counts as a play for every rule that counts or reacts to plays": the Combo counter, the
        // played list and the game counter Ceaseless Void reads (R55).
        assert_eq!(sink.state.players.p1.turn_log.cards_played, 1);
        assert_eq!(sink.state.players.p1.turn_log.played_ids, vec![card.id.clone()]);
        assert_eq!(sink.state.counters.played, played_before + 1);

        // "It fires the card's Cry or spell script", and the Spell then goes to the graveyard (§10.5).
        assert_eq!(sink.state.players.p2.hero.health, 30 - 2);
        let graveyard: Vec<String> = sink.state.players.p1.graveyard.iter().map(|entry| entry.id.clone()).collect();
        assert_eq!(graveyard, vec![card.id.clone()]);
        assert!(sink.state.players.p1.hand.is_empty());
    }

    #[test]
    fn r78_leaving_the_field_resets_the_instance_while_cost_mod_cost_override_and_radiant_persist() {
        let mut state = game("r78-reset");
        let unit = put_radiant(&mut state, &plain.id, slot(P1, Units, 1));

        let turn = state.turn;
        {
            let card = live_mut(&mut state, &unit);
            card.damage = 2;
            card.buffs = AttackHealth { attack: 3, health: 4 };
            card.granted_keywords = vec![Keyword::Taunt];
            card.vanilla = true;
            card.counters = Counters { plague: Some(2), grade: Some(3) };
            card.memory = [("meal".to_string(), json!("felinor"))].into_iter().collect();
            card.exertion = Exertion { attacked: true, switched: true, attacks: None };
            card.position = Some(Position::Def);
            card.summoned_turn = Some(turn);
            card.stats_override = Some(AttackHealth { attack: 9, health: 9 });
            card.taunt_suppressed_turn = Some(turn);
            card.controller = P2;
            card.divine_shield_spent = Some(true);
            card.marked_destroyed = Some(true);
            card.last_damaged_by = Some("c77".into());
            card.cost_mod = 2;
            card.cost_override = Some(1);
        }

        let moving = live(&state, &unit).clone();
        assert_eq!(
            move_to_zone(&mut state, &moving, ZoneName::Graveyard, Default::default()),
            MoveResult::Moved
        );

        // Every field R78 names is back to its default.
        let after = live(&state, &unit);
        assert_eq!(after.damage, 0);
        assert_eq!(after.buffs, AttackHealth { attack: 0, health: 0 });
        assert!(after.granted_keywords.is_empty());
        assert!(!after.vanilla);
        assert_eq!(json_of(&after.counters), json!({}));
        assert!(after.memory.is_empty());
        assert_eq!(after.exertion, Exertion { attacked: false, switched: false, attacks: None });
        assert_eq!(after.position, None);
        assert_eq!(after.summoned_turn, None);
        assert_eq!(after.stats_override, None);
        assert_eq!(after.taunt_suppressed_turn, None);
        assert_eq!(after.divine_shield_spent, None);
        assert_eq!(after.marked_destroyed, None);
        assert_eq!(after.last_damaged_by, None);
        // R12 and R78: control returns to the owner, which is where the card goes.
        assert_eq!(after.controller, P1);
        assert_eq!(after.zone, Zone::Graveyard { player: P1 });

        // The three that persist in every zone.
        assert_eq!(after.cost_mod, 2);
        assert_eq!(after.cost_override, Some(1));
        assert!(after.radiant);
    }

    #[test]
    fn r78_a_card_that_never_was_on_the_field_keeps_what_it_carries() {
        let mut state = game("r78-off-field");
        let card = must(in_hand(&mut state, &plain.id, P1, 1).into_iter().next(), "a hand card");
        {
            let held = live_mut(&mut state, &card);
            held.memory = [("note".to_string(), json!("kept"))].into_iter().collect();
            held.cost_mod = -1;
        }

        // Hand to library is not "leaving the field", so nothing is reset (R78).
        let moving = live(&state, &card).clone();
        assert_eq!(
            move_to_zone(&mut state, &moving, ZoneName::Library, Default::default()),
            MoveResult::Moved
        );
        assert_eq!(json_of(&live(&state, &card).memory), json!({ "note": "kept" }));
        assert_eq!(live(&state, &card).cost_mod, -1);
    }
}

// ---------------------------------------------------------------------------
// targets.ts: the vocabulary every other verb is written in.
// ---------------------------------------------------------------------------

mod s6_3_target_and_player_specs_targets_ts_m3_t1 {
    use super::*;

    #[test]
    fn s6_3_player_of_reads_self_and_enemy_from_the_controller() {
        let mut state = game("targets-player");
        for controller in [P1, P2] {
            let (mine, theirs) = with_context(&mut state, RunOptions::as_p(controller), |ctx| {
                (player_of(ctx, json_as(json!("self"))), player_of(ctx, json_as(json!("enemy"))))
            });
            assert_eq!(mine, controller);
            assert_eq!(theirs, if controller == P1 { P2 } else { P1 });
        }
    }

    #[test]
    fn s6_3_resolve_target_reads_self_both_heroes_and_the_chosen_selections_in_order_r81() {
        let mut state = game("targets-resolve");
        let self_card = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let first = put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
        let second = put(&mut state, &taunter.id, slot(P2, Units, 2), json!({}));

        let options = RunOptions {
            controller: Some(P1),
            self_: Some(self_card.clone()),
            targets: Some(vec![
                Selection::Instance { instance_id: first.id.clone() },
                Selection::Instance { instance_id: second.id.clone() },
            ]),
        };
        with_context(&mut state, options, |ctx| {
            let self_now = live(ctx.state, &self_card).clone();
            let first_now = live(ctx.state, &first).clone();
            let second_now = live(ctx.state, &second).clone();
            assert_eq!(
                target_json(resolve_target(ctx, &spec(json!({ "of": "self" })))),
                json!({ "kind": "unit", "instance": self_now })
            );
            assert_eq!(
                target_json(resolve_target(ctx, &spec(json!({ "of": "selfHero" })))),
                json!({ "kind": "hero", "player": "p1" })
            );
            assert_eq!(
                target_json(resolve_target(ctx, &spec(json!({ "of": "enemyHero" })))),
                json!({ "kind": "hero", "player": "p2" })
            );
            // "The n-th selection the play carried, default the first."
            assert_eq!(
                target_json(resolve_target(ctx, &spec(json!({ "of": "chosen" })))),
                json!({ "kind": "unit", "instance": first_now })
            );
            assert_eq!(
                target_json(resolve_target(ctx, &spec(json!({ "of": "chosen", "index": 1 })))),
                json!({ "kind": "unit", "instance": second_now })
            );
            assert_eq!(target_json(resolve_target(ctx, &spec(json!({ "of": "chosen", "index": 2 })))), Value::Null);
        });
    }

    #[test]
    fn s6_3_resolve_target_returns_nothing_when_the_spec_names_nothing_on_the_board() {
        let mut state = game("targets-empty");
        let bare = with_context(&mut state, RunOptions::as_p(P1), |ctx| {
            target_json(resolve_target(ctx, &spec(json!({ "of": "self" }))))
        });
        assert_eq!(bare, Value::Null);

        let hero = with_context(
            &mut state,
            RunOptions { controller: Some(P1), targets: Some(vec![Selection::Hero { player: P2 }]), ..Default::default() },
            |ctx| target_json(resolve_target(ctx, &spec(json!({ "of": "chosen" })))),
        );
        assert_eq!(hero, json!({ "kind": "hero", "player": "p2" }));

        let gone = with_context(
            &mut state,
            RunOptions {
                controller: Some(P1),
                targets: Some(vec![Selection::Instance { instance_id: "c9999".into() }, Selection::None]),
                ..Default::default()
            },
            |ctx| {
                (
                    target_json(resolve_target(ctx, &spec(json!({ "of": "chosen" })))),
                    target_json(resolve_target(ctx, &spec(json!({ "of": "chosen", "index": 1 })))),
                )
            },
        );
        assert_eq!(gone, (Value::Null, Value::Null));
    }
}

// ---------------------------------------------------------------------------
// draw, addToHand, shuffleInto.
// ---------------------------------------------------------------------------

mod s6_3_draw_add_to_hand_shuffle_into_m3_t1 {
    use super::*;

    #[test]
    fn s6_3_draw_takes_the_top_cards_for_the_player_the_effect_names() {
        let mut state = game("draw-effect");
        set_library(&mut state, P1, &[plain.id.clone(), taunter.id.clone(), plain.id.clone()]);
        set_library(&mut state, P2, &[taunter.id.clone(), plain.id.clone()]);

        let mine = run(&mut state, vec![draw(json_as(json!({ "count": 2 })))], RunOptions::as_p(P1));
        assert_eq!(def_ids(&state.players.p1.hand), vec![plain.id.clone(), taunter.id.clone()]);
        assert_eq!(state.players.p1.library.len(), 1);
        assert_eq!(field_of(&mine, GameEventType::Drawn, "player"), vec![json!("p1"), json!("p1")]);

        let theirs = run(
            &mut state,
            vec![draw(json_as(json!({ "count": 1, "player": "enemy" })))],
            RunOptions::as_p(P1),
        );
        assert_eq!(def_ids(&state.players.p2.hand), vec![taunter.id.clone()]);
        assert_eq!(field_of(&theirs, GameEventType::Drawn, "player"), vec![json!("p2")]);

        // A count of 0 draws nothing.
        assert_eq!(
            run(&mut state, vec![draw(json_as(json!({ "count": 0 })))], RunOptions::as_p(P1)),
            Vec::<GameEvent>::new()
        );
    }

    #[test]
    fn s6_3_add_to_hand_creates_a_fresh_card_in_the_hand_it_names_radiant_or_free_when_asked() {
        let mut state = game("add-to-hand");
        let events = run(
            &mut state,
            vec![
                add_to_hand(json_as(json!({ "defId": plain.id }))),
                add_to_hand(json_as(json!({ "defId": taunter.id, "player": "enemy" }))),
                add_to_hand(json_as(json!({ "defId": plain.id, "radiant": true, "costOverride": 0 }))),
            ],
            RunOptions::as_p(P1),
        );

        assert_eq!(def_ids(&state.players.p1.hand), vec![plain.id.clone(), plain.id.clone()]);
        assert_eq!(def_ids(&state.players.p2.hand), vec![taunter.id.clone()]);
        let created = must(state.players.p1.hand.get(1), "the third card");
        assert!(created.radiant);
        assert_eq!(created.cost_override, Some(0));
        assert_eq!(created.owner, P1);
        assert_eq!(created.zone, Zone::Hand { player: P1 });
        assert_eq!(
            field_of(&events, GameEventType::AddedToHand, "player"),
            vec![json!("p1"), json!("p2"), json!("p1")]
        );
    }

    #[test]
    fn s2_4_a_card_added_to_a_full_hand_is_burned_instead_r4() {
        let mut state = game("add-to-hand-full");
        in_hand(&mut state, &plain.id, P1, HAND_CAP);

        let events = run(&mut state, vec![add_to_hand(json_as(json!({ "defId": taunter.id })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.hand.len(), HAND_CAP as usize);
        assert_eq!(events_of_type(&events, GameEventType::Burned).len(), 1);
        assert_eq!(def_ids(&state.players.p1.graveyard), vec![taunter.id.clone()]);
    }

    /// `[plain, taunter, stockpile]`, each put in p1's hand and moved to the graveyard.
    fn bury(state: &mut GameState) -> Vec<CardInstance> {
        [plain.id.clone(), taunter.id.clone(), stockpile().id]
            .iter()
            .map(|def_id| {
                let card = must(in_hand(state, def_id, P1, 1).into_iter().next(), def_id);
                move_to_zone(state, &card, ZoneName::Graveyard, Default::default());
                card
            })
            .collect()
    }

    #[test]
    fn c37_a_random_graveyard_card_moves_to_hand_drawn_from_the_match_rng() {
        let mut state = game("add-random-graveyard");
        let buried = bury(&mut state);

        run(&mut state, vec![add_random_from_graveyard(Default::default())], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.hand.len(), 1);
        let moved = must(state.players.p1.hand.first(), "the returned card").clone();
        // The same instance moved, rather than a copy being created (§6.3 Add to hand).
        assert!(buried.iter().any(|card| card.id == moved.id));
        assert_eq!(state.players.p1.graveyard.len(), 2);

        // Seeded: the same state and seed return the same card.
        let mut again = game("add-random-graveyard");
        let buried_again = bury(&mut again);
        run(&mut again, vec![add_random_from_graveyard(Default::default())], RunOptions::as_p(P1));
        let index = buried.iter().position(|card| card.id == moved.id).expect("moved was buried");
        assert_eq!(
            must(again.players.p1.hand.first(), "the returned card").id,
            must(buried_again.get(index), "the same slot").id
        );

        // An empty graveyard gives nothing.
        let mut empty = game("add-random-empty");
        assert_eq!(
            run(&mut empty, vec![add_random_from_graveyard(Default::default())], RunOptions::as_p(P1)),
            Vec::<GameEvent>::new()
        );
        assert!(empty.players.p1.hand.is_empty());
    }

    #[test]
    fn s6_3_shuffle_into_puts_fresh_copies_in_the_library_at_rng_positions() {
        let mut state = game("shuffle-into");
        set_library(&mut state, P1, &[plain.id.clone(), plain.id.clone()]);

        let events = run(
            &mut state,
            vec![shuffle_into(json_as(json!({ "defId": taunter.id, "count": 2 })))],
            RunOptions::as_p(P1),
        );
        assert_eq!(state.players.p1.library.len(), 4);
        assert_eq!(state.players.p1.library.iter().filter(|card| card.def_id == taunter.id).count(), 2);
        let shuffled = events_of_type(&events, GameEventType::ShuffledIn);
        assert_eq!(shuffled.len(), 2);
        for event in &shuffled {
            match event {
                GameEvent::ShuffledIn { player, position, .. } => {
                    assert_eq!(*player, P1);
                    assert!(*position >= 0);
                }
                other => panic!("not a shuffledIn: {other:?}"),
            }
        }

        // The enemy's library when the effect says so, and a Radiant copy when it asks for one (R57).
        run(
            &mut state,
            vec![shuffle_into(json_as(json!({ "defId": taunter.id, "count": 1, "player": "enemy", "radiant": true })))],
            RunOptions::as_p(P1),
        );
        let theirs = must(
            state.players.p2.library.iter().find(|card| card.def_id == taunter.id),
            "the shuffled copy",
        );
        assert!(theirs.radiant);
        assert_eq!(theirs.owner, P2);
    }

    #[test]
    fn c90_1_shuffle_copies_of_self_copies_the_running_cards_definition_and_its_radiant_flag() {
        let mut state = game("shuffle-copies");
        let self_card = must(in_hand(&mut state, &taunter.id, P1, 1).into_iter().next(), "the running card");
        live_mut(&mut state, &self_card).radiant = true;

        run(
            &mut state,
            vec![shuffle_copies_of_self(json_as(json!({ "count": 3 })))],
            RunOptions { controller: Some(P1), self_: Some(self_card.clone()), ..Default::default() },
        );
        let copies: Vec<&CardInstance> =
            state.players.p1.library.iter().filter(|card| card.def_id == taunter.id).collect();
        assert_eq!(copies.len(), 3);
        assert!(copies.iter().all(|card| card.radiant));
        assert!(copies.iter().all(|card| card.id != self_card.id));

        // With no card running there is nothing to copy.
        let mut bare = game("shuffle-copies-bare");
        set_library(&mut bare, P1, &[] as &[&str]);
        assert_eq!(
            run(&mut bare, vec![shuffle_copies_of_self(json_as(json!({ "count": 2 })))], RunOptions::as_p(P1)),
            Vec::<GameEvent>::new()
        );
        assert!(bare.players.p1.library.is_empty());
    }
}

// ---------------------------------------------------------------------------
// loseHealth, mana, memory, position.
// ---------------------------------------------------------------------------

mod s6_3_lose_health_r18_m3_t1 {
    use super::*;

    #[test]
    fn r18_lose_health_lowers_the_hero_directly_no_armor_no_hero_cap_no_damage_event() {
        let mut state = game("lose-health");
        state.players.p2.hero.armor = 5;
        put(&mut state, &anti_oneshot().id, slot(P2, Backrow, 1), json!({})); // caps each damage instance at 5

        let events = run(
            &mut state,
            vec![lose_health(json_as(json!({ "player": "enemy", "amount": 12 })))],
            RunOptions::as_p(P1),
        );
        assert_eq!(state.players.p2.hero.health, 30 - 12);
        assert_eq!(
            json_of(&events_of_type(&events, GameEventType::HealthLost)),
            json!([{ "type": "healthLost", "player": "p2", "amount": 12 }])
        );
        assert!(events_of_type(&events, GameEventType::Damage).is_empty());

        // Your own hero when the effect names itself (#98's draw power), and 0 does nothing.
        run(&mut state, vec![lose_health(json_as(json!({ "player": "self", "amount": 2 })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.hero.health, 30 - 2);
        assert_eq!(
            run(&mut state, vec![lose_health(json_as(json!({ "player": "self", "amount": 0 })))], RunOptions::as_p(P1)),
            Vec::<GameEvent>::new()
        );
    }
}

mod s6_3_mana_and_next_turn_mana_s2_3_m3_t1 {
    use super::*;

    #[test]
    fn s2_3_gain_mana_adds_to_current_mana_and_may_take_it_above_max() {
        let mut state = game("gain-mana");
        state.players.p1.mana = ManaState { current: 1, max: 1, next_turn_mod: 0, perm_mod: 0 };

        let events = run(&mut state, vec![gain_mana(json_as(json!({ "amount": 2 })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.mana.current, 3);
        assert_eq!(state.players.p1.mana.max, 1); // above max is allowed (§2.3)
        assert_eq!(
            json_of(&events_of_type(&events, GameEventType::ManaChanged)),
            json!([{ "type": "manaChanged", "player": "p1", "current": 3, "max": 1 }])
        );

        // The enemy's mana when the effect names it, and a drain floors at 0.
        run(&mut state, vec![gain_mana(json_as(json!({ "amount": 1, "player": "enemy" })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p2.mana.current, 1);
        run(&mut state, vec![gain_mana(json_as(json!({ "amount": -5 })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.mana.current, 0);
    }

    #[test]
    fn r364_refresh_gives_back_spent_mana_up_to_max_and_never_past_it_s6_3_refresh() {
        let mut state = game("refresh-mana");
        state.players.p1.mana = ManaState { current: 0, max: 4, next_turn_mod: 0, perm_mod: 0 };

        // 0 of 4: three come back.
        let events = run(&mut state, vec![refresh_effect(json_as(json!({ "amount": 3 })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.mana.current, 3);
        assert_eq!(
            json_of(&events_of_type(&events, GameEventType::ManaChanged)),
            json!([{ "type": "manaChanged", "player": "p1", "current": 3, "max": 4 }])
        );

        // 3 of 4: only one is spent, so only one comes back — a Refresh never goes past max.
        run(&mut state, vec![refresh_effect(json_as(json!({ "amount": 3 })))], RunOptions::as_p(P1));
        assert_eq!(state.players.p1.mana.current, 4);

        // Temporary mana above max is kept, and a Refresh there gives nothing and announces nothing.
        state.players.p1.mana.current = 6;
        let over = run(&mut state, vec![refresh_effect(json_as(json!({ "amount": 3 })))], RunOptions::as_p(P1));
        assert!(events_of_type(&over, GameEventType::ManaChanged).is_empty());
        assert_eq!(state.players.p1.mana.current, 6);

        // The enemy's pool when the effect names it.
        state.players.p2.mana = ManaState { current: 1, max: 2, next_turn_mod: 0, perm_mod: 0 };
        run(
            &mut state,
            vec![refresh_effect(json_as(json!({ "amount": 3, "player": "enemy" })))],
            RunOptions::as_p(P1),
        );
        assert_eq!(state.players.p2.mana.current, 2);
    }

    #[test]
    fn c21_next_turn_mana_changes_the_next_refresh_only_and_the_refresh_floors_at_0_s2_3() {
        let mut state = game("next-turn-mana");
        state.players.p2.turns_started = 3;
        state.players.p2.mana = ManaState { current: 3, max: 3, next_turn_mod: 0, perm_mod: 0 };

        let events = run(
            &mut state,
            vec![next_turn_mana(json_as(json!({ "amount": -1, "player": "enemy" })))],
            RunOptions::as_p(P1),
        );
        assert_eq!(state.players.p2.mana.next_turn_mod, -1);
        assert_eq!(state.players.p2.mana.current, 3); // this turn is untouched
        assert_eq!(events_of_type(&events, GameEventType::ModifierChanged).len(), 1);

        // §2.3: the rider lowers what the refresh gives, not max mana, which is min(turns, 4) plus the
        // persistent modifiers only.
        state.players.p2.turns_started = 4;
        assert_eq!(max_mana_for(&state.players.p2), MAX_MANA);
        refresh_mana(&mut state.players.p2);
        assert_eq!((state.players.p2.mana.current, state.players.p2.mana.max), (MAX_MANA - 1, MAX_MANA));
        // One refresh only: the modifier is spent.
        assert_eq!(state.players.p2.mana.next_turn_mod, 0);

        // A big penalty floors the refresh at 0 rather than going negative.
        run(
            &mut state,
            vec![next_turn_mana(json_as(json!({ "amount": -9, "player": "enemy" })))],
            RunOptions::as_p(P1),
        );
        refresh_mana(&mut state.players.p2);
        assert_eq!((state.players.p2.mana.current, state.players.p2.mana.max), (0, MAX_MANA));
    }
}

mod s10_1_memory_what_a_card_remembers_r43_m3_t1 {
    use super::*;

    #[test]
    fn s10_1_remember_stores_a_value_on_the_card_that_is_running_under_the_key_it_names() {
        let mut state = game("remember");
        let self_card = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let as_self = || RunOptions { controller: Some(P1), self_: Some(self_card.clone()), ..Default::default() };

        run(
            &mut state,
            vec![remember(json_as(json!({ "key": "meal", "value": { "attack": 3, "health": 3 } })))],
            as_self(),
        );
        assert_eq!(live(&state, &self_card).memory.get("meal"), Some(&json!({ "attack": 3, "health": 3 })));

        // A second write replaces the first, and another key lives beside it.
        run(
            &mut state,
            vec![
                remember(json_as(json!({ "key": "meal", "value": "eaten" }))),
                remember(json_as(json!({ "key": "grade", "value": 2 }))),
            ],
            as_self(),
        );
        // Beside them, the keys the card's texts remembered under, which a Fuse that keeps the card
        // moves with those texts (R102, `work.REMEMBERED_KEY`).
        assert_eq!(
            json_of(&live(&state, &self_card).memory),
            json!({ "meal": "eaten", "grade": 2, (REMEMBERED_KEY): ["meal", "grade"] })
        );

        // With no card running there is nowhere to remember anything.
        assert_eq!(
            run(&mut state, vec![remember(json_as(json!({ "key": "meal", "value": 1 })))], RunOptions::as_p(P1)),
            Vec::<GameEvent>::new()
        );
    }

    #[test]
    fn r43_remember_random_picks_from_the_match_rng_so_the_same_seed_remembers_the_same_option() {
        let options = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"];

        let pick = |seed: &str| -> Option<Value> {
            let mut state = game(seed);
            let self_card = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
            run(
                &mut state,
                vec![remember_random(json_as(json!({ "key": "power", "options": options })))],
                RunOptions { controller: Some(P1), self_: Some(self_card.clone()), ..Default::default() },
            );
            live(&state, &self_card).memory.get("power").cloned()
        };

        let first = pick("remember-random");
        assert!(options.iter().any(|option| first == Some(json!(option))));
        assert_eq!(pick("remember-random"), first);

        // An empty option list remembers nothing, and so does a call with no card running.
        let mut state = game("remember-random-empty");
        let self_card = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        run(
            &mut state,
            vec![remember_random(json_as(json!({ "key": "power", "options": [] })))],
            RunOptions { controller: Some(P1), self_: Some(self_card.clone()), ..Default::default() },
        );
        assert_eq!(live(&state, &self_card).memory.get("power"), None);
        assert_eq!(
            run(
                &mut state,
                vec![remember_random(json_as(json!({ "key": "power", "options": options })))],
                RunOptions::as_p(P1),
            ),
            Vec::<GameEvent>::new()
        );
    }
}

mod s6_3_switch_position_as_an_effect_r20_m3_t1 {
    use super::*;

    #[test]
    fn r20_switching_a_named_unit_spends_no_exertion_and_flips_or_sets_the_position() {
        let mut state = game("switch-position");
        let unit = put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
        let at_unit = || RunOptions { controller: Some(P1), targets: on_instance(&unit), ..Default::default() };

        let events = run(
            &mut state,
            vec![switch_position_of(json_as(json!({ "target": { "of": "chosen" } })))],
            at_unit(),
        );
        assert_eq!(live(&state, &unit).position, Some(Position::Def));
        // R20: an effect's switch is free, so the unit can still act on its own turn (§4.1).
        assert_eq!(live(&state, &unit).exertion, Exertion { attacked: false, switched: false, attacks: None });
        assert_eq!(
            json_of(&events_of_type(&events, GameEventType::PositionSwitched)),
            json!([{ "type": "positionSwitched", "instanceId": unit.id, "position": "DEF" }])
        );

        // A named position rather than a flip, and naming the one it is already in changes nothing.
        run(
            &mut state,
            vec![switch_position_of(json_as(json!({ "to": "ATK", "target": { "of": "chosen" } })))],
            at_unit(),
        );
        assert_eq!(live(&state, &unit).position, Some(Position::Atk));
        assert_eq!(
            run(
                &mut state,
                vec![switch_position_of(json_as(json!({ "to": "ATK", "target": { "of": "chosen" } })))],
                at_unit(),
            ),
            Vec::<GameEvent>::new()
        );

        // The card running the effect may switch itself (§6.3), and a hero pick does nothing.
        let self_card = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        run(
            &mut state,
            vec![switch_position_of(json_as(json!({ "target": { "of": "self" } })))],
            RunOptions { controller: Some(P1), self_: Some(self_card.clone()), ..Default::default() },
        );
        assert_eq!(live(&state, &self_card).position, Some(Position::Def));
        assert_eq!(
            run(
                &mut state,
                vec![switch_position_of(json_as(json!({ "target": { "of": "chosen" } })))],
                RunOptions {
                    controller: Some(P1),
                    targets: Some(vec![Selection::Hero { player: P2 }]),
                    ..Default::default()
                },
            ),
            Vec::<GameEvent>::new()
        );
    }

    #[test]
    fn c48_switch_all_positions_flips_every_unit_or_one_sides_and_never_puts_spikey_pillow_in_defense() {
        let mut state = game("switch-all");
        let mine = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let also_mine = put(&mut state, &taunter.id, slot(P1, Units, 2), json!({}));
        let pillow = put(&mut state, &spikey_pillow.id, slot(P1, Units, 3), json!({})); // neverDefense (§4.1)
        let theirs = put(&mut state, &plain.id, slot(P2, Units, 1), json!({}));
        live_mut(&mut state, &also_mine).position = Some(Position::Def);

        run(&mut state, vec![switch_all_positions(json_as(json!({ "side": "both" })))], RunOptions::as_p(P1));
        assert_eq!(live(&state, &mine).position, Some(Position::Def));
        assert_eq!(live(&state, &also_mine).position, Some(Position::Atk));
        assert_eq!(live(&state, &theirs).position, Some(Position::Def));
        // §4.1: Spikey Pillow cannot be switched to Defense, by an action or by an effect.
        assert_eq!(live(&state, &pillow).position, Some(Position::Atk));
        // R20 again: nothing spent anywhere.
        assert!([&mine, &also_mine, &theirs].iter().all(|unit| !live(&state, unit).exertion.switched));

        // One side only when the effect names it.
        run(&mut state, vec![switch_all_positions(json_as(json!({ "side": "enemy" })))], RunOptions::as_p(P1));
        assert_eq!(live(&state, &theirs).position, Some(Position::Atk));
        assert_eq!(live(&state, &mine).position, Some(Position::Def));

        run(&mut state, vec![switch_all_positions(json_as(json!({ "side": "self" })))], RunOptions::as_p(P1));
        assert_eq!(live(&state, &mine).position, Some(Position::Atk));
        assert_eq!(live(&state, &theirs).position, Some(Position::Atk));
    }
}

// ---------------------------------------------------------------------------
// The effects a card script may see at all.
// ---------------------------------------------------------------------------

mod s6_3_the_effects_barrel_m3_t1 {
    use super::*;

    #[test]
    fn s6_3_every_verb_a_card_script_imports_is_a_factory_that_returns_an_effect() {
        let state = game("barrel");
        let built: Vec<Effect> = vec![
            damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 1 }))),
            draw(json_as(json!({ "count": 1 }))),
            add_to_hand(json_as(json!({ "defId": plain.id }))),
            shuffle_into(json_as(json!({ "defId": plain.id, "count": 1 }))),
            lose_health(json_as(json!({ "player": "self", "amount": 1 }))),
            gain_mana(json_as(json!({ "amount": 1 }))),
            next_turn_mana(json_as(json!({ "amount": 1 }))),
            refresh_effect(json_as(json!({ "amount": 1 }))),
            remember(json_as(json!({ "key": "k", "value": 1 }))),
            remember_random(json_as(json!({ "key": "k", "options": [1] }))),
            switch_position_of(json_as(json!({ "target": { "of": "self" } }))),
            switch_all_positions(json_as(json!({ "side": "both" }))),
        ];

        for effect in &built {
            // TS checked `typeof effect.kind === "string"` and `typeof effect.apply === "function"`;
            // here the types say both, and the kind must still name something.
            let _: &'static str = effect.kind;
            let _: &EffectApply = &effect.apply;
            assert!(!effect.kind.is_empty());
        }
        // Each names itself, so an event log and a stack trace read as the verb list of §6.3.
        assert_eq!(built.iter().map(|effect| effect.kind).collect::<IndexSet<_>>().len(), built.len());
        // Building an effect changes nothing until it is applied (CLAUDE.md rule 5).
        assert_eq!(state.players.p2.hero.health, 30);
    }
}
