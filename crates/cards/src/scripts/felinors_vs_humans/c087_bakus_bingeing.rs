//! SPEC §8.14 FvH #87 Baku's Bingeing ((3) Spell, Epic), a card of the held Felinors vs Humans set. Built
//! by S10 (#5260).
//!   Base:    "Draw every (1) Cost card from your deck. Exile every even Cost card in your deck, hand and
//!            graveyard, except X Cost cards. Gain {mana} mana." (mana 1)
//!   Radiant: the same text. (mana 5)
//!
//! Core #94 Genn's Greed's shape with (1), even and its own mana, in the text's order: every (1) Cost card
//! of the deck drawn, top down, X Cost cards never (costs read now, R65, R66; the hand cap burns the rest,
//! R4); then every even Cost card of the deck, hand and graveyard exiled, X Cost cards excepted (the
//! designer's "X or adaptive" read as X Cost), a (0) being even, in R135's order, so a (1) just drawn is
//! spared; then `gain_mana` of the declared `mana`, this turn's, which may pass the max (§2.3). Tunes:
//! mana 1 ↑ (Radiant 5); the (1) and the parity are prices (R1748003).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-087";

/// The one cost the draw clause names.
const DRAWN_COST: i32 = 1;
/// The parity the exile clause names, as `exile_matching` spells it.
const EXILED_PARITY: &str = "even";

/// R66: every card of the deck that costs (1) now, X Cost cards never, top down, by instance id.
fn drawn_cards(ctx: &mut EffectContext<'_>) -> Vec<String> {
    zone_cards(ctx.state, ctx.controller, OffFieldZone::Library)
        .iter()
        .filter(|card| {
            !is_x_cost(ctx.state, card) && effective_cost(ctx.state, card, Default::default()) == DRAWN_COST
        })
        .map(|card| card.id.clone())
        .collect()
}

fn bingeing() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let mana = param(&*ctx, "mana");
            vec![
                // One draw per card, read once as the clause begins (R66, R113).
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(drawn_cards),
                    each: Arc::new(|instance_id: &str| {
                        draw_from_library(json_as(json!({ "instanceId": instance_id })))
                    }),
                }),
                // R135: deck, then hand, then graveyard, after the draws.
                exile_matching(json_as(json!({ "parity": EXILED_PARITY, "exemptXCost": true }))),
                gain_mana(json_as(json!({ "amount": mana }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The faces differ only in the declared number: catalog data.
    let base = bingeing();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #87 Baku's Bingeing — SPEC §8.14 row 87 (R4, R65, R66, R135, R386, R1748003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{def_ids, weapon_board};

    const P1: PlayerId = PlayerId::P1;

    /// Odd and (1): what the draw clause takes.
    const ONE: &str = "stand-in (1) 1/1";
    /// Even: what the exile clause takes, a (0) among them.
    const ZERO: &str = "stand-in (0) Spell";
    const TWO: &str = "stand-in (2) 2/2";
    const FOUR: &str = "stand-in (4) Spell";
    /// Odd and not (1): neither clause touches it.
    const THREE: &str = "stand-in (3) 3/3";
    /// X Cost, spared by both clauses (R66).
    const X_COST: &str = "core-074"; // Adaptive UI

    /// The mana before the play, its cost and the gain, base and Radiant.
    const MANA: i32 = 10;
    const COST: i32 = 3;
    const GAIN: i32 = 1;
    const RADIANT_GAIN: i32 = 5;

    /// p1's four off-field piles, by def id, each sorted.
    struct Piles {
        hand: Vec<String>,
        library: Vec<String>,
        graveyard: Vec<String>,
        exile: Vec<String>,
    }

    fn piles(s: &Scenario) -> Piles {
        let sorted = |zone: &str| {
            let mut ids = def_ids(&s.pile(P1, zone));
            ids.sort();
            ids
        };
        Piles {
            hand: sorted("hand"),
            library: sorted("library"),
            graveyard: sorted("graveyard"),
            exile: sorted("exile"),
        }
    }

    fn sorted(ids: &[&str]) -> Vec<String> {
        let mut ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        ids.sort();
        ids
    }

    /// An odd, an even, an X and (in the deck) two (1)s in each pile.
    fn binged(radiant: bool) -> Scenario {
        let mut s = weapon_board(
            "fvh087",
            json!({
                "hand": [{ "def": ID, "radiant": radiant }, TWO, THREE, X_COST],
                "library": [THREE, ONE, FOUR, ONE, X_COST, ZERO],
                "graveyard": [FOUR, THREE, ONE],
                "mana": MANA,
            }),
            json!({}),
        );
        s.play(ID, json!({}));
        s
    }

    fn binges(radiant: bool, gain: i32) {
        let mut s = binged(radiant);
        let piles = piles(&s);
        // Both (1)s drawn, and spared as odd once in hand.
        assert_eq!(piles.hand, sorted(&[ONE, ONE, THREE, X_COST]));
        assert_eq!(piles.library, sorted(&[THREE, X_COST]));
        // The Spell itself is odd, so its graveyard keeps it.
        assert_eq!(piles.graveyard, sorted(&[ID, ONE, THREE]));
        assert_eq!(piles.exile, sorted(&[FOUR, FOUR, TWO, ZERO]));
        s.expect_mana(P1, MANA - COST + gain);
    }

    mod base {
        use super::*;

        #[test]
        fn r66_draws_every_1_cost_card_exiles_every_even_one_but_x_and_gains_1_mana() {
            binges(false, GAIN);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r66_draws_every_1_cost_card_exiles_every_even_one_but_x_and_gains_5_mana() {
            binges(true, RADIANT_GAIN);
        }
    }
}
