//! S T-shield-layer Drone Battleship (Shield Layer) (SPEC §8.10 row T-shield-layer): (5) Unit, Token
//! (printed Mythic), 12/12 with Armor 4 → 24/24 with Armor 8, a token of the Stinkyscape set.
//!   Both faces: "Starship, Can't attack, Taunt, Armor 4\nLethal damage your hero or your other Units would
//!   take is redirected to this." (Armor 8 on the Radiant face)
//! Engine: the top of the Drone Battleship's pile (S #36, R1842000). Starship, Can't attack, Taunt and Armor
//!   are catalog keywords. Unan's lethal guard (R1204): `ReplacementDef { on: LethalHit, instead: redirect
//!   SelfCard }`, so a lethal hit on its controller's hero or another of their Units is dealt to it
//!   instead, through its own Armor, while it tops its pile (R1842001); a buried card acts for nothing
//!   (R13), and when it dies the Propagation Layer beneath resumes. A unit token: it ceases to exist off the
//!   field (R11).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-shield-layer";

pub fn script() -> CardScripts {
    // Both faces guard the same way; the Radiant face's 24/24 and Armor 8 are catalog data.
    let base = Script {
        replacements: vec![ReplacementDef {
            id: "shield-layer-lethal-guard".to_string(),
            on: ReplacementMoment::LethalHit,
            where_: None,
            when: None,
            instead: ReplacementInstead {
                redirect: Some(InsteadRedirect::SelfCard),
                ..Default::default()
            },
            then: None,
            by: None,
        }],
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S T-shield-layer — SPEC §8.10 row T-shield-layer, BUILD M12 row S T-shield-layer.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const SHIELD: &str = "stinkyscape-t-shield-layer";
    const PROPAGATION: &str = "stinkyscape-t-propagation-layer";
    const MELTDOWN: &str = "stinkyscape-t-meltdown-layer";
    const ALLY: &str = "core-008"; // (1) 4/4.
    const BURN: &str = "classic-036"; // (0) Spell: deal {damage} damage.
    const STRIKER: &str = "classic-049"; // (3) 9/9 Rush.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const FILLER: &str = "core-005";

    const HIT: i32 = 9;

    fn guarded(radiant: bool, p1: Value, p2: Value) -> Scenario {
        let p1 = crate::merged(
            json!({
                "hand": [FILLER],
                "field": [
                    { "def": MELTDOWN, "lane": 1 },
                    { "def": PROPAGATION, "stack": true },
                    { "def": SHIELD, "radiant": radiant, "stack": true },
                    { "def": ALLY, "lane": 2 },
                ],
                "library": filler(4),
            }),
            p1,
        );
        let p2 = crate::merged(json!({ "hand": [BURN, FILLER], "library": filler(4), "mana": 10 }), p2);
        let mut s = scenario(json!({ "active": "p2", "p1": p1, "p2": p2 }));
        set_param(s.card_mut(BURN), "damage", HIT);
        s
    }

    fn burn(s: &mut Scenario, target: Value) {
        s.play(BURN, json!({ "targets": [target] }));
    }

    fn at(id: &str) -> Value {
        json!({ "pick": "instance", "instanceId": id })
    }

    fn redirects(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Redirected { .. }))
            .count()
    }

    fn top(s: &Scenario) -> CardInstance {
        s.unit(P1, 1).expect("the pile's top")
    }

    #[test]
    fn r1842001_a_lethal_hit_on_another_unit_goes_to_it_through_armor() {
        let mut s = guarded(false, json!({}), json!({}));
        let ally = s.card(ALLY).id.clone();
        burn(&mut s, at(&ally));
        assert_eq!(s.card(ally.as_str()).damage, 0, "the ally is untouched");
        assert_eq!(s.stats(SHIELD).health, 12 - (HIT - 4), "9 through Armor 4");
        assert_eq!(redirects(&s), 1);
    }

    #[test]
    fn r1842001_the_hero_is_guarded() {
        let mut s = guarded(false, json!({ "health": 5 }), json!({}));
        burn(&mut s, json!({ "pick": "hero", "player": "p1" }));
        s.expect_health(P1, 5);
        assert_eq!(s.stats(SHIELD).health, 12 - (HIT - 4));
    }

    #[test]
    fn r1842001_r13_buried_it_guards_nothing() {
        let mut s = scenario(json!({
            "active": "p2",
            "p1": {
                "hand": [FILLER],
                "field": [{ "def": SHIELD, "lane": 1 }, { "def": ALLY, "stack": true }, { "def": ALLY, "lane": 2 }],
                "library": filler(4),
            },
            "p2": { "hand": [BURN, FILLER], "library": filler(4), "mana": 10 },
        }));
        set_param(s.card_mut(BURN), "damage", HIT);
        let ally = s.unit(P1, 2).expect("the ally").id;
        burn(&mut s, at(&ally));
        s.expect_in_zone(ally.as_str(), "graveyard");
        assert_eq!(redirects(&s), 0);
        assert_eq!(s.card(SHIELD).damage, 0);
    }

    #[test]
    fn it_never_attacks() {
        let mut s = guarded(false, json!({}), json!({}));
        s.end_turn();
        let shield = top(&s).id;
        s.expect_refused(|s| s.attack(shield.as_str(), "hero"));
    }

    #[test]
    fn taunt_enemies_must_attack_it_first() {
        let mut s = guarded(false, json!({}), json!({ "field": [STRIKER] }));
        s.expect_refused(|s| s.attack(STRIKER, "hero"));
        let ally = s.card(ALLY).id.clone();
        s.expect_refused(|s| s.attack(STRIKER, ally.as_str()));
        let shield = top(&s).id;
        s.attack(STRIKER, shield.as_str());
        assert_eq!(s.stats(shield.as_str()).health, 12 - (9 - 4), "9 through Armor 4");
    }

    #[test]
    fn r13_dead_the_propagation_layer_resumes() {
        let mut s = guarded(false, json!({}), json!({ "hand": [BURN, HIT_JOB, FILLER] }));
        let shield = top(&s).id;
        s.play(HIT_JOB, json!({ "targets": [at(&shield)] }));
        assert_eq!(top(&s).def_id, PROPAGATION);
        let ally = s.card(ALLY).id.clone();
        burn(&mut s, at(&ally));
        s.expect_in_zone(ally.as_str(), "graveyard");
    }

    #[test]
    fn radiant_guards_through_armor_8() {
        let mut s = guarded(true, json!({}), json!({}));
        let shield = top(&s).id;
        let stats = s.stats(shield.as_str());
        assert_eq!((stats.attack, stats.health), (24, 24));
        let ally = s.card(ALLY).id.clone();
        burn(&mut s, at(&ally));
        assert_eq!(s.card(ally.as_str()).damage, 0);
        assert_eq!(s.stats(shield.as_str()).health, 24 - (HIT - 8), "9 through Armor 8");
    }
}
