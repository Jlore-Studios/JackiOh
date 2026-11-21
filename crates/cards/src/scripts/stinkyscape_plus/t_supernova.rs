//! SPEC §8.12 S+ #T-supernova Supernova (4) Spell, Terminus, Mythic, a Terminus token of the held
//! Stinkyscape+ set (R1420, R3535000; R3569002).
//!   Base:    "Fuse all Units in the board into an Alnilam."
//!   Radiant: "Fuse all Units in the board into a Radiant Alnilam."
//!
//! Engine: `fuseBoardInto` takes every Unit of both sides the Spell reaches out of the game as the
//! ingredients of a fusion (R77, R86: no Death, nothing destroyed), summons a fresh S+ #T-alnilam, on
//! its Radiant face for the Radiant card, onto the caster's side, and fuses the ingredients into it, so
//! the one card that stands there is "Alnilam + …" with every stat, keyword and text (R3569002). A
//! Dormant or Immutable Unit and one Immune to Spells stays. With no Unit to fuse it is summoned alone,
//! and with no room for it nothing happens. On a slashed board its room is a zone on the half it
//! resolves on (R4318001).

use jackioh_engine::prelude::*;

use crate::scripts::stinkyscape_plus::t_alnilam;

pub const ID: &str = "stinkyscapeplus-t-supernova";

fn supernova(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![fuse_board_into(json_as(json!({
                "defId": t_alnilam::ID,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: supernova(false),
        radiant: supernova(true),
    }
}

// S+ #T-supernova — SPEC §8.12 (R3569002; R77, R86).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const ALNILAM: &str = t_alnilam::ID;
    /// Units of stats the sums below name.
    const SMALL: &str = "stand-in (1) 1/2";
    const MEDIUM: &str = "stand-in (1) 3/4";

    fn game(seed: &str, radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": p1_field,
                "library": filler(6),
                "mana": 4,
            },
            "p2": { "hand": [], "field": p2_field, "library": filler(6) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    /// Whether `card` is a fusion that Alnilam leads (R77, R179).
    fn leads_with_alnilam(s: &Scenario, card: &CardInstance) -> bool {
        fused_id_parts(Some(s.state()), &card.def_id)
            .is_some_and(|parts| parts.first().map(String::as_str) == Some(ALNILAM))
    }

    fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5).filter_map(|lane| s.unit(player, lane)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3569002_r4353003_both_boards_fuse_into_an_alnilam_its_indestructible_refusing_nothing() {
            let mut s = game(
                "supernova-base",
                false,
                json!([SMALL, MEDIUM]),
                json!([MEDIUM, SMALL]),
            );
            s.play(ID, json!({}));
            assert!(units_of(&s, P2).is_empty(), "the enemy's Units are fused away");
            let mine = units_of(&s, P1);
            assert_eq!(mine.len(), 1, "the one fused card stands on the caster's side");
            let fused = &mine[0];
            assert!(leads_with_alnilam(&s, fused), "{}", fused.def_id);
            assert!(!fused.radiant);
            let stats = s.stats(fused.id.as_str());
            assert_eq!(
                (stats.attack, stats.health),
                (10 + 1 + 3 + 3 + 1, 10 + 2 + 4 + 4 + 2)
            );
            let name = def_of(Some(s.state()), &fused.def_id).name.clone();
            assert!(name.starts_with("Alnilam + "), "{name}");
            // R77, R86: nothing of it died or was destroyed.
            assert!(s.pile(P1, "graveyard").iter().all(|card| card.def_id == ID));
            assert!(s.pile(P2, "graveyard").is_empty());
        }

        #[test]
        fn r3569002_with_no_unit_a_bare_alnilam_is_summoned() {
            let mut s = game("supernova-bare", false, json!([]), json!([]));
            s.play(ID, json!({}));
            let mine = units_of(&s, P1);
            assert_eq!(mine.len(), 1);
            assert_eq!(mine[0].def_id, ALNILAM);
            let stats = s.stats(mine[0].id.as_str());
            assert_eq!((stats.attack, stats.health), (10, 10));
        }
    }

    /// R4318001: a board slashed at lane 3 (R3554003), the caster's Right half filled by two Dormant
    /// Units, which stay: cast on the Right half, Alnilam has no zone there, free or freed by the
    /// caster's Unit on the Left, so nothing leaves and nothing is made.
    #[test]
    fn r4318001_on_a_slashed_board_with_no_room_on_its_half_nothing_leaves() {
        for radiant in [false, true] {
            let mut s = game(
                "supernova-slashed",
                radiant,
                json!([
                    { "def": SMALL, "lane": 1 },
                    { "def": MEDIUM, "lane": 4, "slumber": {} },
                    { "def": MEDIUM, "lane": 5, "slumber": {} },
                ]),
                json!([{ "def": SMALL, "lane": 2 }]),
            );
            s.state_mut().slashed_lane = Some(3);
            let ids = |s: &Scenario, player: PlayerId| -> Vec<String> {
                units_of(s, player).into_iter().map(|card| card.id).collect()
            };
            let (mine, theirs) = (ids(&s, P1), ids(&s, P2));
            s.play(ID, json!({}));
            s.answer(json!(["right"]));
            assert_eq!(ids(&s, P1), mine, "radiant {radiant}: the caster's Units stay");
            assert_eq!(ids(&s, P2), theirs, "radiant {radiant}: the enemy's Unit stays");
            assert!(
                !units_of(&s, P1).iter().any(|card| leads_with_alnilam(&s, card)),
                "radiant {radiant}: no Alnilam is made"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3569002_r4353003_radiant_supernova_makes_a_radiant_alnilam() {
            let mut s = game("supernova-radiant", true, json!([SMALL]), json!([MEDIUM]));
            s.play(ID, json!({}));
            assert!(units_of(&s, P2).is_empty());
            let mine = units_of(&s, P1);
            assert_eq!(mine.len(), 1);
            assert!(leads_with_alnilam(&s, &mine[0]), "{}", mine[0].def_id);
            assert!(mine[0].radiant, "Alnilam is Radiant, so the fusion is");
            let stats = s.stats(mine[0].id.as_str());
            assert!(stats.attack >= 20 && stats.health >= 20, "{stats:?}");
        }

        #[test]
        fn r3569002_radiant_with_no_unit_summons_the_bare_radiant_card() {
            let mut s = game("supernova-radiant-bare", true, json!([]), json!([]));
            s.play(ID, json!({}));
            let mine = units_of(&s, P1);
            assert_eq!(mine.len(), 1);
            assert_eq!(mine[0].def_id, ALNILAM);
            assert!(mine[0].radiant);
            let stats = s.stats(mine[0].id.as_str());
            assert_eq!((stats.attack, stats.health), (20, 20));
        }
    }
}
