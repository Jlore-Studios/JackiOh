//! S T-AI-4 Predictive Shield (SPEC §7, §8.10 row T-AI-4; §4.2 step 4, R44, R61, R482, R1174000,
//! R1881002): (1) Trap, AI, Token (no printed rarity), an AI generated token of the Stinkyscape
//! set, built by S21.
//!   Base:    "Reveals when an enemy Unit attacks one of your Units: Cancel the attack and Freeze the
//!            attacker."
//!   Radiant: "Reveals when an enemy Unit attacks you or one of your Units: Cancel the attack and Freeze
//!            the attacker."
//! Engine: §4.2 step 4's trap window, on `attackDeclared`: an enemy Unit's declared attack on a Unit its
//!   controller has (the Radiant face: or on their hero). Every condition is in `when`, so any other
//!   attack leaves it set (R61). A forced attack declares nothing and springs nothing (R1881002). Then
//!   `cancel_attack` (R44), the exertion spent all the same, and a Freeze of the attacker, which thaws at
//!   the cleanup of its controller's next turn (R1174000). Declares no numbers (R482).

use jackioh_engine::damage::DamageTarget;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-ai-04";

/// The attacker of an enemy Unit's declared attack this trap answers (R1881002).
fn guarded_attacker(ctx: &EffectContext<'_>, event: &GameEvent, guards_hero: bool) -> Option<String> {
    let GameEvent::AttackDeclared {
        attacker_id,
        target_id,
        forced,
        ..
    } = event
    else {
        return None;
    };
    if *forced {
        return None;
    }
    let attacker = find_instance(&*ctx.state, attacker_id)?;
    if attacker.controller == ctx.controller {
        return None;
    }
    let guarded = match attack_target_of(&*ctx.state, target_id)? {
        DamageTarget::Unit { instance } => instance.controller == ctx.controller,
        DamageTarget::Hero { player } => guards_hero && player == ctx.controller,
    };
    guarded.then(|| attacker_id.clone())
}

fn face(guards_hero: bool) -> Script {
    let reveal = TriggerDef::new("predictive-shield", &[GameEventType::AttackDeclared], move |ctx, event| {
        let Some(attacker) = guarded_attacker(ctx, event, guards_hero) else {
            return vec![];
        };
        vec![
            cancel_attack(CancelAttackArgs { destroy_attacker: None }),
            freeze(json_as(json!({ "target": { "of": "instance", "instanceId": attacker } }))),
        ]
    })
    .with_when(move |ctx, event| guarded_attacker(ctx, event, guards_hero).is_some());
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-AI-4 Predictive Shield — SPEC §8.10 row T-AI-4, BUILD M12 row S T-AI-4.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SHIELD: &str = "stinkyscape-t-ai-04";
    const ATTACKER: &str = "core-008";
    const VANILLA: &str = "core-001";
    const FILLER: &str = "core-005";

    fn set(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "predictive-shield",
            "active": "p2",
            "p1": {
                "health": 30,
                "backrow": [{ "def": SHIELD, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [FILLER],
                "library": filler(6),
            },
            "p2": { "field": [ATTACKER], "hand": [FILLER], "library": filler(6) },
        }))
    }

    fn fired(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::TrapFired { .. }))
    }

    fn canceled(s: &Scenario) -> bool {
        s.events().iter().any(|event| matches!(event, GameEvent::AttackCancelled { .. }))
    }

    #[test]
    fn r44_an_attack_on_one_of_your_units_is_canceled_and_the_attacker_frozen() {
        let mut s = set(false);
        s.attack(ATTACKER, VANILLA);
        assert!(fired(&s));
        assert!(canceled(&s));
        s.expect_stats(VANILLA, json!({ "health": 7 }));
        s.expect_stats(ATTACKER, json!({ "health": 4 }));
        assert!(s.card(ATTACKER).frozen.is_some(), "the attacker is Frozen");
        s.expect_in_zone(SHIELD, "graveyard");
    }

    #[test]
    fn r1174000_the_frozen_attacker_cannot_attack_on_its_controllers_next_turn() {
        let mut s = set(false);
        s.attack(ATTACKER, VANILLA);
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P2);
        assert!(s.card(ATTACKER).frozen.is_some(), "still Frozen on its next turn");
        let attacker = s.card(ATTACKER).id.clone();
        assert!(!legal_actions(s.state(), P2)
            .iter()
            .any(|action| matches!(action, ActionBody::Attack { attacker_id, .. } if *attacker_id == attacker)));
    }

    #[test]
    fn r61_an_attack_on_your_hero_leaves_the_base_face_set() {
        let mut s = set(false);
        s.attack(ATTACKER, "hero");
        assert!(!fired(&s));
        s.expect_health("p1", 26);
        assert!(s.card(ATTACKER).frozen.is_none());
        s.expect_in_zone(SHIELD, "field");
    }

    #[test]
    fn radiant_an_attack_on_your_hero_springs_it_too() {
        let mut s = set(true);
        s.attack(ATTACKER, "hero");
        assert!(fired(&s));
        assert!(canceled(&s));
        s.expect_health("p1", 30);
        assert!(s.card(ATTACKER).frozen.is_some());
    }

    #[test]
    fn r1881002_your_own_attack_leaves_it_set() {
        let mut s = scenario(json!({
            "seed": "predictive-shield-own",
            "p1": { "backrow": [SHIELD], "field": [ATTACKER], "hand": [FILLER], "library": filler(6) },
            "p2": { "field": [VANILLA], "hand": [FILLER], "library": filler(6) },
        }));
        s.attack(ATTACKER, VANILLA);
        assert!(!fired(&s));
        assert!(s.unit(P1, 1).is_some());
        s.expect_in_zone(SHIELD, "field");
    }

    #[test]
    fn r1881002_a_forced_attack_springs_nothing() {
        // #9 Moths to the Flame: "Start of turn: Every enemy Unit attacks this", a forced attack (R53).
        const MOTHS: &str = "core-009";
        let mut s = scenario(json!({
            "seed": "predictive-shield-forced",
            "active": "p2",
            "p1": {
                "backrow": [SHIELD],
                "field": [MOTHS],
                "hand": [FILLER],
                "library": filler(6),
            },
            "p2": { "field": [ATTACKER], "hand": [FILLER], "library": filler(6) },
        }));
        s.end_turn(); // p1's start of turn: the Vanilla is made to attack the Moths.
        assert_eq!(s.state().active, P1);
        assert!(
            s.last_events().iter().any(
                |event| matches!(event, GameEvent::Damage { target_id, .. } if *target_id == s.card(MOTHS).id)
            ),
            "the forced attack happened"
        );
        assert!(!fired(&s));
        assert!(!canceled(&s));
        assert!(s.card(ATTACKER).frozen.is_none());
        s.expect_in_zone(SHIELD, "field");
    }
}
