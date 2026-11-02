//! Stinkyscape #142 Jade Launcher (SPEC §8.10 row 142). Unit 0/6 → 0/12, CN, Weapon, cost 3, Epic.
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal {damage} damage to an enemy, then gain the reward an Auspicious Rock gives."
//!            — damage 2
//!   Radiant: "Starship Piece, Equip
//!            Activate 2: Deal {damage} damage to an enemy, then gain the reward an Auspicious Rock
//!            gives." — damage 2
//!
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! The Activate's target is a declared enemy Unit or the enemy hero (R81, R1862000), once a turn and
//! on the Radiant face twice (R384, R1857003). R1862005: the target takes the 2 the Auspicious Rock
//! (M #39.1) costs its caster (`damage`, whole on a hero), then the Rock's reward: one roll on the
//! Rock's own table (`add_rolled`, R960) adds a Dud, a Jade or a Red Jade to its controller's hand, a
//! full hand burning it (§2.4). The Launcher prints no Lucky, but the roll keeps a best, so its
//! controller's Luck rolls it again (§6.1 Luck X, R987). No Rock is made or cast, and no health is lost. The number is the
//! declared `damage` (R386), read through `param`.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-142";

/// The table the Auspicious Rock rolls on (R960), M #39.1's.
const ROCK_TABLE: &str = "auspiciousRock";

/// "Activate" is once a turn, the Radiant face's "Activate 2" twice (R1857003).
const BASE_USES: i32 = 1;
const RADIANT_USES: i32 = 2;

fn launch(uses: i32) -> ActivationDecl {
    ActivationDecl {
        id: "jade".into(),
        label: "Deal damage to an enemy, then gain an Auspicious Rock's reward".into(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "enemy", "of": ["unit", "hero"] }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| {
            vec![
                damage(json_as(
                    json!({ "to": { "of": "chosen" }, "amount": param(&*ctx, "damage") }),
                )),
                add_rolled(json_as(json!({ "table": ROCK_TABLE }))),
            ]
        }),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![launch(BASE_USES)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![launch(RADIANT_USES)],
            ..Script::default()
        },
    }
}

// Stinkyscape #142 Jade Launcher — SPEC §8.10 row 142, BUILD M12 row S 142.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{def_ids, filler, hits, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const REWARDS: [&str; 3] = ["meditative-039-2", "meditative-039-3", "meditative-039-4"];
    const DAMAGE: i32 = 2;

    fn at(target: &str) -> Value {
        json!({ "ability": "jade", "targets": pick::id(target) })
    }

    fn at_hero() -> Value {
        json!({ "ability": "jade", "targets": [{ "pick": "hero", "player": "p2" }] })
    }

    fn launcher_on(radiant: bool, p1: Value) -> Scenario {
        let mut p1 = p1;
        p1["field"] = json!([{ "def": ID, "radiant": radiant }]);
        weapon_board("s142", p1, json!({ "field": [VANILLA] }))
    }

    fn rewards(cards: &[CardInstance]) -> Vec<String> {
        def_ids(cards)
            .into_iter()
            .filter(|def| REWARDS.contains(&def.as_str()))
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1862005_deals_2_to_the_chosen_enemy_and_adds_the_rocks_reward() {
            let mut s = launcher_on(false, json!({}));
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, DAMAGE)]);
            assert_eq!(rewards(&s.hand(P1)).len(), 1, "{:?}", def_ids(&s.hand(P1)));
            assert!(s.state().pending.is_none());
            s.expect_health(P1, HERO_HEALTH);
        }

        #[test]
        fn r1862005_the_enemy_hero_takes_the_2_whole() {
            let mut s = launcher_on(false, json!({}));
            s.activate(ID, at_hero());
            s.expect_health(P2, HERO_HEALTH - DAMAGE);
            assert_eq!(rewards(&s.hand(P1)).len(), 1);
        }

        #[test]
        fn r1862005_a_full_hand_burns_the_reward() {
            let mut s = launcher_on(false, json!({ "hand": filler(HAND_CAP as usize) }));
            s.activate(ID, at_hero());
            assert!(rewards(&s.hand(P1)).is_empty());
            let burned: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(burned.len(), 1);
            assert!(REWARDS.contains(&burned[0].as_str()), "{burned:?}");
        }

        #[test]
        fn r384_once_a_turn_a_second_use_refused() {
            let mut s = launcher_on(false, json!({}));
            s.activate(ID, at_hero());
            s.expect_refused_with(|s| s.activate(ID, at_hero()), "already been used");
        }

        #[test]
        fn r987_its_controllers_luck_rolls_the_reward_again_keeping_the_best() {
            // M #40 Feng Shui's Luck 1 gives the Launcher's roll a second roll, keeping the better.
            const FENG_SHUI: &str = "meditative-040";
            // Worst to best (R960): Dud < Jade < Red Jade.
            const RANKED: [&str; 3] = ["meditative-039-3", "meditative-039-2", "meditative-039-4"];
            const SEEDS: u32 = 30;
            let rank = |s: &Scenario| {
                let got = rewards(&s.hand(P1));
                assert_eq!(got.len(), 1, "{got:?}");
                RANKED.iter().position(|id| *id == got[0]).expect("a reward")
            };
            let (mut worse_never, mut better_once) = (true, false);
            for n in 0..SEEDS {
                let seed = format!("jade-luck-{n}");
                let field = json!([{ "def": ID }]);
                let mut plain = weapon_board(&seed, json!({ "field": field }), json!({ "field": [VANILLA] }));
                plain.activate(ID, at_hero());
                let mut lucky = weapon_board(
                    &seed,
                    json!({ "field": field, "backrow": [FENG_SHUI] }),
                    json!({ "field": [VANILLA] }),
                );
                assert_eq!(luck_of(lucky.state(), P1), 1);
                lucky.activate(ID, at_hero());
                let (without, with) = (rank(&plain), rank(&lucky));
                worse_never &= with >= without;
                better_once |= with > without;
            }
            assert!(worse_never, "Luck keeps the best of the rolls");
            assert!(better_once, "over {SEEDS} seeds some roll improves");
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = launcher_on(false, json!({}));
            let amount = crate::upgrade_number(&mut s, ID, "damage");
            let target = s.unit_at(P2, 1).id;
            s.activate(ID, at(&target));
            assert_eq!(hits(s.last_events()), vec![(target, amount)]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1857003_two_uses_a_turn_and_the_third_refused() {
            let mut s = launcher_on(true, json!({}));
            for _ in 0..RADIANT_USES {
                s.activate(ID, at_hero());
            }
            s.expect_health(P2, HERO_HEALTH - RADIANT_USES * DAMAGE);
            assert_eq!(rewards(&s.hand(P1)).len(), RADIANT_USES as usize);
            s.expect_refused_with(
                |s| s.activate(ID, at_hero()),
                &format!("that ability has been used {RADIANT_USES} times this turn"),
            );
        }

        #[test]
        fn its_health_is_doubled() {
            let s = launcher_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, 12));
        }
    }
}
