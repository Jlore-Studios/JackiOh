//! R4590000–R4590003: the Jailbreak verbs IA #134, #130/#142, #135 and #138 add to the engine.
//!
//! A revealed Unit's `revealed` hook, a count of mana crystals stolen, a control swap between two
//! Units, and declared Spell targets drawn at random for one player only.

use jackioh_engine::alt_play::CLOAKED_REVEAL;
use jackioh_engine::effects::{
    RandomTargetsNextSpellsArgs, StealSomeManaCrystalsArgs, SwapControlArgs, random_targets_next_spells,
    steal_some_mana_crystals, swap_control,
};
use jackioh_engine::testkit::*;

use super::fixtures::harness::{act, by, in_hand, put, run_with_options, sink_for, slot};
use super::fixtures::keywords_783::{
    board_783 as jailbreak_board, cloaked_bolt, grunt, random_targets_aura, revealed_drawer, sniper,
};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

/// The revealed-drawer's fresh id in `state`'s backrow lane, set face-down by its Cloaked.
fn set_drawer(state: &GameState, id: &str) -> GameState {
    act(
        state,
        json!({
            "type": "play",
            "instanceId": id,
            "zone": { "row": "backrow", "lane": 1 },
            "faceDown": "cloaked",
            "playerId": "p1",
        }),
    )
}

fn reveal(state: &GameState, id: &str) -> GameState {
    act(
        state,
        json!({ "type": "activate", "instanceId": id, "ability": CLOAKED_REVEAL, "playerId": "p1" }),
    )
}

#[test]
fn r4590000_a_revealed_hook_runs_at_the_reveal_and_never_on_a_face_up_play() {
    let mut state = jailbreak_board("jb-revealed");
    let drawer = in_hand(&mut state, &revealed_drawer().id, P1, 1)
        .pop()
        .expect("drawer in hand");
    let before = state.players.p1.hand.len();
    let state = set_drawer(&state, &drawer.id);
    assert_eq!(
        state.players.p1.hand.len(),
        before - 1,
        "the set took the card out of the hand"
    );
    let set = must(
        state.players.p1.backrow[0].clone(),
        "the set card in its backrow zone",
    );
    let state = reveal(&state, &set.id);
    assert_eq!(state.players.p1.hand.len(), before, "the reveal drew one card");
    let unit = must(
        find_instance(&state, &set.id).cloned(),
        "the revealed Unit on the field",
    );
    assert_eq!(unit.def_id, revealed_drawer().id);
    assert!(matches!(unit.zone, Zone::Field { .. }));
}

#[test]
fn r4590000_a_face_up_play_runs_no_revealed_hook() {
    let mut state = jailbreak_board("jb-revealed-face-up");
    let drawer = in_hand(&mut state, &revealed_drawer().id, P1, 1)
        .pop()
        .expect("drawer in hand");
    let before = state.players.p1.hand.len();
    let state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": drawer.id,
            "zone": { "row": "units", "lane": 1 },
            "playerId": "p1",
        }),
    );
    assert_eq!(state.players.p1.hand.len(), before - 1, "the play drew nothing");
    assert!(state.pending.is_none(), "no question opened");
}

fn random_spell_setup(seed: &str) -> (GameState, CardInstance, CardInstance, CardInstance) {
    let mut state = jailbreak_board(seed);
    let bolt = in_hand(&mut state, &cloaked_bolt().id, P1, 1)
        .pop()
        .expect("a bolt in hand");
    let first = put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
    let second = put(&mut state, &grunt().id, slot(P2, Row::Units, 2), json!({}));
    (state, bolt, first, second)
}

fn random_targets_live(state: &GameState, player: PlayerId) -> usize {
    state.players[player]
        .mods
        .iter()
        .filter(|modifier| {
            matches!(modifier.kind, ModifierKind::RandomTargetsNextSpell) && modifier_is_live(state, modifier)
        })
        .count()
}

/// The ids of the field Units of both sides that carry damage.
fn damaged(state: &GameState) -> Vec<String> {
    [P1, P2]
        .into_iter()
        .flat_map(|player| zones::units_on_field_of(state, player))
        .filter(|unit| unit.damage > 0)
        .map(|unit| unit.id.clone())
        .collect()
}

/// The ids of `player`'s field Units that carry damage.
fn damaged_of(state: &GameState, player: PlayerId) -> Vec<String> {
    zones::units_on_field_of(state, player)
        .into_iter()
        .filter(|unit| unit.damage > 0)
        .map(|unit| unit.id.clone())
        .collect()
}

#[test]
fn r4590001_a_next_spell_modifier_draws_one_spells_targets_and_is_spent() {
    let (mut state, bolt, first, second) = random_spell_setup("jb-mod-draw");
    run_with_options(
        &mut state,
        random_targets_next_spells(RandomTargetsNextSpellsArgs {
            count: 1,
            player: None,
        }),
        by(P1),
    );
    assert_eq!(random_targets_live(&state, P1), 1);
    let plays: Vec<ActionBody> = plays_in(&state, P1, &bolt.id)
        .into_iter()
        .filter(|play| matches!(play, ActionBody::Play { face_down: None, .. }))
        .collect();
    assert_eq!(plays.len(), 1, "one face-up play, not one per target: {plays:?}");
    assert!(
        matches!(&plays[0], ActionBody::Play { targets: None, .. }),
        "no targets while the modifier holds: {:?}",
        plays[0]
    );
    let state = act(
        &state,
        json!({ "type": "play", "instanceId": bolt.id, "playerId": "p1" }),
    );
    let hit = damaged_of(&state, P2);
    assert_eq!(hit.len(), 1, "the drawn target took the hit: {hit:?}");
    assert!(
        [first.id, second.id].contains(hit.first().expect("one hit")),
        "the hit landed on a legal target",
    );
    assert_eq!(random_targets_live(&state, P1), 0, "the Spell spent the modifier");
}

#[test]
fn r4590001_the_aura_draws_only_the_opponents_spells_and_lists_them_untargeted() {
    let mut state = jailbreak_board("jb-aura-draw");
    put(
        &mut state,
        &random_targets_aura().id,
        slot(P1, Row::Units, 1),
        json!({}),
    );
    put(&mut state, &grunt().id, slot(P2, Row::Units, 1), json!({}));
    let enemy_bolt = in_hand(&mut state, &cloaked_bolt().id, P2, 1)
        .pop()
        .expect("p2's bolt");
    let own_bolt = in_hand(&mut state, &cloaked_bolt().id, P1, 1)
        .pop()
        .expect("p1's bolt");
    let own_plays: Vec<ActionBody> = plays_in(&state, P1, &own_bolt.id)
        .into_iter()
        .filter(|play| matches!(play, ActionBody::Play { face_down: None, .. }))
        .collect();
    assert_eq!(
        own_plays.len(),
        2,
        "your own Spell still lists each target: {own_plays:?}"
    );
    assert!(
        own_plays
            .iter()
            .all(|play| matches!(play, ActionBody::Play { targets: Some(_), .. })),
        "your own Spell keeps its targets",
    );
    state.active = P2;
    let enemy_plays: Vec<ActionBody> = plays_in(&state, P2, &enemy_bolt.id)
        .into_iter()
        .filter(|play| matches!(play, ActionBody::Play { face_down: None, .. }))
        .collect();
    assert_eq!(
        enemy_plays.len(),
        1,
        "one face-up play, not one per target: {enemy_plays:?}"
    );
    assert!(
        matches!(&enemy_plays[0], ActionBody::Play { targets: None, .. }),
        "the opponent's Spell lists no targets: {:?}",
        enemy_plays[0]
    );
    let state = act(
        &state,
        json!({ "type": "play", "instanceId": enemy_bolt.id, "playerId": "p2" }),
    );
    assert_eq!(damaged(&state).len(), 1, "the drawn target took the hit");
}

#[test]
fn r4590001_a_unit_play_and_a_face_down_set_keep_their_targets() {
    let (mut state, _bolt, first, second) = random_spell_setup("jb-mod-unit");
    let sniper_card = in_hand(&mut state, &sniper().id, P1, 1)
        .pop()
        .expect("a sniper in hand");
    let bolt = in_hand(&mut state, &cloaked_bolt().id, P1, 1)
        .pop()
        .expect("a bolt in hand");
    run_with_options(
        &mut state,
        random_targets_next_spells(RandomTargetsNextSpellsArgs {
            count: 1,
            player: None,
        }),
        by(P1),
    );
    let sniper_plays = plays_in(&state, P1, &sniper_card.id);
    assert!(
        !sniper_plays.is_empty()
            && sniper_plays
                .iter()
                .all(|play| matches!(play, ActionBody::Play { targets: Some(_), .. })),
        "the Unit still lists each target: {sniper_plays:?}"
    );
    // The Unit's chosen target is honored, and the modifier waits for a Spell.
    let state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": sniper_card.id,
            "zone": { "row": "units", "lane": 1 },
            "targets": [{ "pick": "instance", "instanceId": first.id }],
            "playerId": "p1",
        }),
    );
    assert_eq!(
        damaged_of(&state, P2),
        vec![first.id.clone()],
        "the chosen Unit took the 2"
    );
    assert_eq!(random_targets_live(&state, P1), 1, "a Unit play spends nothing");
    // A face-down set draws nothing and spends nothing.
    let state = act(
        &state,
        json!({
            "type": "play",
            "instanceId": bolt.id,
            "zone": { "row": "backrow", "lane": 2 },
            "faceDown": "cloaked",
            "playerId": "p1",
        }),
    );
    assert_eq!(
        random_targets_live(&state, P1),
        1,
        "a face-down set spends nothing"
    );
    let other = must(
        find_instance(&state, &second.id).cloned(),
        "the other grunt stands",
    );
    assert!(matches!(other.zone, Zone::Field { .. }), "untouched on the field");
}

fn crystal_setup(seed: &str, victim_turns: i32, thief_turns: i32) -> GameState {
    let mut state = jailbreak_board(seed);
    for (player, turns) in [(P1, thief_turns), (P2, victim_turns)] {
        state.players[player].turns_started = turns;
        state.players[player].mana_crystals = None;
        state.players[player].stolen_mana_crystals = None;
        state.players[player].mana.perm_mod = 0;
    }
    state.players[P2].mana.max = mana::max_mana_for(&state.players[P2]);
    state.players[P2].mana.current = state.players[P2].mana.max;
    state.players[P1].mana.max = mana::max_mana_for(&state.players[P1]);
    state.players[P1].mana.current = state.players[P1].mana.max;
    state
}

fn steal_some(state: &mut GameState, count: i32) {
    let mut sink = sink_for(state);
    let mut ctx = make_context(&mut sink, None, by(P1));
    apply_effects(
        &[steal_some_mana_crystals(StealSomeManaCrystalsArgs { count })],
        &mut ctx,
    );
}

#[test]
fn r4590002_one_crystal_from_turn_six_leaves_three_and_returns_next_turn() {
    let mut state = crystal_setup("jb-mana-one", 6, 2);
    assert_eq!(mana::mana_crystal_count(&state.players[P2]), 6);
    steal_some(&mut state, 1);
    assert_eq!(
        mana::mana_crystal_count(&state.players[P2]),
        3,
        "held four, less the one taken"
    );
    assert_eq!(state.players[P2].mana.max, 3);
    assert_eq!(state.players[P2].mana.current, 3);
    assert_eq!(
        mana::stolen_mana_crystals_of(&state.players[P1]),
        1,
        "the thief holds it"
    );
    // The victim's own crystals come back one a turn.
    state.players[P2].turns_started = 7;
    assert_eq!(mana::mana_crystal_count(&state.players[P2]), 4);
}

#[test]
fn r4590002_the_thief_holds_it_past_the_cap() {
    let mut state = crystal_setup("jb-mana-cap", 1, 4);
    assert_eq!(state.players[P1].mana.max, 4, "the thief starts at the cap");
    steal_some(&mut state, 1);
    assert_eq!(state.players[P2].mana.max, 0, "the victim lost their crystal");
    assert_eq!(state.players[P2].mana.current, 0);
    assert_eq!(mana::stolen_mana_crystals_of(&state.players[P1]), 1);
    assert_eq!(state.players[P1].mana.max, 5, "past the cap");
    assert_eq!(state.players[P1].mana.current, 5);
}

fn swap_setup(seed: &str) -> (GameState, Vec<String>, Vec<String>) {
    let mut state = jailbreak_board(seed);
    let mut p1 = Vec::new();
    let mut p2 = Vec::new();
    for lane in 1..=5 {
        p1.push(put(&mut state, &grunt().id, slot(P1, Row::Units, lane), json!({})).id);
        p2.push(put(&mut state, &grunt().id, slot(P2, Row::Units, lane), json!({})).id);
    }
    (state, p1, p2)
}

fn swap(state: &mut GameState, mine_id: &str, theirs_id: &str) {
    let mine = must(find_instance(state, mine_id).cloned(), "the swapping Unit");
    let mut sink = sink_for(state);
    let mut ctx = make_context(&mut sink, Some(&mine), by(P1));
    apply_effects(
        &[swap_control(SwapControlArgs {
            instance_id: theirs_id.to_string(),
        })],
        &mut ctx,
    );
}

#[test]
fn r4590003_swap_control_with_both_boards_full_moves_both_and_destroys_nothing() {
    let (mut state, p1, p2) = swap_setup("jb-swap-full");
    let turn = state.turn;
    swap(&mut state, &p1[0], &p2[0]);
    let standing: Vec<CardInstance> = [P1, P2]
        .into_iter()
        .flat_map(|player| zones::units_on_field_of(&state, player))
        .cloned()
        .collect();
    assert_eq!(standing.len(), 10, "nothing was destroyed");
    let mine = must(find_instance(&state, &p1[0]).cloned(), "the swapped-away Unit");
    let theirs = must(find_instance(&state, &p2[0]).cloned(), "the swapped-in Unit");
    assert_eq!(mine.controller, P2, "each card changed sides");
    assert_eq!(theirs.controller, P1);
    assert_eq!(
        mine.zone,
        Zone::Field {
            player: P2,
            row: Row::Units,
            lane: 1
        }
    );
    assert_eq!(
        theirs.zone,
        Zone::Field {
            player: P1,
            row: Row::Units,
            lane: 1
        }
    );
    assert_eq!(mine.summoned_turn, Some(turn), "both are summoning sick");
    assert_eq!(theirs.summoned_turn, Some(turn));
    assert!(
        standing
            .iter()
            .all(|unit| matches!(unit.zone, Zone::Field { .. })),
        "nothing left the field",
    );
}

#[test]
fn r4590003_a_dormant_target_swaps_nothing() {
    let (mut state, p1, p2) = swap_setup("jb-swap-dormant");
    find_instance_mut(&mut state, &p2[0]).expect("the target").slumber = Some(Slumber::default());
    swap(&mut state, &p1[0], &p2[0]);
    let mine = must(find_instance(&state, &p1[0]).cloned(), "the swapping Unit");
    let theirs = must(find_instance(&state, &p2[0]).cloned(), "the dormant Unit");
    assert_eq!(mine.controller, P1, "nothing moved");
    assert_eq!(theirs.controller, P2);
    assert_eq!(
        mine.zone,
        Zone::Field {
            player: P1,
            row: Row::Units,
            lane: 1
        }
    );
    assert_eq!(
        theirs.zone,
        Zone::Field {
            player: P2,
            row: Row::Units,
            lane: 1
        }
    );
}
