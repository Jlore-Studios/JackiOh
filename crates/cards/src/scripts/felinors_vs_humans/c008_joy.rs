//! FvH #8 Joy! (SPEC §8.14 row 8): (1) Spell, Common, of the Felinors vs Humans set.
//!   Base:    "Give a Unit Cleave, Lifesteal and Pierce for {turns|turn|turns}." (turns 1)
//!   Radiant: the same, turns 2.
//!
//! One declared Unit of either side (R81) is given the three keywords by `grant_for_turns` for the
//! declared `turns` (R5237001, FV2's "for a turn"): until the caster's next turn starts, or on the Radiant
//! face their second turn from now, whoever's turn it was cast on. Both faces run one script: the Radiant
//! face's 2 is its declared `turns`, read through `param` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-008";

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|ctx| {
            vec![grant_for_turns(GrantForTurnsArgs {
                target: TargetSpec::Chosen { index: None },
                keywords: vec![Keyword::Cleave, Keyword::Lifesteal, Keyword::Pierce],
                attack: None,
                turns: param(&*ctx, "turns"),
            })]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's two turns are its declared `turns`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #8 Joy! — SPEC §8.14 row 8 (R5237001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    /// A sturdy vanilla Unit for each side's lane 1.
    const WALL: &str = "stand-in (1) 1/5";
    /// Stockpile: a Spell each hand keeps, so no turn ends by itself.
    const FILLER: &str = "core-005";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "joy",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [WALL], "library": filler(6) },
            "p2": { "hand": [FILLER], "field": [WALL], "library": filler(6) },
        }))
    }

    /// Whether the Unit holds all three granted keywords; they always go together.
    fn joyful(s: &Scenario, id: &str) -> bool {
        let held: Vec<bool> = [KeywordKind::Cleave, KeywordKind::Lifesteal, KeywordKind::Pierce]
            .into_iter()
            .map(|kind| unit_has(s.state(), s.card(id), kind))
            .collect();
        assert!(held.iter().all(|has| *has == held[0]), "the three go together: {held:?}");
        held[0]
    }

    mod base {
        use super::*;

        #[test]
        fn r5237001_holds_through_the_opponents_turn_and_ends_as_the_casters_next_turn_starts() {
            let mut s = board(false);
            let wall = s.unit_at(P1, 1).id;
            assert!(!joyful(&s, &wall));
            s.play(ID, pick::targets(pick::id(&wall)));
            assert!(joyful(&s, &wall), "given as the Spell resolves");
            s.end_turn();
            assert!(joyful(&s, &wall), "and through the opponent's turn");
            s.end_turn();
            assert!(!joyful(&s, &wall), "gone as the caster's next turn starts");
        }

        #[test]
        fn it_may_be_given_to_an_enemy_unit() {
            let mut s = board(false);
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&theirs)));
            assert!(joyful(&s, &theirs));
            assert!(!joyful(&s, &s.unit_at(P1, 1).id), "the caster's own Unit gets nothing");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237001_holds_through_the_casters_next_turn_and_ends_as_their_second_starts() {
            let mut s = board(true);
            let wall = s.unit_at(P1, 1).id;
            s.play(ID, pick::targets(pick::id(&wall)));
            s.end_turn().end_turn();
            assert_eq!(s.state().active, P1);
            assert!(joyful(&s, &wall), "through the caster's next turn");
            s.end_turn();
            assert!(joyful(&s, &wall), "and the opponent's turn after it");
            s.end_turn();
            assert!(!joyful(&s, &wall), "gone as the caster's second turn from now starts");
        }
    }
}
