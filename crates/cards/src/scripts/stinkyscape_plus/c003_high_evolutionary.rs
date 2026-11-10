//! SPEC §8.12 S+ #3 High Evolutionary (Unit 7/7 → 14/14, cost 3, Legendary), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Cry: Evolve a friendly permanent. / Aura: Your evolutions trigger twice."
//!   Radiant: "Cry: Evolve a friendly permanent. / Aura: Your evolutions trigger thrice."
//!
//! R3542001: the Cry declares one friendly permanent (a Unit or a backrow card) that can evolve now: acting
//! on the field, neither face-down nor Dormant, not evolved yet and printing an Evolve text
//! (`evolution::can_evolve`); with no such permanent nothing is offered and the Cry does nothing. The
//! `evolve` verb runs the pick's Evolve text, and the Aura (`Script.evolve_extra`, R3536003) gives its
//! controller's evolutions 1 (twice) or 2 (thrice) extra runs of it while this acts on the field: the highest
//! multiplier holds, never the sum, and this card's own Cry already runs under its own Aura.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-003";

/// R3542001: the declared pick's check, so only a permanent that can evolve now is offered.
const CAN_EVOLVE: &str = "highEvolutionaryCanEvolve";

/// R3536003: the extra runs "twice" and "thrice" give: one and two more than the evolution's own.
const TWICE_EXTRA_RUNS: i32 = 1;
const THRICE_EXTRA_RUNS: i32 = 2;

fn evolutionary(extra: i32) -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "ally", "of": ["unit", "backrow"], "check": CAN_EVOLVE }),
        )],
        target_checks: IndexMap::from([(
            CAN_EVOLVE,
            target_check(|args| {
                args.candidate.is_some_and(|card| {
                    acts_on_field(args.state, card)
                        && !jackioh_engine::preview::lasting_text_off(args.state, card)
                        && jackioh_engine::evolution::can_evolve(args.state, card)
                })
            }),
        )]),
        cry: Some(hook(|_ctx| {
            vec![evolve(json_as(json!({ "target": { "of": "chosen" } })))]
        })),
        evolve_extra: Some(read_hook(move |_args| extra)),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: evolutionary(TWICE_EXTRA_RUNS),
        radiant: evolutionary(THRICE_EXTRA_RUNS),
    }
}

// S+ #3 High Evolutionary — SPEC §8.12 row 3 (R3542001; R3536000, R3536003, R3542000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{built_starship, launch_starship, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EXPLORER: &str = "stinkyscapeplus-075"; // J-lease S-Space EX-plorer: EX-plorer: Evolve: a Pack on top of your deck.
    const PACK: &str = "stinkyscapeplus-075-1"; // J-lease S-Space EX-plorer Pack: EX-plorer Pack
    const FLORA: &str = "stinkyscapeplus-087"; // Frontier Flora: a Field Spell, Evolve: Plague Counters.
    const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: Starship Piece, keywords only
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// p1 holds the Evolutionary (Radiant if `radiant`) and 10 mana, with `field` and a stocked library.
    fn board(seed: &str, radiant: bool, field: Value, backrow: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": field,
                "backrow": backrow,
                "library": crate::filler(6),
            },
            "p2": { "hand": crate::filler(1), "field": [VANILLA], "library": crate::filler(4) },
        }))
    }

    fn packs_on_top(s: &Scenario, n: usize) -> usize {
        s.pile(P1, "library").iter().take(n).filter(|card| card.def_id == PACK).count()
    }

    fn evolved(s: &Scenario) -> usize {
        s.events().iter().filter(|event| matches!(event, GameEvent::Evolved { .. })).count()
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_evolves_the_chosen_card_and_its_aura_runs_the_evolve_text_twice() {
            let mut s = board("s003-base", false, json!([EXPLORER]), json!([]));
            let target = s.unit_at(P1, 1);
            s.play(ID, json!({ "zone": 2, "targets": pick::instance(&target) }));
            assert_eq!(s.card(&target).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 1, "R3536000: one evolving, two runs");
            assert_eq!(packs_on_top(&s, 2), 2, "the Evolve text ran twice");
            assert_eq!(s.pile(P1, "library").iter().filter(|card| card.def_id == PACK).count(), 2);
            assert_eq!(evolved(&s), 1);
        }

        #[test]
        fn its_aura_doubles_an_evolution_it_did_not_cause() {
            let (mut s, ship) = built_starship(
                "s003-launch",
                json!([{ "def": AP_ROUNDS, "lane": 1 }]),
                json!({ "field": [EXPLORER, ID] }),
                json!({}),
            );
            launch_starship(&mut s, &ship);
            let explorer = s.unit_at(P1, 2);
            assert_eq!(explorer.def_id, EXPLORER);
            assert_eq!(s.card(&explorer).evolved, Some(true));
            assert_eq!(packs_on_top(&s, 2), 2, "a launch put 2 Packs on the deck");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_aura_runs_it_thrice() {
            let mut s = board("s003-radiant", true, json!([EXPLORER]), json!([]));
            let target = s.unit_at(P1, 1);
            s.play(ID, json!({ "zone": 2, "targets": pick::instance(&target) }));
            assert_eq!(s.card(&target).evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 1);
            assert_eq!(packs_on_top(&s, 3), 3, "the Evolve text ran three times");
        }
    }

    #[test]
    fn r3542001_an_evolved_card_a_textless_unit_and_an_enemy_card_are_not_targets() {
        let mut s = board("s003-refused", false, json!([EXPLORER, EXPLORER, VANILLA]), json!([]));
        let spent = s.unit_at(P1, 2);
        s.card_mut(&spent).evolved = Some(true);
        let textless = s.unit_at(P1, 3);
        let enemy = s.unit_at(P2, 1);
        for card in [&spent, &textless, &enemy] {
            s.expect_refused_with(
                |s| s.play(ID, json!({ "zone": 4, "targets": pick::instance(card) })),
                "not a legal target",
            );
        }
        assert_eq!(evolutions_of(s.state(), P1), 0);
    }

    #[test]
    fn r3542001_with_no_target_the_cry_does_nothing() {
        let mut s = board("s003-none", false, json!([VANILLA]), json!([]));
        s.play(ID, json!({}));
        s.expect_in_zone(ID, "field");
        assert_eq!(evolutions_of(s.state(), P1), 0);
        assert_eq!(evolved(&s), 0);
    }

    #[test]
    fn r3542001_it_evolves_a_field_spell_in_the_backrow() {
        let mut s = board("s003-flora", false, json!([]), json!([{ "def": FLORA, "lane": 4 }]));
        let flora = s.backrow(P1, 4).expect("the Flora");
        s.play(ID, json!({ "targets": pick::instance(&flora) }));
        // Flora's Evolve places 2 Plague Counters per run, and the Aura runs it twice: 4 counters, 12/12.
        let card = s.card(&flora).clone();
        assert!(matches!(card.zone, Zone::Field { row: Row::Units, .. }), "{:?}", card.zone);
        let stats = s.stats(&card);
        assert_eq!((stats.attack, stats.health), (12, 12));
    }
}
