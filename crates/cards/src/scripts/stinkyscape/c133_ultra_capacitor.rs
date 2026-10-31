//! S #133 Ultra Capacitor (SPEC §8.10 row 133; R1852000, R1852003, R1852005, R1859001, R1859004): (2)
//! Unit, Common, 2/2 → 4/4, Starship Piece, a card of the Stinkyscape set.
//!
//! Base:    "Starship Piece\nWhen launched, your Starship gains +{per}/+{per} for each other Starship
//!          Piece fused into it."
//! Radiant: "Starship Piece\nWhen launched, your Starship gains +{per}/+{per} for each Starship Piece
//!          fused into it." Per 1, Radiant 2.
//! Engine: its death fuses a copy into its owner's Dormant Starship (R1852000); When launched
//! (`when_launched`, R1852003) runs only as part of that Starship. The Starship's memory
//! (`starship_pieces_of`, R1852005) counts its pieces, each copy of the same card counted, this one's
//! copy left out on the base face and counted on the Radiant face (R1859004); then `buff` +N/+N per
//! piece on the Starship, lasting, nothing with none to count. Per is read on the face the piece fused
//! on (`piece_number`, R1859001). Tunes: per 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-133";

fn ultra_capacitor(radiant: bool) -> Script {
    Script {
        triggers: vec![when_launched("ultra-capacitor", move |ctx, _event| {
            let held = ctx
                .live_self()
                .map_or(0, |ship| starship_pieces_of(ship).len());
            let counted = i32::try_from(held).unwrap_or(i32::MAX);
            let counted = if radiant { counted } else { counted - 1 };
            if counted <= 0 {
                return vec![];
            }
            let gain = counted * piece_number(ctx, ID, radiant, "per");
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": gain,
                "health": gain,
            })))]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: ultra_capacitor(false),
        radiant: ultra_capacitor(true),
    }
}

// S #133 Ultra Capacitor — SPEC §8.10 row 133, BUILD M12 row S 133: "when the Starship it fused into
// launches holding it and three other pieces, the Starship gains +3/+3; alone in it, nothing; per read on
// the face it fused on (R1859001); radiant 4/4 and +2/+2 for each of the four, +8/+8".
#[cfg(test)]
mod tests {
    use super::ID;
    use crate::{built_starship, launch_starship};
    use jackioh_engine::testkit::*;

    const AP_ROUNDS: &str = "stinkyscape-100"; // (1) 3/1 Starship Piece, Equip, Pierce: keywords only.

    /// p1's Starship holding the Capacitor (lane 1) and `others` AP Rounds, launched; its stats before
    /// and after the launch.
    fn launched(seed: &str, radiant: bool, others: i32) -> ((i32, i32), (i32, i32)) {
        let mut pieces = vec![json!({ "def": ID, "lane": 1, "radiant": radiant })];
        pieces.extend((2..2 + others).map(|lane| json!({ "def": AP_ROUNDS, "lane": lane })));
        let (mut s, ship) = built_starship(seed, json!(pieces), json!({}), json!({}));
        let before = s.stats(ship.as_str());
        launch_starship(&mut s, &ship);
        let after = s.stats(ship.as_str());
        ((before.attack, before.health), (after.attack, after.health))
    }

    #[test]
    fn r1859004_three_other_pieces_give_the_starship_3_3() {
        let (before, after) = launched("r1859004-three", false, 3);
        assert_eq!(before, (2 + 9, 2 + 3));
        assert_eq!(after, (before.0 + 3, before.1 + 3));
    }

    #[test]
    fn r1859004_alone_in_its_starship_it_gives_nothing() {
        let (before, after) = launched("r1859004-alone", false, 0);
        assert_eq!(before, (2, 2));
        assert_eq!(after, before);
    }

    #[test]
    fn r1859004_radiant_it_counts_itself_at_2_2_a_piece() {
        let (before, after) = launched("r1859004-radiant", true, 3);
        assert_eq!(before, (4 + 9, 4 + 3));
        assert_eq!(after, (before.0 + 8, before.1 + 8));
        let (alone_before, alone_after) = launched("r1859004-radiant-alone", true, 0);
        assert_eq!(alone_after, (alone_before.0 + 2, alone_before.1 + 2));
    }
}
