//! SPEC §8.12 S+ #45 Surge Alloy (§6.1 Rush, Charge, Windfury; §6.3 Give; R81, R83, R1420, R3535000,
//! R3547000): (1) Spell, Legendary, Mindustrial, Mineral, a card of the held Stinkyscape+ set.
//!   Base:    "Give a Unit Rush attack and Windfury."
//!   Radiant: "Give a Unit Charge attack and Windfury."
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then `grant_keyword` twice:
//! the face's attack keyword, Rush on the base face and Charge on the Radiant one ("Rush attack" is
//! the keyword Rush, the issue's reading), and Windfury on both. The keywords carry their own rules
//! and the card adds none: a Unit that entered this turn may attack a Unit with Rush and the hero
//! too with Charge, and a Unit with Windfury may attack twice a turn (§6.1). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-045";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// `attack` is the keyword the face gives beside Windfury.
fn surge_alloy(attack: &'static str) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |_ctx| {
            vec![
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": attack } }))),
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Windfury" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: surge_alloy("Rush"),
        radiant: surge_alloy("Charge"),
    }
}

// S+ #45 Surge Alloy — SPEC §8.12 row S+ 45 (R81, R83, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ALLOY: &str = "stinkyscapeplus-045";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const WALL: &str = "stand-in (1) 1/30";
    const FILLER: &str = "core-005";

    /// p1 holds the Alloy and a Unit to play (which lands summoning sick); p2 has a Wall to hit.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ALLOY, "radiant": radiant }, VANILLA, FILLER], "mana": 8 },
            "p2": { "hand": [FILLER], "field": [WALL] },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": pick::instance(card) })
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let alloy = s.card(ALLOY).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == alloy => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = scenario(json!({
                "p1": { "hand": [ALLOY, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "field": [WALL] },
            }));
            let offered = offered(&s);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn rush_and_windfury_and_no_charge() {
            let mut s = board(false);
            s.play(VANILLA, json!({ "zone": 1 }));
            let unit = s.unit_at(P1, 1);
            s.play(ALLOY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Rush), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Windfury), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Charge), 0);
        }

        #[test]
        fn a_unit_that_just_landed_may_then_attack_a_unit_twice_but_never_the_hero() {
            let mut s = board(false);
            s.play(VANILLA, json!({ "zone": 1 }));
            let unit = s.unit_at(P1, 1);
            let wall = s.unit_at(P2, 1);
            s.expect_refused(|s| s.attack(&unit, &wall));
            s.play(ALLOY, on(&unit));

            s.expect_refused(|s| s.attack(&unit, "hero"));
            s.attack(&unit, &wall);
            s.attack(&unit, &wall);

            assert_eq!(s.card(&wall).damage, 8, "two attacks of 4");
            s.expect_health(P2, HERO_HEALTH);
            s.expect_refused(|s| s.attack(&unit, &wall));
        }

        #[test]
        fn an_enemy_unit_takes_both_keywords_too() {
            let mut s = board(false);
            let wall = s.unit_at(P2, 1);
            s.play(ALLOY, on(&wall));
            assert_eq!(keyword_count(&s, &wall, KeywordKind::Rush), 1);
            assert_eq!(keyword_count(&s, &wall, KeywordKind::Windfury), 1);
        }

        #[test]
        fn windfury_once_on_a_unit_that_has_it() {
            let mut s = scenario(json!({
                "p1": { "hand": [ALLOY, FILLER], "field": ["stand-in (1) 4/4 Windfury"] },
                "p2": { "hand": [FILLER] },
            }));
            let unit = s.unit_at(P1, 1);
            s.play(ALLOY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Windfury), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn charge_and_windfury_and_no_rush() {
            let mut s = board(true);
            s.play(VANILLA, json!({ "zone": 1 }));
            let unit = s.unit_at(P1, 1);
            s.play(ALLOY, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Charge), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Windfury), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Rush), 0);
        }

        #[test]
        fn a_unit_that_just_landed_may_then_attack_the_hero_twice() {
            let mut s = board(true);
            s.play(VANILLA, json!({ "zone": 1 }));
            let unit = s.unit_at(P1, 1);
            s.expect_refused(|s| s.attack(&unit, "hero"));
            s.play(ALLOY, on(&unit));

            s.attack(&unit, "hero");
            s.attack(&unit, "hero");

            s.expect_health(P2, HERO_HEALTH - 8);
            s.expect_refused(|s| s.attack(&unit, "hero"));
        }
    }
}
