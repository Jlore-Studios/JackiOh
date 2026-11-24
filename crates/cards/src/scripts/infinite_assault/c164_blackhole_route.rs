//! SPEC §8.13 IA #164 Blackhole Route ((2) Trap, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reveals when an enemy Unit attacks your hero: Exile it."
//!   Radiant: "Reveals when an enemy Unit attacks your hero: Exile it. Its owner discards a random card."
//!
//! Engine: §4.2 step 4's trap window, on `attackDeclared`: an enemy Unit's declared attack on the
//! controller's hero (R121: a forced attack declares nothing and springs nothing). The attacker is
//! read first, then exiled (§6.3: no Death, so no combat, R44, R220); on the Radiant face its owner
//! then discards a random card (§6.3 Discard at random, R16). Every condition is in `when`, so any
//! other attack leaves it set (R61).

use jackioh_engine::effects::{discard_random, exile};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-164";

/// One card discarded at random.
const RANDOM_DISCARDS: i32 = 1;

/// The attacker of an enemy Unit's declared attack on this trap's controller's hero.
fn attacks_your_hero(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
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
    };
    let attacker = find_instance(&*ctx.state, attacker_id)?;
    if attacker.controller == ctx.controller {
        return None;
    }
    let hero = matches!(
        attack_target_of(&*ctx.state, target_id),
        Some(AttackTarget::Hero { player }) if player == ctx.controller
    );
    hero.then(|| attacker_id.clone())
}

fn blackhole(radiant: bool) -> Script {
    let reveal = TriggerDef::new("blackhole-route", &[GameEventType::AttackDeclared], move |ctx, event| {
        let Some(attacker) = attacks_your_hero(ctx, event) else {
            return vec![];
        };
        let owner = find_instance(&*ctx.state, &attacker).map(|card| card.owner);
        let mut effects = vec![exile(json_as(json!({
            "target": { "of": "instance", "instanceId": attacker },
        })))];
        if radiant {
            let side = if owner == Some(ctx.controller) { "self" } else { "enemy" };
            effects.push(discard_random(json_as(json!({
                "count": RANDOM_DISCARDS,
                "player": side,
            }))));
        }
        effects
    })
    .with_when(move |ctx, event| attacks_your_hero(ctx, event).is_some());
    Script {
        triggers: vec![reveal],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: blackhole(false),
        radiant: blackhole(true),
    }
}

// IA #164 Blackhole Route — SPEC §8.13 row 164 (R44, R61, R121, R220).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const BLACKHOLE: &str = "infiniteassault-164";
    const ATTACKER: &str = "stand-in (2) 3/3";
    const VANILLA: &str = "stand-in (1) 2/5";

    fn set(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "blackhole-route",
            "active": "p2",
            "p1": {
                "backrow": [{ "def": BLACKHOLE, "radiant": radiant }],
                "field": [VANILLA],
                "hand": [filler(1)],
                "library": filler(6),
            },
            "p2": {
                "field": [ATTACKER],
                "hand": [filler(1), filler(1)],
                "library": filler(6),
            },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn an_attack_on_your_hero_exiles_the_attacker_with_no_combat() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, "hero");
            s.expect_in_zone(&attacker, "exile");
            s.expect_health(P1, 30);
            assert_eq!(s.count("damage"), 0);
            s.expect_in_zone(BLACKHOLE, "graveyard");
        }

        #[test]
        fn an_attack_on_a_unit_leaves_it_set() {
            let mut s = set(false);
            let attacker = s.unit_at(P2, 1);
            let target = s.unit_at(P1, 1);
            s.attack(&attacker, &target);
            assert_eq!(s.count("trapFired"), 0);
            s.expect_in_zone(&attacker, "field");
            s.expect_in_zone(BLACKHOLE, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_exiled_attackers_owner_discards_a_random_card() {
            let mut s = set(true);
            let hand = s.hand(P2).len();
            let attacker = s.unit_at(P2, 1);
            s.attack(&attacker, "hero");
            s.expect_in_zone(&attacker, "exile");
            s.expect_health(P1, 30);
            assert_eq!(s.hand(P2).len(), hand - 1, "the owner discards one");
        }
    }
}
