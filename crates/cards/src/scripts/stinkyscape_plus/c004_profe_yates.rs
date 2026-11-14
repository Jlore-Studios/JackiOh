//! SPEC §8.12 S+ #4 Profe Yates, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    7/7; "Cry: Exile every card with a M, O, U, S, E in its name. Cards in CN are
//!            immune. Then summon two random Traps. Then choose one of the following Activates.
//!            Activate 1: Overwhelm: Freeze all Units in this lane and the surrounding lanes.
//!            Activate 3: Wolf's Maw: Pull a Flying Unit down and deal 4 damage to it. It loses
//!            Flying. Activate 1: Iron Mandate: Fuse this into a Unit. This does not trigger
//!            its Cry again."
//!   Radiant: 14/14; "Cry:" the same exile, then three random Radiant Traps, then the same choice.
//!
//! Engine: the Cry exiles every card of both sides' fields, libraries, hands and graveyards
//! whose name holds M, O, U, S or E in either case (`exile_named`, R3567000), then summons
//! the traps — independent `summon_random` draws of non-token Traps/Field Traps of the
//! shipped sets, each face down in the leftmost open backrow zone (R3567001) — then remembers
//! the mode chosen at play (R81, ZolarKeth's shape). Yates keeps only that Activate while it
//! stays (R3567002): Overwhelm freezes every Unit of both sides in its lane and the lanes
//! beside it, Yates included (R3567003); Wolf's Maw grounds a declared Flying Unit of either
//! side — it loses Flying (`lose_keyword`, kept through every zone) then takes the declared
//! damage (R3567004); Iron Mandate fuses Yates into a declared friendly Unit, kept, with no
//! Cry run (R3567002). "Activate N" is N uses a turn (R384, R1857003).
//! An Equip fused into Yates reroots `yatesActivate` to `yatesActivate@0`
//! (`work::reroot_remembered`), hiding the chosen Activate; ZolarKeth's kept script shares
//! the limitation (R3567002).
//!
//! *{Spawn voiceline: Every pack needs an alpha I don't see anyone else stepping up}*
//! *{Attack voiceline: a mouse once bit my sister}*
//! *{Death voiceline: I will not see the Northern Lights again}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

use crate::query::TRAP_TYPES;

pub const ID: &str = "stinkyscapeplus-004";

/// The exile's letters: a card whose name holds M, O, U, S or E, in either case, is exiled.
const MOUSE: &str = "MOUSE";
/// The Cry's three Activate modes, in the order the card prints them (R81).
const OVERWHELM: &str = "overwhelm";
const WOLFS_MAW: &str = "wolfsMaw";
const IRON_MANDATE: &str = "ironMandate";
/// What the Cry remembers: the chosen Activate, kept while Yates stays (R78).
const CHOSEN_KEY: &str = "yatesActivate";
/// §6.2 Activate N, the script's literal, as S+ #11's (`ActivationUses` has no param); matches the declared `overwhelm`/`wolf`/`iron`.
const OVERWHELM_USES: i32 = 1;
const WOLFS_MAW_USES: i32 = 3;
const IRON_MANDATE_USES: i32 = 1;
/// Traps the Cry summons: two, three Radiant ones on the Radiant face.
const BASE_TRAPS: i32 = 2;
const RADIANT_TRAPS: i32 = 3;
/// Overwhelm's reach: Yates's lane and the lanes beside it.
const LANE_REACH: i32 = 1;
/// Wolf's Maw's target check: a Flying Unit.
const FLYING: &str = "flying";

/// R3567003: Overwhelm's Units — every Unit of both sides, controller first, in Yates's lane
/// or the lanes beside it, Yates included (no "other").
fn overwhelm_cards(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let state: &GameState = &*ctx.state;
    let Some(lane) = ctx
        .self_
        .as_ref()
        .and_then(|me| slot_of(state, me))
        .map(|slot| slot.lane)
    else {
        return Vec::new();
    };
    [ctx.controller, opponent_of(ctx.controller)]
        .into_iter()
        .flat_map(|player| active_units_of(state, player))
        .filter(|unit| slot_of(state, unit).is_some_and(|slot| (slot.lane - lane).abs() <= LANE_REACH))
        .map(|unit| unit.id.clone())
        .collect()
}

fn overwhelm_ability() -> ActivationDecl {
    ActivationDecl {
        id: OVERWHELM.to_string(),
        label: "Overwhelm".to_string(),
        uses: ActivationUses::Count(OVERWHELM_USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: Some(read_hook(|args| {
            args.self_.memory.get(CHOSEN_KEY).and_then(Value::as_str) == Some(OVERWHELM)
        })),
        run: hook(|_ctx| {
            vec![for_each_card(ForEachCardArgs {
                cards: Arc::new(overwhelm_cards),
                each: Arc::new(|instance_id: &str| {
                    freeze(FreezeArgs {
                        target: TargetSpec::Instance {
                            instance_id: instance_id.to_string(),
                        },
                        forever: None,
                    })
                }),
            })]
        }),
    }
}

fn wolfs_maw_ability() -> ActivationDecl {
    let mut target = TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"], "check": FLYING }));
    target.required = Some(true);
    ActivationDecl {
        id: WOLFS_MAW.to_string(),
        label: "Wolf's Maw".to_string(),
        uses: ActivationUses::Count(WOLFS_MAW_USES),
        cost: None,
        targets: vec![target],
        modes: vec![],
        can_activate: None,
        has: Some(read_hook(|args| {
            args.self_.memory.get(CHOSEN_KEY).and_then(Value::as_str) == Some(WOLFS_MAW)
        })),
        run: hook(|ctx| {
            vec![
                lose_keyword(LoseKeywordArgs {
                    target: TargetSpec::Chosen { index: None },
                    kind: KeywordKind::Flying,
                }),
                damage(json_as(json!({
                    "to": { "of": "chosen" },
                    "amount": param(&*ctx, "damage"),
                }))),
            ]
        }),
    }
}

fn iron_mandate_ability() -> ActivationDecl {
    let mut target = TargetDecl::target(
        1,
        1,
        json!({ "side": "ally", "of": ["unit"], "excludeSelf": true }),
    );
    target.required = Some(true);
    ActivationDecl {
        id: IRON_MANDATE.to_string(),
        label: "Iron Mandate".to_string(),
        uses: ActivationUses::Count(IRON_MANDATE_USES),
        cost: None,
        targets: vec![target],
        modes: vec![],
        can_activate: None,
        has: Some(read_hook(|args| {
            args.self_.memory.get(CHOSEN_KEY).and_then(Value::as_str) == Some(IRON_MANDATE)
        })),
        run: hook(|ctx| {
            let Some(me) = ctx.self_.as_ref().map(|me| me.id.clone()) else {
                return Vec::new();
            };
            let Some(Selection::Instance { instance_id: target }) = ctx.targets.first().cloned() else {
                return Vec::new();
            };
            vec![fuse_cards(FuseCardsArgs {
                instance_ids: Some(vec![me]),
                target_instance_id: Some(target),
                ..FuseCardsArgs::default()
            })]
        }),
    }
}

fn yates(radiant: bool) -> Script {
    let traps = if radiant { RADIANT_TRAPS } else { BASE_TRAPS };
    Script {
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![
                OVERWHELM.to_string(),
                WOLFS_MAW.to_string(),
                IRON_MANDATE.to_string(),
            ],
        }],
        cry: Some(hook(move |ctx| {
            let mut effects = vec![exile_named(ExileNamedArgs {
                letters: MOUSE.to_string(),
                spare_tags: vec![Tag::Cn],
            })];
            for _ in 0..traps {
                let mut summon = json!({ "query": { "type": TRAP_TYPES }, "player": "self" });
                if radiant {
                    summon["radiant"] = json!(true);
                }
                effects.push(summon_random(json_as(summon)));
            }
            if let Some(mode) = ctx.modes.first().cloned() {
                effects.push(remember(RememberArgs {
                    key: CHOSEN_KEY.to_string(),
                    value: json!(mode),
                }));
            }
            effects
        })),
        activations: vec![overwhelm_ability(), wolfs_maw_ability(), iron_mandate_ability()],
        target_checks: IndexMap::from([(
            FLYING,
            target_check(|args| {
                args.candidate
                    .is_some_and(|c| unit_has(args.state, c, KeywordKind::Flying))
            }),
        )]),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: yates(false),
        radiant: yates(true),
    }
}

// S+ #4 Profe Yates — SPEC §8.12 row 4 (R3567000–R3567004).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    use crate::{filler, give_flying, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A 6/4 Unit whose name holds no M, O, U, S or E.
    const TYLANNT: &str = "stinkyscape-006"; // Tylannt
    /// A Spell whose name holds no M, O, U, S or E.
    const RAPID: &str = "classic-026"; // Rapid Draw
    /// A Spell whose name holds M, O, U, S and E.
    const STOCKPILE: &str = "core-005"; // Stockpile
    /// A CN-tagged Spell: spared whatever its name holds.
    const PEPTIDES: &str = "meditative-036"; // CN Peptides

    const MODES: [&str; 3] = [OVERWHELM, WOLFS_MAW, IRON_MANDATE];

    fn abilities(s: &Scenario, id: &str) -> Vec<String> {
        activate_actions_for(s.state(), P1, s.card(id))
            .into_iter()
            .filter_map(|action| action.ability)
            .collect()
    }

    fn backrow_count(s: &Scenario, player: PlayerId) -> usize {
        s.state().players[player].backrow.iter().flatten().count()
    }

    fn exile_count(s: &Scenario, player: PlayerId) -> usize {
        s.pile(player, "exile").len()
    }

    /// Yates in hand, both sides holding named, unnamed and CN cards.
    fn unplayed(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "mana": 10,
                "hand": [{ "def": ID, "radiant": radiant }, RAPID, STOCKPILE],
                "field": [{ "def": TYLANNT, "lane": 2 }],
                "library": filler(4),
            },
            "p2": {
                "hand": [RAPID, STOCKPILE, PEPTIDES],
                "field": [{ "def": TYLANNT, "lane": 4 }],
                "library": filler(4),
            },
        }))
    }

    /// Yates played with `mode`.
    fn played(seed: &str, radiant: bool, mode: &str) -> Scenario {
        let mut s = unplayed(seed, radiant);
        s.play(ID, json!({ "zone": 1, "modes": [mode] }));
        s
    }

    #[test]
    fn r3567000_the_cry_exiles_named_cards_and_spares_cn_unnamed_and_itself() {
        let s = played("s004-exile", false, OVERWHELM);
        // The unnamed Rapid Draw stays in both hands, the CN-tagged Peptides in its...
        assert!(s.hand(P1).iter().any(|card| card.def_id == RAPID));
        assert!(s.hand(P2).iter().any(|card| card.def_id == RAPID));
        assert!(s.hand(P2).iter().any(|card| card.def_id == PEPTIDES));
        // ...while the named Stockpile is gone from both, hand and library alike.
        for player in [P1, P2] {
            assert!(s.hand(player).iter().all(|card| card.def_id != STOCKPILE));
            assert!(
                s.state().players[player]
                    .library
                    .iter()
                    .all(|card| card.def_id != STOCKPILE)
            );
            assert_eq!(
                s.pile(player, "exile")
                    .iter()
                    .filter(|card| card.def_id == STOCKPILE)
                    .count(),
                5,
                "{player} exiled its hand and library Stockpiles"
            );
        }
        // The unnamed Tylannt stands on both fields, and Yates itself stands.
        assert!(s.unit(P1, 2).is_some_and(|unit| unit.def_id == TYLANNT));
        assert!(s.unit(P1, 1).is_some_and(|unit| unit.def_id == ID));
        assert!(s.unit(P2, 4).is_some_and(|unit| unit.def_id == TYLANNT));
    }

    #[test]
    fn r3567001_base_sets_two_face_down_traps() {
        let s = played("s004-traps", false, OVERWHELM);
        let traps: Vec<CardInstance> = s.state().players[P1].backrow.iter().flatten().cloned().collect();
        assert_eq!(traps.len(), 2);
        for trap in &traps {
            assert!(trap.face_up != Some(true), "face down");
            assert!(!trap.radiant, "base");
            assert!(
                matches!(
                    crate::card_def(&trap.def_id).type_,
                    CardType::Trap | CardType::FieldTrap
                ),
                "a Trap"
            );
        }
    }

    #[test]
    fn r3567001_radiant_sets_three_radiant_traps() {
        let s = played("s004-traps-radiant", true, OVERWHELM);
        let traps: Vec<CardInstance> = s.state().players[P1].backrow.iter().flatten().cloned().collect();
        assert_eq!(traps.len(), 3);
        for trap in &traps {
            assert!(trap.face_up != Some(true), "face down");
            assert!(trap.radiant, "Radiant");
        }
    }

    #[test]
    fn r3567002_only_the_chosen_activate_is_listed() {
        for radiant in [false, true] {
            // The AI is offered a play for each mode...
            let s = unplayed("s004-modes", radiant);
            let yates = s
                .hand(P1)
                .into_iter()
                .find(|card| card.def_id == ID)
                .expect("Yates")
                .id;
            let offered: Vec<Vec<String>> = policy_actions(s.state(), P1, Default::default())
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play {
                        instance_id,
                        modes: Some(modes),
                        ..
                    } if instance_id == yates => Some(modes),
                    _ => None,
                })
                .collect();
            for mode in MODES {
                assert!(
                    offered.iter().any(|m| m.len() == 1 && m[0] == mode),
                    "{mode} is offered"
                );
            }
            // ...and each mode lists only its Activate once played.
            for mode in [OVERWHELM, IRON_MANDATE] {
                let s = played(&format!("s004-mode-{mode}"), radiant, mode);
                let yates = s.unit_at(P1, 1).id;
                assert_eq!(abilities(&s, &yates), vec![mode.to_string()]);
            }
            // Wolf's Maw needs a Flying Unit to be offered at all.
            let mut s = unplayed("s004-mode-wolf", radiant);
            let flyer = s.unit_at(P2, 4).id;
            give_flying(&mut s, &flyer);
            s.play(ID, json!({ "zone": 1, "modes": [WOLFS_MAW] }));
            let yates = s.unit_at(P1, 1).id;
            assert_eq!(abilities(&s, &yates), vec![WOLFS_MAW.to_string()]);
        }
    }

    #[test]
    fn r3567002_iron_mandate_fuses_yates_into_an_ally_without_its_cry() {
        let mut s = scenario(json!({
            "seed": "s004-iron",
            "p1": {
                "mana": 10,
                "hand": [ID],
                "field": [{ "def": TYLANNT, "lane": 2 }],
                "library": filler(4),
            },
            "p2": { "hand": [RAPID], "library": filler(4) },
        }));
        s.play(ID, json!({ "zone": 1, "modes": [IRON_MANDATE] }));
        let yates = s.unit_at(P1, 1).id;
        let host = s.unit_at(P1, 2).id;
        let exiled = exile_count(&s, P1) + exile_count(&s, P2);
        let backrow = backrow_count(&s, P1) + backrow_count(&s, P2);
        let pick = pick::instance(s.card(&host));
        s.activate(&yates, json!({ "ability": IRON_MANDATE, "targets": pick }));
        // Yates fused into the Tylannt, kept: 6 + 7.
        s.expect_stats(&host, json!({ "attack": 13 }));
        s.expect_in_zone(&yates, "gone");
        // No Cry ran again: no new exiles, no new traps.
        assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), exiled);
        assert_eq!(backrow_count(&s, P1) + backrow_count(&s, P2), backrow);
    }

    #[test]
    fn r3567003_overwhelm_freezes_its_lane_and_the_adjacent_ones_on_both_sides() {
        let mut s = scenario(json!({
            "seed": "s004-overwhelm",
            "p1": {
                "mana": 10,
                "hand": [ID],
                "field": [{ "def": TYLANNT, "lane": 1 }, { "def": TYLANNT, "lane": 4 }],
                "library": filler(4),
            },
            "p2": {
                "field": [{ "def": TYLANNT, "lane": 2 }, { "def": TYLANNT, "lane": 5 }],
                "library": filler(4),
            },
        }));
        s.play(ID, json!({ "zone": 2, "modes": [OVERWHELM] }));
        let yates = s.unit_at(P1, 2).id;
        s.activate(&yates, json!({ "ability": OVERWHELM }));
        assert!(s.card(&s.unit_at(P1, 1).id).frozen.is_some(), "lane 1 frozen");
        assert!(s.card(&s.unit_at(P1, 4).id).frozen.is_none(), "lane 4 open");
        assert!(s.card(&s.unit_at(P2, 2).id).frozen.is_some(), "lane 2 frozen");
        assert!(s.card(&s.unit_at(P2, 5).id).frozen.is_none(), "lane 5 open");
        assert!(s.card(&yates).frozen.is_some(), "Yates freezes too");
    }

    #[test]
    fn r3567003_overwhelm_from_the_middle_lane_leaves_the_outer_lanes_open() {
        let mut s = scenario(json!({
            "seed": "s004-overwhelm-middle",
            "p1": {
                "mana": 10,
                "hand": [ID],
                "field": [{ "def": TYLANNT, "lane": 1 }, { "def": TYLANNT, "lane": 5 }],
                "library": filler(4),
            },
            "p2": {
                "field": [
                    { "def": TYLANNT, "lane": 2 },
                    { "def": TYLANNT, "lane": 4 },
                    { "def": TYLANNT, "lane": 5 },
                ],
                "library": filler(4),
            },
        }));
        s.play(ID, json!({ "zone": 3, "modes": [OVERWHELM] }));
        let yates = s.unit_at(P1, 3).id;
        s.activate(&yates, json!({ "ability": OVERWHELM }));
        assert!(s.card(&s.unit_at(P1, 1).id).frozen.is_none(), "lane 1 open");
        assert!(s.card(&s.unit_at(P1, 5).id).frozen.is_none(), "lane 5 open");
        assert!(s.card(&s.unit_at(P2, 2).id).frozen.is_some(), "lane 2 frozen");
        assert!(s.card(&s.unit_at(P2, 4).id).frozen.is_some(), "lane 4 frozen");
        assert!(s.card(&s.unit_at(P2, 5).id).frozen.is_none(), "lane 5 open");
        assert!(s.card(&yates).frozen.is_some(), "Yates freezes too");
    }

    #[test]
    fn r3567004_wolfs_maw_grounds_a_flying_unit_and_deals_it_damage() {
        let mut s = scenario(json!({
            "seed": "s004-wolf",
            "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
            "p2": {
                "field": [{ "def": TYLANNT, "radiant": true }],
                "library": filler(4),
            },
        }));
        let flyer = s.unit_at(P2, 1).id;
        give_flying(&mut s, &flyer);
        s.play(ID, json!({ "zone": 1, "modes": [WOLFS_MAW] }));
        let yates = s.unit_at(P1, 1).id;
        let pick = pick::instance(s.card(&flyer));
        s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick }));
        assert!(
            !unit_has(s.state(), s.card(&flyer), KeywordKind::Flying),
            "grounded"
        );
        assert_eq!(s.card(&flyer).damage, 4);
    }

    #[test]
    fn r3567004_with_no_flying_unit_wolfs_maw_is_not_listed() {
        let s = played("s004-wolf-none", false, WOLFS_MAW);
        let yates = s.unit_at(P1, 1).id;
        assert!(abilities(&s, &yates).is_empty(), "no Flying Unit, no Activate");
    }

    #[test]
    fn wolfs_maw_is_used_at_most_three_times_a_turn() {
        let mut s = scenario(json!({
            "seed": "s004-wolf-uses",
            "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
            "p2": {
                "field": [
                    { "def": TYLANNT, "radiant": true },
                    { "def": TYLANNT, "radiant": true },
                    { "def": TYLANNT, "radiant": true },
                ],
                "library": filler(4),
            },
        }));
        for lane in 1..=3 {
            let id = s.unit_at(P2, lane).id;
            give_flying(&mut s, &id);
        }
        // Radiant hosts survive the 4 damage.
        s.play(ID, json!({ "zone": 1, "modes": [WOLFS_MAW] }));
        let yates = s.unit_at(P1, 1).id;
        for lane in 1..=3 {
            let target = s.unit_at(P2, lane).id;
            let pick = pick::instance(s.card(&target));
            s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick }));
        }
        assert!(abilities(&s, &yates).is_empty(), "all three uses spent");
        let target = s.unit_at(P2, 1).id;
        let pick = pick::instance(s.card(&target));
        s.expect_refused(|s| s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick })));
    }
}
