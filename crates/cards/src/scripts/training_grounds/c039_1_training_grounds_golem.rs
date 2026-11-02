//! SPEC §7 Training Grounds 39.1 Training-Grounds Golem: (4) Unit, Token (printed Legendary), 10/10 →
//! 20/20, Rush, Armor 10 (Radiant 20), Taunt, Divine Shield and Pierce, a token of the held Training
//! Grounds set (R1420, R2194000). Keywords only, catalog data read by §10.4 layer 1, as Core #95.1 Chaos
//! Golem; its maker is TG #39 Call to Chaos (Training-Grounds Edition), P7's (#2200).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-039-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Armor 20 and the doubled stats are catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #39.1 Training-Grounds Golem — SPEC §7, BUILD M13 row TG 39.1.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005";
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4

    /// The face's keywords, sorted, as the view prints them.
    fn keywords_of(s: &Scenario) -> Vec<String> {
        let mut out: Vec<String> = s
            .stats(super::ID)
            .keywords
            .iter()
            .map(|keyword| crate::js(keyword)["kind"].as_str().unwrap_or_default().to_string())
            .collect();
        out.sort();
        out
    }

    fn defending(radiant: bool, attack: i32) -> Scenario {
        let hitter = json!({ "def": BODY, "statsOverride": { "attack": attack, "health": 99 } });
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [hitter, hitter] },
            "p2": { "hand": [FILLER], "field": [{ "def": super::ID, "radiant": radiant }, BODY] },
        }))
    }

    /// p1's two hitters swing at the Golem: the first spends its Divine Shield, the second lands.
    fn hit_twice(s: &mut Scenario) {
        for lane in [1, 2] {
            let hitter = s.unit_at(P1, lane);
            s.attack(&hitter, super::ID);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_a_10_10_with_rush_armor_taunt_divine_shield_and_pierce() {
            let mut s = defending(false, 12);
            s.expect_stats(super::super::ID, json!({ "attack": 10, "health": 10 }));
            assert_eq!(keywords_of(&s), ["Armor", "Divine Shield", "Pierce", "Rush", "Taunt"]);
            assert!(s
                .stats(super::super::ID)
                .keywords
                .iter()
                .any(|keyword| matches!(keyword, Keyword::Armor { n: 10 })));
        }

        #[test]
        fn s6_1_rush_it_attacks_a_unit_the_turn_it_lands_never_the_hero() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [super::super::ID] },
                "p2": { "hand": [FILLER], "field": [BODY] },
            }));
            let golem = s.unit_at(P1, 1);
            let turn = s.state().turn;
            s.card_mut(&golem).summoned_turn = Some(turn);
            s.expect_refused(|s| s.attack(&golem, "hero"));
            let body = s.unit_at(P2, 1);
            s.attack(&golem, &body);
            s.expect_in_zone(&body, "graveyard");
        }

        #[test]
        fn s4_4_armor_10_takes_10_off_each_hit() {
            let mut s = defending(false, 12);
            hit_twice(&mut s);
            s.expect_stats(super::super::ID, json!({ "health": 8, "damage": 2 }));
        }

        #[test]
        fn s6_1_taunt_holds_attacks() {
            let mut s = defending(false, 12);
            let hitter = s.unit_at(P1, 1);
            let body = s.unit_at(P2, 2);
            s.expect_refused(|s| s.attack(&hitter, &body));
            s.expect_refused(|s| s.attack(&hitter, "hero"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s4_4_a_20_20_whose_armor_20_takes_20_off_each_hit() {
            let mut s = defending(true, 23);
            s.expect_stats(super::super::ID, json!({ "attack": 20, "health": 20 }));
            assert_eq!(keywords_of(&s), ["Armor", "Divine Shield", "Pierce", "Rush", "Taunt"]);
            hit_twice(&mut s);
            s.expect_stats(super::super::ID, json!({ "health": 17, "damage": 3 }));
        }
    }
}
