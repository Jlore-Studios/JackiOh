//! Projected attack damage and the lethal check of R44 (BUILD M3-T7): a plain hit, hero Armor, the
//! Anti-oneshot cap in both forms, Trample excess from an attack on a unit, a non-lethal hit, and
//! the rule that a projection is a projection: SPEC §4.2 step 4, §4.3 and the §4.4 pipeline.
//!
//! A card is re-read from the state before each call (`live`) and written through
//! `find_instance_mut` (`edit`).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::subsystems::lethal::{defending_hero, is_lethal, projected_damage};

use crate::rules::fixtures::board_geometry::{FUSE, register_geometry_fixtures};
use crate::rules::fixtures::burn_toxic::{
    BOTH, FRAIL, NOXIOUS, PLAIN as BT_PLAIN, STONE, VOLCANIC, add_burn_toxic_fixtures,
};
use crate::rules::fixtures::combat::{
    armored, big_body, cleaver, first_striker, indestructible, lifestealer, plain, shielded, trampler,
};
use crate::rules::fixtures::combat_reactions::{POOL, POOL_SIZE, register_e6_fixtures};
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

fn on_unit(instance: &CardInstance) -> AttackTarget {
    AttackTarget::Unit {
        instance: instance.clone(),
    }
}

fn on_hero() -> AttackTarget {
    AttackTarget::Hero { player: P2 }
}

/// Writes through to the card in `state`.
fn edit(state: &mut GameState, card: &CardInstance, change: impl FnOnce(&mut CardInstance)) {
    change(find_instance_mut(state, &card.id).unwrap_or_else(|| panic!("no card {}", card.id)));
}

fn projected(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> i32 {
    projected_damage(state, &live(state, attacker), target)
}

fn lethal(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> bool {
    is_lethal(state, &live(state, attacker), target)
}

/// The unit target as the card stands now.
fn unit_now(state: &GameState, card: &CardInstance) -> AttackTarget {
    on_unit(&live(state, card))
}

mod r44_lethal_projection_m3_t7 {
    use super::*;

    #[test]
    fn r44_projects_a_plain_hit_on_the_hero_as_the_attacker_s_attack() {
        let mut state = board("plain-hit");
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10

        assert_eq!(unit_view(&state, &live(&state, &attacker)).attack, 5);
        assert_eq!(defending_hero(&on_hero()), P2);
        assert_eq!(projected(&state, &attacker, &on_hero()), 5);

        // §4.5 step 2 ends the game at 0 or less, so R44's "≥ health" is exactly that hit.
        assert!(!lethal(&state, &attacker, &on_hero()));
        state.players.p2.hero.health = 6;
        assert!(!lethal(&state, &attacker, &on_hero()));
        state.players.p2.hero.health = 5;
        assert!(lethal(&state, &attacker, &on_hero()));
        state.players.p2.hero.health = 4;
        assert!(lethal(&state, &attacker, &on_hero()));
    }

    #[test]
    fn r346_r44_an_attacker_with_pierce_projects_its_whole_attack_through_hero_and_unit_armor() {
        let mut state = board("pierce");
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        edit(&mut state, &attacker, |card| {
            card.granted_keywords.push(Keyword::Pierce);
            card.granted_keywords.push(Keyword::Trample);
        });
        state.players.p2.hero.armor = 3;
        assert_eq!(projected(&state, &attacker, &on_hero()), 5);
        state.players.p2.hero.health = 5;
        assert!(lethal(&state, &attacker, &on_hero()));

        // Through a unit's Armor too: at 12 attack against a 7/7 with Armor 7, all 12 land on the unit,
        // 5 of them past its health trample on, and the hero's Armor 3 takes none of that either.
        edit(&mut state, &attacker, |card| card.buffs.attack = 7);
        let wall = put(&mut state, &armored.id, slot(P2, Row::Units, 1), json!({}));
        assert_eq!(unit_view(&state, &live(&state, &wall)).armor, 7);
        let on_wall = unit_now(&state, &wall);
        assert_eq!(projected(&state, &attacker, &on_wall), 5);
        // Without Pierce the wall's Armor leaves 5 of the 12, which its 7 health holds, and the hero's
        // Armor 3 takes 3 off a direct swing.
        edit(&mut state, &attacker, |card| {
            card.granted_keywords = vec![Keyword::Trample]
        });
        assert_eq!(projected(&state, &attacker, &on_wall), 0);
        assert_eq!(projected(&state, &attacker, &on_hero()), 9);
    }

    #[test]
    fn r44_subtracts_the_defending_hero_s_armor_before_the_comparison_s4_4_step_2() {
        let mut state = board("hero-armor");
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10
        state.players.p2.hero.armor = 2;

        assert_eq!(projected(&state, &attacker, &on_hero()), 3);
        state.players.p2.hero.health = 4;
        assert!(!lethal(&state, &attacker, &on_hero()));
        state.players.p2.hero.health = 3;
        assert!(lethal(&state, &attacker, &on_hero()));

        // Armor above the attack leaves nothing at all, so a hero on 1 is safe.
        state.players.p2.hero.armor = 5;
        state.players.p2.hero.health = 1;
        assert_eq!(projected(&state, &attacker, &on_hero()), 0);
        assert!(!lethal(&state, &attacker, &on_hero()));
    }

    #[test]
    fn r44_clamps_the_projection_with_anti_oneshot_armor_base_4_and_radiant_3_s4_4_step_3() {
        let mut state = board("cap-base");
        let attacker = put(
            &mut state,
            &big_body.id,
            slot(P1, Row::Units, 1),
            json!({ "radiant": true }),
        ); // 10/20
        put(
            &mut state,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );

        assert_eq!(unit_view(&state, &live(&state, &attacker)).attack, 10);
        assert_eq!(projected(&state, &attacker, &on_hero()), ANTI_ONESHOT_CAP.base);
        state.players.p2.hero.health = ANTI_ONESHOT_CAP.base + 1;
        assert!(!lethal(&state, &attacker, &on_hero()));
        state.players.p2.hero.health = ANTI_ONESHOT_CAP.base;
        assert!(lethal(&state, &attacker, &on_hero()));

        // Radiant Anti-oneshot Armor clamps to 3, so the same 10 attack needs a hero on 3.
        let mut radiant = board("cap-radiant");
        let bigger = put(
            &mut radiant,
            &big_body.id,
            slot(P1, Row::Units, 1),
            json!({ "radiant": true }),
        );
        put(
            &mut radiant,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 1),
            json!({ "radiant": true }),
        );

        assert_eq!(projected(&radiant, &bigger, &on_hero()), ANTI_ONESHOT_CAP.radiant);
        radiant.players.p2.hero.health = 4;
        assert!(!lethal(&radiant, &bigger, &on_hero()));
        radiant.players.p2.hero.health = 3;
        assert!(lethal(&radiant, &bigger, &on_hero()));

        // Armor is step 2 and the cap step 3: a hit already under the cap still pays Armor.
        radiant.players.p2.hero.armor = 8;
        assert_eq!(projected(&radiant, &bigger, &on_hero()), 2);
    }

    #[test]
    fn r44_counts_the_trample_excess_of_an_attack_on_a_unit_toward_lethal_s4_4_step_9() {
        let mut state = board("trample-excess");
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
        let blocker = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3

        // 6 into a 3-health unit: 3 lands on the unit, 3 goes on to its controller's hero.
        assert_eq!(defending_hero(&unit_now(&state, &blocker)), P2);
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &blocker)), 3);
        state.players.p2.hero.health = 4;
        assert!(!lethal(&state, &attacker, &unit_now(&state, &blocker)));
        state.players.p2.hero.health = 3;
        assert!(lethal(&state, &attacker, &unit_now(&state, &blocker)));

        // The excess is an ordinary hero instance, so hero Armor and the cap still apply to it.
        state.players.p2.hero.armor = 1;
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &blocker)), 2);
        state.players.p2.hero.armor = 0;

        let mut capped = board("trample-capped");
        let big = put(
            &mut capped,
            &trampler.id,
            slot(P1, Row::Units, 1),
            json!({ "radiant": true }),
        ); // 12/8 Trample
        let small = put(&mut capped, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        put(
            &mut capped,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        assert_eq!(
            projected(&capped, &big, &unit_now(&capped, &small)),
            ANTI_ONESHOT_CAP.base
        );
    }

    #[test]
    fn r44_projects_nothing_at_a_hero_from_an_attack_on_a_unit_without_trample() {
        let mut state = board("no-trample");
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({})); // 5/10, no Trample
        let blocker = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        state.players.p2.hero.health = 1;

        assert_eq!(projected(&state, &attacker, &unit_now(&state, &blocker)), 0);
        assert!(!lethal(&state, &attacker, &unit_now(&state, &blocker)));
    }

    #[test]
    fn r44_stops_the_trample_excess_wherever_the_hit_on_the_unit_stops_s4_4_steps_1_2_and_4() {
        let mut state = board("trample-stopped");
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
        let shield = put(&mut state, &shielded.id, slot(P2, Row::Units, 1), json!({})); // 2/2 Divine Shield
        let armor = put(&mut state, &armored.id, slot(P2, Row::Units, 2), json!({})); // 7/7 Armor 7
        let immortal = put(&mut state, &indestructible.id, slot(P2, Row::Units, 3), json!({})); // 4/4 Indestructible
        state.players.p2.hero.health = 1;

        // Step 1: Divine Shield negates the whole hit, so nothing tramples through.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &shield)), 0);
        // Step 2: Armor 7 leaves 0 of a 6, so there is no excess either.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &armor)), 0);
        // Step 4: an Indestructible unit takes nothing at all.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &immortal)), 0);
        for target in [&shield, &armor, &immortal] {
            assert!(!lethal(&state, &attacker, &unit_now(&state, target)));
        }

        // A spent Divine Shield is gone (§6.1), so the excess flows again.
        edit(&mut state, &shield, |card| card.divine_shield_spent = Some(true));
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &shield)), 4);
        assert!(lethal(&state, &attacker, &unit_now(&state, &shield)));
    }

    #[test]
    fn r44_is_a_projection_neither_call_touches_the_state() {
        let mut state = board("no-mutation");
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({}));
        let blocker = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let shield = put(&mut state, &shielded.id, slot(P2, Row::Units, 2), json!({}));
        put(
            &mut state,
            &anti_oneshot().id,
            slot(P2, Row::Backrow, 1),
            json!({}),
        );
        state.players.p2.hero.armor = 1;
        state.players.p2.hero.health = 3;

        let before = serde_json::to_string(&state).expect("serializes");
        let targets: Vec<AttackTarget> =
            vec![on_hero(), unit_now(&state, &blocker), unit_now(&state, &shield)];
        let now = live(&state, &attacker);
        for target in &targets {
            projected_damage(&state, &now, target);
            is_lethal(&state, &now, target);
        }

        assert_eq!(serde_json::to_string(&state).expect("serializes"), before);
        assert_eq!(state.players.p2.hero.health, 3);
        assert_eq!(live(&state, &blocker).damage, 0);
        assert_eq!(live(&state, &shield).divine_shield_spent, None);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
    }
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

fn attack_of(attacker: &CardInstance, target_id: &str) -> Value {
    json!({ "type": "attack", "playerId": "p1", "attackerId": attacker.id, "targetId": target_id })
}

mod r4032000_lethal_after_conversion_and_the_pool {
    use super::*;

    #[test]
    fn r4032000_the_pool_comes_off_the_blow_before_lethal_is_judged() {
        let mut state = pool_board("r4032000-blow");
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        edit(&mut state, &attacker, |card| card.buffs.attack = 1); // 6/10
        put_pool(&mut state, P2);
        state.players.p2.hero.health = 4;

        // §4.4 step 3b: 6 less the pool's 5 leaves 1, short of 4, and the projection spends nothing.
        let before = serde_json::to_string(&state).expect("serializes");
        assert_eq!(projected(&state, &attacker, &on_hero()), 1);
        assert!(!lethal(&state, &attacker, &on_hero()));
        assert_eq!(serde_json::to_string(&state).expect("serializes"), before);
        let after = act_ok(&state, attack_of(&attacker, "hero-p2"));
        assert_eq!(after.state.players.p2.hero.health, 3);

        state.players.p2.hero.health = 1;
        assert!(lethal(&state, &attacker, &on_hero()));

        // Armor first ("after Armor", R3550004): 6 less 2 is 4, which the pool soaks whole.
        state.players.p2.hero.armor = 2;
        assert_eq!(projected(&state, &attacker, &on_hero()), 0);
        assert!(!lethal(&state, &attacker, &on_hero()));
    }

    #[test]
    fn r4032000_the_hits_of_one_attack_share_the_one_pool() {
        let mut state = pool_board("r4032000-trample");
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
        let blocker = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        put_pool(&mut state, P2);

        // The pool soaks 5 of the 6, so the 1 left stops on the 3-health unit.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &blocker)), 0);
        // 11 less 5 is 6 into the unit, 3 past its health; the pool is spent, so all 3 land.
        edit(&mut state, &attacker, |card| card.buffs.attack = 5);
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &blocker)), 3);
        let after = act_ok(&state, attack_of(&attacker, &blocker.id));
        assert_eq!(after.state.players.p2.hero.health, HERO_HEALTH - 3);

        // Cleave (§4.4 step 10): the blow's 8 leaves 3 for a 3-health unit after the pool, no excess;
        // each neighbour's 8, the pool spent, tramples 5 on to the hero.
        let mut cleaving = pool_board("r4032000-cleave");
        let cleaver_unit = put(&mut cleaving, &cleaver.id, slot(P1, Row::Units, 2), json!({})); // 3/8 Cleave
        edit(&mut cleaving, &cleaver_unit, |card| {
            card.buffs.attack = 5;
            card.granted_keywords.push(Keyword::Trample);
        });
        let middle = put(&mut cleaving, &plain.id, slot(P2, Row::Units, 2), json!({}));
        put(&mut cleaving, &plain.id, slot(P2, Row::Units, 1), json!({}));
        put(&mut cleaving, &plain.id, slot(P2, Row::Units, 3), json!({}));
        put_pool(&mut cleaving, P2);
        assert_eq!(
            projected(&cleaving, &cleaver_unit, &unit_now(&cleaving, &middle)),
            10
        );
        let after = act_ok(&cleaving, attack_of(&cleaver_unit, &middle.id));
        assert_eq!(after.state.players.p2.hero.health, HERO_HEALTH - 10);
    }

    #[test]
    fn r4032000_a_blow_that_burns_or_is_toxic_projects_nothing() {
        // §4.4 step 3a (R3540003): the hit is its target's Burn or Toxic, not damage.
        for def in [VOLCANIC, NOXIOUS, BOTH] {
            let mut state = pool_board(&format!("r4032000-{def}"));
            let attacker = put(&mut state, def, slot(P1, Row::Units, 1), json!({})); // 4/8
            state.players.p2.hero.health = 2;
            assert_eq!(projected(&state, &attacker, &on_hero()), 0, "{def}");
            assert!(!lethal(&state, &attacker, &on_hero()), "{def}");
        }

        let mut state = pool_board("r4032000-volcanic-trample");
        let attacker = put(&mut state, VOLCANIC, slot(P1, Row::Units, 1), json!({})); // 4/8
        edit(&mut state, &attacker, |card| {
            card.granted_keywords.push(Keyword::Trample)
        });
        let frail = put(&mut state, FRAIL, slot(P2, Row::Units, 1), json!({})); // 2/2
        let control = put(&mut state, &big_body.id, slot(P1, Row::Units, 2), json!({})); // 5/10
        state.players.p2.hero.health = 2;
        // A 4 into a 2-health unit would trample 2, but a converted hit has no excess.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &frail)), 0);
        assert!(!lethal(&state, &attacker, &unit_now(&state, &frail)));
        // The same hero falls to a blow that is damage.
        assert!(lethal(&state, &control, &on_hero()));

        let after = act_ok(&state, attack_of(&attacker, "hero-p2"));
        assert_eq!(after.state.players.p2.hero.health, 2);
        assert!(after.state.result.is_none());
    }

    #[test]
    fn r4032000_conversion_and_an_indestructible_unit_leave_the_pool_unspent() {
        let mut state = pool_board("r4032000-unspent");
        let body = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({}));
        let volcanic = put(&mut state, VOLCANIC, slot(P1, Row::Units, 2), json!({}));
        let stone = put(&mut state, STONE, slot(P2, Row::Units, 1), json!({})); // 2/20 Indestructible
        let wall = put(&mut state, BT_PLAIN, slot(P2, Row::Units, 2), json!({})); // 2/20
        let pool = put_pool(&mut state, P2);
        let (body, volcanic) = (live(&state, &body), live(&state, &volcanic));
        let hero = DamageTarget::Hero { player: P2 };
        let unit = |card: &CardInstance| DamageTarget::Unit {
            instance: live(&state, card),
        };

        // A converted hit deals nothing, so the pool is whole for the next.
        let mut pools = ProjectedPools::default();
        assert_eq!(project_hit(&state, Some(&volcanic), &hero, 4, &mut pools), 0);
        assert_eq!(project_hit(&state, Some(&body), &hero, 9, &mut pools), 4);

        // Step 4 read first: an Indestructible unit takes nothing and spends no pool.
        let mut pools = ProjectedPools::default();
        assert_eq!(project_hit(&state, Some(&body), &unit(&stone), 9, &mut pools), 0);
        assert_eq!(project_hit(&state, Some(&body), &hero, 9, &mut pools), 4);

        // A unit's hit spends its side's pool, which the hero's hit then finds 3 short.
        let mut pools = ProjectedPools::default();
        assert_eq!(project_hit(&state, Some(&body), &unit(&wall), 3, &mut pools), 0);
        assert_eq!(project_hit(&state, Some(&body), &hero, 9, &mut pools), 7);

        // The pool covers its own side only.
        let mut pools = ProjectedPools::default();
        let own = DamageTarget::Hero { player: P1 };
        assert_eq!(project_hit(&state, Some(&body), &own, 3, &mut pools), 3);

        // A projection writes nothing.
        assert_eq!(damage_pool_left(&state, &live(&state, &pool)), Some(POOL_SIZE));
    }

    #[test]
    fn r4032000_the_strike_back_is_projected_through_the_attacker_s_pool() {
        // A First Strike defender's 4 destroys the 4-health attacker first (§4.3 step 1)...
        let mut state = pool_board("r4032000-first-strike");
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
        let striker = put(&mut state, &first_striker.id, slot(P2, Row::Units, 1), json!({})); // 4/4
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &striker)), 0);
        // ...unless the attacker's own pool soaks it: then 6 into 4 health tramples 2.
        put_pool(&mut state, P1);
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &striker)), 2);
        let after = act_ok(&state, attack_of(&attacker, &striker.id));
        assert_eq!(after.state.players.p2.hero.health, HERO_HEALTH - 2);

        // A Lifesteal strike back the attacker's pool soaks heals nothing (§4.4 step 8).
        let mut state = pool_board("r4032000-lifesteal");
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
        let leech = put(&mut state, &lifestealer.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        state.players.p2.hero.health = 3;
        // 3 tramples through, and the 3 struck back heals it all.
        assert!(!lethal(&state, &attacker, &unit_now(&state, &leech)));
        put_pool(&mut state, P1);
        assert!(lethal(&state, &attacker, &unit_now(&state, &leech)));
        let after = act_ok(&state, attack_of(&attacker, &leech.id));
        assert!(after.state.result.is_some());
    }
}

mod r4156000_a_first_strike_sweeper_projects_its_plain_strike_back {
    use super::*;

    /// R4156000, §4.3 step 1: a First Strike sweeper that is attacked strikes first with its plain attack,
    /// so R44's projection lets an attacker that survives it land its blow and trample on.
    #[test]
    fn r4156000_a_trampler_into_a_first_strike_sweeper_still_tramples_to_lethal() {
        let mut state = board("r4156000-trample");
        register_geometry_fixtures();
        let attacker = put(&mut state, &trampler.id, slot(P1, Row::Units, 1), json!({})); // 6/4 Trample
        let fuse = put(&mut state, FUSE, slot(P2, Row::Units, 1), json!({})); // 2/20
        edit(&mut state, &fuse, |card| {
            card.damage = 18;
            card.granted_keywords.push(Keyword::FirstStrike);
        });
        // 2 first leaves the trampler standing (weighted, 10 would have destroyed it first): its 6 meets
        // the Fuse's 2 health left and the other 4 trample through.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &fuse)), 4);
        state.players.p2.hero.health = 4;
        assert!(lethal(&state, &attacker, &unit_now(&state, &fuse)));
    }
}
