//! S #88 Siva 2 (SPEC §8.10 row 88): (3) Unit, Human, Acclaimed, Legendary, 4/4 → 8/8, of the
//! Stinkyscape set.
//!   Base:    "Aura: Your Humans have Poisonous."
//!   Radiant: "Aura: Your Humans have Poisonous. / Cry: Fill your hand with Bigots."
//! Engine: Aura (§10.4): every Human Unit its controller has on the field (granted tags read, R923; one
//! standing in the backrow too, R1081000), this one included, has Poisonous. Radiant: Cry (R1): Core #2
//! Bigots added while the hand has room (§2.4, M+ #24's fill, R1185004), none burned: the room is read as
//! the Cry resolves.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-088";

/// Core #2 Bigot.
const BIGOT: &str = "core-002";

fn poisonous_humans() -> AuraHook {
    aura_hook(|ctx| {
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller
                    && in_units_row_or_standing(ctx.state, unit)
                    && tags_of(ctx.state, unit).contains(&Tag::Human)
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Poisonous]),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        aura: Some(poisonous_humans()),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|ctx| {
            (0..room_in(ctx.state, ctx.controller, OffFieldZone::Hand))
                .map(|_| add_to_hand(json_as(json!({ "defId": BIGOT }))))
                .collect()
        })),
        ..base.clone()
    };
    CardScripts { base, radiant }
}

// S #88 Siva 2 — SPEC §8.10 row 88, BUILD M12 row S 88.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{BIGOT, ID};
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4, a Human.
    const MENACE: &str = "core-019"; // (3) 9/9, no tag.
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
    const FILLER: &str = "core-005";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, HIT_JOB],
                "field": [VANILLA, MENACE],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(4) },
        }))
    }

    fn poisonous(s: &Scenario, card: &CardInstance) -> bool {
        unit_has(s.state(), card, KeywordKind::Poisonous)
    }

    #[test]
    fn s10_4_your_humans_have_poisonous_this_one_included_never_an_enemys() {
        let mut s = board(false);
        s.play(ID, json!({ "zone": 3 }));
        assert!(poisonous(&s, &s.unit_at(P1, 1)), "your Human");
        assert!(poisonous(&s, &s.unit_at(P1, 3)), "Siva 2 itself");
        assert!(!poisonous(&s, &s.unit_at(P1, 2)), "a non-Human");
        assert!(!poisonous(&s, &s.unit_at(P2, 1)), "an enemy Human");
        s.expect_stats(ID, json!({ "attack": 4, "health": 4 }));
    }

    #[test]
    fn s10_4_the_poisonous_ends_as_it_leaves_the_field() {
        let mut s = board(false);
        s.play(ID, json!({ "zone": 3 }));
        let siva = s.unit_at(P1, 3).id;
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": siva }] }));
        assert!(s.unit(P1, 3).is_none());
        assert!(!poisonous(&s, &s.unit_at(P1, 1)));
    }

    #[test]
    fn s2_4_the_base_face_adds_nothing() {
        let mut s = board(false);
        s.play(ID, json!({ "zone": 3 }));
        assert_eq!(s.hand(P1).len(), 1);
    }

    #[test]
    fn r275_radiant_8_8_fills_your_hand_with_bigots_none_burned() {
        let mut s = board(true);
        s.play(ID, json!({ "zone": 3 }));
        let hand = s.hand(P1);
        assert_eq!(hand.len() as i32, HAND_CAP);
        assert_eq!(hand.iter().filter(|card| card.def_id == BIGOT).count() as i32, HAND_CAP - 1);
        assert!(!s.last_events().iter().any(|event| matches!(event, GameEvent::Burned { .. })));
        assert!(poisonous(&s, &s.unit_at(P1, 1)));
        s.expect_stats(ID, json!({ "attack": 8, "health": 8 }));
    }
}
