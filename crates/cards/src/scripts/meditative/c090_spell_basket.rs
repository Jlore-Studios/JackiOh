//! M #90 Spell Basket (SPEC §8.8 row 90): (1) Spell, Common.
//!
//! Base:    "Add {cards|random Spell|random Spells} to your hand."
//! Radiant: "Add {cards|random Radiant Spell|random Radiant Spells} to your hand."
//! Engine: `add_random_from_catalog` over `{ type: "Spell" }`: the Spell type, never a Field Spell, as
//! C+ #38.1's random Spells are; non-token Spells of every set that ships (R380, R1420), never this
//! card (R387), repeats allowed (R60), each on its base face at its printed cost; the Radiant face's
//! are Radiant (§5.2). The hand cap burns the rest (§2.4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-090";

fn spell_basket(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "type": "Spell" },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spell_basket(false),
        radiant: spell_basket(true),
    }
}

// M #90 Spell Basket — SPEC §8.8 row 90, BUILD M10 row M 90: "Adds three random non-token Spells (the
// Spell type only, never a Field Spell) of any set, never itself, at their printed costs, hidden; the
// hand cap burns the rest; cards reads through `param()`; radiant they are Radiant".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FILLER: &str = "core-016";

    fn holding(seed: &str, radiant: bool, others: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend((0..others).map(|_| json!(FILLER)));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    fn added(s: &Scenario, kept: usize) -> Vec<CardInstance> {
        s.hand(P1).into_iter().skip(kept).collect()
    }

    mod m90_spell_basket {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn adds_three_spells_never_field_spells_or_itself() {
                let _preview = preview_sets(&[SetName::Meditative]);
                for n in 0..8 {
                    let mut s = holding(&format!("spell-basket-{n}"), false, 1);
                    s.play(ID, json!({}));
                    let added = added(&s, 1);
                    assert_eq!(added.len(), 3);
                    for card in &added {
                        let def = crate::card_def(&card.def_id);
                        assert_eq!(def.type_, CardType::Spell, "{}", card.def_id);
                        assert!(!def.token, "{}", card.def_id);
                        assert_ne!(card.def_id, ID);
                        assert!(!card.radiant);
                        // At its printed cost.
                        assert_eq!((card.cost_mod, card.cost_override), (0, None));
                    }
                }
            }

            #[test]
            fn hidden_from_the_opponent() {
                let mut s = holding("spell-basket-hidden", false, 1);
                s.play(ID, json!({}));
                let added: Vec<String> = added(&s, 1).into_iter().map(|card| card.id).collect();
                let theirs = s.view(P2);
                assert_eq!(theirs.opponent.hand, HandView::Count { count: 4 });
                for event in theirs.events.iter().filter(|event| event.event_type() == GameEventType::AddedToHand) {
                    let event = crate::js(event);
                    assert!(!added.iter().any(|id| event["instanceId"] == json!(id)));
                }
            }

            #[test]
            fn s2_4_the_cap_burns_the_rest() {
                let mut s = holding("spell-basket-cap", false, 9);
                s.play(ID, json!({}));
                assert_eq!(s.hand(P1).len() as i32, HAND_CAP);
                let burned = s.events().iter().filter(|event| event.event_type() == GameEventType::Burned).count();
                assert_eq!(burned, 2);
            }

            #[test]
            fn cards_reads_through_param() {
                let mut s = holding("spell-basket-param", false, 1);
                set_param(s.card_mut(ID), "cards", 5);
                s.play(ID, json!({}));
                assert_eq!(added(&s, 1).len(), 5);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_spells_radiant() {
                let mut s = holding("spell-basket-radiant", true, 1);
                s.play(ID, json!({}));
                let added = added(&s, 1);
                assert_eq!(added.len(), 3);
                for card in &added {
                    assert!(card.radiant);
                    assert_eq!(crate::card_def(&card.def_id).type_, CardType::Spell);
                }
            }
        }
    }
}
