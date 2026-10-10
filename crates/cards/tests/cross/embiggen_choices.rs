//! #492: an Embiggen card's two prices are offered exactly when both are affordable (SPEC §2.3, R81),
//! and its owner's view says what the embiggen price comes to.
//!
//! An "A embiggen B" card's price is one of a play's own choices, and R81 puts it in the `play`
//! action that `legalActions` enumerates: every play of the card is listed with `embiggen: false`,
//! and, while B mana is there to pay, listed again with `embiggen: true` and every other choice the
//! same. The client opens its price picker only off that list (CLAUDE.md rule 7), and shows the
//! Embiggened option at the view's `embiggenCost`, which must be what that play then charges: under
//! Professor Curvature's "(4)+ Cost" discount (R363), which reaches B and not A, and AI Alignment
//! Tax's surcharge (R455) too. This is the engine half of the choice;
//! `apps/web/src/game/embiggen-real.test.tsx` is the client's, on the same cards. Every Embiggen card
//! in the catalog is driven, on both faces, so a new one is covered the day it is added.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::P1;

use super::scenario;

/// Every "A embiggen B" card in the catalog: its id, A and B.
fn embiggen_cards() -> Vec<(String, i32, i32)> {
    jackioh_cards::register_all();
    catalog::registered_catalog()
        .values()
        .filter_map(|def| match def.cost {
            CardCost::Embiggen { base, embiggen } => Some((def.id.clone(), base, embiggen)),
            _ => None,
        })
        .collect()
}

/// p1's main phase with the card (on the face asked for) alone in hand and `mana` to spend.
fn holding(def_id: &str, radiant: bool, mana: i32) -> Scenario {
    scenario(json!({
        "p1": { "hand": [{ "def": def_id, "radiant": radiant }], "mana": mana },
    }))
}

/// Professor Curvature: "Cry: (4)+ Cost cards cost (1) less on your next turn."
const CURVATURE: &str = "core-077";

/// AI Alignment Tax: "Your opponent's cards cost (1) more during their next turn."
const ALIGNMENT_TAX: &str = "classicplus-t-ai-07";

/// p1's hand card of `def_id`.
fn held(g: &Scenario, def_id: &str) -> String {
    g.hand(P1)
        .into_iter()
        .find(|card| card.def_id == def_id)
        .map(|card| card.id)
        .unwrap_or_else(|| panic!("p1 holds no {def_id}"))
}

/// The `play`s `legalActions` lists for p1's hand card, as (zone, embiggen) pairs.
fn offered(g: &Scenario, card: &str) -> Vec<(Option<ZoneChoice>, Option<bool>)> {
    legal_actions(g.state(), P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id,
                zone,
                embiggen,
                ..
            } if instance_id == card => Some((zone, embiggen)),
            _ => None,
        })
        .collect()
}

/// The zones the plays at one price name, in the order `legalActions` lists them.
fn zones_at(plays: &[(Option<ZoneChoice>, Option<bool>)], embiggened: bool) -> Vec<Option<ZoneChoice>> {
    plays
        .iter()
        .filter(|(_, price)| *price == Some(embiggened))
        .map(|(zone, _)| *zone)
        .collect()
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// A play of p1's hand card at the price asked for, through `reduce`.
fn play(g: &Scenario, card: &str, zone: Option<ZoneChoice>, embiggen: bool) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let body = ActionBody::Play {
        instance_id: card.to_string(),
        zone,
        x: None,
        embiggen: Some(embiggen),
        tributes: None,
        targets: None,
        modes: None,
        plague: None,
        face_down: None,
    };
    reduce(g.state(), &Action::new(body, P1, format!("embiggen-{nonce}")))
}

/// What the play paid and whether it was embiggened, as its `cardPlayed` says.
fn paid(result: &ReduceResult) -> Option<(i32, Option<bool>)> {
    result.events.iter().find_map(|event| match event {
        GameEvent::CardPlayed {
            cost_paid,
            embiggened,
            ..
        } => Some((*cost_paid, *embiggened)),
        _ => None,
    })
}

#[test]
fn r81_the_catalog_has_embiggen_cards_to_drive() {
    assert!(!embiggen_cards().is_empty());
}

#[test]
fn r81_with_mana_for_the_embiggen_price_every_embiggen_card_is_offered_at_both_prices_for_every_zone() {
    for (id, base, embiggen) in embiggen_cards() {
        for radiant in [false, true] {
            for mana in [embiggen, embiggen + 1] {
                let g = holding(&id, radiant, mana);
                let card = held(&g, &id);
                let plays = offered(&g, &card);
                let normal = zones_at(&plays, false);
                let bigger = zones_at(&plays, true);
                let what = format!("{id} (radiant {radiant}) with {mana} mana");
                assert!(!normal.is_empty(), "{what}: the normal price is offered");
                assert_eq!(
                    normal, bigger,
                    "{what}: the embiggen price is offered for every zone the normal one is"
                );
                assert_eq!(
                    plays.len(),
                    normal.len() + bigger.len(),
                    "{what}: every play names its price"
                );

                // Each form is accepted, and pays its own price.
                let zone = normal[0];
                let small = play(&g, &card, zone, false);
                assert_eq!(small.error, None, "{what}: the normal play");
                assert_eq!(
                    paid(&small),
                    Some((base, Some(false))),
                    "{what}: the normal play pays A"
                );
                let big = play(&g, &card, zone, true);
                assert_eq!(big.error, None, "{what}: the embiggened play");
                assert_eq!(
                    paid(&big),
                    Some((embiggen, Some(true))),
                    "{what}: the embiggened play pays B"
                );
            }
        }
    }
}

#[test]
fn r81_with_mana_for_the_normal_price_only_every_embiggen_card_is_offered_at_that_price_alone() {
    for (id, base, embiggen) in embiggen_cards() {
        for radiant in [false, true] {
            for mana in base..embiggen {
                let g = holding(&id, radiant, mana);
                let card = held(&g, &id);
                let plays = offered(&g, &card);
                let what = format!("{id} (radiant {radiant}) with {mana} mana");
                assert!(!plays.is_empty(), "{what}: the normal price is offered");
                assert!(
                    plays.iter().all(|(_, embiggened)| *embiggened == Some(false)),
                    "{what}: only the normal price is offered, {plays:?}"
                );
                let zone = plays[0].0;
                assert!(
                    play(&g, &card, zone, true).error.is_some(),
                    "{what}: the embiggen price is refused"
                );
            }
        }
    }
}

#[test]
fn r81_without_mana_for_the_normal_price_no_embiggen_card_is_offered() {
    for (id, base, _) in embiggen_cards() {
        for radiant in [false, true] {
            let g = holding(&id, radiant, base - 1);
            assert!(
                offered(&g, &held(&g, &id)).is_empty(),
                "{id} (radiant {radiant}) with {} mana",
                base - 1
            );
        }
    }
}

/// p1's own view of its hand card: (cost, embiggenCost).
fn shown(g: &Scenario, card: &str) -> (i32, Option<i32>) {
    let HandView::Cards(hand) = g.view(P1).you.hand else {
        panic!("p1 reads its own hand");
    };
    let view = hand
        .into_iter()
        .find(|view| view.instance_id == card)
        .expect("p1's view lists the card");
    (view.cost, view.embiggen_cost)
}

/// The view's two prices, and that each is what its play then charges.
fn view_prices_are_charged(g: &Scenario, card: &str, normal: i32, embiggened: i32, what: &str) {
    assert_eq!(
        shown(g, card),
        (normal, Some(embiggened)),
        "{what}: the view's cost and embiggenCost"
    );
    let zone = zones_at(&offered(g, card), true)
        .first()
        .copied()
        .unwrap_or_else(|| panic!("{what}: the embiggen price is offered"));
    assert_eq!(
        paid(&play(g, card, zone, true)),
        Some((embiggened, Some(true))),
        "{what}: the embiggened play charges embiggenCost"
    );
    assert_eq!(
        paid(&play(g, card, zone, false)),
        Some((normal, Some(false))),
        "{what}: the normal play charges cost"
    );
}

/// Ends turns until p1's first turn after the setup turn `from`. R82 may already have ended a turn
/// with nothing left to do in it, so the walk reads whose turn it is rather than counting.
fn to_p1s_next_turn(g: &mut Scenario, from: i32) {
    while g.state().active != P1 || g.state().turn <= from {
        g.end_turn();
    }
}

#[test]
fn r81_every_embiggen_cards_view_shows_its_embiggen_price_as_it_stands_and_that_is_what_the_play_charges() {
    for (id, base, embiggen) in embiggen_cards() {
        for radiant in [false, true] {
            let what = format!("{id} (radiant {radiant}) unchanged");
            let g = holding(&id, radiant, embiggen);
            view_prices_are_charged(&g, &held(&g, &id), base, embiggen, &what);

            // The card's own discount reaches both prices (R65), each floored at (0), so a (0)
            // base price stays (0).
            let g = scenario(json!({
                "p1": { "hand": [{ "def": id, "radiant": radiant, "costMod": -1 }], "mana": embiggen },
            }));
            let what = format!("{id} (radiant {radiant}) at costMod -1");
            view_prices_are_charged(
                &g,
                &held(&g, &id),
                (base - 1).max(0),
                (embiggen - 1).max(0),
                &what,
            );
        }
    }
}

/// R363: Professor Curvature takes (1) off a price that is (4) or more, and leaves a lower one.
fn under_curvature(price: i32) -> i32 {
    if price >= 4 { price - 1 } else { price }
}

#[test]
fn r363_professor_curvature_discounts_the_embiggen_price_and_not_the_base_one_and_the_view_says_so() {
    let cards = embiggen_cards();
    // The case the ruling is about: A below (4) and B at (4)+, so the discount reaches B alone.
    assert!(
        cards
            .iter()
            .any(|(_, base, embiggen)| *base < 4 && *embiggen >= 4),
        "the catalog holds an embiggen card with A below (4) and B at (4)+"
    );
    // Every embiggen card is driven: each price is reached exactly when it is (4)+, so M #81
    // Deadman's Hand's (0) embiggen (2) is reached at neither.
    for (id, base, embiggen) in cards {
        for radiant in [false, true] {
            let mut g = scenario(json!({
                "p1": { "hand": [CURVATURE, { "def": id, "radiant": radiant }] },
            }));
            let from = g.state().turn;
            g.play(CURVATURE, json!({}));
            to_p1s_next_turn(&mut g, from);
            g.state_mut().players.p1.mana.current = embiggen;
            let what = format!("{id} (radiant {radiant}) under Professor Curvature");
            view_prices_are_charged(
                &g,
                &held(&g, &id),
                under_curvature(base),
                under_curvature(embiggen),
                &what,
            );
        }
    }
}

#[test]
fn r455_ai_alignment_tax_raises_both_prices_and_the_view_says_so() {
    for (id, base, embiggen) in embiggen_cards() {
        for radiant in [false, true] {
            let mut g = scenario(json!({
                "active": "p2",
                "p1": { "hand": [{ "def": id, "radiant": radiant }] },
                "p2": { "hand": [ALIGNMENT_TAX] },
            }));
            let from = g.state().turn;
            g.play(ALIGNMENT_TAX, json!({}));
            to_p1s_next_turn(&mut g, from);
            g.state_mut().players.p1.mana.current = embiggen + 1;
            let what = format!("{id} (radiant {radiant}) under AI Alignment Tax");
            view_prices_are_charged(&g, &held(&g, &id), base + 1, embiggen + 1, &what);
        }
    }
}
