//! SPEC §8.12 S+ #16 Servo Surge (Unit 2/1 → 4/2, Starship Piece, Equip, Windfury, cost 2, Rare), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base and Radiant: "Starship Piece, Equip, Windfury"
//!
//! Every word is a catalog keyword the engine plays, so there is nothing to run. Played onto one of your
//! Units it fuses into it (R1086), which attacks twice a turn with it (§6.1); the Radiant face differs by
//! its stats alone.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-016";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face only doubles the stats: catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #16 Servo Surge — SPEC §8.12 row 16 (§6.1; R1086).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// An equipped host (a piece standing alone on the field is fused into its Dormant Starship, R1852000;
    /// played onto a host it takes its Windfury, R1086), and a durable enemy.
    const HOST: &str = "stand-in (1) 3/5 R9/10";
    const TOUGH: &str = "stand-in (1) 2/60";

    fn windfury(s: &Scenario, id: &str) -> bool {
        s.stats(id).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Windfury)
    }

    fn equipped(radiant: bool) -> (Scenario, String) {
        let mut s = weapon_board(
            "s016",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [{ "def": HOST, "radiant": radiant }], "mana": 10 }),
            json!({ "field": [TOUGH] }),
        );
        let host = s.unit_at(P1, 1).id;
        assert!(!windfury(&s, &host));
        s.play(ID, json!({ "zone": 1, "equip": true }));
        (s, host)
    }

    mod base {
        use super::*;

        #[test]
        fn r1086_an_equipped_host_gains_windfury_and_attacks_twice_no_third_time() {
            let (mut s, host) = equipped(false);
            assert!(windfury(&s, &host));
            let enemy = s.unit_at(P2, 1).id;
            let attack = s.stats(host.as_str()).attack;
            s.attack(host.as_str(), enemy.as_str());
            s.attack(host.as_str(), enemy.as_str());
            s.expect_stats(enemy.as_str(), json!({ "health": 60 - 2 * attack }));
            s.expect_refused(|s| s.attack(host.as_str(), enemy.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1086_a_radiant_host_gains_windfury_and_attacks_twice_no_third_time() {
            let (mut s, host) = equipped(true);
            assert!(windfury(&s, &host));
            let enemy = s.unit_at(P2, 1).id;
            let attack = s.stats(host.as_str()).attack;
            s.attack(host.as_str(), enemy.as_str());
            s.attack(host.as_str(), enemy.as_str());
            s.expect_stats(enemy.as_str(), json!({ "health": 60 - 2 * attack }));
            s.expect_refused(|s| s.attack(host.as_str(), enemy.as_str()));
        }
    }
}
