//! TG #9 Ravager (SPEC §8.11 row 9): (4) Unit, Human, Common, 7/7 → 14/17, a card of the held Training
//! Grounds set (R1420, R2194000).
//!   Base and Radiant: "Rush / After this destroys a Unit, it gains +{buff}/+{buff}." — buff 5, Radiant 10
//! Engine: C+ #19.5 Bot Loser's kill trigger: a `destroyed` whose `killer_id` is this card, R42's killer
//!   (the hit, or Poisonous hit, that first dooms the victim, on any turn, a defending kill included); a
//!   destroy effect credits nobody. A field trigger, "After", so a Ravager that dies in the same combat
//!   gains nothing (R1245, R2396000). The gain is a permanent buff to both stats; the damage it took stays. Tunes: buff 5 ↑.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-009";

/// "After this destroys a Unit, it gains +{buff}/+{buff}." (`when` is read for traps only, so the test is in `run`.)
fn on_kill() -> TriggerDef {
    TriggerDef::new("ravager-kill", &[GameEventType::Destroyed], |ctx, event| {
        let killed_by_me = match (event, ctx.self_.as_ref()) {
            (GameEvent::Destroyed { killer_id: Some(killer), .. }, Some(me)) => *killer == me.id,
            _ => false,
        };
        if !killed_by_me {
            return Vec::new();
        }
        let gain = param(&*ctx, "buff");
        vec![buff(json_as(json!({ "target": { "of": "self" }, "attack": gain, "health": gain })))]
    })
}

pub fn script() -> CardScripts {
    // The Radiant face is the same text at +10/+10, a catalog value.
    let base = Script {
        triggers: vec![on_kill()],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// TG #9 Ravager — SPEC §8.11 row 9, BUILD M13 row TG 9: "base: 7/7 with Rush, may attack a Unit the turn
// it is played, not the hero; attacking and killing a Unit it survives: 12/12 (the damage taken stays); a
// Unit that attacks it and dies: +5/+5; if it dies in the same combat it gains nothing (R1245); a hit that
// does not kill: no gain; a hit on a Divine Shield: no gain; two kills: +10/+10; the buff reads
// `param("buff")`. radiant 14/17 with Rush; each kill +10/+10".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const VANILLA: &str = "core-008"; // (1) Mr. Vanilla, 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14.
    const DEFENDER: &str = "core-003"; // (1) Right-house defender, 1/1 Taunt, Divine Shield, Reborn.
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.

    /// p1 has a Ravager (`radiant` its face) in lane 1, not summoning sick; p2 has `theirs`.
    fn board(radiant: bool, theirs: Value) -> Scenario {
        crate::scenario(json!({
            "seed": "ravager",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "hand": [FILLER],
                "library": crate::filler(6),
            },
            "p2": { "field": theirs, "hand": [FILLER], "library": crate::filler(6) },
        }))
    }

    fn ravager(s: &Scenario) -> String {
        s.unit_at(P1, 1).id
    }

    mod base {
        use super::*;

        #[test]
        fn rush_it_is_7_7_and_may_attack_a_unit_the_turn_it_is_played_but_not_the_hero() {
            let mut s = crate::scenario(json!({
                "seed": "ravager-rush",
                "p1": { "hand": [ID, FILLER], "library": crate::filler(4), "mana": 10 },
                "p2": { "field": [VANILLA], "hand": [FILLER], "library": crate::filler(4) },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let me = s.card(ID).id.clone();
            s.expect_stats(me.as_str(), json!({ "attack": 7, "maxHealth": 7 }));
            assert!(unit_has(s.state(), s.card(me.as_str()), KeywordKind::Rush));
            s.expect_refused(|s| s.attack(me.as_str(), "hero"));
            s.attack(me.as_str(), VANILLA);
            s.expect_in_zone(VANILLA, "graveyard");
        }

        #[test]
        fn r42_attacking_and_killing_a_unit_it_survives_makes_it_12_12_and_the_damage_stays() {
            let mut s = board(false, json!([VANILLA]));
            let me = ravager(&s);
            s.attack(me.as_str(), VANILLA);
            s.expect_in_zone(VANILLA, "graveyard");
            s.expect_stats(me.as_str(), json!({ "attack": 12, "maxHealth": 12, "health": 8 }));
        }

        #[test]
        fn r42_a_unit_that_attacks_it_and_dies_is_its_kill() {
            let mut s = board(false, json!([VANILLA]));
            let me = ravager(&s);
            crate::next_turn_of(&mut s, P2);
            let theirs = s.unit_at(P2, 1).id;
            s.attack(theirs.as_str(), me.as_str());
            s.expect_in_zone(theirs.as_str(), "graveyard");
            s.expect_stats(me.as_str(), json!({ "attack": 12, "maxHealth": 12, "health": 8 }));
        }

        #[test]
        fn r1245_if_it_dies_in_the_same_combat_it_gains_nothing() {
            let mut s = board(false, json!([VANILLA]));
            let me = ravager(&s);
            s.card_mut(me.as_str()).damage = 3;
            s.attack(me.as_str(), VANILLA);
            s.expect_in_zone(VANILLA, "graveyard");
            s.expect_in_zone(me.as_str(), "graveyard");
            assert!(
                !s.last_events().iter().any(|event| matches!(event, GameEvent::Buffed { .. })),
                "no buff: it was off the field as the destroyed event was dispatched"
            );
        }

        #[test]
        fn a_hit_that_does_not_kill_gains_nothing() {
            let mut s = board(false, json!([MOTHS]));
            let me = ravager(&s);
            s.attack(me.as_str(), MOTHS);
            s.expect_in_zone(MOTHS, "field");
            s.expect_stats(me.as_str(), json!({ "attack": 7, "maxHealth": 7 }));
        }

        #[test]
        fn a_hit_on_a_divine_shield_gains_nothing() {
            let mut s = board(false, json!([DEFENDER]));
            let me = ravager(&s);
            s.attack(me.as_str(), DEFENDER);
            s.expect_in_zone(DEFENDER, "field");
            s.expect_stats(me.as_str(), json!({ "attack": 7, "maxHealth": 7 }));
        }

        #[test]
        fn two_kills_give_10_10() {
            let mut s = board(false, json!([VANILLA, { "def": VANILLA, "lane": 2 }]));
            let me = ravager(&s);
            let first = s.unit_at(P2, 1).id;
            let second = s.unit_at(P2, 2).id;
            s.attack(me.as_str(), first.as_str());
            crate::next_turn_of(&mut s, P1);
            s.attack(me.as_str(), second.as_str());
            s.expect_in_zone(second.as_str(), "graveyard");
            s.expect_stats(me.as_str(), json!({ "attack": 17, "maxHealth": 17, "health": 9 }));
        }

        #[test]
        fn the_buff_reads_param() {
            let mut s = board(false, json!([VANILLA]));
            let me = ravager(&s);
            assert_eq!(crate::upgrade_number(&mut s, &me, "buff"), 6);
            s.attack(me.as_str(), VANILLA);
            s.expect_stats(me.as_str(), json!({ "attack": 13, "maxHealth": 13 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn it_is_14_17_with_rush() {
            let s = board(true, json!([]));
            let me = ravager(&s);
            let view = s.stats(me.as_str());
            assert_eq!((view.attack, view.max_health), (14, 17));
            assert!(unit_has(s.state(), s.card(me.as_str()), KeywordKind::Rush));
        }

        #[test]
        fn r42_each_kill_gives_10_10() {
            let mut s = board(true, json!([VANILLA]));
            let me = ravager(&s);
            s.attack(me.as_str(), VANILLA);
            s.expect_stats(me.as_str(), json!({ "attack": 24, "maxHealth": 27, "health": 23 }));
        }
    }
}
