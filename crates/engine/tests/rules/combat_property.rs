//! The M2 gate (BUILD.md): 1,000 random combats between random keyword combinations never produce
//! negative health, never leave a unit at health ≤ 0 on the field unless it is Indestructible with max
//! health above 0 (R69), and never emit a `damage` event on an Indestructible target.
//! R69: an Indestructible unit at 0 or less health stays while its max health is above 0, and is
//! collected like any other once its max health falls to 0 or less. R46: a would-destroy on it
//! switches it to Attack Position and takes its Taunt away for the turn.
//!
//! Case `i` draws from the seed `m2-combat-<i>`, so a violation reproduces from that seed alone
//! (§10.7, §9.3). It pairs two keyword fixtures with zero to two granted keywords each, flips radiant
//! and position, adds earlier damage or drags max health to 0, aims at a unit or the hero, and enters
//! through `declareAttack` when §4.2 allows it or `forceAttack` when it does not (R53), then runs the
//! §4.5 state check; step 2 stops it the moment a hero dies, so a game that is over may keep a body.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{
    armoured, big_body, big_dfender, charger, cleaver, deft_duelist, first_striker, indestructible,
    lifestealer, moths, pacifist, plain, poisonous, rusher, shielded, spikey_pillow, stacker, taunter,
    trample_lifesteal, trampler, zero_attack,
};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

/// The gate's number.
const COMBATS: i32 = 1000;

/// Every keyword fixture of §6.1, as the bodies a random combat is fought between.
fn bodies() -> Vec<CardDef> {
    vec![
        plain.clone(),
        big_body.clone(),
        zero_attack.clone(),
        taunter.clone(),
        rusher.clone(),
        charger.clone(),
        first_striker.clone(),
        shielded.clone(),
        armoured.clone(),
        indestructible.clone(),
        poisonous.clone(),
        lifestealer.clone(),
        trampler.clone(),
        trample_lifesteal.clone(),
        cleaver.clone(),
        pacifist.clone(),
        stacker.clone(),
        moths.clone(),
        big_dfender.clone(),
        deft_duelist.clone(),
        spikey_pillow.clone(),
    ]
}

/// The grants that make the combinations: R21's pool plus the three keywords R21 leaves out but
/// §6.1 defines for a unit in combat (Indestructible, a bigger Armor, "Can't attack").
fn grants() -> Vec<Keyword> {
    vec![
        Keyword::Taunt,
        Keyword::Rush,
        Keyword::Charge,
        Keyword::FirstStrike,
        Keyword::Poisonous,
        Keyword::Lifesteal,
        Keyword::Reborn,
        Keyword::DivineShield,
        Keyword::Trample,
        Keyword::Cleave,
        Keyword::Armor { n: 1 },
        Keyword::Armor { n: 3 },
        Keyword::Indestructible,
        Keyword::CantAttack,
    ]
}

struct Violation {
    seed: String,
    invariant: String,
    detail: String,
}

#[derive(Default)]
struct Tally {
    combats: i32,
    damage_events: i32,
    deaths: i32,
    hero_targets: i32,
    forced: i32,
    radiant_sides: i32,
    /// R69's second sentence: left on the field at 0 or less health, max health above 0.
    r69_survivors: i32,
    /// R69's first sentence: collected because its max health fell to 0 or less.
    r69_collected: i32,
    /// Tallied and never asserted on.
    #[allow(dead_code)]
    games_over: i32,
}

fn case_seed(index: i32) -> String {
    format!("m2-combat-{index}")
}

fn grants_for(rng: &mut Rng) -> Vec<Keyword> {
    let pool = grants();
    let count = rng.int(3); // 0, 1 or 2
    let mut picked: Vec<Keyword> = Vec::new();
    for _ in 0..count {
        if let Some(keyword) = rng.pick(&pool) {
            picked.push(keyword.clone());
        }
    }
    picked
}

/// The instance as the state holds it now.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} is in no zone"))
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// Every unit the board holds, the cards dormant under a Stack included (§3.2).
fn units_on_field(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let mut found: Vec<String> = active_units_of(state, player)
        .iter()
        .map(|card| card.id.clone())
        .collect();
    found.extend(dormant_units_of(state, player).iter().map(|card| card.id.clone()));
    found.iter().map(|id| live(state, id)).collect()
}

fn is_indestructible(state: &GameState, unit: &CardInstance) -> bool {
    has_keyword(&unit_view(state, unit).keywords, KeywordKind::Indestructible)
}

/// Dress one unit up: radiant face, granted keywords, position, earlier damage, R69's max health.
fn dress(state: &mut GameState, unit_id: &str, rng: &mut Rng) {
    let keywords = grants_for(rng);
    let turn = state.turn;
    {
        let unit = find_instance_mut(state, unit_id).expect("dressed unit");
        unit.granted_keywords.extend(keywords);
        if rng.chance(0.25) {
            unit.position = Some(Position::Def);
        }
        if rng.chance(0.2) {
            unit.summoned_turn = Some(turn);
        }
    }

    let view = unit_view(state, &live(state, unit_id));
    if rng.chance(0.1) {
        // R69's first sentence: max health dragged to 0 or less, with no destroy effect involved.
        find_instance_mut(state, unit_id)
            .expect("dressed unit")
            .buffs
            .health -= view.max_health;
    } else if rng.chance(0.5) && view.max_health > 1 {
        // Earlier damage, the range running past max health on purpose: R69's second sentence (an
        // Indestructible unit at 0 or less health, max health above 0, stays) is reachable no other
        // way, since §4.4 step 4 stops an Indestructible unit taking damage. Others are collected (§4.5).
        let earlier = rng.int(view.max_health + 3);
        find_instance_mut(state, unit_id).expect("dressed unit").damage = earlier;
    }
}

fn describe_unit(state: &GameState, unit: &CardInstance) -> String {
    let view = unit_view(state, unit);
    let kinds: Vec<&str> = view.keywords.iter().map(|k| k.kind().as_str()).collect();
    format!(
        "{}{} {} health {}/{} damage {} keywords [{}]",
        unit.def_id,
        if unit.radiant { "(r)" } else { "" },
        unit.id,
        view.health,
        view.max_health,
        unit.damage,
        kinds.join(",")
    )
}

fn run_one_combat(index: i32, tally: &mut Tally) -> Vec<Violation> {
    let seed = case_seed(index);
    let mut rng = Rng::new(&seed, 0);
    let mut state = new_game(&seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;

    let bodies = bodies();
    let attacker_def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
    let defender_def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
    let attacker_radiant = rng.chance(0.4);
    let defender_radiant = rng.chance(0.4);
    if attacker_radiant {
        tally.radiant_sides += 1;
    }
    if defender_radiant {
        tally.radiant_sides += 1;
    }

    // Lane 3 on each side, so a Cleave has a neighbour on either hand (§4.4 step 10, §3.1).
    let attacker = put(
        &mut state,
        &attacker_def.id,
        slot(P1, Row::Units, 3),
        json!({ "radiant": attacker_radiant }),
    );
    let defender = put(
        &mut state,
        &defender_def.id,
        slot(P2, Row::Units, 3),
        json!({ "radiant": defender_radiant }),
    );
    dress(&mut state, &attacker.id, &mut rng);
    dress(&mut state, &defender.id, &mut rng);

    for lane in [2, 4] {
        if !rng.chance(0.5) {
            continue;
        }
        let neighbour_def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
        let radiant = rng.chance(0.3);
        let neighbour = put(
            &mut state,
            &neighbour_def.id,
            slot(P2, Row::Units, lane),
            json!({ "radiant": radiant }),
        );
        dress(&mut state, &neighbour.id, &mut rng);
    }
    if rng.chance(0.3) {
        let ally_def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
        let radiant = rng.chance(0.3);
        let ally = put(
            &mut state,
            &ally_def.id,
            slot(P1, Row::Units, 2),
            json!({ "radiant": radiant }),
        );
        dress(&mut state, &ally.id, &mut rng);
    }

    state.players[P1].hero.health = 1 + rng.int(HERO_HEALTH);
    state.players[P2].hero.health = 1 + rng.int(HERO_HEALTH);
    state.players[P1].hero.armor = rng.int(3);
    state.players[P2].hero.armor = rng.int(3);

    let at_hero = rng.chance(0.25);
    let attacker = live(&state, &attacker.id);
    let defender = live(&state, &defender.id);
    let target: AttackTarget = if at_hero {
        AttackTarget::Hero { player: P2 }
    } else {
        AttackTarget::Unit {
            instance: defender.clone(),
        }
    };
    if at_hero {
        tally.hero_targets += 1;
    }

    // Every unit that is Indestructible as the combat starts: nothing here grants or removes the
    // keyword mid-combat, so this is the set §4.4 step 4 will be asked about on every hit.
    let indestructible_ids: IndexSet<String> = [P1, P2]
        .into_iter()
        .flat_map(|player| units_on_field(&state, player))
        .filter(|unit| is_indestructible(&state, unit))
        .map(|unit| unit.id)
        .collect();

    let mut events: Vec<GameEvent> = Vec::new();
    let declared = rng.chance(0.5) && can_attack(&state, &attacker, &target);
    {
        let mut sink_rng = Rng::new(&state.seed, state.rng_cursor);
        let mut sink = EngineSink::new(&mut state, &mut events, &mut sink_rng);
        if declared {
            let _ = declare_attack(&mut sink, &attacker, &target);
        } else {
            force_attack(&mut sink, &attacker, &target);
            tally.forced += 1;
        }
        // The gate's "and the state check": idempotent, so running it again must find nothing to do.
        state_check(&mut sink);
    }

    tally.combats += 1;
    tally.damage_events += events_of_type(&events, GameEventType::Damage).len() as i32;
    tally.deaths += events_of_type(&events, GameEventType::Destroyed).len() as i32;
    if state.result.is_some() {
        tally.games_over += 1;
    }

    let mut out: Vec<Violation> = Vec::new();
    let mut fail = |invariant: &str, detail: String| {
        out.push(Violation {
            seed: seed.clone(),
            invariant: invariant.to_string(),
            detail,
        });
    };

    // Invariant 1, "never produce negative health": a hero below 0 is how a hero dies (§4.5 step 2),
    // so no *live* game holds one; and no instance carries negative damage, the counter every health
    // on the board is computed from (§10.4 layer 6).
    for player in [P1, P2] {
        let side = &state.players[player];
        if state.result.is_none() && side.hero.health <= 0 {
            fail(
                "negative health",
                format!(
                    "{player}'s hero is at {} with the game still running",
                    side.hero.health
                ),
            );
        }
        let mut everywhere = units_on_field(&state, player);
        everywhere.extend(side.hand.iter().cloned());
        everywhere.extend(side.library.iter().cloned());
        everywhere.extend(side.graveyard.iter().cloned());
        everywhere.extend(side.exile.iter().cloned());
        for card in &everywhere {
            if card.damage < 0 {
                fail(
                    "negative health",
                    format!("{} {} carries damage {}", card.def_id, card.id, card.damage),
                );
            }
        }
        // A unit left on the field below 0 health is R69's exception and nothing else.
        for unit in units_on_field(&state, player) {
            let view = unit_view(&state, &unit);
            if view.health >= 0 {
                continue;
            }
            let exempt = has_keyword(&view.keywords, KeywordKind::Indestructible) && view.max_health > 0;
            if !exempt && state.result.is_none() {
                fail(
                    "negative health",
                    format!(
                        "{player} lane unit below 0 health: {}",
                        describe_unit(&state, &unit)
                    ),
                );
            }
        }
    }

    // Invariant 2: no unit at health ≤ 0 on the field, R69's exception aside.
    for player in [P1, P2] {
        for unit in units_on_field(&state, player) {
            let view = unit_view(&state, &unit);
            if view.health > 0 {
                continue;
            }
            let is_indestructible_now = has_keyword(&view.keywords, KeywordKind::Indestructible);
            if is_indestructible_now && view.max_health > 0 {
                tally.r69_survivors += 1;
                continue;
            }
            // §4.5 step 2: the check stops at the hero check, so a finished game may keep a body.
            if state.result.is_some() {
                continue;
            }
            fail(
                "dead unit left on the field",
                format!("{player}: {}", describe_unit(&state, &unit)),
            );
        }
    }
    for destroyed in events_of_type(&events, GameEventType::Destroyed).iter() {
        let GameEvent::Destroyed { instance_id, .. } = destroyed else {
            panic!("not a destroyed event");
        };
        let buried = [P1, P2]
            .into_iter()
            .flat_map(|player| state.players[player].graveyard.iter())
            .any(|card| card.id == *instance_id);
        if buried && indestructible_ids.contains(instance_id.as_str()) {
            tally.r69_collected += 1;
        }
    }

    // Invariant 3: no `damage` event on an Indestructible target (§4.4 step 4).
    for hit in events_of_type(&events, GameEventType::Damage).iter() {
        let GameEvent::Damage {
            source_id,
            target_id,
            amount,
            ..
        } = hit
        else {
            panic!("not a damage event");
        };
        if indestructible_ids.contains(target_id.as_str()) {
            fail(
                "damage on an Indestructible target",
                format!(
                    "{amount} to {target_id} from {}",
                    source_id.as_deref().unwrap_or("nothing")
                ),
            );
        }
        // R63's zero rule: a hit reduced to 0 emits nothing at all, so no event carries 0 or less.
        if *amount <= 0 {
            fail(
                "damage event with a non-positive amount",
                format!("{amount} to {target_id}"),
            );
        }
    }

    // The attacker and the defender are either on the field or in a pile: never in both, never lost.
    for unit in [&attacker, &defender] {
        let now = find_instance(&state, &unit.id)
            .cloned()
            .unwrap_or_else(|| unit.clone());
        let on_field = is_active_on_field(&state, &now);
        let in_pile = [P1, P2].into_iter().any(|player| {
            let side = &state.players[player];
            side.graveyard
                .iter()
                .chain(side.hand.iter())
                .chain(side.exile.iter())
                .any(|card| card.id == unit.id)
        });
        if on_field && in_pile {
            fail("instance in two zones", describe_unit(&state, &now));
        }
    }

    out
}

/// M2 gate: 1,000 random combats (BUILD M2, §4.3, §4.4, §4.5).
mod m2_gate_1_000_random_combats_build_m2_s4_3_s4_4_s4_5 {
    use super::*;

    #[test]
    fn r69_m2_gate_1_000_random_combats_never_leave_negative_health_never_leave_a_dead_unit_on_the_field_and_never_damage_an_indestructible_target()
     {
        let mut tally = Tally::default();

        let mut violations: Vec<Violation> = Vec::new();
        for index in 0..COMBATS {
            violations.extend(run_one_combat(index, &mut tally));
            // Each failure names its seed; stop at a handful so the report stays readable.
            if violations.len() >= 5 {
                break;
            }
        }

        let report: Vec<String> = violations
            .iter()
            .map(|entry| format!("{} [{}] {}", entry.seed, entry.invariant, entry.detail))
            .collect();
        assert_eq!(
            report,
            Vec::<String>::new(),
            "M2 gate invariant violations, each reproducible from its seed"
        );
        assert_eq!(tally.combats, COMBATS);
    }

    #[test]
    fn m2_gate_the_1_000_cases_actually_exercise_the_rules_they_are_meant_to_non_vacuity() {
        let mut tally = Tally::default();
        for index in 0..COMBATS {
            run_one_combat(index, &mut tally);
        }

        // Damage landed, units died, heroes were attacked, both entry points of §4.2 were used, and
        // radiant faces were fought with.
        assert!(tally.damage_events > COMBATS);
        assert!(tally.deaths > COMBATS / 4);
        assert!(tally.hero_targets > COMBATS / 8);
        assert!(tally.forced > COMBATS / 8);
        assert!(tally.forced < COMBATS);
        assert!(tally.radiant_sides > COMBATS / 4);

        // And both halves of R69 happened, so invariant 2's exception is not a dead branch.
        assert!(tally.r69_survivors > 0);
        assert!(tally.r69_collected > 0);
    }
}

/// R69 and R46: the exception invariant 2 carves out.
mod r69_and_r46_the_exception_invariant_2_carves_out {
    use super::*;

    #[test]
    fn r69_leaves_an_indestructible_unit_at_0_or_less_health_on_the_field_while_its_max_health_is_above_0() {
        let mut state = new_game("r69-stays", None);
        state.turn = 4;
        let unit = put(&mut state, &indestructible.id, slot(P1, Row::Units, 1), json!({})); // 4/4 Indestructible
        // "Damage taken before it became Indestructible": 6 on a 4-health body.
        find_instance_mut(&mut state, &unit.id)
            .expect("on the field")
            .damage = 6;

        let view = unit_view(&state, &live(&state, &unit.id));
        assert_eq!(view.health, -2);
        assert_eq!(view.max_health, 4);

        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        state_check(&mut EngineSink::new(&mut state, &mut events, &mut rng));
        let active: Vec<String> = active_units_of(&state, P1)
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(active, vec![unit.id.clone()]);
        assert!(state.players[P1].graveyard.is_empty());
        assert_eq!(state.counters.destroyed, 0);
    }

    #[test]
    fn r69_collects_an_indestructible_unit_whose_max_health_falls_to_0_or_less_like_any_other_unit() {
        let mut state = new_game("r69-collected", None);
        state.turn = 4;
        let unit = put(&mut state, &indestructible.id, slot(P1, Row::Units, 1), json!({})); // 4/4 Indestructible
        // Suppressive Aura's effect, as a layer-4 buff: max health to 0, with no destroy involved.
        find_instance_mut(&mut state, &unit.id)
            .expect("on the field")
            .buffs
            .health = -4;
        assert_eq!(unit_view(&state, &live(&state, &unit.id)).max_health, 0);

        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        state_check(&mut EngineSink::new(&mut state, &mut events, &mut rng));

        assert!(active_units_of(&state, P1).is_empty());
        assert_eq!(ids(&state.players[P1].graveyard), vec![unit.id.clone()]);
        let destroyed: Vec<String> = events_of_type(&events, GameEventType::Destroyed)
            .iter()
            .map(|event| match event {
                GameEvent::Destroyed { instance_id, .. } => instance_id.clone(),
                other => panic!("not a destroyed event: {other:?}"),
            })
            .collect();
        assert_eq!(destroyed, vec![unit.id.clone()]);
        // "Counts toward Ceaseless Void's destroyed counter" (R69, R55).
        assert_eq!(state.counters.destroyed, 1);
    }

    #[test]
    fn r46_a_would_destroy_on_an_indestructible_unit_switches_it_to_attack_position_and_takes_its_taunt_for_the_turn()
     {
        let mut state = new_game("r46", None);
        state.turn = 4;
        let unit = put(&mut state, &indestructible.id, slot(P1, Row::Units, 1), json!({}));
        {
            let held = find_instance_mut(&mut state, &unit.id).expect("on the field");
            held.granted_keywords.push(Keyword::Taunt);
            held.position = Some(Position::Def);
            held.marked_destroyed = Some(true);
        }

        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        state_check(&mut EngineSink::new(&mut state, &mut events, &mut rng));

        let active: Vec<String> = active_units_of(&state, P1)
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(active, vec![unit.id.clone()]);
        let now = live(&state, &unit.id);
        assert_eq!(now.marked_destroyed, Some(false));
        assert_eq!(now.position, Some(Position::Atk));
        assert_eq!(now.taunt_suppressed_turn, Some(state.turn));
        assert!(
            !unit_view(&state, &now)
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Taunt)
        );
        assert_eq!(
            serde_json::to_value(events_of_type(&events, GameEventType::PositionSwitched))
                .expect("serialises"),
            json!([{ "type": "positionSwitched", "instanceId": unit.id, "position": "ATK" }])
        );

        // R347: next turn it is still Indestructible, so the granted Taunt stays off; stripped of its
        // Indestructible (a Vanilla), the stamp no longer names this turn and the grant is back (§10.4).
        state.turn += 1;
        assert!(
            !unit_view(&state, &live(&state, &unit.id))
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Taunt)
        );
        find_instance_mut(&mut state, &unit.id)
            .expect("on the field")
            .vanilla = true;
        assert!(
            unit_view(&state, &live(&state, &unit.id))
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Taunt)
        );
    }

    #[test]
    fn s4_4_step_4_an_indestructible_defender_takes_no_damage_and_emits_no_damage_event() {
        let mut state = new_game("indestructible-no-damage", None);
        state.turn = 4;
        state.active = P1;
        state.phase = Phase::Main;
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        let defender = put(&mut state, &indestructible.id, slot(P2, Row::Units, 1), json!({})); // 4/4 Indestructible

        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let _ = declare_attack(
                &mut sink,
                &attacker,
                &AttackTarget::Unit {
                    instance: defender.clone(),
                },
            );
        }

        let hits: Vec<String> = events_of_type(&events, GameEventType::Damage)
            .iter()
            .map(|hit| match hit {
                GameEvent::Damage { target_id, .. } => target_id.clone(),
                other => panic!("not a damage event: {other:?}"),
            })
            .collect();
        assert_eq!(hits, vec![attacker.id.clone()]); // only the strike back landed
        assert_eq!(live(&state, &defender.id).damage, 0);
        assert_eq!(live(&state, &attacker.id).damage, 4);
        assert_eq!(UNIT_ZONES, 5);
    }
}
