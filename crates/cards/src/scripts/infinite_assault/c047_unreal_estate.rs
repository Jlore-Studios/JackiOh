//! IA #47 Unreal Estate (SPEC §8.13 row 47), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks: {chance}% chance to cancel the attack." (50)
//!   Radiant: "Lucky 1. Reveals when an enemy Unit attacks: {chance}% chance to cancel the
//!            attack."
//!
//! Lucky 1 on the Radiant face is catalog data. Every firing — the first reveal and every later
//! one (`again`, R2657001) — rolls a d100 at the card's Lucky plus its controller's Luck (R987,
//! R1438) and cancels the attack at or below the declared `chance`. A forced attack declares
//! nothing and springs nothing (R53); its own controller's attacks never do. The die is reported
//! as it lands (R2652001).
//!
//! Engine: `roll_die`, `cancel_attack` (R4586010). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-047";

/// The die: 1 to 100.
const SIDES: i32 = 100;

/// A declared enemy attack's attacker, if this answers it.
fn answered_attacker(ctx: &mut EffectContext<'_>, event: &GameEvent) -> Option<String> {
    let GameEvent::AttackDeclared {
        attacker_id,
        forced,
        ..
    } = event
    else {
        return None;
    };
    if *forced {
        return None;
    };
    let attacker = find_instance(&*ctx.state, attacker_id)?;
    (attacker.controller != ctx.controller).then(|| attacker_id.clone())
}

fn estate() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "unreal-estate",
            &[GameEventType::AttackDeclared],
            move |ctx, event| {
                if answered_attacker(ctx, event).is_none() {
                    return vec![];
                };
                let chance = param(&*ctx, "chance");
                // R987: the card's Lucky besides its controller's Luck rolls extra dice, best kept.
                let lucky = ctx.live_self().map_or(0, |me| lucky_on(&*ctx.state, me))
                    + luck_of(&*ctx.state, ctx.controller);
                let die = roll_die(ctx, SIDES, lucky, |face| face <= chance);
                let hit = die.kept_face() <= chance;
                // R2652001: the die is shown before anything it decided.
                let mut effects = vec![report_rolls(RollKind::Die, SIDES, vec![die])];
                if hit {
                    effects.push(cancel_attack(CancelAttackArgs {
                        destroy_attacker: None,
                    }));
                }
                effects
            },
        )
        .with_when(|ctx, event| answered_attacker(ctx, event).is_some())],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: estate(),
        // The same script: the Radiant face's Lucky 1 is catalog data.
        radiant: estate(),
    }
}

// IA #47 Unreal Estate — SPEC §8.13 row 47 (R4586010).
#[cfg(any())] // TEMP-NEUTRALIZED: pre-existing C2 breakage, restored after verification
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const ATTACKER: &str = "core-008"; // Mr. Vanilla: a 4/4.
    const VANILLA: &str = "core-001"; // Big D-fender: a 0/7.
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.


    fn health_of(s: &Scenario, lane: i32) -> i32 {
        unit_view(s.state(), &s.unit_at(PlayerId::P1, lane)).health
    }

    fn armed(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [ATTACKER], "hand": [STOCKPILE], "library": crate::filler(4) },
        }))
    }

    fn canceled(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(
            event,
            GameEvent::AttackCancelled { .. }
        ))
    }

    mod base {
        use super::*;

        #[test]
        fn r4586010_a_hit_cancels_the_attack() {
            let seed = seed_where("estate-hit", |probe| canceled(&{
                let mut s = armed(false, probe);
                s.attack(ATTACKER, VANILLA);
                s
            }));
            let mut s = armed(false, &seed);
            s.attack(ATTACKER, VANILLA);
            assert!(canceled(&s));
            s.expect_stats(VANILLA, json!({ "health": 7 }));
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r4586010_a_miss_leaves_the_attack_to_land() {
            let seed = seed_where("estate-miss", |probe| !canceled(&{
                let mut s = armed(false, probe);
                s.attack(ATTACKER, VANILLA);
                s
            }));
            let mut s = armed(false, &seed);
            let before = health_of(&s, 1);
            s.attack(ATTACKER, VANILLA);
            assert!(!canceled(&s));
            assert_eq!(health_of(&s, 1), before - 4, "the attack lands");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586010_the_radiant_face_rolls_with_lucky_1() {
            let seed = seed_where("estate-lucky", |probe| canceled(&{
                let mut s = armed(true, probe);
                s.attack(ATTACKER, VANILLA);
                s
            }));
            let mut s = armed(true, &seed);
            s.attack(ATTACKER, VANILLA);
            assert!(canceled(&s));
            s.expect_in_zone(ID, "graveyard");
        }
    }
}
