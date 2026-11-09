//! S T-propagation-layer Drone Battleship (Propagation Layer) (SPEC §8.10 row T-propagation-layer): (5)
//! Unit, Token (printed Mythic), 12/12 → 24/24, a token of the Stinkyscape set.
//!   Base:    "Flying, Ranged, Can't attack\nActivate 2: Summon a Drone Interceptor.\nActivate: Summon a
//!            Drone Corvette."
//!   Radiant: "Flying, Ranged, Can't attack\nActivate 2: Summon a Radiant Drone Interceptor.\nActivate:
//!            Summon a Radiant Drone Corvette."
//! Engine: the middle of the Drone Battleship's pile (S #36, R1842000): beneath the Shield Layer it acts for
//!   nothing and offers no Activate (R13); on top, two Activate abilities (R384), their uses counted apart
//!   (R1751000): "Activate 2" summons a Drone Interceptor, twice a turn at most, and "Activate" a Drone
//!   Corvette, once, each into the leftmost open unit zone (R64), no Cry (R1), the drone drawing its own
//!   combat keywords as it arrives (R781004). Flying, Ranged and Can't attack are catalog keywords. It is
//!   no Starship (R3038002): no Starship tag, so it is never launched and no piece fuses into it. A unit
//!   token: it ceases to exist off the field (R11). Radiant: Radiant drones.

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

    /// R3038002: no Starship. It prints Flying and Ranged where it printed the Starship keyword and
    /// carries no Starship tag, so it offers no launch and takes no piece: a dying piece summons its
    /// controller a Dormant Starship instead.
    #[test]
    fn r3038002_it_has_flying_and_ranged_is_never_launched_and_takes_no_piece() {
        use jackioh_engine::prelude::{STARSHIP_LAUNCH_ABILITY, is_launched_starship, tags_of};
        const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: Starship Piece, Equip, Pierce
        for radiant in [false, true] {
            let (mut s, ship) = crate::built_starship(
                "r3038002-propagation",
                json!([{ "def": AP_ROUNDS, "lane": 2 }]),
                json!({ "field": [{ "def": PROPAGATION, "lane": 1, "radiant": radiant }] }),
                json!({}),
            );
            let layer = s.unit_at(P1, 1);
            assert_eq!(layer.def_id, PROPAGATION);
            let stats = s.stats(layer.id.as_str());
            let kinds: Vec<KeywordKind> = stats.keywords.iter().map(Keyword::kind).collect();
            assert!(kinds.contains(&KeywordKind::Flying), "{kinds:?}");
            assert!(kinds.contains(&KeywordKind::Ranged), "{kinds:?}");
            assert!(!kinds.contains(&KeywordKind::Starship), "{kinds:?}");
            assert!(!tags_of(s.state(), &layer).contains(&Tag::Starship));
            // The dead piece went into a Dormant Starship of its own, not into this.
            assert_ne!(ship, layer.id);
            assert!(is_dormant_starship(s.card(ship.as_str())));
            let after = s.card(layer.id.as_str()).clone();
            assert_eq!(after.starship, None, "no piece is fused into it");
            assert_eq!(after.def_id, PROPAGATION);
            let printed = if radiant { (24, 24) } else { (12, 12) };
            assert_eq!((stats.attack, stats.health), printed, "radiant {radiant}");
            // And no launch: neither offered nor taken.
            assert!(!is_dormant_starship(&after) && !is_launched_starship(&after));
            let launch = ActionBody::Activate {
                instance_id: layer.id.clone(),
                ability: Some(STARSHIP_LAUNCH_ABILITY.to_string()),
                targets: None,
                modes: None,
                tributes: None,
            };
            assert!(!legal_actions(s.state(), P1).contains(&launch));
            s.expect_refused(|s| {
                s.activate(
                    layer.id.as_str(),
                    json!({ "ability": STARSHIP_LAUNCH_ABILITY }),
                )
            });
        }
    }
}
