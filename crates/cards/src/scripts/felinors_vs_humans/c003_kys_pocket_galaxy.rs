//! SPEC §8.14 FvH #3 KY's Pocket Galaxy ((3) Spell, KY, Legendary), a card of the Felinors vs Humans set
//! (R5235000).
//!   Base:    "Every card in your hand and deck now costs ({setCost})." (2)
//!   Radiant: "Every card in your hand and deck now costs ({setCost}). Replenish your mana."
//!
//! M+ #3 Wondrous Wand's shape: `for_each_card` over the caster's hand and deck as the Spell resolves, a
//! `set_cost_override` of the declared `setCost` on each, so a price change after it still moves it
//! (R65) and an Embiggened price follows the normal one (R5109000); a card that reaches the hand or deck
//! later keeps its own. It is how a Travertine Stack (FvH #16.1, (9999)) is played. Radiant: then §6.3
//! Replenish (R364), every spent crystal back up to max mana, as C+ #27 Zephrys Zealotism's.

use jackioh_engine::effects::{ForEachCardArgs, for_each_card, refresh_mana, set_cost_override};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-003";

/// "Replenish your mana": every spent crystal, capped at max mana (R364). Half of `i32::MAX`, as the
/// Replenish adds this to current mana before capping and the sum must stay in range (C+ #27's).
const ALL_MANA: i32 = i32::MAX / 2;

/// The caster's hand and deck, by id, as the Spell resolves (the Spell itself is resolving, in neither).
fn hand_and_deck(ctx: &mut EffectContext<'_>) -> Vec<String> {
    [OffFieldZone::Hand, OffFieldZone::Library]
        .into_iter()
        .flat_map(|zone| zone_cards(&*ctx.state, ctx.controller, zone))
        .map(|card| card.id)
        .collect()
}

/// One face; `replenish` adds the Radiant face's last sentence.
fn face(replenish: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let cost = param(&*ctx, "setCost");
            let mut effects = vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(hand_and_deck),
                each: Arc::new(move |instance_id: &str| {
                    set_cost_override(json_as(json!({
                        "target": { "of": "instance", "instanceId": instance_id },
                        "cost": cost,
                    })))
                }),
            })];
            if replenish {
                effects.push(refresh_mana(json_as(json!({ "amount": ALL_MANA }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #3 KY's Pocket Galaxy — SPEC §8.14 row 3 (R65, R364, R1748003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const DEAR: &str = "stand-in (5) 5/5";
    const CHEAP: &str = "stand-in (0) Spell";
    /// FvH #16.1 Travertine Stack: (9999), played only at a lowered price.
    const STACK: &str = "felinorsvshumans-016-1";

    fn price(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), s.card(card), CostOptions::default())
    }

    /// p1 holds the Galaxy (`radiant` its face), DEAR and CHEAP, with DEAR and CHEAP in the deck; p2 holds
    /// a DEAR too.
    fn galaxy(seed: &str, radiant: bool) -> Scenario {
        weapon_board(
            seed,
            json!({ "hand": [{ "def": ID, "radiant": radiant }, DEAR, CHEAP], "library": [DEAR, CHEAP] }),
            json!({ "hand": [DEAR] }),
        )
    }

    /// Every card of p1's hand and deck.
    fn mine(s: &Scenario) -> Vec<CardInstance> {
        let mut cards = s.hand(P1);
        cards.extend(s.pile(P1, "library"));
        cards
    }

    mod base {
        use super::*;

        #[test]
        fn every_card_in_your_hand_and_deck_now_costs_2() {
            let mut s = galaxy("fvh003-base", false);
            s.play(ID, json!({}));
            assert_eq!(mine(&s).len(), 4);
            for card in nonempty(mine(&s), "p1's hand and deck") {
                assert_eq!(card.cost_override, Some(2), "{}", card.def_id);
                assert_eq!(price(&s, &card), 2, "{}", card.def_id);
            }
            // No Replenish on the base face: 4 less the (3).
            s.expect_mana(P1, 1);
        }

        #[test]
        fn the_opponents_cards_keep_their_prices() {
            let mut s = galaxy("fvh003-theirs", false);
            s.play(ID, json!({}));
            for card in nonempty(s.hand(P2), "p2's hand") {
                assert_eq!(card.cost_override, None);
                assert_eq!(price(&s, &card), 5);
            }
        }

        #[test]
        fn r65_a_cost_mod_still_moves_the_new_price() {
            let mut s = weapon_board(
                "fvh003-mod",
                json!({ "hand": [ID, { "def": DEAR, "costMod": -1 }] }),
                json!({ "hand": [DEAR] }),
            );
            s.play(ID, json!({}));
            let held = s.card(DEAR).clone();
            assert_eq!(held.cost_override, Some(2));
            assert_eq!(price(&s, &held), 1);
        }

        #[test]
        fn it_is_how_a_travertine_stack_is_played() {
            let mut s = weapon_board(
                "fvh003-stack",
                json!({ "hand": [ID, STACK], "mana": 10 }),
                json!({ "hand": [DEAR] }),
            );
            assert!(!s.can_play(STACK));
            s.play(ID, json!({}));
            let stack = s.card(STACK).clone();
            assert_eq!(price(&s, &stack), 2);
            assert!(s.can_play(STACK));
        }

        #[test]
        fn r1748003_set_cost_moves_with_a_buff_or_a_nerf() {
            let mut nerfed = galaxy("fvh003-nerf", false);
            assert_eq!(crate::degrade_number(&mut nerfed, ID, "setCost"), 3);
            nerfed.play(ID, json!({}));
            assert_eq!(nerfed.card(DEAR).cost_override, Some(3));

            let mut buffed = galaxy("fvh003-buff", false);
            assert_eq!(crate::upgrade_number(&mut buffed, ID, "setCost"), 1);
            buffed.play(ID, json!({}));
            assert_eq!(buffed.card(DEAR).cost_override, Some(1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r364_every_card_costs_2_and_your_mana_is_replenished() {
            let mut s = galaxy("fvh003-radiant", true);
            s.play(ID, json!({}));
            for card in nonempty(mine(&s), "p1's hand and deck") {
                assert_eq!(price(&s, &card), 2, "{}", card.def_id);
            }
            // The (3) paid, then every crystal back, up to max mana.
            s.expect_mana(P1, MAX_MANA);
        }
    }
}
