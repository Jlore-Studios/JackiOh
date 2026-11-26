//! S #176 Preemptive Strike (SPEC §8.10 row 176; R70, R113, R1839002, R1852003, R1852005, R1852006,
//! R1857002, R1859006, R3869001, R5106000): (2) Spell, Legendary, a card of the Stinkyscape set (#2521).
//!
//! Base:    "If you have a Dormant Starship, launch it, then Discover a War Doctrine and cast it."
//! Radiant: "If you have a Dormant Starship, launch it, then Discover a Radiant War Doctrine and cast it."
//! Engine: S #137 Starfleet Commander's Cry as a Spell, its resolution hanging off `cry` (Core #5
//! Stockpile's shape): with a Dormant Starship (`building_a_ship`, R1852005), `awaken` it, which
//! launches it for nothing, spending no launch price (R1839002, R1852003, R1852006); then a Discover
//! (§6.3) of three of the four War Doctrines, S #137.1 to S #137.4 (R1857002), the War Doctrine pool
//! (`query::war_doctrines`, R3869001, which named them by id through v0.6.2b), and a
//! `cast_new` of the pick, free and counted as played (R70), Radiant on the Radiant face. The Doctrine
//! resolves before the Starship's fused When launched answers the launch (R113, R1859006). With no
//! Dormant Starship, nothing: no launch and no Discover. R4189000: in hand the card glows while
//! its controller has a Dormant Starship. The prompt names the card as the game's
//! catalog spells it (`def_of`). Declares no numbers.
//!
//! R5106000: the text is S #137's (`crate::war_doctrine`): the Dormant Starship read once, as the
//! Spell begins, so a fused Cry that asks during the launch (S #40 Poison Capsule's placement) is
//! answered and the Discover follows. Before issue #5106 (`script_before_launch_read_once`, R767000)
//! the rebuild after that answer found the Starship launched, and the Discover was lost.

use jackioh_engine::prelude::*;

use crate::war_doctrine::{PICKED, Read, cast_the_pick, doctrine_text, launch_then_discover};

pub const ID: &str = "stinkyscape-176";

fn preemptive_strike(radiant: bool, read: Read) -> Script {
    let doctrine = doctrine_text(radiant);
    Script {
        // R4189000: in hand the card glows while its controller has a Dormant Starship, the same
        // fact the Cry branches on.
        condition_met: Some(condition_hook(|ctx: ConditionContext<'_>| {
            ctx.zone == ConditionZone::Hand
                && dormant_starship_of(ctx.state, ctx.controller).is_some()
        })),
        cry: Some(hook(move |ctx| {
            let name = &def_of(Some(&*ctx.state), ID).name;
            let prompt = format!("{name}: Discover {doctrine} to cast");
            launch_then_discover(ctx, prompt, read)
        })),
        resume: IndexMap::from([(PICKED, cast_the_pick(radiant))]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: preemptive_strike(false, Read::Once),
        radiant: preemptive_strike(true, Read::Once),
    }
}

/// R767000: what it ran before issue #5106, its Dormant Starship read again each time the Spell's
/// list was built, so a fused Cry that asked during the launch lost the Discover (R5106000).
pub fn script_before_launch_read_once() -> CardScripts {
    CardScripts {
        base: preemptive_strike(false, Read::EachBuild),
        radiant: preemptive_strike(true, Read::EachBuild),
    }
}

// S #176 Preemptive Strike — SPEC §8.10 row 176, BUILD M12 row S 176 (R1852006, R70, R1859006).
#[cfg(test)]
mod tests {
    use super::ID;

    /// R3869001: the War Doctrine pool today, S #137.1 to S #137.4, pinned so a change to it is
    /// deliberate: the four this card named by id through v0.6.2b.
    const WAR_DOCTRINES: [&str; 4] = [
        "stinkyscape-137-1",
        "stinkyscape-137-2",
        "stinkyscape-137-3",
        "stinkyscape-137-4",
    ];
    use crate::built_starship;
    use jackioh_engine::prelude::{building_a_ship, is_launched_starship};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const CHASSIS: &str = "stinkyscape-048"; // Ultralight Chassis: Starship Piece, Rush: keywords only
    const CAPACITOR: &str = "stinkyscape-133"; // Ultra Capacitor: When launched: +attack/+health per other piece.
    const CAPSULE: &str = "stinkyscape-040"; // Poison Capsule: Starship Piece, Cry: place Plague Counters (asks where).
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    /// Its printed cost, (2).
    const COST: i32 = 2;

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

    /// p1's Starship fused from a Poison Capsule and an Ultralight Chassis, and Preemptive Strike cast
    /// from hand: the launch runs the Capsule's fused Cry, which asks where its counters go, and the
    /// answer puts them on the Starship.
    fn cast_with_a_capsule(seed: &str) -> (Scenario, String) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": CAPSULE, "lane": 1 }, { "def": CHASSIS, "lane": 2 }]),
            json!({ "hand": [ID] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        );
        s.play(ID, json!({}));
        assert!(is_launched_starship(s.card(ship.as_str())));
        let prompt = &s.state().pending.as_ref().expect("the Capsule's placement").prompt;
        assert!(!prompt.starts_with("Preemptive Strike:"), "the placement asks first: {prompt}");
        s.answer(json!(ship));
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
            let options = s.prompt_modes();
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
            let pick = s.prompt_modes()[0].clone();
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

        /// R5106000: the Dormant Starship is read once, as the Spell begins, so a fused Cry that asks
        /// during the launch delays the Discover and loses nothing.
        #[test]
        fn r5106000_a_fused_cry_that_asks_during_the_launch_is_answered_then_the_discover_opens() {
            let (mut s, ship) = cast_with_a_capsule("r5106000-s176");
            assert_eq!(s.card(ship.as_str()).counters.plague, Some(2), "the Capsule's counters");
            let prompt = &s.state().pending.as_ref().expect("the Discover").prompt;
            assert!(prompt.starts_with("Preemptive Strike:"), "{prompt}");
            let options = s.prompt_modes();
            assert!(options.iter().all(|option| WAR_DOCTRINES.contains(&option.as_str())), "{options:?}");
            let pick = options[0].clone();
            s.answer(json!(pick));
            assert!(!cast_doctrine(&s, &pick).radiant);
            s.expect_in_zone(ID, "graveyard");
        }

        /// R767000: the script kept for the replays from before issue #5106 rebuilt its list after
        /// the Capsule's answer, found the Starship launched, and lost the Discover (R5106000).
        #[test]
        fn r767000_r5106000_the_kept_script_loses_the_discover_after_a_fused_cry_asks() {
            crate::under_kept(ID, super::super::script_before_launch_read_once(), || {
                let (mut s, _ship) = cast_with_a_capsule("r767000-s176-kept");
                assert!(s.state().pending.is_none(), "no Discover");
                s.expect_in_zone(ID, "graveyard");
            });
        }

        /// p1's Starship built and Preemptive Strike (Radiant if `radiant`) in hand, unplayed.
        fn holding(seed: &str, radiant: bool) -> (Scenario, String) {
            built_starship(
                seed,
                json!([{ "def": CHASSIS, "lane": 1 }, { "def": CAPACITOR, "lane": 2 }]),
                json!({ "hand": [{ "def": ID, "radiant": radiant }] }),
                json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
            )
        }

        /// R4189000: with a Dormant Starship the Strike glows in hand, and the cast it
        /// promised launches the Starship and opens the Discover.
        #[test]
        fn r4189000_glows_with_a_dormant_starship_and_the_cast_then_launches_it() {
            for radiant in [false, true] {
                let (mut s, ship) = holding(&format!("r4189000-glow-{radiant}"), radiant);
                let strike = s.card(ID).id.clone();
                assert!(hand_glows(&s, &strike, P1), "radiant {radiant}");
                assert!(
                    !serde_json::to_string(&s.view(PlayerId::P2)).unwrap().contains("conditionActive"),
                    "the opponent never sees the flag"
                );
                s.play(ID, json!({}));
                assert!(is_launched_starship(s.card(ship.as_str())), "radiant {radiant}");
                assert!(s.state().pending.is_some(), "the Discover opens");
            }
        }

        /// R4189000: with no Dormant Starship it stays dark, and the cast does nothing.
        #[test]
        fn r4189000_dark_without_a_dormant_starship_and_the_cast_then_does_nothing() {
            for radiant in [false, true] {
                let mut s = crate::scenario(json!({
                    "seed": format!("r4189000-dark-{radiant}"),
                    "active": "p1",
                    "p1": { "hand": [{ "def": ID, "radiant": radiant }, VANILLA], "mana": 10, "library": crate::filler(4) },
                    "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
                }));
                let strike = s.card(ID).id.clone();
                assert!(!hand_glows(&s, &strike, P1), "radiant {radiant}");
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
            }
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
            let pick = s.prompt_modes()[0].clone();
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
