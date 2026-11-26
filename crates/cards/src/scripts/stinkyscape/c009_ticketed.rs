//! S #9 Ticketed (SPEC §8.10 row 9; R4665000, R4835000, R1850003, R1875001): (2) Field Spell, Common,
//! of the Stinkyscape set.
//!   Base:    "Brittle 3 / Stack this on a permanent anywhere."
//!   Radiant: the same without Brittle 3.
//! Engine: Brittle 3 is a catalog keyword (R638). Since issue #4665 (R4665000) its play takes no zone of
//! its own (`StaticFlags.stacks_on_target`): it declares one permanent on the field (R81), either side's,
//! a Unit or a backrow card, face-down or not, and lands on top of that permanent's pile, in its zone and
//! that side's control (§3.2 Stack). Its play is offered only a permanent whose zone a card played onto
//! it may enter: no GIGA, none in a Locked zone (R4835000). The permanent lies beneath it as a card
//! beneath a Stack does, out of play and keeping its damage, and resumes when Ticketed leaves the field,
//! any way it leaves. On a unit zone it lies there, no Unit (`zones::lies_on_unit_zone`): it fills the
//! zone, nothing attacks it, no health judges it, and no Unit sweep, Cleave or other read of the Units
//! beside a card reaches it (`zones::is_adjacent_card`); an effect that names a backrow card, or the
//! permanents beside a pick (R2198003, `zones::is_adjacent_permanent`), may name it, and a sweep of the
//! backrow, of the Field Spells or of every permanent meets it after that side's backrow lanes. With no
//! permanent left to stack on it lands in its graveyard (R138). Radiant: no Brittle, so it stays until it
//! leaves. Through v0.6.12c its Cry held another face-up permanent beneath it in its own zone
//! (`hold_beneath`, R1850003, R1875001), which `script_v0_6_12c` keeps (R767000). Declares no numbers.

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
/// row, face-down or not, its pile the one this lands on. R4835000: never a permanent whose zone refuses a
/// card played onto it (a GIGA, a Locked zone).
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

/// R767000, R4665000: through v0.6.12c its Cry held another face-up permanent beneath it in its own
/// zone (R1850003, R1875001); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_12c() -> CardScripts {
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
    const PAWN: &str = "core-096"; // My Pawn: a Trap that fires only on a lethal attack, never on a play
    const PLAGUE_NUKE: &str = "classic-043"; // Plague Nuke: "Destroy all Units. …"
    const GUY_ATT: &str = "classicplus-005"; // Guy Att: Radiant "Cry: Destroy every backrow card."
    const FIRE: &str = "classicplus-t-ai-06"; // Datacenter Fire: "Destroy all Field Spells. Deal 1 damage …"
    const CLEAVER: &str = "stand-in (1) 4/9 Cleave";
    const BEAN: &str = "traininggrounds-027"; // Hopeful Jelly Bean: Radiant "Cleave / Make a permanent Radiant."
    const LASER_DRILL: &str = "stinkyscapeplus-053"; // Laser Drill: a GIGA Field Spell
    const LLM: &str = "stinkyscapeplus-t-ai-04"; // Jlockeed Large Language Model: a GIGA Unit token
    const LOCKDOWN: &str = "classic-084"; // Lockdown: a Crater Field Spell, Locked in its zone
    const MAYOR: &str = "meditative-086"; // Mayor Medinamogger: "Aura: All targets are chosen at random."
    const BRINGER: &str = "classic-076"; // Plague Bringer: "Cry: Place 2 Plague Counters. Draw 1."

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

    /// R4665000: p2 holds a Vanilla in unit lanes 1 and 3 and a set My Pawn in backrow lane 1; p1 stacks
    /// Ticketed on the lane-3 Vanilla. The ids: the lane-1 Vanilla, My Pawn, Ticketed, the Vanilla beneath.
    fn stacked(hand: Value) -> (Scenario, [String; 4]) {
        let mut s = board(
            false,
            hand,
            json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]),
            json!([{ "def": PAWN, "faceUp": false, "lane": 1 }]),
        );
        let left = s.unit_at(P2, 1).id;
        let under = s.unit_at(P2, 3).id;
        let pawn = s.backrow(P2, 1).expect("the set My Pawn").id;
        stack_on(&mut s, &under);
        let ticketed = s.unit(P2, 3).expect("Ticketed").id;
        (s, [left, pawn, ticketed, under])
    }

    /// The ids a board scope walks now, read under a context p1 controls (R68's order).
    fn scope_ids(s: &mut Scenario, scope: Value) -> Vec<String> {
        let scope: jackioh_engine::effects::BoardScope = json_as(scope);
        let mut rng = create_rng(&s.state().seed, s.state().rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
        let ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            },
        );
        jackioh_engine::effects::cards_in_scope(&ctx, &scope)
            .into_iter()
            .map(|card| card.id)
            .collect()
    }

    /// The ids `permanents_on_field` lists (`reachable_permanents_on_field` with `reachable`), `first`'s
    /// side first.
    fn permanent_ids(s: &Scenario, first: PlayerId, reachable: bool) -> Vec<String> {
        let cards = if reachable {
            jackioh_engine::query::reachable_permanents_on_field(s.state(), first)
        } else {
            jackioh_engine::query::permanents_on_field(s.state(), first)
        };
        cards.into_iter().map(|card| card.id.clone()).collect()
    }

    /// The instance ids the open prompt offers, in order.
    fn option_ids(s: &Scenario) -> Vec<String> {
        must(s.state().pending.as_ref(), "an open prompt")
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
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

        /// R4665000: lying on a unit zone it is no Unit: its controller can neither switch it nor attack
        /// with it, and no switch or attack of it is listed.
        #[test]
        fn r4665000_a_lying_ticketed_neither_switches_nor_attacks() {
            let mut s = board(false, json!([]), json!([{ "def": LAR, "lane": 2 }]), json!([]));
            let lar = s.unit_at(P2, 2).id;
            stack_on(&mut s, &lar);
            let ticketed = s.unit(P2, 2).expect("Ticketed").id;
            s.end_turn();
            assert_eq!(s.state().active, P2);
            s.expect_refused(|s| s.switch_position(ticketed.as_str()));
            s.expect_refused(|s| s.attack(ticketed.as_str(), "hero"));
            let listed = legal_actions(s.state(), P2);
            assert!(
                !listed.iter().any(|action| crate::js(action)["instanceId"] == json!(ticketed)),
                "{listed:?}"
            );
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

        /// R638, R4665000: Brittle 3 ticks at its controller's turns, p2's here, from the first a full
        /// turn cycle after it entered (`BRITTLE_FIRST_TICK_TURNS`), so p2's fourth turn crumbles it;
        /// crumbled, the Unit beneath resumes on top.
        #[test]
        fn r638_r4665000_brittle_3_crumbles_it_and_the_unit_beneath_resumes() {
            let mut s = board(false, json!([]), json!([{ "def": SORCERER, "lane": 3 }]), json!([]));
            let sorcerer = s.unit_at(P2, 3).id;
            stack_on(&mut s, &sorcerer);
            for _ in 0..3 {
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

        /// R4665000: a board scope reads it as the backrow card it is: a unit scope passes it by, and a
        /// backrow scope, or one of both rows, meets it once, after that side's backrow lanes.
        #[test]
        fn r4665000_a_scope_reads_it_as_a_backrow_card_after_that_sides_backrow_lanes() {
            let (mut s, [left, pawn, ticketed, _]) = stacked(json!([]));
            assert_eq!(scope_ids(&mut s, json!({ "side": "enemy" })), vec![left.clone()]);
            assert_eq!(
                scope_ids(&mut s, json!({ "side": "enemy", "rows": ["backrow"] })),
                vec![pawn.clone(), ticketed.clone()]
            );
            assert_eq!(
                scope_ids(&mut s, json!({ "side": "enemy", "rows": ["units", "backrow"] })),
                vec![left, pawn, ticketed]
            );
        }

        /// R4665000: "Destroy all Units" (Plague Nuke) passes it by, so the Unit beneath stays beneath.
        #[test]
        fn r4665000_a_unit_sweep_passes_it_by_and_the_unit_beneath_stays_beneath() {
            let (mut s, [left, _, ticketed, under]) = stacked(json!([PLAGUE_NUKE]));
            s.play(PLAGUE_NUKE, json!({}));
            s.expect_in_zone(left.as_str(), "graveyard");
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(ticketed));
            assert_eq!(beneath(&s, P2, Row::Units, 3), vec![under]);
        }

        /// R4665000: "Destroy every backrow card" (Radiant Guy Att) destroys it on its unit zone, with the
        /// set Trap; the Unit beneath resumes.
        #[test]
        fn r4665000_a_backrow_sweep_destroys_it_and_the_unit_beneath_resumes() {
            let (mut s, [_, pawn, ticketed, under]) = stacked(json!([{ "def": GUY_ATT, "radiant": true }]));
            s.play(GUY_ATT, json!({ "zone": 1 }));
            s.expect_in_zone(pawn.as_str(), "graveyard");
            s.expect_in_zone(ticketed.as_str(), "graveyard");
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(under));
        }

        /// R4665000, T-AI-6: "Destroy all Field Spells" (Datacenter Fire) destroys it, the one Field Spell
        /// (My Pawn is a Trap), and counts it: 1 damage to each hero.
        #[test]
        fn r4665000_a_field_spell_sweep_destroys_and_counts_it() {
            let (mut s, [_, _, ticketed, under]) = stacked(json!([FIRE]));
            s.play(FIRE, json!({}));
            s.expect_in_zone(ticketed.as_str(), "graveyard");
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(under));
            s.expect_health(P1, HERO_HEALTH - 1).expect_health(P2, HERO_HEALTH - 1);
        }

        /// R4665000, §4.4 step 10: Cleave strikes the Units beside its target, and a card lying on the
        /// zone beside it is none: neither the attack nor its projection (`cleave_struck`) reaches it.
        #[test]
        fn r4665000_cleave_neither_strikes_nor_projects_onto_it() {
            let mut s = crate::scenario(json!({
                "seed": "ticketed-cleave",
                "p1": {
                    "hand": [ID, FILLER],
                    "field": [{ "def": CLEAVER, "lane": 1 }],
                    "library": crate::filler(8),
                    "mana": 10,
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": VANILLA, "lane": 1 },
                        { "def": VANILLA, "lane": 2 },
                        { "def": VANILLA, "lane": 3 },
                    ],
                    "library": crate::filler(8),
                },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let cleaver = s.unit_at(P1, 1).id;
            let left = s.unit_at(P2, 1).id;
            let defender = s.unit_at(P2, 2).id;
            let under = s.unit_at(P2, 3).id;
            stack_on(&mut s, &under);
            let ticketed = s.unit(P2, 3).expect("Ticketed").id;
            let struck: Vec<String> =
                jackioh_engine::query::cleave_struck(s.state(), &s.unit_at(P1, 1), &s.unit_at(P2, 2))
                    .into_iter()
                    .map(|card| card.id)
                    .collect();
            assert_eq!(struck, vec![defender.clone(), left.clone()]);
            s.attack(cleaver.as_str(), defender.as_str());
            s.expect_in_zone(left.as_str(), "graveyard");
            assert_eq!(s.card(ticketed.as_str()).damage, 0);
            assert_eq!(s.card(under.as_str()).damage, 0);
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(ticketed));
        }

        /// R4665000, §3.1, R2198003: "the permanents adjacent to" a pick (Radiant Collateral Damage) are
        /// more than its Units: it lies beside the pick as the Field Spell it is, so it goes too, and the Unit
        /// beneath resumes, untouched.
        #[test]
        fn r4665000_the_permanents_beside_a_pick_take_it_and_the_unit_beneath_resumes() {
            let mut s = board(
                false,
                json!([{ "def": COLLATERAL, "radiant": true }]),
                json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
                json!([]),
            );
            let pick = s.unit_at(P2, 2).id;
            let under = s.unit_at(P2, 3).id;
            stack_on(&mut s, &under);
            let ticketed = s.unit(P2, 3).expect("Ticketed").id;
            s.play(COLLATERAL, json!({ "targets": pick::id(&pick) }));
            s.expect_in_zone(pick.as_str(), "exile");
            s.expect_in_zone(ticketed.as_str(), "exile");
            assert_eq!(s.unit(P2, 3).map(|top| top.id), Some(under));
        }

        /// R4665000, R2198003: a Spell's Cleave on a board pick (Radiant Hopeful Jelly Bean) reaches the
        /// permanents beside it, so it turns Radiant beside the pick.
        #[test]
        fn r4665000_a_spells_cleave_on_a_board_pick_reaches_it() {
            let mut s = board(
                false,
                json!([{ "def": BEAN, "radiant": true }]),
                json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 3 }]),
                json!([]),
            );
            let pick = s.unit_at(P2, 2).id;
            let under = s.unit_at(P2, 3).id;
            stack_on(&mut s, &under);
            let ticketed = s.unit(P2, 3).expect("Ticketed").id;
            assert!(!s.card(ticketed.as_str()).radiant);
            s.play(BEAN, json!({ "targets": pick::id(&pick) }));
            assert!(s.card(pick.as_str()).radiant);
            assert!(s.card(ticketed.as_str()).radiant);
            assert!(!s.card(under.as_str()).radiant);
        }

        /// R3550009, R4665000: every enemy permanent lists it after their backrow, and never the Unit
        /// beneath it.
        #[test]
        fn r4665000_enemy_permanents_lists_it_after_their_backrow() {
            let (s, [left, pawn, ticketed, _]) = stacked(json!([]));
            let permanents: Vec<String> = jackioh_engine::query::enemy_permanents(s.state(), P1)
                .into_iter()
                .map(|card| card.id)
                .collect();
            assert_eq!(permanents, vec![left, pawn, ticketed]);
        }

        /// R4665000: every permanent on the field meets it once, after its own side's backrow and before
        /// the other side, whichever side is met first.
        #[test]
        fn r4665000_every_permanent_meets_it_after_that_sides_backrow_and_once() {
            let (mut s, [left, pawn, ticketed, _]) = stacked(json!([BRINGER]));
            s.play(BRINGER, json!({}));
            let bringer = s.card(BRINGER).id.clone();
            let theirs_last = vec![bringer.clone(), left.clone(), pawn.clone(), ticketed.clone()];
            let theirs_first = vec![left, pawn, ticketed, bringer];
            for reachable in [false, true] {
                assert_eq!(permanent_ids(&s, P1, reachable), theirs_last, "reachable {reachable}");
                assert_eq!(permanent_ids(&s, P2, reachable), theirs_first, "reachable {reachable}");
            }
        }

        /// R4665000: a Plague Counter placement offers it after that side's backrow, and may land on it.
        #[test]
        fn r4665000_a_plague_placement_offers_it_after_that_sides_backrow() {
            let (mut s, [left, pawn, ticketed, _]) = stacked(json!([BRINGER]));
            s.play(BRINGER, json!({}));
            let bringer = s.card(BRINGER).id.clone();
            assert_eq!(option_ids(&s), vec![bringer, left, pawn, ticketed.clone()]);
            s.answer(json!(ticketed));
            assert_eq!(s.card(ticketed.as_str()).counters.plague, Some(2));
        }

        /// R4665000, R60: a random exile of the field draws over that order: at a cursor where it and the
        /// order that met it among the Units part ways, the card the spec's order draws goes.
        #[test]
        fn r4665000_a_random_exile_of_the_field_draws_over_the_spec_order() {
            let (mut s, [left, pawn, ticketed, _]) = stacked(json!([]));
            let spec = vec![left.clone(), pawn.clone(), ticketed.clone()];
            let among_units = vec![left, ticketed, pawn]; // where the list met it before R4665000's order
            let seed = s.state().seed.clone();
            let cursor = cursor_where(&seed, |cursor| {
                create_rng(&seed, cursor).shuffle(&spec)[0] != create_rng(&seed, cursor).shuffle(&among_units)[0]
            });
            let hit = create_rng(&seed, cursor).shuffle(&spec)[0].clone();
            let spared = create_rng(&seed, cursor).shuffle(&among_units)[0].clone();
            let mut events: Vec<GameEvent> = Vec::new();
            let mut rng = create_rng(&seed, cursor);
            {
                let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                let mut ctx = make_context(
                    &mut sink,
                    None,
                    HookOptions {
                        controller: Some(P1),
                        ..HookOptions::default()
                    },
                );
                let exile = jackioh_engine::effects::exile_random_from(json_as(json!({
                    "player": "enemy", "from": "field", "count": 1,
                })));
                jackioh_engine::resolve::apply_effects(&[exile], &mut ctx);
            }
            s.expect_in_zone(hit.as_str(), "exile");
            s.expect_in_zone(spared.as_str(), "field");
        }

        /// R90, R138, R4665000: with no permanent on the field to stack on, it lands in its graveyard.
        #[test]
        fn r4665000_with_no_permanent_to_stack_on_it_lands_in_its_graveyard() {
            let mut s = board(false, json!([]), json!([]), json!([]));
            s.play(ID, json!({}));
            s.expect_in_zone(ID, "graveyard");
            assert!(s.backrow(P1, 1).is_none());
        }

        /// R4835000, R3546000: no GIGA, its own side's or the enemy's, is offered, and a play naming one is
        /// refused, nothing paid and Ticketed still in hand; a permanent it can stack on still takes it.
        #[test]
        fn r4835000_it_is_offered_no_giga_and_a_play_naming_one_is_refused() {
            let mut s = crate::scenario(json!({
                "seed": "ticketed-giga",
                "p1": {
                    "hand": [ID, FILLER],
                    "field": [{ "def": VANILLA, "lane": 5 }],
                    "backrow": [{ "def": LASER_DRILL, "lane": 1, "faceUp": true }],
                    "library": crate::filler(8),
                    "mana": 10,
                },
                "p2": {
                    "hand": [FILLER],
                    "field": [{ "def": LLM, "lane": 1 }, { "def": VANILLA, "lane": 5 }],
                    "library": crate::filler(8),
                },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let own = s.unit_at(P1, 5).id;
            let enemy = s.unit_at(P2, 5).id;
            assert_eq!(pick::instance_ids(&s.play_targets(ID)), vec![own, enemy.clone()]);
            let drill = s.backrow(P1, 1).expect("the Laser Drill").id;
            let llm = s.unit_at(P2, 1).id;
            for giga in [drill, llm] {
                s.expect_refused_with(|s| s.play(ID, json!({ "targets": pick::id(&giga) })), "not a legal target");
            }
            s.expect_in_zone(ID, "hand");
            assert_eq!(s.state().players[P1].mana.current, 10, "nothing paid");
            stack_on(&mut s, &enemy);
            assert_eq!(s.unit(P2, 5).map(|top| top.def_id), Some(ID.to_string()));
        }

        /// R4835000, R688: no permanent in a Locked zone is offered, Lockdown in the zone its Crater Locked
        /// among them, and a play naming one is refused; an unlocked one still takes it.
        #[test]
        fn r4835000_it_is_offered_no_permanent_in_a_locked_zone() {
            let mut s = board(
                false,
                json!([]),
                json!([{ "def": VANILLA, "lane": 2 }, { "def": VANILLA, "lane": 4 }]),
                json!([{ "def": LOCKDOWN, "lane": 3, "faceUp": true }]),
            );
            for (row, lane) in [(Row::Units, 2), (Row::Backrow, 3)] {
                jackioh_engine::zones::lock_zone(
                    s.state_mut(),
                    jackioh_engine::zones::ZoneSlot { player: P2, row, lane },
                );
            }
            let locked = s.unit_at(P2, 2).id;
            let lockdown = s.backrow(P2, 3).expect("Lockdown").id;
            let open = s.unit_at(P2, 4).id;
            assert_eq!(pick::instance_ids(&s.play_targets(ID)), vec![open.clone()]);
            for named in [locked, lockdown] {
                s.expect_refused_with(|s| s.play(ID, json!({ "targets": pick::id(&named) })), "not a legal target");
            }
            stack_on(&mut s, &open);
            assert_eq!(s.unit(P2, 4).map(|top| top.def_id), Some(ID.to_string()));
            assert_eq!(beneath(&s, P2, Row::Units, 4), vec![open]);
        }

        /// R4835000, R1200: while a Mayor acts the target drawn for it is one it can stack on, never a GIGA:
        /// with the Mayor and an enemy GIGA on the field, every seed stacks it on the Mayor.
        #[test]
        fn r4835000_under_a_mayor_the_drawn_target_is_never_a_giga() {
            for n in 0..12 {
                let mut s = crate::scenario(json!({
                    "seed": format!("ticketed-mayor-{n}"),
                    "p1": {
                        "hand": [ID, FILLER],
                        "field": [{ "def": MAYOR, "lane": 5 }],
                        "library": crate::filler(8),
                        "mana": 10,
                    },
                    "p2": { "hand": [FILLER], "field": [{ "def": LLM, "lane": 1 }], "library": crate::filler(8) },
                }));
                let mayor = s.unit_at(P1, 5).id;
                s.play(ID, json!({}));
                assert_eq!(s.unit(P1, 5).map(|top| top.def_id), Some(ID.to_string()), "seed {n}");
                assert_eq!(beneath(&s, P1, Row::Units, 5), vec![mayor], "seed {n}");
            }
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

        const IKWZKI: &str = "stinkyscape-110"; // iKwzki: "Cry: Duo with a permanent and make it Radiant next turn."
        const HANGER: &str = "stinkyscape-168"; // Drone Hanger: a face-up Field Spell to stack on
        const PRESTIGE: &str = "meditative-053"; // Prestige: "De-Radiant a Radiant card on the field or …"

        /// p1 holds Ticketed (`radiant` or not) and `hand`, with a face-up Drone Hanger in backrow lane 2;
        /// Ticketed is played onto the Hanger and returned.
        fn on_the_hanger(radiant: bool, hand: Value) -> (Scenario, CardInstance) {
            let mut cards = vec![json!({ "def": ID, "radiant": radiant })];
            cards.extend(hand.as_array().cloned().unwrap_or_default());
            cards.extend([json!(FILLER), json!(FILLER), json!(FILLER)]);
            let mut s = crate::scenario(json!({
                "seed": "ticketed-face",
                "p1": {
                    "hand": cards,
                    "backrow": [{ "def": HANGER, "faceUp": true, "lane": 2 }],
                    "library": crate::filler(14),
                    "mana": 10,
                },
                "p2": { "hand": [FILLER, FILLER, FILLER], "library": crate::filler(14) },
            }));
            for player in [P1, P2] {
                s.state_mut().players[player].auto_end_turn = Some(false);
            }
            let hanger = s.backrow(P1, 2).expect("the Drone Hanger");
            stack_on(&mut s, &hanger.id);
            let ticketed = s.card(ID).clone();
            (s, ticketed)
        }

        fn on_field(s: &Scenario, card: &CardInstance) -> bool {
            matches!(s.card(card).zone, Zone::Field { .. })
        }

        /// R4817000, R1875001: a base Ticketed made Radiant where it stands (S #110 iKwzki's Duo, at the
        /// start of p1's next turn) wears a face with no Brittle, so the count its base face's Brittle 3
        /// started goes and it holds until it is removed.
        #[test]
        fn r4817000_a_base_ticketed_made_radiant_where_it_stands_stops_crumbling() {
            let (mut s, ticketed) = on_the_hanger(false, json!([IKWZKI]));
            assert_eq!(s.card(&ticketed).brittle.map(|brittle| brittle.count), Some(3));
            s.play(IKWZKI, json!({ "zone": 1, "targets": pick::instance(&ticketed) }));
            s.end_turn();
            s.end_turn();
            assert!(s.card(&ticketed).radiant, "iKwzki's Duo made it Radiant at the start of p1's next turn");
            assert!(s.card(&ticketed).brittle.is_none(), "its base face's count went with that face");
            for _ in 0..3 {
                s.end_turn();
                s.end_turn();
            }
            assert!(
                on_field(&s, &ticketed),
                "R1875001: the Radiant face has no Brittle; it is in {:?} with {} `crumbled`",
                s.card(&ticketed).zone,
                s.count("crumbled")
            );
        }

        /// R4817000: the mirror. A Radiant Ticketed De-Radianted where it stands (M #53 Prestige) wears
        /// its base face again, so its Brittle 3 starts then and crumbles it three of p1's turns later.
        #[test]
        fn r4817000_a_radiant_ticketed_de_radianted_where_it_stands_starts_brittle_3() {
            let (mut s, ticketed) = on_the_hanger(true, json!([PRESTIGE]));
            assert!(s.card(&ticketed).brittle.is_none());
            s.play(PRESTIGE, pick::targets(pick::id(&ticketed.id)));
            assert!(!s.card(&ticketed).radiant, "Prestige De-Radianted it");
            assert_eq!(s.card(&ticketed).brittle.map(|brittle| brittle.count), Some(3));
            for _ in 0..2 {
                s.end_turn();
                s.end_turn();
            }
            assert!(on_field(&s, &ticketed), "its count is at 1");
            s.end_turn();
            s.end_turn();
            assert!(matches!(s.card(&ticketed).zone, Zone::Graveyard { .. }), "its Brittle 3 crumbled it");
        }
    }

    /// R767000: the script kept for v0.6.12c, its Cry holding a face-up permanent beneath it in its own
    /// zone (R1850003, R1875001).
    mod kept_v0_6_12c {
        use super::*;

        mod base {
            use super::*;

            /// R2476000: after "the permanent beneath it", the Unit it holds by name, to both seats.
            #[test]
            fn r2476000_it_names_the_permanent_it_holds() {
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
                crate::under_kept(ID, super::super::super::script_v0_6_12c(), || {
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
