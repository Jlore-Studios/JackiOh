//! FvH #7 Whimsy! (SPEC §8.14 row 7): (1) Spell, Common, of the Felinors vs Humans set.
//!   Base:    "Give a Unit Immune to Spells and Indestructible for {turns|turn|turns}." (turns 1)
//!   Radiant: the same, turns 2.
//!
//! One declared Unit of either side (R81) is given both keywords by `grant_for_turns` for the declared
//! `turns` (R5237001, FV2's "for a turn"): they hold until the caster has started that many more turns,
//! so "for a turn" covers the opponent's turn and ends as the caster's next turn starts, and "for two
//! turns" ends as their second turn from now starts. They are given as the Spell resolves, so the Immune
//! to Spells it grants does not stop the Spell itself. Both faces run one script: the Radiant face's 2 is
//! its declared `turns`, read through `param` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-007";

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(|ctx| {
            vec![grant_for_turns(GrantForTurnsArgs {
                target: TargetSpec::Chosen { index: None },
                keywords: vec![Keyword::ImmuneToSpells, Keyword::Indestructible],
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

// FvH #7 Whimsy! — SPEC §8.14 row 7 (R5237001).
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
    /// Hit Job: the opponent's removal, a Spell.
    const KILL: &str = "core-016";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "whimsy",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": [WALL], "library": filler(6) },
            "p2": { "hand": [FILLER, KILL], "field": [WALL], "library": filler(6) },
        }))
    }

    /// Whether the Unit holds both granted keywords; the two always go together.
    fn shielded(s: &Scenario, id: &str) -> bool {
        let immune = unit_has(s.state(), s.card(id), KeywordKind::ImmuneToSpells);
        let indestructible = unit_has(s.state(), s.card(id), KeywordKind::Indestructible);
        assert_eq!(immune, indestructible, "Immune to Spells and Indestructible go together");
        immune
    }

    mod base {
        use super::*;

        #[test]
        fn r5237001_holds_through_the_opponents_turn_and_ends_as_the_casters_next_turn_starts() {
            let mut s = board(false);
            let wall = s.unit_at(P1, 1).id;
            assert!(!shielded(&s, &wall));
            s.play(ID, pick::targets(pick::id(&wall)));
            assert!(shielded(&s, &wall), "given as the Spell resolves");
            s.end_turn();
            assert!(shielded(&s, &wall), "and through the opponent's turn");
            s.end_turn();
            assert!(!shielded(&s, &wall), "gone as the caster's next turn starts");
        }

        #[test]
        fn the_opponents_spell_cannot_aim_at_it_on_their_turn() {
            let mut s = board(false);
            let wall = s.unit_at(P1, 1).id;
            s.play(ID, pick::targets(pick::id(&wall)));
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let kill = s.card(KILL).id.clone();
            let aims = pick::instance_ids(&s.play_targets(&kill));
            assert!(!aims.contains(&wall), "Immune to Spells keeps Hit Job off it: {aims:?}");
            let theirs = s.unit_at(P2, 1).id;
            assert!(aims.contains(&theirs), "their own Unit is still an aim");
        }

        #[test]
        fn it_may_be_given_to_an_enemy_unit() {
            let mut s = board(false);
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&theirs)));
            assert!(shielded(&s, &theirs));
            assert!(!shielded(&s, &s.unit_at(P1, 1).id), "the caster's own Unit gets nothing");
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
            assert!(shielded(&s, &wall), "through the caster's next turn");
            s.end_turn();
            assert!(shielded(&s, &wall), "and the opponent's turn after it");
            s.end_turn();
            assert!(!shielded(&s, &wall), "gone as the caster's second turn from now starts");
        }
    }
}
