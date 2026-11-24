// The Infinite Assault's Batch verbs (issue #4588), each through a fixture script
// (`fixtures/batch.rs`) before a card uses it: "your Units have +N Attack" and "your Units have
// <keyword>" this turn (R4588000), doubling a Unit's stats (R4588005) and a copy of the cheapest or
// the highest-Attack Unit (R4588001).

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::batch::*;

/// `p1`'s turn, its main phase, each side's setup as given, with plain cards to draw, and no turn
/// that ends by itself (R345), so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_batch();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([RALLY, RALLY, RALLY, RALLY]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

fn has_rush(s: &Scenario, id: &str) -> bool {
    unit_has(s.state(), s.card(id), KeywordKind::Rush)
}

const UNIT: &str = "stand-in (1) 2/2";

mod r4588000_units_have_attack_and_keyword_this_turn {
    use super::*;

    #[test]
    fn r4588000_units_have_the_attack_and_rush_this_turn_and_later_arrivals_too() {
        let mut s = board(
            "r4588000-rally",
            json!({ "hand": [RALLY, UNIT], "field": [{ "def": UNIT, "lane": 1 }], "mana": 10 }),
            json!({}),
        );
        let first = id_at(&s, P1, 1);
        s.play(RALLY, json!({}));
        s.expect_stats(first.as_str(), json!({ "attack": 4 }));
        assert!(has_rush(&s, &first));
        // A Unit that arrives later has them too.
        s.play(UNIT, json!({ "zone": 2 }));
        let second = id_at(&s, P1, 2);
        s.expect_stats(second.as_str(), json!({ "attack": 4 }));
        assert!(has_rush(&s, &second));
        // After the turn both are gone, and the modifier with them.
        s.end_turn();
        s.expect_stats(first.as_str(), json!({ "attack": 2 }));
        assert!(!has_rush(&s, &first));
        assert!(s.state().players[P1].mods.is_empty(), "this turn ends at cleanup");
    }

    #[test]
    fn r4588000_the_enemys_units_have_none_and_both_seats_read_the_badge() {
        let mut s = board(
            "r4588000-badge",
            json!({ "hand": [RALLY], "field": [{ "def": UNIT, "lane": 1 }] }),
            json!({ "field": [{ "def": UNIT, "lane": 1 }] }),
        );
        s.play(RALLY, json!({}));
        let theirs = id_at(&s, P2, 1);
        s.expect_stats(theirs.as_str(), json!({ "attack": 2 }));
        assert!(!has_rush(&s, &theirs), "the enemy's Units take none");
        for (viewer, side) in [(P1, true), (P2, false)] {
            let view = s.view(viewer);
            let mods = if side {
                &view.you.modifiers
            } else {
                &view.opponent.modifiers
            };
            assert!(
                mods.iter().any(|badge| badge.label == "Your Units have +2 Attack"),
                "{viewer:?}: {mods:?}"
            );
        }
    }
}

mod r4588005_double_stats {
    use super::*;

    #[test]
    fn r4588005_double_stats_doubles_attack_and_what_health_is_left() {
        let mut s = board(
            "r4588005-double",
            json!({ "hand": [DOUBLE], "field": [{ "def": "stand-in (1) 3/5", "damage": 2, "lane": 1 }] }),
            json!({}),
        );
        let unit = s.unit_at(P1, 1);
        s.play(DOUBLE, pick::targets(pick::instance(&unit)));
        s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r4588005_double_stats_stops_at_the_cap() {
        let mut s = board(
            "r4588005-cap",
            json!({ "hand": [DOUBLE], "field": [{ "def": "stand-in (1) 600/2", "lane": 1 }] }),
            json!({}),
        );
        let unit = s.unit_at(P1, 1);
        s.play(DOUBLE, pick::targets(pick::instance(&unit)));
        s.expect_stats(&unit, json!({ "attack": 999 }));
    }
}

mod r4588001_cheapest_and_mightiest {
    use super::*;

    const CHEAP: &str = "stand-in (1) 2/2";
    const BIG: &str = "stand-in (3) 5/5";

    fn copies_of(s: &Scenario, def: &str) -> usize {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == def)
            .count()
    }

    #[test]
    fn r4588001_the_cheapest_and_the_mightiest_are_copied() {
        let mut cheap = board(
            "r4588001-cheapest",
            json!({ "hand": [COPY_CHEAPEST], "field": [{ "def": CHEAP, "lane": 1 }, { "def": BIG, "lane": 2 }] }),
            json!({}),
        );
        cheap.play(COPY_CHEAPEST, json!({}));
        assert_eq!(copies_of(&cheap, CHEAP), 2, "the cheapest is copied");
        assert_eq!(copies_of(&cheap, BIG), 1, "the mightiest is not");
        let mut mighty = board(
            "r4588001-mightiest",
            json!({ "hand": [COPY_MIGHTIEST], "field": [{ "def": CHEAP, "lane": 1 }, { "def": BIG, "lane": 2 }] }),
            json!({}),
        );
        mighty.play(COPY_MIGHTIEST, json!({}));
        assert_eq!(copies_of(&mighty, BIG), 2, "the mightiest is copied");
        assert_eq!(copies_of(&mighty, CHEAP), 1, "the cheapest is not");
    }

    /// Which of the two tied Units the copy is made of, by def id.
    fn copied_def(seed: &str) -> String {
        let mut s = board(
            seed,
            json!({ "hand": [COPY_CHEAPEST], "field": [{ "def": CHEAP, "lane": 1 }, { "def": "stand-in (1) 3/3", "lane": 2 }] }),
            json!({}),
        );
        s.play(COPY_CHEAPEST, json!({}));
        for def in [CHEAP, "stand-in (1) 3/3"] {
            if copies_of(&s, def) == 2 {
                return def.to_string();
            }
        }
        panic!("no tied Unit was copied");
    }

    #[test]
    fn r4588001_a_tie_is_broken_by_the_match_rng() {
        let first = seed_where("r4588001-tie", |seed| copied_def(seed) == CHEAP);
        let second = seed_where("r4588001-tie", |seed| copied_def(seed) == "stand-in (1) 3/3");
        assert_ne!(copied_def(&first), copied_def(&second), "each tied Unit is copied on some seed");
    }

    #[test]
    fn r4588001_no_unit_copies_nothing() {
        let mut s = board("r4588001-empty", json!({ "hand": [COPY_CHEAPEST] }), json!({}));
        s.play(COPY_CHEAPEST, json!({}));
        assert!(active_units_of(s.state(), P1).is_empty());
    }
}
