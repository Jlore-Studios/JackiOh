// Stinkyscape's combat and replacement verbs (issue #1849, the set's phase 0): the engine primitives
// the set's cards are written with, each through a fixture script (`fixtures/stinkyscape_verbs.rs`)
// before a card uses it — Immune to Spell Damage (R1849000), Evolve (R1849001), destroy becomes exile
// (R1849002), reflected damage (R1849003), the own-turn redirect (R1849004), the Gauntlet's lanes
// (R1849005), "Units cannot be in Defense Position" (R1849006), a random forced attacker (R1849007),
// "when one of your Units is attacked" (R1849008) and a trap's chance to reveal (R1849009); and S15's
// Trap that answers a Unit's death in combat before anything moves (R1875006, issue #1875).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::types_of;

use crate::rules::fixtures::stinkyscape_verbs::*;

/// `active`'s turn 9, its main phase; each side's setup as given, with plain cards to draw, and no
/// turn that ends by itself (R345), so each test ends every turn it means to.
fn board_as(seed: &str, active: &str, p1: Value, p2: Value) -> Scenario {
    register_stinkyscape_verbs();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": active, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    board_as(seed, "p1", p1, p2)
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn backrow_id(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.backrow(player, lane)
        .unwrap_or_else(|| panic!("no backrow card at {player:?}'s lane {lane}"))
        .id
}

fn pick(id: &str) -> Value {
    json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
}

fn has_flying(s: &Scenario, id: &str) -> bool {
    unit_has(s.state(), s.card(id), KeywordKind::Flying)
}

/// The attackers of the forced attacks among `events`, in order.
fn forced_attackers(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AttackDeclared {
                attacker_id,
                forced: true,
                ..
            } => Some(attacker_id.clone()),
            _ => None,
        })
        .collect()
}

/// The ids every attack action `player` may take names as its target, for one attacker.
fn attack_targets_offered(s: &Scenario, attacker: &str) -> Vec<String> {
    let player = s.card(attacker).controller;
    legal_actions(s.state(), player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Attack {
                attacker_id,
                target_id,
            } if attacker_id == attacker => Some(target_id),
            _ => None,
        })
        .collect()
}

mod r1849000_immune_to_spell_damage {
    use super::*;

    #[test]
    fn r1849000_a_spells_hit_deals_nothing_and_keeps_divine_shield() {
        let mut s = board(
            "r1849000-bolt",
            json!({ "hand": [BOLT] }),
            json!({ "field": [IMMUNE] }),
        );
        let immune = id_at(&s, P2, 1);
        s.play(BOLT, pick(&immune));
        let kinds = types_of(s.last_events());
        assert!(
            !kinds.contains(&GameEventType::Damage),
            "no damage instance: {kinds:?}"
        );
        assert!(
            !kinds.contains(&GameEventType::DivineShieldLost),
            "the shield stays: {kinds:?}"
        );
        assert_eq!(s.card(immune.as_str()).divine_shield_spent, None);
        s.expect_stats(immune.as_str(), json!({ "health": 4 }));
    }

    #[test]
    fn r1849000_a_spell_still_targets_it_and_buffs_it() {
        let mut s = board(
            "r1849000-rally",
            json!({ "hand": [RALLY] }),
            json!({ "field": [IMMUNE] }),
        );
        let immune = id_at(&s, P2, 1);
        s.play(RALLY, pick(&immune));
        s.expect_stats(immune.as_str(), json!({ "attack": 3 }));
    }

    #[test]
    fn r1849000_units_and_field_spells_still_damage_it() {
        let mut s = board(
            "r1849000-units",
            json!({ "hand": [EMBER], "field": [GRUNT] }),
            json!({ "field": [IMMUNE] }),
        );
        let immune = id_at(&s, P2, 1);
        // A Field Spell is no Spell: its hit takes the Divine Shield.
        s.play(EMBER, json!({}));
        assert!(types_of(s.last_events()).contains(&GameEventType::DivineShieldLost));
        let grunt = id_at(&s, P1, 1);
        s.attack(grunt.as_str(), immune.as_str());
        s.expect_stats(immune.as_str(), json!({ "health": 1 }));
    }
}

mod r1849001_evolve {
    use super::*;

    #[test]
    fn r1849001_the_verb_runs_its_evolve_hook() {
        let mut s = board(
            "r1849001-verb",
            json!({ "hand": [EVOLVER], "field": [WASP] }),
            json!({}),
        );
        let wasp = id_at(&s, P1, 1);
        assert!(!has_flying(&s, &wasp));
        s.play(EVOLVER, pick(&wasp));
        assert!(has_flying(&s, &wasp), "Evolve: gain Flying");
    }

    #[test]
    fn r1849001_after_it_attacks_a_copy_comes_then_it_evolves() {
        let mut s = board("r1849001-attack", json!({ "field": [WASP] }), json!({}));
        let wasp = id_at(&s, P1, 1);
        s.attack(wasp.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 3);
        let copy = s.unit(P1, 2).expect("the copy takes the next free zone");
        assert_eq!(copy.def_id, WASP);
        assert!(
            !has_flying(&s, &copy.id),
            "the copy was summoned before the wasp evolved"
        );
        assert!(has_flying(&s, &wasp), "then the wasp evolved");
    }

    #[test]
    fn r1849001_no_hook_or_off_the_field_does_nothing() {
        // A Unit with no Evolve evolves into nothing.
        let mut s = board(
            "r1849001-none",
            json!({ "hand": [EVOLVER], "field": [GRUNT] }),
            json!({}),
        );
        let grunt = id_at(&s, P1, 1);
        s.play(EVOLVER, pick(&grunt));
        assert!(!types_of(s.last_events()).contains(&GameEventType::KeywordGranted));
        assert!(!has_flying(&s, &grunt));

        // R2285001: a wasp that died attacking runs no `afterAttack`, so it leaves no copy and nothing
        // evolves.
        let mut s = board(
            "r1849001-dead",
            json!({ "field": [WASP] }),
            json!({ "field": [GRUNT] }),
        );
        let wasp = id_at(&s, P1, 1);
        let grunt = id_at(&s, P2, 1);
        s.attack(wasp.as_str(), grunt.as_str());
        s.expect_in_zone(wasp.as_str(), "graveyard");
        assert!(s.unit(P1, 1).is_none(), "no copy takes the zone the wasp left");
        assert!(!types_of(s.last_events()).contains(&GameEventType::KeywordGranted));
    }
}

mod r1849002_destroy_becomes_exile {
    use super::*;

    #[test]
    fn r1849002_a_lethal_hit_exiles_with_no_death_or_reborn() {
        let mut s = board(
            "r1849002-lethal",
            json!({ "hand": [BOLT], "field": [DISINTEGREX] }),
            json!({ "field": [PHOENIX] }),
        );
        let phoenix = id_at(&s, P2, 1);
        s.play(BOLT, pick(&phoenix));
        s.expect_in_zone(phoenix.as_str(), "exile");
        assert!(s.unit(P2, 1).is_none(), "no Reborn body came back");
        assert!(s.hand(P2).is_empty(), "no Death drew a card");
        let kinds = types_of(s.last_events());
        assert!(kinds.contains(&GameEventType::Exiled));
        assert!(!kinds.contains(&GameEventType::Destroyed), "{kinds:?}");
        assert_eq!(s.state().counters.destroyed, 0);
    }

    #[test]
    fn r1849002_a_destroyed_field_spell_is_exiled() {
        let mut s = board(
            "r1849002-field",
            json!({ "hand": [SMASH], "field": [DISINTEGREX] }),
            json!({ "backrow": [FIELD] }),
        );
        let field = backrow_id(&s, P2, 1);
        s.play(SMASH, pick(&field));
        s.expect_in_zone(field.as_str(), "exile");
    }

    #[test]
    fn r1849002_the_aura_card_dying_with_them_is_exiled_too() {
        let mut s = board(
            "r1849002-wipe",
            json!({ "hand": [WIPE], "field": [DISINTEGREX, GRUNT] }),
            json!({ "field": [GRUNT] }),
        );
        let ids = [id_at(&s, P1, 1), id_at(&s, P1, 2), id_at(&s, P2, 1)];
        s.play(WIPE, json!({}));
        for id in &ids {
            s.expect_in_zone(id.as_str(), "exile");
        }
        assert!(!types_of(s.last_events()).contains(&GameEventType::Destroyed));
    }

    #[test]
    fn r1849002_the_radiant_face_exiles_only_enemies() {
        let mut s = board(
            "r1849002-radiant",
            json!({ "hand": [WIPE], "field": [DISINTEGREX_R, GRUNT] }),
            json!({ "field": [GRUNT] }),
        );
        let (aura, mine, theirs) = (id_at(&s, P1, 1), id_at(&s, P1, 2), id_at(&s, P2, 1));
        s.play(WIPE, json!({}));
        s.expect_in_zone(aura.as_str(), "graveyard");
        s.expect_in_zone(mine.as_str(), "graveyard");
        s.expect_in_zone(theirs.as_str(), "exile");
    }

    #[test]
    fn r1849002_a_sacrifice_still_reaches_the_graveyard() {
        let mut s = board(
            "r1849002-sacrifice",
            json!({ "hand": [OFFERING], "field": [DISINTEGREX, GRUNT] }),
            json!({}),
        );
        let grunt = id_at(&s, P1, 2);
        s.play(OFFERING, pick(&grunt));
        s.expect_in_zone(grunt.as_str(), "graveyard");
        assert!(types_of(s.last_events()).contains(&GameEventType::Destroyed));
    }
}

mod r1849003_reflected_damage {
    use super::*;

    fn damage_events(events: &[GameEvent]) -> Vec<(String, i32)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    target_id, amount, ..
                } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1849003_damage_taken_hits_the_enemy_hero() {
        let mut s = board(
            "r1849003-hero",
            json!({ "hand": [BOLT] }),
            json!({ "field": [PLATING] }),
        );
        let plating = id_at(&s, P2, 1);
        s.play(BOLT, pick(&plating));
        s.expect_stats(plating.as_str(), json!({ "health": 6 - BOLT_DAMAGE }));
        s.expect_health(P1, HERO_HEALTH - BOLT_DAMAGE);
        s.expect_health(P2, HERO_HEALTH);
    }

    #[test]
    fn r1849003_the_rider_hits_the_attacker_on_its_strike() {
        let mut s = board(
            "r1849003-attacker",
            json!({ "field": [GRUNT] }),
            json!({ "field": [PLATING_R] }),
        );
        let (grunt, plating) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(grunt.as_str(), plating.as_str());
        s.expect_stats(plating.as_str(), json!({ "health": 3 }));
        s.expect_health(P1, HERO_HEALTH - 3);
        s.expect_in_zone(grunt.as_str(), "graveyard");
    }

    #[test]
    fn r1849003_a_strike_back_or_spell_is_not_reflected_to_the_source() {
        // A Spell's hit reaches the enemy hero alone.
        let mut s = board(
            "r1849003-spell",
            json!({ "hand": [BOLT] }),
            json!({ "field": [PLATING_R] }),
        );
        let plating = id_at(&s, P2, 1);
        s.play(BOLT, pick(&plating));
        assert_eq!(
            damage_events(s.last_events()),
            vec![
                (plating.clone(), BOLT_DAMAGE),
                ("hero-p1".to_string(), BOLT_DAMAGE)
            ]
        );

        // What a strike back deals the rider goes to the enemy hero, never to the defender.
        let mut s = board(
            "r1849003-strike-back",
            json!({ "field": [SPIKED] }),
            json!({ "field": [GRUNT] }),
        );
        let (spiked, grunt) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(spiked.as_str(), grunt.as_str());
        s.expect_stats(spiked.as_str(), json!({ "health": 3 }));
        s.expect_stats(grunt.as_str(), json!({ "health": 1 }));
        s.expect_health(P2, HERO_HEALTH - 3);
        s.expect_health(P1, HERO_HEALTH);
    }

    #[test]
    fn r1849003_two_riders_fighting_end() {
        let mut s = board(
            "r1849003-two",
            json!({ "field": [SPIKED] }),
            json!({ "field": [PLATING_R] }),
        );
        let (spiked, plating) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(spiked.as_str(), plating.as_str());
        // The plating took the spiked's 2: to p1's hero and back to the spiked, which took it from
        // no attacker's strike and so sends it to p2's hero alone.
        s.expect_stats(plating.as_str(), json!({ "health": 4 }));
        s.expect_stats(spiked.as_str(), json!({ "health": 4 }));
        s.expect_health(P1, HERO_HEALTH - 2);
        s.expect_health(P2, HERO_HEALTH - 2);
        assert_eq!(damage_events(s.last_events()).len(), 4);
    }
}

mod r1849004_own_turn_redirect {
    use super::*;

    /// The redirects among `events`: `(from, to, by)`.
    fn redirects(events: &[GameEvent]) -> Vec<(String, String, Option<String>)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Redirected {
                    what: RedirectWhat::Damage,
                    from_id,
                    to_id,
                    by_instance_id,
                } => Some((from_id.clone(), to_id.clone(), by_instance_id.clone())),
                _ => None,
            })
            .collect()
    }

    fn face_up(def: &str) -> Value {
        json!([{ "def": def, "faceUp": true }])
    }

    #[test]
    fn r1849004_own_turn_hits_move_up_to_the_cap() {
        let mut s = board(
            "r1849004-cap",
            json!({ "hand": [BURN, SINGE], "backrow": face_up(MASOCHISM) }),
            json!({}),
        );
        let trap = backrow_id(&s, P1, 1);
        s.play(BURN, json!({}));
        s.expect_health(P1, HERO_HEALTH);
        s.expect_health(P2, HERO_HEALTH - BURN_DAMAGE);
        assert_eq!(
            redirects(s.last_events()),
            vec![("hero-p1".to_string(), "hero-p2".to_string(), Some(trap.clone()))]
        );
        // 15 of the 20 moved: of the next 10, 5 more move and 5 are taken.
        s.play(SINGE, json!({}));
        s.expect_health(P2, HERO_HEALTH - REDIRECT_CAP);
        s.expect_health(P1, HERO_HEALTH - (BURN_DAMAGE + SINGE_DAMAGE - REDIRECT_CAP));
    }

    #[test]
    fn r1849004_the_multiplier_doubles_what_moves() {
        let mut s = board(
            "r1849004-double",
            json!({ "hand": [BURN, SINGE], "backrow": face_up(MASOCHISM_R) }),
            json!({ "health": 60 }),
        );
        s.play(BURN, json!({}));
        s.expect_health(P2, 60 - REDIRECT_MULTIPLIER * BURN_DAMAGE);
        s.play(SINGE, json!({}));
        // At most 20 is taken off p1 a turn, so at most 40 dealt.
        s.expect_health(P2, 60 - REDIRECT_MULTIPLIER * REDIRECT_CAP);
        s.expect_health(P1, HERO_HEALTH - (BURN_DAMAGE + SINGE_DAMAGE - REDIRECT_CAP));
    }

    #[test]
    fn r1849004_the_cap_restarts_next_turn() {
        let mut s = board(
            "r1849004-turn",
            json!({ "hand": [BURN, SINGE, BURN], "backrow": face_up(MASOCHISM) }),
            json!({ "health": 60, "hand": [PLAIN] }),
        );
        s.play(BURN, json!({}));
        s.play(SINGE, json!({}));
        s.expect_health(P1, HERO_HEALTH - 5);
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
        s.play(BURN, json!({}));
        s.expect_health(P1, HERO_HEALTH - 5);
        s.expect_health(P2, 60 - REDIRECT_CAP - BURN_DAMAGE);
    }

    #[test]
    fn r1849004_the_opponents_turn_is_untouched() {
        let mut s = board_as(
            "r1849004-theirs",
            "p2",
            json!({ "backrow": face_up(MASOCHISM) }),
            json!({ "hand": [ZAP] }),
        );
        s.play(ZAP, json!({}));
        s.expect_health(P1, HERO_HEALTH - ZAP_DAMAGE);
        s.expect_health(P2, HERO_HEALTH);
        assert!(redirects(s.last_events()).is_empty());
    }

    #[test]
    fn r1849004_the_field_trap_reveals_then_redirects() {
        let mut s = board(
            "r1849004-reveal",
            json!({ "hand": [SINGE, BURN], "backrow": [MASOCHISM] }),
            json!({}),
        );
        let trap = backrow_id(&s, P1, 1);
        assert_ne!(s.card(trap.as_str()).face_up, Some(true));
        // The hit that reveals it is taken.
        s.play(SINGE, json!({}));
        s.expect_health(P1, HERO_HEALTH - SINGE_DAMAGE);
        assert!(types_of(s.last_events()).contains(&GameEventType::TrapFired));
        assert_eq!(s.card(trap.as_str()).face_up, Some(true));
        // Face-up, it redirects.
        s.play(BURN, json!({}));
        s.expect_health(P1, HERO_HEALTH - SINGE_DAMAGE);
        s.expect_health(P2, HERO_HEALTH - BURN_DAMAGE);
    }
}

mod r1849005_gauntlet_lanes {
    use super::*;

    const GAUNTLET_REFUSAL: &str = "Gauntlet";

    #[test]
    fn r1849005_inside_attacks_only_inside_never_the_hero() {
        let mut s = board(
            "r1849005-inside",
            json!({ "backrow": [GAUNTLET], "field": [{ "def": GRUNT, "lane": 3 }] }),
            json!({ "field": [{ "def": GRUNT, "lane": 1 }, { "def": GRUNT, "lane": 2 }] }),
        );
        let inside = id_at(&s, P1, 3);
        let (outer, inner) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        s.expect_refused_with(|s| s.attack(inside.as_str(), "hero"), GAUNTLET_REFUSAL);
        s.expect_refused_with(|s| s.attack(inside.as_str(), outer.as_str()), GAUNTLET_REFUSAL);
        s.attack(inside.as_str(), inner.as_str());
        s.expect_in_zone(inner.as_str(), "graveyard");
    }

    #[test]
    fn r1849005_outside_cannot_attack_inside() {
        let mut s = board(
            "r1849005-outside",
            json!({ "backrow": [GAUNTLET], "field": [{ "def": GRUNT, "lane": 1 }] }),
            json!({ "field": [{ "def": GRUNT, "lane": 2 }] }),
        );
        let outside = id_at(&s, P1, 1);
        let inner = id_at(&s, P2, 2);
        s.expect_refused_with(|s| s.attack(outside.as_str(), inner.as_str()), GAUNTLET_REFUSAL);
        s.attack(outside.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 3);
    }

    #[test]
    fn r1849005_a_taunt_inside_binds_only_inside() {
        let mut s = board(
            "r1849005-taunt",
            json!({ "backrow": [GAUNTLET], "field": [{ "def": GRUNT, "lane": 1 }, { "def": GRUNT, "lane": 2 }] }),
            json!({ "field": [{ "def": TAUNTER, "lane": 3 }, { "def": GRUNT, "lane": 4 }] }),
        );
        let (outside, inside) = (id_at(&s, P1, 1), id_at(&s, P1, 2));
        let (taunter, grunt) = (id_at(&s, P2, 3), id_at(&s, P2, 4));
        // No Taunt it may attack binds the attacker outside.
        s.attack(outside.as_str(), "hero");
        s.expect_health(P2, HERO_HEALTH - 3);
        // The one inside must attack the Taunt inside.
        s.expect_refused_with(|s| s.attack(inside.as_str(), grunt.as_str()), "Taunt");
        s.attack(inside.as_str(), taunter.as_str());
        s.expect_stats(taunter.as_str(), json!({ "health": 1 }));
    }

    #[test]
    fn r1849005_forced_attacks_obey_it() {
        let mut s = board(
            "r1849005-forced",
            json!({
                "hand": [GOAD],
                "backrow": [GAUNTLET],
                "field": [{ "def": GRUNT, "lane": 1 }, { "def": GRUNT, "lane": 3 }],
            }),
            json!({ "field": [{ "def": TAUNTER, "lane": 2 }] }),
        );
        let inside = id_at(&s, P1, 3);
        let taunter = id_at(&s, P2, 2);
        s.play(GOAD, pick(&taunter));
        assert_eq!(forced_attackers(s.last_events()), vec![inside]);
        s.expect_stats(taunter.as_str(), json!({ "health": 1 }));
    }

    #[test]
    fn r1849005_legal_actions_agree() {
        let s = board(
            "r1849005-legal",
            json!({
                "backrow": [GAUNTLET],
                "field": [{ "def": GRUNT, "lane": 1 }, { "def": GRUNT, "lane": 3 }],
            }),
            json!({ "field": [{ "def": GRUNT, "lane": 1 }, { "def": GRUNT, "lane": 2 }] }),
        );
        let (outside, inside) = (id_at(&s, P1, 1), id_at(&s, P1, 3));
        let (outer, inner) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        assert_eq!(attack_targets_offered(&s, &inside), vec![inner]);
        assert_eq!(
            attack_targets_offered(&s, &outside),
            vec![outer, "hero-p2".to_string()]
        );
    }
}

mod r1849006_units_cannot_be_in_defense {
    use super::*;

    const REFUSAL: &str = "Defense Position";

    fn offers_switch(s: &Scenario, id: &str) -> bool {
        let unit = s.card(id).clone();
        legal_actions(s.state(), unit.controller).iter().any(
            |action| matches!(action, ActionBody::SwitchPosition { instance_id } if *instance_id == unit.id),
        )
    }

    #[test]
    fn r1849006_no_switch_to_defense_either_side() {
        let mut s = board(
            "r1849006-switch",
            json!({ "hand": [SHOVE], "backrow": [WARP], "field": [GRUNT] }),
            json!({ "hand": [PLAIN], "field": [GRUNT] }),
        );
        let (mine, theirs) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        assert!(!offers_switch(&s, &mine));
        s.expect_refused_with(|s| s.switch_position(mine.as_str()), REFUSAL);
        // An effect's switch does nothing either.
        s.play(SHOVE, pick(&theirs));
        assert_ne!(s.card(theirs.as_str()).position, Some(Position::Def));
        assert!(!types_of(s.last_events()).contains(&GameEventType::PositionSwitched));
        // The aura reaches the other side on its own turn too.
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert!(!offers_switch(&s, &theirs));
        s.expect_refused_with(|s| s.switch_position(theirs.as_str()), REFUSAL);
    }

    #[test]
    fn r1849006_defense_units_flip_as_it_arrives() {
        let mut s = board(
            "r1849006-flip",
            json!({ "hand": [WARP], "field": [{ "def": GRUNT, "position": "DEF" }] }),
            json!({ "field": [{ "def": GRUNT, "position": "DEF" }] }),
        );
        let (mine, theirs) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.play(WARP, json!({}));
        assert_eq!(s.card(mine.as_str()).position, Some(Position::Atk));
        assert_eq!(s.card(theirs.as_str()).position, Some(Position::Atk));
        let switched: Vec<String> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::PositionSwitched {
                    instance_id,
                    position: Position::Atk,
                } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(switched, vec![mine.clone(), theirs.clone()]);
        // No exertion was spent: the unit may still attack this turn.
        assert!(!s.card(mine.as_str()).exertion.switched);
        s.attack(mine.as_str(), theirs.as_str());
    }

    #[test]
    fn r1849006_gone_it_allows_defense_again() {
        let mut s = board(
            "r1849006-gone",
            json!({ "hand": [SMASH], "backrow": [WARP], "field": [GRUNT] }),
            json!({}),
        );
        let (grunt, warp) = (id_at(&s, P1, 1), backrow_id(&s, P1, 1));
        assert!(!offers_switch(&s, &grunt));
        s.play(SMASH, pick(&warp));
        s.expect_in_zone(warp.as_str(), "graveyard");
        assert!(offers_switch(&s, &grunt));
        s.switch_position(grunt.as_str());
        assert_eq!(s.card(grunt.as_str()).position, Some(Position::Def));
    }

    #[test]
    fn r1849006_a_dormant_aura_bars_nothing() {
        // A Dormant card's text is off until it awakens (R1839000), its aura among it.
        let mut s = board(
            "r1849006-dormant",
            json!({ "field": [{ "def": WARDEN, "slumber": { "turns": 2 } }, GRUNT] }),
            json!({ "field": [{ "def": GRUNT, "position": "DEF" }] }),
        );
        let (grunt, theirs) = (id_at(&s, P1, 2), id_at(&s, P2, 1));
        assert!(offers_switch(&s, &grunt));
        s.switch_position(grunt.as_str());
        assert_eq!(s.card(grunt.as_str()).position, Some(Position::Def));
        assert_eq!(s.card(theirs.as_str()).position, Some(Position::Def));
    }
}

mod r1849007_random_attacker {
    use super::*;

    #[test]
    fn r1849007_one_random_unit_attacks_the_enemys_summon() {
        let mut s = board_as(
            "r1849007-one",
            "p2",
            json!({ "backrow": [WARP], "field": [GRUNT, GRUNT, GRUNT] }),
            json!({ "hand": [TAUNTER] }),
        );
        let mine = [id_at(&s, P1, 1), id_at(&s, P1, 2), id_at(&s, P1, 3)];
        let cursor = s.state().rng_cursor;
        s.play(TAUNTER, json!({}));
        let attackers = forced_attackers(s.last_events());
        assert_eq!(attackers.len(), 1, "one of them attacks");
        assert!(mine.contains(&attackers[0]));
        assert_eq!(s.state().rng_cursor, cursor + 1, "drawn once");
    }

    #[test]
    fn r1849007_only_a_unit_that_may_attack_it_is_drawn() {
        for seed in ["r1849007-flying-a", "r1849007-flying-b", "r1849007-flying-c"] {
            let mut s = board_as(
                seed,
                "p2",
                json!({ "backrow": [WARP], "field": [GRUNT, FLYER, GRUNT] }),
                json!({ "hand": [FLYER] }),
            );
            let flyer = id_at(&s, P1, 2);
            s.play(FLYER, json!({}));
            assert_eq!(forced_attackers(s.last_events()), vec![flyer], "{seed}");
        }
    }

    #[test]
    fn r1849007_none_able_draws_nothing() {
        let mut s = board_as(
            "r1849007-none",
            "p2",
            json!({ "backrow": [WARP], "field": [GRUNT, GRUNT] }),
            json!({ "hand": [FLYER] }),
        );
        let cursor = s.state().rng_cursor;
        s.play(FLYER, json!({}));
        assert!(forced_attackers(s.last_events()).is_empty());
        assert_eq!(s.state().rng_cursor, cursor);
    }

    #[test]
    fn r1849007_without_random_every_unit_attacks() {
        let mut s = board_as(
            "r1849007-all",
            "p2",
            json!({ "backrow": [WARP_II], "field": [GRUNT, GRUNT] }),
            json!({ "hand": [TAUNTER] }),
        );
        let mine = vec![id_at(&s, P1, 1), id_at(&s, P1, 2)];
        let cursor = s.state().rng_cursor;
        s.play(TAUNTER, json!({}));
        assert_eq!(forced_attackers(s.last_events()), mine);
        assert_eq!(s.state().rng_cursor, cursor);
    }
}

mod r1849008_ally_attacked {
    use super::*;

    #[test]
    fn r1849008_a_taunt_ally_attacked_is_buffed_before_combat() {
        let mut s = board(
            "r1849008-jempire",
            json!({ "field": [GRUNT] }),
            json!({ "field": [JEMPIRE, TAUNTER] }),
        );
        let grunt = id_at(&s, P1, 1);
        let taunter = id_at(&s, P2, 2);
        s.attack(grunt.as_str(), taunter.as_str());
        // +2 before the combat: its strike back of 4 kills the 3/3, which a 2 would not.
        s.expect_stats(
            taunter.as_str(),
            json!({ "attack": 2 + JEMPIRE_ATTACK, "health": 1 }),
        );
        s.expect_in_zone(grunt.as_str(), "graveyard");
    }

    #[test]
    fn r1849008_hero_and_forced_attacks_wake_nothing() {
        let mut s = board(
            "r1849008-quiet",
            json!({ "hand": [GOAD], "field": [GRUNT] }),
            json!({ "field": [ASKER] }),
        );
        let (grunt, asker) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        s.attack(grunt.as_str(), "hero");
        assert!(s.state().pending.is_none(), "an attack on the hero asks nothing");
        s.expect_health(P2, HERO_HEALTH - 3);
        s.play(GOAD, pick(&asker));
        assert!(s.state().pending.is_none(), "a forced attack asks nothing (R121)");
        assert_eq!(forced_attackers(s.last_events()), vec![grunt]);
        s.expect_stats(asker.as_str(), json!({ "attack": 1, "health": 2 }));
    }

    #[test]
    fn r1849008_a_hook_that_asks_pauses_the_combat_until_answered() {
        for (seed, asker_def, buffed_before) in [
            ("r1849008-ask-first", ASKER, false),
            ("r1849008-ask-last", ASKER_LAST, true),
        ] {
            let mut s = board(seed, json!({ "field": [GRUNT] }), json!({ "field": [asker_def] }));
            let (grunt, asker) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
            s.attack(grunt.as_str(), asker.as_str());
            let pending = s.state().pending.clone().expect("the hook asks its controller");
            assert_eq!(pending.player_id, P2, "{seed}");
            assert!(
                !types_of(s.last_events()).contains(&GameEventType::Damage),
                "{seed}: no combat before the answer"
            );
            let attack = if buffed_before { 1 + ASKER_ATTACK } else { 1 };
            s.expect_stats(asker.as_str(), json!({ "attack": attack }));

            s.answer(json!("left"));
            assert!(s.state().pending.is_none(), "{seed}");
            assert!(s.state().work.is_empty(), "{seed}: nothing is left owed");
            // Buffed, its strike back of 4 kills the 3/3; it takes 3 of its 5.
            s.expect_stats(asker.as_str(), json!({ "attack": 1 + ASKER_ATTACK, "health": 2 }));
            s.expect_in_zone(grunt.as_str(), "graveyard");
        }
    }
}

mod r1849009_reveal_chance {
    use super::*;

    /// The first rng cursor whose `rolls` next draws against Aerelret's 30% are `hits`, in order.
    fn cursor_rolling(s: &Scenario, hits: &[bool]) -> u32 {
        let seed = s.state().seed.clone();
        let chance = f64::from(AERELRET_CHANCE) / f64::from(PERCENT);
        cursor_where(&seed, |cursor| {
            let mut rng = Rng::new(&seed, cursor);
            hits.iter().all(|hit| (rng.next() < chance) == *hit)
        })
    }

    fn trapped(seed: &str, trap: &str) -> (Scenario, String, String) {
        let s = board(seed, json!({ "field": [GRUNT] }), json!({ "backrow": [trap] }));
        let grunt = id_at(&s, P1, 1);
        let trap = backrow_id(&s, P2, 1);
        (s, grunt, trap)
    }

    #[test]
    fn r1849009_a_roll_under_the_chance_reveals() {
        let (mut s, grunt, trap) = trapped("r1849009-hit", AERELRET);
        let cursor = cursor_rolling(&s, &[true]);
        s.state_mut().rng_cursor = cursor;
        s.attack(grunt.as_str(), "hero");
        assert!(types_of(s.last_events()).contains(&GameEventType::TrapFired));
        s.expect_in_zone(trap.as_str(), "graveyard");
        s.expect_stats(grunt.as_str(), json!({ "health": 3 - AERELRET_DAMAGE }));
        assert_eq!(s.state().rng_cursor, cursor + 1);
    }

    #[test]
    fn r1849009_a_miss_leaves_it_set_and_draws_once() {
        let (mut s, grunt, trap) = trapped("r1849009-miss", AERELRET);
        let cursor = cursor_rolling(&s, &[false]);
        s.state_mut().rng_cursor = cursor;
        s.attack(grunt.as_str(), "hero");
        assert!(!types_of(s.last_events()).contains(&GameEventType::TrapFired));
        assert_eq!(backrow_id(&s, P2, 1), trap, "still set where it was");
        assert_ne!(s.card(trap.as_str()).face_up, Some(true));
        s.expect_stats(grunt.as_str(), json!({ "health": 3 }));
        s.expect_health(P2, HERO_HEALTH - 3);
        assert_eq!(s.state().rng_cursor, cursor + 1);
    }

    #[test]
    fn r1849009_luck_rolls_again() {
        let (mut s, grunt, trap) = trapped("r1849009-lucky", LUCKY_AERELRET);
        let cursor = cursor_rolling(&s, &[false, true]);
        s.state_mut().rng_cursor = cursor;
        s.attack(grunt.as_str(), "hero");
        assert!(types_of(s.last_events()).contains(&GameEventType::TrapFired));
        s.expect_in_zone(trap.as_str(), "graveyard");
        let rolls = 1 + u32::try_from(AERELRET_LUCKY).expect("a small Lucky");
        assert_eq!(s.state().rng_cursor, cursor + rolls);
    }

    #[test]
    fn r1849009_no_chance_draws_nothing() {
        let (mut s, grunt, trap) = trapped("r1849009-sure", SURE_TRAP);
        let cursor = s.state().rng_cursor;
        s.attack(grunt.as_str(), "hero");
        assert!(types_of(s.last_events()).contains(&GameEventType::TrapFired));
        s.expect_in_zone(trap.as_str(), "graveyard");
        assert_eq!(s.state().rng_cursor, cursor);
    }
}

/// R1875006 (issue #1875, Stinkyscape #138 Pocket Peptides): a `wouldDie` replacement that answers a
/// Unit killed in combat, its follow-up resolving before any card moves and the check starting over; and
/// a new card's cast with an aim it cannot take casting nothing (R1180005's `aimOnly`, for `castNew`).
mod r1875006_would_die_in_combat {
    use super::*;

    fn reprieved(seed: &str, p1: Value) -> (Scenario, String, String) {
        let s = board(
            seed,
            p1,
            json!({ "field": [{ "def": GRUNT, "lane": 2 }], "backrow": [REPRIEVE] }),
        );
        let grunt = id_at(&s, P2, 2);
        let trap = backrow_id(&s, P2, 1);
        (s, grunt, trap)
    }

    #[test]
    fn r1875006_a_unit_killed_in_combat_is_answered_before_anything_moves_and_the_recheck_spares_it() {
        let (mut s, grunt, trap) = reprieved(
            "r1875006-combat",
            json!({ "field": [{ "def": GRUNT, "lane": 2 }] }),
        );
        let attacker = id_at(&s, P1, 2);
        s.attack(attacker.as_str(), grunt.as_str());
        let types = types_of(s.last_events());
        let fired = types.iter().position(|t| *t == GameEventType::TrapFired);
        let destroyed = types.iter().position(|t| *t == GameEventType::Destroyed);
        assert!(fired.is_some() && fired < destroyed, "{types:?}");
        s.expect_in_zone(trap.as_str(), "graveyard");
        // The 3/3 took 3 and was given +10 Health before anything moved: the re-check spares it.
        s.expect_in_zone(grunt.as_str(), "field");
        s.expect_stats(grunt.as_str(), json!({ "health": REPRIEVE_HEALTH }));
        // Its attacker had no reprieve and dies in the check that starts over.
        s.expect_in_zone(attacker.as_str(), "graveyard");
    }

    #[test]
    fn r1875006_a_spells_hit_or_a_destroy_is_no_death_in_combat() {
        let (mut s, grunt, trap) = reprieved("r1875006-spell", json!({ "hand": [BOLT, SMASH] }));
        s.play(BOLT, pick(&grunt));
        assert!(!types_of(s.last_events()).contains(&GameEventType::TrapFired));
        s.expect_in_zone(grunt.as_str(), "graveyard");
        assert_eq!(backrow_id(&s, P2, 1), trap);
        let mut s = board(
            "r1875006-destroy",
            json!({ "hand": [SMASH] }),
            json!({ "field": [{ "def": GRUNT, "lane": 2 }], "backrow": [REPRIEVE] }),
        );
        let grunt = id_at(&s, P2, 2);
        s.play(SMASH, pick(&grunt));
        assert!(!types_of(s.last_events()).contains(&GameEventType::TrapFired));
        s.expect_in_zone(grunt.as_str(), "graveyard");
    }

    #[test]
    fn r1875006_a_new_cards_cast_with_an_aim_it_cannot_take_casts_nothing() {
        // A bolt aims at Units alone, so "hero:p2" is no legal pick of it.
        let mut s = board(
            "r1875006-aim-only",
            json!({ "hand": [AIM_ONLY, AIM_FALLBACK] }),
            json!({ "field": [GRUNT] }),
        );
        s.play(AIM_ONLY, json!({}));
        assert!(s.state().pending.is_none());
        assert!(
            !s.last_events()
                .iter()
                .any(|event| matches!(event, GameEvent::CardAnnounced { def_id, .. } if def_id == BOLT))
        );
        // Without `aimOnly` the cast falls back to its caster's choice.
        s.play(AIM_FALLBACK, json!({}));
        assert!(s.state().pending.is_some());
    }
}

/// R1884001 (issue #1884, the Stinkyscape release): a card whose `resolveFirst` would-die replacement is
/// resolving is not asked again by a state check its own follow-up runs, so one that no firing spends
/// (a Field Spell, as a fused one holding Stinkyscape #138's text) answers a death once, not without end.
mod r1884001_a_would_die_replacement_is_not_asked_inside_its_own_follow_up {
    use super::*;

    #[test]
    fn r1884001_an_unspent_replacement_answers_once_and_the_unit_dies() {
        let mut s = board(
            "r1884001-endless",
            json!({ "field": [{ "def": GRUNT, "lane": 2 }] }),
            json!({ "field": [{ "def": GRUNT, "lane": 2 }], "backrow": [ENDLESS_REPRIEVE] }),
        );
        let grunt = id_at(&s, P2, 2);
        let reprieve = backrow_id(&s, P2, 1);
        let attacker = id_at(&s, P1, 2);
        s.attack(attacker.as_str(), grunt.as_str());
        let bolts = s
            .last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::CardAnnounced { def_id, .. } if def_id == BOLT))
            .count();
        assert_eq!(bolts, 1, "the follow-up ran once");
        s.expect_in_zone(grunt.as_str(), "graveyard");
        s.expect_in_zone(attacker.as_str(), "graveyard");
        // A Field Spell is no Trap a firing spends: it stays, for the next death.
        assert_eq!(backrow_id(&s, P2, 1), reprieve);
    }

    #[test]
    fn r1884001_a_follow_up_that_waits_on_a_pick_resumes_and_still_answers_once() {
        for pick in [0, 1] {
            let mut s = board(
                "r1884001-asking",
                json!({ "field": [{ "def": GRUNT, "lane": 2 }] }),
                json!({ "field": [{ "def": GRUNT, "lane": 2 }], "backrow": [ASKING_REPRIEVE] }),
            );
            let grunt = id_at(&s, P2, 2);
            let attacker = id_at(&s, P1, 2);
            s.attack(attacker.as_str(), grunt.as_str());
            // The follow-up's bolt waits on its controller's pick: the reduce ends, and the next one
            // resumes it.
            let pending = s.state().pending.clone().expect("the bolt's pick");
            assert_eq!((pending.kind, pending.player_id), (PromptKind::Target, P2));
            s.answer(json!(pending.options[pick].key));
            let bolts = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::CardAnnounced { def_id, .. } if def_id == BOLT))
                .count();
            assert_eq!(bolts, 1, "pick {pick}: the replacement answered once");
            assert!(s.state().pending.is_none(), "pick {pick}: nothing asks again");
            s.expect_in_zone(grunt.as_str(), "graveyard");
        }
    }
}

/// R1859007 (issue #1859, Stinkyscape's War Doctrines): "For the rest of the game, your Units have X",
/// a `unitsHaveKeyword` player modifier: every Unit its holder controls on the field has the keyword,
/// those that arrive later too, but a Vanilla one, a card in a hand and the opponent's Units do not.
mod r1859007_units_have_keyword {
    use super::*;

    fn has_trample(s: &Scenario, id: &str) -> bool {
        unit_has(s.state(), s.card(id), KeywordKind::Trample)
    }

    #[test]
    fn r1859007_units_have_keyword_reaches_every_unit_its_holder_controls_now_and_later() {
        let mut s = board(
            "r1859007",
            json!({
                "hand": [PLAIN],
                "field": [
                    { "def": PLAIN, "lane": 1 },
                    { "def": PLAIN, "lane": 2 },
                    { "def": PLAIN, "lane": 3 },
                ],
            }),
            json!({ "field": [{ "def": PLAIN, "lane": 1 }] }),
        );
        let ours = [id_at(&s, P1, 1), id_at(&s, P1, 2)];
        let bland = id_at(&s, P1, 3);
        let theirs = id_at(&s, P2, 1);
        let held = s.state().players.p1.hand[0].id.clone();
        s.card_mut(bland.as_str()).vanilla = true;
        assert!(!has_trample(&s, &ours[0]));
        s.state_mut().players.p1.mods.push(PlayerModifier {
            id: "r1859007".to_string(),
            expiry: ModifierExpiry::Never,
            kind: ModifierKind::UnitsHaveKeyword {
                keyword: Keyword::Trample,
            },
        });
        assert!(ours.iter().all(|id| has_trample(&s, id)));
        assert!(!has_trample(&s, &bland), "a Vanilla Unit takes none");
        assert!(!has_trample(&s, &theirs), "the opponent's Units take none");
        assert!(!has_trample(&s, &held), "a card in a hand takes none");
        // A Unit that arrives later has it too, and both seats read the badge.
        s.play(PLAIN, json!({ "zone": 4 }));
        assert!(has_trample(&s, &held));
        for (viewer, side) in [(P1, true), (P2, false)] {
            let view = s.view(viewer);
            let mods = if side {
                &view.you.modifiers
            } else {
                &view.opponent.modifiers
            };
            assert!(
                mods.iter().any(|badge| badge.label == "Your Units have Trample"),
                "{viewer:?}: {mods:?}"
            );
        }
    }
}
