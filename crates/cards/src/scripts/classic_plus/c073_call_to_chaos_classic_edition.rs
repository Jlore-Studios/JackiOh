//! C+ #73 Call to Chaos (Classic+ Edition) (SPEC §8.7 row 73, B7, R28, R87, R423, R436). (4) Spell, Call
//! to Chaos, Legendary.
//!   Base:    "One random effect: Add 5 random Fruits to your hand, which cost (0); add 3 random Books to
//!            your hand, which cost (0); destroy all enemy permanents; add 3 random Classic cards to your
//!            hand, which cost (0); Buff every card in your hand and deck twice; fuse a random card
//!            into each card in your deck, each keeping its cost; Nerf every card on your opponent's
//!            field and in their hand three times; summon a Classic Golem; replace your deck with random
//!            Call to Chaos cards, which cost (0); cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem with this edition's table (`CHAOS_PLUS_EFFECTS`): the roll, the announcement both
//! players read (R436), the chain cap counting casts of either edition (R28) and the order the Radiant's
//! three resolve in (R423) are the one rule both editions share.

use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-073";

pub fn script() -> CardScripts {
    // §8.7: "One random effect" of the ten.
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_PLUS_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.7, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_PLUS_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// BUILD M9 Classic+ row C+ 73, one test per entry of the ten: the Fruit pool (R382), Upgrade twice (R386),
// fuse with Immutable skipped (R23), the Golem (C+ #73.1), a deck of either edition (R387), and recast,
// where the chain counts both editions (R28) and a roll at the cap resolves into nothing (R87). Both
// players read which effects rolled (R436); deck and hand changes stay hidden (R97, R177, R311); fused
// deck cards survive JSON (R179); the Radiant resolves its three in the list's order (R423).
// The roll is the play's first rng draw: setting `state.rng_cursor` pins it, and `cursor_for` finds the
// cursor of each entry by asking the engine's own roll with this card's table.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CHAOS: &str = "classicplus-073";
    const CORE_CHAOS: &str = "core-095";
    const GOLEM: &str = "classicplus-073-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const TIMMY: &str = "core-011"; // (1) Unit
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const BEAR: &str = "core-060"; // (1) Trap
    const TWINSPELL: &str = "core-079"; // (2) Field Spell
    const HEROIC_POWER: &str = "core-098"; // Field Spell, Indestructible
    const BIG_FELINOR: &str = "core-043"; // (4) Unit
    const FIENDER: &str = "core-092"; // (2) Unit, Stack
    const HINDER: &str = "core-021"; // (0) Spell, cast on draw; base face: discard 1 (R431)

    const SEED: &str = "chaos-plus";
    const CURSOR_SEARCH: u32 = 800;

    use crate::js;

    /// The names of the entries rolled at `cursor` of `seed`, with this card's table.
    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_PLUS_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_for(name: &str) -> u32 {
        for cursor in 0..CURSOR_SEARCH {
            if rolled_names(SEED, cursor, false).first() == Some(&name) {
                return cursor;
            }
        }
        panic!("no cursor rolls {name}");
    }

    fn radiant_cursor_where(accept: impl Fn(&[&str]) -> bool) -> u32 {
        for cursor in 0..CURSOR_SEARCH * 4 {
            if accept(&rolled_names(SEED, cursor, true)) {
                return cursor;
            }
        }
        panic!("no radiant cursor");
    }

    #[derive(Default)]
    struct Opts {
        p1: Option<Value>,
        p2: Option<Value>,
        radiant_cursor: Option<u32>,
        chain: Option<i32>,
    }

    fn spread(target: &mut Value, extra: Option<Value>) {
        if let Some(Value::Object(extra)) = extra {
            for (key, value) in extra {
                target[key.as_str()] = value;
            }
        }
    }

    /// p1 plays C+ #73 with its roll pinned; a spare card in hand keeps §2.5's auto-end away.
    fn chaos(entry: Option<&str>, opts: Opts) -> Scenario {
        let radiant_face = opts.radiant_cursor.is_some();
        let mut p1 = json!({ "hand": [{ "def": CHAOS, "radiant": radiant_face }, VANILLA], "mana": 8 });
        spread(&mut p1, opts.p1);
        let mut p2 = json!({ "hand": [VANILLA] });
        spread(&mut p2, opts.p2);
        let mut s = scenario(json!({ "seed": SEED, "p1": p1, "p2": p2 }));
        if let Some(chain) = opts.chain {
            set_chain(&mut s, chain);
        }
        s.state_mut().rng_cursor = opts.radiant_cursor.unwrap_or_else(|| cursor_for(entry.unwrap_or("fruits")));
        s.play(CHAOS, json!({}));
        s
    }

    fn set_chain(s: &mut Scenario, chain: i32) {
        let id = s.card(CHAOS).id.clone();
        find_instance_mut(s.state_mut(), &id)
            .expect("Call to Chaos in hand")
            .memory
            .insert(subsystems::CHAOS_CHAIN_KEY.to_string(), json!(chain));
    }

    /// The events of one type, as their JSON.
    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.events().iter().filter(|event| event.event_type().as_str() == kind).map(js).collect()
    }

    fn added_to(s: &Scenario) -> Vec<CardInstance> {
        let ids: IndexSet<String> = events_of(s, "addedToHand")
            .iter()
            .map(|event| event["instanceId"].as_str().unwrap_or("").to_string())
            .collect();
        s.hand(P1).into_iter().filter(|card| ids.contains(&card.id)).collect()
    }

    fn deck(cards: Value) -> Value {
        json!({ "hand": [{ "def": CHAOS }, VANILLA], "library": cards, "mana": 8 })
    }

    fn types_of(s: &Scenario) -> Vec<&'static str> {
        s.events().iter().map(|event| event.event_type().as_str()).collect()
    }

    /// −1 when absent; a negative `from` counts back from the end.
    fn index_of_from(order: &[&str], kind: &str, from: i64) -> i64 {
        let len = order.len() as i64;
        let start = (if from < 0 { (len + from).max(0) } else { from }) as usize;
        order
            .iter()
            .enumerate()
            .skip(start)
            .find(|(_, entry)| **entry == kind)
            .map_or(-1, |(at, _)| at as i64)
    }

    fn index_of(order: &[&str], kind: &str) -> i64 {
        index_of_from(order, kind, 0)
    }

    /// The fields this script sets, in `Script`'s order.
    fn object_keys(script: &Script) -> Vec<&'static str> {
        let mut keys = Vec::new();
        let mut set = |key: &'static str, present: bool| {
            if present {
                keys.push(key);
            }
        };
        set("cost", script.cost.is_some());
        set("cry", script.cry.is_some());
        set("death", script.death.is_some());
        set("startOfGame", script.start_of_game.is_some());
        set("resume", !script.resume.is_empty());
        set("delayed", script.delayed.is_some());
        set("setStat", script.set_stat.is_some());
        set("startOfTurn", script.start_of_turn.is_some());
        set("endOfTurn", script.end_of_turn.is_some());
        set("aura", script.aura.is_some());
        set("triggers", !script.triggers.is_empty());
        set("onPlayHook", script.on_play_hook.is_some());
        set("handTriggers", !script.hand_triggers.is_empty());
        set("staticFlags", script.static_flags.is_some());
        set("targets", !script.targets.is_empty());
        set("modes", !script.modes.is_empty());
        set("conditionMet", script.condition_met.is_some());
        set("preview", script.preview.is_some());
        set("activations", !script.activations.is_empty());
        set("targetChecks", !script.target_checks.is_empty());
        set("costAura", script.cost_aura.is_some());
        set("graveyardPlay", script.graveyard_play.is_some());
        set("targetingDiscards", script.targeting_discards.is_some());
        set("recordsPlayAs", script.records_play_as.is_some());
        set("drawLimit", script.draw_limit.is_some());
        set("replacements", !script.replacements.is_empty());
        set("heroGuard", script.hero_guard.is_some());
        set("conditionalKeywords", script.conditional_keywords.is_some());
        set("afterAttack", script.after_attack.is_some());
        set("plagueMultiplier", script.plague_multiplier.is_some());
        set("deckTriggers", !script.deck_triggers.is_empty());
        set("graveyardTriggers", !script.graveyard_triggers.is_empty());
        set("quests", script.quests.is_some());
        set("tributeWhen", script.tribute_when.is_some());
        set("wouldCounter", script.would_counter.is_some());
        set("startOfOpponentTurn", script.start_of_opponent_turn.is_some());
        keys
    }

    #[test]
    fn is_a_4_legendary_spell_of_the_call_to_chaos_tag_and_both_faces_hang_the_card_off_cry() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(js(&def.cost), json!(4));
        assert_eq!(def.type_, CardType::Spell);
        assert_eq!(js(&def.tags), json!(["Call to Chaos"]));
        assert_eq!(def.rarity, Rarity::Legendary);
        let CardScripts { base, radiant } = script();
        assert_eq!(object_keys(&base), vec!["cry"]);
        assert_eq!(object_keys(&radiant), vec!["cry"]);
    }

    #[test]
    fn r436_each_entry_s_label_is_a_clause_of_the_card_s_printed_list_in_the_printed_order() {
        crate::register_all();
        let def = crate::card_def(ID);
        let text = def.base.text.to_lowercase();
        let mut from = 0;
        for entry in subsystems::CHAOS_PLUS_EFFECTS {
            let label = entry.label.to_lowercase();
            let at = text[from..].find(&label).map(|at| at + from);
            assert!(at.is_some(), "{}", entry.label);
            from = at.unwrap_or(from);
        }
        let last = subsystems::CHAOS_PLUS_EFFECTS.last().map(|entry| entry.label.to_lowercase()).unwrap_or_default();
        assert!(def.radiant.text.to_lowercase().contains(&last));
    }

    #[test]
    fn s9_3_the_roll_is_the_play_s_first_rng_draw_a_pinned_cursor_rolls_the_classic_golem_the_one_entry_nothing_else_makes() {
        crate::register_all();
        let mut s = chaos(Some("golem"), Opts::default());
        assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
        s.expect_in_zone(CHAOS, "graveyard");
        s.expect_mana(P1, 4);
    }

    mod base_the_ten_entries {
        use super::*;

        #[test]
        fn r382_1_10_adds_5_random_fruit_pool_cards_that_cost_0_the_grapes_in_the_pool() {
            crate::register_all();
            let s = chaos(Some("fruits"), Opts::default());
            let added = added_to(&s);
            assert_eq!(added.len(), 5);
            for card in &added {
                assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Fruit));
                assert_eq!(card.cost_override, Some(0));
                assert_eq!(jackioh_engine::mana::effective_cost(s.state(), card, Default::default()), 0);
            }
        }

        #[test]
        fn r380_2_10_adds_3_random_non_token_books_that_cost_0() {
            crate::register_all();
            let s = chaos(Some("books"), Opts::default());
            let added = added_to(&s);
            assert_eq!(added.len(), 3);
            for card in &added {
                let def = crate::card_def(&card.def_id);
                assert!(def.tags.contains(&Tag::Book));
                assert!(!def.token);
                assert_eq!(card.cost_override, Some(0));
            }
        }

        #[test]
        fn r46_3_10_destroys_every_enemy_permanent_face_down_ones_too_an_indestructible_one_stays_yours_stay() {
            crate::register_all();
            let s = chaos(
                Some("destroy"),
                Opts {
                    p1: Some(json!({ "hand": [{ "def": CHAOS }, VANILLA], "field": [TIMMY], "mana": 8 })),
                    p2: Some(json!({
                        "field": [MENACE, VANILLA],
                        "backrow": [
                            { "def": BEAR, "faceUp": false },
                            { "def": TWINSPELL, "faceUp": true },
                            { "def": HEROIC_POWER, "faceUp": true }
                        ]
                    })),
                    ..Opts::default()
                },
            );
            assert!([1, 2].iter().all(|lane| s.unit(P2, *lane).is_none()));
            assert!([1, 2].iter().all(|lane| s.backrow(P2, *lane).is_none()));
            assert_eq!(s.backrow(P2, 3).map(|card| card.def_id), Some(HEROIC_POWER.to_string()));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(TIMMY.to_string()));
        }

        #[test]
        fn s2_4_r4_1_10_into_a_nearly_full_hand_what_does_not_fit_is_burned() {
            crate::register_all();
            let mut hand = vec![json!({ "def": CHAOS })];
            hand.extend((0..HAND_CAP - 3).map(|_| json!(VANILLA)));
            let s = chaos(Some("fruits"), Opts { p1: Some(json!({ "hand": hand, "mana": 8 })), ..Opts::default() });
            assert_eq!(s.hand(P1).len(), HAND_CAP as usize);
            assert_eq!(added_to(&s).len(), 3);
            assert_eq!(events_of(&s, "burned").len(), 2);
        }

        #[test]
        fn s3_2_r13_3_10_takes_the_top_of_an_enemy_stack_pile_the_card_dormant_beneath_it_acts_again() {
            crate::register_all();
            let mut s = chaos(
                Some("destroy"),
                Opts { p2: Some(json!({ "field": [BIG_FELINOR, { "def": FIENDER, "stack": true }] })), ..Opts::default() },
            );
            s.expect_in_zone(FIENDER, "graveyard");
            assert_eq!(s.unit(P2, 1).map(|unit| unit.def_id), Some(BIG_FELINOR.to_string()));
        }

        #[test]
        fn r380_4_10_adds_3_random_non_token_classic_cards_that_cost_0() {
            crate::register_all();
            let s = chaos(Some("classic"), Opts::default());
            let added = added_to(&s);
            assert_eq!(added.len(), 3);
            for card in &added {
                let def = crate::card_def(&card.def_id);
                assert_eq!(js(&def.set), json!("Classic"));
                assert!(!def.token);
                assert_eq!(card.cost_override, Some(0));
            }
        }

        #[test]
        fn r386_5_10_upgrades_every_card_in_your_hand_and_deck_twice_none_of_the_opponent_s() {
            crate::register_all();
            let s = chaos(
                Some("upgrade"),
                Opts { p1: Some(deck(json!([MENACE, TIMMY]))), p2: Some(json!({ "hand": [MENACE] })), ..Opts::default() },
            );
            let upgraded = events_of(&s, "upgraded");
            // The Vanilla in hand and the two deck cards: two Upgrades each, every one of them a change.
            assert_eq!(upgraded.len(), 6);
            assert!(s.hand(P2).first().and_then(|card| card.tuning.clone()).is_none());
        }

        #[test]
        fn r311_r177_5_10_s_deck_upgrades_are_hidden_from_both_players_its_hand_upgrades_from_the_opponent() {
            crate::register_all();
            let s = chaos(Some("upgrade"), Opts { p1: Some(deck(json!([MENACE]))), ..Opts::default() });
            let deck_card = s.pile(P1, "library").first().map(|card| card.id.clone()).unwrap_or_else(|| "?".to_string());
            for viewer in [P1, P2] {
                assert!(!serde_json::to_string(&s.view(viewer).events).unwrap().contains(&deck_card));
            }
            let hand_card = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == VANILLA)
                .map(|card| card.id)
                .unwrap_or_else(|| "?".to_string());
            assert!(!serde_json::to_string(&s.view(P2).events).unwrap().contains(&hand_card));
            assert!(serde_json::to_string(&s.view(P1).events).unwrap().contains(&hand_card));
        }

        #[test]
        fn r77_r470_r387_6_10_fuses_a_random_card_into_each_deck_card_which_keeps_its_cost_and_type_never_this_card() {
            crate::register_all();
            let s = chaos(Some("fuse"), Opts { p1: Some(deck(json!([MENACE, TIMMY, STOCKPILE]))), ..Opts::default() });
            let library = s.pile(P1, "library");
            assert_eq!(events_of(&s, "fused").len(), 3);
            assert_eq!(library.len(), 3);
            let costs: Vec<i32> = library
                .iter()
                .map(|card| jackioh_engine::mana::effective_cost(s.state(), card, Default::default()))
                .collect();
            assert_eq!(costs, vec![3, 1, 1]);
            for card in &library {
                let fused = s.state().transient_defs.get(&card.def_id);
                assert!(fused.is_some());
                assert!(
                    !fused
                        .and_then(|def| def.ingredients.as_ref())
                        .is_some_and(|parts| parts.iter().any(|part| part.def_id == CHAOS))
                );
            }
            let types: Vec<Value> = library
                .iter()
                .map(|card| s.state().transient_defs.get(&card.def_id).map(|def| js(&def.type_)).unwrap_or(Value::Null))
                .collect();
            assert_eq!(types, vec![json!("Unit"), json!("Unit"), json!("Spell")]);
        }

        #[test]
        fn r311_r177_6_10_s_deck_fusions_are_hidden_from_both_players() {
            crate::register_all();
            let s = chaos(Some("fuse"), Opts { p1: Some(deck(json!([MENACE, TIMMY]))), ..Opts::default() });
            let library = s.pile(P1, "library");
            for viewer in [P1, P2] {
                let fused: Vec<GameEvent> = s
                    .view(viewer)
                    .events
                    .into_iter()
                    .filter(|event| event.event_type().as_str() == "fused")
                    .collect();
                assert_eq!(fused.len(), 2);
                let text = serde_json::to_string(&fused).unwrap();
                for card in &library {
                    assert!(!text.contains(&card.id));
                    assert!(!text.contains(&card.def_id));
                }
            }
        }

        #[test]
        fn r179_6_10_s_fused_deck_cards_survive_json_and_the_game_plays_on_from_the_round_trip_exactly_as_live() {
            crate::register_all();
            let mut s = chaos(Some("fuse"), Opts { p1: Some(deck(json!([MENACE, TIMMY]))), ..Opts::default() });
            let revived: GameState = serde_json::from_value(js(s.state())).expect("the state round-trips");
            assert_eq!(&revived, s.state());
            let spare = s.hand(P1).into_iter().find(|card| card.def_id == VANILLA).map(|card| card.id);
            let action: Action = json_as(json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": spare.clone().unwrap_or_default(),
                "zone": { "row": "units", "lane": 1 },
                "nonce": "plus-json"
            }));
            let resumed = jackioh_engine::reduce::reduce(&revived, &action);
            assert!(resumed.error.is_none());
            s.play(spare.as_deref().unwrap_or(VANILLA), json!({ "zone": 1 }));
            assert_eq!(hash_state(&resumed.state), hash_state(s.state()));
        }

        #[test]
        fn r386_7_10_degrades_every_card_on_the_opponent_s_field_and_in_their_hand_three_times_none_of_yours() {
            crate::register_all();
            let s = chaos(
                Some("degrade"),
                Opts {
                    p1: Some(json!({ "hand": [{ "def": CHAOS }, VANILLA], "field": [TIMMY], "mana": 8 })),
                    p2: Some(json!({ "hand": [MENACE], "field": [MENACE] })),
                    ..Opts::default()
                },
            );
            let degraded: Vec<Option<String>> = events_of(&s, "degraded")
                .iter()
                .map(|event| event["instanceId"].as_str().map(str::to_string))
                .collect();
            let theirs = [s.unit(P2, 1).map(|unit| unit.id), s.hand(P2).first().map(|card| card.id.clone())];
            let got: IndexSet<Option<String>> = degraded.iter().cloned().collect();
            let want: IndexSet<Option<String>> = theirs.iter().cloned().collect();
            assert_eq!(got.len(), want.len());
            assert!(got.iter().all(|id| want.contains(id)));
            for id in &theirs {
                assert_eq!(degraded.iter().filter(|each| *each == id).count(), 3);
            }
            assert!(s.unit(P1, 1).and_then(|unit| unit.tuning).is_none());
        }

        #[test]
        fn r177_7_10_s_degrades_in_their_hand_are_hidden_from_you() {
            crate::register_all();
            let s = chaos(Some("degrade"), Opts { p2: Some(json!({ "hand": [MENACE] })), ..Opts::default() });
            let theirs = s.hand(P2).first().map(|card| card.id.clone()).unwrap_or_else(|| "?".to_string());
            assert!(!serde_json::to_string(&s.view(P1).events).unwrap().contains(&theirs));
            assert!(serde_json::to_string(&s.view(P2).events).unwrap().contains(&theirs));
        }

        #[test]
        fn r64_8_10_summons_a_classic_golem_into_the_leftmost_free_zone() {
            crate::register_all();
            let mut s = chaos(
                Some("golem"),
                Opts { p1: Some(json!({ "hand": [{ "def": CHAOS }, VANILLA], "field": [TIMMY], "mana": 8 })), ..Opts::default() },
            );
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(GOLEM.to_string()));
            let golem = s.unit(P1, 2).map(|unit| unit.id).unwrap_or_default();
            s.expect_stats(golem.as_str(), json!({ "attack": 10, "health": 10 }));
        }

        #[test]
        fn r35_r387_9_10_replaces_your_deck_one_for_one_with_call_to_chaos_cards_of_either_edition_that_cost_0() {
            crate::register_all();
            let s = chaos(
                Some("replace"),
                Opts { p1: Some(deck(json!([MENACE, TIMMY, STOCKPILE, VANILLA, MENACE, TIMMY]))), ..Opts::default() },
            );
            let library = s.pile(P1, "library");
            assert_eq!(library.len(), 6);
            for card in &library {
                assert!(card.def_id == CORE_CHAOS || card.def_id == CHAOS);
                assert_eq!(card.cost_override, Some(0));
            }
        }

        #[test]
        fn r311_r97_9_10_s_new_deck_cards_are_unknown_to_their_owner_and_unnamed_to_both_players() {
            crate::register_all();
            let s = chaos(Some("replace"), Opts { p1: Some(deck(json!([MENACE, TIMMY]))), ..Opts::default() });
            let library = s.pile(P1, "library");
            assert!(library.iter().all(|card| card.known_as.is_none()));
            for viewer in [P1, P2] {
                let transformed: Vec<GameEvent> = s
                    .view(viewer)
                    .events
                    .into_iter()
                    .filter(|event| event.event_type().as_str() == "transformed")
                    .collect();
                let text = serde_json::to_string(&transformed).unwrap();
                for card in &library {
                    assert!(!text.contains(&card.id));
                }
            }
        }

        #[test]
        fn r70_r28_10_10_casts_a_random_call_to_chaos_of_either_edition_free_and_counted_as_a_play() {
            crate::register_all();
            let mut editions: IndexSet<String> = IndexSet::new();
            for seed in ["a", "b", "c", "d", "e", "f"] {
                let game_seed = format!("{SEED}-{seed}");
                let mut s = scenario(json!({
                    "seed": game_seed,
                    "p1": { "hand": [{ "def": CHAOS }, VANILLA], "mana": 8 },
                    "p2": { "hand": [VANILLA] }
                }));
                set_chain(&mut s, CALL_TO_CHAOS_CHAIN_CAP - 1);
                let mut cursor = 0;
                while rolled_names(&game_seed, cursor, false).first() != Some(&"recast") {
                    cursor += 1;
                }
                s.state_mut().rng_cursor = cursor;
                s.play(CHAOS, json!({}));
                let cast: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
                assert_eq!(cast.len(), 1);
                assert_eq!(cast[0]["costPaid"], json!(0));
                editions.insert(cast[0]["defId"].as_str().unwrap_or("").to_string());
            }
            assert!(editions.iter().all(|id| id == CHAOS || id == CORE_CHAOS));
            assert_eq!(editions.len(), 2);
        }
    }

    mod the_chain {
        use super::*;

        #[test]
        fn r87_a_recursion_rolled_at_call_to_chaos_chain_cap_resolves_into_nothing() {
            crate::register_all();
            let s = chaos(Some("recast"), Opts { chain: Some(CALL_TO_CHAOS_CHAIN_CAP), ..Opts::default() });
            assert_eq!(events_of(&s, "cardPlayed").len(), 1);
            assert_eq!(events_of(&s, "chaosRolled")[0]["effects"], json!(["Cast a random Call to Chaos"]));
        }

        #[test]
        fn r28_the_chain_counts_casts_of_either_edition_one_link_short_of_the_cap_a_cast_happens_and_its_own_recursion_can_t() {
            crate::register_all();
            let s = chaos(Some("recast"), Opts { chain: Some(CALL_TO_CHAOS_CHAIN_CAP - 1), ..Opts::default() });
            let played = events_of(&s, "cardPlayed");
            // The played card, then one cast (link 20). Whatever that cast rolled, a further cast would be link 21.
            assert_eq!(played.len(), 2);
            let edition = played[1]["defId"].as_str().unwrap_or("");
            assert!(edition == CHAOS || edition == CORE_CHAOS);
        }
    }

    mod r436_what_was_rolled_is_public {
        use super::*;

        #[test]
        fn both_players_read_the_rolled_clause_by_this_card_while_the_cards_it_added_stay_hidden_from_the_opponent() {
            crate::register_all();
            let s = chaos(Some("fruits"), Opts::default());
            for viewer in [P1, P2] {
                let rolled: Vec<Value> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter(|event| event.event_type().as_str() == "chaosRolled")
                    .map(js)
                    .collect();
                assert_eq!(rolled.len(), 1);
                assert_eq!(rolled[0]["defId"], json!(CHAOS));
                assert_eq!(rolled[0]["effects"], json!(["Add 5 random Fruits to your hand, which cost (0)"]));
            }
            let added = added_to(&s);
            let theirs = serde_json::to_string(&s.view(P2)).unwrap();
            for card in &added {
                assert!(!theirs.contains(&card.id));
            }
            assert_eq!(js(&s.view(P2).opponent.hand), json!({ "count": s.hand(P1).len() }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_rolls_three_different_entries_and_resolves_them_in_the_list_s_order() {
            crate::register_all();
            let cursor = radiant_cursor_where(|names| names.join(",") == ["fruits", "golem", "replace"].join(","));
            let s = chaos(
                None,
                Opts {
                    radiant_cursor: Some(cursor),
                    p1: Some(json!({ "hand": [{ "def": CHAOS, "radiant": true }, VANILLA], "library": [MENACE, TIMMY], "mana": 8 })),
                    ..Opts::default()
                },
            );
            assert_eq!(
                events_of(&s, "chaosRolled")[0]["effects"],
                json!([
                    "Add 5 random Fruits to your hand, which cost (0)",
                    "Summon a Classic Golem",
                    "Replace your deck with random Call to Chaos cards, which cost (0)"
                ])
            );
            let types = types_of(&s);
            assert!(index_of(&types, "addedToHand") < index_of(&types, "summoned"));
            assert!(index_of(&types, "summoned") < index_of(&types, "transformed"));
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
            assert!(s.pile(P1, "library").iter().all(|card| card.def_id == CHAOS || card.def_id == CORE_CHAOS));
        }

        #[test]
        fn r423_r87_the_recursion_resolves_where_it_falls_last_after_the_two_entries_before_it() {
            crate::register_all();
            let cursor = radiant_cursor_where(|names| names.len() == 3 && names[2] == "recast" && names.contains(&"golem"));
            let s = chaos(None, Opts { radiant_cursor: Some(cursor), chain: Some(CALL_TO_CHAOS_CHAIN_CAP - 1), ..Opts::default() });
            let types = types_of(&s);
            let first_summon = index_of(&types, "summoned");
            let cast = index_of_from(&types, "cardPlayed", index_of(&types, "chaosRolled"));
            assert!(first_summon > -1);
            assert!(first_summon < cast);
        }

        #[test]
        fn r436_r87_the_recursion_finishes_in_one_pass_hinder_s_random_discard_pauses_nothing_and_nothing_runs_twice() {
            crate::register_all();
            // The recursion, one link short of the cap, casts a base Core #95 that rolls "draw your whole
            // deck": the deck's Hinder is cast and discards at random (R682, R431: no prompt), so the draw
            // and the Golem run through with no prompt open (R113) and the chain lands once.
            let mut found: Option<Scenario> = None;
            let mut cursor = 0;
            while cursor < CURSOR_SEARCH * 4 && found.is_none() {
                let at = cursor;
                cursor += 1;
                let names = rolled_names(SEED, at, true);
                if !names.contains(&"recast") || !names.contains(&"golem") {
                    continue;
                }
                let s = chaos(
                    None,
                    Opts {
                        radiant_cursor: Some(at),
                        chain: Some(CALL_TO_CHAOS_CHAIN_CAP - 1),
                        p1: Some(json!({ "hand": [{ "def": CHAOS, "radiant": true }, VANILLA], "library": [HINDER, TIMMY], "mana": 8 })),
                        ..Opts::default()
                    },
                );
                let hinder_cast = s.pile(P1, "graveyard").iter().any(|card| card.def_id == HINDER);
                let golem = events_of(&s, "summoned").iter().any(|event| event["defId"] == json!(GOLEM));
                if s.state().pending.is_none() && hinder_cast && golem {
                    found = Some(s);
                }
            }
            let Some(s) = found else {
                panic!("no roll casts the Hinder through the recursion");
            };
            // Nothing of the Radiant's own roll ran twice, and it landed in the graveyard once the chain was done.
            assert_eq!(events_of(&s, "chaosRolled").iter().filter(|event| event["defId"] == json!(CHAOS)).count(), 1);
            assert_eq!(events_of(&s, "summoned").iter().filter(|event| event["defId"] == json!(GOLEM)).count(), 1);
            assert_eq!(s.pile(P1, "graveyard").iter().filter(|card| card.def_id == CHAOS).count(), 1);
        }

        #[test]
        fn r87_at_the_cap_a_radiant_runs_only_its_other_two() {
            crate::register_all();
            let cursor = radiant_cursor_where(|names| names.contains(&"recast") && names.contains(&"golem"));
            let s = chaos(None, Opts { radiant_cursor: Some(cursor), chain: Some(CALL_TO_CHAOS_CHAIN_CAP), ..Opts::default() });
            assert_eq!(events_of(&s, "cardPlayed").len(), 1);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
        }
    }
}
