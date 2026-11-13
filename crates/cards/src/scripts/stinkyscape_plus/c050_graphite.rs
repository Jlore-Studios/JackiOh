//! SPEC §8.12 S+ #50 Graphite (§4.2 step 5, §6.1 Pierce, Deft, Frenzy, First Strike; R81, R1420,
//! R3535000, R3543003, R3547000): (1) Spell, Rare, Mindustrial, Mineral, a card of the held
//! Stinkyscape+ set.
//!   Base:    "Give a Unit Pierce, Deft, Frenzy (this may attack again)."
//!   Radiant: "Give a Unit Pierce, Deft, Frenzy (this may attack again), and First Strike."
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then `grant_keyword` Pierce,
//! Deft and Frenzy, and on the Radiant face First Strike. The designer's "Piece" is Pierce (SP5)
//! and "Frenzy this may attack again" is E3's Frenzy keyword (R3543003): after the Unit's attack
//! destroys the Unit it attacked, it may attack again. The card adds no code of its own for any of
//! them. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-050";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// The Radiant face adds First Strike to the three the base face gives.
fn graphite(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            let mut kinds = vec!["Pierce", "Deft", "Frenzy"];
            if radiant {
                kinds.push("First Strike");
            }
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
        base: graphite(false),
        radiant: graphite(true),
    }
}

// S+ #50 Graphite — SPEC §8.12 row S+ 50 (R81, R3543003, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const GRAPHITE: &str = "stinkyscapeplus-050";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FRAIL: &str = "stand-in (1) 1/1";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": GRAPHITE, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": pick::instance(card) })
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let graphite = s.card(GRAPHITE).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == graphite => targets.unwrap_or_default(),
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
        fn pierce_deft_frenzy_and_no_first_strike() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(GRAPHITE, on(&unit));
            for kind in [KeywordKind::Pierce, KeywordKind::Deft, KeywordKind::Frenzy] {
                assert_eq!(keyword_count(&s, &unit, kind), 1, "{kind:?}");
            }
            assert_eq!(keyword_count(&s, &unit, KeywordKind::FirstStrike), 0);
            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn a_kill_frees_another_attack_and_each_kill_the_next() {
            let mut s = board(false, json!([VANILLA]), json!([FRAIL, FRAIL, FRAIL]));
            let unit = s.unit_at(P1, 1);
            let frail: Vec<CardInstance> = (1..=3).map(|lane| s.unit_at(P2, lane)).collect();
            s.play(GRAPHITE, on(&unit));

            s.attack(&unit, &frail[0]);
            assert!(s.unit(P2, 1).is_none(), "the first frail died");
            assert!(!s.card(&unit).exertion.attacked, "the kill frees a second attack (R3543003)");
            s.attack(&unit, &frail[1]);
            assert!(!s.card(&unit).exertion.attacked, "and a third");
            s.attack(&unit, &frail[2]);
            assert!(s.unit(P2, 3).is_none());
        }

        #[test]
        fn without_a_kill_there_is_no_second_attack() {
            let mut s = board(false, json!([VANILLA]), json!(["stand-in (1) 1/30"]));
            let unit = s.unit_at(P1, 1);
            let wall = s.unit_at(P2, 1);
            s.play(GRAPHITE, on(&unit));
            s.attack(&unit, &wall);
            assert!(s.card(&unit).exertion.attacked);
            s.expect_refused(|s| s.attack(&unit, &wall));
        }

        #[test]
        fn an_enemy_unit_takes_all_three() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(GRAPHITE, on(&theirs));
            for kind in [KeywordKind::Pierce, KeywordKind::Deft, KeywordKind::Frenzy] {
                assert_eq!(keyword_count(&s, &theirs, kind), 1, "{kind:?}");
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn pierce_deft_frenzy_and_first_strike() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(GRAPHITE, on(&unit));
            for kind in [KeywordKind::Pierce, KeywordKind::Deft, KeywordKind::Frenzy, KeywordKind::FirstStrike] {
                assert_eq!(keyword_count(&s, &unit, kind), 1, "{kind:?}");
            }
        }

        #[test]
        fn first_strike_kills_the_defender_before_it_strikes_back_and_frees_another_attack() {
            let mut s = board(true, json!([VANILLA]), json!(["stand-in (1) 4/4", FRAIL]));
            let unit = s.unit_at(P1, 1);
            let (killer, frail) = (s.unit_at(P2, 1), s.unit_at(P2, 2));
            s.play(GRAPHITE, on(&unit));

            s.attack(&unit, &killer);

            assert!(s.unit(P2, 1).is_none(), "the 4/4 died to the 4 attack");
            assert_eq!(s.card(&unit).damage, 0, "First Strike: it died before it struck back");
            assert!(!s.card(&unit).exertion.attacked);
            s.attack(&unit, &frail);
        }
    }
}
