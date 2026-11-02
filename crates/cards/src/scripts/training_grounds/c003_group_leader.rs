//! SPEC §8.11 Training Grounds 3 Group leader: (1) Unit, Human, Common, 4/1 → 8/2, a card of the held
//! Training Grounds set (R1420, R2194000).
//! Base:    "Aura: Your other Units have +1 Attack."
//! Radiant: "Aura: Your other Units have +2 Attack."
//! Attack-only StatMod computed on read (§10.4 layer 5) for friendly units (standing backrow included, R1081000; C+ #46).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-003";

fn leader() -> Script {
    Script {
        aura: Some(aura_hook(|ctx| {
            let amount = param(&ctx, "attack");
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller
                        // R1081000: a Unit standing in the backrow is one of "your Units".
                        && in_units_row_or_standing(ctx.state, unit)
                        && unit.id != ctx.self_.id
                }),
                mod_: StatMod {
                    attack: Some(amount),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = leader();
    // The same script: the Radiant face's 2 is its declared `attack`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #3 Group leader — SPEC §8.11 row 3, BUILD M13 row TG 3.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LEADER: &str = super::ID;
    const BODY: &str = "core-008"; // Mr. Vanilla 4/4
    const LATE: &str = "traininggrounds-001"; // Spikey Steven 5/2, played after the leader
    const FILLER: &str = "core-005";

    /// p1 holds the leader in lane 1 and a Mr. Vanilla in lane 2; p2 a Mr. Vanilla in lane 1.
    fn leading(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [LATE, FILLER], "field": [{ "def": LEADER, "radiant": radiant }, BODY] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_each_other_friendly_unit_has_1_attack_and_its_health_unchanged() {
            let mut s = leading(false);
            let body = s.unit_at(P1, 2);
            s.expect_stats(&body, json!({ "attack": 5, "health": 4, "maxHealth": 4 }));
        }

        #[test]
        fn s10_4_the_leader_itself_does_not() {
            let mut s = leading(false);
            s.expect_stats(LEADER, json!({ "attack": 4, "health": 1 }));
        }

        #[test]
        fn s10_4_enemy_units_are_untouched() {
            let mut s = leading(false);
            let enemy = s.unit_at(P2, 1);
            s.expect_stats(&enemy, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn s10_4_a_unit_played_after_it_gets_the_bonus() {
            let mut s = leading(false);
            s.play(LATE, json!({ "zone": 3 }));
            s.expect_stats(LATE, json!({ "attack": 6, "health": 2 }));
        }

        #[test]
        fn s10_4_the_bonus_goes_when_the_leader_leaves_the_field() {
            let mut s = leading(false);
            let body = s.unit_at(P1, 2);
            let enemy = s.unit_at(P2, 1);
            // The 4/1 leader trades into the enemy 4/4 and dies.
            s.attack(LEADER, &enemy);
            s.expect_in_zone(LEADER, "graveyard");
            s.expect_stats(&body, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn r386_an_upgrade_gives_2_attack_and_a_degrade_stays_at_its_minimum_1() {
            for (step, attack) in [(1, 6), (-1, 5)] {
                let mut s = leading(false);
                step_param(s.card_mut(LEADER), "attack", step);
                let body = s.unit_at(P1, 2);
                s.expect_stats(&body, json!({ "attack": attack }));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_an_8_2_that_gives_each_other_friendly_unit_2_attack() {
            let mut s = leading(true);
            s.expect_stats(LEADER, json!({ "attack": 8, "health": 2 }));
            let body = s.unit_at(P1, 2);
            s.expect_stats(&body, json!({ "attack": 6, "health": 4 }));
            let enemy = s.unit_at(P2, 1);
            s.expect_stats(&enemy, json!({ "attack": 4 }));
        }
    }
}
