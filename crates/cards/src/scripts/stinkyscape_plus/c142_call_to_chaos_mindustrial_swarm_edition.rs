//! SPEC §8.12 S+ #142 Call to Chaos (Mindustrial-Swarm Edition) (R28, R87, R423, R436, R3570000–R3570005),
//! a card of the Stinkyscape+ set (R1420, R3535000). (4) Spell, Call to Chaos, Legendary.
//!   Base:    "One of the following random effects: add 3 random Paperclip cards to your hand, they cost
//!            (0), replenish 2 mana; add 3 random Evolved Swarm cards to your hand, they cost (0); summon
//!            two random Radiant Units, make them GIGA; summon or cast 5 random Jlockeed cards; add 3
//!            random Terminus cards to your hand, replenish 4 mana; add the Toxopid to your hand, it costs
//!            (0); Toxic 3 and Burn 3 all enemies; for the rest of the game, when you launch a Starship,
//!            cast a random Call to Chaos and summon a random (2) Cost Weapon; summon a Mindustrial Golem
//!            and a Swarm Golem; cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_MS_EFFECTS`,
//! `subsystems/call_to_chaos_mindustrial_swarm.rs`): the roll, the announcement both players read (R436), the
//! chain cap counting every edition's casts (R28) and the Radiant's order (R423) are shared by every edition.
//! The face is passed explicitly, as C+ #73's file does. R3570000 reads the designer's text: "Refresh" is
//! Replenish (§6.3, R364), the Toxopid is S+ #121, a Jlockeed pool takes Units (R3570003); the rest are
//! R3570001 (Evolved), R3570002 (GIGA), R3570004 (the launch watch) and R3570005 (the recursion that never
//! draws this card). Its Golems are S+ #142.1 and #142.2 (R3570006).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-142";

pub fn script() -> CardScripts {
    // §8.12: "One of the following random effects".
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_MS_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.12, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_MS_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// S+ #142 Call to Chaos (Mindustrial-Swarm Edition) — SPEC §8.12 row 142, R3570000–R3570005.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{built_starship, card_def, filler, js, launch_starship, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const VANILLA: &str = "core-008"; // Mr. Vanilla
    const WASP: &str = "stinkyscape-091"; // The Propagation's Wasp: Swarm, "Evolve: Gain Flying"
    const TOXOPID: &str = "stinkyscapeplus-121";
    const CRAWLER: &str = "stinkyscapeplus-120";
    const MINDUSTRIAL_GOLEM: &str = "stinkyscapeplus-142-1";
    const SWARM_GOLEM: &str = "stinkyscapeplus-142-2";
    const CRYSTAL: &str = "stinkyscape-132"; // Defensive Crystal: a Starship Piece

    /// What every Call to Chaos prints.
    const CALL_PRICE: i32 = 4;
    const SEED: &str = "chaos-mindustrial-swarm";
    /// How many cursors the sweeps of the table walk.
    const SWEEP: u32 = 800;
    /// How many games the pool sweeps play.
    const GAMES: u32 = 8;

    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_MS_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_in(seed: &str, name: &str) -> u32 {
        cursor_where(seed, |cursor| {
            rolled_names(seed, cursor, false).first() == Some(&name)
        })
    }

    fn label_of(name: &str) -> &'static str {
        subsystems::CHAOS_MS_EFFECTS
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
        chaos_in(
            seed,
            cursor_in(seed, entry),
            side(false, json!([])),
            json!({ "hand": [VANILLA] }),
        )
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

    fn has_tag(def_id: &str, tag: Tag) -> bool {
        card_def(def_id).tags.contains(&tag)
    }

    #[test]
    fn r436_r3570000_each_label_is_a_clause_of_both_printed_faces_in_order() {
        crate::register_all();
        let def = card_def(ID);
        for face in [FaceKind::Base, FaceKind::Radiant] {
            let text = fill_params(&def, face, None).to_lowercase();
            let mut from = 0;
            for entry in subsystems::CHAOS_MS_EFFECTS {
                let label = entry.label.to_lowercase();
                let at = text[from..].find(&label).map(|at| at + from);
                assert!(at.is_some(), "{:?}: {}", face, entry.label);
                from = at.unwrap_or(from) + label.len();
            }
        }
        assert_eq!(subsystems::CHAOS_MS_EFFECTS.len(), 10);
        // The badge of entry 8 is its own clause.
        assert_eq!(label_of("launchWatch"), subsystems::CHAOS_MS_LAUNCH_WATCH_LABEL);
    }

    #[test]
    fn each_of_the_ten_entries_is_rolled_over_the_seeds() {
        let mut seen: Vec<&str> = Vec::new();
        for cursor in 0..SWEEP {
            seen.extend(rolled_names(SEED, cursor, false));
        }
        for entry in subsystems::CHAOS_MS_EFFECTS {
            assert!(seen.contains(&entry.name), "{} never rolled", entry.name);
        }
    }

    #[test]
    fn r3570000_the_card_is_of_the_shipped_stinkyscape_plus_set() {
        crate::register_all();
        let def = card_def(ID);
        assert_eq!(def.set, SetName::StinkyscapePlus);
        assert!(set_ships(def.set));
    }

    mod base {
        use super::*;

        #[test]
        fn r436_both_players_read_the_rolled_clause() {
            let mut s = chaos("golems", side(false, json!([])), json!({ "hand": [VANILLA] }));
            for viewer in [P1, P2] {
                let rolled: Vec<Value> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter(|event| event.event_type().as_str() == "chaosRolled")
                    .map(js)
                    .collect();
                assert_eq!(rolled.len(), 1);
                assert_eq!(rolled[0]["effects"], json!([label_of("golems")]));
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r3570000_entry_paperclips_adds_three_paperclip_cards_at_zero_then_replenishes_two() {
            assert_eq!((CHAOS_MS_CARD_COST, CHAOS_MS_PAPERCLIP_REPLENISH), (0, 2));
            let mut p1 = side(false, json!([]));
            p1["mana"] = json!(CALL_PRICE);
            let s = chaos("paperclips", p1, json!({ "hand": [VANILLA] }));
            let added = added_to(&s, P1);
            assert_eq!(added.len(), 3);
            for card in &added {
                assert!(has_tag(&card.def_id, Tag::Paperclip), "{}", card.def_id);
                assert_eq!(card.cost_override, Some(0));
                assert_eq!(cost_of(&s, card), 0);
            }
            // All of it spent on this card, then Replenish 2 gives back up to 2, never past max (R364).
            let mana = &s.state().players.p1.mana;
            assert_eq!(mana.current, CHAOS_MS_PAPERCLIP_REPLENISH.min(mana.max));
            assert!(
                mana.max >= CHAOS_MS_PAPERCLIP_REPLENISH,
                "the board's max lets 2 come back"
            );
        }

        #[test]
        fn r3570000_the_replenish_is_given_though_a_full_hand_burns_the_cards() {
            let mut p1 = side(false, json!([]));
            p1["mana"] = json!(CALL_PRICE);
            let Value::Array(hand) = &mut p1["hand"] else {
                panic!("a hand");
            };
            while (hand.len() as i32) < HAND_CAP {
                hand.push(json!(VANILLA));
            }
            let s = chaos("paperclips", p1, json!({ "hand": [VANILLA] }));
            assert!(added_to(&s, P1).len() < 3, "the full hand burned some");
            let mana = &s.state().players.p1.mana;
            assert_eq!(mana.current, CHAOS_MS_PAPERCLIP_REPLENISH.min(mana.max));
        }

        #[test]
        fn r3570001_entry_swarm_adds_three_swarm_cards_at_zero_marked_where_they_can_evolve() {
            // Over the games every card added is a Swarm card of the shipped or held sets, never a token,
            // at (0), marked evolved exactly when its text prints an Evolve hook.
            let mut wasp_game: Option<Scenario> = None;
            for n in 0..400 {
                let s = chaos_seeded(&format!("{SEED}-swarm-{n}"), "swarm");
                let added = added_to(&s, P1);
                assert_eq!(added.len(), 3);
                for card in &added {
                    let def = card_def(&card.def_id);
                    assert!(def.tags.contains(&Tag::Swarm) && !def.token, "{}", def.id);
                    assert_eq!(card.cost_override, Some(0));
                    assert_eq!(cost_of(&s, card), 0);
                    let can = jackioh_engine::evolution::can_evolve(s.state(), card);
                    assert_eq!(card.enters_evolved == Some(true), can, "{}", def.id);
                }
                if wasp_game.is_none() && added.iter().any(|card| card.def_id == WASP) {
                    wasp_game = Some(s);
                }
                if wasp_game.is_some() && n >= GAMES {
                    break;
                }
            }
            let s = wasp_game.expect("a game added the Wasp");
            let wasp = added_to(&s, P1)
                .into_iter()
                .find(|card| card.def_id == WASP)
                .expect("the Wasp");
            assert_eq!(wasp.enters_evolved, Some(true), "the Wasp prints an Evolve hook");
            // The owner reads the mark; the hidden hand shows the other seat none (CLAUDE.md rule 7).
            assert!(js(&s.view(P1)).to_string().contains("\"evolved\":true"));
            assert!(!js(&s.view(P2)).to_string().contains("\"evolved\":true"));
        }

        #[test]
        fn r3563000_the_swarm_cards_are_unique_within_the_effect_while_the_rule_holds() {
            with_unique_draws(true, || {
                let s = chaos_seeded(&format!("{SEED}-unique"), "swarm");
                let mut ids: Vec<String> = added_to(&s, P1).into_iter().map(|card| card.def_id).collect();
                assert_eq!(ids.len(), 3);
                ids.sort();
                ids.dedup();
                assert_eq!(ids.len(), 3, "E9: three different cards");
            });
        }

        #[test]
        fn r3570002_entry_giga_summons_two_radiant_giga_units() {
            assert_eq!(CHAOS_MS_GIGA_UNITS, 2);
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-giga-{n}"), "giga");
                // Each takes the leftmost four-zone fit (R3546000): lanes 1 and 3.
                let first = s.unit(P1, 1).expect("the first lands in lane 1");
                let second = s.unit(P1, 3).expect("the second lands in lane 3");
                assert!(s.unit(P1, 2).is_none() && s.unit(P1, 4).is_none());
                for card in [&first, &second] {
                    assert!(card.radiant, "{}", card.def_id);
                    assert_eq!(card.giga, Some(true), "{}", card.def_id);
                    let def = card_def(&card.def_id);
                    assert_eq!(def.type_, CardType::Unit);
                    // R380: a pool that names no set draws the sets that ship.
                    assert!(set_ships(def.set), "{}", def.id);
                }
            }
        }

        #[test]
        fn r3570002_with_no_four_zone_fit_entry_giga_summons_nothing() {
            let mut p1 = side(false, json!([]));
            p1["field"] = json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }]);
            let s = chaos("giga", p1, json!({ "hand": [VANILLA] }));
            assert!(events_of(&s, "summoned").is_empty());
        }

        #[test]
        fn r3570003_entry_jlockeed_summons_its_permanents_and_casts_its_spells() {
            crate::register_all();
            let (mut summons, mut casts) = (0, 0);
            for n in 0..GAMES {
                let s = chaos_seeded(&format!("{SEED}-jlockeed-{n}"), "jlockeed");
                let mut generated = 0;
                for event in events_of(&s, "summoned") {
                    let def_id = event["defId"].as_str().unwrap_or("");
                    if has_tag(def_id, Tag::Jlockeed) {
                        // A permanent is summoned, never played, so it fires no Cry (R1).
                        assert!(
                            !events_of(&s, "cardPlayed")
                                .iter()
                                .any(|played| played["defId"] == event["defId"]
                                    && card_def(def_id).type_ != CardType::Spell),
                            "{def_id} was cast"
                        );
                        generated += 1;
                        summons += 1;
                    }
                }
                for event in events_of(&s, "cardPlayed") {
                    let def_id = event["defId"].as_str().unwrap_or("");
                    if def_id == ID {
                        continue;
                    }
                    if has_tag(def_id, Tag::Jlockeed) && card_def(def_id).type_ == CardType::Spell {
                        generated += 1;
                        casts += 1;
                    }
                }
                assert!(generated >= 5, "game {n}: five picks, {generated} made");
            }
            assert!(summons > 0 && casts > 0, "{summons} summoned, {casts} cast");
        }

        #[test]
        fn r3570000_entry_terminus_adds_three_terminus_cards_then_replenishes_four() {
            assert_eq!(CHAOS_MS_TERMINUS_REPLENISH, 4);
            let mut tokens = 0;
            for n in 0..GAMES {
                let mut p1 = side(false, json!([]));
                p1["mana"] = json!(CALL_PRICE);
                let mut s = scenario(json!({
                    "seed": format!("{SEED}-terminus-{n}"),
                    "p1": p1,
                    "p2": { "hand": [VANILLA], "library": filler(6) },
                }));
                s.state_mut().rng_cursor = cursor_in(&format!("{SEED}-terminus-{n}"), "terminus");
                s.play(ID, json!({}));
                let added = added_to(&s, P1);
                assert_eq!(added.len(), 3);
                for card in &added {
                    let def = card_def(&card.def_id);
                    assert!(def.tags.contains(&Tag::Terminus), "{}", def.id);
                    assert_eq!(card.cost_override, None, "the card prints no (0) here");
                    tokens += usize::from(def.token);
                }
                let mana = &s.state().players.p1.mana;
                assert_eq!(mana.current, CHAOS_MS_TERMINUS_REPLENISH.min(mana.max));
            }
            assert!(tokens > 0, "R3570000: the Terminus pool takes tokens");
        }

        #[test]
        fn r3570000_entry_toxopid_adds_the_toxopid_at_zero_and_its_crawler() {
            let s = chaos("toxopid", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s, P1);
            let toxopids: Vec<&CardInstance> = added.iter().filter(|card| card.def_id == TOXOPID).collect();
            assert_eq!(toxopids.len(), 1);
            assert_eq!(toxopids[0].cost_override, Some(0));
            assert_eq!(cost_of(&s, toxopids[0]), 0);
            // Its own text adds the Crawler the first time it enters a hand this game (R3560000), at the
            // Crawler's printed price: that is the Toxopid's rule, not this card's.
            let others: Vec<&CardInstance> = added.iter().filter(|card| card.def_id != TOXOPID).collect();
            assert_eq!(others.len(), 1, "{others:?}");
            assert_eq!(others[0].def_id, CRAWLER);
            assert_eq!(others[0].cost_override, None);
        }

        #[test]
        fn r3570000_entry_afflict_gives_every_enemy_and_their_hero_toxic_and_burn_three() {
            let mut p1 = side(false, json!([]));
            p1["field"] = json!([{ "def": "stand-in (1) 9/9", "lane": 1 }]);
            let s = chaos(
                "afflict",
                p1,
                json!({
                    "hand": [VANILLA],
                    "field": [{ "def": "stand-in (1) 9/9", "lane": 1 }, { "def": "stand-in (1) 9/9", "lane": 3 }],
                }),
            );
            for lane in [1, 3] {
                let enemy = s.unit_at(P2, lane);
                assert_eq!(jackioh_engine::afflictions::toxic_on(&enemy), 3, "lane {lane}");
                assert_eq!(jackioh_engine::afflictions::burn_on(&enemy), 3, "lane {lane}");
            }
            assert_eq!(jackioh_engine::afflictions::hero_toxic_of(s.state(), P2), 3);
            assert_eq!(jackioh_engine::afflictions::hero_burn_of(s.state(), P2), 3);
            // Its caster's side is untouched.
            let own = s.unit_at(P1, 1);
            assert_eq!(jackioh_engine::afflictions::toxic_on(&own), 0);
            assert_eq!(jackioh_engine::afflictions::burn_on(&own), 0);
            assert_eq!(jackioh_engine::afflictions::hero_toxic_of(s.state(), P1), 0);
            assert_eq!(jackioh_engine::afflictions::hero_burn_of(s.state(), P1), 0);
        }

        #[test]
        fn r3570004_entry_launch_watch_badges_its_caster_and_answers_a_launch() {
            let seed = format!("{SEED}-watch");
            let (mut s, ship) = built_starship(
                &seed,
                json!([{ "def": CRYSTAL, "lane": 1 }]),
                json!({ "hand": [ID], "field": [{ "def": VANILLA, "lane": 3 }] }),
                json!({}),
            );
            s.state_mut().rng_cursor = cursor_in(&seed, "launchWatch");
            s.state_mut().players.p1.mana.current = 10;
            s.play(ID, json!({}));
            // The caster's badge reads the clause as printed (R169); the other seat reads it too.
            for viewer in [P1, P2] {
                let view = s.view(viewer);
                let mods = if viewer == P1 {
                    &view.you.modifiers
                } else {
                    &view.opponent.modifiers
                };
                assert!(
                    mods.iter()
                        .any(|badge| badge.label == subsystems::CHAOS_MS_LAUNCH_WATCH_LABEL),
                    "{viewer:?}"
                );
            }
            assert_eq!(subsystems::launch_watches_held(s.state(), P1), 1);
            let before = s.events().len();
            s.state_mut().players.p1.mana.current = 10;
            launch_starship(&mut s, &ship);
            let after: Vec<Value> = s.events()[before..].iter().map(js).collect();
            let launched = after
                .iter()
                .position(|event| event["type"] == "starshipLaunched")
                .expect("the Starship launched");
            // After the launch: a Call to Chaos cast (a play of a card with the tag), and a (2) cost Weapon summoned.
            let cast = after[launched..]
                .iter()
                .position(|event| {
                    event["type"] == "cardPlayed"
                        && has_tag(event["defId"].as_str().unwrap_or(""), Tag::CallToChaos)
                })
                .expect("a Call to Chaos was cast");
            assert!(cast > 0);
            let weapon = after[launched..].iter().find(|event| {
                event["type"] == "summoned" && {
                    let def_id = event["defId"].as_str().unwrap_or("");
                    has_tag(def_id, Tag::Weapon)
                        && card_def(def_id).cost == CardCost::Fixed(CHAOS_MS_WEAPON_COST)
                }
            });
            assert!(weapon.is_some(), "a (2) cost Weapon was summoned");
            let answers = subsystems::launch_watch_answers_this_turn(s.state(), P1);
            assert!(answers >= 1, "the watch counted its answer");
            assert_eq!(subsystems::launch_watch_answers_this_turn(s.state(), P2), 0);
        }

        #[test]
        fn r3570000_entry_golems_summons_both_golems() {
            let s = chaos("golems", side(false, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(s.unit_at(P1, 1).def_id, MINDUSTRIAL_GOLEM);
            assert_eq!(s.unit_at(P1, 2).def_id, SWARM_GOLEM);
            assert!(!s.unit_at(P1, 1).radiant && !s.unit_at(P1, 2).radiant);
        }

        /// R3570005: the tenth entry's own pick never draws this card, though the pool holds it since the
        /// set ships; another edition's recursion may. The other editions run nothing here, so a pick
        /// of this card could only be the tenth entry's.
        #[test]
        fn r3570005_entry_recast_never_casts_this_edition() {
            crate::register_all();
            let pool: Vec<String> = catalog::query(&json_as(json!({ "tags": ["Call to Chaos"] })))
                .iter()
                .map(|def| def.id.clone())
                .collect();
            assert!(
                pool.iter().any(|id| id == ID),
                "the pool holds this edition"
            );
            assert!(pool.len() > 2, "{pool:?}");

            let saved = jackioh_engine::scripts::registered_scripts();
            let mut scripts = saved.clone();
            for edition in pool.iter().filter(|edition| edition.as_str() != ID) {
                scripts.insert(edition.clone(), CardScripts::default());
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

            let mut plays = 0;
            for cursor in (0..SWEEP).filter(|at| rolled_names(SEED, *at, false).first() == Some(&"recast")) {
                let s = chaos_in(SEED, cursor, side(false, json!([])), json!({ "hand": [VANILLA] }));
                let played: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
                assert!(!played.is_empty(), "cursor {cursor}: the recast cast a card");
                assert_ne!(played[0]["defId"], json!(ID), "cursor {cursor}");
                plays += 1;
                if plays >= 60 {
                    break;
                }
            }
            assert_eq!(plays, 60);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_the_radiant_face_rolls_three_different_entries_in_list_order() {
            let cursor = cursor_where(SEED, |at| {
                rolled_names(SEED, at, true) == ["paperclips", "toxopid", "golems"]
            });
            let s = chaos_in(SEED, cursor, side(true, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(
                events_of(&s, "chaosRolled")[0]["effects"],
                json!([label_of("paperclips"), label_of("toxopid"), label_of("golems")])
            );
            let added = added_to(&s, P1);
            assert_eq!(
                added
                    .iter()
                    .filter(|card| has_tag(&card.def_id, Tag::Paperclip))
                    .count(),
                3
            );
            assert_eq!(added.iter().filter(|card| card.def_id == TOXOPID).count(), 1);
            assert_eq!(s.unit_at(P1, 1).def_id, MINDUSTRIAL_GOLEM);
            assert_eq!(s.unit_at(P1, 2).def_id, SWARM_GOLEM);
            let order: Vec<&str> = subsystems::CHAOS_MS_EFFECTS
                .iter()
                .map(|entry| entry.name)
                .collect();
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
        const LOOP_GAMES: u32 = 20;

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
    }
}
