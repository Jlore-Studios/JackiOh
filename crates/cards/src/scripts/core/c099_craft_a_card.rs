//! #99 Craft a Card (SPEC §8.5, §6.3 Fuse and Discover, §10.6, R77). Spell, Mythic.
//!   Base:    "Discover a Unit, then Discover another; Fuse them; the result costs 0 and goes to
//!             your hand"
//!   Radiant: the same with a third Discover (§8's cell "Three Discovers", R275). Neither face draws.
//!   Engine:  "Fuse per 6.3 creates a transient definition stored in match state".
//!
//! THE CHAIN (§10.6). Each Discover is a `PendingChoice` whose `resume` names the next step in the
//! card's own `resume` table (`prompts::hook_for`): no callback or closure in state (§9.3). The picks
//! ride in `data`, the only place a chained effect keeps anything; `resume_self` merges it onward.
//! FUSE (R77, `subsystems/fuse.rs`): no `target`, `toHand` the caster, both faces fused into
//! `state.transientDefs`, cost overridden to 0 on the instance (`CRAFTED_CARD_COST`), not the def.
//! The ingredients are Discovered definitions never on a board, so nothing a player could see ceases
//! to exist.

use jackioh_engine::effects::{chosen_options, discover_from_catalog, fuse_cards};
use jackioh_engine::prelude::*;

pub const ID: &str = "core-099";

/// One resume step per Discover, named for the Discover whose answer it receives (§10.6).
const FIRST: &str = "first";
const SECOND: &str = "second";
const THIRD: &str = "third";

/// The `data` key the picks travel in; `data` is the only place a chained step may keep anything.
const PICKS: &str = "picks";

/// §6.3 Discover: 1 of 3 Units, drawn without replacement and shown only to the chooser. A plain
/// `type` filter drops tokens (§5.1), and each Discover is an independent draw over the same pool,
/// so a Unit can be offered twice; §8.5 and R77 don't forbid it, and R60 lets generated cards repeat.
fn discover_unit(step: &'static str, picks: &[String]) -> Effect {
    discover_from_catalog(json_as(json!({
        "step": step,
        "query": { "type": "Unit" },
        "prompt": "Discover a Unit",
        "data": { PICKS: picks },
    })))
}

/// The ids picked so far, read back out of the captured data.
fn picks_of(ctx: &EffectContext<'_>) -> Vec<String> {
    let Some(Value::Array(stored)) = ctx.data.get(PICKS) else {
        return vec![];
    };
    stored
        .iter()
        .filter_map(|entry| entry.as_str().map(str::to_string))
        .collect()
}

/// Those ids plus the one this step was answered with.
fn with_answer(ctx: &EffectContext<'_>) -> Vec<String> {
    let picked = chosen_options(ctx).into_iter().next();
    let mut picks = picks_of(ctx);
    match picked {
        None => picks,
        Some(picked) => {
            picks.push(picked);
            picks
        }
    }
}

/// R77: two or three definitions, no target on the field, and a fresh non-Radiant hand card with
/// `costOverride` 0. Fewer than `FUSE_MIN_INGREDIENTS` is not a fusion — "Fewer is not a fusion" —
/// so a chain that lost an answer fizzles and the spell still counts as played (§8 Conventions).
fn craft(picks: &[String]) -> Vec<Effect> {
    if picks.len() < subsystems::FUSE_MIN_INGREDIENTS {
        return vec![];
    }
    // No target: R77 takes the ingredients' shared type ("Unit") and "your hand" is the caster's
    // (§8.5). `fuse_cards` wraps `subsystems/fuse.rs`, taking catalog ids (`defIds`) where `fuse`
    // takes instances and a sink, so no state is mutated here (CLAUDE.md rule 5); each pick is an
    // ingredient in no pile (`{ z: "gone" }`, R86) and R102 composes the result.
    vec![fuse_cards(json_as(json!({ "defIds": picks, "toHand": "self" })))]
}

/// `discovers` is the whole of the difference between the two faces (§8.5's radiant cell): 2 or 3.
fn craft_a_card(discovers: i32) -> Script {
    let open_second: Hook = hook(|ctx| vec![discover_unit(SECOND, &with_answer(ctx))]);
    let open_third: Hook = hook(|ctx| vec![discover_unit(THIRD, &with_answer(ctx))]);
    let fuse_them: Hook = hook(|ctx| craft(&with_answer(ctx)));

    let mut resume: IndexMap<&'static str, Hook> = IndexMap::new();
    resume.insert(FIRST, open_second);
    // The last Discover's answer fuses; the radiant face has one more before that.
    resume.insert(SECOND, if discovers == 3 { open_third } else { fuse_them.clone() });
    if discovers == 3 {
        resume.insert(THIRD, fuse_them);
    }

    Script {
        cry: Some(hook(|_ctx| vec![discover_unit(FIRST, &[])])),
        resume,
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: craft_a_card(2),
        radiant: craft_a_card(3),
    }
}

// #99 Craft a Card — SPEC §8.5, §6.3 (Fuse, Discover), §10.5, §10.6, R4, R23, R60, R65, R77, R86,
// R102, R113; BUILD M4-T4 row 99. R102's "the whole verb does nothing at all" guards are
// unreachable from a #99 play, so they are asserted against `subsystems::fuse`, where that rule
// lives. `FUSE_COST_CAP` and `HAND_CAP` are R77's and R4's numbers, in `config.rs`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const CRAFT: &str = "core-099"; // Spell, 3, Mythic
    /// #53 Reno, a 3-cost Unit: the spare card that keeps §2.5's auto-end off the assertions.
    const SPARE: &str = "core-053";
    /// #19 Midrange Menace: base 9/9 Taunt, radiant 18/18 Taunt + Immutable — R23's target.
    const MENACE: &str = "core-019";
    /// #11 Tempo Timmy, a 1-cost Unit, for a plain on-field ingredient.
    const TIMMY: &str = "core-011";
    /// #26 Glowy Jelly Bean: "Choose a card in your hand; it becomes Radiant" — a real Make Radiant.
    const GLOWY: &str = "core-026";

    use crate::js;

    /// The name of every member the face sets.
    fn members(script: &Script) -> Vec<&'static str> {
        let mut keys = Vec::new();
        let mut note = |present: bool, key: &'static str| {
            if present {
                keys.push(key);
            }
        };
        note(script.cost.is_some(), "cost");
        note(script.cry.is_some(), "cry");
        note(script.death.is_some(), "death");
        note(script.start_of_game.is_some(), "startOfGame");
        note(!script.resume.is_empty(), "resume");
        note(script.delayed.is_some(), "delayed");
        note(script.set_stat.is_some(), "setStat");
        note(script.start_of_turn.is_some(), "startOfTurn");
        note(script.end_of_turn.is_some(), "endOfTurn");
        note(script.aura.is_some(), "aura");
        note(!script.triggers.is_empty(), "triggers");
        note(script.on_play_hook.is_some(), "onPlayHook");
        note(!script.hand_triggers.is_empty(), "handTriggers");
        note(script.static_flags.is_some(), "staticFlags");
        note(!script.targets.is_empty(), "targets");
        note(!script.modes.is_empty(), "modes");
        note(script.condition_met.is_some(), "conditionMet");
        note(script.preview.is_some(), "preview");
        note(!script.activations.is_empty(), "activations");
        note(!script.target_checks.is_empty(), "targetChecks");
        note(script.cost_aura.is_some(), "costAura");
        note(script.graveyard_play.is_some(), "graveyardPlay");
        note(script.targeting_discards.is_some(), "targetingDiscards");
        note(script.records_play_as.is_some(), "recordsPlayAs");
        note(script.draw_limit.is_some(), "drawLimit");
        note(!script.replacements.is_empty(), "replacements");
        note(script.hero_guard.is_some(), "heroGuard");
        note(script.conditional_keywords.is_some(), "conditionalKeywords");
        note(script.after_attack.is_some(), "afterAttack");
        note(script.plague_multiplier.is_some(), "plagueMultiplier");
        note(!script.deck_triggers.is_empty(), "deckTriggers");
        note(!script.graveyard_triggers.is_empty(), "graveyardTriggers");
        note(script.quests.is_some(), "quests");
        note(script.tribute_when.is_some(), "tributeWhen");
        note(script.would_counter.is_some(), "wouldCounter");
        note(script.start_of_opponent_turn.is_some(), "startOfOpponentTurn");
        keys
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("expected {what}"))
    }

    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.events()
            .iter()
            .map(js)
            .filter(|event| event["type"] == kind)
            .collect()
    }

    fn open(s: &Scenario) -> Value {
        js(must(s.state().pending.as_ref(), "an open prompt"))
    }

    /// A Discover's options are `mode` selections carrying catalog ids (§10.6).
    fn option_ids(pending: &Value) -> Vec<String> {
        pending["options"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|option| option["selection"]["pick"] == "mode")
            .map(|option| option["selection"]["option"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// The key of the prompt's first option, the answer the tests give.
    fn first_key(pending: &Value) -> Value {
        must(pending["options"].as_array().and_then(|options| options.first()), "the first option")["key"].clone()
    }

    #[derive(Default)]
    struct CraftOpts {
        radiant_face: bool,
        seed: Option<String>,
        p1: Option<Value>,
    }

    fn craft_scenario(opts: CraftOpts) -> Scenario {
        let extra = opts.p1.unwrap_or_else(|| json!({}));
        let mut p1 = json!({ "mana": 8 });
        if let Some(fields) = extra.as_object() {
            for (key, value) in fields {
                p1[key.as_str()] = value.clone();
            }
        }
        let mut hand = vec![json!({ "def": CRAFT, "radiant": opts.radiant_face }), json!(SPARE)];
        hand.extend(extra["hand"].as_array().cloned().unwrap_or_default());
        p1["hand"] = json!(hand);
        scenario(json!({
            "seed": opts.seed.unwrap_or_else(|| "craft".to_string()),
            "p1": p1,
        }))
    }

    /// Play #99 and answer every Discover with its first option, collecting the picks in order.
    fn craft(opts: CraftOpts) -> (Scenario, Vec<String>) {
        let expected = if opts.radiant_face { 3 } else { 2 };
        let mut s = craft_scenario(opts);
        s.play(CRAFT, json!({}));
        let mut picks = Vec::new();
        for step in 0..expected {
            let pending = open(&s);
            let pick = must(option_ids(&pending).into_iter().next(), &format!("an option on Discover {}", step + 1));
            picks.push(pick);
            s.answer(first_key(&pending));
        }
        (s, picks)
    }

    /// The one transient definition a #99 resolution should have written (§10.1, R77).
    fn fused_def_of(state: &GameState) -> CardDef {
        let defs: Vec<&CardDef> = state.transient_defs.values().collect();
        assert_eq!(
            defs.len(),
            1,
            "R77: a #99 resolution fuses its Discovered Units once, into one transient definition in \
             `state.transientDefs`",
        );
        must(defs.first().map(|def| (*def).clone()), "the fused definition")
    }

    /// R77: a fused face sums its ingredients' attack and health on that same face.
    fn face_sums(picks: &[String], key: &str) -> (i64, i64) {
        picks
            .iter()
            .map(|id| js(&crate::card_def(id))[key].clone())
            .fold((0, 0), |(attack, health), face| {
                (
                    attack + face["attack"].as_i64().unwrap_or(0),
                    health + face["health"].as_i64().unwrap_or(0),
                )
            })
    }

    fn keyword_kinds(keywords: &Value) -> Vec<String> {
        keywords
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|keyword| keyword["kind"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    /// The rng a sink over the live state starts from; the sink itself is built where it is used,
    /// since it borrows the state.
    fn rng_for(s: &Scenario) -> Rng {
        Rng::new(&s.state().seed, s.state().rng_cursor)
    }

    fn effective(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), card, Default::default())
    }

    fn unit_ids() -> Vec<String> {
        crate::query::query(&json_as(json!({ "type": "Unit" })))
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    // The card and the Discover chain (§8.5, §10.6, R113).

    mod n99_craft_a_card_the_discover_chain {
        use super::*;

        #[test]
        fn s8_5_is_a_4_cost_mythic_spell_patch_v0_1_1_it_cost_3_uncastable_on_3_mana() {
            crate::register_all();
            let def = js(&crate::card_def(CRAFT));
            assert_eq!(def["type"], "Spell");
            assert_eq!(def["cost"], 4);
            assert_eq!(def["rarity"], "Mythic");

            let mut s = scenario(json!({ "p1": { "hand": [CRAFT], "mana": 3 } }));
            s.expect_refused_with(|s| s.play(CRAFT, json!({})), "costs 4, more than your mana");
        }

        #[test]
        fn s10_9_both_faces_are_a_cry_plus_a_resume_table_and_the_radiant_face_has_one_more_step() {
            crate::register_all();
            let scripts = script();
            assert_eq!(members(&scripts.base), vec!["cry", "resume"]);
            assert_eq!(members(&scripts.radiant), vec!["cry", "resume"]);
            // §8.5's radiant cell, "Three Discovers": one more Discover step.
            assert_eq!(scripts.base.resume.len(), 2);
            assert_eq!(scripts.radiant.resume.len(), 3);
        }

        #[test]
        fn s6_3_discover_the_first_prompt_offers_3_distinct_non_token_units_to_the_caster_only() {
            crate::register_all();
            let mut s = craft_scenario(CraftOpts::default());
            s.play(CRAFT, json!({}));

            let pending = open(&s);
            assert_eq!(pending["kind"], "discover");
            assert_eq!(pending["playerId"], "p1");
            assert_eq!(pending["min"], 1);
            assert_eq!(pending["max"], 1);
            assert_eq!(pending["options"].as_array().map(Vec::len), Some(3));

            let ids = option_ids(&pending);
            assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), 3); // drawn without replacement (§6.3)
            let units = unit_ids();
            for id in &ids {
                assert!(units.contains(id));
                assert_eq!(js(&crate::card_def(id))["type"], "Unit");
                // §5.1: a plain type filter asks for no tokens, so no Rush Token or Chaos Golem is craftable.
                assert!(!crate::card_def(id).token);
            }

            // §10.8: the opponent is told a prompt is open and nothing about its options.
            let as_seen_by_p2 = js(&s.view(PlayerId::P2))["pending"].clone();
            assert_eq!(as_seen_by_p2, json!({ "forYou": false, "pendingFor": "p1" }));
        }

        #[test]
        fn r113_answering_the_first_discover_opens_the_second_the_sequence_resumes_through_state_work() {
            crate::register_all();
            let mut s = craft_scenario(CraftOpts::default());
            s.play(CRAFT, json!({}));

            let first = open(&s);
            let first_pick = must(option_ids(&first).into_iter().next(), "an option on the first Discover");
            s.answer(first_key(&first));

            let second = open(&s);
            assert_ne!(second["id"], first["id"]);
            assert_eq!(second["kind"], "discover");
            assert_eq!(second["options"].as_array().map(Vec::len), Some(3));
            assert_eq!(events_of(&s, "promptOpened").len(), 2);

            // The second pool is an independent draw over the same query: nothing is narrowed by the
            // first pick, so the same Unit can be offered — and picked — twice (R60, and §8.5 forbids
            // neither).
            let units = unit_ids();
            for id in option_ids(&second) {
                assert!(units.contains(&id));
            }

            s.answer(first_key(&second));
            assert!(s.state().pending.is_none());
            // The spell only finishes after the whole chain: §10.5 steps 6-8 were owed while it paused.
            s.expect_in_zone(CRAFT, "graveyard");
            assert!(!first_pick.is_empty());
        }

        #[test]
        fn r60_the_second_discover_can_offer_the_card_the_first_one_gave_over_the_seeds() {
            crate::register_all();
            // Proves the second pool is not narrowed: some seed offers the first pick again.
            let mut offered_again = false;
            let mut n = 0;
            while n < 40 && !offered_again {
                let mut s = craft_scenario(CraftOpts { seed: Some(format!("craft-repeat-{n}")), ..CraftOpts::default() });
                s.play(CRAFT, json!({}));
                let first = open(&s);
                let picked = must(option_ids(&first).into_iter().next(), "an option");
                s.answer(first_key(&first));
                offered_again = option_ids(&open(&s)).contains(&picked);
                n += 1;
            }
            assert!(offered_again);
        }

        #[test]
        fn s8_5_radiant_opens_three_discovers_each_resuming_into_the_next_r113() {
            crate::register_all();
            let mut s = craft_scenario(CraftOpts { radiant_face: true, ..CraftOpts::default() });
            s.play(CRAFT, json!({}));

            for _ in 0..3 {
                let pending = open(&s);
                assert_eq!(pending["kind"], "discover");
                assert_eq!(pending["options"].as_array().map(Vec::len), Some(3));
                s.answer(first_key(&pending));
            }
            assert!(s.state().pending.is_none());
            assert_eq!(events_of(&s, "promptOpened").len(), 3);
            s.expect_in_zone(CRAFT, "graveyard");
        }

        #[test]
        fn s10_5_step_4_the_spell_counts_as_played_before_it_asks_anything() {
            crate::register_all();
            let mut s = craft_scenario(CraftOpts::default());
            s.play(CRAFT, json!({}));
            assert_eq!(s.state().counters.played, 1);
            assert_eq!(s.state().players.p1.turn_log.cards_played, 1);
            s.expect_mana(P1, 4); // 8 − 4
        }
    }

    // The fused result (R77, R102).

    mod n99_craft_a_card_the_fused_result_r77_r102 {
        use super::*;

        #[test]
        fn r77_puts_a_fresh_card_in_your_hand_with_costoverride_0_and_nothing_on_the_field() {
            crate::register_all();
            let (s, _) = craft(CraftOpts::default());
            let fused = fused_def_of(s.state());

            let crafted: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == fused.id).collect();
            assert_eq!(crafted.len(), 1);
            let card = must(crafted.first().cloned(), "the crafted card");
            assert_eq!(card.cost_override, Some(subsystems::CRAFTED_CARD_COST));
            // "the result costs 0": the price the play validator reads, not merely a rider (R65).
            assert_eq!(effective(&s, &card), 0);
            // "a fresh, NON-Radiant hand card" (R77), and the fusion happened in a hand, not on a board.
            assert!(!card.radiant);
            assert!(s.unit(P1, 1).is_none());
            assert_eq!(events_of(&s, "fused").len(), 1);
        }

        #[test]
        fn r102_sums_the_stats_unions_the_tags_dedupes_the_keywords_and_fuses_both_faces() {
            crate::register_all();
            let (s, picks) = craft(CraftOpts::default());
            let fused = js(&fused_def_of(s.state()));
            let ingredients: Vec<Value> = picks.iter().map(|id| js(&crate::card_def(id))).collect();

            for key in ["base", "radiant"] {
                let face = &fused[key];
                let faces: Vec<&Value> = ingredients.iter().map(|def| &def[key]).collect();
                assert_eq!(face["attack"], json!(faces.iter().map(|one| one["attack"].as_i64().unwrap_or(0)).sum::<i64>()));
                assert_eq!(face["health"], json!(faces.iter().map(|one| one["health"].as_i64().unwrap_or(0)).sum::<i64>()));
                // One entry per distinct keyword, so Armor 1 and Armor 2 both survive (§10.4).
                let kinds = keyword_kinds(&face["keywords"]);
                assert_eq!(kinds.iter().collect::<IndexSet<_>>().len(), kinds.len());
                for one in &faces {
                    for kind in keyword_kinds(&one["keywords"]) {
                        assert!(kinds.contains(&kind));
                    }
                }
            }

            // Identity, member by member (R102).
            assert_eq!(fused["type"], "Unit"); // no target on the field, so the ingredients' shared type
            let names: Vec<&str> = ingredients.iter().map(|def| def["name"].as_str().unwrap_or_default()).collect();
            assert_eq!(fused["name"], json!(names.join(" + ")));
            assert_eq!(fused["token"], false);
            let mut tags: Vec<String> = fused["tags"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|tag| tag.as_str().unwrap_or_default().to_string())
                .collect();
            tags.sort();
            let mut union: Vec<String> = ingredients
                .iter()
                .flat_map(|def| def["tags"].as_array().cloned().unwrap_or_default())
                .map(|tag| tag.as_str().unwrap_or_default().to_string())
                .collect::<IndexSet<_>>()
                .into_iter()
                .collect();
            union.sort();
            assert_eq!(tags, union);
            // Cost is the capped sum of the printed costs read per R65 — not the crafted instance's 0.
            let sum: i32 = picks.iter().map(|id| query_cost(&crate::card_def(id))).sum();
            assert_eq!(fused["cost"], json!(sum.min(FUSE_COST_CAP)));
        }

        #[test]
        fn r77_both_forms_are_fused_so_making_the_hand_card_radiant_switches_to_the_fused_radiant_form() {
            crate::register_all();
            let (mut s, picks) = craft(CraftOpts::default());
            let fused = fused_def_of(s.state());
            let card = must(
                s.hand(P1).into_iter().find(|entry| entry.def_id == fused.id),
                "the crafted card in hand",
            );

            let (base_attack, base_health) = face_sums(&picks, "base");
            s.expect_stats(&card, json!({ "attack": base_attack, "maxHealth": base_health }));
            // §5.2 is a flag on the instance, so the same card read as Radiant reads the fused radiant face.
            s.card_mut(&card).radiant = true;
            let (radiant_attack, radiant_health) = face_sums(&picks, "radiant");
            s.expect_stats(&card, json!({ "attack": radiant_attack, "maxHealth": radiant_health }));
        }

        #[test]
        fn r77_the_crafted_card_is_playable_and_it_is_the_fused_card_that_lands() {
            crate::register_all();
            let (mut s, _) = craft(CraftOpts::default());
            let fused = fused_def_of(s.state());
            let card = must(
                s.hand(P1).into_iter().find(|entry| entry.def_id == fused.id),
                "the crafted card in hand",
            );

            s.play(&card, json!({}));
            let landed = must(s.unit(P1, 1), "the crafted Unit on the field");
            assert_eq!(landed.def_id, fused.id);
            // It cost 0, so the 4 left after #99 is untouched.
            s.expect_mana(P1, 4);
        }

        #[test]
        fn r102_and_r86_the_ingredients_cease_to_exist_no_graveyard_no_death_trigger_no_destroyed() {
            crate::register_all();
            let (s, _) = craft(CraftOpts::default());
            fused_def_of(s.state());
            // #99's ingredients are Discovered DEFINITIONS that were never cards on a board, so the only
            // thing to assert is that the fusion created no corpse and counted no destruction.
            assert_eq!(
                s.pile(P1, "graveyard").into_iter().map(|card| card.def_id).collect::<Vec<_>>(),
                vec![CRAFT.to_string()],
            );
            assert_eq!(s.state().counters.destroyed, 0);
            assert!(events_of(&s, "destroyed").is_empty());
            assert_eq!(
                events_of(&s, "enteredGraveyard").iter().map(|event| event["defId"].clone()).collect::<Vec<_>>(),
                vec![json!(CRAFT)],
            );
        }

        #[test]
        fn s8_5_r77_radiant_fuses_all_three_picks_into_one_card_both_faces_summed_over_the_three() {
            crate::register_all();
            // #26 in hand, and the mana for it after #99, to make the crafted card Radiant by a real play.
            let (mut s, picks) = craft(CraftOpts {
                radiant_face: true,
                p1: Some(json!({ "hand": [GLOWY], "library": [TIMMY], "mana": 8 })),
                ..CraftOpts::default()
            });
            assert_eq!(picks.len(), 3);
            let fused_def = fused_def_of(s.state());
            let fused = js(&fused_def);
            let ingredients: Vec<Value> = picks.iter().map(|id| js(&crate::card_def(id))).collect();

            let names: Vec<&str> = ingredients.iter().map(|def| def["name"].as_str().unwrap_or_default()).collect();
            assert_eq!(fused["name"], json!(names.join(" + ")));
            // R77: the base form sums the base faces and the radiant form the radiant faces, all three of
            // each, and each unions its own faces' keywords.
            for key in ["base", "radiant"] {
                let (attack, health) = face_sums(&picks, key);
                assert_eq!(fused[key]["attack"], json!(attack), "the fused {key} attack");
                assert_eq!(fused[key]["health"], json!(health), "the fused {key} health");
                let kinds = keyword_kinds(&fused[key]["keywords"]);
                for def in &ingredients {
                    for kind in keyword_kinds(&def[key]["keywords"]) {
                        assert!(kinds.contains(&kind));
                    }
                }
            }

            // A fresh, non-Radiant hand card at 0 (R77), even from the Radiant face.
            let crafted: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id == fused_def.id).collect();
            assert_eq!(crafted.len(), 1);
            let card = must(crafted.first().cloned(), "the crafted card");
            assert!(!card.radiant);
            assert_eq!(effective(&s, &card), 0);
            let (base_attack, base_health) = face_sums(&picks, "base");
            s.expect_stats(&card, json!({ "attack": base_attack, "maxHealth": base_health }));

            // Made Radiant in hand, it reads the three-way fused radiant face.
            s.play(GLOWY, json!({ "targets": [{ "pick": "instance", "instanceId": card.id }] }));
            assert!(s.card(&card).radiant);
            let (radiant_attack, radiant_health) = face_sums(&picks, "radiant");
            s.expect_stats(&card, json!({ "attack": radiant_attack, "maxHealth": radiant_health }));
        }

        #[test]
        fn r4_the_crafted_card_goes_through_s2_4_s_pipeline_and_takes_the_slot_n99_vacated() {
            crate::register_all();
            // §2.4's cap is HAND_CAP. #99 leaves the hand at step 4, so a full hand has one slot free
            // by the time the fusion lands: the crafted card fits and nothing burns. A burn would need
            // a card added between #99 leaving and the fusion landing, so this is R4's reachable half.
            let filler = vec![json!(SPARE); (HAND_CAP - 2) as usize];
            let (s, _) = craft(CraftOpts { p1: Some(json!({ "hand": filler, "mana": 8 })), ..CraftOpts::default() });
            let fused = fused_def_of(s.state());
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == fused.id).count(), 1);
            assert!(events_of(&s, "burned").is_empty());
        }

        #[test]
        fn the_base_face_draws_nothing_after_the_fusion() {
            crate::register_all();
            let (s, _) = craft(CraftOpts { p1: Some(json!({ "library": [TIMMY, TIMMY], "mana": 8 })), ..CraftOpts::default() });
            fused_def_of(s.state());
            assert!(events_of(&s, "drawn").is_empty());
            assert_eq!(s.pile(P1, "library").len(), 2);
        }
    }

    mod n99_craft_a_card_the_radiant_face_draws_nothing_patch_v0_2_7 {
        use super::*;

        #[test]
        fn the_radiant_face_draws_nothing_after_the_fusion_and_the_library_is_untouched() {
            crate::register_all();
            let (s, _) = craft(CraftOpts {
                radiant_face: true,
                p1: Some(json!({ "library": [TIMMY, MENACE], "mana": 8 })),
                ..CraftOpts::default()
            });
            let fused = fused_def_of(s.state());

            assert!(events_of(&s, "drawn").is_empty());
            assert_eq!(
                s.hand(P1).into_iter().map(|card| card.def_id).collect::<Vec<_>>(),
                vec![SPARE.to_string(), fused.id.clone()],
            );
            assert_eq!(
                s.pile(P1, "library").into_iter().map(|card| card.def_id).collect::<Vec<_>>(),
                vec![TIMMY.to_string(), MENACE.to_string()],
            );
        }

        #[test]
        fn r4_with_the_hand_full_the_crafted_card_still_lands_and_nothing_burns() {
            crate::register_all();
            // #99 leaves the hand to resolve and the crafted card takes that slot, so the hand ends at
            // HAND_CAP with no draw after it to burn.
            let filler = vec![json!(SPARE); (HAND_CAP - 2) as usize];
            let (s, _) = craft(CraftOpts {
                radiant_face: true,
                p1: Some(json!({ "hand": filler, "library": [TIMMY], "mana": 8 })),
                ..CraftOpts::default()
            });
            let fused = fused_def_of(s.state());

            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == fused.id).count(), 1);
            assert!(events_of(&s, "burned").is_empty());
        }

        #[test]
        fn s2_4_an_empty_library_costs_the_radiant_face_nothing_no_draw_so_no_fatigue() {
            crate::register_all();
            let (mut s, _) = craft(CraftOpts {
                radiant_face: true,
                p1: Some(json!({ "health": 20, "mana": 8 })),
                ..CraftOpts::default()
            });
            fused_def_of(s.state());
            s.expect_health(P1, 20);
        }
    }

    // R102's "the whole verb does nothing at all", asserted where the rule lives.

    mod fuse_does_nothing_at_all_r102_r23 {
        use super::*;

        fn untouched(s: &Scenario, ingredients: &[CardInstance]) {
            assert!(s.state().transient_defs.is_empty());
            for card in ingredients {
                assert_eq!(s.card(card).def_id, card.def_id);
            }
        }

        /// `subsystems::fuse`'s argument, from a literal (instances as their JSON).
        fn fuse_args(literal: Value) -> subsystems::FuseArgs {
            json_as(literal)
        }

        #[test]
        fn r102_with_fewer_than_two_ingredients_nothing_is_built_and_nothing_changes() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [TIMMY], "hand": [SPARE] } }));
            let timmy = must(s.unit(P1, 1), "the #11 on the field");
            let mut rng = rng_for(&s);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                assert!(
                    subsystems::fuse::fuse(&mut sink, fuse_args(json!({ "ingredients": [], "toHand": "p1" }))).is_none()
                );
                assert!(
                    subsystems::fuse::fuse(&mut sink, fuse_args(json!({ "ingredients": [js(&timmy)], "toHand": "p1" })))
                        .is_none()
                );
            }
            assert_eq!(subsystems::FUSE_MIN_INGREDIENTS, 2);
            assert!(events.is_empty());
            untouched(&s, &[timmy]);
        }

        #[test]
        fn r23_with_an_immutable_target_the_fusion_is_refused_and_the_target_keeps_its_identity() {
            crate::register_all();
            // A radiant #19 Midrange Menace is Taunt + Immutable (§8.2), and R23 blocks Fuse-onto.
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": MENACE, "radiant": true }, TIMMY], "hand": [SPARE] },
            }));
            let immutable = must(s.unit(P1, 1), "the radiant #19");
            let timmy = must(s.unit(P1, 2), "the #11");
            let kinds: Vec<String> = s
                .stats(&immutable)
                .keywords
                .iter()
                .map(|keyword| js(keyword)["kind"].as_str().unwrap_or_default().to_string())
                .collect();
            assert!(kinds.contains(&"Immutable".to_string()));
            let mut rng = rng_for(&s);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                assert!(
                    subsystems::fuse::fuse(
                        &mut sink,
                        fuse_args(json!({ "ingredients": [js(&timmy)], "target": js(&immutable) })),
                    )
                    .is_none()
                );
            }
            assert!(events.is_empty());
            untouched(&s, &[immutable, timmy]);
        }

        #[test]
        fn r102_with_a_target_that_is_not_on_the_field_refused() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [TIMMY], "hand": [SPARE, MENACE] } }));
            let timmy = must(s.unit(P1, 1), "the #11");
            let in_hand = must(s.hand(P1).into_iter().find(|card| card.def_id == MENACE), "the #19 in hand");
            let mut rng = rng_for(&s);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                assert!(
                    subsystems::fuse::fuse(
                        &mut sink,
                        fuse_args(json!({ "ingredients": [js(&timmy)], "target": js(&in_hand) })),
                    )
                    .is_none()
                );
            }
            assert!(events.is_empty());
            untouched(&s, &[timmy, in_hand]);
        }

        #[test]
        fn r102_with_neither_a_target_nor_a_destination_hand_refused() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [TIMMY, MENACE], "hand": [SPARE] } }));
            let timmy = must(s.unit(P1, 1), "the #11");
            let menace = must(s.unit(P1, 2), "the #19");
            let mut rng = rng_for(&s);
            let mut events: Vec<GameEvent> = Vec::new();
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                assert!(
                    subsystems::fuse::fuse(&mut sink, fuse_args(json!({ "ingredients": [js(&timmy), js(&menace)] })))
                        .is_none()
                );
            }
            assert!(events.is_empty());
            untouched(&s, &[timmy, menace]);
        }

        #[test]
        fn r102_an_ingredient_s_own_cost_hook_is_dropped_so_r77_s_cap_wins_n100_s_case() {
            crate::register_all();
            // #100 Ceaseless Void's printed cost is a hook (100 minus four game counters, R55). R102:
            // "Cost is the capped sum of the printed costs and any ingredient `cost` hook is dropped, so
            // R77's cap wins over a cost-rewriting hook."
            let mut s = scenario(json!({ "p1": { "field": ["core-100", TIMMY], "hand": [SPARE] } }));
            let empty = must(s.unit(P1, 1), "the #100 on the field");
            let timmy = must(s.unit(P1, 2), "the #11");
            assert!(printed_cost(s.state(), &empty) > FUSE_COST_CAP);

            let mut rng = rng_for(&s);
            let mut events: Vec<GameEvent> = Vec::new();
            let result: CardInstance = {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                must(
                    subsystems::fuse::fuse(
                        &mut sink,
                        fuse_args(json!({ "ingredients": [js(&empty)], "target": js(&timmy) })),
                    ),
                    "the fused card",
                )
                .clone()
            };
            let fused = js(&fused_def_of(s.state()));
            assert_eq!(fused["cost"], json!(FUSE_COST_CAP));
            // The kept instance now reads the fused def's flat cost, not #100's hook.
            assert_eq!(printed_cost(s.state(), s.card(&result)), FUSE_COST_CAP);
        }
    }
}
