//! SPEC §8.10 Stinkyscape 164 The Syndicate, a card of the Stinkyscape set.
//!   (1) Field Spell, Epic (#2521: was (2)).
//!   Base:    "Tradable / Aura: Cards in your hand have Tradable. / Whenever you Trade a card, the card
//!            you put into your deck has a {exileChance}% chance to be exiled and a {radiantChance}%
//!            chance to become Radiant." (30% and 30%)
//!   Radiant: the same words, at 30% and 70%.
//! Engine: Tradable on both faces is catalog data the engine reads in the hand (R1838000). The aura
//!   lends Tradable to every card in its controller's hand while it acts on their field
//!   (`tradable_hand_aura`, R1864002). A trigger on its controller's own `traded` (R1864003) is handed
//!   the card the Trade put into the deck (`traded_card_of`) and rolls once out of `PERCENT` as it
//!   resolves: below `exileChance` the card is exiled from the deck, below `exileChance` plus
//!   `radiantChance` it is made Radiant where it lies, and otherwise nothing; the two chances are
//!   exclusive, so the Radiant face (30 and 70) never leaves the card as it was. A card no longer in
//!   that deck (the Trade's own draw took it straight back) rolls nothing (R129), and the draw stands
//!   either way. The exile names the card, as every exile does; a card made Radiant in a deck stays
//!   hidden (R97, R1838001). The roll has no better side, so Luck does not touch it (R987).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-164";

/// The trigger's id.
const ROLL: &str = "syndicateRoll";

/// R1864003: the one roll on the card the Trade put into the deck, or nothing when it is no longer
/// there (R129).
fn roll(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let Some(traded) = traded_card_of(ctx) else {
        return vec![];
    };
    let controller = ctx.controller;
    let in_deck = find_instance(ctx.state, &traded)
        .is_some_and(|card| matches!(card.zone, Zone::Library { player } if player == controller));
    if !in_deck {
        return vec![];
    }
    let exile_chance = param(&*ctx, "exileChance");
    let radiant_chance = param(&*ctx, "radiantChance");
    let roll = ctx.rng.int(PERCENT);
    if roll < exile_chance {
        vec![exile(json_as(json!({ "target": { "of": "instance", "instanceId": traded } })))]
    } else if roll < exile_chance + radiant_chance {
        vec![set_radiant(json_as(json!({ "instanceId": traded })))]
    } else {
        vec![]
    }
}

fn syndicate() -> Script {
    Script {
        aura: Some(tradable_hand_aura()),
        triggers: vec![
            TriggerDef::new(ROLL, &[GameEventType::Traded], |ctx, _event| roll(ctx))
                .with_when(|ctx, event| matches!(event, GameEvent::Traded { player } if *player == ctx.controller)),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = syndicate();
    // The faces differ only in their declared numbers, catalog data `param()` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #164 The Syndicate — SPEC §8.10 row 164, BUILD M12 row S 164.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-005"; // Stockpile, a Spell that prints no Tradable.
    const PRESTIGE: &str = "meditative-053"; // Prints Tradable.
    const TRADE_UNION: &str = "stinkyscape-165"; // The same aura, and no roll.
    const FENG_SHUI: &str = "meditative-040"; // You have Luck 1.

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Outcome {
        Exiled,
        Radiant,
        Left,
        DrawnBack,
    }

    fn with(seed: &str, field_spell: &str, radiant: bool, beside: &[&str]) -> Scenario {
        let mut field = vec![json!({ "def": field_spell, "lane": 1, "radiant": radiant })];
        field.extend(beside.iter().map(|def| json!({ "def": def })));
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [FILLER, FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [PRESTIGE, FILLER], "library": filler(4) },
        }))
    }

    fn syndicate(seed: &str, radiant: bool) -> Scenario {
        with(seed, ID, radiant, &[])
    }

    fn stockpile(s: &Scenario) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == FILLER)
            .map(|card| card.id)
            .expect("a Stockpile in p1's hand")
    }

    fn outcome(s: &Scenario, id: &str) -> Outcome {
        let card = s.card(id);
        match card.zone {
            Zone::Exile { .. } => Outcome::Exiled,
            Zone::Hand { .. } => Outcome::DrawnBack,
            Zone::Library { .. } if card.radiant => Outcome::Radiant,
            Zone::Library { .. } => Outcome::Left,
            ref other => panic!("a traded card in {other:?}"),
        }
    }

    fn traded(s: &mut Scenario) -> (String, Outcome) {
        let id = stockpile(s);
        s.trade(&id);
        let became = outcome(s, &id);
        (id, became)
    }

    fn outcome_on(seed: &str, radiant: bool) -> Outcome {
        traded(&mut syndicate(seed, radiant)).1
    }

    #[test]
    fn r1864002_cards_in_your_hand_have_tradable_and_only_yours() {
        for radiant in [false, true] {
            let mut s = syndicate("syndicate-lent", radiant);
            let mine = stockpile(&s);
            assert!(is_tradable(s.state(), s.card(&mine)), "radiant {radiant}: lent by the aura");
            let theirs = s
                .hand(P2)
                .into_iter()
                .find(|card| card.def_id == FILLER)
                .expect("p2's Stockpile");
            assert!(!is_tradable(s.state(), &theirs), "radiant {radiant}: not the opponent's hand");
            s.trade(&mine);
            s.expect_events(json!(["traded", "shuffledIn", "drawn"]));
            assert_eq!(trades_left(s.state(), P1), 0, "one Trade a turn");
        }
    }

    #[test]
    fn r1864003_one_roll_exiles_makes_radiant_or_leaves_the_card() {
        for wanted in [Outcome::Exiled, Outcome::Radiant, Outcome::Left] {
            let seed = seed_where(&format!("syndicate-{wanted:?}"), |seed| outcome_on(seed, false) == wanted);
            let mut s = syndicate(&seed, false);
            let hand = s.hand(P1).len();
            let (id, became) = traded(&mut s);
            assert_eq!(became, wanted);
            assert_eq!(s.hand(P1).len(), hand, "{wanted:?}: the draw stands");
            let p2_events = js(&s.view(P2).events).to_string();
            match wanted {
                Outcome::Exiled => assert!(s
                    .last_events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::Exiled { instance_id, .. } if *instance_id == id))),
                // A card made Radiant in a deck stays hidden from the other seat (R97, R1838001).
                Outcome::Radiant => {
                    assert!(s
                        .last_events()
                        .iter()
                        .any(|event| event.event_type() == GameEventType::RadiantSet));
                    assert!(!p2_events.contains(&format!("\"{id}\"")), "{p2_events}");
                }
                Outcome::Left | Outcome::DrawnBack => {
                    assert!(!p2_events.contains(&format!("\"{id}\"")), "{p2_events}");
                }
            }
        }
    }

    #[test]
    fn r1864003_over_200_seeds_about_30_percent_each() {
        let mut counts: IndexMap<&str, usize> = IndexMap::new();
        for i in 0..200 {
            let key = match outcome_on(&format!("syndicate-odds-{i}"), false) {
                Outcome::Exiled => "exiled",
                Outcome::Radiant => "radiant",
                Outcome::Left => "left",
                Outcome::DrawnBack => continue,
            };
            *counts.entry(key).or_default() += 1;
        }
        let rolled: usize = counts.values().sum();
        for key in ["exiled", "radiant"] {
            let share = counts.get(key).copied().unwrap_or(0) as f64 / rolled as f64;
            assert!((0.15..=0.45).contains(&share), "{key}: {share} of {rolled} ({counts:?})");
        }
        assert!(counts.get("left").copied().unwrap_or(0) > 0, "{counts:?}");
    }

    #[test]
    fn r1864003_the_radiant_face_never_leaves_a_card_in_the_deck_as_it_was() {
        let mut seen: Vec<Outcome> = Vec::new();
        for i in 0..40 {
            let became = outcome_on(&format!("syndicate-radiant-{i}"), true);
            assert_ne!(became, Outcome::Left, "seed {i}: 30% exiled, else 70% Radiant");
            if !seen.contains(&became) {
                seen.push(became);
            }
        }
        assert!(seen.contains(&Outcome::Exiled) && seen.contains(&Outcome::Radiant), "{seen:?}");
    }

    fn cursor_after(seed: &str, field_spell: &str, beside: &[&str]) -> u32 {
        let mut s = with(seed, field_spell, false, beside);
        traded(&mut s);
        s.state().rng_cursor
    }

    #[test]
    fn r1864003_r129_a_card_drawn_straight_back_rolls_nothing() {
        // The Trade Union lends the same Tradable and never rolls, so its cursor is the Trade's alone. A
        // traded card is drawn straight back only from a deck it is alone in (R2657003).
        let back = |field_spell: &str| {
            let mut s = scenario(json!({
                "seed": "syndicate-back",
                "p1": { "hand": [FILLER, FILLER], "field": [{ "def": field_spell, "lane": 1 }], "library": [] },
                "p2": { "hand": [PRESTIGE, FILLER], "library": filler(4) },
            }));
            let (_, became) = traded(&mut s);
            assert_eq!(became, Outcome::DrawnBack);
            s.state().rng_cursor
        };
        assert_eq!(back(ID), back(TRADE_UNION), "nothing rolled");
        let kept = seed_where("syndicate-kept", |seed| outcome_on(seed, false) != Outcome::DrawnBack);
        assert_eq!(cursor_after(&kept, ID, &[]), cursor_after(&kept, TRADE_UNION, &[]) + 1, "one roll");
    }

    #[test]
    fn r1864003_r987_luck_rolls_it_no_more_times() {
        let kept = seed_where("syndicate-luck", |seed| {
            traded(&mut with(seed, ID, false, &[FENG_SHUI])).1 != Outcome::DrawnBack
        });
        assert!(luck_of(with(&kept, ID, false, &[FENG_SHUI]).state(), P1) > 0, "p1 has Luck");
        assert_eq!(
            cursor_after(&kept, ID, &[FENG_SHUI]),
            cursor_after(&kept, TRADE_UNION, &[FENG_SHUI]) + 1,
            "one roll, Luck or not"
        );
    }

    #[test]
    fn r1864003_the_opponents_trades_roll_nothing() {
        let their_trade = |field_spell: &str| {
            let mut s = with("syndicate-theirs", field_spell, true, &[]);
            s.end_turn();
            assert_eq!(s.state().active, P2);
            let prestige = s.card(PRESTIGE).id.clone();
            s.trade(&prestige);
            let kinds: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
            (kinds, s.state().rng_cursor)
        };
        let (kinds, cursor) = their_trade(ID);
        assert!(kinds.contains(&GameEventType::Traded));
        assert!(!kinds.contains(&GameEventType::Exiled), "{kinds:?}");
        assert!(!kinds.contains(&GameEventType::RadiantSet), "{kinds:?}");
        assert_eq!(cursor, their_trade(TRADE_UNION).1, "nothing rolled");
    }

    #[test]
    fn r1864003_past_100_radiant_takes_what_is_left() {
        for i in 0..30 {
            let mut s = syndicate(&format!("syndicate-over-{i}"), false);
            let syndicate = s.backrow(P1, 1).expect("The Syndicate").id;
            set_param(s.card_mut(&syndicate), "radiantChance", PERCENT);
            assert_ne!(traded(&mut s).1, Outcome::Left, "seed {i}");
        }
    }

    #[test]
    fn r1864003_the_exile_chance_reads_through_param() {
        for i in 0..20 {
            let mut s = syndicate(&format!("syndicate-param-{i}"), false);
            let syndicate = s.backrow(P1, 1).expect("The Syndicate").id;
            set_param(s.card_mut(&syndicate), "exileChance", PERCENT);
            let became = traded(&mut s).1;
            assert!(
                matches!(became, Outcome::Exiled | Outcome::DrawnBack),
                "seed {i}: {became:?}"
            );
        }
    }
}
