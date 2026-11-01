//! M #90 Spell Basket (SPEC §8.8 row 90): (1) Spell, Common.
//!
//! Base:    "Add {cards|random Spell|random Spells} to your hand. Give each Temporary {temporary}."
//! Radiant: "Add {cards|random Radiant Spell|random Radiant Spells} to your hand. Give each Temporary
//!          {temporary}."
//! Engine: `add_random_from_catalog` over `{ type: "Spell" }`: the Spell type, never a Field Spell, as
//! C+ #38.1's random Spells are; non-token Spells of every set that ships (R380, R1420), never this
//! card (R387), repeats allowed (R60), each on its base face at its printed cost; the Radiant face's
//! are Radiant (§5.2). The hand cap burns the rest (§2.4). Since #2657 each takes Temporary
//! {temporary} (R2179001) as it reaches the hand, C+ #23 Dropshipping's way, so a card the cap burns
//! takes none; through v0.5.9e they took nothing (`script_v0_5_9e`, kept, R767000).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-090";

/// One face; `temporary` gives each added Spell its Temporary {temporary} (R2179001), as it has since
/// #2657.
fn spell_basket(radiant: bool, temporary: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut add = json!({
                "query": { "type": "Spell" },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
            });
            if temporary {
                add["keywords"] = json!([{ "kind": "Temporary", "n": param(&*ctx, "temporary") }]);
            }
            vec![add_random_from_catalog(json_as(add))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: spell_basket(false, true),
        radiant: spell_basket(true, true),
    }
}

/// R767000: through v0.5.9e the Spells it added were not Temporary; `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_5_9e() -> CardScripts {
    CardScripts {
        base: spell_basket(false, false),
        radiant: spell_basket(true, false),
    }
}

// M #90 Spell Basket — SPEC §8.8 row 90, BUILD M10 row M 90: "Adds three random non-token Spells (the
// Spell type only, never a Field Spell) of any set, never itself, at their printed costs, hidden, each
// Temporary 2; the hand cap burns the rest; cards and temporary read through `param()`; radiant they are
// Radiant".
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

    /// The Temporary count each added card carries, as its keywords show it (R2179001).
    fn temporaries(s: &Scenario, kept: usize) -> Vec<Option<i32>> {
        added(s, kept)
            .iter()
            .map(|card| {
                card_keywords(s.state(), card).iter().find_map(|keyword| match keyword {
                    Keyword::Temporary { n } => Some(n.unwrap_or(1)),
                    _ => None,
                })
            })
            .collect()
    }

    mod m90_spell_basket {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn adds_three_spells_never_field_spells_or_itself() {
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
            fn r2179001_each_added_spell_is_temporary_2_and_leaves_after_two_of_your_ends_of_turn() {
                let mut s = holding("spell-basket-temporary", false, 1);
                s.play(ID, json!({}));
                assert_eq!(temporaries(&s, 1), vec![Some(2); 3]);
                let ids: Vec<String> = added(&s, 1).into_iter().map(|card| card.id).collect();
                s.end_turn();
                s.end_turn();
                assert!(ids.iter().all(|id| s.hand(P1).iter().any(|card| card.id == *id)), "one left");
                s.end_turn();
                assert!(!ids.iter().any(|id| s.hand(P1).iter().any(|card| card.id == *id)), "discarded");
            }

            #[test]
            fn temporary_reads_through_param() {
                let mut s = holding("spell-basket-temporary-param", false, 1);
                set_param(s.card_mut(ID), "temporary", 4);
                s.play(ID, json!({}));
                assert_eq!(temporaries(&s, 1), vec![Some(4); 3]);
            }

            #[test]
            fn r767000_the_script_kept_for_v0_5_9e_adds_them_with_no_temporary() {
                crate::under_kept(ID, super::super::super::script_v0_5_9e(), || {
                    let mut s = holding("spell-basket-kept", false, 1);
                    s.play(ID, json!({}));
                    assert_eq!(temporaries(&s, 1), vec![None; 3]);
                });
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
