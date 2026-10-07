//! C+ #9 Silence (SPEC §8.7 row 9, R407). (0) Spell, Common.
//! Vanilla a target Unit on either side (§6.3; Immutable refuses it, R23). Radiant: any permanent, a
//! backrow card face-down or not included (R407: an aura stops, a trap never fires, Animated is lost).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-009";

/// `of` is TS `TargetFilter["of"]`, written as its literal.
fn silence(of: Value) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": of }))],
        cry: Some(hook(|_ctx| vec![vanilla(Default::default())])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: silence(json!(["unit"])),
        radiant: silence(json!(["unit", "backrow"])),
    }
}

// C+ #9 Silence — SPEC §8.7 row 9, BUILD M9 Classic+ row C+ 9: "Vanilla a target Unit on either side:
// printed keywords and text gone, stats, buffs, damage and granted keywords kept (§10.4); an Immutable
// Unit is unchanged (R23); radiant any permanent (R407): a Vanilla Field Spell's aura stops, a Vanilla
// face-down trap never fires and sits inert, an Animated card loses Animated where it stands and no
// longer moves; an enemy face-down trap is offered by id only and the Vanilla on it names nothing to the
// caster (R97, R177)".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const SILENCE: &str = "classicplus-009";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt. End of turn: heal to full. Radiant: Immutable.
    const SURGERY: &str = "core-063"; // (1) Give target Unit +3/+3 and 1 random keyword.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const WEAPONS: &str = "core-014"; // Field Spell: your Units have +4 attack, Rush and First Strike.
    const SHEEPISH: &str = "core-041"; // (1) Trap: transforms the opponent's played Unit.
    const TESLA: &str = "classic-005"; // Animated Field Trap: fires when the opponent summons a Unit.
    const FROSTSPATULA: &str = "classicplus-012-8"; // Field Spell token: Animated on your turn, Rush.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    use crate::merged;

    fn setup(p1: Value, p2: Value, radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": SILENCE, "radiant": radiant_face }, FILLER],
                    "library": [STOCKPILE, STOCKPILE],
                    "mana": 8,
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [STOCKPILE, STOCKPILE] }), p2),
        }))
    }

    fn at(id: &str) -> Value {
        json!([{ "pick": "instance", "instanceId": id }])
    }

    fn kinds(s: &Scenario, id: &str) -> Vec<KeywordKind> {
        s.stats(id).keywords.iter().map(|k| k.kind()).collect()
    }

    fn count(s: &Scenario, type_: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == type_).count()
    }

    #[test]
    fn is_a_0_spell_targeting_a_unit_radiant_a_unit_or_a_backrow_card_on_either_side() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(def.cost, CardCost::Fixed(0));
        let scripts = script();
        assert_eq!(
            serde_json::to_value(&scripts.base.targets).unwrap(),
            json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit"] } }])
        );
        assert_eq!(
            serde_json::to_value(&scripts.radiant.targets).unwrap(),
            json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "any", "of": ["unit", "backrow"] } }])
        );
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_an_enemy_unit_loses_its_printed_keywords_and_text_and_keeps_its_stats_buffs_damage_and_granted_keywords() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [{ "def": SILENCE }, SURGERY, FILLER] }),
                json!({ "field": [{ "def": MENACE, "damage": 3 }] }),
                false,
            );
            let menace = s.card(MENACE).clone();
            s.play(SURGERY, json!({ "targets": at(&menace.id) }));
            let granted: Vec<KeywordKind> = s.card(&menace).granted_keywords.iter().map(|k| k.kind()).collect();
            assert_eq!(granted.len(), 1);

            s.play(SILENCE, json!({ "targets": at(&menace.id) }));

            assert!(s.card(&menace).vanilla);
            s.expect_stats(&menace, json!({ "attack": 12, "maxHealth": 12, "health": 9 }));
            let now = kinds(&s, &menace.id);
            assert_eq!(now, granted);
            assert!(!now.contains(&KeywordKind::Taunt)); // R21 never grants a keyword the unit has, so Taunt was printed only
            s.expect_events(json!(["transformed"]));
        }

        #[test]
        fn its_text_is_gone_a_vanilla_midrange_menace_no_longer_heals_to_full_at_end_of_turn() {
            crate::register_all();
            let mut s = setup(json!({ "field": [{ "def": MENACE, "damage": 4 }] }), json!({}), false);
            let menace = s.card(MENACE).clone();

            s.play(SILENCE, json!({ "targets": at(&menace.id) }));
            s.end_turn();

            s.expect_stats(&menace, json!({ "health": 5 }));
        }

        #[test]
        fn reaches_your_own_unit_too() {
            crate::register_all();
            let mut s = setup(json!({ "field": [MENACE] }), json!({}), false);

            let menace = s.card(MENACE).id.clone();
            s.play(SILENCE, json!({ "targets": at(&menace) }));

            assert!(!kinds(&s, MENACE).contains(&KeywordKind::Taunt));
        }

        #[test]
        fn r23_an_immutable_unit_is_unchanged() {
            crate::register_all();
            let mut s = setup(json!({}), json!({ "field": [{ "def": MENACE, "radiant": true }] }), false);
            let menace = s.card(MENACE).clone();

            s.play(SILENCE, json!({ "targets": at(&menace.id) }));

            assert!(!s.card(&menace).vanilla);
            let now = kinds(&s, &menace.id);
            assert!(now.contains(&KeywordKind::Taunt));
            assert!(now.contains(&KeywordKind::Immutable));
            s.expect_in_zone(SILENCE, "graveyard");
        }

        #[test]
        fn r81_a_backrow_card_is_no_target_for_the_base_face_with_no_unit_on_the_field_it_is_played_and_fizzles_s8() {
            crate::register_all();
            let mut s = setup(json!({ "backrow": [WEAPONS] }), json!({}), false);
            let weapons = s.card(WEAPONS).id.clone();
            s.expect_refused(|s| s.play(SILENCE, json!({ "targets": at(&weapons) })));

            s.play(SILENCE, json!({}));

            s.expect_in_zone(SILENCE, "graveyard");
            assert!(!s.card(WEAPONS).vanilla);
            assert_eq!(count(&s, "transformed"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r407_a_vanilla_field_spell_s_aura_stops() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA], "backrow": [WEAPONS] }), json!({}), true);
            s.expect_stats(VANILLA, json!({ "attack": 8 }));

            let weapons = s.card(WEAPONS).id.clone();
            s.play(SILENCE, json!({ "targets": at(&weapons) }));

            s.expect_stats(VANILLA, json!({ "attack": 4 }));
            assert!(!kinds(&s, VANILLA).contains(&KeywordKind::Rush));
            assert!(s.card(WEAPONS).vanilla);
        }

        #[test]
        fn r407_a_vanilla_face_down_trap_never_fires_and_sits_inert_in_its_zone_still_face_down() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [{ "def": SILENCE, "radiant": true }, VANILLA, FILLER] }),
                json!({ "backrow": [{ "def": SHEEPISH, "faceUp": false, "lane": 2 }] }),
                true,
            );
            let trap = s.backrow(P2, 2).unwrap();

            s.play(SILENCE, json!({ "targets": at(&trap.id) }));
            s.play(VANILLA, json!({ "zone": 1 }));

            assert_eq!(count(&s, "trapFired"), 0);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(VANILLA.to_string()));
            assert!(matches!(s.card(&trap).zone, Zone::Field { row: Row::Backrow, lane: 2, .. }));
            assert_eq!(s.card(&trap).face_up, Some(false));
            assert!(s.card(&trap).vanilla);
        }

        #[test]
        fn r97_r177_an_enemy_face_down_trap_is_offered_by_id_only_and_the_vanilla_on_it_names_nothing_to_the_caster() {
            crate::register_all();
            let mut s = setup(json!({}), json!({ "backrow": [{ "def": SHEEPISH, "faceUp": false, "lane": 2 }] }), true);
            let trap = s.backrow(P2, 2).unwrap();
            let silence = s.card(SILENCE).clone();
            let offers: Vec<ActionBody> = legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == silence.id))
                .collect();
            let offered = serde_json::to_string(&offers).unwrap();
            assert!(offered.contains(&trap.id));
            assert!(!offered.contains(SHEEPISH));

            s.play(SILENCE, json!({ "targets": at(&trap.id) }));

            assert!(!serde_json::to_string(&s.view(P1)).unwrap().contains(SHEEPISH));
            assert!(serde_json::to_string(&s.view(P2).you.backrow).unwrap().contains(SHEEPISH));
        }

        #[test]
        fn r407_r383_a_face_down_animated_trap_made_vanilla_never_fires_so_never_animates() {
            crate::register_all();
            let mut s = setup(
                json!({ "hand": [{ "def": SILENCE, "radiant": true }, VANILLA, FILLER] }),
                json!({ "backrow": [{ "def": TESLA, "faceUp": false, "lane": 3 }] }),
                true,
            );
            let tesla = s.backrow(P2, 3).unwrap();

            s.play(SILENCE, json!({ "targets": at(&tesla.id) }));
            s.play(VANILLA, json!({ "zone": 1 }));

            assert_eq!(count(&s, "trapFired") + count(&s, "animated"), 0);
            s.expect_stats(VANILLA, json!({ "health": 4 }));
            assert!(matches!(s.card(&tesla).zone, Zone::Field { row: Row::Backrow, lane: 3, .. }));
        }

        #[test]
        fn r407_r383_an_animated_on_your_turn_card_made_vanilla_in_its_backrow_zone_no_longer_animates() {
            crate::register_all();
            let mut s = setup(json!({ "backrow": [{ "def": FROSTSPATULA, "lane": 2 }] }), json!({}), true);
            let spatula = s.card(FROSTSPATULA).clone();

            s.play(SILENCE, json!({ "targets": at(&spatula.id) }));
            s.end_turn();
            s.end_turn();

            assert_eq!(s.state().active, P1);
            assert!(matches!(s.card(&spatula).zone, Zone::Field { row: Row::Backrow, lane: 2, .. }));
            assert_eq!(count(&s, "animated"), 0);
        }

        #[test]
        fn r407_r383_one_made_vanilla_while_animated_stays_a_unit_and_no_longer_returns_at_cleanup() {
            crate::register_all();
            let mut s = setup(json!({ "backrow": [{ "def": FROSTSPATULA, "lane": 2 }] }), json!({}), true);
            let spatula = s.card(FROSTSPATULA).clone();
            s.start_turn();
            assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(spatula.id.clone()));

            s.play(SILENCE, json!({ "targets": at(&spatula.id) }));
            s.end_turn();

            assert!(matches!(s.card(&spatula).zone, Zone::Field { row: Row::Units, lane: 2, .. }));
            assert_eq!(count(&s, "deanimated"), 0);
        }
    }
}
