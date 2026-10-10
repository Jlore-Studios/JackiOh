//! M #55 Dragon Fruit (SPEC §8.8 row 55): (2) Spell, CN, Fruit, Rare.
//!   Base:    "Add {cards|random Prime card|random Prime cards} to your hand." — cards 1
//!   Radiant: "Add {cards|random Radiant Prime card|random Radiant Prime cards} to your hand."
//! Engine: the Prime pool (R1421): Prime-tagged tokens (C+ #38.1, C+ #46.1, M #45.1, M #91.1),
//!   via `add_random_from_catalog` with a Prime tag query (R382's pool takes its tokens);
//!   repeats allowed (R60); a full hand burns; hidden (R97). A Fruit, it joins Fruit pools (R382).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-055";

fn dragon_fruit(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Prime"] },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: dragon_fruit(false),
        radiant: dragon_fruit(true),
    }
}

// M #55 Dragon Fruit — SPEC §8.8 row 55, BUILD M10 row M 55.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FRUIT: &str = "meditative-055";
    const FILLER: &str = "core-005";
    const PRIME_ONE: &str = "classicplus-038-1";
    const PRIME_TWO: &str = "classicplus-046-1";

    fn fruited(radiant: bool, seed: Option<&str>, fillers: Option<usize>) -> Scenario {
        let mut hand = vec![json!({ "def": FRUIT, "radiant": radiant })];
        hand.extend((0..fillers.unwrap_or(1)).map(|_| json!(FILLER)));
        let mut opts = json!({
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        });
        if let Some(seed) = seed {
            opts["seed"] = json!(seed);
        }
        scenario(opts)
    }

    fn added(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id } => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn adds_one_random_prime_token() {
        crate::register_all();
        let mut s = fruited(false, None, None);
        s.play(FRUIT, json!({}));
        let got = added(&s);
        assert_eq!(got.len(), 1);
        let card = s.card(&got[0].0).clone();
        let def = def_of(Some(s.state()), &card.def_id);
        assert!(def.tags.contains(&Tag::Prime));
        assert!(def.token);
        assert!(!card.radiant);
    }

    #[test]
    fn r1421_both_classic_plus_prime_tokens_come_up_over_40_seeds() {
        crate::register_all();
        let mut seen: IndexSet<String> = IndexSet::new();
        for i in 0..40 {
            let mut s = fruited(false, Some(&format!("dragon-{i}")), None);
            s.play(FRUIT, json!({}));
            for (_, def_id) in added(&s) {
                seen.insert(def_id);
            }
        }
        assert!(seen.contains(PRIME_ONE));
        assert!(seen.contains(PRIME_TWO));
    }

    #[test]
    fn r1420_under_preview_a_fruit_pool_holds_dragon_fruit() {
        crate::register_all();
        let _guard = preview_sets(&[SetName::Meditative]);
        let ids: Vec<String> = crate::query::query(&json_as(json!({ "tags": ["Fruit"] })))
            .iter()
            .map(|def| def.id.clone())
            .collect();
        assert!(ids.contains(&ID.to_string()));
    }

    #[test]
    fn r97_the_opponent_sees_hidden() {
        crate::register_all();
        let mut s = fruited(false, None, None);
        s.play(FRUIT, json!({}));
        let ids: Vec<String> = added(&s).into_iter().map(|(id, _)| id).collect();
        let theirs = s.view(PlayerId::P2);
        let events: Vec<&GameEvent> = theirs
            .events
            .iter()
            .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
            .collect();
        assert_eq!(events.len(), 1);
        for event in events {
            assert!(matches!(
                event,
                GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden"
            ));
        }
        let text = serde_json::to_string(&theirs).unwrap();
        for id in &ids {
            assert!(!text.contains(&format!("\"{id}\"")));
        }
    }

    #[test]
    fn r386_cards_reads_through_param_and_a_full_hand_burns_the_second() {
        crate::register_all();
        let mut up = fruited(false, None, None);
        crate::upgrade_number(&mut up, FRUIT, "cards");
        up.play(FRUIT, json!({}));
        assert_eq!(added(&up).len(), 2);

        crate::register_all();
        let mut full = fruited(false, None, Some(8));
        full.play(FRUIT, json!({}));
        assert_eq!(added(&full).len(), 1);
        assert_eq!(
            full.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(),
            1
        );
    }

    #[test]
    fn radiant_the_prime_card_is_radiant() {
        crate::register_all();
        let mut s = fruited(true, None, None);
        s.play(FRUIT, json!({}));
        let got = added(&s);
        assert_eq!(got.len(), 1);
        assert!(s.card(&got[0].0).radiant);
    }
}
