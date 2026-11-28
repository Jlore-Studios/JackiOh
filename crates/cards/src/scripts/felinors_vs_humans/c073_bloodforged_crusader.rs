//! FvH #73 Bloodforged Crusader (SPEC §8.14 row 73): (1) Unit, Human, Quickdraw, Mythic, 3/2 → 6/4.
//! Built by S08 (#5258).
//!   Base:    "Cry: Exile every non-Human Unit on your board and in your hand, deck and graveyard. For the
//!            rest of the game, your Humans have +{stats}/+{stats}. Shuffle {humans} random Human Units
//!            into your deck." — stats 2, humans 5
//!   Radiant: the same with +{stats}/+{stats} 4 and {humans} random Radiant Human Units.
//!
//! Quickdraw is the tag (R225, R640). The Cry, in the text's order: one `exile` per non-Human Unit (the
//! type it has now, tags now with granted ones, R923) on its controller's field and in their hand, deck
//! and graveyard, read as the Cry runs (S #163 Swear Loyalities' loop; a Dormant card is not reached,
//! R1839000); then `give_units_stats` with tags Human, a never-expiring player modifier on its
//! controller's Human Units on the field, itself and later arrivals included, which outlasts the
//! Crusader (R5237007); then `shuffle_random_from_catalog` of `humans` non-token Human Units of the
//! sets that ship (R5235000), no repeat until the pool runs out (R3563000), Radiant on the Radiant
//! face, turned away at the deck cap (R80).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-073";

/// "Your hand, deck and graveyard", after the board.
const PILES: [OffFieldZone; 3] = [OffFieldZone::Hand, OffFieldZone::Library, OffFieldZone::Graveyard];

fn non_human_unit(state: &GameState, card: &CardInstance) -> bool {
    card_type_of(state, card) == CardType::Unit && !tags_of(state, card).contains(&Tag::Human)
}

fn crusade(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let stats = param(ctx, "stats");
            let humans = param(ctx, "humans");
            let me = ctx.controller;
            let state: &GameState = &*ctx.state;
            let mut doomed: Vec<String> = active_units_of(state, me)
                .into_iter()
                .filter(|card| non_human_unit(state, card))
                .map(|card| card.id.clone())
                .collect();
            for pile in PILES {
                doomed.extend(
                    zone_cards(state, me, pile)
                        .into_iter()
                        .filter(|card| non_human_unit(state, card))
                        .map(|card| card.id),
                );
            }
            let mut effects: Vec<Effect> = doomed
                .iter()
                .map(|id| exile(json_as(json!({ "target": { "of": "instance", "instanceId": id } }))))
                .collect();
            effects.push(give_units_stats(GiveUnitsStatsArgs {
                player: None,
                attack: stats,
                health: stats,
                tags: Some(vec![Tag::Human]),
                expiry: ModifierExpiry::Never,
            }));
            effects.push(shuffle_random_from_catalog(json_as(json!({
                "query": { "type": "Unit", "tags": ["Human"] },
                "count": humans,
                "radiant": radiant,
            }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: crusade(false),
        radiant: crusade(true),
    }
}

// FvH #73 Bloodforged Crusader — SPEC §8.14 row 73.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human
    const OTHER: &str = "stand-in (1) 3/3";
    const SPELL: &str = "stand-in (0) Spell";
    const KILL: &str = "core-016"; // Hit Job: the removal
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    /// The Crusader in p1's hand, a non-Human Unit in each of p1's places and a Human on p1's field
    /// (lane 2) and in p1's hand and deck; p2 holds a non-Human Unit and a Human on the field.
    fn crusade_game(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, OTHER, HUMAN, SPELL, KILL, FILLER],
                "field": [OTHER, HUMAN],
                "library": [OTHER, HUMAN],
                "graveyard": [OTHER],
            },
            "p2": { "hand": [OTHER, FILLER], "field": [OTHER, HUMAN] },
        }))
    }

    /// p1's non-Human Units, wherever they are.
    fn others_of_p1(s: &Scenario) -> Vec<String> {
        let mut out = vec![s.unit_at(P1, 1).id];
        for pile in ["hand", "library", "graveyard"] {
            out.extend(s.pile(P1, pile).into_iter().filter(|card| card.def_id == OTHER).map(|card| card.id));
        }
        out
    }

    fn is_human_unit(s: &Scenario, card: &CardInstance) -> bool {
        crate::card_def(&card.def_id).type_ == CardType::Unit
            && tags_of(s.state(), card).contains(&Tag::Human)
    }

    mod base {
        use super::*;

        #[test]
        fn exiles_every_non_human_unit_of_yours_on_the_board_and_in_hand_deck_and_graveyard() {
            let mut s = crusade_game(false);
            let others = others_of_p1(&s);
            assert_eq!(others.len(), 4);
            s.play(ID, json!({}));
            for id in &others {
                s.expect_in_zone(id.as_str(), "exile");
            }
            assert!(s.hand(P1).iter().any(|card| card.def_id == SPELL), "a Spell is no Unit");
            assert!(s.hand(P1).iter().any(|card| card.def_id == HUMAN));
        }

        #[test]
        fn the_opponents_cards_are_untouched() {
            let mut s = crusade_game(false);
            let theirs = s.unit_at(P2, 1).id;
            s.play(ID, json!({}));
            s.expect_in_zone(theirs.as_str(), "field");
            assert!(s.hand(P2).iter().any(|card| card.def_id == OTHER));
        }

        #[test]
        fn your_humans_have_2_2_itself_included_and_the_opponents_do_not() {
            let mut s = crusade_game(false);
            let mine = s.unit_at(P1, 2).id;
            let theirs = s.unit_at(P2, 2).id;
            let (before, their_before) = (s.stats(mine.as_str()), s.stats(theirs.as_str()));
            s.play(ID, json!({}));
            let after = s.stats(mine.as_str());
            assert_eq!((after.attack, after.max_health), (before.attack + 2, before.max_health + 2));
            let theirs_after = s.stats(theirs.as_str());
            assert_eq!(
                (theirs_after.attack, theirs_after.max_health),
                (their_before.attack, their_before.max_health)
            );
            let crusader = s.stats(ID);
            let printed = crate::card_def(ID);
            assert_eq!(Some(crusader.attack - 2), printed.base.attack);
        }

        #[test]
        fn a_human_arriving_later_has_it_and_it_outlasts_the_crusader() {
            let mut s = crusade_game(false);
            let standing = s.unit_at(P1, 2).id;
            s.play(ID, json!({}));
            let crusader = s.card(ID).id.clone();
            s.play(KILL, pick::targets(pick::id(&crusader)));
            s.expect_in_zone(crusader.as_str(), "graveyard");
            s.play(HUMAN, json!({}));
            let later = must(
                active_units_of(s.state(), P1)
                    .into_iter()
                    .find(|card| card.def_id == HUMAN && card.id != standing)
                    .map(|card| card.id.clone()),
                "the Human played later",
            );
            let theirs = s.stats(s.unit_at(P2, 2).id.as_str());
            let grown = s.stats(later.as_str());
            assert_eq!((grown.attack, grown.max_health), (theirs.attack + 2, theirs.max_health + 2));
        }

        #[test]
        fn shuffles_5_random_human_units_into_your_deck() {
            let mut s = crusade_game(false);
            s.play(ID, json!({}));
            let library = nonempty(s.pile(P1, "library"), "p1's deck");
            assert_eq!(library.len(), 1 + 5, "the Human it held and five more");
            for card in &library {
                assert!(is_human_unit(&s, card), "{}", card.def_id);
                assert!(!card.radiant);
            }
        }

        #[test]
        fn the_numbers_tune() {
            let mut s = crusade_game(false);
            let mine = s.unit_at(P1, 2).id;
            let before = s.stats(mine.as_str()).attack;
            set_param(s.card_mut(ID), "stats", 1);
            set_param(s.card_mut(ID), "humans", 2);
            s.play(ID, json!({}));
            assert_eq!(s.stats(mine.as_str()).attack, before + 1);
            assert_eq!(s.pile(P1, "library").len(), 1 + 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_humans_have_4_4() {
            let mut s = crusade_game(true);
            let mine = s.unit_at(P1, 2).id;
            let before = s.stats(mine.as_str());
            s.play(ID, json!({}));
            let after = s.stats(mine.as_str());
            assert_eq!((after.attack, after.max_health), (before.attack + 4, before.max_health + 4));
        }

        #[test]
        fn exiles_every_non_human_unit_of_yours() {
            let mut s = crusade_game(true);
            let others = others_of_p1(&s);
            s.play(ID, json!({}));
            for id in nonempty(others, "p1's non-Human Units") {
                s.expect_in_zone(id.as_str(), "exile");
            }
        }

        #[test]
        fn shuffles_5_random_radiant_human_units_into_your_deck() {
            let mut s = crusade_game(true);
            s.play(ID, json!({}));
            let fresh: Vec<CardInstance> = s
                .pile(P1, "library")
                .into_iter()
                .filter(|card| card.def_id != HUMAN || card.radiant)
                .collect();
            assert_eq!(fresh.len(), 5);
            for card in &fresh {
                assert!(is_human_unit(&s, card), "{}", card.def_id);
                assert!(card.radiant);
            }
        }
    }
}
