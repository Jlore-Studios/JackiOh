//! SPEC §8.14 FvH #60 Toolmaster Luna ((2) Unit, Human, Legendary, 2/4 → 4/8), a card of the held Felinors
//! vs Humans set. Built by S10 (#5260).
//!   Base:    "Cry: Choose one: Summon the Tools of Creation, or the Tools of Cremation."
//!   Radiant: "Cry: Choose one: Summon the Radiant Tools of Creation, or the Radiant Tools of Cremation."
//!
//! Cry (R1) with two modes, declared with the play (R81), so the AI, the random policy and the timeout
//! answer it as they build the play: `summon` of FvH #60.1 Tools of Creation or #60.2 Tools of Cremation,
//! a Field Spell token, into its controller's leftmost open backrow zone (R64), summoned so it runs no
//! Cry; Radiant on the Radiant face. A full backrow summons nothing (R129). A play that carried no mode
//! fizzles (R90). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-060";

/// FvH #60.1 and #60.2, the two Field Spells it may summon.
const CREATION: &str = "felinorsvshumans-060-1";
const CREMATION: &str = "felinorsvshumans-060-2";

/// The two modes, as the play action spells them (R81).
const CREATION_MODE: &str = "Summon the Tools of Creation";
const CREMATION_MODE: &str = "Summon the Tools of Cremation";

fn luna(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![CREATION_MODE.to_string(), CREMATION_MODE.to_string()],
        }],
        cry: Some(hook(move |ctx| {
            let tools = match chosen_options(ctx).first().map(String::as_str) {
                Some(CREATION_MODE) => CREATION,
                Some(CREMATION_MODE) => CREMATION,
                _ => return vec![],
            };
            vec![summon(json_as(json!({ "defId": tools, "radiant": radiant })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: luna(false),
        radiant: luna(true),
    }
}

// FvH #60 Toolmaster Luna — SPEC §8.14 row 60 (R1, R64, R81, R129).
#[cfg(test)]
mod tests {
    use super::{CREATION, CREATION_MODE, CREMATION, CREMATION_MODE, ID};
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;

    /// A Field Spell already standing in p1's backrow.
    const FIELD: &str = "stand-in (1) Field Spell";

    fn luna_played(radiant: bool, mode: &str, backrow: Value) -> Scenario {
        let mut s = weapon_board(
            "fvh060",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "backrow": backrow, "mana": 10 }),
            json!({}),
        );
        s.play(ID, json!({ "modes": [mode] }));
        s
    }

    /// p1's backrow, lane by lane: each card's def id and whether it is Radiant.
    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5).map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant))).collect()
    }

    fn tools(def_id: &str, radiant: bool) -> Option<(String, bool)> {
        Some((def_id.to_string(), radiant))
    }

    fn summoned(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn offers_both_modes_with_the_play() {
        let s = weapon_board("fvh060-modes", json!({ "hand": [ID], "mana": 10 }), json!({}));
        let modes: Vec<Vec<String>> = s
            .plays_of(ID)
            .into_iter()
            .filter_map(|play| match play {
                ActionBody::Play { modes, .. } => modes,
                _ => None,
            })
            .collect();
        assert!(modes.contains(&vec![CREATION_MODE.to_string()]));
        assert!(modes.contains(&vec![CREMATION_MODE.to_string()]));
    }

    mod base {
        use super::*;

        #[test]
        fn r64_summons_the_tools_of_creation_into_the_leftmost_open_backrow_zone() {
            let mut s = luna_played(false, CREATION_MODE, json!([{ "def": FIELD, "lane": 1 }]));
            assert_eq!(backrow(&s)[1], tools(CREATION, false));
            assert_eq!(summoned(&s), [CREATION]);
            s.expect_in_zone(ID, "field");
        }

        #[test]
        fn summons_the_tools_of_cremation() {
            let s = luna_played(false, CREMATION_MODE, json!([]));
            assert_eq!(backrow(&s)[0], tools(CREMATION, false));
            assert_eq!(summoned(&s), [CREMATION]);
        }

        #[test]
        fn r129_a_full_backrow_summons_nothing() {
            let s = luna_played(false, CREATION_MODE, json!([FIELD, FIELD, FIELD, FIELD, FIELD]));
            assert!(summoned(&s).is_empty());
            assert!(backrow(&s).iter().all(|card| card.as_ref().is_some_and(|(def, _)| def == FIELD)));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_the_radiant_tools_of_creation() {
            let s = luna_played(true, CREATION_MODE, json!([]));
            assert_eq!(backrow(&s)[0], tools(CREATION, true));
        }

        #[test]
        fn summons_the_radiant_tools_of_cremation() {
            let s = luna_played(true, CREMATION_MODE, json!([]));
            assert_eq!(backrow(&s)[0], tools(CREMATION, true));
        }
    }
}
