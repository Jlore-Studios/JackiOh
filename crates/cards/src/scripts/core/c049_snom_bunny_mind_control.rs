//! #49 Snom Bunny Mind Control (SPEC §8.2): "Steal target enemy permanent", radiant "It also becomes
//! Radiant".
//!
//! Reading the radiant cell (§8 Conventions): "Also" adds an effect and the base clause it does not
//! restate is kept, so the radiant face steals the same permanent and then sets its flag.
//!
//! "Permanent" is §6.3's word (the Recruit row names them): Unit, Field Spell, Trap and Field Trap.
//! Units sit in the units row and the other three in the backrow, so the declaration below offers
//! both rows of the enemy board. It is a DECLARED play-time target travelling in the play action's
//! `targets` (R81), validated by R90 — which also means a face-down enemy trap is a legal pick the
//! client can name without ever seeing what it is (§9.1), and that only the top card of a Stack pile
//! is on offer (R13). An empty enemy board fizzles and the spell still counts as played (§8
//! Conventions).
//!
//! Everything about where the card lands is §6.3 Steal and R15, in `effects/steal.rs`: the same lane
//! on this side when that zone is free, else the first free zone of the same row, and if the row has
//! no free zone at all the card stays with the opponent. Control is a field-only notion (R12), so
//! the steal moves `controller` and nothing else: the card keeps its owner and will still go to that
//! owner's graveyard, hand or library when it later leaves the field, and because it never leaves
//! the field it keeps its damage, buffs, counters and position (R78 is about leaving). R33 does the
//! rest for a face-down trap: `face_up` is untouched, so the new controller is the one who may read
//! it and the previous controller stops seeing it.
//!
//! The radiant clause is `set_radiant`, which is the flag and nothing else (R74). On the field that
//! means the base stat layer swaps at once while damage and buffs stay and no Cry re-fires (R22), so
//! a stolen 4/3 that had taken 2 damage becomes an 8/6 that has taken 2. Order matters only for
//! readability: both effects resolve the same selection, and `set_radiant` resolves it by instance id
//! (`resolve_target` → `find_instance`), so the steal having already moved the card between zones
//! cannot make the second effect miss.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-049";

/// §6.3: a permanent is a Unit, Field Spell, Trap or Field Trap, so both enemy rows are on offer.
fn enemy_permanent() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit", "backrow"] }))]
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: enemy_permanent(),
            cry: Some(hook(|_ctx| vec![steal(json_as(json!({ "target": { "of": "chosen" } })))])),
            ..Script::default()
        },
        radiant: Script {
            targets: enemy_permanent(),
            cry: Some(hook(|_ctx| {
                vec![
                    steal(json_as(json!({ "target": { "of": "chosen" } }))),
                    set_radiant(json_as(json!({ "target": { "of": "chosen" } }))),
                ]
            })),
            ..Script::default()
        },
    }
}

// #49 Snom Bunny Mind Control — SPEC §8.2, BUILD M4-T4: "Steal placement per R15; radiant sets the
// flag on the stolen card".
//
// R15 has three clauses and each gets its own case: the same lane when that zone is free, the first
// free zone of the row when it is not, and "excess remain with the opponent" when the row is full.
// R12 is asserted alongside every steal: `controller` moves and `owner` never does.
//
// The radiant clause is R22/R74: the flag swaps the base stat layer at once while damage and buffs
// stay and no Cry re-fires, and a card that is already Radiant is untouched (§6.3 Make Radiant).
// The order inside the script (steal, then set_radiant) is proved safe here: `set_radiant` resolves
// the same selection by instance id, so the card having already changed zone cannot make it miss.
//
// Props with no script beyond printed keywords: #25 4-mana 7/7, #20 Pointmaster, #45 Deft Duelist
// (4/3 → 8/6, the damage-and-radiant case), #11 Tempo Timmy, #8 Mr. Vanilla. #15 Me and Mr Token
// carries a Cry that would summon a Rush Token, which is how "no Cry re-fires" is checked, and
// #41 Sheepish is the face-down Trap: a Trap that fires on a UNIT play, so playing this Spell can
// never set it off.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const MIND_CONTROL: &str = "core-049";
    const SEVEN_SEVEN: &str = "core-025";
    const POINTMASTER: &str = "core-020";
    const DUELIST: &str = "core-045";
    const TIMMY: &str = "core-011";
    const VANILLA: &str = "core-008";
    const TOKEN_MAKER: &str = "core-015";
    const SHEEPISH: &str = "core-041";

    /// The radiant face in hand, so the radiant text is the one that resolves (§5.2).
    fn radiant_mind_control() -> Value {
        json!({ "def": MIND_CONTROL, "radiant": true })
    }

    #[test]
    fn r15_base_the_stolen_unit_lands_in_the_same_lane_when_that_zone_is_free() {
        let mut g = scenario(json!({
            "p1": { "hand": [MIND_CONTROL] },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
        }));
        let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the 7/7 in lane 3").clone();

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

        assert_eq!(g.unit(PlayerId::P1, 3).map(|u| u.id.clone()), Some(prey.id.clone()));
        assert!(g.unit(PlayerId::P2, 3).is_none());
        // R12: control is a field-only notion; ownership never moves.
        assert_eq!(g.card(&prey).controller, PlayerId::P1);
        assert_eq!(g.card(&prey).owner, PlayerId::P2);
        g.expect_events(json!(["cardPlayed", "controlChanged", "enteredGraveyard"]));
        // (4) since patch v0.2.0 (issue #40), out of the harness's 4.
        g.expect_mana(PlayerId::P1, 0);
    }

    #[test]
    fn r15_base_the_first_free_zone_of_the_row_when_the_same_lane_is_taken() {
        let mut g = scenario(json!({
            "p1": { "hand": [MIND_CONTROL], "field": [{ "def": VANILLA, "lane": 3 }] },
            "p2": { "field": [{ "def": SEVEN_SEVEN, "lane": 3 }] },
        }));
        let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the 7/7 in lane 3").clone();

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

        // Lane 3 is occupied, so `first_free_zone` scans lanes 1..5 and lane 1 wins.
        assert_eq!(g.unit(PlayerId::P1, 1).map(|u| u.id.clone()), Some(prey.id.clone()));
        assert_eq!(g.unit(PlayerId::P1, 3).map(|u| u.def_id.clone()), Some(VANILLA.to_string()));
        assert!(g.unit(PlayerId::P2, 3).is_none());
    }

    #[test]
    fn r15_base_a_full_row_leaves_the_card_with_the_opponent_and_the_spell_still_resolves() {
        let mut g = scenario(json!({
            "p1": {
                "hand": [MIND_CONTROL],
                "field": [
                    { "def": SEVEN_SEVEN, "lane": 1 },
                    { "def": POINTMASTER, "lane": 2 },
                    { "def": DUELIST, "lane": 3 },
                    { "def": TIMMY, "lane": 4 },
                    { "def": VANILLA, "lane": 5 },
                ],
            },
            "p2": { "field": [{ "def": TOKEN_MAKER, "lane": 3 }] },
        }));
        let prey = g.unit(PlayerId::P2, 3).expect("setup: p2 should hold the target").clone();

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

        assert_eq!(g.unit(PlayerId::P2, 3).map(|u| u.id.clone()), Some(prey.id.clone()));
        assert_eq!(g.card(&prey).controller, PlayerId::P2);
        g.expect_in_zone(MIND_CONTROL, "graveyard");
        assert!(!g.events().iter().any(|event| event.event_type().as_str() == "controlChanged"));
    }

    #[test]
    fn r15_r33_base_a_face_down_enemy_trap_is_a_permanent_and_it_stays_face_down() {
        let mut g = scenario(json!({
            "p1": { "hand": [MIND_CONTROL] },
            "p2": { "backrow": [{ "def": SHEEPISH, "lane": 2 }] },
        }));
        let trap = g.backrow(PlayerId::P2, 2).expect("setup: p2 should hold Sheepish in backrow lane 2").clone();
        assert_ne!(trap.face_up, Some(true));

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": trap.id }] }));

        // §6.3's "permanent" is Unit, Field Spell, Trap or Field Trap, and R15 places it in its row.
        assert_eq!(g.backrow(PlayerId::P1, 2).map(|c| c.id.clone()), Some(trap.id.clone()));
        assert!(g.backrow(PlayerId::P2, 2).is_none());
        assert_eq!(g.card(&trap).controller, PlayerId::P1);
        assert_eq!(g.card(&trap).owner, PlayerId::P2);
        // R33: `face_up` is untouched — who may READ it follows the controller, in `view_for`.
        assert_ne!(g.card(&trap).face_up, Some(true));
    }

    #[test]
    fn r22_r74_radiant_the_stolen_card_becomes_radiant_keeping_its_damage() {
        let mut g = scenario(json!({
            "p1": { "hand": [radiant_mind_control()] },
            "p2": { "field": [{ "def": DUELIST, "lane": 2, "damage": 2 }] },
        }));
        let prey = g.unit(PlayerId::P2, 2).expect("setup: p2 should hold Deft Duelist").clone();
        g.expect_stats(&prey, json!({ "attack": 4, "maxHealth": 3, "health": 1 }));

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

        assert_eq!(g.unit(PlayerId::P1, 2).map(|u| u.id.clone()), Some(prey.id.clone()));
        assert!(g.card(&prey).radiant);
        // R22: the base layer swaps at once (4/3 → 8/6) and the 2 damage stays.
        g.expect_stats(&prey, json!({ "attack": 8, "maxHealth": 6, "health": 4 }));
        assert_eq!(g.card(&prey).damage, 2);
        // The steal moved the card first; `set_radiant` still found it, because it resolves by id.
        g.expect_events(json!(["controlChanged", "radiantSet"]));
    }

    #[test]
    fn r22_radiant_the_flag_fires_no_cry_nothing_is_summoned_alongside_the_steal() {
        let mut g = scenario(json!({
            "p1": { "hand": [radiant_mind_control()] },
            "p2": { "field": [{ "def": TOKEN_MAKER, "lane": 1 }] },
        }));
        let prey = g.unit(PlayerId::P2, 1).expect("setup: p2 should hold Me and Mr Token").clone();

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

        assert!(g.card(&prey).radiant);
        // Me and Mr Token's Cry summons a Rush Token; setting a flag is not an entry to the field.
        let mine: Vec<String> = [1, 2, 3, 4, 5]
            .into_iter()
            .filter_map(|lane| g.unit(PlayerId::P1, lane).map(|unit| unit.def_id.clone()))
            .collect();
        assert_eq!(mine, vec![prey.def_id.clone()]);
    }

    #[test]
    fn s6_3_radiant_a_card_that_is_already_radiant_is_stolen_and_left_alone() {
        let mut g = scenario(json!({
            "p1": { "hand": [radiant_mind_control()] },
            "p2": { "field": [{ "def": DUELIST, "lane": 4, "radiant": true }] },
        }));
        let prey = g.unit(PlayerId::P2, 4).expect("setup: p2 should hold a radiant Deft Duelist").clone();

        g.play(MIND_CONTROL, json!({ "targets": [{ "pick": "instance", "instanceId": prey.id }] }));

        assert_eq!(g.unit(PlayerId::P1, 4).map(|u| u.id.clone()), Some(prey.id.clone()));
        assert!(g.card(&prey).radiant);
        // §6.3 Make Radiant: no effect on a Radiant card, so the flag change emits nothing.
        assert_eq!(
            g.events().iter().filter(|event| event.event_type().as_str() == "radiantSet").count(),
            0
        );
    }

    #[test]
    fn r81_both_faces_declare_one_enemy_permanent_and_the_radiant_face_adds_the_flag() {
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, MIND_CONTROL);
        assert_eq!(serde_json::to_value(&def.type_).unwrap(), json!("Spell"));
        assert_eq!(serde_json::to_value(&def.cost).unwrap(), json!(4));
        let scripts = super::script();
        for face in [&scripts.base, &scripts.radiant] {
            assert_eq!(
                serde_json::to_value(&face.targets).unwrap(),
                json!([{ "kind": "target", "min": 1, "max": 1, "filter": { "side": "enemy", "of": ["unit", "backrow"] } }])
            );
            assert!(face.modes.is_empty());
        }
    }
}
