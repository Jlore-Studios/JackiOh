//! A card's play gate (`Script.play_allowed`, R1851000–R1851002, issue #1851): a card whose play
//! needs something on the board to act on, though it declares no target its play needs (R703), is
//! offered, accepted and explained by one check (`play_choices::play_allowed`), read off the face the
//! play resolves. A cast, an Echo repeat and a random cast never ask it (R70), a set Spell's Reveal
//! waits for it as for a needed pick (R783006), a fusion holds every ingredient's and a copier its
//! copied face's. The fixtures in `fixtures/play_gate.rs` stand in for Meditative #53 Prestige's
//! Radiant face.

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::testkit::*;

use super::fixtures::copied_text::{CT, register_copied_text};
use super::fixtures::harness::{act, in_hand, put, slot};
use super::fixtures::play_gate::{dummy, gated, gated_cloaked, gated_echo, plain_spell, register_play_gate};
use super::fixtures::prompt_harness::board;

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// R1851000's refusal, after the card's name.
const NOTHING_TO_ACT_ON: &str = "cannot be played without something to act on";

/// p1's main phase on turn 3 with 4 mana a side and the gate's cards registered; each side holds a
/// plain Unit, so no turn ends by itself.
fn playing(seed: &str) -> GameState {
    let mut state = board(seed);
    register_play_gate();
    in_hand(&mut state, &dummy().id, P1, 1);
    in_hand(&mut state, &dummy().id, P2, 1);
    state
}

fn hero_health(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero.health
}

fn offers(state: &GameState, id: &str) -> bool {
    legal_actions(state, P1)
        .iter()
        .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id))
}

fn play(state: &GameState, id: &str) -> ReduceResult {
    let body = play_choices::PlayAction {
        instance_id: id.to_string(),
        ..play_choices::PlayAction::default()
    }
    .into_body();
    reduce(state, &Action::new(body, P1, "gate"))
}

fn refusal(result: &ReduceResult) -> String {
    result.error.clone().unwrap_or_default()
}

/// Why p1's own view says this hand card cannot be played (`playReason`, R979000).
fn play_reason(state: &GameState, id: &str) -> Option<String> {
    let HandView::Cards(cards) = view_for(state, P1).you.hand else {
        panic!("the viewer's own hand is a list");
    };
    cards
        .into_iter()
        .find(|card| card.instance_id == id)
        .unwrap_or_else(|| panic!("{id} in the viewer's hand"))
        .play_reason
}

/// An enemy Unit in p2's units lane 1, which every gate here asks for.
fn enemy_unit(state: &mut GameState) -> CardInstance {
    put(state, &dummy().id, slot(P2, Row::Units, 1), json!({}))
}

#[test]
fn r1851000_a_gate_that_does_not_hold_is_neither_offered_nor_accepted_and_the_view_says_why() {
    let mut state = playing("r1851000-gate");
    let card = in_hand(&mut state, &gated().id, P1, 1).remove(0);
    // A friendly Unit is not what this gate asks for.
    put(&mut state, &dummy().id, slot(P1, Row::Units, 1), json!({}));

    assert!(play_choices::play_choice_combinations(&state, P1, &card, None).is_empty());
    assert!(!offers(&state, &card.id));
    let refused = play(&state, &card.id);
    assert!(
        refusal(&refused).contains(NOTHING_TO_ACT_ON),
        "{:?}",
        refused.error
    );
    assert_eq!(refused.state, state, "a refusal leaves the state untouched");
    assert_eq!(
        play_reason(&state, &card.id),
        Some(format!("gated (play gate) {NOTHING_TO_ACT_ON}")),
        "the view gives the refusal's own words"
    );

    // The gate holds.
    enemy_unit(&mut state);
    assert!(offers(&state, &card.id));
    assert_eq!(play_reason(&state, &card.id), None);
    let played = play(&state, &card.id);
    assert_eq!(played.error, None);
    assert_eq!(hero_health(&played.state, P2), hero_health(&state, P2) - 1);
}

#[test]
fn r1851001_r70_a_cast_never_asks_the_gate() {
    let mut state = playing("r1851001-cast");
    let card = in_hand(&mut state, &gated().id, P1, 1).remove(0);
    let before = hero_health(&state, P2);
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
        resolve::cast_card(&mut sink, &card, CastOptions::default());
    }
    assert_eq!(
        hero_health(&state, P2),
        before - 1,
        "the cast resolved with no enemy Unit"
    );
}

#[test]
fn r1851001_an_echo_repeat_never_asks_the_gate() {
    let mut state = playing("r1851001-echo");
    let card = in_hand(&mut state, &gated_echo().id, P1, 1).remove(0);
    let foe = enemy_unit(&mut state);
    assert!(offers(&state, &card.id));
    let played = play(&state, &card.id);
    assert_eq!(played.error, None);
    // The first resolution destroyed the only enemy Unit; the repeat resolved all the same.
    assert!(
        find_instance(&played.state, &foe.id).is_none_or(|card| !matches!(card.zone, Zone::Field { .. }))
    );
    assert_eq!(hero_health(&played.state, P2), hero_health(&state, P2) - 2);
}

#[test]
fn r1851001_r783006_a_set_spells_reveal_waits_for_its_gate() {
    let mut state = playing("r1851001-reveal");
    let card = in_hand(&mut state, &gated_cloaked().id, P1, 1).remove(0);
    // A set card declares nothing, so it may be set with its gate shut.
    state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": card.id,
            "zone": { "row": "backrow", "lane": 1 },
            "faceDown": "cloaked",
            "playerId": "p1",
        }),
    );
    let set = must(state.players.p1.backrow[0].clone(), "the set card");
    let reveal_offered = |state: &GameState| {
        legal_actions(state, P1).iter().any(|action| {
            matches!(action, ActionBody::Activate { instance_id, ability, .. }
                if *instance_id == set.id && ability.as_deref() == Some("reveal"))
        })
    };
    assert!(!reveal_offered(&state));

    // An enemy Unit opens it, and the Spell resolves as it reveals.
    enemy_unit(&mut state);
    assert!(reveal_offered(&state));
    let before = hero_health(&state, P2);
    state = act(
        &state,
        json!({ "type": "activate", "instanceId": set.id, "ability": "reveal", "playerId": "p1" }),
    );
    assert_eq!(hero_health(&state, P2), before - 1);
}

/// p1 crafts `def_ids` into hand (Fuse, R77): the state's fused card there.
fn craft(state: &mut GameState, def_ids: &[String]) -> CardInstance {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..Default::default()
            },
        );
        apply_effects(
            &[fuse_cards(json_as(
                json!({ "defIds": def_ids, "toHand": "self" }),
            ))],
            &mut ctx,
        );
    }
    state.rng_cursor = rng.cursor();
    must(
        state
            .players
            .p1
            .hand
            .iter()
            .find(|card| state.transient_defs.contains_key(&card.def_id))
            .cloned(),
        "the fused card in p1's hand",
    )
}

#[test]
fn r1851002_a_fusion_is_played_only_while_every_ingredients_gate_holds() {
    let mut state = playing("r1851002-fuse");
    let fused = craft(&mut state, &[plain_spell().id, gated().id]);
    assert!(
        !offers(&state, &fused.id),
        "the gated ingredient's gate does not hold"
    );
    assert!(refusal(&play(&state, &fused.id)).contains(NOTHING_TO_ACT_ON));

    enemy_unit(&mut state);
    assert!(offers(&state, &fused.id));
    let played = play(&state, &fused.id);
    assert_eq!(played.error, None);
    assert_eq!(
        hero_health(&played.state, P2),
        hero_health(&state, P2) - 2,
        "both ingredients resolve"
    );
}

#[test]
fn r1851002_r399_a_copier_is_gated_by_the_copied_faces_gate() {
    let mut state = playing("r1851002-copy");
    register_copied_text();
    let echo = in_hand(&mut state, &CT.echo.id, P1, 1).remove(0);
    state.last_spell = Some(PlayRecord {
        def_id: gated().id,
        radiant: false,
    });
    assert!(
        !offers(&state, &echo.id),
        "Echo has the gated Spell's text, and its gate"
    );
    assert!(refusal(&play(&state, &echo.id)).contains(NOTHING_TO_ACT_ON));

    enemy_unit(&mut state);
    assert!(offers(&state, &echo.id));
    let played = play(&state, &echo.id);
    assert_eq!(played.error, None);
    assert_eq!(hero_health(&played.state, P2), hero_health(&state, P2) - 1);
}
