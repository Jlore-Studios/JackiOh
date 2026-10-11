//! #95 Call to Chaos (Core Edition) (SPEC §8.4, R28, R58, R60, R70, R87, R423, R436, BUILD M4-T4 row 95).
//!
//! Base: "One random effect" out of ten. Radiant (R423): "Three different random effects, resolved
//! in the order listed", the shape Classic+ #73 has. The recursion is one of the ten like any other.
//!
//! The whole card is `subsystems::call_to_chaos`; each face here is a one-line hook. The subsystem
//! owns the ten effects, the roll, the announcement and the chain counter:
//!   * each effect reads the board when it RESOLVES, not when the hook builds it: a rolled recursion
//!     resolves its whole chain before the effects after it (R87).
//!   * the chain length is game state on the cast's `memory` (§10.1, `CHAOS_CHAIN_KEY`), so a resumed
//!     game keeps its cap and two Calls in one turn never share a counter.
//!   * R28's cap of 20 (`CALL_TO_CHAOS_CHAIN_CAP`) is a HARD stop: a recursion rolled at the cap
//!     resolves into nothing and no substitute is rolled (R87).
//!   * R436: before anything resolves, `chaosRolled` names the rolled clauses to both players.
//!
//! R70 makes the recursion a Cast (free, counted as a play, the card's own script, the caster picks
//! targets); R87 sends a card cast from no zone to the graveyard (§10.5 step 7), feeding Gravedigger.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-095";

/// §8: "One random effect" out of the ten.
fn base() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: None,
            })]
        })),
        ..Script::default()
    }
}

/// §8, R423: three different random effects of the ten, resolved in the order the list writes them.
fn radiant() -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: None,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: base(),
        radiant: radiant(),
    }
}

// #95 Call to Chaos (Core Edition) and #95.1 Chaos Golem — SPEC §8.4, §7, §5.1, §10.5, §10.8,
// R4, R11, R28, R60, R64, R70, R87, R423, R436.
//
// BUILD M4-T4 row 95: "Each of the 10 effects has a test; recursion stops at 20 (R28); radiant rolls
// three different effects of the ten, resolved in the list's order (R423); what was rolled is named
// to both players (R436)". Row 95.1: "10/10 with all four keywords"; its Radiant face (R276) is 20/20
// "Charge, Lifesteal, Divine Shield, First Strike" (R275).
//
// HOW AN EFFECT IS FORCED. `subsystems::roll_chaos_effects` is a pure function of the rng, whose
// whole state is `(state.seed, state.rngCursor)` (§9.3, §10.7), and it is the FIRST rng draw of the
// play action, so setting `state.rngCursor` before the play pins which of the ten resolves.
// `cursor_for` finds that cursor with the engine's own roll; each test asserts what the card did.
// `s9_3_the_roll_is_a_pure_function_of_seed_cursor_…` pins the assumption by name.
//
// The effects' machinery (lazy wrappers, chain counter, R87's order) is proved in
// `crates/engine/tests/rules/call_to_chaos.rs`. This file owes the card: both faces wired to it
// with the right §5.2 flag, and the ten effects against the REAL catalog down the §10.5 play path.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    // R28's number lives in `config.rs` (`CALL_TO_CHAOS_CHAIN_CAP`) and is used by name below.

    const CHAOS: &str = "core-095"; // Spell, 4, Legendary, tag "Call to Chaos"
    const GOLEM: &str = "core-095-1"; // #95.1, Unit token, 10/10 → 20/20
    const RUSH_TOKEN: &str = "core-t-rush";

    /// Fillers with known data: #19 is a 3-cost 9/9 Taunt Unit, #36 a 1-cost Spell, #53 a 3-cost Unit.
    const MENACE: &str = "core-019";
    const JAMMED: &str = "core-036";
    const RENO: &str = "core-053";
    /// A radiant #91 Fed Fauci is 2/12: the one enemy body that survives a 10-attack First Strike.
    const FAUCI: &str = "core-091";
    /// #81 Radiant Saintess (2/2, Reborn; Death: your other units become Radiant) and #44 True Strike.
    const SAINTESS: &str = "core-081";
    const TRUE_STRIKE: &str = "core-044";

    const SEED: &str = "chaos-card";

    const CURSOR_SEARCH: u32 = 500;

    /// A radiant #19 is 18/18, which is lethal to a 10/10 whose Divine Shield is already spent.
    fn big_menace() -> Value {
        json!({ "def": MENACE, "radiant": true })
    }

    use crate::js;

    use crate::matches_object;

    /// A shallow object spread: `{ ...base, ...extra }`.
    fn merge(base: Value, extra: Value) -> Value {
        let mut out = base;
        if let (Some(into), Value::Object(from)) = (out.as_object_mut(), extra) {
            for (key, value) in from {
                into.insert(key, value);
            }
        }
        out
    }

    /// The keys of every member the face sets.
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

    /// `query(args)`, by id.
    fn query_ids(args: Value) -> Vec<String> {
        crate::query::query(&json_as(args))
            .iter()
            .map(|def| def.id.clone())
            .collect()
    }

    /// What the engine's base roll picks for (SEED, cursor).
    fn rolled_at(cursor: u32) -> Option<String> {
        subsystems::roll_chaos_effects(&mut Rng::new(SEED, cursor), false, None)
            .first()
            .map(|effect| effect.name.to_string())
    }

    /// R423: the three the engine's Radiant roll picks for (SEED, cursor), in the order they resolve.
    fn radiant_roll_at(cursor: u32) -> Vec<String> {
        subsystems::roll_chaos_effects(&mut Rng::new(SEED, cursor), true, None)
            .iter()
            .map(|effect| effect.name.to_string())
            .collect()
    }

    fn cursor_for(effect: &str) -> u32 {
        for cursor in 0..CURSOR_SEARCH {
            if rolled_at(cursor).as_deref() == Some(effect) {
                return cursor;
            }
        }
        panic!("no cursor below {CURSOR_SEARCH} rolls \"{effect}\" from seed \"{SEED}\"");
    }

    /// R423: a cursor whose Radiant roll the predicate accepts.
    fn radiant_cursor_where(accept: impl Fn(&[String]) -> bool, what: &str) -> u32 {
        for cursor in 0..CURSOR_SEARCH * 4 {
            if accept(&radiant_roll_at(cursor)) {
                return cursor;
            }
        }
        panic!(
            "no cursor below {} rolls {what} on the Radiant face from seed \"{SEED}\"",
            CURSOR_SEARCH * 4
        );
    }

    fn has(rolled: &[String], name: &str) -> bool {
        rolled.iter().any(|entry| entry == name)
    }

    /// p1 holds #95 and a spare card and has mana to spare, so §2.5's auto-end never fires under the
    /// assertions: the spare is still affordable once the 4 is paid, which is a "meaningful" action.
    fn side(extra: Value) -> Value {
        merge(json!({ "hand": [CHAOS, MENACE], "mana": 8 }), extra)
    }

    #[derive(Default)]
    struct ChaosOpts {
        p1: Option<Value>,
        p2: Option<Value>,
        radiant_cursor: Option<u32>,
        chain: Option<i32>,
    }

    /// Writes `memory[key]` on the card a reference names.
    fn set_memory(s: &mut Scenario, card: &str, key: &str, value: Value) {
        let id = s.card(card).id.clone();
        find_instance_mut(s.state_mut(), &id)
            .expect("the card is in the state")
            .memory
            .insert(key.to_string(), value);
    }

    /// Play #95 with the roll pinned: a base roll of `effect`, or — `radiant_cursor` given — the Radiant
    /// roll that cursor draws. `chain` seeds R28's counter on the played card.
    fn chaos(effect: Option<&str>, opts: ChaosOpts) -> Scenario {
        let radiant_face = opts.radiant_cursor.is_some();
        let mut p1 = opts.p1.unwrap_or_else(|| side(json!({})));
        let hand = p1.get("hand").cloned().unwrap_or_else(|| json!([CHAOS, MENACE]));
        let hand: Vec<Value> = hand
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|entry| {
                if entry == json!(CHAOS) {
                    json!({ "def": CHAOS, "radiant": radiant_face })
                } else {
                    entry
                }
            })
            .collect();
        p1["hand"] = json!(hand);
        let mut options = json!({ "seed": SEED, "p1": p1 });
        if let Some(p2) = opts.p2 {
            options["p2"] = p2;
        }
        let mut s = scenario(options);
        if let Some(chain) = opts.chain {
            set_memory(&mut s, CHAOS, subsystems::CHAOS_CHAIN_KEY, json!(chain));
        }
        s.state_mut().rng_cursor = match opts.radiant_cursor {
            Some(cursor) => cursor,
            None => cursor_for(must(effect, "a base effect to pin")),
        };
        s.play(CHAOS, json!({}));
        s
    }

    fn events_json(s: &Scenario) -> Vec<Value> {
        s.events().iter().map(js).collect()
    }

    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        events_json(s).into_iter().filter(|event| event["type"] == kind).collect()
    }

    fn units_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5).filter_map(|lane| s.unit(player, lane)).collect()
    }

    fn backrow_of(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        (1..=5).filter_map(|lane| s.backrow(player, lane)).collect()
    }

    fn keywords_of(s: &Scenario, card: &CardInstance) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| js(keyword)["kind"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    fn sorted(mut list: Vec<String>) -> Vec<String> {
        list.sort();
        list
    }

    /// How many times a #95 was played or cast: R70 makes a cast a play, so both emit `cardPlayed`.
    fn chaos_plays(s: &Scenario) -> Vec<Value> {
        events_of(s, "cardPlayed")
            .into_iter()
            .filter(|event| event["defId"] == CHAOS)
            .collect()
    }

    fn must<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("expected {what}"))
    }

    fn effective(s: &Scenario, card: &CardInstance) -> i32 {
        effective_cost(s.state(), card, Default::default())
    }

    fn id_of(event: &Value) -> String {
        event["instanceId"].as_str().unwrap_or_default().to_string()
    }

    // The card, its two faces and the fixture assumption.

    mod n95_call_to_chaos_the_card {
        use super::*;

        #[test]
        fn s8_4_is_a_4_cost_spell_that_resolves_and_reaches_the_graveyard_s10_5_step_7() {
            crate::register_all();
            let mut s = chaos(Some("heal"), ChaosOpts::default());
            s.expect_in_zone(CHAOS, "graveyard");
            s.expect_mana(P1, 4); // 8 − 4
            assert_eq!(events_of(&s, "cardPlayed")[0]["costPaid"], 4);
        }

        #[test]
        fn s8_4_costs_4_it_is_uncastable_on_3_mana() {
            crate::register_all();
            let mut s = scenario(json!({ "seed": SEED, "p1": { "hand": [CHAOS], "mana": 3 } }));
            s.expect_refused_with(|s| s.play(CHAOS, json!({})), "costs 4, more than your mana");
        }

        #[test]
        fn s10_9_both_faces_hang_the_whole_card_off_cry_and_nothing_else() {
            crate::register_all();
            // §8.4's two cells differ only in how many effects are rolled, which is the subsystem's
            // argument; a second hook here would be a rule this card does not have.
            let scripts = script();
            assert_eq!(members(&scripts.base), vec!["cry"]);
            assert_eq!(members(&scripts.radiant), vec!["cry"]);
        }

        #[test]
        fn s9_3_the_roll_is_a_pure_function_of_seed_cursor_so_pinning_the_cursor_pins_the_effect() {
            crate::register_all();
            // The fixture assumption of this file, asserted rather than assumed: the roll is the play's
            // first rng draw, so `rolled_at(cursor)` is what the card does. Proved through the card with
            // the one effect nothing else can be mistaken for — the Chaos Golem is a token only #95 makes.
            let golem_cursor = cursor_for("golem");
            let play = || -> Scenario {
                let mut s = scenario(json!({ "seed": SEED, "p1": side(json!({})) }));
                s.state_mut().rng_cursor = golem_cursor;
                s.play(CHAOS, json!({}));
                s
            };
            let ids = |s: &Scenario| -> Vec<String> { units_of(s, P1).into_iter().map(|unit| unit.def_id).collect() };
            assert_eq!(ids(&play()), vec![GOLEM.to_string()]);
            // …and the same seed and cursor replay to the same roll (R60, §9.3).
            assert_eq!(ids(&play()), vec![GOLEM.to_string()]);
        }

        #[test]
        fn r28_the_base_face_rolls_exactly_one_of_the_ten_and_all_ten_are_reachable() {
            crate::register_all();
            let mut names: IndexSet<String> = IndexSet::new();
            for cursor in 0..CURSOR_SEARCH {
                let rolled: Vec<String> = subsystems::roll_chaos_effects(&mut Rng::new(SEED, cursor), false, None)
                    .iter()
                    .map(|effect| effect.name.to_string())
                    .collect();
                assert_eq!(rolled.len(), 1);
                names.insert(must(rolled.first().cloned(), "a rolled effect"));
            }
            assert_eq!(names.len(), 10);
        }
    }

    // The ten effects of §8.4, in the order the card lists them.

    mod n95_call_to_chaos_base_the_ten_effects {
        use super::*;

        #[test]
        fn s8_4_1_10_summons_3_random_3_cost_units_into_the_leftmost_free_zones_r60_r64() {
            crate::register_all();
            let s = chaos(Some("units"), ChaosOpts::default());
            let summoned = events_of(&s, "summoned");
            assert_eq!(summoned.len(), 3);
            assert_eq!(summoned.iter().map(|event| event["lane"].clone()).collect::<Vec<_>>(), vec![json!(1), json!(2), json!(3)]);
            assert!(summoned.iter().all(|event| event["row"] == "units"));

            // The pool is the real catalog's 3-cost non-token Units, read per R65.
            let pool = query_ids(json!({ "type": "Unit", "cost": 3 }));
            assert!(pool.len() > 1);
            for unit in units_of(&s, P1) {
                let def = crate::card_def(&unit.def_id);
                assert_eq!(js(&def)["type"], "Unit");
                assert!(!def.token);
                assert_eq!(query_cost(&def), 3);
                assert!(pool.contains(&unit.def_id));
            }
            // A summon is not a play: none of the three fired its Cry (R1).
            assert_eq!(events_of(&s, "cardPlayed").len(), 1);
        }

        #[test]
        fn s8_4_2_10_heals_your_hero_30_and_only_yours_past_30_s6_3_heal_has_no_hero_cap() {
            crate::register_all();
            let mut hurt = chaos(
                Some("heal"),
                ChaosOpts { p1: Some(side(json!({ "health": 12 }))), p2: Some(json!({ "health": 12 })), ..ChaosOpts::default() },
            );
            hurt.expect_health(P1, 42);
            hurt.expect_health(P2, 12);
            assert_eq!(events_of(&hurt, "healed").len(), 1);

            let mut full = chaos(Some("heal"), ChaosOpts::default());
            full.expect_health(P1, 60);
        }

        #[test]
        fn s8_4_3_10_draws_your_whole_library_and_gains_4_mana_r58() {
            crate::register_all();
            let mut s = chaos(
                Some("draw"),
                ChaosOpts { p1: Some(side(json!({ "library": [RENO, JAMMED, MENACE] }))), ..ChaosOpts::default() },
            );
            assert!(s.pile(P1, "library").is_empty());
            assert_eq!(
                sorted(s.hand(P1).into_iter().map(|card| card.def_id).collect()),
                sorted(vec![MENACE.to_string(), RENO.to_string(), JAMMED.to_string(), MENACE.to_string()]),
            );
            s.expect_mana(P1, 8); // 8 − 4 paid + 4 gained
            assert_eq!(events_of(&s, "drawn").len(), 3);
        }

        #[test]
        fn s8_4_3_10_on_an_empty_library_draws_nothing_and_takes_no_fatigue_r3_r58() {
            crate::register_all();
            let mut s = chaos(Some("draw"), ChaosOpts::default());
            assert!(events_of(&s, "drawn").is_empty());
            s.expect_health(P1, 30);
            assert_eq!(s.state().players.p1.fatigue_count, 0);
            s.expect_mana(P1, 8);
        }

        #[test]
        fn s8_4_4_10_adds_3_random_cards_to_hand_each_costing_0_r60_r65() {
            crate::register_all();
            let s = chaos(Some("add"), ChaosOpts::default());
            let added = events_of(&s, "addedToHand");
            assert_eq!(added.len(), 3);

            let fresh: Vec<CardInstance> = s
                .hand(P1)
                .into_iter()
                .filter(|card| added.iter().any(|event| id_of(event) == card.id))
                .collect();
            assert_eq!(fresh.len(), 3);
            for card in &fresh {
                // The 0 is a `costOverride` on the new instance (R65), and it is the price the play
                // validator reads — not merely a rider nothing consults.
                assert_eq!(card.cost_override, Some(0));
                assert_eq!(effective(&s, card), 0);
                assert!(!crate::card_def(&card.def_id).token);
            }
        }

        #[test]
        fn s8_4_5_10_makes_every_card_in_your_hand_radiant_and_leaves_an_already_radiant_one_alone() {
            crate::register_all();
            let s = chaos(
                Some("radiant"),
                ChaosOpts {
                    p1: Some(json!({ "hand": [CHAOS, MENACE, { "def": RENO, "radiant": true }], "mana": 8 })),
                    ..ChaosOpts::default()
                },
            );
            let hand = s.hand(P1);
            assert_eq!(hand.len(), 2); // #95 left the hand to resolve
            assert!(hand.iter().all(|card| card.radiant));
            // The already-Radiant #53 keeps its flag, which is never unset; both hand cards are cued all
            // the same, because a hidden card's cue must not depend on its face (R177, R97).
            assert_eq!(
                events_of(&s, "radiantSet").iter().map(id_of).collect::<Vec<_>>(),
                hand.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            );
        }

        #[test]
        fn s8_4_6_10_summons_five_radiant_rush_tokens_s7_s_6_6_not_a_bespoke_5_5() {
            crate::register_all();
            let mut s = chaos(Some("tokens"), ChaosOpts::default());
            let tokens = units_of(&s, P1);
            assert_eq!(
                tokens.iter().map(|unit| unit.def_id.clone()).collect::<Vec<_>>(),
                vec![RUSH_TOKEN.to_string(); 5],
            );
            // §7: the printed token is 3/3 and its Radiant face is 6/6. #95 summons the token's own
            // Radiant face with no `statsOverride`, so the stats live in the catalog and nowhere else.
            let token_def = js(&crate::card_def(RUSH_TOKEN));
            assert_eq!(token_def["base"]["attack"], 3);
            assert_eq!(token_def["radiant"]["attack"], 6);
            for token in &tokens {
                assert!(token.radiant);
                assert!(token.stats_override.is_none(), "no bespoke stats any more");
                s.expect_stats(token, json!({ "attack": 6, "health": 6, "maxHealth": 6 }));
                // §7: the Radiant Rush Token prints Rush and Cleave (R275), and #95 grants nothing more.
                assert_eq!(sorted(keywords_of(&s, token)), vec!["Cleave".to_string(), "Rush".to_string()]);
            }
        }

        #[test]
        fn s8_4_6_10_takes_the_zones_that_are_free_and_fizzles_the_rest_r64() {
            crate::register_all();
            let s = chaos(
                Some("tokens"),
                ChaosOpts { p1: Some(side(json!({ "field": [MENACE, MENACE, MENACE] }))), ..ChaosOpts::default() },
            );
            assert_eq!(units_of(&s, P1).iter().filter(|unit| unit.def_id == RUSH_TOKEN).count(), 2);
            // The summons never reach across the board (§3.2).
            assert!(units_of(&s, P2).is_empty());
        }

        #[test]
        fn s8_4_7_10_makes_every_card_in_your_hand_and_library_cost_2_less_floored_at_0_r65_r78() {
            crate::register_all();
            let s = chaos(
                Some("discount"),
                ChaosOpts {
                    p1: Some(json!({ "hand": [CHAOS, MENACE, JAMMED], "library": [RENO], "mana": 8 })),
                    ..ChaosOpts::default()
                },
            );

            let menace = must(s.hand(P1).into_iter().find(|card| card.def_id == MENACE), "#19 in hand");
            let jammed = must(s.hand(P1).into_iter().find(|card| card.def_id == JAMMED), "#36 in hand");
            let reno = must(s.pile(P1, "library").into_iter().next(), "#53 in the library");
            for card in [&menace, &jammed, &reno] {
                assert_eq!(card.cost_mod, -2);
            }

            // The price the validator reads: #19 costs 3 → 1, #36 costs 1 → 0 rather than −1 (R65's floor).
            assert_eq!(effective(&s, &menace), 1);
            assert_eq!(effective(&s, &jammed), 0);
            assert_eq!(effective(&s, &reno), 1);
            assert_eq!(events_of(&s, "costChanged").len(), 3);
        }

        #[test]
        fn s8_4_7_10_is_your_hand_and_library_it_changes_those_cards_not_the_player_not_p2() {
            crate::register_all();
            let mut s = chaos(
                Some("discount"),
                ChaosOpts {
                    p1: Some(side(json!({ "library": [RENO] }))),
                    p2: Some(json!({ "hand": [MENACE], "library": [RENO] })),
                    ..ChaosOpts::default()
                },
            );

            // Not the player: no `costDiscount` modifier is created, so a card that arrives later — a
            // draw, an add — pays full price. The −2 rides on the instances that were there (R78).
            let discounts: Vec<Value> = js(&s.state().players.p1.mods)
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|modifier| modifier["kind"] == "costDiscount")
                .collect();
            assert_eq!(discounts, Vec::<Value>::new());
            // …and the discount travels WITH the card it landed on, out of the library and into the hand.
            s.start_turn();
            let drawn = must(s.hand(P1).into_iter().find(|card| card.def_id == RENO), "the drawn #53");
            assert_eq!(drawn.cost_mod, -2);
            assert_eq!(effective(&s, &drawn), 1); // 3 − 2

            // "your hand and library": the opponent's cards are untouched.
            for card in s.hand(P2).into_iter().chain(s.pile(P2, "library")) {
                assert_eq!(card.cost_mod, 0);
            }
        }

        #[test]
        fn s8_4_8_10_summons_a_chaos_golem_s7_s_10_10_token_of_index_95_1() {
            crate::register_all();
            let mut s = chaos(Some("golem"), ChaosOpts::default());
            assert_eq!(
                units_of(&s, P1).into_iter().map(|unit| unit.def_id).collect::<Vec<_>>(),
                vec![GOLEM.to_string()],
            );
            s.expect_stats(GOLEM, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
        }

        #[test]
        fn s8_4_9_10_summons_5_random_field_spells_or_traps_into_your_backrow_traps_face_down_r33() {
            crate::register_all();
            let s = chaos(Some("backrow"), ChaosOpts::default());
            let summoned = events_of(&s, "summoned");
            assert_eq!(summoned.len(), 5);
            assert!(summoned.iter().all(|event| event["row"] == "backrow"));
            assert_eq!(
                summoned.iter().map(|event| event["lane"].clone()).collect::<Vec<_>>(),
                vec![json!(1), json!(2), json!(3), json!(4), json!(5)],
            );

            let placed = backrow_of(&s, P1);
            assert_eq!(placed.len(), 5);
            for card in &placed {
                let def = js(&crate::card_def(&card.def_id));
                // "Field Spells or Traps (Field Traps included)".
                assert!([json!("Field Spell"), json!("Trap"), json!("Field Trap")].contains(&def["type"]));
                // §3.2, R33: a Field Spell is public and a Trap or Field Trap is face-down.
                assert_eq!(card.face_up == Some(true), def["type"] == "Field Spell");
            }

            // §10.8: the opponent is told a face-down zone is occupied and nothing more.
            let as_seen_by_p2 = js(&s.view(P2))["opponent"]["backrow"].clone();
            for (at, card) in placed.iter().enumerate() {
                let seen = &as_seen_by_p2[at];
                if js(&crate::card_def(&card.def_id))["type"] == "Field Spell" {
                    assert!(matches_object(seen, &json!({ "faceDown": false, "defId": card.def_id })));
                } else {
                    assert_eq!(seen, &json!({ "faceDown": true, "cost": 1 }));
                }
            }
        }

        #[test]
        fn s8_4_10_10_casts_a_random_call_to_chaos_free_counted_as_a_play_its_script_run_r70() {
            crate::register_all();
            let mut s = chaos(Some("recast"), ChaosOpts::default());
            let plays = chaos_plays(&s);
            assert_eq!(plays.len(), 2);
            // R70: "a cast is free … with cost paid 0".
            assert_eq!(plays[0]["costPaid"], 4);
            assert_eq!(plays[1]["costPaid"], 0);
            // R70: "counts as a play for every rule that counts or reacts to plays".
            assert_eq!(s.state().counters.played, 2);
            assert_eq!(s.state().players.p1.turn_log.cards_played, 2);
            assert!(s.state().players.p1.turn_log.played_ids.contains(&id_of(&plays[1])));

            // §5.1's exception: the pool for "a random Call to Chaos" includes #95 itself, and the card
            // cast is the BASE form however the caster was rolled (R28).
            let cast = s.card(id_of(must(plays.get(1), "the cast card's event")).as_str()).clone();
            assert_eq!(cast.def_id, CHAOS);
            assert!(!cast.radiant);
            // It was the chain's first cast while it resolved, and it has landed as the printed card
            // again (R215): the chain's count stays with the chain, never with the card a graveyard holds.
            assert!(cast.memory.get(subsystems::CHAOS_CHAIN_KEY).is_none());

            // R87: a card cast from no zone goes to the caster's graveyard when it resolves, which is
            // what feeds Gravedigger and Reminisce down a long chain.
            s.expect_in_zone(&cast, "graveyard");
            assert_eq!(s.pile(P1, "graveyard").iter().filter(|card| card.def_id == CHAOS).count(), 2);
        }
    }

    // R4's hand cap and R28's chain cap.

    mod n95_call_to_chaos_the_two_caps {
        use super::*;

        #[test]
        fn r4_a_full_hand_takes_what_it_can_and_burns_the_rest() {
            crate::register_all();
            // §2.4's cap is 10. The hand holds #95 plus nine others, so it is back to nine once #95 is
            // resolving: one of the three added cards fits and two are burned to the graveyard.
            let mut hand = vec![json!(CHAOS)];
            hand.extend(std::iter::repeat_n(json!(MENACE), 9));
            let s = chaos(
                Some("add"),
                ChaosOpts { p1: Some(json!({ "hand": hand, "mana": 8 })), ..ChaosOpts::default() },
            );
            assert_eq!(s.hand(P1).len(), 10);
            assert_eq!(events_of(&s, "addedToHand").len(), 1);
            assert_eq!(events_of(&s, "burned").len(), 2);
        }

        #[test]
        fn r28_the_chain_is_a_hard_stop_at_the_cap_the_recursion_resolves_into_nothing() {
            crate::register_all();
            let mut s = chaos(
                Some("recast"),
                ChaosOpts { chain: Some(CALL_TO_CHAOS_CHAIN_CAP), ..ChaosOpts::default() },
            );
            // The played card is already at the cap, so its roll of the recursion casts nothing at all —
            // and no substitute effect is rolled in its place (R87).
            assert_eq!(chaos_plays(&s).len(), 1);
            assert_eq!(s.state().counters.played, 1);
            s.expect_in_zone(CHAOS, "graveyard");
        }

        #[test]
        fn r28_one_below_the_cap_casts_exactly_one_more_and_that_cast_is_the_last() {
            crate::register_all();
            let mut s = chaos(
                Some("recast"),
                ChaosOpts { chain: Some(CALL_TO_CHAOS_CHAIN_CAP - 1), ..ChaosOpts::default() },
            );
            let plays = chaos_plays(&s);
            assert_eq!(plays.len(), 2);
            // The card it cast is AT the cap, so whatever that one rolled, it cast nothing further — the
            // two plays above are the whole chain — and it landed as the printed card again (R215).
            let cast = s.card(id_of(must(plays.get(1), "the cast card's event")).as_str()).clone();
            s.expect_in_zone(&cast, "graveyard");
            assert!(cast.memory.get(subsystems::CHAOS_CHAIN_KEY).is_none());
        }

        #[test]
        fn r28_r380_the_recursion_s_pool_is_every_call_to_chaos_of_every_set_both_editions() {
            crate::register_all();
            let pool = query_ids(json!({ "tags": ["Call to Chaos"] }));
            assert!(pool.contains(&CHAOS.to_string()));
            assert!(pool.contains(&"classicplus-073".to_string()));
            assert!(pool.iter().all(|id| {
                js(&crate::card_def(id))["tags"]
                    .as_array()
                    .is_some_and(|tags| tags.contains(&json!("Call to Chaos")))
            }));
        }

        #[test]
        fn r28_the_counter_is_instance_state_so_two_calls_in_one_turn_do_not_share_it() {
            crate::register_all();
            let s = chaos(
                Some("recast"),
                ChaosOpts { p1: Some(json!({ "hand": [CHAOS, CHAOS], "mana": 8 })), ..ChaosOpts::default() },
            );
            assert_eq!(chaos_plays(&s).len(), 2);
            // The second copy still carries no counter: nothing global was spent on the first chain.
            let second = must(s.hand(P1).into_iter().next(), "the second #95");
            assert_eq!(second.def_id, CHAOS);
            assert!(second.memory.get(subsystems::CHAOS_CHAIN_KEY).is_none());
        }
    }

    // The radiant face (§8.4, R423: "Three different random effects, resolved in the order listed").

    /// The labels `chaosRolled` named, in order (R436).
    fn announced(s: &Scenario) -> Vec<Vec<String>> {
        events_of(s, "chaosRolled")
            .iter()
            .map(|event| {
                event["effects"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|label| label.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .collect()
    }

    fn labels_of(names: &[String]) -> Vec<String> {
        names
            .iter()
            .map(|name| {
                must(
                    subsystems::CHAOS_EFFECTS.iter().find(|effect| effect.name == name.as_str()),
                    name,
                )
                .label
                .to_string()
            })
            .collect()
    }

    mod n95_call_to_chaos_radiant {
        use super::*;

        #[test]
        fn r423_rolls_three_different_effects_of_the_ten_and_resolves_them_in_the_order_the_list_writes_them() {
            crate::register_all();
            let order: Vec<String> = subsystems::CHAOS_EFFECTS.iter().map(|effect| effect.name.to_string()).collect();
            let mut reached: IndexSet<String> = IndexSet::new();
            for cursor in 0..60 {
                // At the cap, so a rolled recursion casts nothing and each play is this card alone (R87).
                let mut s = scenario(json!({
                    "seed": SEED,
                    "p1": { "hand": [{ "def": CHAOS, "radiant": true }, MENACE], "mana": 8 },
                }));
                set_memory(&mut s, CHAOS, subsystems::CHAOS_CHAIN_KEY, json!(CALL_TO_CHAOS_CHAIN_CAP));
                s.state_mut().rng_cursor = cursor;
                s.play(CHAOS, json!({}));

                let rolled = radiant_roll_at(cursor);
                assert_eq!(rolled.iter().collect::<IndexSet<_>>().len(), 3);
                let at: Vec<usize> = rolled
                    .iter()
                    .map(|name| must(order.iter().position(|entry| entry == name), name))
                    .collect();
                let mut in_order = at.clone();
                in_order.sort();
                assert_eq!(at, in_order);
                // R436: the card itself named exactly those three, in that order.
                assert_eq!(announced(&s), vec![labels_of(&rolled)]);
                for name in rolled {
                    reached.insert(name);
                }
            }
            assert_eq!(reached.len(), 10);
        }

        #[test]
        fn r423_the_recursion_is_not_guaranteed_any_more_a_roll_without_it_casts_nothing() {
            crate::register_all();
            let cursor = radiant_cursor_where(|rolled| !has(rolled, "recast"), "no recursion");
            let s = chaos(None, ChaosOpts { radiant_cursor: Some(cursor), ..ChaosOpts::default() });
            assert_eq!(chaos_plays(&s).len(), 1);
            assert_eq!(s.state().counters.played, 1);
        }

        #[test]
        fn r87_r423_a_rolled_recursion_resolves_where_the_list_puts_it_last_the_golem_is_summoned_before_the_chain_is_cast() {
            crate::register_all();
            let cursor = radiant_cursor_where(
                |rolled| has(rolled, "golem") && has(rolled, "recast"),
                "the Golem and the recursion",
            );
            let s = chaos(
                None,
                ChaosOpts {
                    radiant_cursor: Some(cursor),
                    chain: Some(CALL_TO_CHAOS_CHAIN_CAP - 1),
                    ..ChaosOpts::default()
                },
            );
            let all = events_json(&s);
            let cast_at = must(
                all.iter()
                    .enumerate()
                    .filter(|(_, event)| event["type"] == "cardPlayed" && event["defId"] == CHAOS)
                    .map(|(at, _)| at)
                    .nth(1),
                "the cast #95",
            );
            let golem_at = all
                .iter()
                .position(|event| event["type"] == "summoned" && event["defId"] == GOLEM)
                .map_or(-1, |at| at as isize);
            assert!(golem_at >= 0);
            assert!((cast_at as isize) > golem_at);
        }

        #[test]
        fn r28_r87_at_the_cap_a_rolled_recursion_resolves_into_nothing_and_the_other_two_still_run() {
            crate::register_all();
            let cursor = radiant_cursor_where(
                |rolled| has(rolled, "golem") && has(rolled, "recast"),
                "the Golem and the recursion",
            );
            let s = chaos(
                None,
                ChaosOpts {
                    radiant_cursor: Some(cursor),
                    chain: Some(CALL_TO_CHAOS_CHAIN_CAP),
                    ..ChaosOpts::default()
                },
            );
            assert_eq!(chaos_plays(&s).len(), 1);
            assert!(units_of(&s, P1).iter().any(|unit| unit.def_id == GOLEM));
            // R436: the recursion was rolled, and is named, though it did nothing.
            assert!(announced(&s)[0].contains(&"Cast a random Call to Chaos".to_string()));
        }

        #[test]
        fn s5_2_a_base_copy_rolls_one_effect_however_deep_the_chain_is() {
            crate::register_all();
            // Both faces call the same subsystem; the `radiant` argument each face passes is the whole
            // difference, which is why the base face can roll any single one of the ten.
            let mut s = chaos(
                Some("heal"),
                ChaosOpts { chain: Some(CALL_TO_CHAOS_CHAIN_CAP - 1), ..ChaosOpts::default() },
            );
            assert_eq!(chaos_plays(&s).len(), 1);
            s.expect_health(P1, 60);
            assert_eq!(announced(&s), vec![vec!["Heal your hero 30".to_string()]]);
        }
    }

    // R436: what was rolled, named to both players.

    mod n95_call_to_chaos_r436_names_what_it_rolled_to_both_players {
        use super::*;

        #[test]
        fn r436_both_seats_read_the_same_chaosrolled_the_card_and_the_labels_unredacted_before_the_first_effect_lands() {
            crate::register_all();
            let s = chaos(Some("golem"), ChaosOpts::default());
            let event = must(events_of(&s, "chaosRolled").into_iter().next(), "the announcement");
            assert!(matches_object(
                &event,
                &json!({ "player": "p1", "defId": CHAOS, "effects": ["Summon a Chaos Golem"] }),
            ));
            let all = events_json(&s);
            let at = must(all.iter().position(|entry| entry == &event), "the announcement's place");
            let summon_at = all
                .iter()
                .position(|entry| entry["type"] == "summoned" && entry["defId"] == GOLEM)
                .map_or(-1, |found| found as isize);
            assert!((at as isize) < summon_at);

            for viewer in [P1, P2] {
                let seen: Vec<Value> = js(&s.view(viewer))["events"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|entry| entry["type"] == "chaosRolled")
                    .collect();
                assert_eq!(seen, vec![event.clone()]);
            }
        }

        #[test]
        fn r436_every_label_is_a_clause_of_the_card_s_printed_list() {
            crate::register_all();
            let text = js(&crate::card_def(CHAOS))["base"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_lowercase();
            for effect in subsystems::CHAOS_EFFECTS.iter() {
                assert!(text.contains(&effect.label.to_string().to_lowercase()));
            }
        }

        #[test]
        fn r436_hinder_s_random_discard_pauses_nothing_the_whole_deck_draw_runs_through_announced_once() {
            crate::register_all();
            // A Radiant roll with the whole-deck draw and a later effect, the Golem. The library's Hinder
            // is cast by that draw and discards at random (R682, R431: no prompt), so the draw and the
            // Golem behind it run through in one pass.
            let cursor = radiant_cursor_where(
                |rolled| has(rolled, "draw") && has(rolled, "golem") && !has(rolled, "recast") && !has(rolled, "tokens"),
                "the draw and the Golem, with room for it",
            );
            let mut s = chaos(
                None,
                ChaosOpts {
                    radiant_cursor: Some(cursor),
                    p1: Some(json!({
                        "hand": [CHAOS, MENACE, JAMMED],
                        "mana": 8,
                        "library": ["core-021", RENO, RENO],
                    })),
                    ..ChaosOpts::default()
                },
            );
            // No question was asked at any point: with no prompt open the roll cannot have paused (R113).
            assert!(s.state().pending.is_none());
            assert_eq!(events_of(&s, "chaosRolled").len(), 1);
            // The draw completed: the library is empty and both Renos are in hand.
            assert!(s.pile(P1, "library").is_empty());
            assert_eq!(s.hand(P1).iter().filter(|card| card.def_id == RENO).count(), 2);
            // The Hinder was cast by the draw and is in the graveyard, and its random discard took exactly
            // one of the two hand cards: the other is still in hand.
            s.expect_in_zone("core-021", "graveyard");
            let menace = s.hand(P1).iter().any(|card| card.def_id == MENACE);
            let jammed = s.hand(P1).iter().any(|card| card.def_id == JAMMED);
            assert!(menace != jammed);
            s.expect_in_zone(if menace { JAMMED } else { MENACE }, "graveyard");
            // The Golem behind the draw resolved too, and nothing was rolled or announced again.
            assert!(events_of(&s, "summoned").iter().any(|event| event["defId"] == GOLEM));
        }
    }

    // #95.1 Chaos Golem (§8.4, §7, R11).

    mod n95_1_chaos_golem {
        use super::*;

        #[test]
        fn s7_the_base_face_is_a_10_10_with_rush_lifesteal_divine_shield_and_first_strike() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "field": [GOLEM], "hand": [MENACE] } }));
            let golem = must(s.unit(P1, 1), "p1's Chaos Golem");
            s.expect_stats(&golem, json!({ "attack": 10, "health": 10, "maxHealth": 10 }));
            assert_eq!(
                sorted(keywords_of(&s, &golem)),
                vec!["Divine Shield".to_string(), "First Strike".to_string(), "Lifesteal".to_string(), "Rush".to_string()],
            );
        }

        #[test]
        fn r275_the_radiant_face_is_a_20_20_with_charge_in_rush_s_place() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [MENACE] },
                "p2": { "field": [{ "def": GOLEM, "radiant": true }] },
            }));
            let golem = must(s.unit(P2, 1), "p2's Radiant Chaos Golem");
            s.expect_stats(&golem, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));
            assert_eq!(
                sorted(keywords_of(&s, &golem)),
                vec!["Charge".to_string(), "Divine Shield".to_string(), "First Strike".to_string(), "Lifesteal".to_string()],
            );

            // The difference is data on the catalog's two faces; there is no script on either (§10.4
            // layer 1 reads the stats and keywords straight off the face the flag picks).
            let golem_scripts = crate::scripts::core::c095_1_chaos_golem::script();
            assert_eq!(members(&golem_scripts.radiant), members(&golem_scripts.base));
            assert!(members(&golem_scripts.base).is_empty());
            let def = js(&crate::card_def(GOLEM));
            assert_ne!(def["radiant"], def["base"]);
        }

        #[test]
        fn r275_a_chaos_golem_made_radiant_the_turn_it_arrives_is_a_20_20_that_may_charge_the_hero() {
            crate::register_all();
            // #95 summons a base Golem into lane 1; a Saintess in lane 5 then dies to True Strike and her
            // Death makes it Radiant on the field (R22: the base layer swaps, keywords apply at once).
            let mut s = chaos(
                Some("golem"),
                ChaosOpts {
                    p1: Some(side(json!({
                        "hand": [CHAOS, TRUE_STRIKE, MENACE],
                        "field": [{ "def": SAINTESS, "lane": 5 }],
                    }))),
                    p2: Some(json!({ "hand": [MENACE], "health": 30 })),
                    ..ChaosOpts::default()
                },
            );
            let golem = must(s.unit(P1, 1), "the summoned Chaos Golem");
            assert_eq!(golem.def_id, GOLEM);
            assert!(!golem.radiant);
            // Rush alone: the hero is out of reach on the turn it arrived.
            s.expect_refused(|s| s.attack(&golem, "hero"));

            let saintess = must(s.unit(P1, 5), "the Saintess");
            s.play(TRUE_STRIKE, json!({ "targets": [{ "pick": "instance", "instanceId": saintess.id }] }));

            assert!(s.card(&golem).radiant);
            s.expect_stats(&golem, json!({ "attack": 20, "health": 20, "maxHealth": 20 }));
            let now = s.card(&golem).clone();
            assert!(keywords_of(&s, &now).contains(&"Charge".to_string()));
            assert!(!keywords_of(&s, &now).contains(&"Rush".to_string()));

            // Charge lifts summoning sickness for the hero too.
            s.attack(&golem, "hero");
            s.expect_health(P2, 10);
        }

        #[test]
        fn r275_the_radiant_golem_s_lifesteal_heals_what_its_20_dealt() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": GOLEM, "radiant": true }], "hand": [MENACE], "health": 5 },
                "p2": { "health": 30 },
            }));
            let golem = must(s.unit(P1, 1), "the Radiant Golem");

            s.attack(&golem, "hero");
            s.expect_health(P2, 10); // 30 − 20
            s.expect_health(P1, 25); // 5 + the 20 dealt
        }

        #[test]
        fn s6_1_rush_the_turn_it_arrives_it_may_attack_a_unit_but_not_the_hero() {
            crate::register_all();
            let mut s = chaos(
                Some("golem"),
                ChaosOpts { p2: Some(json!({ "field": [MENACE] })), ..ChaosOpts::default() },
            );
            let golem = must(s.unit(P1, 1), "the summoned Chaos Golem");
            assert_eq!(golem.def_id, GOLEM);

            s.expect_refused(|s| s.attack(&golem, "hero"));
            let victim = must(s.unit(P2, 1), "the enemy #19");
            s.attack(&golem, &victim);
            // §4.3: #19 is 9/9, so First Strike kills it before it strikes back and the shield is intact.
            s.expect_in_zone(&victim, "graveyard");
            assert!(s.card(&golem).divine_shield_spent.is_none());
            s.expect_stats(&golem, json!({ "health": 10 }));
        }

        #[test]
        fn s4_4_step_1_divine_shield_negates_the_whole_first_hit_and_is_spent() {
            crate::register_all();
            // A radiant #91 Fed Fauci is 2/12, the one enemy body that survives a 10-attack First Strike
            // and so gets to strike back into the shield.
            let mut s = scenario(json!({
                "p1": { "field": [GOLEM], "hand": [MENACE], "health": 15 },
                "p2": { "field": [{ "def": FAUCI, "radiant": true }] },
            }));
            let golem = must(s.unit(P1, 1), "the Golem");
            let fauci = must(s.unit(P2, 1), "the radiant #91");

            s.attack(&golem, &fauci);
            s.expect_stats(&fauci, json!({ "health": 2 })); // 12 − 10
            s.expect_stats(&golem, json!({ "health": 10 })); // the 2 back was negated whole
            assert_eq!(s.card(&golem).divine_shield_spent, Some(true));
            let now = s.card(&golem).clone();
            assert!(!keywords_of(&s, &now).contains(&"Divine Shield".to_string()));
            assert_eq!(events_of(&s, "divineShieldLost").len(), 1);
        }

        #[test]
        fn s4_4_step_8_lifesteal_heals_its_controller_s_hero_by_the_amount_dealt() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "field": [GOLEM], "hand": [MENACE], "health": 15 },
                "p2": { "health": 30 },
            }));
            let golem = must(s.unit(P1, 1), "the Golem");

            s.attack(&golem, "hero");
            s.expect_health(P2, 20); // 30 − 10
            s.expect_health(P1, 25); // 15 + the 10 dealt
            assert_eq!(events_of(&s, "healed").len(), 1);
        }

        #[test]
        fn r11_it_is_a_unit_token_dying_is_ceasing_to_exist_with_no_graveyard_and_no_card_to_eat() {
            crate::register_all();
            // `divineShieldSpent` is the state one hit leaves behind (§4.4 step 1), set here so a single
            // exchange can be lethal; a radiant #19 is 18/18, which kills an unshielded 10/10.
            let mut s = scenario(json!({
                "p1": { "field": [GOLEM], "hand": [MENACE] },
                "p2": { "field": [big_menace()] },
            }));
            let golem = must(s.unit(P1, 1), "the Golem");
            find_instance_mut(s.state_mut(), &golem.id)
                .expect("the Golem is on the field")
                .divine_shield_spent = Some(true);
            assert!(crate::card_def(GOLEM).token);

            let target = must(s.unit(P2, 1), "the radiant #19");
            s.attack(&golem, &target);
            // §3.2, R11: it never enters a graveyard, so #89 Corpse Eater can never feed on one either.
            s.expect_in_zone(&golem, "gone");
            assert!(!s.pile(P1, "graveyard").iter().any(|card| card.def_id == GOLEM));
            assert!(!s.pile(P1, "exile").iter().any(|card| card.def_id == GOLEM));
        }

        #[test]
        fn s5_1_it_is_a_token_so_no_random_pool_and_no_discover_can_reach_it() {
            crate::register_all();
            assert!(!query_ids(json!({})).contains(&GOLEM.to_string()));
            assert!(!query_ids(json!({ "type": "Unit" })).contains(&GOLEM.to_string()));
            // …which is what makes #99 Craft a Card unable to craft one.
            assert!(!query_ids(json!({ "type": "Unit", "cost": 4 })).contains(&GOLEM.to_string()));
        }
    }

    mod n95_s_printed_text_round_10_of_the_polish_4_edge_case_hunt {
        use super::*;

        #[test]
        fn names_the_radiant_rush_tokens_it_summons_not_the_5_5s_issue_n1_took_out_s8_4_n95_s7() {
            crate::register_all();
            // §8.4 #95's base effect: "summon five Radiant Rush Tokens", and §7: it summons the token's
            // own Radiant face (6/6), not a bespoke 5/5. The catalog's printed text, which the web
            // renders, says so too.
            let text = js(&crate::card_def(CHAOS))["base"]["text"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            assert!(!text.contains("5/5"));
            // The count is written as a number: "summon 5 Radiant Rush Tokens".
            assert!(text.contains("summon 5 Radiant Rush Tokens"));
        }
    }
}
