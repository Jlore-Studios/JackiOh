//! Meditative #17 True Craft a Card (SPEC §8.8, ME-CRAFT, R880–R883). Spell, cost 0, Mythic.
//!   Base:    "Craft a card and add it to your hand."
//!   Radiant: "Craft a Radiant card and add it to your hand."
//!
//! THE CHAIN (§10.6). The Spell resolves and opens a `number` prompt (Classic #18's kind) for the
//! crafted card's cost, 0 to `CRAFT_MAX_COST` (4); its answer re-enters `resume.cost`, which opens
//! the `craft` prompt at that budget; its answer re-enters `resume.craft`, which compiles the
//! recipe and adds a fresh card of it to the caster's hand. The picks travel in `ctx.targets`,
//! read with `chosen_number` and `chosen_recipe` — never `ctx.targets` by hand.
//!
//! THE PROMPT (R880, `subsystems/craft.rs`). The `craft` prompt's options are the four seeded
//! presets (a Unit, a Spell, a Field Spell and a Trap); its answer is one `Selection::Craft`, a
//! preset or the player's own recipe, which the reducer validates against the block table, the
//! point budget (`2 + 5 × cost`), the line budget (24) and the type's rules. An invalid recipe is
//! refused with the state unchanged and the prompt still open. A valid one is compiled into a
//! transient definition whose id names the recipe (R882), of the Meditative set, not a token,
//! Mythic, its `loc` the counted lines, every number a param — and the card reaches the hand at
//! its cost, Radiant on the Radiant face, or burns on a full one (§2.4).
//!
//! THE FACES differ only in `radiant`: the Radiant face crafts the card Radiant, whose face
//! doubles every number and stat (`CRAFT_RADIANT_MULTIPLIER` 2, R275 by construction).
//!
//! THE CLOCK runs while crafting, as for Classic+ #42 KY's Test (R420, R79): no prompt of its own.
//! The opponent sees only that a prompt is open, then a hidden card (R97).
//!
//! Voice audio is owed: the `card-audio.json5` entry is written, and a person with macOS renders
//! it (`docs/ADDING_CARDS.md` §7).
//!
//! `CRAFT_MAX_COST` lives in `config.rs` (CLAUDE.md rule 9).

use jackioh_engine::effects::{choose_craft, choose_number, chosen_number, chosen_recipe, craft_card};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-017";

/// The `data`-less resume steps: the cost answer opens the editor, the craft answer mints.
const COST: &str = "cost";
const CRAFT: &str = "craft";

/// The two faces differ only in `radiant`, as Core #99's differ only in their Discover count.
fn true_craft_a_card(radiant: bool) -> Script {
    let open_editor: Hook = hook(move |ctx| match chosen_number(ctx) {
        Some(cost) => vec![choose_craft(json_as(json!({
            "step": CRAFT,
            "cost": cost,
            "radiant": radiant,
        })))],
        None => vec![],
    });
    let mint: Hook = hook(move |ctx| match chosen_recipe(ctx) {
        Some(recipe) => vec![craft_card(json_as(json!({
            "recipe": recipe,
            "radiant": radiant,
        })))],
        None => vec![],
    });
    Script {
        cry: Some(hook(|_ctx| {
            vec![choose_number(json_as(json!({
                "step": COST,
                "from": 0,
                "to": CRAFT_MAX_COST,
                "prompt": "Choose the crafted card's Cost",
            })))]
        })),
        resume: IndexMap::from([(COST, open_editor), (CRAFT, mint)]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: true_craft_a_card(false),
        radiant: true_craft_a_card(true),
    }
}

// Meditative #17 True Craft a Card — SPEC §8.8, ME-CRAFT, §10.6, R79, R97, R420, R739, R880–R883.
//
// BUILD M10 row M 17: "Opens a `number` prompt for the cost (0 to 4), then a `craft` prompt whose
// 4 presets (a Unit, a Spell, a Field Spell, a Trap) are rolled from the match rng within the
// budget (MD-A30); a recipe over the points (2 + 5 × the cost), the lines of code (24) or 8
// effects, or one breaking its type's rules (a Spell with no effect, a Trap without its reveal
// hat), is refused with the state unchanged and the prompt open; a valid one adds a fresh card of
// a transient definition whose id names the recipe and whose script rebuilds after a JSON round
// trip and a replay (R179, R468), of the Meditative set, not a token, Mythic, its `loc` the
// counted lines, every number a param that a Nerf or Buff moves (MD-A32); the hand cap burns it;
// the AI and a timeout answer only with presets (MD-A31); the opponent sees only that a prompt is
// open, then a hidden card (R97); the editor's preview reports the same validity as the reducer;
// radiant the card is Radiant, its face doubling every number and stat".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// Nine simple cards, enough to fill a hand to the cap next to the card being played.
    const NINE_FILLERS: [&str; 9] = [
        "core-001", "core-002", "core-003", "core-004", "core-008", "core-011", "core-012",
        "core-015", "core-020",
    ];

    /// A minimal valid recipe at the given cost: a Unit with all the budget in stats.
    fn unit_recipe(cost: i32, attack: i32, health: i32) -> Value {
        json!({
            "cost": cost,
            "type": "Unit",
            "adjective": "Pure",
            "noun": "Closure",
            "attack": attack,
            "health": health,
            "keywords": [],
            "echo": 0,
            "hats": [],
        })
    }

    /// The transient definition of a hand card, read off the state (no catalog holds it).
    fn transient_of(s: &Scenario, card: &CardInstance) -> CardDef {
        s.state()
            .transient_defs
            .get(&card.def_id)
            .unwrap_or_else(|| panic!("no transient def for {}", card.def_id))
            .clone()
    }

    /// Play the card and answer the cost prompt with `cost`.
    fn crafted_at(cost: i32) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ID], "mana": 10 },
            "p2": { "hand": ["core-053"] },
        }));
        s.play(ID, json!({}));
        s.answer(json!([cost.to_string()]));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn r880_asks_a_cost_then_offers_four_presets() {
            let s = crafted_at(2);
            let pending = s.state().pending.clone().expect("the craft prompt is open");
            assert_eq!(pending.kind, PromptKind::Craft);
            assert_eq!(pending.options.len(), 4);
            assert_eq!(pending.budget, Some(2));
            let types: Vec<String> = pending
                .options
                .iter()
                .map(|option| match &option.selection {
                    Selection::Craft { recipe } => recipe.type_.as_str().to_string(),
                    other => panic!("a preset is a recipe, not {other:?}"),
                })
                .collect();
            assert_eq!(types, vec!["Unit", "Spell", "Field Spell", "Trap"]);
        }

        #[test]
        fn r880_refuses_an_over_budget_recipe() {
            let mut s = crafted_at(0);
            let before = jackioh_engine::replay::hash_state(s.state());
            // A 10/10 Unit costs 19 points, over the budget of 2.
            s.expect_refused_with(
                |s| s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(0, 10, 10) }])),
                "cannot be crafted",
            );
            assert!(s.state().pending.is_some(), "the prompt stays open");
            assert_eq!(
                jackioh_engine::replay::hash_state(s.state()),
                before,
                "the state is unchanged"
            );
        }

        #[test]
        fn r882_adds_a_crafted_card_to_hand() {
            let mut s = crafted_at(1);
            // A 3/4 Unit costs 6 points, within the budget of 7.
            s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(1, 3, 4) }]));
            assert!(s.state().pending.is_none());
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 1, "the crafted card is the hand: {hand:?}");
            assert!(
                hand[0].def_id.starts_with("craft:"),
                "a digest id: {}",
                hand[0].def_id
            );
            let def = transient_of(&s, &hand[0]);
            assert_eq!(def.set, SetName::Meditative);
            assert_eq!(def.rarity, Rarity::Mythic);
            assert!(!def.token);
            assert!(!hand[0].radiant, "the base face crafts it plain");
        }

        #[test]
        fn r882_a_crafted_spell_deals_its_damage() {
            let mut s = crafted_at(1);
            // N=5 at 1 point each: 5 points, within the budget of 7.
            s.answer(json!([{
                "pick": "craft",
                "recipe": {
                    "cost": 1, "type": "Spell", "adjective": "Pure", "noun": "Closure",
                    "attack": 0, "health": 1, "keywords": [], "echo": 0,
                    "hats": [{ "hat": "whenCast",
                        "effects": [{ "verb": "damageEnemyHero", "n": 5 }] }],
                },
            }]));
            let id = s.hand(P1)[0].id.clone();
            s.play(id, json!({}));
            s.expect_health(P2, 25);
        }

        #[test]
        fn r882_full_hand_burns_it() {
            crate::register_all();
            let mut hand = vec![json!(ID)];
            hand.extend(NINE_FILLERS.iter().map(|id| json!(id)));
            let mut s = scenario(json!({
                "p1": { "hand": hand, "mana": 10 },
                "p2": { "hand": ["core-053"] },
            }));
            s.play(ID, json!({}));
            s.answer(json!(["1"]));
            // Playing the card leaves 9 in hand; the tenth filler fills it before the mint.
            let filler =
                new_instance(s.state_mut(), "core-001", P1, Zone::Hand { player: P1 });
            s.state_mut().players.p1.hand.push(filler);
            s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(1, 3, 4) }]));
            let held = s.hand(P1);
            assert_eq!(held.len(), 10, "a full hand burns the crafted card: {held:?}");
            assert!(
                held.iter().all(|card| !card.def_id.starts_with("craft:")),
                "nothing crafted stays: {held:?}"
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r882_the_crafted_card_is_radiant_and_doubles_every_number_and_stat() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }], "mana": 10 },
                "p2": { "hand": ["core-053"] },
            }));
            s.play(ID, json!({}));
            s.answer(json!(["2"]));
            // A 4/5 Unit with Armor 2 and a Cry dealing N=3: 4+5-1+4+3 = 15 points, over 12 —
            // so a 3/4 with Armor 2 and N=3: 3+4-1+4+3 = 13, still over — a 3/4 with Armor 1
            // and N=3: 3+4-1+2+3 = 11, within 12.
            s.answer(json!([{ "pick": "craft", "recipe": {
                "cost": 2, "type": "Unit", "adjective": "Pure", "noun": "Closure",
                "attack": 3, "health": 4,
                "keywords": [{ "kind": "Armor", "n": 1 }], "echo": 0,
                "hats": [{ "hat": "cry", "effects": [{ "verb": "damageEnemyHero", "n": 3 }] }],
            }}]));
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 1);
            assert!(hand[0].radiant, "the Radiant face crafts it Radiant");
            let def = transient_of(&s, &hand[0]);
            assert_eq!([def.base.attack, def.base.health], [Some(3), Some(4)]);
            assert_eq!([def.radiant.attack, def.radiant.health], [Some(6), Some(8)]);
            let params = def.params.as_ref().expect("a declared number");
            assert_eq!(params.len(), 1);
            assert_eq!([params[0].base, params[0].radiant], [3, 6]);
        }

        #[test]
        fn r882_a_stats_only_unit_gains_the_cheapest_keyword() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": ID, "radiant": true }], "mana": 10 },
                "p2": { "hand": ["core-053"] },
            }));
            s.play(ID, json!({}));
            s.answer(json!(["1"]));
            s.answer(json!([{ "pick": "craft", "recipe": unit_recipe(1, 3, 4) }]));
            let def = transient_of(&s, &s.hand(P1)[0]);
            assert!(def.base.keywords.is_empty());
            assert_eq!(
                def.radiant.keywords,
                vec![Keyword::Deft],
                "the cheapest table keyword"
            );
        }
    }
}
