//! S #15 Arg Arg Defender (SPEC §8.10 row 15): (1) Unit, Human, Common, 2/3 → 4/6, of the
//! Stinkyscape set.
//!   Base:    "Cry: Give adjacent Units +{buff}/+{buff} and Taunt."
//!   Radiant: "Cry: Give adjacent Units +{buff}/+{buff}, Divine Shield and Taunt."
//! Engine: Cry (R1): each Unit in the unit zones beside it on its own side (§3.1, `adjacent_to`, as C+
//! #20 Mushroom Power reads them) gets a +{buff}/+{buff} buff (kept, R78) and Taunt (`grant_keyword`);
//! an empty neighbour gets nothing, and the enemy's Unit across the lane is no neighbour. The buff is
//! its Cry's number (R1748000). Radiant: +2/+2, Divine Shield and Taunt.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-015";

/// A `for_each_card` of `each` over the Units beside this one on its side, read as the clause begins.
fn beside(each: impl Fn(&str) -> Effect + Send + Sync + 'static) -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(|ctx: &mut EffectContext<'_>| {
            adjacent_to(ctx, &json_as(json!({ "of": "self" })), &BoardScope::default())
                .into_iter()
                .map(|card| card.id)
                .collect()
        }),
        each: Arc::new(each),
    })
}

fn grant(kind: &'static str) -> Effect {
    beside(move |instance_id: &str| {
        grant_keyword(json_as(json!({
            "target": { "of": "instance", "instanceId": instance_id },
            "keyword": { "kind": kind },
        })))
    })
}

fn defender(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let amount = param(&*ctx, "buff");
            let mut effects = vec![beside(move |instance_id: &str| {
                buff(json_as(json!({
                    "target": { "of": "instance", "instanceId": instance_id },
                    "attack": amount,
                    "health": amount,
                })))
            })];
            if radiant {
                effects.push(grant("Divine Shield"));
            }
            effects.push(grant("Taunt"));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: defender(false),
        radiant: defender(true),
    }
}

// S #15 Arg Arg Defender — SPEC §8.10 row 15, BUILD M12 row S 15.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    fn holding(radiant: bool, p1_field: Value, p2_field: Value) -> Scenario {
        crate::scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": p1_field },
            "p2": { "hand": [FILLER], "field": p2_field },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    mod base {
        use super::*;

        #[test]
        fn both_neighbours_get_plus1_and_taunt() {
            let mut s = holding(
                false,
                json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]),
                json!([]),
            );
            s.play(ID, json!({ "zone": 2 }));
            for lane in [1, 3] {
                let unit = s.unit_at(P1, lane);
                s.expect_stats(&unit, json!({ "attack": 5, "health": 5 }));
                assert!(has(&s, &unit, KeywordKind::Taunt), "lane {lane}");
                assert!(!has(&s, &unit, KeywordKind::DivineShield), "lane {lane}");
            }
            let defender = s.unit_at(P1, 2);
            s.expect_stats(&defender, json!({ "attack": 2, "health": 3 }));
            assert!(!has(&s, &defender, KeywordKind::Taunt), "not itself");
        }

        #[test]
        fn an_empty_side_and_the_enemy_across_get_nothing() {
            let mut s = holding(false, json!([{ "def": VANILLA, "lane": 2 }]), json!([VANILLA]));
            s.play(ID, json!({ "zone": 1 }));
            let neighbour = s.unit_at(P1, 2);
            s.expect_stats(&neighbour, json!({ "attack": 5, "health": 5 }));
            assert!(has(&s, &neighbour, KeywordKind::Taunt));
            let across = s.unit_at(P2, 1);
            s.expect_stats(&across, json!({ "attack": 4, "health": 4 }));
            assert!(!has(&s, &across, KeywordKind::Taunt));
            let buffed = s
                .events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Buffed { .. }))
                .count();
            assert_eq!(buffed, 1, "the empty side and the enemy get nothing");
        }

        #[test]
        fn buff_reads_through_param() {
            let mut s = holding(false, json!([{ "def": VANILLA, "lane": 2 }]), json!([]));
            step_param(s.card_mut(ID), "buff", 1);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(s.unit_at(P1, 2), json!({ "attack": 6, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn plus2_divine_shield_and_taunt() {
            let mut s = holding(
                true,
                json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]),
                json!([]),
            );
            s.play(ID, json!({ "zone": 2 }));
            s.expect_stats(s.unit_at(P1, 2), json!({ "attack": 4, "health": 6 }));
            for lane in [1, 3] {
                let unit = s.unit_at(P1, lane);
                s.expect_stats(&unit, json!({ "attack": 6, "health": 6 }));
                assert!(has(&s, &unit, KeywordKind::Taunt), "lane {lane}");
                assert!(has(&s, &unit, KeywordKind::DivineShield), "lane {lane}");
            }
        }
    }
}
