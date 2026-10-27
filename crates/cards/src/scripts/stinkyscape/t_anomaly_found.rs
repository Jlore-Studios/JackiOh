//! SPEC §7 Stinkyscape T-anomaly-found Anomaly Found! (§8.10 row T-anomaly-found; §3.2, §6.3; R452,
//! R1081000, R1839000, R1842000, R1855000–R1855002, R1879000–R1879002). (4) Spell, a token (printed
//! Mythic) of the held Stinkyscape set (R1420), found by S #166 and S #167's scan and S #7's reward.
//!   Base:    "Replace your side of the field with a random Anomaly: exile every card on it, then lay
//!            that Anomaly's board in its place. Every Cry on the new board fires, with random targets
//!            and choices."
//!   Radiant: the same with "a random rich Anomaly".
//!
//! Engine: #1855's board swap, `replace_board`, on its caster's side only (ST9 Q7): what the Spell
//! reaches there is exiled, a Dormant card staying (R1839000), the board is laid zone by zone, a
//! summon each, and every Unit it laid fires its Cry in list order with its caster's choices random
//! (R1855000, R1855001). The board is drawn from the match rng as the Spell resolves, each equally
//! likely: on the base face one of the seven `BASE_BOARDS`, on the Radiant face one of the three rich
//! `RICH_BOARDS` (R1879000, ST-S20-2). The ten boards are the designer's, "Small Asteroid" read as
//! S #169 Tiny Asteroid (ST-S20-3), an asteroid field's "only gives …" a `resourcePool` rider on each
//! of its asteroids (R1855002), SP's backrow Drone Interceptors standing there by their Flying
//! (R1879001), and BF's Drone Battleship its three layers in one pile, Meltdown at the bottom and the
//! Shield on top (R1879002, R1842000). S #151 Call to Chaos (Scape Edition) lays one of `ALL_BOARDS`.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-anomaly-found";

const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
const CORVETTE: &str = "stinkyscape-t-drone-corvette";
const SHIELD_LAYER: &str = "stinkyscape-t-shield-layer";
const PROPAGATION_LAYER: &str = "stinkyscape-t-propagation-layer";
const MELTDOWN_LAYER: &str = "stinkyscape-t-meltdown-layer";
const WORMHOLE: &str = "stinkyscape-139";
const THE_GAUNTLET: &str = "stinkyscape-157";
const DRONE_HANGER: &str = "stinkyscape-168";
const SPICE_TOWER: &str = "stinkyscape-173";
const SANCTUM: &str = "stinkyscape-152";
const HALO: &str = "stinkyscape-083";
const PHALANX: &str = "stinkyscape-082";
const THE_SYNDICATE: &str = "stinkyscape-164";
/// ST-S20-3: the designer's "Small Asteroid".
const TINY_ASTEROID: &str = "stinkyscape-169";
const MEDIUM_ASTEROID: &str = "stinkyscape-170";
const LARGE_ASTEROID: &str = "stinkyscape-171";
const HUGE_ASTEROID: &str = "stinkyscape-172";
const NARCOR: &str = "stinkyscape-t-narcor";
const RED_NARCOR: &str = "stinkyscape-t-red-narcor";
const AXNIT: &str = "stinkyscape-t-axnit";

/// One card of a board, as the designer lists it.
#[derive(Clone, Copy, Debug)]
pub struct Lay {
    pub def_id: &'static str,
    pub row: Row,
    pub lane: i32,
    pub radiant: bool,
    /// R1879002: laid on top of the card the entry before it laid in its zone.
    pub stack: bool,
}

/// One of the ten Anomalies: the designer's code and name, its cards in list order, and the Resources
/// its asteroids give in place of every Resource (R1855002), none for an unrestricted board.
#[derive(Clone, Copy, Debug)]
pub struct Board {
    pub code: &'static str,
    pub name: &'static str,
    pub cards: &'static [Lay],
    pub resource_pool: &'static [&'static str],
}

const fn unit(def_id: &'static str, lane: i32, radiant: bool) -> Lay {
    Lay {
        def_id,
        row: Row::Units,
        lane,
        radiant,
        stack: false,
    }
}

const fn back(def_id: &'static str, lane: i32, radiant: bool) -> Lay {
    Lay {
        def_id,
        row: Row::Backrow,
        lane,
        radiant,
        stack: false,
    }
}

const fn on_top(def_id: &'static str, lane: i32) -> Lay {
    Lay {
        def_id,
        row: Row::Units,
        lane,
        radiant: false,
        stack: true,
    }
}

pub const WH: Board = Board {
    code: "WH",
    name: "Wormhole",
    cards: &[
        unit(INTERCEPTOR, 1, true),
        unit(INTERCEPTOR, 2, false),
        unit(CORVETTE, 4, false),
        back(WORMHOLE, 1, false),
        back(WORMHOLE, 2, true),
        back(THE_GAUNTLET, 3, false),
        back(WORMHOLE, 4, false),
        back(WORMHOLE, 5, false),
    ],
    resource_pool: &[],
};

pub const BF: Board = Board {
    code: "BF",
    name: "Battlefields",
    cards: &[
        unit(MELTDOWN_LAYER, 2, false),
        on_top(PROPAGATION_LAYER, 2),
        on_top(SHIELD_LAYER, 2),
        unit(INTERCEPTOR, 5, true),
        back(DRONE_HANGER, 3, false),
    ],
    resource_pool: &[],
};

pub const AL: Board = Board {
    code: "AL",
    name: "Asteroid Field: Large",
    cards: &[unit(LARGE_ASTEROID, 3, true), unit(HUGE_ASTEROID, 4, false)],
    resource_pool: &[],
};

pub const AT: Board = Board {
    code: "AT",
    name: "Asteroid Field: Small",
    cards: &[
        unit(TINY_ASTEROID, 1, true),
        unit(MEDIUM_ASTEROID, 2, true),
        unit(TINY_ASTEROID, 3, false),
        unit(TINY_ASTEROID, 4, false),
        unit(TINY_ASTEROID, 5, true),
    ],
    resource_pool: &[],
};

pub const SP: Board = Board {
    code: "SP",
    name: "Wrecked Spice Platform",
    cards: &[
        unit(SPICE_TOWER, 1, false),
        unit(SPICE_TOWER, 2, false),
        unit(SPICE_TOWER, 4, true),
        back(INTERCEPTOR, 1, false),
        back(INTERCEPTOR, 3, false),
        back(INTERCEPTOR, 4, true),
        back(DRONE_HANGER, 5, false),
    ],
    resource_pool: &[],
};

pub const MT: Board = Board {
    code: "MT",
    name: "Monument",
    cards: &[
        unit(TINY_ASTEROID, 1, false),
        unit(TINY_ASTEROID, 2, false),
        unit(TINY_ASTEROID, 3, false),
        unit(TINY_ASTEROID, 4, false),
        unit(TINY_ASTEROID, 5, false),
        back(SANCTUM, 3, true),
    ],
    resource_pool: &[],
};

pub const DH: Board = Board {
    code: "DH",
    name: "Drone Hideouts",
    cards: &[
        unit(CORVETTE, 2, false),
        unit(CORVETTE, 4, false),
        back(DRONE_HANGER, 1, false),
        back(DRONE_HANGER, 3, true),
        back(DRONE_HANGER, 5, false),
    ],
    resource_pool: &[],
};

pub const FO: Board = Board {
    code: "FO",
    name: "Frontier Outpost",
    cards: &[unit(HALO, 2, true), unit(PHALANX, 4, true), back(THE_SYNDICATE, 3, false)],
    resource_pool: &[],
};

pub const AN: Board = Board {
    code: "AN",
    name: "Asteroid Field: Narcour",
    cards: &[
        unit(TINY_ASTEROID, 1, false),
        unit(LARGE_ASTEROID, 2, true),
        unit(TINY_ASTEROID, 3, true),
        unit(LARGE_ASTEROID, 5, false),
    ],
    resource_pool: &[NARCOR, RED_NARCOR],
};

pub const AA: Board = Board {
    code: "AA",
    name: "Asteroid Field: Axnit",
    cards: &[
        unit(MEDIUM_ASTEROID, 1, true),
        unit(LARGE_ASTEROID, 2, false),
        unit(LARGE_ASTEROID, 3, true),
        unit(MEDIUM_ASTEROID, 4, true),
        unit(TINY_ASTEROID, 5, true),
    ],
    resource_pool: &[AXNIT],
};

/// ST-S20-2: the base face's seven, the list's boards less the Radiant three.
pub const BASE_BOARDS: [Board; 7] = [WH, BF, AL, AT, SP, MT, DH];
/// The Radiant face's three rich boards.
pub const RICH_BOARDS: [Board; 3] = [AN, AA, FO];
/// All ten, S #151's "any Anomaly" (ST-S25-7).
pub const ALL_BOARDS: [Board; 10] = [WH, BF, AL, AT, SP, MT, DH, FO, AN, AA];

/// `board` as `replace_board`'s preset (R1855000), its pool a rider on every card it lays (R1855002).
pub fn preset(board: &Board) -> ReplaceBoardArgs {
    let memory = (!board.resource_pool.is_empty()).then(|| {
        IndexMap::from([(
            RESOURCE_POOL_KEY.to_string(),
            json!(board.resource_pool),
        )])
    });
    ReplaceBoardArgs {
        board: board
            .cards
            .iter()
            .map(|lay| PresetCard {
                def_id: lay.def_id.to_string(),
                row: lay.row,
                lane: lay.lane,
                radiant: lay.radiant.then_some(true),
                position: None,
                memory: memory.clone(),
                stack: lay.stack.then_some(true),
            })
            .collect(),
    }
}

/// R1879000: one of `boards`, drawn from the match rng as the Spell resolves, each equally likely.
pub fn replace_with_random(rng: &mut Rng, boards: &[Board]) -> Vec<Effect> {
    let Ok(count) = i32::try_from(boards.len()) else {
        return vec![];
    };
    let Some(board) = usize::try_from(rng.int(count)).ok().and_then(|at| boards.get(at)) else {
        return vec![];
    };
    vec![replace_board(preset(board))]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| replace_with_random(&mut *ctx.rng, &BASE_BOARDS))),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| replace_with_random(&mut *ctx.rng, &RICH_BOARDS))),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// Stinkyscape T-anomaly-found Anomaly Found! — SPEC §8.10 row T-anomaly-found, BUILD M12 row S
// T-anomaly-found: "its caster's side is exiled (a Dormant card stays) and one of the seven boards
// laid, the other side untouched; each laid Unit's Cry fires once the board is laid, its choices
// random; over the seeds each of the seven appears; radiant one of the three rich boards"
// (R1855000, R1855001, R1879000–R1879002).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const FILLER: &str = "core-005";
    /// The games the board is drawn over.
    const SEEDS: u32 = 120;

    /// P1's main phase with an Anomaly Found! on `radiant`'s face in hand, a Unit in P1's lane 1 and
    /// one in P2's, and `field` beside them.
    fn setup(seed: &str, radiant: bool, field: Value) -> Scenario {
        crate::register_all();
        let mut mine = vec![json!({ "def": VANILLA, "lane": 1 })];
        mine.extend(field.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": mine,
                "library": [FILLER, FILLER, FILLER],
            },
            "p2": { "field": [VANILLA], "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    /// The `summoned` events of the last step, as (definition, row, lane).
    fn summoned(s: &Scenario) -> Vec<(String, Row, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    def_id, row, lane, ..
                } => Some((def_id.clone(), *row, *lane)),
                _ => None,
            })
            .collect()
    }

    /// The board a play laid: the one of `boards` whose cards are the summons, in list order.
    fn laid(s: &Scenario, boards: &[Board]) -> &'static str {
        let summons = summoned(s);
        let found: Vec<&Board> = boards
            .iter()
            .filter(|board| {
                let cards: Vec<(String, Row, i32)> = board
                    .cards
                    .iter()
                    .map(|lay| (lay.def_id.to_string(), lay.row, lay.lane))
                    .collect();
                // A laid Cry may summon more after the board; the board's own cards come first.
                summons.len() >= cards.len() && summons[..cards.len()] == cards[..]
            })
            .collect();
        assert_eq!(found.len(), 1, "one board was laid: {summons:?}");
        found[0].code
    }

    /// A seed on `radiant`'s face that lays `code`.
    fn seed_laying(code: &str, radiant: bool, boards: &[Board]) -> String {
        (0..SEEDS)
            .map(|seed| format!("anomaly-{code}-{seed}"))
            .find(|seed| {
                let mut s = setup(seed, radiant, json!([]));
                s.play(ID, json!({}));
                laid(&s, boards) == code
            })
            .expect("a seed laying the board")
    }

    /// The card on top of P1's zone, by row.
    fn top(s: &Scenario, row: Row, lane: i32) -> Option<CardInstance> {
        match row {
            Row::Units => s.unit(P1, lane),
            Row::Backrow => s.backrow(P1, lane),
        }
    }

    #[test]
    fn r1879000_the_ten_boards_are_the_designers() {
        crate::register_all();
        assert_eq!(ALL_BOARDS.len(), BASE_BOARDS.len() + RICH_BOARDS.len());
        for board in ALL_BOARDS {
            for lay in board.cards {
                let def = def_of(None, lay.def_id);
                assert_eq!(def.set, SetName::Stinkyscape, "{} {}", board.code, lay.def_id);
                let face = if lay.radiant { &def.radiant } else { &def.base };
                let flies = face.keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Flying);
                let fits = match def.type_ {
                    CardType::Unit => lay.row == Row::Units || flies,
                    CardType::Spell => false,
                    _ => lay.row == Row::Backrow,
                };
                assert!(fits, "{} {} fits its row", board.code, lay.def_id);
                assert!((1..=5).contains(&lay.lane));
            }
            for pool in board.resource_pool {
                assert!(def_of(None, pool).resource.is_some(), "{pool} is a Resource");
            }
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r1879000_over_the_seeds_each_of_the_seven_appears_and_no_rich_one() {
            let mut seen: IndexSet<&str> = IndexSet::new();
            for seed in 0..SEEDS {
                let mut s = setup(&format!("anomaly-base-{seed}"), false, json!([]));
                s.play(ID, json!({}));
                seen.insert(laid(&s, &ALL_BOARDS));
            }
            let base: IndexSet<&str> = BASE_BOARDS.iter().map(|board| board.code).collect();
            assert_eq!(seen.len(), base.len(), "{seen:?}");
            assert!(seen.iter().all(|code| base.contains(code)), "{seen:?}");
        }

        #[test]
        fn r1855000_its_casters_side_is_exiled_and_the_board_laid_the_other_untouched() {
            let seed = seed_laying("WH", false, &BASE_BOARDS);
            let mut s = setup(&seed, false, json!([]));
            let old = s.unit(P1, 1).expect("P1's unit").id;
            let theirs = s.unit(P2, 1).expect("P2's unit").id;
            s.play(ID, json!({}));

            assert_eq!(s.card(old.as_str()).zone.z(), ZoneName::Exile, "the old Unit is exiled");
            assert_eq!(s.unit(P2, 1).expect("P2's unit").id, theirs, "the other side is untouched");
            for lay in WH.cards {
                let card = top(&s, lay.row, lay.lane).expect("the board's card");
                assert_eq!(card.def_id, lay.def_id, "{:?} {}", lay.row, lay.lane);
                assert_eq!(card.radiant, lay.radiant, "{:?} {}", lay.row, lay.lane);
            }
            assert!(s.unit(P1, 3).is_none() && s.unit(P1, 5).is_none(), "an unnamed zone is empty");
            let wormhole = s.backrow(P1, 1).expect("a Wormhole");
            assert_ne!(wormhole.face_up, Some(true), "a laid Trap is face-down");
            assert_eq!(s.backrow(P1, 3).expect("The Gauntlet").face_up, Some(true));
        }

        #[test]
        fn r1855000_a_dormant_card_stays_and_its_zone_takes_nothing() {
            let seed = seed_laying("WH", false, &BASE_BOARDS);
            let mut s = setup(
                &seed,
                false,
                json!([{ "def": VANILLA, "lane": 2, "slumber": { "turns": 2 } }]),
            );
            let dormant = s.unit(P1, 2).expect("the Dormant unit").id;
            s.play(ID, json!({}));
            assert_eq!(s.unit(P1, 2).expect("its zone").id, dormant, "it stays");
            assert!(
                !summoned(&s).contains(&(INTERCEPTOR.to_string(), Row::Units, 2)),
                "WH's Drone Interceptor for its zone is skipped"
            );
            assert_eq!(s.unit(P1, 4).expect("WH's Drone Corvette").def_id, CORVETTE);
        }

        #[test]
        fn r1879001_sp_lays_its_drone_interceptors_standing_in_the_backrow() {
            let seed = seed_laying("SP", false, &BASE_BOARDS);
            let mut s = setup(&seed, false, json!([]));
            s.play(ID, json!({}));
            for lane in [1, 3, 4] {
                let card = s.backrow(P1, lane).expect("a Drone Interceptor");
                assert_eq!(card.def_id, INTERCEPTOR, "backrow {lane}");
                assert_eq!(card.radiant, lane == 4);
            }
            assert_eq!(s.backrow(P1, 5).expect("the Drone Hanger").def_id, DRONE_HANGER);
            assert!(s.unit(P1, 3).is_none() && s.backrow(P1, 2).is_none());
        }

        #[test]
        fn r1879002_bf_lays_the_drone_battleship_as_one_pile_shield_on_top() {
            let seed = seed_laying("BF", false, &BASE_BOARDS);
            let mut s = setup(&seed, false, json!([]));
            s.play(ID, json!({}));
            let pile: Vec<String> = pile_at(
                s.state(),
                ZoneSlot {
                    player: P1,
                    row: Row::Units,
                    lane: 2,
                },
            )
            .expect("the Drone Battleship")
            .iter()
            .map(|card| card.def_id.clone())
            .collect();
            assert_eq!(pile, vec![SHIELD_LAYER, PROPAGATION_LAYER, MELTDOWN_LAYER], "top first");
            assert!(s.unit(P1, 5).expect("the Drone Interceptor").radiant);
        }

        #[test]
        fn r1855001_no_laid_cry_ever_asks_its_caster() {
            // Every board's laid Cries (FO's Halo and Phalanx Discover) draw their caster's choices at
            // random (R452's mode): no play leaves a question for P1.
            for seed in 0..SEEDS {
                for radiant in [false, true] {
                    let mut s = setup(&format!("anomaly-cry-{radiant}-{seed}"), radiant, json!([]));
                    s.play(ID, json!({}));
                    assert!(
                        s.state().pending.as_ref().is_none_or(|choice| choice.player_id != P1),
                        "its caster is never asked"
                    );
                }
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1879000_over_the_seeds_each_of_the_three_rich_boards_appears_and_no_other() {
            let mut seen: IndexSet<&str> = IndexSet::new();
            for seed in 0..SEEDS {
                let mut s = setup(&format!("anomaly-rich-{seed}"), true, json!([]));
                s.play(ID, json!({}));
                seen.insert(laid(&s, &ALL_BOARDS));
            }
            let rich: IndexSet<&str> = RICH_BOARDS.iter().map(|board| board.code).collect();
            assert_eq!(seen, rich, "{seen:?}");
        }

        #[test]
        fn r1855001_fo_s_halo_and_phalanx_arm_themselves_with_random_picks() {
            // Their Cries (S15, R1862002) Discover Weapons and Equip them; laid, every pick is drawn
            // at random, so each ends armed with no question left for its caster.
            let seed = seed_laying("FO", true, &RICH_BOARDS);
            let mut s = setup(&seed, true, json!([]));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none(), "no question is left");
            for (lane, def_id, picks) in [(2, HALO, 3), (4, PHALANX, 2)] {
                let card = s.unit(P1, lane).expect("the laid Unit");
                // R77: each Equip fuses a Weapon into the host, whose definition names every one.
                assert!(card.def_id.ends_with(&format!("+{def_id}{}", ")".repeat(picks - 1))), "{}", card.def_id);
                assert_eq!(card.def_id.matches('+').count(), picks, "{def_id} equipped {picks}: {}", card.def_id);
                assert!(card.radiant);
                let printed = crate::card_def(def_id).radiant.health.unwrap_or_default();
                assert!(
                    s.stats(card.id.as_str()).health > printed,
                    "{def_id} took its Weapons' health"
                );
                assert!(
                    !activate_actions_for(s.state(), P1, &card).is_empty(),
                    "{def_id} took its Weapons' Activates"
                );
            }
            assert_eq!(s.backrow(P1, 3).expect("The Syndicate").def_id, THE_SYNDICATE);
        }

        #[test]
        fn r1855002_an_asteroid_field_lays_its_pool_on_every_asteroid() {
            for (board, pool) in [(AN, vec![NARCOR, RED_NARCOR]), (AA, vec![AXNIT])] {
                let seed = seed_laying(board.code, true, &RICH_BOARDS);
                let mut s = setup(&seed, true, json!([]));
                s.play(ID, json!({}));
                for lay in board.cards {
                    let card = s.unit(P1, lay.lane).expect("an asteroid");
                    assert_eq!(card.def_id, lay.def_id);
                    assert_eq!(card.radiant, lay.radiant);
                    assert_eq!(
                        card.memory.get(RESOURCE_POOL_KEY),
                        Some(&json!(pool)),
                        "{} lane {}",
                        board.code,
                        lay.lane
                    );
                }
            }
        }
    }
}
