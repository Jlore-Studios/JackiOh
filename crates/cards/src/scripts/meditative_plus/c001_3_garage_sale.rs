//! Meditative+ #1.3 Garage Sale (SPEC §8.9 row 1.3, §7, R1178002, R1563003, R11, §2.4). (1) Spell, AI,
//! Token: an AI generated card M+ #1 Bob's Plan offers after M #20 Aestheticize the Game (#1563).
//!   Base:    "Fill your hand with Coins, then discard your hand."
//!   Radiant: "Do this {rounds|time|times}: fill your hand with Coins, then discard your hand."
//!
//! M #20's graveyard line (100 cards, Radiant 90) wants cards in the graveyard. Each round adds as many
//! Coins as the hand has room for (its owner's hand size, §6.3, less the cards held; this Spell is
//! resolving, out of the hand), then discards the whole hand, the Coins with it: a Coin is a Spell token,
//! so it reaches the graveyard as any discarded card does (R11 spares only a unit-token card). The
//! numbers are read as the hook runs, so a second round fills a hand the first emptied.

use jackioh_engine::effects::discard_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-001-3";

/// The Coin.
const COIN: &str = "core-t-coin";

fn garage_sale() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let cap = hand_cap_of(ctx.state, ctx.controller);
            let mut held = zone_count(ctx.state, ctx.controller, OffFieldZone::Hand);
            let mut effects = Vec::new();
            for _ in 0..param(&*ctx, "rounds").max(1) {
                effects.extend((held..cap).map(|_| add_to_hand(json_as(json!({ "defId": COIN })))));
                effects.push(discard_hand(json_as(json!({ "player": "self" }))));
                held = 0;
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = garage_sale();
    // The Radiant face's extra round is its declared `rounds`: the same script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Meditative+ #1.3 Garage Sale — SPEC §8.9 row 1.3, BUILD M11 row M+ 1.3: "fills its caster's hand with
// Coins up to their hand size, then discards the hand, every card of it to the graveyard; a hand at its
// size gets no Coin and is discarded; radiant two rounds, its number read through `param()` (R749)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const FILLER: &str = "core-005";

    fn sale(radiant: bool, hand_extra: usize) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(std::iter::repeat_n(json!(FILLER), hand_extra));
        scenario(json!({
            "seed": "garage-sale",
            "p1": { "hand": hand, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER, FILLER] },
        }))
    }

    fn graveyard(s: &Scenario, def: &str) -> usize {
        s.pile(P1, "graveyard").iter().filter(|card| card.def_id == def).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r1563003_fills_the_hand_with_coins_then_discards_it_all_to_the_graveyard() {
            let mut s = sale(false, 3);
            let cap = hand_cap_of(s.state(), P1) as usize;
            s.play(ID, json!({}));
            assert!(s.hand(P1).is_empty());
            assert_eq!(graveyard(&s, COIN), cap - 3, "a Coin for each free place in the hand");
            assert_eq!(graveyard(&s, FILLER), 3);
            assert_eq!(graveyard(&s, ID), 1);
        }

        #[test]
        fn r1563003_a_hand_at_its_size_takes_no_coin_and_is_discarded() {
            crate::register_all();
            let cap = HAND_CAP as usize;
            let mut s = sale(false, cap - 1);
            // Garage Sale leaves the hand as it resolves, so one more card fills it again.
            let extra = s.card(FILLER).clone();
            s.state_mut().players.p1.hand.push(extra);
            s.play(ID, json!({}));
            assert_eq!(graveyard(&s, COIN), 0);
            assert!(s.hand(P1).is_empty());
        }

    }

    mod radiant {
        use super::*;

        #[test]
        fn r1563003_two_rounds_the_second_fills_the_emptied_hand() {
            let mut s = sale(true, 3);
            let cap = hand_cap_of(s.state(), P1) as usize;
            s.play(ID, json!({}));
            assert!(s.hand(P1).is_empty());
            assert_eq!(graveyard(&s, COIN), (cap - 3) + cap);
            assert_eq!(graveyard(&s, FILLER), 3);
        }

        #[test]
        fn r749_r386_rounds_is_the_radiant_face_s_number_and_reads_through_param() {
            let mut s = sale(true, 0);
            let cap = hand_cap_of(s.state(), P1) as usize;
            step_param(s.card_mut(ID), "rounds", 1);
            s.play(ID, json!({}));
            assert_eq!(graveyard(&s, COIN), 3 * cap);
        }
    }
}
