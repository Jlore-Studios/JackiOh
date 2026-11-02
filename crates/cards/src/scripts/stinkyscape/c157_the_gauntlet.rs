//! S #157 The Gauntlet (SPEC §8.10 row 157): (1) Field Spell, Epic, of the Stinkyscape set.
//!   Base:    "Brittle 3 / The unit zones of the middle 3 lanes are trapped in a Gauntlet. Units there can
//!            attack only each other, and no other Unit can attack them."
//!   Radiant: the same, and "Activate: Choose one: give this +{brittleGain} Brittle, or Tribute this."
//! Engine: Brittle 3 is a catalog keyword (§6.1), its count started as the card enters the field.
//! `StaticFlags.gauntlet` (R1849005): while it acts on the field, the unit zones of lanes
//! `GAUNTLET_LANES` on both sides are in it; at §4.2 step 2 a Unit in them may attack only an enemy Unit
//! in them, never a hero, and a Unit outside attacks none of them, forced attacks included; one check
//! for `legalActions` and the refusal. Attacks only: a Unit's text still targets across it. Radiant:
//! Activate (R384), once a turn, two modes (R81): `gain_brittle` +{brittleGain} on itself, or Tribute
//! this, a `sacrifice` of itself (R101).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-157";

const GAIN_BRITTLE_MODE: &str = "gain Brittle";
const TRIBUTE_MODE: &str = "Tribute this";

/// R384: "Activate", once a turn.
const ACTIVATE_USES: i32 = 1;

fn choose_one(ctx: &EffectContext<'_>) -> Vec<Effect> {
    match ctx.modes.first().map(String::as_str) {
        Some(GAIN_BRITTLE_MODE) => vec![gain_brittle(json_as(json!({
            "target": { "of": "self" },
            "n": param(ctx, "brittleGain"),
        })))],
        Some(TRIBUTE_MODE) => vec![sacrifice(json_as(json!({ "target": { "of": "self" } })))],
        _ => Vec::new(),
    }
}

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            gauntlet: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = Script {
        activations: vec![ActivationDecl {
            id: "gauntlet".to_string(),
            label: "Choose one".to_string(),
            uses: ActivationUses::Count(ACTIVATE_USES),
            cost: None,
            targets: Vec::new(),
            modes: vec![ModeDecl {
                kind: PromptKind::Mode,
                options: vec![GAIN_BRITTLE_MODE.to_string(), TRIBUTE_MODE.to_string()],
            }],
            can_activate: None,
            has: None,
            run: hook(|ctx| choose_one(ctx)),
        }],
        ..base.clone()
    };
    CardScripts { base, radiant }
}

// S #157 The Gauntlet — SPEC §8.10 row 157, BUILD M12 row S 157.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{GAIN_BRITTLE_MODE, ID, TRIBUTE_MODE};

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const MOTHS: &str = "core-009"; // (2) 1/14: at its controller's start of turn every enemy Unit attacks it.

    const BARRED: &str = "the Gauntlet's Units fight only each other";

    fn played(radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }],
                "library": crate::filler(10),
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                "library": crate::filler(10),
            },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn offered_targets(s: &Scenario, attacker: &CardInstance) -> Vec<String> {
        legal_actions(s.state(), P1)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Attack { attacker_id, target_id } if *attacker_id == attacker.id => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn brittle_count(s: &Scenario) -> Option<i32> {
        s.card(ID).brittle.map(|brittle| brittle.count)
    }

    mod base {
        use super::*;

        #[test]
        fn a_middle_unit_attacks_only_a_middle_enemy_and_legal_actions_agree() {
            let mut s = played(false);
            let middle = s.unit_at(P1, 3);
            let inside = s.unit_at(P2, 2);
            assert_eq!(offered_targets(&s, &middle), vec![inside.id.clone()]);
            s.expect_refused_with(|s| s.attack(&middle, "hero"), BARRED);
            s.expect_refused_with(|s| s.attack(&middle, s.unit_at(P2, 1)), BARRED);
            s.attack(&middle, &inside);
            s.expect_in_zone(&inside, "graveyard");
        }

        #[test]
        fn an_outer_unit_attacks_none_of_them() {
            let mut s = played(false);
            let outer = s.unit_at(P1, 1);
            let mut offered = offered_targets(&s, &outer);
            offered.sort();
            let mut expected = vec![s.unit_at(P2, 1).id, "hero-p2".to_string()];
            expected.sort();
            assert_eq!(offered, expected);
            s.expect_refused_with(|s| s.attack(&outer, s.unit_at(P2, 2)), BARRED);
            s.attack(&outer, "hero");
            s.expect_health(P2, HERO_HEALTH - 4);
        }

        #[test]
        fn forced_attacks_obey_it_too() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, FILLER], "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }] },
                "p2": { "hand": [FILLER], "field": [{ "def": MOTHS, "lane": 2 }] },
            }));
            s.play(ID, json!({ "zone": 1 }));
            let outer = s.unit_at(P1, 1);
            let middle = s.unit_at(P1, 3);
            s.end_turn();
            let forced: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::AttackDeclared { attacker_id, forced: true, .. } => Some(attacker_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(forced, vec![middle.id.clone()]);
            assert!(!forced.contains(&outer.id));
        }

        #[test]
        fn brittle_3_crumbles_it() {
            let mut s = played(false);
            assert_eq!(brittle_count(&s), Some(3));
            for left in [2, 1] {
                s.end_turn().end_turn();
                assert_eq!(brittle_count(&s), Some(left));
                s.expect_in_zone(ID, "field");
            }
            s.end_turn().end_turn();
            s.expect_in_zone(ID, "graveyard");
            s.attack(s.unit_at(P1, 3), "hero");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn mode_1_adds_2_brittle_through_param() {
            let mut s = played(true);
            assert_eq!(brittle_count(&s), Some(3));
            s.activate(ID, json!({ "modes": [GAIN_BRITTLE_MODE] }));
            assert_eq!(brittle_count(&s), Some(5));
            s.expect_refused(|s| s.activate(ID, json!({ "modes": [GAIN_BRITTLE_MODE] })));
            s.end_turn().end_turn();
            step_param(s.card_mut(ID), "brittleGain", 1);
            let before = brittle_count(&s).expect("a count");
            s.activate(ID, json!({ "modes": [GAIN_BRITTLE_MODE] }));
            assert_eq!(brittle_count(&s), Some(before + 3));
        }

        #[test]
        fn mode_2_tributes_it() {
            let mut s = played(true);
            s.activate(ID, json!({ "modes": [TRIBUTE_MODE] }));
            s.expect_in_zone(ID, "graveyard");
            s.attack(s.unit_at(P1, 3), "hero");
            s.expect_health(P2, HERO_HEALTH - 4);
        }
    }
}
