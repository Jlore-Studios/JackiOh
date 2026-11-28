//! SPEC §8.14 FvH #16.1 Travertine Stack ((9999) Spell, Token, printed Legendary), a token of the Felinors
//! vs Humans set (R5235000) that FvH #16 Travertine adds.
//!   Base:    "When you play or discard this: Gain {mana} mana and {credits} Credits, and summon a Travertine
//!            Golem." (2, 15)
//!   Radiant: the same with 4 and 30, and a Radiant Travertine Golem.
//!
//! It prints no Temporary: its count is the one #16 gives it (R5238000), which a printed one would floor
//! (R2179001). At (9999) it is played only at a lowered price, as FvH #3's (2) (FV2 #16). As it resolves,
//! and from its graveyard on its own `Discarded` (any discard, Temporary's included: "when discarded or
//! played"), the same list: `gain_mana` of `mana`, `gain_market_credits` of `credits` (R1840005, the
//! shape of R2250001) and a `summon` of FvH #16.2 per R64, none with no open zone (R129). A Temporary
//! discard comes in cleanup after the turn's Float is counted, so that mana is gone at the next refresh
//! (R62).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-016-1";

/// FvH #16.2 Travertine Golem, the Unit it summons.
const GOLEM: &str = "felinorsvshumans-016-2";

/// "Gain {mana} mana and {credits} Credits, and summon a (Radiant) Travertine Golem".
fn payout(ctx: &EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    vec![
        gain_mana(json_as(json!({ "amount": param(ctx, "mana") }))),
        gain_market_credits(json_as(json!({ "amount": param(ctx, "credits") }))),
        summon(json_as(json!({ "defId": GOLEM, "radiant": radiant }))),
    ]
}

/// One face; `radiant` is the face its Golem arrives on.
fn stack(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| payout(ctx, radiant))),
        graveyard_triggers: vec![TriggerDef::new(
            "felinorsvshumans-016-1-discarded",
            &[GameEventType::Discarded],
            move |ctx, event| {
                let GameEvent::Discarded { instance_id, .. } = event else {
                    return Vec::new();
                };
                if ctx.self_.as_ref().is_some_and(|card| card.id == *instance_id) {
                    payout(ctx, radiant)
                } else {
                    Vec::new()
                }
            },
        )],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: stack(false),
        radiant: stack(true),
    }
}

// FvH #16.1 Travertine Stack — SPEC §8.14 row 16.1 (R1840005, R2250001, R64, R129).
#[cfg(test)]
mod tests {
    use super::{GOLEM, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const FILLER: &str = "core-005"; // Stockpile: a Spell, each side's spare card (§2.5).
    const HINDER: &str = "core-021"; // Hinder: discards a random card of its controller's hand
    /// The lowered price it is played at, as FvH #3 KY's Pocket Galaxy sets it.
    const LOWERED: i32 = 2;

    fn credits(s: &Scenario) -> i32 {
        s.state().players[P1].market_credits.unwrap_or(0)
    }

    /// p1 holds the Stack (`radiant` its face), its price lowered, beside `with`.
    fn holding(seed: &str, radiant: bool, with: Value) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant, "costOverride": LOWERED })];
        hand.push(with);
        weapon_board(seed, json!({ "hand": hand }), json!({ "hand": [FILLER] }))
    }

    /// The payout landed: `mana` now, `credits` Credits and one Golem on `radiant` in lane 1.
    fn expect_payout(s: &mut Scenario, mana: i32, gained: i32, radiant: bool) {
        s.expect_mana(P1, mana);
        assert_eq!(credits(s), gained);
        let golem = s.unit_at(P1, 1);
        assert_eq!((golem.def_id.as_str(), golem.radiant), (GOLEM, radiant));
    }

    #[test]
    fn at_its_printed_9999_it_cannot_be_played() {
        let s = weapon_board("fvh016-1-printed", json!({ "hand": [ID, FILLER], "mana": 10 }), json!({}));
        assert!(!s.can_play(ID));
    }

    mod base {
        use super::*;

        #[test]
        fn played_it_gains_2_mana_and_15_credits_and_summons_a_golem() {
            let mut s = holding("fvh016-1-played", false, json!(FILLER));
            s.play(ID, json!({}));
            expect_payout(&mut s, MAX_MANA - LOWERED + 2, 15, false);
        }

        #[test]
        fn r2250001_discarded_it_pays_the_same() {
            let mut s = holding("fvh016-1-discarded", false, json!(HINDER));
            let stack = s.card(ID).clone();
            let hinder = query_cost(&crate::card_def(HINDER));
            s.play(HINDER, json!({}));
            s.expect_in_zone(&stack, "graveyard");
            assert_eq!(s.last_count("discarded"), 1);
            expect_payout(&mut s, MAX_MANA - hinder + 2, 15, false);
        }

        #[test]
        fn r129_with_no_open_zone_no_golem_comes() {
            let mut s = weapon_board(
                "fvh016-1-full",
                json!({
                    "hand": [{ "def": ID, "costOverride": LOWERED }, FILLER],
                    "field": ["stand-in (1) 1/1", "stand-in (1) 1/1", "stand-in (1) 1/1", "stand-in (1) 1/1", "stand-in (1) 1/1"],
                }),
                json!({ "hand": [FILLER] }),
            );
            s.play(ID, json!({}));
            s.expect_mana(P1, MAX_MANA - LOWERED + 2);
            assert_eq!(credits(&s), 15);
            assert_eq!(s.last_count("summoned"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn played_it_gains_4_mana_and_30_credits_and_summons_a_radiant_golem() {
            let mut s = holding("fvh016-1-radiant", true, json!(FILLER));
            s.play(ID, json!({}));
            expect_payout(&mut s, MAX_MANA - LOWERED + 4, 30, true);
        }

        #[test]
        fn r2250001_discarded_it_pays_the_same() {
            let mut s = holding("fvh016-1-radiant-discarded", true, json!(HINDER));
            let hinder = query_cost(&crate::card_def(HINDER));
            s.play(HINDER, json!({}));
            expect_payout(&mut s, MAX_MANA - hinder + 4, 30, true);
        }
    }
}
