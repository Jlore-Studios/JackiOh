//! Meditative #64, Traitorous Blood (docs/meditative-set.md M6): the attack half of §6.3 Redirect
//! (MD-D19, R1122).
//!
//! Base: reveals when an enemy Unit with an adjacent Unit attacks, and re-aims the attack at that
//! adjacent Unit, so §4.3 resolves a combat between allies. Radiant: then summon a copy of each Unit
//! that combat destroyed (MD-D20, R1123). A declared attack only — a forced attack never opens the
//! window (M10) — and a neighbour the attacker may attack then (Taunt is no wall, M10). Two
//! neighbours are picked at random (M10); a later trap in the window skips the re-aimed attack (M10).

use jackioh_engine::prelude::*;
use serde_json::json;
use std::sync::Arc;

pub const ID: &str = "meditative-064";

/// A declared, unforced attack by one of the controller's enemies that has somewhere to go.
fn enemy_declared_with_neighbour(ctx: &mut EffectContext<'_>, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared {
        attacker_id, forced, ..
    } = event
    else {
        return false;
    };
    if *forced {
        return false;
    }
    let Some(attacker) = find_instance(ctx.state, attacker_id) else {
        return false;
    };
    if attacker.controller != opponent_of(ctx.controller) {
        return false;
    }
    !redirect_neighbours(ctx.state, attacker).is_empty()
}

fn traitorous_blood(copy_destroyed: bool) -> Script {
    let when: TriggerWhen = Arc::new(enemy_declared_with_neighbour);
    Script {
        triggers: vec![TriggerDef {
            id: "traitorousBlood".to_string(),
            on: vec![GameEventType::AttackDeclared],
            when: Some(when),
            run: Arc::new(move |ctx: &mut EffectContext<'_>, event: &GameEvent| {
                let GameEvent::AttackDeclared { attacker_id, .. } = event else {
                    return Vec::new();
                };
                let Some(attacker) = find_instance(ctx.state, attacker_id) else {
                    return Vec::new();
                };
                let neighbours = redirect_neighbours(ctx.state, attacker);
                let Some(to) = ctx.rng.pick(&neighbours) else {
                    return Vec::new();
                };
                vec![redirect_attack(json_as(json!({
                    "to": to.id,
                    "copyDestroyed": copy_destroyed,
                })))]
            }),
        }],
        ..Script::default()
    }
}

pub fn base() -> Script {
    traitorous_blood(false)
}

pub fn radiant() -> Script {
    traitorous_blood(true)
}

pub fn script() -> CardScripts {
    CardScripts {
        base: traitorous_blood(false),
        radiant: traitorous_blood(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Human 4/4, untagged Scarab 1/1, big Felinor 3/10.
    const SCARAB: &str = "core-007";
    const VANILLA: &str = "core-008";
    const BIG: &str = "core-043";

    /// `SideSetup.backrow` takes no `radiant` flag: flip the instance directly.
    fn make_radiant(s: &mut Scenario, card: &str) {
        let id = s.card(card).id.clone();
        match find_instance_mut(s.state_mut(), &id) {
            Some(instance) => instance.radiant = true,
            None => panic!("no instance {id}"),
        }
    }

    fn blood_game() -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": { "backrow": [{ "def": ID, "faceDown": true }] },
            "p2": { "field": [VANILLA, SCARAB] },
        }))
    }

    /// Each combat `damage` event of the last step as `(sourceId, targetId, amount)`. A unit the hit
    /// destroyed lies in the graveyard with its damage gone, so the events are the record. Fatigue
    /// from the turns that end by themselves after the attack (R82) is not combat and is left out.
    fn hits(s: &Scenario) -> Vec<(Option<String>, String, i32)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Damage {
                    source_id,
                    target_id,
                    amount,
                    combat: true,
                    ..
                } => Some((source_id.clone(), target_id.clone(), *amount)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn redirects_to_neighbour() {
        let mut s = blood_game();
        let foe = s.unit(P2, 1).unwrap().id;
        let neighbour = s.unit(P2, 2).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        // The attack fought the ally instead: the neighbour took the Vanilla's 4 and struck back
        // for its 1, the hero took nothing, and the 1/1 is destroyed.
        assert_eq!(
            hits(&s),
            vec![
                (Some(foe.clone()), neighbour.clone(), 4),
                (Some(neighbour.clone()), foe.clone(), 1),
            ]
        );
        s.expect_in_zone(&neighbour, "graveyard");
        s.expect_events(json!(["attackDeclared", "trapFired", "redirected"]));
        s.expect_in_zone(ID, "graveyard");
    }

    #[test]
    fn lonely_stays_set() {
        // No adjacent Unit: Blood stays set and the hero takes the hit. (A forced attack never
        // opens the window either; the engine's redirect tests cover that path.)
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "backrow": [{ "def": ID, "faceDown": true }] },
            "p2": { "field": [VANILLA] },
        }));
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        // The hero took the Vanilla's 4 (the turns that end by themselves afterwards add fatigue,
        // which is no part of the attack), and nothing fired.
        assert_eq!(hits(&s), vec![(Some(foe.clone()), "hero-p1".to_string(), 4)]);
        let kinds: Vec<&str> = s
            .last_events()
            .iter()
            .map(|event| event.event_type().as_str())
            .collect();
        assert!(!kinds.contains(&"trapFired") && !kinds.contains(&"redirected"));
        s.expect_in_zone(ID, "field");
    }

    #[test]
    fn two_neighbours_random() {
        // Either neighbour may take it; both are legal re-aims of the same declaration.
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "backrow": [{ "def": ID, "faceDown": true }] },
            "p2": { "field": [SCARAB, VANILLA, BIG] },
        }));
        let foe = s.unit(P2, 2).unwrap().id;
        let left = s.unit(P2, 1).unwrap().id;
        let right = s.unit(P2, 3).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        // The Vanilla's 4 lands on exactly one of the two, and on nothing else.
        let struck: Vec<String> = hits(&s)
            .into_iter()
            .filter(|(source, _, amount)| source.as_deref() == Some(foe.as_str()) && *amount == 4)
            .map(|(_, target, _)| target)
            .collect();
        assert!(
            struck == vec![left.clone()] || struck == vec![right.clone()],
            "one neighbour takes the 4: {struck:?}"
        );
        s.expect_events(json!(["attackDeclared", "trapFired", "redirected"]));
    }

    /// Moths to the Flame (Core #9): "Start of turn: Every enemy Unit attacks this." — forced attacks.
    const MOTHS: &str = "core-009";
    /// Midrange Menace (Core #19): a 9/9 Taunt.
    const MENACE: &str = "core-019";

    fn kinds_of(events: &[GameEvent]) -> Vec<&'static str> {
        events.iter().map(|event| event.event_type().as_str()).collect()
    }

    #[test]
    fn r33_it_is_face_down_until_it_fires() {
        let mut s = blood_game();
        let seen = serde_json::to_string(&s.view(P2)).expect("the view serialises");
        assert!(!seen.contains(ID), "p2 reads the set trap");
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        let seen = serde_json::to_string(&s.view(P2).opponent.graveyard).expect("JSON");
        assert!(seen.contains(ID), "fired, it is public");
    }

    #[test]
    fn r121_a_forced_attack_never_sets_it_off() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "field": [MOTHS],
                "backrow": [{ "def": ID, "faceDown": true }],
                "library": ["core-005", "core-005"],
            },
            "p2": { "field": [VANILLA, SCARAB], "library": ["core-005", "core-005"] },
        }));
        // p1's start of turn: Moths makes every enemy Unit attack it, each attack forced.
        s.start_turn();
        let events = s.last_events().to_vec();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. })),
            "the forced attacks were made: {:?}",
            kinds_of(&events)
        );
        assert!(!kinds_of(&events).contains(&"redirected"));
        assert!(!kinds_of(&events).contains(&"trapFired"));
        s.expect_in_zone(ID, "field");
    }

    #[test]
    fn its_own_sides_attack_never_sets_it_off() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [VANILLA, SCARAB], "backrow": [{ "def": ID, "faceDown": true }] },
            "p2": { "field": [BIG] },
        }));
        let mine = s.unit(P1, 1).unwrap().id;
        s.attack(&mine, "hero");
        assert!(!kinds_of(s.last_events()).contains(&"redirected"));
        s.expect_in_zone(ID, "field");
    }

    #[test]
    fn taunt_is_ignored() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [MENACE], "backrow": [{ "def": ID, "faceDown": true }] },
            "p2": { "field": [VANILLA, SCARAB] },
        }));
        let foe = s.unit(P2, 1).unwrap().id;
        let neighbour = s.unit(P2, 2).unwrap().id;
        let menace = s.unit(P1, 1).unwrap().id;
        s.end_turn();
        // The Taunt forces the declaration onto the Menace; the redirect still goes to the ally.
        s.attack(&foe, &menace);
        assert_eq!(hits(&s)[0], (Some(foe.clone()), neighbour.clone(), 4));
        s.expect_in_zone(&neighbour, "graveyard");
        s.expect_stats(&menace, json!({ "health": 9 }));
    }

    #[test]
    fn redirected_is_public() {
        let mut s = blood_game();
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        for viewer in [P1, P2] {
            let events = serde_json::to_value(s.view(viewer).events).expect("events");
            let redirected: Vec<&Value> = events
                .as_array()
                .into_iter()
                .flatten()
                .filter(|event| event["type"] == json!("redirected"))
                .collect();
            assert_eq!(redirected.len(), 1, "{viewer:?} reads the redirect");
        }
    }

    #[test]
    fn the_trap_is_consumed() {
        let mut s = blood_game();
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        assert!(s.backrow(P1, 1).is_none(), "its backrow zone is empty");
        s.expect_in_zone(ID, "graveyard");
    }

    #[test]
    fn radiant_copies_each_unit_the_combat_destroyed() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "backrow": [{ "def": ID, "faceDown": true }] },
            "p2": { "field": [SCARAB, SCARAB] },
        }));
        make_radiant(&mut s, ID);
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        // The two 1/1s destroyed each other: p1 gets a fresh copy of each.
        assert!(s.unit(P2, 1).is_none() && s.unit(P2, 2).is_none());
        let copies: Vec<CardInstance> = (1..=UNIT_ZONES).filter_map(|lane| s.unit(P1, lane)).collect();
        assert_eq!(copies.len(), 2);
        assert!(
            copies
                .iter()
                .all(|copy| copy.def_id == SCARAB && copy.controller == P1)
        );
    }

    #[test]
    fn radiant_a_full_board_takes_fewer_copies() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "field": [VANILLA, VANILLA, VANILLA, VANILLA],
                "backrow": [{ "def": ID, "faceDown": true }],
            },
            "p2": { "field": [SCARAB, SCARAB] },
        }));
        make_radiant(&mut s, ID);
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        assert!(s.unit(P2, 1).is_none() && s.unit(P2, 2).is_none());
        let scarabs = (1..=UNIT_ZONES)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| unit.def_id == SCARAB)
            .count();
        assert_eq!(scarabs, 1, "one open zone, one copy");
    }

    #[test]
    fn radiant_copies_destroyed() {
        let mut s = blood_game();
        make_radiant(&mut s, ID);
        let foe = s.unit(P2, 1).unwrap().id;
        s.end_turn();
        s.attack(&foe, "hero");
        // The Vanilla's 4 destroyed the 1/1 Scarab; the Blood's controller gets a fresh copy of it.
        s.expect_in_zone(&foe, "field");
        let copy = s
            .unit(P1, 1)
            .unwrap_or_else(|| panic!("p1 should hold the copied Scarab"));
        assert_eq!(copy.def_id, SCARAB);
        assert!(!copy.radiant);
    }
}
