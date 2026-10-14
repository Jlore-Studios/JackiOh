//! C+ #6 Wrong-House Attacker (SPEC §8.7 row 6). (1) Unit, Human, Common, 1/1 → 2/2.
//! Rush, Lifesteal, Poisonous (Radiant: plus Reborn): keywords only, printed in the catalog and applied
//! by the layers (§10.4), so neither face has anything to run.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-006";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Reborn and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// C+ #6 Wrong-House Attacker — SPEC §8.7 row 6, BUILD M9 Classic+ row C+ 6: "Rush, Lifesteal,
// Poisonous: attacks a unit on its summon turn but not the hero, its hit destroys any unit it damages
// and heals your hero by the amount dealt; radiant 2/2 with Reborn, returning once at 1 health without
// Reborn". Keywords only: both faces are catalog data, applied by the layers (§10.4).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const WRONG: &str = "classicplus-006";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4, no keywords.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";

    fn setup(radiant_face: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [{ "def": WRONG, "radiant": radiant_face }, HIT_JOB, HIT_JOB, FILLER],
                "library": [STOCKPILE, STOCKPILE],
                "health": 20,
                "mana": 8,
            },
            "p2": { "hand": [FILLER], "library": [STOCKPILE], "field": [MENACE] },
        }))
    }

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(|k| k.kind()).collect()
    }

    /// TS `expect(face).toEqual({})`: a script with no hook, no declaration and no flag.
    fn is_empty_script(face: &Script) -> bool {
        face.cost.is_none()
            && face.cry.is_none()
            && face.death.is_none()
            && face.start_of_game.is_none()
            && face.resume.is_empty()
            && face.delayed.is_none()
            && face.set_stat.is_none()
            && face.start_of_turn.is_none()
            && face.end_of_turn.is_none()
            && face.aura.is_none()
            && face.triggers.is_empty()
            && face.on_play_hook.is_none()
            && face.hand_triggers.is_empty()
            && face.static_flags.is_none()
            && face.targets.is_empty()
            && face.modes.is_empty()
            && face.condition_met.is_none()
            && face.preview.is_none()
            && face.activations.is_empty()
            && face.target_checks.is_empty()
            && face.cost_aura.is_none()
            && face.graveyard_play.is_none()
            && face.targeting_discards.is_none()
            && face.records_play_as.is_none()
            && face.draw_limit.is_none()
            && face.replacements.is_empty()
            && face.hero_guard.is_none()
            && face.conditional_keywords.is_none()
            && face.after_attack.is_none()
            && face.plague_multiplier.is_none()
            && face.deck_triggers.is_empty()
            && face.graveyard_triggers.is_empty()
            && face.quests.is_none()
            && face.tribute_when.is_none()
            && face.would_counter.is_none()
            && face.start_of_opponent_turn.is_none()
    }

    #[test]
    fn is_a_1_1_1_human_unit_with_rush_lifesteal_poisonous_radiant_2_2_plus_reborn_no_script_on_either_face() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(1), Some(1), Some(2), Some(2)]
        );
        assert_eq!(
            kinds(&def.base.keywords),
            vec![KeywordKind::Rush, KeywordKind::Lifesteal, KeywordKind::Poisonous]
        );
        assert_eq!(
            kinds(&def.radiant.keywords),
            vec![KeywordKind::Rush, KeywordKind::Lifesteal, KeywordKind::Poisonous, KeywordKind::Reborn]
        );
        let scripts = script();
        assert!(is_empty_script(&scripts.base));
        // TS `expect(radiant).toBe(base)`: the one empty script runs both faces.
        assert!(is_empty_script(&scripts.radiant));
    }

    mod base {
        use super::*;

        #[test]
        fn rush_on_its_summon_turn_it_may_attack_a_unit_but_not_the_hero_which_it_may_attack_a_turn_later() {
            crate::register_all();
            // No Taunt on p2's side, so only summoning sickness can refuse the swing at the hero.
            let mut s = scenario(json!({
                "p1": { "hand": [WRONG, FILLER], "library": [STOCKPILE, STOCKPILE], "mana": 8 },
                "p2": { "hand": [FILLER], "library": [STOCKPILE, STOCKPILE], "field": [VANILLA] },
            }));
            s.play(WRONG, json!({ "zone": 1 }));

            s.expect_refused(|s| s.attack(WRONG, "hero"));
            s.end_turn().end_turn();
            s.attack(WRONG, "hero");
            s.expect_health(P2, 29);
        }

        #[test]
        fn rush_on_its_summon_turn_it_attacks_a_unit() {
            crate::register_all();
            let mut s = setup(false);
            s.play(WRONG, json!({ "zone": 1 }));

            s.attack(WRONG, MENACE);

            assert!(s.events().iter().any(|event| event.event_type() == GameEventType::AttackDeclared));
        }

        #[test]
        fn poisonous_and_lifesteal_its_hit_destroys_the_9_9_it_damages_and_heals_your_hero_by_the_1_dealt() {
            crate::register_all();
            let mut s = setup(false);
            let menace = s.card(MENACE).clone();
            s.play(WRONG, json!({ "zone": 1 }));

            s.attack(WRONG, &menace);

            s.expect_in_zone(&menace, "graveyard");
            s.expect_in_zone(WRONG, "graveyard");
            s.expect_health(P1, 21);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r64_reborn_it_returns_once_at_1_health_without_reborn_then_dies_for_good() {
            crate::register_all();
            let mut s = setup(true);
            let wrong = s.card(WRONG).clone();
            s.play(&wrong, json!({ "zone": 1 }));
            s.expect_stats(&wrong, json!({ "attack": 2, "health": 2 }));

            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": wrong.id }] }));

            let back = s.unit(P1, 1).expect("the Reborn body");
            assert_eq!(back.def_id, WRONG);
            s.expect_stats(&back, json!({ "attack": 2, "health": 1 }));
            let now = kinds(&s.stats(&back).keywords);
            assert!(!now.contains(&KeywordKind::Reborn));
            for kind in [KeywordKind::Rush, KeywordKind::Lifesteal, KeywordKind::Poisonous] {
                assert!(now.contains(&kind));
            }

            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": back.id }] }));

            s.expect_in_zone(&wrong, "graveyard");
            assert!(s.unit(P1, 1).is_none());
        }
    }
}
