//! Projected attack damage and the lethal check of R44 (BUILD M3-T7): a plain hit, hero Armor, the
//! Anti-oneshot cap in both forms, Trample excess from an attack on a unit, a non-lethal hit, and
//! the rule that a projection is a projection: SPEC §4.2 step 4, §4.3 and the §4.4 pipeline.
//!
//! Port of `packages/engine/test/lethal.test.ts`. TS held the live card `put` returned and wrote
//! through it; here the card is re-read from the state before each call (`live`) and written through
//! `find_instance_mut` (`edit`).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::subsystems::lethal::{defending_hero, is_lethal, projected_damage};

use crate::rules::fixtures::combat::{armoured, big_body, indestructible, plain, shielded, trampler};
use crate::rules::fixtures::harness::{new_game, put, slot};
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

/// The card as it stands in `state` now (TS held the live object).
fn live(state: &GameState, card: &CardInstance) -> CardInstance {
    find_instance(state, &card.id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {}", card.id))
}

/// Writes through to the card in `state` (TS wrote to the live object).
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

/// lethal projection (M3-T7, R44)
mod lethal_projection_m3_t7_r44 {
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
        let wall = put(&mut state, &armoured.id, slot(P2, Row::Units, 1), json!({}));
        assert_eq!(unit_view(&state, &live(&state, &wall)).armor, 7);
        let on_wall = unit_now(&state, &wall);
        assert_eq!(projected(&state, &attacker, &on_wall), 5);
        // Without Pierce the wall's Armor leaves 5 of the 12, which its 7 health holds, and the hero's
        // Armor 3 takes 3 off a direct swing.
        edit(&mut state, &attacker, |card| card.granted_keywords = vec![Keyword::Trample]);
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
    fn r44_clamps_the_projection_with_anti_oneshot_armor_base_5_and_radiant_3_s4_4_step_3() {
        let mut state = board("cap-base");
        let attacker = put(&mut state, &big_body.id, slot(P1, Row::Units, 1), json!({ "radiant": true })); // 10/20
        put(&mut state, &anti_oneshot.id, slot(P2, Row::Backrow, 1), json!({}));

        assert_eq!(unit_view(&state, &live(&state, &attacker)).attack, 10);
        assert_eq!(projected(&state, &attacker, &on_hero()), ANTI_ONESHOT_CAP.base);
        state.players.p2.hero.health = 6;
        assert!(!lethal(&state, &attacker, &on_hero()));
        state.players.p2.hero.health = 5;
        assert!(lethal(&state, &attacker, &on_hero()));

        // Radiant Anti-oneshot Armor clamps to 3, so the same 10 attack needs a hero on 3.
        let mut radiant = board("cap-radiant");
        let bigger = put(&mut radiant, &big_body.id, slot(P1, Row::Units, 1), json!({ "radiant": true }));
        put(&mut radiant, &anti_oneshot.id, slot(P2, Row::Backrow, 1), json!({ "radiant": true }));

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
        let big = put(&mut capped, &trampler.id, slot(P1, Row::Units, 1), json!({ "radiant": true })); // 12/8 Trample
        let small = put(&mut capped, &plain.id, slot(P2, Row::Units, 1), json!({})); // 3/3
        put(&mut capped, &anti_oneshot.id, slot(P2, Row::Backrow, 1), json!({}));
        assert_eq!(projected(&capped, &big, &unit_now(&capped, &small)), ANTI_ONESHOT_CAP.base);
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
        let armour = put(&mut state, &armoured.id, slot(P2, Row::Units, 2), json!({})); // 7/7 Armor 7
        let immortal = put(&mut state, &indestructible.id, slot(P2, Row::Units, 3), json!({})); // 4/4 Indestructible
        state.players.p2.hero.health = 1;

        // Step 1: Divine Shield negates the whole hit, so nothing tramples through.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &shield)), 0);
        // Step 2: Armor 7 leaves 0 of a 6, so there is no excess either.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &armour)), 0);
        // Step 4: an Indestructible unit takes nothing at all.
        assert_eq!(projected(&state, &attacker, &unit_now(&state, &immortal)), 0);
        for target in [&shield, &armour, &immortal] {
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
        put(&mut state, &anti_oneshot.id, slot(P2, Row::Backrow, 1), json!({}));
        state.players.p2.hero.armor = 1;
        state.players.p2.hero.health = 3;

        let before = serde_json::to_string(&state).expect("serialises");
        let targets: Vec<AttackTarget> = vec![on_hero(), unit_now(&state, &blocker), unit_now(&state, &shield)];
        let now = live(&state, &attacker);
        for target in &targets {
            projected_damage(&state, &now, target);
            is_lethal(&state, &now, target);
        }

        assert_eq!(serde_json::to_string(&state).expect("serialises"), before);
        assert_eq!(state.players.p2.hero.health, 3);
        assert_eq!(live(&state, &blocker).damage, 0);
        assert_eq!(live(&state, &shield).divine_shield_spent, None);
        assert_eq!(state.players.p1.hero.health, HERO_HEALTH);
    }
}
