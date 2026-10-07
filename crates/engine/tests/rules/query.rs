//! `src/query.ts`: the read-only board surface a card script asks its questions with
//! (BUILD M3-T1's "scripts never touch state", SPEC §10.9's "a hook may read state to compute an
//! effect's arguments; it never writes").
//!
//! Each accessor is checked against a fact the ENGINE put in the state — a draw, a damage instance,
//! a move to the graveyard, a play through `reduce` — rather than against a field the test itself
//! assigned, so a test here fails if the accessor stops answering what the engine did. The copy
//! tests are the other half of the surface's promise: a card file holds no reference it can write
//! the game through, which is the property the acceptance grep is really protecting.
//!
//! Port of `packages/engine/test/query.test.ts`. TS's `CardInstance | string` arguments are
//! `CardOrId::Card(&card)` / `CardOrId::Id(id)`, and `null` `CardOrId::Nothing`. TS held the live card
//! `put` and `inHand` returned; here a card is re-read from the state (`live`).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, new_game, put, set_library, sink_for, slot};
use crate::rules::fixtures::scripts::stockpile;

/// A state in the main phase with nothing dealt, so every pile below is exactly what a test built.
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// TS's module `let nonce`.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn input(body: Value) -> ActionInput {
    json_as(body)
}

fn act(state: &GameState, body: ActionInput) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let result = reduce(state, &body.with_nonce(format!("q{nonce}")));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result.state
}

/// Past the mulligans, in the main phase of turn 1, so `play` is a legal action.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    let keep: Vec<String> = state.players.p1.hand.iter().map(|c| c.id.clone()).collect();
    state = act(
        &state,
        input(json!({ "type": "mulligan", "keep": keep, "playerId": "p1" })),
    );
    let keep: Vec<String> = state.players.p2.hand.iter().map(|c| c.id.clone()).collect();
    state = act(
        &state,
        input(json!({ "type": "mulligan", "keep": keep, "playerId": "p2" })),
    );
    // Turn 1 refreshes 1 mana (§2.3) and these tests play two 1-cost fixtures; §2.3 lets current mana
    // sit above max, which is the same thing #6 Mana Well does.
    state.players.p1.mana.current = 4;
    state
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// The card as it stands in `state` now (TS held the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {}", card.id))
}

fn play_into(state: &GameState, card: &CardInstance, lane: i32) -> GameState {
    act(
        state,
        input(
            json!({ "type": "play", "instanceId": card.id, "zone": { "row": "units", "lane": lane }, "playerId": "p1" }),
        ),
    )
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn def_ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn top_of(state: &GameState, player: PlayerId, lane: usize) -> Option<CardInstance> {
    state.players[player].units[lane]
        .as_ref()
        .and_then(|pile| pile.first())
        .cloned()
}

/// the card-facing read surface (BUILD M3-T1, SPEC §10.9)
mod the_card_facing_read_surface_build_m3_t1_spec_s10_9 {
    use super::*;

    // -------------------------------------------------------------------------
    // heroOf — #68 Twisted Sorcerer's threshold, #70 Spiteful Stab's missing health.
    // -------------------------------------------------------------------------

    #[test]
    fn hero_of_reads_the_health_the_engine_just_took_off_for_the_player_named_and_no_other() {
        let mut state = board("hero-health");
        assert_eq!(hero_of(&state, P1).health, HERO_HEALTH);

        lose_health(&mut sink_for(&mut state), P1, 7);

        assert_eq!(hero_of(&state, P1).health, HERO_HEALTH - 7);
        // The accessor is keyed on the player it is given: p2 took nothing.
        assert_eq!(hero_of(&state, P2).health, HERO_HEALTH);
    }

    #[test]
    fn hero_of_reads_the_stored_armor_field_of_s4_4_step_2() {
        let mut state = board("hero-armor");
        assert_eq!(hero_of(&state, P1).armor, 0);

        // Only `createPlayerState` writes this field today (#84 is blocked on an engine verb), so the
        // test plays the part of whatever eventually will, and the accessor has to see it.
        state.players.p1.hero.armor = 3;

        assert_eq!(hero_of(&state, P1).armor, 3);
        assert_eq!(hero_of(&state, P2).armor, 0);
    }

    #[test]
    // The writes to `hero` are the test: they land on a copy nothing reads again.
    #[allow(unused_variables, unused_assignments)]
    fn hero_of_hands_back_a_copy_so_a_script_cannot_write_a_hero_s_health_through_it() {
        let state = board("hero-copy");
        let mut hero = hero_of(&state, P1);

        hero.health = 1;
        hero.armor = 99;

        assert_eq!(
            state.players.p1.hero,
            HeroState {
                health: HERO_HEALTH,
                armor: 0
            }
        );
        assert_eq!(hero_of(&state, P1).health, HERO_HEALTH);
    }

    // -------------------------------------------------------------------------
    // zoneCards / zoneCount — #30 Archivist, #51 Private Tutor, #71 Intern Stimmy, #83 Transmogulate.
    // -------------------------------------------------------------------------

    #[test]
    fn zone_cards_is_one_player_s_pile_in_zone_order_the_library_top_first() {
        let mut state = board("zone-order");
        set_library(&mut state, P1, &["fx-1", "fx-2", "fx-3"]);

        assert_eq!(
            def_ids(&zone_cards(&state, P1, OffFieldZone::Library)),
            vec!["fx-1", "fx-2", "fx-3"]
        );

        // "Top" is whatever `drawOne` takes, which is what #30's top-down scan and #51's reveal mean.
        let top = must(
            zone_cards(&state, P1, OffFieldZone::Library).first().cloned(),
            "a library card",
        );
        draw_one(&mut sink_for(&mut state), P1, None);

        assert_eq!(
            ids(&zone_cards(&state, P1, OffFieldZone::Hand)),
            vec![top.id.clone()]
        );
        assert_eq!(
            def_ids(&zone_cards(&state, P1, OffFieldZone::Library)),
            vec!["fx-2", "fx-3"]
        );
    }

    #[test]
    fn zone_count_follows_the_engine_moving_a_card_from_one_pile_to_another() {
        let mut state = board("zone-count");
        set_library(&mut state, P1, &["fx-1", "fx-2", "fx-3"]);
        in_hand(&mut state, "fx-4", P1, 2);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Library), 3);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Hand), 2);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Graveyard), 0);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Exile), 0);

        draw_one(&mut sink_for(&mut state), P1, None);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Library), 2);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Hand), 3);

        // All four zone names answer, which is what #83 walks and what #70's exile term counts.
        let unit = put(&mut state, "fx-5", slot(P1, Row::Units, 1), json!({}));
        let mut now = live(&state, &unit);
        move_to_zone(&mut state, &mut now, OffFieldZone::Graveyard, Default::default());
        assert_eq!(zone_count(&state, P1, OffFieldZone::Graveyard), 1);
        assert_eq!(
            ids(&zone_cards(&state, P1, OffFieldZone::Graveyard)),
            vec![unit.id.clone()]
        );

        let mut now = live(&state, &unit);
        move_to_zone(&mut state, &mut now, OffFieldZone::Exile, Default::default());
        assert_eq!(zone_count(&state, P1, OffFieldZone::Graveyard), 0);
        assert_eq!(zone_count(&state, P1, OffFieldZone::Exile), 1);
    }

    #[test]
    fn zone_count_reads_the_player_it_is_given_not_the_active_one_c71_s_two_libraries() {
        let mut state = board("zone-sides");
        set_library(&mut state, P1, &["fx-1", "fx-2", "fx-3"]);
        set_library(&mut state, P2, &["fx-4"]);

        assert_eq!(zone_count(&state, P1, OffFieldZone::Library), 3);
        assert_eq!(zone_count(&state, P2, OffFieldZone::Library), 1);
        assert_eq!(
            def_ids(&zone_cards(&state, P2, OffFieldZone::Library)),
            vec!["fx-4"]
        );
    }

    #[test]
    fn zone_cards_hands_back_a_copy_so_a_script_cannot_push_into_or_empty_a_zone_through_it() {
        let mut state = board("zone-copy");
        let library = set_library(&mut state, P1, &["fx-1", "fx-2"]);
        let stray = must(
            in_hand(&mut state, "fx-3", P1, 1).into_iter().next(),
            "a hand card",
        );

        let mut cards = zone_cards(&state, P1, OffFieldZone::Library);
        cards.push(stray.clone());
        cards.remove(0);

        assert_eq!(zone_count(&state, P1, OffFieldZone::Library), 2);
        assert_eq!(ids(&zone_cards(&state, P1, OffFieldZone::Library)), ids(&library));
        assert_eq!(zone_count(&state, P1, OffFieldZone::Hand), 1);
    }

    // -------------------------------------------------------------------------
    // The turn log — #23, #24, #31's one-shot return; #38's Combo X; #39's replay of the turn.
    // -------------------------------------------------------------------------

    #[test]
    fn cards_played_this_turn_and_played_ids_this_turn_follow_the_plays_in_play_order() {
        let mut state = playing("played-log");
        assert_eq!(cards_played_this_turn(&state, P1), 0);
        assert_eq!(played_ids_this_turn(&state, P1), Vec::<String>::new());

        let first = must(
            in_hand(&mut state, "fx-1", P1, 1).into_iter().next(),
            "a first card",
        );
        let second = must(
            in_hand(&mut state, "fx-2", P1, 1).into_iter().next(),
            "a second card",
        );
        let unplayed = must(
            in_hand(&mut state, "fx-3", P1, 1).into_iter().next(),
            "a third card",
        );

        state = play_into(&state, &first, 1);
        // §10.5 step 4 counts the card as it is played, which is the fact #38 subtracts 1 from.
        assert_eq!(cards_played_this_turn(&state, P1), 1);

        state = play_into(&state, &second, 2);
        assert_eq!(cards_played_this_turn(&state, P1), 2);
        assert_eq!(
            played_ids_this_turn(&state, P1),
            vec![first.id.clone(), second.id.clone()]
        );

        assert!(was_played_this_turn(&state, P1, CardOrId::Card(&first)));
        assert!(was_played_this_turn(&state, P1, CardOrId::Id(&second.id)));
        assert!(!was_played_this_turn(&state, P1, CardOrId::Card(&unplayed)));

        // The log is per player: p1's plays are not in p2's log, and p2 has played nothing.
        assert!(!was_played_this_turn(&state, P2, CardOrId::Card(&first)));
        assert_eq!(cards_played_this_turn(&state, P2), 0);
        assert_eq!(played_ids_this_turn(&state, P2), Vec::<String>::new());
    }

    #[test]
    fn r68_start_turn_clears_the_log_so_a_played_this_turn_gate_fires_once_s5_1() {
        let mut state = playing("played-cleared");
        let card = must(in_hand(&mut state, "fx-1", P1, 1).into_iter().next(), "a card");
        state = play_into(&state, &card, 1);
        assert!(was_played_this_turn(&state, P1, CardOrId::Card(&card)));

        start_turn(&mut sink_for(&mut state), P1);

        assert_eq!(cards_played_this_turn(&state, P1), 0);
        assert_eq!(played_ids_this_turn(&state, P1), Vec::<String>::new());
        assert!(!was_played_this_turn(&state, P1, CardOrId::Card(&card)));
    }

    // -------------------------------------------------------------------------
    // playedEarlier — §6.2's Combo X "at play time" (#10, and §10.5 step 5's #38 and #78).
    // -------------------------------------------------------------------------

    #[test]
    fn played_earlier_counts_a_played_card_s_own_place_in_the_log_and_every_play_so_far_for_a_card_in_hand() {
        let mut state = playing("played-earlier");
        let first = must(
            in_hand(&mut state, "fx-1", P1, 1).into_iter().next(),
            "a first card",
        );
        let second = must(
            in_hand(&mut state, "fx-2", P1, 1).into_iter().next(),
            "a second card",
        );
        let held = must(
            in_hand(&mut state, "fx-3", P1, 1).into_iter().next(),
            "a card kept in hand",
        );

        // Nothing played yet: a card in hand would be the first play.
        assert_eq!(
            played_earlier(&state, P1, CardOrId::Card(&live(&state, &held))),
            0
        );

        state = play_into(&state, &first, 1);
        state = play_into(&state, &second, 2);

        // Its place in the log, by instance or by id: nothing before the first play, one before the second.
        assert_eq!(played_earlier(&state, P1, CardOrId::Id(&first.id)), 0);
        let on_field = must(top_of(&state, P1, 1), "the second card on the field");
        assert_eq!(played_earlier(&state, P1, CardOrId::Card(&on_field)), 1);
        // A card still in hand has not been played: both plays so far are earlier than its would be.
        assert_eq!(played_earlier(&state, P1, CardOrId::Id(&held.id)), 2);
        // An id the log does not hold is read the same way.
        assert_eq!(played_earlier(&state, P1, CardOrId::Id("no-such-card")), 2);
        // The log is per player: p1's plays are not earlier than anything of p2's.
        assert_eq!(played_earlier(&state, P2, CardOrId::Id(&first.id)), 0);
    }

    #[test]
    fn played_earlier_counts_a_card_played_twice_this_turn_from_its_latest_play() {
        let mut state = playing("played-earlier-twice");
        let twice = must(
            in_hand(&mut state, "fx-1", P1, 1).into_iter().next(),
            "the card played twice",
        );
        let other = must(
            in_hand(&mut state, "fx-2", P1, 1).into_iter().next(),
            "another card",
        );

        state = play_into(&state, &twice, 1);
        state = play_into(&state, &other, 2);
        // Back to the hand (a bounce), and played again: the log holds it twice.
        let mut on_field = must(top_of(&state, P1, 0), "the first card on the field");
        move_to_zone(&mut state, &mut on_field, OffFieldZone::Hand, Default::default());
        assert_eq!(played_earlier(&state, P1, CardOrId::Id(&twice.id)), 2);
        state = play_into(&state, &twice, 3);

        assert_eq!(
            played_ids_this_turn(&state, P1),
            vec![twice.id.clone(), other.id.clone(), twice.id.clone()]
        );
        assert_eq!(played_earlier(&state, P1, CardOrId::Id(&twice.id)), 2);
    }

    #[test]
    fn r70_played_earlier_does_not_count_a_card_cast_while_the_play_resolved_which_is_played_after_it() {
        let mut state = playing("played-earlier-cast");
        // Stockpile draws 2, and the top of p1's library is Hinder, which casts on draw (§2.4).
        set_library(&mut state, P1, &["fx-hinder", "fx-5", "fx-6"]);
        let stockpile = must(
            in_hand(&mut state, "fx-stockpile", P1, 1).into_iter().next(),
            "Stockpile",
        );

        state = act(
            &state,
            input(json!({ "type": "play", "instanceId": stockpile.id, "playerId": "p1" })),
        );

        // The cast counts as a play (R70), logged after the play whose draw made it.
        let log = played_ids_this_turn(&state, P1);
        assert_eq!(log.first(), Some(&stockpile.id));
        assert_eq!(log.len(), 2);
        assert_eq!(cards_played_this_turn(&state, P1), 2);
        // So the play that cast it still has nothing before it, and the cast has the play before it.
        assert_eq!(played_earlier(&state, P1, CardOrId::Id(&stockpile.id)), 0);
        let cast = must(log.get(1).cloned(), "the cast");
        assert_eq!(played_earlier(&state, P1, CardOrId::Id(&cast)), 1);
    }

    #[test]
    fn r127_played_earlier_with_no_card_takes_the_latest_play_as_the_running_script_s_own() {
        let mut state = playing("played-earlier-null");
        assert_eq!(played_earlier(&state, P1, CardOrId::Nothing), 0);
        let first = must(
            in_hand(&mut state, "fx-1", P1, 1).into_iter().next(),
            "a first card",
        );
        let second = must(
            in_hand(&mut state, "fx-2", P1, 1).into_iter().next(),
            "a second card",
        );
        state = play_into(&state, &first, 1);
        assert_eq!(played_earlier(&state, P1, CardOrId::Nothing), 0);
        state = play_into(&state, &second, 2);
        assert_eq!(played_earlier(&state, P1, CardOrId::Nothing), 1);
    }

    #[test]
    fn played_ids_this_turn_hands_back_a_copy_so_a_script_cannot_forge_a_play() {
        let mut state = playing("played-copy");
        let card = must(in_hand(&mut state, "fx-1", P1, 1).into_iter().next(), "a card");
        let other = must(
            in_hand(&mut state, "fx-2", P1, 1).into_iter().next(),
            "another card",
        );
        state = play_into(&state, &card, 1);

        let mut ids = played_ids_this_turn(&state, P1);
        ids.push(other.id.clone());
        ids.clear();

        assert_eq!(played_ids_this_turn(&state, P1), vec![card.id.clone()]);
        assert!(!was_played_this_turn(&state, P1, CardOrId::Card(&other)));
        assert_eq!(cards_played_this_turn(&state, P1), 1);
    }
}

/// R361 killerOf: the Unit that destroyed a card, as its Death hook reads it (R42)
mod r361_r42_killer_of_the_unit_that_destroyed_a_card_as_its_death_hook_reads_it {
    use super::*;

    /// The victim as its Death hook would read it: the snapshot §4.5 takes before R78 resets it.
    fn snapshot_of(state: &GameState, card: &CardInstance) -> CardInstance {
        live(state, card)
    }

    fn hit(state: &mut GameState, source: &CardInstance, target: &CardInstance, amount: i32) {
        let source = live(state, source);
        let instance = live(state, target);
        deal_damage(
            &mut sink_for(state),
            DamageArgs {
                source: Some(source),
                target: DamageTarget::Unit { instance },
                amount,
                flags: None,
            },
        );
    }

    #[test]
    fn r361_names_the_unit_whose_hit_was_lethal_while_that_unit_stands_on_the_field() {
        let mut state = board("killer-unit");
        let killer = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let victim = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        hit(&mut state, &killer, &victim, 3);
        let dying = snapshot_of(&state, &victim);
        state_check(&mut sink_for(&mut state));
        assert_eq!(live(&state, &victim).zone.z(), ZoneName::Graveyard);
        assert_eq!(
            killer_of(&state, Some(&dying)).map(|card| card.id.clone()),
            Some(killer.id.clone())
        );
    }

    #[test]
    fn r361_names_nobody_for_a_hit_that_was_not_lethal_a_spell_s_hit_or_no_card_at_all() {
        let mut state = board("killer-none");
        let striker = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let survivor = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        hit(&mut state, &striker, &survivor, 1);
        assert!(killer_of(&state, Some(&snapshot_of(&state, &survivor))).is_none());

        let spell = new_instance(&mut state, &stockpile().id, P2, Zone::Resolving { player: P2 });
        state.players.p2.resolving.push(spell.clone());
        let burnt = put(&mut state, &plain.id, slot(P1, Row::Units, 3), json!({}));
        hit(&mut state, &spell, &burnt, 3);
        assert!(killer_of(&state, Some(&snapshot_of(&state, &burnt))).is_none());

        assert!(killer_of(&state, None).is_none());
    }

    #[test]
    fn r361_r13_r78_names_nobody_once_the_killer_has_left_the_field_or_lies_dormant_under_a_stack() {
        let mut state = board("killer-gone");
        let killer = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let victim = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        hit(&mut state, &killer, &victim, 3);
        let dying = snapshot_of(&state, &victim);

        // Buried under a Stack pile, it is no longer acting on the field.
        let mut top = new_instance(&mut state, &plain.id, P2, Zone::Hand { player: P2 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(P2, Row::Units, 2),
            json_as(json!({ "stack": true }))
        ));
        assert!(killer_of(&state, Some(&dying)).is_none());

        // And in a graveyard it is gone for good.
        let mut now = live(&state, &killer);
        move_to_zone(&mut state, &mut now, OffFieldZone::Graveyard, Default::default());
        assert!(killer_of(&state, Some(&dying)).is_none());
    }
}

/// the Classic #1–#45 reads: max mana (C #36 Burn)
mod the_classic_c1_c45_reads_max_mana_c_c36_burn {
    use super::*;

    #[test]
    fn max_mana_of_is_s2_3_s_max_mana_which_temporary_mana_above_it_does_not_move() {
        let mut state = playing("max-mana");
        assert_eq!(max_mana_of(&state, P1), state.players.p1.mana.max);
        state.players.p1.mana.current = 9;
        assert_eq!(max_mana_of(&state, P1), 1);
        assert_eq!(max_mana_of(&state, P2), 0);
    }
}
