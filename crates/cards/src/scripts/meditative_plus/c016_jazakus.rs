//! Meditative+ #16 Jazakus (SPEC §8.9 row 16, ME-CRAFT's potion mode, R880–R883, R1182000, R1182001,
//! R1563007, R1563008).
//! (1) Unit, Human, Catalyst, Epic, 2/2 → 4/4.
//!   Base:    "Cry: Craft a custom Potion. It costs (1).
//!             Death: Shuffle a Jazakus Prime into your deck."
//!   Radiant: "Cry: Craft a custom Radiant Potion. It costs (1).
//!             Death: Shuffle a Radiant Jazakus Prime into your deck."
//!
//! THE CRY (R1182000, R1563008). Meditative #17's `craft` prompt in its potion mode, with no cost
//! prompt: the frame is fixed, a Spell tagged Potion with no stats whose one hat is When cast, with
//! the effects the player snaps under it, at cost (1) (`CRAFT_POTION_COST`), `CRAFT_POTION_POINTS`
//! points and 24 lines. The blocks are the prompt's offer (R1563007): the keywords a Spell takes, its
//! When cast, and three verbs that fit the points, drawn as the prompt opens. A valid recipe compiles
//! into a transient definition named by its digest (R882), of the Meditative+ set, tagged Potion, and
//! a fresh card of it reaches the hand at (1), Radiant on the Radiant face, or burns on a full one
//! (§2.4). The prompt's options are seeded Potions of the offer, so the AI, the random policy and a
//! timeout answer with one (R881). The picks travel in `ctx.targets`, read with `chosen_recipe`.
//!
//! THE DEATH (Catalyst, C+ #38's shape, R1421): a Jazakus Prime (M+ #16.1) shuffled into its owner's
//! deck, Radiant on the Radiant face.
//!
//! Voice audio is owed with the patch that ships the set (R1420): the `card-audio.json5` entry is
//! written, and the release renders it (`[gen:voice]`, R1448).

use jackioh_engine::effects::{choose_craft, chosen_recipe, craft_card, shuffle_into};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditativeplus-016";
pub const PRIME: &str = "meditativeplus-016-1";

/// The `data`-less resume step the craft answer re-enters.
const CRAFT: &str = "craft";

/// The two faces differ only in `radiant`: the potion and the Prime are Radiant on the Radiant face.
fn jazakus(radiant: bool) -> Script {
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
                "mode": "potion",
                "radiant": radiant,
                "prompt": "Craft a custom Potion",
            })))]
        })),
        resume: IndexMap::from([(CRAFT, mint)]),
        death: Some(hook(move |_ctx| {
            vec![shuffle_into(json_as(json!({
                "defId": PRIME,
                "count": 1,
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: jazakus(false),
        radiant: jazakus(true),
    }
}

// Meditative+ #16 Jazakus — SPEC §8.9 row 16, R79, R97, R881, R882, R1182000, R1421, R1563007, R1563008.
//
// BUILD M11 row M+ 16: "Cry (played): opens the crafter in potion mode, its frame a Spell tagged
// Potion with one When cast and cost (1), its blocks the prompt's offer; a recipe over the points or
// the lines, with another hook or a block the offer lacks, is refused with the prompt open; a valid
// one adds a transient Potion whose script survives a JSON round trip and a replay (R882); the AI,
// the random policy and a timeout answer with a preset (R881); Death shuffles a base Jazakus Prime into
// your deck; radiant 4/4, the potion Radiant and the Prime Radiant".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::replay::hash_state;
    use jackioh_engine::subsystems::ai_policy::choose_action;
    use jackioh_engine::subsystems::craft::whole_palette;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const HIT_JOB: &str = "core-016"; // Spell: destroy a target Unit.
    const VANILLA: &str = "core-008"; // 4/4

    /// A Potion that deals 4 to the enemy hero: 4 points, the whole budget.
    fn potion_recipe() -> Value {
        json!({
            "cost": 1, "type": "Spell", "adjective": "Pure", "noun": "Closure",
            "attack": 0, "health": 1, "keywords": [], "echo": 0,
            "hats": [{ "hat": "whenCast", "effects": [{ "verb": "damageEnemyHero", "n": 4 }] }],
            "frame": "potion",
        })
    }

    /// Jazakus played into lane 1, its craft prompt open.
    fn crafting(radiant: bool) -> Scenario {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "mana": 10, "library": [VANILLA] },
            "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA] },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    /// `crafting`, its prompt's offer widened to every block, for the tests of the frame's rules,
    /// which the offer (R1563007) only narrows.
    fn any_block(radiant: bool) -> Scenario {
        let mut s = crafting(radiant);
        if let Some(pending) = s.state_mut().pending.as_mut() {
            pending.craft_offer = Some(whole_palette());
        }
        s
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

    /// The crafted card in p1's hand.
    fn crafted(state: &GameState) -> CardInstance {
        state
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id.starts_with("craft:"))
            .expect("the crafted card")
            .clone()
    }

    /// Refused with `reason`, the state unchanged and the prompt still open.
    fn refused(recipe: Value, reason: &str) {
        let mut s = any_block(false);
        let before = hash_state(s.state());
        s.expect_refused_with(|s| s.answer(json!([{ "pick": "craft", "recipe": recipe }])), reason);
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.kind),
            Some(PromptKind::Craft),
            "the prompt stays open"
        );
        assert_eq!(hash_state(s.state()), before, "the state is unchanged");
    }

    /// `body` as p1's action.
    fn by_p1(body: Value) -> Action {
        json_as(body)
    }

    /// Jazakus in lane 2 destroyed by Hit Job: the Primes its Death put in p1's deck.
    fn primes_after_death(radiant: bool) -> Vec<CardInstance> {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "mana": 8,
                "hand": [HIT_JOB],
                "field": [{ "def": ID, "radiant": radiant, "lane": 2 }],
                "library": [VANILLA],
            },
            "p2": { "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }] },
        }));
        let jazakus = s.unit(P1, 2).expect("Jazakus");
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": jazakus.id }] }),
        );
        s.expect_in_zone(&jazakus, "graveyard");
        s.pile(P1, "library")
            .into_iter()
            .filter(|card| card.def_id == PRIME)
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
        fn r1182000_r1563007_r1563008_the_cry_opens_the_crafter_in_potion_mode_at_cost_1() {
            let s = crafting(false);
            let pending = s.state().pending.clone().expect("the craft prompt is open");
            assert_eq!(pending.kind, PromptKind::Craft);
            assert_eq!(pending.player_id, P1);
            assert_eq!(pending.budget, Some(CRAFT_POTION_COST));
            assert_eq!(pending.craft_mode, Some(CraftMode::Potion));
            let offer = pending.craft_offer.clone().expect("a potion prompt rolls an offer");
            assert_eq!(offer.hats, vec![CraftHatKind::WhenCast]);
            assert_eq!(offer.keywords.len(), CRAFT_OFFER_KEYWORDS);
            assert!(!offer.verbs.is_empty() && offer.verbs.len() <= CRAFT_OFFER_VERBS);
            let offered = presets(&s);
            assert!(!offered.is_empty() && offered.len() <= offer.verbs.len());
            for recipe in &offered {
                assert_eq!(recipe.frame, Some(CraftFrame::Potion));
                assert_eq!(recipe.type_, CardType::Spell);
                assert_eq!(recipe.cost, 1);
                assert_eq!((recipe.attack, recipe.health), (0, 1), "a Potion has no stats");
                assert_eq!(recipe.hats.len(), 1);
                assert_eq!(recipe.hats[0].hat, CraftHatKind::WhenCast);
                assert!(recipe.hats[0].effects.iter().all(|effect| offer.verbs.contains(&effect.verb)));
            }
        }

        #[test]
        fn r1182000_r1563008_a_valid_recipe_adds_a_meditative_plus_potion_spell_to_the_hand() {
            let mut s = any_block(false);
            s.answer(json!([{ "pick": "craft", "recipe": potion_recipe() }]));
            assert!(s.state().pending.is_none());
            let card = crafted(s.state());
            assert!(!card.radiant, "the base face crafts it plain");
            let def = s.state().transient_defs.get(&card.def_id).expect("its definition").clone();
            assert_eq!(def.set, SetName::MeditativePlus);
            assert_eq!(def.tags, vec![Tag::Potion]);
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.name, "Pure Closure Potion");
            assert!(!def.token);
            assert_eq!(def.base.text, "Deal {b1} damage to the enemy hero.");
        }

        #[test]
        fn r1563008_the_potion_is_cast_for_its_effects_then_goes_to_the_graveyard() {
            let mut s = any_block(false);
            s.answer(json!([{ "pick": "craft", "recipe": potion_recipe() }]));
            let potion = crafted(s.state());
            s.play(&potion.id, json!({}));
            s.expect_health(P2, 26);
            s.expect_in_zone(&potion.id, "graveyard");
        }

        #[test]
        fn r1182000_r1563007_a_recipe_over_the_points_or_the_lines_with_another_hook_or_unoffered_is_refused() {
            // Dealing 5: 5 points, over the budget of 4.
            let mut over_points = potion_recipe();
            over_points["hats"][0]["effects"][0]["n"] = json!(5);
            refused(over_points, "over the budget of 4");
            // Nine aimed blocks: 3 + 2 + 9 × 2 = 23 lines, and over 8 effects.
            let aimed = json!({ "verb": "damageTarget", "n": 1 });
            let mut over_effects = potion_recipe();
            over_effects["hats"][0]["effects"] = json!([aimed, aimed, aimed, aimed, aimed, aimed, aimed, aimed, aimed]);
            refused(over_effects, "more than 8");
            // A Cry instead of the When cast, or a Unit with stats.
            let mut cry = potion_recipe();
            cry["hats"][0]["hat"] = json!("cry");
            refused(cry, "exactly one hat, its When cast");
            let mut unit = potion_recipe();
            unit["type"] = json!("Unit");
            unit["attack"] = json!(1);
            refused(unit, "a Potion is a Spell");
            // Meditative #17's recipe, outside the frame.
            let mut free = potion_recipe();
            free.as_object_mut().expect("an object").remove("frame");
            refused(free, "this prompt crafts a Potion");
            // A verb the prompt's own offer lacks (R1563007).
            let mut s = crafting(false);
            let offer = s.state().pending.clone().and_then(|pending| pending.craft_offer).expect("an offer");
            let missing = CRAFT_VERB_PRICES
                .iter()
                .map(|row| row.verb)
                .find(|verb| !offer.verbs.contains(verb) && *verb != CraftVerb::GrantKeyword)
                .expect("three verbs of the table leave some out");
            let mut unoffered = potion_recipe();
            unoffered["hats"][0]["effects"] = json!([{ "verb": missing, "n": 1 }]);
            s.expect_refused_with(
                |s| s.answer(json!([{ "pick": "craft", "recipe": unoffered }])),
                "was not offered",
            );
        }

        #[test]
        fn r882_the_potion_survives_a_json_round_trip_and_a_replay() {
            let mut s = any_block(false);
            s.answer(json!([{ "pick": "craft", "recipe": potion_recipe() }]));
            let potion = crafted(s.state());
            let tripped: GameState = serde_json::from_value(crate::js(s.state())).expect("the state round-trips");
            let cast = by_p1(json!({
                "type": "play", "playerId": "p1", "instanceId": potion.id, "nonce": "jazakus-replay",
            }));
            let live = reduce(s.state(), &cast);
            let after_trip = reduce(&tripped, &cast);
            assert_eq!(live.error, None);
            assert_eq!(after_trip.error, None);
            assert_eq!(after_trip.state.players.p2.hero.health, 26, "the rebuilt When cast deals its 4");
            assert_eq!(hash_state(&after_trip.state), hash_state(&live.state), "it replays exactly");
        }

        #[test]
        fn r881_the_policy_and_a_timeout_answer_with_a_potion_preset() {
            let s = crafting(false);
            let offered = presets(&s);
            let mut rng = Rng::new("jazakus-policy", 0);
            match choose_action(s.state(), P1, &mut rng).expect("the policy answers") {
                ActionBody::Answer { selection, .. } => match &selection[..] {
                    [Selection::Craft { recipe }] => assert!(offered.contains(recipe)),
                    other => panic!("one preset, not {other:?}"),
                },
                other => panic!("an answer, not {other:?}"),
            }
            let timed = reduce(
                s.state(),
                &by_p1(json!({ "type": "timeout", "playerId": "p1", "nonce": "jazakus-t1" })),
            );
            assert_eq!(timed.error, None);
            let card = crafted(&timed.state);
            let def = timed.state.transient_defs.get(&card.def_id).expect("its definition");
            assert!(offered.contains(def.craft.as_ref().expect("its recipe")));
            assert_eq!(def.tags, vec![Tag::Potion]);
        }

        #[test]
        fn r97_the_opponent_sees_only_an_open_prompt_then_a_hidden_card() {
            let mut s = any_block(false);
            let open = serde_json::to_string(&s.view(P2)).expect("the view serializes");
            assert!(
                !open.contains("recipe") && !open.contains("craftMode") && !open.contains("craftOffer"),
                "{open}"
            );
            s.answer(json!([{ "pick": "craft", "recipe": potion_recipe() }]));
            let view = s.view(P2);
            assert!(
                view.defs.as_ref().is_none_or(|defs| defs.keys().all(|id| !id.starts_with("craft:"))),
                "no crafted def leaks"
            );
        }

        #[test]
        fn death_shuffles_a_jazakus_prime_into_your_deck() {
            let primes = primes_after_death(false);
            assert_eq!(primes.len(), 1);
            assert!(!primes[0].radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1182000_r1563008_the_potion_is_radiant_doubling_its_numbers() {
            let mut s = any_block(true);
            s.answer(json!([{ "pick": "craft", "recipe": potion_recipe() }]));
            let potion = crafted(s.state());
            assert!(potion.radiant, "the Radiant face crafts it Radiant");
            s.play(&potion.id, json!({}));
            s.expect_health(P2, 22);
            s.expect_in_zone(&potion.id, "graveyard");
        }

        #[test]
        fn death_shuffles_a_radiant_jazakus_prime_into_your_deck() {
            let primes = primes_after_death(true);
            assert_eq!(primes.len(), 1);
            assert!(primes[0].radiant);
        }
    }
}
