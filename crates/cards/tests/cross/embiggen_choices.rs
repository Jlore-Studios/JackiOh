//! #492: an Embiggen card's two prices are offered exactly when both are affordable (SPEC §2.3, R81).
//!
//! An "A embiggen B" card's price is one of a play's own choices, and R81 puts it in the `play`
//! action that `legalActions` enumerates: every play of the card is listed with `embiggen: false`,
//! and, while B mana is there to pay, listed again with `embiggen: true` and every other choice the
//! same. The client opens its price picker only off that list (CLAUDE.md rule 7), so this is the
//! engine half of the choice; `apps/web/src/game/embiggen-real.test.tsx` is the client's, on the same
//! cards. Every Embiggen card in the catalog is driven, on both faces, so a new one is covered the day
//! it is added.

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

/// The `play`s `legalActions` lists for p1's one hand card, as (zone, embiggen) pairs.
fn offered(g: &Scenario) -> Vec<(Option<ZoneChoice>, Option<bool>)> {
    let card = g.hand(P1)[0].id.clone();
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

/// A play of p1's one hand card at the price asked for, through `reduce`.
fn play(g: &Scenario, zone: Option<ZoneChoice>, embiggen: bool) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let body = ActionBody::Play {
        instance_id: g.hand(P1)[0].id.clone(),
        zone,
        x: None,
        embiggen: Some(embiggen),
        tributes: None,
        targets: None,
        modes: None,
        plague: None,
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
                let plays = offered(&g);
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
                let small = play(&g, zone, false);
                assert_eq!(small.error, None, "{what}: the normal play");
                assert_eq!(
                    paid(&small),
                    Some((base, Some(false))),
                    "{what}: the normal play pays A"
                );
                let big = play(&g, zone, true);
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
                let plays = offered(&g);
                let what = format!("{id} (radiant {radiant}) with {mana} mana");
                assert!(!plays.is_empty(), "{what}: the normal price is offered");
                assert!(
                    plays.iter().all(|(_, embiggened)| *embiggened == Some(false)),
                    "{what}: only the normal price is offered, {plays:?}"
                );
                let zone = plays[0].0;
                assert!(
                    play(&g, zone, true).error.is_some(),
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
                offered(&g).is_empty(),
                "{id} (radiant {radiant}) with {} mana",
                base - 1
            );
        }
    }
}
