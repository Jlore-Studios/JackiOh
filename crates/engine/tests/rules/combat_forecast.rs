//! The combat forecast (R3100000, SPEC §9.10): what each attack a seat may declare would do, projected
//! from the board and held to the engine's own combat. The tutorial's coach names an attack by it and does
//! no combat maths of its own (CLAUDE.md rule 7). Every case plays the forecast attack for real and reads
//! what the reducer did: the damage events, the Divine Shield lost and who is left on the field.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::combat::{
    armored, big_body, charger, first_striker, indestructible, lifestealer, plain, poisonous, rusher,
    shielded, taunter, trampler, zero_attack,
};
use crate::rules::fixtures::combat_judge::{combat_judge_game, snare};
use crate::rules::fixtures::harness::{act_ok, live, new_game, put, slot};
use crate::rules::fixtures::scripts::anti_oneshot;

/// p1's main phase on turn 4, so nothing placed with `put` is summoning sick (§4.1).
fn board(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    state.turn = 4;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

fn attack_of(attacker_id: &str, target_id: &str) -> Value {
    json!({ "type": "attack", "playerId": "p1", "attackerId": attacker_id, "targetId": target_id })
}

fn forecast_of(state: &GameState, attacker_id: &str, target_id: &str) -> Option<AttackForecast> {
    combat_forecast(state, P1)
        .attacks
        .into_iter()
        .find(|entry| entry.attacker_id == attacker_id && entry.target_id == target_id)
}

fn on_field(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
}

/// What the engine's combat did when `attacker_id` attacked `target_id`, as the forecast reports it.
#[derive(Debug, PartialEq, Eq)]
struct Played {
    dealt: i32,
    taken: i32,
    kills: bool,
    survives: bool,
    pops: bool,
}

fn play_out(state: &GameState, attacker_id: &str, target_id: &str) -> Played {
    let after = act_ok(state, attack_of(attacker_id, target_id));
    let sum_into = |id: &str| -> i32 {
        after
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    target_id, amount, ..
                } if target_id == id => Some(*amount),
                _ => None,
            })
            .sum()
    };
    let pops = after.events.iter().any(
        |event| matches!(event, GameEvent::DivineShieldLost { instance_id } if instance_id == target_id),
    );
    let kills = match target_id.strip_prefix("hero-") {
        Some(_) => after.state.result.is_some(),
        None => !on_field(&after.state, target_id),
    };
    Played {
        dealt: sum_into(target_id),
        taken: sum_into(attacker_id),
        kills,
        survives: on_field(&after.state, attacker_id),
        pops,
    }
}

fn played_as_forecast(forecast: &AttackForecast) -> Played {
    Played {
        dealt: forecast.dealt,
        taken: forecast.taken,
        kills: forecast.kills,
        survives: forecast.survives,
        pops: forecast.pops,
    }
}

fn grant(state: &mut GameState, id: &str, keyword: Keyword) {
    find_instance_mut(state, id)
        .expect("on the field")
        .granted_keywords
        .push(keyword);
}

mod r3100000_the_forecast_is_the_engine_s_own_combat {
    use super::*;

    #[test]
    fn r3100000_armor_takes_its_part_of_the_blow_and_the_strike_back() {
        let mut state = board("r3100000-armor");
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        let wall = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        grant(&mut state, &wall.id, Keyword::Armor { n: 2 });
        grant(&mut state, &striker.id, Keyword::Armor { n: 1 });

        let forecast = forecast_of(&state, &striker.id, &wall.id).expect("the attack is legal");
        // 5 less Armor 2 into the wall, 3 less Armor 1 back.
        assert_eq!((forecast.dealt, forecast.taken), (3, 2));
        assert!(forecast.kills, "3 damage destroys a 3-health unit");
        assert!(forecast.survives);
        assert!(!forecast.pops);
        assert_eq!((forecast.attacker_size, forecast.target_size), (15, 6));
        assert_eq!(
            played_as_forecast(&forecast),
            play_out(&state, &striker.id, &wall.id)
        );
    }

    #[test]
    fn r3100000_armor_that_takes_the_whole_blow_leaves_the_target_alone() {
        let mut state = board("r3100000-armor-whole");
        let striker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let wall = put(&mut state, &armored.id, slot(P2, Row::Units, 1), json!({})); // 7/7 Armor 7

        let forecast = forecast_of(&state, &striker.id, &wall.id).expect("the attack is legal");
        assert_eq!((forecast.dealt, forecast.taken), (0, 7));
        assert!(!forecast.kills);
        assert!(!forecast.survives, "7 damage destroys a 3-health unit");
        assert_eq!(
            played_as_forecast(&forecast),
            play_out(&state, &striker.id, &wall.id)
        );
    }

    #[test]
    fn r3100000_a_divine_shield_takes_the_whole_blow_and_breaks() {
        let mut state = board("r3100000-shield");
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        let shield = put(&mut state, &shielded.id, slot(P2, Row::Units, 1), json!({})); // 2/2 Divine Shield

        let forecast = forecast_of(&state, &striker.id, &shield.id).expect("the attack is legal");
        assert_eq!((forecast.dealt, forecast.taken), (0, 2));
        assert!(
            !forecast.kills,
            "the shield stops a blow that is bigger than the body"
        );
        assert!(forecast.pops);
        assert_eq!(
            played_as_forecast(&forecast),
            play_out(&state, &striker.id, &shield.id)
        );

        // The shield is gone once it broke: the next blow lands whole.
        let after = act_ok(&state, attack_of(&striker.id, &shield.id)).state;
        assert!(!unit_has(
            &after,
            &live(&after, &shield.id),
            KeywordKind::DivineShield
        ));
    }

    #[test]
    fn r3100000_a_hero_s_armor_comes_off_the_blow_and_the_forecast_ends_the_game_only_when_it_kills() {
        let mut state = board("r3100000-hero");
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        state.players[P2].hero.armor = 2;
        state.players[P2].hero.health = 3;

        let forecast = forecast_of(&state, &striker.id, "hero-p2").expect("the hero may be attacked");
        assert_eq!((forecast.dealt, forecast.taken), (3, 0));
        assert!(forecast.kills && forecast.survives && !forecast.pops);
        assert_eq!(forecast.target_size, 0);
        assert_eq!(
            played_as_forecast(&forecast),
            play_out(&state, &striker.id, "hero-p2")
        );

        state.players[P2].hero.health = 4;
        let forecast = forecast_of(&state, &striker.id, "hero-p2").expect("the hero may be attacked");
        assert!(!forecast.kills, "3 damage leaves the hero at 1");
        assert_eq!(
            played_as_forecast(&forecast),
            play_out(&state, &striker.id, "hero-p2")
        );
    }

    #[test]
    fn r3100000_the_board_is_lethal_when_each_attacker_s_one_blow_adds_up_to_the_hero_s_life() {
        let mut state = board("r3100000-lethal");
        put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3
        put(&mut state, &charger.id, slot(P1, Row::Units, 2), json!({})); // 3
        state.players[P2].hero.health = 6;
        assert!(combat_forecast(&state, P1).lethal, "3 + 3 finishes 6 life");

        state.players[P2].hero.armor = 1;
        assert!(
            !combat_forecast(&state, P1).lethal,
            "Armor 1 takes 1 off each blow: 2 + 2 leaves 2"
        );

        state.players[P2].hero.armor = 0;
        state.players[P2].hero.health = 7;
        assert!(!combat_forecast(&state, P1).lethal);

        // A unit that cannot reach the hero adds nothing: a Taunt wall stands in the way.
        state.players[P2].hero.health = 6;
        put(&mut state, &taunter.id, slot(P2, Row::Units, 3), json!({}));
        let forecast = combat_forecast(&state, P1);
        assert!(!forecast.lethal, "a Taunt unit has to be attacked first");
        assert!(forecast.attacks.iter().all(|entry| entry.target_id != "hero-p2"));
    }

    #[test]
    fn r3100000_a_hero_that_cannot_lose_is_never_lethal() {
        let mut state = board("r3100000-floor");
        put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        state.players[P2].hero.health = 1;
        assert!(combat_forecast(&state, P1).lethal);
        state.players[P2].handicap = Some(Handicap {
            hero_floor: Some(1),
            ..HUMAN_HANDICAP
        });
        assert!(
            !combat_forecast(&state, P1).lethal,
            "R2201000: no hit is lethal to a hero that cannot lose"
        );
    }

    #[test]
    fn r3100000_it_reads_only_what_the_board_shows() {
        let mut state = combat_judge_game("r3100000-hidden", None);
        state.turn = 4;
        state.active = P1;
        state.phase = Phase::Main;
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let plain_forecast = combat_forecast(&state, P1);

        // The opponent's hand, library and a face-down Trap change nothing: none of it shows.
        let mut hidden = state.clone();
        hidden.players[P2].hand.clear();
        hidden.players[P2].library.reverse();
        put(&mut hidden, &snare.id, slot(P2, Row::Backrow, 1), json!({}));
        assert_eq!(combat_forecast(&hidden, P1), plain_forecast);

        // A card on the board for both players to read, a Field Spell, is read: Anti-oneshot Armor caps a hit
        // on its hero.
        let mut visible = state.clone();
        put(
            &mut visible,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        let plain_hit = forecast_of(&state, &striker.id, "hero-p2").expect("the hero may be attacked");
        let capped_hit = forecast_of(&visible, &striker.id, "hero-p2").expect("the hero may be attacked");
        assert!(
            capped_hit.dealt < plain_hit.dealt,
            "{} against {}",
            capped_hit.dealt,
            plain_hit.dealt
        );
    }
}

/// The bodies the property test pairs: every keyword that changes what a blow does or what comes back,
/// and none whose effect is past the two combatants (Cleave's neighbours, Reborn's return).
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
        armored.clone(),
        indestructible.clone(),
        poisonous.clone(),
        lifestealer.clone(),
        trampler.clone(),
    ]
}

fn grants() -> Vec<Keyword> {
    vec![
        Keyword::FirstStrike,
        Keyword::Poisonous,
        Keyword::DivineShield,
        Keyword::Pierce,
        Keyword::Ranged,
        Keyword::Trample,
        Keyword::Armor { n: 1 },
        Keyword::Armor { n: 3 },
        Keyword::Indestructible,
    ]
}

/// A body dressed with up to two keywords, maybe in Defense Position (Armor +1 and Taunt, §4.1) and
/// maybe hurt, but never to 0 health (an already dead unit is the state check's).
fn dress(state: &mut GameState, unit_id: &str, rng: &mut Rng) {
    let pool = grants();
    let mut keywords = Vec::new();
    for _ in 0..rng.int(3) {
        if let Some(keyword) = rng.pick(&pool) {
            keywords.push(keyword.clone());
        }
    }
    let health = unit_view(state, &live(state, unit_id)).max_health;
    let unit = find_instance_mut(state, unit_id).expect("dressed unit");
    unit.granted_keywords.extend(keywords);
    if rng.chance(0.25) {
        unit.position = Some(Position::Def);
    }
    if health > 1 && rng.chance(0.4) {
        unit.damage = rng.int(health - 1);
    }
}

/// The forecast of every attack on a random board, held to what the reducer does when it is played.
#[test]
fn r3100000_every_forecast_attack_is_what_the_engine_s_combat_does() {
    const CASES: i32 = 400;
    let mut checked = 0;
    let mut pops = 0;
    let mut kills = 0;
    let mut stopped = 0;
    for index in 0..CASES {
        let seed = format!("r3100000-property-{index}");
        let mut rng = Rng::new(&seed, 0);
        let mut state = board(&seed);
        let bodies = bodies();
        let attacker_def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
        let defender_def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
        let attacker = put(
            &mut state,
            &attacker_def.id,
            slot(P1, Row::Units, 3),
            json!({ "radiant": rng.chance(0.4) }),
        );
        let defender = put(
            &mut state,
            &defender_def.id,
            slot(P2, Row::Units, 3),
            json!({ "radiant": rng.chance(0.4) }),
        );
        dress(&mut state, &attacker.id, &mut rng);
        dress(&mut state, &defender.id, &mut rng);
        state.players[P2].hero.health = 1 + rng.int(8);
        state.players[P2].hero.armor = rng.int(3);

        for forecast in combat_forecast(&state, P1).attacks {
            let target = &forecast.target_id;
            assert_eq!(
                played_as_forecast(&forecast),
                play_out(&state, &forecast.attacker_id, target),
                "{seed}: {} on {target}: attacker {:?}, defender {:?}",
                forecast.attacker_id,
                unit_view(&state, &live(&state, &attacker.id)),
                unit_view(&state, &live(&state, &defender.id)),
            );
            checked += 1;
            pops += i32::from(forecast.pops);
            kills += i32::from(forecast.kills);
            stopped += i32::from(forecast.dealt == 0);
        }
    }
    // The cases reach every branch the forecast has, or the comparison above proves little.
    assert!(checked > CASES, "checked {checked} attacks");
    assert!(
        pops > 10 && kills > 10 && stopped > 10,
        "pops {pops}, kills {kills}, stopped {stopped}"
    );
}
