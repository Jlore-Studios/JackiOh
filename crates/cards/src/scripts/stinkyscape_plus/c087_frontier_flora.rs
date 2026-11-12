//! SPEC §8.12 S+ #87 Frontier Flora (Field Spell, cost 1, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Animated / Evolves when a Starship is launched. / Evolve: Place {counters} Plague Counters on this. /
//!            Aura: Has 3X/3X stats for each Plague Counter." (2 counters)
//!   Radiant: the same, 4 counters.
//!
//! R3542000: it prints no stats, so as a Field Spell it enters the field and stays one while it carries no
//! Plague Counter: its own text gives it a stat line (`Script.stat_line_when`) only while it carries one.
//! "Evolves when a Starship is launched" is its own trigger on `starshipLaunched`, either player's launch
//! (R3536001); its Evolve text places the declared `counters` on it (`place_plague`, R471), and the first
//! counter owes it its animation at the state check (R2280001): it steps into a unit zone as a Unit, an
//! entry, with 3 Attack and 3 Health per counter (a self aura, §10.4 layer 5; the 3 is a term of a
//! formula, `tests/cross/params.rs`). With no open unit zone it stays a Field Spell, owed nothing more.
//! Once animated it stays a Unit, and losing every counter leaves a 0/0 that dies. It evolves once
//! (R3536000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-087";

/// "3X/3X stats for each Plague Counter": the 3 of that formula.
const STATS_PER_COUNTER: i32 = 3;

fn flora() -> Script {
    Script {
        triggers: vec![TriggerDef::new(
            "flora-launch",
            &[GameEventType::StarshipLaunched],
            |_ctx, _event| vec![evolve(json_as(json!({ "target": { "of": "self" } })))],
        )],
        evolve: Some(hook(|ctx| {
            vec![place_plague(json_as(json!({ "amount": param(&*ctx, "counters") })))]
        })),
        stat_line_when: Some(read_hook(|args| plague_on(args.self_) > 0)),
        aura: Some(aura_hook(|args| {
            let counters = plague_on(args.self_);
            if counters == 0 {
                return Vec::new();
            }
            let id = args.self_.id.clone();
            vec![AuraEntry {
                applies: Box::new(move |card: &CardInstance| card.id == id),
                mod_: StatMod {
                    attack: Some(counters * STATS_PER_COUNTER),
                    max_health: Some(counters * STATS_PER_COUNTER),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script serves both faces: the Radiant face's 4 counters are the declared `counters` number.
    let base = flora();
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S+ #87 Frontier Flora — SPEC §8.12 row 87 (R3542000; R383, R471, R2280000, R2280001, R3536000, R3536001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::effects::place_plague;
    use jackioh_engine::testkit::*;

    use crate::{built_starship, launch_starship, run_as};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: Starship Piece, keywords only
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    /// p1's Starship and a Flora (Radiant if `radiant`) in the backrow lane 2 of `owner`; `p2_field` is p2's board.
    fn board(seed: &str, radiant: bool, owner: PlayerId, p2_field: Value) -> (Scenario, String) {
        let flora = json!([{ "def": ID, "lane": 2, "radiant": radiant }]);
        let (p1, p2) = if owner == P1 {
            (json!({ "backrow": flora, "hand": [ID] }), json!({ "field": p2_field }))
        } else {
            (json!({ "hand": [ID] }), json!({ "backrow": flora, "field": p2_field }))
        };
        built_starship(seed, json!([{ "def": AP_ROUNDS, "lane": 1 }]), p1, p2)
    }

    fn animated(s: &Scenario) -> usize {
        s.events().iter().filter(|event| matches!(event, GameEvent::Animated { .. })).count()
    }

    #[test]
    fn r3542000_played_it_stays_a_field_spell() {
        let mut s = crate::scenario(json!({
            "p1": { "mana": 10, "hand": [ID], "library": crate::filler(3) },
            "p2": { "hand": crate::filler(1), "library": crate::filler(3) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        let flora = s.backrow(P1, 2).expect("it stays in the backrow");
        assert_eq!(flora.def_id, ID);
        assert!(s.unit(P1, 2).is_none());
        assert_eq!(animated(&s), 0);
    }

    mod base {
        use super::*;

        #[test]
        fn r3542000_a_launch_evolves_it_and_it_animates_as_a_6_6() {
            let (mut s, ship) = board("s087-base", false, P1, json!([]));
            let flora = s.backrow(P1, 2).expect("the Flora");
            launch_starship(&mut s, &ship);
            let card = s.card(&flora).clone();
            assert_eq!(card.evolved, Some(true));
            assert_eq!(evolutions_of(s.state(), P1), 1);
            assert_eq!(plague_on(&card), 2);
            assert!(matches!(card.zone, Zone::Field { row: Row::Units, lane: 2, .. }), "{:?}", card.zone);
            assert_eq!(animated(&s), 1);
            s.expect_stats(&flora, json!({ "attack": 6, "health": 6 }));
        }

        #[test]
        fn each_counter_gives_it_3_3() {
            let (mut s, ship) = board("s087-counters", false, P1, json!([]));
            let flora = s.backrow(P1, 2).expect("the Flora");
            launch_starship(&mut s, &ship);
            run_as(
                &mut s,
                place_plague(json_as(json!({ "amount": 1, "target": { "of": "instance", "instanceId": flora.id } }))),
                P1,
            );
            s.expect_stats(&flora, json!({ "attack": 9, "health": 9 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3542000_4_counters_a_12_12() {
            let (mut s, ship) = board("s087-radiant", true, P1, json!([]));
            let flora = s.backrow(P1, 2).expect("the Flora");
            launch_starship(&mut s, &ship);
            assert_eq!(plague_on(s.card(&flora)), 4);
            s.expect_stats(&flora, json!({ "attack": 12, "health": 12 }));
        }
    }

    #[test]
    fn either_players_launch_evolves_it() {
        let (mut s, ship) = board("s087-theirs", false, P2, json!([]));
        let flora = s.backrow(P2, 2).expect("the Flora");
        launch_starship(&mut s, &ship);
        assert_eq!(s.card(&flora).evolved, Some(true));
        assert_eq!(evolutions_of(s.state(), P2), 1);
        assert_eq!(evolutions_of(s.state(), P1), 0);
        s.expect_stats(&flora, json!({ "attack": 6, "health": 6 }));
    }

    #[test]
    fn r3542000_with_every_unit_zone_full_it_stays_with_its_counters() {
        let (mut s, ship) = board("s087-full", false, P2, json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]));
        let flora = s.backrow(P2, 2).expect("the Flora");
        launch_starship(&mut s, &ship);
        let card = s.card(&flora).clone();
        assert_eq!(plague_on(&card), 2);
        assert!(matches!(card.zone, Zone::Field { row: Row::Backrow, lane: 2, .. }), "{:?}", card.zone);
        assert_eq!(animated(&s), 0);
    }
}
