//! FvH #68 Closeted Commentator (SPEC §8.14 row 68): (1) Unit, Human, Rare, 3/1 → 6/2. Built by S08 (#5258).
//!   Base:    "Evolves when a card transforms.\nEvolve: Gain the Felinor tag and Allure every enemy Unit."
//!   Radiant: "Evolves immediately.\nEvolve: Gain the Felinor tag and Allure every enemy Unit."
//!
//! "Evolves when" (R3536001). Base: the card's own trigger on `transformed`, any card of either side in
//! any zone while it stands on the field (a hidden zone's transform shows only that one happened, R177),
//! which evolves it (IA #112 Evolutionary Loop's shape). Radiant (the list's "Evolve IMMEDIATELY"): an
//! always-true `evolves_when`, so it evolves at the first settled state check after it enters, played or
//! summoned. It evolves once (R3536000), the multiplier as ever (R3536003). Evolve: `grant_tag` Felinor
//! on itself (R923) and `allure_enemy_units`, each enemy Unit stolen at the start of its controller's
//! next turn, or destroyed with no room (R963). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-068";

/// Whether this card stands on the field not yet evolved.
fn unevolved(ctx: &EffectContext<'_>) -> bool {
    ctx.live_self().is_some_and(|me| me.evolved != Some(true))
}

/// "Evolve: Gain the Felinor tag and Allure every enemy Unit."
fn come_out() -> Hook {
    hook(|_ctx| {
        vec![
            grant_tag(GrantTagArgs {
                target: Some(TargetSpec::SelfCard),
                instance_id: None,
                tag: Tag::Felinor,
            }),
            allure_enemy_units(),
        ]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![
            TriggerDef::new("closeted-commentator", &[GameEventType::Transformed], |ctx, _event| {
                if !unevolved(ctx) {
                    return vec![];
                }
                vec![evolve(json_as(json!({ "target": { "of": "self" } })))]
            })
            .with_when(|ctx, _event| unevolved(ctx)),
        ],
        evolve: Some(come_out()),
        ..Script::default()
    };
    let radiant = Script {
        evolves_when: Some(read_hook(|_args| true)),
        evolve: Some(come_out()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// FvH #68 Closeted Commentator — SPEC §8.14 row 68.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const POLYMORPH: &str = "meditative-024"; // Polymorph: transforms a Unit
    const SPELL: &str = "stand-in (0) Spell";
    const OTHER: &str = "stand-in (1) 3/3";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand

    fn evolved(s: &Scenario) -> bool {
        s.card(ID).evolved == Some(true)
    }

    fn is_felinor(s: &Scenario) -> bool {
        tags_of(s.state(), s.card(ID)).contains(&Tag::Felinor)
    }

    fn enemy_units(s: &Scenario) -> usize {
        active_units_of(s.state(), P2).len()
    }

    mod base {
        use super::*;

        fn watching() -> Scenario {
            scenario(json!({
                "p1": { "field": [ID, OTHER], "hand": [POLYMORPH, SPELL, FILLER] },
                "p2": { "field": [OTHER], "hand": [FILLER] },
            }))
        }

        #[test]
        fn an_enemy_units_transform_evolves_it_into_a_felinor_that_allures_every_enemy_unit() {
            let mut s = watching();
            let theirs = s.unit_at(P2, 1).id;
            assert!(!is_felinor(&s));
            s.play(POLYMORPH, pick::targets(pick::id(&theirs)));
            assert!(evolved(&s));
            assert!(is_felinor(&s));
            assert_eq!(enemy_units(&s), 1, "the Allure lands on its controller's next turn");
            s.end_turn().end_turn();
            assert_eq!(enemy_units(&s), 0, "stolen at the start of p1's next turn");
            assert_eq!(active_units_of(s.state(), P1).len(), 3);
        }

        #[test]
        fn a_transform_on_its_own_side_counts_too() {
            let mut s = watching();
            let mine = s.unit_at(P1, 2).id;
            s.play(POLYMORPH, pick::targets(pick::id(&mine)));
            assert!(evolved(&s));
            assert!(is_felinor(&s));
        }

        #[test]
        fn no_transform_no_evolve() {
            let mut s = watching();
            s.play(SPELL, json!({}));
            assert!(!evolved(&s));
            assert!(!is_felinor(&s));
            s.end_turn().end_turn();
            assert_eq!(enemy_units(&s), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn played_it_evolves_at_once_into_a_felinor_that_allures_every_enemy_unit() {
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER] },
                "p2": { "field": [OTHER, OTHER], "hand": [FILLER] },
            }));
            s.play(ID, json!({}));
            assert!(evolved(&s));
            assert!(is_felinor(&s));
            assert_eq!(enemy_units(&s), 2);
            s.end_turn().end_turn();
            assert_eq!(enemy_units(&s), 0);
            assert_eq!(active_units_of(s.state(), P1).len(), 3);
        }
    }
}
