//! #100 Ceaseless Void (SPEC §8.5, §6.3 Cost and Exile, §10.4, R55, R65, R70, R275). Unit, cost 100,
//! 10/10 → 20/20, Mythic.
//!   Base:    "Cry: exile all other permanents on both sides. Costs 1 less per card drawn, played,
//!             destroyed or exiled this game by either player"
//!   Radiant: "Charge. Cry: …; same" — §8 Conventions: "Plus X" adds keyword X to the base keywords
//!            and restates no clause, so both clauses above are kept unchanged. The differences are
//!            catalog data on the radiant face: the printed Charge, and R275's doubled stats.
//!   Engine:  "Four game-level counters; cost recomputed on read; floor 0".
//!
//! THE COST (R55, R65, §10.9). R55: "Count both players' draws, plays, destructions and exiles from
//! the start of the game". Those four counters are `state.counters` — `{ drawn, played, destroyed,
//! exiled }` — which §10.1 puts on `GameState` for this card alone, so the discount is one
//! subtraction over the whole game rather than anything this card has to record. §10.9 gives it the
//! one `cost` hook in the catalog ("`cost` is Ceaseless Void's computed cost, R55"), and R65 starts
//! its calculation from it: "start from `costOverride`, else the printed cost (Ceaseless Void's
//! computed cost, …); add `costMod`; add player discounts; apply Professor Curvature if the result is
//! then 4; floor at 0". So the hook returns the printed 100 minus the four counters and nothing else:
//! `costMod`, the player's discounts and #77's clamp are all `effectiveCost`'s, applied after this.
//!
//! "Cost recomputed on read" is what a hook *is* — `mana.printedCost` calls it on every read — so
//! nothing is ever stored. The floor is here as well as in `printedCost` and `effectiveCost`,
//! because §8.5's Engine cell puts it on this card ("floor 0") and a negative number should never
//! leave the hook.
//!
//! The printed 100 is read from the catalog through `queryCost(def)` rather than written down: a
//! script never restates its own cost, and `queryCost` is R65's out-of-play reading of a definition,
//! which for a plain number is that number.
//!
//! WHICH PLAYS COUNT. `state.counters.played` is bumped by `resolve.countAsPlayed`, which every play
//! and every Cast goes through, so R70's "a cast … counts as a play for every rule that counts or
//! reacts to plays … (Ceaseless Void)" is already true, and `move.counter` rolls the counter back for
//! a Countered card that "is treated as never played" (§6.3). Exiles likewise: `move.exile` bumps
//! `counters.exiled` only when the card actually reaches an exile pile, so a unit token that ceased
//! to exist instead is not counted (R11). None of that is this card's to enforce.
//!
//! PLAYING IT COUNTS TOO, but after the fact: §10.5 pays the cost at step 2 and increments the play
//! counter at step 4, so the card is never 1 cheaper for having been played.
//!
//! THE CRY. "Exile all other permanents on both sides": both rows of §3.1 on both sides — §5.1's
//! permanents are Unit, Field Spell, Trap and Field Trap, which is exactly "everything in the unit
//! zones and the backrow" — with this unit itself excluded. Exile takes no Death trigger (§6.3) and
//! Indestructible does not stop it ("can be exiled or sacrificed", §6.1), so there is nothing to
//! filter and nothing to sequence: one effect, and the state check after it (R59).
//!
//! `exileAll({ side, rows, excludeSelf })` is the board-wide exile (`effects/move.ts`), written in
//! the shared `BoardScope` of `effects/targets.ts` — the same scope #2, #17, #43 and #88 use for the
//! destroy half of the same shape. It exiles every card the scope matches in R68's order, each one
//! through the path `exile` itself uses: `moveToZone` to the exile pile, `state.counters.exiled`
//! bumped per card that gets there, an `exiled` event each, a unit token ceasing to exist instead
//! (R11). It matches only cards on the field, so a card dormant under a Stack pile is not one (R13);
//! the promotion that leaves behind is a rule the spec has not made — see the report.
//!
//! THE GLOW (R662). In hand it lights up once the reductions have brought it within its controller's
//! mana: what a play of it costs now (`playCost`, R65, with this card's own computed cost and every
//! discount) is no more than the mana they hold. The cost itself is on the face (R280), and the glow
//! marks the moment the count has done its work, whether or not a zone is free; the same on both faces.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-100";

/// R65's reading of the printed cost — 100 — taken from the catalog, never restated. TS's module
/// constant `PRINTED_COST = queryCost(def)`: read once, when the script is built (`script()`), and
/// carried into the cost hook.
fn printed_cost_of_def() -> i32 {
    query_cost(&crate::card_def(ID))
}

/// R55: "Costs 1 less per card drawn, played, destroyed or exiled this game by either player",
/// floored at 0 (§8.5's Engine cell). The four counters are game-level, so neither side's identity
/// is read and nothing is stored on the instance.
fn cost_now(state: &GameState, printed_cost: i32) -> i32 {
    let counters = &state.counters;
    let spent = counters.drawn + counters.played + counters.destroyed + counters.exiled;
    (printed_cost - spent).max(0)
}

/// "Cry: exile all other permanents on both sides" — §3.1's two rows, both sides, this unit
/// excluded. `excludeSelf` is what "other" means here; R11 makes a unit token cease to exist
/// rather than reach the pile, so it is not counted (§3.2).
fn exile_every_other_permanent() -> Vec<Effect> {
    vec![exile_all(json_as(json!({ "side": "any", "rows": ["units", "backrow"], "excludeSelf": true })))]
}

/// One script for both faces: "Plus Charge" is a printed keyword on the catalog's radiant face and
/// the 20/20 its printed stats, so both are §10.4 layer 1 and not a line of script, and both of the
/// row's clauses are kept.
pub fn script() -> CardScripts {
    let printed_cost = printed_cost_of_def();
    let void_ = Script {
        cost: Some(cost_hook(move |args| cost_now(args.state, printed_cost))),
        cry: Some(hook(|_ctx| exile_every_other_permanent())),
        // R662: the reductions have brought it within its controller's mana.
        condition_met: Some(condition_hook(|ctx| {
            ctx.zone == ConditionZone::Hand
                && play_cost(ctx.state, ctx.self_) <= unspent_mana_of(ctx.state, ctx.controller)
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: void_.clone(),
        base: void_,
    }
}

// #100 Ceaseless Void — SPEC §8.5, §6.3 (Cost, Exile), §10.4, §10.5, R11, R55, R65, R70.
//
// BUILD M4-T4 row 100: "Cost = 100 − (drawn + played + destroyed + exiled by both players),
// floor 0 (R55); Cry exiles every other permanent; radiant Charge". R275 doubles the radiant face's
// stats: it is a 20/20 with Charge.
//
// The cost half is complete and is tested THROUGH the cost pipeline — `printedCost`,
// `effectiveCost` and the play validator's own refusal — never by reading the hook's return value
// or the catalog's 100 back to itself.
//
// The Cry is `exileAll({ side: "any", rows: ["units", "backrow"], excludeSelf: true })`, the
// board-wide exile the effects barrel exports.
//
// R662's yellow glow (`conditionMet`): in hand once what a play of it costs now is within its
// controller's mana, both faces, at the end of this file.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::mana::{effective_cost, printed_cost};
    use jackioh_engine::state::find_instance_mut;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VOID: &str = "core-100"; // Unit, 100, Mythic, 10/10 → 20/20 plus Charge
    const PRINTED: i32 = 100;

    /// #11 Tempo Timmy, a 1-cost 3/3 Unit. #58 Rush Token Farm, a 2-cost Field Spell with no triggers
    /// on a play. A radiant #25 is an Indestructible 7/7, and #98 an Indestructible Field Spell.
    const TIMMY: &str = "core-011";
    const FARM: &str = "core-058";
    const ROCKY: &str = "core-066"; // #66 The Rock: Indestructible on both faces
    const HEROIC: &str = "core-098";
    const RUSH_TOKEN: &str = "core-t-rush";
    /// #53 Reno, a 3-cost Unit: the spare card that keeps §2.5's auto-end off the assertions.
    const SPARE: &str = "core-053";
    /// #36 Magic Jammed destroys a chosen backrow card; #72 Reminisce exiles itself on resolving.
    const JAMMED: &str = "core-036";
    const REMINISCE: &str = "core-072";
    /// #21 Hinder casts itself on the draw (R40, R70), so it counts as a play nobody played.
    const HINDER: &str = "core-021";

    /// TS's `type Counters` is the engine's `GameCounters` (R55's four game counters).
    type Counters = GameCounters;

    fn must<T>(value: Option<T>, what: &str) -> T {
        match value {
            Some(value) => value,
            None => panic!("expected {what}"),
        }
    }

    fn events_of(s: &Scenario, event_type: GameEventType) -> Vec<GameEvent> {
        s.events()
            .iter()
            .filter(|event| event.event_type() == event_type)
            .cloned()
            .collect()
    }

    fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        [1, 2, 3, 4, 5].into_iter().filter_map(|lane| s.unit(player, lane)).collect()
    }

    fn backrow_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        [1, 2, 3, 4, 5].into_iter().filter_map(|lane| s.backrow(player, lane)).collect()
    }

    fn total_of(counters: &Counters) -> i32 {
        counters.drawn + counters.played + counters.destroyed + counters.exiled
    }

    /// R55's four counters are game-level numbers the engine increments (`draw`, `resolve.countAsPlayed`,
    /// the state check, `move.exile`). Writing them is writing exactly what those paths write, which is
    /// how a test reaches a game that has seen 97 cards without playing one.
    fn set_counters(s: &mut Scenario, counters: Value) -> Counters {
        let mut merged = serde_json::to_value(s.state().counters).expect("the counters serialise");
        for (key, value) in counters.as_object().expect("a partial of the four counters") {
            merged[key.as_str()] = value.clone();
        }
        s.state_mut().counters = json_as(merged);
        s.state().counters
    }

    /// TS `voidScenario({ radiantFace?, p1?, p2? })`: p1 holds the Void (on the face asked for) and the
    /// spare, ahead of any hand `p1` names, with 4 mana unless `p1` says otherwise.
    fn void_scenario(opts: Value) -> Scenario {
        let radiant_face = opts["radiantFace"] == json!(true);
        let mut p1 = json!({ "mana": 4 });
        if let Some(side) = opts["p1"].as_object() {
            for (key, value) in side {
                p1[key.as_str()] = value.clone();
            }
        }
        let mut hand = vec![json!({ "def": VOID, "radiant": radiant_face }), json!(SPARE)];
        if let Some(extra) = opts["p1"]["hand"].as_array() {
            hand.extend(extra.iter().cloned());
        }
        p1["hand"] = Value::Array(hand);
        let mut options = json!({ "seed": "void", "p1": p1 });
        if !opts["p2"].is_null() {
            options["p2"] = opts["p2"].clone();
        }
        scenario(options)
    }

    fn held(s: &Scenario) -> CardInstance {
        must(s.hand(P1).into_iter().find(|card| card.def_id == VOID), "the Void in hand")
    }

    /// TS wrote through `held(s)`, the live instance; here the hand card is reached by id.
    fn held_mut(s: &mut Scenario) -> &mut CardInstance {
        let id = held(s).id;
        must(find_instance_mut(s.state_mut(), &id), "the Void in hand")
    }

    /// `eventsOf(s, "cardPlayed")[0]?.costPaid`.
    fn first_cost_paid(s: &Scenario) -> Option<i32> {
        events_of(s, GameEventType::CardPlayed).first().and_then(|event| match event {
            GameEvent::CardPlayed { cost_paid, .. } => Some(*cost_paid),
            _ => None,
        })
    }

    fn kinds(keywords: &[Keyword]) -> Vec<KeywordKind> {
        keywords.iter().map(Keyword::kind).collect()
    }

    fn def_ids(cards: &[CardInstance]) -> Vec<String> {
        cards.iter().map(|card| card.def_id.clone()).collect()
    }

    /// TS `Object.keys(script)`: the TS names of the fields a script sets.
    fn set_fields(script: &Script) -> Vec<&'static str> {
        let mut keys = Vec::new();
        let mut put = |set: bool, key: &'static str| {
            if set {
                keys.push(key);
            }
        };
        put(script.cost.is_some(), "cost");
        put(script.cry.is_some(), "cry");
        put(script.death.is_some(), "death");
        put(script.start_of_game.is_some(), "startOfGame");
        put(!script.resume.is_empty(), "resume");
        put(script.delayed.is_some(), "delayed");
        put(script.set_stat.is_some(), "setStat");
        put(script.start_of_turn.is_some(), "startOfTurn");
        put(script.end_of_turn.is_some(), "endOfTurn");
        put(script.aura.is_some(), "aura");
        put(!script.triggers.is_empty(), "triggers");
        put(script.on_play_hook.is_some(), "onPlayHook");
        put(!script.hand_triggers.is_empty(), "handTriggers");
        put(script.static_flags.is_some(), "staticFlags");
        put(!script.targets.is_empty(), "targets");
        put(!script.modes.is_empty(), "modes");
        put(script.condition_met.is_some(), "conditionMet");
        put(script.preview.is_some(), "preview");
        put(!script.activations.is_empty(), "activations");
        put(!script.target_checks.is_empty(), "targetChecks");
        put(script.cost_aura.is_some(), "costAura");
        put(script.graveyard_play.is_some(), "graveyardPlay");
        put(script.targeting_discards.is_some(), "targetingDiscards");
        put(script.records_play_as.is_some(), "recordsPlayAs");
        put(script.draw_limit.is_some(), "drawLimit");
        put(!script.replacements.is_empty(), "replacements");
        put(script.hero_guard.is_some(), "heroGuard");
        put(script.conditional_keywords.is_some(), "conditionalKeywords");
        put(script.after_attack.is_some(), "afterAttack");
        put(script.plague_multiplier.is_some(), "plagueMultiplier");
        put(!script.deck_triggers.is_empty(), "deckTriggers");
        put(!script.graveyard_triggers.is_empty(), "graveyardTriggers");
        put(script.quests.is_some(), "quests");
        put(script.tribute_when.is_some(), "tributeWhen");
        put(script.would_counter.is_some(), "wouldCounter");
        put(script.start_of_opponent_turn.is_some(), "startOfOpponentTurn");
        keys
    }

    // ---------------------------------------------------------------------------
    // The card.
    // ---------------------------------------------------------------------------

    mod n100_ceaseless_void_the_card {
        use super::*;

        #[test]
        fn s8_5_is_a_mythic_10_10_20_20_unit_printed_at_100_with_charge_only_on_the_radiant_face() {
            crate::register_all();
            let def = crate::card_def(VOID);
            assert_eq!(def.type_, CardType::Unit);
            assert_eq!(serde_json::to_value(def.cost).unwrap(), json!(PRINTED));
            assert_eq!(serde_json::to_value(def.rarity).unwrap(), json!("Mythic"));
            assert_eq!((def.base.attack, def.base.health), (Some(10), Some(10)));
            // R275: a Radiant Unit's attack and health are each at least double its base face's.
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(20), Some(20)));
            // §8 Conventions: "Plus X" adds keyword X and restates no clause.
            assert!(!kinds(&def.base.keywords).contains(&KeywordKind::Charge));
            assert_eq!(kinds(&def.radiant.keywords), vec![KeywordKind::Charge]);
        }

        #[test]
        fn s10_9_one_script_serves_both_faces_the_cost_hook_the_cry_and_r662_s_glow_and_nothing_else() {
            crate::register_all();
            // "Plus Charge" and the 20/20 are printed on the radiant face and so §10.4 layer 1, not a line
            // of script, which is why the two faces are the same object and both of the row's clauses are
            // kept.
            let scripts = script();
            let (void_base, void_radiant) = (&scripts.base, &scripts.radiant);
            // TS `toBe`: the radiant face's hooks are the base face's own (shared `Arc`s).
            assert!(Arc::ptr_eq(void_radiant.cost.as_ref().unwrap(), void_base.cost.as_ref().unwrap()));
            assert!(Arc::ptr_eq(void_radiant.cry.as_ref().unwrap(), void_base.cry.as_ref().unwrap()));
            assert!(Arc::ptr_eq(
                void_radiant.condition_met.as_ref().unwrap(),
                void_base.condition_met.as_ref().unwrap()
            ));
            assert_eq!(set_fields(void_radiant), set_fields(void_base));
            let mut keys = set_fields(void_base);
            keys.sort();
            assert_eq!(keys, vec!["conditionMet", "cost", "cry"]);
        }

        #[test]
        fn s10_4_r275_the_base_face_reads_10_10_and_the_radiant_face_20_20_through_the_layers() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [VOID], "hand": [SPARE] },
                "p2": { "field": [{ "def": VOID, "radiant": true }] },
            }));
            let base_void = must(s.unit(P1, 1), "the base Void");
            s.expect_stats(&base_void, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
            let radiant_void = must(s.unit(P2, 1), "the radiant Void");
            s.expect_stats(&radiant_void, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));
        }
    }

    // ---------------------------------------------------------------------------
    // The cost (R55, R65). Tested through the pipeline and the play validator.
    // ---------------------------------------------------------------------------

    mod n100_ceaseless_void_the_cost_r55_r65 {
        use super::*;

        #[test]
        fn s8_5_on_a_fresh_game_it_costs_its_printed_100_and_no_play_can_afford_it() {
            crate::register_all();
            let mut s = void_scenario(json!({}));
            assert_eq!(s.state().counters, GameCounters { drawn: 0, played: 0, destroyed: 0, exiled: 0 });
            // The hook is consulted ahead of the printed cost, and on a fresh game they agree.
            assert_eq!(printed_cost(s.state(), &held(&s)), PRINTED);
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), PRINTED);
            s.expect_refused_with(|s| s.play(VOID, json!({})), "Ceaseless Void costs 100, more than your mana");
        }

        #[test]
        fn r55_each_of_the_four_counters_takes_1_off_on_either_player_s_behalf() {
            crate::register_all();
            let mut s = void_scenario(json!({}));
            for key in ["drawn", "played", "destroyed", "exiled"] {
                let mut counters = json!({ "drawn": 0, "played": 0, "destroyed": 0, "exiled": 0 });
                counters[key] = json!(1);
                set_counters(&mut s, counters);
                assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), PRINTED - 1);
            }
            // They add up: four counters at 25 is 100 spent.
            set_counters(&mut s, json!({ "drawn": 25, "played": 25, "destroyed": 25, "exiled": 25 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 0);
        }

        #[test]
        fn s8_5_floors_at_0_a_long_game_never_gives_it_a_negative_cost() {
            crate::register_all();
            let mut s = void_scenario(json!({}));
            set_counters(&mut s, json!({ "drawn": 200, "played": 50, "destroyed": 10, "exiled": 10 }));
            assert_eq!(printed_cost(s.state(), &held(&s)), 0);
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 0);
        }

        #[test]
        fn s8_5_cost_recomputed_on_read_nothing_is_stored_on_the_instance() {
            crate::register_all();
            let mut s = void_scenario(json!({}));
            let card = held(&s);
            assert!(card.cost_override.is_none());

            set_counters(&mut s, json!({ "drawn": 10 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 90);
            set_counters(&mut s, json!({ "drawn": 40 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 60);
            set_counters(&mut s, json!({ "drawn": 0 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), PRINTED);
            assert!(held(&s).cost_override.is_none());
            assert_eq!(held(&s).cost_mod, 0);
        }

        #[test]
        fn r65_the_hook_is_the_start_of_the_calculation_costmod_and_discounts_still_apply_after_it() {
            crate::register_all();
            let mut s = void_scenario(json!({}));
            set_counters(&mut s, json!({ "drawn": 96 }));
            assert_eq!(printed_cost(s.state(), &held(&s)), 4);
            // `costMod` is what `setCostMod` writes, and R65 adds it to the printed cost — which for this
            // card is the hook's number, not the catalog's 100.
            held_mut(&mut s).cost_mod = -2;
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 2);
            held_mut(&mut s).cost_mod = 0;
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 4);
        }

        #[test]
        fn r55_the_counters_are_the_engine_s_real_draws_plays_destructions_and_exiles_move_them() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "void-real",
                "p1": {
                    "hand": [VOID, JAMMED, REMINISCE, SPARE],
                    "library": [SPARE, SPARE],
                    "graveyard": [TIMMY],
                    "mana": 8,
                },
                "p2": { "backrow": [FARM] },
            }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), PRINTED);

            // A draw.
            s.start_turn();
            let after_draw = effective_cost(s.state(), &held(&s), Default::default());
            assert!(s.state().counters.drawn >= 1);
            assert_eq!(after_draw, PRINTED - total_of(&s.state().counters));

            // A play that destroys: #36 Magic Jammed on p2's Field Spell.
            let farm = must(s.backrow(P2, 1), "p2's #58");
            s.play(JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": farm.id }] }));
            assert!(s.state().counters.played >= 1);
            assert!(s.state().counters.destroyed >= 1);

            // A play that exiles: #72 Reminisce exiles itself once its Discover is answered.
            s.play(REMINISCE, json!({}));
            let key = must(
                s.state().pending.as_ref().and_then(|pending| pending.options.first()).map(|option| option.key.clone()),
                "a graveyard card",
            );
            s.answer(json!(key));
            assert!(s.state().counters.exiled >= 1);

            let total = total_of(&s.state().counters);
            assert!(total >= 4);
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), PRINTED - total);
        }

        #[test]
        fn r70_a_cast_counts_as_a_play_so_a_card_nobody_played_still_makes_the_void_cheaper() {
            crate::register_all();
            // #21 Hinder casts itself on the draw. R70: "counts as a play for every rule that counts or
            // reacts to plays … (Ceaseless Void)".
            let mut s = scenario(json!({
                "seed": "void-cast",
                // The Radiant face discards nothing (R431), so the cast asks no question.
                "p1": { "hand": [VOID, SPARE], "library": [{ "def": HINDER, "radiant": true }, SPARE, SPARE], "mana": 4 },
                "p2": { "hand": [SPARE] },
            }));
            s.start_turn();

            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::CardPlayed { def_id, .. } if def_id == HINDER))
            );
            assert!(s.state().counters.played >= 1);
            // …and no `play` action was ever sent: the counter moved on a cast alone.
            assert_eq!(
                effective_cost(s.state(), &held(&s), Default::default()),
                PRINTED - total_of(&s.state().counters)
            );
        }

        #[test]
        fn s10_5_playing_it_never_makes_itself_cheaper_the_cost_is_paid_before_the_play_is_counted() {
            crate::register_all();
            let mut s = void_scenario(json!({ "p1": { "field": [TIMMY], "mana": 4 } }));
            set_counters(&mut s, json!({ "drawn": 97 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 3);

            s.play(VOID, json!({}));
            // Step 2 pays, step 4 increments the play counter, so 3 was paid rather than 2.
            assert_eq!(first_cost_paid(&s), Some(3));
            s.expect_mana(P1, 1);
            assert_eq!(s.state().counters.played, 1);
        }

        #[test]
        fn s8_5_once_the_game_has_spent_100_cards_it_is_free_and_it_lands_as_a_10_10() {
            crate::register_all();
            let mut s = void_scenario(json!({ "p1": { "mana": 0 } }));
            set_counters(&mut s, json!({ "drawn": 60, "played": 20, "destroyed": 10, "exiled": 10 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 0);

            s.play(VOID, json!({}));
            let landed = must(s.unit(P1, 1), "the Void on the field");
            assert_eq!(landed.def_id, VOID);
            s.expect_stats(&landed, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
            assert_eq!(first_cost_paid(&s), Some(0));
        }
    }

    // ---------------------------------------------------------------------------
    // The Cry (§8.5, §6.3 Exile, R11). FAILING: `exileAll` is not re-exported by effects/index.ts.
    // ---------------------------------------------------------------------------

    mod n100_ceaseless_void_the_cry_exiles_every_other_permanent {
        use super::*;

        /// A board with a permanent of every interesting kind on both sides, and #100 in p1's hand.
        fn board_scenario(radiant_face: bool) -> Scenario {
            let mut s = void_scenario(json!({
                "radiantFace": radiant_face,
                "p1": { "field": [TIMMY, RUSH_TOKEN], "backrow": [FARM], "mana": 4 },
                "p2": { "field": [{ "def": ROCKY, "radiant": true }], "backrow": [HEROIC] },
            }));
            set_counters(&mut s, json!({ "drawn": 100 })); // free, so the play itself is never the thing under test
            s
        }

        #[test]
        fn s8_5_exiles_both_rows_on_both_sides_and_leaves_itself_standing() {
            crate::register_all();
            let mut s = board_scenario(false);
            let others: Vec<CardInstance> = [
                units_of(&s, P1),
                backrow_of(&s, P1),
                units_of(&s, P2),
                backrow_of(&s, P2),
            ]
            .concat();
            assert_eq!(others.len(), 5);

            s.play(VOID, json!({}));

            // "all OTHER permanents": the Void is the only card left standing, wherever it landed (R64).
            assert_eq!(def_ids(&units_of(&s, P1)), vec![VOID.to_string()]);
            assert!(backrow_of(&s, P1).is_empty());
            assert!(units_of(&s, P2).is_empty());
            assert!(backrow_of(&s, P2).is_empty());
            assert_eq!(events_of(&s, GameEventType::Exiled).len(), 5);
        }

        #[test]
        fn s6_1_indestructible_does_not_stop_an_exile_a_unit_or_a_field_spell() {
            crate::register_all();
            let mut s = board_scenario(false);
            let rocky = must(s.unit(P2, 1), "the radiant #66");
            let heroic = must(s.backrow(P2, 1), "the #98 Heroic Power");
            assert!(kinds(&s.stats(&rocky).keywords).contains(&KeywordKind::Indestructible));
            assert!(kinds(&s.stats(&heroic).keywords).contains(&KeywordKind::Indestructible));

            s.play(VOID, json!({}));
            s.expect_in_zone(&rocky, "exile");
            s.expect_in_zone(&heroic, "exile");
        }

        #[test]
        fn s6_3_an_exile_takes_no_death_trigger_and_counts_nothing_as_destroyed() {
            crate::register_all();
            let mut s = board_scenario(false);
            let before = s.state().counters.destroyed;

            s.play(VOID, json!({}));
            assert!(events_of(&s, GameEventType::Destroyed).is_empty());
            assert!(events_of(&s, GameEventType::EnteredGraveyard).is_empty());
            assert_eq!(s.state().counters.destroyed, before);
            assert!(s.pile(P1, "graveyard").is_empty());
            assert!(s.pile(P2, "graveyard").is_empty());
        }

        #[test]
        fn r11_a_unit_token_ceases_to_exist_instead_of_reaching_the_exile_pile_and_is_not_counted() {
            crate::register_all();
            let mut s = board_scenario(false);
            let token = must(
                units_of(&s, P1).into_iter().find(|unit| unit.def_id == RUSH_TOKEN),
                "the Rush Token on the field",
            );
            let timmy = must(units_of(&s, P1).into_iter().find(|unit| unit.def_id == TIMMY), "the #11");
            let exiled_before = s.state().counters.exiled;

            s.play(VOID, json!({}));
            s.expect_in_zone(&token, "gone");
            s.expect_in_zone(&timmy, "exile");
            assert!(!s.pile(P1, "exile").iter().any(|card| card.def_id == RUSH_TOKEN));
            // R55 counts a card that REACHES the pile, so the four real cards count and the token does not.
            assert_eq!(s.state().counters.exiled, exiled_before + 4);
        }

        #[test]
        fn s3_1_it_reaches_the_field_and_only_the_field_hands_libraries_and_graveyards_are_untouched() {
            crate::register_all();
            let mut s = void_scenario(json!({
                "p1": { "field": [TIMMY], "library": [SPARE, SPARE], "graveyard": [SPARE], "mana": 4 },
                "p2": { "hand": [SPARE, SPARE], "library": [SPARE], "graveyard": [SPARE] },
            }));
            set_counters(&mut s, json!({ "drawn": 100 }));

            s.play(VOID, json!({}));
            assert_eq!(s.pile(P1, "library").len(), 2);
            assert_eq!(s.pile(P1, "graveyard").len(), 1);
            assert_eq!(s.hand(P2).len(), 2);
            assert_eq!(s.pile(P2, "library").len(), 1);
            assert_eq!(s.pile(P2, "graveyard").len(), 1);
            // p1's own hand still holds the spare; only the board was cleared.
            assert_eq!(def_ids(&s.hand(P1)), vec![SPARE.to_string()]);
        }

        #[test]
        fn s8_conventions_the_radiant_face_keeps_the_cry_plus_charge_restates_no_clause() {
            crate::register_all();
            let mut s = board_scenario(true);
            s.play(VOID, json!({}));
            assert_eq!(def_ids(&units_of(&s, P1)), vec![VOID.to_string()]);
            assert!(units_of(&s, P2).is_empty());
            assert_eq!(events_of(&s, GameEventType::Exiled).len(), 5);
        }
    }

    // ---------------------------------------------------------------------------
    // The radiant face: "Plus Charge" (§6.1).
    // ---------------------------------------------------------------------------

    mod n100_ceaseless_void_radiant_plus_charge {
        use super::*;

        /// The mana is 4 rather than 0 so the spare in hand stays affordable: a side with nothing it can
        /// do auto-ends its turn (§2.5) and the fatigue of the turns that follow would land on the heroes
        /// this test measures. That it costs 0 here is asserted by `costPaid`.
        fn play_freely(radiant_face: bool) -> Scenario {
            let mut s = void_scenario(json!({ "radiantFace": radiant_face, "p1": { "mana": 4 } }));
            set_counters(&mut s, json!({ "drawn": 100 }));
            s.play(VOID, json!({}));
            assert_eq!(first_cost_paid(&s), Some(0));
            s
        }

        #[test]
        fn s6_1_charge_the_radiant_face_may_attack_the_enemy_hero_the_turn_it_is_played_for_20() {
            crate::register_all();
            let mut s = play_freely(true);
            let card = must(s.unit(P1, 1), "the radiant Void");
            assert!(kinds(&s.stats(&card).keywords).contains(&KeywordKind::Charge));
            s.expect_stats(&card, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));

            s.attack(&card, "hero");
            s.expect_health(P2, 10); // 30 − 20
        }

        #[test]
        fn s4_1_the_base_face_is_summoning_sick_so_it_may_attack_nothing_on_its_own_turn() {
            crate::register_all();
            let mut s = play_freely(false);
            let card = must(s.unit(P1, 1), "the base Void");
            assert!(!kinds(&s.stats(&card).keywords).contains(&KeywordKind::Charge));
            s.expect_refused(|s| s.attack(&card, "hero"));
            s.expect_health(P2, 30);
        }

        #[test]
        fn s8_conventions_the_radiant_face_keeps_the_cost_clause_too() {
            crate::register_all();
            let mut s = void_scenario(json!({ "radiantFace": true }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), PRINTED);
            set_counters(&mut s, json!({ "drawn": 30, "played": 30, "destroyed": 20, "exiled": 15 }));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 5);
        }
    }

    mod n100_ceaseless_void_glows_once_the_count_brings_it_within_your_mana_r662 {
        use super::*;

        /// TS's `for (const radiantFace of [false, true])` body, first `it`.
        fn glows_at_97_and_can_be_played(radiant_face: bool) {
            crate::register_all();
            let mut s = void_scenario(json!({ "radiantFace": radiant_face }));
            set_counters(&mut s, json!({ "drawn": 97 }));
            assert!(hand_glows(&s, &held(&s).id, P1));
            s.play(VOID, json!({}));
            s.expect_in_zone(VOID, "field");
        }

        /// TS's `for (const radiantFace of [false, true])` body, second `it`.
        fn does_not_glow_at_95(radiant_face: bool) {
            crate::register_all();
            let mut s = void_scenario(json!({ "radiantFace": radiant_face }));
            set_counters(&mut s, json!({ "drawn": 95 }));
            assert!(!hand_glows(&s, &held(&s).id, P1));
            assert_eq!(effective_cost(s.state(), &held(&s), Default::default()), 5);
        }

        #[test]
        fn r662_base_at_97_counted_it_costs_3_of_your_4_mana_and_glows_and_it_can_be_played() {
            glows_at_97_and_can_be_played(false);
        }

        #[test]
        fn r662_base_at_95_counted_it_costs_5_more_than_your_4_and_does_not_glow() {
            does_not_glow_at_95(false);
        }

        #[test]
        fn r662_radiant_at_97_counted_it_costs_3_of_your_4_mana_and_glows_and_it_can_be_played() {
            glows_at_97_and_can_be_played(true);
        }

        #[test]
        fn r662_radiant_at_95_counted_it_costs_5_more_than_your_4_and_does_not_glow() {
            does_not_glow_at_95(true);
        }
    }
}
