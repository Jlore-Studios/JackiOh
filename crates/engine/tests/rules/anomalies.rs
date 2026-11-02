// Anomalies, the board swap, and the Scanning Probes (issue #1855): `replace_board` (R1855000, R1855001,
// R1855002) and `scan_for_anomaly` (R1855003), each through a fixture script (`fixtures/anomalies.rs`):
// Stinkyscape's Anomaly Found!, its ten boards and the Scanning Probes (S20, R1879001, R1879002).

use jackioh_engine::effects::{AnomalyScan, anomaly_chances, anomaly_scan_of};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::anomalies::{
    AIM_DAMAGE, ASK_DAMAGE, FLIER_DAMAGE, MODE_THREE, MODE_TWO, aimer, anomaly, asker, banisher, crowd,
    exodus, field, fleet, flier, found, greeter, grunt, hangar, lucky_probe, mine, misfit, modal, parley,
    planner, polite, probe, register_anomaly_fixtures, scheme, spinner, trap, wall, ward, welcome, whirl,
    wipe,
};
use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::resources::{ASTEROID, ASTEROID_YIELD, MYTHIC};

/// `p1`'s turn 9, its main phase; each side's setup as given, with plain cards to draw so no turn ends
/// by itself (§2.5).
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_anomaly_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([plain.id, plain.id, plain.id, plain.id]);
        }
    }
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

/// The `summoned` events of the last step, as (definition, row, lane).
fn summoned(events: &[GameEvent]) -> Vec<(String, Row, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned {
                def_id, row, lane, ..
            } => Some((def_id.clone(), *row, *lane)),
            _ => None,
        })
        .collect()
}

/// The `damage` events of the last step, as (source, amount).
fn damage_dealt(events: &[GameEvent]) -> Vec<(Option<String>, i32)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                source_id, amount, ..
            } => Some((source_id.clone(), *amount)),
            _ => None,
        })
        .collect()
}

/// Where the first event of `kind` sits in `events`.
fn first_of(events: &[GameEvent], kind: GameEventType) -> Option<usize> {
    events.iter().position(|event| event.event_type() == kind)
}

fn in_exile(s: &Scenario, id: &str) -> bool {
    s.card(id).zone.z() == ZoneName::Exile
}

/// The id of the Unit on top of `player`'s unit zone `lane`.
fn unit_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane).expect("a unit in that zone").id
}

mod r1855000_a_board_replacement_exiles_its_casters_side_and_lays_the_preset {
    use super::*;

    #[test]
    fn r1855000_the_casters_side_goes_to_exile_and_the_preset_takes_its_zones() {
        let mut s = board(
            "r1855000-swap",
            json!({
                "hand": [anomaly.id],
                "field": [{ "def": plain.id, "lane": 1 }, { "def": plain.id, "lane": 2 }],
                "backrow": [{ "def": trap.id, "lane": 1 }],
            }),
            json!({ "field": [plain.id] }),
        );
        let old = vec![
            unit_id(&s, P1, 1),
            unit_id(&s, P1, 2),
            s.backrow(P1, 1).expect("the old trap").id,
        ];
        let theirs = unit_id(&s, P2, 1);
        let caster = s.hand(P1)[0].id.clone();
        s.play(anomaly.id.as_str(), json!({}));

        for id in &old {
            assert!(in_exile(&s, id), "{id} was exiled");
        }
        assert_eq!(unit_id(&s, P2, 1), theirs, "the other side is untouched");
        assert!(!in_exile(&s, &theirs));

        let turn = s.state().turn;
        let first = s.unit(P1, 1).expect("the grunt");
        assert_eq!(first.def_id, grunt.id);
        assert_eq!(first.position.unwrap_or(Position::Atk), Position::Atk);
        assert_eq!(first.summoned_turn, Some(turn), "a summon is summoning sick");
        assert!(!first.radiant);
        assert!(s.unit(P1, 2).is_none(), "the preset leaves lane 2 empty");
        let third = s.unit(P1, 3).expect("the wall");
        assert_eq!(third.def_id, wall.id);
        assert!(third.radiant, "the entry is Radiant");
        assert_eq!(
            third.position,
            Some(Position::Def),
            "the entry is in Defense Position"
        );
        assert_eq!(third.summoned_turn, Some(turn));
        assert!(
            !third.exertion.switched,
            "R20: an effect's switch spends no exertion"
        );

        let set = s.backrow(P1, 2).expect("the trap");
        assert_eq!(set.def_id, trap.id);
        assert_ne!(set.face_up, Some(true), "a summoned Trap is face-down");
        assert!(
            matches!(s.view(P2).opponent.backrow[1], Some(BackrowView::FaceDown(_))),
            "the other seat sees a face-down card"
        );
        let laid = s.backrow(P1, 4).expect("the field spell");
        assert_eq!(laid.def_id, field.id);
        assert_eq!(laid.face_up, Some(true), "a Field Spell is public");

        let events = s.last_events();
        let replaced = first_of(events, GameEventType::BoardReplaced).expect("boardReplaced");
        assert_eq!(events[replaced], GameEvent::BoardReplaced { player: P1 });
        assert!(
            replaced < first_of(events, GameEventType::Exiled).expect("exiled"),
            "boardReplaced comes before the cards go"
        );
        for viewer in [P1, P2] {
            assert!(
                s.view(viewer)
                    .events
                    .contains(&GameEvent::BoardReplaced { player: P1 }),
                "{viewer:?} sees the replacement"
            );
        }
        assert_eq!(
            summoned(events),
            vec![
                (grunt.id.clone(), Row::Units, 1),
                (wall.id.clone(), Row::Units, 3),
                (trap.id.clone(), Row::Backrow, 2),
                (field.id.clone(), Row::Backrow, 4),
            ],
            "the preset is laid in list order"
        );
        assert!(
            events.iter().all(|event| match event {
                GameEvent::Summoned { source_id, .. } => source_id.as_deref() == Some(caster.as_str()),
                _ => true,
            }),
            "each laid card names the replacing card (R1088)"
        );
    }

    #[test]
    fn r1855000_a_dormant_card_stays_and_its_zone_takes_nothing() {
        let mut s = board(
            "r1855000-dormant",
            json!({
                "hand": [anomaly.id],
                "field": [{ "def": plain.id, "lane": 3, "slumber": { "turns": 2 } }],
            }),
            json!({}),
        );
        let dormant = unit_id(&s, P1, 3);
        s.play(anomaly.id.as_str(), json!({}));

        assert_eq!(unit_id(&s, P1, 3), dormant, "the Dormant card stays where it is");
        assert!(s.card(dormant.as_str()).slumber.is_some(), "still Dormant");
        assert!(
            !summoned(s.last_events())
                .iter()
                .any(|(def_id, _, _)| *def_id == wall.id),
            "the preset's card for its zone is skipped"
        );
        assert_eq!(s.unit(P1, 1).expect("the grunt").def_id, grunt.id);
    }

    #[test]
    fn r1855000_a_unit_the_spell_cannot_reach_stays_and_fires_no_cry() {
        let mut s = board(
            "r1855000-ward",
            json!({ "hand": [anomaly.id], "field": [{ "def": ward.id, "lane": 1 }] }),
            json!({}),
        );
        let warded = unit_id(&s, P1, 1);
        let health = s.state().players[P2].hero.health;
        s.play(anomaly.id.as_str(), json!({}));

        assert_eq!(unit_id(&s, P1, 1), warded, "a Unit immune to Spells stays");
        assert!(
            !summoned(s.last_events())
                .iter()
                .any(|(def_id, _, _)| *def_id == grunt.id),
            "nothing is laid in its zone"
        );
        assert_eq!(
            s.state().players[P2].hero.health,
            health,
            "a card the preset did not lay fires no Cry"
        );
    }

    #[test]
    fn r1855000_a_stack_pile_goes_to_exile_whole() {
        let mut s = board(
            "r1855000-stack",
            json!({ "hand": [anomaly.id], "field": [plain.id, { "def": wall.id, "stack": true }] }),
            json!({}),
        );
        let pile: Vec<String> = pile_at(
            s.state(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 1,
            },
        )
        .expect("the pile")
        .iter()
        .map(|card| card.id.clone())
        .collect();
        assert_eq!(pile.len(), 2);
        s.play(anomaly.id.as_str(), json!({}));

        for id in &pile {
            assert!(in_exile(&s, id), "{id} was exiled");
        }
        assert_eq!(s.unit(P1, 1).expect("the grunt").def_id, grunt.id);
    }

    #[test]
    fn r1855000_an_entry_that_does_not_fit_lays_nothing() {
        let mut s = board("r1855000-misfit", json!({ "hand": [misfit.id] }), json!({}));
        s.play(misfit.id.as_str(), json!({}));

        assert!(
            summoned(s.last_events()).is_empty(),
            "a row the card does not belong in, a Spell and a lane off the row lay nothing"
        );
        for lane in 1..=5 {
            assert!(s.unit(P1, lane).is_none(), "unit lane {lane}");
            assert!(s.backrow(P1, lane).is_none(), "backrow lane {lane}");
        }
    }

    #[test]
    fn r1855000_a_locked_zone_takes_its_card_and_a_reserved_one_takes_nothing() {
        let mut s = board("r1855000-zones", json!({ "hand": [anomaly.id] }), json!({}));
        lock_zone(
            s.state_mut(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 1,
            },
        );
        reserve_zone(
            s.state_mut(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 3,
            },
        );
        s.play(anomaly.id.as_str(), json!({}));

        assert_eq!(
            s.unit(P1, 1).expect("the grunt").def_id,
            grunt.id,
            "R688: a named summon enters a Locked zone"
        );
        assert!(s.unit(P1, 3).is_none(), "R64: a reserved zone takes nothing");
    }
}

mod r1855001_a_replaced_boards_laid_units_fire_their_cries_at_random {
    use super::*;

    /// `crowd` played on `seed`: the modal's damage, once each laid Cry has fired.
    fn crowd_on(seed: &str) -> Scenario {
        let mut s = board(seed, json!({ "hand": [crowd.id] }), json!({}));
        s.play(crowd.id.as_str(), json!({}));
        s
    }

    fn modal_damage(s: &Scenario) -> Option<i32> {
        let modal_id = s.unit(P1, 2)?.id;
        damage_dealt(s.last_events())
            .into_iter()
            .find(|(source, _)| source.as_deref() == Some(modal_id.as_str()))
            .map(|(_, amount)| amount)
    }

    #[test]
    fn r1855001_every_laid_cry_fires_with_its_choices_drawn_at_random() {
        let s = crowd_on("r1855001-crowd");
        assert!(s.state().pending.is_none(), "the caster is never asked");

        let ids = [unit_id(&s, P1, 1), unit_id(&s, P1, 2), unit_id(&s, P1, 3)];
        assert_eq!(s.card(ids[0].as_str()).def_id, aimer.id);
        assert_eq!(s.card(ids[1].as_str()).def_id, modal.id);
        assert_eq!(s.card(ids[2].as_str()).def_id, asker.id);
        let events = s.last_events();
        let dealt = damage_dealt(events);
        let from = |id: &str| -> Vec<i32> {
            dealt
                .iter()
                .filter(|(source, _)| source.as_deref() == Some(id))
                .map(|(_, amount)| *amount)
                .collect()
        };
        assert_eq!(
            from(&ids[0]),
            vec![AIM_DAMAGE],
            "the aimer's Cry, its target drawn"
        );
        let moded = from(&ids[1]);
        assert_eq!(moded.len(), 1, "the modal's Cry, its mode drawn");
        assert!([MODE_TWO, MODE_THREE].contains(&moded[0]));
        assert_eq!(
            from(&ids[2]),
            vec![ASK_DAMAGE],
            "the asker's own question answered at random"
        );

        let order: Vec<&str> = dealt.iter().filter_map(|(source, _)| source.as_deref()).collect();
        assert_eq!(
            order,
            vec![ids[0].as_str(), ids[1].as_str(), ids[2].as_str()],
            "in list order"
        );
        let last_summon = events
            .iter()
            .rposition(|event| event.event_type() == GameEventType::Summoned)
            .expect("summoned");
        assert!(
            last_summon < first_of(events, GameEventType::Damage).expect("damage"),
            "every card is laid before the first Cry fires"
        );
    }

    #[test]
    fn r1855001_the_draws_replay_and_vary() {
        let once = crowd_on("r1855001-replay");
        let again = crowd_on("r1855001-replay");
        assert_eq!(
            serde_json::to_value(once.state()).expect("serializes"),
            serde_json::to_value(again.state()).expect("serializes"),
            "the same seed draws the same choices"
        );

        let two = seed_where("r1855001-two", |seed| {
            modal_damage(&crowd_on(seed)) == Some(MODE_TWO)
        });
        assert_eq!(modal_damage(&crowd_on(&two)), Some(MODE_TWO));
        let three = seed_where("r1855001-three", |seed| {
            modal_damage(&crowd_on(seed)) == Some(MODE_THREE)
        });
        assert_eq!(modal_damage(&crowd_on(&three)), Some(MODE_THREE));
    }

    #[test]
    fn r1855001_a_laid_unit_that_left_before_its_cry_fires_nothing() {
        let mut s = board("r1855001-exodus", json!({ "hand": [exodus.id] }), json!({}));
        s.play(exodus.id.as_str(), json!({}));

        assert_eq!(s.unit(P1, 1).expect("the banisher").def_id, banisher.id);
        assert!(s.unit(P1, 2).is_none(), "the banisher's Cry exiled the aimer");
        let aimer_id = s
            .pile(P1, "exile")
            .into_iter()
            .find(|card| card.def_id == aimer.id)
            .expect("the aimer in exile")
            .id;
        assert!(
            damage_dealt(s.last_events())
                .iter()
                .all(|(source, _)| source.as_deref() != Some(aimer_id.as_str())),
            "R174: the aimer's Cry fires nothing"
        );
    }

    #[test]
    fn r1855001_a_laid_unit_an_earlier_cry_moved_fires_where_it_stands() {
        let mut s = board("r1855001-whirl", json!({ "hand": [whirl.id] }), json!({}));
        s.play(whirl.id.as_str(), json!({}));

        let events = s.last_events();
        assert!(
            first_of(events, GameEventType::Rotated).is_some(),
            "the spinner's Cry rotated the board"
        );
        let mut on_field: Vec<CardInstance> = Vec::new();
        for player in [P1, P2] {
            for lane in 1..=5 {
                on_field.extend(s.unit(player, lane));
            }
        }
        assert!(on_field.iter().any(|card| card.def_id == spinner.id));
        let moved = on_field
            .iter()
            .find(|card| card.def_id == aimer.id)
            .expect("the aimer, still on the field");
        assert_ne!(
            s.unit(P1, 2).map(|card| card.id),
            Some(moved.id.clone()),
            "the aimer left the zone it was laid in"
        );
        let from_it: Vec<i32> = damage_dealt(events)
            .into_iter()
            .filter(|(source, _)| source.as_deref() == Some(moved.id.as_str()))
            .map(|(_, amount)| amount)
            .collect();
        assert_eq!(from_it, vec![AIM_DAMAGE], "its Cry fires where it stands now");
    }

    #[test]
    fn r1855000_r1855001_a_question_asked_on_arrival_pauses_the_rest_which_resumes_and_fires_the_cries() {
        let mut s = board("r1855001-welcome", json!({ "hand": [welcome.id] }), json!({}));
        s.play(welcome.id.as_str(), json!({}));
        assert!(
            s.state().pending.is_some(),
            "the greeter asks as it arrives (R151)"
        );
        assert_eq!(s.unit(P1, 1).expect("the greeter").def_id, greeter.id);
        assert!(s.unit(P1, 2).is_none(), "the rest waits for the answer");
        // §9.3: the paused replacement is data.
        let round: GameState =
            serde_json::from_str(&serde_json::to_string(s.state()).expect("serializes")).expect("parses");
        assert_eq!(&round, s.state());

        s.answer(json!("hello"));
        assert!(s.state().pending.is_none());
        let aimer_id = unit_id(&s, P1, 2);
        assert_eq!(
            s.card(aimer_id.as_str()).def_id,
            aimer.id,
            "the rest resumes (R113)"
        );
        assert_eq!(
            damage_dealt(s.last_events()),
            vec![(Some(aimer_id), AIM_DAMAGE)],
            "and the laid Cries fire once it is laid"
        );
    }

    #[test]
    fn r1855001_a_cry_the_other_player_pauses_keeps_its_casters_choices_random_when_it_resumes() {
        let mut s = board("r1855001-parley", json!({ "hand": [parley.id] }), json!({}));
        let health = s.state().players[P2].hero.health;
        s.play(parley.id.as_str(), json!({}));
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.player_id),
            Some(P2),
            "the other player's question is theirs"
        );
        assert!(s.unit(P1, 2).is_some(), "the whole preset is laid first");
        // §9.3: the paused Cry is data, and nothing of the random mode is left on the state at rest.
        let round: GameState =
            serde_json::from_str(&serde_json::to_string(s.state()).expect("serializes")).expect("parses");
        assert_eq!(&round, s.state());
        assert!(s.state().casts_resolving.is_none());

        s.answer(json!("heads"));
        assert!(
            s.state().pending.is_none(),
            "the rest of the Cry draws its caster's question at random"
        );
        let polite_id = unit_id(&s, P1, 1);
        let aimer_id = unit_id(&s, P1, 2);
        assert_eq!(s.card(polite_id.as_str()).def_id, polite.id);
        assert_eq!(
            damage_dealt(s.last_events()),
            vec![(Some(polite_id), ASK_DAMAGE), (Some(aimer_id), AIM_DAMAGE)],
            "the paused Cry ends, then the next one fires, its target drawn"
        );
        assert!(s.state().players[P2].hero.health <= health - ASK_DAMAGE);
        assert!(s.state().casts_resolving.is_none());
    }

    #[test]
    fn r1855001_a_delayed_effect_a_laid_cry_sets_up_asks_its_controller() {
        let mut s = board("r1855001-scheme", json!({ "hand": [scheme.id] }), json!({}));
        s.play(scheme.id.as_str(), json!({}));
        assert_eq!(s.unit(P1, 1).expect("the planner").def_id, planner.id);
        assert!(s.state().pending.is_none());

        s.end_turn();
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.player_id),
            Some(P1),
            "R62: a run of its own, so its question is its controller's"
        );
    }
}

mod r1855002_a_preset_cards_rider_is_memory_its_own_text_reads {
    use super::*;

    #[test]
    fn r1855002_a_laid_card_gains_only_from_its_pool() {
        let mut s = board("r1855002-pool", json!({ "hand": [mine.id, wipe.id] }), json!({}));
        s.play(mine.id.as_str(), json!({}));
        assert_eq!(s.unit(P1, 1).expect("the asteroid").def_id, ASTEROID);
        s.play(wipe.id.as_str(), json!({}));

        // R2250000: its Resources join the hand.
        let gained = resources_in_hand_of(s.state(), P1);
        assert_eq!(
            gained.len() as i32,
            ASTEROID_YIELD,
            "its Death gains its Resources"
        );
        assert!(
            gained.iter().all(|card| card.def_id == MYTHIC),
            "every one from the rider's pool: {:?}",
            gained.iter().map(|card| card.def_id.clone()).collect::<Vec<_>>()
        );
    }
}

mod r1855003_an_anomaly_scan_finds_at_5_and_20_percent_and_each_miss_adds_1_and_4 {
    use super::*;

    /// `card` (a probe) played on `seed`, with P1's misses set to `misses` first.
    fn scan_on(seed: &str, card: &CardDef, misses: Option<i32>) -> Scenario {
        let mut s = board(seed, json!({ "hand": [card.id] }), json!({}));
        s.state_mut().players[P1].anomaly_misses = misses;
        s.play(card.id.as_str(), json!({}));
        s
    }

    /// The definitions the last step added to P1's hand.
    fn added(events: &[GameEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player, def_id, .. } if *player == P1 => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn finds(s: &Scenario) -> Vec<CardInstance> {
        s.pile(P1, "hand")
            .into_iter()
            .filter(|card| card.def_id == found.id)
            .collect()
    }

    #[test]
    fn r1855003_the_chances_and_their_steps() {
        assert_eq!(ANOMALY_RADIANT_CHANCE, 5);
        assert_eq!(ANOMALY_PLAIN_CHANCE, 20);
        assert_eq!(ANOMALY_RADIANT_STEP, 1);
        assert_eq!(ANOMALY_PLAIN_STEP, 4);
        assert_eq!(ANOMALY_ROLL_SIDES, 100);

        assert_eq!(anomaly_chances(0), (5, 20));
        assert_eq!(anomaly_chances(1), (6, 24));
        assert_eq!(anomaly_chances(15), (20, 80));
        assert_eq!(anomaly_chances(95), (100, 0), "the two never sum past the roll");

        assert_eq!(anomaly_scan_of(4, 0), AnomalyScan::Radiant);
        assert_eq!(anomaly_scan_of(5, 0), AnomalyScan::Plain);
        assert_eq!(anomaly_scan_of(24, 0), AnomalyScan::Plain);
        assert_eq!(anomaly_scan_of(25, 0), AnomalyScan::Miss);
    }

    #[test]
    fn r1855003_a_miss_counts_and_a_find_adds_the_card() {
        let missed = seed_where("r1855003-miss", |seed| {
            finds(&scan_on(seed, &probe, None)).is_empty()
        });
        let s = scan_on(&missed, &probe, None);
        assert!(finds(&s).is_empty());
        assert_eq!(s.state().players[P1].anomaly_misses, Some(1), "a miss counts");
        assert_eq!(s.state().players[P2].anomaly_misses, None, "for its player alone");
        assert!(added(s.last_events()).is_empty(), "silently");

        let hit = seed_where("r1855003-find", |seed| {
            finds(&scan_on(seed, &probe, None)).len() == 1
        });
        let s = scan_on(&hit, &probe, None);
        assert_eq!(finds(&s).len(), 1, "a find adds the card");
        assert_eq!(
            added(s.last_events()),
            vec![found.id.clone()],
            "with its addedToHand"
        );
        assert_eq!(s.state().players[P1].anomaly_misses, None);
        assert_eq!(s.state().players[P2].anomaly_misses, None);
    }

    #[test]
    fn r1855003_a_find_keeps_the_count() {
        let s = scan_on("r1855003-keep", &probe, Some(95));
        let cards = finds(&s);
        assert_eq!(cards.len(), 1, "at 95 misses every roll finds");
        assert!(cards[0].radiant, "and finds the Radiant one");
        assert_eq!(
            s.state().players[P1].anomaly_misses,
            Some(95),
            "the count is kept"
        );
    }

    #[test]
    fn r1855003_lucky_rolls_again_and_keeps_the_better() {
        let seed = seed_where("r1855003-lucky", |seed| {
            finds(&scan_on(seed, &probe, None)).is_empty()
                && finds(&scan_on(seed, &lucky_probe, None)).len() == 1
        });
        assert!(
            finds(&scan_on(&seed, &probe, None)).is_empty(),
            "the first roll misses"
        );
        let s = scan_on(&seed, &lucky_probe, None);
        assert_eq!(finds(&s).len(), 1, "Lucky rolls again and keeps the find");
        assert_eq!(s.state().players[P1].anomaly_misses, None);
    }
}

mod r1879001_a_preset_flying_unit_named_in_the_backrow_stands_there {
    use super::*;

    #[test]
    fn r1879001_a_flying_unit_stands_in_its_backrow_zone_and_fires_its_cry() {
        let mut s = board("r1879001-hangar", json!({ "hand": [hangar.id] }), json!({}));
        let health = s.state().players[P2].hero.health;
        s.play(hangar.id.as_str(), json!({}));

        assert_eq!(
            summoned(s.last_events()),
            vec![
                (flier.id.clone(), Row::Backrow, 1),
                (flier.id.clone(), Row::Backrow, 3),
            ],
            "the fliers stand in their backrow zones; a Unit without Flying lays nothing there"
        );
        let first = s.backrow(P1, 1).expect("the flier");
        assert_eq!(first.def_id, flier.id);
        assert!(s.backrow(P1, 2).is_none(), "the grunt lays nothing");
        let third = s.backrow(P1, 3).expect("the Radiant flier");
        assert!(third.radiant, "the entry is Radiant");
        assert_eq!(third.position, Some(Position::Def), "and in Defense Position");
        for lane in 1..=5 {
            assert!(s.unit(P1, lane).is_none(), "unit lane {lane} stays empty");
        }
        assert_eq!(
            s.state().players[P2].hero.health,
            health - 2 * FLIER_DAMAGE,
            "each flier's Cry fires once the board is laid"
        );
    }
}

mod r1879002_a_stacked_preset_entry_tops_the_card_laid_before_it {
    use super::*;

    #[test]
    fn r1879002_a_stacked_entry_makes_a_pile_on_a_laid_card_and_lays_alone_in_an_empty_zone() {
        let mut s = board("r1879002-fleet", json!({ "hand": [fleet.id] }), json!({}));
        s.play(fleet.id.as_str(), json!({}));

        let pile: Vec<String> = pile_at(
            s.state(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 2,
            },
        )
        .expect("the pile")
        .iter()
        .map(|card| card.def_id.clone())
        .collect();
        assert_eq!(
            pile,
            vec![grunt.id.clone(), wall.id.clone()],
            "the grunt on top, the wall beneath (a pile is top first)"
        );
        assert_eq!(s.unit(P1, 2).expect("the top").def_id, grunt.id);
        assert_eq!(
            s.unit(P1, 4).expect("the lone grunt").def_id,
            grunt.id,
            "an empty zone takes a stacked entry as any entry"
        );
    }

    #[test]
    fn r1879002_a_stacked_entry_lays_nothing_on_a_card_the_replacement_did_not_lay() {
        let mut s = board(
            "r1879002-dormant",
            json!({
                "hand": [fleet.id],
                "field": [{ "def": plain.id, "lane": 2, "slumber": { "turns": 2 } }],
            }),
            json!({}),
        );
        let dormant = unit_id(&s, P1, 2);
        s.play(fleet.id.as_str(), json!({}));

        assert_eq!(unit_id(&s, P1, 2), dormant, "the Dormant card stays on top");
        let pile = pile_at(
            s.state(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 2,
            },
        )
        .expect("its zone");
        assert_eq!(pile.len(), 1, "nothing is stacked onto it");
        assert_eq!(s.unit(P1, 4).expect("the lone grunt").def_id, grunt.id);
    }
}
