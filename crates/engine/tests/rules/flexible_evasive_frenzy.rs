//! Flexible, Evasive and Frenzy (issue #3543, R3543000–R3543003): a Flexible Unit plays into
//! either row and is a normal Unit in the backrow, Flying has Flexible innately and shows Flying
//! alone, a Flexible Field Spell with stats plays into a unit zone as a Unit while one without keeps
//! to the backrow, Evasive bars every attack and implies Can't Defend, and Frenzy's kill frees one
//! more attack. Proved on the fixtures of `fixtures/flexible_evasive_frenzy.rs`, with no card yet.

use jackioh_engine::effects::{TuneDirection, tune_once};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::fixtures::flexible_evasive_frenzy::*;
use super::fixtures::harness::{events_of_type, live, run_effect, run_on_sink as run};

/// p1's main phase with `p1` and `p2` as given, plain cards to draw, and no turn that ends by
/// itself, so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_fef_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([GRUNT, GRUNT, GRUNT, GRUNT]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

/// Every zone a play of `id` is offered, as (row, lane).
fn play_zones(s: &Scenario, id: &str) -> Vec<ZoneChoice> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id, zone, ..
            } if instance_id == id => zone,
            _ => None,
        })
        .collect()
}

/// Whether `legal_actions` offers `attacker` an attack on `target_id` for `player`.
fn offers(s: &Scenario, player: PlayerId, attacker: &str, target_id: &str) -> bool {
    legal_actions(s.state(), player).iter().any(|action| {
        matches!(
            action,
            ActionBody::Attack { attacker_id, target_id: offered }
                if attacker_id == attacker && offered == target_id
        )
    })
}

/// Declare `attacker`'s attack on the unit `target`, answering the refusal's text or `None`.
fn declare(state: &mut GameState, attacker: &str, target: &str) -> Option<String> {
    let attacker = live(state, attacker);
    let target = AttackTarget::Unit {
        instance: live(state, target),
    };
    run(state, |sink| {
        declare_attack(sink, &attacker, &target)
            .err()
            .map(|error| error.message)
    })
    .0
}

/// `force_attack` on the unit `target`, with both read live.
fn force(state: &mut GameState, attacker: &str, target: &str) -> Vec<GameEvent> {
    let attacker = live(state, attacker);
    let target = AttackTarget::Unit {
        instance: live(state, target),
    };
    run(state, |sink| force_attack(sink, &attacker, &target)).1
}

/// The keyword kinds the card has now, its tuning included.
fn kinds_of(s: &Scenario, id: &str) -> Vec<KeywordKind> {
    card_keywords(s.state(), s.card(id))
        .iter()
        .map(Keyword::kind)
        .collect()
}

/// One Buff or Nerf on `id`, drawn from the match rng at the state's cursor.
fn tune(s: &mut Scenario, id: &str, direction: TuneDirection) {
    let card = s.card(id).clone();
    let state = s.state_mut();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, HookOptions::default());
        tune_once(&mut ctx, &card, direction, true);
    }
    state.rng_cursor = rng.cursor();
}

fn damage_on(state: &GameState, id: &str) -> i32 {
    find_instance(state, id).map_or(0, |card| card.damage)
}

fn field_unit(def: &str, lane: i32) -> Value {
    json!({ "def": def, "lane": lane })
}

fn backrow_unit(def: &str, lane: i32) -> Value {
    json!({ "def": def, "lane": lane, "row": "backrow" })
}

mod r3543000_flexible {
    use super::*;

    #[test]
    fn r3543000_a_flexible_unit_is_offered_both_rows_and_plays_into_the_backrow() {
        let mut s = board("r3543000-offered", json!({ "hand": [FLEXER] }), json!({}));
        let id = s.card(FLEXER).id.clone();
        let zones = play_zones(&s, &id);
        for lane in 1..=5 {
            assert!(
                zones.contains(&ZoneChoice {
                    row: Row::Units,
                    lane
                }),
                "unit lane {lane} offered"
            );
            assert!(
                zones.contains(&ZoneChoice {
                    row: Row::Backrow,
                    lane
                }),
                "backrow lane {lane} offered"
            );
        }
        s.play(FLEXER, json!({ "zone": 2, "row": "backrow" }));
        let stood = s.card(id.as_str()).clone();
        assert!(
            matches!(
                stood.zone,
                Zone::Field {
                    row: Row::Backrow,
                    lane: 2,
                    ..
                }
            ),
            "the flexer stands in backrow lane 2"
        );
        let view = s.view(P1);
        let entry = view.you.backrow[1].as_ref().expect("a backrow entry in lane 2");
        let BackrowView::Public(entry) = entry else {
            panic!("the backrow flexer is public");
        };
        let unit = entry.unit.as_ref().expect("the entry carries its unit");
        assert_eq!(unit.instance_id, id);
    }

    #[test]
    fn r3543000_a_flexible_unit_in_the_backrow_attacks_is_attacked_and_defends() {
        let mut s = board(
            "r3543000-fights",
            json!({ "field": [backrow_unit(FLEXER, 2)] }),
            json!({ "field": [field_unit(FRAIL, 1), field_unit(FRAIL, 2)] }),
        );
        let flexer = s
            .backrow(P1, 2)
            .map(|unit| unit.id)
            .expect("no flexer in backrow lane 2");
        // It attacks: the 3/4 kills the 1/1 and lives with 1 damage.
        let prey = s.unit(P2, 1).expect("no prey in lane 1").id;
        s.attack(flexer.as_str(), prey.as_str());
        assert!(s.unit(P2, 1).is_none(), "the frail died");
        assert_eq!(damage_on(s.state(), &flexer), 1);
        s.end_turn();
        s.end_turn();
        // Next turn it attacks the hero.
        s.attack(flexer.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 3);
        // A grunt with no Flying may attack it there.
        s.end_turn();
        let hunter = s.unit(P2, 2).expect("no hunter in lane 2").id;
        assert!(
            offers(&s, P2, &hunter, &flexer),
            "a plain Unit may attack the backrow flexer"
        );
        s.attack(hunter.as_str(), flexer.as_str());
        assert_eq!(damage_on(s.state(), &flexer), 2);
        // And it switches to Defense as a normal Unit does.
        s.end_turn();
        s.switch_position(flexer.as_str());
        assert_eq!(
            s.card(flexer.as_str()).position,
            Some(Position::Def),
            "it defends"
        );
    }

    #[test]
    fn r3543000_a_summon_into_a_full_unit_row_stands_it_in_the_backrow() {
        let mut s = board(
            "r3543000-summon",
            json!({
                "field": [
                    field_unit(GRUNT, 1),
                    field_unit(GRUNT, 2),
                    field_unit(GRUNT, 3),
                    field_unit(GRUNT, 4),
                    field_unit(GRUNT, 5),
                ]
            }),
            json!({}),
        );
        let events = run_effect(
            s.state_mut(),
            effects::summon(json_as(json!({ "defId": FLEXER }))),
            P1,
        );
        let placed: Vec<(Row, i32)> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { row, lane, .. } => Some((*row, *lane)),
                _ => None,
            })
            .collect();
        assert_eq!(placed, vec![(Row::Backrow, 1)]);
        let flexer = s.backrow(P1, 1).expect("no card in backrow lane 1");
        assert_eq!(flexer.def_id, FLEXER);
        assert!(
            active_units_of(s.state(), P1)
                .iter()
                .any(|unit| unit.id == flexer.id),
            "a Unit for every rule"
        );
    }

    #[test]
    fn r3543000_flying_has_flexible_innately_and_shows_flying_alone() {
        let mut s = board(
            "r3543000-innate",
            json!({ "hand": [FLYER] }),
            json!({ "field": [field_unit(GRUNT, 1)] }),
        );
        let id = s.card(FLYER).id.clone();
        let zones = play_zones(&s, &id);
        for lane in 1..=5 {
            assert!(
                zones.contains(&ZoneChoice {
                    row: Row::Units,
                    lane
                }),
                "unit lane {lane} offered"
            );
            assert!(
                zones.contains(&ZoneChoice {
                    row: Row::Backrow,
                    lane
                }),
                "backrow lane {lane} offered"
            );
        }
        assert!(
            jackioh_engine::layers::is_flexible(s.state(), s.card(id.as_str())),
            "a Flying Unit is Flexible"
        );
        assert!(
            !jackioh_engine::layers::unit_has(s.state(), s.card(id.as_str()), KeywordKind::Flexible),
            "but Flexible is never added to its keywords"
        );
        s.play(FLYER, json!({ "zone": 1, "row": "backrow" }));
        let view = s.view(P1);
        let entry = view.you.backrow[0].as_ref().expect("a backrow entry in lane 1");
        let BackrowView::Public(entry) = entry else {
            panic!("the backrow flyer is public");
        };
        let kinds: Vec<KeywordKind> = entry
            .unit
            .as_ref()
            .expect("the entry carries its unit")
            .keywords
            .iter()
            .map(Keyword::kind)
            .collect();
        assert!(kinds.contains(&KeywordKind::Flying), "{kinds:?}");
        assert!(!kinds.contains(&KeywordKind::Flexible), "{kinds:?}");
        // And a grunt still may not attack it there.
        let hunter = s.unit(P2, 1).expect("no hunter in lane 1").id;
        let refused = declare(s.state_mut(), &hunter, &id);
        assert!(
            refused.is_some_and(|refusal| refusal.contains("Flying or Ranged")),
            "a plain Unit cannot attack it"
        );
    }

    #[test]
    fn r3543000_a_nerf_may_take_a_printed_flexible_off() {
        let mut lost = false;
        for n in 0..200 {
            let mut s = board(
                &format!("r3543000-nerf-{n}"),
                json!({ "hand": [FLEXER] }),
                json!({}),
            );
            let id = s.card(FLEXER).id.clone();
            tune(&mut s, &id, TuneDirection::Degrade);
            if kinds_of(&s, &id).contains(&KeywordKind::Flexible) {
                continue;
            }
            lost = true;
            let zones = play_zones(&s, &id);
            assert!(!zones.is_empty(), "its unit row is still offered");
            assert!(
                zones.iter().all(|zone| zone.row == Row::Units),
                "no backrow zone is offered any more"
            );
            break;
        }
        assert!(lost, "some Nerf took the printed Flexible off");
    }
}

mod r3543001_flexible_field_spells {
    use super::*;

    #[test]
    fn r3543001_a_flexible_field_spell_with_stats_plays_into_a_unit_zone_as_a_unit() {
        let mut s = board(
            "r3543001-field",
            json!({ "hand": [FLEX_FIELD] }),
            json!({ "field": [field_unit(GRUNT, 1)] }),
        );
        let id = s.card(FLEX_FIELD).id.clone();
        let zones = play_zones(&s, &id);
        for lane in 1..=5 {
            assert!(
                zones.contains(&ZoneChoice {
                    row: Row::Units,
                    lane
                }),
                "unit lane {lane} offered"
            );
            assert!(
                zones.contains(&ZoneChoice {
                    row: Row::Backrow,
                    lane
                }),
                "backrow lane {lane} offered"
            );
        }
        s.play(FLEX_FIELD, json!({ "zone": 1, "row": "units" }));
        let played = s.card(id.as_str()).clone();
        assert!(
            matches!(
                played.zone,
                Zone::Field {
                    row: Row::Units,
                    lane: 1,
                    ..
                }
            ),
            "it lands in unit lane 1"
        );
        assert_eq!(played.position, Some(Position::Atk), "in Attack Position");
        assert_eq!(
            jackioh_engine::faces::card_type_of(s.state(), &played),
            CardType::Unit,
            "a Unit there"
        );
        // Its text keeps working there: the end of its controller's turn hits the enemy hero.
        s.end_turn();
        s.expect_health(P2, HERO_HEALTH - 1);
        // Next turn it attacks as a normal Unit.
        s.end_turn();
        s.attack(id.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 1 - 2);
        // And a grunt may attack and kill it there.
        s.end_turn();
        let hunter = s.unit(P2, 1).expect("no hunter in lane 1").id;
        s.attack(hunter.as_str(), id.as_str());
        assert!(s.unit(P1, 1).is_none(), "the field spell died as a Unit");
    }

    #[test]
    fn r3543001_a_flexible_field_spell_without_stats_keeps_to_its_backrow() {
        let mut s = board("r3543001-bare", json!({ "hand": [BARE_FIELD] }), json!({}));
        let id = s.card(BARE_FIELD).id.clone();
        let zones = play_zones(&s, &id);
        assert!(!zones.is_empty(), "its backrow is still offered");
        assert!(
            zones.iter().all(|zone| zone.row == Row::Backrow),
            "no unit zone is offered"
        );
        s.expect_refused_with(
            |s| s.play(BARE_FIELD, json!({ "zone": 1, "row": "units" })),
            "goes in the backrow row",
        );
    }
}

mod r3543002_evasive {
    use super::*;

    #[test]
    fn r3543002_no_attack_may_target_an_evasive_unit_declared_or_forced() {
        let mut s = board(
            "r3543002-target",
            json!({ "field": [field_unit(GRUNT, 1)] }),
            json!({ "field": [field_unit(EVADER, 1)] }),
        );
        let hunter = s.unit(P1, 1).expect("no hunter in lane 1").id;
        let evader = s.unit(P2, 1).expect("no evader in lane 1").id;
        let refused = declare(s.state_mut(), &hunter, &evader);
        assert!(
            refused.is_some_and(|refusal| refusal.contains("cannot be attacked")),
            "declared attacks are refused"
        );
        assert!(!offers(&s, P1, &hunter, &evader), "no attack on it is offered");
        let events = force(s.state_mut(), &hunter, &evader);
        assert!(
            events_of_type(&events, GameEventType::AttackDeclared).is_empty(),
            "a forced attack on it does not happen"
        );
        assert_eq!(damage_on(s.state(), &evader), 0, "it takes nothing");
        // A Spell's damage still reaches it: Evasive is no immunity.
        run_effect(
            s.state_mut(),
            effects::damage(json_as(
                json!({ "to": { "of": "instance", "instanceId": evader }, "amount": 2 }),
            )),
            P1,
        );
        assert_eq!(damage_on(s.state(), &evader), 2, "a Spell's hit lands");
    }

    #[test]
    fn r3543002_an_evasive_taunt_binds_no_attacker() {
        let s = board(
            "r3543002-taunt",
            json!({ "field": [field_unit(GRUNT, 1)] }),
            json!({ "field": [field_unit(EVASIVE_TAUNT, 1), field_unit(GRUNT, 2)] }),
        );
        let hunter = s.unit(P1, 1).expect("no hunter in lane 1").id;
        let plain = s.unit(P2, 2).expect("no plain unit in lane 2").id;
        assert!(offers(&s, P1, &hunter, &plain), "the Evasive Taunt binds nobody");
    }

    #[test]
    fn r3543002_an_evasive_unit_is_never_in_defense_position() {
        let mut s = board(
            "r3543002-defense",
            json!({ "field": [field_unit(EVADER, 1)] }),
            json!({}),
        );
        let id = s.unit(P1, 1).expect("no evader in lane 1").id;
        let switches: Vec<String> = legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::SwitchPosition { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert!(!switches.contains(&id), "no switch to Defense is offered");
        s.expect_refused_with(|s| s.switch_position(id.as_str()), "Defense Position");
        // Set to Defense, the state check switches it back.
        if let Some(card) = find_instance_mut(s.state_mut(), &id) {
            card.position = Some(Position::Def);
        }
        s.end_turn();
        assert_eq!(
            s.card(id.as_str()).position,
            Some(Position::Atk),
            "the check switches it back"
        );
        // The view lists Evasive alone, never an added Can't Defend.
        let kinds: Vec<KeywordKind> = card_keywords(s.state(), s.card(id.as_str()))
            .iter()
            .map(Keyword::kind)
            .collect();
        assert!(kinds.contains(&KeywordKind::Evasive), "{kinds:?}");
        assert!(!kinds.contains(&KeywordKind::CantDefend), "{kinds:?}");
    }

    #[test]
    fn r3543002_a_nerf_may_take_evasive_off_and_with_it_the_implied_cant_defend() {
        let mut lost = false;
        for n in 0..200 {
            let mut s = board(
                &format!("r3543002-nerf-{n}"),
                json!({ "field": [field_unit(EVADER, 1)] }),
                json!({}),
            );
            let id = s.unit(P1, 1).expect("no evader in lane 1").id;
            tune(&mut s, &id, TuneDirection::Degrade);
            if kinds_of(&s, &id).contains(&KeywordKind::Evasive) {
                continue;
            }
            lost = true;
            s.switch_position(id.as_str());
            assert_eq!(
                s.card(id.as_str()).position,
                Some(Position::Def),
                "with Evasive gone it defends again"
            );
            break;
        }
        assert!(lost, "some Nerf took Evasive off");
    }
}

mod r3543003_frenzy {
    use super::*;

    #[test]
    fn r3543003_a_frenzied_unit_that_kills_the_unit_it_attacked_may_attack_again() {
        let mut s = board(
            "r3543003-chain",
            json!({ "field": [field_unit(FRENZIED, 1)] }),
            json!({ "field": [field_unit(FRAIL, 1), field_unit(FRAIL, 2), field_unit(FRAIL, 3)] }),
        );
        let frenzied = s.unit(P1, 1).expect("no frenzied in lane 1").id;
        let first = s.unit(P2, 1).expect("no first frail").id;
        s.attack(frenzied.as_str(), first.as_str());
        assert!(s.unit(P2, 1).is_none(), "the first frail died");
        assert!(
            !s.card(frenzied.as_str()).exertion.attacked,
            "the kill frees a second attack"
        );
        let second = s.unit(P2, 2).expect("no second frail").id;
        assert!(offers(&s, P1, &frenzied, &second), "it is offered again");
        s.attack(frenzied.as_str(), second.as_str());
        assert!(s.unit(P2, 2).is_none(), "the second frail died");
        assert!(
            !s.card(frenzied.as_str()).exertion.attacked,
            "another kill frees a third"
        );
        let third = s.unit(P2, 3).expect("no third frail").id;
        s.attack(frenzied.as_str(), third.as_str());
        assert!(s.unit(P2, 3).is_none(), "the third frail died");
    }

    #[test]
    fn r3543003_no_kill_a_hero_hit_or_a_cleave_kill_gives_no_second_attack() {
        // The wall survives: no kill, no second attack.
        let mut s = board(
            "r3543003-wall",
            json!({ "field": [field_unit(FRENZIED, 1)] }),
            json!({ "field": [field_unit(WALL, 1)] }),
        );
        let frenzied = s.unit(P1, 1).expect("no frenzied in lane 1").id;
        let wall = s.unit(P2, 1).expect("no wall in lane 1").id;
        s.attack(frenzied.as_str(), wall.as_str());
        assert!(s.unit(P2, 1).is_some(), "the wall stands");
        assert!(
            s.card(frenzied.as_str()).exertion.attacked,
            "no kill frees nothing"
        );
        // A hit on the hero is no kill either.
        let mut hero = board(
            "r3543003-hero",
            json!({ "field": [field_unit(FRENZIED, 1)] }),
            json!({}),
        );
        let striker = hero.unit(P1, 1).expect("no striker in lane 1").id;
        hero.attack(striker.as_str(), "hero");
        assert!(
            hero.card(striker.as_str()).exertion.attacked,
            "a hero hit frees nothing"
        );
        // Cleave's kill of a neighbour is not the attacked Unit's death.
        let mut cleave = board(
            "r3543003-cleave",
            json!({ "field": [field_unit(FRENZIED_CLEAVER, 1)] }),
            json!({ "field": [field_unit(FRAIL, 1), field_unit(WALL, 2)] }),
        );
        let cleaver = cleave.unit(P1, 1).expect("no cleaver in lane 1").id;
        let target = cleave.unit(P2, 2).expect("no wall in lane 2").id;
        cleave.attack(cleaver.as_str(), target.as_str());
        assert!(cleave.unit(P2, 1).is_none(), "the neighbouring frail died");
        assert!(cleave.unit(P2, 2).is_some(), "the attacked wall stands");
        assert!(
            cleave.card(cleaver.as_str()).exertion.attacked,
            "a Cleave kill frees nothing"
        );
    }

    #[test]
    fn r3543003_a_forced_attack_s_kill_counts_and_a_dead_attacker_gets_nothing() {
        // A forced attack's kill counts: Frenzy answers forced attacks too.
        let mut s = board(
            "r3543003-forced",
            json!({ "field": [field_unit(FRENZIED, 1)] }),
            json!({ "field": [field_unit(FRAIL, 1)] }),
        );
        let frenzied = s.unit(P1, 1).expect("no frenzied in lane 1").id;
        let frail = s.unit(P2, 1).expect("no frail in lane 1").id;
        force(s.state_mut(), &frenzied, &frail);
        assert!(s.unit(P2, 1).is_none(), "the frail died");
        assert!(
            !s.card(frenzied.as_str()).exertion.attacked,
            "the forced kill frees another attack"
        );
        // An attacker that died with its target gets nothing.
        let mut last = board(
            "r3543003-mutual",
            json!({ "field": [field_unit(FRENZIED, 1)] }),
            json!({ "field": [field_unit(GRUNT, 1)] }),
        );
        let doomed = last.unit(P1, 1).expect("no doomed in lane 1").id;
        let slayer = last.unit(P2, 1).expect("no slayer in lane 1").id;
        if let Some(card) = find_instance_mut(last.state_mut(), &slayer) {
            card.buffs = AttackHealth { attack: 3, health: 0 };
        }
        last.attack(doomed.as_str(), slayer.as_str());
        assert!(last.unit(P2, 1).is_none(), "the 6-attack grunt died");
        assert!(last.unit(P1, 1).is_none(), "the frenzied died striking back");
    }

    #[test]
    fn r3543003_a_unit_that_entered_this_turn_stays_sick() {
        let mut s = board(
            "r3543003-sick",
            json!({ "hand": [FRENZIED] }),
            json!({ "field": [field_unit(FRAIL, 1), field_unit(FRAIL, 2)] }),
        );
        s.play(FRENZIED, json!({ "zone": 1 }));
        let striker = s.unit(P1, 1).expect("no striker in lane 1").id;
        if let Some(card) = find_instance_mut(s.state_mut(), &striker) {
            card.granted_keywords.push(Keyword::Rush);
        }
        let first = s.unit(P2, 1).expect("no first frail").id;
        s.attack(striker.as_str(), first.as_str());
        assert!(s.unit(P2, 1).is_none(), "the first frail died");
        assert_eq!(
            s.card(striker.as_str()).summoned_turn,
            Some(s.state().turn),
            "still summoning sick"
        );
        let second = s.unit(P2, 2).expect("no second frail").id;
        assert!(
            offers(&s, P1, &striker, &second),
            "Rush still lets it attack a Unit"
        );
        assert!(
            !offers(&s, P1, &striker, "hero-p2"),
            "never the hero on its summon turn"
        );
    }

    #[test]
    fn r3543003_a_nerf_may_take_frenzy_off() {
        let mut lost = false;
        for n in 0..200 {
            let mut s = board(
                &format!("r3543003-nerf-{n}"),
                json!({ "field": [field_unit(FRENZIED, 1)] }),
                json!({ "field": [field_unit(FRAIL, 1)] }),
            );
            let id = s.unit(P1, 1).expect("no frenzied in lane 1").id;
            tune(&mut s, &id, TuneDirection::Degrade);
            if kinds_of(&s, &id).contains(&KeywordKind::Frenzy) {
                continue;
            }
            lost = true;
            let frail = s.unit(P2, 1).expect("no frail in lane 1").id;
            s.attack(id.as_str(), frail.as_str());
            assert!(s.unit(P2, 1).is_none(), "the frail died");
            assert!(
                s.card(id.as_str()).exertion.attacked,
                "without Frenzy the kill frees nothing"
            );
            break;
        }
        assert!(lost, "some Nerf took Frenzy off");
    }
}
