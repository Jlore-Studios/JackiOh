//! S #176 Preemptive Strike (SPEC §8.10 row 176; R70, R113, R1839002, R1852003, R1852005, R1852006,
//! R1857002, R1859006): (2) Spell, Legendary, a card of the Stinkyscape set (#2521).
//!
//! Base:    "If you have a Dormant Starship, launch it, then Discover a War Doctrine and cast it."
//! Radiant: "If you have a Dormant Starship, launch it, then Discover a Radiant War Doctrine and cast it."
//! Engine: S #137 Starfleet Commander's Cry as a Spell, its resolution hanging off `cry` (Core #5
//! Stockpile's shape): with a Dormant Starship (`building_a_ship`, R1852005), `awaken` it, which
//! launches it for nothing, spending no launch price (R1839002, R1852003, R1852006); then a Discover
//! (§6.3) of three of the four War Doctrines, S #137.1 to S #137.4, named by id (R1857002), and a
//! `cast_new` of the pick, free and counted as played (R70), Radiant on the Radiant face. The Doctrine
//! resolves before the Starship's fused When launched answers the launch (R113, R1859006). With no
//! Dormant Starship, nothing: no launch and no Discover. The prompt names the card as the game's
//! catalog spells it (`def_of`). Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-176";

/// The four War Doctrines, S #137.1 to S #137.4: the Discover's pool (R1857002).
const WAR_DOCTRINES: [&str; 4] = [
    "stinkyscape-137-1",
    "stinkyscape-137-2",
    "stinkyscape-137-3",
    "stinkyscape-137-4",
];

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

fn preemptive_strike(radiant: bool) -> Script {
    let doctrine = if radiant { "a Radiant War Doctrine" } else { "a War Doctrine" };
    Script {
        cry: Some(hook(move |ctx| {
            let Some(ship) = dormant_starship_of(&*ctx.state, ctx.controller) else {
                return vec![];
            };
            let name = &def_of(Some(&*ctx.state), ID).name;
            let prompt = format!("{name}: Discover {doctrine} to cast");
            vec![
                awaken(json_as(json!({ "target": { "of": "instance", "instanceId": ship.id } }))),
                discover_from_catalog(json_as(json!({
                    "step": PICKED,
                    "query": { "defId": WAR_DOCTRINES },
                    "prompt": prompt,
                }))),
            ]
        })),
        resume: IndexMap::from([(
            PICKED,
            hook(move |ctx| {
                let Some(picked) = chosen_options(ctx).into_iter().next() else {
                    return vec![];
                };
                vec![cast_new(json_as(json!({ "def": picked, "radiant": radiant })))]
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: preemptive_strike(false),
        radiant: preemptive_strike(true),
    }
}

// S #176 Preemptive Strike — SPEC §8.10 row 176, BUILD M12 row S 176: "with a Dormant Starship, it
// launches it free (no launch price spent, R1852006), then Discovers one of the four War Doctrines (three
// offered) and casts it for its caster (R70), before the Starship's When launched (R1859006); with none,
// nothing, no Discover; radiant: the Doctrine Radiant".
#[cfg(test)]
mod tests {
    use super::{ID, WAR_DOCTRINES};
    use crate::built_starship;
    use jackioh_engine::prelude::{building_a_ship, is_launched_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const CHASSIS: &str = "stinkyscape-048"; // (1) 4/1 Starship Piece, Equip, Rush: keywords only.
    const CAPACITOR: &str = "stinkyscape-133"; // When launched: +1/+1 per other piece.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    /// Its printed cost, (2).
    const COST: i32 = 2;

    /// The def ids the open Discover offers.
    fn offered(s: &Scenario) -> Vec<String> {
        s.state()
            .pending
            .as_ref()
            .expect("the Discover")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    /// p1's Starship (an Ultralight Chassis and an Ultra Capacitor), and Preemptive Strike (Radiant if
    /// `radiant`) cast from hand.
    fn cast(seed: &str, radiant: bool) -> (Scenario, String) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": CHASSIS, "lane": 1 }, { "def": CAPACITOR, "lane": 2 }]),
            json!({ "hand": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        );
        let mana = s.state().players.p1.mana.current;
        s.play(ID, json!({}));
        assert_eq!(
            s.state().players.p1.mana.current,
            mana - COST,
            "the launch costs nothing beside the card"
        );
        (s, ship)
    }

    /// The Doctrine `pick` in p1's graveyard once cast.
    fn cast_doctrine(s: &Scenario, pick: &str) -> CardInstance {
        s.state()
            .players
            .p1
            .graveyard
            .iter()
            .find(|card| card.def_id == pick)
            .expect("the Doctrine in the graveyard")
            .clone()
    }

    mod base {
        use super::*;

        #[test]
        fn r1859006_with_a_dormant_starship_it_launches_it_then_discovers_three_war_doctrines() {
            let (s, ship) = cast("s176-launch", false);
            assert!(is_launched_starship(s.card(ship.as_str())));
            assert!(!building_a_ship(s.state(), P1));
            let prompt = &s.state().pending.as_ref().expect("the Discover").prompt;
            assert!(prompt.starts_with("Preemptive Strike:"), "{prompt}");
            let options = offered(&s);
            assert_eq!(options.len(), DISCOVER_OPTIONS as usize, "{options:?}");
            assert!(options.iter().all(|option| WAR_DOCTRINES.contains(&option.as_str())));
            let mut distinct = options.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(distinct.len(), options.len());
        }

        #[test]
        fn r1859006_the_pick_is_cast_free_before_the_starships_when_launched() {
            let (mut s, ship) = cast("s176-cast", false);
            let max_health = s.stats(ship.as_str()).max_health;
            let mana = s.state().players.p1.mana.current;
            let pick = offered(&s)[0].clone();
            s.answer(json!(pick));
            assert_eq!(s.state().players.p1.mana.current, mana, "the cast is free");
            let events = s.last_events();
            let resolved = events
                .iter()
                .position(|event| matches!(event, GameEvent::CardResolved { def_id, .. } if *def_id == pick))
                .expect("the Doctrine resolved");
            let buffed = events
                .iter()
                .rposition(|event| event.event_type() == GameEventType::Buffed)
                .expect("the Capacitor's When launched");
            assert!(resolved < buffed, "the Doctrine first, then the When launched: {events:?}");
            assert_eq!(s.stats(ship.as_str()).max_health, max_health + 1);
            assert!(!cast_doctrine(&s, &pick).radiant);
        }

        #[test]
        fn r1859006_with_no_dormant_starship_it_does_nothing() {
            let mut s = crate::scenario(json!({
                "seed": "s176-none",
                "active": "p1",
                "p1": { "hand": [ID, VANILLA], "mana": 10, "library": crate::filler(4) },
                "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
            }));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none(), "no Discover");
            assert!(
                !s.last_events()
                    .iter()
                    .any(|event| event.event_type() == GameEventType::StarshipLaunched)
            );
            assert!(
                !s.last_events().iter().any(|event| matches!(
                    event,
                    GameEvent::CardResolved { def_id, .. } if WAR_DOCTRINES.contains(&def_id.as_str())
                )),
                "no Doctrine cast"
            );
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1859006_it_launches_the_starship_and_casts_a_radiant_doctrine() {
            let (mut s, ship) = cast("s176-radiant", true);
            assert!(is_launched_starship(s.card(ship.as_str())));
            let prompt = &s.state().pending.as_ref().expect("the Discover").prompt;
            assert!(prompt.contains("Radiant War Doctrine"), "{prompt}");
            let pick = offered(&s)[0].clone();
            s.answer(json!(pick));
            assert!(cast_doctrine(&s, &pick).radiant, "the Doctrine is cast Radiant");
        }

        #[test]
        fn with_no_dormant_starship_it_does_nothing() {
            let mut s = crate::scenario(json!({
                "seed": "s176-radiant-none",
                "active": "p1",
                "p1": { "hand": [{ "def": ID, "radiant": true }, VANILLA], "mana": 10, "library": crate::filler(4) },
                "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
            }));
            s.play(ID, json!({}));
            assert!(s.state().pending.is_none(), "no Discover");
        }
    }
}
