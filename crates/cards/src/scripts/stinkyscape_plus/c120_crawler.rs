//! SPEC §8.12 S+ #120 Crawler, a card of the Stinkyscape+ set (R1420, R3535000). Built by S13 (#3560).
//! The first time this game that this enters your board, add a **Toxopid** to your hand.
//! May be played from your GY for (1)
//! 1/3
//! **Can’t defend**
//! After this attack DIE
//! **Death:** deal 2 damage to the last unit this damaged
//! \~\~\~
//! The first time this game that this enters your board, add a **Radiant Toxopid** to your hand.
//! May be played from your GY for (1)
//! 2/6
//! **Can’t defend**
//! After this attack DIE
//! **Death:** deal 4 damage to the last unit this damaged
//!
//! R3560000: the first arrival each game is one mark per player; any arrival counts, both faces share
//! it. R3560001: the graveyard play at (1), any number of times. R3560002: after it attacks it is
//! destroyed (Indestructible keeps it). R3560003: its Death hits the last Unit it damaged.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-120";

/// The Toxopid its arrival adds.
const TOXOPID: &str = "stinkyscapeplus-121";
/// R3560000: the once-a-game first-arrival mark.
const BOARD_KEY: &str = "stinkyscapeplus-120:board";
/// R3560003: the last Unit this damaged.
const LAST_DAMAGED: &str = "lastDamaged";
/// R3560001: "May be played from your graveyard for (1)".
const GRAVEYARD_PRICE: i32 = 1;

/// This card arrived on the field (played or summoned): its `Summoned` names its own id.
fn arrived(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(me) = ctx.live_self() else {
        return false;
    };
    matches!(event, GameEvent::Summoned { instance_id, .. } if *instance_id == me.id)
}

/// A `Damage` event this card dealt to a Unit: its source is this card, it dealt more than 0, and its
/// target is a card (heroes are no instances).
fn dealt_to_unit(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let me = ctx.self_.as_ref()?;
    let GameEvent::Damage {
        source_id,
        target_id,
        amount,
        ..
    } = event
    else {
        return None;
    };
    if source_id.as_deref() != Some(me.id.as_str()) || *amount <= 0 {
        return None;
    }
    find_instance(ctx.state, target_id).map(|_| target_id.clone())
}

fn crawler(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            graveyard_price: Some(GRAVEYARD_PRICE),
            ..StaticFlags::default()
        }),
        triggers: vec![
            TriggerDef::new("crawler-board", &[GameEventType::Summoned], move |ctx, event| {
                if !arrived(ctx, event) {
                    return vec![];
                }
                if once_a_game_used(ctx.state, ctx.controller, BOARD_KEY) {
                    return vec![];
                }
                vec![
                    mark_once_a_game(json_as(json!({ "key": BOARD_KEY }))),
                    add_to_hand(json_as(json!({ "defId": TOXOPID, "radiant": radiant }))),
                ]
            })
            .with_when(|ctx, event| arrived(ctx, event)),
            TriggerDef::new(
                "crawler-damaged",
                &[GameEventType::Damage],
                |ctx, event| match dealt_to_unit(ctx, event) {
                    None => vec![],
                    Some(target_id) => vec![remember(json_as(json!({
                        "key": LAST_DAMAGED,
                        "value": target_id,
                    })))],
                },
            ),
        ],
        after_attack: Some(hook(|_ctx| {
            vec![destroy(json_as(json!({ "target": { "of": "self" } })))]
        })),
        death: Some(hook(|ctx| {
            // A kill in the same pass as its own death never reached the trigger: read it off this
            // action's events first (Frostspatula's same-pass fallback), then the memory.
            let late = ctx
                .events
                .iter()
                .rev()
                .filter_map(|event| dealt_to_unit(ctx, event))
                .next();
            let remembered = recalled(ctx, LAST_DAMAGED).and_then(|value| value.as_str().map(String::from));
            let Some(target_id) = late.or(remembered) else {
                return vec![];
            };
            vec![damage(json_as(json!({
                "to": { "of": "instance", "instanceId": target_id },
                "amount": param(ctx, "damage"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: crawler(false),
        radiant: crawler(true),
    }
}

// S+ #120 Crawler — SPEC §8.12 row 120.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, weapon_board};

    const FILLER: &str = "core-005";
    const BODY: &str = "stand-in (1) 1/10";
    const BIG: &str = "stand-in (1) 9/9";

    fn hand(radiant: bool, extra_hand: Vec<Value>, p2_extra: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(extra_hand);
        let p1 = json!({ "hand": hand, "library": filler(4) });
        let mut p2 = json!({ "library": filler(4) });
        for (key, value) in p2_extra.as_object().cloned().unwrap_or_default() {
            p2[key] = value;
        }
        weapon_board("s120", p1, p2)
    }

    fn toxopids(s: &Scenario) -> Vec<String> {
        s.hand(P1)
            .iter()
            .filter(|card| card.def_id == TOXOPID)
            .map(|card| card.id.clone())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3560000_its_first_arrival_adds_a_toxopid_once_per_game() {
            let mut s = hand(false, vec![json!(ID)], json!({}));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(toxopids(&s).len(), 1);
            assert!(!s.card(&toxopids(&s)[0]).radiant);
            // A second arrival adds nothing: the mark is spent.
            s.play(ID, json!({ "zone": 2 }));
            assert_eq!(toxopids(&s).len(), 1);
        }

        #[test]
        fn r3560000_a_summon_is_an_arrival() {
            // S+ #136 Crawler Assembly Line's Activate summons a Crawler: an arrival like a play's, which
            // `run_as` (an effect with no step after it to dispatch the event) would not show.
            let mut s = weapon_board(
                "s120-summon",
                json!({ "backrow": ["stinkyscapeplus-136"], "library": filler(4) }),
                json!({ "library": filler(4) }),
            );
            assert_eq!(toxopids(&s).len(), 0);
            s.activate("stinkyscapeplus-136", json!({}));
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "the Assembly Line summoned one");
            assert_eq!(toxopids(&s).len(), 1);
        }

        #[test]
        fn r3560001_it_is_played_from_the_graveyard_for_1() {
            // From the graveyard at (1): offered, and the play costs 1.
            let s = weapon_board(
                "s120-grave",
                json!({ "mana": 1, "graveyard": [ID], "library": filler(4) }),
                json!({}),
            );
            let id = s
                .pile(P1, "graveyard")
                .into_iter()
                .find(|card| card.def_id == ID)
                .expect("setup")
                .id;
            let offered: Vec<String> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, .. } => Some(instance_id),
                    _ => None,
                })
                .collect();
            assert!(offered.contains(&id));
            let mana = s.state().players[P1].mana.current;
            let action = json!({ "type": "play", "playerId": "p1", "instanceId": id, "zone": { "row": "units", "lane": 2 }, "nonce": "s120-grave-1" });
            let result = reduce(s.state(), &json_as::<Action>(action));
            assert!(result.error.is_none(), "{:?}", result.error);
            assert_eq!(result.state.players[P1].mana.current, mana - 1);
        }

        #[test]
        fn r3560002_after_it_attacks_it_dies() {
            let mut s = weapon_board(
                "s120-attack",
                json!({ "field": [ID] }),
                json!({ "field": [BODY] }),
            );
            crate::next_turn_of(&mut s, P1);
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r3560003_its_death_hits_the_last_unit_it_damaged() {
            let mut s = weapon_board(
                "s120-death",
                json!({ "field": [{ "def": ID, "lane": 1 }] }),
                json!({ "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 2 }] }),
            );
            crate::next_turn_of(&mut s, P1);
            let first = s.unit_at(P2, 1).id;
            let second = s.unit_at(P2, 2).id;
            s.attack(ID, first.as_str());
            // Its after-attack destroy fired; its Death hit the last Unit it damaged.
            assert_eq!(s.stats(first.as_str()).health, 10 - 1 - 2);
            assert_eq!(s.stats(second.as_str()).health, 10);
        }

        #[test]
        fn r3560003_remembered_from_a_strike_back_on_an_earlier_turn() {
            let mut s = weapon_board(
                "s120-strike",
                json!({ "field": [{ "def": ID, "lane": 1 }] }),
                json!({ "field": [{ "def": BIG, "lane": 1 }] }),
            );
            crate::next_turn_of(&mut s, P2);
            let crawler = s.unit_at(P1, 1).id;
            let big = s.unit_at(P2, 1).id;
            // The big Unit strikes back: the Crawler remembers the striker across turns.
            s.attack(big.as_str(), crawler.as_str());
            s.expect_in_zone(ID, "graveyard");
            // The Crawler strikes back for its 1, and its Death hits the Unit it damaged for 2.
            assert_eq!(s.stats(big.as_str()).health, 9 - 1 - 2);
        }

        #[test]
        fn r3560003_nothing_when_that_unit_is_gone_or_it_hit_only_the_hero() {
            // It hit only the hero: its Death hits nothing.
            let mut s = weapon_board("s120-hero", json!({ "field": [ID] }), json!({}));
            crate::next_turn_of(&mut s, P1);
            s.attack(ID, "hero");
            s.expect_health(P2, 30 - 1);
            // No Unit took Death damage: only the hero moved.
            // A Unit that left the field is hit no more: the damage fizzles.
            let mut s = weapon_board(
                "s120-gone",
                json!({ "field": [{ "def": ID, "lane": 1 }] }),
                json!({ "field": [{ "def": BODY, "lane": 1 }], "hand": [FILLER] }),
            );
            crate::next_turn_of(&mut s, P1);
            let victim = s.unit_at(P2, 1).id;
            s.attack(ID, victim.as_str());
            assert_eq!(s.stats(victim.as_str()).health, 10 - 1 - 2);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3560000_radiant_adds_a_radiant_toxopid() {
            let mut s = hand(true, vec![], json!({}));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(toxopids(&s).len(), 1);
            assert!(s.card(&toxopids(&s)[0]).radiant);
        }

        #[test]
        fn r3560003_radiant_death_deals_4() {
            let mut s = weapon_board(
                "s120-radiant-death",
                json!({ "field": [{ "def": ID, "radiant": true, "lane": 1 }] }),
                json!({ "field": [{ "def": BODY, "lane": 1 }] }),
            );
            crate::next_turn_of(&mut s, P1);
            let victim = s.unit_at(P2, 1).id;
            s.attack(ID, victim.as_str());
            assert_eq!(s.stats(victim.as_str()).health, 10 - 2 - 4);
        }
    }
}
