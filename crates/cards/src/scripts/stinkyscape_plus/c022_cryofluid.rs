//! SPEC §8.12 S+ #22 Cryofluid (§6.1 Deft, Frozen; §6.3 Buff, Freeze; R81, R1174000, R1420, R3535000, R3547000): (1)
//! Spell, Rare, Mindustrial, a card of the held Stinkyscape+ set.
//!   Base:    "Give a Unit +{buff}/+{buff}, Deft, and Freeze it." — 6.
//!   Radiant: "Give a Unit +{buff2}/+{buff2}, Deft, and Freeze it." — 12.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then in the text's order a `buff` of
//! attack and health by the same number, `grant_keyword` Deft and `freeze` on the chosen Unit. The
//! two faces differ by the number alone, `buff` and `buff2`, each tuned on its own face. A Freeze
//! is R1174000's: the Unit loses its next attack or position switch, a Unit Immune to Spells is
//! passed by (and a Spell cannot pick one), an Immutable one is frozen all the same.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-022";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// Both faces run the same three verbs; `key` names the number the face prints.
fn cryofluid(key: &'static str) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let n = param(&*ctx, key);
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "attack": n, "health": n }))),
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Deft" } }))),
                freeze(json_as(json!({ "target": { "of": "chosen" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: cryofluid("buff"),
        radiant: cryofluid("buff2"),
    }
}

// S+ #22 Cryofluid — SPEC §8.12 row S+ 22 (R81, R1174000, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CRYO: &str = "stinkyscapeplus-022";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005";

    /// A filler in each hand: with nothing meaningful left the harness would end the turn, and a
    /// freeze on the caster's own Unit would thaw at its cleanup before the assert reads it (R1174000).
    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": CRYO, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": pick::instance(card) })
    }

    fn frozen(s: &Scenario, card: &CardInstance) -> bool {
        s.card(card).frozen.is_some()
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let cryo = s.card(CRYO).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == cryo => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = offered(&s);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn the_unit_gains_six_six_deft_and_a_freeze() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(CRYO, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 10, "health": 10 }));
            assert_eq!(s.stats(&unit).max_health, 10);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Deft), 1);
            assert!(frozen(&s, &unit));
        }

        #[test]
        fn an_enemy_unit_takes_all_three() {
            let mut s = board(false, json!([VANILLA]), json!([VANILLA]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(CRYO, on(&theirs));
            s.expect_stats(&theirs, json!({ "attack": 10, "health": 10 }));
            assert_eq!(keyword_count(&s, &theirs, KeywordKind::Deft), 1);
            assert!(frozen(&s, &theirs));
            assert!(!frozen(&s, &mine));
            s.expect_stats(&mine, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn the_number_reads_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, CRYO, "buff"), 7);
            let unit = s.unit_at(P1, 1);
            s.play(CRYO, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 11, "health": 11 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_unit_gains_twelve_twelve_deft_and_a_freeze() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(CRYO, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 16, "health": 16 }));
            assert_eq!(s.stats(&unit).max_health, 16);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Deft), 1);
            assert!(frozen(&s, &unit));
        }

        #[test]
        fn the_number_reads_through_param() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, CRYO, "buff2"), 13);
            let unit = s.unit_at(P1, 1);
            s.play(CRYO, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 17, "health": 17 }));
        }
    }
}
