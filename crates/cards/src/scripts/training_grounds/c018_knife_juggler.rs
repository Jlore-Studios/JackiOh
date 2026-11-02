//! TG #18 Knife Juggler (SPEC §8.11 row 18; R2197000): (1) Unit, Human, Rare, 3/2 → 6/4, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base and Radiant: "Pierce / After you summon a Unit, deal {damage} damage to a random enemy." — damage
//!   1, Radiant 3
//! Arrival trigger (S #23, R1875003, R2197000): friendly unit arrival (Recruit, token, copy, Reborn, resolved play after Cry as C #5; not itself).
//! Deals damage to random enemy (damage_random_enemies, R60). Source unit prints Pierce (§6.1). Tunes: damage 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-018";

/// R2197000: the Unit this event brings onto its controller's side, or None: a summon of a Unit onto it
/// (Recruit, a token, a copy, Reborn), or a played one once it resolves — its play's own placement (§10.5
/// step 4, which carries `exitsFrom`) waits for that. Never this card itself.
fn arrival(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let state: &GameState = &*ctx.state;
    let me = ctx.self_.as_ref()?;
    let instance = match event {
        GameEvent::Summoned {
            player,
            row,
            exits_from,
            instance_id,
            ..
        } => {
            if *player != ctx.controller || exits_from.is_some() {
                return None;
            }
            // R1081000: a Unit summoned standing in a backrow zone is a summoned Unit too.
            let unit = find_instance(state, instance_id)?;
            (*row == Row::Units || stands_in_backrow(state, unit)).then(|| instance_id.clone())?
        }
        GameEvent::CardResolved {
            permanent,
            def_id,
            instance_id,
            ..
        } => {
            if !*permanent || def_of(Some(state), def_id).type_ != CardType::Unit {
                return None;
            }
            let unit = find_instance(state, instance_id)?;
            (unit.zone.z() == ZoneName::Field && unit.controller == ctx.controller).then(|| instance_id.clone())?
        }
        _ => return None,
    };
    (instance != me.id).then_some(instance)
}

fn juggler() -> Script {
    let throw = TriggerDef::new(
        "knife-juggler",
        &[GameEventType::Summoned, GameEventType::CardResolved],
        |ctx, event| {
            if arrival(ctx, event).is_none() {
                return Vec::new();
            }
            vec![damage_random_enemies(json_as(json!({ "count": 1, "amount": param(&*ctx, "damage") })))]
        },
    )
    .with_when(|ctx, event| arrival(ctx, event).is_some());
    Script {
        triggers: vec![throw],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face is the same text at 3 damage, a catalog value.
    CardScripts {
        base: juggler(),
        radiant: juggler(),
    }
}

// TG #18 Knife Juggler — SPEC §8.11 row 18, BUILD M13 row TG 18 (Core #15, TG #16, #2196).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, Cry: Summon a Rush Token.
    const CALL_TO_ARMS: &str = "core-069"; // (2) Spell: Recruit 3 Units of (1) Cost or less.
    const DEFENDER: &str = "core-003"; // (1) Right-house defender, 1/1 Taunt, Divine Shield, Reborn.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const ZAO_GAO: &str = "core-080"; // (2) Spell: discard 2 random cards, summon 2 Rush Tokens.
    const RUSH_TOKEN: &str = "core-t-rush";

    /// p1, active with 10 mana, has a Knife Juggler (`radiant` its face) in lane 5 and `hand`; p2 has a
    /// 1/14 in lane 1 so a hit may land on a Unit.
    fn board(radiant: bool, hand: Value, library: Value) -> Scenario {
        crate::scenario(json!({
            "seed": "knife-juggler",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant, "lane": 5 }],
                "hand": hand,
                "library": library,
                "mana": 10,
            },
            "p2": { "field": [MOTHS], "hand": [VANILLA, FILLER], "library": crate::filler(6), "mana": 10 },
        }))
    }

    fn juggler(s: &Scenario) -> String {
        s.unit_at(P1, 5).id
    }

    /// The Juggler's hits among the last step's events: (target, amount).
    fn throws(s: &Scenario) -> Vec<(String, i32)> {
        let me = juggler(s);
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id: Some(source),
                    target_id,
                    amount,
                    ..
                } if *source == me => Some((target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    /// Whether `target` is an enemy of p1's: p2's hero or a Unit of p2's.
    fn is_enemy(s: &Scenario, target: &str) -> bool {
        target == "hero-p2" || s.unit(P2, 1).is_some_and(|unit| unit.id == target)
    }

    mod base {
        use super::*;

        #[test]
        fn it_prints_pierce() {
            let s = board(false, json!([FILLER]), crate::filler(4));
            let me = juggler(&s);
            assert!(unit_has(s.state(), s.card(me.as_str()), KeywordKind::Pierce));
            let view = s.stats(me.as_str());
            assert_eq!((view.attack, view.max_health), (3, 2));
        }

        #[test]
        fn r2197000_a_unit_you_play_draws_one_hit_on_a_random_enemy_after_it_resolves() {
            let mut s = board(false, json!([VANILLA, FILLER]), crate::filler(4));
            let cursor = s.state().rng_cursor;
            s.play(VANILLA, json!({ "zone": 1 }));
            let hits = throws(&s);
            assert_eq!(hits.len(), 1, "{hits:?}");
            assert_eq!(hits[0].1, 1);
            assert!(is_enemy(&s, &hits[0].0), "{hits:?}");
            assert!(s.state().rng_cursor > cursor, "drawn from the match rng");
            // The hit follows the play's resolution.
            let types: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
            let resolved = types.iter().position(|t| *t == GameEventType::CardResolved);
            let hit = types.iter().rposition(|t| *t == GameEventType::Damage);
            assert!(resolved < hit, "{types:?}");
        }

        #[test]
        fn r2197000_a_recruit_draws_a_hit() {
            let mut s = board(false, json!([CALL_TO_ARMS, FILLER]), json!([VANILLA, FILLER, FILLER]));
            s.play(CALL_TO_ARMS, json!({}));
            assert_eq!(s.unit_at(P1, 1).def_id, VANILLA, "the Recruit");
            assert_eq!(throws(&s).len(), 1);
        }

        #[test]
        fn r2197000_a_token_your_spell_summons_draws_a_hit() {
            // Core #80 Zao Gao, a Spell, summons two Rush Tokens: each is an arrival, so two hits.
            let mut s = board(false, json!([ZAO_GAO, FILLER, FILLER, FILLER]), crate::filler(4));
            s.play(ZAO_GAO, json!({}));
            let tokens = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Summoned { def_id, player, .. } if def_id == RUSH_TOKEN && *player == P1))
                .count();
            assert_eq!(tokens, 2, "the Spell summoned two tokens");
            let hits = throws(&s);
            assert_eq!(hits.len(), 2, "{hits:?}");
            assert!(hits.iter().all(|(target, amount)| *amount == 1 && is_enemy(&s, target)), "{hits:?}");
        }

        #[test]
        fn r2197000_a_reborn_return_draws_a_hit() {
            let mut s = crate::scenario(json!({
                "seed": "knife-juggler-reborn",
                "p1": {
                    "field": [DEFENDER, { "def": ID, "lane": 5 }],
                    "hand": [HIT_JOB, FILLER],
                    "library": crate::filler(4),
                    "mana": 10,
                },
                "p2": { "field": [MOTHS], "hand": [FILLER], "library": crate::filler(4) },
            }));
            let defender = s.unit_at(P1, 1).id;
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": defender }] }));
            assert!(
                s.last_events().iter().any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == DEFENDER)),
                "Reborn returned it"
            );
            assert_eq!(throws(&s).len(), 1);
        }

        #[test]
        fn r2197000_two_arrivals_a_cry_token_and_the_played_unit_give_two_hits() {
            let mut s = board(false, json!([MR_TOKEN, FILLER]), crate::filler(4));
            s.play(MR_TOKEN, json!({ "zone": 1 }));
            let hits = throws(&s);
            assert_eq!(hits.len(), 2, "{hits:?}");
            assert!(hits.iter().all(|(target, amount)| *amount == 1 && is_enemy(&s, target)));
        }

        #[test]
        fn r2197000_its_own_arrival_does_not_count() {
            let mut s = crate::scenario(json!({
                "seed": "knife-juggler-own",
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4), "mana": 10 },
                "p2": { "field": [MOTHS], "hand": [FILLER], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 5 }));
            assert!(crate::hits(s.last_events()).is_empty());
        }

        #[test]
        fn r2197000_a_unit_the_opponent_summons_does_not_count() {
            let mut s = board(false, json!([FILLER]), crate::filler(4));
            crate::next_turn_of(&mut s, P2);
            s.play(VANILLA, json!({ "zone": 2 }));
            assert!(throws(&s).is_empty());
        }

        #[test]
        fn s6_1_its_pierce_applies_an_armor_2_enemy_unit_with_1_health_dies_to_it() {
            // The hit's enemy is a match-rng draw between the hero and the Unit: the first seed whose
            // draw lands on the Unit shows Pierce at work.
            for n in 0..32 {
                let mut s = crate::scenario(json!({
                    "seed": format!("knife-juggler-pierce-{n}"),
                    "p1": {
                        "field": [{ "def": ID, "lane": 5 }],
                        "hand": [VANILLA, FILLER],
                        "library": crate::filler(4),
                        "mana": 10,
                    },
                    "p2": {
                        "field": [{ "def": VANILLA, "damage": 3 }],
                        "hand": [FILLER],
                        "library": crate::filler(4),
                    },
                }));
                let armored = s.unit_at(P2, 1).id;
                s.card_mut(armored.as_str()).granted_keywords = vec![json_as(json!({ "kind": "Armor", "n": 2 }))];
                assert_eq!(s.stats(armored.as_str()).armor, 2);
                s.play(VANILLA, json!({ "zone": 1 }));
                if throws(&s).iter().any(|(target, _)| *target == armored) {
                    s.expect_in_zone(armored.as_str(), "graveyard");
                    return;
                }
            }
            panic!("no seed's draw landed on the Unit");
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = board(false, json!([VANILLA, FILLER]), crate::filler(4));
            let me = juggler(&s);
            assert_eq!(crate::upgrade_number(&mut s, &me, "damage"), 2);
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(throws(&s).iter().map(|(_, amount)| *amount).collect::<Vec<_>>(), vec![2]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_is_6_4_with_pierce() {
            let s = board(true, json!([FILLER]), crate::filler(4));
            let me = juggler(&s);
            let view = s.stats(me.as_str());
            assert_eq!((view.attack, view.max_health), (6, 4));
            assert!(unit_has(s.state(), s.card(me.as_str()), KeywordKind::Pierce));
        }

        #[test]
        fn r2197000_each_arrival_deals_3() {
            let mut s = board(true, json!([MR_TOKEN, FILLER]), crate::filler(4));
            s.play(MR_TOKEN, json!({ "zone": 1 }));
            assert_eq!(throws(&s).iter().map(|(_, amount)| *amount).collect::<Vec<_>>(), vec![3, 3]);
        }
    }
}
