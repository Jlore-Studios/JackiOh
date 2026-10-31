//! Stinkyscape #129 Jlockeed's Missile Pod (SPEC §8.10 row 129). (1, Embiggened 2) Unit, Jlockeed, Epic;
//! 2/1 → 4/2.
//!   Base:    "End of turn: Deal damage equal to its Attack divided by {divisor}, rounded down, to all
//!            enemies. / Embiggened (2): Starship Piece and Equip." — divisor 4
//!   Radiant: the same words — divisor 3
//!
//! End of turn (R62): one hit of ⌊Attack ÷ divisor⌋ to the enemy hero and every enemy Unit, one effect
//! (`damage_all`, R59). R1878002: its Attack is the card's as it stands then, through the layers, buffs and
//! auras included (§10.4); a hit of 0 is no hit (R63). Equipped, its text runs on its host and reads the
//! host's Attack (R102, R1086). R1878001: the Embiggened version is a Starship Piece and Equip, both lent by
//! its text to the card stamped embiggened, so Equip is a play option only at (2), as Core #56 Jilliax's
//! (R831013); equipped onto a host it leaves no piece (R1852001). The divisor is the declared number (R386).

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-129";

/// R1878001: "Embiggened (2): Starship Piece and Equip.": both hold only on a card stamped embiggened.
fn embiggened_piece_and_equip(args: HookArgs<'_>) -> Vec<Keyword> {
    if args.self_.embiggened == Some(true) {
        vec![Keyword::StarshipPiece, Keyword::Equip]
    } else {
        Vec::new()
    }
}

/// R1878002: "its Attack divided by {divisor}, rounded down", read as the turn ends.
fn barrage(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(attack) = ctx.live_self().map(|me| unit_view(&*ctx.state, me).attack) else {
        return Vec::new();
    };
    // The declared divisor floors at its `min`, 1 (R386).
    let amount = attack / param(&*ctx, "divisor");
    vec![damage_all(json_as(json!({ "side": "enemy", "amount": amount, "heroes": true })))]
}

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(barrage)),
        conditional_keywords: Some(read_hook(embiggened_piece_and_equip)),
        ..Script::default()
    };
    // One script: the Radiant face's 4/2 and divisor 3 are catalog data.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// Stinkyscape #129 Jlockeed's Missile Pod — SPEC §8.10 row 129, BUILD M12 row S 129 (R1086, R63).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const POD: &str = "stinkyscape-129";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn standing(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "missile-pod",
            "p1": { "hand": [FILLER], "field": [{ "def": POD, "lane": 1, "radiant": radiant }], "library": filler(6) },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }],
                "library": filler(6),
            },
        }))
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

    #[test]
    fn r63_at_2_attack_its_quarter_is_0_and_nothing_is_hit() {
        let mut s = standing(false);
        s.end_turn();
        assert_eq!(hits(&s), Vec::<(String, i32)>::new());
        s.expect_health(PlayerId::P2, 30);
    }

    #[test]
    fn r1878002_at_4_attack_the_enemy_hero_and_every_enemy_unit_take_1_at_its_controllers_end_of_turn() {
        let mut s = standing(false);
        let id = pod(&s);
        s.card_mut(id.as_str()).buffs.attack = 2;
        s.end_turn();
        let hits = hits(&s);
        assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![1, 1, 1], "{hits:?}");
        assert_eq!(hits.last().map(|hit| hit.0.as_str()), Some("hero-p2"));
        s.expect_health(PlayerId::P2, 29);
        s.expect_health(PlayerId::P1, 30);
        s.end_turn();
        s.expect_health(PlayerId::P2, 29);
    }

    #[test]
    fn r386_the_divisor_reads_through_param() {
        let mut s = standing(false);
        let id = pod(&s);
        set_param(s.card_mut(id.as_str()), "divisor", 2);
        s.end_turn();
        s.expect_health(PlayerId::P2, 29);
    }

    #[test]
    fn r1878001_played_at_2_it_is_a_starship_piece_with_equip_and_at_1_neither() {
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
    }

    #[test]
    fn r1878002_r1852001_equipped_at_2_it_runs_on_its_host_reading_the_hosts_attack_and_leaves_it_no_piece() {
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
        // The host keeps its zone and takes the Pod's 2/1: 6/5, no Starship Piece (R1852001).
        assert_eq!(s.unit(P1, 1).expect("the fused host").id, host);
        s.expect_stats(host.as_str(), json!({ "attack": 6, "health": 5 }));
        assert!(!has_keyword(&s.stats(host.as_str()).keywords, KeywordKind::StarshipPiece));
        s.end_turn();
        s.expect_health(PlayerId::P2, 29);
    }

    #[test]
    fn radiant_4_attack_divided_by_3_hits_each_enemy_for_1() {
        let mut s = standing(true);
        s.end_turn();
        let hits = hits(&s);
        assert_eq!(hits.iter().map(|hit| hit.1).collect::<Vec<_>>(), vec![1, 1, 1], "{hits:?}");
    }
}
