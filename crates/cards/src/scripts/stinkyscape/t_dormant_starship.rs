//! S T-dormant-starship Dormant Starship (SPEC §8.10 row T-dormant-starship; R349, R1839000, R1852000,
//! R1852003–R1852007, R1857001, R1859013): (3) Unit, Legendary, 0/0, the set's one entry that is
//! neither a numbered card nor a token, of the held Stinkyscape set (R1420).
//!
//! Base and Radiant: "Dormant until it launches.\nEach of your Starship Pieces that dies fuses a copy
//! into this.\nA card played onto this by Stack while it is Dormant is exiled before it resolves.
//! \nActivate: Pay (3) to launch this. It takes a random name, then every Cry fused into it fires,
//! then every When launched."
//! Engine: its text is the engine's rules written out (R1859013), so its script is the hull alone:
//! `StaticFlags { slumber, starshipHull }` on both faces, Dormant with no countdown (R1839000), its
//! stats only what fuses into it (R77). The engine summons it when a piece dies (R1852000), exiles a
//! Stack card played onto it (R1852007) and gives it its one ability, the launch for
//! `STARSHIP_LAUNCH_COST` (`starship::launch_decl`, R1852003, moved by R1852006): a name from
//! `STARSHIP_NAMES` (R1852004), its fused Cries, then its fused When launched. No token, so it reaches
//! a graveyard; in no pool and no deck (R1857001); its Radiant face is R349's fallback
//! (`radiantFallback`). Declares no numbers: the price is the engine's constant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-dormant-starship";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText::default()),
            starship_hull: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S T-dormant-starship Dormant Starship — SPEC §8.10 row T-dormant-starship, BUILD M12 row S
// T-dormant-starship: "a Starship Piece's death summons it 0/0 into its owner's leftmost open unit zone,
// else on top of lane 3, and the piece's copy fuses in at once (R1852000); it is Dormant, unattackable and
// unreachable (R1839000); a Stack card played onto it is exiled before it resolves (R1852007); its
// Activate for (3) launches it: the Starship keyword, a name (R1852004), its fused Cries, then When
// launched (R1852003); "Your Starship launch costs (0)" prices it (R1852006); no pool and no deck holds it
// (R1857001); radiant R349's fallback, 0/0".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use crate::{built_starship, card_def, launch_starship};
    use jackioh_engine::prelude::{STARSHIP_LAUNCH_ABILITY, STARSHIP_LAUNCH_COST, is_launched_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const STARTOS: &str = "stinkyscape-068"; // (1) 3/3 Starship Piece, Flying: keywords only.

    #[test]
    fn r1859013_its_script_is_the_hull_on_both_faces() {
        let scripts = script();
        for face in [&scripts.base, &scripts.radiant] {
            let flags = face.static_flags.as_ref().expect("the hull's flags");
            assert_eq!(flags.starship_hull, Some(true));
            assert!(flags.slumber.is_some());
            assert!(face.triggers.is_empty() && face.activations.is_empty() && face.cry.is_none());
        }
        let def = card_def(ID);
        assert!(!def.token);
        assert_eq!(def.rarity, Rarity::Legendary);
        assert_eq!(def.radiant_fallback, Some(true));
        assert_eq!((def.base.attack, def.base.health), (Some(0), Some(0)));
    }

    #[test]
    fn r1859013_a_dying_piece_summons_it_holding_the_copy_and_its_launch_is_offered_at_3() {
        let (mut s, ship) = built_starship(
            "r1859013-hull",
            json!([{ "def": STARTOS, "lane": 2 }]),
            json!({ "field": [{ "def": STARTOS, "lane": 1 }] }),
            json!({}),
        );
        let card = s.card(ship.as_str()).clone();
        assert!(matches!(card.zone, Zone::Field { player: P1, row: Row::Units, lane: 2 }));
        let stats = s.stats(ship.as_str());
        assert_eq!((stats.attack, stats.health), (3, 3), "only what fused into it");
        let view = s.view(P1);
        let unit = view.you.units[1].as_ref().expect("lane 2 shows the Starship");
        let abilities = unit.activations.clone().expect("the launch");
        assert_eq!(abilities.len(), 1);
        assert_eq!(abilities[0].ability, STARSHIP_LAUNCH_ABILITY);
        assert_eq!(abilities[0].label, format!("Launch ({STARSHIP_LAUNCH_COST})"));
        let mana = s.state().players.p1.mana.current;
        launch_starship(&mut s, &ship);
        assert_eq!(s.state().players.p1.mana.current, mana - STARSHIP_LAUNCH_COST);
        assert!(is_launched_starship(s.card(ship.as_str())));
        assert!(
            s.stats(ship.as_str())
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::Starship)
        );
    }
}
