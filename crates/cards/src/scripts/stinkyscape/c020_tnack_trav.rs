//! S #20 Tnack Trav (SPEC §8.10 row 20): (2) Unit, Epic, 6/6 → 12/12, of the held Stinkyscape set
//! (R1420). Not Tmack Trav, S #94 (ST9).
//!   Base:    "Cry: Copy the text of a Legendary or Mythic Unit on the field."
//!   Radiant: "Cry: Copy the Radiant text of a Legendary or Mythic Unit on the field."
//! Engine: Cry (R1): a declared target (R81), a Unit on the field, either side, not itself, whose rarity
//! is Legendary or Mythic (a token's is Token, whatever it printed; ST-S17-1, R1876003);
//! `copy_unit_text` (R1850000): its keywords, aura, triggers, hooks, Death, Activates and numbers on this
//! body until it leaves the field (R78), its own name, cost, type, tags, rarity and stats kept; the
//! copied Cry does not run. An Immutable Tnack Trav copies nothing. Radiant: `radiant: true`, the
//! target's Radiant text. No target, nothing.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-020";

/// The target check's name (`TargetFilter.check`).
const LEGEND: &str = "legendOrMythic";

fn trav(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit"], "excludeSelf": true, "check": LEGEND }),
        )],
        target_checks: IndexMap::from([(
            LEGEND,
            target_check(|args| {
                args.candidate.is_some_and(|candidate| {
                    matches!(
                        def_of(Some(args.state), &candidate.def_id).rarity,
                        Rarity::Legendary | Rarity::Mythic
                    )
                })
            }),
        )]),
        cry: Some(hook(move |_ctx| {
            vec![copy_unit_text(CopyUnitTextArgs {
                target: None,
                radiant: radiant.then_some(true),
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: trav(false),
        radiant: trav(true),
    }
}

// S #20 Tnack Trav — SPEC §8.10 row 20, BUILD M12 row S 20: "Cry (played): a Legendary or Mythic Unit on
// either side, not itself, chosen; this Unit has its keywords, aura, triggers and Death from then on, its
// own stats and name kept (R1850000); a bounce ends it; no target, nothing; radiant 12/12 and the target's
// Radiant text". The copy's reach (aura, triggers, Death, abilities) and its end are the engine's, proven
// by its tests (R1850000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    /// C #80 BOOM! Big Max, Legendary: Rush, Trample, Indestructible, Crater; Radiant: Charge for Rush.
    const BIG_MAX: &str = "classic-080";
    /// S #114 Arctic Citadel, Epic.
    const CITADEL: &str = "stinkyscape-114";
    const VANILLA: &str = "core-008"; // (1) 4/4, Common.
    const FILLER: &str = "core-005";

    fn board(radiant: bool, p2_field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": CITADEL, "lane": 2 }],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": p2_field, "library": filler(4) },
        }))
    }

    fn pick(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }], "zone": 1 })
    }

    /// The `targets` lists p1's legal plays of Tnack Trav declare.
    fn offered(s: &Scenario) -> Vec<Vec<Selection>> {
        let trav = s.hand(P1).into_iter().find(|card| card.def_id == ID).expect("Tnack Trav in hand");
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == trav.id => {
                    Some(targets.unwrap_or_default())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r1876003_only_a_legendary_or_mythic_unit_may_be_chosen() {
        let s = board(false, json!([BIG_MAX, VANILLA]));
        let max = s.unit_at(P2, 1);
        for targets in offered(&s) {
            for selection in targets {
                assert!(
                    matches!(&selection, Selection::Instance { instance_id, .. } if *instance_id == max.id),
                    "{selection:?}"
                );
            }
        }
        let vanilla = s.unit_at(P2, 2);
        let mut refused = s;
        refused.expect_refused(|s| s.play(ID, pick(&vanilla)));
    }

    #[test]
    fn r1850000_it_copies_the_text_its_own_stats_and_name_kept() {
        let mut s = board(false, json!([BIG_MAX]));
        let max = s.unit_at(P2, 1);
        s.play(ID, pick(&max));
        let trav = s.unit_at(P1, 1);
        assert_eq!(trav.def_id, ID);
        assert!(unit_has(s.state(), &trav, KeywordKind::Trample));
        assert!(unit_has(s.state(), &trav, KeywordKind::Rush));
        assert!(!unit_has(s.state(), &trav, KeywordKind::Charge));
        s.expect_stats(&trav, json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r1850000_with_no_legend_on_the_field_it_copies_nothing() {
        let mut s = board(false, json!([VANILLA]));
        s.play(ID, json!({ "zone": 1 }));
        let trav = s.unit_at(P1, 1);
        assert!(!unit_has(s.state(), &trav, KeywordKind::Trample));
        s.expect_stats(&trav, json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r275_radiant_12_12_copies_the_radiant_text() {
        let mut s = board(true, json!([BIG_MAX]));
        let max = s.unit_at(P2, 1);
        s.play(ID, pick(&max));
        let trav = s.unit_at(P1, 1);
        assert!(unit_has(s.state(), &trav, KeywordKind::Charge), "Big Max's Radiant face");
        assert!(unit_has(s.state(), &trav, KeywordKind::Trample));
        s.expect_stats(&trav, json!({ "attack": 12, "health": 12 }));
    }
}
