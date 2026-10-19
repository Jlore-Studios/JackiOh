//! Meditative+ #26 Scratch Pad (SPEC §8.9 row 26, ME-CRAFT's from-scratch mode, R880–R883,
//! R1182002, R178). (0) Spell, Mythic.
//!   Base:    "Code a card from scratch and add it to your hand. Exile this."
//!   Radiant: "Code a Radiant card from scratch and add it to your hand. Exile this."
//!
//! THE CHAIN is Meditative #17 True Craft a Card's (§10.6, R880): a `number` prompt for the crafted
//! card's cost, 0 to `CRAFT_MAX_COST` (4), whose answer opens the `craft` prompt at that budget
//! (`2 + 5 ×` the cost points, 24 lines), whose answer mints the card into the caster's hand —
//! Radiant on the Radiant face — or burns it on a full one (§2.4). The craft prompt carries the
//! from-scratch flag (`CraftMode::Scratch`, R1182002): the client opens an empty canvas with no
//! presets, tooltips or hints, while the engine still rolls M #17's four presets so the AI, the
//! random policy and a timeout answer with one (R881). The recipes, the budget and the card a
//! recipe makes are M #17's: "from scratch" is the canvas, not a smaller budget.
//!
//! EXILE THIS (R178): after the mint, Scratch Pad goes to its owner's exile, not the graveyard.
//!
//! Voice audio is owed with the patch that ships the set (R1420): the `card-audio.json5` entry is
//! written, and the release renders it (`[gen:voice]`, R1448).

use jackioh_engine::effects::{choose_craft, choose_number, chosen_number, chosen_recipe, craft_card, exile};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-026";

/// The `data`-less resume steps: the cost answer opens the canvas, the craft answer mints.
const COST: &str = "cost";
const CRAFT: &str = "craft";

/// The two faces differ only in `radiant`: the Radiant face codes the card Radiant.
fn scratch_pad(radiant: bool) -> Script {
    let open_canvas: Hook = hook(move |ctx| match chosen_number(ctx) {
        Some(cost) => vec![choose_craft(json_as(json!({
            "step": CRAFT,
            "cost": cost,
            "mode": "scratch",
            "radiant": radiant,
            "prompt": "Code a card from scratch",
        })))],
        None => vec![],
    });
    let mint: Hook = hook(move |ctx| match chosen_recipe(ctx) {
        Some(recipe) => vec![
            craft_card(json_as(json!({
                "recipe": recipe,
                "radiant": radiant,
            }))),
            exile(json_as(json!({ "target": { "of": "self" } }))),
        ],
        None => vec![],
    });
    Script {
        cry: Some(hook(|_ctx| {
            vec![choose_number(json_as(json!({
                "step": COST,
                "from": 0,
                "to": CRAFT_MAX_COST,
                "prompt": "Choose the coded card's Cost",
            })))]
        })),
        resume: IndexMap::from([(COST, open_canvas), (CRAFT, mint)]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scratch_pad(false),
        radiant: scratch_pad(true),
    }
}

// Meditative+ #26 Scratch Pad — SPEC §8.9 row 26, R178, R880, R881, R882, R1182002.
//
// BUILD M11 row M+ 26: "opens M 17's cost and craft prompts with the from-scratch flag set (the
// client's empty canvas); a valid recipe adds its card, a refused one leaves the prompt open; the
// AI and a timeout answer with a preset (R881); Scratch Pad goes to exile, not the graveyard;
// radiant the card is Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::replay::hash_state;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::subsystems::craft::{compile_crafted_def, craft_id};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // 4/4

    /// A 5/6 Unit at cost 2: 10 of 12 points.
    fn unit_recipe() -> Value {
        json!({
            "cost": 2, "type": "Unit", "adjective": "Lazy", "noun": "Iterator",
            "attack": 5, "health": 6, "keywords": [], "echo": 0, "hats": [],
        })
    }

    /// Scratch Pad played, its cost prompt answered with `cost`: the canvas open.
    fn coding(cost: i32, radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10, "library": [VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        s.play(ID, json!({}));
        s.answer(json!([cost.to_string()]));
        s
    }

    /// The crafted card in p1's hand.
    fn coded(state: &GameState) -> CardInstance {
        state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:"))
            .expect("the coded card")
            .clone()
    }

    /// The open craft prompt's presets.
    fn presets(s: &Scenario) -> Vec<CraftRecipe> {
        s.state()
            .pending
            .clone()
            .expect("the craft prompt is open")
            .options
            .iter()
            .map(|option| match &option.selection {
                Selection::Craft { recipe } => recipe.clone(),
                other => panic!("a preset is a recipe, not {other:?}"),
            })
            .collect()
    }

    #[test]
    fn declares_no_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r1182002_opens_m17s_cost_prompt_then_the_craft_prompt_with_the_from_scratch_flag() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [ID], "mana": 10 },
                "p2": { "hand": [VANILLA] },
            }));
            s.play(ID, json!({}));
            let cost = s.state().pending.clone().expect("the cost prompt is open");
            assert_eq!(cost.kind, PromptKind::Number);
            let keys: Vec<&str> = cost.options.iter().map(|option| option.key.as_str()).collect();
            assert_eq!(keys, vec!["mode:0", "mode:1", "mode:2", "mode:3", "mode:4"]);
            s.answer(json!(["2"]));
            let pending = s.state().pending.clone().expect("the craft prompt is open");
            assert_eq!(pending.kind, PromptKind::Craft);
            assert_eq!(pending.budget, Some(2));
            assert_eq!(pending.craft_mode, Some(CraftMode::Scratch));
            // The engine still rolls Meditative #17's four presets, which the client does not show.
            let types: Vec<CardType> = presets(&s).iter().map(|recipe| recipe.type_).collect();
            assert_eq!(
                types,
                vec![CardType::Unit, CardType::Spell, CardType::FieldSpell, CardType::Trap]
            );
            match s.view(P1).pending {
                Some(PendingView::ForYou(view)) => assert_eq!(view.craft_mode, Some(CraftMode::Scratch)),
                other => panic!("p1's own prompt, not {other:?}"),
            }
        }

        #[test]
        fn r1182002_a_valid_recipe_adds_its_card_and_scratch_pad_goes_to_exile() {
            let mut s = coding(2, false);
            s.answer(json!([{ "pick": "craft", "recipe": unit_recipe() }]));
            assert!(s.state().pending.is_none());
            let card = coded(s.state());
            assert!(!card.radiant, "the base face codes it plain");
            // The card M #17 would make of the same recipe: the canvas is all that differs.
            let recipe: CraftRecipe = json_as(unit_recipe());
            assert_eq!(card.def_id, craft_id(&recipe));
            assert_eq!(
                s.state().transient_defs.get(&card.def_id),
                Some(&compile_crafted_def(&recipe))
            );
            s.expect_in_zone(ID, "exile");
            assert!(s.state().players.p1.graveyard.iter().all(|card| card.def_id != ID));
        }

        #[test]
        fn r1182002_a_refused_recipe_leaves_the_prompt_open_and_the_state_unchanged() {
            let mut s = coding(2, false);
            let before = hash_state(s.state());
            let mut over = unit_recipe();
            over["attack"] = json!(10);
            over["health"] = json!(10);
            s.expect_refused_with(
                |s| s.answer(json!([{ "pick": "craft", "recipe": over }])),
                "over the budget of 12",
            );
            let potion = json!({
                "cost": 1, "type": "Unit", "adjective": "Pure", "noun": "Closure",
                "attack": 1, "health": 1, "keywords": [], "echo": 0,
                "hats": [{ "hat": "activate", "effects": [{ "verb": "gainArmor", "n": 1 }] }],
                "frame": "potion",
            });
            s.expect_refused_with(
                |s| s.answer(json!([{ "pick": "craft", "recipe": potion }])),
                "this prompt crafts no Potion",
            );
            assert_eq!(
                s.state().pending.as_ref().map(|pending| pending.kind),
                Some(PromptKind::Craft),
                "the prompt stays open"
            );
            assert_eq!(hash_state(s.state()), before, "the state is unchanged");
        }

        #[test]
        fn r881_the_policy_and_a_timeout_answer_with_a_preset() {
            let s = coding(3, false);
            let offered = presets(&s);
            let mut rng = Rng::new("scratch-policy", 0);
            match choose_action(s.state(), P1, &mut rng).expect("the policy answers") {
                ActionBody::Answer { selection, .. } => match &selection[..] {
                    [Selection::Craft { recipe }] => assert!(offered.contains(recipe)),
                    other => panic!("one preset, not {other:?}"),
                },
                other => panic!("an answer, not {other:?}"),
            }
            let timed = reduce(
                s.state(),
                &json_as::<Action>(json!({ "type": "timeout", "playerId": "p1", "nonce": "scratch-t1" })),
            );
            assert_eq!(timed.error, None);
            let card = coded(&timed.state);
            let def = timed.state.transient_defs.get(&card.def_id).expect("its definition");
            assert!(offered.contains(def.craft.as_ref().expect("its recipe")));
            assert!(timed.state.players.p1.exile.iter().any(|card| card.def_id == ID));
        }

        #[test]
        fn r97_the_opponent_sees_neither_the_canvas_nor_the_recipe() {
            let s = coding(1, false);
            let theirs = serde_json::to_string(&s.view(P2)).expect("the view serializes");
            assert!(!theirs.contains("recipe") && !theirs.contains("craftMode"), "{theirs}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1182002_the_coded_card_is_radiant() {
            let mut s = coding(2, true);
            assert_eq!(
                s.state().pending.as_ref().and_then(|pending| pending.craft_mode),
                Some(CraftMode::Scratch)
            );
            s.answer(json!([{ "pick": "craft", "recipe": unit_recipe() }]));
            let card = coded(s.state());
            assert!(card.radiant, "the Radiant face codes it Radiant");
            let def = s.state().transient_defs.get(&card.def_id).expect("its definition").clone();
            assert_eq!([def.radiant.attack, def.radiant.health], [Some(10), Some(12)]);
            s.expect_in_zone(ID, "exile");
        }
    }
}
