//! Port of `packages/engine/test/zones.test.ts`: adjacency, the rotation rings, Locks and free
//! zones, Stack piles, tokens leaving a zone, "fill your board" and the slot lists (BUILD M1-T4).

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{spell_def, token_def, unit_def, vanilla_catalog, vanilla_deck};

fn rush_token() -> CardDef {
    token_def("rush", [Tag::Token])
}

/// `{ ...spellDef(900), id: "fx-token-spell", index: "T-spell", token: true, tags: ["Token"] }`.
fn spell_token() -> CardDef {
    let mut def = spell_def(900, json!({}));
    def.id = "fx-token-spell".to_string();
    def.index = "T-spell".to_string();
    def.token = true;
    def.tags = vec![Tag::Token];
    def
}

fn stack_unit() -> CardDef {
    unit_def(500, json!({ "keywords": [{ "kind": "Stack" }] }))
}

fn catalog() -> CardDefs {
    let mut catalog = vanilla_catalog(40, 1);
    for def in [rush_token(), spell_token(), stack_unit()] {
        catalog.insert(def.id.clone(), def);
    }
    catalog
}

/// TS's `createGame({ …, catalog })` registered the catalog for the process; the Rust one only
/// validates against it (part 1), so the test registers it through the testkit override first.
fn game() -> GameState {
    register_catalog(catalog());
    create_game(&CreateGameOptions {
        seed: "zones".to_string(),
        decks: (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21)),
        catalog: Some(catalog()),
        ..Default::default()
    })
}

/// The card under this id as it stands in the state (TS held the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn put(state: &mut GameState, def_id: &str, at: ZoneSlot, stack: bool) -> CardInstance {
    let mut card = new_instance(state, def_id, at.player, Zone::Hand { player: at.player });
    let ok = place_on_field(state, &mut card, at, PlaceOnFieldOptions { stack: Some(stack) });
    assert!(ok);
    live(state, &card.id).clone()
}

fn unit_slot(player: PlayerId, lane: i32) -> ZoneSlot {
    ZoneSlot { player, row: Row::Units, lane }
}

fn back_slot(player: PlayerId, lane: i32) -> ZoneSlot {
    ZoneSlot { player, row: Row::Backrow, lane }
}

fn lanes(slots: &[ZoneSlot]) -> Vec<i32> {
    slots.iter().map(|s| s.lane).collect()
}

fn ids<'a>(cards: impl IntoIterator<Item = &'a CardInstance>) -> Vec<String> {
    cards.into_iter().map(|c| c.id.clone()).collect()
}

mod adjacency_m1_t4 {
    use super::*;

    #[test]
    fn lane_1_has_one_neighbour_lane_3_has_two_and_never_across_sides() {
        assert_eq!(lanes(&adjacent(unit_slot(PlayerId::P1, 1))), vec![2]);
        assert_eq!(lanes(&adjacent(unit_slot(PlayerId::P1, 3))), vec![2, 4]);
        assert_eq!(lanes(&adjacent(unit_slot(PlayerId::P1, 5))), vec![4]);
        assert!(adjacent(unit_slot(PlayerId::P1, 3)).iter().all(|s| s.player == PlayerId::P1 && s.row == Row::Units));
        assert_eq!(lanes(&adjacent(back_slot(PlayerId::P2, 2))), vec![1, 3]);
    }
}

mod r14_rotation_rings_m1_t4 {
    use super::*;

    #[test]
    fn runs_your_lanes_1_5_then_the_opponents_5_1() {
        assert_eq!(
            ring_order(Row::Units, PlayerId::P1).iter().map(|s| format!("{}{}", s.player, s.lane)).collect::<Vec<_>>(),
            vec!["p11", "p12", "p13", "p14", "p15", "p25", "p24", "p23", "p22", "p21"]
        );
    }

    #[test]
    fn rotating_right_from_your_lane_5_lands_on_the_opponents_lane_5() {
        assert_eq!(
            ring_neighbor(unit_slot(PlayerId::P1, 5), RotationDirection::Right, PlayerId::P1),
            unit_slot(PlayerId::P2, 5)
        );
    }

    #[test]
    fn rotating_right_from_the_opponents_lane_1_lands_on_your_lane_1() {
        assert_eq!(
            ring_neighbor(unit_slot(PlayerId::P2, 1), RotationDirection::Right, PlayerId::P1),
            unit_slot(PlayerId::P1, 1)
        );
    }

    #[test]
    fn rotating_left_is_the_inverse_of_rotating_right() {
        for slot in ring_order(Row::Units, PlayerId::P2) {
            let right = ring_neighbor(slot, RotationDirection::Right, PlayerId::P2);
            assert_eq!(ring_neighbor(right, RotationDirection::Left, PlayerId::P2), slot);
        }
    }

    #[test]
    fn keeps_the_backrow_ring_independent_of_the_unit_ring() {
        assert_eq!(
            ring_neighbor(back_slot(PlayerId::P1, 5), RotationDirection::Right, PlayerId::P1),
            back_slot(PlayerId::P2, 5)
        );
        assert!(ring_order(Row::Backrow, PlayerId::P1).iter().all(|s| s.row == Row::Backrow));
    }
}

mod locks_and_free_zones_m1_t4 {
    use super::*;

    #[test]
    fn a_summon_into_a_full_row_finds_no_zone_and_changes_nothing() {
        let mut state = game();
        for lane in [1, 2, 3, 4, 5] {
            put(&mut state, "fx-1", unit_slot(PlayerId::P1, lane), false);
        }
        assert_eq!(first_free_zone(&state, PlayerId::P1, Row::Units), None);

        let before = state.clone();
        let mut extra = new_instance(&mut state, "fx-2", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        extra.id = "no-zone-probe".to_string();
        assert!(!place_on_field(&mut state, &mut extra, unit_slot(PlayerId::P1, 3), Default::default()));
        assert_eq!(active_units_of(&state, PlayerId::P1).len(), 5);
        assert_eq!(state.players.p1.units, before.players.p1.units);
    }

    #[test]
    fn r688_a_locked_zone_takes_a_summon_but_no_play_and_stays_locked_after_its_occupant_leaves() {
        let mut state = game();
        let card = put(&mut state, "fx-1", unit_slot(PlayerId::P1, 2), false);
        lock_zone(&mut state, unit_slot(PlayerId::P1, 2));
        let _ = remove_from_field(&mut state, &card, Default::default());

        assert!(card_at(&state, unit_slot(PlayerId::P1, 2)).is_none());
        assert!(!is_open(&state, unit_slot(PlayerId::P1, 2)));
        assert!(takes_move(&state, unit_slot(PlayerId::P1, 2)));
        let mut third = new_instance(&mut state, "fx-3", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        assert!(place_on_field(&mut state, &mut third, unit_slot(PlayerId::P1, 2), Default::default()));
        assert_eq!(lanes(&open_zones(&state, PlayerId::P1, Row::Units)), vec![1, 3, 4, 5]);
        assert_eq!(first_free_zone(&state, PlayerId::P1, Row::Units), Some(unit_slot(PlayerId::P1, 1)));
    }

    #[test]
    fn r64_a_zone_reserved_for_a_reborn_unit_is_not_open() {
        let mut state = game();
        reserve_zone(&mut state, unit_slot(PlayerId::P1, 1));
        assert!(!is_open(&state, unit_slot(PlayerId::P1, 1)));
        assert_eq!(first_free_zone(&state, PlayerId::P1, Row::Units), Some(unit_slot(PlayerId::P1, 2)));
    }
}

mod stack_piles_3_2_m1_t4 {
    use super::*;

    #[test]
    fn makes_the_pushed_card_the_top_keeps_only_it_active_and_resumes_the_card_beneath_with_its_damage() {
        let mut state = game();
        let under = put(&mut state, "fx-1", unit_slot(PlayerId::P1, 1), false);
        find_instance_mut(&mut state, &under.id).expect("the card beneath").damage = 3;

        let top = put(&mut state, &stack_unit().id, unit_slot(PlayerId::P1, 1), true);
        assert_eq!(card_at(&state, unit_slot(PlayerId::P1, 1)).map(|c| c.id.clone()), Some(top.id.clone()));
        assert_eq!(ids(active_units_of(&state, PlayerId::P1)), vec![top.id.clone()]);
        assert_eq!(ids(dormant_units_of(&state, PlayerId::P1)), vec![under.id.clone()]);

        let _ = remove_from_field(&mut state, &top, Default::default());
        assert_eq!(card_at(&state, unit_slot(PlayerId::P1, 1)).map(|c| c.id.clone()), Some(under.id.clone()));
        assert_eq!(live(&state, &under.id).damage, 3);
    }

    #[test]
    fn refuses_an_occupied_zone_without_stack() {
        let mut state = game();
        put(&mut state, "fx-1", unit_slot(PlayerId::P1, 1), false);
        let mut other = new_instance(&mut state, "fx-2", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        assert!(!place_on_field(&mut state, &mut other, unit_slot(PlayerId::P1, 1), Default::default()));
    }
}

mod r11_tokens_leaving_a_zone_m1_t4 {
    use super::*;

    #[test]
    fn a_bounced_or_exiled_unit_token_is_in_no_hand_library_graveyard_or_exile() {
        for zone in [OffFieldZone::Hand, OffFieldZone::Exile, OffFieldZone::Graveyard] {
            let mut state = game();
            let mut token = put(&mut state, &rush_token().id, unit_slot(PlayerId::P1, 1), false);
            assert_eq!(move_to_zone(&mut state, &mut token, zone, Default::default()), MoveResult::Vanished);

            let side = &state.players.p1;
            let mut everywhere = side.hand.iter().chain(&side.library).chain(&side.graveyard).chain(&side.exile);
            assert!(!everywhere.any(|c| c.id == token.id));
            assert_eq!(active_units_of(&state, PlayerId::P1).len(), 0);
        }
    }

    #[test]
    fn a_spell_token_that_resolves_goes_to_the_graveyard() {
        let mut state = game();
        let mut card = new_instance(&mut state, &spell_token().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players.p1.hand.push(card.clone());
        assert_eq!(
            move_to_zone(&mut state, &mut card, OffFieldZone::Graveyard, Default::default()),
            MoveResult::Moved
        );
        assert_eq!(ids(&state.players.p1.graveyard), vec![card.id.clone()]);
        assert_eq!(state.players.p1.hand.len(), 0);
    }

    #[test]
    fn r78_a_non_token_card_leaving_the_field_resets_but_keeps_cost_mod_and_radiant() {
        let mut state = game();
        let card = put(&mut state, "fx-1", unit_slot(PlayerId::P1, 1), false);
        {
            let on_field = find_instance_mut(&mut state, &card.id).expect("the card on the field");
            on_field.damage = 2;
            on_field.buffs = AttackHealth { attack: 3, health: 3 };
            on_field.radiant = true;
            on_field.cost_mod = -1;
            on_field.counters = Counters { plague: Some(2), ..Default::default() };
        }

        let mut moving = live(&state, &card.id).clone();
        let _ = move_to_zone(&mut state, &mut moving, OffFieldZone::Hand, Default::default());
        let moved = live(&state, &card.id);
        assert_eq!(moved.damage, 0);
        assert_eq!(moved.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(moved.counters, Counters::default());
        assert!(moved.radiant);
        assert_eq!(moved.cost_mod, -1);
        assert_eq!(moved.zone, Zone::Hand { player: PlayerId::P1 });
    }

    #[test]
    fn r12_a_card_always_goes_to_its_owners_zone_even_under_another_controller() {
        let mut state = game();
        let mut card = new_instance(&mut state, "fx-1", PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
        let _ = place_on_field(&mut state, &mut card, unit_slot(PlayerId::P1, 1), Default::default());
        assert_eq!(live(&state, &card.id).controller, PlayerId::P1);

        let mut moving = live(&state, &card.id).clone();
        let _ = move_to_zone(&mut state, &mut moving, OffFieldZone::Graveyard, Default::default());
        assert_eq!(ids(&state.players.p2.graveyard), vec![card.id.clone()]);
        assert_eq!(state.players.p1.graveyard.len(), 0);
        assert_eq!(live(&state, &card.id).controller, PlayerId::P2);
    }

    #[test]
    fn shuffles_into_a_library_at_a_chosen_position() {
        let mut state = game();
        let mut card = new_instance(&mut state, "fx-1", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players.p1.hand.push(card.clone());
        let _ = move_to_zone(
            &mut state,
            &mut card,
            OffFieldZone::Library,
            MoveToZoneOptions { position: Some(LibraryPosition::At(3)), ..Default::default() },
        );
        assert_eq!(state.players.p1.library.get(3).map(|c| c.id.clone()), Some(card.id.clone()));
        assert_eq!(state.players.p1.library.len(), (DECK_SIZE + 1) as usize);
    }
}

mod r64_fill_your_board_m1_t4 {
    use super::*;

    #[test]
    fn lists_every_empty_unlocked_unit_zone_left_to_right() {
        let mut state = game();
        put(&mut state, "fx-1", unit_slot(PlayerId::P1, 2), false);
        lock_zone(&mut state, unit_slot(PlayerId::P1, 4));
        assert_eq!(lanes(&fill_board_zones(&state, PlayerId::P1)), vec![1, 3, 5]);
    }

    #[test]
    fn lists_nothing_when_the_row_is_full() {
        let mut state = game();
        for lane in [1, 2, 3, 4, 5] {
            put(&mut state, "fx-1", unit_slot(PlayerId::P1, lane), false);
        }
        assert_eq!(fill_board_zones(&state, PlayerId::P1), Vec::<ZoneSlot>::new());
    }
}

mod slots {
    use super::*;

    #[test]
    fn lists_five_zones_per_row_per_player() {
        assert_eq!(slots_of(PlayerId::P1, Row::Units).len(), 5);
        assert_eq!(lanes(&slots_of(PlayerId::P2, Row::Backrow)), vec![1, 2, 3, 4, 5]);
    }
}
