//! SPEC §8.11 Training Grounds 39 Call to Chaos (Training-Grounds Edition) (R28, R87, R423, R436,
//! R2200000–R2200004), a card of the Training Grounds set (R1420, R2194000), which ships since its
//! release (#2204). (4) Spell, Call to Chaos, Legendary.
//!   Base:    "One random effect: Summon 3 Rush Tokens, then give your Units on the field, in your hand
//!            and in your deck Taunt and Divine Shield; fill your hand with Goetzmann Game Designs and
//!            refresh 4 mana; add 4 random Training Grounds cards to your hand, which cost (0); gain 8
//!            mana and draw 2 cards; summon 6 random Training Grounds permanents; add 2 random Radiant
//!            Training Grounds cards to your hand, which cost (0); summon 5 Rush Tokens, and for the rest
//!            of the game your Units have Rush, Trample and Pierce; summon a Training-Grounds Golem; add a
//!            random Call to Chaos to your hand, which costs (0), now and at the start of every second
//!            turn of yours for the rest of the game; cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_TG_EFFECTS`,
//! `subsystems/call_to_chaos_training_grounds.rs`): the roll, the announcement both players read (R436),
//! the chain cap counting every edition's casts (R28) and the Radiant's order (R423) are shared by every
//! edition. The face is passed explicitly, as C+ #73's file does.

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-039";

pub fn script() -> CardScripts {
    // §8.11: "One random effect" of the ten.
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_TG_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.11, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_TG_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// TG #39 Call to Chaos (Training-Grounds Edition) — SPEC §8.11 row 39, BUILD M13 row TG 39.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The Call to Chaos pool of the sets that ship (R380, R1420, R2200004), this edition among them.
    const SHIPPED_EDITIONS: [&str; 6] = [
        "core-095",
        "classicplus-073",
        "meditative-095",
        "stinkyscape-150",
        "stinkyscape-151",
        ID,
    ];
    const RUSH_TOKEN: &str = "core-t-rush";
    const DESIGN: &str = "traininggrounds-040";
    const GOLEM: &str = "traininggrounds-039-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const BEAR: &str = "core-060"; // (1) Trap

    const SEED: &str = "chaos-training-grounds";
    /// How many cursors the sweeps of the table walk.
    const SWEEP: u32 = 800;
    /// How many games the pool sweeps play.
    const GAMES: u32 = 6;

    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_TG_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_in(seed: &str, name: &str) -> u32 {
        cursor_where(seed, |cursor| rolled_names(seed, cursor, false).first() == Some(&name))
    }

    fn label_of(name: &str) -> &'static str {
        subsystems::CHAOS_TG_EFFECTS
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.label)
            .unwrap_or_else(|| panic!("no entry {name}"))
    }

    /// p1 holds this card, a spare Vanilla (which keeps §2.5's auto-end away) and `extra`, with 10 mana.
    fn side(radiant: bool, extra: Value) -> Value {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant }), json!(VANILLA)];
        if let Value::Array(more) = extra {
            hand.extend(more);
        }
        json!({ "hand": hand, "library": filler(6), "mana": 10 })
    }

    fn chaos_in(seed: &str, cursor: u32, p1: Value, p2: Value) -> Scenario {
        let mut p2 = p2;
        if p2.get("library").is_none() {
            p2["library"] = filler(6);
        }
        let mut s = scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }));
        s.state_mut().rng_cursor = cursor;
        s.play(ID, json!({}));
        s
    }

    /// The base face, rolling `entry`.
    fn chaos(entry: &str, p1: Value, p2: Value) -> Scenario {
        chaos_in(SEED, cursor_in(SEED, entry), p1, p2)
    }

    fn chaos_seeded(seed: &str, entry: &str) -> Scenario {
        chaos_in(seed, cursor_in(seed, entry), side(false, json!([])), json!({ "hand": [VANILLA] }))
    }

    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .map(js)
            .collect()
    }

    fn added_to(s: &Scenario, player: PlayerId) -> Vec<CardInstance> {
        let ids: Vec<String> = events_of(s, "addedToHand")
            .iter()
            .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
            .collect();
        s.hand(player)
            .into_iter()
            .filter(|card| ids.contains(&card.id))
            .collect()
    }

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        jackioh_engine::layers::unit_has(s.state(), card, kind)
    }

    /// R2200000: a non-token card of the Training Grounds set, never this one (R387).
    fn is_tg_card(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        def.set == SetName::TrainingGrounds && !def.token && def_id != ID
    }

    /// The Calls to Chaos at (0) in p1's hand, the adds; this edition is a Call to Chaos too (R2200004).
    fn free_calls(s: &Scenario) -> Vec<String> {
        s.hand(P1)
            .iter()
            .filter(|card| crate::card_def(&card.def_id).tags.contains(&Tag::CallToChaos))
            .filter(|card| cost_of(s, card) == CHAOS_TG_COST)
            .map(|card| card.def_id.clone())
            .collect()
    }

    #[test]
    fn r436_each_entry_s_label_is_a_clause_of_the_card_s_printed_list_in_the_printed_order() {
        crate::register_all();
        let def = crate::card_def(ID);
        for text in [def.base.text.to_lowercase(), def.radiant.text.to_lowercase()] {
            let mut from = 0;
            for entry in subsystems::CHAOS_TG_EFFECTS {
                let label = entry.label.to_lowercase();
                let at = text[from..].find(&label).map(|at| at + from);
                assert!(at.is_some(), "{}", entry.label);
                from = at.unwrap_or(from) + label.len();
            }
        }
        assert_eq!(subsystems::CHAOS_TG_EFFECTS.len(), 10);
    }

    #[test]
    fn each_of_the_ten_entries_is_rolled_over_the_seeds() {
        let mut seen: Vec<&str> = Vec::new();
        for cursor in 0..SWEEP {
            seen.extend(rolled_names(SEED, cursor, false));
        }
        for entry in subsystems::CHAOS_TG_EFFECTS {
            assert!(seen.contains(&entry.name), "{} never rolled", entry.name);
        }
    }

    #[test]
    fn r2194000_the_card_is_of_the_training_grounds_set_which_ships_since_its_release() {
        crate::register_all();
        let def = crate::card_def(ID);
        assert_eq!(def.set, SetName::TrainingGrounds);
        assert!(set_ships(def.set));
    }

    mod base {
        use super::*;

        #[test]
        fn r436_both_players_read_the_rolled_clause() {
            let mut s = chaos("golem", side(false, json!([])), json!({ "hand": [VANILLA] }));
            for viewer in [P1, P2] {
                let rolled: Vec<Value> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter(|event| event.event_type().as_str() == "chaosRolled")
                    .map(js)
                    .collect();
                assert_eq!(rolled.len(), 1);
                assert_eq!(rolled[0]["effects"], json!([label_of("golem")]));
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r2200001_entry_fortify_summons_three_tokens_then_gives_every_unit_card_taunt_and_divine_shield() {
            let mut p1 = side(false, json!([BEAR]));
            p1["field"] = json!([{ "def": VANILLA, "lane": 4 }]);
            p1["library"] = json!([VANILLA, BEAR, VANILLA]);
            let s = chaos(
                "fortify",
                p1,
                json!({ "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }] }),
            );
            for lane in 1..=CHAOS_TG_RUSH_TOKENS {
                let token = s.unit(P1, lane).expect("a Rush Token");
                assert_eq!(token.def_id, RUSH_TOKEN);
                assert!(!token.radiant, "a base face");
            }
            let mut units: Vec<CardInstance> = s.state().players.p1.units.iter().flatten().flatten().cloned().collect();
            units.extend(s.hand(P1));
            units.extend(s.state().players.p1.library.iter().cloned());
            let units: Vec<CardInstance> =
                units.into_iter().filter(|card| crate::card_def(&card.def_id).type_ == CardType::Unit).collect();
            // 3 tokens and the Vanilla on the field, the spare in hand, two in the deck.
            assert_eq!(units.len(), 7);
            for card in &units {
                assert!(has(&s, card, KeywordKind::Taunt), "{} Taunt", card.def_id);
                assert!(has(&s, card, KeywordKind::DivineShield), "{} Divine Shield", card.def_id);
            }
            let theirs = s.unit(P2, 1).expect("their Unit");
            assert!(!has(&s, &theirs, KeywordKind::DivineShield));
        }

        #[test]
        fn r2200001_entry_designs_fills_the_hand_then_refreshes_up_to_four() {
            let mut p1 = side(false, json!([]));
            p1["mana"] = json!(CHAOS_TG_REFRESH);
            let s = chaos("designs", p1, json!({ "hand": [VANILLA] }));
            let cap = hand_cap_of(s.state(), P1);
            let hand = s.hand(P1);
            assert_eq!(hand.len() as i32, cap);
            assert_eq!(hand.iter().filter(|card| card.def_id == DESIGN).count() as i32, cap - 1);
            // All of it spent on this card, then Refresh 4 gives back up to max.
            let mana = &s.state().players.p1.mana;
            assert_eq!(mana.current, CHAOS_TG_REFRESH.min(mana.max));
        }

        #[test]
        fn r2200001_entry_designs_refresh_gives_nothing_at_or_above_max() {
            let s = chaos("designs", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let mana = &s.state().players.p1.mana;
            // 10 less this card's 4 is still above max: Refresh is no gain (R364).
            assert!(mana.current >= mana.max);
            assert_eq!(mana.current, 10 - 4);
        }

        #[test]
        fn r2200000_entry_cards_adds_four_training_grounds_cards_at_zero_never_this() {
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-cards-{n}"), "cards");
                let added = added_to(&s, P1);
                assert_eq!(added.len() as i32, CHAOS_TG_CARDS);
                for card in &added {
                    assert!(is_tg_card(&card.def_id), "{}", card.def_id);
                    assert!(!card.radiant);
                    assert_eq!(cost_of(&s, card), CHAOS_TG_COST);
                }
            }
        }

        #[test]
        fn entry_mana_gains_eight_past_max_and_draws_two() {
            let s = chaos("mana", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let mana = &s.state().players.p1.mana;
            assert_eq!(mana.current, 10 - 4 + CHAOS_TG_MANA);
            assert!(mana.current > mana.max);
            assert_eq!(s.state().players.p1.library.len() as i32, 6 - CHAOS_TG_DRAW);
        }

        #[test]
        fn r2200000_entry_permanents_summons_six_training_grounds_permanents_by_type() {
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-permanents-{n}"), "permanents");
                let summoned = events_of(&s, "summoned");
                assert!(!summoned.is_empty());
                assert!(summoned.len() as i32 <= CHAOS_TG_PERMANENTS);
                for event in &summoned {
                    let def_id = event["defId"].as_str().unwrap_or("");
                    assert!(is_tg_card(def_id), "{def_id}");
                    assert_ne!(crate::card_def(def_id).type_, CardType::Spell);
                }
                // Each card stands in the row its type names, a Trap face-down.
                for card in s.state().players.p1.units.iter().flatten().flatten() {
                    assert_eq!(crate::card_def(&card.def_id).type_, CardType::Unit);
                }
                for card in s.state().players.p1.backrow.iter().flatten() {
                    let type_ = crate::card_def(&card.def_id).type_;
                    assert_ne!(type_, CardType::Unit);
                    if matches!(type_, CardType::Trap | CardType::FieldTrap) {
                        assert_ne!(card.face_up, Some(true), "{} face-down", card.def_id);
                    }
                }
            }
        }

        #[test]
        fn r2200000_entry_radiant_adds_two_radiant_training_grounds_cards_at_zero() {
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-radiant-{n}"), "radiant");
                let added = added_to(&s, P1);
                assert_eq!(added.len() as i32, CHAOS_TG_RADIANT_CARDS);
                for card in &added {
                    assert!(is_tg_card(&card.def_id), "{}", card.def_id);
                    assert!(card.radiant);
                    assert_eq!(cost_of(&s, card), CHAOS_TG_COST);
                }
            }
        }

        #[test]
        fn r2200003_entry_army_summons_five_tokens_and_every_unit_has_rush_trample_and_pierce_for_good() {
            let mut s = chaos("army", side(false, json!([])), json!({ "hand": [VANILLA] }));
            for lane in 1..=CHAOS_TG_ARMY_TOKENS {
                assert_eq!(s.unit(P1, lane).map(|unit| unit.def_id), Some(RUSH_TOKEN.to_string()));
            }
            let lent: Vec<Keyword> = s
                .state()
                .players
                .p1
                .mods
                .iter()
                .filter_map(|held| match &held.kind {
                    ModifierKind::UnitsHaveKeyword { keyword } => Some(keyword.clone()),
                    _ => None,
                })
                .collect();
            let kinds: Vec<KeywordKind> = lent.iter().map(Keyword::kind).collect();
            assert_eq!(kinds, vec![KeywordKind::Rush, KeywordKind::Trample, KeywordKind::Pierce]);
            // A Unit that arrives later has them too, the turn after, with this card long gone.
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s.state_mut().players.p2.auto_end_turn = Some(false);
            s.end_turn();
            s.end_turn();
            // Lane 1 cleared by hand, so the Vanilla has a zone.
            s.state_mut().players.p1.units[0] = Default::default();
            s.play(VANILLA, json!({ "lane": 1 }));
            let vanilla = s.unit(P1, 1).expect("the Vanilla");
            assert_eq!(vanilla.def_id, VANILLA);
            for kind in [KeywordKind::Rush, KeywordKind::Trample, KeywordKind::Pierce] {
                assert!(has(&s, &vanilla, kind), "{kind:?}");
            }
            assert!(s.unit(P2, 1).is_none_or(|unit| !has(&s, &unit, KeywordKind::Rush)));
        }

        #[test]
        fn entry_golem_summons_a_base_training_grounds_golem() {
            let s = chaos("golem", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let golem = s.unit(P1, 1).expect("a Golem");
            assert_eq!(golem.def_id, GOLEM);
            assert!(!golem.radiant);
            assert!(s.unit(P1, 2).is_none());
        }

        #[test]
        fn r2200002_entry_alternate_adds_one_now_none_next_turn_one_the_turn_after() {
            let mut s = chaos("alternate", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let now = free_calls(&s);
            assert_eq!(now.len(), 1);
            assert!(SHIPPED_EDITIONS.contains(&now[0].as_str()), "{}", now[0]);
            assert_eq!(subsystems::alternate_effects_held(s.state(), P1), 1);
            assert_eq!(subsystems::alternate_effects_held(s.state(), P2), 0);
            // R169: its badge both players read.
            assert!(
                js(&s.view(P2))["opponent"]["modifiers"]
                    .as_array()
                    .is_some_and(|mods| mods
                        .iter()
                        .any(|entry| entry["label"] == json!(subsystems::CHAOS_TG_ALTERNATE_LABEL)))
            );
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s.state_mut().players.p2.auto_end_turn = Some(false);
            let mut counts = Vec::new();
            // The caster's next four turns, each after an opponent's.
            for _ in 0..4 {
                s.end_turn();
                s.end_turn();
                counts.push(free_calls(&s).len());
            }
            assert_eq!(counts, vec![1, 2, 2, 3]);
            for def_id in free_calls(&s) {
                assert!(SHIPPED_EDITIONS.contains(&def_id.as_str()), "{def_id}");
            }
            assert!(
                s.hand(P2)
                    .iter()
                    .all(|card| !crate::card_def(&card.def_id).tags.contains(&Tag::CallToChaos))
            );
        }

        #[test]
        fn r2200002_an_extra_turn_counts_as_one_of_the_caster_s_turns() {
            let mut s = chaos("alternate", side(false, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(free_calls(&s).len(), 1);
            // Two turns begun since the entry resolved: the second adds, whichever kind it was.
            let from = s.state().players.p1.turns_started;
            s.state_mut().players.p1.turns_started = from + 1;
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s.state_mut().players.p2.auto_end_turn = Some(false);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().players.p1.turns_started, from + 2);
            assert_eq!(free_calls(&s).len(), 2);
        }

        #[test]
        fn r2200002_a_roll_at_the_cap_does_nothing_not_even_the_add_now() {
            crate::register_all();
            let cursor = cursor_in(SEED, "alternate");
            let hand: Vec<Value> =
                (0..=CHAOS_TG_ALTERNATE_CAP).map(|_| json!(ID)).chain([json!(VANILLA)]).collect();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": hand, "library": filler(6), "mana": 20 },
                "p2": { "hand": [VANILLA], "library": filler(6) }
            }));
            for held in 1..=CHAOS_TG_ALTERNATE_CAP + 1 {
                s.state_mut().rng_cursor = cursor;
                s.play(ID, json!({}));
                assert_eq!(
                    s.last_events()
                        .iter()
                        .filter(|event| event.event_type().as_str() == "chaosRolled")
                        .map(js)
                        .next()
                        .map(|event| event["effects"].clone()),
                    Some(json!([label_of("alternate")]))
                );
                let capped = held.min(CHAOS_TG_ALTERNATE_CAP);
                assert_eq!(subsystems::alternate_effects_held(s.state(), P1), capped);
                assert_eq!(free_calls(&s).len() as i32, capped);
            }
        }

        /// BUILD M13 TG 39, R28, R215: tests cast of another edition at chain depth 1.
        #[test]
        fn r2200004_entry_recast_casts_a_shipped_edition_at_chain_depth_one() {
            crate::register_all();
            let saved = jackioh_engine::scripts::registered_scripts();
            let (runs_in, runs) = std::sync::mpsc::channel::<(String, i32)>();
            let report: Hook = hook(move |ctx| {
                let id = ctx.self_.as_ref().map(|card| card.id.clone()).unwrap_or_default();
                let _ = runs_in.send((id, subsystems::chaos_chain_of(ctx.self_.as_ref())));
                vec![]
            });
            let mut scripts = saved.clone();
            for edition in SHIPPED_EDITIONS.iter().filter(|edition| **edition != ID) {
                let face = Script {
                    cry: Some(report.clone()),
                    ..Script::default()
                };
                scripts.insert(edition.to_string(), CardScripts { base: face.clone(), radiant: face });
            }
            jackioh_engine::testkit::register_scripts(scripts);
            struct Restore(Option<IndexMap<String, CardScripts>>);
            impl Drop for Restore {
                fn drop(&mut self) {
                    if let Some(saved) = self.0.take() {
                        jackioh_engine::testkit::register_scripts(saved);
                    }
                }
            }
            let _restore = Restore(Some(saved));

            for cursor in (0..SWEEP).filter(|at| rolled_names(SEED, *at, false).first() == Some(&"recast")) {
                let s = chaos_in(SEED, cursor, side(false, json!([])), json!({ "hand": [VANILLA] }));
                let cast: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
                assert!(!cast.is_empty(), "cursor {cursor}: the recast cast a card");
                let def_id = cast[0]["defId"].as_str().unwrap_or("").to_string();
                let depths: Vec<(String, i32)> = runs.try_iter().collect();
                if def_id == ID {
                    continue;
                }
                assert!(SHIPPED_EDITIONS.contains(&def_id.as_str()), "{def_id}");
                assert_eq!(cast[0]["costPaid"], json!(0));
                assert_ne!(cast[0]["radiant"], json!(true));
                // R28: the played card is depth 0, so the card it casts is the chain's first cast.
                let cast_id = cast[0]["instanceId"].as_str().unwrap_or("").to_string();
                assert_eq!(depths, vec![(cast_id, 1)], "{def_id}");
                return;
            }
            panic!("no cursor below {SWEEP} rolls the recast into another edition");
        }

        /// The pool that names no set holds this card, last in catalog order.
        #[test]
        fn r2200004_once_its_set_ships_the_call_to_chaos_pool_holds_this_card() {
            crate::register_all();
            let pool: Vec<String> = catalog::query(&json_as(json!({ "tags": ["Call to Chaos"] })))
                .iter()
                .map(|def| def.id.clone())
                .collect();
            assert_eq!(pool, SHIPPED_EDITIONS.map(String::from).to_vec());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_three_different_entries_in_list_order() {
            let cursor =
                cursor_where(SEED, |at| rolled_names(SEED, at, true) == ["fortify", "mana", "golem"]);
            let s = chaos_in(SEED, cursor, side(true, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(
                events_of(&s, "chaosRolled")[0]["effects"],
                json!([label_of("fortify"), label_of("mana"), label_of("golem")])
            );
            for lane in 1..=CHAOS_TG_RUSH_TOKENS {
                assert_eq!(s.unit(P1, lane).map(|unit| unit.def_id), Some(RUSH_TOKEN.to_string()));
            }
            assert_eq!(
                s.unit(P1, CHAOS_TG_RUSH_TOKENS + 1).map(|unit| unit.def_id),
                Some(GOLEM.to_string())
            );
            assert_eq!(s.state().players.p1.mana.current, 10 - 4 + CHAOS_TG_MANA);
            let order: Vec<&str> = subsystems::CHAOS_TG_EFFECTS.iter().map(|entry| entry.name).collect();
            for at in 0..SWEEP {
                let names = rolled_names(SEED, at, true);
                assert_eq!(names.len(), 3);
                let places: Vec<usize> = names
                    .iter()
                    .map(|name| order.iter().position(|each| each == name).unwrap_or(usize::MAX))
                    .collect();
                assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{names:?}");
            }
        }

        /// The most actions one play's prompts may take before the test gives up on it.
        const ANSWER_BOUND: usize = 200;
        /// How many seeded Radiant plays the loop sweep makes.
        const LOOP_GAMES: u32 = 40;

        #[test]
        fn r28_many_seeds_end_every_play_within_the_caps() {
            for n in 0..LOOP_GAMES {
                let mut s = scenario(json!({
                    "seed": format!("{SEED}-loops-{n}"),
                    "p1": side(true, json!([])),
                    "p2": { "hand": [VANILLA], "library": filler(6) },
                }));
                s.play(ID, json!({}));
                for _ in 0..ANSWER_BOUND {
                    let Some(pending) = s.state().pending.clone() else {
                        break;
                    };
                    if s.state().result.is_some() {
                        break;
                    }
                    let action = legal_actions(s.state(), pending.player_id)
                        .into_iter()
                        .next()
                        .expect("a prompt always has an answer");
                    s.act(action, pending.player_id, "answer the prompt");
                }
                assert!(
                    s.state().pending.is_none() || s.state().result.is_some(),
                    "seed {n}: the play completes"
                );
            }
        }

        #[test]
        fn r28_the_recast_casts_a_base_form() {
            let cursor =
                cursor_where(SEED, |at| rolled_names(SEED, at, true).last() == Some(&"recast"));
            let s = chaos_in(SEED, cursor, side(true, json!([])), json!({ "hand": [VANILLA] }));
            let cast: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
            assert!(!cast.is_empty());
            assert_ne!(cast[0]["radiant"], json!(true));
        }
    }
}
