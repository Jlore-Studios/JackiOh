//! R4022000 with the shipped cards (SPEC §4.4 steps 3b and 4a): Force Projector's per-turn damage pool
//! is spent only by a hit that lands. A hit Unan catches from a dying ally spends only what the pool
//! soaks of it at Unan, past Unan's own Armor; a hit Final Gambit cancels spends none of it.

use jackioh_engine::testkit::*;

use super::scenario;

const FORCE_PROJECTOR: &str = "stinkyscapeplus-042"; // Force Projector
const UNAN: &str = "meditative-092"; // Unan
const GAMBIT: &str = "classic-052"; // Final Gambit
const FILLER: &str = "core-005"; // Stockpile
const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];
const ALLY: &str = "stand-in (1) 1/2";
const STRIKER: &str = "stand-in (1) 7/7";

/// p2 to move, its 7/7 ready to attack, against `p1`'s side. No turn ends by itself, since p1's next
/// turn would refill the pool.
fn struck(p1: Value) -> Scenario {
    let mut p1 = p1;
    p1["library"] = json!(DECK);
    let mut s = scenario(json!({
        "active": "p2",
        "p1": p1,
        "p2": { "field": [STRIKER], "library": DECK },
    }));
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

fn pool_left(s: &Scenario) -> Option<i32> {
    damage_pool_left(s.state(), s.card(FORCE_PROJECTOR))
}

#[test]
fn r4022000_force_projector_is_spent_by_unan_taking_the_hit_not_by_the_ally_it_left() {
    let mut s = struck(json!({
        "field": [ALLY, UNAN],
        "backrow": [{ "def": FORCE_PROJECTOR, "lane": 1, "faceUp": true }],
    }));
    assert_eq!(pool_left(&s), Some(5));
    let before = s.stats(UNAN).health;
    // 7 less the pool's 5 would kill the 2-health ally, so Unan catches the 7: its Armor 2 leaves 5,
    // which the untouched pool soaks whole.
    s.attack(STRIKER, ALLY);
    assert_eq!(s.stats(UNAN).health, before, "Unan takes nothing");
    assert_eq!(s.stats(ALLY).health, 2, "the ally the hit left is untouched");
    assert_eq!(pool_left(&s), Some(0), "the pool is spent once, at Unan");
}

#[test]
fn r4022000_force_projector_is_untouched_by_a_hit_final_gambit_cancels() {
    let mut s = struck(json!({
        "health": 2,
        "backrow": [
            { "def": FORCE_PROJECTOR, "lane": 1, "faceUp": true },
            { "def": GAMBIT, "lane": 2, "faceUp": false },
        ],
    }));
    // 7 less the pool's 5 leaves 2, lethal on a 2-health hero: Final Gambit cancels the hit.
    s.attack(STRIKER, "hero");
    s.expect_in_zone(GAMBIT, "graveyard");
    assert_eq!(pool_left(&s), Some(5), "a canceled hit spends none of the pool");
}
