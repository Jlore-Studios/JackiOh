//! SPEC §8.10 Stinkyscape 134 Felinor Containment Chamber, a card of the Stinkyscape
//! set. (1) Unit, Common, 2/1 → 4/2: Starship Piece.
//! "Death: Summon a Felinor Token with this Unit's Attack and Health." / Radiant: "Death: Summon
//! a Felinor Token with twice this Unit's Attack and Health. It gains Divine Shield." (R1860007).
//!
//! Engine: Death (§6.2), on the snapshot it died with (R89): `summon` the Felinor Token (§7) with
//! a `statsOverride` of its Attack and its maximum Health then — its current Health is 0 or less
//! as it died — buffs included, into the leftmost open unit zone (R64), before its own copy fuses
//! into the Dormant Starship (R1852000), which may then stack on lane 3 if the token took the last
//! zone; the Radiant face doubles both and grants Divine Shield through `summon`'s `keywords`

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-134";

/// §7's shared Felinor Token. Read through `card_def` so a wrong id fails as the script is built,
/// not in play.
fn felinor_token() -> String {
    crate::card_def("core-t-felinor").id
}

fn chamber(doubled: bool, shielded: bool) -> Script {
    let token = felinor_token();
    Script {
        death: Some(hook(move |ctx| {
            let Some(this) = ctx.self_.clone() else {
                return vec![];
            };
            let view = unit_view(ctx.state, &this);
            let attack = if doubled { view.attack * 2 } else { view.attack };
            let health = if doubled {
                view.max_health * 2
            } else {
                view.max_health
            };
            if health <= 0 {
                return vec![];
            }
            let keywords = if shielded {
                json!([{ "kind": "Divine Shield" }])
            } else {
                json!([])
            };
            vec![summon(json_as(json!({
                "defId": token,
                "player": "self",
                "statsOverride": { "attack": attack.max(0), "health": health },
                "keywords": keywords,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: chamber(false, false),
        radiant: chamber(true, true),
    }
}

// Stinkyscape #134 Felinor Containment Chamber — SPEC §8.10 row 134, BUILD M12 row S 134
// (R1852000, R89).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const CHAMBER: &str = "stinkyscape-134";
    const FELINOR: &str = "core-t-felinor"; // §7's shared Felinor Token, 1/1.
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1860007_death_summons_a_felinor_token_of_its_attack_and_max_health_buffs_included() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [CHAMBER], "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.card_mut(CHAMBER).buffs.attack += 3;
            s.card_mut(CHAMBER).buffs.health += 3;
            let me = s.card(CHAMBER).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(CHAMBER, "graveyard");
            // The token lands in the leftmost open unit zone, before the copy fuses (R1852000).
            let token = s.unit(PlayerId::P1, 1).expect("the token in lane 1");
            assert_eq!(token.def_id, FELINOR);
            s.expect_stats(token.id.as_str(), json!({ "attack": 5, "health": 4, "maxHealth": 4 }));
            assert!(!kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
            let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
            assert!(ship.is_some(), "its copy still fuses into a Dormant Starship");
        }

        #[test]
        fn r1860007_a_full_board_summons_none() {
            crate::register_all();
            // The Chamber tops a pile on lane 1; when it dies the Vanilla under it resumes there
            // (§3.2), so no zone is open.
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [VANILLA, { "def": CHAMBER, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CHAMBER).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            s.expect_in_zone(CHAMBER, "graveyard");
            // No zone is open, so no token lands — but the copy still fuses (R1852000).
            for lane in 1..=5 {
                assert_ne!(
                    s.unit(PlayerId::P1, lane).map(|unit| unit.def_id),
                    Some(FELINOR.to_string()),
                    "no Felinor Token in lane {lane}"
                );
            }
            assert!(dormant_starship_of(s.state(), PlayerId::P1).is_some());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1860007_radiant_death_summons_a_doubled_token_with_divine_shield() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": {
                    "hand": [HIT_JOB, ANCHOR],
                    "mana": 10,
                    "field": [{ "def": CHAMBER, "radiant": true }],
                    "library": filler(4),
                },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            let me = s.card(CHAMBER).clone();
            s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

            let token = s.unit(PlayerId::P1, 1).expect("the token in lane 1");
            assert_eq!(token.def_id, FELINOR);
            s.expect_stats(token.id.as_str(), json!({ "attack": 8, "health": 4, "maxHealth": 4 }));
            assert!(kinds(&s, token.id.as_str()).contains(&"Divine Shield".to_string()));
        }
    }
}
