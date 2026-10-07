//! Player modifiers and delayed effects (BUILD M3-T5, SPEC §2.2, §10.1, R48, R62, R68).
//!
//! Two halves. The modifiers half proves each of the three expiries `ModifierExpiry` offers —
//! `thisTurn`, `nextTurnOf(player)` and `used` — at its boundary, with R48's timing (a next-turn
//! modifier does nothing on the turn it was made) in a test of its own. The delayed half proves
//! they resolve at their R62 point in creation order, that K-Pop Fanatic's steal fires after its
//! unit has died (§8 #50, R76), that a continuation whose instance has ceased to exist still
//! resolves with `ctx.self === null` (R127), and that Efficiency Dividend's mana is a
//! `mana.nextTurnMod` rather than a delayed effect at all (§8 #24).
//!
//! The expiry boundaries call `expireModifiers` directly, so one state object carries a whole test
//! and the modifier ids stay comparable; `reduce` clones, so the tests that need a real turn
//! boundary re-find their cards in the state that comes back.
//!
//! Port of `packages/engine/test/modifiers.test.ts`. The TS fixtures' delayed steps live on a
//! script's `activate` hook, which SURFACE §7.2 does not port; here they are the script's `delayed`
//! hook (`Resume.hook` "delayed", `Script::hook_named`'s name for the same step) — see
//! `.fullsend/notes/spec-gaps-part-25-3.md`.

use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;

use jackioh_engine::effects::{damage, draw, next_turn_mana, steal};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{indestructible, plain, trampler};
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, sink_for, slot};

// ---------------------------------------------------------------------------
// Local defs. Indices start above 1300 so they never collide with a fixture catalog or with
// another test file's local defs.
// ---------------------------------------------------------------------------

/// TS `{ ...base, ...extra }`: the keys `extra` names replace the defaults'.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Value::Object(base), Value::Object(extra)) = (&mut base, extra) {
        for (key, value) in extra {
            base.insert(key, value);
        }
    }
    base
}

/// TS `def(overrides)`; `index` is the value TS's running `nextIndex` (from 1300) gives it.
fn def(index: u32, overrides: Value) -> CardDef {
    json_as(spread(
        json!({
            "index": index.to_string(),
            "set": "Core",
            "type": "Unit",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": { "keywords": [], "text": "" },
            "radiant": { "keywords": [], "text": "" },
        }),
        overrides,
    ))
}

/// A body whose delayed hook deals `data.amount` to the enemy hero. Two delayed effects on one
/// instance then differ only in their captured data, so the order of their `damage` events is the
/// order they resolved in (R68).
fn delayed_bolt() -> CardDef {
    def(
        1301,
        json!({
            "id": "md-delayed-bolt",
            "name": "Delayed Bolt (modifiers fixture)",
            "base": { "attack": 1, "health": 9, "keywords": [], "text": "delayed: deal data.amount to the enemy hero" },
            "radiant": { "attack": 2, "health": 18, "keywords": [], "text": "same" },
        }),
    )
}

/// §8 #50 K-Pop Fanatic, 1/1 → 2/2 Divine Shield: the delayed steal is its delayed hook.
fn kpop_fanatic() -> CardDef {
    def(
        1302,
        json!({
            "id": "md-kpop-fanatic",
            "name": "K-Pop Fanatic (modifiers fixture)",
            "rarity": "Epic",
            "cost": 1,
            "base": {
                "attack": 1,
                "health": 1,
                "keywords": [],
                "text": "Cry: choose an enemy permanent; at the start of your next turn, steal it",
            },
            "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Divine Shield" }], "text": "Divine Shield; same" },
        }),
    )
}

/// An end-of-turn trigger whose event (`drawn`) is nothing a delayed effect here also emits.
fn end_of_turn_drawer() -> CardDef {
    def(
        1303,
        json!({
            "id": "md-end-drawer",
            "name": "End-of-turn Drawer (modifiers fixture)",
            "cost": 2,
            "base": { "attack": 1, "health": 9, "keywords": [], "text": "End of turn: draw a card" },
            "radiant": { "attack": 2, "health": 18, "keywords": [], "text": "same" },
        }),
    )
}

/// A printed 5 for R48: a costMod brings it down to 4, which is where Curvature reads it.
fn cost_five() -> CardDef {
    def(
        1304,
        json!({
            "id": "md-cost-five",
            "name": "Cost Five (modifiers fixture)",
            "cost": 5,
            "base": { "attack": 5, "health": 5, "keywords": [], "text": "5/5" },
            "radiant": { "attack": 10, "health": 10, "keywords": [], "text": "10/10" },
        }),
    )
}

/// A 2-mana Spell, for Lunar Eclipse's Spell-only discount.
fn cheap_spell() -> CardDef {
    def(
        1305,
        json!({
            "id": "md-cheap-spell",
            "name": "Cheap Spell (modifiers fixture)",
            "type": "Spell",
            "cost": 2,
        }),
    )
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn mod_defs() -> Vec<CardDef> {
    vec![
        delayed_bolt(),
        kpop_fanatic(),
        end_of_turn_drawer(),
        cost_five(),
        cheap_spell(),
    ]
}

thread_local! {
    /// Every `ctx.self` `delayed_bolt`'s continuation has been re-entered with, as an instance id or
    /// `None`. R127 ("a continuation with no instance still resolves … with `ctx.self === null`") is a
    /// statement about the context the step runs in, which is only observable from inside the script.
    /// (TS: a module array; here one per test thread, as each `#[test]` runs on its own.)
    static SELF_AT_RESUME: Cell<Vec<Option<String>>> = const { Cell::new(Vec::new()) };
}

fn record_self_at_resume(id: Option<String>) {
    SELF_AT_RESUME.with(|cell| {
        let mut seen = cell.take();
        seen.push(id);
        cell.set(seen);
    });
}

fn self_at_resume() -> Vec<Option<String>> {
    SELF_AT_RESUME.with(|cell| {
        let seen = cell.take();
        cell.set(seen.clone());
        seen
    })
}

fn mod_scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            delayed_bolt().id,
            both(Script {
                delayed: Some(hook(|ctx| {
                    record_self_at_resume(ctx.self_.as_ref().map(|card| card.id.clone()));
                    let amount = ctx.data.get("amount").and_then(Value::as_i64).unwrap_or(0) as i32;
                    vec![damage(json_as(
                        json!({ "to": { "of": "enemyHero" }, "amount": amount }),
                    ))]
                })),
                ..Script::default()
            }),
        ),
        (
            kpop_fanatic().id,
            both(Script {
                delayed: Some(hook(|ctx| {
                    let target = ctx
                        .data
                        .get("target")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    vec![steal(json_as(json!({ "instanceId": target })))]
                })),
                ..Script::default()
            }),
        ),
        (
            end_of_turn_drawer().id,
            both(Script {
                end_of_turn: Some(hook(|_ctx| vec![draw(json_as(json!({ "count": 1 })))])),
                ..Script::default()
            }),
        ),
    ]
}

static NONCE: AtomicU32 = AtomicU32::new(0);

fn next_nonce() -> String {
    format!("md{}", NONCE.fetch_add(1, Ordering::Relaxed) + 1)
}

fn act(state: &GameState, body: Value) -> GameState {
    let mut action = body;
    action["nonce"] = json!(next_nonce());
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn end_turn(state: &GameState) -> ReduceResult {
    let action = json!({ "type": "endTurn", "playerId": state.active, "nonce": next_nonce() });
    let result = reduce(state, &json_as::<Action>(action));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

fn end_turns(state: &GameState, count: usize) -> GameState {
    let mut next = state.clone();
    for _ in 0..count {
        next = end_turn(&next).state;
    }
    next
}

/// Past the mulligans, in p1's main phase of turn 1, with the local defs folded in.
fn playing(seed: &str) -> GameState {
    let fresh = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for entry in mod_defs() {
        catalog.insert(entry.id.clone(), entry);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts();
    for (id, script) in mod_scripts() {
        scripts.insert(id, script);
    }
    register_scripts(scripts);
    let mut state = begin_game(&fresh).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|card| card.id.clone()).collect();
    state = act(
        &state,
        json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }),
    );
    state
}

fn hand_card(state: &GameState, id: &str) -> CardInstance {
    state
        .players
        .p1
        .hand
        .iter()
        .find(|card| card.id == id)
        .cloned()
        .unwrap_or_else(|| panic!("no {id} in p1's hand"))
}

fn mod_of(state: &GameState, id: &str) -> PlayerModifier {
    state
        .players
        .p1
        .mods
        .iter()
        .find(|modifier| modifier.id == id)
        .cloned()
        .unwrap_or_else(|| panic!("no modifier {id} on p1"))
}

/// One card in hand, as a value rather than a possibly-absent index.
fn one(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("expected a card")
}

/// The serializable continuation a delayed effect carries (§10.1, §10.6).
fn resume(def_id: &str, instance_id: &str, data: Value) -> Resume {
    Resume {
        def_id: def_id.to_string(),
        hook: "delayed".to_string(),
        step: "start".to_string(),
        radiant: false,
        instance_id: Some(instance_id.to_string()),
        data: json_as(data),
    }
}

fn at(phase: Phase, player: PlayerId) -> DelayedAt {
    DelayedAt { phase, player }
}

/// `{ kind: "costDiscount", amount }`.
fn discount(amount: i32) -> ModifierKind {
    ModifierKind::CostDiscount {
        amount,
        only_type: None,
        min_current_cost: None,
        once_per_turn: None,
    }
}

/// #77 Professor Curvature's `{ kind: "costDiscount", amount: 1, minCurrentCost: 4 }`.
fn curvature() -> ModifierKind {
    ModifierKind::CostDiscount {
        amount: 1,
        only_type: None,
        min_current_cost: Some(4),
        once_per_turn: None,
    }
}

fn cost(state: &GameState, card: &CardInstance) -> i32 {
    effective_cost(state, card, CostOptions::default())
}

fn mod_ids(state: &GameState) -> Vec<String> {
    state
        .players
        .p1
        .mods
        .iter()
        .map(|modifier| modifier.id.clone())
        .collect()
}

fn json_of<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// The `modifierChanged` events, as their JSON.
fn modifier_changes(events: &[GameEvent]) -> Vec<Value> {
    events_of_type(events, GameEventType::ModifierChanged)
        .into_iter()
        .map(json_of)
        .collect()
}

fn removals(events: &[GameEvent]) -> usize {
    modifier_changes(events)
        .iter()
        .filter(|event| event["added"] != json!(true))
        .count()
}

/// `eventsOfType(events, "damage").map((e) => e.amount)`.
fn amounts(events: &[GameEvent]) -> Vec<i64> {
    events_of_type(events, GameEventType::Damage)
        .into_iter()
        .map(|event| json_of(event)["amount"].as_i64().unwrap_or_default())
        .collect()
}

/// TS `types.indexOf(type)`: -1 when absent.
fn index_of(types: &[GameEventType], kind: GameEventType) -> i64 {
    types
        .iter()
        .position(|found| *found == kind)
        .map_or(-1, |at| at as i64)
}

/// TS `types.lastIndexOf(type)`: -1 when absent.
fn last_index_of(types: &[GameEventType], kind: GameEventType) -> i64 {
    types
        .iter()
        .rposition(|found| *found == kind)
        .map_or(-1, |at| at as i64)
}

fn types_of(events: &[GameEvent]) -> Vec<GameEventType> {
    events.iter().map(GameEvent::event_type).collect()
}

/// vitest's `toMatchObject` over JSON.
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

mod player_modifiers_and_their_three_expiries_2_2_10_1 {
    use super::*;

    #[test]
    fn s2_2_a_this_turn_modifier_is_live_through_its_own_turn_and_gone_at_that_turns_cleanup() {
        let mut state = playing("this-turn");
        let four = one(in_hand(&mut state, &indestructible.id, P1, 1));
        let turn = state.turn;
        let mut sink = sink_for(&mut state);

        let modifier = add_modifier(&mut sink, P1, ModifierExpiry::ThisTurn { turn }, discount(1));
        assert_eq!(cost(sink.state, &four), 3);
        assert!(modifier_is_live(sink.state, &modifier));

        // A stamp naming another turn is not this turn's business: cleanup leaves it alone.
        let later = add_modifier(
            &mut sink,
            P1,
            ModifierExpiry::ThisTurn { turn: turn + 1 },
            discount(1),
        );
        expire_modifiers(&mut sink, P1);
        assert_eq!(mod_ids(sink.state), vec![later.id.clone()]);
        assert_eq!(cost(sink.state, &four), 3);

        // And on the turn it names, it goes.
        sink.state.turn += 1;
        expire_modifiers(&mut sink, P1);
        assert!(sink.state.players.p1.mods.is_empty());
        assert_eq!(cost(sink.state, &four), 4);
        let added: Vec<Value> = modifier_changes(sink.events)
            .iter()
            .map(|event| event["added"].clone())
            .collect();
        assert_eq!(added, vec![json!(true), json!(true), json!(false), json!(false)]);
    }

    #[test]
    fn r48_a_next_turn_of_modifier_does_nothing_on_the_turn_it_was_made_and_applies_on_that_players_next_turn()
     {
        let mut state = playing("curvature-timing");
        let four = one(in_hand(&mut state, &indestructible.id, P1, 1));

        // #77 Professor Curvature's Cry: "during your next turn, cards whose cost is 4 cost 1 less".
        let from_turn = state.turn;
        let modifier = add_modifier(
            &mut sink_for(&mut state),
            P1,
            ModifierExpiry::NextTurnOf {
                player: P1,
                from_turn,
            },
            curvature(),
        );

        // Inert on the turn it was made: the card still costs its printed 4 (R48).
        assert_eq!(state.turn, 1);
        assert!(!modifier_is_live(&state, &modifier));
        assert_eq!(cost(&state, &four), 4);

        // Live once p1's next turn has begun.
        state = end_turns(&state, 2);
        assert_eq!(state.turn, 3);
        assert_eq!(state.active, P1);
        assert!(modifier_is_live(&state, &mod_of(&state, &modifier.id)));
        assert_eq!(cost(&state, &hand_card(&state, &four.id)), 3);
    }

    #[test]
    fn r48_a_next_turn_of_modifier_is_not_live_during_the_opponents_turn_in_between() {
        let mut state = playing("curvature-between");
        let four = one(in_hand(&mut state, &indestructible.id, P1, 1));
        let from_turn = state.turn;
        let modifier = add_modifier(
            &mut sink_for(&mut state),
            P1,
            ModifierExpiry::NextTurnOf {
                player: P1,
                from_turn,
            },
            curvature(),
        );

        state = end_turns(&state, 1);
        assert_eq!(state.active, P2);
        assert_eq!(state.turn, 2);

        // DISCREPANCY: src/mana.ts does A; SPEC §8 #77 and R48 say B.
        //   A: `modifierIsLive` is `state.turn > mod.expiry.fromTurn`, which is already true on the
        //      opponent's intervening turn, so p1's discount reads live on turns 2 and 3 alike —
        //      `expireModifiers` only ends it at the cleanup of p1's own next turn.
        //   B: #77's text is "during your next turn", and BUILD M3-T5's acceptance is "applies only on
        //      the next turn"; turn 2 is p2's turn, not p1's next turn, so the modifier owes nothing
        //      there. It is observable through §10.8's `viewFor`, which shows p1 their hand's costs
        //      while the opponent is playing.
        assert!(!modifier_is_live(&state, &mod_of(&state, &modifier.id)));
        assert_eq!(cost(&state, &hand_card(&state, &four.id)), 4);
    }

    #[test]
    fn r48_r363_professor_curvature_reads_the_cost_as_it_currently_stands_not_the_printed_one() {
        let mut state = playing("curvature-current");
        let turn = state.turn;
        // `fromTurn` one turn back, so the modifier is already live (R48 is the test above).
        add_modifier(
            &mut sink_for(&mut state),
            P1,
            ModifierExpiry::NextTurnOf {
                player: P1,
                from_turn: turn - 1,
            },
            curvature(),
        );

        // A printed 4 is discounted, and so is a printed 5 that a costMod has brought down to 4.
        let four = one(in_hand(&mut state, &indestructible.id, P1, 1));
        let five_modded = one(in_hand(&mut state, &cost_five().id, P1, 1));
        let five_plain = one(in_hand(&mut state, &cost_five().id, P1, 1));
        let three = one(in_hand(&mut state, &trampler.id, P1, 1));
        let five_modded = {
            let live = find_instance_mut(&mut state, &five_modded.id).expect("the modded five");
            live.cost_mod = -1;
            live.clone()
        };
        assert_eq!(cost(&state, &four), 3);
        assert_eq!(cost(&state, &five_modded), 3);
        // R363: a printed 5 standing at 5 is 4 or more, so it is discounted too; a printed 3 is not.
        assert_eq!(cost(&state, &five_plain), 4);
        assert_eq!(cost(&state, &three), 3);

        // R65's order: other discounts first, then Curvature against the result. A second −1 moves
        // every card's current cost, so which card Curvature reaches moves with it.
        add_modifier(
            &mut sink_for(&mut state),
            P1,
            ModifierExpiry::ThisTurn { turn },
            discount(1),
        );
        // 4 − 1 = 3, no longer 4: Curvature stops applying, and the card still pays 3.
        assert_eq!(cost(&state, &four), 3);
        // 5 − 1 = 4: Curvature still applies, now to a 4.
        assert_eq!(cost(&state, &five_plain), 3);
        // 5 − 1 (costMod) − 1 = 3: past 4 in the other direction, so Curvature is out again.
        assert_eq!(cost(&state, &five_modded), 3);
        assert_eq!(cost(&state, &three), 2);
    }

    #[test]
    fn r48_a_next_turn_of_modifier_expires_at_the_cleanup_of_that_players_next_turn_and_not_at_the_other_players()
     {
        let mut state = playing("next-turn-expiry");
        let from_turn = state.turn;
        let mut sink = sink_for(&mut state);
        let modifier = add_modifier(
            &mut sink,
            P1,
            ModifierExpiry::NextTurnOf {
                player: P1,
                from_turn,
            },
            curvature(),
        );

        // The cleanup of the turn it was made keeps it: its own turn has not happened yet (R48).
        expire_modifiers(&mut sink, P1);
        assert_eq!(mod_ids(sink.state), vec![modifier.id.clone()]);

        // The opponent's cleanup never ends a modifier keyed to p1's turn.
        sink.state.turn += 1;
        expire_modifiers(&mut sink, P2);
        assert_eq!(mod_ids(sink.state), vec![modifier.id.clone()]);

        // p1's next turn: live during it, and gone at its cleanup.
        sink.state.turn += 1;
        assert!(modifier_is_live(sink.state, &mod_of(sink.state, &modifier.id)));
        expire_modifiers(&mut sink, P1);
        assert!(sink.state.players.p1.mods.is_empty());
        assert_eq!(removals(sink.events), 1);
    }

    #[test]
    fn r30_an_until_used_modifier_survives_cleanup_and_goes_only_when_it_is_consumed() {
        let mut state = playing("until-used");
        let mut sink = sink_for(&mut state);
        let echo = add_modifier(
            &mut sink,
            P1,
            ModifierExpiry::Used,
            ModifierKind::EchoNextSpell {
                amount: 1,
                source_id: None,
            },
        );

        // §2.2 cleanup expires "this turn" effects; R30's pending Echo is not turn-scoped, so no
        // number of cleanups on either side reaches it.
        expire_modifiers(&mut sink, P1);
        sink.state.turn += 2;
        expire_modifiers(&mut sink, P1);
        expire_modifiers(&mut sink, P2);
        assert_eq!(mod_ids(sink.state), vec![echo.id.clone()]);

        // It goes the moment it applies.
        consume_modifier(&mut sink, P1, &echo.id);
        assert!(sink.state.players.p1.mods.is_empty());
        let changes = modifier_changes(sink.events);
        assert!(matches_object(
            changes.last().expect("a modifierChanged event"),
            &json!({ "modifierId": echo.id, "added": false })
        ));

        // Consuming it twice is not a second event: nothing was there to remove.
        remove_modifier(&mut sink, P1, &echo.id);
        assert_eq!(removals(sink.events), 1);
    }

    #[test]
    fn n35_lunar_eclipses_discount_applies_to_the_next_spell_only_and_expires_at_cleanup() {
        let mut state = playing("lunar-eclipse");
        let spells = in_hand(&mut state, &cheap_spell().id, P1, 2);
        let first = one(spells.clone());
        let second = spells.get(1).cloned().expect("a second Spell");
        let unit = one(in_hand(&mut state, &trampler.id, P1, 1));
        let turn = state.turn;
        let mut sink = sink_for(&mut state);

        let spell_discount = || ModifierKind::CostDiscount {
            amount: 1,
            only_type: Some(CardType::Spell),
            min_current_cost: None,
            once_per_turn: None,
        };
        let applied = add_modifier(&mut sink, P1, ModifierExpiry::ThisTurn { turn }, spell_discount());

        // "The next Spell you play this turn costs 1 less": a Unit pays full (§8 #35).
        assert_eq!(cost(sink.state, &first), 1);
        assert_eq!(cost(sink.state, &unit), 3);

        // Consumed on use, so the Spell after it pays full: the next Spell *only*.
        consume_modifier(&mut sink, P1, &applied.id);
        assert_eq!(cost(sink.state, &second), 2);

        // Unused, it expires at cleanup instead (§2.2 names it there by name).
        add_modifier(&mut sink, P1, ModifierExpiry::ThisTurn { turn }, spell_discount());
        assert_eq!(cost(sink.state, &second), 1);
        expire_modifiers(&mut sink, P1);
        assert!(sink.state.players.p1.mods.is_empty());
        assert_eq!(cost(sink.state, &second), 2);
    }
}

mod delayed_effects_10_1_r62_r68 {
    use super::*;

    fn ids(effects: Vec<DelayedEffect>) -> Vec<String> {
        effects.into_iter().map(|effect| effect.id).collect()
    }

    #[test]
    fn s10_1_due_delayed_hands_back_only_the_effects_due_for_that_phase_and_player_in_creation_order() {
        let mut state = playing("due-delayed");
        let bolt = put(
            &mut state,
            &delayed_bolt().id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        let bolt_id = delayed_bolt().id;

        let start_a = schedule_delayed(
            &mut sink,
            P1,
            at(Phase::Start, P1),
            resume(&bolt_id, &bolt.id, json!({})),
            None,
            None,
        );
        let end_b = schedule_delayed(
            &mut sink,
            P1,
            at(Phase::End, P1),
            resume(&bolt_id, &bolt.id, json!({})),
            None,
            None,
        );
        let start_c = schedule_delayed(
            &mut sink,
            P2,
            at(Phase::Start, P2),
            resume(&bolt_id, &bolt.id, json!({})),
            None,
            None,
        );
        let start_d = schedule_delayed(
            &mut sink,
            P1,
            at(Phase::Start, P1),
            resume(&bolt_id, &bolt.id, json!({})),
            None,
            None,
        );

        // R68 is "the order they were created", not the order the array happens to hold them in.
        assert!(start_a.seq < start_d.seq);
        sink.state.delayed.reverse();

        assert_eq!(
            ids(due_delayed(sink.state, Phase::Start, P1)),
            vec![start_a.id.clone(), start_d.id.clone()]
        );
        assert_eq!(
            ids(due_delayed(sink.state, Phase::End, P1)),
            vec![end_b.id.clone()]
        );
        assert_eq!(
            ids(due_delayed(sink.state, Phase::Start, P2)),
            vec![start_c.id.clone()]
        );
        assert!(due_delayed(sink.state, Phase::End, P2).is_empty());

        drop_delayed(sink.state, &start_a.id);
        assert_eq!(
            ids(due_delayed(sink.state, Phase::Start, P1)),
            vec![start_d.id.clone()]
        );
    }

    #[test]
    fn r62_start_of_turn_delayed_effects_resolve_before_the_start_of_turn_triggers_and_the_draw_in_creation_order()
     {
        let mut state = playing("delayed-start");
        let bolt = put(
            &mut state,
            &delayed_bolt().id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        {
            let mut sink = sink_for(&mut state);
            schedule_delayed(
                &mut sink,
                P1,
                at(Phase::Start, P1),
                resume(&delayed_bolt().id, &bolt.id, json!({ "amount": 1 })),
                None,
                None,
            );
            schedule_delayed(
                &mut sink,
                P1,
                at(Phase::Start, P1),
                resume(&delayed_bolt().id, &bolt.id, json!({ "amount": 4 })),
                None,
                None,
            );
        }
        state.delayed.reverse();

        state = end_turns(&state, 1);
        let to_p1 = end_turn(&state);
        state = to_p1.state.clone();
        assert_eq!(state.turn, 3);
        assert_eq!(state.active, P1);

        // Creation order, whatever order `state.delayed` held them in (R68).
        assert_eq!(amounts(&to_p1.events), vec![1, 4]);
        assert_eq!(state.players.p2.hero.health, 25);

        // R62: "Refresh → start-of-turn delayed effects → start-of-turn triggers → draw".
        let types = types_of(&to_p1.events);
        let first_damage = index_of(&types, GameEventType::Damage);
        assert!(index_of(&types, GameEventType::TurnStarted) < first_damage);
        assert!(index_of(&types, GameEventType::Drawn) > last_index_of(&types, GameEventType::Damage));

        // Each one resolves once: the queue is drained, not re-read.
        assert!(state.delayed.is_empty());
    }

    #[test]
    fn r62_end_of_turn_delayed_effects_resolve_after_the_end_of_turn_triggers_and_before_cleanup_in_creation_order()
     {
        let mut state = playing("delayed-end");
        put(
            &mut state,
            &end_of_turn_drawer().id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let bolt = put(
            &mut state,
            &delayed_bolt().id,
            slot(P1, Row::Units, 2),
            Default::default(),
        );
        {
            let mut sink = sink_for(&mut state);
            schedule_delayed(
                &mut sink,
                P1,
                at(Phase::End, P1),
                resume(&delayed_bolt().id, &bolt.id, json!({ "amount": 2 })),
                None,
                None,
            );
            schedule_delayed(
                &mut sink,
                P1,
                at(Phase::End, P1),
                resume(&delayed_bolt().id, &bolt.id, json!({ "amount": 5 })),
                None,
                None,
            );
        }
        state.delayed.reverse();

        let ended = end_turn(&state);
        assert_eq!(amounts(&ended.events), vec![2, 5]);
        assert_eq!(ended.state.players.p2.hero.health, 23);

        // R62: "end-of-turn triggers → the end-of-turn trap window → end-of-turn delayed effects →
        // cleanup". `turnEnded` is no longer the cleanup marker: `turn.ts` emits it *before* the trap
        // window, because a trap in that window reads the event (#18 Bread and Butter answers
        // `event.unspentMana`, which only exists while the turn log is still open). So the window's
        // own boundary is read off `turnEnded` and cleanup's off `turnStarted`, the first event the
        // next turn pushes after cleanup has run.
        let types = types_of(&ended.events);
        let first_damage = index_of(&types, GameEventType::Damage);
        assert!(index_of(&types, GameEventType::Drawn) < first_damage);
        assert!(index_of(&types, GameEventType::TurnEnded) < first_damage);
        assert!(index_of(&types, GameEventType::TurnStarted) > last_index_of(&types, GameEventType::Damage));
        assert!(ended.state.delayed.is_empty());
    }

    #[test]
    fn s8_50_k_pop_fanatics_steal_fires_at_the_next_start_of_turn_after_the_unit_has_died_r76() {
        let mut state = playing("kpop-fanatic");
        let kpop = put(
            &mut state,
            &kpop_fanatic().id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        let prize = put(&mut state, &plain.id, slot(P2, Row::Units, 1), Default::default());

        {
            // The Cry's half: a delayed effect keyed to the chosen instance, due at p1's next turn start.
            let mut sink = sink_for(&mut state);
            schedule_delayed(
                &mut sink,
                P1,
                at(Phase::Start, P1),
                resume(&kpop_fanatic().id, &kpop.id, json!({ "target": prize.id })),
                None,
                None,
            );

            // K-Pop Fanatic dies well before its own effect is due.
            find_instance_mut(sink.state, &kpop.id)
                .expect("K-Pop Fanatic")
                .damage = 5;
            state_check(&mut sink);
            assert!(sink.state.players.p1.units[0].is_none());
            assert!(
                sink.state
                    .players
                    .p1
                    .graveyard
                    .iter()
                    .any(|card| card.id == kpop.id)
            );
        }

        // R76: it fires at p1's next start of turn all the same.
        state = end_turns(&state, 2);
        assert_eq!(state.turn, 3);
        let stolen = state.players.p1.units[0].as_ref().and_then(|pile| pile.first());
        assert_eq!(stolen.map(|card| card.id.clone()), Some(prize.id.clone()));
        // R12: control moved and ownership did not.
        assert_eq!(stolen.map(|card| card.controller), Some(P1));
        assert_eq!(stolen.map(|card| card.owner), Some(P2));
        assert!(state.players.p2.units[0].is_none());
        assert!(state.delayed.is_empty());
    }

    #[test]
    fn s8_24_efficiency_dividends_mana_is_a_mana_next_turn_mod_not_a_delayed_effect() {
        let mut state = playing("efficiency-dividend");
        {
            let mut sink = sink_for(&mut state);
            let mut ctx = make_context(
                &mut sink,
                None,
                HookOptions {
                    controller: Some(P1),
                    ..Default::default()
                },
            );

            // "gain floor(X/2) mana next turn" with X = 5 (§8 #24).
            apply_effects(&[next_turn_mana(json_as(json!({ "amount": 2 })))], &mut ctx);
        }

        // The shape: a number on the player's mana. Nothing was queued and no modifier was created.
        assert_eq!(state.players.p1.mana.next_turn_mod, 2);
        assert!(state.delayed.is_empty());
        assert!(due_delayed(&state, Phase::Start, P1).is_empty());
        assert!(due_delayed(&state, Phase::End, P1).is_empty());
        assert!(state.players.p1.mods.is_empty());

        // §2.3: the refresh reads it once, into that turn's current mana — temporary mana, which "adds to
        // current mana" while max stays min(turns, 4) — and clears it.
        state = end_turns(&state, 2);
        assert_eq!(state.players.p1.turns_started, 2);
        assert!(matches_object(
            &json_of(state.players.p1.mana),
            &json!({ "max": 2, "current": 4, "nextTurnMod": 0 })
        ));

        // And it is one-shot: the turn after is the ordinary refresh again.
        state = end_turns(&state, 2);
        assert_eq!(state.players.p1.turns_started, 3);
        assert!(matches_object(
            &json_of(state.players.p1.mana),
            &json!({ "max": 3, "current": 3, "nextTurnMod": 0 })
        ));
    }

    #[test]
    fn r127_a_delayed_effect_whose_instance_has_ceased_to_exist_still_resolves_with_ctx_self_null() {
        let mut state = playing("delayed-missing");
        let ghost = new_instance(&mut state, &delayed_bolt().id, P1, Zone::Gone { player: P1 });
        schedule_delayed(
            &mut sink_for(&mut state),
            P1,
            at(Phase::Start, P1),
            resume(&delayed_bolt().id, &ghost.id, json!({ "amount": 3 })),
            None,
            None,
        );
        SELF_AT_RESUME.with(|cell| cell.set(Vec::new()));

        // The intent this test was written for still holds — the missing instance must not throw:
        // `endTurns` goes through `act`, which rethrows anything `reduce` raised.
        state = end_turns(&state, 2);
        assert_eq!(state.turn, 3);

        // R127: the continuation names its script by stored def id, so it re-enters all the same —
        // dropping it would silently lose a sequence, which R113 forbids. The 3 comes out of
        // `resume.data`, which is where a step keeps what it needs precisely because `self` may be gone.
        assert_eq!(state.players.p2.hero.health, 27);
        assert_eq!(self_at_resume(), vec![None]);

        // Resolved once and dropped: R127 makes it fire, not fire twice.
        assert!(state.delayed.is_empty());
    }
}
