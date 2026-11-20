//! Stinkyscape #129 Jlockeed's Missile Pod (SPEC §8.10 row 129). (1, Embiggened 3) Unit, Jlockeed, Epic;
//! 2/2 → 4/4 (1/1 → 2/2 through v0.6.7k, 2/1 → 4/2 until issue #3837).
//!   Base:    "End of turn: Deal damage equal to its Attack split among all enemies.
//!            Embiggened (3): Starship Piece."
//!   Radiant: "End of turn: Deal damage equal to twice its Attack split among all enemies.
//!            Embiggened (3): Starship Piece."
//!
//! End of turn (R62), R4285002: its Attack as it stands then, through the layers, buffs and auras included
//! (§10.4, R1878002), times `ATTACK_MULTIPLE` (Radiant `RADIANT_ATTACK_MULTIPLE`), dealt by §6.3 Damage's
//! split (`damage_split`, R59, R60): one point at a time, each to a random enemy still standing, the enemy
//! hero or an enemy Unit, by the match rng; an Attack of 0 or less deals nothing (R63). R4285001: the
//! Embiggened version is a Starship Piece and no longer has Equip, the piece lent by its text to the card
//! stamped embiggened (R1878001). Equipped by a lent Equip (S #16 Rod of Ages, R1869000), its text runs on
//! its host and reads the host's Attack (R102, R1086).
//!
//! Kept for replays (R767000): through v0.6.7k (`script_v0_6_7k`, issue #4285) one hit of ⌈Attack ÷
//! divisor⌉ (the declared `divisor`, 4, Radiant 3) to the enemy hero and every enemy Unit (`damage_all`,
//! R59), and Embiggened (2) a Starship Piece and Equip, so Equip was a play option only at (2), as Core #56
//! Jilliax's (R831013), equipped onto a host it leaving no piece (R1852001); through v0.5.11r
//! (`script_v0_5_11o`) the quotient was rounded down, so a 2/1 Pod hit nothing.

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-129";

/// R4285002: "its Attack": the base face's hits are its Attack.
const ATTACK_MULTIPLE: i32 = 1;
/// R4285002: "twice its Attack", on the Radiant face.
const RADIANT_ATTACK_MULTIPLE: i32 = 2;

/// R4285001: "Embiggened (3): Starship Piece.": it holds only on a card stamped embiggened (R1878001).
fn embiggened_piece(args: HookArgs<'_>) -> Vec<Keyword> {
    if args.self_.embiggened == Some(true) {
        vec![Keyword::StarshipPiece]
    } else {
        Vec::new()
    }
}

/// R4285002: "Deal damage equal to [twice] its Attack split among all enemies", the Attack read as the turn
/// ends: `multiple` × Attack points, each a hit of 1 on a random enemy still standing (`damage_split`).
fn split_barrage(ctx: &mut EffectContext<'_>, multiple: i32) -> Vec<Effect> {
    let Some(attack) = ctx.live_self().map(|me| unit_view(&*ctx.state, me).attack) else {
        return Vec::new();
    };
    let amount = attack.max(0) * multiple;
    if amount <= 0 {
        return Vec::new();
    }
    vec![damage_split(json_as(json!({ "amount": amount })))]
}

fn split_pod(multiple: i32) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| split_barrage(ctx, multiple))),
        conditional_keywords: Some(read_hook(embiggened_piece)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: split_pod(ATTACK_MULTIPLE),
        radiant: split_pod(RADIANT_ATTACK_MULTIPLE),
    }
}

/// R1878001: "Embiggened (2): Starship Piece and Equip." through v0.6.7k: both held only on a card stamped
/// embiggened.
fn embiggened_piece_and_equip(args: HookArgs<'_>) -> Vec<Keyword> {
    if args.self_.embiggened == Some(true) {
        vec![Keyword::StarshipPiece, Keyword::Equip]
    } else {
        Vec::new()
    }
}

/// R1878002: "its Attack divided by {divisor}, rounded up" (`round_up`; through v0.5.11r rounded down),
/// read as the turn ends, through v0.6.7k.
fn barrage(ctx: &mut EffectContext<'_>, round_up: bool) -> Vec<Effect> {
    let Some(attack) = ctx.live_self().map(|me| unit_view(&*ctx.state, me).attack) else {
        return Vec::new();
    };
    // The declared divisor floors at its `min`, 1 (R386). An Attack below 0 hits for 0, as it reads 0.
    let (attack, divisor) = (attack.max(0), param(&*ctx, "divisor"));
    let amount = attack / divisor + i32::from(round_up && attack % divisor > 0);
    vec![damage_all(json_as(json!({ "side": "enemy", "amount": amount, "heroes": true })))]
}

fn missile_pod(round_up: bool) -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(move |ctx| barrage(ctx, round_up))),
        conditional_keywords: Some(read_hook(embiggened_piece_and_equip)),
        ..Script::default()
    };
    // One script: the Radiant face's divisor 3 was catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

/// R767000, R4285002: after v0.5.11r through v0.6.7k the hit was the Attack divided by the declared divisor,
/// rounded up, to every enemy, and Embiggened it had Equip too (issue #4285); `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_7k() -> CardScripts {
    missile_pod(true)
}

/// R767000: through v0.5.11r the hit was the Attack divided by the divisor rounded down (R1878002);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    missile_pod(false)
}

// Stinkyscape #129 Jlockeed's Missile Pod — SPEC §8.10 row 129, BUILD M12 row S 129 (R1086, R63,
// R4285001, R4285002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ATTACK_MULTIPLE, ID, RADIANT_ATTACK_MULTIPLE, script_v0_5_11o, script_v0_6_7k};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const POD: &str = "stinkyscape-129";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const BIG: &str = "stand-in (1) 20/20 R40/40";
    const FILLER: &str = "core-005";

    fn standing_against(radiant: bool, enemy: &str) -> Scenario {
        scenario(json!({
            "seed": "missile-pod",
            "p1": { "hand": [FILLER], "field": [{ "def": POD, "lane": 1, "radiant": radiant }], "library": filler(6) },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": enemy, "lane": 2 }, { "def": enemy, "lane": 4 }],
                "library": filler(6),
            },
        }))
    }

    fn standing(radiant: bool) -> Scenario {
        standing_against(radiant, VANILLA)
    }

    fn hits(s: &Scenario) -> Vec<(String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage { target_id, amount, .. } => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    fn pod(s: &Scenario) -> String {
        s.unit(P1, 1).expect("the Pod").id
    }

    /// The enemy hero and the enemy Units' ids: every split hit lands on one of them.
    fn enemies(s: &Scenario) -> Vec<String> {
        let mut out: Vec<String> = s.field(P2).iter().map(|unit| unit.id.clone()).collect();
        out.push("hero-p2".to_string());
        out
    }

    /// R4285002: the hits of a Pod of `attack` Attack on the face `radiant`: one point each, as many as the
    /// face's multiple of its Attack, every one on an enemy.
    fn splits(radiant: bool, attack: i32) {
        let mut s = standing_against(radiant, BIG);
        let id = pod(&s);
        let printed = s.stats(id.as_str()).attack;
        s.card_mut(id.as_str()).buffs.attack = attack - printed;
        assert_eq!(s.stats(id.as_str()).attack, attack);
        let targets = enemies(&s);
        s.end_turn();
        let hits = hits(&s);
        let multiple = if radiant { RADIANT_ATTACK_MULTIPLE } else { ATTACK_MULTIPLE };
        assert_eq!(hits.len() as i32, attack * multiple, "{hits:?}");
        assert!(hits.iter().all(|hit| hit.1 == 1), "{hits:?}");
        assert!(hits.iter().all(|hit| targets.contains(&hit.0)), "{hits:?}");
        let to_hero = hits.iter().filter(|hit| hit.0 == "hero-p2").count() as i32;
        s.expect_health(P2, 30 - to_hero);
        s.expect_health(P1, 30);
    }

    #[test]
    fn r4285002_end_of_turn_splits_its_attack_one_point_at_a_time_among_enemies() {
        splits(false, 2);
        splits(false, 5);
    }

    #[test]
    fn r4285002_radiant_end_of_turn_splits_twice_its_attack_among_enemies() {
        splits(true, 4);
        splits(true, 1);
    }

    #[test]
    fn r4285002_the_split_draws_its_targets_from_the_match_rng() {
        // The same seed lands the same hits, and some seed lands them otherwise (R60).
        let landed = |seed: &str| {
            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "hand": [FILLER], "field": [{ "def": POD, "lane": 1 }], "library": filler(6) },
                "p2": { "hand": [FILLER], "field": [{ "def": BIG, "lane": 2 }, { "def": BIG, "lane": 4 }], "library": filler(6) },
            }));
            let id = pod(&s);
            s.card_mut(id.as_str()).buffs.attack = 6;
            s.end_turn();
            hits(&s)
                .into_iter()
                .map(|hit| if hit.0 == "hero-p2" { "hero".to_string() } else { "unit".to_string() })
                .collect::<Vec<_>>()
        };
        assert_eq!(landed("pod-rng-a"), landed("pod-rng-a"));
        let first = landed("pod-rng-a");
        assert!(
            (0..64).any(|n| landed(&format!("pod-rng-{n}")) != first),
            "the hits never move with the seed"
        );
    }

    #[test]
    fn r63_r4285002_at_0_attack_nothing_is_hit() {
        for radiant in [false, true] {
            let mut s = standing(radiant);
            let id = pod(&s);
            let printed = s.stats(id.as_str()).attack;
            s.card_mut(id.as_str()).buffs.attack = -printed - 1;
            assert_eq!(s.stats(id.as_str()).attack, 0);
            s.end_turn();
            assert_eq!(hits(&s), Vec::<(String, i32)>::new());
            s.expect_health(P2, 30);
        }
    }

    #[test]
    fn its_stats_are_2_2_and_radiant_4_4_and_it_embiggens_at_3() {
        let s = standing(false);
        s.expect_stats(pod(&s).as_str(), json!({ "attack": 2, "health": 2 }));
        let s = standing(true);
        s.expect_stats(pod(&s).as_str(), json!({ "attack": 4, "health": 4 }));
        assert_eq!(
            crate::card_def(POD).cost,
            CardCost::Embiggen { base: 1, embiggen: 3 }
        );
    }

    #[test]
    fn r4285001_embiggened_it_is_a_starship_piece_without_equip_and_at_1_neither() {
        for radiant in [false, true] {
            for embiggened in [true, false] {
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": POD, "radiant": radiant }, FILLER], "library": filler(6), "mana": 10 },
                    "p2": { "hand": [FILLER], "library": filler(6) },
                }));
                s.play(POD, json!({ "zone": 1, "embiggen": embiggened }));
                let keywords = s.stats(pod(&s).as_str()).keywords;
                assert_eq!(has_keyword(&keywords, KeywordKind::StarshipPiece), embiggened, "radiant {radiant}");
                assert!(!has_keyword(&keywords, KeywordKind::Equip), "radiant {radiant}");
            }
        }
    }

    #[test]
    fn r4285001_no_equip_play_is_offered_embiggened_or_not() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [POD, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.expect_refused(|s| s.play(POD, json!({ "zone": 1, "equip": true, "embiggen": false })));
        s.expect_refused(|s| s.play(POD, json!({ "zone": 1, "equip": true, "embiggen": true })));
    }

    /// R767000: through v0.6.7k (issue #4285): a 1/1 (Radiant 2/2) whose hit was its Attack divided by the
    /// declared divisor, rounded up, to every enemy, and Embiggened (2) a Starship Piece with Equip.
    mod v0_6_7k {
        use super::*;

        fn kept<T>(run: impl FnOnce() -> T) -> T {
            crate::under_kept_as(
                ID,
                script_v0_6_7k(),
                |def| {
                    def.params = serde_json::from_value(json!([
                        { "key": "divisor", "base": 4, "radiant": 3, "better": "down", "step": 1, "min": 1 }
                    ]))
                    .expect("the v0.6.7k params");
                    def.cost = CardCost::Embiggen { base: 1, embiggen: 2 };
                    def.base.attack = Some(1);
                    def.base.health = Some(1);
                    def.radiant.attack = Some(2);
                    def.radiant.health = Some(2);
                },
                run,
            )
        }

        #[test]
        fn r1878002_r767000_at_1_attack_its_quarter_rounded_up_is_1_so_each_enemy_takes_1() {
            kept(|| {
                let mut s = standing(false);
                s.end_turn();
                let hits = hits(&s);
                assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![1, 1, 1], "{hits:?}");
                assert_eq!(hits.last().map(|hit| hit.0.as_str()), Some("hero-p2"));
                s.expect_health(P2, 29);
            });
        }

        #[test]
        fn r1878002_r767000_at_5_attack_its_quarter_rounded_up_is_2() {
            kept(|| {
                let mut s = standing(false);
                let id = pod(&s);
                s.card_mut(id.as_str()).buffs.attack = 4;
                s.end_turn();
                let hits = hits(&s);
                assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![2, 2, 2], "{hits:?}");
                s.expect_health(P2, 28);
            });
        }

        #[test]
        fn r767000_radiant_2_attack_divided_by_3_rounded_up_hits_each_enemy_for_1() {
            kept(|| {
                let mut s = standing(true);
                s.end_turn();
                let hits = hits(&s);
                assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![1, 1, 1], "{hits:?}");
            });
        }

        #[test]
        fn r1878001_r767000_played_at_2_it_was_a_starship_piece_with_equip_and_at_1_neither() {
            kept(|| {
                for radiant in [false, true] {
                    for (embiggened, lent) in [(true, true), (false, false)] {
                        let mut s = scenario(json!({
                            "p1": { "hand": [{ "def": POD, "radiant": radiant }, FILLER], "library": filler(6), "mana": 10 },
                            "p2": { "hand": [FILLER], "library": filler(6) },
                        }));
                        s.play(POD, json!({ "zone": 1, "embiggen": embiggened }));
                        let keywords = s.stats(pod(&s).as_str()).keywords;
                        assert_eq!(has_keyword(&keywords, KeywordKind::StarshipPiece), lent, "radiant {radiant}");
                        assert_eq!(has_keyword(&keywords, KeywordKind::Equip), lent, "radiant {radiant}");
                    }
                }
            });
        }

        #[test]
        fn r1878002_r1852001_r767000_equipped_at_2_it_ran_on_its_host_reading_the_hosts_attack() {
            kept(|| {
                let mut s = scenario(json!({
                    "seed": "missile-pod-equip",
                    "p1": {
                        "hand": [POD, FILLER],
                        "field": [{ "def": VANILLA, "lane": 1 }],
                        "library": filler(6),
                        "mana": 10,
                    },
                    "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "lane": 2 }], "library": filler(6) },
                }));
                let host = s.unit(P1, 1).expect("the host").id;
                s.expect_refused(|s| s.play(POD, json!({ "zone": 1, "equip": true, "embiggen": false })));
                s.play(POD, json!({ "zone": 1, "equip": true, "embiggen": true }));
                // The host keeps its zone and takes the Pod's 1/1: 5/5, no Starship Piece (R1852001).
                assert_eq!(s.unit(P1, 1).expect("the fused host").id, host);
                s.expect_stats(host.as_str(), json!({ "attack": 5, "health": 5 }));
                assert!(!has_keyword(&s.stats(host.as_str()).keywords, KeywordKind::StarshipPiece));
                s.end_turn();
                // The host's 5 Attack ÷ 4, rounded up, is 2.
                s.expect_health(P2, 28);
            });
        }

        /// R767000: through v0.5.11r the quotient was rounded down.
        #[test]
        fn r1878002_r767000_the_kept_script_rounds_down_so_a_1_attack_pod_hits_nothing_and_a_5_attack_one_for_1() {
            crate::under_kept_as(
                ID,
                script_v0_5_11o(),
                |def| {
                    def.params = serde_json::from_value(json!([
                        { "key": "divisor", "base": 4, "radiant": 3, "better": "down", "step": 1, "min": 1 }
                    ]))
                    .expect("the v0.5.11r params");
                    def.base.attack = Some(1);
                    def.base.health = Some(1);
                },
                || {
                    let mut s = standing(false);
                    s.end_turn();
                    assert_eq!(hits(&s), Vec::<(String, i32)>::new());
                    s.expect_health(P2, 30);

                    let mut s = standing(false);
                    let id = pod(&s);
                    s.card_mut(id.as_str()).buffs.attack = 4;
                    s.end_turn();
                    let hits = hits(&s);
                    assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![1, 1, 1], "{hits:?}");
                },
            );
        }
    }
}
