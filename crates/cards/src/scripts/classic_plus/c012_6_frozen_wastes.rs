//! C+ #12.6 Frozen Wastes (SPEC §8.7 row 12.6, R408): (2) Spell, Pancake, Token (printed Legendary;
//! balance patch 1: a Spell, not a Field Spell).
//!   Base:    "Cry: Destroy all Units. Exile the top card of your deck for each one destroyed."
//!   Radiant: "… Exile the top card of your opponent's deck for each one destroyed."
//! A Spell's unlabelled one-time text is its Cry; the card then goes to the graveyard (R408).
//! "Each one" is the Units the destroy dooms, read as it resolves (the exile follows in the same list,
//! before the state check, R59): an Indestructible one isn't, a Reborn one is. A short deck exiles
//! what it has, with no fatigue. The preview (R280) is how many cards it would exile now.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-012-6";

/// The Units a destroy of all Units dooms now: every acting Unit but the Indestructible ones (R46).
fn doomed(state: &GameState) -> i32 {
    [PlayerId::P1, PlayerId::P2]
        .iter()
        .flat_map(|&player| active_units_of(state, player))
        .filter(|unit| !has_keyword(&unit_view(state, unit).keywords, KeywordKind::Indestructible))
        .count() as i32
}

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

fn frozen_wastes(deck_of: impl Fn(PlayerId) -> PlayerId + Copy + Send + Sync + 'static) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let count = doomed(ctx.state);
            let deck = deck_of(ctx.controller);
            vec![
                destroy_all(json_as(json!({ "side": "any" }))),
                for_each_card(ForEachCardArgs {
                    cards: cards_of(move |at| {
                        zone_cards(at.state, deck, OffFieldZone::Library)
                            .into_iter()
                            .take(count.max(0) as usize)
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: each_of(|instance_id| {
                        exile(json_as(json!({ "target": { "of": "instance", "instanceId": instance_id } })))
                    }),
                }),
            ]
        })),
        preview: Some(condition_hook(move |c| {
            let value = doomed(c.state).min(zone_count(c.state, deck_of(c.controller), OffFieldZone::Library));
            vec![json_as::<PreviewValue>(json!({ "label": "for each one destroyed", "value": value }))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: frozen_wastes(|controller| controller),
        radiant: frozen_wastes(opponent_of),
    }
}

// C+ #12.6 Frozen Wastes — SPEC §8.7 row 12.6, R408, BUILD M9 Classic+ row C+ 12.6: "Spell (a Field
// Spell until balance patch 1): destroys every Unit on both sides, then exiles the top card of
// your deck once for each Unit that died (Indestructible survivors don't count, a Reborn unit that died
// does), a short deck exiling what it has with no fatigue; the exiles are public (R97); it then goes to
// the graveyard, needing no backrow zone to play; its preview is the cards it would exile now (R280);
// radiant exiles from the top of the
// opponent's deck instead". The R280 proof is here (its preview reads deck sizes, never contents).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const WASTES: &str = "classicplus-012-6";
    const MENACE: &str = "core-019"; // 9/9
    const ROCK: &str = "core-066"; // Indestructible
    const DEFENDER: &str = "core-003"; // Taunt, Divine Shield, Reborn
    const MANA_WELL: &str = "core-006"; // Field Spell
    const MAGIC_JAMMED: &str = "core-036"; // destroy target backrow card
    const STOCKPILE: &str = "core-005";
    const LUNAR: &str = "core-035";

    fn deck(n: usize) -> Value {
        json!((0..n).map(|i| if i % 2 == 0 { LUNAR } else { STOCKPILE }).collect::<Vec<_>>())
    }

    use crate::merged;

    fn wastes(radiant: bool, p1: Value, p2: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": merged(
                json!({
                    "hand": [{ "def": WASTES, "radiant": radiant }, MAGIC_JAMMED, STOCKPILE],
                    "field": [MENACE],
                    "library": deck(6),
                    "mana": 8,
                }),
                p1,
            ),
            "p2": merged(json!({ "hand": [STOCKPILE], "field": [MENACE, ROCK, DEFENDER], "library": deck(6) }), p2),
        }))
    }

    /// The Frozen Wastes in p1's hand, as p1 reads it.
    fn wastes_in_hand(s: &Scenario) -> Option<CardView> {
        let HandView::Cards(hand) = s.view(P1).you.hand else {
            panic!("own hand in full");
        };
        hand.into_iter().find(|card| card.def_id == WASTES)
    }

    fn preview_of(s: &Scenario) -> Option<i32> {
        wastes_in_hand(s).and_then(|card| card.preview).and_then(|preview| preview.first().map(|each| each.value))
    }

    fn ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r408_its_cry_destroys_every_unit_on_both_sides_an_indestructible_one_survives() {
            crate::register_all();
            let mut s = wastes(false, json!({}), json!({}));
            s.play(WASTES, json!({}));
            assert!(s.pile(P1, "graveyard").iter().any(|card| card.def_id == MENACE));
            assert!(s.pile(P2, "graveyard").iter().any(|card| card.def_id == MENACE));
            s.expect_in_zone(ROCK, "field");
        }

        #[test]
        fn r408_exiles_the_top_card_of_your_deck_once_per_unit_that_died_a_reborn_unit_counts_the_indestructible_one_doesn_t() {
            crate::register_all();
            let mut s = wastes(false, json!({}), json!({}));
            let top: Vec<String> = ids(&s.pile(P1, "library")).into_iter().take(3).collect();
            s.play(WASTES, json!({}));
            // Your Menace, their Menace and their Defender (back again by Reborn): three.
            assert_eq!(ids(&s.pile(P1, "exile")), top);
            assert_eq!(s.pile(P1, "library").len(), 3);
            assert_eq!(s.unit(P2, 3).map(|unit| unit.def_id), Some(DEFENDER.to_string()));
            assert_eq!(s.pile(P2, "exile").len(), 0);
        }

        #[test]
        fn a_short_deck_exiles_what_it_has_with_no_fatigue() {
            crate::register_all();
            let mut s = wastes(false, json!({ "library": deck(1) }), json!({}));
            s.play(WASTES, json!({}));
            assert_eq!(s.pile(P1, "exile").len(), 1);
            assert!(!s.events().iter().any(|event| event.event_type() == GameEventType::Fatigue));
        }

        #[test]
        fn r97_the_exiles_are_public() {
            crate::register_all();
            let mut s = wastes(false, json!({}), json!({}));
            s.play(WASTES, json!({}));
            let exiled: Vec<GameEvent> = s
                .view(P2)
                .events
                .into_iter()
                .filter(|event| event.event_type() == GameEventType::Exiled)
                .collect();
            assert_eq!(exiled.len(), 3);
            for event in &exiled {
                if let GameEvent::Exiled { def_id, .. } = event {
                    assert_ne!(def_id, "hidden");
                }
            }
            assert_eq!(s.view(P2).opponent.exile.len(), 3);
        }

        #[test]
        fn r408_a_spell_since_balance_patch_1_it_resolves_and_goes_to_the_graveyard_with_no_backrow_body_left() {
            crate::register_all();
            let mut s = wastes(false, json!({ "field": [] }), json!({ "field": [] }));
            s.play(WASTES, json!({}));
            s.expect_in_zone(WASTES, "graveyard");
            assert!(s.backrow(P1, 1).is_none());
        }

        #[test]
        fn it_needs_no_open_backrow_zone_to_play() {
            crate::register_all();
            let mut s = wastes(
                false,
                json!({ "backrow": [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL] }),
                json!({}),
            );
            s.play(WASTES, json!({}));
            s.expect_in_zone(WASTES, "graveyard");
        }

        #[test]
        fn r280_its_preview_is_the_cards_it_would_exile_now_and_the_cry_then_exiles_that_many() {
            crate::register_all();
            let mut s = wastes(false, json!({}), json!({}));
            assert_eq!(preview_of(&s), Some(3));
            s.play(WASTES, json!({}));
            assert_eq!(s.pile(P1, "exile").len(), 3);

            let short = wastes(false, json!({ "library": deck(2) }), json!({}));
            assert_eq!(preview_of(&short), Some(2));

            let empty = wastes(false, json!({ "field": [] }), json!({ "field": [ROCK] }));
            assert_eq!(preview_of(&empty), Some(0));
        }

        #[test]
        fn r280_the_preview_label_sits_in_each_face_s_text() {
            crate::register_all();
            let s = wastes(false, json!({}), json!({}));
            let label = wastes_in_hand(&s)
                .and_then(|card| card.preview)
                .and_then(|preview| preview.first().map(|each| each.label.clone()))
                .unwrap_or_default();
            assert_ne!(label, "");
            let def = crate::card_def(ID);
            assert!(def.base.text.contains(&label));
            assert!(def.radiant.text.contains(&label));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn exiles_from_the_top_of_the_opponent_s_deck_instead() {
            crate::register_all();
            let mut s = wastes(true, json!({}), json!({}));
            let top: Vec<String> = ids(&s.pile(P2, "library")).into_iter().take(3).collect();
            s.play(WASTES, json!({}));
            assert_eq!(ids(&s.pile(P2, "exile")), top);
            assert_eq!(s.pile(P1, "exile").len(), 0);
        }

        #[test]
        fn r280_its_preview_reads_the_opponent_s_deck_size() {
            crate::register_all();
            let s = wastes(true, json!({}), json!({ "library": deck(2) }));
            assert_eq!(preview_of(&s), Some(2));
        }
    }
}
