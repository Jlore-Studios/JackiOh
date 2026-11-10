//! SPEC §8.12 S+ #75.1 J-lease S-Space EX-plorer Pack (Spell, Token printed Legendary, cost 2), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Cast on Draw: Add {draw} random Radiant Stinkyscape or Stinkyscape+ cards to your hand."
//!   Radiant: "… They cost (0)."
//!
//! "Cast on draw" (§6.2, R58): the cards come from both sets' collectible cards (the text names them, R380:
//! tokens never, repeats allowed, R60), each made Radiant before it reaches the hand (R74); the hand cap burns
//! what does not fit (§2.4). The Radiant face sets `costOverride` to (0), a price the text names
//! (`tests/cross/params.rs`: no Buff or Nerf moves it), and it lands only on a card that reached the hand
//! (R4). A spell token, so it goes to the graveyard after it resolves (§7).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-075-1";

/// "They cost (0)": the price the Radiant face's cards carry.
const FREE: i32 = 0;

fn pack(free: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "query": { "set": [SetName::Stinkyscape, SetName::StinkyscapePlus] },
                "count": param(&*ctx, "draw"),
                "radiant": true,
            });
            if free {
                args["costOverride"] = json!(FREE);
            }
            vec![add_random_from_catalog(json_as(args))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: pack(false),
        radiant: pack(true),
    }
}

// S+ #75.1 J-lease S-Space EX-plorer Pack — SPEC §8.12 row 75.1 (R58, R70, R380, R1420).
// While the set is held the cards a Pack adds are drawn from the held set too: each test opens it with `preview_sets`.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{js, scenario};

    const PACK: &str = "stinkyscapeplus-075-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: "Draw 2. Heal your hero 2."
    const FILLER: &str = "core-011"; // Tempo Timmy: deck filler
    const MENACE: &str = "core-019";

    const P1: PlayerId = PlayerId::P1;

    /// The `addedToHand` and `burned` events of the last step that name a generated card.
    fn generated(s: &Scenario) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| match event {
                GameEvent::AddedToHand { def_id, .. } | GameEvent::Burned { def_id, .. } => {
                    def_id != FILLER && def_id != MENACE
                }
                _ => false,
            })
            .cloned()
            .collect()
    }

    fn of_type(events: Vec<GameEvent>, kind: GameEventType) -> Vec<GameEvent> {
        events.into_iter().filter(|event| event.event_type() == kind).collect()
    }

    fn instance_id_of(event: &GameEvent) -> String {
        js(event)["instanceId"].as_str().unwrap_or_default().to_string()
    }

    fn def_id_of(event: &GameEvent) -> String {
        js(event)["defId"].as_str().unwrap_or_default().to_string()
    }

    fn menaces(n: usize) -> Vec<Value> {
        (0..n).map(|_| json!(MENACE)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn drawn_it_casts_itself_and_adds_5_radiant_cards() {
            let _open = preview_sets(&[SetName::StinkyscapePlus]);
            let mut s = scenario(json!({
                "p1": { "hand": [STOCKPILE, MENACE], "library": [PACK, FILLER, FILLER] },
                "p2": { "hand": [MENACE] },
            }));
            s.play(STOCKPILE, json!({}));
            s.expect_in_zone(PACK, "graveyard");
            let added = of_type(generated(&s), GameEventType::AddedToHand);
            assert_eq!(added.len(), 5);
            for event in &added {
                let card = s.card(instance_id_of(event).as_str()).clone();
                let printed = def_of(Some(s.state()), &card.def_id);
                assert!([SetName::Stinkyscape, SetName::StinkyscapePlus].contains(&printed.set));
                assert!(!printed.token);
                assert!(card.radiant);
                assert!(card.cost_override.is_none());
            }
        }

        #[test]
        fn over_many_seeds_the_cards_are_of_both_sets_never_a_token() {
            let _open = preview_sets(&[SetName::StinkyscapePlus]);
            let mut sets: IndexSet<String> = IndexSet::new();
            for i in 0..20 {
                let mut s = scenario(json!({
                    "seed": format!("pack-{i}"),
                    "p1": { "hand": [PACK, MENACE] },
                    "p2": { "hand": [MENACE] },
                }));
                s.play(PACK, json!({}));
                for event in generated(&s) {
                    let printed = def_of(Some(s.state()), &def_id_of(&event));
                    sets.insert(printed.set.as_str().to_string());
                    assert!(!printed.token);
                }
            }
            let mut sorted: Vec<String> = sets.into_iter().collect();
            sorted.sort();
            assert_eq!(sorted, vec!["Stinkyscape", "Stinkyscape+"]);
        }

        #[test]
        fn a_full_hand_burns_the_rest() {
            let _open = preview_sets(&[SetName::StinkyscapePlus]);
            let mut hand = vec![json!(PACK)];
            hand.extend(menaces(7));
            let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [MENACE] } }));
            s.play(PACK, json!({}));
            let events = generated(&s);
            assert_eq!(of_type(events.clone(), GameEventType::AddedToHand).len(), 3);
            assert_eq!(of_type(events, GameEventType::Burned).len(), 2);
            assert_eq!(s.hand(P1).len(), 10);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_five_cost_0() {
            let _open = preview_sets(&[SetName::StinkyscapePlus]);
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": PACK, "radiant": true }, MENACE] },
                "p2": { "hand": [MENACE] },
            }));
            s.play(PACK, json!({}));
            let added = of_type(generated(&s), GameEventType::AddedToHand);
            assert_eq!(added.len(), 5);
            assert!(added.iter().all(|event| {
                let card = s.card(instance_id_of(event).as_str());
                card.radiant && card.cost_override == Some(0)
            }));
        }
    }
}
