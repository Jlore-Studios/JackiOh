//! C+ #75.1 J-lease J-Jungle EX-plorer Pack (SPEC §8.7 row 75.1). (2) Spell, Token (printed Legendary).
//!   Base:    "Cast on draw: Add {cards|random Radiant Classic or Classic+ card|…cards} to your hand."
//!   Radiant: "… Each costs (0)."
//!   Engine:  "Cast on draw (§6.2, R58). Non-token cards of the Classic and Classic+ sets (the text names
//!            them, R380), repeats allowed (R60), made Radiant; the hand cap burns what doesn't fit
//!            (§2.4); the Radiant sets `costOverride` 0. A spell token, so it goes to the graveyard
//!            after it resolves (§7). Tunes: cards 5 ↑."
//!
//! Cast on draw is the static flag §2.4's draw reads (R58): drawn, it casts itself at once and the draw
//! repeats, so its owner draws again; played from a hand it resolves the same way. The pool names its two
//! sets, so Core never comes up; a Pack is a token, so it is never in its own pool (§5.1, R387). Each card
//! is made Radiant before it reaches the hand (R74), and the Radiant face's price lands only on a card that
//! reached it (§2.4, R4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-075-1";

/// §8.7 row 75.1: "Each costs (0)" on the Radiant face.
const SET_COST: i32 = 0;

fn pack(free: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            cast_on_draw: Some(true),
            ..StaticFlags::default()
        }),
        cry: Some(hook(move |ctx| {
            let mut args = json!({
                "query": { "set": ["Classic", "Classic+"] },
                "count": param(&*ctx, "cards"),
                "radiant": true,
            });
            if free {
                args["costOverride"] = json!(SET_COST);
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

// C+ #75.1 J-lease J-Jungle EX-plorer Pack — SPEC §8.7 row 75.1, BUILD M9 Classic+ row C+ 75.1: "Cast on
// draw (R70, R58): adds 5 random Radiant non-token Classic or Classic+ cards (never Core) to your hand,
// repeats allowed, a full hand burning the rest, then you draw again; played from a hand it does the
// same; the five are hidden from the opponent (R97); the count reads through `param()`; radiant they also
// cost (0)".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const PACK: &str = "classicplus-075-1";
    const STOCKPILE: &str = "core-005"; // (1) "Draw 2. Heal your hero 2."
    const FILLER: &str = "core-011"; // Tempo Timmy, deck filler
    const MENACE: &str = "core-019";

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    use crate::js;

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

    fn view_events(s: &Scenario, player: PlayerId) -> Vec<Value> {
        js(&s.view(player))["events"].as_array().cloned().unwrap_or_default()
    }

    fn menaces(n: usize) -> Vec<Value> {
        (0..n).map(|_| json!(MENACE)).collect()
    }

    mod c_n75_1_j_lease_j_jungle_ex_plorer_pack {
        use super::*;

        #[test]
        fn is_a_2_spell_token_printed_legendary_that_casts_itself_on_draw() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, PACK);
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(js(&def.cost), json!(2));
            assert_eq!(def.printed_rarity, Some(PrintedRarity::Legendary));
            let scripts = super::super::script();
            assert_eq!(scripts.base.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
            assert_eq!(scripts.radiant.static_flags.as_ref().and_then(|flags| flags.cast_on_draw), Some(true));
        }

        #[test]
        fn r482_the_text_agrees_with_its_count_at_every_value_and_the_radiant_price_reads_for_one_card_or_five() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            let one: IndexMap<String, i32> = IndexMap::from([("cards".to_string(), 1)]);
            assert_eq!(
                fill_params(&def, FaceKind::Base, None),
                "Cast on draw: Add 5 random Radiant Classic or Classic+ cards to your hand."
            );
            assert_eq!(
                fill_params(&def, FaceKind::Base, Some(&one)),
                "Cast on draw: Add 1 random Radiant Classic or Classic+ card to your hand."
            );
            assert_eq!(
                fill_params(&def, FaceKind::Radiant, Some(&one)),
                "Cast on draw: Add 1 random Radiant Classic or Classic+ card to your hand. Each costs (0)."
            );
        }

        mod base {
            use super::*;

            #[test]
            fn r58_drawn_it_casts_itself_5_random_radiant_classic_or_classic_plus_cards_reach_your_hand_then_you_draw_again() {
                crate::register_all();
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
                    assert!([SetName::Classic, SetName::ClassicPlus].contains(&printed.set));
                    assert!(!printed.token);
                    assert!(card.radiant);
                    assert!(card.cost_override.is_none());
                }
                // R58: the draw repeats after the cast, and Stockpile's second draw follows: both Timmies drawn.
                assert!(s.pile(P1, "library").is_empty());
                assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == FILLER).count(), 2);
                s.expect_events(json!(["drawn", "cardPlayed", "addedToHand", "drawn"]));
            }

            #[test]
            fn r70_played_from_a_hand_it_does_the_same() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PACK, MENACE] }, "p2": { "hand": [MENACE] } }));
                s.play(PACK, json!({}));
                assert_eq!(of_type(generated(&s), GameEventType::AddedToHand).len(), 5);
                s.expect_in_zone(PACK, "graveyard");
            }

            #[test]
            fn r380_over_many_seeds_the_cards_are_of_classic_and_classic_plus_only_never_core_never_a_token() {
                crate::register_all();
                let mut sets: IndexSet<String> = IndexSet::new();
                for i in 0..20 {
                    let seed = format!("pack-{i}");
                    let mut s = scenario(json!({
                        "seed": seed,
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
                assert_eq!(sorted, vec!["Classic", "Classic+"]);
            }

            #[test]
            fn s2_4_r4_a_full_hand_burns_the_rest() {
                crate::register_all();
                let mut hand = vec![json!(PACK)];
                hand.extend(menaces(7));
                let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [MENACE] } }));
                s.play(PACK, json!({}));
                let events = generated(&s);
                assert_eq!(of_type(events.clone(), GameEventType::AddedToHand).len(), 3);
                assert_eq!(of_type(events, GameEventType::Burned).len(), 2);
                assert_eq!(s.hand(P1).len(), 10);
            }

            #[test]
            fn r97_the_five_reach_the_opponent_under_the_sentinel() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [PACK, MENACE] }, "p2": { "hand": [MENACE] } }));
                s.play(PACK, json!({}));
                let theirs: Vec<Value> =
                    view_events(&s, P2).into_iter().filter(|event| event["type"] == "addedToHand").collect();
                assert_eq!(theirs.len(), 5);
                assert!(
                    theirs
                        .iter()
                        .all(|event| event["defId"] == "hidden" && event["instanceId"] == "hidden")
                );
            }

            #[test]
            fn r386_an_upgrade_adds_6_a_degrade_4() {
                crate::register_all();
                let mut up = scenario(json!({ "p1": { "hand": [PACK, MENACE] }, "p2": { "hand": [MENACE] } }));
                step_param(up.card_mut(PACK), "cards", 1);
                up.play(PACK, json!({}));
                assert_eq!(generated(&up).len(), 6);

                let mut down = scenario(json!({ "p1": { "hand": [PACK, MENACE] }, "p2": { "hand": [MENACE] } }));
                step_param(down.card_mut(PACK), "cards", -1);
                down.play(PACK, json!({}));
                assert_eq!(generated(&down).len(), 4);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_five_are_radiant_and_cost_0() {
                crate::register_all();
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
                let hand = js(&s.view(P1))["you"]["hand"].clone();
                let free = hand.as_array().map(|cards| cards.iter().filter(|card| card["cost"] == 0).count());
                assert_eq!(free, Some(5));
            }

            #[test]
            fn r58_a_drawn_radiant_pack_casts_itself_the_same_way() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, MENACE], "library": [{ "def": PACK, "radiant": true }, FILLER] },
                    "p2": { "hand": [MENACE] },
                }));
                s.play(STOCKPILE, json!({}));
                let added = of_type(generated(&s), GameEventType::AddedToHand);
                assert_eq!(added.len(), 5);
                assert!(
                    added
                        .iter()
                        .all(|event| s.card(instance_id_of(event).as_str()).cost_override == Some(0))
                );
            }

            #[test]
            fn s2_4_a_burned_card_keeps_no_price() {
                crate::register_all();
                let mut hand = vec![json!({ "def": PACK, "radiant": true })];
                hand.extend(menaces(9));
                let mut s = scenario(json!({ "p1": { "hand": hand }, "p2": { "hand": [MENACE] } }));
                s.play(PACK, json!({}));
                let burned = of_type(generated(&s), GameEventType::Burned);
                assert_eq!(burned.len(), 4);
                assert!(
                    burned
                        .iter()
                        .all(|event| s.card(instance_id_of(event).as_str()).cost_override.is_none())
                );
            }
        }
    }
}
