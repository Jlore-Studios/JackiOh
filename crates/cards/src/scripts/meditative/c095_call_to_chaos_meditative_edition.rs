//! M #95 Call to Chaos (Meditative Edition) (SPEC §8.8 row 95, R28, R87, R423, R436, R1240–R1244). (4)
//! Spell, Call to Chaos, Legendary.
//!   Base:    "One random effect: Fuse your hand into one card and add 2 copies of it to your hand, all
//!            three of which cost (0); add 3 random CN cards to your hand, which cost (0); add 2 random
//!            Prime cards to your hand, which cost (0); your hero gains 8 Armor and you heal it 8; summon a
//!            Jade Beauty; summon 3 random Acclaimed cards; Bounce every enemy permanent, then Nerf each
//!            card bounced; summon a CN Golem; for the rest of the game, at the start of each of your
//!            turns, cast a random Call to Chaos; cast a random Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_MED_EFFECTS`,
//! `subsystems/call_to_chaos_meditative.rs`): the roll, the announcement both players read (R436), the
//! chain cap counting casts of every edition (R28, R1240) and the order the Radiant's three resolve in
//! (R423) are the one rule the three editions share. The face is passed explicitly, as C+ #73's file does.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-095";

pub fn script() -> CardScripts {
    // §8.8: "One random effect" of the ten.
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_MED_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.8, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_MED_EFFECTS),
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

    use crate::js;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const CORE_CHAOS: &str = "core-095";
    const PLUS_CHAOS: &str = "classicplus-073";
    const JADE_BEAUTY: &str = "meditative-039-5";
    const GOLEM: &str = "meditative-095-1";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt; Radiant, Immutable too
    const TIMMY: &str = "core-011"; // (1) Unit
    const BEAR: &str = "core-060"; // (1) Trap

    const SEED: &str = "chaos-meditative";
    const CURSOR_SEARCH: u32 = 800;

    /// The names of the entries the roll at `cursor` of `seed` picks, with this card's table.
    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_MED_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_for(name: &str) -> u32 {
        (0..CURSOR_SEARCH)
            .find(|cursor| rolled_names(SEED, *cursor, false).first() == Some(&name))
            .unwrap_or_else(|| panic!("no cursor rolls {name}"))
    }

    fn radiant_cursor_where(accept: impl Fn(&[&str]) -> bool) -> u32 {
        (0..CURSOR_SEARCH * 4)
            .find(|cursor| accept(&rolled_names(SEED, *cursor, true)))
            .unwrap_or_else(|| panic!("no radiant cursor"))
    }

    fn label_of(name: &str) -> &'static str {
        subsystems::CHAOS_MED_EFFECTS
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.label)
            .unwrap_or_else(|| panic!("no entry {name}"))
    }

    /// p1 holds this card, a spare Vanilla (which keeps §2.5's auto-end away) and `extra`, with 8 mana.
    fn side(radiant: bool, extra: Value) -> Value {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant }), json!(VANILLA)];
        if let Value::Array(more) = extra {
            hand.extend(more);
        }
        json!({ "hand": hand, "mana": 8 })
    }

    /// p1 plays this card with its roll pinned to `cursor`.
    fn chaos_at(cursor: u32, p1: Value, p2: Value) -> Scenario {
        let mut s = scenario(json!({ "seed": SEED, "p1": p1, "p2": p2 }));
        s.state_mut().rng_cursor = cursor;
        s.play(ID, json!({}));
        s
    }

    /// The base face, rolling `entry`.
    fn chaos(entry: &str, p1: Value, p2: Value) -> Scenario {
        chaos_at(cursor_for(entry), p1, p2)
    }

    /// The events of one type, as their JSON.
    fn events_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.events()
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .map(js)
            .collect()
    }

    fn types_of(s: &Scenario) -> Vec<&'static str> {
        s.events()
            .iter()
            .map(|event| event.event_type().as_str())
            .collect()
    }

    fn index_of(order: &[&str], kind: &str) -> usize {
        order
            .iter()
            .position(|entry| *entry == kind)
            .unwrap_or_else(|| panic!("no {kind} event"))
    }

    /// The hand cards an `addedToHand` of this play named.
    fn added_to(s: &Scenario) -> Vec<CardInstance> {
        let ids: Vec<String> = events_of(s, "addedToHand")
            .iter()
            .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
            .collect();
        s.hand(P1)
            .into_iter()
            .filter(|card| ids.contains(&card.id))
            .collect()
    }

    fn cost_of(s: &Scenario, card: &CardInstance) -> i32 {
        jackioh_engine::mana::effective_cost(s.state(), card, Default::default())
    }

    /// The events of one type of the most recent step only, as their JSON.
    fn last_of(s: &Scenario, kind: &str) -> Vec<Value> {
        s.last_events()
            .iter()
            .filter(|event| event.event_type().as_str() == kind)
            .map(js)
            .collect()
    }

    /// The cards the most recent step played or cast, by def id.
    fn casts(s: &Scenario) -> Vec<String> {
        last_of(s, "cardPlayed")
            .iter()
            .filter_map(|event| event["defId"].as_str().map(str::to_string))
            .collect()
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
        assert!(base.cry.is_some() && radiant.cry.is_some());
        assert!(base.triggers.is_empty() && radiant.triggers.is_empty());
    }

    #[test]
    fn r436_each_entry_s_label_is_a_clause_of_the_card_s_printed_list_in_the_printed_order() {
        crate::register_all();
        let def = crate::card_def(ID);
        for text in [def.base.text.to_lowercase(), def.radiant.text.to_lowercase()] {
            let mut from = 0;
            for entry in subsystems::CHAOS_MED_EFFECTS {
                let label = entry.label.to_lowercase();
                let at = text[from..].find(&label).map(|at| at + from);
                assert!(at.is_some(), "{}", entry.label);
                from = at.unwrap_or(from) + label.len();
            }
        }
        assert_eq!(subsystems::CHAOS_MED_EFFECTS.len(), 10);
    }

    mod base {
        use super::*;

        #[test]
        fn r436_both_players_read_the_rolled_clause() {
            crate::register_all();
            let mut s = chaos("cn", side(false, json!([])), json!({ "hand": [VANILLA] }));
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
                assert_eq!(rolled[0]["effects"], json!([label_of("cn")]));
            }
            // The cards it added stay hidden from the opponent.
            let theirs = serde_json::to_string(&s.view(P2)).unwrap_or_default();
            for card in added_to(&s) {
                assert!(!theirs.contains(&card.id));
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn r1242_entry_fuse_keeps_immutable_cards_and_hides_the_fusion() {
            crate::register_all();
            let s = chaos(
                "fuse",
                side(false, json!([TIMMY, { "def": MENACE, "radiant": true }])),
                json!({ "hand": [VANILLA] }),
            );
            let hand = s.hand(P1);
            // The Immutable Menace stays where it was, at its own cost; the Vanilla and Timmy became one
            // fresh card, and two copies of it follow, all three at (0).
            assert_eq!(events_of(&s, "fused").len(), 1);
            assert_eq!(hand.len(), 1 + 1 + CHAOS_MED_FUSE_COPIES as usize);
            assert_eq!(hand[0].def_id, MENACE);
            assert_eq!(cost_of(&s, &hand[0]), 3);
            let fusion = hand[1].def_id.clone();
            assert!(s.state().transient_defs.contains_key(&fusion));
            for card in &hand[1..] {
                assert_eq!(card.def_id, fusion);
                assert_eq!(card.cost_override, Some(CHAOS_MED_COST));
                assert_eq!(cost_of(&s, card), CHAOS_MED_COST);
            }
            assert!(
                !hand
                    .iter()
                    .any(|card| card.def_id == VANILLA || card.def_id == TIMMY)
            );
            // R470: a hand's fusion is hidden as the hand is.
            let theirs = serde_json::to_string(&s.view(P2)).unwrap_or_default();
            assert!(!theirs.contains(&fusion));
            assert_eq!(js(&s.view(P2).opponent.hand), json!({ "count": hand.len() }));
        }

        #[test]
        fn r1242_entry_fuse_one_card_takes_the_copies() {
            crate::register_all();
            let s = chaos("fuse", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let hand = s.hand(P1);
            assert!(events_of(&s, "fused").is_empty());
            assert_eq!(hand.len(), 1 + CHAOS_MED_FUSE_COPIES as usize);
            for card in &hand {
                assert_eq!(card.def_id, VANILLA);
                assert_eq!(cost_of(&s, card), CHAOS_MED_COST);
            }
        }

        #[test]
        fn entry_cn_adds_three_cn_cards_at_zero() {
            crate::register_all();
            let s = chaos("cn", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s);
            assert_eq!(added.len() as i32, CHAOS_MED_CN_CARDS);
            for card in &added {
                let def = crate::card_def(&card.def_id);
                assert!(def.tags.contains(&Tag::Cn));
                assert!(!def.token);
                // R1420: the Meditative CN cards are not in a pool before their set ships.
                assert_ne!(def.set, SetName::Meditative);
                assert_eq!(cost_of(&s, card), CHAOS_MED_COST);
            }
        }

        #[test]
        fn r1421_entry_prime_adds_two_prime_tokens_at_zero() {
            crate::register_all();
            let s = chaos("prime", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s);
            assert_eq!(added.len() as i32, CHAOS_MED_PRIME_CARDS);
            for card in &added {
                let def = crate::card_def(&card.def_id);
                assert!(def.tags.contains(&Tag::Prime));
                assert!(def.token);
                assert_ne!(def.set, SetName::Meditative);
                assert_eq!(cost_of(&s, card), CHAOS_MED_COST);
            }
        }

        #[test]
        fn entry_armor_gives_8_armor_and_8_health() {
            crate::register_all();
            let mut p1 = side(false, json!([]));
            p1["health"] = json!(20);
            let mut s = chaos("armor", p1, json!({ "hand": [VANILLA] }));
            s.expect_health(P1, 20 + CHAOS_MED_HEAL);
            assert_eq!(s.state().players.p1.hero.armor, CHAOS_MED_HERO_ARMOR);
            assert_eq!(s.state().players.p2.hero.armor, 0);
        }

        #[test]
        fn entry_jade_summons_a_base_jade_beauty() {
            crate::register_all();
            let s = chaos("jade", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let jade = s.unit(P1, 1);
            assert_eq!(jade.as_ref().map(|unit| unit.def_id.as_str()), Some(JADE_BEAUTY));
            assert!(!jade.is_some_and(|unit| unit.radiant));
        }

        #[test]
        fn r1243_entry_acclaimed_summons_three_acclaimed_permanents() {
            crate::register_all();
            let s = chaos("acclaimed", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let summoned = events_of(&s, "summoned");
            assert_eq!(summoned.len() as i32, CHAOS_MED_ACCLAIMED);
            for event in &summoned {
                let def = crate::card_def(event["defId"].as_str().unwrap_or(""));
                assert!(def.tags.contains(&Tag::Acclaimed));
                assert!(!def.token);
                assert_ne!(def.type_, CardType::Spell);
            }
            assert_eq!(
                (1..=5).filter(|lane| s.unit(P1, *lane).is_some()).count() as i32,
                CHAOS_MED_ACCLAIMED
            );
        }

        #[test]
        fn r1244_entry_bounce_nerfs_each_bounced_card_once() {
            crate::register_all();
            let s = chaos(
                "bounce",
                {
                    let mut p1 = side(false, json!([]));
                    p1["field"] = json!([TIMMY]);
                    p1
                },
                json!({ "hand": [MENACE], "field": [VANILLA], "backrow": [{ "def": BEAR, "faceUp": false }] }),
            );
            assert!(s.unit(P2, 1).is_none() && s.backrow(P2, 1).is_none());
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(TIMMY.to_string()));
            let theirs = s.hand(P2);
            let bounced: Vec<String> = theirs
                .iter()
                .filter(|card| card.def_id != MENACE)
                .map(|card| card.id.clone())
                .collect();
            assert_eq!(bounced.len(), 2);
            let degraded: Vec<String> = events_of(&s, "degraded")
                .iter()
                .filter_map(|event| event["instanceId"].as_str().map(str::to_string))
                .collect();
            for id in &bounced {
                assert_eq!(
                    degraded.iter().filter(|each| *each == id).count() as i32,
                    CHAOS_MED_NERFS
                );
            }
            // The card already in their hand is not Nerfed, and the Nerfs in their hand are hidden from you.
            assert_eq!(degraded.len(), bounced.len());
            let mine = serde_json::to_string(&s.view(P1).events).unwrap_or_default();
            assert!(bounced.iter().all(|id| !mine.contains(id.as_str())));
        }

        #[test]
        fn entry_golem_summons_a_cn_golem() {
            crate::register_all();
            let mut s = chaos("golem", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let golem = s.unit(P1, 1).expect("a Golem");
            assert_eq!(golem.def_id, GOLEM);
            s.expect_stats(golem.id.as_str(), json!({ "attack": 10, "health": 10 }));
        }

        #[test]
        fn r1241_entry_eternal_casts_a_call_from_the_next_start_of_turn() {
            crate::register_all();
            let mut s = chaos("eternal", side(false, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(subsystems::eternal_effects_held(s.state(), P1), 1);
            assert_eq!(casts(&s), vec![ID.to_string()]);
            s.state_mut().players.p1.auto_end_turn = Some(false);
            s.state_mut().players.p2.auto_end_turn = Some(false);
            // The opponent's start of turn casts nothing.
            s.end_turn();
            assert!(casts(&s).is_empty());
            // The caster's next start of turn casts one Call to Chaos of a set that ships (R1420).
            s.end_turn();
            let cast = casts(&s);
            assert!(!cast.is_empty());
            assert!(cast[0] == CORE_CHAOS || cast[0] == PLUS_CHAOS);
            assert_eq!(last_of(&s, "cardPlayed")[0]["costPaid"], json!(0));
            assert_eq!(subsystems::eternal_effects_held(s.state(), P1), 1);
        }

        #[test]
        fn r1241_a_fourth_eternal_roll_does_nothing() {
            crate::register_all();
            let cursor = cursor_for("eternal");
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": { "hand": [ID, ID, ID, ID, VANILLA], "mana": 16 },
                "p2": { "hand": [VANILLA] }
            }));
            for held in 1..=CALL_TO_CHAOS_ETERNAL_CAP + 1 {
                s.state_mut().rng_cursor = cursor;
                s.play(ID, json!({}));
                assert_eq!(
                    last_of(&s, "chaosRolled")[0]["effects"],
                    json!([label_of("eternal")])
                );
                assert_eq!(
                    subsystems::eternal_effects_held(s.state(), P1),
                    held.min(CALL_TO_CHAOS_ETERNAL_CAP)
                );
            }
            assert_eq!(
                s.pile(P1, "graveyard")
                    .iter()
                    .filter(|card| card.def_id == ID)
                    .count(),
                4
            );
        }

        #[test]
        fn r1240_entry_recast_casts_a_shipped_edition() {
            crate::register_all();
            let mut editions: Vec<String> = Vec::new();
            for seed in ["a", "b", "c", "d", "e", "f", "g", "h"] {
                let game_seed = format!("{SEED}-{seed}");
                let mut s = scenario(json!({
                    "seed": game_seed,
                    "p1": side(false, json!([])),
                    "p2": { "hand": [VANILLA] }
                }));
                let id = s.card(ID).id.clone();
                if let Some(card) = find_instance_mut(s.state_mut(), &id) {
                    card.memory.insert(
                        subsystems::CHAOS_CHAIN_KEY.to_string(),
                        json!(CALL_TO_CHAOS_CHAIN_CAP - 1),
                    );
                }
                let cursor = (0..CURSOR_SEARCH)
                    .find(|at| rolled_names(&game_seed, *at, false).first() == Some(&"recast"))
                    .unwrap_or_else(|| panic!("no cursor rolls recast"));
                s.state_mut().rng_cursor = cursor;
                s.play(ID, json!({}));
                let cast: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
                assert_eq!(cast.len(), 1);
                assert_eq!(cast[0]["costPaid"], json!(0));
                editions.push(cast[0]["defId"].as_str().unwrap_or("").to_string());
            }
            // R1420: before the Meditative set ships the pool holds the two editions that ship.
            assert!(editions.iter().all(|id| id == CORE_CHAOS || id == PLUS_CHAOS));
        }

        #[test]
        fn r1240_under_preview_the_pool_holds_this_card() {
            crate::register_all();
            let pool = || -> Vec<String> {
                jackioh_engine::catalog::query(&json_as(json!({ "tags": ["Call to Chaos"] })))
                    .iter()
                    .map(|def| def.id.clone())
                    .collect()
            };
            let shipped = pool();
            assert!(shipped.contains(&CORE_CHAOS.to_string()) && shipped.contains(&PLUS_CHAOS.to_string()));
            assert!(!shipped.contains(&ID.to_string()));
            let _preview = preview_sets(&[SetName::Meditative]);
            let previewed = pool();
            assert!(previewed.contains(&ID.to_string()));
            assert_eq!(previewed.len(), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_three_different_entries_in_list_order() {
            crate::register_all();
            let cursor = radiant_cursor_where(|names| names == ["cn", "armor", "golem"]);
            let s = chaos_at(cursor, side(true, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(
                events_of(&s, "chaosRolled")[0]["effects"],
                json!([label_of("cn"), label_of("armor"), label_of("golem")])
            );
            let types = types_of(&s);
            assert!(index_of(&types, "addedToHand") < index_of(&types, "summoned"));
            assert_eq!(added_to(&s).len() as i32, CHAOS_MED_CN_CARDS);
            assert_eq!(s.state().players.p1.hero.armor, CHAOS_MED_HERO_ARMOR);
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(GOLEM.to_string()));
            // Every Radiant roll is three different entries in the list's order.
            let order: Vec<&str> = subsystems::CHAOS_MED_EFFECTS
                .iter()
                .map(|entry| entry.name)
                .collect();
            for at in 0..CURSOR_SEARCH {
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
