//! The map as a graph (R3583000, R3583001), on both contents over a sweep of seeds: its shape (the
//! acts and their lone bosses), its edges (none crossing, none wider than a column but a boss's), its
//! reach (no orphan, no dead end) and every route's fights and tiles, by dynamic programming over the
//! whole graph rather than by sampling routes. Each check has a hand-built map that fails it.

use super::*;
use crate::config::MAP_SEEDS;
use crate::fight_tests::catalog_content;

/// A run on `content` from seed `n` of the sweep, its starting deck the content's first starting cards.
fn deal(content: &RunContent, n: usize) -> RunState {
    let setup = RunSetup {
        seed: run_seed(&format!("map-3583-{n}")),
        starter: content.starter_access[..RUN_STARTER_SIZE].to_vec(),
        artifacts: Vec::new(),
        curses: Vec::new(),
    };
    run_create(&setup, &RunUnlocks::default(), content).unwrap_or_else(|e| panic!("seed {n}: no run: {e}"))
}

/// The fewest and the most fights, and the fewest tiles, on any route from a node of the first row to
/// one of the last.
#[derive(Debug, PartialEq, Eq)]
struct Bounds {
    fights_min: i32,
    fights_max: i32,
    tiles_min: i32,
}

/// `Bounds` by dynamic programming, row by row: each node reached keeps the fewest and the most fights
/// and the fewest tiles of the routes to it, a node no route reaches is skipped.
fn route_bounds(map: &[Vec<RunNode>]) -> Bounds {
    let own = |node: &RunNode| {
        let fight = i32::from(node.kind.is_fight());
        (fight, fight, 1 - fight)
    };
    let mut reached: Vec<Option<(i32, i32, i32)>> = map[0].iter().map(|node| Some(own(node))).collect();
    for r in 1..map.len() {
        reached = map[r]
            .iter()
            .enumerate()
            .map(|(j, node)| {
                let from: Vec<(i32, i32, i32)> = map[r - 1]
                    .iter()
                    .zip(&reached)
                    .filter(|(below, _)| below.next.contains(&j))
                    .filter_map(|(_, sums)| *sums)
                    .collect();
                let (fights, _, tiles) = own(node);
                Some((
                    from.iter().map(|s| s.0).min()? + fights,
                    from.iter().map(|s| s.1).max()? + fights,
                    from.iter().map(|s| s.2).min()? + tiles,
                ))
            })
            .collect();
    }
    let ends: Vec<(i32, i32, i32)> = reached.into_iter().flatten().collect();
    Bounds {
        fights_min: ends.iter().map(|s| s.0).min().unwrap_or(0),
        fights_max: ends.iter().map(|s| s.1).max().unwrap_or(0),
        tiles_min: ends.iter().map(|s| s.2).min().unwrap_or(0),
    }
}

/// Whether row `r` is a boss's: the last of each act.
fn boss_row(r: usize) -> bool {
    (r + 1).is_multiple_of(RUN_ACT_ROWS + 1)
}

/// The acts and their lone bosses: `RUN_ACTS` acts of `RUN_ACT_ROWS` rows and a boss's row, each boss
/// alone on the middle column (the last the final boss), every other row 1 to `RUN_MAP_COLUMNS` nodes
/// on rising columns, each act's first row `RUN_MAP_PATHS` starts, the first row all fights, and every
/// node's `next` ascending and in range: the row before a boss leads to it, a mini-boss to every node
/// of the next act's first row, the final boss nowhere.
fn check_shape(map: &[Vec<RunNode>]) -> Result<(), String> {
    if map.len() != RUN_ACTS * (RUN_ACT_ROWS + 1) {
        return Err(format!("the map holds {} rows", map.len()));
    }
    for (r, row) in map.iter().enumerate() {
        let last = r + 1 == map.len();
        if boss_row(r) {
            let kind = if last {
                RunNodeKind::Boss
            } else {
                RunNodeKind::MiniBoss
            };
            if row.len() != 1 || row[0].kind != kind || row[0].column != RUN_MAP_COLUMNS / 2 {
                return Err(format!("row {r} is no lone {kind:?} on the middle column"));
            }
        } else {
            if row.is_empty() || row.len() > RUN_MAP_COLUMNS {
                return Err(format!("row {r} holds {} nodes", row.len()));
            }
            if row.iter().any(|node| node.kind.is_boss()) {
                return Err(format!("row {r} holds a boss"));
            }
            if row.windows(2).any(|pair| pair[0].column >= pair[1].column)
                || row.iter().any(|node| node.column >= RUN_MAP_COLUMNS)
            {
                return Err(format!("row {r}'s columns do not rise across the grid"));
            }
        }
        if r.is_multiple_of(RUN_ACT_ROWS + 1) && row.len() != RUN_MAP_PATHS {
            return Err(format!("act's first row {r} holds {} starts", row.len()));
        }
        let above = map.get(r + 1).map_or(0, Vec::len);
        for (c, node) in row.iter().enumerate() {
            if node.next.windows(2).any(|pair| pair[0] >= pair[1]) || node.next.iter().any(|j| *j >= above) {
                return Err(format!("({r}, {c}) leads to {:?} of {above}", node.next));
            }
            let fan: Vec<usize> = (0..above).collect();
            let wanted = if last {
                Some(&[][..])
            } else if boss_row(r + 1) {
                Some(&[0][..])
            } else if boss_row(r) {
                Some(&fan[..])
            } else {
                None
            };
            if wanted.is_some_and(|wanted| node.next != wanted) {
                return Err(format!("({r}, {c}) leads to {:?}, not {wanted:?}", node.next));
            }
        }
    }
    if let Some(c) = map[0].iter().position(|node| !node.kind.is_fight()) {
        return Err(format!("the first row's node {c} is a {:?}", map[0][c].kind));
    }
    Ok(())
}

/// No two edges between the same two rows cross, and none touching no boss spans more than a column:
/// a boss's are the fans to and from its lone node.
fn check_edges(map: &[Vec<RunNode>]) -> Result<(), String> {
    for r in 0..map.len().saturating_sub(1) {
        let edges: Vec<(i64, i64)> = map[r]
            .iter()
            .flat_map(|node| {
                node.next
                    .iter()
                    .map(|j| (node.column as i64, map[r + 1][*j].column as i64))
            })
            .collect();
        for (i, (a, b)) in edges.iter().enumerate() {
            if !boss_row(r) && !boss_row(r + 1) && (a - b).abs() > 1 {
                return Err(format!(
                    "rows {r}-{}: edge {a}->{b} spans more than a column",
                    r + 1
                ));
            }
            if let Some((c, d)) = edges[..i].iter().find(|(c, d)| (a - c) * (b - d) < 0) {
                return Err(format!("rows {r}-{}: edges {a}->{b} and {c}->{d} cross", r + 1));
            }
        }
    }
    Ok(())
}

/// Every node past the first row is led to from the row below, and every node but the final boss
/// leads on: with `check_shape`'s ranges, every node is reached from a start and reaches the final boss.
fn check_reach(map: &[Vec<RunNode>]) -> Result<(), String> {
    for (r, row) in map.iter().enumerate() {
        for (c, node) in row.iter().enumerate() {
            if r > 0 && !map[r - 1].iter().any(|below| below.next.contains(&c)) {
                return Err(format!("({r}, {c}) is an orphan: nothing leads to it"));
            }
            if r + 1 < map.len() && node.next.is_empty() {
                return Err(format!("({r}, {c}) is a dead end: it leads nowhere"));
            }
        }
    }
    Ok(())
}

/// The three checks, then every route's bounds: `RUN_ROUTE_FIGHTS_MIN` to `RUN_ROUTE_FIGHTS_MAX`
/// fights and `RUN_ROUTE_TILES_MIN` tiles.
fn check_map(map: &[Vec<RunNode>]) -> Result<Bounds, String> {
    check_shape(map)?;
    check_edges(map)?;
    check_reach(map)?;
    let bounds = route_bounds(map);
    if bounds.fights_min < RUN_ROUTE_FIGHTS_MIN || bounds.fights_max > RUN_ROUTE_FIGHTS_MAX {
        return Err(format!(
            "a route holds {} to {} fights",
            bounds.fights_min, bounds.fights_max
        ));
    }
    if bounds.tiles_min < RUN_ROUTE_TILES_MIN {
        return Err(format!("a route holds only {} tiles", bounds.tiles_min));
    }
    Ok(bounds)
}

/// A hand-built node: its kind, its column and the next row's nodes it leads to.
fn spot(kind: RunNodeKind, column: usize, next: &[usize]) -> RunNode {
    RunNode {
        kind,
        enemy: None,
        event: None,
        artifact: None,
        column,
        next: next.to_vec(),
    }
}

#[test]
fn r3583000_r3583001_every_seeds_map_meets_at_the_bosses_and_every_route_holds_its_bounds() {
    let mut spread = false;
    let mut mixed = false;
    for (tag, content) in [
        ("placeholder", run_placeholder_content()),
        ("catalog", catalog_content()),
    ] {
        for n in 0..MAP_SEEDS {
            let map = deal(&content, n).map;
            let bounds = check_map(&map).unwrap_or_else(|e| panic!("{tag} seed {n}: {e}"));
            spread |= bounds.fights_min < bounds.fights_max;
            mixed |= map.iter().enumerate().any(|(r, row)| {
                r > 0
                    && !boss_row(r)
                    && row.iter().any(|node| node.kind.is_fight())
                    && row.iter().any(|node| !node.kind.is_fight())
            });
        }
    }
    assert!(spread, "no map offers a fast way and a slow one");
    assert!(mixed, "no row mixes fights and tiles");
}

#[test]
fn map_one_seed_deals_one_map_edges_included() {
    let content = run_placeholder_content();
    let map = deal(&content, 0).map;
    assert_eq!(map, deal(&content, 0).map);
    assert_ne!(map, deal(&content, 1).map);
}

#[test]
fn map_route_bounds_take_the_fewest_and_most_over_every_route() {
    let map = vec![
        vec![spot(RunNodeKind::Fight, 3, &[0, 1])],
        vec![
            spot(RunNodeKind::Shop, 2, &[0]),
            spot(RunNodeKind::Elite, 3, &[0]),
        ],
        vec![spot(RunNodeKind::Boss, 3, &[])],
    ];
    assert_eq!(
        route_bounds(&map),
        Bounds {
            fights_min: 2,
            fights_max: 3,
            tiles_min: 0,
        }
    );
}

#[test]
fn map_a_crossing_edge_fails() {
    let map = vec![
        vec![
            spot(RunNodeKind::Fight, 2, &[1]),
            spot(RunNodeKind::Fight, 3, &[0]),
        ],
        vec![spot(RunNodeKind::Shop, 2, &[]), spot(RunNodeKind::Shop, 3, &[])],
    ];
    let err = check_edges(&map).unwrap_err();
    assert!(err.contains("cross"), "{err}");
}

#[test]
fn map_an_edge_two_columns_wide_fails() {
    let map = vec![
        vec![spot(RunNodeKind::Fight, 1, &[0])],
        vec![spot(RunNodeKind::Shop, 3, &[])],
    ];
    let err = check_edges(&map).unwrap_err();
    assert!(err.contains("column"), "{err}");
}

#[test]
fn map_a_dead_end_fails() {
    let map = vec![
        vec![spot(RunNodeKind::Fight, 2, &[0, 1])],
        vec![spot(RunNodeKind::Shop, 1, &[]), spot(RunNodeKind::Shop, 2, &[0])],
        vec![spot(RunNodeKind::Boss, 3, &[])],
    ];
    let err = check_reach(&map).unwrap_err();
    assert!(err.contains("dead end"), "{err}");
}

#[test]
fn map_an_orphan_fails() {
    let map = vec![
        vec![spot(RunNodeKind::Fight, 2, &[0])],
        vec![spot(RunNodeKind::Shop, 2, &[0]), spot(RunNodeKind::Shop, 3, &[0])],
        vec![spot(RunNodeKind::Boss, 3, &[])],
    ];
    let err = check_reach(&map).unwrap_err();
    assert!(err.contains("orphan"), "{err}");
}

#[test]
fn map_too_many_or_too_few_fights_fail() {
    let dealt = deal(&run_placeholder_content(), 0).map;
    assert!(check_map(&dealt).is_ok());
    let with = |kind: RunNodeKind, from: usize, of: fn(RunNodeKind) -> bool| {
        let mut map = dealt.clone();
        for node in map.iter_mut().skip(from).flatten() {
            if of(node.kind) {
                node.kind = kind;
            }
        }
        map
    };
    // Every tile a fight: every route holds every row's fight.
    let err = check_map(&with(RunNodeKind::Fight, 0, |kind| !kind.is_fight())).unwrap_err();
    assert!(err.contains("fights"), "{err}");
    // Every node past the first row but the bosses a shop: a route holds the first row's and the bosses'.
    let err = check_map(&with(RunNodeKind::Shop, 1, |kind| !kind.is_boss())).unwrap_err();
    assert!(err.contains("fights"), "{err}");
}
