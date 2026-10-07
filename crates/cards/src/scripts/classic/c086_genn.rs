//! C #86 Genn (SPEC §8.6 row 86). (4) Unit, Common, 14/14 → 42/42.
//!   Base:    no text
//!   Radiant: no text; the Radiant face is its tripled stats
//!   Engine:  "A vanilla Unit: no keywords and no script, as #8 Mr. Vanilla, whose Radiant face triples
//!            its stats too. The designer printed 14/14 on both faces; R276 refuses identical faces, so
//!            the Radiant face is 42/42. Tunes: none."
//!
//! Nothing to script: both faces' stats are the catalog's (§10.4 layer 1), and a card made Radiant on
//! the field takes its Radiant face at once (§5.2). Its proof: `test/classic/086-genn.test.ts`.

use jackioh_engine::prelude::*;

pub const ID: &str = "classic-086";

pub fn script() -> CardScripts {
    let base = Script::default();

    // The same empty script: the Radiant face's 42/42 is catalog data (R276).
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #86 Genn (SPEC §8.6 row 86; BUILD M9 row C 86). (4) Unit, Common, 14/14 → 42/42: no text, a vanilla
// Unit like #8 Mr. Vanilla; the Radiant face is its tripled stats (R276).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;

    const GENN: &str = "classic-086";
    const SAINTESS: &str = "core-081"; // Death: make your other Units Radiant
    const HIT_JOB: &str = "core-016";
    const STOCKPILE: &str = "core-005";
    const VANILLA: &str = "core-008";

    use crate::scenario;

    fn def() -> CardDef {
        crate::register_all();
        crate::card_def(ID)
    }

    use crate::js;

    /// TS `const SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] }`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": spare_library() })
    }

    fn spare_library() -> Value {
        json!([VANILLA, VANILLA])
    }

    /// TS `expect(script).toEqual({})`: no hook, no declaration and no flag on the script.
    fn is_empty_script(script: &Script) -> bool {
        script.cost.is_none()
            && script.cry.is_none()
            && script.death.is_none()
            && script.start_of_game.is_none()
            && script.resume.is_empty()
            && script.delayed.is_none()
            && script.set_stat.is_none()
            && script.start_of_turn.is_none()
            && script.end_of_turn.is_none()
            && script.aura.is_none()
            && script.triggers.is_empty()
            && script.on_play_hook.is_none()
            && script.hand_triggers.is_empty()
            && script.static_flags.is_none()
            && script.targets.is_empty()
            && script.modes.is_empty()
            && script.condition_met.is_none()
            && script.preview.is_none()
            && script.activations.is_empty()
            && script.target_checks.is_empty()
            && script.cost_aura.is_none()
            && script.graveyard_play.is_none()
            && script.targeting_discards.is_none()
            && script.records_play_as.is_none()
            && script.draw_limit.is_none()
            && script.replacements.is_empty()
            && script.hero_guard.is_none()
            && script.conditional_keywords.is_none()
            && script.after_attack.is_none()
            && script.plague_multiplier.is_none()
            && script.deck_triggers.is_empty()
            && script.graveyard_triggers.is_empty()
            && script.quests.is_none()
            && script.tribute_when.is_none()
            && script.would_counter.is_none()
            && script.start_of_opponent_turn.is_none()
    }

    /// base
    mod base {
        use super::*;

        /// §8.6 a 14/14 with no text, no keywords and no script, as #8 Mr. Vanilla
        #[test]
        fn s8_6_a_14_14_with_no_text_no_keywords_and_no_script_as_n8_mr_vanilla() {
            let mut s = scenario(json!({ "p1": { "hand": [GENN, STOCKPILE], "library": spare_library() }, "p2": spare() }));
            s.play(GENN, json!({}));
            s.expect_stats(GENN, json!({ "attack": 14, "health": 14 }));
            assert!(s.stats(GENN).keywords.is_empty());
            assert_eq!(json!([def().base.text, js(&def().base.keywords)]), json!(["", []]));
            assert!(is_empty_script(&script().base));
        }
    }

    /// radiant
    mod radiant {
        use super::*;

        /// R276 42/42 with no text: tripled as #8's, so the faces differ
        #[test]
        fn r276_42_42_with_no_text_tripled_as_n8_s_so_the_faces_differ() {
            let mut s = scenario(json!({ "p1": { "field": [{ "def": GENN, "radiant": true }] } }));
            s.expect_stats(GENN, json!({ "attack": 42, "health": 42 }));
            assert_eq!(json!([def().radiant.text, js(&def().radiant.keywords)]), json!(["", []]));
            // TS `expect(radiant).toBe(base)`: the one empty script on both faces.
            let scripts = script();
            assert!(is_empty_script(&scripts.radiant) && is_empty_script(&scripts.base));
        }

        /// §5.2 made Radiant on the field, it takes that face at once
        #[test]
        fn s5_2_made_radiant_on_the_field_it_takes_that_face_at_once() {
            let mut s = scenario(json!({
                "p1": { "field": [GENN, SAINTESS], "hand": [HIT_JOB, STOCKPILE], "library": spare_library() },
                "p2": spare()
            }));
            let saintess = s.card(SAINTESS).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": saintess }] }));
            assert!(s.card(GENN).radiant);
            s.expect_stats(GENN, json!({ "attack": 42, "health": 42 }));
        }
    }
}
