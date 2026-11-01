//! SPEC §7 Stinkyscape T-anomaly-found Anomaly Found! (§8.10 row T-anomaly-found; §3.2, §6.3; R452,
//! R1081000, R1839000, R1842000, R1855000–R1855002, R1879000–R1879002, R2204002). (4) Spell, a token
//! (printed Mythic) of the Stinkyscape set, found by S #166 and S #167's scan and S #7's reward.
//!   Base:    "Replace your side of the field with a random Anomaly: exile every card on it, then lay
//!            that Anomaly's board in its place. Every Cry on the new board fires, with random targets
//!            and choices."
//!   Radiant: the same with "a random rich Anomaly".
//!
//! Board swap via replace_board (#1855) on caster's side: exiles board except Dormant (R1839000); lays zones and fires Cries with random choices (R1855000, R1855001).
//! Rolls BASE_BOARDS / RICH_BOARDS (R43, R151, R1879000, ST-S20-2, ST-S20-3, R2247000).
//! Asteroid resourcePool (R1855002, #169); Flying backrow drones (R1879001); Drone Battleship stack (R1879002, R1842000, #151).

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

/// R2204002: `boards` but those a held game's pools leave out (R2201001), a board that lays a
/// Legendary or Mythic card (MT's Sanctum); every board when the game holds nothing.
fn unheld(boards: &[Board]) -> Vec<Board> {
    boards
        .iter()
        .filter(|board| !board.cards.iter().any(|lay| held_out(def_of(None, lay.def_id))))
        .copied()
        .collect()
}

/// R1879000: one of `boards`, drawn from the match rng as the Spell resolves, each equally likely
/// (R2204002: of those a held game leaves in).
pub fn replace_with_random(rng: &mut Rng, boards: &[Board]) -> Vec<Effect> {
    let boards = unheld(boards);
    let Ok(count) = i32::try_from(boards.len()) else {
        return vec![];
    };
    let Some(board) = usize::try_from(rng.int(count)).ok().and_then(|at| boards.get(at)) else {
        return vec![];
    };
    vec![replace_board(preset(board))]
}

/// R2247000: the board this card holds, by the code its label remembers, any of the ten: a card made
/// Radiant after it drew keeps the board it shows.
pub fn held_board(card: Option<&CardInstance>) -> Option<&'static Board> {
    let code = card?.memory.get(CARD_LABEL_KEY)?.as_str()?;
    ALL_BOARDS.iter().find(|board| board.code == code)
}

/// R2247000: as the card arrives in a hand, a library or on the field, it draws one of `boards`, each
/// equally likely (R2204002: of those a held game leaves in), and remembers its code as its label; a
/// card that holds a board already draws nothing.
fn draw_board(ctx: &EffectContext<'_>, boards: &[Board]) -> Vec<Effect> {
    if held_board(ctx.self_.as_ref()).is_some() {
        return vec![];
    }
    vec![remember_random(RememberRandomArgs {
        key: CARD_LABEL_KEY.to_string(),
        options: unheld(boards).iter().map(|board| json!(board.code)).collect(),
    })]
}

/// R2247000: the board the card holds, else (R1879000) one of `boards` drawn as it resolves.
fn lay(ctx: &mut EffectContext<'_>, boards: &[Board]) -> Vec<Effect> {
    match held_board(ctx.self_.as_ref()) {
        Some(board) => vec![replace_board(preset(board))],
        None => replace_with_random(&mut *ctx.rng, boards),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        start_of_game: Some(hook(|ctx| draw_board(ctx, &BASE_BOARDS))),
        cry: Some(hook(|ctx| lay(ctx, &BASE_BOARDS))),
        ..Script::default()
    };
    let radiant = Script {
        start_of_game: Some(hook(|ctx| draw_board(ctx, &RICH_BOARDS))),
        cry: Some(hook(|ctx| lay(ctx, &RICH_BOARDS))),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// Stinkyscape T-anomaly-found Anomaly Found! — SPEC §8.10 row T-anomaly-found, BUILD M12 row S
// (R1855000, R1855001, R1879000, R1879002).
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
    const SEEDS: u32 = 120;

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
                summons.len() >= cards.len() && summons[..cards.len()] == cards[..]
            })
            .collect();
        assert_eq!(found.len(), 1, "one board was laid: {summons:?}");
        found[0].code
    }

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

    mod r2247000_the_board_it_holds {
        use super::*;

        const PROBE: &str = "stinkyscape-166";

        fn scanned(seed: &str, misses: i32) -> (Scenario, Option<CardInstance>) {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": seed,
                "p1": {
                    "hand": [PROBE],
                    "mana": 10,
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "library": [FILLER, FILLER, FILLER],
                },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
            }));
            s.state_mut().players[P1].anomaly_misses = Some(misses);
            s.play(PROBE, json!({}));
            let found = s.hand(P1).into_iter().find(|card| card.def_id == ID);
            (s, found)
        }

        fn code_of(card: &CardInstance) -> Option<&str> {
            card.memory.get(CARD_LABEL_KEY).and_then(Value::as_str)
        }

        fn labels(s: &Scenario, id: &str) -> (Option<String>, bool) {
            let mine = match &s.view(P1).you.hand {
                HandView::Cards(cards) => cards
                    .iter()
                    .find(|card| card.instance_id == id)
                    .and_then(|card| card.label.clone()),
                HandView::Count { .. } => None,
            };
            let theirs = matches!(&s.view(P2).opponent.hand, HandView::Cards(_));
            (mine, theirs)
        }

        #[test]
        fn r2247000_a_found_radiant_one_holds_a_rich_board_its_owner_reads_and_play_lays_it() {
            let rich: Vec<&str> = RICH_BOARDS.iter().map(|board| board.code).collect();
            let mut seen: IndexSet<String> = IndexSet::new();
            for seed in 0..40 {
                // R1855003: at 95 misses every scan finds a Radiant one.
                let (mut s, found) = scanned(&format!("r2247000-rich-{seed}"), 95);
                let card = found.expect("a find");
                assert!(card.radiant);
                let code = code_of(&card).expect("it holds a board").to_string();
                assert!(rich.contains(&code.as_str()), "{code} is a rich board");
                let (mine, theirs_readable) = labels(&s, &card.id);
                assert_eq!(mine.as_deref(), Some(code.as_str()), "its owner reads the code");
                assert!(!theirs_readable, "the other player reads a count");
                s.play(card.id.as_str(), json!({}));
                assert_eq!(laid(&s, &ALL_BOARDS), code, "it lays the board it showed");
                seen.insert(code);
            }
            assert_eq!(seen.len(), rich.len(), "each rich board is found over the seeds");
        }

        #[test]
        fn r2247000_a_found_plain_one_holds_one_of_the_seven_and_play_lays_it() {
            let base: Vec<&str> = BASE_BOARDS.iter().map(|board| board.code).collect();
            let mut checked = 0;
            for seed in 0..80 {
                let (mut s, found) = scanned(&format!("r2247000-base-{seed}"), 16);
                let card = found.expect("a find");
                if card.radiant {
                    continue;
                }
                let code = code_of(&card).expect("it holds a board").to_string();
                assert!(base.contains(&code.as_str()), "{code} is one of the seven");
                s.play(card.id.as_str(), json!({}));
                assert_eq!(laid(&s, &ALL_BOARDS), code);
                checked += 1;
            }
            assert!(checked > 20, "{checked} plain finds checked");
        }

        #[test]
        fn r2247000_a_card_that_holds_a_board_lays_it_on_either_face_and_draws_no_other() {
            for (code, radiant) in [("FO", false), ("WH", true), ("AA", false)] {
                let mut s = setup(&format!("r2247000-held-{code}"), radiant, json!([]));
                let id = s.card(ID).id.clone();
                s.card_mut(&id).memory.insert(CARD_LABEL_KEY.to_string(), json!(code));
                s.play(id.as_str(), json!({}));
                assert_eq!(laid(&s, &ALL_BOARDS), code);
            }
        }

        #[test]
        fn r2247000_r1879000_one_that_holds_no_board_still_draws_as_it_resolves() {
            let mut s = setup("r2247000-none", false, json!([]));
            assert!(code_of(s.card(ID)).is_none());
            s.play(ID, json!({}));
            let code = laid(&s, &ALL_BOARDS);
            assert!(BASE_BOARDS.iter().any(|board| board.code == code));
        }
    }

    /// R2204002: a held game (R2201001) never draws MT, the one board that lays a Legendary card (S #152
    /// Sanctum), neither as the card resolves nor as it arrives; the same seeds unheld do.
    mod r2204002_a_held_game_draws_no_board_that_lays_a_held_card {
        use super::*;

        const PROBE: &str = "stinkyscape-166";

        fn hold(s: &mut Scenario) {
            s.state_mut().players[P1].handicap = Some(Handicap {
                pool_hold: Some(true),
                ..TUTORIAL_HUMAN
            });
        }

        fn resolved(seed: &str, held: bool) -> &'static str {
            let mut s = setup(seed, false, json!([]));
            if held {
                hold(&mut s);
            }
            s.play(ID, json!({}));
            laid(&s, &ALL_BOARDS)
        }

        fn found(seed: &str, held: bool) -> Option<String> {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": seed,
                "p1": { "hand": [PROBE], "mana": 10, "library": [FILLER, FILLER, FILLER] },
                "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
            }));
            if held {
                hold(&mut s);
            }
            s.state_mut().players[P1].anomaly_misses = Some(16);
            s.play(PROBE, json!({}));
            let card = s.hand(P1).into_iter().find(|card| card.def_id == ID)?;
            card.memory.get(CARD_LABEL_KEY).and_then(Value::as_str).map(str::to_string)
        }

        #[test]
        fn r2204002_r2201001_as_it_resolves() {
            let seeds: Vec<String> = (0..SEEDS).map(|n| format!("r2204002-resolve-{n}")).collect();
            for seed in &seeds {
                assert_ne!(resolved(seed, true), MT.code, "{seed}");
            }
            assert!(seeds.iter().any(|seed| resolved(seed, false) == MT.code), "unheld, MT is drawn");
        }

        #[test]
        fn r2204002_r2247000_as_it_arrives() {
            let seeds: Vec<String> = (0..SEEDS).map(|n| format!("r2204002-arrive-{n}")).collect();
            for seed in &seeds {
                assert_ne!(found(seed, true).as_deref(), Some(MT.code), "{seed}");
            }
            assert!(
                seeds.iter().any(|seed| found(seed, false).as_deref() == Some(MT.code)),
                "unheld, a find holds MT"
            );
        }
    }
}
