//! #892: `layers::AuraTable` over the shipped auras — Jlockeed's Weapons (core-014) and Siphon Squad
//! (classic-088), kept in Rust, Big D-fender (core-001), Spikey Pillow (core-065-1) and Suppressive
//! Aura (core-046) — reads every unit as `layers::unit_view` reads it alone: a Vanilla'd source projects
//! nothing (§6.1, §6.3), a card dormant under a Stack takes no aura (§3.2, R13), and a face-down Field
//! Trap's aura is live (R403).

use jackioh_engine::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::scenario;

const BIG_D: &str = "core-001";
const VANILLA: &str = "core-008";
const WEAPONS: &str = "core-014";
const MENACE: &str = "core-019";
const SUPPRESSIVE: &str = "core-046";
const PILLOW: &str = "core-065-1";
const BACK_BREAKER: &str = "classic-051";
const SIPHON: &str = "classic-088";
const LIBRARY: [&str; 2] = [VANILLA; 2];

fn view(attack: i32, max_health: i32, keywords: Vec<Keyword>, armor: i32, position: Position) -> UnitView {
    UnitView {
        attack,
        max_health,
        health: max_health,
        keywords,
        armor,
        position,
    }
}

fn unit_at(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
}

#[test]
fn a_table_reads_the_shipped_auras_as_a_single_read_does() {
    let mut s = scenario(json!({
        "seed": "aura-table-shipped",
        "p1": {
            "field": [
                VANILLA,
                BIG_D,
                { "def": MENACE, "position": "DEF" },
                VANILLA,
                { "def": BACK_BREAKER, "stack": true },
            ],
            "backrow": [WEAPONS, WEAPONS, SIPHON],
            "library": LIBRARY,
        },
        "p2": {
            "field": [PILLOW, VANILLA],
            "backrow": [SUPPRESSIVE],
            "library": LIBRARY,
        },
    }));
    let vanilla = s.backrow(P1, 2).expect("the second Weapons").id;
    s.card_mut(vanilla.as_str()).vanilla = true;

    let state = s.state();
    let dormant = zones::dormant_units_of(state, P1)
        .first()
        .copied()
        .cloned()
        .expect("the Mr. Vanilla under the Back Breaker");
    let armed = |keywords: &[Keyword]| {
        let mut all = keywords.to_vec();
        all.extend([Keyword::Rush, Keyword::FirstStrike]);
        all
    };
    let expected: Vec<(&str, CardInstance, UnitView)> = vec![
        ("p1 lane 1 Mr. Vanilla", unit_at(&s, P1, 1), view(7, 3, armed(&[]), 0, Position::Atk)),
        ("p1 lane 2 Big D-fender", unit_at(&s, P1, 2), view(3, 6, armed(&[]), 0, Position::Atk)),
        (
            "p1 lane 3 Midrange Menace",
            unit_at(&s, P1, 3),
            view(
                12,
                8,
                vec![
                    Keyword::Taunt,
                    Keyword::Armor { n: 2 },
                    Keyword::Rush,
                    Keyword::FirstStrike,
                    Keyword::Armor { n: 1 },
                ],
                3,
                Position::Def,
            ),
        ),
        (
            "p1 lane 4 Back Breaker",
            unit_at(&s, P1, 4),
            view(6, 1, armed(&[Keyword::Stack]), 0, Position::Atk),
        ),
        ("p1 dormant Mr. Vanilla", dormant, view(4, 4, vec![], 0, Position::Atk)),
        ("p2 lane 1 Spikey Pillow", unit_at(&s, P2, 1), view(0, 1, vec![], 0, Position::Atk)),
        ("p2 lane 2 Mr. Vanilla", unit_at(&s, P2, 2), view(0, 3, vec![], 0, Position::Atk)),
    ];
    for (what, card, want) in &expected {
        let single = layers::unit_view(state, card);
        assert_eq!(&single, want, "{what} read alone");
    }
    assert_eq!(layers::unclamped_attack(state, &unit_at(&s, P2, 1)), -7);
    assert_eq!(layers::unclamped_attack(state, &unit_at(&s, P2, 2)), -3);
}
