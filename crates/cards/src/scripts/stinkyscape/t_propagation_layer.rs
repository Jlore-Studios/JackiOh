//! S T-propagation-layer Drone Battleship (Propagation Layer) (SPEC §8.10 row T-propagation-layer): (5)
//! Unit, Token (printed Mythic), 12/12 → 24/24, a token of the Stinkyscape set.
//!   Base:    "Starship, Can't attack\nActivate 2: Summon a Drone Interceptor.\nActivate: Summon a Drone
//!            Corvette."
//!   Radiant: "Starship, Can't attack\nActivate 2: Summon a Radiant Drone Interceptor.\nActivate: Summon a
//!            Radiant Drone Corvette."
//! Engine: the middle of the Drone Battleship's pile (S #36, R1842000): beneath the Shield Layer it acts for
//!   nothing and offers no Activate (R13); on top, two Activate abilities (R384), their uses counted apart
//!   (R1751000): "Activate 2" summons a Drone Interceptor, twice a turn at most, and "Activate" a Drone
//!   Corvette, once, each into the leftmost open unit zone (R64), no Cry (R1), the drone drawing its own
//!   combat keywords as it arrives (R781004). Starship and Can't attack are catalog keywords. A unit token:
//!   it ceases to exist off the field (R11). Radiant: Radiant drones.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-propagation-layer";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

/// "Activate 2": the Interceptor's uses a turn.
const INTERCEPTOR_USES: i32 = 2;
/// "Activate": the Corvette's use a turn.
const CORVETTE_USES: i32 = 1;

/// R384: one Activate that summons a `def_id` drone, Radiant on `radiant`.
fn summoning(id: &str, label: &str, uses: i32, def_id: &'static str, radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: id.to_string(),
        label: label.to_string(),
        uses: ActivationUses::Count(uses),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |_ctx| vec![summon(json_as(json!({ "defId": def_id, "radiant": radiant })))]),
    }
}

fn face(radiant: bool) -> Script {
    let (interceptor, corvette) = if radiant {
        ("Summon a Radiant Drone Interceptor", "Summon a Radiant Drone Corvette")
    } else {
        ("Summon a Drone Interceptor", "Summon a Drone Corvette")
    };
    Script {
        activations: vec![
            summoning("interceptor", interceptor, INTERCEPTOR_USES, INTERCEPTOR, radiant),
            summoning("corvette", corvette, CORVETTE_USES, CORVETTE, radiant),
        ],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-propagation-layer — SPEC §8.10 row T-propagation-layer, BUILD M12 row S T-propagation-layer.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const PROPAGATION: &str = "stinkyscape-t-propagation-layer";
    const SHIELD: &str = "stinkyscape-t-shield-layer";
    const MELTDOWN: &str = "stinkyscape-t-meltdown-layer";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const FILLER: &str = "core-005";

    fn pile(radiant: bool, above: &[&str]) -> Scenario {
        let mut field = vec![
            json!({ "def": MELTDOWN, "radiant": radiant, "lane": 1 }),
            json!({ "def": PROPAGATION, "radiant": radiant, "stack": true }),
        ];
        field.extend(above.iter().map(|def| json!({ "def": def, "radiant": radiant, "stack": true })));
        scenario(json!({
            "p1": { "hand": [FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn activate<'a>(s: &'a mut Scenario, ability: &str) -> &'a mut Scenario {
        s.activate(PROPAGATION, json!({ "ability": ability }))
    }

    fn drones(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (2..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    #[test]
    fn r13_beneath_the_shield_layer_it_offers_no_activate() {
        let mut s = pile(false, &[SHIELD]);
        assert_eq!(s.unit(P1, 1).expect("the top").def_id, SHIELD);
        s.expect_refused(|s| activate(s, "interceptor"));
        s.expect_refused(|s| activate(s, "corvette"));
        assert_eq!(drones(&s), vec![None; 4]);
    }

    #[test]
    fn r1751000_r384_two_interceptors_and_one_corvette_a_turn() {
        let mut s = pile(false, &[]);
        activate(&mut s, "interceptor");
        activate(&mut s, "corvette");
        activate(&mut s, "interceptor");
        s.expect_refused(|s| activate(s, "interceptor"));
        s.expect_refused(|s| activate(s, "corvette"));
        let drone = |def: &str| Some((def.to_string(), false));
        assert_eq!(
            drones(&s),
            vec![drone(INTERCEPTOR), drone(CORVETTE), drone(INTERCEPTOR), None]
        );
        s.end_turn();
        s.end_turn();
        activate(&mut s, "interceptor");
        assert_eq!(drones(&s)[3], drone(INTERCEPTOR));
    }

    #[test]
    fn it_never_attacks() {
        let mut s = pile(false, &[]);
        s.expect_refused(|s| s.attack(PROPAGATION, "hero"));
    }

    #[test]
    fn radiant_24_24_and_its_drones_are_radiant() {
        let mut s = pile(true, &[]);
        let stats = s.stats(PROPAGATION);
        assert_eq!((stats.attack, stats.health), (24, 24));
        activate(&mut s, "interceptor");
        activate(&mut s, "corvette");
        let drone = |def: &str| Some((def.to_string(), true));
        assert_eq!(drones(&s), vec![drone(INTERCEPTOR), drone(CORVETTE), None, None]);
    }
}
