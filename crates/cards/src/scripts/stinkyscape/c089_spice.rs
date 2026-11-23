//! SPEC §8.10 Stinkyscape 89 Spice, a card of the Stinkyscape set.
//!   Base:    "Gives Credits like a Resource when played or discarded.\nDiscover a Stinkyscape card."
//!   Radiant: "Gives Credits like a Resource when played or discarded.\nDiscover a Radiant Stinkyscape
//!            card."
//!
//! It is no Resource (`paysCredits`, R4410000): played or discarded from its owner's hand, it pays its
//! quote in Credits as a Resource does (R2250001), the Marketplace's this turn while it is open (quoted
//! after the Resources', R1853002), else one draw in `HAND_SALE_PRICE`, 5–15, Radiant 9–29 (since issue
//! #4440's "~15% more", R4440001; 4–13 and 8–25 from issue #4154's "~25% more", R4154206; 3–10 and 6–20
//! before, a Rare Resource's), shown in braces on it in
//! hand while the Marketplace is open (R3047004); a cast by an effect pays nothing. Until issue #4410 it
//! sold from the hand instead (R1861000). Played, it Discovers (§6.3) a non-token Stinkyscape card: the
//! pool names the set, so R1420's gate would not empty it were the set held (R380), and the Discover
//! never offers the card that made it (R387), so never Spice. The pick goes to the hand (§2.4's cap
//! burning it), Radiant on the Radiant face, M #88 The True Sheep's shape. The Credits come first, right
//! after `cardPlayed`, and the Discover after them (§10.5 step 4).

use jackioh_engine::effects::{add_to_hand, discover_from_catalog};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-089";

/// The one resume step: the Discover's answer comes back here (§10.6).
const PICKED: &str = "spice";

fn face(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            pays_credits: Some(true),
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

// S 89 Spice — SPEC §8.10 row 89, BUILD M12 row S 89 (R4410000).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::{cast, discard_random, gain_market_credits};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, run_as, scenario};

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";
    const OPENING_CREDITS: i32 = 10;
    /// A Rare Resource, whose range Spice's was until issue #4154 (R1861000).
    const GELLIUM: &str = "stinkyscape-t-gellium"; // Gellium

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

    /// P1 gains `OPENING_CREDITS`, which opens the Marketplace (R1853000).
    fn open_marketplace(s: &mut Scenario) {
        run_as(
            s,
            gain_market_credits(json_as(json!({ "amount": OPENING_CREDITS }))),
            P1,
        );
    }

    /// The braces on Spice in P1's hand (R3047004).
    fn braces_on_spice(s: &Scenario) -> Option<Vec<PreviewValue>> {
        let id = spice_id(s);
        match s.view(P1).you.hand {
            HandView::Cards(cards) => cards
                .into_iter()
                .find(|card| card.instance_id == id)
                .expect("Spice in the view")
                .preview,
            HandView::Count { .. } => panic!("the owner reads their hand"),
        }
    }

    fn in_range(price: ResourcePrice, credits: i32) -> bool {
        (price.sell_min..=price.sell_max).contains(&credits)
    }

    /// Where the first event of `kind` sits among the last action's events.
    fn position_of(s: &Scenario, kind: GameEventType) -> usize {
        let kinds: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
        kinds
            .iter()
            .position(|each| *each == kind)
            .unwrap_or_else(|| panic!("no {kind:?} in {kinds:?}"))
    }

    /// The open prompt is shown to its caster and offers only cards (every option a Mode).
    fn assert_discover_of_cards(s: &Scenario) {
        let pending = s.state().pending.as_ref().expect("the Discover is open");
        assert_eq!(pending.player_id, P1, "shown to its caster");
        assert_eq!(s.prompt_modes().len(), pending.options.len(), "a Discover offers cards");
    }

    #[test]
    fn discovers_among_three_stinkyscape_cards_never_spice() {
        for seed in (0..10).map(|n| format!("spice-discover-{n}")) {
            let mut s = spice(&seed, false);
            s.play(ID, json!({}));
            assert_discover_of_cards(&s);
            let options = s.prompt_modes();
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
    fn r4410000_r1861000_played_with_the_marketplace_open_it_pays_its_quote_then_discovers() {
        let mut s = spice("spice-play-open", false);
        open_marketplace(&mut s);
        let shown = braces_on_spice(&s).expect("its quote in braces");
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].label, SELL_QUOTE_LABEL);
        let quote = shown[0].value;
        assert!(in_range(HAND_SALE_PRICE, quote), "{quote}");
        s.play(ID, json!({}));
        assert_eq!(market_credits_of(s.state(), P1), OPENING_CREDITS + quote);
        assert!(
            position_of(&s, GameEventType::CardPlayed) < position_of(&s, GameEventType::MarketCreditsChanged),
            "paid as the play is made"
        );
        assert_discover_of_cards(&s);
    }

    #[test]
    fn r4410000_closed_its_play_pays_a_draw_in_range() {
        for seed in (0..5).map(|n| format!("spice-play-closed-{n}")) {
            let mut s = spice(&seed, false);
            assert!(s.state().marketplace.is_none(), "{seed}");
            assert_eq!(braces_on_spice(&s), None, "{seed}: no quote while closed");
            s.play(ID, json!({}));
            let paid = market_credits_of(s.state(), P1);
            assert!(in_range(HAND_SALE_PRICE, paid), "{seed}: {paid}");
            assert!(s.state().marketplace.is_some(), "{seed}: the Credits opened it");
            assert_discover_of_cards(&s);
        }
    }

    #[test]
    fn r4410000_discarded_from_the_hand_it_pays_its_quote() {
        let mut s = spice("spice-discard", false);
        open_marketplace(&mut s);
        let quote = braces_on_spice(&s).expect("its quote")[0].value;
        let id = spice_id(&s);
        s.state_mut().players[P1].hand.retain(|card| card.def_id == ID);
        let events = run_as(&mut s, discard_random(json_as(json!({ "count": 1 }))), P1);
        assert!(events.iter().any(|event| matches!(
            event,
            GameEvent::Discarded { instance_id, .. } if *instance_id == id
        )));
        assert_eq!(market_credits_of(s.state(), P1), OPENING_CREDITS + quote);
        assert!(s.pile(P1, "graveyard").iter().any(|card| card.id == id));
    }

    #[test]
    fn r4410000_radiant_it_pays_the_radiant_quote() {
        // Open: the Radiant face's quote, in its range.
        let mut s = spice("spice-radiant-open", true);
        open_marketplace(&mut s);
        let quote = braces_on_spice(&s).expect("its quote")[0].value;
        assert!(in_range(HAND_SALE_RADIANT_PRICE, quote), "{quote}");
        s.play(ID, json!({}));
        assert_eq!(market_credits_of(s.state(), P1), OPENING_CREDITS + quote);
        // Closed: a draw in the Radiant range.
        for seed in (0..5).map(|n| format!("spice-radiant-closed-{n}")) {
            let mut s = spice(&seed, true);
            s.play(ID, json!({}));
            let paid = market_credits_of(s.state(), P1);
            assert!(in_range(HAND_SALE_RADIANT_PRICE, paid), "{seed}: {paid}");
        }
    }

    #[test]
    fn r4410000_cast_by_an_effect_it_pays_nothing() {
        let mut s = spice("spice-cast", false);
        open_marketplace(&mut s);
        let id = spice_id(&s);
        let events = run_as(
            &mut s,
            cast(json_as(
                json!({ "target": { "of": "instance", "instanceId": id } }),
            )),
            P1,
        );
        assert!(
            events
                .iter()
                .any(|event| event.event_type() == GameEventType::CardPlayed),
            "it was cast"
        );
        assert_eq!(market_credits_of(s.state(), P1), OPENING_CREDITS, "a cast pays nothing");
    }

    #[test]
    fn r4410000_r1853003_it_is_never_sold_and_is_no_resource() {
        let mut s = spice("spice-no-sale", false);
        open_marketplace(&mut s);
        let id = spice_id(&s);
        assert!(sales_listed(&s).is_empty(), "no sale is listed");
        s.expect_refused_with(
            |s| {
                s.act(
                    ActionBody::MarketSell {
                        instance_id: id.clone(),
                    },
                    P1,
                    "sell the Spice",
                )
            },
            NOTHING_SELLS,
        );
        let card = s.hand(P1).into_iter().find(|card| card.id == id).expect("still in hand");
        assert!(!is_resource_card(s.state(), &card), "no Resource");
        assert!(!resource_def_ids().contains(&ID.to_string()), "no Resource roll gives it");
        let def = card_def(ID);
        assert_eq!(def.cost, CardCost::Fixed(1));
        assert_eq!(def.rarity, Rarity::Rare);
    }

    /// `percent` of `credits`, rounded to the nearest Credit, a half up, as R4154206 and R4440001 round.
    fn raised(credits: i32, percent: i32) -> i32 {
        (credits * percent + 50) / 100
    }

    /// R4154206, R4440001: issue #4154's "~25% more" took a Rare Resource's 3 to 10 (Radiant 6 to 20) to 4
    /// to 13 (8 to 25), each end a quarter higher, rounded to the nearest Credit, a half up; issue #4440's
    /// "~15% more" raises each end of that by 15%, rounded the same way: 5 to 15 (Radiant 9 to 29).
    #[test]
    fn r4440001_r4154206_it_pays_15_percent_more_than_the_quarter_above_a_rare_resource() {
        let gellium = card_def(GELLIUM).resource.expect("Gellium's prices");
        let ends = |price: ResourcePrice| [price.sell_min, price.sell_max];
        let quarter = |price: ResourcePrice| ends(price).map(|end| raised(end, 125));
        assert_eq!(quarter(gellium.base), [4, 13]);
        assert_eq!(quarter(gellium.radiant), [8, 25]);
        assert_eq!(ends(HAND_SALE_PRICE), quarter(gellium.base).map(|end| raised(end, 115)));
        assert_eq!(ends(HAND_SALE_RADIANT_PRICE), quarter(gellium.radiant).map(|end| raised(end, 115)));
        assert_eq!(ends(HAND_SALE_PRICE), [5, 15]);
        assert_eq!(ends(HAND_SALE_RADIANT_PRICE), [9, 29]);
    }

    #[test]
    fn radiant_the_pick_is_radiant() {
        let mut s = spice("spice-radiant", true);
        s.play(ID, json!({}));
        assert_discover_of_cards(&s);
        let options = s.prompt_modes();
        s.answer(json!(options[0].clone()));
        let pick = s.hand(P1).last().cloned().expect("the pick");
        assert_eq!(pick.def_id, options[0]);
        assert!(pick.radiant);
    }
}
