//! S #135 Sacked (SPEC §8.10 row 135; R17, R427, R1850003, R1875004): (1) Trap, Common, of the
//! Stinkyscape set.
//!   Base:    "Reveals when your opponent plays a Unit: After it resolves, stuff it into a Sack they
//!            control."
//!   Radiant: the same, "a Sack you control".
//! Answers §10.5 cardResolved of opponent's played Unit (R427, R1875004, Core #41; R61, R174).
//! Summons Sack (S #135.1, R1850003, R64); holds unit beneath via hold_beneath (R78, R11).

use jackioh_engine::effects::hold_beneath;
use jackioh_engine::prelude::*;
use jackioh_engine::query::left_field_since_resolved;

pub const ID: &str = "stinkyscape-135";

/// S #135.1, the Sack it summons.
const SACK: &str = "stinkyscape-135-1";

/// R427, R1875004: an opponent's Unit's play that has resolved, the Unit still on the field from it.
fn answers(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::CardResolved {
        player,
        def_id,
        permanent,
        ..
    } = event
    else {
        return false;
    };
    // R2179002: a card cast on draw is no play.
    *player != ctx.controller
        && !cast_on_draw_event(event)
        && def_of(Some(&*ctx.state), def_id).type_ == CardType::Unit
        && (*permanent || !left_field_since_resolved(&*ctx.state, event))
}

/// R1875004: "a Sack they control" is the Sack summoned for the opponent, the Unit's controller;
/// "a Sack you control" for the Trap's controller. Either way the Unit comes back to the Sack's.
fn sacked(sack_for: &'static str) -> Script {
    let stuff = TriggerDef::new("sacked", &[GameEventType::CardResolved], move |_ctx, event| {
        let GameEvent::CardResolved {
            instance_id,
            permanent,
            ..
        } = event
        else {
            return Vec::new();
        };
        if !*permanent {
            return Vec::new();
        }
        vec![hold_beneath(json_as(json!({
            "target": { "of": "instance", "instanceId": instance_id },
            "release": "holdersController",
            "holder": { "defId": SACK, "player": sack_for },
        })))]
    })
    .with_when(|ctx, event| answers(ctx, event));
    Script {
        triggers: vec![stuff],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: sacked("enemy"),
        radiant: sacked("self"),
    }
}

// S #135 Sacked — SPEC §8.10 row 135, BUILD M12 row S 135.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{ID, SACK};

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const SORCERER: &str = "core-068"; // (2) 5/5: "Cry: Deal 4 damage."
    const LAR: &str = "classicplus-076"; // (1) 1/1: "Death: Summon a Brother Ping."
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const ZAP: &str = "core-036"; // (1) Spell Magic Jammed: no Unit, it leaves the trap set.

    fn trapped(radiant: bool, hand: Value) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": "sacked",
            "p1": { "hand": hand, "library": crate::filler(6), "mana": 10 },
            "p2": {
                "hand": [HIT_JOB, FILLER],
                "backrow": [{ "def": ID, "radiant": radiant, "lane": 1 }],
                "library": crate::filler(6),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn sack_of(s: &Scenario, player: PlayerId) -> Option<CardInstance> {
        (1..=5).filter_map(|lane| s.unit(player, lane)).find(|card| card.def_id == SACK)
    }

    fn held_ids(s: &Scenario) -> Vec<String> {
        s.state()
            .held
            .iter()
            .flatten()
            .map(|held| held.card.id.clone())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1875004_after_its_cry_the_unit_is_stuffed_into_a_sack_its_controller_controls() {
            let mut s = trapped(false, json!([SORCERER, FILLER]));
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(SORCERER, json!({ "zone": 2, "targets": [{ "pick": "hero", "player": "p2" }] }));
            s.expect_health(P2, HERO_HEALTH - 4);
            assert_eq!(held_ids(&s), vec![sorcerer]);
            let sack = sack_of(&s, P1).expect("p1's Sack");
            s.expect_stats(&sack, json!({ "attack": 0, "health": 4 }));
            s.expect_in_zone(ID, "graveyard");
            let types: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
            let summoned = types.iter().rposition(|t| *t == GameEventType::Summoned);
            let held = types.iter().position(|t| *t == GameEventType::Held);
            assert!(summoned < held, "{types:?}");
        }

        #[test]
        fn r1875004_no_death_fires_as_it_is_stuffed() {
            let mut s = trapped(false, json!([LAR, FILLER]));
            s.play(LAR, json!({ "zone": 1 }));
            let units: Vec<String> = (1..=5).filter_map(|lane| s.unit(P1, lane)).map(|card| card.def_id).collect();
            assert_eq!(units, vec![SACK.to_string()], "no Brother Ping");
        }

        #[test]
        fn r1875004_the_sacks_death_summons_it_for_its_controller_sick_and_with_no_cry() {
            let mut s = trapped(false, json!([SORCERER, FILLER]));
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(SORCERER, json!({ "zone": 2, "targets": [{ "pick": "hero", "player": "p2" }] }));
            let sack = sack_of(&s, P1).expect("p1's Sack").id;
            s.end_turn();
            s.state_mut().players[P2].mana.current = 10;
            s.play(HIT_JOB, json!({ "targets": pick::id(&sack) }));
            let back = s.card(sorcerer.as_str()).clone();
            assert!(matches!(back.zone, Zone::Field { player: P1, row: Row::Units, .. }), "{:?}", back.zone);
            assert_eq!(back.summoned_turn, Some(s.state().turn), "summoning sick");
            s.expect_health(P2, HERO_HEALTH - 4);
        }

        #[test]
        fn r61_a_spell_or_its_own_sides_unit_leaves_it_set() {
            let mut s = trapped(false, json!([ZAP, FILLER]));
            s.play(ZAP, json!({ "targets": pick::id(&s.backrow(P2, 1).expect("set").id) }));
            assert!(!s.last_events().iter().any(|event| event.event_type() == GameEventType::TrapFired));
            let mut own = crate::scenario(json!({
                "p1": {
                    "hand": [LAR, FILLER],
                    "backrow": [{ "def": ID, "lane": 1 }],
                    "library": crate::filler(4),
                    "mana": 10,
                },
            }));
            own.play(LAR, json!({ "zone": 1 }));
            assert_eq!(own.backrow(P1, 1).map(|card| card.def_id), Some(ID.to_string()));
            assert!(sack_of(&own, P1).is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1875004_the_sack_stands_on_its_casters_side_and_its_death_gives_the_unit_to_them() {
            let mut s = trapped(true, json!([SORCERER, FILLER]));
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(SORCERER, json!({ "zone": 2, "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert!(sack_of(&s, P1).is_none());
            let sack = sack_of(&s, P2).expect("p2's Sack");
            s.expect_stats(&sack, json!({ "attack": 0, "health": 4 }));
            s.end_turn();
            s.state_mut().players[P2].mana.current = 10;
            s.play(HIT_JOB, json!({ "targets": pick::id(&sack.id) }));
            let back = s.card(sorcerer.as_str()).clone();
            assert!(matches!(back.zone, Zone::Field { player: P2, row: Row::Units, .. }), "{:?}", back.zone);
            assert_eq!(back.controller, P2);
        }
    }
}
