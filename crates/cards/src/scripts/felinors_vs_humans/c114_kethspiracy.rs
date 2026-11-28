//! SPEC §8.14 FvH #114 Kethspiracy ((1) Spell, Legendary), a card of the Felinors vs Humans set
//! (R5235000).
//!   Base:    "Give every simple Human and Felinor card on your field, in your hand and in your deck
//!            Starship Piece."
//!   Radiant: "Give every simple Human and Felinor card on your field, in your hand and in your deck
//!            Starship Piece. They cost ({discount}) less." (2)
//!
//! "Simple" is `is_simple` (R5239009): a script of at most `SIMPLE_CODE_LINES` (10) code lines, R1846000's
//! count, never the catalog's `loc`; a fused or crafted card is not simple. Every card of its caster's on
//! the field, in the hand and in the deck whose tags now (`tags_of`) hold Human or Felinor, that is simple
//! and not a Starship Piece already, and that is a permanent card (a Unit, Field Spell, Trap or Field
//! Trap; a Spell, which never dies on the field, is passed by) (R5261002): `grant_keyword` Starship Piece,
//! which a hand or deck card keeps onto the field (R1878003); a Unit Immune to Spells is passed by (§6.1).
//! Radiant: `set_cost_mod` of −`discount` on the same cards, which rides the card (R766), so a field
//! card's discount shows once it is back in a hand. The printed 10 stays off the card, as BUSTED's and
//! complicated's do. Built by S11 (#5261).

use jackioh_engine::effects::{cards_in_card_scope, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-114";

/// R5261002: the card types a Starship Piece can be: the permanents, whose death on the field fuses it.
const PIECE_TYPES: [CardType; 4] =
    [CardType::Unit, CardType::FieldSpell, CardType::Trap, CardType::FieldTrap];

/// The tribes the text names.
const TRIBES: [Tag; 2] = [Tag::Human, Tag::Felinor];

/// R5261002: the caster's cards the Spell reaches, by id, field first, then hand, then deck.
fn simple_tribesmen(ctx: &EffectContext<'_>) -> Vec<String> {
    let scope = CardScope {
        side: Some(ScopeSide::SelfSide),
        zones: vec![CardZone::Field, CardZone::Hand, CardZone::Library],
        types: Some(PIECE_TYPES.to_vec()),
        ..CardScope::default()
    };
    cards_in_card_scope(ctx, &scope, None)
        .into_iter()
        .filter(|scoped| scoped.matches)
        .map(|scoped| scoped.card)
        .filter(|card| tags_of(ctx.state, card).iter().any(|tag| TRIBES.contains(tag)))
        .filter(|card| is_simple(&card.def_id) && !is_starship_piece(ctx.state, card))
        .map(|card| card.id)
        .collect()
}

fn kethspiracy(discounted: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let discount = if discounted { param(&*ctx, "discount") } else { 0 };
            let mut effects = Vec::new();
            for id in simple_tribesmen(ctx) {
                let target = TargetSpec::Instance { instance_id: id };
                effects.push(grant_keyword(GrantKeywordArgs {
                    target: target.clone(),
                    keyword: Keyword::StarshipPiece,
                }));
                if discounted {
                    effects.push(set_cost_mod(SetCostModArgs {
                        target: Some(target),
                        amount: -discount,
                        ..SetCostModArgs::default()
                    }));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: kethspiracy(false),
        radiant: kethspiracy(true),
    }
}

// FvH #114 Kethspiracy — SPEC §8.14 row 114 (R5261002, R5239009).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const KEEPER: &str = "stand-in (0) 1/1"; // a play always open, so no turn ends by itself
    const VANILLA: &str = "core-008"; // Mr. Vanilla: a Human
    const POINTMASTER: &str = "core-020"; // Pointmaster: a Human
    const FANATIC: &str = "core-050"; // K-Pop Fanatic: a Human
    const FELINORS: &str = "core-012"; // Duplicating Felinors: a Felinor
    const STOCKPILE: &str = "core-005"; // Stockpile: a Spell, no tribe
    const PLAIN: &str = "stand-in (1) 2/2"; // no tribe

    /// R1846000's counts this test reads in place of the compiled ones: the tribesmen simple but K-Pop
    /// Fanatic, and the plain cards simple too, so only their missing tribe keeps them out.
    fn code_lines() {
        register_code_lines(vec![
            (VANILLA, 7),
            (POINTMASTER, SIMPLE_CODE_LINES),
            (FANATIC, SIMPLE_CODE_LINES + 1),
            (FELINORS, 5),
            (STOCKPILE, 5),
            (PLAIN, 3),
        ]);
    }

    /// Kethspiracy in p1's hand; a Human on the field, simple and not; a Felinor, a Human and plain
    /// cards in hand and deck; the opponent's Human.
    fn board(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, POINTMASTER, FELINORS, FANATIC, PLAIN, KEEPER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": FANATIC, "lane": 2 }],
                "library": [VANILLA, PLAIN, STOCKPILE, POINTMASTER],
            },
            "p2": { "hand": [KEEPER], "field": [VANILLA], "library": crate::filler(4) },
        }));
        code_lines();
        s.play(ID, json!({}));
        s
    }

    /// Given Starship Piece, read off the card and through the layers.
    fn is_piece(s: &Scenario, card: &CardInstance) -> bool {
        let live = s.card(card);
        live.granted_keywords.contains(&Keyword::StarshipPiece) || is_starship_piece(s.state(), live)
    }

    /// p1's cards of `def_id` in `zone` (a pile, or "field").
    fn p1_cards(s: &Scenario, zone: &str, def_id: &str) -> Vec<CardInstance> {
        let cards: Vec<CardInstance> = if zone == "field" {
            (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).collect()
        } else {
            s.pile(P1, zone)
        };
        cards.into_iter().filter(|card| card.def_id == def_id).collect()
    }

    /// Every simple Human and Felinor permanent of p1's is a piece, wherever it is; nothing else is.
    fn gives_starship_piece(radiant: bool) {
        let s = board(radiant);
        let reached = [
            ("field", VANILLA),
            ("hand", POINTMASTER),
            ("hand", FELINORS),
            ("library", VANILLA),
            ("library", POINTMASTER),
        ];
        for (zone, def_id) in reached {
            for card in nonempty(p1_cards(&s, zone, def_id), def_id) {
                assert!(is_piece(&s, &card), "{def_id} in the {zone} is a Starship Piece");
            }
        }
        let passed = [
            ("field", FANATIC),
            ("hand", FANATIC),
            ("hand", PLAIN),
            ("library", PLAIN),
            ("library", STOCKPILE),
        ];
        for (zone, def_id) in passed {
            for card in nonempty(p1_cards(&s, zone, def_id), def_id) {
                assert!(!is_piece(&s, &card), "{def_id} in the {zone} is passed by");
            }
        }
        let theirs = s.unit_at(P2, 1);
        assert!(!is_piece(&s, &theirs), "the opponent's Human is not the caster's");
    }

    mod base {
        use super::*;

        #[test]
        fn r5261002_every_simple_human_and_felinor_card_of_yours_gains_starship_piece() {
            gives_starship_piece(false);
        }

        #[test]
        fn r5261002_their_costs_stay_as_they_were() {
            let s = board(false);
            let pointmaster = must(p1_cards(&s, "hand", POINTMASTER).pop(), "Pointmaster in hand");
            let printed = query_cost(def_of(Some(s.state()), POINTMASTER));
            assert_eq!(effective_cost(s.state(), &pointmaster, Default::default()), printed);
        }

        #[test]
        fn r5239009_a_card_past_simple_code_lines_is_not_simple() {
            let s = board(false);
            for card in nonempty(p1_cards(&s, "hand", FANATIC), "K-Pop Fanatic in hand") {
                assert!(!is_simple(&card.def_id));
                assert!(!is_piece(&s, &card));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5261002_every_simple_human_and_felinor_card_of_yours_gains_starship_piece() {
            gives_starship_piece(true);
        }

        #[test]
        fn r5261002_they_cost_2_less_and_the_rest_cost_as_they_did() {
            let s = board(true);
            let discount = must(param_decl_of(s.state(), ID, "discount"), "the declared discount").radiant;
            for (def_id, discounted) in [(POINTMASTER, true), (FELINORS, true), (FANATIC, false)] {
                for card in nonempty(p1_cards(&s, "hand", def_id), def_id) {
                    let printed = query_cost(def_of(Some(s.state()), def_id));
                    let expected = if discounted { (printed - discount).max(0) } else { printed };
                    assert_eq!(effective_cost(s.state(), &card, Default::default()), expected, "{def_id}");
                }
            }
        }
    }
}
