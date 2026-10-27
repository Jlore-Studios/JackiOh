//! SPEC §8.10 Stinkyscape 151 Call to Chaos (Scape Edition) (R28, R87, R423, R436, R1882005–R1882009).
//! (4) Spell, Call to Chaos, Legendary.
//!   Base:    "One random effect: Replace your side of the field with a random Anomaly, any of the ten;
//!            summon 3 Drone Corvettes; summon the Drone Battleship; cast 2 different cards of Core System,
//!            Secure System, Unsecure System and Wild System, picked at random; add 4 random Starship
//!            Pieces to your hand, which cost (0); summon a Drone Interceptor into each of your unit zones,
//!            on top of any Unit there; fuse 4 random Starship Pieces into your Dormant Starship, then
//!            launch it; add 2 random Radiant Weapons to your hand, which cost (0); for the rest of the
//!            game, the Marketplace offers you a Call to Chaos for 20 Credits each cycle; cast a random
//!            Call to Chaos."
//!   Radiant: "Three different random effects, resolved in the order listed: …" (the same ten).
//!
//! Core #95's subsystem (`subsystems::call_to_chaos`) with this edition's table (`CHAOS_SCAPE_EFFECTS`,
//! `subsystems/call_to_chaos_scape.rs`): the roll, the announcement both players read (R436), the
//! chain cap counting casts of every edition (R28) and the order the Radiant's three resolve in (R423)
//! are the one rule every edition shares. The face is passed explicitly, as C+ #73's file does.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-151";

pub fn script() -> CardScripts {
    // §8.10: "One random effect" of the ten.
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(false),
                table: Some(subsystems::CHAOS_SCAPE_EFFECTS),
            })]
        })),
        ..Script::default()
    };

    // §8.10, R423: three different random effects of the ten, resolved in the order the list writes them.
    let radiant = Script {
        cry: Some(hook(|_ctx| {
            vec![subsystems::call_to_chaos(subsystems::CallToChaosArgs {
                radiant: Some(true),
                table: Some(subsystems::CHAOS_SCAPE_EFFECTS),
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

    /// The Call to Chaos pool of the sets that ship (R380, R1420).
    const SHIPPED_EDITIONS: [&str; 3] = ["core-095", "classicplus-073", "meditative-095"];
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    /// The Drone Battleship's layers, bottom first.
    const BATTLESHIP: [&str; 3] = [
        "stinkyscape-t-meltdown-layer",
        "stinkyscape-t-propagation-layer",
        "stinkyscape-t-shield-layer",
    ];
    const SYSTEMS: [&str; 4] = ["stinkyscape-117", "stinkyscape-118", "stinkyscape-119", "stinkyscape-120"];
    const DORMANT_STARSHIP: &str = "stinkyscape-t-dormant-starship";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt

    const SEED: &str = "chaos-scape";
    /// How many cursors the sweeps of the table walk.
    const SWEEP: u32 = 800;

    fn rolled_names(seed: &str, cursor: u32, radiant: bool) -> Vec<&'static str> {
        let mut rng = Rng::new(seed, cursor);
        subsystems::roll_chaos_effects(&mut rng, radiant, Some(subsystems::CHAOS_SCAPE_EFFECTS))
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn cursor_in(seed: &str, name: &str) -> u32 {
        cursor_where(seed, |cursor| rolled_names(seed, cursor, false).first() == Some(&name))
    }

    fn label_of(name: &str) -> &'static str {
        subsystems::CHAOS_SCAPE_EFFECTS
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

    /// The definitions of the pile in `player`'s unit zone `lane`, bottom first (a pile is kept top
    /// first).
    fn pile_defs(s: &Scenario, player: PlayerId, lane: i32) -> Vec<String> {
        let mut defs: Vec<String> = s.state().players[player].units[(lane - 1) as usize]
            .iter()
            .flatten()
            .map(|card| card.def_id.clone())
            .collect();
        defs.reverse();
        defs
    }

    fn is_piece(def_id: &str) -> bool {
        let def = crate::card_def(def_id);
        !def.token
            && def.set == SetName::Stinkyscape
            && def
                .base
                .keywords
                .iter()
                .any(|keyword| keyword.kind() == KeywordKind::StarshipPiece)
    }

    #[test]
    fn r436_each_entry_s_label_is_a_clause_of_the_card_s_printed_list_in_the_printed_order() {
        crate::register_all();
        let def = crate::card_def(ID);
        for text in [def.base.text.to_lowercase(), def.radiant.text.to_lowercase()] {
            let mut from = 0;
            for entry in subsystems::CHAOS_SCAPE_EFFECTS {
                let label = entry.label.to_lowercase();
                let at = text[from..].find(&label).map(|at| at + from);
                assert!(at.is_some(), "{}", entry.label);
                from = at.unwrap_or(from) + label.len();
            }
        }
        assert_eq!(subsystems::CHAOS_SCAPE_EFFECTS.len(), 10);
    }

    #[test]
    fn each_of_the_ten_entries_is_rolled_over_the_seeds() {
        let mut seen: Vec<&str> = Vec::new();
        for cursor in 0..SWEEP {
            seen.extend(rolled_names(SEED, cursor, false));
        }
        for entry in subsystems::CHAOS_SCAPE_EFFECTS {
            assert!(seen.contains(&entry.name), "{} never rolled", entry.name);
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r436_both_players_read_the_rolled_clause() {
            let mut s = chaos("corvettes", side(false, json!([])), json!({ "hand": [VANILLA] }));
            for viewer in [P1, P2] {
                let rolled: Vec<Value> = s
                    .view(viewer)
                    .events
                    .iter()
                    .filter(|event| event.event_type().as_str() == "chaosRolled")
                    .map(js)
                    .collect();
                assert_eq!(rolled.len(), 1);
                assert_eq!(rolled[0]["effects"], json!([label_of("corvettes")]));
            }
            s.expect_in_zone(ID, "graveyard");
        }

        #[test]
        fn entry_corvettes_summons_three_drone_corvettes() {
            let s = chaos("corvettes", side(false, json!([])), json!({ "hand": [VANILLA] }));
            for lane in 1..=CHAOS_SCAPE_CORVETTES {
                assert_eq!(s.unit(P1, lane).map(|unit| unit.def_id), Some(CORVETTE.to_string()));
            }
            assert!(s.unit(P1, CHAOS_SCAPE_CORVETTES + 1).is_none());
        }

        #[test]
        fn entry_battleship_summons_the_three_layers_in_one_pile_shield_on_top() {
            let s = chaos("battleship", side(false, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(pile_defs(&s, P1, 1), BATTLESHIP.map(String::from).to_vec());
            assert!(s.unit(P1, 2).is_none());
        }

        #[test]
        fn r1882006_entry_systems_casts_two_different_systems() {
            let mut pairs: Vec<Vec<String>> = Vec::new();
            for n in 0..12 {
                let seed = format!("{SEED}-systems-{n}");
                let s = chaos_in(
                    &seed,
                    cursor_in(&seed, "systems"),
                    side(false, json!([])),
                    json!({ "hand": [VANILLA], "field": [{ "def": VANILLA, "lane": 1 }] }),
                );
                let cast: Vec<String> = events_of(&s, "cardPlayed")
                    .iter()
                    .skip(1)
                    .filter_map(|event| event["defId"].as_str().map(str::to_string))
                    .collect();
                assert_eq!(cast.len() as i32, CHAOS_SCAPE_SYSTEMS, "{cast:?}");
                assert_ne!(cast[0], cast[1]);
                for def_id in &cast {
                    assert!(SYSTEMS.contains(&def_id.as_str()), "{def_id}");
                }
                for event in events_of(&s, "cardPlayed").iter().skip(1) {
                    assert_eq!(event["player"], json!("p1"));
                    assert_eq!(event["costPaid"], json!(0));
                }
                pairs.push(cast);
            }
            for system in SYSTEMS {
                assert!(pairs.iter().flatten().any(|id| id == system), "{system} never cast");
            }
        }

        #[test]
        fn entry_pieces_adds_four_starship_pieces_at_zero() {
            let s = chaos("pieces", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s, P1);
            assert_eq!(added.len() as i32, CHAOS_SCAPE_HAND_PIECES);
            for card in &added {
                assert!(is_piece(&card.def_id), "{}", card.def_id);
                assert_eq!(cost_of(&s, card), CHAOS_STINKYSCAPE_COST);
            }
        }

        #[test]
        fn r1882007_entry_interceptors_tops_each_unit_zone_a_unit_already_there_beneath() {
            let mut p1 = side(false, json!([]));
            p1["field"] = json!([{ "def": MENACE, "lane": 2 }]);
            let s = chaos("interceptors", p1, json!({ "hand": [VANILLA] }));
            for lane in 1..=UNIT_ZONES {
                assert_eq!(
                    s.unit(P1, lane).map(|unit| unit.def_id),
                    Some(INTERCEPTOR.to_string()),
                    "lane {lane}"
                );
            }
            assert_eq!(pile_defs(&s, P1, 2), vec![MENACE.to_string(), INTERCEPTOR.to_string()]);
            assert_eq!(pile_defs(&s, P1, 1), vec![INTERCEPTOR.to_string()]);
            // The enemy's side is untouched.
            assert!(s.unit(P2, 1).is_none());
        }

        #[test]
        fn r1882008_entry_launch_summons_a_starship_fuses_four_pieces_and_launches_it() {
            let s = chaos("launch", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let fused = events_of(&s, "starshipFused");
            assert_eq!(fused.len() as i32, CHAOS_SCAPE_FUSED_PIECES);
            for event in &fused {
                assert_eq!(event["player"], json!("p1"));
                assert!(is_piece(event["defId"].as_str().unwrap_or("")));
            }
            let launched = events_of(&s, "starshipLaunched");
            assert_eq!(launched.len(), 1);
            assert_eq!(launched[0]["player"], json!("p1"));
            assert_eq!(
                launched[0]["pieces"].as_array().map(Vec::len),
                Some(CHAOS_SCAPE_FUSED_PIECES as usize)
            );
            // The ship is summoned first and launched for nothing: no mana beyond this card's (4).
            assert_eq!(events_of(&s, "summoned")[0]["defId"], json!(DORMANT_STARSHIP));
            let ship = s
                .state()
                .players
                .p1
                .units
                .iter()
                .flatten()
                .flatten()
                .find(|card| card.starship.is_some())
                .cloned()
                .expect("a Starship");
            assert!(ship.starship.as_ref().is_some_and(|memory| memory.launched.is_some()));
        }

        #[test]
        fn r1882008_an_existing_dormant_starship_takes_four_more_pieces() {
            // A Dormant Starship already standing takes the four pieces: no second one is summoned.
            let mut p1 = side(false, json!([]));
            // A 0/0 hull is kept alive by the piece fused into it; here it stands on an X/X of its own.
            p1["field"] = json!([{
                "def": DORMANT_STARSHIP,
                "lane": 3,
                "slumber": {},
                "statsOverride": { "attack": 0, "health": 1 }
            }]);
            let mut s = scenario(json!({
                "seed": SEED,
                "p1": p1,
                "p2": { "hand": [VANILLA], "library": filler(6) },
            }));
            let ship = s.card(DORMANT_STARSHIP).id.clone();
            s.card_mut(&ship).starship.get_or_insert_with(Default::default);
            s.state_mut().rng_cursor = cursor_in(SEED, "launch");
            s.play(ID, json!({}));
            assert!(events_of(&s, "summoned").is_empty(), "no second Starship");
            let fused = events_of(&s, "starshipFused");
            assert_eq!(fused.len() as i32, CHAOS_SCAPE_FUSED_PIECES);
            assert!(fused.iter().all(|event| event["instanceId"] == json!(ship)));
            assert_eq!(events_of(&s, "starshipLaunched").len(), 1);
        }

        #[test]
        fn entry_weapons_adds_two_radiant_weapons_at_zero() {
            let s = chaos("weapons", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let added = added_to(&s, P1);
            assert_eq!(added.len() as i32, CHAOS_SCAPE_WEAPONS);
            for card in &added {
                assert!(crate::card_def(&card.def_id).tags.contains(&Tag::Weapon));
                assert!(card.radiant);
                assert_eq!(cost_of(&s, card), CHAOS_STINKYSCAPE_COST);
            }
        }

        #[test]
        fn r1882009_entry_market_offers_its_caster_alone_a_call_to_chaos_at_twenty() {
            let s = chaos("market", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let market = s.state().marketplace.as_ref().expect("the Marketplace opened");
            let lots: Vec<_> = market.offers.iter().filter(|offer| offer.owner.is_some()).collect();
            assert_eq!(lots.len(), 1);
            let lot = lots[0];
            assert_eq!(lot.owner, Some(P1));
            assert!(SHIPPED_EDITIONS.contains(&lot.def_id.as_str()), "{}", lot.def_id);
            assert_eq!((lot.base, lot.price), (CHAOS_SCAPE_LOT_PRICE, CHAOS_SCAPE_LOT_PRICE));
            assert_eq!(lot.radiant, None);
            // Its caster is offered it; the opponent is not.
            let mine = js(&s.view(P1).marketplace);
            let theirs = js(&s.view(P2).marketplace);
            assert!(mine.to_string().contains(&lot.def_id));
            assert!(
                !theirs["offers"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|offer| offer["defId"] == json!(lot.def_id))
            );
        }

        #[test]
        fn entry_recast_casts_a_shipped_edition() {
            let s = chaos("recast", side(false, json!([])), json!({ "hand": [VANILLA] }));
            let cast: Vec<Value> = events_of(&s, "cardPlayed").into_iter().skip(1).collect();
            assert!(!cast.is_empty());
            assert_eq!(cast[0]["costPaid"], json!(0));
            assert!(SHIPPED_EDITIONS.contains(&cast[0]["defId"].as_str().unwrap_or("")));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r423_three_different_entries_in_list_order() {
            let cursor =
                cursor_where(SEED, |at| rolled_names(SEED, at, true) == ["corvettes", "pieces", "weapons"]);
            let s = chaos_in(SEED, cursor, side(true, json!([])), json!({ "hand": [VANILLA] }));
            assert_eq!(
                events_of(&s, "chaosRolled")[0]["effects"],
                json!([label_of("corvettes"), label_of("pieces"), label_of("weapons")])
            );
            assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(CORVETTE.to_string()));
            let added = added_to(&s, P1);
            assert_eq!(added.len() as i32, CHAOS_SCAPE_HAND_PIECES + CHAOS_SCAPE_WEAPONS);
            let order: Vec<&str> = subsystems::CHAOS_SCAPE_EFFECTS.iter().map(|entry| entry.name).collect();
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
