//! Exile, Bounce, Discard and Counter (SPEC §6.3, §2.4, §3.2, R11, R12, R16, R78, BUILD M3-T1).
//! Fixture defs and scripts are registered here on top of the shared catalog, so no shared fixture has
//! to grow for them (CLAUDE.md, BUILD §0).

use jackioh_engine::testkit::*;

use jackioh_engine::announce::begin_announce;
use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::config::{HAND_CAP, HERO_HEALTH};
use jackioh_engine::effects::{bounce, counter, damage, discard, discard_random, exile};
use jackioh_engine::play_choices::gifted_makes_radiant;
use jackioh_engine::resolve::{HookOptions, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{CardScripts, Effect, EngineSink, Script, hook};
use jackioh_engine::scripts::registered_scripts;
use jackioh_engine::state::{
    AnnounceRecord, CardInstance, GameState, find_instance, find_instance_mut, new_instance,
};
use jackioh_engine::zones::{PlaceOnFieldOptions, card_at, place_on_field};

use super::fixtures::catalog::token_def;
use super::fixtures::harness::{in_hand, new_game, put, slot};

// Fixture cards.

fn unit_def_of(name: &str, index: i32) -> CardDef {
    json_as(json!({
        "id": format!("mv-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (move)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    }))
}

/// A unit with both a Cry and a Death hook: Exile and Counter must fire neither.
fn noisy() -> CardDef {
    unit_def_of("noisy", 801)
}

/// A permanent carrying #64 Gifted Program's static flag (threshold 1), for R213's count.
fn gifted() -> CardDef {
    unit_def_of("gifted", 802)
}

/// A unit-token card that can sit in a hand (#75), for R11's "leaves that zone" clause.
fn hand_token() -> CardDef {
    CardDef {
        id: "mv-hand-token".to_string(),
        index: "T-hand".to_string(),
        ..token_def("rush", vec![Tag::Token])
    }
}

fn rush_token() -> CardDef {
    token_def("rush", vec![Tag::Token])
}

fn defs() -> Vec<CardDef> {
    vec![noisy(), gifted(), hand_token()]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn local_scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        noisy().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 4 }),
                ))]
            })),
            death: Some(hook(|_ctx| {
                vec![damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 6 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        gifted().id,
        both(Script {
            static_flags: Some(json_as(json!({ "giftedProgram": 1 }))),
            ..Script::default()
        }),
    );
    scripts
}

/// A fresh game whose catalog and script registry also carry this file's fixtures.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.extend(local_scripts());
    register_scripts(scripts);
    state.turn = 3;
    state
}

#[derive(Default)]
struct RunOptions {
    controller: Option<PlayerId>,
    self_: Option<CardInstance>,
}

/// Apply one effect the way a script's hook would, and hand back the events it emitted.
fn run(
    state: &mut GameState,
    effect: Effect,
    target: Option<&CardInstance>,
    options: RunOptions,
) -> Vec<GameEvent> {
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let targets: Vec<Selection> = match target {
            None => Vec::new(),
            Some(card) => vec![Selection::Instance {
                instance_id: card.id.clone(),
            }],
        };
        let mut ctx = make_context(
            &mut sink,
            options.self_.as_ref(),
            HookOptions {
                controller: Some(options.controller.unwrap_or(PlayerId::P1)),
                targets: Some(targets),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn as_controller(player: PlayerId) -> RunOptions {
    RunOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn chosen() -> Value {
    json!({ "target": { "of": "chosen" } })
}

fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

fn ids_of_type(events: &[GameEvent], kind: &str) -> Vec<String> {
    of_type(events, kind)
        .iter()
        .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
        .collect()
}

fn types(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| {
            serde_json::to_value(event).expect("an event serialises")["type"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect()
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// The card under `id` as it stands in the state now.
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card was put in hand")
}

fn at(state: &GameState, player: PlayerId, row: Row, lane: i32) -> Option<String> {
    card_at(state, slot(player, row, lane)).map(|card| card.id.clone())
}

/// A unit p1 controls that `player` owns: made in `player`'s hand, placed on p1's unit lane 1.
fn placed_from_hand_of(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    assert!(place_on_field(
        state,
        &mut card,
        slot(PlayerId::P1, Row::Units, 1),
        PlaceOnFieldOptions::default()
    ));
    card
}

// exile

mod exile_s6_3_m3_t1 {
    use super::*;

    #[test]
    fn r55_s6_3_moves_a_unit_from_the_field_to_its_owners_exile_pile_and_counts_it() {
        let mut state = game("effects-move");
        let victim = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 2), json!({}));

        let events = run(
            &mut state,
            exile(json_as(chosen())),
            Some(&victim),
            RunOptions::default(),
        );

        assert_eq!(at(&state, PlayerId::P1, Row::Units, 2), None);
        assert_eq!(ids(&state.players.p1.exile), vec![victim.id.clone()]);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(state.counters.exiled, 1);
        assert_eq!(
            of_type(&events, "exiled"),
            vec![json!({ "type": "exiled", "instanceId": victim.id, "defId": "fx-1", "owner": "p1" })]
        );
    }

    #[test]
    fn s6_3_exiles_from_anywhere_a_hand_card_and_a_graveyard_card_both_reach_the_pile() {
        let mut state = game("effects-move");
        let from_hand = first(in_hand(&mut state, "fx-1", PlayerId::P1, 1));
        let from_graveyard = new_instance(
            &mut state,
            "fx-2",
            PlayerId::P1,
            Zone::Graveyard { player: PlayerId::P1 },
        );
        state.players.p1.graveyard.push(from_graveyard.clone());

        run(
            &mut state,
            exile(json_as(chosen())),
            Some(&from_hand),
            RunOptions::default(),
        );
        run(
            &mut state,
            exile(json_as(chosen())),
            Some(&from_graveyard),
            RunOptions::default(),
        );

        assert_eq!(state.players.p1.hand.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(
            state
                .players
                .p1
                .exile
                .iter()
                .map(|c| c.def_id.clone())
                .collect::<Vec<_>>(),
            vec!["fx-1", "fx-2"]
        );
        assert_eq!(state.counters.exiled, 2);
    }

    #[test]
    fn s6_3_fires_no_death_trigger() {
        let mut state = game("effects-move");
        let victim = put(
            &mut state,
            &noisy().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        let events = run(
            &mut state,
            exile(json_as(chosen())),
            Some(&victim),
            RunOptions::default(),
        );

        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert!(of_type(&events, "damage").is_empty());
        assert!(of_type(&events, "destroyed").is_empty());
    }

    #[test]
    fn r11_an_exiled_unit_token_vanishes_and_never_enters_the_exile_pile() {
        let mut state = game("effects-move");
        let token = put(
            &mut state,
            &rush_token().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        let events = run(
            &mut state,
            exile(json_as(chosen())),
            Some(&token),
            RunOptions::default(),
        );

        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1), None);
        assert_eq!(state.players.p1.exile.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(state.counters.exiled, 0);
        assert_eq!(ids_of_type(&events, "exiled"), vec![token.id.clone()]);
    }

    #[test]
    fn r12_a_stolen_unit_is_exiled_to_its_owners_pile() {
        let mut state = game("effects-move");
        let theirs = placed_from_hand_of(&mut state, "fx-4", PlayerId::P2);

        run(
            &mut state,
            exile(json_as(chosen())),
            Some(&theirs),
            as_controller(PlayerId::P1),
        );

        assert_eq!(ids(&state.players.p2.exile), vec![theirs.id.clone()]);
        assert_eq!(state.players.p1.exile.len(), 0);
    }

    #[test]
    fn r78_r766_exile_resets_the_instance_and_its_price_but_keeps_radiant() {
        let mut state = game("effects-move");
        let victim = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        {
            let card = live_mut(&mut state, &victim.id);
            card.damage = 1;
            card.buffs = AttackHealth { attack: 2, health: 2 };
            card.radiant = true;
            card.cost_mod = -2;
        }

        run(
            &mut state,
            exile(json_as(chosen())),
            Some(&victim),
            RunOptions::default(),
        );

        let now = live(&state, &victim.id);
        assert_eq!(now.damage, 0);
        assert_eq!(now.buffs, AttackHealth { attack: 0, health: 0 });
        assert!(now.radiant);
        // R766: an exile pile takes the price too, so the card costs its printed cost there.
        assert_eq!(now.cost_mod, 0);
    }
}

// bounce

mod bounce_s6_3_m3_t1 {
    use super::*;

    #[test]
    fn r78_returns_a_unit_to_its_controllers_hand_and_drops_its_damage_buffs_and_position() {
        let mut state = game("effects-move");
        let victim = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 3), json!({}));
        {
            let card = live_mut(&mut state, &victim.id);
            card.damage = 1;
            card.buffs = AttackHealth { attack: 3, health: 3 };
            card.position = Some(Position::Def);
            card.counters = Counters {
                plague: Some(2),
                ..Default::default()
            };
            card.cost_mod = -1;
            card.radiant = true;
        }

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&victim),
            RunOptions::default(),
        );

        assert_eq!(at(&state, PlayerId::P1, Row::Units, 3), None);
        assert_eq!(ids(&state.players.p1.hand), vec![victim.id.clone()]);
        let now = live(&state, &victim.id);
        assert_eq!(now.damage, 0);
        assert_eq!(now.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(now.position, None);
        assert_eq!(now.counters, Counters::default());
        assert_eq!(now.cost_mod, -1);
        assert!(now.radiant);
        assert_eq!(
            of_type(&events, "bounced"),
            vec![json!({ "type": "bounced", "instanceId": victim.id, "defId": "fx-1", "owner": "p1" })]
        );
        assert_eq!(ids_of_type(&events, "addedToHand"), vec![victim.id.clone()]);
    }

    #[test]
    fn r11_a_bounced_unit_token_vanishes_instead_of_reaching_a_hand() {
        let mut state = game("effects-move");
        let token = put(
            &mut state,
            &rush_token().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&token),
            RunOptions::default(),
        );

        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1), None);
        assert_eq!(state.players.p1.hand.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(ids_of_type(&events, "bounced"), vec![token.id.clone()]);
        assert!(of_type(&events, "addedToHand").is_empty());
    }

    #[test]
    fn s2_4_a_bounce_into_a_full_hand_burns_the_card_to_the_graveyard() {
        let mut state = game("effects-move");
        in_hand(&mut state, "fx-2", PlayerId::P1, HAND_CAP);
        let victim = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&victim),
            RunOptions::default(),
        );

        assert_eq!(state.players.p1.hand.len(), HAND_CAP as usize);
        assert_eq!(ids(&state.players.p1.graveyard), vec![victim.id.clone()]);
        assert_eq!(ids_of_type(&events, "bounced"), vec![victim.id.clone()]);
        assert_eq!(ids_of_type(&events, "burned"), vec![victim.id.clone()]);
        assert_eq!(ids_of_type(&events, "enteredGraveyard"), vec![victim.id.clone()]);
    }

    #[test]
    fn r747_a_stolen_unit_bounces_to_its_controllers_hand_and_becomes_the_controllers_card() {
        let mut state = game("effects-move");
        let theirs = placed_from_hand_of(&mut state, "fx-4", PlayerId::P2);

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&theirs),
            as_controller(PlayerId::P1),
        );

        assert_eq!(ids(&state.players.p1.hand), vec![theirs.id.clone()]);
        assert_eq!(state.players.p2.hand.len(), 0);
        let now = live(&state, &theirs.id);
        assert_eq!(now.owner, PlayerId::P1);
        assert_eq!(now.controller, PlayerId::P1);
        assert_eq!(now.zone, Zone::Hand { player: PlayerId::P1 });
        assert_eq!(
            of_type(&events, "bounced"),
            vec![json!({ "type": "bounced", "instanceId": theirs.id, "defId": "fx-4", "owner": "p1" })]
        );
        assert_eq!(
            of_type(&events, "addedToHand"),
            vec![json!({ "type": "addedToHand", "player": "p1", "instanceId": theirs.id, "defId": "fx-4" })]
        );
    }

    #[test]
    fn r747_a_stolen_unit_bounced_into_a_full_controller_hand_burns_to_the_controllers_graveyard() {
        let mut state = game("effects-move");
        in_hand(&mut state, "fx-2", PlayerId::P1, HAND_CAP);
        let theirs = placed_from_hand_of(&mut state, "fx-4", PlayerId::P2);

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&theirs),
            as_controller(PlayerId::P1),
        );

        assert_eq!(state.players.p1.hand.len(), HAND_CAP as usize);
        assert_eq!(ids(&state.players.p1.graveyard), vec![theirs.id.clone()]);
        assert_eq!(state.players.p2.graveyard.len(), 0);
        assert_eq!(ids_of_type(&events, "bounced"), vec![theirs.id.clone()]);
        assert_eq!(ids_of_type(&events, "burned"), vec![theirs.id.clone()]);
    }

    #[test]
    fn s6_3_fires_no_death_trigger() {
        let mut state = game("effects-move");
        let victim = put(
            &mut state,
            &noisy().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&victim),
            RunOptions::default(),
        );

        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert!(of_type(&events, "damage").is_empty());
    }

    #[test]
    fn s6_3_leaves_a_card_that_is_already_in_its_owners_hand_alone() {
        let mut state = game("effects-move");
        let card = first(in_hand(&mut state, "fx-1", PlayerId::P1, 1));

        let events = run(
            &mut state,
            bounce(json_as(chosen())),
            Some(&card),
            RunOptions::default(),
        );

        assert_eq!(ids(&state.players.p1.hand), vec![card.id.clone()]);
        assert!(events.is_empty());
    }
}

// discard

mod r16_discard_s6_3_m3_t1 {
    use super::*;

    #[test]
    fn r16_a_named_card_goes_from_the_hand_to_the_graveyard() {
        let mut state = game("effects-move");
        let pair = in_hand(&mut state, "fx-1", PlayerId::P1, 2);
        let (keep, toss) = (pair[0].clone(), pair[1].clone());

        let events = run(
            &mut state,
            discard(json_as(chosen())),
            Some(&toss),
            RunOptions::default(),
        );

        assert_eq!(ids(&state.players.p1.hand), vec![keep.id.clone()]);
        assert_eq!(ids(&state.players.p1.graveyard), vec![toss.id.clone()]);
        assert_eq!(types(&events), vec!["discarded", "enteredGraveyard"]);
        assert_eq!(of_type(&events, "discarded")[0]["owner"], json!("p1"));
    }

    #[test]
    fn r16_the_random_form_draws_its_pick_from_the_match_rng() {
        let mut state = game("effects-move");
        let hand = ids(&in_hand(&mut state, "fx-1", PlayerId::P1, 5));
        let expected = hand[Rng::new(&state.seed, state.rng_cursor).int(hand.len() as i32) as usize].clone();

        let events = run(
            &mut state,
            discard_random(Default::default()),
            None,
            RunOptions::default(),
        );

        assert_eq!(ids(&state.players.p1.graveyard), vec![expected.clone()]);
        assert_eq!(ids_of_type(&events, "discarded"), vec![expected]);
        assert_eq!(state.rng_cursor, 1);
    }

    #[test]
    fn r16_the_random_form_replays_identically_from_the_same_seed() {
        let mut first_game = game("replay-seed");
        in_hand(&mut first_game, "fx-1", PlayerId::P1, 4);
        run(
            &mut first_game,
            discard_random(json_as(json!({ "count": 2 }))),
            None,
            RunOptions::default(),
        );

        let mut second = game("replay-seed");
        in_hand(&mut second, "fx-1", PlayerId::P1, 4);
        run(
            &mut second,
            discard_random(json_as(json!({ "count": 2 }))),
            None,
            RunOptions::default(),
        );

        assert_eq!(
            ids(&first_game.players.p1.graveyard),
            ids(&second.players.p1.graveyard)
        );
        assert_eq!(first_game.players.p1.graveyard.len(), 2);
    }

    #[test]
    fn r16_a_random_discard_of_more_cards_than_the_hand_holds_empties_it_and_stops() {
        let mut state = game("effects-move");
        in_hand(&mut state, "fx-1", PlayerId::P1, 2);

        run(
            &mut state,
            discard_random(json_as(json!({ "count": 5 }))),
            None,
            RunOptions::default(),
        );

        assert_eq!(state.players.p1.hand.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 2);
    }

    #[test]
    fn r11_a_discarded_unit_token_card_vanishes_and_reaches_no_graveyard() {
        let mut state = game("effects-move");
        let token = first(in_hand(&mut state, &hand_token().id, PlayerId::P1, 1));

        let events = run(
            &mut state,
            discard(json_as(chosen())),
            Some(&token),
            RunOptions::default(),
        );

        assert_eq!(state.players.p1.hand.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(types(&events), vec!["discarded"]);
    }

    #[test]
    fn s6_3_discards_the_enemys_hand_when_the_effect_says_so() {
        let mut state = game("effects-move");
        in_hand(&mut state, "fx-1", PlayerId::P2, 3);

        run(
            &mut state,
            discard_random(json_as(json!({ "player": "enemy" }))),
            None,
            as_controller(PlayerId::P1),
        );

        assert_eq!(state.players.p2.hand.len(), 2);
        assert_eq!(state.players.p2.graveyard.len(), 1);
        assert_eq!(state.players.p1.graveyard.len(), 0);
    }

    #[test]
    fn s6_3_ignores_a_card_that_is_not_in_a_hand() {
        let mut state = game("effects-move");
        let unit = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));

        let events = run(
            &mut state,
            discard(json_as(chosen())),
            Some(&unit),
            RunOptions::default(),
        );

        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1), Some(unit.id.clone()));
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert!(events.is_empty());
    }
}

// counter

/// B5 E1, R448: what §10.5's announce leaves for a Counter to answer — the card moved out of its
/// player's hand into the resolving zone and its announce open. These pin the verb.
fn announced(state: &mut GameState, card: Option<CardInstance>) -> CardInstance {
    let Some(mut card) = card else {
        panic!("no card to announce");
    };
    let controller = card.controller;
    let side = &mut state.players[controller];
    side.hand.retain(|held| held.id != card.id);
    card.zone = Zone::Resolving { player: controller };
    side.resolving.push(card.clone());
    begin_announce(
        state,
        AnnounceRecord {
            instance_id: card.id.clone(),
            player: controller,
            face_down: None,
            countered: None,
        },
    );
    card
}

fn first_in_hand(state: &mut GameState, def_id: &str, player: PlayerId) -> Option<CardInstance> {
    in_hand(state, def_id, player, 1).into_iter().next()
}

mod r448_counter_s6_3_m3_t1_b5_e1 {
    use super::*;

    #[test]
    fn s6_3_sends_the_announced_card_to_the_graveyard_with_no_cry_and_no_death() {
        let mut state = game("effects-move");
        let held = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let card = announced(&mut state, held);

        let events = run(
            &mut state,
            counter(json_as(chosen())),
            Some(&card),
            RunOptions::default(),
        );

        assert_eq!(state.players.p1.resolving.len(), 0);
        assert_eq!(ids(&state.players.p1.graveyard), vec![card.id.clone()]);
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(types(&events), vec!["countered", "enteredGraveyard"]);
        assert!(of_type(&events, "destroyed").is_empty());
    }

    #[test]
    fn r448_a_counter_counts_nothing_back_the_play_it_cancels_was_never_counted() {
        let mut state = game("effects-move");
        let held = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let card = announced(&mut state, held);

        run(
            &mut state,
            counter(json_as(chosen())),
            Some(&card),
            RunOptions::default(),
        );

        let side = &state.players.p1;
        assert_eq!(side.turn_log.played_ids, Vec::<String>::new());
        assert_eq!(side.turn_log.cards_played, 0);
        assert_eq!(state.counters.played, 0);
    }

    #[test]
    fn r213_a_countered_play_leaves_the_turns_costs_as_they_were_so_the_next_cheap_card_is_still_gifted_programs_first()
     {
        let mut state = game("effects-move");
        put(
            &mut state,
            &gifted().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let earlier = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let held = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let card = announced(&mut state, held);
        // An earlier 3-cost play this turn; the announced 1-cost one has not reached step 4's log.
        let earlier_id = earlier.map(|c| c.id).unwrap_or_default();
        {
            let side = &mut state.players.p1;
            side.turn_log.played_ids.push(earlier_id.clone());
            side.turn_log.costs_paid = Some(vec![3]);
            side.turn_log.cards_played = 1;
        }
        state.counters.played = 1;

        run(
            &mut state,
            counter(json_as(chosen())),
            Some(&card),
            RunOptions::default(),
        );

        let side = &state.players.p1;
        assert_eq!(side.turn_log.played_ids, vec![earlier_id]);
        assert_eq!(side.turn_log.costs_paid, Some(vec![3]));
        assert!(gifted_makes_radiant(&state, PlayerId::P1, 1));
    }

    #[test]
    fn r448_with_no_target_it_counters_the_innermost_announce_still_live() {
        let mut state = game("effects-move");
        let held = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let outer = announced(&mut state, held);
        let held = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let inner = announced(&mut state, held);

        run(
            &mut state,
            counter(Default::default()),
            None,
            RunOptions::default(),
        );

        assert_eq!(ids(&state.players.p1.graveyard), vec![inner.id.clone()]);
        assert_eq!(live(&state, &outer.id).zone.z(), ZoneName::Resolving);
    }

    #[test]
    fn r448_a_card_that_is_not_announced_or_already_countered_is_not_countered_again() {
        let mut state = game("effects-move");
        let in_hand_card = first_in_hand(&mut state, &noisy().id, PlayerId::P1).expect("a hand card");
        let held = first_in_hand(&mut state, &noisy().id, PlayerId::P1);
        let card = announced(&mut state, held);

        assert!(
            run(
                &mut state,
                counter(json_as(chosen())),
                Some(&in_hand_card),
                RunOptions::default()
            )
            .is_empty()
        );
        assert_eq!(ids(&state.players.p1.hand), vec![in_hand_card.id.clone()]);
        run(
            &mut state,
            counter(json_as(chosen())),
            Some(&card),
            RunOptions::default(),
        );
        assert!(
            run(
                &mut state,
                counter(json_as(chosen())),
                Some(&card),
                RunOptions::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn r11_a_countered_unit_token_card_vanishes_and_reaches_no_graveyard() {
        let mut state = game("effects-move");
        let held = first_in_hand(&mut state, &hand_token().id, PlayerId::P1);
        let token = announced(&mut state, held);

        let events = run(
            &mut state,
            counter(json_as(chosen())),
            Some(&token),
            RunOptions::default(),
        );

        assert_eq!(state.players.p1.resolving.len(), 0);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(
            events
                .iter()
                .map(|event| serde_json::to_value(event).expect("an event serialises"))
                .collect::<Vec<_>>(),
            vec![json!({
                "type": "countered", "player": "p1", "instanceId": token.id, "defId": hand_token().id,
                "byInstanceId": null, "to": "gone"
            })]
        );
    }

    #[test]
    fn r12_a_countered_card_goes_to_its_owners_graveyard() {
        let mut state = game("effects-move");
        let held = first_in_hand(&mut state, "fx-4", PlayerId::P2);
        let theirs = announced(&mut state, held);

        run(
            &mut state,
            counter(json_as(chosen())),
            Some(&theirs),
            as_controller(PlayerId::P1),
        );

        assert_eq!(ids(&state.players.p2.graveyard), vec![theirs.id.clone()]);
        assert_eq!(state.players.p1.graveyard.len(), 0);
    }
}
