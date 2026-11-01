//! SPEC §8.10 Stinkyscape 89 Spice, a card of the Stinkyscape set.
//!   Base:    "Can be sold from your hand at the Marketplace, like a Resource.\nDiscover a Stinkyscape
//!            card."
//!   Radiant: "Can be sold from your hand at the Marketplace, like a Resource.\nDiscover a Radiant
//!            Stinkyscape card."
//!
//! Marketplace is open (R1853000), its owner may sell it from their hand in their main phase (R1853003),
//! at a quote drawn each turn after the Resources' (`HAND_SALE_PRICE`, 3–10,
//! Radiant 6–20); the public `marketSold` names it, it ceases to exist and its Credits are gained
//! (R1840005). Played, it Discovers (§6.3) a non-token Stinkyscape card: the pool names the set, so R1420's
//! gate would not empty it were the set held (R380), and the Discover never offers the card that made it
//! (R387), so never Spice. The pick goes to the hand (§2.4's cap burning it), Radiant on the Radiant
//! face, M #88 The True Sheep's shape.

use jackioh_engine::effects::{add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-089";

/// The one resume step: the Discover's answer comes back here (§10.6).
const PICKED: &str = "spice";

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            hand_sale: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |_ctx| {
            let discover: DiscoverFromCatalogArgs = json_as(json!({
                "step": PICKED,
                "query": { "set": "Stinkyscape" },
                "prompt": if radiant {
                    "Discover a Radiant Stinkyscape card"
                } else {
                    "Discover a Stinkyscape card"
                },
            }));
            vec![discover_from_catalog(discover)]
        })),
        resume: IndexMap::from([(
            PICKED,
            // A Discover's pick arrives as a `mode` selection carrying a catalog id (§10.6).
            hook(move |ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![add_to_hand(json_as(
                    json!({ "defId": picked, "radiant": radiant }),
                ))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S 89 Spice — SPEC §8.10 row 89, BUILD M12 row S 89 (R1861000).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::gain_market_credits;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const FILLER: &str = "core-005";
    const OPENING_CREDITS: i32 = 10;

    fn spice(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn spice_id(s: &Scenario) -> String {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == ID)
            .expect("Spice in hand")
            .id
    }

    fn sales_listed(s: &Scenario) -> Vec<String> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|body| match body {
                ActionBody::MarketSell { instance_id } => Some(instance_id),
                _ => None,
            })
            .collect()
    }

    fn offered(s: &Scenario) -> Vec<String> {
        let pending = s.state().pending.as_ref().expect("the Discover is open");
        assert_eq!(pending.player_id, P1, "shown to its caster");
        pending
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Mode { option } => option.clone(),
                other => panic!("a Discover offers cards: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn discovers_among_three_stinkyscape_cards_never_spice() {
        for seed in (0..10).map(|n| format!("spice-discover-{n}")) {
            let mut s = spice(&seed, false);
            s.play(ID, json!({}));
            let options = offered(&s);
            assert_eq!(options.len(), 3, "{seed}");
            for id in &options {
                let def = card_def(id);
                assert_eq!(def.set, SetName::Stinkyscape, "{seed}: {id}");
                assert!(!def.token, "{seed}: {id} is no token");
                assert_ne!(id, ID, "{seed}: never Spice");
            }
            s.answer(json!(options[0].clone()));
            let hand = s.hand(P1);
            assert_eq!(
                hand.last().map(|card| card.def_id.clone()),
                Some(options[0].clone())
            );
            assert!(!hand.last().expect("the pick").radiant);
        }
    }

    #[test]
    fn r1861000_open_marketplace_sells_it_from_the_hand() {
        let mut s = spice("spice-sale", false);
        run_as(
            &mut s,
            gain_market_credits(json_as(json!({ "amount": OPENING_CREDITS }))),
            P1,
        );
        let id = spice_id(&s);
        assert_eq!(
            sales_listed(&s),
            vec![id.clone()],
            "the Spice in hand is on offer"
        );
        let sale = s
            .view(P1)
            .marketplace
            .expect("open")
            .sales
            .into_iter()
            .find(|sale| sale.instance_id == id)
            .expect("shown to its owner");
        assert!((HAND_SALE_PRICE.sell_min..=HAND_SALE_PRICE.sell_max).contains(&sale.credits));
        assert!(
            s.view(P2).marketplace.expect("open").sales.is_empty(),
            "the opponent's view lists none"
        );
        s.act(
            ActionBody::MarketSell {
                instance_id: id.clone(),
            },
            P1,
            "sell the Spice",
        );
        assert!(s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::MarketSold { instance_id, credits, .. } if *instance_id == id && *credits == sale.credits
        )));
        assert_eq!(market_credits_of(s.state(), P1), OPENING_CREDITS + sale.credits);
        assert!(
            s.hand(P1).iter().all(|card| card.def_id != ID),
            "gone from the hand"
        );
        assert!(
            s.pile(P1, "graveyard").iter().all(|card| card.def_id != ID),
            "it ceased to exist"
        );
    }

    #[test]
    fn r1861000_closed_it_is_not_listed() {
        let mut s = spice("spice-closed", false);
        let id = spice_id(&s);
        assert!(sales_listed(&s).is_empty());
        assert!(s.view(P1).marketplace.is_none());
        s.expect_refused(|s| {
            s.act(
                ActionBody::MarketSell {
                    instance_id: id.clone(),
                },
                P1,
                "sell",
            )
        });
    }

    #[test]
    fn radiant_the_pick_is_radiant() {
        let mut s = spice("spice-radiant", true);
        s.play(ID, json!({}));
        let options = offered(&s);
        s.answer(json!(options[0].clone()));
        let pick = s.hand(P1).last().cloned().expect("the pick");
        assert_eq!(pick.def_id, options[0]);
        assert!(pick.radiant);
    }

    #[test]
    fn radiant_sells_at_the_radiant_quote() {
        let mut s = spice("spice-radiant-sale", true);
        run_as(
            &mut s,
            gain_market_credits(json_as(json!({ "amount": OPENING_CREDITS }))),
            P1,
        );
        let id = spice_id(&s);
        let sale = s
            .view(P1)
            .marketplace
            .expect("open")
            .sales
            .into_iter()
            .find(|sale| sale.instance_id == id)
            .expect("shown");
        assert!(
            (HAND_SALE_RADIANT_PRICE.sell_min..=HAND_SALE_RADIANT_PRICE.sell_max).contains(&sale.credits)
        );
    }
}
