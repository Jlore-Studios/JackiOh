//! TG #30 Floodgates (SPEC §8.11 row 30; R2197003, R2197005): (3) Unit, Epic, 0/12 → 0/24, a card of the
//! held Training Grounds set (R1420, R2194000).
//!   Base:    "Armor 3 / Whenever a Unit is summoned, this gains +{armorGain} Armor. / Death: Destroy all your
//!            Units." — armorGain 1
//!   Radiant: the same with Armor 6 — armorGain 2
//! Engine: Armor 3 and Armor 6 are printed keywords (§6.1, R1360). The trigger is S #23 Warp Disruptor's
//!   arrival (R1875003) on either side (R2197003), never Floodgates' own; each grants Armor {armorGain}
//!   (`grant_keyword`; Armor stacks), kept until it leaves the field (R78). Death: `destroy_all` of its
//!   controller's Units as it died (R89). Tunes: armorGain 1 ↑.
//! The end (REVIEW A3.6, R2197005): a fused self-resummon (S #112 Job Application's) stays, and two such
//!   bodies on one side cannot destroy each other's return forever.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-030";

/// R2197003: the Unit this event brings onto either side: a summon, or a played one once it resolves (its
/// own placement, §10.5 step 4, carries `exitsFrom` and waits for that). Never this card itself.
fn arrival(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let state: &GameState = &*ctx.state;
    let me = ctx.self_.as_ref()?;
    let instance = match event {
        GameEvent::Summoned {
            row,
            exits_from,
            instance_id,
            ..
        } => {
            if exits_from.is_some() {
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
            (unit.zone.z() == ZoneName::Field).then(|| instance_id.clone())?
        }
        _ => return None,
    };
    (instance != me.id).then_some(instance)
}

fn floodgates() -> Script {
    let gain = TriggerDef::new(
        "floodgates",
        &[GameEventType::Summoned, GameEventType::CardResolved],
        |ctx, event| {
            if arrival(ctx, event).is_none() {
                return Vec::new();
            }
            vec![grant_keyword(json_as(json!({
                "target": { "of": "self" },
                "keyword": { "kind": "Armor", "n": param(&*ctx, "armorGain") },
            })))]
        },
    )
    .with_when(|ctx, event| arrival(ctx, event).is_some());
    Script {
        triggers: vec![gain],
        // R89: "your Units" are its controller's as it died.
        death: Some(hook(|ctx| {
            let Some(this) = ctx.self_.as_ref() else {
                return vec![];
            };
            // R2197005: a fused or granted body that arrived this turn destroys nothing.
            if this.def_id != ID && this.summoned_turn == Some(ctx.state.turn) {
                return vec![];
            }
            // R2197005: not the body it dies on, so its own resummon (S #112 Job Application's) stays.
            vec![destroy_all(json_as(json!({ "side": "self", "excludeSelf": true })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face is the same text at Armor 6 and +2, catalog values.
    CardScripts {
        base: floodgates(),
        radiant: floodgates(),
    }
}

// TG #30 Floodgates — SPEC §8.11 row 30, BUILD M13 row TG 30 (S #112, R2197005, Core #12, TG #23, #2196).
// Armor gain on unit summons; Death destroys all own units.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const MR_TOKEN: &str = "core-015"; // (1) 1/1, Cry: Summon a Rush Token.
    const DUPLICATING: &str = "core-012"; // (2) 3/4, Cry: Summon a copy of this.
    const CALL_TO_ARMS: &str = "core-069"; // (2) Spell: Recruit 3 Units of (1) Cost or less.
    const FARM: &str = "core-058"; // (2) Field Spell Rush Token Farm.
    const HONEYPOT: &str = "core-060"; // (1) Trap Bear Honeypot.
    const STATE_OF_THE_GAME: &str = "classic-041"; // (1) 3/3 Indestructible.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const FLOOD: &str = "core-017"; // (4) Spell: Bounce all Units.
    const COLLATERAL: &str = "core-034"; // (4) Spell: Exile a permanent and a random card from the enemy deck.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    /// Floodgates stands in p1's lane 5; p2 holds a Mr. Vanilla to play on its turn.
    fn board(radiant: bool, mine: Value, hand: Value, theirs: Value) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant, "lane": 5 })];
        field.extend(mine.as_array().cloned().unwrap_or_default());
        crate::scenario(json!({
            "seed": "floodgates",
            "p1": { "field": field, "hand": hand, "library": json!([VANILLA, FILLER, FILLER, FILLER]), "mana": 30 },
            "p2": { "field": theirs, "hand": [VANILLA, FILLER], "library": crate::filler(6), "mana": 30 },
        }))
    }

    fn gates(s: &Scenario) -> String {
        s.unit_at(P1, 5).id
    }

    fn armor(s: &Scenario) -> i32 {
        s.stats(gates(s).as_str()).armor
    }

    mod base {
        use super::*;

        #[test]
        fn s6_1_it_is_0_12_with_armor_3_and_a_5_damage_hit_deals_2() {
            let mut s = board(false, json!([]), json!([FILLER]), json!([VANILLA]));
            let me = gates(&s);
            let view = s.stats(me.as_str());
            assert_eq!((view.attack, view.max_health, view.armor), (0, 12, 3));
            let theirs = s.unit_at(P2, 1).id;
            s.card_mut(theirs.as_str()).buffs = AttackHealth { attack: 1, health: 0 };
            crate::next_turn_of(&mut s, P2);
            s.attack(theirs.as_str(), me.as_str());
            assert_eq!(s.card(me.as_str()).damage, 2);
        }

        #[test]
        fn r2197003_a_unit_you_play_gives_1_armor() {
            let mut s = board(false, json!([]), json!([VANILLA, FILLER]), json!([]));
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(armor(&s), 4);
        }

        #[test]
        fn r2197003_an_enemy_played_unit_gives_1_armor() {
            let mut s = board(false, json!([]), json!([FILLER]), json!([]));
            crate::next_turn_of(&mut s, P2);
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(armor(&s), 4);
        }

        #[test]
        fn r2197003_a_cry_token_and_its_played_unit_give_1_each() {
            let mut s = board(false, json!([]), json!([MR_TOKEN, FILLER]), json!([]));
            s.play(MR_TOKEN, json!({ "zone": 1 }));
            assert_eq!(armor(&s), 5);
        }

        #[test]
        fn r2197003_a_recruit_gives_1() {
            let mut s = board(false, json!([]), json!([CALL_TO_ARMS, FILLER]), json!([]));
            s.play(CALL_TO_ARMS, json!({}));
            assert_eq!(s.unit_at(P1, 1).def_id, VANILLA, "the Recruit");
            assert_eq!(armor(&s), 4);
        }

        #[test]
        fn r2197003_a_copy_and_its_played_unit_give_1_each() {
            let mut s = board(false, json!([]), json!([DUPLICATING, FILLER]), json!([]));
            s.play(DUPLICATING, json!({ "zone": 1 }));
            assert_eq!(s.unit_at(P1, 2).def_id, DUPLICATING, "the copy");
            assert_eq!(armor(&s), 5);
        }

        #[test]
        fn r2197003_a_field_spell_or_a_trap_placed_gives_nothing() {
            let mut s = board(false, json!([]), json!([FARM, HONEYPOT, FILLER]), json!([]));
            s.play(FARM, json!({ "zone": 1 }));
            s.play(HONEYPOT, json!({ "zone": 2 }));
            assert_eq!(armor(&s), 3);
        }

        #[test]
        fn r2197003_its_own_arrival_gives_nothing() {
            let mut s = crate::scenario(json!({
                "seed": "floodgates-own",
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4), "mana": 10 },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.stats(ID).armor, 3);
        }

        #[test]
        fn r2197003_r89_destroyed_it_destroys_your_other_units_an_indestructible_one_staying_and_the_enemys_untouched() {
            let mut s = board(
                false,
                json!([{ "def": VANILLA, "lane": 1 }, { "def": STATE_OF_THE_GAME, "lane": 2 }]),
                json!([HIT_JOB, FILLER]),
                json!([VANILLA]),
            );
            let me = gates(&s);
            let mine = s.unit_at(P1, 1).id;
            let warded = s.unit_at(P1, 2).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
            s.expect_in_zone(me.as_str(), "graveyard");
            s.expect_in_zone(mine.as_str(), "graveyard");
            s.expect_in_zone(warded.as_str(), "field");
            s.expect_in_zone(theirs.as_str(), "field");
        }

        #[test]
        fn s6_3_bounced_it_has_no_death() {
            let mut s = board(false, json!([]), json!([FLOOD, FILLER]), json!([]));
            let me = gates(&s);
            s.play(FLOOD, json!({}));
            s.expect_in_zone(me.as_str(), "hand");
            assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Destroyed { .. })));
        }

        #[test]
        fn s6_3_exiled_it_has_no_death_and_your_units_stay() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1 }]), json!([COLLATERAL, FILLER]), json!([]));
            let me = gates(&s);
            let mine = s.unit_at(P1, 1).id;
            s.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
            s.expect_in_zone(me.as_str(), "exile");
            s.expect_in_zone(mine.as_str(), "field");
        }

        #[test]
        fn armor_gain_reads_through_param() {
            let mut s = board(false, json!([]), json!([VANILLA, FILLER]), json!([]));
            let me = gates(&s);
            assert_eq!(crate::upgrade_number(&mut s, &me, "armorGain"), 2);
            s.play(VANILLA, json!({ "zone": 1 }));
            assert_eq!(armor(&s), 5);
        }
    }

    /// R2197005: Floodgates' Death on a body that resummons itself (S #112 Job Application fused in), as
    /// the AI sweep's `sweep:hard:traininggrounds-014:6` found.
    mod r2197005_a_fused_self_resummon_ends {
        use super::*;

        const JOB_APPLICATION: &str = "stinkyscape-112";

        /// Fuses Job Application into the Unit `id`, as `fuseCards` does.
        fn fuse_job_application(s: &mut Scenario, id: &str, controller: PlayerId) -> String {
            fuse_into(s, id, &[JOB_APPLICATION], controller)
        }

        /// Fuses `defs` into the Unit `id` (R77): their scripts run after the Unit's own, in this order.
        fn fuse_into(s: &mut Scenario, id: &str, defs: &[&str], controller: PlayerId) -> String {
            let effect = jackioh_engine::effects::fuse_cards(json_as(json!({
                "instanceIds": [id],
                "defIds": defs,
                "targetInstanceId": id,
            })));
            crate::run_as(s, effect, controller);
            let fused = s.card(id).clone();
            assert!(fused.def_id.starts_with("t-"), "fused into a new definition: {}", fused.def_id);
            fused.id
        }

        fn deaths_of(s: &Scenario, id: &str) -> usize {
            s.last_events()
                .iter()
                .filter(|event| matches!(event, GameEvent::Destroyed { instance_id, .. } if instance_id == id))
                .count()
        }

        #[test]
        fn r2197005_a_fused_body_resummons_once_and_stays() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1 }]), json!([HIT_JOB, FILLER]), json!([]));
            let id = gates(&s);
            let me = fuse_job_application(&mut s, &id, P1);
            let mine = s.unit_at(P1, 1).id;
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
            assert_eq!(deaths_of(&s, &me), 1, "it died once");
            s.expect_in_zone(me.as_str(), "field");
            s.expect_in_zone(mine.as_str(), "graveyard");
        }

        /// The sweep's order: Job Application's resummon before Floodgates' sweep, on a third Unit.
        #[test]
        fn r2197005_a_resummon_before_the_sweep_ends() {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 1 }]), json!([HIT_JOB, FILLER]), json!([]));
            let host = s.unit_at(P1, 1).id;
            let me = fuse_into(&mut s, &host, &[JOB_APPLICATION, super::ID], P1);
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
            assert!(deaths_of(&s, &me) <= 2, "it died at most twice");
            s.expect_in_zone(me.as_str(), "field");
        }

        #[test]
        fn r2197005_two_fused_bodies_on_one_side_end() {
            let mut s = board(
                false,
                json!([{ "def": super::ID, "lane": 1 }]),
                json!([HIT_JOB, FILLER]),
                json!([]),
            );
            let (a, b) = (gates(&s), s.unit_at(P1, 1).id);
            let first = fuse_job_application(&mut s, &a, P1);
            let second = fuse_job_application(&mut s, &b, P1);
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": first }] }));
            assert!(deaths_of(&s, &first) <= 2 && deaths_of(&s, &second) <= 2, "each dies at most twice");
            assert!(s.state().result.is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_is_0_24_with_armor_6() {
            let s = board(true, json!([]), json!([FILLER]), json!([]));
            let view = s.stats(gates(&s).as_str());
            assert_eq!((view.attack, view.max_health, view.armor), (0, 24, 6));
        }

        #[test]
        fn r2197003_each_arrival_gives_2_armor() {
            let mut s = board(true, json!([]), json!([MR_TOKEN, FILLER]), json!([]));
            s.play(MR_TOKEN, json!({ "zone": 1 }));
            assert_eq!(armor(&s), 10);
        }

        #[test]
        fn r89_its_death_destroys_your_other_units() {
            let mut s = board(true, json!([{ "def": VANILLA, "lane": 1 }]), json!([HIT_JOB, FILLER]), json!([VANILLA]));
            let me = gates(&s);
            let mine = s.unit_at(P1, 1).id;
            let theirs = s.unit_at(P2, 1).id;
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": me }] }));
            s.expect_in_zone(mine.as_str(), "graveyard");
            s.expect_in_zone(theirs.as_str(), "field");
        }
    }
}
