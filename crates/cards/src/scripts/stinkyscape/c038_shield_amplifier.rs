//! SPEC §8.10 Stinkyscape 38 Shield Amplifier, a card of the held Stinkyscape set (R1420).
//! (1) Unit, Common, 1/1 → 2/2: Starship Piece, Equip, Divine Shield.
//! "Whenever this survives an attack, it gains +{buff}/+{buff}." — buff 3, Radiant 6 (R1860000).
//!
//! Engine: Equip (R1086): played onto one of your Units it fuses into it, the host keeping its zone
//! and taking its stats, keywords and text (R77, R102), Starship Piece only into a piece (R1852001);
//! played alone its death fuses a copy into its owner's Dormant Starship (R1852000); an equipped
//! host gains the text and the Divine Shield. The survival is read by two hooks: `after_attack`,
//! after an attack it made, when its facts say `survived`, and `after_attacked` (R1026), after an
//! attack made on it, declared or forced, when it still stands after that combat's state check —
//! read as the absence of `SELF_KEY` in the hook's data (present means the defender died and runs
//! on its snapshot, R78, R89); a Cleave splash is no attack on it. Then `buff` +N/+N on itself,
//! lasting. The Radiant face runs the same script: its numbers are the declared `buff` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-038";

fn amplifier() -> Script {
    Script {
        after_attack: Some(hook(|ctx| {
            let survived = after_attack_of(ctx).is_some_and(|facts| facts.survived);
            if !survived {
                return vec![];
            }
            let n = param(ctx, "buff");
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": n,
                "health": n,
            })))]
        })),
        after_attacked: Some(hook(|ctx| {
            if ctx.data.contains_key(SELF_KEY) {
                return vec![];
            }
            let n = param(ctx, "buff");
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": n,
                "health": n,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's +6/+6 is its declared `buff`, which `param` reads off the
    // running face; its stats are the catalog's.
    let base = amplifier();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// Stinkyscape #38 Shield Amplifier — SPEC §8.10 row 38, BUILD M12 row S 38: "survives an attack it
// made: +3/+3; survives one made on it, its Divine Shield taking the hit: +3/+3; dies in the
// combat: nothing; a Cleave splash it survives: nothing; equipped onto a Unit, the host gains the
// text and Divine Shield and stays no piece unless it was one (R1852001); buff reads through
// `param()`; radiant 2/2 and +6/+6".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const AMPLIFIER: &str = "stinkyscape-038";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const CLEAVER: &str = "classic-073"; // Nurse Cleaver 3/6, Rush, Cleave, Lifesteal.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn open() -> PreviewGuard {
        preview_sets(&[SetName::Stinkyscape])
    }

    /// The `buffed` events on `id`, in order.
    fn buffed_on(s: &Scenario, id: &str) -> usize {
        s.events()
            .iter()
            .filter(|event| {
                matches!(event, GameEvent::Buffed { instance_id, .. } if instance_id == id)
            })
            .count()
    }

    fn buffs_of(s: &Scenario, card: &str) -> (i32, i32) {
        let stats = s.stats(card);
        let def = crate::card_def(AMPLIFIER);
        let base = if s.card(card).radiant { &def.radiant } else { &def.base };
        let (printed_attack, printed_health) = (
            base.attack.unwrap_or(0),
            base.health.unwrap_or(0),
        );
        (stats.attack - printed_attack, stats.health - printed_health)
    }

    mod base {
        use super::*;

        #[test]
        fn r1860000_survives_an_attack_it_made_gains_3_3() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "p1": { "hand": [AMPLIFIER, ANCHOR], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(AMPLIFIER, json!({ "zone": 1 }));
            // Past its summoning sickness: a full round, then it attacks the hero and survives.
            s.end_turn();
            s.end_turn();
            s.attack(AMPLIFIER, "hero");

            // One hit of 1 from the Amplifier, and nothing else: both libraries keep every
            // turn's draws from running dry.
            assert_eq!(buffs_of(&s, AMPLIFIER), (3, 3));
            s.expect_health(PlayerId::P2, 29);
        }

        #[test]
        fn r1860000_survives_one_made_on_it_its_divine_shield_taking_the_hit_gains_3_3() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "hand": [ANCHOR], "field": [AMPLIFIER], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
            }));
            s.attack(VANILLA, AMPLIFIER);

            // The 4-attack hit took the Divine Shield instead: the Amplifier stands at 1/1.
            s.expect_in_zone(AMPLIFIER, "field");
            assert_eq!(buffs_of(&s, AMPLIFIER), (3, 3));
        }

        #[test]
        fn r1860000_dies_in_the_combat_nothing() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [AMPLIFIER], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "field": [MENACE], "library": filler(4) },
            }));
            // First combat: the 9-attack strike back takes the Divine Shield, so it survives
            // and gains +3/+3, standing 4/4 with no shield.
            let id = s.card(AMPLIFIER).id.clone();
            s.attack(AMPLIFIER, MENACE);
            s.expect_in_zone(AMPLIFIER, "field");
            assert_eq!(buffed_on(&s, &id), 1);
            // Next turn it attacks again and dies: the hook reads `survived` false and buffs
            // nothing, so no second buff lands.
            s.end_turn();
            s.end_turn();
            s.attack(AMPLIFIER, MENACE);

            s.expect_in_zone(AMPLIFIER, "graveyard");
            assert_eq!(buffed_on(&s, &id), 1);
            // Its Death fused a copy into the Dormant Starship, which now stands (R1852000).
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
            assert_eq!(
                starship_pieces_of(&ship.expect("the ship"))
                    .iter()
                    .map(|piece| piece.def_id.as_str())
                    .collect::<Vec<_>>(),
                vec![AMPLIFIER]
            );
        }

        #[test]
        fn r1860000_a_cleave_splash_it_survives_is_no_attack_on_it() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "seed": "amplifier-cleave",
                "active": "p2",
                "p1": {
                    "hand": [ANCHOR],
                    "field": [{ "def": AMPLIFIER, "lane": 1 }, { "def": VANILLA, "lane": 2 }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "field": [{ "def": CLEAVER, "lane": 2 }], "library": filler(4) },
            }));
            let cleaver = s.unit(PlayerId::P2, 2).expect("cleaver").clone();
            let vanilla = s.unit(PlayerId::P1, 2).expect("vanilla").clone();
            s.attack(&cleaver, &vanilla);

            // The splash took the Divine Shield, but it was not attacked: no buff.
            assert!(s.card(AMPLIFIER).damage == 0, "the splash is absorbed by Divine Shield");
            assert_eq!(buffs_of(&s, AMPLIFIER), (0, 0));
        }

        #[test]
        fn r1852001_equipped_onto_a_unit_the_host_gains_the_text_and_divine_shield_and_stays_no_piece() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA, AMPLIFIER, ANCHOR], "library": filler(4) },
                "p2": { "hand": [ANCHOR] },
            }));
            s.play(VANILLA, json!({ "zone": 1 }));
            s.play(AMPLIFIER, json!({ "zone": 1, "equip": true }));

            // The host is kept: a 4/4 that is also a 1/1, with the text and Divine Shield.
            let host = s.unit(PlayerId::P1, 1).expect("the fused host");
            let stats = s.stats(host.id.as_str());
            assert_eq!((stats.attack, stats.health), (5, 5));
            let kinds: Vec<String> = stats
                .keywords
                .iter()
                .map(|keyword| keyword.kind().as_str().to_string())
                .collect();
            assert!(kinds.contains(&"Divine Shield".to_string()), "{kinds:?}");
            // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
            assert!(!kinds.contains(&"Starship Piece".to_string()), "{kinds:?}");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860000_radiant_survives_an_attack_it_made_gains_6_6() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": AMPLIFIER, "radiant": true }, ANCHOR], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(AMPLIFIER, json!({ "zone": 1 }));
            // Past its summoning sickness: a full round, then it attacks the hero and survives.
            s.end_turn();
            s.end_turn();
            s.attack(AMPLIFIER, "hero");

            assert_eq!(buffs_of(&s, AMPLIFIER), (6, 6));
            s.expect_health(PlayerId::P2, 28);
        }

        #[test]
        fn r1860000_radiant_dies_in_the_combat_nothing() {
            crate::register_all();
            let _preview = open();
            let mut s = scenario(json!({
                "p1": { "hand": [ANCHOR], "field": [{ "def": AMPLIFIER, "radiant": true }], "library": filler(4) },
                "p2": {
                    "hand": [ANCHOR],
                    "field": [{ "def": MENACE, "lane": 1 }, { "def": MENACE, "lane": 2 }],
                    "library": filler(4),
                },
            }));
            // First combat: the 9-attack strike back takes the Divine Shield, so it survives
            // and gains +6/+6, standing 8/8 with no shield.
            let id = s.card(AMPLIFIER).id.clone();
            let first = s.unit(PlayerId::P2, 1).expect("first Menace").clone();
            s.attack(AMPLIFIER, &first);
            s.expect_in_zone(AMPLIFIER, "field");
            assert_eq!(buffed_on(&s, &id), 1);
            // Next turn it hits the fresh 9/9: 8 leaves it at 1, and its 9-attack strike back
            // kills. The hook reads `survived` false and buffs nothing: no second buff lands.
            s.end_turn();
            s.end_turn();
            let second = s.unit(PlayerId::P2, 2).expect("second Menace").clone();
            s.attack(AMPLIFIER, &second);

            s.expect_in_zone(AMPLIFIER, "graveyard");
            assert_eq!(buffed_on(&s, &id), 1);
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
        }
    }
}
