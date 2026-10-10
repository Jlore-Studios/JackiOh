//! M #87 Tatches the Totem (SPEC §8.8 row 87): (1) Unit, All Tribes (Human, Felinor, KY, CN,
//! Jlockeed), Legendary, 0/3 → 0/6.
//!
//! Base:    "While this is in your deck: After you play a card with a tribal tag, summon this.
//!          End of turn: Buff {cards|random card|random cards} in your hand or deck."
//! Radiant: "While this is in your deck: After you play a card with a tribal tag, summon this.
//!          End of turn: Buff {cards|random card|random cards} in your hand or deck. Then Buff each
//!          Unit adjacent to this."
//! Engine: a deck trigger (R464, C+ #37 Wardrum's) on `cardResolved` of its owner's play, a cast
//! included (R70), whose running face carries a tribal tag (TRIBAL_TAGS, R1424): `summon_this` into
//! the leftmost open unit zone (R64), no Cry (R1), the first either player sees of it (R97); with no
//! open zone it stays in the deck. A face-down play is a Trap with no tags until revealed (R448), so
//! it summons nothing. End of turn: `upgrade({ scope: { side: "self", zones: ["hand", "library"] },
//! random: cards })`, one uniform pick over both piles (R440); Radiant, then one Buff on each Unit
//! `adjacent_to` this (§3.1). "All Tribes" is all five tribal tags in the catalog's `tags`, read off
//! them by the client — no catalog field of its own.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-087";

/// Whether this resolution summons Tatches: its owner's play or cast (R70) of a card whose running
/// face is neither Trap nor Field Trap and carries a tribal tag (R1424, MD-E11).
fn tribal_play(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardResolved { player, def_id, radiant, .. } = event else {
        return false;
    };
    if *player != ctx.controller {
        return false;
    }
    let state: &GameState = ctx.state;
    if matches!(
        card_type_of_face(state, def_id, *radiant == Some(true)),
        CardType::Trap | CardType::FieldTrap
    ) {
        return false;
    }
    def_of(Some(state), def_id).tags.iter().any(|tag| TRIBAL_TAGS.contains(tag))
}

fn summon_on_tribal() -> TriggerDef {
    // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
    TriggerDef::new("tatches-summon", &[GameEventType::CardResolved], |ctx, event| {
        if tribal_play(ctx, event) { vec![summon_this()] } else { vec![] }
    })
}

fn end_of_turn(radiant: bool) -> Hook {
    hook(move |ctx| {
        let mut effects = vec![upgrade(json_as(json!({
            "scope": { "side": "self", "zones": ["hand", "library"] },
            "random": param(&*ctx, "cards"),
        })))];
        if radiant {
            // C+ #20 Mushroom Power's loop, with a Buff in place of the stat buff.
            effects.push(for_each_card(ForEachCardArgs {
                cards: Arc::new(|each: &mut EffectContext<'_>| {
                    adjacent_to(each, &json_as(json!({ "of": "self" })), &BoardScope::default())
                        .into_iter()
                        .map(|card| card.id)
                        .collect()
                }),
                each: Arc::new(|instance_id: &str| {
                    upgrade(json_as(json!({ "instanceId": instance_id })))
                }),
            }));
        }
        effects
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            deck_triggers: vec![summon_on_tribal()],
            end_of_turn: Some(end_of_turn(false)),
            ..Script::default()
        },
        radiant: Script {
            deck_triggers: vec![summon_on_tribal()],
            end_of_turn: Some(end_of_turn(true)),
            ..Script::default()
        },
    }
}

// M #87 Tatches the Totem — SPEC §8.8 row 87, BUILD M10 row M 87: "In your deck, after you play or
// cast a card with a tribal tag (Human, Felinor, KY, CN or Jlockeed; MD-E11) and it resolves, it is
// summoned into your leftmost open unit zone with no Cry, hidden until then; a card with no tribal
// tag, or a face-down play, does nothing; with no open zone it stays; the frame prints All Tribes
// and its tags list all five, so any tag filter for one of them sees it; at your end of turn one
// random card of your hand and deck together gets one Buff, cued per R440; cards reads through
// `param()`; radiant 0/6, then one Buff on each Unit adjacent to it".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TATCHES: &str = "meditative-087";
    const VANILLA: &str = "core-008"; // (1) Human Unit: a tribal play.
    const STATELESS: &str = "classic-041"; // (1) Unit with no tribal tag.
    const GROOM: &str = "classicplus-002"; // (3) Trap with the Felinor tag.
    const FILL_BOARD: &str = "core-062"; // (3) Felinor Spell: fills your board.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt Unit.
    const FILLER: &str = "core-005";

    fn filler(count: usize) -> Value {
        json!(vec!["core-005"; count])
    }

    /// p1's deck holds Tatches between fillers; both sides hold fillers and p1 has mana to play.
    fn decked(seed: &str, hand: Value) -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": seed,
            "p1": { "mana": 10, "hand": hand, "library": [FILLER, TATCHES, FILLER] },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn upgraded_for(events: &[GameEvent], id: &str) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .count()
    }

    fn upgraded_count(events: &[GameEvent]) -> usize {
        events.iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count()
    }

    fn in_deck(s: &Scenario) -> bool {
        s.pile(P1, "library").iter().any(|card| card.def_id == TATCHES)
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(|event| serde_json::to_value(event).expect("an event is JSON")).collect()
    }

    mod m87_tatches_the_totem {
        use super::*;

        #[test]
        fn is_a_1_0_3_legendary_unit_radiant_0_6_with_all_five_tribal_tags() {
            let def = crate::card_def(ID);
            assert_eq!(def.id, TATCHES);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(def.rarity, Rarity::Legendary);
            assert_eq!(
                [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                [Some(0), Some(3), Some(0), Some(6)]
            );
            let params = def.params.expect("cards");
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].key, "cards");
            assert_eq!((params[0].base, params[0].radiant), (1, 1));
        }

        #[test]
        fn r1162_its_tags_are_the_five_tribal_tags() {
            assert_eq!(
                crate::card_def(ID).tags,
                vec![Tag::Human, Tag::Felinor, Tag::Ky, Tag::Cn, Tag::Jlockeed]
            );
        }

        mod base {
            use super::*;

            #[test]
            fn r1162_a_tribal_unit_played_summons_it_into_the_leftmost_open_zone_with_no_cry() {
                // Lane 1 is taken, so the Vanilla takes lane 2 and Tatches the leftmost open one.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tatches-summon",
                    "p1": {
                        "mana": 10,
                        "hand": [VANILLA],
                        "field": [{ "def": MENACE, "lane": 1 }],
                        "library": [FILLER, TATCHES, FILLER],
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                s.play(VANILLA, json!({}));
                assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some(VANILLA.to_string()));
                assert_eq!(s.unit(P1, 3).map(|card| card.def_id), Some(TATCHES.to_string()));
                assert!(!in_deck(&s));
                // Summoned after the play resolved, with no Cry (no play of its own).
                let events = events_json(&s);
                let resolved =
                    events.iter().rposition(|event| event["type"] == "cardResolved").map_or(-1, |at| at as i64);
                let summoned = events
                    .iter()
                    .position(|event| event["type"] == "summoned" && event["defId"] == TATCHES)
                    .map_or(-1, |at| at as i64);
                assert!(summoned > resolved);
                assert!(!events.iter().any(|event| event["type"] == "cardPlayed" && event["defId"] == TATCHES));
            }

            #[test]
            fn r1162_a_card_with_no_tribal_tag_does_nothing() {
                let mut s = decked("tatches-plain", json!([STATELESS]));
                s.play(STATELESS, json!({}));
                assert!(s.unit(P1, 2).is_none());
                assert!(in_deck(&s));
            }

            #[test]
            fn r1162_a_face_down_tribal_trap_does_nothing() {
                let mut s = decked("tatches-trap", json!([GROOM]));
                s.play(GROOM, json!({ "zone": 1 }));
                // Set face-down, and Tatches stays in the deck.
                let trap = s.backrow(P1, 1).expect("the set trap");
                assert_eq!(trap.def_id, GROOM);
                assert!(trap.face_up != Some(true));
                assert!(in_deck(&s));
            }

            #[test]
            fn r1162_the_opponents_tribal_play_does_nothing() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tatches-foe",
                    "p1": { "mana": 10, "hand": [FILLER], "library": [FILLER, TATCHES, FILLER] },
                    "p2": { "hand": [VANILLA], "library": filler(4) },
                }));
                s.end_turn(); // p1 passes; p2 to act.
                let foe = s.hand(P2).iter().find(|card| card.def_id == VANILLA).expect("foe vanilla").id.clone();
                s.play(&foe, json!({}));
                assert!(in_deck(&s));
                assert!(s.unit(P2, 1).is_some());
            }

            #[test]
            fn with_no_open_zone_it_stays_in_the_deck() {
                // Friend of Felinors fills the board and is itself the tribal play.
                let mut s = decked("tatches-full", json!([FILL_BOARD]));
                s.play(FILL_BOARD, json!({}));
                assert_eq!((1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count(), 5);
                assert!(in_deck(&s));
            }

            #[test]
            fn r97_hidden_until_summoned() {
                let mut s = decked("tatches-hidden", json!([VANILLA, VANILLA]));
                let view_text =
                    |s: &Scenario| serde_json::to_string(&s.view(P2)).expect("a view is JSON");
                assert!(!view_text(&s).contains(TATCHES));
                s.play(VANILLA, json!({}));
                let theirs = s.view(P2);
                let theirs_json = serde_json::to_value(&theirs).expect("a view is JSON");
                assert_eq!(theirs_json["opponent"]["units"][0]["defId"], TATCHES);
                let shown = theirs_json["events"].as_array().cloned().unwrap_or_default();
                assert!(shown.iter().any(|event| event["type"] == "summoned" && event["defId"] == TATCHES));
            }

            #[test]
            fn r440_end_of_turn_one_random_card_of_hand_and_deck_gets_one_buff() {
                let mut hand_hit = false;
                let mut deck_hit = false;
                for seed in 1..=20 {
                    crate::register_all();
                    let mut s = scenario(json!({
                        "seed": format!("tatches-eot-{seed}"),
                        "p1": {
                            "hand": [FILLER, FILLER],
                            "field": [{ "def": TATCHES, "lane": 1 }],
                            "library": filler(3),
                        },
                        "p2": { "hand": [FILLER], "library": filler(4) },
                    }));
                    let hand: Vec<String> =
                        s.hand(P1).iter().map(|card| card.id.clone()).collect();
                    let deck: Vec<String> =
                        s.pile(P1, "library").iter().map(|card| card.id.clone()).collect();
                    let before = upgraded_count(s.events());
                    s.end_turn();
                    assert_eq!(upgraded_count(s.events()), before + 1, "one Buff (seed {seed})");
                    let buffed = s
                        .events()
                        .iter()
                        .filter_map(|event| match event {
                            GameEvent::Upgraded { instance_id, .. } => Some(instance_id.clone()),
                            _ => None,
                        })
                        .last()
                        .expect("the Buff");
                    if hand.contains(&buffed) {
                        hand_hit = true;
                    } else if deck.contains(&buffed) {
                        deck_hit = true;
                    } else {
                        panic!("the Buff hit neither pile: {buffed}");
                    }
                }
                assert!(hand_hit && deck_hit, "both piles are reached");
            }

            #[test]
            fn r386_cards_reads_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tatches-param",
                    "p1": {
                        "hand": [FILLER, FILLER, FILLER],
                        "field": [{ "def": TATCHES, "lane": 1 }],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                crate::upgrade_number(&mut s, TATCHES, "cards");
                let before = upgraded_count(s.events());
                s.end_turn();
                assert_eq!(upgraded_count(s.events()), before + 2);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_then_one_buff_on_each_adjacent_unit() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "tatches-adjacent",
                    "p1": {
                        "hand": [FILLER],
                        "field": [
                            { "def": VANILLA, "lane": 1 },
                            { "def": TATCHES, "radiant": true, "lane": 2 },
                            { "def": VANILLA, "lane": 3 },
                            { "def": VANILLA, "lane": 5 },
                        ],
                        "library": filler(3),
                    },
                    "p2": { "hand": [FILLER], "library": filler(4) },
                }));
                let (one, three, five) = (
                    s.unit(P1, 1).expect("lane 1").id.clone(),
                    s.unit(P1, 3).expect("lane 3").id.clone(),
                    s.unit(P1, 5).expect("lane 5").id.clone(),
                );
                s.end_turn();
                assert_eq!(upgraded_for(s.events(), &one), 1);
                assert_eq!(upgraded_for(s.events(), &three), 1);
                assert_eq!(upgraded_for(s.events(), &five), 0, "lane 5 is untouched");
            }
        }
    }
}
