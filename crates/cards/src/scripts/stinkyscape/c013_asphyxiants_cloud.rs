//! S #13 Asphyxiants Cloud (SPEC §8.10 row S 13; §4.4, §6.1, §6.2, §10.5, R59, R783012, R1848002, R3041000):
//! (2) Spell, Rare.
//!   Base:    "Flashback / Deal {damage} damage to all enemies. Your next Spell this turn has Poisonous." — 1.
//!   Radiant: the same words with 2.
//!
//! Engine: Flashback is the catalog's keyword: the card in its owner's graveyard may be played from there
//! at its cost (R783012). The Cry deals `damage` to every enemy Unit and the enemy hero in one effect
//! (R59), then `give_next_spell_keyword` puts a `keywordNextSpell` modifier on its caster for this turn:
//! the next Spell they play face-up takes Poisonous at §10.5 step 4 for that resolution alone, a Unit,
//! Field Spell or Trap played meanwhile leaving it waiting, and the turn's cleanup ending it (R1848002).
//! The damage comes first, as the words read (R3041000). Its own damage has no Poisonous and the Radiant
//! face no Echo, and it draws nothing: until #3041 (`script_v0_5_11o`, kept, R767000) it had Poisonous
//! itself, drew {draw}, gave the modifier first and, Radiant, resolved twice (Echo 1).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-013";

fn next_spell_is_poisonous() -> Effect {
    give_next_spell_keyword(json_as(json!({ "keyword": { "kind": "Poisonous" } })))
}

/// Both faces: damage, then the modifier (R3041000); the Radiant one's 2 is catalog data.
pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                damage_all(json_as(json!({
                    "amount": param(&*ctx, "damage"),
                    "side": "enemy",
                    "heroes": true,
                }))),
                next_spell_is_poisonous(),
            ]
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

/// R767000: through v0.5.11o the Cloud had Poisonous of its own, gave the modifier first, dealt its
/// damage and drew `draw`, and its Radiant face had Echo 1; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                next_spell_is_poisonous(),
                damage_all(json_as(json!({
                    "amount": param(&*ctx, "damage"),
                    "side": "enemy",
                    "heroes": true,
                }))),
                draw(json_as(json!({ "count": param(&*ctx, "draw") }))),
            ]
        })),
        ..Script::default()
    };

    let radiant = Script {
        static_flags: Some(StaticFlags {
            echo: Some(1),
            ..StaticFlags::default()
        }),
        ..base.clone()
    };

    CardScripts { base, radiant }
}

// S #13 Asphyxiants Cloud — SPEC §8.10 row S 13, BUILD M12 row S 13 (R1848002, R3041000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, script_v0_5_11o};
    use crate::{filler, scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CLOUD: &str = ID;
    const GREASER: &str = "meditative-100"; // (2) Unit 7/7.
    const JILLIAX: &str = "core-056"; // (2) Unit 3/2: Rush, Taunt, Lifesteal, Divine Shield.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const WHIRLWIND: &str = "classicplus-021"; // (0) Spell: 1 damage to all Units.
    const FILLER: &str = "core-005";

    fn hero_health(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player].hero.health
    }

    fn cloud_board(radiant: bool, more: Value, mine: Value, theirs: Value) -> Scenario {
        crate::register_all();
        let mut hand = vec![json!({ "def": CLOUD, "radiant": radiant })];
        hand.extend(more.as_array().cloned().unwrap_or_default());
        scenario(json!({
            "p1": { "hand": hand, "field": mine, "mana": 10, "library": filler(6) },
            "p2": { "hand": [FILLER], "field": theirs, "library": filler(6) },
        }))
    }

    /// The Cloud as `legalActions` offers it from p1's graveyard (R783012): the play actions naming it.
    fn offered_from_the_graveyard(s: &Scenario, id: &str) -> usize {
        legal_actions(s.state(), P1)
            .iter()
            .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id))
            .count()
    }

    /// A play of the graveyard card, sent to `reduce` as `legalActions` offers it (the harness's `play`
    /// takes a card from a hand).
    fn flashed_back(s: &mut Scenario, id: &str) {
        let action = json_as::<Action>(json!({
            "type": "play",
            "playerId": "p1",
            "instanceId": id,
            "nonce": format!("cloud-flashback-{id}"),
        }));
        let result = reduce(s.state(), &action);
        assert_eq!(result.error, None, "the Flashback play was refused");
        *s.state_mut() = result.state;
    }

    mod base {
        use super::*;

        #[test]
        fn r1848002_one_hit_of_1_on_each_enemy_with_no_poisonous_of_its_own_and_no_draw() {
            let mut s = cloud_board(
                false,
                json!([FILLER]),
                json!([GREASER]),
                json!([GREASER, GREASER, JILLIAX]),
            );
            let (first, second, shielded) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3));
            let mine = s.unit_at(P1, 1);
            let held = s.hand(P1).len();

            s.play(CLOUD, json!({}));

            for unit in [&first, &second] {
                s.expect_in_zone(unit, "field");
                assert_eq!(s.card(unit).damage, 1, "its own hit is not Poisonous");
            }
            s.expect_in_zone(&shielded, "field");
            assert_eq!(s.card(&shielded).damage, 0, "the Divine Shield took it");
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 1);
            assert_eq!(s.card(&mine).damage, 0);
            assert_eq!(hero_health(&s, P1), HERO_HEALTH);
            assert_eq!(s.hand(P1).len(), held - 1, "the Cloud left the hand and nothing was drawn");
        }

        #[test]
        fn prints_flashback_not_poisonous() {
            let s = cloud_board(false, json!([FILLER]), json!([]), json!([]));
            let cloud = s.card(CLOUD).clone();
            let keywords = s.stats(&cloud).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Flashback));
            assert!(!has_keyword(&keywords, KeywordKind::Poisonous));
        }

        #[test]
        fn r783012_it_is_offered_from_the_graveyard_by_its_flashback_at_its_cost() {
            let mut s = cloud_board(false, json!([FILLER]), json!([]), json!([]));
            let id = s.card(CLOUD).id.clone();
            s.play(CLOUD, json!({}));
            s.expect_in_zone(&id, "graveyard");
            assert!(offered_from_the_graveyard(&s, &id) > 0, "legalActions lists the play from the graveyard");

            let mana = s.state().players.p1.mana.current;
            flashed_back(&mut s, &id);
            assert_eq!(s.state().players.p1.mana.current, mana - 2, "at its cost");
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2, "its damage again");
        }

        #[test]
        fn r1848002_the_next_spell_is_poisonous_and_the_one_after_is_not() {
            let mut s = cloud_board(
                false,
                json!([WHIRLWIND, WHIRLWIND, VANILLA, FILLER]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([]),
            );
            let greaser = s.unit_at(P1, 1);
            s.play(CLOUD, json!({}));

            let first = s.card(WHIRLWIND).id.clone();
            s.play(&first, json!({}));
            s.expect_in_zone(&greaser, "graveyard");

            s.play(VANILLA, json!({ "zone": 2 }));
            let vanilla = s.unit_at(P1, 2);
            s.play(WHIRLWIND, json!({}));
            s.expect_in_zone(&vanilla, "field");
            s.expect_stats(&vanilla, json!({ "attack": 4, "health": 3 }));
        }

        #[test]
        fn r1848002_a_unit_played_between_leaves_it_waiting() {
            let mut s = cloud_board(
                false,
                json!([VANILLA, WHIRLWIND, FILLER]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([]),
            );
            let greaser = s.unit_at(P1, 1);
            s.play(CLOUD, json!({}));

            s.play(VANILLA, json!({ "zone": 2 }));
            s.play(WHIRLWIND, json!({}));

            s.expect_in_zone(&greaser, "graveyard");
        }

        #[test]
        fn r1848002_it_ends_with_the_turn() {
            let mut s = cloud_board(
                false,
                json!([WHIRLWIND, FILLER]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([]),
            );
            let greaser = s.unit_at(P1, 1);
            s.play(CLOUD, json!({}));
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);

            s.play(WHIRLWIND, json!({}));

            s.expect_in_zone(&greaser, "field");
            s.expect_stats(&greaser, json!({ "attack": 7, "health": 6 }));
        }

        #[test]
        fn r3041000_it_deals_its_damage_before_it_gives_the_next_spell_poisonous() {
            let mut s = cloud_board(false, json!([FILLER]), json!([]), json!([{ "def": GREASER, "lane": 1 }]));
            s.play(CLOUD, json!({}));
            let events = s.last_events();
            let at = |wanted: &dyn Fn(&GameEvent) -> bool| events.iter().position(wanted);
            let hit = at(&|event| matches!(event, GameEvent::Damage { .. })).expect("the damage");
            let given = at(&|event| matches!(event, GameEvent::ModifierChanged { added: true, .. }))
                .expect("the modifier");
            assert!(hit < given, "damage at {hit}, the modifier at {given}: {events:?}");
            let last_hit = events.iter().rposition(|event| matches!(event, GameEvent::Damage { .. })).expect("a hit");
            assert!(last_hit < given, "every hit is dealt before the modifier is given");
        }

        #[test]
        fn damage_reads_through_param() {
            let mut s = cloud_board(false, json!([FILLER]), json!([]), json!([]));
            assert_eq!(upgrade_number(&mut s, CLOUD, "damage"), 2);

            s.play(CLOUD, json!({}));

            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2);
        }

        /// R767000: through v0.5.11o the Cloud had Poisonous of its own, drew 1 and gave the modifier
        /// first. Its definition then had `draw` and Poisonous in place of Flashback.
        #[test]
        fn r767000_the_kept_script_hits_poisonously_and_draws() {
            crate::under_kept_as(ID, script_v0_5_11o(), old_definition, || {
                let mut s = cloud_board(
                    false,
                    json!([FILLER]),
                    json!([GREASER]),
                    json!([GREASER, GREASER, JILLIAX]),
                );
                let (first, second, shielded) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3));
                let held = s.hand(P1).len();

                s.play(CLOUD, json!({}));

                s.expect_in_zone(&first, "graveyard");
                s.expect_in_zone(&second, "graveyard");
                s.expect_in_zone(&shielded, "field");
                assert_eq!(hero_health(&s, P2), HERO_HEALTH - 1);
                assert_eq!(s.hand(P1).len(), held, "one drawn for the Cloud played");
                let id = s.card(CLOUD).id.clone();
                s.expect_in_zone(&id, "graveyard");
                assert_eq!(offered_from_the_graveyard(&s, &id), 0, "no Flashback, so no play from there");
                let events = s.last_events();
                let given = events
                    .iter()
                    .position(|event| matches!(event, GameEvent::ModifierChanged { added: true, .. }))
                    .expect("the modifier");
                let hit = events
                    .iter()
                    .position(|event| matches!(event, GameEvent::Damage { .. }))
                    .expect("the damage");
                assert!(given < hit, "the modifier was given first");
            });
        }
    }

    /// The Cloud's definition through v0.5.11o: Poisonous, `draw` 1 and `damage` 1 on both faces.
    fn old_definition(def: &mut CardDef) {
        for face in [&mut def.base, &mut def.radiant] {
            face.keywords = vec![Keyword::Poisonous];
        }
        let params = def.params.get_or_insert_with(Vec::new);
        for param in params.iter_mut() {
            param.radiant = param.base;
        }
        params.push(json_as(json!({
            "key": "draw", "base": 1, "radiant": 1, "better": "up", "step": 1, "min": 1,
        })));
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1848002_one_hit_of_2_no_echo_no_draw_and_the_next_spell_poisonous() {
            let mut s = cloud_board(
                true,
                json!([FILLER, WHIRLWIND]),
                json!([{ "def": GREASER, "lane": 1 }]),
                json!([{ "def": JILLIAX, "lane": 1 }]),
            );
            let shielded = s.unit_at(P2, 1);
            let greaser = s.unit_at(P1, 1);
            let held = s.hand(P1).len();

            s.play(CLOUD, json!({}));

            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2, "once, with no Echo");
            assert_eq!(s.hand(P1).len(), held - 1, "nothing drawn");
            s.expect_in_zone(&shielded, "field");
            assert_eq!(s.card(&shielded).damage, 0, "the Divine Shield took the 2");
            let hits = s
                .last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Damage { target_id, .. } if target_id == "hero-p2"))
                .count();
            assert_eq!(hits, 1);

            s.play(WHIRLWIND, json!({}));
            s.expect_in_zone(&greaser, "graveyard");
        }

        #[test]
        fn r783012_it_is_offered_from_the_graveyard_by_its_flashback_too() {
            let mut s = cloud_board(true, json!([FILLER]), json!([]), json!([]));
            let id = s.card(CLOUD).id.clone();
            let keywords = s.stats(s.card(CLOUD).clone()).keywords;
            assert!(has_keyword(&keywords, KeywordKind::Flashback));
            s.play(CLOUD, json!({}));
            assert!(offered_from_the_graveyard(&s, &id) > 0);
            flashed_back(&mut s, &id);
            assert_eq!(hero_health(&s, P2), HERO_HEALTH - 4);
        }

        /// R767000: through v0.5.11o it all happened twice (Echo 1) and drew 1 each time.
        #[test]
        fn r767000_the_kept_script_all_happens_twice() {
            crate::under_kept_as(ID, script_v0_5_11o(), old_definition, || {
                let mut s = cloud_board(true, json!([FILLER]), json!([]), json!([{ "def": JILLIAX, "lane": 1 }]));
                let shielded = s.unit_at(P2, 1);
                let held = s.hand(P1).len();

                s.play(CLOUD, json!({}));

                assert_eq!(hero_health(&s, P2), HERO_HEALTH - 2);
                assert_eq!(s.hand(P1).len(), held + 1, "two drawn for the Cloud played");
                s.expect_in_zone(&shielded, "graveyard");
            });
        }
    }
}
