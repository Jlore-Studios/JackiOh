//! SPEC §8.12 S+ #159 Raven (Unit 2/2 → 4/4, 1/1 → 2/2 until issue #4440, cost 0, Rare), a card of the
//! Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Tribute 2
//!            Cry: Discover 1 Rare Weapon; Equip it into this."
//!   Radiant: "Tribute 2
//!            Cry: Discover 1 Epic Weapon; Equip it into this."
//!
//! Tribute 2 since issue #4440 (Tribute 1 before it) is the play validator's (§6.3, R101), read off
//! `staticFlags.tribute`, the keyword line's printed Tribute (R366): two Units of the player's are destroyed
//! to pay it, and the Cry runs once it has landed. `script_v0_6_9n` keeps Tribute 1 for the replays of
//! v0.6.9n and before (R767000).
//! R1862002: the Cry is `equip_picks` (R1841002), one Discover of three from the Weapon-tagged cards of the
//! printed rarity (the readings: Rare, Radiant Epic), the pool naming its set so that it reaches them while
//! the set is held (R1420). The pick is a fresh copy, not made Radiant, fused into this with its Activate
//! (R1751000): no Cry, no hand. The fusion wears the face its kept host wears (R77), so a Radiant Raven
//! wields its Weapon's Radiant face. The Weapons it takes, each a Starship Piece, leave it none (R1852001).
//! The count is the declared `rare` (R386, `param`).

use jackioh_engine::effects::equip_picks;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-159";

/// "Tribute 2" (R101), since issue #4440.
const TRIBUTE: i32 = 2;
/// "Tribute 1", through v0.6.9n.
const TRIBUTE_V0_6_9G: i32 = 1;

/// R1862002: "Discover `rare` `rarity` Weapons; Equip them into this", played for `tribute` Units.
fn arm(rarity: Rarity, tribute: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(tribute),
            ..StaticFlags::default()
        }),
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
        base: arm(Rarity::Rare, TRIBUTE),
        radiant: arm(Rarity::Epic, TRIBUTE),
    }
}

/// R767000: through v0.6.9n it was Tribute 1 (issue #4440); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_9n() -> CardScripts {
    CardScripts {
        base: arm(Rarity::Rare, TRIBUTE_V0_6_9G),
        radiant: arm(Rarity::Epic, TRIBUTE_V0_6_9G),
    }
}

// S+ #159 Raven — SPEC §8.12 row 159 (R1862002; R101, R1841002, R1751000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::effects::EQUIP_PICK_STEP;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// The S+ Rare and Epic Weapons (§8.12): each rarity's pool is these three.
    const RARE: [&str; 3] = ["stinkyscapeplus-008", "stinkyscapeplus-143", "stinkyscapeplus-146"];
    const EPIC: [&str; 3] = ["stinkyscapeplus-009", "stinkyscapeplus-144", "stinkyscapeplus-147"];

    /// Plays the card, two Units paying its Tribute 2, and answers its one Discover with the first offer.
    fn arms(radiant: bool, pool: &[&str]) -> (Scenario, String, String) {
        let mut s = weapon_board(
            "s159",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [VANILLA, VANILLA] }),
            json!({ "field": [VANILLA] }),
        );
        let tributes = [1, 2].map(|lane| s.unit_at(P1, lane).id);
        s.play(ID, json!({ "zone": 1, "tributes": tributes }));
        let host = s.unit_at(P1, 1).id;
        let pending = s.state().pending.clone().expect("a Discover is open");
        assert_eq!((pending.kind, pending.player_id), (PromptKind::Discover, P1));
        assert_eq!(pending.resume.step, EQUIP_PICK_STEP);
        let mut offered = s.prompt_modes();
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

        /// Issue #4440: Tribute 2 on both faces, 2/2 (Radiant 4/4).
        #[test]
        fn r101_it_needs_tribute_2_on_both_faces() {
            for radiant in [false, true] {
                let mut s = weapon_board(
                    "s159-tribute",
                    json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": [VANILLA, VANILLA] }),
                    json!({}),
                );
                assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
                // No tribute, or one Unit, is refused; two Units pay it.
                let [first, second] = [1, 2].map(|lane| s.unit_at(P1, lane).id);
                s.expect_refused(|s| s.play(ID, json!({ "zone": 3 })));
                s.expect_refused(|s| s.play(ID, json!({ "zone": 3, "tributes": [first.as_str()] })));
                assert!(s.state().pending.is_none());
                s.play(ID, json!({ "zone": 1, "tributes": [first.as_str(), second.as_str()] }));
                assert!(s.state().pending.is_some(), "radiant {radiant}: the Cry opens its Discover");
                s.expect_in_zone(first.as_str(), "graveyard");
                s.expect_in_zone(second.as_str(), "graveyard");
                let raven = s.unit_at(P1, 1).id;
                let stats = s.stats(raven.as_str());
                assert_eq!((stats.attack, stats.health), if radiant { (4, 4) } else { (2, 2) });
            }
        }

        #[test]
        fn r101_a_third_tribute_is_refused() {
            let mut s = weapon_board(
                "s159-over",
                json!({ "hand": [ID], "field": [VANILLA, VANILLA, VANILLA] }),
                json!({}),
            );
            let all = [1, 2, 3].map(|lane| s.unit_at(P1, lane).id);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 4, "tributes": all })));
        }

        /// R767000: through v0.6.9n one Unit paid it (issue #4440).
        #[test]
        fn r767000_the_script_kept_for_v0_6_9n_needs_tribute_1() {
            crate::under_kept(ID, script_v0_6_9n(), || {
                let mut s = weapon_board(
                    "s159-kept",
                    json!({ "hand": [ID], "field": [VANILLA, VANILLA] }),
                    json!({}),
                );
                assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE_V0_6_9G);
                let first = s.unit_at(P1, 1).id;
                s.play(ID, json!({ "zone": 1, "tributes": [first.as_str()] }));
                s.expect_in_zone(first.as_str(), "graveyard");
                assert!(s.unit(P1, 2).is_some(), "the second Unit stays");
            });
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
