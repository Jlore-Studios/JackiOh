//! Meditative+ #16.1 Jazakus Prime (SPEC §8.9 row 16.1, §7; ME-CRAFT's potion mode, R880–R883,
//! R1182000, R1182001). (1) Unit, Human, Prime, Token (printed Epic), 8/8 → 16/16.
//!   Base:    "Cry: Craft a HUGE custom Potion. It costs (3)."
//!   Radiant: "Cry: Craft a HUGE custom Radiant Potion. It costs (3)."
//!
//! M+ #16 Jazakus's crafter in its HUGE frame (R1182001): the same fixed frame, a Unit tagged
//! Potion whose one hat is "Activate: Tribute this.", at cost (3) (`CRAFT_HUGE_POTION_COST`) with
//! `CRAFT_HUGE_POTION_BUDGET_MULTIPLE` (5) times a Potion's points, 35 against 7: "about 5 times
//! better than the normal custom potion". Its lines (24), effects (8) and numbers (1 to 10) are the
//! frame's. The prompt's four seeded HUGE Potions answer for the AI, the policy and a timeout
//! (R881). Only M+ #16's Death puts one in a deck (a Radiant one on its Radiant face).
//!
//! Voice audio is owed until the set ships (R1420, R1448): the `card-audio.json5` entry is written.

use jackioh_engine::effects::{choose_craft, chosen_recipe, craft_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-016-1";

/// The `data`-less resume step the craft answer re-enters.
const CRAFT: &str = "craft";

/// The two faces differ only in `radiant`: the HUGE Potion is Radiant on the Radiant face.
fn jazakus_prime(radiant: bool) -> Script {
    let mint: Hook = hook(move |ctx| match chosen_recipe(ctx) {
        Some(recipe) => vec![craft_card(json_as(json!({
            "recipe": recipe,
            "radiant": radiant,
        })))],
        None => vec![],
    });
    Script {
        cry: Some(hook(move |_ctx| {
            vec![choose_craft(json_as(json!({
                "step": CRAFT,
                "mode": "hugePotion",
                "radiant": radiant,
                "prompt": "Craft a HUGE custom Potion",
            })))]
        })),
        resume: IndexMap::from([(CRAFT, mint)]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jazakus_prime(false),
        radiant: jazakus_prime(true),
    }
}

// Meditative+ #16.1 Jazakus Prime — SPEC §8.9 row 16.1, §7, R70, R881, R1182001.
//
// BUILD M11 row M+ 16.1: "Cry (played or cast): the potion crafter at cost (3) with five times the
// normal potion's points; a recipe within it is valid and one over it refused; radiant 16/16 and the
// potion Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::effects::{CastNewArgs, cast_new};
    use jackioh_engine::subsystems::craft::points_budget;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // 4/4

    /// A 10/10 HUGE Potion whose Activate deals 10 and gains `armor` Armor: 19 + 10 + `armor` points.
    fn huge_recipe(armor: i32) -> Value {
        json!({
            "cost": 3, "type": "Unit", "adjective": "Lazy", "noun": "Iterator",
            "attack": 10, "health": 10, "keywords": [], "echo": 0,
            "hats": [{ "hat": "activate", "effects": [
                { "verb": "damageEnemyHero", "n": 10 }, { "verb": "gainArmor", "n": armor },
            ] }],
            "frame": "hugePotion",
        })
    }

    fn sides(radiant: bool) -> Value {
        json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10, "library": [VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        })
    }

    /// The Prime played into lane 1, its craft prompt open.
    fn crafting(radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(sides(radiant));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// The crafted card in p1's hand.
    fn crafted(s: &Scenario) -> CardInstance {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id.starts_with("craft:"))
            .expect("the crafted card")
    }

    #[test]
    fn declares_no_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r1182001_the_cry_opens_the_huge_potion_crafter_at_cost_3_with_five_times_the_points() {
            let s = crafting(false);
            let pending = s.state().pending.clone().expect("the craft prompt is open");
            assert_eq!(pending.kind, PromptKind::Craft);
            assert_eq!(pending.budget, Some(CRAFT_HUGE_POTION_COST));
            assert_eq!(pending.craft_mode, Some(CraftMode::HugePotion));
            assert_eq!(
                points_budget(Some(CraftFrame::HugePotion), 3),
                CRAFT_HUGE_POTION_BUDGET_MULTIPLE * points_budget(Some(CraftFrame::Potion), 1)
            );
            assert_eq!(pending.options.len(), CRAFT_PRESETS);
            for option in &pending.options {
                match &option.selection {
                    Selection::Craft { recipe } => {
                        assert_eq!(recipe.frame, Some(CraftFrame::HugePotion));
                        assert_eq!(recipe.cost, 3);
                    }
                    other => panic!("a preset is a recipe, not {other:?}"),
                }
            }
        }

        #[test]
        fn r1182001_a_recipe_within_the_35_points_is_crafted_and_one_over_is_refused() {
            let mut s = crafting(false);
            // 19 + 10 + 7 = 36 points, one over.
            s.expect_refused_with(
                |s| s.answer(json!([{ "pick": "craft", "recipe": huge_recipe(7) }])),
                "over the budget of 35",
            );
            assert!(s.state().pending.is_some(), "the prompt stays open");
            // 19 + 10 + 6 = 35, the whole budget.
            s.answer(json!([{ "pick": "craft", "recipe": huge_recipe(6) }]));
            let potion = crafted(&s);
            assert!(!potion.radiant);
            let def = s.state().transient_defs.get(&potion.def_id).expect("its definition").clone();
            assert_eq!(def.name, "HUGE Lazy Iterator Potion");
            assert_eq!(def.cost, CardCost::Fixed(3));
            assert_eq!(def.tags, vec![Tag::Potion]);
            assert_eq!(def.set, SetName::MeditativePlus);
            s.play(&potion.id, json!({ "zone": 2 }));
            s.activate(&potion.id, json!({}));
            s.expect_health(P2, 20);
            assert_eq!(s.state().players.p1.hero.armor, 6);
            s.expect_in_zone(&potion.id, "graveyard");
        }

        #[test]
        fn r1182001_a_normal_potion_is_refused_in_the_huge_frame() {
            let mut s = crafting(false);
            let normal = json!({
                "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
                "attack": 2, "health": 2, "keywords": [], "echo": 0,
                "hats": [{ "hat": "activate", "effects": [{ "verb": "damageEnemyHero", "n": 4 }] }],
                "frame": "potion",
            });
            s.expect_refused_with(
                |s| s.answer(json!([{ "pick": "craft", "recipe": normal }])),
                "this prompt crafts a HUGE Potion",
            );
        }

        #[test]
        fn r70_a_cast_prime_opens_the_crafter_too() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "mana": 10, "library": [VANILLA] },
                "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
            }));
            // A fixture Spell that casts a Jazakus Prime, as an effect's cast would (R70).
            let face = json!({ "keywords": [], "text": "fx-cast-prime" });
            let caster: CardDef = json_as(json!({
                "id": "fx-cast-prime", "index": "fx-cast-prime", "name": "fx-cast-prime", "set": "Core",
                "type": "Spell", "tags": [], "rarity": "Common", "token": false, "cost": 0,
                "base": face, "radiant": face,
            }));
            s.state_mut().transient_defs.insert(caster.id.clone(), caster);
            let script = Script {
                cry: Some(hook(|_ctx| {
                    vec![cast_new(CastNewArgs {
                        def: ID.into(),
                        radiant: None,
                        how: Default::default(),
                    })]
                })),
                ..Script::default()
            };
            let mut scripts = registered_scripts();
            scripts.insert(
                "fx-cast-prime".to_string(),
                CardScripts {
                    base: script.clone(),
                    radiant: script,
                },
            );
            register_scripts(scripts);
            let spell = new_instance(s.state_mut(), "fx-cast-prime", P1, Zone::Hand { player: P1 });
            s.state_mut().players.p1.hand.push(spell.clone());
            s.play(&spell, json!({}));
            let pending = s.state().pending.clone().expect("the cast Prime's craft prompt");
            assert_eq!(pending.player_id, P1);
            assert_eq!(pending.craft_mode, Some(CraftMode::HugePotion));
            s.answer(json!([{ "pick": "craft", "recipe": huge_recipe(6) }]));
            assert!(crafted(&s).def_id.starts_with("craft:"));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1182001_the_huge_potion_is_radiant_doubling_its_numbers_and_stats() {
            let mut s = crafting(true);
            s.answer(json!([{ "pick": "craft", "recipe": huge_recipe(6) }]));
            let potion = crafted(&s);
            assert!(potion.radiant, "the Radiant face crafts it Radiant");
            let def = s.state().transient_defs.get(&potion.def_id).expect("its definition").clone();
            assert_eq!([def.radiant.attack, def.radiant.health], [Some(20), Some(20)]);
            s.play(&potion.id, json!({ "zone": 2 }));
            s.activate(&potion.id, json!({}));
            s.expect_health(P2, 10);
            assert_eq!(s.state().players.p1.hero.armor, 12);
        }
    }
}
