//! C+ #1 Doom Shroom (SPEC §8.7 row 1). (3) Trap, Epic.
//! Fires in §4.2 step 4's trap window (as Core #96 My Pawn) when an enemy Unit declares an attack on
//! your hero: exiles every Unit (Radiant: every enemy Unit), tops of piles only, so the attacker is gone
//! and no combat happens (R44, R220); then Locks its own backrow zone (E20) and is consumed.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-001";

/// A declared attack (a forced one opens no window, R121) by an enemy Unit on this trap's controller's hero.
fn attacks_your_hero(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared { attacker_id, target_id, forced } = event else {
        return false;
    };
    if *forced {
        return false;
    }
    let target = attack_target_of(&ctx.state, target_id);
    find_instance(&ctx.state, attacker_id).map(|attacker| attacker.controller) == Some(opponent_of(ctx.controller))
        && matches!(target, Some(AttackTarget::Hero { player }) if player == ctx.controller)
}

fn doom_shroom(side: &'static str) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("doom-shroom", &[GameEventType::AttackDeclared], move |_ctx, _event| {
                vec![exile_all(json_as(json!({ "side": side }))), lock_own_zone()]
            })
            .with_when(attacks_your_hero),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: doom_shroom("any"),
        radiant: doom_shroom("enemy"),
    }
}

// C+ #1 Doom Shroom — SPEC §8.7 row 1, BUILD M9 Classic+ row C+ 1: "Face-down Trap that fires in §4.2
// step 4's trap window when an enemy Unit declares an attack on your hero, never on an attack on a
// unit; exiles every Unit on both sides (the tops of piles, a card dormant beneath resuming), so the
// attacker is gone and no combat resolves (R44's cancel); no Death fires and exiled tokens cease to
// exist; it goes to your graveyard as it fires and its backrow zone is Locked, so a later play into
// that zone is refused; the opponent sees only a face-down card until `trapFired` (R33, R97); radiant
// exiles enemy Units only, yours stay".
//
// Doom Shroom sits face-down in p1's backrow lane 2; p2 is active and attacks.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const DOOM: &str = "classicplus-001";
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FIENDER: &str = "core-092"; // (2) 5/7 Stack.
    const DEFENDER: &str = "core-003"; // (1) Taunt, Divine Shield, Reborn; Radiant: Death summons a base one.
    const TOKEN: &str = "core-t-rush"; // a unit token
    const SHEEPISH: &str = "core-041"; // (1) Trap
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    use crate::merged;

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({
                    "hand": [FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                    "backrow": [{ "def": DOOM, "radiant": radiant_face, "faceUp": false, "lane": 2 }],
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE], "field": [TIMMY] }), p2),
        }))
    }

    fn count(s: &Scenario, type_: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == type_).count()
    }

    #[test]
    fn is_a_3_trap_each_face_declares_one_trap_trigger_on_attackdeclared() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.type_, CardType::Trap);
        assert_eq!(def.cost, CardCost::Fixed(3));
        let scripts = script();
        let ons = |s: &Script| s.triggers.iter().map(|t| t.on.clone()).collect::<Vec<_>>();
        assert_eq!(ons(&scripts.base), vec![vec![GameEventType::AttackDeclared]]);
        assert_eq!(ons(&scripts.radiant), vec![vec![GameEventType::AttackDeclared]]);
    }

    mod base {
        use super::*;

        #[test]
        fn r44_an_enemy_unit_s_attack_on_your_hero_sets_it_off_every_unit_on_both_sides_is_exiled_and_no_combat_resolves()
        {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, TIMMY] }), json!({ "field": [TIMMY, VANILLA] }), false);
            let attacker = s.unit(P2, 1).unwrap();
            let doom = s.card(DOOM).clone();

            s.attack(&attacker, "hero");

            s.expect_events(json!(["attackDeclared", "trapFired", "exiled"]));
            assert_eq!(count(&s, "exiled"), 4);
            for lane in [1, 2] {
                assert!(s.unit(P1, lane).is_none());
                assert!(s.unit(P2, lane).is_none());
            }
            s.expect_in_zone(&attacker, "exile");
            s.expect_health(P1, 30);
            assert_eq!(count(&s, "damage"), 0);
            s.expect_in_zone(&doom, "graveyard");
        }

        #[test]
        fn never_fires_on_an_attack_on_a_unit_the_combat_resolves_and_the_trap_stays_set() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA] }), json!({}), false);
            let doom = s.card(DOOM).clone();

            let attacker = s.unit(P2, 1).unwrap();
            let target = s.unit(P1, 1).unwrap();
            s.attack(&attacker, &target);

            assert_eq!(count(&s, "trapFired"), 0);
            assert!(count(&s, "damage") > 0);
            assert!(matches!(s.card(&doom).zone, Zone::Field { row: Row::Backrow, lane: 2, .. }));
            assert_eq!(s.card(&doom).face_up, Some(false));
        }

        #[test]
        fn never_fires_on_its_own_controller_s_attack_on_the_enemy_hero() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [TIMMY], "backrow": [{ "def": DOOM, "faceUp": false, "lane": 2 }] },
                "p2": { "hand": [FILLER] },
            }));

            let attacker = s.unit(P1, 1).unwrap();
            s.attack(&attacker, "hero");

            assert_eq!(count(&s, "trapFired"), 0);
            s.expect_health(P2, 27);
            assert!(matches!(s.card(DOOM).zone, Zone::Field { row: Row::Backrow, .. }));
        }

        #[test]
        fn s3_2_r13_it_exiles_the_tops_of_piles_only_a_card_dormant_beneath_resumes_and_stays() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, { "def": FIENDER, "stack": true }] }), json!({}), false);
            let vanilla = s.card(VANILLA).clone();
            let fiender = s.card(FIENDER).clone();

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_in_zone(&fiender, "exile");
            assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(vanilla.id.clone()));
        }

        #[test]
        fn r11_s6_3_no_death_fires_reborn_does_not_return_and_an_exiled_token_ceases_to_exist() {
            crate::register_all();
            // The Radiant defender (Reborn; Death: summon a base one) stands on the attacker's side, where
            // its Taunt binds nobody.
            let mut s = setup(
                json!({ "field": [TOKEN] }),
                json!({ "field": [TIMMY, { "def": DEFENDER, "radiant": true }] }),
                false,
            );
            let defender = s.card(DEFENDER).clone();
            let token = s.unit(P1, 1).unwrap();

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_in_zone(&defender, "exile");
            s.expect_in_zone(&token, "gone");
            assert_eq!(count(&s, "summoned"), 0);
            assert_eq!(count(&s, "destroyed"), 0);
            assert!(s.unit(P1, 1).is_none());
            assert!(s.unit(P2, 2).is_none());
        }

        #[test]
        fn e20_its_backrow_zone_is_locked_as_it_fires_and_a_later_play_into_that_zone_is_refused() {
            crate::register_all();
            let mut s = setup(json!({ "hand": [SHEEPISH, SHEEPISH, FILLER] }), json!({}), false);

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_events(json!(["trapFired", "locked"]));
            assert_eq!(s.view(P1).you.locks.backrow, vec![false, true, false, false, false]);
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.expect_refused(|s| s.play(SHEEPISH, json!({ "zone": 2 })));
            s.play(SHEEPISH, json!({ "zone": 3 }));
            assert!(s.backrow(P1, 2).is_none());
        }

        #[test]
        fn r33_r97_the_opponent_reads_only_a_face_down_card_until_it_fires_then_its_graveyard_names_it() {
            crate::register_all();
            let mut s = setup(json!({}), json!({}), false);
            assert!(serde_json::to_string(&s.view(P1).you.backrow).unwrap().contains(DOOM));
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(DOOM));

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            let graveyard: Vec<String> =
                s.view(P2).opponent.graveyard.iter().map(|card| card.def_id.clone()).collect();
            assert!(graveyard.iter().any(|def_id| def_id == DOOM));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn exiles_enemy_units_only_the_attacker_is_gone_no_combat_and_your_units_stay() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA, TIMMY] }), json!({ "field": [TIMMY, VANILLA] }), true);
            let attacker = s.unit(P2, 1).unwrap();
            let mine = [s.unit(P1, 1).unwrap(), s.unit(P1, 2).unwrap()];

            s.attack(&attacker, "hero");

            s.expect_in_zone(&attacker, "exile");
            assert!(s.unit(P2, 2).is_none());
            assert_eq!(
                mine.iter().map(|card| s.card(card).zone.z()).collect::<Vec<_>>(),
                vec![ZoneName::Field, ZoneName::Field]
            );
            s.expect_health(P1, 30);
            assert_eq!(count(&s, "damage"), 0);
            assert!(s.view(P1).you.locks.backrow[1]);
            s.expect_in_zone(DOOM, "graveyard");
        }
    }
}
