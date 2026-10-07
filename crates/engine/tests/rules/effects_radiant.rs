//! Make Radiant (SPEC §6.3, §5.2, BUILD M3-T1): the on-field layer swap without a Cry (R22), a card
//! with no radiant form still setting the flag (R74), and R60's random picks. The fixture cards
//! these tests need are registered here, so no shared fixture has to grow for them (BUILD §0).
//!
//! Port of `packages/engine/test/effects-radiant.test.ts`.

use std::collections::BTreeSet;

use jackioh_engine::testkit::*;

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::config::HERO_HEALTH;
use jackioh_engine::effects::{damage, set_radiant, set_radiant_random};
use jackioh_engine::layers::{unit_has, unit_view};
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{CardScripts, EngineSink, Effect, Script, hook};
use jackioh_engine::scripts::registered_scripts;
use jackioh_engine::state::{CardInstance, GameState, find_instance, find_instance_mut, new_instance};
use jackioh_engine::zones::{PlaceOnFieldOptions, place_on_field};

use super::fixtures::catalog::unit_def;
use super::fixtures::combat::{plain, stacker};
use super::fixtures::harness::{in_hand, new_game, put, set_library, slot};

/// A Cry that would be loud if making a card Radiant ever re-fired one (R22).
fn crier() -> CardDef {
    unit_def(731, json!({ "id": "rd-crier", "name": "Crier (fixture)", "attack": 2, "health": 2 }))
}

/// A radiant face that adds Taunt to a base face with no keywords (§5.2).
fn glow_up() -> CardDef {
    CardDef {
        radiant: json_as(json!({ "attack": 4, "health": 4, "keywords": [{ "kind": "Taunt" }], "text": "radiant" })),
        ..unit_def(732, json!({ "id": "rd-glow-up", "name": "Glow Up (fixture)", "attack": 2, "health": 2 }))
    }
}

/// R74: a card with no listed Radiant form is unchanged by it, but the flag still sets.
fn no_form() -> CardDef {
    CardDef {
        radiant: json_as(json!({ "attack": 3, "health": 3, "keywords": [], "text": "same" })),
        ..unit_def(733, json!({ "id": "rd-no-form", "name": "No Radiant Form (fixture)", "attack": 3, "health": 3 }))
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        crier().id,
        CardScripts {
            base: Script {
                cry: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))])),
                ..Script::default()
            },
            radiant: Script {
                cry: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 9 })))])),
                ..Script::default()
            },
        },
    );
    scripts
}

fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in [crier(), glow_up(), no_form()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state
}

/// Apply one effect the way `resolve.ts` does, and hand back the events it emitted. `self_` is TS's
/// `options.self` (null by default); `hook` the rest of the options.
fn run(state: &mut GameState, effect: Effect, self_: Option<&CardInstance>, hook: HookOptions) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, self_, hook);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn as_p1() -> HookOptions {
    HookOptions {
        controller: Some(PlayerId::P1),
        ..Default::default()
    }
}

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

fn radiant_ids(events: &[GameEvent]) -> Vec<String> {
    of_type(events, "radiantSet")
        .iter()
        .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
        .collect()
}

/// The card under `id` as it stands in the state now (TS read the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn named(id: &str) -> Effect {
    set_radiant(json_as(json!({ "instanceId": id })))
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

mod r22_r74_make_radiant_on_a_named_card_sec6_3_sec5_2_m3_t1 {
    use super::*;

    #[test]
    fn r22_swaps_the_base_stat_layer_at_once_keeps_damage_and_buffs_and_re_fires_no_cry() {
        let mut state = game("radiant-test");
        // The fixture 2/2 has a 4/4 radiant face.
        let unit = put(&mut state, &crier().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        {
            let card = find_instance_mut(&mut state, &unit.id).expect("the unit");
            card.damage = 1;
            card.buffs = AttackHealth { attack: 1, health: 1 };
        }
        assert_eq!(unit_view(&state, live(&state, &unit.id)).attack, 3);
        assert_eq!(unit_view(&state, live(&state, &unit.id)).health, 2);

        let events = run(&mut state, named(&unit.id), None, as_p1());

        let now = live(&state, &unit.id);
        assert!(now.radiant);
        let view = unit_view(&state, now);
        // Printed 2/2 becomes 4/4 and the layer-4 buff is still on top of it.
        assert_eq!(view.attack, 5);
        assert_eq!(view.max_health, 5);
        // Damage taken stays, so health is the new max minus the damage already on the card.
        assert_eq!(view.health, 4);
        assert_eq!(now.damage, 1);
        assert_eq!(now.buffs, AttackHealth { attack: 1, health: 1 });

        // R22: the Cry does not re-fire, so the enemy hero takes nothing.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(
            events
                .iter()
                .map(|event| serde_json::to_value(event).expect("an event serialises"))
                .collect::<Vec<_>>(),
            vec![json!({
                "type": "radiantSet",
                "instanceId": unit.id,
                "defId": crier().id,
                "zone": { "z": "field", "player": "p1", "row": "units", "lane": 2 },
            })]
        );
    }

    #[test]
    fn r22_applies_a_keyword_the_radiant_face_adds_at_once() {
        let mut state = game("radiant-test");
        let unit = put(&mut state, &glow_up().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        assert!(!unit_has(&state, live(&state, &unit.id), KeywordKind::Taunt));

        run(&mut state, named(&unit.id), None, as_p1());

        assert!(unit_has(&state, live(&state, &unit.id), KeywordKind::Taunt));
        assert_eq!(unit_view(&state, live(&state, &unit.id)).attack, 4);
    }

    #[test]
    fn r22_81_targets_the_card_whose_script_is_running_so_radiant_saintess_includes_itself() {
        let mut state = game("radiant-test");
        let saintess = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 4), json!({}));

        let events = run(
            &mut state,
            set_radiant(json_as(json!({ "target": { "of": "self" } }))),
            Some(&saintess),
            HookOptions::default(),
        );

        assert!(live(&state, &saintess.id).radiant);
        assert_eq!(radiant_ids(&events), vec![saintess.id.clone()]);
        assert_eq!(unit_view(&state, live(&state, &saintess.id)).attack, 6);
    }

    #[test]
    fn sec6_3_does_nothing_to_a_card_that_is_already_radiant() {
        let mut state = game("radiant-test");
        let unit = put(
            &mut state,
            &plain().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({ "radiant": true }),
        );

        assert_eq!(run(&mut state, named(&unit.id), None, as_p1()), Vec::<GameEvent>::new());
        assert!(live(&state, &unit.id).radiant);
    }

    #[test]
    fn r74_sets_the_flag_on_a_card_whose_radiant_form_is_the_same_as_its_base() {
        let mut state = game("radiant-test");
        let unit = put(&mut state, &no_form().id, slot(PlayerId::P1, Row::Units, 1), json!({}));

        let events = run(&mut state, named(&unit.id), None, as_p1());

        // The flag still sets, so counting effects behave, and the stats are unchanged.
        assert!(live(&state, &unit.id).radiant);
        assert_eq!(radiant_ids(&events), vec![unit.id.clone()]);
        assert_eq!(unit_view(&state, live(&state, &unit.id)).attack, 3);
        assert_eq!(unit_view(&state, live(&state, &unit.id)).max_health, 3);
    }

    #[test]
    fn sec5_2_swaps_a_hand_cards_stats_and_text_where_it_sits() {
        let mut state = game("radiant-test");
        let held = in_hand(&mut state, &plain().id, PlayerId::P1, 1)
            .into_iter()
            .next()
            .expect("a hand card");

        let events = run(&mut state, named(&held.id), None, as_p1());

        let now = live(&state, &held.id);
        assert!(now.radiant);
        assert_eq!(now.zone, Zone::Hand { player: PlayerId::P1 });
        assert_eq!(unit_view(&state, now).attack, 6);
        assert_eq!(of_type(&events, "radiantSet")[0]["zone"], json!({ "z": "hand", "player": "p1" }));
    }

    #[test]
    fn makes_nothing_radiant_when_no_target_was_picked() {
        let mut state = game("radiant-test");
        assert_eq!(
            run(&mut state, set_radiant(Default::default()), None, as_p1()),
            Vec::<GameEvent>::new()
        );
    }
}

mod r60_make_radiant_at_random_m3_t1 {
    use super::*;

    fn at_random(args: Value) -> Effect {
        set_radiant_random(json_as(args))
    }

    #[test]
    fn r60_chooses_only_among_non_radiant_cards() {
        let mut state = game("radiant-test");
        let hand = in_hand(&mut state, &plain().id, PlayerId::P1, 3);
        find_instance_mut(&mut state, &hand[0].id).expect("hand card").radiant = true;
        find_instance_mut(&mut state, &hand[2].id).expect("hand card").radiant = true;

        let events = run(&mut state, at_random(json!({ "zones": "hand", "count": 3 })), None, as_p1());

        // Only one card was eligible, so only it changes: a pick never lands on a Radiant card.
        assert_eq!(
            hand.iter().map(|card| live(&state, &card.id).radiant).collect::<Vec<_>>(),
            vec![true, true, true]
        );
        // R177: the two picks R60 could not make are cued on the hand's Radiant cards, so the other
        // seat's stream holds three cues whatever the hidden hand held — and the pick and the cues go out
        // together in hand order, so where the real pick stands among them says nothing either.
        assert_eq!(radiant_ids(&events), ids(&hand));
    }

    #[test]
    fn r60_r177_r129_changes_no_card_and_draws_nothing_when_no_non_radiant_card_is_left_though_the_hidden_hand_is_cued() {
        let mut state = game("radiant-test");
        let hand = in_hand(&mut state, &plain().id, PlayerId::P1, 2);
        for card in &hand {
            find_instance_mut(&mut state, &card.id).expect("hand card").radiant = true;
        }
        let cursor = state.rng_cursor;

        let events = run(&mut state, at_random(json!({ "zones": "hand", "count": 2 })), None, as_p1());
        // Nothing changes and no random number is drawn (R129)...
        assert_eq!(state.rng_cursor, cursor);
        assert!(hand.iter().all(|card| live(&state, &card.id).radiant));
        // ...and the hidden hand is cued as a pick of two would cue it (R177).
        assert_eq!(radiant_ids(&events), ids(&hand));
    }

    #[test]
    fn r60_28_picks_n_different_cards_from_the_union_of_hand_library_and_field() {
        let mut state = game("radiant-test");
        let hand = in_hand(&mut state, &plain().id, PlayerId::P1, 2);
        let library = set_library(&mut state, PlayerId::P1, &[plain().id.as_str(), crier().id.as_str()]);
        let on_field = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let pool: Vec<CardInstance> = hand.iter().chain(library.iter()).cloned().chain([on_field]).collect();

        let events = run(
            &mut state,
            at_random(json!({ "zones": ["hand", "library", "field"], "count": 4 })),
            None,
            as_p1(),
        );

        let picked = radiant_ids(&events);
        assert_eq!(picked.len(), 4);
        assert_eq!(picked.iter().collect::<BTreeSet<_>>().len(), 4);
        assert!(picked.iter().all(|id| pool.iter().any(|card| &card.id == id)));
        assert_eq!(pool.iter().filter(|card| live(&state, &card.id).radiant).count(), 4);

        // More than the pool holds takes all of it (R60).
        let mut all = game("radiant-test");
        let all_hand = in_hand(&mut all, &plain().id, PlayerId::P1, 2);
        let all_field = put(&mut all, &plain().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let everything = run(
            &mut all,
            at_random(json!({ "zones": ["hand", "field"], "count": 9 })),
            None,
            as_p1(),
        );
        let expected: BTreeSet<String> = all_hand.iter().chain([&all_field]).map(|card| card.id.clone()).collect();
        assert_eq!(radiant_ids(&everything).into_iter().collect::<BTreeSet<_>>(), expected);
    }

    #[test]
    fn r13_offers_only_the_top_of_a_stack_pile_never_the_dormant_card_beneath() {
        let mut state = game("radiant-test");
        let beneath = put(&mut state, &plain().id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let mut top = new_instance(&mut state, &stacker().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            &slot(PlayerId::P1, Row::Units, 3),
            PlaceOnFieldOptions { stack: Some(true) }
        ));

        let events = run(&mut state, at_random(json!({ "zones": "field", "count": 5 })), None, as_p1());

        assert_eq!(radiant_ids(&events), vec![top.id.clone()]);
        assert!(!live(&state, &beneath.id).radiant);
    }

    #[test]
    fn reads_the_enemys_zone_when_the_effect_names_it() {
        let mut state = game("radiant-test");
        let mine = in_hand(&mut state, &plain().id, PlayerId::P1, 1)
            .into_iter()
            .next()
            .expect("p1's card");
        let theirs = in_hand(&mut state, &plain().id, PlayerId::P2, 1)
            .into_iter()
            .next()
            .expect("p2's card");

        let events = run(
            &mut state,
            at_random(json!({ "zones": "hand", "player": "enemy", "count": 1 })),
            None,
            as_p1(),
        );

        assert_eq!(radiant_ids(&events), vec![theirs.id.clone()]);
        assert!(!live(&state, &mine.id).radiant);
    }

    #[test]
    fn draws_from_the_match_rng_so_the_same_seed_picks_the_same_cards_and_another_seed_does_not() {
        let picks = |seed: &str| -> Vec<String> {
            let mut state = game(seed);
            in_hand(&mut state, &plain().id, PlayerId::P1, 6);
            radiant_ids(&run(&mut state, at_random(json!({ "zones": "hand", "count": 2 })), None, as_p1()))
        };

        assert_eq!(picks("radiant-seed-a"), picks("radiant-seed-a"));
        assert_ne!(picks("radiant-seed-a"), picks("radiant-seed-b"));
        // The pick also moves the cursor on, so nothing draws the same numbers twice.
        let mut state = game("radiant-test");
        in_hand(&mut state, &plain().id, PlayerId::P1, 6);
        assert_eq!(state.rng_cursor, 0);
        run(&mut state, at_random(json!({ "zones": "hand", "count": 2 })), None, as_p1());
        assert!(state.rng_cursor > 0);
    }
}
