//! SPEC §8.12 S+ #46 Phase Fabric (§6.1 Immutable, Immune to Spells, Evasive; R81, R781001, R1420,
//! R3535000, R3543002, R3547000, R3547002): (1) Spell, Epic, Mindustrial, Mineral, a card of the held
//! Stinkyscape+ set.
//!   Base:    "Give a Unit Immutable, Immune to Spells and Divine Shield."
//!   Radiant: "Give a Unit Immutable, Immune to Spells, Divine Shield and Can't be Attacked."
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then `grant_keyword` Immutable
//! and Immune to Spells, and on the Radiant face Evasive, which is "Can't be attacked" as a keyword
//! (R3543002, the issue's reading): no attack may target the Unit, it implies Can't Defend without
//! adding it, and effects still reach it. Immutable and Immune to Spells bar every Spell's pick of the
//! Unit from then on, this card's own among them (R781001, E35); Immune to Spells is given last, so the
//! Spell's own remaining grants are not turned away by it (R3547002). Declares no numbers. Issue #3951
//! added Divine Shield to both faces; through v0.6.2b neither gave it, which `script_v0_6_2b` keeps for
//! those replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-046";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// The Radiant face adds Evasive to the two the base face gives. Immune to Spells goes on last: once
/// it stands, "a Spell doesn't affect it" (E35) turns away every later effect of this same Spell, so
/// the three are given together as R3547002 reads them and the printed order is not observable.
fn phase_fabric(radiant: bool, shield: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            let mut kinds = vec!["Immutable"];
            if shield {
                kinds.push("Divine Shield");
            }
            if radiant {
                kinds.push("Evasive");
            }
            kinds.push("Immune to Spells");
            kinds
                .into_iter()
                .map(|kind| grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": kind } }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: phase_fabric(false, true),
        radiant: phase_fabric(true, true),
    }
}

/// R767000: the scripts through v0.6.2b, which gave no Divine Shield (issue #3951). `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_2b() -> CardScripts {
    CardScripts {
        base: phase_fabric(false, false),
        radiant: phase_fabric(true, false),
    }
}

// S+ #46 Phase Fabric — SPEC §8.12 row S+ 46 (R81, R781001, R3543002, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FABRIC: &str = "stinkyscapeplus-046";
    const CRYO: &str = "stinkyscapeplus-022"; // Cryofluid: a Spell that declares one Unit, for the "no Spell picks it" check.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const WALL: &str = "stand-in (1) 1/30";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FABRIC, "radiant": radiant }, CRYO, FILLER], "field": mine, "mana": 8 },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    /// The picks `spell`'s play is offered p1.
    fn offered(s: &Scenario, spell: &str) -> Vec<Selection> {
        let spell = s.card(spell).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == spell => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = offered(&s, FABRIC);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn immutable_immune_to_spells_and_divine_shield_and_no_evasive() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Immutable), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::ImmuneToSpells), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Evasive), 0);
            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn no_spell_may_pick_the_unit_afterwards() {
            let mut s = board(false, json!([VANILLA, VANILLA]), json!([]));
            let (fabric, other) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            assert!(offered(&s, CRYO).contains(&Selection::Instance { instance_id: fabric.id.clone() }));
            s.play(FABRIC, pick::targets(pick::instance(&fabric)));
            let offered = offered(&s, CRYO);
            assert!(!offered.contains(&Selection::Instance { instance_id: fabric.id }));
            assert!(offered.contains(&Selection::Instance { instance_id: other.id }));
        }

        #[test]
        fn an_attack_still_reaches_the_unit() {
            let mut s = board(false, json!([VANILLA]), json!([WALL]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(FABRIC, pick::targets(pick::instance(&theirs)));
            s.attack(&mine, &theirs);
            assert_eq!(s.card(&theirs).damage, 0, "its Divine Shield took the hit");
            assert_eq!(keyword_count(&s, &theirs, KeywordKind::DivineShield), 0, "the attack reached it");
        }

        /// R767000: through v0.6.2b it gave no Divine Shield (issue #3951).
        #[test]
        fn r767000_the_script_kept_for_v0_6_2b_gives_no_divine_shield() {
            crate::under_kept(FABRIC, super::super::script_v0_6_2b(), || {
                let mut s = board(false, json!([VANILLA]), json!([]));
                let unit = s.unit_at(P1, 1);
                s.play(FABRIC, pick::targets(pick::instance(&unit)));
                assert_eq!(keyword_count(&s, &unit, KeywordKind::Immutable), 1);
                assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 0);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3547002_immutable_immune_to_spells_divine_shield_and_evasive_all_land() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Immutable), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::ImmuneToSpells), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Evasive), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::CantDefend), 0, "R3543002: implied, never added");
        }

        #[test]
        fn r3543002_no_attack_may_target_it_afterwards() {
            let mut s = board(true, json!([VANILLA]), json!([WALL]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(FABRIC, pick::targets(pick::instance(&theirs)));
            s.expect_refused_with(|s| s.attack(&mine, &theirs), "cannot be attacked");
            assert_eq!(s.card(&theirs).damage, 0);
        }

        #[test]
        fn no_spell_may_pick_the_unit_afterwards() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert!(!offered(&s, CRYO).contains(&Selection::Instance { instance_id: unit.id }));
        }
    }
}
