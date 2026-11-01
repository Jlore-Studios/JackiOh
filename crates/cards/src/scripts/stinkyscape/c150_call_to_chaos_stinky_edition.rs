//! SPEC §8.10 Stinkyscape 150 Call to Chaos (Stinky Edition) (R28, R87, R423, R436, R1882001–R1882004).
//! (4) Spell, Call to Chaos, Legendary.
//!   Base:    "One random effect: Add a Radiant Faction: ETHICS and a Radiant Faction: Vanguard Coalition to
//!            your hand; add A Very Dandy Lad and an Arctic Citadel to your hand, which cost (0); summon 3
//!            random Quest or Wincon Field Spells into your backrow; add a random Acclaimed Stinkyscape
//!            Unit to your hand and refresh as much mana as it costs; switch every enemy Unit to Attack
//!            Position and Freeze them; fill your board with random Human Stinkyscape Units; Allure every
//!            enemy Unit; for the rest of the game, whenever an Acclaimed card dies, its controller casts a
//!            random Call to Chaos; summon a Stinkyscape Golem; cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_STINKY_EFFECTS`,
//! `subsystems/call_to_chaos_stinky.rs`): the roll, the announcement both players read (R436), the
//! chain cap counting casts of every edition (R28) and the order the Radiant's three resolve in (R423)
//! are the one rule every edition shares. The face is passed explicitly, as C+ #73's file does. Its first
//! entry's S #159 and S #95 are named Faction: ETHICS and Faction: Vanguard Coalition since #2521, the
//! cards and the entry unchanged.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-150";

pub fn script() -> CardScripts {
    // §8.10: "One random effect" of the ten.
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_STINKY_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.10, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_STINKY_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, js, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// The Call to Chaos pool of the sets that ship (R380, R1420). Re-pinned for the Training Grounds
    /// release (#2204): its #39 joins it (R2200004).
    const SHIPPED_EDITIONS: [&str; 6] = [
        "core-095",
        "classicplus-073",
        "meditative-095",
        "stinkyscape-150",
        "stinkyscape-151",
        "traininggrounds-039",
    ];
    /// S #159 Faction: ETHICS and S #95 Faction: Vanguard Coalition, the first entry's.
    const ETHICS: &str = "stinkyscape-159";
    const VANGUARD: &str = "stinkyscape-095";
    const DANDY_LAD: &str = "stinkyscape-113";
    const ARCTIC_CITADEL: &str = "stinkyscape-114";
    const GOLEM: &str = "stinkyscape-150-1";
    /// R1882001: the Field Spells tagged Quest or Wincon, of every set the catalog orders.
    const QUEST_FIELD_SPELLS: [&str; 4] = [
        "classic-090",
        "stinkyscape-007",
        "stinkyscape-112",
        "stinkyscape-148",
    ];
    /// S #87 Siva 1, a 2/2 Acclaimed Unit.
    const SIVA: &str = "stinkyscape-087";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt
    const FILLER: &str = "core-005"; // (1) Spell
    const BEAR: &str = "core-060"; // (1) Trap

    const SEED: &str = "chaos-stinky";
    /// How many cursors the sweeps of the table walk.
    const SWEEP: u32 = 800;
    /// How many games a pool sweep plays.
    const GAMES: u32 = 40;

    /// The names of the entries the roll at `cursor` of `seed` picks, with this card's table.
    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_STINKY_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_in(seed: &str, name: &str) -> u32 {
        cursor_where(seed, |cursor| rolled_names(seed, cursor, false).first() == Some(&name))
    }

    fn label_of(name: &str) -> &'static str {
        subsystems::CHAOS_STINKY_EFFECTS
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

    /// p1 plays this card with its roll pinned to `cursor` of `seed`.
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

    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .map(js)
            .collect()
    }

    fn last_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.last_events()
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .map(js)
            .collect()
    }

    /// The hand cards an `addedToHand` of this game named.
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

    /// The (player, defId) of every play or cast of the most recent step.
    fn casts(s: &Scenario) -> Vec<(String, String)> {
        last_of(s, "cardPlayed")
            .iter()
            .map(|event| {
                (
                    event["player"].as_str().unwrap_or("").to_string(),
                    event["defId"].as_str().unwrap_or("").to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn r436_each_entry_s_label_is_a_clause_of_the_card_s_printed_list_in_the_printed_order() {
        crate::register_all();
        let def = crate::card_def(ID);
        for text in [def.base.text.to_lowercase(), def.radiant.text.to_lowercase()] {
            let mut from = 0;
            for entry in subsystems::CHAOS_STINKY_EFFECTS {
                let label = entry.label.to_lowercase();
                let at = text[from..].find(&label).map(|at| at + from);
                assert!(at.is_some(), "{}", entry.label);
                from = at.unwrap_or(from) + label.len();
            }
        }
        assert_eq!(subsystems::CHAOS_STINKY_EFFECTS.len(), 10);
    }

    #[test]
    fn each_of_the_ten_entries_is_rolled_over_the_seeds() {
        let mut seen: Vec<&str> = Vec::new();
        for cursor in 0..SWEEP {
            seen.extend(rolled_names(SEED, cursor, false));
        }
        for entry in subsystems::CHAOS_STINKY_EFFECTS {
            assert!(seen.contains(&entry.name), "{} never rolled", entry.name);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r436_both_players_read_the_rolled_clause_before_it_resolves() {
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
                assert_eq!(rolled[0]["defId"], json!(ID));
                assert_eq!(rolled[0]["effects"], json!([label_of("golem")]));
            }
            let types: Vec<&str> = s.events().iter().map(|event| event.event_type().as_str()).collect();
            let rolled = types.iter().position(|kind| *kind == "chaosRolled");
            let summoned = types.iter().position(|kind| *kind == "summoned");
            assert!(rolled < summoned);
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn entry_ethics_adds_a_radiant_faction_ethics_and_a_radiant_faction_vanguard_coalition() {
            let s = chaos("ethics", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s, P1);
            let ids: Vec<&str> = added.iter().map(|card| card.def_id.as_str()).collect();
            assert_eq!(ids, vec![ETHICS, VANGUARD]);
            for card in &added {
                assert!(card.radiant);
                assert_eq!(card.cost_override, None);
            }
        }

        #[test]
        fn entry_dandy_adds_a_very_dandy_lad_and_an_arctic_citadel_at_zero() {
            let s = chaos("dandy", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s, P1);
            let ids: Vec<&str> = added.iter().map(|card| card.def_id.as_str()).collect();
            assert_eq!(ids, vec![DANDY_LAD, ARCTIC_CITADEL]);
            for card in &added {
                assert!(!card.radiant);
                assert_eq!(cost_of(&s, card), CHAOS_STINKYSCAPE_COST);
            }
        }

        #[test]
        fn r1882001_entry_quests_summons_three_quest_or_wincon_field_spells_into_the_backrow() {
            let s = chaos("quests", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let summoned: Vec<String> = (1..=5)
                .filter_map(|lane| s.backrow(P1, lane))
                .map(|card| card.def_id)
                .collect();
            assert_eq!(summoned.len() as i32, CHAOS_STINKY_FIELD_SPELLS);
            for def_id in &summoned {
                assert!(QUEST_FIELD_SPELLS.contains(&def_id.as_str()), "{def_id}");
            }
        }

        #[test]
        fn r1882001_the_pool_holds_every_set_s_quest_field_spells_and_no_spell() {
            let mut seen: Vec<String> = Vec::new();
            for n in 0..GAMES {
                let seed = format!("{SEED}-quests-{n}");
                let s = chaos_in(
                    &seed,
                    cursor_in(&seed, "quests"),
                    side(false, json!([])),
                    json!({ "hand": [VANILLA] }),
                );
                seen.extend((1..=5).filter_map(|lane| s.backrow(P1, lane)).map(|card| card.def_id));
            }
            for def_id in QUEST_FIELD_SPELLS {
                assert!(seen.iter().any(|id| id == def_id), "{def_id} never summoned");
            }
            // Repeats are allowed, and only those four ever land.
            assert!(seen.iter().all(|id| QUEST_FIELD_SPELLS.contains(&id.as_str())));
        }

        #[test]
        fn r1882001_a_full_backrow_takes_none_past_it() {
            let mut p1 = side(false, json!([]));
            p1["backrow"] = json!([
                { "def": BEAR, "lane": 1, "faceUp": false },
                { "def": BEAR, "lane": 2, "faceUp": false },
                { "def": BEAR, "lane": 3, "faceUp": false },
                { "def": BEAR, "lane": 4, "faceUp": false },
            ]);
            let s = chaos("quests", p1, json!({ "hand": [VANILLA] }));
            let lane5 = s.backrow(P1, 5).map(|card| card.def_id);
            assert!(lane5.is_some_and(|id| QUEST_FIELD_SPELLS.contains(&id.as_str())));
            assert_eq!(events_of(&s, "summoned").len(), 1);
        }

        #[test]
        fn r1882002_entry_acclaimed_adds_an_acclaimed_unit_and_refreshes_its_cost() {
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": side(false, json!([])),
                "p2": { "hand": [VANILLA], "library": filler(6) },
            }));
            s.state_mut().players.p1.mana.max = 10;
            s.state_mut().players.p1.mana.current = 10;
            s.state_mut().rng_cursor = cursor_in(SEED, "acclaimed");
            s.play(ID, json!({}));
            let added = added_to(&s, P1);
            assert_eq!(added.len(), 1);
            let def = crate::card_def(&added[0].def_id);
            assert!(def.tags.contains(&Tag::Acclaimed));
            assert_eq!(def.set, SetName::Stinkyscape);
            assert_eq!(def.type_, CardType::Unit);
            assert!(!def.token);
            let mana = &s.state().players.p1.mana;
            // 10 mana, 4 paid for this card, then refreshed by the Unit's printed cost.
            assert_eq!(mana.current, 10 - 4 + jackioh_engine::catalog::query_cost(&def));
            assert!(mana.current <= mana.max);
        }

        #[test]
        fn r1882002_a_full_hand_burns_the_unit_and_refreshes_nothing() {
            let extra: Vec<Value> = (0..(HAND_CAP - 1)).map(|_| json!(FILLER)).collect();
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": side(false, json!(extra)),
                "p2": { "hand": [VANILLA], "library": filler(6) },
            }));
            s.state_mut().players.p1.mana.max = 10;
            s.state_mut().players.p1.mana.current = 10;
            s.state_mut().rng_cursor = cursor_in(SEED, "acclaimed");
            s.play(ID, json!({}));
            assert_eq!(events_of(&s, "burned").len(), 1);
            assert_eq!(s.state().players.p1.mana.current, 10 - 4);
        }

        #[test]
        fn r1882003_entry_freeze_switches_every_enemy_unit_to_attack_and_freezes_it() {
            let s = chaos(
                "freeze",
                {
                    let mut p1 = side(false, json!([]));
                    p1["field"] = json!([{ "def": VANILLA, "lane": 1, "position": "DEF" }]);
                    p1
                },
                json!({
                    "hand": [VANILLA],
                    "field": [
                        { "def": VANILLA, "lane": 1, "position": "DEF" },
                        { "def": MENACE, "lane": 2 },
                    ],
                }),
            );
            for lane in [1, 2] {
                let unit = s.unit(P2, lane).expect("an enemy Unit");
                assert_eq!(unit.position.unwrap_or(Position::Atk), Position::Atk);
                assert!(unit.frozen.is_some(), "lane {lane}");
            }
            // Its caster's own Unit is passed by.
            let mine = s.unit(P1, 1).expect("p1's Unit");
            assert_eq!(mine.position, Some(Position::Def));
            assert!(mine.frozen.is_none());
            // A hero is never touched; one switch reported, an effect's (no exertion spent).
            assert_eq!(events_of(&s, "positionSwitched").len(), 1);
            let switched = s.unit(P2, 1).expect("an enemy Unit");
            assert!(!switched.exertion.switched);
        }

        #[test]
        fn entry_humans_fills_the_board_with_human_stinkyscape_units() {
            let mut p1 = side(false, json!([]));
            p1["field"] = json!([{ "def": VANILLA, "lane": 2 }]);
            let s = chaos("humans", p1, json!({ "hand": [VANILLA] }));
            assert_eq!(s.unit(P1, 2).map(|unit| unit.def_id), Some(VANILLA.to_string()));
            for lane in [1, 3, 4, 5] {
                let unit = s.unit(P1, lane).expect("a Human");
                let def = crate::card_def(&unit.def_id);
                assert!(def.tags.contains(&Tag::Human), "{}", unit.def_id);
                assert_eq!(def.set, SetName::Stinkyscape);
                assert_eq!(def.type_, CardType::Unit);
                assert!(!def.token);
            }
        }

        #[test]
        fn entry_allure_steals_every_enemy_unit_at_its_caster_s_next_turn() {
            let mut s = chaos(
                "allure",
                side(false, json!([])),
                json!({ "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 3 }] }),
            );
            let prey = s.unit(P2, 3).expect("p2's unit").id.clone();
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s.state_mut().players.p2.auto_end_turn = Some(false);
            s.end_turn();
            assert_eq!(s.card(&prey).controller, P2);
            s.end_turn();
            assert_eq!(s.card(&prey).controller, P1);
        }

        /// p1 rolls the death rule with a Menace and a Siva 1 on the field against p2's Siva 1 and
        /// a Vanilla, each Siva facing a bigger Unit.
        fn death_rule() -> Scenario {
            let mut p1 = side(false, json!([]));
            p1["field"] = json!([{ "def": MENACE, "lane": 1 }, { "def": SIVA, "lane": 2 }]);
            let mut s = chaos(
                "deathWatch",
                p1,
                json!({
                    "hand": [VANILLA],
                    "field": [{ "def": SIVA, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                }),
            );
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s.state_mut().players.p2.auto_end_turn = Some(false);
            s
        }

        #[test]
        fn r1882004_entry_death_watch_an_enemy_acclaimed_death_casts_for_its_controller() {
            let mut s = death_rule();
            assert_eq!(subsystems::death_watches_held(s.state(), P1), 1);
            // R169: its badge, in the card's own words.
            let view = serde_json::to_string(&s.view(P1)).unwrap_or_default();
            assert!(view.contains(subsystems::CHAOS_DEATH_WATCH_LABEL));
            let mine = s.unit(P1, 1).expect("p1's Menace").id.clone();
            let prey = s.unit(P2, 1).expect("p2's Siva").id.clone();
            s.attack(mine, prey);
            let cast = casts(&s);
            assert_eq!(cast.len(), 1, "{cast:?}");
            assert_eq!(cast[0].0, "p2");
            assert!(SHIPPED_EDITIONS.contains(&cast[0].1.as_str()), "{}", cast[0].1);
            assert_eq!(last_of(&s, "cardPlayed")[0]["costPaid"], json!(0));
        }

        #[test]
        fn r1882004_entry_death_watch_its_caster_s_own_acclaimed_death_casts_for_them() {
            let mut s = death_rule();
            let siva = s.unit(P1, 2).expect("p1's Siva").id.clone();
            let vanilla = s.unit(P2, 2).expect("p2's Vanilla").id.clone();
            s.attack(siva, vanilla);
            let cast = casts(&s);
            assert_eq!(cast.len(), 1, "{cast:?}");
            assert_eq!(cast[0].0, "p1");
            assert!(SHIPPED_EDITIONS.contains(&cast[0].1.as_str()));
        }

        #[test]
        fn r1882004_a_death_of_a_card_that_is_not_acclaimed_casts_nothing() {
            let mut p1 = side(false, json!([]));
            p1["field"] = json!([{ "def": MENACE, "lane": 1 }]);
            let mut s = chaos(
                "deathWatch",
                p1,
                json!({ "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }] }),
            );
            let mine = s.unit(P1, 1).expect("p1's Menace").id.clone();
            let prey = s.unit(P2, 1).expect("p2's Vanilla").id.clone();
            s.attack(mine, prey);
            assert_eq!(last_of(&s, "destroyed").len(), 1);
            assert!(casts(&s).is_empty());
        }

        #[test]
        fn r1882004_a_fourth_death_watch_roll_does_nothing() {
            let cursor = cursor_in(SEED, "deathWatch");
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [ID, ID, ID, ID, VANILLA], "library": filler(6), "mana": 16 },
                "p2": { "hand": [VANILLA], "library": filler(6) }
            }));
            for held in 1..=CHAOS_STINKY_DEATH_WATCH_CAP + 1 {
                s.state_mut().rng_cursor = cursor;
                s.play(ID, json!({}));
                assert_eq!(
                    last_of(&s, "chaosRolled")[0]["effects"],
                    json!([label_of("deathWatch")])
                );
                assert_eq!(
                    subsystems::death_watches_held(s.state(), P1),
                    held.min(CHAOS_STINKY_DEATH_WATCH_CAP)
                );
            }
        }

        /// R2176001: sets `player`'s count of the death watch's casts to `count` on `turn`.
        fn watch_casts(s: &mut Scenario, player: PlayerId, turn: i32, count: i32) {
            s.state_mut().players[player].death_watch_casts = Some(jackioh_engine::state::DrawCount { turn, count });
        }

        #[test]
        fn r2176001_a_player_the_watch_has_cast_for_its_bound_this_turn_casts_nothing_more() {
            let mut s = death_rule();
            let turn = s.state().turn;
            watch_casts(&mut s, P2, turn, CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN);
            let mine = s.unit(P1, 1).expect("p1's Menace").id.clone();
            let prey = s.unit(P2, 1).expect("p2's Siva").id.clone();
            s.attack(mine, prey.clone());
            s.expect_in_zone(prey.as_str(), "graveyard");
            assert!(casts(&s).is_empty(), "{:?}", casts(&s));
            // The bound is each player's own: p1's Siva still casts, and counts.
            let siva = s.unit(P1, 2).expect("p1's Siva").id.clone();
            let vanilla = s.unit(P2, 2).expect("p2's Vanilla").id.clone();
            s.attack(siva, vanilla);
            let cast = casts(&s);
            assert_eq!(cast.len(), 1, "{cast:?}");
            assert_eq!(cast[0].0, "p1");
            assert_eq!(subsystems::death_watch_casts_this_turn(s.state(), P1), 1);
        }

        #[test]
        fn r2176001_one_cast_short_of_the_bound_still_casts_and_reaches_it() {
            let mut s = death_rule();
            let turn = s.state().turn;
            watch_casts(&mut s, P2, turn, CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN - 1);
            let mine = s.unit(P1, 1).expect("p1's Menace").id.clone();
            let prey = s.unit(P2, 1).expect("p2's Siva").id.clone();
            s.attack(mine, prey);
            assert_eq!(casts(&s).len(), 1);
            assert_eq!(
                subsystems::death_watch_casts_this_turn(s.state(), P2),
                CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN
            );
        }

        #[test]
        fn r2176001_a_count_kept_on_an_earlier_turn_reads_as_none() {
            let mut s = death_rule();
            let turn = s.state().turn;
            watch_casts(&mut s, P2, turn - 1, CHAOS_STINKY_DEATH_WATCH_CASTS_PER_TURN);
            assert_eq!(subsystems::death_watch_casts_this_turn(s.state(), P2), 0);
            let mine = s.unit(P1, 1).expect("p1's Menace").id.clone();
            let prey = s.unit(P2, 1).expect("p2's Siva").id.clone();
            s.attack(mine, prey);
            assert_eq!(casts(&s).len(), 1);
            assert_eq!(subsystems::death_watch_casts_this_turn(s.state(), P2), 1);
        }

        #[test]
        fn entry_golem_summons_a_stinkyscape_golem() {
            let mut s = chaos("golem", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let golem = s.unit(P1, 1).expect("a Golem");
            assert_eq!(golem.def_id, GOLEM);
            assert!(!golem.radiant);
            s.expect_stats(golem.id.as_str(), json!({ "attack": 10, "health": 10 }));
        }

        #[test]
        fn entry_recast_casts_a_shipped_edition() {
            let s = chaos("recast", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let cast: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
            assert!(!cast.is_empty());
            assert_eq!(cast[0]["costPaid"], json!(0));
            // R1420: the pool is the shipped sets' editions, this set's own two among them since it shipped.
            assert!(SHIPPED_EDITIONS.contains(&cast[0]["defId"].as_str().unwrap_or("")));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_three_different_entries_in_list_order() {
            let cursor = cursor_where(SEED, |at| rolled_names(SEED, at, true) == ["ethics", "dandy", "golem"]);
            let s = chaos_in(SEED, cursor, side(true, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(
                events_of(&s, "chaosRolled")[0]["effects"],
                json!([label_of("ethics"), label_of("dandy"), label_of("golem")])
            );
            let ids: Vec<String> = added_to(&s, P1).iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(ids, vec![ETHICS, VANGUARD, DANDY_LAD, ARCTIC_CITADEL]);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
            let order: Vec<&str> = subsystems::CHAOS_STINKY_EFFECTS.iter().map(|entry| entry.name).collect();
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
    }
}
