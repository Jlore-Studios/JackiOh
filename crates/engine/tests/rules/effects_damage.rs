//! The `damage` effect itself (BUILD M3-T1 "every effect has its own test file"; SPEC §6.3, §4.4,
//! R85). `damage.test.ts` covers the ten pipeline steps of `dealDamage`; this file covers the
//! factory a card script writes: what it declares, what it hands the pipeline, what it emits, and
//! what it does when its declared target is not there.
//!
//! Port of `packages/engine/test/effects-damage.test.ts`.

use jackioh_engine::effects::damage;
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::Units,
};

use super::fixtures::combat::{armoured, plain, shielded};
use super::fixtures::harness::{events_of_type, new_game, put, slot};

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

/// A Spell, so a damage effect can run with the resolving card as its source.
fn bolt_def() -> CardDef {
    json_as(json!({
        "id": "ed-bolt",
        "index": "1201",
        "name": "Damage effect bolt",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": "bolt" },
        "radiant": { "keywords": [], "text": "bolt" },
    }))
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    let bolt = bolt_def();
    catalog.insert(bolt.id.clone(), bolt);
    register_catalog(catalog);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// A context as a resolving card would see it, so the effect runs the way a script runs it. TS
/// spread `extra` (here only ever `targets`) over the context `makeContext` built.
fn ctx_for<'a>(sink: EngineSink<'a>, self_: Option<&CardInstance>, targets: Option<Vec<Selection>>) -> EffectContext<'a> {
    let mut ctx = make_context(
        sink,
        self_.cloned(),
        HookOptions { controller: Some(P1), ..Default::default() },
    );
    if let Some(targets) = targets {
        ctx.targets = targets;
    }
    ctx
}

fn run(ctx: &mut EffectContext<'_>, effects: Vec<Effect>) {
    for effect in &effects {
        (effect.apply)(ctx);
    }
}

/// A Spell mid-resolution: the source a Spell's own damage carries (§10.5 step 4).
fn resolving_bolt(state: &mut GameState) -> CardInstance {
    let card = new_instance(state, "ed-bolt", P1, Zone::Resolving { player: P1 });
    state.players.p1.resolving.push(card.clone());
    card
}

fn on_instance(instance: &CardInstance) -> Option<Vec<Selection>> {
    Some(vec![Selection::Instance { instance_id: instance.id.clone() }])
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// One field of every `damage` event, in order (TS `eventsOfType(events, "damage").map((e) => e.<key>)`).
fn damage_field(events: &[GameEvent], key: &str) -> Vec<Value> {
    events_of_type(events, GameEventType::Damage).iter().map(|event| json_of(event)[key].clone()).collect()
}

mod the_damage_effect_s6_3_s4_4_r85_m3_t1 {
    use super::*;

    #[test]
    fn declares_its_target_and_amount_and_deals_one_instance_through_the_pipeline_s4_4() {
        let mut state = game("damage-args");
        let self_card = put(&mut state, &plain().id, slot(P1, Units, 1));
        let enemy = put(&mut state, &plain().id, slot(P2, Units, 1));

        let effect = damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 3 })));
        assert_eq!(effect.kind, "damage");

        let mut sink = sink_for(&mut state);
        let mut ctx = ctx_for(sink.sink(), Some(&self_card), on_instance(&enemy));
        run(&mut ctx, vec![effect]);
        assert_eq!(live(ctx.state, &enemy).damage, 3);
        assert_eq!(live(ctx.state, &enemy).last_damaged_by, Some(self_card.id.clone()));
        assert_eq!(
            json_of(&events_of_type(ctx.events, GameEventType::Damage)),
            json!([{ "type": "damage", "sourceId": self_card.id, "targetId": enemy.id, "amount": 3, "combat": false }])
        );
    }

    #[test]
    fn reads_every_target_spec_the_effect_can_name_a_hero_on_either_side_and_itself() {
        let mut state = game("damage-targets");
        let self_card = put(&mut state, &plain().id, slot(P1, Units, 1));
        let mut sink = sink_for(&mut state);
        let mut ctx = ctx_for(sink.sink(), Some(&self_card), None);

        run(&mut ctx, vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 4 })))]);
        assert_eq!(ctx.state.players.p2.hero.health, 26);

        run(&mut ctx, vec![damage(json_as(json!({ "to": { "of": "selfHero" }, "amount": 2 })))]);
        assert_eq!(ctx.state.players.p1.hero.health, 28);

        run(&mut ctx, vec![damage(json_as(json!({ "to": { "of": "self" }, "amount": 1 })))]);
        assert_eq!(live(ctx.state, &self_card).damage, 1);

        assert_eq!(
            damage_field(ctx.events, "targetId"),
            vec![json!("hero-p2"), json!("hero-p1"), json!(self_card.id)]
        );
    }

    #[test]
    fn s4_4_step_2_ignore_armor_puts_the_whole_amount_through_on_a_unit_and_on_a_hero() {
        let mut state = game("damage-ignore-armor");
        let self_card = resolving_bolt(&mut state);
        let armoured_unit = put(&mut state, &armoured().id, slot(P2, Units, 1)); // Armor 7
        state.players.p2.hero.armor = 3;

        // Without the flag Armor eats the hit, and a hit reduced to 0 emits nothing (R63).
        {
            let mut sink = sink_for(&mut state);
            let mut blocked = ctx_for(sink.sink(), Some(&self_card), on_instance(&armoured_unit));
            run(&mut blocked, vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 4 })))]);
            assert_eq!(live(blocked.state, &armoured_unit).damage, 0);
            assert_eq!(events_of_type(blocked.events, GameEventType::Damage).len(), 0);
        }

        {
            let mut sink = sink_for(&mut state);
            let mut through = ctx_for(sink.sink(), Some(&self_card), on_instance(&armoured_unit));
            run(
                &mut through,
                vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 4, "ignoreArmor": true })))],
            );
            assert_eq!(live(through.state, &armoured_unit).damage, 4);
        }

        let mut sink = sink_for(&mut state);
        let mut hero = ctx_for(sink.sink(), Some(&self_card), None);
        run(&mut hero, vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))]);
        assert_eq!(hero.state.players.p2.hero.health, 28); // 5 less the hero's 3 Armor
        run(
            &mut hero,
            vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5, "ignoreArmor": true })))],
        );
        assert_eq!(hero.state.players.p2.hero.health, 23);
    }

    #[test]
    fn marks_the_instance_as_combat_damage_only_when_the_effect_says_so_s4_4() {
        let mut state = game("damage-combat-flag");
        let self_card = put(&mut state, &plain().id, slot(P1, Units, 1));
        let enemy = put(&mut state, &plain().id, slot(P2, Units, 1));
        let mut sink = sink_for(&mut state);
        let mut ctx = ctx_for(sink.sink(), Some(&self_card), on_instance(&enemy));

        run(
            &mut ctx,
            vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1 }))),
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 1, "combat": true }))),
            ],
        );
        assert_eq!(damage_field(ctx.events, "combat"), vec![json!(false), json!(true)]);
    }

    #[test]
    fn r85_heals_the_sources_controller_when_the_effects_own_text_has_lifesteal() {
        let mut state = game("damage-lifesteal");
        let self_card = put(&mut state, &plain().id, slot(P1, Units, 1)); // no Lifesteal keyword of its own
        state.players.p1.hero.health = 20;

        {
            let mut sink = sink_for(&mut state);
            let mut plain_hit = ctx_for(sink.sink(), Some(&self_card), None);
            run(&mut plain_hit, vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 })))]);
            assert_eq!(plain_hit.state.players.p1.hero.health, 20);
            assert_eq!(events_of_type(plain_hit.events, GameEventType::Healed).len(), 0);
        }

        let mut sink = sink_for(&mut state);
        let mut stealing = ctx_for(sink.sink(), Some(&self_card), None);
        run(
            &mut stealing,
            vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3, "lifesteal": true })))],
        );
        assert_eq!(stealing.state.players.p1.hero.health, 23);
        assert_eq!(
            json_of(&events_of_type(stealing.events, GameEventType::Healed)),
            json!([{ "type": "healed", "targetId": "hero-p1", "amount": 3 }])
        );
    }

    #[test]
    fn r85_heals_the_amount_actually_dealt_and_heals_nobody_when_the_effect_has_no_source() {
        let mut state = game("damage-lifesteal-amount");
        let self_card = put(&mut state, &plain().id, slot(P1, Units, 1));
        state.players.p1.hero.health = 20;
        state.players.p2.hero.armor = 4;

        // Step 8 reads the amount after Armor and the cap, not the printed amount.
        {
            let mut sink = sink_for(&mut state);
            let mut capped = ctx_for(sink.sink(), Some(&self_card), None);
            run(
                &mut capped,
                vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 6, "lifesteal": true })))],
            );
            assert_eq!(capped.state.players.p2.hero.health, 28);
            assert_eq!(capped.state.players.p1.hero.health, 22);
        }

        // A Divine Shield negates the hit at step 1, so there is nothing to steal.
        let shielded_unit = put(&mut state, &shielded().id, slot(P2, Units, 1));
        {
            let mut sink = sink_for(&mut state);
            let mut negated = ctx_for(sink.sink(), Some(&self_card), on_instance(&shielded_unit));
            run(
                &mut negated,
                vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5, "lifesteal": true })))],
            );
            assert_eq!(live(negated.state, &shielded_unit).damage, 0);
            assert_eq!(negated.state.players.p1.hero.health, 22);
            assert_eq!(events_of_type(negated.events, GameEventType::Healed).len(), 0);
        }

        // With no instance to be the source the damage still lands, but no hero heals.
        let mut sink = sink_for(&mut state);
        let mut sourceless = ctx_for(sink.sink(), None, None);
        run(
            &mut sourceless,
            vec![damage(json_as(
                json!({ "to": { "of": "enemyHero" }, "amount": 2, "lifesteal": true, "ignoreArmor": true }),
            ))],
        );
        assert_eq!(sourceless.state.players.p2.hero.health, 26);
        assert_eq!(sourceless.state.players.p1.hero.health, 22);
        assert_eq!(damage_field(sourceless.events, "sourceId").first(), Some(&Value::Null));
    }

    #[test]
    fn s6_3_fizzles_with_no_legal_target_nothing_dealt_nothing_emitted() {
        let mut state = game("damage-fizzle");
        let enemy = put(&mut state, &plain().id, slot(P2, Units, 1));
        let mut seen: Vec<Vec<GameEvent>> = Vec::new();

        // A chosen target the play never carried, one that is nowhere, and a mode pick.
        {
            let mut sink = sink_for(&mut state);
            let mut empty = ctx_for(sink.sink(), None, None);
            run(&mut empty, vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 })))]);
            seen.push(empty.events.clone());
        }
        {
            let mut sink = sink_for(&mut state);
            let mut gone = ctx_for(
                sink.sink(),
                None,
                Some(vec![Selection::Instance { instance_id: "no-such-card".into() }]),
            );
            run(&mut gone, vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 })))]);
            seen.push(gone.events.clone());
        }
        {
            let mut sink = sink_for(&mut state);
            let mut mode = ctx_for(sink.sink(), None, Some(vec![Selection::Mode { option: "burn".into() }]));
            run(&mut mode, vec![damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 5 })))]);
            seen.push(mode.events.clone());
        }
        // And "itself" while no instance is resolving (a Spell's own script).
        {
            let mut sink = sink_for(&mut state);
            let mut no_self = ctx_for(sink.sink(), None, None);
            run(&mut no_self, vec![damage(json_as(json!({ "to": { "of": "self" }, "amount": 5 })))]);
            seen.push(no_self.events.clone());
        }

        assert_eq!(live(&state, &enemy).damage, 0);
        assert_eq!(state.players.p1.hero.health, 30);
        assert_eq!(state.players.p2.hero.health, 30);
        for events in &seen {
            assert_eq!(events, &Vec::<GameEvent>::new());
        }
    }

    #[test]
    fn r63_an_amount_of_0_or_less_is_not_a_damage_instance_at_all() {
        let mut state = game("damage-zero");
        let self_card = put(&mut state, &plain().id, slot(P1, Units, 1));
        let shielded_unit = put(&mut state, &shielded().id, slot(P2, Units, 1));
        let mut sink = sink_for(&mut state);
        let mut ctx = ctx_for(sink.sink(), Some(&self_card), on_instance(&shielded_unit));

        run(
            &mut ctx,
            vec![
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": 0 }))),
                damage(json_as(json!({ "to": { "of": "chosen" }, "amount": -3 }))),
            ],
        );
        assert_eq!(live(ctx.state, &shielded_unit).damage, 0);
        assert_eq!(live(ctx.state, &shielded_unit).divine_shield_spent, None);
        assert_eq!(ctx.events.len(), 0);
    }
}
