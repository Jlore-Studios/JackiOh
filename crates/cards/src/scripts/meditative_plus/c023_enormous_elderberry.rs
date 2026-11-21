//! Meditative+ #23 Enormous Elderberry (SPEC §8.9 row 23, BUILD M11 row M+ 23, R4350000, R1178001):
//! (2, Embiggened 4) Spell, Fruit, Epic.
//!   Base:    "Add {cards} random embiggen card to your hand. Their Embiggened version costs no extra
//!            mana. / Embiggened (4): Add {embiggenedCards} instead." (1; 3)
//!   Radiant: the same, the cards Radiant.
//!
//! `add_random_from_catalog` over the embiggen pool (R1178001: the non-token cards of the sets that
//! ship that print two prices), never this card (R387), repeats allowed (R60), the count the
//! Embiggened version's where it was played (R81, `ctx.embiggened`). Each card added has no Embiggen
//! premium (R4350000, `noEmbiggenPremium`): its Embiggened version costs its normal price, read live,
//! so every modifier (R65) moves both prices alike, and under Mountaintop Garden its dearer choice
//! costs the normal price too (R1181002); the normal version keeps its price. A full hand burns the
//! rest (§2.4, R4), and R766 takes the mark off a card that reaches a graveyard or an exile pile.
//! Through v0.6.8d each carried a price for its Embiggened version alone instead, `EMBIGGENED_COST`
//! (R1181003, `embiggenCostOverride`), the script `KEPT_SCRIPTS` keeps (`script_v0_6_8d`). Tunes: cards
//! 1 ↑; embiggenedCards 3 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-023";

/// R1181003: v0.6.8d's "Their Embiggened version costs (0)", the kept script's price.
const EMBIGGENED_COST: i32 = 0;

/// The Cry: each card added with no Embiggen premium when `no_premium` (R4350000), else with v0.6.8d's
/// (0) for its Embiggened version (R1181003).
fn elderberry(radiant: bool, no_premium: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = param(&*ctx, if ctx.embiggened { "embiggenedCards" } else { "cards" });
            vec![add_random_from_catalog(AddRandomFromCatalogArgs {
                query: Some(excluding_def_id(None, &crate::query::embiggen_cards(), Some(ID))),
                count: Some(count),
                radiant: radiant.then_some(true),
                no_embiggen_premium: no_premium.then_some(true),
                embiggen_cost_override: (!no_premium).then_some(EMBIGGENED_COST),
                ..AddRandomFromCatalogArgs::default()
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: elderberry(false, true),
        radiant: elderberry(true, true),
    }
}

/// R767000, R1181003: each card added priced its Embiggened version at (0), through v0.6.8d (issue
/// #4350); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_8d() -> CardScripts {
    CardScripts {
        base: elderberry(false, false),
        radiant: elderberry(true, false),
    }
}

// Meditative+ #23 Enormous Elderberry — SPEC §8.9 row 23, BUILD M11 row M+ 23 (R4350000, R1181003).
#[cfg(test)]
mod tests {
    use super::{ID, script_v0_6_8d};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const FILLER: &str = "core-008";

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

    fn added(s: &Scenario) -> Vec<CardInstance> {
        s.hand(P1)
            .into_iter()
            .filter(|card| card.def_id != FILLER)
            .collect()
    }

    fn shown(s: &Scenario, id: &str) -> CardView {
        match s.view(P1).you.hand {
            HandView::Cards(cards) => cards
                .into_iter()
                .find(|card| card.instance_id == id)
                .expect("p1 sees its own hand card"),
            HandView::Count { .. } => panic!("p1's own hand is shown as cards"),
        }
    }

    fn pool() -> Vec<String> {
        crate::query::pool(ID, &crate::query::embiggen_cards())
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// The card's printed prices, `(base, embiggen)`.
    fn printed(card: &CardInstance) -> (i32, i32) {
        let CardCost::Embiggen { base, embiggen } = crate::card_def(&card.def_id).cost else {
            panic!("{} prints two prices", card.def_id);
        };
        (base, embiggen)
    }

    /// R4350000: each card added is an embiggen card with no Embiggen premium, both its prices its
    /// normal one.
    fn assert_priced(s: &Scenario, cards: &[CardInstance]) {
        let pool = pool();
        for card in cards {
            assert!(pool.contains(&card.def_id), "{} is an embiggen card", card.def_id);
            assert_ne!(card.def_id, ID, "never itself (R387)");
            assert_eq!(card.no_embiggen_premium, Some(true));
            assert_eq!(card.embiggen_cost_override, None, "no (0) since #4350");
            assert_eq!(card.cost_override, None, "the normal version keeps its price");
            let view = shown(s, &card.id);
            let (base, _) = printed(card);
            assert_eq!(view.cost, base, "{}: its normal price", card.def_id);
            assert_eq!(view.embiggen_cost, Some(base), "{}: its Embiggened version", card.def_id);
        }
    }

    /// A game whose cast of the normal version added a card printing a normal price above 0, so the
    /// Embiggened price shows apart from (0), and the card.
    fn priced_pick(radiant: bool) -> (Scenario, CardInstance) {
        for n in 0..20 {
            let mut s = game(&format!("m+023-priced-{n}"), radiant, 1);
            cast(&mut s, false);
            let card = added(&s).remove(0);
            if printed(&card).0 > 0 {
                return (s, card);
            }
        }
        panic!("twenty casts added no card printing a normal price above 0");
    }

    /// R4350000: an added card printed `a embiggen b` (b > a) offers its Embiggened version at `a`, not
    /// at (0) nor at `b`, and that play pays `a`.
    fn assert_plays_embiggened_at_its_normal_price(radiant: bool) {
        let (mut s, card) = priced_pick(radiant);
        assert_eq!(card.radiant, radiant);
        let (base, embiggen) = printed(&card);
        assert!(embiggen > base, "{} prints a dearer Embiggened price", card.def_id);
        let view = shown(&s, &card.id);
        assert_eq!((view.cost, view.embiggen_cost), (base, Some(base)), "{}", card.def_id);
        let play = s
            .plays_of(&card)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { embiggen: Some(true), face_down: None, .. }))
            .expect("the Embiggened version is offered");
        s.act(play, P1, "the Embiggened play");
        let paid = s.last_events().iter().find_map(|event| match event {
            GameEvent::CardPlayed { cost_paid, embiggened, .. } => Some((*cost_paid, *embiggened)),
            _ => None,
        });
        assert_eq!(paid, Some((base, Some(true))), "{}: the normal price, Embiggened", card.def_id);
    }

    mod base {
        use super::*;

        #[test]
        fn r4350000_adds_one_embiggen_card_with_no_embiggen_premium() {
            let mut s = game("m+023-one", false, 1);
            cast(&mut s, false);
            let got = added(&s);
            assert_eq!(got.len(), 1);
            assert!(!got[0].radiant);
            assert_priced(&s, &got);
        }

        #[test]
        fn r4350000_both_versions_are_offered() {
            let mut s = game("m+023-offered", false, 1);
            cast(&mut s, false);
            let card = added(&s).remove(0);
            let mut flags: Vec<Option<bool>> = s
                .plays_of(&card)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play { embiggen, face_down: None, .. } => Some(embiggen),
                    _ => None,
                })
                .collect();
            flags.sort();
            flags.dedup();
            assert_eq!(flags, vec![Some(false), Some(true)], "{}", card.def_id);
        }

        #[test]
        fn r4350000_the_embiggened_version_costs_the_normal_price_not_0_nor_the_dearer_one() {
            assert_plays_embiggened_at_its_normal_price(false);
        }

        /// R4350000, R65: the Embiggened price is read live from the normal one, so a discount on the
        /// card or a `costOverride` set after it was added moves both prices alike.
        #[test]
        fn r4350000_a_cost_change_to_the_normal_price_moves_the_embiggened_price() {
            let (mut s, card) = priced_pick(false);
            let (base, _) = printed(&card);
            s.card_mut(card.id.as_str()).cost_mod = -1;
            let view = shown(&s, &card.id);
            assert_eq!((view.cost, view.embiggen_cost), (base - 1, Some(base - 1)), "a discount");
            let held = s.card_mut(card.id.as_str());
            held.cost_mod = 0;
            held.cost_override = Some(base + 3);
            let view = shown(&s, &card.id);
            assert_eq!((view.cost, view.embiggen_cost), (base + 3, Some(base + 3)), "a costOverride");
        }

        /// R1181003, R767000: through v0.6.8d each card added priced its Embiggened version alone at (0),
        /// and a replay of those games folds under that script.
        #[test]
        fn r1181003_r767000_the_kept_v0_6_8d_script_prices_the_embiggened_version_at_0() {
            crate::under_kept(ID, script_v0_6_8d(), || {
                let mut s = game("m+023-kept", false, 1);
                cast(&mut s, true);
                let got = added(&s);
                assert_eq!(got.len(), 3);
                for card in got {
                    assert_eq!(card.embiggen_cost_override, Some(0), "{}", card.def_id);
                    assert_eq!(card.no_embiggen_premium, None, "{}", card.def_id);
                    let view = shown(&s, &card.id);
                    assert_eq!(view.cost, printed(&card).0, "{}: its normal price", card.def_id);
                    assert_eq!(view.embiggen_cost, Some(0), "{}: its Embiggened version", card.def_id);
                }
            });
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
        fn r387_never_itself_over_five_seeds() {
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
        fn r4350000_adds_radiant_embiggen_cards_with_no_embiggen_premium() {
            let mut s = game("m+023-radiant", true, 1);
            cast(&mut s, true);
            let got = added(&s);
            assert_eq!(got.len(), 3);
            assert!(got.iter().all(|card| card.radiant), "each added Radiant");
            assert_priced(&s, &got);
        }

        #[test]
        fn r4350000_radiant_cards_play_embiggened_at_their_normal_price() {
            assert_plays_embiggened_at_its_normal_price(true);
        }
    }
}
