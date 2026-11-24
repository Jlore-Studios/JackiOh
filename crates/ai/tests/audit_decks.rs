//! Audit (decks area): exploration.

use jackioh_ai::{AI_DECK, AI_GATE, AI_GATE_BUDGET, AiDeckOptions, Matchup, build_ai_deck, game_config};
use jackioh_engine::testkit::{AI_DIFFICULTY, CardType, create_rng, def_of};

fn units_in(deck: &[String]) -> usize {
    deck.iter()
        .filter(|id| def_of(None, id).type_ == CardType::Unit)
        .count()
}

#[test]
fn explore_default_unit_floor() {
    jackioh_cards::register_all();
    for (name, handicap) in [
        ("easy", AI_DIFFICULTY.easy),
        ("medium", AI_DIFFICULTY.medium),
        ("hard", AI_DIFFICULTY.hard),
    ] {
        let size = handicap.deck_size;
        let units_needed = (f64::from(size) * AI_DECK.min_unit_share).ceil() as usize;
        let opts = AiDeckOptions {
            mana_cap: Some(handicap.mana_cap),
            ..AiDeckOptions::default()
        };
        let mut miss = 0;
        let mut first = None;
        for n in 1..=5000 {
            let seed = format!("audit-default:{name}:{n}");
            let deck = build_ai_deck(&mut create_rng(&seed, 0), size, &opts);
            let units = units_in(&deck);
            if units < units_needed {
                miss += 1;
                if first.is_none() {
                    first = Some(format!("{seed}: {units} of {size}, needs {units_needed}"));
                }
            }
        }
        eprintln!("{name} size {size} cap {}: {miss} of 5000 miss; first {first:?}", handicap.mana_cap);
    }
    for matchup in Matchup::ALL {
        for n in 1..=AI_GATE.full_seeds[*matchup] {
            let config = game_config(*matchup, n, AI_GATE_BUDGET, AI_GATE.seed_series);
            for (seat, deck) in [("p1", &config.decks.0), ("p2", &config.decks.1)] {
                let size = deck.len() as i32;
                let units_needed = (f64::from(size) * AI_DECK.min_unit_share).ceil() as usize;
                let units = units_in(deck);
                if units < units_needed {
                    eprintln!("gate {matchup} game {n} {seat}: {units} units of {size}, needs {units_needed}");
                }
            }
        }
    }
}
