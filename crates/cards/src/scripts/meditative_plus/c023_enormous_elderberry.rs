//! Meditative+ #23 Enormous Elderberry (SPEC §8.9 row 23, BUILD M11 row M+ 23, R1181003, R1178001):
//! (2, Embiggened 4) Spell, Fruit, Epic.
//!   Base:    "Add {cards} random embiggen card to your hand. Their Embiggened version costs (0). /
//!            Embiggened (4): Add {embiggenedCards} instead." (1; 3)
//!   Radiant: the same, the cards Radiant.
//!
//! `add_random_from_catalog` over the embiggen pool (R1178001: the non-token cards of the sets that
//! ship that print two prices), never this card (R387), repeats allowed (R60), the count the
//! Embiggened version's where it was played (R81, `ctx.embiggened`). Each card added carries a price
//! for its Embiggened version alone (R1181003, `embiggenCostOverride`): that play costs (0) before
//! every modifier, while the normal version keeps its price; a full hand burns the rest (§2.4, R4),
//! and R766 takes the price off a card that reaches a graveyard. Tunes: cards 1 ↑; embiggenedCards 3 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-023";

/// "Their Embiggened version costs (0)."
const EMBIGGENED_COST: i32 = 0;

fn elderberry(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, if ctx.embiggened { "embiggenedCards" } else { "cards" });
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(excluding_def_id(None, &crate::query::embiggen_cards(), Some(ID))),
                count: Some(count),
                radiant: radiant.then_some(true),
                embiggen_cost_override: Some(EMBIGGENED_COST),
                ..AddRandomFromCatalogArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: elderberry(false),
        radiant: elderberry(true),
    }
}

// Meditative+ #23 Enormous Elderberry — SPEC §8.9 row 23, BUILD M11 row M+ 23: "adds one card of the
// embiggen pool (the shipped eight, never itself) whose Embiggened version costs (0) and whose normal
// version keeps its price, both offered; Embiggened at (4) 3 of them; a full hand burns the rest; both
// counts read through `param()`; radiant the cards are Radiant".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// A (1) 4/4 with no text.
    const FILLER: &str = "core-008";

    /// p1 holds the Elderberry (Radiant on `radiant`) and `fillers` fillers, with 10 mana.
    fn game(seed: &str, radiant: bool, fillers: usize) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(std::iter::repeat_n(json!(FILLER), fillers));
        crate::scenario(json!({
            "seed": seed,
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER], "mana": 10 },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn cast(s: &mut Scenario, embiggen: bool) {
        s.play(ID, if embiggen { json!({ "embiggen": true }) } else { json!({}) });
    }

    /// The cards the Cry added: p1's hand past the fillers it kept.
    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    /// The card as p1's view shows it.
    fn shown(s: &Scenario, id: &str) -> CardView {
        match s.view(P1).you.hand {
            HandView::Cards(cards) => cards
                .into_iter()
                .find(|card| card.instance_id == id)
                .expect("p1 sees its own hand card"),
            HandView::Count { .. } => panic!("p1's own hand is shown as cards"),
        }
    }

    /// The embiggen pool of the sets that ship (R1178001), less this card.
    fn pool() -> Vec<String> {
        crate::query::pool(ID, &crate::query::embiggen_cards())
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// Each added card: of the pool, its normal price printed and its Embiggened version at (0).
    fn assert_priced(s: &Scenario, cards: &[CardInstance]) {
        let pool = pool();
        for card in cards {
            assert!(pool.contains(&card.def_id), "{} is an embiggen card", card.def_id);
            assert_ne!(card.def_id, ID, "never itself (R387)");
            assert_eq!(card.embiggen_cost_override, Some(0));
            assert_eq!(card.cost_override, None, "the normal version keeps its price");
            let view = shown(s, &card.id);
            let CardCost::Embiggen { base, .. } = crate::card_def(&card.def_id).cost else {
                panic!("{} prints two prices", card.def_id);
            };
            assert_eq!(view.cost, base, "{}: its normal price", card.def_id);
            assert_eq!(view.embiggen_cost, Some(0), "{}: its Embiggened version", card.def_id);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r1181003_adds_one_embiggen_card_whose_embiggened_version_costs_0() {
            let mut s = game("m+023-one", false, 1);
            cast(&mut s, false);
            let got = added(&s);
            assert_eq!(got.len(), 1);
            assert!(!got[0].radiant);
            assert_priced(&s, &got);
        }

        #[test]
        fn r1181003_both_versions_are_offered() {
            let mut s = game("m+023-offered", false, 1);
            cast(&mut s, false);
            let card = added(&s).remove(0);
            let mut flags: Vec<Option<bool>> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play {
                        instance_id,
                        embiggen,
                        face_down: None,
                        ..
                    } if instance_id == card.id => Some(embiggen),
                    _ => None,
                })
                .collect();
            flags.sort();
            flags.dedup();
            assert_eq!(flags, vec![Some(false), Some(true)], "{}", card.def_id);
        }

        #[test]
        fn embiggened_it_adds_3() {
            let mut s = game("m+023-three", false, 1);
            cast(&mut s, true);
            let got = added(&s);
            assert_eq!(got.len(), 3);
            assert_priced(&s, &got);
        }

        #[test]
        fn r4_a_full_hand_burns_the_rest() {
            let mut s = game("m+023-full", false, 8);
            cast(&mut s, true);
            assert_eq!(s.hand(P1).len(), 10, "the hand of 8 holds two of the three");
            assert_eq!(added(&s).len(), 2);
            let burned = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Burned { .. }))
                .count();
            assert_eq!(burned, 1);
        }

        #[test]
        fn r387_never_itself_with_its_set_previewed() {
            let _preview = preview_sets(&[SetName::MeditativePlus]);
            for seed in 0..5 {
                let mut s = game(&format!("m+023-never-itself-{seed}"), false, 1);
                cast(&mut s, true);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                assert!(got.iter().all(|card| card.def_id != ID), "never Enormous Elderberry");
            }
        }

        #[test]
        fn r386_an_upgrade_adds_one_more() {
            let mut s = game("m+023-tuned", false, 1);
            let berry = s.hand(P1)[0].id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &berry, "cards"), 2);
            assert_eq!(crate::upgrade_number(&mut s, &berry, "embiggenedCards"), 4);
            cast(&mut s, false);
            assert_eq!(added(&s).len(), 2);
        }

        #[test]
        fn r386_played_embiggened_an_upgrade_adds_one_more() {
            let mut s = game("m+023-tuned-embiggened", false, 1);
            let berry = s.hand(P1)[0].id.clone();
            assert_eq!(crate::upgrade_number(&mut s, &berry, "embiggenedCards"), 4);
            cast(&mut s, true);
            let got = added(&s);
            assert_eq!(got.len(), 4);
            assert_priced(&s, &got);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn adds_radiant_embiggen_cards() {
            let mut s = game("m+023-radiant", true, 1);
            cast(&mut s, true);
            let got = added(&s);
            assert_eq!(got.len(), 3);
            assert!(got.iter().all(|card| card.radiant), "each added Radiant");
            assert_priced(&s, &got);
        }
    }
}
