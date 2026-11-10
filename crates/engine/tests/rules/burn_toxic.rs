//! Burn and Toxic (issue #3540, R3540000–R3540005): the counters the verbs add, the start-of-turn tick,
//! the riders and the conversion on a hit, a Vanilla's clearing and the view, proved on the fixtures of
//! `fixtures/burn_toxic.rs` with no card yet.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::burn_toxic::*;

/// p1's turn 9, its main phase; each side's setup as given, with plain cards to draw, and no turn that
/// ends by itself (R345), so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_burn_toxic_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

/// A setup entry for a Unit that already carries counters.
fn with(def: &str, counters: Value) -> Value {
    json!({ "def": def, "counters": counters })
}

fn counter_events(events: &[GameEvent], counter: CounterKind) -> Vec<&GameEvent> {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::CounterChanged { counter: c, .. } if *c == counter))
        .collect()
}

fn burn(s: &Scenario, id: &str) -> i32 {
    query::burn_on(s.card(id))
}

fn toxic(s: &Scenario, id: &str) -> i32 {
    query::toxic_on(s.card(id))
}

/// The damage events of a step, as (target, amount, via) in the order they happened.
fn hits(events: &[GameEvent]) -> Vec<(String, i32, Option<KeywordKind>)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id,
                amount,
                via,
                ..
            } => Some((target_id.clone(), *amount, *via)),
            _ => None,
        })
        .collect()
}

mod r3540000_counters {
    use super::*;

    #[test]
    fn r3540000_the_verbs_add_to_the_counter_already_there() {
        let mut s = board(
            "r3540000-verbs",
            json!({ "hand": [BURNER, BURNER, TOXICANT, TOXICANT] }),
            json!({ "field": [PLAIN] }),
        );
        let target = id_at(&s, P2, 1);
        s.play(BURNER, pick(&target));
        assert_eq!(burn(&s, &target), BT_BURN);
        s.play(BURNER, pick(&target));
        assert_eq!(burn(&s, &target), 2 * BT_BURN);
        let reported: Vec<(i32, Option<i32>)> = counter_events(s.events(), CounterKind::Burn)
            .into_iter()
            .map(|event| match event {
                GameEvent::CounterChanged {
                    instance_id,
                    value,
                    placed,
                    ..
                } => {
                    assert_eq!(instance_id, &target);
                    (*value, *placed)
                }
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            reported,
            vec![(BT_BURN, Some(BT_BURN)), (2 * BT_BURN, Some(BT_BURN))]
        );

        s.play(TOXICANT, json!({}));
        s.play(TOXICANT, json!({}));
        assert_eq!(query::hero_toxic_of(s.state(), P2), 2 * BT_TOXIC);
        assert_eq!(s.state().players[P2].hero_toxic, Some(2 * BT_TOXIC));
        let last = counter_events(s.events(), CounterKind::Toxic)
            .last()
            .copied()
            .cloned();
        assert_eq!(
            last,
            Some(GameEvent::CounterChanged {
                instance_id: "hero-p2".to_string(),
                counter: CounterKind::Toxic,
                value: 2 * BT_TOXIC,
                placed: Some(BT_TOXIC),
            })
        );
    }

    #[test]
    fn r3540000_a_sweep_reaches_every_enemy_unit_and_hero_only() {
        let mut s = board(
            "r3540000-sweep",
            json!({ "hand": [SCOURGE], "field": [PLAIN] }),
            json!({ "field": [PLAIN, FRAIL] }),
        );
        let own = id_at(&s, P1, 1);
        let first = id_at(&s, P2, 1);
        let second = id_at(&s, P2, 2);
        s.play(SCOURGE, json!({}));
        for enemy in [&first, &second] {
            assert_eq!(burn(&s, enemy), BT_SWEEP);
            assert_eq!(toxic(&s, enemy), BT_SWEEP);
        }
        assert_eq!(query::hero_burn_of(s.state(), P2), BT_SWEEP);
        assert_eq!(query::hero_toxic_of(s.state(), P2), BT_SWEEP);
        assert_eq!(burn(&s, &own), 0);
        assert_eq!(toxic(&s, &own), 0);
        assert_eq!(query::hero_burn_of(s.state(), P1), 0);
        assert_eq!(query::hero_toxic_of(s.state(), P1), 0);
    }

    #[test]
    fn r3540000_a_unit_leaving_the_field_sheds_both() {
        let mut s = board(
            "r3540000-leave",
            json!({ "hand": [BURNER, BOUNCER] }),
            json!({ "field": [with(PLAIN, json!({ "toxic": 2 }))] }),
        );
        let target = id_at(&s, P2, 1);
        s.play(BURNER, pick(&target));
        assert_eq!(burn(&s, &target), BT_BURN);
        s.play(BOUNCER, pick(&target));
        s.expect_in_zone(&target, "hand");
        assert_eq!(burn(&s, &target), 0);
        assert_eq!(toxic(&s, &target), 0);
    }
}

mod r3540001_tick {
    use super::*;

    #[test]
    fn r3540001_burn_ticks_unpierced_and_drops_by_three_until_gone() {
        let mut s = board(
            "r3540001-burn",
            json!({ "field": [with(PLAIN, json!({ "burn": 5 }))] }),
            json!({ "field": [with(ARMORED, json!({ "burn": 8 }))] }),
        );
        s.state_mut().players[P2].hero_burn = Some(1);
        let armored = id_at(&s, P2, 1);
        let own = id_at(&s, P1, 1);

        // p1's turn ends: p2's turn starts, and only p2's counters tick.
        s.end_turn();
        let tick = hits(s.last_events());
        assert_eq!(
            tick,
            vec![
                ("hero-p2".to_string(), 1, Some(KeywordKind::Burn)),
                (armored.clone(), 6, Some(KeywordKind::Burn)),
            ],
            "the hero first, then the Unit; Armor 2 took 2 of the 8"
        );
        for event in s.last_events() {
            if let GameEvent::Damage { source_id, .. } = event {
                assert_eq!(source_id, &None, "a tick has no source");
            }
        }
        s.expect_health(P2, HERO_HEALTH - 1);
        assert_eq!(s.state().players[P2].hero_burn, None);
        assert_eq!(s.card(&armored).damage, 6);
        assert_eq!(burn(&s, &armored), 5);
        assert_eq!(s.card(&own).damage, 0);
        assert_eq!(burn(&s, &own), 5);

        // p2's turn ends and p1's own Burn ticks; then p2's Unit ticks again.
        s.end_turn();
        assert_eq!(s.card(&own).damage, 5);
        assert_eq!(burn(&s, &own), 2);
        s.end_turn();
        assert_eq!(s.card(&armored).damage, 9);
        assert_eq!(burn(&s, &armored), 2);

        // The next one: Armor takes the whole 2, the Burn still drops, and the count is gone.
        s.end_turn();
        s.end_turn();
        assert_eq!(s.card(&armored).damage, 9);
        assert_eq!(burn(&s, &armored), 0);
        assert_eq!(s.card(&armored).counters.burn, None);
    }

    #[test]
    fn r3540001_toxic_pierces_armor_never_fades_and_ticks_after_burn() {
        let mut s = board(
            "r3540001-toxic",
            json!({}),
            json!({ "armor": 5, "field": [with(ARMORED, json!({ "burn": 5, "toxic": 3 }))] }),
        );
        s.state_mut().players[P2].hero_toxic = Some(2);
        let armored = id_at(&s, P2, 1);

        s.end_turn();
        assert_eq!(
            hits(s.last_events()),
            vec![
                ("hero-p2".to_string(), 2, Some(KeywordKind::Toxic)),
                (armored.clone(), 3, Some(KeywordKind::Burn)),
                (armored.clone(), 3, Some(KeywordKind::Toxic)),
            ],
            "the hero's Toxic pierces its Armor 5; the Unit's Burn meets Armor 2, then its Toxic does not"
        );
        s.expect_health(P2, HERO_HEALTH - 2);
        assert_eq!(s.state().players[P2].hero.armor, 5);
        assert_eq!(s.card(&armored).damage, 6);
        assert_eq!(burn(&s, &armored), 2);

        // A cycle later Burn is spent against Armor and Toxic has not faded.
        s.end_turn();
        s.end_turn();
        assert_eq!(toxic(&s, &armored), 3);
        assert_eq!(query::hero_toxic_of(s.state(), P2), 2);
        assert_eq!(s.card(&armored).damage, 9);
    }

    #[test]
    fn r3540001_a_divine_shield_takes_the_first_tick() {
        let mut s = board(
            "r3540001-shield",
            json!({}),
            json!({ "field": [with(SHIELDED, json!({ "burn": 4, "toxic": 2 }))] }),
        );
        let shielded = id_at(&s, P2, 1);
        s.end_turn();
        assert_eq!(s.card(&shielded).divine_shield_spent, Some(true));
        assert!(s.last_events().iter().any(
            |event| matches!(event, GameEvent::DivineShieldLost { instance_id } if instance_id == &shielded)
        ));
        // Burn's hit was the shield's; Toxic's then landed.
        assert_eq!(
            hits(s.last_events()),
            vec![(shielded.clone(), 2, Some(KeywordKind::Toxic))]
        );
        assert_eq!(s.card(&shielded).damage, 2);
        assert_eq!(burn(&s, &shielded), 1, "the Burn drops whatever its hit did");
        assert_eq!(toxic(&s, &shielded), 2);
    }

    #[test]
    fn r3540001_the_tick_kills_before_the_draw() {
        let mut s = board(
            "r3540001-kill",
            json!({}),
            json!({ "field": [with(FRAIL, json!({ "burn": 2 }))] }),
        );
        let frail = id_at(&s, P2, 1);
        s.end_turn();
        let events = s.last_events();
        let died = events
            .iter()
            .position(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == &frail),
            )
            .expect("the tick destroyed the Unit");
        let drew = events
            .iter()
            .position(|event| matches!(event, GameEvent::Drawn { player, .. } if *player == P2))
            .expect("p2 drew");
        assert!(died < drew, "the Unit died at the tick, before p2's draw");
    }

    #[test]
    fn r3540001_a_lethal_burn_ends_the_game_at_its_owners_turn() {
        let mut s = board("r3540001-lethal", json!({}), json!({ "health": 3 }));
        s.state_mut().players[P2].hero_burn = Some(5);
        s.end_turn();
        let result = s.state().result.expect("the burn killed the hero");
        assert_eq!(result.winner, Winner::P1);
    }
}

mod r3540002_riders {
    use super::*;

    #[test]
    fn r3540002_damage_also_burns_after_each_hit_its_target_survives() {
        let mut s = board(
            "r3540002-burn",
            json!({ "field": [MACE] }),
            json!({ "field": [PLAIN] }),
        );
        let (mace, plain) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&mace, &plain);
        assert_eq!(burn(&s, &plain), BT_RIDER);

        let mut s = board("r3540002-hero", json!({ "field": [MACE] }), json!({}));
        let mace = id_at(&s, P1, 1);
        s.attack(&mace, "hero");
        s.expect_health(P2, HERO_HEALTH - 3);
        assert_eq!(query::hero_burn_of(s.state(), P2), BT_RIDER);

        let mut s = board(
            "r3540002-toxic",
            json!({ "field": [VENOM] }),
            json!({ "field": [PLAIN] }),
        );
        let (venom, plain) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&venom, &plain);
        assert_eq!(toxic(&s, &plain), BT_RIDER);
        assert_eq!(burn(&s, &plain), 0);
    }

    #[test]
    fn r3540002_no_rider_on_a_kill_or_a_hit_that_dealt_nothing() {
        let mut s = board(
            "r3540002-kill",
            json!({ "field": [MACE] }),
            json!({ "field": [FRAIL] }),
        );
        let (mace, frail) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&mace, &frail);
        assert!(
            s.events().iter().any(
                |event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == &frail)
            ),
            "the hit killed its target"
        );
        assert!(counter_events(s.events(), CounterKind::Burn).is_empty());

        let mut s = board(
            "r3540002-shield",
            json!({ "field": [MACE] }),
            json!({ "field": [SHIELDED] }),
        );
        let (mace, shielded) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&mace, &shielded);
        assert_eq!(s.card(&shielded).divine_shield_spent, Some(true));
        assert!(counter_events(s.events(), CounterKind::Burn).is_empty());
        assert_eq!(burn(&s, &shielded), 0);
    }

    #[test]
    fn r3540002_an_equip_adds_its_rider_to_its_host() {
        let mut s = board(
            "r3540002-equip",
            json!({ "hand": [ROUNDS], "field": [MACE] }),
            json!({ "field": [PLAIN] }),
        );
        s.play(ROUNDS, json!({ "zone": 1, "equip": true }));
        let (host, plain) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&host, &plain);
        assert_eq!(burn(&s, &plain), BT_RIDER + BT_ROUNDS);
    }
}

mod r3540003_conversion {
    use super::*;

    #[test]
    fn r3540003_damage_dealt_burns_deals_nothing_and_burns_what_it_would_have() {
        let mut s = board(
            "r3540003-burns",
            json!({ "field": [VOLCANIC] }),
            json!({ "field": [ARMORED] }),
        );
        let (volcanic, armored) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&volcanic, &armored);
        assert_eq!(s.card(&armored).damage, 0);
        assert_eq!(burn(&s, &armored), 2, "4 damage past Armor 2");
        assert!(
            hits(s.events()).iter().all(|(target, _, _)| target != &armored),
            "no damage event names the target"
        );
        assert_eq!(s.card(&volcanic).damage, 2, "the attacker is still struck back");

        let mut s = board("r3540003-hero", json!({ "field": [VOLCANIC] }), json!({}));
        let volcanic = id_at(&s, P1, 1);
        s.attack(&volcanic, "hero");
        s.expect_health(P2, HERO_HEALTH);
        assert_eq!(query::hero_burn_of(s.state(), P2), 4);
        assert!(hits(s.events()).iter().all(|(target, _, _)| target != "hero-p2"));
    }

    #[test]
    fn r3540003_is_toxic_converts_to_toxic_and_burn_wins_both() {
        let mut s = board(
            "r3540003-toxic",
            json!({ "field": [NOXIOUS] }),
            json!({ "field": [PLAIN] }),
        );
        let (noxious, plain) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&noxious, &plain);
        assert_eq!(toxic(&s, &plain), 4);
        assert_eq!(burn(&s, &plain), 0);
        assert_eq!(s.card(&plain).damage, 0);

        let mut s = board(
            "r3540003-both",
            json!({ "field": [BOTH] }),
            json!({ "field": [PLAIN] }),
        );
        let (both, plain) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&both, &plain);
        assert_eq!(burn(&s, &plain), 4);
        assert_eq!(toxic(&s, &plain), 0);
    }

    #[test]
    fn r3540003_a_shield_or_indestructible_takes_the_hit_whole() {
        let mut s = board(
            "r3540003-shield",
            json!({ "field": [VOLCANIC] }),
            json!({ "field": [SHIELDED] }),
        );
        let (volcanic, shielded) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&volcanic, &shielded);
        assert_eq!(s.card(&shielded).divine_shield_spent, Some(true));
        assert_eq!(burn(&s, &shielded), 0);

        let mut s = board(
            "r3540003-stone",
            json!({ "field": [VOLCANIC] }),
            json!({ "field": [STONE] }),
        );
        let (volcanic, stone) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(&volcanic, &stone);
        assert_eq!(burn(&s, &stone), 0);
        assert_eq!(s.card(&stone).damage, 0);
    }
}

mod r3540004_vanilla {
    use super::*;

    #[test]
    fn r3540004_a_vanilla_clears_both_and_a_nerf_keeps_them() {
        let mut s = board(
            "r3540004-clear",
            json!({ "hand": [NERF, VANILLA] }),
            json!({ "field": [with(PLAIN, json!({ "burn": 5, "toxic": 2 }))] }),
        );
        let target = id_at(&s, P2, 1);
        s.play(NERF, pick(&target));
        assert_eq!(burn(&s, &target), 5);
        assert_eq!(toxic(&s, &target), 2);

        s.play(VANILLA, pick(&target));
        assert_eq!(burn(&s, &target), 0);
        assert_eq!(toxic(&s, &target), 0);
        let cleared: Vec<(CounterKind, i32)> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CounterChanged {
                    counter,
                    value,
                    placed,
                    ..
                } => {
                    assert_eq!(*placed, None, "a clearing is no gain");
                    Some((*counter, *value))
                }
                _ => None,
            })
            .collect();
        assert_eq!(cleared, vec![(CounterKind::Burn, 0), (CounterKind::Toxic, 0)]);
    }
}

mod r3540005_view {
    use super::*;

    #[test]
    fn r3540005_both_seats_see_the_counters() {
        let mut s = board(
            "r3540005-view",
            json!({ "field": [PLAIN] }),
            json!({ "field": [with(PLAIN, json!({ "burn": 5, "toxic": 2 }))] }),
        );
        s.state_mut().players[P2].hero_burn = Some(6);
        s.state_mut().players[P2].hero_toxic = Some(1);
        for viewer in [P1, P2] {
            let view = s.view(viewer);
            let side = if viewer == P2 { &view.you } else { &view.opponent };
            let unit = side
                .units
                .iter()
                .flatten()
                .next()
                .unwrap_or_else(|| panic!("{viewer:?} sees p2's unit"));
            assert_eq!(unit.counters.burn, Some(5));
            assert_eq!(unit.counters.toxic, Some(2));
            assert_eq!(side.hero.burn, Some(6));
            assert_eq!(side.hero.toxic, Some(1));
            let own = if viewer == P2 { &view.opponent } else { &view.you };
            assert_eq!(own.hero.burn, None, "p1 has none, so its hero shows none");
            assert_eq!(own.hero.toxic, None);
        }
    }
}
