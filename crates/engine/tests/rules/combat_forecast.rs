//! The combat forecast (R3100000, SPEC §9.10): what each attack a seat may declare would do, projected
//! from the board and held to the engine's own combat. The tutorial's coach names an attack by it and does
//! no combat maths of its own (CLAUDE.md rule 7). Every case plays the forecast attack for real and reads
//! what the reducer did through the testkit's `PlayedAttack::read`, the reading the fuzz's I18 shares
//! (#4111): the damage events, the Divine Shield lost and who is left on the field. R4290000 adds each
//! Cleave hit beside the target and the Trample excess onto the target's hero.

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::board_geometry::{FUSE, register_geometry_fixtures};
use crate::rules::fixtures::burn_toxic::{
    PLAIN as BT_PLAIN, SHIELDED as BT_SHIELDED, VOLCANIC, add_burn_toxic_fixtures,
};
use crate::rules::fixtures::combat::{
    armored, big_body, charger, cleaver, first_striker, indestructible, lifestealer, plain, poisonous,
    rusher, shielded, taunter, trampler, zero_attack,
};
use crate::rules::fixtures::combat_judge::{
    banisher, combat_judge_game, register_combat_judge_fixtures, snare,
};
use crate::rules::fixtures::combat_reactions::{POOL, POOL_SIZE, register_e6_fixtures};
use crate::rules::fixtures::damage_combat;
use crate::rules::fixtures::giga::{
    CLEAVER as GIGA_CLEAVER, GIGA_UNIT, PLAIN as GIGA_PLAIN, register_giga_fixtures,
};
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

/// What the engine's combat did when `attacker_id` attacked `target_id`, as the forecast reports it: the
/// units its splash names are read first, in its order (R4290000).
fn play_out(state: &GameState, attacker_id: &str, target_id: &str) -> PlayedAttack {
    let splashed: Vec<String> = forecast_of(state, attacker_id, target_id)
        .map(|entry| entry.splash.into_iter().map(|hit| hit.instance_id).collect())
        .unwrap_or_default();
    let after = act_ok(state, attack_of(attacker_id, target_id));
    PlayedAttack::read(&after.events, &after.state, attacker_id, target_id, &splashed)
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
            PlayedAttack::forecast(&forecast),
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
            PlayedAttack::forecast(&forecast),
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
            PlayedAttack::forecast(&forecast),
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
            PlayedAttack::forecast(&forecast),
            play_out(&state, &striker.id, "hero-p2")
        );

        state.players[P2].hero.health = 4;
        let forecast = forecast_of(&state, &striker.id, "hero-p2").expect("the hero may be attacked");
        assert!(!forecast.kills, "3 damage leaves the hero at 1");
        assert_eq!(
            PlayedAttack::forecast(&forecast),
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

    #[test]
    fn r2179005_a_capped_attacker_outlasts_a_first_strike_defender_and_lands_its_blow() {
        let mut state = board("r2179005-first-strike");
        damage_combat::register();
        // A 4/4 whose own guard caps each hit on it at 1, into a 4/4 First Strike: the strike back comes
        // first but deals it 1, so it lives to land its 4.
        let guard = put(
            &mut state,
            &damage_combat::self_guard.id,
            slot(P1, Row::Units, 1),
            json!({}),
        );
        let striker = put(&mut state, &first_striker.id, slot(P2, Row::Units, 1), json!({}));

        let forecast = forecast_of(&state, &guard.id, &striker.id).expect("the attack is legal");
        assert_eq!((forecast.dealt, forecast.taken), (4, 1));
        assert!(forecast.kills && forecast.survives);
        assert_eq!(
            PlayedAttack::forecast(&forecast),
            play_out(&state, &guard.id, &striker.id)
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
                PlayedAttack::forecast(&forecast),
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

/// `board` with the E6 fixtures (`e6-pool`'s damage pool) and the Burn and Toxic ones registered on
/// top, after `new_game` has registered the engine's own.
fn pool_board(seed: &str) -> GameState {
    let state = board(seed);
    register_e6_fixtures();
    add_burn_toxic_fixtures();
    state
}

/// A damage pool of `POOL_SIZE` in `player`'s backrow, face up and unspent.
fn put_pool(state: &mut GameState, player: PlayerId) -> CardInstance {
    let pool = put(state, POOL, slot(player, Row::Backrow, 1), json!({}));
    assert_eq!(damage_pool_left(state, &live(state, &pool)), Some(POOL_SIZE));
    pool
}

/// The forecast of one attack, held to what the engine's combat does with it.
fn forecast_played(state: &GameState, attacker_id: &str, target_id: &str) -> AttackForecast {
    let forecast = forecast_of(state, attacker_id, target_id).expect("the attack is legal");
    assert_eq!(
        PlayedAttack::forecast(&forecast),
        play_out(state, attacker_id, target_id),
        "{attacker_id} on {target_id}"
    );
    forecast
}

mod r4032000_the_forecast_reads_the_pool_and_conversion {
    use super::*;

    #[test]
    fn r4032000_a_blow_into_a_pool_deals_what_is_left() {
        let mut state = pool_board("r4032000-forecast-blow");
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &striker.id)
            .expect("on the field")
            .buffs
            .attack = 1; // 6/10
        put_pool(&mut state, P2);
        state.players.p2.hero.health = 4;

        // 6 less the pool's 5 is 1, short of 4.
        let forecast = forecast_played(&state, &striker.id, "hero-p2");
        assert_eq!(forecast.dealt, 1);
        assert!(!forecast.kills);
        assert!(!combat_forecast(&state, P1).lethal);

        state.players.p2.hero.health = 1;
        let forecast = forecast_played(&state, &striker.id, "hero-p2");
        assert!(forecast.kills);
        assert!(combat_forecast(&state, P1).lethal);
    }

    #[test]
    fn r4032000_the_blows_the_lethal_sum_adds_share_the_pool() {
        let mut state = pool_board("r4032000-forecast-sum");
        let first = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let second = put(&mut state, &charger.id, slot(P1, Row::Units, 2), json!({})); // 3/3
        put_pool(&mut state, P2);
        state.players.p2.hero.health = 2;

        // 3 + 3 is 6, but the pool soaks the first 5 of them: 1 lands, short of 2.
        assert!(!combat_forecast(&state, P1).lethal);
        for attacker in [&first, &second] {
            assert_eq!(forecast_played(&state, &attacker.id, "hero-p2").dealt, 0);
        }
        let once = act_ok(&state, attack_of(&first.id, "hero-p2"));
        assert_eq!(once.state.players.p2.hero.health, 2);
        let twice = act_ok(&once.state, attack_of(&second.id, "hero-p2"));
        assert_eq!(twice.state.players.p2.hero.health, 1);

        state.players.p2.hero.health = 1;
        assert!(combat_forecast(&state, P1).lethal);
    }

    #[test]
    fn r4032000_a_blow_on_a_unit_behind_a_pool_kills_only_with_what_is_left() {
        let mut state = pool_board("r4032000-forecast-unit");
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        let wall = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        let stone = put(&mut state, &indestructible.id, slot(P2, Row::Units, 2), json!({})); // 4/4
        let pool = put_pool(&mut state, P2);

        // The pool soaks all 5, and the 3 struck back lands.
        let forecast = forecast_played(&state, &striker.id, &wall.id);
        assert_eq!((forecast.dealt, forecast.taken), (0, 3));
        assert!(!forecast.kills);

        // An Indestructible unit takes nothing and leaves the pool whole (§4.4 step 4).
        assert_eq!(forecast_played(&state, &striker.id, &stone.id).dealt, 0);
        let after = act_ok(&state, attack_of(&striker.id, &stone.id));
        assert_eq!(
            damage_pool_left(&after.state, &live(&after.state, &pool)),
            Some(POOL_SIZE)
        );

        // 8 less 5 leaves 3, which still destroys the 3-health unit.
        find_instance_mut(&mut state, &striker.id)
            .expect("on the field")
            .buffs
            .attack = 3;
        let forecast = forecast_played(&state, &striker.id, &wall.id);
        assert_eq!(forecast.dealt, 3);
        assert!(forecast.kills);
    }

    #[test]
    fn r4032000_a_converting_blow_deals_and_kills_nothing() {
        let mut state = pool_board("r4032000-forecast-volcanic");
        let volcanic = put(&mut state, VOLCANIC, slot(P1, Row::Units, 1), json!({})); // 4/8
        let wall = put(&mut state, BT_PLAIN, slot(P2, Row::Units, 1), json!({})); // 2/20
        let shield = put(&mut state, BT_SHIELDED, slot(P2, Row::Units, 2), json!({})); // 2/20
        state.players.p2.hero.health = 3;
        // R345: p1's turn stays open after the blow, so no Burn ticks at p2's start of turn.
        state.players.p1.auto_end_turn = Some(false);

        // §4.4 step 3a: the hit is Burn, not damage; the strike back is damage all the same.
        let forecast = forecast_played(&state, &volcanic.id, &wall.id);
        assert_eq!((forecast.dealt, forecast.taken), (0, 2));
        assert!(!forecast.kills);
        // Step 1 comes first, so the blow still breaks a Divine Shield.
        let forecast = forecast_played(&state, &volcanic.id, &shield.id);
        assert_eq!(forecast.dealt, 0);
        assert!(forecast.pops);

        let forecast = forecast_played(&state, &volcanic.id, "hero-p2");
        assert_eq!(forecast.dealt, 0);
        assert!(!forecast.kills);
        assert!(!combat_forecast(&state, P1).lethal);
    }
}

/// `board` with the combat judge's fixtures (`cj-banisher`, a 2/2 whose damage exiles what it lands
/// on) registered on top, after `new_game` has registered the engine's own.
fn banisher_board(seed: &str) -> GameState {
    let state = board(seed);
    register_combat_judge_fixtures();
    state
}

mod r4111000_the_forecast_reads_exile_on_damage {
    use super::*;

    /// The forecast's (dealt, taken, kills, survives), held to what the engine's combat does with it.
    fn forecast_and_play(state: &GameState, attacker_id: &str, target_id: &str) -> (i32, i32, bool, bool) {
        let forecast = forecast_played(state, attacker_id, target_id);
        (forecast.dealt, forecast.taken, forecast.kills, forecast.survives)
    }

    #[test]
    fn r4111000_a_banisher_s_blow_exiles_the_unit_it_damages() {
        let mut state = banisher_board("r4111000-blow");
        let exiler = put(&mut state, &banisher.id, slot(P1, Row::Units, 1), json!({})); // 2/2
        let wall = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        // 2 is short of 3 health, but the exile removes the wall all the same (R1124).
        assert_eq!(
            forecast_and_play(&state, &exiler.id, &wall.id),
            (2, 3, true, false)
        );
    }

    #[test]
    fn r4111000_a_banisher_s_strike_back_exiles_its_attacker() {
        let mut state = banisher_board("r4111000-back");
        let striker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let exiler = put(&mut state, &banisher.id, slot(P2, Row::Units, 1), json!({})); // 2/2
        // 2 back leaves a 3-health attacker standing, but exiled.
        assert_eq!(
            forecast_and_play(&state, &striker.id, &exiler.id),
            (3, 2, true, false)
        );
    }

    #[test]
    fn r4111000_a_first_strike_banisher_exiles_its_attacker_before_its_blow() {
        let mut state = banisher_board("r4111000-first-strike");
        let striker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        let exiler = put(&mut state, &banisher.id, slot(P2, Row::Units, 1), json!({})); // 2/2
        grant(&mut state, &exiler.id, Keyword::FirstStrike);
        // §4.3 step 1: the banisher's 2 comes first and marks the 10-health attacker, which lands nothing.
        assert_eq!(
            forecast_and_play(&state, &striker.id, &exiler.id),
            (0, 2, false, false)
        );
    }

    #[test]
    fn r4111000_a_hit_that_deals_nothing_exiles_nothing() {
        let mut state = banisher_board("r4111000-nothing");
        let exiler = put(&mut state, &banisher.id, slot(P1, Row::Units, 1), json!({})); // 2/2
        let wall = put(&mut state, &armored.id, slot(P2, Row::Units, 1), json!({})); // 7/7 Armor 7
        let shield = put(&mut state, &shielded.id, slot(P2, Row::Units, 2), json!({})); // 2/2 Divine Shield
        // Armor takes the whole blow: nothing lands, so nothing is marked (R1124).
        assert_eq!(
            forecast_and_play(&state, &exiler.id, &wall.id),
            (0, 7, false, false)
        );
        let forecast = forecast_played(&state, &exiler.id, &shield.id);
        assert_eq!(forecast.dealt, 0);
        assert!(forecast.pops && !forecast.kills);
    }
}

/// `board` with the board-geometry fixtures (`geo-fuse`, a 2/20 Ranged Unit whose attacks weigh 5× in its
/// lane and 3× beside it) registered on top, after `new_game` has registered the engine's own.
fn sweep_board(seed: &str) -> GameState {
    let state = board(seed);
    register_geometry_fixtures();
    state
}

mod r4156000_the_forecast_strikes_back_a_sweeper_plain {
    use super::*;

    /// The forecast's (dealt, taken, kills, survives), held to what the engine's combat does with it.
    fn forecast_and_play(state: &GameState, attacker_id: &str, target_id: &str) -> (i32, i32, bool, bool) {
        let forecast = forecast_played(state, attacker_id, target_id);
        (forecast.dealt, forecast.taken, forecast.kills, forecast.survives)
    }

    #[test]
    fn r4156000_an_attack_on_a_sweeper_takes_its_plain_attack_back() {
        let mut state = sweep_board("r4156000-forecast");
        let across = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let beside = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({})); // 3/3
        let fuse = put(&mut state, FUSE, slot(P2, Row::Units, 1), json!({})); // 2/20
        // 2 back from either lane, where its weight would have made it 10 across and 6 beside.
        for striker in [&across, &beside] {
            assert_eq!(
                forecast_and_play(&state, &striker.id, &fuse.id),
                (3, 2, false, true)
            );
        }
    }

    #[test]
    fn r4156000_a_first_strike_sweeper_does_not_fell_its_attacker_first() {
        let mut state = sweep_board("r4156000-first-strike");
        let striker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let fuse = put(&mut state, FUSE, slot(P2, Row::Units, 1), json!({})); // 2/20
        grant(&mut state, &fuse.id, Keyword::FirstStrike);
        // Its First Strike hits for 2, short of 3 health, so the blow still lands; weighted, the 10 would
        // have destroyed the attacker before it struck.
        assert_eq!(
            forecast_and_play(&state, &striker.id, &fuse.id),
            (3, 2, false, true)
        );
    }
}

mod r4221003_the_forecast_counts_a_sweep_s_trample_onto_the_hero {
    use super::*;

    /// Fuzz seed 7606: a trampling sweeper's attack on the enemy hero sweeps the enemy Units too (R3554008),
    /// and what each sweep hit deals beyond its Unit's health goes on to the hero (§4.4 step 9), as Cleave's
    /// excess does (R176): the forecast's `dealt` and `kills`, and the seat's `lethal`, count it.
    #[test]
    fn r4221003_a_trampling_sweeper_s_attack_on_the_hero_counts_its_sweep_s_excess() {
        let mut state = sweep_board("r4221003-forecast");
        let fuse = put(&mut state, FUSE, slot(P1, Row::Units, 1), json!({})); // 2/20
        grant(&mut state, &fuse.id, Keyword::Trample);
        // A 3/3 beside it takes 3x2 = 6, 3 of it beyond its health.
        put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let forecast = forecast_played(&state, &fuse.id, "hero-p2");
        assert_eq!((forecast.dealt, forecast.kills), (2 + 3, false));

        state.players[P2].hero.health = 5;
        let forecast = forecast_played(&state, &fuse.id, "hero-p2");
        assert_eq!((forecast.dealt, forecast.kills), (5, true));
        assert!(combat_forecast(&state, P1).lethal);
    }

    /// With no Trample, the sweep's hits end on their Units: the hero takes the blow alone.
    #[test]
    fn r4221003_a_sweeper_with_no_trample_deals_the_hero_its_blow_alone() {
        let mut state = sweep_board("r4221003-no-trample");
        let fuse = put(&mut state, FUSE, slot(P1, Row::Units, 1), json!({})); // 2/20
        put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        let forecast = forecast_played(&state, &fuse.id, "hero-p2");
        assert_eq!((forecast.dealt, forecast.kills), (2, false));
    }
}

/// `giga.rs`'s board (R3546001): a 3/8 Cleave Unit in p1's lane 3, facing a 2/6 GIGA across p2's
/// lanes 1 and 2 and a 1/3 in p2's lane 3, each side with a library and its turn kept open. The ids of
/// the cleaver, the GIGA and the 1/3.
fn giga_board(seed: &str) -> (GameState, String, String, String) {
    register_giga_fixtures();
    let mut s = scenario(json!({
        "seed": seed,
        "active": "p1",
        "p1": {
            "field": [{ "def": GIGA_CLEAVER, "lane": 3 }],
            "library": [GIGA_PLAIN, GIGA_PLAIN, GIGA_PLAIN, GIGA_PLAIN],
        },
        "p2": {
            "field": [{ "def": GIGA_UNIT, "lane": 1 }, { "def": GIGA_PLAIN, "lane": 3 }],
            "library": [GIGA_PLAIN, GIGA_PLAIN, GIGA_PLAIN, GIGA_PLAIN],
        },
    }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    let striker = s.unit(P1, 3).expect("no cleaver in lane 3").id.clone();
    let giga = s.unit(P2, 1).expect("no GIGA in lane 1").id.clone();
    let plain_id = s.unit(P2, 3).expect("no 1/3 in lane 3").id.clone();
    (s.state().clone(), striker, giga, plain_id)
}

/// The splash hit on `id` in `forecast`.
fn splash_on<'a>(forecast: &'a AttackForecast, id: &str) -> &'a SplashHit {
    forecast
        .splash
        .iter()
        .find(|hit| hit.instance_id == id)
        .unwrap_or_else(|| panic!("no splash on {id} in {:?}", forecast.splash))
}

mod r4290000_splash_and_overflow {
    use super::*;

    /// A 3/8 Cleave Unit in p1's lane 2, `target` in p2's lane 2 and `left` and `right` beside it.
    fn cleave_board(
        seed: &str,
        target: &CardDef,
        left: &CardDef,
        right: &CardDef,
    ) -> (GameState, String, String, String, String) {
        let mut state = board(seed);
        let striker = put(&mut state, &cleaver.id, slot(P1, Row::Units, 2), json!({}));
        let target = put(&mut state, &target.id, slot(P2, Row::Units, 2), json!({}));
        let left = put(&mut state, &left.id, slot(P2, Row::Units, 1), json!({}));
        let right = put(&mut state, &right.id, slot(P2, Row::Units, 3), json!({}));
        (state, striker.id, target.id, left.id, right.id)
    }

    #[test]
    fn r4290000_a_cleave_names_each_neighbour_s_splash() {
        let (state, striker, target, left, right) =
            cleave_board("r4290000-splash", &plain, &shielded, &plain);
        let forecast = forecast_played(&state, &striker, &target);
        assert_eq!((forecast.dealt, forecast.kills), (3, true));
        assert_eq!(forecast.splash.len(), 2, "{:?}", forecast.splash);
        let shield = splash_on(&forecast, &left);
        assert_eq!(
            (shield.dealt, shield.kills, shield.pops),
            (0, false, true),
            "the Divine Shield takes the whole hit and breaks"
        );
        let body = splash_on(&forecast, &right);
        assert_eq!((body.dealt, body.kills, body.pops), (3, true, false));
        assert_eq!(forecast.overflow, None, "no Trample, no overflow");
    }

    #[test]
    fn r4290000_the_splash_lands_when_the_blow_is_stopped() {
        for (index, stopper) in [&shielded, &indestructible, &armored].into_iter().enumerate() {
            let (state, striker, target, left, right) =
                cleave_board(&format!("r4290000-stopped-{index}"), stopper, &plain, &plain);
            let forecast = forecast_played(&state, &striker, &target);
            assert_eq!(forecast.dealt, 0, "{} stops the blow", stopper.id);
            for id in [&left, &right] {
                let hit = splash_on(&forecast, id);
                assert_eq!(
                    (hit.dealt, hit.kills),
                    (3, true),
                    "Cleave is the attack's, not the blow's (R63): beside {}",
                    stopper.id
                );
            }
        }
    }

    #[test]
    fn r4290000_a_giga_beside_the_target_takes_the_splash_once() {
        let (state, striker, giga, plain_id) = giga_board("r4290000-giga");
        let forecast = forecast_played(&state, &striker, &plain_id);
        assert_eq!(forecast.splash.len(), 1, "{:?}", forecast.splash);
        let hit = splash_on(&forecast, &giga);
        assert_eq!((hit.dealt, hit.kills), (3, false), "a 2/6 GIGA takes 3 once");

        // Beside the GIGA's two lanes, the 1/3 takes the same 3.
        let forecast = forecast_played(&state, &striker, &giga);
        assert_eq!(forecast.splash.len(), 1, "{:?}", forecast.splash);
        let hit = splash_on(&forecast, &plain_id);
        assert_eq!((hit.dealt, hit.kills), (3, true));
    }

    /// Fuzz seeds 129, 201, 267, 472, 483, 559, 777 and 884: nothing reaches a Dormant card (§3.1,
    /// R1839000), so a Cleave beside one strikes it nothing and its Trample sends nothing on from it.
    #[test]
    fn r4290000_a_dormant_unit_beside_the_target_takes_no_splash() {
        let (mut state, striker, target, left, right) =
            cleave_board("r4290000-dormant", &plain, &plain, &plain);
        grant(&mut state, &striker, Keyword::Trample);
        let dormant = find_instance_mut(&mut state, &left).expect("on the field");
        dormant.slumber = Some(Slumber { turns: None });
        dormant.damage = 2; // 3/1: struck, it would pass 2 on to the hero
        let forecast = forecast_played(&state, &striker, &target);
        assert_eq!(
            forecast.splash.iter().map(|hit| hit.instance_id.as_str()).collect::<Vec<_>>(),
            vec![right.as_str()]
        );
        // The blow's 3 and the right's 3 each end on a 3/3: no excess.
        assert_eq!(forecast.overflow, None);
    }

    #[test]
    fn r4290000_a_ranged_cleaver_splashes_and_takes_nothing_back() {
        let (mut state, striker, target, left, right) =
            cleave_board("r4290000-ranged", &big_body, &plain, &plain);
        grant(&mut state, &striker, Keyword::Ranged);
        let forecast = forecast_played(&state, &striker, &target);
        assert_eq!((forecast.taken, forecast.survives), (0, true));
        for id in [&left, &right] {
            assert_eq!(
                splash_on(&forecast, id).dealt,
                3,
                "a Ranged attacker's Cleave splashes as ever (R783001)"
            );
        }
    }

    #[test]
    fn r4290000_a_cleaver_a_first_strike_defender_destroys_first_splashes_nothing() {
        let (mut state, striker, target, _, _) =
            cleave_board("r4290000-first-strike", &big_body, &plain, &plain);
        find_instance_mut(&mut state, &striker)
            .expect("on the field")
            .damage = 4; // 3/4 left
        grant(&mut state, &target, Keyword::FirstStrike);
        let forecast = forecast_played(&state, &striker, &target);
        assert_eq!((forecast.dealt, forecast.survives), (0, false));
        assert!(
            forecast.splash.is_empty(),
            "an attacker that falls first never cleaves (R176): {:?}",
            forecast.splash
        );
        assert_eq!(forecast.overflow, None);
    }

    #[test]
    fn r4290000_the_splash_spends_the_pool_after_the_blow() {
        let mut state = pool_board("r4290000-pool");
        let striker = put(&mut state, &cleaver.id, slot(P1, Row::Units, 2), json!({}));
        let target = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        put(&mut state, &plain.id, slot(P2, Row::Units, 3), json!({}));
        put_pool(&mut state, P2);
        // The pool's 5 soaks the blow's 3 whole, then 2 of the first Cleave hit's 3 (R4032000).
        let forecast = forecast_played(&state, &striker.id, &target.id);
        assert_eq!(forecast.dealt, 0);
        let mut dealt: Vec<i32> = forecast.splash.iter().map(|hit| hit.dealt).collect();
        dealt.sort_unstable();
        assert_eq!(dealt, vec![3 - (POOL_SIZE - 3), 3]);
    }

    #[test]
    fn r4290000_a_trample_overflow_passes_the_hero_s_armor_and_cap() {
        let mut state = board("r4290000-overflow");
        let striker = put(&mut state, &trampler.id, slot(P1, Row::Units, 2), json!({})); // 6/4
        let wall = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({})); // 3/3
        state.players[P2].hero.armor = 2;
        // 6 on a 3-health unit: 3 goes on, less Armor 2.
        let forecast = forecast_played(&state, &striker.id, &wall.id);
        assert_eq!(forecast.dealt, 3);
        assert_eq!(
            forecast.overflow,
            Some(TrampleOverflow {
                hero_id: "hero-p2".into(),
                dealt: 1,
                kills: false,
            })
        );
        state.players[P2].hero.health = 1;
        let forecast = forecast_played(&state, &striker.id, &wall.id);
        assert!(
            forecast.overflow.as_ref().is_some_and(|overflow| overflow.kills),
            "{:?}",
            forecast.overflow
        );

        // Anti-oneshot Armor caps the 9 a 12/8 tramples past a 3/3 as one instance.
        let mut state = board("r4290000-overflow-cap");
        let striker = put(
            &mut state,
            &trampler.id,
            slot(P1, Row::Units, 2),
            json!({ "radiant": true }),
        );
        let wall = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        put(&mut state, &anti_oneshot().id, slot(P2, Row::Backrow, 1), json!({}));
        let forecast = forecast_played(&state, &striker.id, &wall.id);
        let overflow = forecast.overflow.expect("the excess reaches the hero");
        assert_eq!(overflow.dealt, jackioh_engine::config::ANTI_ONESHOT_CAP.base);
        assert!(overflow.dealt < 12 - 3);
    }

    #[test]
    fn r4290000_a_cleaving_trampler_s_overflow_is_each_hit_s_excess() {
        let (mut state, striker, target, left, right) =
            cleave_board("r4290000-cleave-trample", &plain, &plain, &plain);
        grant(&mut state, &striker, Keyword::Trample);
        for id in [&target, &left, &right] {
            find_instance_mut(&mut state, id)
                .expect("on the field")
                .damage = 2; // 3/1
        }
        state.players[P2].hero.armor = 1;
        let forecast = forecast_played(&state, &striker, &target);
        assert_eq!(forecast.dealt, 1, "a Trample hit deals no more than the health");
        for id in [&left, &right] {
            assert_eq!(
                (splash_on(&forecast, id).dealt, splash_on(&forecast, id).kills),
                (1, true)
            );
        }
        // Three instances of 2 past a 1-health unit, each less Armor 1 (R176).
        assert_eq!(forecast.overflow.map(|overflow| overflow.dealt), Some(3));
    }

    #[test]
    fn r4290000_no_strike_back_or_hero_target_reports_an_overflow() {
        let mut state = board("r4290000-no-overflow");
        let striker = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({})); // 3/3
        let stomper = put(&mut state, &trampler.id, slot(P2, Row::Units, 1), json!({})); // 6/4
        // A strike back never tramples (R1750000), however far past the attacker's health it goes.
        let forecast = forecast_played(&state, &striker.id, &stomper.id);
        assert_eq!((forecast.taken, &forecast.overflow), (6, &None));

        let mut state = board("r4290000-hero-target");
        let striker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({}));
        // A hero target's `dealt` is the whole of what reaches it.
        let forecast = forecast_played(&state, &striker.id, "hero-p2");
        assert_eq!((forecast.dealt, &forecast.overflow), (6, &None));

        // A plain entry's wire is what it was before the fields came.
        let wire = serde_json::to_value(&forecast).expect("a forecast serializes");
        assert!(wire.get("splash").is_none() && wire.get("overflow").is_none(), "{wire}");
    }
}

/// The forecast of every attack a Cleave Unit may declare on a random board of three, Trample on half
/// of them, held to what the reducer does when it is played: the splash on each neighbour and the
/// overflow onto the hero included (R4290000).
#[test]
fn r4290000_every_cleave_and_trample_forecast_is_what_the_engine_s_combat_does() {
    const CASES: usize = 200;
    let mut checked = 0;
    let mut splashed = 0;
    let mut popped = 0;
    let mut overflowed = 0;
    for index in 0..CASES {
        let seed = format!("r4290000-property-{index}");
        let mut rng = Rng::new(&seed, 0);
        let mut state = board(&seed);
        let bodies = bodies();
        let striker = put(
            &mut state,
            &cleaver.id,
            slot(P1, Row::Units, 2),
            json!({ "radiant": rng.chance(0.4) }),
        );
        dress(&mut state, &striker.id, &mut rng);
        if index % 2 == 0 {
            grant(&mut state, &striker.id, Keyword::Trample);
        }
        for lane in 1..=3 {
            let def = rng.pick(&bodies).cloned().unwrap_or_else(|| plain.clone());
            let unit = put(
                &mut state,
                &def.id,
                slot(P2, Row::Units, lane),
                json!({ "radiant": rng.chance(0.4) }),
            );
            dress(&mut state, &unit.id, &mut rng);
        }
        state.players[P2].hero.health = 1 + rng.int(8);
        state.players[P2].hero.armor = rng.int(3);

        for forecast in combat_forecast(&state, P1).attacks {
            let target = &forecast.target_id;
            assert_eq!(
                PlayedAttack::forecast(&forecast),
                play_out(&state, &forecast.attacker_id, target),
                "{seed}: {} on {target}: attacker {:?}",
                forecast.attacker_id,
                unit_view(&state, &live(&state, &striker.id)),
            );
            checked += 1;
            splashed += forecast.splash.iter().filter(|hit| hit.dealt > 0).count();
            popped += forecast.splash.iter().filter(|hit| hit.pops).count();
            overflowed += usize::from(forecast.overflow.is_some());
        }
    }
    // The cases reach every branch the new fields have, or the comparison above proves little.
    assert!(checked > CASES, "checked {checked} attacks");
    assert!(
        splashed > 20 && popped > 3 && overflowed > 5,
        "splashed {splashed}, popped {popped}, overflowed {overflowed}"
    );
}
