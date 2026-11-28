//! SPEC §8.14 FvH #70 Felinor Commander ((4) Unit, Felinor, Legendary, 7/7 → 14/14), a card of the held
//! Felinors vs Humans set (R5235000).
//!   Base:    "Cry: Fill your board with random Felinor Units. Overload ({mana}).\nAura: Your Felinors have
//!            +{aura}/+{aura}." (mana 4, aura 2)
//!   Radiant: "Cry: Fill your board with random Radiant Felinor Units. Overload ({mana}).\nAura: Your
//!            Felinors have +{aura}/+{aura}." (mana 4, aura 4)
//!
//! Cry (R1): `fill_board_random` of non-token Felinor Units of the sets that ship, one pick per open unit
//! zone, a Locked one included (R1080, R1136000), no repeat while the pool lasts (R3563000), Radiant on the
//! Radiant face, none running its Cry. The list's standalone "Overload 4" is folded into the Cry (FV2,
//! R4353021, R4751000): its controller's next refresh gives the declared `mana` less, whatever the fill
//! finds; a summon runs no Cry and pays nothing. Aura (§10.4 layer 5): +aura/+aura on its controller's
//! Felinor-tagged Units, itself included (C+ #46's Radiant wording, R923's tags). Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-070";

/// "Your Felinors have +{aura}/+{aura}", itself among them.
fn your_felinors() -> AuraHook {
    aura_hook(|ctx| {
        let amount = param(&ctx, "aura");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller
                    && in_units_row_or_standing(ctx.state, unit)
                    && tags_of(ctx.state, unit).contains(&Tag::Felinor)
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

fn commander(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![
                fill_board_random(json_as(json!({
                    "query": { "type": "Unit", "tags": ["Felinor"] },
                    "radiant": radiant,
                }))),
                // FV2: the standalone "Overload ({mana})." is the Cry's (Core #2's shape).
                next_turn_mana(json_as(json!({ "amount": -param(ctx, "mana") }))),
            ]
        })),
        aura: Some(your_felinors()),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: commander(false),
        radiant: commander(true),
    }
}

// FvH #70 Felinor Commander — SPEC §8.14 row 70.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FELINOR: &str = "core-t-felinor"; // Felinor Token: a Felinor with no text
    const HUMAN: &str = "core-008"; // Mr. Vanilla: a Human, no Felinor
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn played(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": field, "mana": 10, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    /// The Units p1 summoned in the last step, by `summoned`: the Commander's own arrival as it is played
    /// is a `Summoned` too, left out.
    fn summoned(s: &Scenario) -> Vec<CardInstance> {
        let commander = commander_of(s).id;
        s.last_events_of(GameEventType::Summoned)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::Summoned {
                    player: P1,
                    instance_id,
                    ..
                } if instance_id != commander => Some(s.card(instance_id.as_str()).clone()),
                _ => None,
            })
            .collect()
    }

    fn next_refresh(s: &Scenario) -> i32 {
        s.state().players.p1.mana.next_turn_mod
    }

    fn commander_of(s: &Scenario) -> CardInstance {
        must(
            active_units_of(s.state(), P1)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the Felinor Commander on p1's field",
        )
    }

    fn fills_the_board_with_felinor_units(radiant: bool) {
        let s = played(radiant, json!([]));
        let units = nonempty(summoned(&s), "the Felinors the fill summoned");
        assert_eq!(
            units.len(),
            usize::try_from(UNIT_ZONES - 1).unwrap_or(0),
            "every open zone but its own"
        );
        for unit in &units {
            let def = def_of(Some(s.state()), &unit.def_id);
            assert_eq!(def.type_, CardType::Unit);
            assert!(def.tags.contains(&Tag::Felinor), "{} is a Felinor", def.id);
            assert!(!def.token, "{} is no token", def.id);
            assert_ne!(def.id, ID, "the pool never offers the card that made it");
            assert_eq!(unit.radiant, radiant);
        }
        assert_eq!(next_refresh(&s), -4, "Overload (4)");
    }

    mod base {
        use super::*;

        #[test]
        fn r1080_its_cry_fills_the_board_with_random_felinor_units_and_overloads_4() {
            fills_the_board_with_felinor_units(false);
        }

        #[test]
        fn r4353021_a_full_board_fills_nothing_and_still_overloads() {
            let s = played(false, json!([HUMAN, HUMAN, HUMAN, HUMAN]));
            assert!(summoned(&s).is_empty());
            assert_eq!(next_refresh(&s), -4);
        }

        #[test]
        fn its_aura_gives_your_felinors_itself_included_plus_2_plus_2() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [ID, FELINOR, HUMAN], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [FELINOR], "library": crate::filler(4) },
            }));
            s.expect_stats(commander_of(&s), json!({ "attack": 7 + 2, "health": 7 + 2 }));
            let token = s.unit_at(P1, 2);
            s.expect_stats(&token, json!({ "attack": 1 + 2, "health": 1 + 2 }));
            let human = s.unit_at(P1, 3);
            s.expect_stats(&human, json!({ "attack": 4, "health": 4 }));
            let theirs = s.unit_at(P2, 1);
            s.expect_stats(&theirs, json!({ "attack": 1, "health": 1 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1080_its_cry_fills_the_board_with_random_radiant_felinor_units_and_overloads_4() {
            fills_the_board_with_felinor_units(true);
        }

        #[test]
        fn its_aura_gives_your_felinors_plus_4_plus_4() {
            let mut s = scenario(json!({
                "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": true }, FELINOR], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "library": crate::filler(4) },
            }));
            s.expect_stats(commander_of(&s), json!({ "attack": 14 + 4, "health": 14 + 4 }));
            let token = s.unit_at(P1, 2);
            s.expect_stats(&token, json!({ "attack": 1 + 4, "health": 1 + 4 }));
        }
    }
}
