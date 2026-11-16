//! S #46 Circadian Halfcycler (SPEC §8.10 row 46): (1) Field Spell, Epic, a card of the Stinkyscape set.
//!   Base:    "Aura: The AI plays the first 3 actions of each of your turns.
//!             End of turn: Summon 1 to 2 Drone Interceptors."
//!   Radiant: "Aura: The AI plays the first 3 actions of each of your turns.
//!             End of turn: Summon 1 to 2 Radiant Drone Interceptors."
//! Engine: the autopilot as an aura (R1844000), `pilotsActions: 3`: each of its controller's turns while
//!   it stands engages R1183002's autopilot as the turn begins, which plays its first 3 actions (R456's
//!   plays and attacks, never an answer) and then hands the turn back for good; one that arrives mid-turn
//!   waits for its controller's next turn. End of turn (R62), its controller's: one draw of the match rng
//!   (R60) picks 1 Drone Interceptor two times in three and 2 one time in three (the designer's ⅔ and ⅓,
//!   R3951002), each summoned into their leftmost open unit zone (R64); with no open zone nothing.
//!   Radiant: Radiant Interceptors. Through v0.6.4c it summoned one, which `script_v0_6_4c` keeps for
//!   those replays (issue #3951, R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-046";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

/// R1844000: "the first 3 actions of each of your turns", the printed count the aura plays.
const PILOTED_ACTIONS: i32 = 3;

/// R3951002: "1 to 2" Interceptors: one draw over `INTERCEPTOR_ODDS` outcomes, the first of which summons
/// `MOST_INTERCEPTORS`, the rest `FEWEST_INTERCEPTORS` (a third and two thirds).
const INTERCEPTOR_ODDS: i32 = 3;
const FEWEST_INTERCEPTORS: i32 = 1;
const MOST_INTERCEPTORS: i32 = 2;

/// `rolls`: whether the end of turn rolls 1 to 2 Interceptors (since issue #3951) or summons one.
fn face(radiant: bool, rolls: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            pilots_actions: Some(PILOTED_ACTIONS),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(move |ctx| {
            let count = if rolls && ctx.rng.int(INTERCEPTOR_ODDS) == 0 {
                MOST_INTERCEPTORS
            } else {
                FEWEST_INTERCEPTORS
            };
            (0..count)
                .map(|_| summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false, true),
        radiant: face(true, true),
    }
}

/// R767000: the scripts through v0.6.4c, which summoned one Interceptor (issue #3951). `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    CardScripts {
        base: face(false, false),
        radiant: face(true, false),
    }
}

// S #46 Circadian Halfcycler — SPEC §8.10 row 46, BUILD M12 row S 46 (R1844000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HALFCYCLER: &str = "stinkyscape-046";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005";

    fn fielded(radiant: bool, active: &str, hand: Value, field: Value) -> Scenario {
        seeded("halfcycler", radiant, active, hand, field)
    }

    fn seeded(seed: &str, radiant: bool, active: &str, hand: Value, field: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": active,
            "p1": {
                "hand": hand,
                "backrow": [{ "def": HALFCYCLER, "radiant": radiant, "faceUp": true }],
                "field": field,
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }))
    }

    /// #3142: it costs (1) on both faces, where it cost (2).
    #[test]
    fn it_costs_1_on_both_faces() {
        for radiant in [false, true] {
            let s = scenario(json!({
                "p1": { "hand": [{ "def": HALFCYCLER, "radiant": radiant }] },
                "p2": { "hand": [FILLER] },
            }));
            assert_eq!(effective_cost(s.state(), s.card(HALFCYCLER), Default::default()), 1, "radiant {radiant}");
        }
    }

    fn engaged(events: &[GameEvent], player: PlayerId) -> bool {
        events
            .iter()
            .any(|event| matches!(event, GameEvent::AutopilotEngaged { player: by, .. } if *by == player))
    }

    fn actions(events: &[GameEvent]) -> usize {
        events
            .iter()
            .filter(|event| {
                matches!(event, GameEvent::CardPlayed { player: P1, .. } | GameEvent::AttackDeclared { .. })
            })
            .count()
    }

    fn units(s: &Scenario, player: PlayerId) -> Vec<(String, bool)> {
        (1..=5)
            .filter_map(|lane| s.unit(player, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    #[test]
    fn r1844000_the_autopilot_plays_your_first_3_actions_then_hands_the_turn_back() {
        let mut s = fielded(false, "p2", json!(vec![VANILLA; 4]), json!([]));
        s.end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1), "p1's turn began piloted");
        assert_eq!(actions(events), 3, "the aura's count, and no more");
        assert_eq!(s.state().active, P1, "then the turn is p1's again");
        assert_eq!(s.state().players[P1].autopilot, None, "handed back for good");
        assert!(
            legal_actions(s.state(), P1)
                .iter()
                .any(|action| action.action_type() == ActionType::Play),
            "with plays left to make"
        );
        assert!(
            !units(&s, P1).iter().any(|(def_id, _)| def_id == INTERCEPTOR),
            "no Interceptor until p1's own end of turn"
        );
    }

    #[test]
    fn r1844000_played_mid_turn_it_waits_for_your_next_turn() {
        let mut hand = vec![HALFCYCLER];
        hand.extend([VANILLA; 4]);
        let mut s = scenario(json!({
            "p1": { "hand": hand, "library": filler(6) },
            "p2": { "hand": [FILLER], "library": filler(6) },
        }));
        s.play(HALFCYCLER, json!({ "zone": 1 }));
        assert!(!engaged(s.last_events(), P1), "the turn it arrives is p1's own");
        assert_eq!(s.state().players[P1].autopilot, None);
        s.end_turn().end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1), "p1's next turn began piloted");
        assert_eq!(actions(events), 3);
    }

    /// p1's end of turn on `seed`: the Interceptors it summoned, each with its Radiant flag.
    fn summoned_on(seed: &str, radiant: bool) -> Vec<(String, bool)> {
        let mut s = seeded(seed, radiant, "p1", json!([FILLER]), json!([]));
        s.end_turn();
        assert!(units(&s, P2).is_empty());
        units(&s, P1)
    }

    /// Issue #3951: "Summon 1 to 2 Drone Interceptors", two a third of the time.
    #[test]
    fn r62_r3951002_at_your_end_of_turn_it_summons_1_or_2_drone_interceptors() {
        let one = seed_where("halfcycler-one", |seed| summoned_on(seed, false).len() == 1);
        assert_eq!(summoned_on(&one, false), vec![(INTERCEPTOR.to_string(), false)]);
        let two = seed_where("halfcycler-two", |seed| summoned_on(seed, false).len() == 2);
        assert_eq!(summoned_on(&two, false), vec![(INTERCEPTOR.to_string(), false); 2]);
    }

    #[test]
    fn r3951002_two_come_about_a_third_of_the_time() {
        let seeds = 90;
        let twos = (0..seeds)
            .map(|n| summoned_on(&format!("halfcycler-odds-{n}"), false).len())
            .inspect(|count| assert!((1..=2).contains(count), "{count}"))
            .filter(|count| *count == 2)
            .count();
        assert!((15..=45).contains(&twos), "{twos} twos in {seeds}");
    }

    /// R767000: through v0.6.4c it summoned one (issue #3951).
    #[test]
    fn r767000_the_script_kept_for_v0_6_4c_summons_one() {
        crate::under_kept(HALFCYCLER, super::script_v0_6_4c(), || {
            for n in 0..12 {
                assert_eq!(summoned_on(&format!("halfcycler-kept-{n}"), false).len(), 1, "seed {n}");
            }
        });
    }

    #[test]
    fn r64_a_full_unit_row_summons_nothing() {
        let mut s = fielded(false, "p1", json!([FILLER]), json!(vec![VANILLA; 5]));
        s.end_turn();
        assert_eq!(units(&s, P1), vec![(VANILLA.to_string(), false); 5]);
    }

    #[test]
    fn radiant_radiant_interceptors_and_the_same_3_actions() {
        for n in 0..6 {
            let made = summoned_on(&format!("halfcycler-radiant-{n}"), true);
            assert!((1..=2).contains(&made.len()), "{made:?}");
            assert!(made.iter().all(|unit| *unit == (INTERCEPTOR.to_string(), true)), "{made:?}");
        }

        let mut s = fielded(true, "p2", json!(vec![VANILLA; 4]), json!([]));
        s.end_turn();
        let events = s.last_events();
        assert!(engaged(events, P1));
        assert_eq!(actions(events), 3);
        assert_eq!(s.state().players[P1].autopilot, None);
    }
}
