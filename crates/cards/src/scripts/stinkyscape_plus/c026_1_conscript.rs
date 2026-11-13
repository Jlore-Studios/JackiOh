//! SPEC §8.12 S+ #26.1 Conscript, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    1/1, Ranged.
//!   Radiant: 2/2, Ranged, Rush, Pierce.
//!
//! Its catalog row, sound, flavour and Chinese entries are already written.
//!
//! All keywords are catalog data, so there is nothing to run: S+ #26 summons it, base or Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-026-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // All faces are catalog data: base 1/1 Ranged, Radiant 2/2 Ranged, Rush, Pierce.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #26.1 Conscript — SPEC §8.12 row S+ 26.1: a keyword-only token, no script on either face.
#[cfg(test)]
mod tests {
    use super::*;

    use crate::scenario;

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(|k| k.kind()).collect()
    }

    /// A script with no hook, no declaration and no flag.
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
    fn both_faces_have_no_script() {
        let scripts = script();
        assert!(is_empty_script(&scripts.base));
        // The one empty script runs both faces.
        assert!(is_empty_script(&scripts.radiant));
    }

    #[test]
    fn base_1_1_ranged_and_radiant_2_2_rush_pierce() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert!(def.is_token());
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(1), Some(1), Some(2), Some(2)]
        );
        assert_eq!(kinds(&def.base.keywords), vec![KeywordKind::Ranged]);
        assert_eq!(
            kinds(&def.radiant.keywords),
            vec![KeywordKind::Ranged, KeywordKind::Rush, KeywordKind::Pierce]
        );
    }

    #[test]
    fn summoned_it_stands_as_its_face_prints() {
        for (radiant, attack, health) in [(false, 1, 1), (true, 2, 2)] {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": radiant }] },
                "p2": {},
            }));
            s.expect_stats(ID, json!({ "attack": attack, "health": health, "maxHealth": health }));
        }
    }
}
