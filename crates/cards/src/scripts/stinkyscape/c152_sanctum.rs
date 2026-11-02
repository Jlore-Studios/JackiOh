//! S #152 Sanctum (SPEC §8.10 row 152): (4) Field Spell, Legendary, of the Stinkyscape set.
//!   Base:    "Indestructible / Aura: Every card in your deck and graveyard has Flashback."
//!   Radiant: "Indestructible / Cry: Gain {mana} mana. / Aura: Every card in your deck and graveyard has
//!            Flashback."
//! Engine: Indestructible is a catalog keyword (§6.1): a destroy leaves it standing. R1869003: while it
//! acts on the field, an aura (§10.4 layer 5) lends Flashback (R783012) to every card in its
//! controller's deck and graveyard, so each card in that graveyard is offered as a play from there at
//! its cost (`graveyard_play`), and a played one's `flashbackSpent` keeps the aura off it for good
//! (R783013). The entry reads the card's zone alone, no layer, so it cannot recurse. A deck card holds
//! the keyword but is never played from the deck, and the opponent's cards are not reached. Radiant: Cry
//! (R1): gain {mana} mana this turn (§6.3 Mana).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-152";

/// R1869003: "Every card in your deck and graveyard has Flashback": the cards in its controller's
/// library and graveyard, read off their zone.
fn flashback_aura() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let controller = args.self_.controller;
        vec![AuraEntry {
            applies: Box::new(move |card: &CardInstance| {
                matches!(card.zone, Zone::Library { player } | Zone::Graveyard { player } if player == controller)
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Flashback]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(flashback_aura()),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| vec![gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") })))])),
        ..base.clone()
    };
    CardScripts { base, radiant }
}

// S #152 Sanctum — SPEC §8.10 row 152, BUILD M12 row S 152.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: draw 2, heal 2.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const EXILER: &str = "core-034"; // (4) Spell: Exile a permanent and a random card from the opponent's deck.
    const SHATTER: &str = "core-036"; // (1) Spell: Destroy a backrow card. Lock its zone.

    fn standing(radiant: bool, graveyard: Value) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "hand": [FILLER],
                "backrow": [{ "def": ID, "radiant": radiant }],
                "graveyard": graveyard,
                "library": crate::filler(4),
            },
            "p2": { "hand": [FILLER], "graveyard": [VANILLA], "library": crate::filler(4) },
        }))
    }

    fn graveyard_defs_offered(s: &Scenario, player: PlayerId) -> Vec<String> {
        let offered: Vec<String> = legal_actions(s.state(), player)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        s.pile(player, "graveyard")
            .into_iter()
            .filter(|card| offered.contains(&card.id))
            .map(|card| card.def_id)
            .collect()
    }

    fn play_from_graveyard(s: &mut Scenario, card: &CardInstance, extra: Value) {
        let mut action = json!({ "type": "play", "playerId": "p1", "instanceId": card.id, "nonce": format!("sanctum-{}", card.id) });
        if let (Some(into), Some(more)) = (action.as_object_mut(), extra.as_object()) {
            for (key, value) in more {
                into.insert(key.clone(), value.clone());
            }
        }
        let result = reduce(s.state(), &json_as::<Action>(action));
        assert_eq!(result.error, None);
        *s.state_mut() = result.state;
    }

    fn flashback(s: &Scenario, card: &CardInstance) -> bool {
        has_flashback(s.state(), s.card(&card.id))
    }

    mod base {
        use super::*;

        #[test]
        fn r1869003_each_graveyard_card_is_offered_and_plays_once() {
            let mut s = standing(false, json!([VANILLA, FILLER]));
            assert_eq!(graveyard_defs_offered(&s, P1), vec![VANILLA, FILLER]);

            let vanilla = s.pile(P1, "graveyard")[0].clone();
            play_from_graveyard(&mut s, &vanilla, json!({ "zone": { "row": "units", "lane": 2 } }));
            assert_eq!(s.unit_at(P1, 2).id, vanilla.id);
            s.expect_mana(P1, 4 - 1);
            let stockpile = s.pile(P1, "graveyard")[0].clone();
            play_from_graveyard(&mut s, &stockpile, json!({}));
            s.expect_mana(P1, 4 - 2);
            // Back in the graveyard, its Flashback spent for good (R783013): not offered again.
            s.expect_in_zone(&stockpile, "graveyard");
            assert!(s.card(&stockpile.id).flashback_spent == Some(true));
            assert!(!flashback(&s, &stockpile));
            assert!(graveyard_defs_offered(&s, P1).is_empty());
        }

        #[test]
        fn r1869003_the_opponents_graveyard_is_not_offered() {
            let s = standing(false, json!([]));
            let theirs = s.pile(P2, "graveyard")[0].clone();
            assert!(!flashback(&s, &theirs));
            assert!(!legal_actions(s.state(), P1).iter().any(|action| matches!(
                action,
                ActionBody::Play { instance_id, .. } if *instance_id == theirs.id
            )));
        }

        #[test]
        fn r1869003_the_offer_ends_with_sanctum() {
            let mut s = crate::scenario(json!({
                "p1": {
                    "hand": [EXILER, FILLER],
                    "backrow": [ID],
                    "graveyard": [VANILLA],
                    "library": crate::filler(4),
                },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            let vanilla = s.pile(P1, "graveyard")[0].clone();
            assert!(flashback(&s, &vanilla));
            let sanctum = s.backrow(P1, 1).expect("the Sanctum");
            s.play(EXILER, json!({ "targets": pick::instance(&sanctum) }));
            s.expect_in_zone(&sanctum, "exile");
            s.state_mut().players.p1.mana.current = 4;
            assert!(!flashback(&s, &vanilla));
            assert!(graveyard_defs_offered(&s, P1).is_empty());
        }

        #[test]
        fn a_deck_card_has_flashback_and_no_play() {
            let s = standing(false, json!([]));
            let top = s.pile(P1, "library")[0].clone();
            assert!(flashback(&s, &top));
            assert!(!legal_actions(s.state(), P1).iter().any(|action| matches!(
                action,
                ActionBody::Play { instance_id, .. } if *instance_id == top.id
            )));
            assert!(!serde_json::to_string(&s.view(P1)).expect("serializes").contains("Flashback"));
        }

        #[test]
        fn destroyed_by_nothing() {
            let mut s = crate::scenario(json!({
                "active": "p2",
                "p1": { "hand": [FILLER], "backrow": [ID], "library": crate::filler(4) },
                "p2": { "hand": [SHATTER, FILLER], "library": crate::filler(4) },
            }));
            let sanctum = s.backrow(P1, 1).expect("the Sanctum");
            s.play(SHATTER, json!({ "targets": pick::instance(&sanctum) }));
            s.expect_in_zone(&sanctum, "field");
            assert!(has_keyword(&s.stats(&sanctum).keywords, KeywordKind::Indestructible));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn cry_gains_4_mana_through_param() {
            let holding = |radiant: bool| {
                crate::scenario(json!({
                    "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(4) },
                    "p2": { "hand": [FILLER], "library": crate::filler(4) },
                }))
            };
            let mut s = holding(true);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_mana(P1, 4);

            let mut s = holding(true);
            step_param(s.card_mut(ID), "mana", 1);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_mana(P1, 5);

            let mut s = holding(false);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_mana(P1, 0);
        }

        #[test]
        fn r1869003_the_radiant_aura_lends_flashback_too() {
            let s = standing(true, json!([VANILLA]));
            assert_eq!(graveyard_defs_offered(&s, P1), vec![VANILLA]);
        }
    }
}
