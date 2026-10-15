//! M #98 Showdown (SPEC §8.8 row 98): (1) Spell, Rare — the designer's first #28.
//!
//! Base:    "Choose an empty lane. Lock every other lane until the start of your next turn."
//! Radiant: the same, plus " This turn, after you put a card into that lane, cast Book of Buff on
//!           it."
//! Engine:
//! - **The lane** is a play-time choice of five modes (`lane-1` … `lane-5`, R81), random under a
//!   random cast. Only an empty lane is offered, all four of its zones empty (`mode_allowed`,
//!   `lane_is_empty`, R831008): with none the card is not offered and its play is refused, and a
//!   cast picks among the empty lanes, resolving into nothing when there is none.
//! - **The Cry** Locks every zone of the four other lanes, both sides and both rows, that is not
//!   already Locked (Lock, §6.3; R688: plays refused, summons still landing), then arms a delayed
//!   effect at the start of the caster's next turn (R62's delayed stage) that unlocks exactly those
//!   zones still Locked — a zone Locked before this resolved stays Locked (MD-B21, R945).
//! - **The Radiant face** also sets a turn watcher (MD-B22, R946): after a card of the caster's
//!   enters one of their zones in the chosen lane — by a play once it resolves, or by a summon of
//!   theirs — the caster casts a new C+ #71 Book of Buff (its base face, Buff 5 times) aimed at it,
//!   a play (R70) that goes to the caster's graveyard (R87). A move (an animate, a steal, a
//!   rotation) is not putting a card there: only a resolution and a summon answer.

use jackioh_engine::effects::{
    LockArgs, UnlockArgs, WatchLaneArgs, ZoneSpec, cast_new, chosen_options, delay, lock, unlock,
    watch_lane_this_turn,
};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-098";

/// C+ #71 Book of Buff, cast on each card the watch answers.
const BOOK: &str = "classicplus-071";

/// The lane modes' prefix: `lane-1` … `lane-5`.
const LANE_PREFIX: &str = "lane-";

/// The step that unlocks the lanes at the start of the caster's next turn.
const UNLOCK_STEP: &str = "unlock";

/// The step the turn watcher re-enters for each card put into the lane.
const BOOK_STEP: &str = "book";

fn lane_of(option: &str) -> Option<i32> {
    option.strip_prefix(LANE_PREFIX)?.parse().ok()
}

/// R831008: a lane option may be chosen only while that lane is empty; an option that names no lane
/// is not this card's to judge (a fusion asks every ingredient's check).
fn empty_lane_only(state: &GameState, _player: PlayerId, option: &str) -> bool {
    lane_of(option).is_none_or(|lane| lane_is_empty(state, lane))
}

fn zone_value(player: PlayerId, row: Row, lane: i32) -> Value {
    let row = match row {
        Row::Units => "units",
        Row::Backrow => "backrow",
    };
    json!({ "player": player.as_str(), "row": row, "lane": lane })
}

/// The lane zone a stored entry names, relative to the controller running the unlock.
fn zone_spec(entry: &Value, controller: PlayerId) -> Option<ZoneSpec> {
    let player = match entry.get("player").and_then(Value::as_str) {
        Some("p1") => PlayerId::P1,
        Some("p2") => PlayerId::P2,
        _ => return None,
    };
    let row = match entry.get("row").and_then(Value::as_str) {
        Some("units") => Row::Units,
        Some("backrow") => Row::Backrow,
        _ => return None,
    };
    let lane = entry
        .get("lane")
        .and_then(Value::as_i64)
        .and_then(|lane| i32::try_from(lane).ok())?;
    let side = if player == controller {
        PlayerSpec::SelfSide
    } else {
        PlayerSpec::Enemy
    };
    Some(ZoneSpec::Lane {
        row,
        lane,
        player: Some(side),
    })
}

/// MD-B21, R945: unlock exactly the zones this Showdown locked. `unlock` leaves a zone that is
/// not Locked as it is, but a zone Locked before this resolved was never stored, so it stays.
fn unlock_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    ctx.data
        .get("zones")
        .and_then(Value::as_array)
        .map(|zones| {
            zones
                .iter()
                .filter_map(|entry| zone_spec(entry, ctx.controller))
                .map(|zone| unlock(UnlockArgs { zone }))
                .collect()
        })
        .unwrap_or_default()
}

/// MD-B22, R946: cast Book of Buff aimed at the card the watch answered.
fn book_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(target) = ctx.data.get("instanceId").and_then(Value::as_str) else {
        return vec![];
    };
    vec![cast_new(json_as(json!({ "def": BOOK, "aimAt": target })))]
}

fn showdown(radiant: bool) -> Script {
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: (1..=5).map(|lane| format!("{LANE_PREFIX}{lane}")).collect(),
        }],
        mode_allowed: Some(mode_allowed_hook(empty_lane_only)),
        cry: Some(hook(move |ctx| {
            // R831008: a cast made with no empty lane chose none, and does nothing.
            let Some(lane) = chosen_options(ctx).iter().find_map(|option| lane_of(option)) else {
                return vec![];
            };
            let mut effects: Vec<Effect> = Vec::new();
            let mut zones: Vec<Value> = Vec::new();
            for player in [PlayerId::P1, PlayerId::P2] {
                for row in [Row::Units, Row::Backrow] {
                    for other in 1..=5 {
                        if other == lane {
                            continue;
                        }
                        let slot = ZoneSlot { player, row, lane: other };
                        if is_locked(ctx.state, slot) {
                            continue;
                        }
                        let side = if player == ctx.controller {
                            PlayerSpec::SelfSide
                        } else {
                            PlayerSpec::Enemy
                        };
                        effects.push(lock(LockArgs {
                            zone: ZoneSpec::Lane { row, lane: other, player: Some(side) },
                        }));
                        zones.push(zone_value(player, row, other));
                    }
                }
            }
            effects.push(delay(json_as(json!({
                "at": { "phase": "start", "player": "self" },
                "step": UNLOCK_STEP,
                "hook": RESUME_HOOK,
                "data": { "zones": zones },
            }))));
            if radiant {
                effects.push(watch_lane_this_turn(WatchLaneArgs {
                    lane,
                    step: BOOK_STEP.to_string(),
                    label: format!(
                        "This turn, after you put a card into lane {lane}, cast Book of Buff on it"
                    ),
                    hook: None,
                    data: None,
                }));
            }
            effects
        })),
        resume: IndexMap::from([(UNLOCK_STEP, hook(unlock_step)), (BOOK_STEP, hook(book_step))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: showdown(false),
        radiant: showdown(true),
    }
}

// M #98 Showdown — SPEC §8.8 row 98, BUILD M10 row M 98.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHOWDOWN: &str = "meditative-098";
    const BOOK: &str = "classicplus-071";
    const VANILLA: &str = "core-008"; // (1) 4/4 Human.
    const RECRUIT: &str = "core-069"; // (2) Spell: Recruit a 3-or-less Unit.
    const FIENDER: &str = "core-092"; // (2) 5/7 Stack: played onto a taken unit zone.
    const FILLER: &str = "core-005";

    fn library() -> Value {
        json!([FILLER, FILLER, FILLER, FILLER, FILLER, FILLER])
    }

    /// p1 holds Showdown (Radiant on `radiant`) over `library`; p2 holds fillers.
    fn casting(seed: &str, radiant: bool, library: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": SHOWDOWN, "radiant": radiant }, FILLER],
                "library": library,
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }))
    }

    fn locked(s: &Scenario, player: PlayerId, row: Row, lane: i32) -> bool {
        is_locked(s.state(), ZoneSlot { player, row, lane })
    }

    /// The lanes `legalActions` offers p1's Showdown for, in order.
    fn offered_lanes(s: &Scenario) -> Vec<String> {
        let showdown = s.card(SHOWDOWN).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, modes, .. } if instance_id == showdown => {
                    modes.and_then(|modes| modes.first().cloned())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r945_locks_every_zone_of_the_other_four_lanes_on_both_sides() {
        let mut s = casting("showdown-lock", false, library());
        s.play(SHOWDOWN, json!({ "modes": ["lane-2"] }));
        for player in [P1, P2] {
            for row in [Row::Units, Row::Backrow] {
                for lane in 1..=5 {
                    assert_eq!(
                        locked(&s, player, row, lane),
                        lane != 2,
                        "lane {lane} {row:?} of {player:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn r945_plays_are_refused_there_and_summons_still_land() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "showdown-refuse",
            "p1": {
                "mana": 10,
                "hand": [{ "def": SHOWDOWN }, { "def": VANILLA }, { "def": RECRUIT }, FILLER],
                "library": [{ "def": VANILLA }, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }));
        s.play(SHOWDOWN, json!({ "modes": ["lane-2"] }));
        // A unit into a Locked unit zone is refused, on either side's row naming …
        s.expect_refused(|s| s.play(VANILLA, json!({ "zone": 1 })));
        s.expect_refused(|s| s.play(VANILLA, json!({ "zone": 3 })));
        // … but a summon still lands (first entry: the open lane 2).
        s.play(RECRUIT, json!({}));
        let landed = s.unit(P1, 2).expect("the recruit lands in the open lane");
        assert_eq!(landed.def_id, VANILLA);
    }

    #[test]
    fn r945_at_your_next_turn_exactly_those_zones_unlock_and_a_prelocked_zone_stays() {
        let mut s = casting("showdown-unlock", false, library());
        // Lock p1's lane-3 unit zone before Showdown resolves: it is never stored.
        lock_zone(s.state_mut(), ZoneSlot { player: P1, row: Row::Units, lane: 3 });
        s.play(SHOWDOWN, json!({ "modes": ["lane-2"] }));
        assert!(locked(&s, P1, Row::Units, 3), "pre-Locked before");
        // At the start of the caster's next turn every Showdown Locked zone opens …
        s.end_turn();
        s.end_turn();
        for player in [P1, P2] {
            for row in [Row::Units, Row::Backrow] {
                for lane in 1..=5 {
                    let still = player == P1 && row == Row::Units && lane == 3;
                    assert_eq!(locked(&s, player, row, lane), still, "lane {lane} {row:?} of {player:?}");
                }
            }
        }
    }

    #[test]
    fn r946_radiant_casts_book_of_buff_on_a_card_played_into_the_lane() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "showdown-book",
            "p1": {
                "mana": 10,
                "hand": [{ "def": SHOWDOWN, "radiant": true }, { "def": VANILLA }, FILLER],
                "library": library(),
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }));
        s.play(SHOWDOWN, json!({ "modes": ["lane-2"] }));
        let mark = s.events().len();
        s.play(VANILLA, json!({ "zone": 2 }));
        let played = s.unit(P1, 2).expect("played");
        // The base Book Upgrades 5 times: five `upgraded` events on the played card.
        let changes: Vec<TuningChange> = s.events()[mark..]
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, change, .. } if instance_id == &played.id => {
                    Some(change.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(changes.len(), 5, "the Book is cast on the played card");
        // Each Buff draws its row (§6.1: cost, stats or a keyword for a vanilla Unit); every stats
        // Buff adds its 4 across attack and health, and each lands on the played card.
        let stat_totals: Vec<i32> = changes
            .iter()
            .filter_map(|change| match change {
                TuningChange::Stats { attack, health } => Some(attack + health),
                _ => None,
            })
            .collect();
        assert!(stat_totals.iter().all(|total| *total == TUNE_STAT_TOTAL));
        let grown = s.stats(&played.id);
        assert_eq!(
            grown.attack + grown.health,
            4 + 4 + stat_totals.iter().sum::<i32>(),
            "each stats Buff lands (+4 split across attack and health)"
        );
    }

    #[test]
    fn r946_radiant_casts_on_a_summon_into_the_lane_and_not_on_other_lanes() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "showdown-summon",
            "p1": {
                "mana": 10,
                "hand": [{ "def": SHOWDOWN, "radiant": true }, { "def": VANILLA }, { "def": RECRUIT }, FILLER],
                "library": [{ "def": VANILLA }, FILLER, FILLER],
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }));
        s.play(SHOWDOWN, json!({ "modes": ["lane-2"] }));
        // A play into the lane answers …
        s.play(VANILLA, json!({ "zone": 2 }));
        let books = |s: &Scenario| s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == BOOK).count();
        assert_eq!(books(&s), 1, "one Book for the played card");
        // … a summon into the taken lane lands Locked, in lane 1, and answers nothing …
        s.play(RECRUIT, json!({}));
        let recruit = s.unit(P1, 1).expect("the recruit lands Locked in lane 1");
        assert_eq!(recruit.def_id, VANILLA);
        assert!(locked(&s, P1, Row::Units, 1), "lane 1 is still Locked");
        assert_eq!(books(&s), 1, "no Book for another lane");
    }

    #[test]
    fn r946_the_watch_ends_with_the_turn_and_the_book_counts_as_a_play() {
        crate::register_all();
        let mut s = scenario(json!({
            "seed": "showdown-watch-end",
            "p1": {
                "mana": 10,
                "hand": [{ "def": SHOWDOWN, "radiant": true }, { "def": VANILLA }, { "def": FIENDER }, FILLER],
                "library": library(),
            },
            "p2": { "hand": [FILLER], "library": self::library() },
        }));
        s.play(SHOWDOWN, json!({ "modes": ["lane-2"] }));
        s.play(VANILLA, json!({ "zone": 2 }));
        // Showdown plus the Book: two Spells played this turn, and the Book rests in the graveyard.
        assert_eq!(played_this_turn_of_type(s.state(), P1, &[CardType::Spell]), 2);
        assert_eq!(s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == BOOK).count(), 1);
        // Next turn the lanes are open and the watch is gone: a play into the lane (Stacked onto
        // the Vanilla) casts no more Books.
        s.end_turn();
        s.end_turn();
        s.play(FIENDER, json!({ "zone": 2 }));
        assert_eq!(s.unit(P1, 2).expect("the pile's top").def_id, FIENDER);
        assert_eq!(s.pile(P1, "graveyard").into_iter().filter(|card| card.def_id == BOOK).count(), 1);
    }

    mod r831008_only_an_empty_lane {
        use super::*;

        const PALANTIR: &str = "classic-004"; // C #4: steals a (1) Spell and casts it at random.
        const TRAP: &str = "core-096"; // My Pawn, a Trap, set face down.

        /// p1 (Showdown on the Radiant face on `radiant`) faces p2's Vanillas in lanes 2 and 3 and a
        /// face-down Trap in p2's backrow lane 5, so lanes 1 and 4 are the empty ones.
        fn crowded(radiant: bool) -> Scenario {
            crate::register_all();
            scenario(json!({
                "seed": "showdown-empty",
                "p1": {
                    "mana": 10,
                    "hand": [{ "def": SHOWDOWN, "radiant": radiant }, FILLER],
                    "library": library(),
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }],
                    "backrow": [{ "def": TRAP, "lane": 5 }],
                    "library": library(),
                },
            }))
        }

        #[test]
        fn r831008_legal_actions_offer_only_the_lanes_with_all_four_zones_empty() {
            for radiant in [false, true] {
                let s = crowded(radiant);
                assert_eq!(offered_lanes(&s), vec!["lane-1", "lane-4"], "radiant {radiant}");
            }
        }

        #[test]
        fn r831008_a_taken_lane_is_refused_an_enemy_unit_or_a_face_down_card_alike() {
            for radiant in [false, true] {
                let mut s = crowded(radiant);
                for lane in ["lane-2", "lane-3", "lane-5"] {
                    s.expect_refused(|s| s.play(SHOWDOWN, json!({ "modes": [lane] })));
                }
                s.play(SHOWDOWN, json!({ "modes": ["lane-4"] }));
                assert!(!locked(&s, P1, Row::Units, 4), "the chosen lane stays open");
                assert!(locked(&s, P2, Row::Units, 2), "a taken lane is Locked");
            }
        }

        #[test]
        fn r831008_with_no_empty_lane_showdown_cannot_be_played() {
            crate::register_all();
            for radiant in [false, true] {
                let mut s = scenario(json!({
                    "seed": "showdown-full",
                    "p1": {
                        "mana": 10,
                        "hand": [{ "def": SHOWDOWN, "radiant": radiant }, FILLER],
                        "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }],
                        "library": library(),
                    },
                    "p2": {
                        "hand": [FILLER],
                        "field": [{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }],
                        "backrow": [{ "def": TRAP, "lane": 5 }],
                        "library": library(),
                    },
                }));
                assert!(offered_lanes(&s).is_empty(), "radiant {radiant}: no lane is offered");
                for lane in 1..=5 {
                    s.expect_refused(|s| s.play(SHOWDOWN, json!({ "modes": [format!("lane-{lane}")] })));
                }
            }
        }

        #[test]
        fn r831008_a_random_cast_picks_only_an_empty_lane() {
            crate::register_all();
            // The Radiant Palantir (which stays on the field: it Tributes itself only at its third
            // steal) sits in p1's backrow lane 1 and p2's Vanillas hold lanes 2, 3 and 5: lane 4 is
            // the only empty one, so whatever the seed, the cast Palantir makes picks it.
            for seed in ["showdown-cast-a", "showdown-cast-b", "showdown-cast-c", "showdown-cast-d"] {
                let mut s = scenario(json!({
                    "seed": seed,
                    "active": "p2",
                    "p1": {
                        "hand": [FILLER],
                        "backrow": [{ "def": PALANTIR, "radiant": true, "faceUp": true, "lane": 1 }],
                        "library": library(),
                    },
                    "p2": {
                        "mana": 10,
                        "hand": [{ "def": SHOWDOWN }, FILLER],
                        "field": [
                            { "def": VANILLA, "lane": 2 },
                            { "def": VANILLA, "lane": 3 },
                            { "def": VANILLA, "lane": 5 },
                        ],
                        "library": library(),
                    },
                }));
                s.play(SHOWDOWN, json!({ "modes": ["lane-4"] }));
                for player in [P1, P2] {
                    for row in [Row::Units, Row::Backrow] {
                        for lane in 1..=5 {
                            assert_eq!(locked(&s, player, row, lane), lane != 4, "{seed}: lane {lane} {row:?} of {player:?}");
                        }
                    }
                }
            }
        }

        #[test]
        fn r831008_lane_is_empty_reads_both_sides_and_both_rows() {
            let s = crowded(false);
            let empty: Vec<i32> = (1..=5).filter(|lane| lane_is_empty(s.state(), *lane)).collect();
            assert_eq!(empty, vec![1, 4]);
        }
    }
}
