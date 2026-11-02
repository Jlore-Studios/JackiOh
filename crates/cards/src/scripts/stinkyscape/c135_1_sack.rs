//! S #135.1 Sack (SPEC §7, §8.10 row 135.1; R1850003, R1875005): (1) Unit, Token (printed Common),
//! 0/4 → 0/8, of the Stinkyscape set.
//!   Base:    "When this leaves the field, summon the Unit stuffed into it."
//!   Radiant: "Brittle 1 / When this leaves the field, summon the Unit stuffed into it."
//! Engine: a holder (R1850003): the Unit S #135 Sacked stuffs into it waits in `state.held`, and at the
//! first state check after the Sack leaves the field, any way (it dies, is Bounced, exiled, transformed
//! or held itself; R1875005), that Unit is summoned for the Sack's controller then, R64's zone, summoning
//! sick, no Cry (R1), or exiled with no zone open. The release is the engine's, so the Sack has no text
//! of its own to run. A Sack holding nothing summons nothing. Radiant: Brittle 1 (R638), catalog data:
//! it crumbles at its controller's next start of turn, releasing its Unit. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-135-1";

/// R2476000: after "the Unit stuffed into it", the card held beneath it on its stay now (`held_beneath`), by name
/// ("Radiant " before a Radiant one), or the dash when it holds none. On the field, to both seats: only
/// a face-up permanent is held (R1850003), and the hold was public.
fn preview() -> PreviewHook {
    condition_hook(|ctx: ConditionContext<'_>| -> Vec<PreviewValue> {
        if ctx.zone != ConditionZone::Field {
            return vec![];
        }
        let held = held_beneath(ctx.state, ctx.self_);
        vec![card_preview(
            ctx.state,
            "the Unit stuffed into it",
            held.first().map(|card| (card.def_id.as_str(), card.radiant)),
        )]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        preview: Some(preview()),
        ..Script::default()
    };
    // The Radiant face's 0/8 and Brittle 1 are catalog data, the release the engine's: the same script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #135.1 Sack — SPEC §7, BUILD M12 row S 135.1 (R638).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const SACKED: &str = "stinkyscape-135";
    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const SORCERER: &str = "core-068"; // (2) 5/5: "Cry: Deal 4 damage."
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."
    const FLOOD: &str = "core-017"; // (4) Spell: "Bounce all Units."
    const COLLATERAL: &str = "core-034"; // (4) Spell: "Exile a permanent and a random card …"
    const POLYMORPH: &str = "meditative-024"; // (3) Spell: "Transform a Unit into a Sheep Token."

    fn sacked(radiant: bool, p1_field: Value, hand: Value) -> (Scenario, String) {
        let mut s = crate::scenario(json!({
            "seed": "sack",
            "p1": { "hand": [SORCERER, FILLER], "field": p1_field, "library": crate::filler(6), "mana": 10 },
            "p2": {
                "hand": hand,
                "backrow": [{ "def": SACKED, "radiant": radiant, "lane": 1 }],
                "library": crate::filler(6),
            },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        let sorcerer = s.card(SORCERER).id.clone();
        s.play(SORCERER, json!({ "zone": 2, "targets": [{ "pick": "hero", "player": "p2" }] }));
        s.end_turn();
        s.state_mut().players[P2].mana.current = 10;
        (s, sorcerer)
    }

    fn sack(s: &Scenario) -> CardInstance {
        [P1, P2]
            .into_iter()
            .flat_map(|player| (1..=5).filter_map(move |lane| s.unit(player, lane)))
            .find(|card| card.def_id == ID)
            .expect("a Sack")
    }

    /// R2476000: after "the Unit stuffed into it", the Unit it holds by name, to both seats.
    #[test]
    fn r2476000_it_names_the_unit_stuffed_into_it() {
        for radiant in [false, true] {
            let (s, _) = sacked(radiant, json!([]), json!([FILLER]));
            let sack = sack(&s).id;
            let named = vec![("the Unit stuffed into it".to_string(), 1, Some("Twisted Sorcerer".to_string()))];
            assert_eq!(crate::preview_seen(&s, P1, &sack), named, "radiant {radiant}");
            assert_eq!(crate::preview_seen(&s, P2, &sack), named, "radiant {radiant}");
        }
    }

    fn back_for(s: &Scenario, id: &str, player: PlayerId) -> bool {
        let card = s.card(id);
        matches!(card.zone, Zone::Field { player: on, row: Row::Units, .. } if on == player)
            && card.summoned_turn == Some(s.state().turn)
    }

    mod base {
        use super::*;

        #[test]
        fn r1875005_destroyed_it_summons_the_unit_for_its_controller_sick_and_with_no_cry() {
            let (mut s, sorcerer) = sacked(false, json!([]), json!([HIT_JOB, FILLER]));
            let sack = sack(&s);
            s.play(HIT_JOB, json!({ "targets": pick::id(&sack.id) }));
            assert!(back_for(&s, &sorcerer, P1));
            s.expect_health(P2, HERO_HEALTH - 4);
        }

        #[test]
        fn r1875005_bounced_or_exiled_it_releases_the_same() {
            for spell in [FLOOD, COLLATERAL] {
                let (mut s, sorcerer) = sacked(false, json!([]), json!([spell, FILLER]));
                let sack = sack(&s);
                if spell == FLOOD {
                    s.play(FLOOD, json!({}));
                } else {
                    s.play(COLLATERAL, json!({ "targets": pick::id(&sack.id) }));
                }
                assert!(back_for(&s, &sorcerer, P1), "{spell}");
            }
        }

        #[test]
        fn r1850003_transformed_on_a_full_side_it_leaves_the_unit_no_zone_and_it_is_exiled() {
            let mut s = crate::scenario(json!({
                "seed": "sack-full",
                "p1": {
                    "hand": [SORCERER, VANILLA, FILLER],
                    "field": ([1, 3, 4].map(|lane| json!({ "def": VANILLA, "lane": lane }))),
                    "library": crate::filler(6),
                    "mana": 10,
                },
                "p2": {
                    "hand": [POLYMORPH, FILLER],
                    "backrow": [{ "def": SACKED, "lane": 1 }],
                    "library": crate::filler(6),
                },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(SORCERER, json!({ "zone": 2, "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert_eq!(s.unit_at(P1, 2).def_id, ID);
            let refill = s.hand(P1).into_iter().find(|card| card.def_id == VANILLA).expect("a Vanilla").id;
            s.play(refill.as_str(), json!({ "zone": 5 }));
            s.end_turn();
            s.state_mut().players[P2].mana.current = 10;
            let sack = s.unit_at(P1, 2).id;
            s.play(POLYMORPH, json!({ "targets": pick::id(&sack) }));
            s.expect_in_zone(sorcerer.as_str(), "exile");
        }

        #[test]
        fn r1850003_a_sack_that_finds_no_zone_holds_nothing_and_the_unit_owed_to_that_side_is_exiled() {
            let mut s = crate::scenario(json!({
                "seed": "sack-none",
                "p1": { "hand": [SORCERER, FILLER], "library": crate::filler(6), "mana": 10 },
                "p2": {
                    "hand": [FILLER],
                    "field": ([1, 2, 3, 4, 5].map(|lane| json!({ "def": VANILLA, "lane": lane }))),
                    "backrow": [{ "def": SACKED, "radiant": true, "lane": 1 }],
                    "library": crate::filler(6),
                },
            }));
            let sorcerer = s.card(SORCERER).id.clone();
            s.play(SORCERER, json!({ "zone": 2, "targets": [{ "pick": "hero", "player": "p2" }] }));
            assert!((1..=5).filter_map(|lane| s.unit(P2, lane)).all(|card| card.def_id != ID));
            assert!(s.state().held.iter().flatten().next().is_none());
            s.expect_in_zone(sorcerer.as_str(), "exile");
        }

        #[test]
        fn a_sack_holding_nothing_summons_nothing() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [HIT_JOB, FILLER], "library": crate::filler(4), "mana": 10 },
                "p2": { "field": [{ "def": ID, "lane": 3 }], "library": crate::filler(4) },
            }));
            let sack = s.unit_at(P2, 3).id;
            s.play(HIT_JOB, json!({ "targets": pick::id(&sack) }));
            assert!((1..=5).all(|lane| s.unit(P2, lane).is_none()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r638_a_radiant_sack_is_0_8_and_crumbles_at_its_brittle_1_releasing_the_unit() {
            let (mut s, sorcerer) = sacked(false, json!([{ "def": ID, "radiant": true, "lane": 5 }]), json!([FILLER]));
            let radiant = s.unit_at(P1, 5);
            s.expect_stats(&radiant, json!({ "attack": 0, "health": 8 }));
            let held = s.state_mut().held.as_mut().expect("the Sorcerer held");
            held[0].holder_id = radiant.id.clone();
            let mut turns = 0;
            // A token ceases to exist as it leaves the field (R11).
            while s.unit(P1, 5).is_some_and(|card| card.id == radiant.id) {
                crate::next_turn_of(&mut s, P1);
                turns += 1;
                assert!(turns <= 2, "Brittle 1 crumbles it within a cycle");
            }
            assert!(matches!(s.card(sorcerer.as_str()).zone, Zone::Field { player: P1, row: Row::Units, .. }));
        }
    }
}
