//! S #9 Ticketed (SPEC §8.10 row 9; R4665000, R1850003, R1875001): (2) Field Spell, Common, of the
//! Stinkyscape set.
//!   Base:    "Brittle 3 / Stack this on a permanent anywhere."
//!   Radiant: the same without Brittle 3.
//! Engine: Brittle 3 is a catalog keyword (R638). Since issue #4665 (R4665000) its play takes no zone of
//! its own (`StaticFlags.stacks_on_target`): it declares one permanent on the field (R81), either side's,
//! a Unit or a backrow card, face-down or not, and lands on top of that permanent's pile, in its zone and
//! that side's control (§3.2 Stack). The permanent lies beneath it as a card beneath a Stack does, out
//! of play and keeping its damage, and resumes when Ticketed leaves the field, any way it leaves. On a
//! unit zone it lies there, no Unit (`zones::lies_on_unit_zone`): it fills the zone, nothing attacks it,
//! no health judges it, and an effect that names a backrow card may name it. With no permanent left to
//! stack on it lands in its graveyard (R138). Radiant: no Brittle, so it stays until it leaves. Through
//! v0.6.12b its Cry held another face-up permanent beneath it in its own zone (`hold_beneath`,
//! R1850003, R1875001), which `script_v0_6_12b` keeps (R767000). Declares no numbers.

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

/// R4665000: "Stack this on a permanent anywhere": one permanent on the field, either side's, either
/// row, face-down or not, its pile the one this lands on.
fn stacks_anywhere() -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"], "excludeSelf": true }),
        )],
        static_flags: Some(StaticFlags {
            stacks_on_target: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The same script: the base face's Brittle 3 is catalog data.
    CardScripts {
        base: stacks_anywhere(),
        radiant: stacks_anywhere(),
    }
}

/// R767000, R4665000: through v0.6.12b its Cry held another face-up permanent beneath it in its own
/// zone (R1850003, R1875001); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_12b() -> CardScripts {
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

    const FILLER: &str = "core-005"; // Stockpile: a Spell, a card in hand, so no turn ends by itself
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const LAR: &str = "classicplus-076"; // Brother Lar: "Death: Summon a Brother Ping."
    const SORCERER: &str = "core-068"; // Twisted Sorcerer: "Cry: Deal 4 damage."
    const ARMOR: &str = "core-073"; // Anti-oneshot Armor: "… Cry: Draw 1."
    const RUSH_TOKEN: &str = "core-t-rush"; // Rush Token: a unit token
    const JAMMED: &str = "core-036"; // Magic Jammed: "Destroy a backrow card. Lock its zone."
    const COLLATERAL: &str = "core-034"; // Collateral Damage: "Exile a permanent and a random card …"
    const TRAP: &str = "core-041"; // Sheepish: a Trap, to set face-down
    const CORE_SYSTEM: &str = "stinkyscape-117"; // Core System: "Bounce all permanents. …"

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

    /// R4665000: the play that stacks it on `target`: no zone, the target declared (R81).
    fn stack_on(s: &mut Scenario, target: &str) {
        s.play(ID, json!({ "targets": pick::id(target) }));
    }

    /// The ids beneath the top of `player`'s `row` zone `lane`, top first.
    fn beneath(s: &Scenario, player: PlayerId, row: Row, lane: i32) -> Vec<String> {
        let slot = jackioh_engine::zones::ZoneSlot { player, row, lane };
        jackioh_engine::zones::beneath_at(s.state(), slot)
            .iter()
            .map(|card| card.id.clone())
            .collect()
    }

    /// p1 holds Ticketed and a Vanilla to attack with; p2 a Vanilla in lane 3.
    fn facing(radiant: bool) -> Scenario {
        let mut s = crate::scenario(json!({
            "seed": "ticketed-facing",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, JAMMED, FILLER],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "library": crate::filler(8),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [{ "def": VANILLA, "lane": 3 }], "library": crate::filler(8) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    mod base {
        use super::*;

        /// R4665000: on an enemy Unit, in that side's zone and control; the Unit lies beneath it, with no
        /// Death, and neither is a Unit in play.
        #[test]
        fn r4665000_it_stacks_on_an_enemy_unit_in_that_sides_control_and_the_unit_lies_beneath() {
            let mut s = board(false, json!([]), json!([{ "def": LAR, "lane": 2 }]), json!([]));
            let lar = s.unit_at(P2, 2).id;
            stack_on(&mut s, &lar);
            let top = s.unit(P2, 2).expect("Ticketed tops the zone");
            assert_eq!(top.def_id, ID);
            assert_eq!(top.controller, P2);
            assert_eq!(top.position, None, "it lies there, no Unit");
            assert_eq!(beneath(&s, P2, Row::Units, 2), vec![lar.clone()]);
            assert!(s.backrow(P1, 1).is_none(), "it takes no zone of its own");
            assert!(s.unit(P2, 1).is_none(), "no Brother Ping: no Death");
            let in_play = jackioh_engine::zones::active_units_of(s.state(), P2);
            assert!(in_play.iter().all(|unit| unit.id != top.id && unit.id != lar), "neither is a Unit");
        }

        /// R4665000: lying on the unit zone it fills it, and no Unit may attack it.
        #[test]
        fn r4665000_on_a_unit_zone_it_fills_the_zone_and_nothing_attacks_it() {
            let mut s = facing(false);
            let enemy = s.unit_at(P2, 3).id;
            stack_on(&mut s, &enemy);
            let ticketed = s.unit(P2, 3).expect("Ticketed").id;
            let slot = jackioh_engine::zones::ZoneSlot { player: P2, row: Row::Units, lane: 3 };
            assert!(!jackioh_engine::zones::is_open(s.state(), slot), "the zone is filled");
            let attacker = s.unit_at(P1, 1).id;
            s.expect_refused(|s| s.attack(attacker.as_str(), ticketed.as_str()));
        }

        /// R4665000: on a face-down Trap, which lies beneath it and cannot fire (§3.2).
        #[test]
        fn r4665000_it_stacks_on_an_enemy_face_down_trap() {
            let mut s = board(false, json!([]), json!([]), json!([{ "def": TRAP, "lane": 2 }]));
            let trap = s.backrow(P2, 2).expect("the set Trap").id;
            stack_on(&mut s, &trap);
            let top = s.backrow(P2, 2).expect("Ticketed tops the zone");
            assert_eq!((top.def_id.as_str(), top.controller), (ID, P2));
            assert_eq!(beneath(&s, P2, Row::Backrow, 2), vec![trap]);
        }

        /// R4665000: anywhere, its own side's permanents included.
        #[test]
        fn r4665000_it_may_stack_on_its_own_sides_permanent() {
            let mut own = crate::scenario(json!({
                "p1": {
                    "hand": [ID, FILLER],
                    "field": [{ "def": VANILLA, "lane": 1 }],
                    "library": crate::filler(4),
                    "mana": 10,
                },
            }));
            let vanilla = own.unit_at(P1, 1).id;
            stack_on(&mut own, &vanilla);
            let top = own.unit(P1, 1).expect("Ticketed");
            assert_eq!((top.def_id.as_str(), top.controller), (ID, P1));
            assert_eq!(beneath(&own, P1, Row::Units, 1), vec![vanilla]);
        }

        /// R638, R4665000: Brittle 3 ticks at its controller's turns, p2's here; crumbled, the Unit
        /// beneath resumes on top.
        #[test]
        fn r638_r4665000_brittle_3_crumbles_it_and_the_unit_beneath_resumes() {
            let mut s = board(false, json!([]), json!([{ "def": SORCERER, "lane": 3 }]), json!([]));
            let sorcerer = s.unit_at(P2, 3).id;
            stack_on(&mut s, &sorcerer);
            for _ in 0..2 {
                crate::next_turn_of(&mut s, P2);
                assert_eq!(s.unit(P2, 3).map(|top| top.def_id), Some(ID.to_string()));
            }
            crate::next_turn_of(&mut s, P2);
            s.expect_in_zone(ID, "graveyard");
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(sorcerer));
        }

        /// R4665000: an effect that names a backrow card may name it on a unit zone; destroyed, the Unit
        /// beneath resumes.
        #[test]
        fn r4665000_destroyed_the_unit_beneath_resumes() {
            let mut s = facing(false);
            let enemy = s.unit_at(P2, 3).id;
            stack_on(&mut s, &enemy);
            let ticketed = s.unit(P2, 3).expect("Ticketed").id;
            s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
            s.expect_in_zone(ticketed.as_str(), "graveyard");
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(enemy));
        }

        /// R90, R138, R4665000: with no permanent on the field to stack on, it lands in its graveyard.
        #[test]
        fn r4665000_with_no_permanent_to_stack_on_it_lands_in_its_graveyard() {
            let mut s = board(false, json!([]), json!([]), json!([]));
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "graveyard");
            assert!(s.backrow(P1, 1).is_none());
        }
    }

    mod radiant {
        use super::*;

        /// R4665000: with no Brittle it stays until it leaves.
        #[test]
        fn r4665000_with_no_brittle_it_stays_until_it_leaves() {
            let mut s = facing(true);
            let enemy = s.unit_at(P2, 3).id;
            stack_on(&mut s, &enemy);
            for _ in 0..4 {
                crate::next_turn_of(&mut s, P1);
            }
            let ticketed = s.unit(P2, 3).expect("Ticketed stays").id;
            assert_eq!(s.card(ticketed.as_str()).def_id, ID);
            s.state_mut().players[P1].mana.current = 10;
            s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(enemy));
        }
    }

    /// R767000: the script kept for v0.6.12b, its Cry holding a face-up permanent beneath it in its own
    /// zone (R1850003, R1875001).
    mod kept_v0_6_12b {
        use super::*;

        mod base {
            use super::*;

            /// R2476000: after "the permanent beneath it", the Unit it holds by name, to both seats.
            #[test]
            fn r2476000_it_names_the_permanent_it_holds() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
                    for radiant in [false, true] {
                        let mut s = board(radiant, json!([]), json!([{ "def": LAR, "lane": 2 }]), json!([]));
                        let lar = s.unit_at(P2, 2).id;
                        ticket(&mut s, &lar);
                        let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
                        let named = vec![("the permanent beneath it".to_string(), 1, Some("Brother Lar".to_string()))];
                        assert_eq!(crate::preview_seen(&s, P1, &ticketed), named, "radiant {radiant}");
                        assert_eq!(crate::preview_seen(&s, P2, &ticketed), named, "radiant {radiant}");
                    }
                });
            }

            #[test]
            fn r1850003_it_holds_a_chosen_enemy_unit_with_no_death_and_frees_its_zone() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
                    let mut s = board(false, json!([]), json!([{ "def": LAR, "lane": 2 }]), json!([]));
                    let lar = s.unit_at(P2, 2).id;
                    ticket(&mut s, &lar);
                    assert_eq!(held_ids(&s), vec![lar.clone()]);
                    assert!(s.unit(P2, 2).is_none(), "its zone is free");
                    assert!(s.unit(P2, 1).is_none(), "no Brother Ping: no Death");
                    assert!(s.last_events().iter().any(|event| event.event_type() == GameEventType::Held));
                });
            }

            #[test]
            fn r638_once_brittle_3_crumbles_it_the_unit_comes_back_sick_and_with_no_cry() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
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
                });
            }

            #[test]
            fn r1875001_destroyed_or_exiled_it_releases_at_once() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
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
                });
            }

            #[test]
            fn r1875001_bounced_it_releases_at_once() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
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
                });
            }

            #[test]
            fn r1875001_a_held_field_spell_comes_back_to_its_backrow_with_no_cry() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
                    let mut s = board(false, json!([JAMMED]), json!([]), json!([{ "def": ARMOR, "lane": 3 }]));
                    let armor = s.backrow(P2, 3).expect("the Field Spell").id;
                    let hand = s.hand(P2).len();
                    ticket(&mut s, &armor);
                    assert_eq!(held_ids(&s), vec![armor.clone()]);
                    let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
                    s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
                    assert!(matches!(s.card(armor.as_str()).zone, Zone::Field { player: P2, row: Row::Backrow, .. }));
                    assert_eq!(s.hand(P2).len(), hand, "no Cry: nothing drawn");
                });
            }

            #[test]
            fn r1875001_it_may_hold_its_own_sides_permanent_but_no_face_down_card() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
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
                });
            }

            #[test]
            fn r11_a_held_unit_token_is_gone_for_good() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
                    let mut s = board(false, json!([JAMMED]), json!([{ "def": RUSH_TOKEN, "lane": 1 }]), json!([]));
                    let token = s.unit_at(P2, 1).id;
                    ticket(&mut s, &token);
                    assert!(held_ids(&s).is_empty());
                    let ticketed = s.backrow(P1, 1).expect("Ticketed").id;
                    s.play(JAMMED, json!({ "targets": pick::id(&ticketed) }));
                    assert!(s.unit(P2, 1).is_none());
                });
            }

            #[test]
            fn r1850003_with_no_open_zone_the_card_is_exiled() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
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
                });
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1875001_with_no_brittle_it_holds_until_it_leaves() {
                crate::under_kept(ID, super::super::super::script_v0_6_12b(), || {
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
                });
            }
        }    }

}
