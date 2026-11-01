//! Stinkyscape #141 Fruit Launcher (SPEC §8.10 row 141). Unit 0/4 → 0/8, Weapon, cost 2, Rare.
//!   Base:    "Starship Piece, Equip
//!            Activate: Choose a permanent. Cast a random Fruit on it."
//!   Radiant: "Starship Piece, Equip
//!            Activate: Choose a permanent. Cast a random Radiant Fruit on it."
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it
//! fuses into it (R1086), which then takes the Activate (R1751000) and is a piece only if it was one
//! (R1852001); played alone, its death fuses a copy into its owner's Dormant Starship (R1852000).
//! The Activate's target is a declared permanent, either side (R81), once a turn (R384). R1862004:
//! `cast_random_at` casts one random card of the Fruit pool, free and counted as played by its
//! controller (R70, R452), aimed at that permanent for its first target where it may take it (R946),
//! every other choice random. Radiant: the Fruit is cast Radiant.
//!
//! The pool (#2521, R2521006): every Fruit-tagged card of the sets that ship, tokens included, which
//! its query states (`withTokens`, as R382 lets a pool name its tokens): the Fruits, and the five
//! Grapes, C+ #65.1 to #65.5, the Fruit tag keeping every other token out; Organic Produce and Fruit
//! Tree left it (R1837000). A Grape picked is rolled again on `GRAPE_ODDS`, as R382 rolls every Grape
//! a pool generates, and the Grape that roll names is the one cast.

use jackioh_engine::effects::cast_random_at;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-141";

fn launch(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "fruit".into(),
        label: "Cast a random Fruit on a permanent".into(),
        uses: ActivationUses::Count(1),
        cost: None,
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"] }),
        )],
        modes: vec![],
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            let Some(aim) = ctx.targets.first().map(option_key) else {
                return Vec::new();
            };
            vec![cast_random_at(
                json_as(json!({ "query": { "tags": [Tag::Fruit], "withTokens": true }, "radiant": radiant })),
                aim,
            )]
        }),
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            activations: vec![launch(false)],
            ..Script::default()
        },
        radiant: Script {
            activations: vec![launch(true)],
            ..Script::default()
        },
    }
}

// Stinkyscape #141 Fruit Launcher — SPEC §8.10 row 141, BUILD M12 row S 141: its Activate takes a
// permanent and casts one random Fruit at it for its controller, free (R70), by an rng pick that
// replays, a Grape among them (tokens included, R2521006); a Fruit whose target cannot be that
// permanent makes its own choices at random; radiant 0/8 and a Radiant Fruit.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon_board;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FIG: &str = "core-047"; // Fig of Life, a Fruit: "Heal a target {heal}."
    /// C+ #65.1 to #65.5, the Fruit tokens (R382).
    const GRAPES: [&str; 5] = [
        "classicplus-065-1",
        "classicplus-065-2",
        "classicplus-065-3",
        "classicplus-065-4",
        "classicplus-065-5",
    ];

    /// p2's Vanilla, 2 damage on it, and p1's hero hurt too, so a heal has somewhere else to go.
    fn board(seed: &str, radiant: bool) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }], "health": 20 }),
            json!({ "field": [{ "def": VANILLA, "damage": 2 }], "health": 20 }),
        )
    }

    fn fire(seed: &str, radiant: bool) -> (Scenario, String, Vec<(PlayerId, String, i32)>) {
        let mut s = board(seed, radiant);
        let vanilla = s.unit_at(P2, 1).id;
        s.activate(ID, json!({ "ability": "fruit", "targets": pick::id(&vanilla) }));
        let cast = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::CardPlayed {
                    player,
                    def_id,
                    cost_paid,
                    ..
                } => Some((*player, def_id.clone(), *cost_paid)),
                _ => None,
            })
            .collect();
        (s, vanilla, cast)
    }

    fn fig_seed(prefix: &str, radiant: bool) -> String {
        seed_where(prefix, |seed| {
            fire(seed, radiant)
                .2
                .first()
                .is_some_and(|(_, def_id, _)| def_id == FIG)
        })
    }

    /// R2521006: the first seed, under `prefix`, whose cast is a Grape.
    fn grape_seed(prefix: &str, radiant: bool) -> String {
        seed_where(prefix, |seed| {
            fire(seed, radiant)
                .2
                .first()
                .is_some_and(|(_, def_id, _)| GRAPES.contains(&def_id.as_str()))
        })
    }

    fn healed(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Healed { target_id, .. } => Some(target_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1862004_casts_one_random_fruit_free_for_its_controller() {
            let (s, _, cast) = fire("s141", false);
            let first = cast.first().cloned().expect("a cast");
            assert_eq!((first.0, first.2), (P1, 0));
            assert!(
                crate::card_def(&first.1).tags.contains(&Tag::Fruit),
                "{}",
                first.1
            );
            assert!(s.state().pending.is_none(), "a random cast asks nobody");
            assert_eq!(fire("s141", false).2.first(), Some(&first));
        }

        #[test]
        fn r2521006_its_pool_takes_the_fruit_tokens_a_grape_is_cast_free_for_its_controller() {
            let (s, _, cast) = fire(&grape_seed("s141-grape", false), false);
            let (player, def_id, paid) = cast.first().cloned().expect("a cast");
            assert!(GRAPES.contains(&def_id.as_str()), "{def_id}");
            assert_eq!((player, paid), (P1, 0));
            let def = crate::card_def(&def_id);
            assert!(def.token && def.tags.contains(&Tag::Fruit), "a Fruit token");
            assert!(s.state().pending.is_none(), "a random cast asks nobody");
            // No other token: every cast over the seeds is a Fruit, and a token only if a Grape.
            for n in 0..16 {
                let (_, _, cast) = fire(&format!("s141-pool-{n}"), false);
                let id = cast.first().map(|(_, id, _)| id.clone()).expect("a cast");
                let def = crate::card_def(&id);
                assert!(def.tags.contains(&Tag::Fruit), "{id}");
                assert!(!def.token || GRAPES.contains(&id.as_str()), "{id}");
            }
        }

        #[test]
        fn r1862004_a_fruit_that_can_take_the_permanent_is_aimed_at_it() {
            for prefix in ["s141-fig-a", "s141-fig-b", "s141-fig-c"] {
                let (s, vanilla, _) = fire(&fig_seed(prefix, false), false);
                assert_eq!(healed(&s), vec![vanilla], "{prefix}");
            }
        }
    }

    mod random_choice {
        use super::*;

        const ARMOR: &str = "core-073"; // (2) Field Spell: no Unit, nothing a heal reaches.

        fn fire_at_field_spell(seed: &str) -> (Scenario, String, Vec<String>) {
            let mut s = weapon_board(
                seed,
                json!({ "field": [ID], "backrow": [ARMOR], "health": 20 }),
                json!({ "field": [{ "def": VANILLA, "damage": 2 }], "health": 20 }),
            );
            let field_spell = s.backrow(P1, 1).expect("the Field Spell").id;
            s.activate(
                ID,
                json!({ "ability": "fruit", "targets": pick::id(&field_spell) }),
            );
            let cast = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::CardPlayed { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            (s, field_spell, cast)
        }

        #[test]
        fn r1862004_a_fruit_that_cannot_take_the_permanent_makes_its_own_choices_at_random() {
            let seed = seed_where("s141-random", |seed| {
                fire_at_field_spell(seed).2.first().is_some_and(|id| id == FIG)
            });
            let (s, field_spell, _) = fire_at_field_spell(&seed);
            assert!(s.state().pending.is_none(), "it asks nobody");
            let healed = healed(&s);
            assert_eq!(healed.len(), 1, "{healed:?}");
            assert_ne!(healed[0], field_spell, "not the Field Spell");
            assert_eq!(super::healed(&fire_at_field_spell(&seed).0), healed);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1862004_the_fruit_is_cast_radiant() {
            let (s, vanilla, _) = fire(&fig_seed("s141-radiant", true), true);
            assert_eq!(healed(&s), vec![vanilla]);
            let fig = s
                .pile(P1, "graveyard")
                .into_iter()
                .find(|card| card.def_id == FIG)
                .expect("the Fig landed");
            assert!(fig.radiant);
        }

        #[test]
        fn r2521006_a_grape_is_cast_radiant() {
            let (s, _, cast) = fire(&grape_seed("s141-grape-radiant", true), true);
            let (_, def_id, _) = cast.first().cloned().expect("a cast");
            assert!(GRAPES.contains(&def_id.as_str()), "{def_id}");
            let grape = s
                .pile(P1, "graveyard")
                .into_iter()
                .find(|card| card.def_id == def_id)
                .expect("the Grape landed in its caster's graveyard (R87)");
            assert!(grape.radiant);
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board("s141", true);
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (0, 8));
        }
    }
}
