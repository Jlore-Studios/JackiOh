//! S T-AI-9 Honeypot (SPEC §7, §8.10 row T-AI-9; §10.5 step 4, R61, R64, R482, R1881005): (1) Trap, AI,
//! Token (no printed rarity), an AI generated token of the Stinkyscape set, built by S21.
//!   Base:    "Reveals when your opponent plays a Unit: Summon a Drone Interceptor. Give it Taunt."
//!   Radiant: "Reveals when your opponent plays a Unit: Summon 2 Drone Interceptors. Give them Taunt."
//! Engine: a face-down Trap on the opponent's Unit play (`cardPlayed`, §10.5 step 4); a Spell, a backrow
//!   card or its controller's own play leaves it set (R61). It summons the Drone Interceptor into its
//!   controller's leftmost open unit zone (R64), the `keywords` rider giving it Taunt as it lands (R1881005); one that
//!   rolled Taunt keeps it. With no open zone nothing. Radiant: two of them. Declares no numbers (R482).

use jackioh_engine::catalog::def_of;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-09";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
/// "a Drone Interceptor", "2 Drone Interceptors".
const DECOYS: usize = 1;
const RADIANT_DECOYS: usize = 2;

/// §8: "your opponent plays a Unit".
fn opponent_played_a_unit(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardPlayed { player, def_id, .. } = event else {
        return false;
    };
    *player != ctx.controller && def_of(Some(&*ctx.state), def_id).type_ == CardType::Unit
}

fn face(decoys: usize) -> Script {
    let reveal = TriggerDef::new("honeypot", &[GameEventType::CardPlayed], move |_ctx, _event| {
        (0..decoys)
            .map(|_| {
                summon(json_as(json!({
                    "defId": INTERCEPTOR,
                    "keywords": [{ "kind": "Taunt" }],
                })))
            })
            .collect()
    })
    .with_when(|ctx, event| opponent_played_a_unit(ctx, event));
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(DECOYS),
        radiant: face(RADIANT_DECOYS),
    }
}

// S T-AI-9 Honeypot — SPEC §8.10 row T-AI-9, BUILD M12 row S T-AI-9.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;

    const HONEYPOT: &str = "stinkyscape-t-ai-09";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    /// #44, a Spell aimed at a hero.
    const SPELL: &str = "core-044";
    const HIT_JOB: &str = "core-016";

    /// p1 holds the trap; p2 is active with `card` to play and a Hit Job so the turn stays open (R82).
    fn set(radiant: bool, card: &str) -> Scenario {
        scenario(json!({
            "seed": "honeypot",
            "active": "p2",
            "p1": { "backrow": [{ "def": HONEYPOT, "radiant": radiant }], "field": [{ "def": VANILLA, "lane": 1 }] },
            "p2": { "hand": [card, HIT_JOB], "mana": 10 },
        }))
    }

    fn decoys(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == INTERCEPTOR)
            .collect()
    }

    fn fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. }))
    }

    #[test]
    fn r1881005_r64_the_opponents_unit_play_summons_a_drone_with_taunt() {
        let mut s = set(false, VANILLA);
        s.play(VANILLA, json!({ "zone": 1 }));
        assert!(fired(&s));
        let made = decoys(&s);
        assert_eq!(made.len(), 1);
        assert_eq!(s.unit(P1, 2).expect("lane 2").def_id, INTERCEPTOR);
        assert!(!made[0].radiant);
        assert!(keywords_of(s.state(), s.card(made[0].id.as_str())).contains(&Keyword::Taunt));
        s.expect_in_zone(HONEYPOT, "graveyard");
    }

    #[test]
    fn r61_a_spell_leaves_it_set() {
        let mut s = set(false, SPELL);
        s.play(SPELL, json!({ "targets": [{ "pick": "hero", "player": "p1" }] }));
        assert!(!fired(&s));
        assert!(decoys(&s).is_empty());
        s.expect_in_zone(HONEYPOT, "field");
    }

    #[test]
    fn r61_its_controllers_own_unit_play_leaves_it_set() {
        let mut s = scenario(json!({
            "seed": "honeypot-own",
            "p1": { "backrow": [HONEYPOT], "hand": [VANILLA, HIT_JOB], "mana": 10 },
        }));
        s.play(VANILLA, json!({ "zone": 1 }));
        assert!(!fired(&s));
        s.expect_in_zone(HONEYPOT, "field");
    }

    #[test]
    fn radiant_two_drones_with_taunt() {
        let mut s = set(true, VANILLA);
        s.play(VANILLA, json!({ "zone": 1 }));
        let made = decoys(&s);
        assert_eq!(made.len(), 2);
        for drone in &made {
            assert!(!drone.radiant, "plain drones (R275's 2×)");
            assert!(keywords_of(s.state(), s.card(drone.id.as_str())).contains(&Keyword::Taunt));
        }
    }
}
