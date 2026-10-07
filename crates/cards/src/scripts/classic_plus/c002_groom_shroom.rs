//! C+ #2 Groom Shroom (SPEC §8.7 row 2, R405). (3) Trap, Felinor, Epic.
//! Fires in C+ #1's window (an enemy Unit's declared attack on your hero) and fills every open unit zone
//! of yours with a random non-token Felinor Unit of any set (R64, R380, R60), each summoned with no Cry
//! (R1) and granted Taunt; Radiant, on their Radiant faces. The attack still hits your hero (R405).

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-002";

/// R405: "Felinors" are Felinor-tagged Units; a pool names no set, so every set (R380).
fn felinor_units() -> Value {
    json!({ "type": "Unit", "tags": ["Felinor"] })
}

/// A declared attack (a forced one opens no window, R121) by an enemy Unit on this trap's controller's hero.
fn attacks_your_hero(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared { attacker_id, target_id, forced } = event else {
        return false;
    };
    if *forced {
        return false;
    }
    let target = attack_target_of(&ctx.state, target_id);
    find_instance(&ctx.state, attacker_id).map(|attacker| attacker.controller) == Some(opponent_of(ctx.controller))
        && matches!(target, Some(AttackTarget::Hero { player }) if player == ctx.controller)
}

/// `forEachCard`'s `cards`, typed (TS `(ctx) => readonly (CardInstance | string)[]`, ids here).
fn cards_of(f: impl Fn(&mut EffectContext<'_>) -> Vec<String> + Send + Sync + 'static) -> ForEachCardCards {
    Arc::new(f)
}

/// `forEachCard`'s `each`, typed (TS `(instanceId) => Effect`).
fn each_of(f: impl Fn(&str) -> Effect + Send + Sync + 'static) -> ForEachCardEach {
    Arc::new(f)
}

fn groom_shroom(radiant: bool) -> Script {
    Script {
        triggers: vec![
            TriggerDef::new("groom-shroom", &[GameEventType::AttackDeclared], move |ctx, _event| {
                let lanes: Vec<i32> =
                    fill_board_zones(&ctx.state, ctx.controller).iter().map(|zone| zone.lane).collect();
                let mut effects: Vec<Effect> = lanes
                    .iter()
                    .map(|&lane| {
                        summon_random(json_as(json!({ "query": felinor_units(), "lane": lane, "radiant": radiant })))
                    })
                    .collect();
                // "Give them Taunt": the Units now standing in the zones the fill took.
                effects.push(for_each_card(ForEachCardArgs {
                    cards: cards_of(move |now| {
                        let player = now.controller;
                        lanes
                            .iter()
                            .filter_map(|&lane| {
                                card_at(&now.state, &ZoneSlot { player, row: Row::Units, lane }).map(|card| card.id.clone())
                            })
                            .collect()
                    }),
                    each: each_of(|instance_id| {
                        grant_keyword(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "keyword": { "kind": "Taunt" },
                        })))
                    }),
                }));
                effects
            })
            .with_when(attacks_your_hero),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: groom_shroom(false),
        radiant: groom_shroom(true),
    }
}

// C+ #2 Groom Shroom — SPEC §8.7 row 2, BUILD M9 Classic+ row C+ 2: "Same window as C+ #1; fills
// every empty, unlocked, unreserved unit zone of yours with a random non-token Felinor Unit of any set
// (R64, R380; Felinor-tagged Units, never a Felinor Spell, R405; repeats allowed, R60), summoned with no
// Cry (R1) and granted Taunt; the declared attack still hits your hero and is never moved to a new
// Taunt, which arrived after §4.2 step 3 (R405); a full board summons nothing and draws no random
// number (R129); trap consumed; hidden until it fires (R33); radiant the Felinors are Radiant".
//
// Groom Shroom sits face-down in p1's backrow lane 1; p2 is active and attacks with a 3/3 Tempo Timmy.
// The two engine setups below (`lockZone`, `reserveZone`) put a Lock and a Reborn reservation on a
// zone, which no Core card can do to a unit zone.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const GROOM: &str = "classicplus-002";
    const TIMMY: &str = "core-011"; // (1) 3/3 Rush, First Strike.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-010";
    const STOCKPILE: &str = "core-005";
    /// Every non-token Felinor-tagged Unit of the three sets (R405, R380).
    const FELINOR_UNITS: &[&str] =
        &["core-012", "core-043", "core-086", "classic-047", "classicplus-030", "classicplus-046"];

    /// TS `{ ...base, ...extra }` on two object literals (a shallow merge; `extra`'s keys win).
    fn merged(base: Value, extra: Value) -> Value {
        let mut out = base;
        if let (Some(into), Value::Object(from)) = (out.as_object_mut(), extra) {
            for (key, value) in from {
                into.insert(key, value);
            }
        }
        out
    }

    fn setup(p1: Value, radiant_face: bool, seed: Option<&str>) -> Scenario {
        crate::register_all();
        let mut options = json!({
            "active": "p2",
            "p1": merged(
                json!({
                    "hand": [FILLER],
                    "library": [STOCKPILE],
                    "backrow": [{ "def": GROOM, "radiant": radiant_face, "faceUp": false, "lane": 1 }],
                }),
                p1,
            ),
            "p2": { "hand": [FILLER], "library": [STOCKPILE], "field": [TIMMY] },
        });
        if let Some(seed) = seed {
            options["seed"] = json!(seed);
        }
        scenario(options)
    }

    fn summoned_ids(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn count(s: &Scenario, type_: &str) -> usize {
        s.events().iter().filter(|event| event.event_type().as_str() == type_).count()
    }

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(|k| k.kind()).collect()
    }

    #[test]
    fn is_a_3_trap_tagged_felinor_each_face_declares_one_trap_trigger_on_attackdeclared() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.type_, CardType::Trap);
        assert_eq!(def.tags, vec![Tag::Felinor]);
        let scripts = script();
        let ons = |s: &Script| s.triggers.iter().map(|t| t.on.clone()).collect::<Vec<_>>();
        assert_eq!(ons(&scripts.base), vec![vec![GameEventType::AttackDeclared]]);
        assert_eq!(ons(&scripts.radiant), vec![vec![GameEventType::AttackDeclared]]);
    }

    mod base {
        use super::*;

        #[test]
        fn r64_r405_fills_every_empty_unit_zone_of_yours_with_a_non_token_felinor_unit_each_granted_taunt_no_cry() {
            crate::register_all();
            let mut s = setup(json!({ "field": [{ "def": VANILLA, "lane": 2 }] }), false, None);
            let groom = s.card(GROOM).clone();

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_events(json!(["attackDeclared", "trapFired", "summoned", "keywordGranted"]));
            let ids = summoned_ids(&s);
            assert_eq!(ids.len(), 4);
            assert_eq!(
                [1, 3, 4, 5].iter().map(|&lane| s.unit(P1, lane).map(|unit| unit.id)).collect::<Vec<_>>(),
                ids.iter().cloned().map(Some).collect::<Vec<_>>()
            );
            for id in &ids {
                let card = s.card(id).clone();
                let card_def = def_of(Some(s.state()), &card.def_id).clone();
                assert!(FELINOR_UNITS.contains(&card.def_id.as_str()));
                assert_eq!(card_def.type_, CardType::Unit);
                assert!(!card_def.token);
                assert_eq!(card.controller, P1);
                assert!(!card.radiant);
                assert!(kinds(&card.granted_keywords).contains(&KeywordKind::Taunt));
                assert!(kinds(&s.stats(&card).keywords).contains(&KeywordKind::Taunt));
            }
            // R1: summoned, not played — no Cry (a Duplicating Felinors' copy, a Big Felinor's sweep).
            assert_eq!(count(&s, "cardPlayed"), 0);
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(VANILLA.to_string()));
            s.expect_in_zone(&groom, "graveyard");
        }

        #[test]
        fn r405_the_declared_attack_still_hits_your_hero_the_new_taunts_arrived_after_s4_2_step_3() {
            crate::register_all();
            let mut s = setup(json!({}), false, None);

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_health(P1, 27);
            assert_eq!(count(&s, "redirected"), 0);
        }

        #[test]
        fn r60_r380_draws_from_every_set_with_repeats_allowed_and_never_a_felinor_spell_or_token() {
            crate::register_all();
            let mut seen: IndexSet<String> = IndexSet::new();
            let mut repeated = false;
            for seed in 1..=12 {
                let mut s = setup(json!({}), false, Some(&format!("groom-{seed}")));
                let attacker = s.unit(P2, 1).unwrap();
                s.attack(&attacker, "hero");
                let defs: Vec<String> = summoned_ids(&s).iter().map(|id| s.card(id).def_id.clone()).collect();
                assert_eq!(defs.len(), 5);
                // R1: no Cry ran — Big Felinor's would destroy the attacker, Felinor Fuser's would open a
                // Discover, Felinor Flagbearer's would give armor.
                assert_eq!(count(&s, "destroyed"), 0);
                assert!(s.state().pending.is_none());
                assert_eq!(s.state().players.p1.hero.armor, 0);
                if defs.iter().cloned().collect::<IndexSet<_>>().len() < defs.len() {
                    repeated = true;
                }
                for id in defs {
                    seen.insert(id);
                }
            }
            assert!(repeated);
            assert!(seen.iter().all(|id| FELINOR_UNITS.contains(&id.as_str())));
            assert!(seen.iter().any(|id| !id.starts_with("core-")));
            // The no-Cry checks above bit: the Felinors whose Cries would show were among those summoned.
            assert!(["core-043", "classicplus-030", "classicplus-046"].iter().all(|id| seen.contains(*id)));
            assert!(!seen.contains("core-062")); // Friend of Felinors, a Felinor Spell
            assert!(!seen.contains("core-t-felinor"));
        }

        #[test]
        fn r64_skips_a_locked_zone_and_a_zone_reserved_for_a_reborn_return() {
            crate::register_all();
            let mut s = setup(json!({}), false, None);
            lock_zone(s.state_mut(), &ZoneSlot { player: P1, row: Row::Units, lane: 2 });
            reserve_zone(s.state_mut(), &ZoneSlot { player: P1, row: Row::Units, lane: 4 });

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            assert_eq!(summoned_ids(&s).len(), 3);
            assert!(s.unit(P1, 2).is_none());
            assert!(s.unit(P1, 4).is_none());
        }

        #[test]
        fn r129_a_full_board_summons_nothing_and_draws_no_random_number_the_trap_is_still_consumed() {
            crate::register_all();
            // Five Mr. Vanilla, no Taunt among them, so the attack on the hero is legal.
            let mut s = setup(json!({ "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] }), false, None);
            let before = s.state().rng_cursor;

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            s.expect_events(json!(["trapFired"]));
            assert_eq!(summoned_ids(&s).len(), 0);
            assert_eq!(s.state().rng_cursor, before);
            s.expect_in_zone(GROOM, "graveyard");
            s.expect_health(P1, 27);
        }

        #[test]
        fn never_fires_on_an_attack_on_a_unit() {
            crate::register_all();
            let mut s = setup(json!({ "field": [VANILLA] }), false, None);

            let attacker = s.unit(P2, 1).unwrap();
            let target = s.unit(P1, 1).unwrap();
            s.attack(&attacker, &target);

            assert_eq!(count(&s, "trapFired"), 0);
            assert!(matches!(s.card(GROOM).zone, Zone::Field { row: Row::Backrow, lane: 1, .. }));
        }

        #[test]
        fn r33_r97_the_opponent_reads_only_a_face_down_card_until_it_fires() {
            crate::register_all();
            let mut s = setup(json!({}), false, None);
            assert!(serde_json::to_string(&s.view(P1).you.backrow).unwrap().contains(GROOM));
            assert!(!serde_json::to_string(&s.view(P2)).unwrap().contains(GROOM));

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            assert!(s.view(P2).opponent.graveyard.iter().any(|card| card.def_id == GROOM));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_felinors_are_summoned_on_their_radiant_faces_each_granted_taunt() {
            crate::register_all();
            let mut s = setup(json!({}), true, None);

            let attacker = s.unit(P2, 1).unwrap();
            s.attack(&attacker, "hero");

            let ids = summoned_ids(&s);
            assert_eq!(ids.len(), 5);
            for id in &ids {
                assert!(s.card(id).radiant);
                assert!(kinds(&s.stats(id).keywords).contains(&KeywordKind::Taunt));
            }
            s.expect_health(P1, 27);
        }
    }
}
