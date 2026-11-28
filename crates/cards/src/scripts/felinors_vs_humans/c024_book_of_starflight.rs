//! SPEC §8.14 FvH #24 Book of Starflight. (1) Spell, Book, Epic. Built by S03 (#5253).
//!   Base:    "Summon a Dormant Starship with {pieces|random Starship Piece|random Starship Pieces} fused
//!            into it." — 2
//!   Radiant: "Summon a Dormant Starship with {pieces|random Radiant Starship Piece|random Radiant Starship
//!            Pieces} fused into it." — 2
//!
//! R5253000 (FV2 #24): one generic Dormant Starship hull (`DORMANT_STARSHIP_DEF_ID`), summoned the way S+
//! #155 The Far Vagabond summons its Terminus Starship (`summon_starships`, R3539001), holding the declared
//! `pieces` (R386) drawn from the Starship Piece pool (`query::starship_pieces()`, R1878000) as it
//! resolves, each draw through the run's tally (R3563000). Nothing pauses this list, so the hook draws
//! them. With no open zone nothing is summoned and nothing is drawn (R129). It launches by hand at its
//! usual price (R1852003). The Radiant face's hull and pieces are Radiant (the hull's two faces are alike,
//! R349).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-024";

/// R5253000: the hull and its `pieces` random Starship Pieces, the draw skipped when no zone is open.
fn starflight(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // R129: a summon with no zone to land in draws nothing.
            if jackioh_engine::starship::open_starship_zone(&*ctx.state, ctx.controller).is_none() {
                return vec![];
            }
            let count = param(&*ctx, "pieces");
            let pool = crate::query::query(&crate::query::starship_pieces());
            let mut pieces: Vec<String> = Vec::new();
            for _ in 0..count.max(0) {
                let Some(piece) = pick_unique(&mut *ctx.sink.rng, &pool, &mut ctx.draw_tally) else {
                    break;
                };
                pieces.push(piece.id.clone());
            }
            vec![summon_starships(SummonStarshipsArgs {
                blueprints: vec![StarshipBlueprint {
                    def_id: DORMANT_STARSHIP_DEF_ID.to_string(),
                    pieces,
                }],
                count: 1,
                radiant: radiant.then_some(true),
                ..SummonStarshipsArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: starflight(false),
        radiant: starflight(true),
    }
}

// FvH #24 Book of Starflight — SPEC §8.14 row 24 (R5253000, R1878000, R3539001, R129, R1852003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-005"; // Stockpile
    const TRAP: &str = "stand-in (1) Trap";
    const BODY: &str = "stand-in (1) 1/1";

    fn holding(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn the_ship(s: &Scenario) -> CardInstance {
        must(dormant_starship_of(s.state(), P1).cloned(), "p1's Dormant Starship")
    }

    fn prints_piece(def_id: &str) -> bool {
        has_keyword(&crate::card_def(def_id).base.keywords, KeywordKind::StarshipPiece)
    }

    mod base {
        use super::*;

        #[test]
        fn r5253000_summons_a_dormant_starship_holding_two_different_random_starship_pieces() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for n in 0..8 {
                let mut s = holding(false, &format!("starflight-{n}"));
                s.play(ID, json!({}));
                let ship = the_ship(&s);
                assert_eq!(hull_def_of(s.state(), &ship.def_id), DORMANT_STARSHIP_DEF_ID);
                assert!(!ship.radiant);
                let pieces = starship_pieces_of(&ship);
                assert_eq!(pieces.len(), 2, "{pieces:?}");
                // R3563000: one run's draws differ.
                assert_ne!(pieces[0].def_id, pieces[1].def_id);
                for piece in nonempty(pieces, "the pieces fused") {
                    assert!(prints_piece(&piece.def_id), "{} is a Starship Piece", piece.def_id);
                    assert!(!crate::card_def(&piece.def_id).is_token(), "{}", piece.def_id);
                    assert_eq!(piece.radiant, None, "{}", piece.def_id);
                    seen.insert(piece.def_id);
                }
            }
            assert!(seen.len() > 2, "{seen:?}");
        }

        #[test]
        fn r1852003_it_launches_by_hand_at_its_usual_price() {
            let mut s = holding(false, "starflight-launch");
            s.play(ID, json!({}));
            let ship = the_ship(&s);
            assert_eq!(hand_launch_cost(s.state(), &ship), Some(STARSHIP_LAUNCH_COST));
            let decls = jackioh_engine::subsystems::activate::abilities_of(s.state(), &ship);
            let launch = must(decls.iter().find(|decl| decl.id == STARSHIP_LAUNCH_ABILITY), "the launch");
            assert_eq!(launch.label, format!("Launch ({STARSHIP_LAUNCH_COST})"));
            crate::launch_starship(&mut s, &ship.id);
            assert!(is_launched_starship(s.card(&ship.id)));
        }

        #[test]
        fn r129_with_no_open_zone_it_summons_nothing_and_draws_nothing() {
            let field = vec![json!(BODY); UNIT_ZONES as usize];
            let backrow = vec![json!({ "def": TRAP, "faceUp": false }); BACKROW_ZONES as usize];
            let mut s = scenario(json!({
                "seed": "starflight-full",
                "p1": { "hand": [ID, FILLER], "field": field, "backrow": backrow, "library": filler(4), "mana": 10 },
                "p2": { "hand": [FILLER], "library": filler(4) },
            }));
            let before = s.state().rng_cursor;
            s.play(ID, json!({}));
            assert!(dormant_starship_of(s.state(), P1).is_none());
            assert_eq!(s.state().rng_cursor, before);
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r386_pieces_reads_through_param() {
            let mut s = holding(false, "starflight-param");
            step_param(s.card_mut(ID), "pieces", 1);
            s.play(ID, json!({}));
            assert_eq!(starship_pieces_of(&the_ship(&s)).len(), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5253000_the_pieces_fused_are_radiant() {
            let mut s = holding(true, "starflight-radiant");
            s.play(ID, json!({}));
            let ship = the_ship(&s);
            assert_eq!(hull_def_of(s.state(), &ship.def_id), DORMANT_STARSHIP_DEF_ID);
            let pieces = starship_pieces_of(&ship);
            assert_eq!(pieces.len(), 2, "{pieces:?}");
            for piece in nonempty(pieces, "the pieces fused") {
                assert!(prints_piece(&piece.def_id), "{}", piece.def_id);
                assert_eq!(piece.radiant, Some(true), "{}", piece.def_id);
            }
        }
    }
}
