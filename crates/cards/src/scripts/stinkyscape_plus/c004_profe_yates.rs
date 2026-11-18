//! SPEC §8.12 S+ #4 Profe Yates, a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    7/7; "Cry: Exile a permanent with M, O, U, S and E in its name and text (CN cards are
//!            immune). Then summon two random Traps."
//!   Radiant: 14/14; "Cry: Exile a permanent with M, O, U, S and E in its name and text (CN cards are
//!            immune). Then summon three random Radiant Traps."
//!
//! Engine: the Cry exiles one declared permanent of either side, a Unit or a backrow card (R81, TG
//! #26 Path of Exile's pick, R3996000), that is face-up, carries no CN tag, and whose name and the
//! text of the face it wears hold each of M, O, U, S and E in either case (R4154100): a face-down card
//! is never offered, its name and text being hidden (R33). With none such standing the play names
//! none and the Cry summons its traps all the same (R90). Then the traps: independent
//! `summon_random` draws of non-token Traps/Field Traps of the shipped sets, each face down in the
//! leftmost open backrow zone (R3567001). It has no Activate since issue #4154.
//! Kept for replays (R767000): through v0.6.6h the Cry exiled any declared permanent and Yates had
//! three Activates of its own, each offered while its target stood and spent on its own uses
//! (R3996000, R1751000): Overwhelm froze every Unit of both sides in its lane and the lanes beside
//! it, Yates included (R3567003); Wolf's Maw grounded a declared Flying Unit of either side, which
//! lost Flying and took the declared damage (R3567004); Iron Mandate fused Yates into a declared
//! friendly Unit, kept, with no Cry run (R3567002) (`script_v0_6_6h`). Through v0.6.5b the Cry exiled
//! every card named with M, O, U, S or E (`exile_named`, R3567000) and remembered the one Activate
//! chosen at play, the only one Yates then had (R3567002) (`script_v0_6_5b`).
//!
//! *{Spawn voiceline: Every pack needs an alpha I don't see anyone else stepping up}*
//! *{Attack voiceline: a mouse once bit my sister}*
//! *{Death voiceline: I will not see the Northern Lights again}*
//! Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::preview::is_face_down;
use jackioh_engine::prelude::*;

use crate::query::TRAP_TYPES;

pub const ID: &str = "stinkyscapeplus-004";

/// The exile's letters: since issue #4154 a permanent whose name and text hold every one of them
/// (R4154100), through v0.6.5b a card whose name holds any (R3567000), in either case.
const MOUSE: &str = "MOUSE";
/// R4154100: the Cry's target check, a permanent `holds_mouse` admits.
const MOUSE_CHECK: &str = "mouse";
/// The three Activates, in the order the card prints them; through v0.6.5b the Cry's modes too (R81).
const OVERWHELM: &str = "overwhelm";
const WOLFS_MAW: &str = "wolfsMaw";
const IRON_MANDATE: &str = "ironMandate";
/// Through v0.6.5b, what the Cry remembered: the chosen Activate, kept while Yates stays (R78).
const CHOSEN_KEY: &str = "yatesActivate";
/// Through v0.6.6h, §6.2 Activate N, the script's literal, as S+ #11's (`ActivationUses` has no param).
const OVERWHELM_USES: i32 = 1;
const WOLFS_MAW_USES: i32 = 3;
const IRON_MANDATE_USES: i32 = 1;
/// Traps the Cry summons: two, three Radiant ones on the Radiant face.
const BASE_TRAPS: i32 = 2;
const RADIANT_TRAPS: i32 = 3;
/// Through v0.6.6h, Overwhelm's reach: Yates's lane and the lanes beside it.
const LANE_REACH: i32 = 1;
/// Through v0.6.6h, Wolf's Maw's target check: a Flying Unit.
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

/// R3567002: through v0.6.5b, an Activate Yates had only when the Cry remembered `mode`.
fn chosen(mode: &'static str, chosen_only: bool) -> Option<HasHook> {
    chosen_only.then(|| read_hook(move |args| args.self_.memory.get(CHOSEN_KEY).and_then(Value::as_str) == Some(mode)))
}

fn overwhelm_ability(chosen_only: bool) -> ActivationDecl {
    ActivationDecl {
        id: OVERWHELM.to_string(),
        label: "Overwhelm".to_string(),
        uses: ActivationUses::Count(OVERWHELM_USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: chosen(OVERWHELM, chosen_only),
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

fn wolfs_maw_ability(chosen_only: bool) -> ActivationDecl {
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
        has: chosen(WOLFS_MAW, chosen_only),
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

fn iron_mandate_ability(chosen_only: bool) -> ActivationDecl {
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
        has: chosen(IRON_MANDATE, chosen_only),
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

/// R3567001: the Cry's traps, two random Traps, three random Radiant ones on the Radiant face.
fn traps(radiant: bool) -> Vec<Effect> {
    let count = if radiant { RADIANT_TRAPS } else { BASE_TRAPS };
    (0..count)
        .map(|_| {
            let mut summon = json!({ "query": { "type": TRAP_TYPES }, "player": "self" });
            if radiant {
                summon["radiant"] = json!(true);
            }
            summon_random(json_as(summon))
        })
        .collect()
}

fn flying_check() -> IndexMap<&'static str, TargetCheck> {
    IndexMap::from([(
        FLYING,
        target_check(|args| {
            args.candidate
                .is_some_and(|c| unit_has(args.state, c, KeywordKind::Flying))
        }),
    )])
}

/// R4154100: whether `card` is a permanent the Cry may exile: face-up, so that its name and text are
/// public (R33), not CN, and its name and the text of the face it wears, its declared numbers filled
/// in, hold every letter of `MOUSE` in either case.
fn holds_mouse(state: &GameState, card: &CardInstance) -> bool {
    if is_face_down(state, card) || tags_of(state, card).contains(&Tag::Cn) {
        return false;
    }
    let def = def_of(Some(state), &card.def_id);
    let face = if card.radiant { FaceKind::Radiant } else { FaceKind::Base };
    let words = format!("{} {}", def.name, fill_params(def, face, None)).to_lowercase();
    MOUSE.chars().all(|letter| words.contains(letter.to_ascii_lowercase()))
}

/// R4154100: "Exile a permanent with M, O, U, S and E in its name and text (CN cards are immune). Then
/// summon two random Traps.", and no Activate.
fn yates(radiant: bool) -> Script {
    Script {
        // A permanent of either side, as TG #26 Path of Exile's "Exile a permanent" (R81), that
        // `holds_mouse` admits.
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"], "check": MOUSE_CHECK }),
        )],
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![exile(json_as(json!({ "target": { "of": "chosen" } })))];
            effects.extend(traps(radiant));
            effects
        })),
        target_checks: IndexMap::from([(
            MOUSE_CHECK,
            target_check(|args| args.candidate.is_some_and(|card| holds_mouse(args.state, card))),
        )]),
        ..Script::default()
    }
}

/// R3996000: through v0.6.6h, "Exile a permanent. Then summon two random Traps.", and the three
/// Activates, each its own.
fn yates_v0_6_6h(radiant: bool) -> Script {
    Script {
        // A permanent of either side, as TG #26 Path of Exile's "Exile a permanent" (R81).
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit", "backrow"] }))],
        cry: Some(hook(move |_ctx| {
            let mut effects = vec![exile(json_as(json!({ "target": { "of": "chosen" } })))];
            effects.extend(traps(radiant));
            effects
        })),
        activations: vec![
            overwhelm_ability(false),
            wolfs_maw_ability(false),
            iron_mandate_ability(false),
        ],
        target_checks: flying_check(),
        ..Script::default()
    }
}

/// R3567000, R3567002: through v0.6.5b, the named exile, the traps, and the one Activate chosen at play.
fn yates_v0_6_5b(radiant: bool) -> Script {
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
            effects.extend(traps(radiant));
            if let Some(mode) = ctx.modes.first().cloned() {
                effects.push(remember(RememberArgs {
                    key: CHOSEN_KEY.to_string(),
                    value: json!(mode),
                }));
            }
            effects
        })),
        activations: vec![
            overwhelm_ability(true),
            wolfs_maw_ability(true),
            iron_mandate_ability(true),
        ],
        target_checks: flying_check(),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: yates(false),
        radiant: yates(true),
    }
}

/// R767000: after v0.6.5b through v0.6.6h, the exile of any declared permanent and the three
/// Activates; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_6h() -> CardScripts {
    CardScripts {
        base: yates_v0_6_6h(false),
        radiant: yates_v0_6_6h(true),
    }
}

/// R767000: through v0.6.5b, the named exile and one chosen Activate; `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: yates_v0_6_5b(false),
        radiant: yates_v0_6_5b(true),
    }
}

// S+ #4 Profe Yates — SPEC §8.12 row 4 (R4154100; R3996000, R3567000–R3567004, R767000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::subsystems::activate::activate_actions_for;
    use jackioh_engine::subsystems::ai_policy::policy_actions;
    use jackioh_engine::testkit::*;

    use crate::{filler, give_flying, merged, scenario};

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
    /// A Field Spell: a backrow permanent for the Cry to exile.
    const MANA_WELL: &str = "core-006"; // Mana Well
    /// A Trap, set face down; its name and text hold M, O, U, S and E.
    const SHEEPISH: &str = "core-041"; // Sheepish
    /// A Unit whose name holds M, O, U, S and E.
    const MUSHROOM: &str = "classicplus-020"; // Mushroom Power
    /// A Unit whose name holds no O, U, S or E, and which prints no text.
    const VANILLA: &str = "core-008"; // Mr. Vanilla
    /// A Unit whose M, O, U, S and E are split between its name and its text.
    const SPECIESIST: &str = "core-002"; // Speciesist
    /// A CN-tagged Unit whose name and text hold M, O, U, S and E.
    const LAR: &str = "classicplus-076"; // Brother Lar
    /// A Unit whose base face prints the U its Radiant face lacks.
    const TIMMY: &str = "core-011"; // Tempo Timmy
    /// A Trap whose name and text hold M, O, U, S and E, which no Unit's play reveals.
    const SHADOWSTEP: &str = "classic-014"; // Shadowstep

    const MODES: [&str; 3] = [OVERWHELM, WOLFS_MAW, IRON_MANDATE];

    /// The Activates Yates is offered, each once, in the order it prints them.
    fn abilities(s: &Scenario, id: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for ability in activate_actions_for(s.state(), P1, s.card(id))
            .into_iter()
            .filter_map(|action| action.ability)
        {
            if !out.contains(&ability) {
                out.push(ability);
            }
        }
        out
    }

    fn backrow_count(s: &Scenario, player: PlayerId) -> usize {
        s.state().players[player].backrow.iter().flatten().count()
    }

    fn exile_count(s: &Scenario, player: PlayerId) -> usize {
        s.pile(player, "exile").len()
    }

    /// Yates in hand, both sides holding named, unnamed and CN cards, a Tylannt on each field and
    /// a Mana Well in p2's backrow for the Cry to exile.
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
                "backrow": [{ "def": MANA_WELL, "lane": 1 }],
                "library": filler(4),
            },
        }))
    }

    /// Yates played into `zone`, exiling p2's backrow card in lane 1.
    fn play_exiling_their_backrow(s: &mut Scenario, zone: i32) {
        let well = s.backrow(P2, 1).expect("p2's backrow card").id;
        s.play(ID, json!({ "zone": zone, "targets": pick::id(&well) }));
    }

    /// Yates played into zone 1, exiling p2's Mana Well.
    fn played(seed: &str, radiant: bool) -> Scenario {
        let mut s = unplayed(seed, radiant);
        play_exiling_their_backrow(&mut s, 1);
        s
    }

    mod cry {
        use super::*;

        /// p1's Yates (`radiant` its face) in hand beside p1's `mine` Units, facing p2's side `theirs`.
        fn facing(seed: &str, radiant: bool, mine: Value, theirs: Value) -> Scenario {
            scenario(json!({
                "seed": seed,
                "p1": {
                    "mana": 10,
                    "hand": [{ "def": ID, "radiant": radiant }],
                    "field": mine,
                    "library": filler(4),
                },
                "p2": merged(json!({ "library": filler(4) }), theirs),
            }))
        }

        /// The cards Yates's plays now offer to exile, each once.
        fn pickable(s: &Scenario) -> Vec<String> {
            let mut out: Vec<String> = Vec::new();
            for id in pick::instance_ids(&s.play_targets(ID)) {
                if !out.contains(&id) {
                    out.push(id);
                }
            }
            out
        }

        #[test]
        fn r4154100_it_exiles_a_declared_permanent_whose_name_and_text_hold_m_o_u_s_and_e() {
            for radiant in [false, true] {
                // Speciesist's letters are split between its name and its text; Mushroom Power's name
                // holds all five.
                let mut s = facing(
                    "s004-mouse",
                    radiant,
                    json!([{ "def": MUSHROOM, "lane": 2 }]),
                    json!({ "field": [{ "def": SPECIESIST, "lane": 4 }], "hand": [STOCKPILE] }),
                );
                let theirs = s.unit_at(P2, 4).id;
                let mine = s.unit_at(P1, 2).id;
                assert_eq!(pickable(&s), vec![mine.clone(), theirs.clone()], "radiant {radiant}");
                s.play(ID, json!({ "zone": 1, "targets": pick::id(&theirs) }));
                s.expect_in_zone(theirs.as_str(), "exile");
                // One permanent, and nothing off the field: Stockpile stays in p2's hand.
                assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), 1, "radiant {radiant}");
                assert!(s.hand(P2).iter().any(|card| card.def_id == STOCKPILE));
                assert!(s.unit(P1, 2).is_some_and(|unit| unit.def_id == MUSHROOM));
            }
        }

        #[test]
        fn r4154100_its_pick_may_be_a_permanent_of_its_own_side() {
            let mut s = facing(
                "s004-mouse-own",
                false,
                json!([{ "def": MUSHROOM, "lane": 2 }]),
                json!({ "field": [{ "def": VANILLA, "lane": 4 }] }),
            );
            let mine = s.unit_at(P1, 2).id;
            s.play(ID, json!({ "zone": 1, "targets": pick::id(&mine) }));
            s.expect_in_zone(mine.as_str(), "exile");
            assert!(s.unit(P2, 4).is_some(), "the enemy Unit stands");
        }

        #[test]
        fn r4154100_a_permanent_missing_a_letter_is_neither_offered_nor_accepted() {
            // Mr. Vanilla's name holds no O, U, S or E, and it prints no text.
            let mut s = facing(
                "s004-mouse-missing",
                false,
                json!([]),
                json!({ "field": [{ "def": VANILLA, "lane": 4 }, { "def": SPECIESIST, "lane": 5 }] }),
            );
            let vanilla = s.unit_at(P2, 4).id;
            let speciesist = s.unit_at(P2, 5).id;
            assert_eq!(pickable(&s), vec![speciesist]);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&vanilla) })));
            s.expect_in_zone(vanilla.as_str(), "field");
        }

        #[test]
        fn r4154100_the_text_read_is_the_face_the_card_wears() {
            // Tempo Timmy's base face prints "Rush" (its U); its Radiant face prints "Charge" instead.
            let mut s = facing(
                "s004-mouse-face",
                false,
                json!([]),
                json!({
                    "field": [
                        { "def": TIMMY, "lane": 4 },
                        { "def": TIMMY, "radiant": true, "lane": 5 },
                    ],
                }),
            );
            let base = s.unit_at(P2, 4).id;
            let radiant = s.unit_at(P2, 5).id;
            assert_eq!(pickable(&s), vec![base]);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&radiant) })));
        }

        #[test]
        fn r4154100_a_cn_card_is_immune_whatever_its_letters() {
            // Brother Lar's name and text hold all five, and it is CN.
            let mut s = facing(
                "s004-mouse-cn",
                false,
                json!([{ "def": LAR, "lane": 2 }]),
                json!({ "field": [{ "def": LAR, "lane": 4 }, { "def": SPECIESIST, "lane": 5 }] }),
            );
            let lar = s.unit_at(P2, 4).id;
            let speciesist = s.unit_at(P2, 5).id;
            assert_eq!(pickable(&s), vec![speciesist]);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&lar) })));
            s.expect_in_zone(lar.as_str(), "field");
        }

        #[test]
        fn r4154100_a_face_down_card_is_never_offered_a_face_up_one_is() {
            // Shadowstep's name and text hold all five: face-down, on either side, it is hidden and never
            // offered (to its own controller neither); face-up it may be exiled.
            let mut s = scenario(json!({
                "seed": "s004-mouse-face-down",
                "p1": {
                    "mana": 10,
                    "hand": [ID],
                    "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "lane": 2 }],
                    "library": filler(4),
                },
                "p2": {
                    "backrow": [
                        { "def": SHADOWSTEP, "faceUp": false, "lane": 3 },
                        { "def": SHADOWSTEP, "faceUp": true, "lane": 4 },
                    ],
                    "library": filler(4),
                },
            }));
            let hidden = s.backrow(P2, 3).expect("the face-down Trap").id;
            let shown = s.backrow(P2, 4).expect("the face-up Trap").id;
            assert_eq!(pickable(&s), vec![shown.clone()]);
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "targets": pick::id(&hidden) })));
            s.play(ID, json!({ "zone": 1, "targets": pick::id(&shown) }));
            s.expect_in_zone(shown.as_str(), "exile");
            s.expect_in_zone(hidden.as_str(), "field");
        }

        #[test]
        fn r4154100_with_none_eligible_the_play_names_none_and_the_traps_still_come() {
            for radiant in [false, true] {
                let mut s = scenario(json!({
                    "seed": "s004-mouse-none",
                    "p1": {
                        "mana": 10,
                        "hand": [{ "def": ID, "radiant": radiant }],
                        "field": [{ "def": LAR, "lane": 2 }],
                        "library": filler(4),
                    },
                    "p2": {
                        "field": [{ "def": VANILLA, "lane": 4 }],
                        "backrow": [{ "def": SHADOWSTEP, "faceUp": false, "lane": 3 }],
                        "library": filler(4),
                    },
                }));
                assert!(s.can_play(ID), "radiant {radiant}");
                assert!(pickable(&s).is_empty(), "radiant {radiant}");
                s.play(ID, json!({ "zone": 1 }));
                assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), 0);
                let traps = if radiant { RADIANT_TRAPS } else { BASE_TRAPS };
                assert_eq!(backrow_count(&s, P1), traps as usize, "radiant {radiant}");
            }
        }

        #[test]
        fn r4154100_while_one_is_eligible_every_play_names_one() {
            let s = facing(
                "s004-mouse-must",
                false,
                json!([]),
                json!({ "field": [{ "def": VANILLA, "lane": 4 }, { "def": SPECIESIST, "lane": 5 }] }),
            );
            let plays = nonempty(s.plays_of(ID), "Yates's plays");
            for play in &plays {
                let ActionBody::Play { targets, .. } = play else {
                    unreachable!()
                };
                assert_eq!(targets.as_ref().map(Vec::len), Some(1), "{play:?}");
            }
        }

        #[test]
        fn r4154100_it_has_no_activate_on_either_face() {
            let scripts = script();
            assert!(scripts.base.activations.is_empty());
            assert!(scripts.radiant.activations.is_empty());
            for radiant in [false, true] {
                let mut s = unplayed("s004-no-activate", radiant);
                let flyer = s.unit_at(P2, 4).id;
                give_flying(&mut s, &flyer);
                play_exiling_their_backrow(&mut s, 1);
                let yates = s.unit_at(P1, 1).id;
                assert!(abilities(&s, &yates).is_empty(), "radiant {radiant}");
            }
        }

        #[test]
        fn r3996000_the_exile_comes_first_so_a_trap_takes_the_zone_it_freed() {
            // p1's backrow is full; Yates exiles one of p1's own Mana Wells and one Trap fits.
            let mut s = scenario(json!({
                "seed": "s004-then",
                "p1": {
                    "mana": 10,
                    "hand": [ID],
                    "backrow": [MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL, MANA_WELL],
                    "library": filler(4),
                },
                "p2": { "library": filler(4) },
            }));
            let well = s.backrow(P1, 3).expect("p1's Mana Well").id;
            s.play(ID, json!({ "zone": 1, "targets": pick::id(&well) }));
            s.expect_in_zone(well.as_str(), "exile");
            let trap = s.backrow(P1, 3).expect("a Trap in the freed zone");
            assert!(matches!(
                crate::card_def(&trap.def_id).type_,
                CardType::Trap | CardType::FieldTrap
            ));
            assert_eq!(backrow_count(&s, P1), 5);
        }

        #[test]
        fn r3567001_base_sets_two_face_down_traps() {
            let s = played("s004-traps", false);
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
            let s = played("s004-traps-radiant", true);
            let traps: Vec<CardInstance> = s.state().players[P1].backrow.iter().flatten().cloned().collect();
            assert_eq!(traps.len(), 3);
            for trap in &traps {
                assert!(trap.face_up != Some(true), "face down");
                assert!(trap.radiant, "Radiant");
            }
        }
    }

    /// R767000: the script kept for the replays after v0.6.5b through v0.6.6h: the exile of any declared
    /// permanent and the three Activates.
    mod v0_6_6h {
        use super::*;

        /// `run` under the script kept for v0.6.6h, with the `damage` its Wolf's Maw read then (4, both
        /// faces).
        fn kept<T>(run: impl FnOnce() -> T) -> T {
            crate::under_kept_as(
                ID,
                script_v0_6_6h(),
                |def| {
                    def.params = serde_json::from_value(json!([
                        { "key": "damage", "base": 4, "radiant": 4, "better": "up", "step": 1, "min": 1 }
                    ]))
                    .expect("the v0.6.6h params");
                },
                run,
            )
        }

        #[test]
        fn r3996000_the_cry_exiles_the_declared_enemy_unit_and_nothing_else() {
            kept(|| {
                let mut s = unplayed("s004-exile-unit", false);
                let theirs = s.unit_at(P2, 4).id;
                s.play(ID, json!({ "zone": 1, "targets": pick::id(&theirs) }));
                s.expect_in_zone(theirs.as_str(), "exile");
                assert_eq!(exile_count(&s, P2), 1);
                assert_eq!(exile_count(&s, P1), 0);
                // No card is exiled for its name any more: Stockpile stays in both hands.
                for player in [P1, P2] {
                    assert!(s.hand(player).iter().any(|card| card.def_id == STOCKPILE));
                }
                assert!(s.backrow(P2, 1).is_some_and(|card| card.def_id == MANA_WELL));
                assert!(s.unit(P1, 2).is_some_and(|unit| unit.def_id == TYLANNT));
            });
        }

        #[test]
        fn r3996000_its_pick_may_be_a_permanent_of_its_own_side() {
            kept(|| {
                let mut s = unplayed("s004-exile-own", false);
                let mine = s.unit_at(P1, 2).id;
                s.play(ID, json!({ "zone": 1, "targets": pick::id(&mine) }));
                s.expect_in_zone(mine.as_str(), "exile");
                assert!(s.unit(P2, 4).is_some(), "the enemy Unit stands");
            });
        }

        #[test]
        fn r3996000_its_pick_may_be_a_face_down_trap_and_a_cn_card_is_no_longer_spared() {
            kept(|| {
                let mut s = scenario(json!({
                    "seed": "s004-exile-trap",
                    "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
                    "p2": {
                        "backrow": [{ "def": SHEEPISH, "faceUp": false, "lane": 3 }],
                        "field": [{ "def": "meditative-047", "lane": 1 }],
                        "library": filler(4),
                    },
                }));
                let trap = s.backrow(P2, 3).expect("the face-down Trap").id;
                s.play(ID, json!({ "zone": 1, "targets": pick::id(&trap) }));
                s.expect_in_zone(trap.as_str(), "exile");
                // 饕餮 carries the CN tag: a pick of its own, never spared.
                let mut s = scenario(json!({
                    "seed": "s004-exile-cn",
                    "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
                    "p2": { "field": [{ "def": "meditative-047", "lane": 1 }], "library": filler(4) },
                }));
                let cn = s.unit_at(P2, 1).id;
                s.play(ID, json!({ "zone": 1, "targets": pick::id(&cn) }));
                s.expect_in_zone(cn.as_str(), "exile");
            });
        }

        #[test]
        fn r3996000_a_permanent_standing_must_be_picked_and_with_none_the_cry_still_summons_its_traps() {
            kept(|| {
                // A board with a permanent: each Yates play the AI is offered names one, and no mode.
                let s = unplayed("s004-offers", false);
                let yates = s
                    .hand(P1)
                    .into_iter()
                    .find(|card| card.def_id == ID)
                    .expect("Yates")
                    .id;
                let offered: Vec<ActionBody> = policy_actions(s.state(), P1, Default::default())
                    .into_iter()
                    .filter(|action| is_play_of(action, &yates))
                    .collect();
                assert!(!offered.is_empty());
                for action in &offered {
                    let ActionBody::Play { targets, modes, .. } = action else {
                        unreachable!()
                    };
                    assert_eq!(targets.as_ref().map(Vec::len), Some(1), "{action:?}");
                    assert!(modes.as_ref().is_none_or(Vec::is_empty), "{action:?}");
                }
                // An empty board: no pick, and the two Traps all the same.
                let mut s = scenario(json!({
                    "seed": "s004-no-permanent",
                    "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
                    "p2": { "library": filler(4) },
                }));
                s.play(ID, json!({ "zone": 1 }));
                assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), 0);
                assert_eq!(backrow_count(&s, P1), BASE_TRAPS as usize);
            });
        }

        #[test]
        fn r3996000_radiant_exiles_one_declared_permanent_too() {
            kept(|| {
                let mut s = unplayed("s004-exile-radiant", true);
                let theirs = s.unit_at(P2, 4).id;
                s.play(ID, json!({ "zone": 1, "targets": pick::id(&theirs) }));
                s.expect_in_zone(theirs.as_str(), "exile");
                assert_eq!(exile_count(&s, P1) + exile_count(&s, P2), 1);
            });
        }

        #[test]
        fn r3996000_all_three_activates_are_offered_on_both_faces() {
            kept(|| {
                for radiant in [false, true] {
                    let mut s = unplayed("s004-all-three", radiant);
                    let flyer = s.unit_at(P2, 4).id;
                    give_flying(&mut s, &flyer);
                    play_exiling_their_backrow(&mut s, 1);
                    let yates = s.unit_at(P1, 1).id;
                    assert_eq!(
                        abilities(&s, &yates),
                        MODES.map(String::from).to_vec(),
                        "radiant {radiant}"
                    );
                }
            });
        }

        #[test]
        fn r3996000_each_activate_needs_its_own_target() {
            kept(|| {
                // No Flying Unit: no Wolf's Maw; Iron Mandate fuses into the Tylannt.
                let s = played("s004-no-flyer", false);
                let yates = s.unit_at(P1, 1).id;
                assert_eq!(
                    abilities(&s, &yates),
                    vec![OVERWHELM.to_string(), IRON_MANDATE.to_string()]
                );
                // No other friendly Unit either: Overwhelm alone.
                let mut s = scenario(json!({
                    "seed": "s004-alone",
                    "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
                    "p2": { "field": [{ "def": TYLANNT, "lane": 3 }], "backrow": [MANA_WELL], "library": filler(4) },
                }));
                play_exiling_their_backrow(&mut s, 1);
                let yates = s.unit_at(P1, 1).id;
                assert_eq!(abilities(&s, &yates), vec![OVERWHELM.to_string()]);
            });
        }

        #[test]
        fn r3996000_each_activate_spends_its_own_uses_and_a_frozen_yates_uses_none() {
            kept(|| {
                let mut s = scenario(json!({
                    "seed": "s004-each-own",
                    "p1": {
                        "mana": 10,
                        "hand": [ID],
                        "field": [{ "def": TYLANNT, "lane": 5 }],
                        "library": filler(4),
                    },
                    "p2": {
                        "field": [
                            { "def": TYLANNT, "radiant": true, "lane": 1 },
                            { "def": TYLANNT, "radiant": true, "lane": 2 },
                            { "def": TYLANNT, "radiant": true, "lane": 3 },
                        ],
                        "backrow": [MANA_WELL],
                        "library": filler(4),
                    },
                }));
                for lane in 1..=3 {
                    let id = s.unit_at(P2, lane).id;
                    give_flying(&mut s, &id);
                }
                play_exiling_their_backrow(&mut s, 1);
                let yates = s.unit_at(P1, 1).id;
                // Wolf's Maw's three uses leave Overwhelm and Iron Mandate theirs.
                for lane in 1..=3 {
                    let target = s.unit_at(P2, lane).id;
                    s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick::id(&target) }));
                }
                assert_eq!(
                    abilities(&s, &yates),
                    vec![OVERWHELM.to_string(), IRON_MANDATE.to_string()]
                );
                // Overwhelm freezes Yates too (R3567003): it uses no Activate until it thaws (R2246000).
                s.activate(&yates, json!({ "ability": OVERWHELM }));
                assert!(s.card(&yates).frozen.is_some());
                assert!(abilities(&s, &yates).is_empty());
                let host = s.unit_at(P1, 5).id;
                s.expect_refused_with(
                    |s| s.activate(&yates, json!({ "ability": IRON_MANDATE, "targets": pick::id(&host) })),
                    "Frozen",
                );
            });
        }

        #[test]
        fn r3567002_iron_mandate_fuses_yates_into_an_ally_without_its_cry() {
            kept(|| {
                let mut s = scenario(json!({
                    "seed": "s004-iron",
                    "p1": {
                        "mana": 10,
                        "hand": [ID],
                        "field": [{ "def": TYLANNT, "lane": 2 }],
                        "library": filler(4),
                    },
                    "p2": { "hand": [RAPID], "backrow": [MANA_WELL], "library": filler(4) },
                }));
                play_exiling_their_backrow(&mut s, 1);
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
            });
        }

        #[test]
        fn r3567003_overwhelm_freezes_its_lane_and_the_adjacent_ones_on_both_sides() {
            kept(|| {
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
                        "backrow": [MANA_WELL],
                        "library": filler(4),
                    },
                }));
                play_exiling_their_backrow(&mut s, 2);
                let yates = s.unit_at(P1, 2).id;
                s.activate(&yates, json!({ "ability": OVERWHELM }));
                assert!(s.card(&s.unit_at(P1, 1).id).frozen.is_some(), "lane 1 frozen");
                assert!(s.card(&s.unit_at(P1, 4).id).frozen.is_none(), "lane 4 open");
                assert!(s.card(&s.unit_at(P2, 2).id).frozen.is_some(), "lane 2 frozen");
                assert!(s.card(&s.unit_at(P2, 5).id).frozen.is_none(), "lane 5 open");
                assert!(s.card(&yates).frozen.is_some(), "Yates freezes too");
            });
        }

        #[test]
        fn r3567003_overwhelm_from_the_middle_lane_leaves_the_outer_lanes_open() {
            kept(|| {
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
                        "backrow": [MANA_WELL],
                        "library": filler(4),
                    },
                }));
                play_exiling_their_backrow(&mut s, 3);
                let yates = s.unit_at(P1, 3).id;
                s.activate(&yates, json!({ "ability": OVERWHELM }));
                assert!(s.card(&s.unit_at(P1, 1).id).frozen.is_none(), "lane 1 open");
                assert!(s.card(&s.unit_at(P1, 5).id).frozen.is_none(), "lane 5 open");
                assert!(s.card(&s.unit_at(P2, 2).id).frozen.is_some(), "lane 2 frozen");
                assert!(s.card(&s.unit_at(P2, 4).id).frozen.is_some(), "lane 4 frozen");
                assert!(s.card(&s.unit_at(P2, 5).id).frozen.is_none(), "lane 5 open");
                assert!(s.card(&yates).frozen.is_some(), "Yates freezes too");
            });
        }

        #[test]
        fn r3567004_wolfs_maw_grounds_a_flying_unit_and_deals_it_damage() {
            kept(|| {
                let mut s = scenario(json!({
                    "seed": "s004-wolf",
                    "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
                    "p2": {
                        "field": [{ "def": TYLANNT, "radiant": true }],
                        "backrow": [MANA_WELL],
                        "library": filler(4),
                    },
                }));
                let flyer = s.unit_at(P2, 1).id;
                give_flying(&mut s, &flyer);
                play_exiling_their_backrow(&mut s, 1);
                let yates = s.unit_at(P1, 1).id;
                let pick = pick::instance(s.card(&flyer));
                s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick }));
                assert!(
                    !unit_has(s.state(), s.card(&flyer), KeywordKind::Flying),
                    "grounded"
                );
                assert_eq!(s.card(&flyer).damage, 4);
            });
        }

        #[test]
        fn r3567004_with_no_flying_unit_wolfs_maw_is_not_listed() {
            kept(|| {
                let s = played("s004-wolf-none", false);
                let yates = s.unit_at(P1, 1).id;
                assert!(
                    !abilities(&s, &yates).contains(&WOLFS_MAW.to_string()),
                    "no Flying Unit, no Wolf's Maw"
                );
            });
        }

        #[test]
        fn wolfs_maw_is_used_at_most_three_times_a_turn() {
            kept(|| {
                let mut s = scenario(json!({
                    "seed": "s004-wolf-uses",
                    "p1": { "mana": 10, "hand": [ID], "library": filler(4) },
                    "p2": {
                        "field": [
                            { "def": TYLANNT, "radiant": true },
                            { "def": TYLANNT, "radiant": true },
                            { "def": TYLANNT, "radiant": true },
                        ],
                        "backrow": [MANA_WELL],
                        "library": filler(4),
                    },
                }));
                for lane in 1..=3 {
                    let id = s.unit_at(P2, lane).id;
                    give_flying(&mut s, &id);
                }
                // Radiant hosts survive the 4 damage.
                play_exiling_their_backrow(&mut s, 1);
                let yates = s.unit_at(P1, 1).id;
                for lane in 1..=3 {
                    let target = s.unit_at(P2, lane).id;
                    let pick = pick::instance(s.card(&target));
                    s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick }));
                }
                assert!(
                    !abilities(&s, &yates).contains(&WOLFS_MAW.to_string()),
                    "all three uses spent"
                );
                let target = s.unit_at(P2, 1).id;
                let pick = pick::instance(s.card(&target));
                s.expect_refused(|s| s.activate(&yates, json!({ "ability": WOLFS_MAW, "targets": pick })));
            });
        }
    }


    /// R767000: the script kept for the replays of v0.6.5b and before: the named exile and the one
    /// Activate chosen at play.
    mod v0_6_5b {
        use super::*;

        /// Yates in hand under the kept script, both sides holding named, unnamed and CN cards.
        fn kept_unplayed(seed: &str, radiant: bool) -> Scenario {
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

        #[test]
        fn r3567000_r767000_the_cry_exiles_named_cards_and_spares_cn_unnamed_and_itself() {
            crate::under_kept(ID, script_v0_6_5b(), || {
                let mut s = kept_unplayed("s004-exile", false);
                s.play(ID, json!({ "zone": 1, "modes": [OVERWHELM] }));
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
            });
        }

        #[test]
        fn r3567002_r767000_only_the_chosen_activate_is_listed() {
            crate::under_kept(ID, script_v0_6_5b(), || {
                for radiant in [false, true] {
                    // The AI is offered a play for each mode...
                    let s = kept_unplayed("s004-modes", radiant);
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
                        let mut s = kept_unplayed(&format!("s004-mode-{mode}"), radiant);
                        s.play(ID, json!({ "zone": 1, "modes": [mode] }));
                        let yates = s.unit_at(P1, 1).id;
                        assert_eq!(abilities(&s, &yates), vec![mode.to_string()]);
                    }
                    // Wolf's Maw needs a Flying Unit to be offered at all.
                    let mut s = kept_unplayed("s004-mode-wolf", radiant);
                    let flyer = s.unit_at(P2, 4).id;
                    give_flying(&mut s, &flyer);
                    s.play(ID, json!({ "zone": 1, "modes": [WOLFS_MAW] }));
                    let yates = s.unit_at(P1, 1).id;
                    assert_eq!(abilities(&s, &yates), vec![WOLFS_MAW.to_string()]);
                }
            });
        }
    }
}
