//! The two cost-changing effects (BUILD M3-T1). R65 owns the order the inputs combine and
//! `effectiveCost` is the only thing that reads them; these tests assert that order rather than
//! recomputing it, plus R78's rule that both inputs travel with the card between zones.
//!
//! Port of `packages/engine/test/effects-cost.test.ts`.

use jackioh_engine::effects::{PlayerModifierSpec, set_cost_mod, set_cost_override};
use jackioh_engine::testkit::*;
use jackioh_engine::{PlayerId::P1, Row::Units};

use super::fixtures::combat::plain;
use super::fixtures::harness::{events_of_type, new_game, put, slot};
use super::fixtures::scripts::x_bolt;

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

/// TS `run`'s options: `{ self?: CardInstance | null; controller?: PlayerId; targets?: Selection[] }`.
#[derive(Default)]
struct RunOptions {
    self_: Option<CardInstance>,
    controller: Option<PlayerId>,
    targets: Option<Vec<Selection>>,
}

/// TS `effect.apply(makeContext(sink, self, rest))`. TS handed over the live card object; the card
/// is looked up again by id, so the context sees it as it stands now.
fn run(sink: &mut EngineSink<'_>, effect: Effect, options: RunOptions) {
    let RunOptions { self_, controller, targets } = options;
    let self_ = self_.map(|card| find_instance(sink.state, &card.id).cloned().unwrap_or(card));
    let mut ctx = make_context(
        sink,
        self_.as_ref(),
        HookOptions { controller, targets, ..Default::default() },
    );
    (effect.apply)(&mut ctx);
}

/// TS `addModifier(sink, player, { kind, ..., expiry })`: the id-less modifier as TS's object literal,
/// split into the `expiry` and `kind` the engine's `add_modifier` takes.
fn add_modifier_spec(sink: &mut EngineSink<'_>, player: PlayerId, spec: PlayerModifierSpec) -> PlayerModifier {
    add_modifier(sink, player, spec.expiry, spec.kind)
}

fn hand_card(state: &mut GameState, def_id: &str) -> CardInstance {
    let card = new_instance(state, def_id, P1, Zone::Hand { player: P1 });
    state.players.p1.hand.push(card.clone());
    card
}

fn on_instance(instance: &CardInstance) -> Vec<Selection> {
    vec![Selection::Instance { instance_id: instance.id.clone() }]
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// `eventsOfType(events, "costChanged").map((e) => e.cost)`.
fn costs(events: &[GameEvent]) -> Vec<i32> {
    events_of_type(events, GameEventType::CostChanged)
        .iter()
        .map(|event| match event {
            GameEvent::CostChanged { cost, .. } => *cost,
            other => panic!("not a costChanged: {other:?}"),
        })
        .collect()
}

mod set_cost_mod_s6_3_cost_r65_m3_t1 {
    use super::*;

    #[test]
    fn r65_adds_to_the_instances_cost_mod_and_reports_the_new_effective_cost() {
        let mut state = new_game("costmod", None);
        let card = hand_card(&mut state, "fx-1"); // printed cost 1
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": -1 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );

        assert_eq!(live(sink.state, &card).cost_mod, -1);
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 0);
        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::CostChanged)),
            json!([{ "type": "costChanged", "instanceId": card.id, "cost": 0 }])
        );

        // #31: each application adds, and the event carries the cost after R65's floor at 0.
        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": 3 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );
        assert_eq!(live(sink.state, &card).cost_mod, 2);
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 3);
        assert_eq!(costs(&sink.events), vec![0, 3]);
    }

    #[test]
    fn r65_the_cost_the_event_reports_is_the_effective_one_with_the_players_discount_included() {
        let mut state = new_game("costmod-discount", None);
        let card = hand_card(&mut state, "fx-1");
        let mut sink = sink_for(&mut state);
        add_modifier_spec(
            &mut sink.sink(),
            P1,
            json_as(json!({ "kind": "costDiscount", "amount": 2, "expiry": { "until": "thisTurn", "turn": 1 } })),
        );

        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": 4 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );

        // printed 1 + costMod 4 − discount 2 = 3.
        assert_eq!(live(sink.state, &card).cost_mod, 4);
        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::CostChanged)),
            json!([{ "type": "costChanged", "instanceId": card.id, "cost": 3 }])
        );
    }

    #[test]
    fn r65_a_cost_mod_never_changes_what_an_x_cost_card_costs_but_an_override_makes_it_free() {
        let mut state = new_game("costmod-x", None);
        let card = hand_card(&mut state, &x_bolt().id); // cost "X"
        find_instance_mut(&mut state, &card.id).unwrap().x = Some(3);
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": -2 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );
        assert_eq!(live(sink.state, &card).cost_mod, -2);
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 3);
        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::CostChanged)),
            json!([{ "type": "costChanged", "instanceId": card.id, "cost": 3 }])
        );

        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 1 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 0);
        assert_eq!(live(sink.state, &card).x, Some(3));
    }

    #[test]
    fn does_nothing_without_a_card_a_hero_selection_an_empty_selection_or_a_zero_amount() {
        let mut state = new_game("costmod-fizzle", None);
        let card = hand_card(&mut state, "fx-1");
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "enemyHero" }, "amount": -1 }))),
            RunOptions { controller: Some(P1), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": -1 }))),
            RunOptions { targets: Some(vec![]), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "self" }, "amount": 0 }))),
            RunOptions { self_: Some(card.clone()), ..Default::default() },
        );

        assert_eq!(live(sink.state, &card).cost_mod, 0);
        assert_eq!(sink.events.len(), 0);
    }
}

mod set_cost_override_s6_3_cost_r65_m3_t1 {
    use super::*;

    #[test]
    fn r65_replaces_the_printed_cost_c54_straazas_1_and_c41r_sheepishs_0() {
        let mut state = new_game("override", None);
        let card = hand_card(&mut state, "fx-1"); // printed 1
        let dear = hand_card(&mut state, "fx-2");
        find_instance_mut(&mut state, &dear.id).unwrap().cost_mod = 3; // printed 1 + 3 = 4 before the override
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 1 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );
        assert_eq!(live(sink.state, &card).cost_override, Some(1));
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 1);

        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 0 }))),
            RunOptions { targets: Some(on_instance(&dear)), ..Default::default() },
        );
        assert_eq!(live(sink.state, &dear).cost_override, Some(0));
        // R65 starts from the override and still adds the instance's costMod.
        assert_eq!(effective_cost(sink.state, live(sink.state, &dear), CostOptions::default()), 3);
        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::CostChanged)),
            json!([
                { "type": "costChanged", "instanceId": card.id, "cost": 1 },
                { "type": "costChanged", "instanceId": dear.id, "cost": 3 },
            ])
        );
    }

    #[test]
    fn r65_and_r48_override_then_cost_mod_then_the_player_discount_then_curvature() {
        let mut state = new_game("override-order", None);
        state.turn = 2; // the turn after the one that made it, so Curvature is live (R48)
        let card = hand_card(&mut state, "fx-1");
        let mut sink = sink_for(&mut state);

        add_modifier_spec(
            &mut sink.sink(),
            P1,
            json_as(json!({ "kind": "costDiscount", "amount": 1, "expiry": { "until": "thisTurn", "turn": 2 } })),
        );
        add_modifier_spec(
            &mut sink.sink(),
            P1,
            json_as(json!({
                "kind": "costDiscount",
                "amount": 1,
                "minCurrentCost": 4,
                "expiry": { "until": "nextTurnOf", "player": "p1", "fromTurn": 0 },
            })),
        );

        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 6 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": -1 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );

        // 6 (override) − 1 (costMod) − 1 (discount) = 4, which Curvature then takes to 3. The override
        // alone was 6 − 1 = 5, which Curvature already reached (R363: 4 or more), so 4.
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 3);
        assert_eq!(costs(&sink.events), vec![4, 3]);

        // Take the override below 4 and Curvature no longer bites: 3 − 1 − 1 = 1.
        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 3 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 1);
    }

    #[test]
    fn r65_a_negative_override_is_floored_at_0_rather_than_turning_into_a_discount() {
        let mut state = new_game("override-floor", None);
        let card = hand_card(&mut state, "fx-1");
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": -5 }))),
            RunOptions { targets: Some(on_instance(&card)), ..Default::default() },
        );

        assert_eq!(live(sink.state, &card).cost_override, Some(0));
        assert_eq!(effective_cost(sink.state, live(sink.state, &card), CostOptions::default()), 0);
    }

    #[test]
    fn r78_cost_mod_and_cost_override_persist_when_the_card_leaves_the_field_while_buffs_reset() {
        let mut state = new_game("cost-persists", None);
        let unit = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            set_cost_mod(json_as(json!({ "target": { "of": "self" }, "amount": 2 }))),
            RunOptions { self_: Some(unit.clone()), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            set_cost_override(json_as(json!({ "target": { "of": "self" }, "cost": 3 }))),
            RunOptions { self_: Some(unit.clone()), ..Default::default() },
        );
        find_instance_mut(sink.state, &unit.id).unwrap().buffs = AttackHealth { attack: 1, health: 1 };

        let mut moving = live(sink.state, &unit).clone();
        move_to_zone(sink.state, &mut moving, OffFieldZone::Graveyard, Default::default());

        let after = live(sink.state, &unit);
        assert_eq!(after.cost_mod, 2);
        assert_eq!(after.cost_override, Some(3));
        assert_eq!(after.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(effective_cost(sink.state, after, CostOptions::default()), 5);
    }
}

mod a_price_for_a_card_in_a_hand_in_hand_only_r4 {
    use super::*;

    #[test]
    fn r4_a_price_given_with_in_hand_only_lands_on_a_card_in_a_hand_and_on_nothing_a_full_hand_burned_s2_4_r78() {
        let mut state = new_game("price-in-hand", None);
        let in_hand = hand_card(&mut state, "fx-1");
        let burned = new_instance(&mut state, "fx-1", P1, Zone::Graveyard { player: P1 });
        state.players.p1.graveyard.push(burned.clone());
        let mut sink = sink_for(&mut state);

        // #31's "+1", #37r's and #72's "costs 1 less", #72r's "costs 0": written after the move, each is
        // the price of a card that reached the hand, and a card the move burned keeps its cost.
        for card in [&in_hand, &burned] {
            run(
                &mut sink.sink(),
                set_cost_mod(json_as(json!({ "target": { "of": "chosen" }, "amount": -1, "inHandOnly": true }))),
                RunOptions { targets: Some(on_instance(card)), ..Default::default() },
            );
            run(
                &mut sink.sink(),
                set_cost_override(json_as(json!({ "target": { "of": "chosen" }, "cost": 0, "inHandOnly": true }))),
                RunOptions { targets: Some(on_instance(card)), ..Default::default() },
            );
        }

        let held = live(sink.state, &in_hand);
        assert_eq!((held.cost_mod, held.cost_override), (-1, Some(0)));
        let kept = live(sink.state, &burned);
        assert_eq!((kept.cost_mod, kept.cost_override), (0, None));
        let ids: Vec<String> = events_of_type(&sink.events, GameEventType::CostChanged)
            .iter()
            .map(|event| match event {
                GameEvent::CostChanged { instance_id, .. } => instance_id.clone(),
                other => panic!("not a costChanged: {other:?}"),
            })
            .collect();
        assert_eq!(ids, vec![in_hand.id.clone(), in_hand.id.clone()]);
    }
}
