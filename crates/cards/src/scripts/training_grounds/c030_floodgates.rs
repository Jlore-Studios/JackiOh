//! TG #30 Floodgates (SPEC §8.11 row 30; R2197003): (3) Unit, Epic, 0/12 → 0/24, a card of the held
//! Training Grounds set (R1420, R2194000).
//!   Base:    "Armor 3 / Whenever a Unit is summoned, this gains +{armorGain} Armor. / Death: Destroy all your
//!            Units." — armorGain 1
//!   Radiant: the same with Armor 6 — armorGain 2
//! Engine: Armor 3 and Armor 6 are printed keywords (§6.1, R1360). The trigger is S #23 Warp Disruptor's
//!   arrival (R1875003) on either side (R2197003): a Unit summoned (a Recruit, a token, a copy, Reborn) at
//!   once, a played one once it resolves, each once, never Floodgates' own; a Field Spell or a Trap is no
//!   Unit. Each grants Armor {armorGain} (`grant_keyword`; Armor stacks), kept until it leaves the field
//!   (R78). Death: every Unit its controller has as it died is destroyed (`destroy_all`, R89), an
//!   Indestructible one staying, the opponent's untouched. Tunes: armorGain 1 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-030";

/// R2197003: the Unit this event brings onto either side, or None: a summon of a Unit (Recruit, a token,
/// a copy, Reborn), or a played one once it resolves — its play's own placement (§10.5 step 4, which
/// carries `exitsFrom`) waits for that. Never this card itself.
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
        death: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "self" })))])),
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

// TG #30 Floodgates — SPEC §8.11 row 30, BUILD M13 row TG 30: "0/12 with Armor 3: a 5-damage hit deals 2;
// a Unit you play: +1 Armor (Armor 4); an enemy's played Unit: +1; a token summon, a Recruit and a copy:
// +1 each; a Field Spell or Trap placed: nothing; its own arrival: nothing; destroyed: every other Unit you
// control is destroyed, an Indestructible one staying, the enemy's untouched; bounced or exiled: no Death;
// the gain reads through `param()`. radiant 0/24, Armor 6, +2 per arrival, the same Death". The copy is
// Core #12 Duplicating Felinors' (TG #23 being another part's, #2196): played, it and its copy give +2.
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

    /// p1, active with 30 mana, has Floodgates (`radiant` its face) in lane 5, `mine` and `hand`; p2 has
    /// `theirs` and a hand of Mr. Vanillas.
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
