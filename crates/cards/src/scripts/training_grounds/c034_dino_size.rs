//! TG #34 Dino-size (SPEC §8.11 row TG 34; §6.3 Set stats, §10.4, R23, R78, R81, R781001, R1185002,
//! R2198000): (3) Spell, Epic, a card of the held Training Grounds set (R1420, R2194000).
//!   Base:    "Set a Unit's Attack and Health to 10/10."
//!   Radiant: "Set a Unit's Attack and Health to 20/20."
//!
//! Engine: R2198000's `set_stats` on any Unit on either side (R81), M+ #21 Harmony Spirit's set
//! (R1185002): layer 1 is 10/10, buffs and the stat half of its tuning go, damage healed, auras still
//! add (§10.4); R78's reset takes it off as the card leaves the field. An Immutable Unit is never offered
//! to a Spell (R781001), so never set (R23). The 10/10 is no param: which way is better hangs on whose
//! Unit is picked (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-034";

const SET_TO: i32 = 10;
const RADIANT_SET_TO: i32 = 20;

fn dino(to: i32) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))],
        cry: Some(hook(move |_ctx| {
            vec![set_stats(json_as(json!({ "target": { "of": "chosen" }, "attack": to, "health": to })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dino(SET_TO),
        radiant: dino(RADIANT_SET_TO),
    }
}

// TG #34 Dino-size — SPEC §8.11 row TG 34, BUILD M13 row TG 34.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const DINO: &str = "traininggrounds-034";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const WEAPONS: &str = "core-014"; // Field Spell: your Units have +3 Attack.
    const PARAGON: &str = "stinkyscape-074"; // Paragon-X 10/1, Immutable.

    fn board(radiant: bool, mine: Value, my_backrow: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": DINO, "radiant": radiant }],
                "field": mine,
                "backrow": my_backrow,
                "mana": 10,
                "library": filler(4),
            },
            "p2": { "hand": filler(1), "field": theirs, "library": filler(4) },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] })
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let dino = s.card(DINO).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == dino => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r2198000_the_target_is_ten_ten_its_damage_healed() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1, "damage": 3 }]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(DINO, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 10, "health": 10 }));
            assert_eq!(s.card(&unit).damage, 0);
            assert_eq!(s.stats(&unit).max_health, 10);
        }

        #[test]
        fn r2198000_its_buffs_are_gone_an_aura_still_adds() {
            let mut s = board(false, json!([VANILLA]), json!([WEAPONS]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.card_mut(&unit).buffs = AttackHealth { attack: 2, health: 2 };
            s.play(DINO, on(&unit));
            assert_eq!(s.card(&unit).buffs, AttackHealth { attack: 0, health: 0 });
            s.expect_stats(&unit, json!({ "attack": 13, "health": 10 }));
        }

        #[test]
        fn r2198000_an_enemy_unit_can_be_targeted() {
            let mut s = board(false, json!([]), json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(DINO, on(&theirs));
            s.expect_stats(&theirs, json!({ "attack": 10, "health": 10 }));
        }

        #[test]
        fn an_immutable_unit_is_never_offered_so_unchanged() {
            let s = board(false, json!([PARAGON, VANILLA]), json!([]), json!([]));
            let (paragon, vanilla) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            let before = s.stats(&paragon);
            let offered = offered(&s);
            assert!(!offered.contains(&Selection::Instance { instance_id: paragon.id.clone() }));
            assert!(offered.contains(&Selection::Instance { instance_id: vanilla.id }));
            assert_eq!(s.stats(&paragon), before);
        }

        #[test]
        fn r2198000_bounced_it_is_its_printed_card_again() {
            let mut s = board(false, json!([VANILLA]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(DINO, on(&unit));
            run_as(&mut s, jackioh_engine::effects::bounce(json_as(json!({ "target": { "of": "instance", "instanceId": unit.id } }))), P1);
            s.expect_in_zone(&unit, "hand");
            assert!(s.card(&unit).stats_override.is_none());
            assert_eq!(current_stats(s.state(), s.card(&unit)), Some(AttackHealth { attack: 4, health: 4 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2198000_twenty_twenty() {
            let mut s = board(true, json!([{ "def": VANILLA, "lane": 1, "damage": 1 }]), json!([]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(DINO, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 20, "health": 20 }));
        }
    }
}
