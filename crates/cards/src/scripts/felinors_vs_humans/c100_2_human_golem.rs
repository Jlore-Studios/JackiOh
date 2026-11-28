//! FvH #100.2 Human Golem (SPEC §8.14 row 100.2; R5237010): (4) Unit, Token (printed Legendary), Human,
//! Golem, 5/5 → 10/10, of the held Felinors vs Humans set. Its maker is FvH #100 Call to Chaos (Felinors
//! vs. Humans Edition), entry 9.
//!   Base:    "Rush, Taunt / Immune to Felinors. / Aura: Your other Humans have +{buff}/+{buff}." — buff 5.
//!   Radiant: the same text, buff 10.
//! Engine: Rush and Taunt are catalog data. Immune to Felinors is `StaticFlags.immune_to_tag` Felinor
//! (R5237010), as #100.1's Immune to Humans: a card whose tags hold Felinor cannot target, affect or damage
//! it, a Felinor attacker's combat damage included; it bars no attack. Aura (§10.4): every other Human Unit
//! its controller has on the field (granted tags read, R923; one standing in the backrow too, R1081000)
//! has +`buff`/+`buff`, S #88 Siva 2's aura over Humans with TG #3's "other".

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-100-2";

/// The declared "+{buff}/+{buff}" (`params`).
const BUFF_KEY: &str = "buff";

fn golem() -> Script {
    Script {
        // R5237010: Immune to Felinors.
        static_flags: Some(StaticFlags {
            immune_to_tag: Some(Tag::Felinor),
            ..StaticFlags::default()
        }),
        aura: Some(aura_hook(|ctx| {
            let amount = param(&ctx, BUFF_KEY);
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller
                        && in_units_row_or_standing(ctx.state, unit)
                        && unit.id != ctx.self_.id
                        && tags_of(ctx.state, unit).contains(&Tag::Human)
                }),
                mod_: StatMod {
                    attack: Some(amount),
                    max_health: Some(amount),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's +10/+10 is its declared `buff`, which `param` reads off the running
    // face; its stats are the catalog's.
    let base = golem();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #100.2 Human Golem — SPEC §8.14 row 100.2, R5237010.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    /// A Unit a test tags Human, another left untagged, and a sturdy one that strikes back.
    const ALLY: &str = "stand-in (1) 2/2";
    const OTHER: &str = "stand-in (1) 2/2";
    const WALL: &str = "stand-in (1) 3/9";
    /// The declared buff on each face.
    const BUFF: i32 = 5;
    const RADIANT_BUFF: i32 = 10;
    const HEALTH: i32 = 5;

    /// p1: the Golem in lane 1, a Human `ALLY` in lane 2, an untagged `OTHER` in lane 3; p2: a Human in lane 1.
    fn board(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [FILLER],
                "field": [
                    { "def": ID, "lane": 1, "radiant": radiant },
                    { "def": ALLY, "lane": 2 },
                    { "def": OTHER, "lane": 3 },
                ],
                "library": filler(6),
            },
            "p2": { "hand": [FILLER], "field": [{ "def": WALL, "lane": 1 }], "library": filler(6) },
        }));
        for (player, lane) in [(P1, 2), (P2, 1)] {
            let id = s.unit_at(player, lane).id;
            s.card_mut(id.as_str()).granted_tags = Some(vec![Tag::Human]);
        }
        s
    }

    fn stats_at(s: &Scenario, player: PlayerId, lane: i32) -> (i32, i32) {
        let view = s.stats(s.unit_at(player, lane).id.as_str());
        (view.attack, view.health)
    }

    /// Your other Humans have +`buff`/+`buff`; the Golem, a non-Human and an enemy Human do not.
    fn aura_gives(radiant: bool, buff: i32, own: (i32, i32)) {
        let s = board(radiant);
        assert_eq!(stats_at(&s, P1, 2), (2 + buff, 2 + buff), "your other Human");
        assert_eq!(stats_at(&s, P1, 3), (2, 2), "a non-Human");
        assert_eq!(stats_at(&s, P2, 1), (3, 9), "an enemy Human");
        assert_eq!(stats_at(&s, P1, 1), own, "the Golem itself");
    }

    /// The Golem attacks a Felinor, which strikes back for nothing.
    fn felinors_strike_it_for_nothing(radiant: bool, health: i32) {
        let mut s = board(radiant);
        let wall = s.unit_at(P2, 1).id;
        s.card_mut(wall.as_str()).granted_tags = Some(vec![Tag::Felinor]);
        let golem = s.unit_at(P1, 1);
        s.attack(&golem, wall.as_str());
        assert_eq!(s.stats(golem.id.as_str()).health, health, "Immune to Felinors");
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_your_other_humans_have_plus_5_plus_5() {
            aura_gives(false, BUFF, (HEALTH, HEALTH));
        }

        #[test]
        fn r5237010_a_felinor_strikes_it_for_nothing() {
            felinors_strike_it_for_nothing(false, HEALTH);
        }

        #[test]
        fn r5237010_a_human_still_hurts_it() {
            let mut s = board(false);
            let wall = s.unit_at(P2, 1).id;
            let golem = s.unit_at(P1, 1);
            s.attack(&golem, wall.as_str());
            assert_eq!(s.stats(golem.id.as_str()).health, HEALTH - 3);
        }

        #[test]
        fn r386_a_buff_of_the_number_raises_the_aura() {
            let mut s = board(false);
            step_param(s.card_mut(ID), "buff", 1);
            assert_eq!(stats_at(&s, P1, 2), (2 + BUFF + 1, 2 + BUFF + 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_your_other_humans_have_plus_10_plus_10() {
            aura_gives(true, RADIANT_BUFF, (2 * HEALTH, 2 * HEALTH));
        }

        #[test]
        fn r5237010_a_felinor_strikes_it_for_nothing() {
            felinors_strike_it_for_nothing(true, 2 * HEALTH);
        }
    }
}
