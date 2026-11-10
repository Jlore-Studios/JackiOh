//! SPEC §8.12 S+ #60 Radi (Unit 1/1 → 2/2, cost 2, Rare), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Cry: Discover 1 Rare Weapon; Equip it into this."
//!   Radiant: "Cry: Discover 1 Epic Weapon; Equip it into this."
//!
//! R1862002: the Cry is `equip_picks` (R1841002), one Discover of three from the Weapon-tagged cards of the
//! printed rarity (the readings: Rare, Radiant Epic), the pool naming its set so that it reaches them while
//! the set is held (R1420). The pick is a fresh copy, not made Radiant, fused into this with its Activate
//! (R1751000): no Cry, no hand. The fusion wears the face its kept host wears (R77), so a Radiant Radi
//! wields its Weapon's Radiant face. The Weapons it takes, each a Starship Piece, leave it none (R1852001).
//! The count is the declared `rare` (R386, `param`).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-060";

/// R1862002: "Discover `rare` `rarity` Weapons; Equip them into this".
fn arm(rarity: Rarity) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![equip_picks(json_as(json!({
                "query": { "set": SetName::StinkyscapePlus, "tags": [Tag::Weapon], "rarity": rarity },
                "times": param(ctx, "rare"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare),
        radiant: arm(Rarity::Epic),
    }
}

// S+ #60 Radi — SPEC §8.12 row 60 (R1862002; R101, R1841002, R1751000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// The S+ Rare and Epic Weapons (§8.12): each rarity's pool is these three.
    const RARE: [&str; 3] = ["stinkyscapeplus-008", "stinkyscapeplus-143", "stinkyscapeplus-146"];
    const EPIC: [&str; 3] = ["stinkyscapeplus-009", "stinkyscapeplus-144", "stinkyscapeplus-147"];

    /// Plays the card and answers its one Discover with the first offer.
    fn arms(radiant: bool, pool: &[&str]) -> (Scenario, String, String) {
        let mut s = weapon_board(
            "s060",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [VANILLA] }),
            json!({ "field": [VANILLA] }),
        );
        s.play(ID, json!({ "zone": 2 }));
        let host = s.unit_at(P1, 2).id;
        let pending = s.state().pending.clone().expect("a Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
        let mut offered: Vec<String> = pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect();
        offered.sort();
        assert_eq!(offered, pool, "radiant {radiant}: the whole pool is offered");
        let before = s.stats(host.as_str()).health;
        s.answer(json!(offered[0]));
        // R77: the fusion wears the face its kept host wears.
        let def = crate::card_def(&offered[0]);
        let gained = if radiant { def.radiant.health } else { def.base.health }.unwrap_or_default();
        assert_eq!(s.stats(host.as_str()).health, before + gained, "{}", offered[0]);
        assert!(s.state().pending.is_none(), "one Discover, answered");
        (s, host, offered[0].clone())
    }

    mod base {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_one_rare_weapon_and_equips_it() {
            let (s, host, _) = arms(false, &RARE);
            assert!(s.card(host.as_str()).def_id.ends_with(ID), "the host wears the Weapon fused into it");
            let card = s.card(host.as_str()).clone();
            assert!(!unit_has(s.state(), &card, KeywordKind::StarshipPiece));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862002_the_cry_discovers_one_epic_weapon_and_equips_it() {
            let (s, host, _) = arms(true, &EPIC);
            assert!(s.card(host.as_str()).def_id.ends_with(ID), "the host wears the Weapon fused into it");
            assert!(s.card(host.as_str()).radiant);
        }
    }
}
