//! Stinkyscape #53 Flak v3 (SPEC §8.10 row 53). Unit 0/4 → 0/8, Weapon, cost 2, Rare; held (R1420).
//!   Base:    "Starship Piece, Equip
//!            Activate: Deal {damage} damage to all enemies." — damage 2
//!   Radiant: the same text — damage 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! R1862001: "all enemies" is the enemy hero and every enemy Unit, one hit each before one state check
//! (`damage_all`, R59). No half damage: the hero takes the hit whole. Once a turn (R384). The number
//! is the declared `damage` (R386), read through `param`.

use jackioh_engine::effects::damage_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-053";

/// R1862001: "deal `amount` damage to all enemies".
fn round(amount: i32) -> Vec<Effect> {
    vec![damage_all(json_as(
        json!({ "amount": amount, "side": "enemy", "heroes": true }),
    ))]
}

fn flak() -> ActivationDecl {
    ActivationDecl {
        id: "flak".into(),
        label: "Deal damage to all enemies".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(|ctx| round(param(&*ctx, "damage"))),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        activations: vec![flak()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #53 Flak v3 — SPEC §8.10 row 53, BUILD M12 row S 53: its Activate deals 2 to the enemy
// hero and every enemy Unit; the enemy hero takes it whole; once a turn, a second use refused;
// equipped onto a Unit, the host takes the Activate; damage reads through `param()`; radiant 0/8, 4
// damage.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{hits, next_turn_of, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    /// The printed `damage`, and the Radiant face's, and its Radiant health.
    const DAMAGE: i32 = 2;
    const RADIANT_DAMAGE: i32 = 4;
    const RADIANT_HEALTH: i32 = 8;
    /// One round.
    const ROUNDS: i32 = 1;

    fn flak_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s053",
            json!({ "field": [{ "def": ID, "radiant": radiant }, VANILLA] }),
            p2,
        )
    }

    /// The round hits both enemy Menaces, then the enemy hero, whole; nothing of p1's is hit.
    fn sweeps(radiant: bool, amount: i32) {
        let mut s = flak_on(radiant, json!({ "field": [MENACE, MENACE] }));
        let menaces = [1, 2].map(|lane| s.unit_at(P2, lane).id);
        s.activate(ID, json!({}));
        let round = vec![
            (menaces[0].clone(), amount),
            (menaces[1].clone(), amount),
            ("hero-p2".to_string(), amount),
        ];
        assert_eq!(
            hits(s.last_events()),
            (0..ROUNDS).flat_map(|_| round.clone()).collect::<Vec<_>>(),
            "radiant {radiant}"
        );
        s.expect_health(P2, HERO_HEALTH - ROUNDS * amount);
        s.expect_health(P1, HERO_HEALTH);
    }

    mod base {
        use super::*;

        #[test]
        fn r1862001_hits_the_enemy_hero_whole_and_every_enemy_unit() {
            sweeps(false, DAMAGE);
        }

        #[test]
        fn r384_once_a_turn_a_second_use_refused() {
            let mut s = flak_on(false, json!({}));
            s.activate(ID, json!({}));
            s.expect_refused_with(|s| s.activate(ID, json!({})), "already been used");
            next_turn_of(&mut s, P1);
            s.activate(ID, json!({}));
            s.expect_health(P2, HERO_HEALTH - 2 * ROUNDS * DAMAGE);
        }

        #[test]
        fn r1086_r1852001_an_equipped_host_takes_the_activate_and_is_no_starship_piece() {
            let mut s = weapon_board(
                "s053-equip",
                json!({ "hand": [ID], "field": [VANILLA] }),
                json!({}),
            );
            let host = s.unit_at(P1, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.card(host.as_str()).clone();
            assert!(!unit_has(s.state(), &fused, KeywordKind::StarshipPiece));
            s.activate(host.as_str(), json!({}));
            s.expect_health(P2, HERO_HEALTH - ROUNDS * DAMAGE);
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = flak_on(false, json!({}));
            let amount = crate::upgrade_number(&mut s, ID, "damage");
            s.activate(ID, json!({}));
            s.expect_health(P2, HERO_HEALTH - ROUNDS * amount);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862001_hits_the_enemy_hero_whole_and_every_enemy_unit_twice_as_hard() {
            sweeps(true, RADIANT_DAMAGE);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = flak_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, RADIANT_HEALTH));
        }
    }
}
