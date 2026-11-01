//! S #137 Starfleet Commander (SPEC §8.10 row 137; R1, R70, R113, R1839002, R1852003, R1852005,
//! R1852006, R1857002, R1859006): (4) Unit, Human, Legendary, 9/9 → 18/18, First Strike, a card of the
//! Stinkyscape set. It was Firststrike Capability, a Common with no tag, until #2521 renamed it; its
//! Cry is unchanged, and its file keeps the old name.
//!
//! Base:    "First Strike\nCry: If you have a Dormant Starship, launch it, then Discover a War Doctrine and
//!          cast it."
//! Radiant: "First Strike\nCry: If you have a Dormant Starship, launch it, then Discover a Radiant War
//!          Doctrine and cast it."
//! Engine: Cry (R1): with a Dormant Starship (`building_a_ship`, R1852005), `awaken` it, which launches
//! it for nothing, spending no launch price (R1839002, R1852003, R1852006); then a Discover (§6.3) of
//! three of the four War Doctrines, S #137.1 to S #137.4, named by id (the pool R1857002 names, which a
//! held set's gate lets in), and a `cast_new` of the pick, free and counted as played (R70), Radiant on
//! the Radiant face. The Doctrine resolves inside the Cry, before the Starship's fused When launched
//! answers the launch (R113), so its Starship line reads the Starship before those (R1859006). With no
//! Dormant Starship, nothing: no launch and no Discover. Declares no numbers. The Discover's prompt names
//! the card as the game's catalog version spells it (`def_of`, R767000), so a replay of a game before
//! the rename asks as it asked then. S #176 Preemptive Strike casts the same effect as a Spell.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-137";

/// The four War Doctrines, S #137.1 to S #137.4: the Discover's pool (R1857002).
const WAR_DOCTRINES: [&str; 4] = [
    "stinkyscape-137-1",
    "stinkyscape-137-2",
    "stinkyscape-137-3",
    "stinkyscape-137-4",
];

/// The resume step the Discover's answer comes back to (§10.6).
const PICKED: &str = "picked";

fn firststrike(radiant: bool) -> Script {
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
        base: firststrike(false),
        radiant: firststrike(true),
    }
}

// S #137 Starfleet Commander — SPEC §8.10 row 137, BUILD M12 row S 137: "with a Dormant Starship, its
// Cry launches it free (its fused Cries, then the Doctrine before the When launched, R1859006; no launch
// price spent, R1852006), then Discovers one of the four War Doctrines (three offered) and casts it for
// its controller (R70); with none, nothing, no Discover; radiant 18/18 and the Doctrine Radiant".
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

    /// p1's Starship (an Ultralight Chassis and an Ultra Capacitor) and Starfleet Commander (Radiant
    /// if `radiant`) in hand, played into lane 4.
    fn played(seed: &str, radiant: bool) -> (Scenario, String) {
        let (mut s, ship) = built_starship(
            seed,
            json!([{ "def": CHASSIS, "lane": 1 }, { "def": CAPACITOR, "lane": 2 }]),
            json!({ "hand": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        );
        let mana = s.state().players.p1.mana.current;
        s.play(ID, json!({ "zone": 4 }));
        assert_eq!(
            s.state().players.p1.mana.current,
            mana - 4,
            "the launch costs nothing beside the card"
        );
        (s, ship)
    }

    #[test]
    fn r1859006_with_a_dormant_starship_its_cry_launches_it_then_discovers_three_war_doctrines() {
        let (s, ship) = played("r1859006-launch", false);
        assert!(is_launched_starship(s.card(ship.as_str())));
        assert!(!building_a_ship(s.state(), P1));
        let prompt = &s.state().pending.as_ref().expect("the Discover").prompt;
        assert!(prompt.starts_with("Starfleet Commander:"), "{prompt}");
        let options = offered(&s);
        assert_eq!(options.len(), 3, "{options:?}");
        assert!(options.iter().all(|option| WAR_DOCTRINES.contains(&option.as_str())));
        let mut distinct = options.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 3);
    }

    #[test]
    fn r1859006_the_pick_is_cast_free_before_the_starships_when_launched() {
        for radiant in [false, true] {
            let (mut s, ship) = played("r1859006-cast", radiant);
            let max_health = s.stats(ship.as_str()).max_health;
            let mana = s.state().players.p1.mana.current;
            let pick = offered(&s)[0].clone();
            s.answer(json!(pick));
            assert_eq!(s.state().players.p1.mana.current, mana, "the cast is free");
            let types: Vec<GameEventType> = s.last_events().iter().map(GameEvent::event_type).collect();
            let cast = types
                .iter()
                .position(|kind| *kind == GameEventType::CardResolved)
                .expect("the Doctrine resolved");
            let buffed = types
                .iter()
                .rposition(|kind| *kind == GameEventType::Buffed)
                .expect("the Capacitor's When launched");
            assert!(cast < buffed, "the Doctrine first, then the When launched: {types:?}");
            assert_eq!(s.stats(ship.as_str()).max_health, max_health + 1);
            let doctrine = s
                .state()
                .players
                .p1
                .graveyard
                .iter()
                .find(|card| card.def_id == pick)
                .expect("the Doctrine in the graveyard")
                .clone();
            assert_eq!(doctrine.radiant, radiant);
            assert_eq!(s.stats(s.unit_at(P1, 4).id.as_str()).attack, if radiant { 18 } else { 9 });
        }
    }

    #[test]
    fn radiant_18_18_and_the_doctrine_cast_radiant() {
        let (mut s, _ship) = played("s137-radiant", true);
        assert_eq!(s.stats(s.unit_at(P1, 4).id.as_str()).attack, 18);
        assert_eq!(s.stats(s.unit_at(P1, 4).id.as_str()).health, 18);
        let pick = offered(&s)[0].clone();
        s.answer(json!(pick));
        let doctrine = s
            .state()
            .players
            .p1
            .graveyard
            .iter()
            .find(|card| card.def_id == pick)
            .expect("the Doctrine in the graveyard")
            .clone();
        assert!(doctrine.radiant, "the Doctrine is cast Radiant");
    }

    #[test]
    fn r1859006_the_launch_spends_no_launch_price() {
        let (mut s, _ship) = built_starship(
            "r1859006-price",
            json!([{ "def": CHASSIS, "lane": 1 }]),
            json!({ "hand": [ID] }),
            json!({}),
        );
        s.state_mut().players.p1.mods.push(PlayerModifier {
            id: "liftoff".to_string(),
            expiry: ModifierExpiry::Used,
            kind: ModifierKind::StarshipLaunchCost { cost: 0 },
        });
        s.play(ID, json!({ "zone": 4 }));
        assert_eq!(s.state().players.p1.mods.len(), 1, "the price stays for a later launch");
    }

    #[test]
    fn r1859006_with_no_dormant_starship_it_does_nothing() {
        let mut s = crate::scenario(json!({
            "seed": "r1859006-none",
            "active": "p1",
            "p1": { "hand": [ID, VANILLA], "mana": 10, "library": crate::filler(4) },
            "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        assert!(s.state().pending.is_none(), "no Discover");
        assert!(
            !s.last_events()
                .iter()
                .any(|event| event.event_type() == GameEventType::StarshipLaunched)
        );
    }
}
