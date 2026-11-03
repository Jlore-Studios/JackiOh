//! S #103 Vxyl (SPEC §8.10 row S 103; §4.4, §6.1, R348, R783004, R783005, R783006): (2) Unit, Human,
//! Rare, 1/4 → 2/8.
//!   Base:    "Cloaked, Poisonous / Cry: Add a No-No Words to your hand."
//!   Radiant: "Cloaked, Poisonous, Divine Shield / Cry: Add a No-No Words to your hand."
//!
//! Engine: its keywords are the catalog's. Cloaked (R783004–R783006): it may be set face-down at its
//! own (2) as a Trap and revealed by its Activate: Reveal, which takes it into a unit zone and runs
//! its Cry; the designer's "On reveal" is that Cry (ST-S14-5), so a face-up play runs it too. The Cry
//! is one `add_to_hand` of a base S #99 No-No Words, whose X its player chooses as they play it
//! (R348), and nothing while the card is set (`is_set`).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-103";

/// S #99 No-No Words.
const NO_NO_WORDS: &str = "stinkyscape-099";

/// R783004: a card set by its Cloaked is a Field Trap whose own text is inert until it reveals, and
/// the reveal clears its stamp before its Cry runs (R783006). The play pipeline runs the Cry of a card
/// it sets face-down all the same (§10.5 step 5, `play_steps::resolve_step`), so the Cry reads the
/// stamp and adds nothing while the card is set: one No-No Words, at the reveal.
fn is_set(ctx: &EffectContext<'_>) -> bool {
    ctx.self_.as_ref().is_some_and(|card| card.set_as.is_some())
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            if is_set(ctx) {
                return vec![];
            }
            vec![add_to_hand(json_as(json!({ "defId": NO_NO_WORDS })))]
        })),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #103 Vxyl — SPEC §8.10 row S 103, BUILD M12 row S 103.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VXYL: &str = "stinkyscape-103";
    const NONO: &str = "stinkyscape-099";
    const GENN: &str = "classic-086"; // (4) Unit 14/14.
    const FILLER: &str = "core-005";

    fn nonos(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1).into_iter().filter(|card| card.def_id == NONO).collect()
    }

    fn vxyl_board(radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "hand": [{ "def": VXYL, "radiant": radiant }, FILLER] },
            "p2": { "hand": [FILLER] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn played_face_up_it_adds_one_base_no_no_words() {
            let mut s = vxyl_board(false);

            s.play(VXYL, json!({ "zone": 1 }));

            s.expect_stats(s.unit_at(P1, 1), json!({ "attack": 1, "health": 4 }));
            let made = nonos(&s);
            assert_eq!(made.len(), 1);
            assert_eq!(made[0].fixed_x, None, "its X is chosen as it is played (R348)");
            assert!(!made[0].radiant);
        }

        #[test]
        fn r783006_revealed_it_takes_a_unit_zone_and_adds_one() {
            let mut s = vxyl_board(false);
            s.play(VXYL, json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }));
            let set = s.backrow(P1, 1).expect("the set Vxyl").id;
            assert!(nonos(&s).is_empty(), "set face-down, its Cry waits");

            s.activate(&set, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));

            let unit = s.unit(P1, 1).expect("it took a unit zone");
            assert_eq!(unit.def_id, VXYL);
            assert_eq!(nonos(&s).len(), 1);
        }

        #[test]
        fn its_hit_destroys_the_unit_it_damages() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [VXYL] },
                "p2": { "hand": [FILLER], "field": [GENN] },
            }));
            let genn = s.unit_at(P2, 1);

            s.attack(VXYL, &genn);

            s.expect_in_zone(&genn, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_2_8_with_divine_shield_and_adds_a_base_no_no_words() {
            let mut s = vxyl_board(true);

            s.play(VXYL, json!({ "zone": 1 }));

            let unit = s.unit_at(P1, 1);
            s.expect_stats(&unit, json!({ "attack": 2, "health": 8 }));
            let kinds: Vec<KeywordKind> = s.stats(&unit).keywords.iter().map(Keyword::kind).collect();
            for kind in [KeywordKind::Cloaked, KeywordKind::Poisonous, KeywordKind::DivineShield] {
                assert!(kinds.contains(&kind), "{kind:?} in {kinds:?}");
            }
            let made = nonos(&s);
            assert_eq!(made.len(), 1);
            assert!(!made[0].radiant);
        }
    }
}
