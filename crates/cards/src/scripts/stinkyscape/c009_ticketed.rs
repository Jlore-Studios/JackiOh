//! S #9 Ticketed (SPEC §8.10 row 9; R1850003, R1875001): (2) Field Spell, Common, of the Stinkyscape set.
//!   Base:    "Brittle 3 / Cry: Hold another permanent beneath this. / When this leaves the field, the
//!            permanent beneath it returns to its controller."
//!   Radiant: the same without Brittle 3.
//! Engine: Brittle 3 is a catalog keyword (R638). Its Cry runs as it is played, its target declared with
//! the play (R81): another permanent acting face-up on the field, either side's (R1875001), a face-down
//! card never offered. `hold_beneath` (R1850003): the permanent leaves the field as an exiled card does,
//! no Death, reset (R78), and waits beneath Ticketed; at the first state check after Ticketed leaves the
//! field any way (Brittle crumbling it, a destroy, a Bounce, an exile; R1875001) it is summoned for its
//! controller as it was taken, R64's zone, summoning sick, no Cry (R1), or exiled with no zone open. A
//! unit token ceases to exist as it is taken (R11). Radiant: no Brittle, so it holds until Ticketed is
//! removed; the same script, the keyword being catalog data. Declares no numbers.

use jackioh_engine::effects::{HoldArgs, hold_beneath};
use jackioh_engine::preview::is_face_down;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-009";

/// R1875001: the pick is a permanent the hold can take, so never a face-down one (R1850003).
const FACE_UP: &str = "faceUp";

/// R2476000: after "the permanent beneath it", the card held beneath it on its stay now (`held_beneath`), by name
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
            "the permanent beneath it",
            held.first().map(|card| (card.def_id.as_str(), card.radiant)),
        )]
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"], "excludeSelf": true, "check": FACE_UP }),
        )],
        target_checks: IndexMap::from([(
            FACE_UP,
            target_check(|args| args.candidate.is_some_and(|card| !is_face_down(args.state, card))),
        )]),
        cry: Some(hook(|_ctx| vec![hold_beneath(HoldArgs::default())])),
        preview: Some(preview()),
        ..Script::default()
    };
    // The Radiant face drops Brittle 3: catalog data, the same script.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #9 Ticketed — SPEC §8.10 row 9, BUILD M12 row S 9 (R11, R638).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell Stockpile: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const LAR: &str = "classicplus-076"; // (1) 1/1: "Death: Summon a Brother Ping."
    const SORCERER: &str = "core-068"; // (2) 5/5: "Cry: Deal 4 damage."
    const ARMOR: &str = "core-073"; // (2) Field Spell: "… Cry: Draw 1."
    const RUSH_TOKEN: &str = "core-t-rush"; // a unit token.
    const JAMMED: &str = "core-036"; // (1) Spell: "Destroy a backrow card. Lock its zone."
    const COLLATERAL: &str = "core-034"; // (4) Spell: "Exile a permanent and a random card …"
    const TRAP: &str = "core-041"; // (1) Trap Sheepish, to set face-down.
    const CORE_SYSTEM: &str = "stinkyscape-117"; // (3) Spell: "Bounce all permanents. …"

    fn board(radiant: bool, hand: Value, field: Value, backrow: Value) -> Scenario {
        let mut cards = vec![json!({ "def": ID, "radiant": radiant }), json!(FILLER)];
        cards.extend(hand.as_array().cloned().unwrap_or_default());
        let mut s = crate::scenario(json!({
            "seed": "ticketed",
            "p1": { "hand": cards, "library": crate::filler(8), "mana": 10 },
            "p2": { "hand": [FILLER], "field": field, "backrow": backrow, "library": crate::filler(8), "mana": 10 },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn ticket(s: &mut Scenario, target: &str) {
        s.play(ID, json!({ "zone": 1, "targets": pick::id(target) }));
    }

    fn held_ids(s: &Scenario) -> Vec<String> {
        s.state()
            .held
            .iter()
            .flatten()
            .map(|held| held.card.id.clone())
            .collect()
    }

    mod base {
        use super::*;

        /// R2476000: after "the permanent beneath it", the Unit it holds by name, to both seats.
        #[test]
        fn r2476000_it_names_the_permanent_it_holds() {
            for radiant in [false, true] {
                let mut s = board(radiant, json!([]), json!([{ "def": LAR, "lane": 2 }]), json!([]));
                let lar = s.unit_at(P2, 2).id;
                ticket(&mut s, &lar);
                let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
                let named = vec![("the permanent beneath it".to_string(), 1, Some("Brother Lar".to_string()))];
                assert_eq!(crate::preview_seen(&s, P1, &ticketed), named, "radiant {radiant}");
                assert_eq!(crate::preview_seen(&s, P2, &ticketed), named, "radiant {radiant}");
            }
        }

        #[test]
        fn r1850003_it_holds_a_chosen_enemy_unit_with_no_death_and_frees_its_zone() {
            let mut s = board(false, json!([]), json!([{ "def": LAR, "lane": 2 }]), json!([]));
            let lar = s.unit_at(P2, 2).id;
            ticket(&mut s, &lar);
            assert_eq!(held_ids(&s), vec![lar.clone()]);
            assert!(s.unit(P2, 2).is_none(), "its zone is free");
            assert!(s.unit(P2, 1).is_none(), "no Brother Ping: no Death");
            assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Held));
        }

        #[test]
        fn r638_once_brittle_3_crumbles_it_the_unit_comes_back_sick_and_with_no_cry() {
            let mut s = board(false, json!([]), json!([{ "def": SORCERER, "lane": 3 }]), json!([]));
            let sorcerer = s.unit_at(P2, 3).id;
            ticket(&mut s, &sorcerer);
            for _ in 0..2 {
                crate::next_turn_of(&mut s, P1);
                assert_eq!(held_ids(&s), vec![sorcerer.clone()]);
            }
            crate::next_turn_of(&mut s, P1);
            s.expect_in_zone(ID, "graveyard");
            let back = s.card(sorcerer.as_str()).clone();
            assert!(matches!(back.zone, Zone::Field { player: P2, row: Row::Units, .. }), "{:?}", back.zone);
            assert_eq!(back.summoned_turn, Some(s.state().turn), "summoning sick");
            s.expect_health(P1, HERO_HEALTH);
            assert!(held_ids(&s).is_empty());
        }

        #[test]
        fn r1875001_destroyed_or_exiled_it_releases_at_once() {
            for (spell, seed) in [(JAMMED, "ticketed-destroyed"), (COLLATERAL, "ticketed-exiled")] {
                let mut s = board(false, json!([spell]), json!([{ "def": VANILLA, "lane": 4 }]), json!([]));
                let vanilla = s.unit_at(P2, 4).id;
                ticket(&mut s, &vanilla);
                let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
                s.play(spell, json!({ "targets": pick::id(&ticketed) }));
                assert!(held_ids(&s).is_empty(), "{seed}");
                assert!(
                    matches!(s.card(vanilla.as_str()).zone, Zone::Field { player: P2, row: Row::Units, .. }),
                    "{seed}"
                );
            }
        }

        #[test]
        fn r1875001_bounced_it_releases_at_once() {
            let mut s = board(
                false,
                json!([CORE_SYSTEM]),
                json!([{ "def": VANILLA, "lane": 4 }]),
                json!([]),
            );
            let vanilla = s.unit_at(P2, 4).id;
            ticket(&mut s, &vanilla);
            assert_eq!(held_ids(&s), vec![vanilla.clone()]);
            s.play(CORE_SYSTEM, json!({}));
            assert!(
                s.hand(P1).iter().any(|card| card.def_id == ID),
                "Ticketed Bounced to hand"
            );
            assert!(held_ids(&s).is_empty());
            let back = s.card(vanilla.as_str()).clone();
            assert!(
                matches!(
                    back.zone,
                    Zone::Field {
                        player: P2,
                        row: Row::Units,
                        ..
                    }
                ),
                "{:?}",
                back.zone
            );
            assert_eq!(back.summoned_turn, Some(s.state().turn), "summoning sick");
        }

        #[test]
        fn r1875001_a_held_field_spell_comes_back_to_its_backrow_with_no_cry() {
            let mut s = board(false, json!([JAMMED]), json!([]), json!([{ "def": ARMOR, "lane": 3 }]));
            let armor = s.backrow(P2, 3).expect("the Field Spell").id;
            let hand = s.hand(P2).len();
            ticket(&mut s, &armor);
            assert_eq!(held_ids(&s), vec![armor.clone()]);
            let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
            s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
            assert!(matches!(s.card(armor.as_str()).zone, Zone::Field { player: P2, row: Row::Backrow, .. }));
            assert_eq!(s.hand(P2).len(), hand, "no Cry: nothing drawn");
        }

        #[test]
        fn r1875001_it_may_hold_its_own_sides_permanent_but_no_face_down_card() {
            let mut s = board(false, json!([]), json!([]), json!([{ "def": TRAP, "lane": 2 }]));
            let trap = s.backrow(P2, 2).expect("the set Trap").id;
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&trap) })));
            let mut own = crate::scenario(json!({
                "p1": {
                    "hand": [ID, FILLER],
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "library": crate::filler(4),
                    "mana": 10,
                },
            }));
            let vanilla = own.unit_at(P1, 1).id;
            own.play(ID, json!({ "zone": 1, "targets": pick::id(&vanilla) }));
            assert_eq!(held_ids(&own), vec![vanilla]);
        }

        #[test]
        fn r11_a_held_unit_token_is_gone_for_good() {
            let mut s = board(false, json!([JAMMED]), json!([{ "def": RUSH_TOKEN, "lane": 1 }]), json!([]));
            let token = s.unit_at(P2, 1).id;
            ticket(&mut s, &token);
            assert!(held_ids(&s).is_empty());
            let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
            s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
            assert!(s.unit(P2, 1).is_none());
        }

        #[test]
        fn r1850003_with_no_open_zone_the_card_is_exiled() {
            let field: Vec<Value> = (1..=5).map(|lane| json!({ "def": VANILLA, "lane": lane })).collect();
            let mut s = crate::scenario(json!({
                "seed": "ticketed-full",
                "p1": { "hand": [ID, JAMMED, FILLER], "library": crate::filler(8), "mana": 10 },
                "p2": { "hand": [VANILLA, FILLER], "field": field, "library": crate::filler(8) },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let held = s.unit_at(P2, 1).id;
            ticket(&mut s, &held);
            s.end_turn();
            s.state_mut().players[P2].mana.current = 10;
            let refill = s
                .hand(P2)
                .into_iter()
                .find(|card| card.def_id == VANILLA)
                .expect("p2's Vanilla")
                .id;
            s.play(refill.as_str(), json!({ "zone": 1 }));
            s.end_turn();
            s.state_mut().players[P1].mana.current = 10;
            let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
            s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
            s.expect_in_zone(held.as_str(), "exile");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1875001_with_no_brittle_it_holds_until_it_leaves() {
            let mut s = board(true, json!([JAMMED]), json!([{ "def": VANILLA, "lane": 2 }]), json!([]));
            let vanilla = s.unit_at(P2, 2).id;
            ticket(&mut s, &vanilla);
            for _ in 0..4 {
                crate::next_turn_of(&mut s, P1);
            }
            assert_eq!(held_ids(&s), vec![vanilla.clone()]);
            s.state_mut().players[P1].mana.current = 10;
            let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
            s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
            assert!(matches!(s.card(vanilla.as_str()).zone, Zone::Field { player: P2, .. }));
        }
    }
}
