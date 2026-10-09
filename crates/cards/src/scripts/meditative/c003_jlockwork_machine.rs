//! M #3 Jlockwork Machine (SPEC §8.8 row 3, §10.5 step 4, R70, R117, R386): (3) Unit, Rare, 10/10 → 20/20.
//!
//! Base:    "Whenever your opponent plays a card, exile the top {exiles|card|cards} of your deck."
//! Radiant: "Rush\nWhenever your opponent plays a card, exile the top {exiles|card|cards} of your deck."
//! Engine: a trigger on `cardPlayed` by the opponent (§10.5 step 4: a cast is a play, R70; a Trap set
//! face-down is a play too; a countered card was never played, §6.3 Counter), exiling the top cards of
//! its controller's library by C+ #12.6 Frozen Wastes' pattern: `for_each_card` over the library's top
//! (index 0) and `exile` of each. A short deck exiles what it has; an empty one exiles nothing and causes
//! no fatigue (Exile is no draw, §6.3). It answers at step 4, before the played card resolves. Rush is
//! catalog data, so the Radiant face's script is the base's.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-003";

fn after_opponent_plays() -> TriggerDef {
    TriggerDef::new("jlockwork-machine-exile", &[GameEventType::CardPlayed], |ctx, event| {
        let GameEvent::CardPlayed { player, .. } = event else {
            return vec![];
        };
        if *player == ctx.controller {
            return vec![];
        }
        let me = ctx.controller;
        let count = param(&*ctx, "exiles").max(0) as usize;
        vec![for_each_card(ForEachCardArgs {
            cards: Arc::new(move |at: &mut EffectContext<'_>| {
                zone_cards(at.state, me, OffFieldZone::Library)
                    .into_iter()
                    .take(count)
                    .map(|card| card.id)
                    .collect()
            }),
            each: Arc::new(|instance_id: &str| {
                exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
            }),
        })]
    })
}

fn jlockwork_machine() -> Script {
    Script {
        triggers: vec![after_opponent_plays()],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jlockwork_machine(),
        radiant: jlockwork_machine(),
    }
}

// M #3 Jlockwork Machine — SPEC §8.8 row 3, BUILD M10 row M 3: "A trigger on the opponent's `cardPlayed` that
// exiles the top 3 cards of its controller's deck, by Frozen Wastes' pattern: a cast counts (R70) and a
// Trap set face-down is a play; it answers at §10.5 step 4, before the played card resolves (a cast's once
// the effect that cast it has resolved, R117); a countered card exiles nothing; its own controller's plays
// exile nothing; a short deck exiles what it has with no fatigue, an empty one nothing; exiles reads
// through param() (R386); the Radiant face adds Rush".
//
// Fixtures: #5 Stockpile is a (1) Spell and #21 Hinder a (0) Spell cast on draw; #17 Counterspell (classic)
// is a (2) Trap that counters a Spell; #8 Mr. Vanilla is a 4/4. Every game keeps a spare #10 Rapid Replenish
// in the acting side's hand and cards in both libraries, so no turn ends by itself (R82).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008";
    const STOCKPILE: &str = "core-005";
    const HINDER: &str = "core-021";
    const FILLER: &str = "core-010";
    const COUNTERSPELL: &str = "classic-017";
    /// Five different cards, so the top three can be told apart from the rest.
    const DECK: [&str; 5] = ["core-008", "core-061", "core-004", "core-013", "core-019"];

    /// p2 is to act, against p1's Machine and a deck of `DECK`'s first `deck` cards.
    fn board(radiant: bool, deck: usize) -> Scenario {
        crate::scenario(json!({
            "active": "p2",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": DECK[..deck].to_vec(),
            },
            "p2": { "hand": [STOCKPILE, STOCKPILE, FILLER], "library": [VANILLA, VANILLA, VANILLA], "mana": 4 },
        }))
    }

    fn ids(cards: Vec<CardInstance>) -> Vec<String> {
        cards.into_iter().map(|card| card.id).collect()
    }

    /// The position of the first event `matches` accepts, in the events of the last step.
    fn position(s: &Scenario, matches: impl Fn(&GameEvent) -> bool) -> usize {
        match s.last_events().iter().position(matches) {
            Some(at) => at,
            None => panic!("the last step holds no such event: {:?}", s.last_events()),
        }
    }

    fn exiled_events(s: &Scenario, owner: PlayerId) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Exiled { owner: who, .. } if *who == owner))
            .count()
    }

    #[test]
    fn is_a_3_cost_10_10_declaring_exiles() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(3));
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(def.rarity, Rarity::Rare);
        assert!(def.tags.is_empty());
        assert_eq!(
            [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
            [Some(10), Some(10), Some(20), Some(20)]
        );
        assert_eq!(
            crate::js(&def.params),
            json!([{ "key": "exiles", "base": 3, "radiant": 3, "better": "down", "step": 1, "min": 1 }])
        );
    }

    mod base {
        use super::*;

        #[test]
        fn each_opponent_play_exiles_the_top_3_of_your_deck() {
            let mut s = board(false, 5);
            let deck = ids(s.pile(P1, "library"));
            s.play(STOCKPILE, json!({}));
            assert_eq!(ids(s.pile(P1, "exile")).len(), 3);
            assert_eq!(ids(s.pile(P1, "exile")), deck[..3].to_vec());
            assert_eq!(ids(s.pile(P1, "library")), deck[3..].to_vec());
            // The opponent's own deck is not touched by it: only its Stockpile's draw.
            assert!(s.pile(P2, "exile").is_empty());
            // The second play exiles the next two, the deck's last cards.
            s.play(STOCKPILE, json!({}));
            assert_eq!(ids(s.pile(P1, "exile")).len(), 5);
            assert!(s.pile(P1, "library").is_empty());
        }

        #[test]
        fn r70_an_opponents_cast_counts() {
            // Hinder is on top of p2's deck: p1 ends its turn, p2 draws it and it is cast on the draw.
            let mut s = crate::scenario(json!({
                "p1": { "field": [ID], "hand": [FILLER], "library": DECK.to_vec() },
                "p2": { "hand": [FILLER], "library": [HINDER, VANILLA, VANILLA] },
            }));
            s.end_turn();
            assert!(s.events().iter().any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == HINDER)));
            assert_eq!(exiled_events(&s, P1), 3);
            assert_eq!(s.pile(P1, "exile").len(), 3);
        }

        #[test]
        fn a_face_down_trap_the_opponent_sets_counts() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "field": [ID], "hand": [FILLER], "library": DECK.to_vec() },
                "p2": { "hand": [COUNTERSPELL, FILLER], "library": [VANILLA, VANILLA], "mana": 4 },
            }));
            s.play(COUNTERSPELL, json!({}));
            assert!(s.backrow(P2, 1).is_some());
            assert_eq!(s.pile(P1, "exile").len(), 3);
        }

        #[test]
        fn it_answers_before_the_played_card_resolves() {
            let mut s = board(false, 5);
            s.play(STOCKPILE, json!({}));
            let resolved = position(&s, |event| {
                matches!(event, GameEvent::CardResolved { def_id, .. } if def_id == STOCKPILE)
            });
            let exiles: Vec<usize> = s
                .last_events()
                .iter()
                .enumerate()
                .filter(|(_, event)| matches!(event, GameEvent::Exiled { owner, .. } if *owner == P1))
                .map(|(at, _)| at)
                .collect();
            assert_eq!(exiles.len(), 3);
            assert!(exiles.iter().all(|at| *at < resolved), "{:?}", s.last_events());
        }

        #[test]
        fn a_countered_card_exiles_nothing() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": {
                    "field": [ID],
                    "backrow": [{ "def": COUNTERSPELL, "faceUp": false }],
                    "hand": [FILLER],
                    "library": DECK.to_vec(),
                },
                "p2": { "hand": [STOCKPILE, FILLER], "library": [VANILLA, VANILLA], "mana": 4 },
            }));
            s.play(STOCKPILE, json!({}));
            assert!(s.events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            assert_eq!(exiled_events(&s, P1), 0);
            assert_eq!(s.pile(P1, "library").len(), 5);
        }

        #[test]
        fn your_own_plays_exile_nothing() {
            let mut s = crate::scenario(json!({
                "p1": { "field": [ID], "hand": [STOCKPILE, FILLER], "library": DECK.to_vec(), "mana": 4 },
                "p2": { "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            s.play(STOCKPILE, json!({}));
            assert_eq!(exiled_events(&s, P1), 0);
            assert!(s.pile(P1, "exile").is_empty());
        }

        #[test]
        fn a_short_deck_exiles_what_it_has_with_no_fatigue() {
            let mut s = board(false, 2);
            let health = s.state().players.p1.hero.health;
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.pile(P1, "exile").len(), 2);
            assert!(s.pile(P1, "library").is_empty());
            assert_eq!(s.state().players.p1.hero.health, health);
        }

        #[test]
        fn an_empty_deck_exiles_nothing() {
            let mut s = board(false, 0);
            let health = s.state().players.p1.hero.health;
            s.play(STOCKPILE, json!({}));
            assert_eq!(exiled_events(&s, P1), 0);
            assert!(s.pile(P1, "exile").is_empty());
            assert_eq!(s.state().players.p1.hero.health, health);
        }

        #[test]
        fn r386_a_nerf_exiles_4() {
            let mut s = board(false, 5);
            assert_eq!(crate::degrade_number(&mut s, ID, "exiles"), 4);
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.pile(P1, "exile").len(), 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_20_20_with_rush_and_attacks_a_unit_the_turn_it_lands() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER], "library": DECK.to_vec() },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": [VANILLA, VANILLA] },
            }));
            s.play(ID, json!({}));
            s.expect_stats(ID, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));
            let kinds: Vec<String> = s
                .view(P1)
                .you
                .units
                .first()
                .cloned()
                .flatten()
                .map(|unit| unit.keywords.iter().map(|keyword| keyword.kind().to_string()).collect())
                .unwrap_or_default();
            assert_eq!(kinds, vec!["Rush".to_string()]);
            // Rush attacks Units only: the hero is refused, the Vanilla is not.
            s.expect_refused(|s| s.attack(ID, "hero"));
            s.attack(ID, VANILLA);
            assert!(s.unit(P2, 1).is_none());
        }

        #[test]
        fn still_exiles_3() {
            let mut s = board(true, 5);
            s.play(STOCKPILE, json!({}));
            assert_eq!(s.pile(P1, "exile").len(), 3);
        }
    }
}
