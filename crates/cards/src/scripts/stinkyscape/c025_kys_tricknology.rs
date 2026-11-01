//! Stinkyscape #25 KY's Tricknology (SPEC §8.10 row 25). (2) Spell, KY, Epic.
//!   Base:    "Choose a zone. Its Jacobian stretches a unit square. Summon a random Starship Piece in each
//!            zone it covers, for the player whose zone it is. F(x, y) = (x² − y, xy)."
//!   Radiant: "Choose a zone. Its Jacobian stretches a unit square. Summon a random Radiant Starship Piece
//!            in each of your zones it covers. F(x, y) = (x² − y, xy)."
//!
//! a zone's point is its center. The caster chooses any zone (`choose_cell`, B5 E18). The unit square
//! mapped through the Jacobian of F at that point, J = [[2x, −1], [y, x]], with its corner at the zone,
//! is the parallelogram of the points (x, y) + s·(2x, y) + t·(−1, x), s and t in 0 to 1, edges included
//! (`covers`; at (1, −2) J is singular and it is a segment). Each unit zone whose point lies in it, open
//! and not Locked, gets one random Starship Piece Unit (`query::starship_pieces()`, R1878000, narrowed to
//! Units as S #26's is, so S #131 Spacerace Stimulus, a Field Trap, is never one) summoned
//! for the player whose zone it is (`summon_random`: no Cry, R1; R387), the caster's zones first, each

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-025";

/// The resume step the chosen zone comes back to (§10.6).
const CHOSEN: &str = "chosen";

/// R1877000: the y of each row, from the caster's seat.
const YOUR_UNITS_Y: i32 = 1;
const YOUR_BACKROW_Y: i32 = 2;

/// R1877000: a zone's point (x, y) from `caster`'s seat.
fn point_of(caster: PlayerId, zone: &ZoneRef) -> (i32, i32) {
    let y = if zone.row == Row::Units {
        YOUR_UNITS_Y
    } else {
        YOUR_BACKROW_Y
    };
    (zone.lane, if zone.player == caster { y } else { -y })
}

/// R1877000: whether `point` lies in the unit square mapped through F's Jacobian at `corner`, cornered
/// there: `point − corner = s·a + t·b` with a = (2x, y), b = (−1, x) and s, t in 0 to 1. With
/// det J = 2x² + y > 0 (every zone but (1, −2)) that is J⁻¹·d in the unit square, J⁻¹ = [[x, 1], [−y, 2x]]
/// / det, in integers; with det J = 0, a and b lie on one line and the cover is the segment of it from
/// min(0, a·b) to a·a + max(0, a·b) along a.
fn covers(corner: (i32, i32), point: (i32, i32)) -> bool {
    let (x, y) = corner;
    let (dx, dy) = (point.0 - x, point.1 - y);
    let det = 2 * x * x + y;
    if det > 0 {
        let s = x * dx + dy;
        let t = -y * dx + 2 * x * dy;
        return (0..=det).contains(&s) && (0..=det).contains(&t);
    }
    let a = (2 * x, y);
    let b = (-1, x);
    let on_line = a.0 * dy - a.1 * dx == 0;
    let along = a.0 * dx + a.1 * dy;
    let ab = a.0 * b.0 + a.1 * b.1;
    let aa = a.0 * a.0 + a.1 * a.1;
    on_line && (ab.min(0)..=aa + ab.max(0)).contains(&along)
}

/// R1877000: the unit zones a choice of `chosen` covers that take a piece now (open zones are
/// `summon_random`'s to find; a Locked one is passed by), the caster's first, each lane upward; on the
/// Radiant face only the caster's.
fn piece_zones(ctx: &EffectContext<'_>, chosen: &ZoneRef, radiant: bool) -> Vec<ZoneRef> {
    let caster = ctx.controller;
    let corner = point_of(caster, chosen);
    let mut sides = vec![caster];
    if !radiant {
        sides.push(opponent_of(caster));
    }
    sides
        .into_iter()
        .flat_map(|player| {
            (1..=UNIT_ZONES).map(move |lane| ZoneRef {
                player,
                row: Row::Units,
                lane,
            })
        })
        .filter(|zone| covers(corner, point_of(caster, zone)) && !is_locked(&*ctx.state, zone))
        .collect()
}

fn tricknology(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![choose_cell(json_as(
                json!({ "step": CHOSEN, "prompt": "KY's Tricknology: choose a zone" }),
            ))]
        })),
        resume: IndexMap::from([(
            CHOSEN,
            hook(move |ctx| {
                let Some(chosen) = chosen_cells(&*ctx).into_iter().next() else {
                    return vec![];
                };
                let mut query = crate::query::starship_pieces();
                query.type_ = Some(OneOrMany::One(CardType::Unit));
                piece_zones(&*ctx, &chosen, radiant)
                    .into_iter()
                    .map(|zone| {
                        summon_random(SummonRandomArgs {
                            query: Some(query.clone()),
                            player: Some(if zone.player == ctx.controller {
                                PlayerSpec::SelfSide
                            } else {
                                PlayerSpec::Enemy
                            }),
                            lane: Some(zone.lane),
                            radiant: radiant.then_some(true),
                            stack: None,
                        })
                    })
                    .collect()
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: tricknology(false),
        radiant: tricknology(true),
    }
}

// Stinkyscape #25 KY's Tricknology — SPEC §8.10 row 25, BUILD M12 row S 25 (R1877000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const POOL_SEEDS: u32 = 20;
    /// The fifteen Units that #2521 took off the Starship Pieces: the pool offers none of them.
    const NO_LONGER_PIECES: [&str; 15] = [
        "stinkyscape-068",
        "stinkyscape-069",
        "stinkyscape-072",
        "stinkyscape-073",
        "stinkyscape-074",
        "stinkyscape-075",
        "stinkyscape-076",
        "stinkyscape-077",
        "stinkyscape-078",
        "stinkyscape-080",
        "stinkyscape-081",
        "stinkyscape-082",
        "stinkyscape-083",
        "stinkyscape-085",
        "stinkyscape-086",
    ];

    fn every_point() -> Vec<(i32, i32)> {
        [1, 2, -1, -2]
            .into_iter()
            .flat_map(|y| (1..=UNIT_ZONES).map(move |x| (x, y)))
            .collect()
    }

    fn covers_by_inverse(corner: (i32, i32), point: (i32, i32)) -> bool {
        let (x, y) = (f64::from(corner.0), f64::from(corner.1));
        let (dx, dy) = (f64::from(point.0) - x, f64::from(point.1) - y);
        let det = 2.0 * x * x + y;
        let s = (x * dx + dy) / det;
        let t = (-y * dx + 2.0 * x * dy) / det;
        let within = |v: f64| (-1e-9..=1.0 + 1e-9).contains(&v);
        within(s) && within(t)
    }

    #[test]
    fn r1877000_the_cover_of_each_of_the_20_zones_follows_the_jacobian() {
        for corner in every_point() {
            let cover: Vec<(i32, i32)> = every_point().into_iter().filter(|p| covers(corner, *p)).collect();
            assert!(cover.contains(&corner), "{corner:?} covers itself");
            if corner == (1, -2) {
                assert_eq!(cover, vec![(1, -2)]);
                continue;
            }
            let by_inverse: Vec<(i32, i32)> =
                every_point().into_iter().filter(|p| covers_by_inverse(corner, *p)).collect();
            assert_eq!(cover, by_inverse, "{corner:?}");
        }
        let at = |corner| -> Vec<(i32, i32)> { every_point().into_iter().filter(|p| covers(corner, *p)).collect() };
        assert_eq!(at((1, 1)), vec![(1, 1), (1, 2), (2, 2), (3, 2)]);
        assert_eq!(at((3, -1)), vec![(3, 1), (4, 1), (5, 1), (2, 2), (3, -1), (4, -1), (5, -1)]);
    }

    fn board(radiant: bool, seed: &str, mine: Value, theirs: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": mine, "library": filler(6), "mana": 10 },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(6) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s
    }

    fn choose(s: &mut Scenario, zone: Selection) {
        s.play(ID, json!({}));
        let pending = s.state().pending.clone().expect("the zone prompt");
        assert_eq!(pending.kind, PromptKind::Cell);
        assert_eq!(pending.options.len(), 20, "any zone of the board");
        let key = pending
            .options
            .iter()
            .find(|option| option.selection == zone)
            .map(|option| option.key.clone())
            .expect("offered");
        s.answer(json!(key));
    }

    fn summoned(s: &Scenario) -> Vec<(PlayerId, i32, String, bool)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player, instance_id, def_id, row: Row::Units, lane, .. } => {
                    Some((*player, *lane, def_id.clone(), s.card(instance_id.as_str()).radiant))
                }
                _ => None,
            })
            .collect()
    }

    fn is_piece(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        def.type_ == CardType::Unit && has_keyword(&def.base.keywords, KeywordKind::StarshipPiece)
    }

    fn enemy_unit_lane(lane: i32) -> Selection {
        Selection::Zone { player: P2, row: Row::Units, lane }
    }

    #[test]
    fn r1877000_each_covered_open_unit_zone_gets_a_piece_for_its_side_the_same_seed_the_same() {
        crate::register_all();
        let run = || {
            let mut s = board(false, "tricknology", json!([]), json!([]));
            choose(&mut s, enemy_unit_lane(3));
            summoned(&s)
        };
        let pieces = run();
        let places: Vec<(PlayerId, i32)> = pieces.iter().map(|(player, lane, ..)| (*player, *lane)).collect();
        assert_eq!(places, vec![(P1, 3), (P1, 4), (P1, 5), (P2, 3), (P2, 4), (P2, 5)]);
        assert!(pieces.iter().all(|(_, _, def_id, radiant)| is_piece(def_id) && !radiant));
        assert_eq!(run(), pieces);
    }

    #[test]
    fn its_pool_is_the_starship_piece_units_never_spacerace_stimulus_s_field_trap() {
        crate::register_all();
        // S #131 Spacerace Stimulus prints Starship Piece on a Field Trap (§8.10 row 131): never summoned.
        const SPACERACE_STIMULUS: &str = "stinkyscape-131";
        let mut seen: Vec<String> = Vec::new();
        for seed in 0..POOL_SEEDS {
            let mut s = board(false, &format!("tricknology-pool-{seed}"), json!([]), json!([]));
            choose(&mut s, enemy_unit_lane(3));
            seen.extend(summoned(&s).into_iter().map(|(_, _, def_id, _)| def_id));
        }
        assert!(seen.iter().all(|def_id| is_piece(def_id)), "{seen:?}");
        assert!(!seen.iter().any(|def_id| def_id == SPACERACE_STIMULUS));
        assert!(
            !seen.iter().any(|def_id| NO_LONGER_PIECES.contains(&def_id.as_str())),
            "{seen:?}"
        );
        assert!(has_keyword(
            &crate::card_def(SPACERACE_STIMULUS).base.keywords,
            KeywordKind::StarshipPiece
        ));
    }

    #[test]
    fn r1877000_an_occupied_or_locked_zone_gets_none() {
        crate::register_all();
        let mut s = board(false, "tricknology-full", json!([{ "def": VANILLA, "lane": 3 }]), json!([]));
        lock_zone(s.state_mut(), ZoneRef { player: P2, row: Row::Units, lane: 4 });
        choose(&mut s, enemy_unit_lane(3));
        let places: Vec<(PlayerId, i32)> = summoned(&s).iter().map(|(player, lane, ..)| (*player, *lane)).collect();
        assert_eq!(places, vec![(P1, 4), (P1, 5), (P2, 3), (P2, 5)]);
    }

    #[test]
    fn r1877000_a_cover_of_backrow_zones_alone_summons_nothing() {
        crate::register_all();
        let mut s = board(false, "tricknology-backrow", json!([]), json!([]));
        choose(&mut s, Selection::Zone { player: P2, row: Row::Backrow, lane: 1 });
        assert!(summoned(&s).is_empty());
    }

    #[test]
    fn r1877000_radiant_only_your_zones_each_piece_radiant() {
        crate::register_all();
        let mut s = board(true, "tricknology-radiant", json!([]), json!([]));
        choose(&mut s, enemy_unit_lane(3));
        let pieces = summoned(&s);
        let places: Vec<(PlayerId, i32)> = pieces.iter().map(|(player, lane, ..)| (*player, *lane)).collect();
        assert_eq!(places, vec![(P1, 3), (P1, 4), (P1, 5)]);
        assert!(pieces.iter().all(|(_, _, def_id, radiant)| is_piece(def_id) && *radiant));
    }
}
