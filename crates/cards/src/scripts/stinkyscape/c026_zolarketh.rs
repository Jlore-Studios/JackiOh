//! S #26 ZolarKeth (SPEC §8.10 row 26): (4) Unit, Human, Acclaimed, Legendary, 2/5 → 4/10 (2/7 → 4/14
//! through v0.6.7k, issue #4285; 2/9 → 4/18 until #3041), of the Stinkyscape set.
//!   Base:    "Tribute 2
//!            Cry: Choose one: summon a University, or this gains 'End of turn: Summon a random Starship
//!            Piece.'"
//!   Radiant: "Tribute 2
//!            Cry: Choose one: summon a Radiant University, or this gains 'End of turn: Summon a random
//!            Radiant Starship Piece.'"
//! Tribute 2 since issue #4440, the play validator's (§6.3, R101), read off `staticFlags.tribute`, the
//! keyword line's printed Tribute (R366); after v0.6.5b through v0.6.9n it was Tribute 1 (issue #3996,
//! `script_v0_6_9n`, kept, R767000), and through v0.6.5b it took none (`script_v0_6_5b`, kept).
//! Engine since #2657: the Cry's two modes are declared at play (R81, R1). `university` summons an
//! M #97.6 University (R64), Radiant from the Radiant face, whose own Activate is its own to use; a full
//! unit row summons nothing. `piece` has ZolarKeth remember its new ability (`memory`), so the End of turn
//! is its own from then on, at its controller's end of turn (R62), while it stays on the field (R78 resets
//! the memory): a random non-token Starship Piece Unit summoned (`query::starship_pieces()`, R1878000,
//! narrowed to Units, R380), Radiant from the Radiant face. Through v0.5.11r (`script_v0_5_11o`, kept,
//! R767000) the memory gave it an Activate for the same summon, once a turn (R384).
//!
//! Through v0.5.9f (`script_v0_5_9f`, kept, R767000) its Cry summoned a base University that couldn't
//! use its own Activate (`refuse_own_activate`, R1876000), and its own Activate chose between a random
//! Starship Piece and "Activate your Universities": each University of its controller's acting on the
//! field, in board order, ran its Activate's effect as an ability of this card, the refused one
//! included (`run_activation`, R1876001), at most once each turn (`spendUse`, R2521003); the Radiant
//! face did both in order, the piece Radiant. Through v0.5.11 its Cry summoned a Radiant University and
//! its runs spent no use, which `script_v0_5_11` keeps.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-026";

/// M #97.6 University.
const UNIVERSITY: &str = "meditative-097-6";
/// §8.10: "Activate" is once a turn.
const USES: i32 = 1;
/// v0.5.9f's Activate's two modes, in the order the card printed them (R81).
const PIECE: &str = "piece";
const UNIVERSITIES: &str = "universities";
/// #2657: the Cry's two modes, in the order the card prints them (R81).
const CRY_UNIVERSITY: &str = "university";
const CRY_PIECE: &str = "piece";
/// #2657: what ZolarKeth remembers once its Cry gave it the Activate (since #3041 the End of turn).
const GAINED_KEY: &str = "gainedPieceActivate";
/// "Tribute 2" (R101, R366: printed, declared nowhere), since issue #4440.
const TRIBUTE: i32 = 2;
/// "Tribute 1", after v0.6.5b through v0.6.9n (issue #3996).
const TRIBUTE_V0_6_9G: i32 = 1;

/// R1876000: the Universities this Cry summoned.
fn summoned_universities(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let state: &GameState = &*ctx.state;
    summoned_so_far(ctx)
        .into_iter()
        .filter(|id| find_instance(state, id).is_some_and(|card| card.def_id == UNIVERSITY))
        .collect()
}

/// R1876001: the Universities its controller has acting on the field: unit tops in lane order, then
/// the Units standing in the backrow (R1081000).
fn your_universities(ctx: &mut EffectContext<'_>) -> Vec<String> {
    let state: &GameState = &*ctx.state;
    let mut units: Vec<&CardInstance> = slots_of(ctx.controller, Row::Units)
        .into_iter()
        .filter_map(|slot| card_at(state, slot))
        .collect();
    units.extend(standing_units_of(state, ctx.controller));
    units
        .into_iter()
        .filter(|card| card.def_id == UNIVERSITY)
        .map(|card| card.id.clone())
        .collect()
}

fn summon_piece(radiant: bool) -> Effect {
    let mut query = crate::query::starship_pieces();
    query.type_ = Some(OneOrMany::One(CardType::Unit));
    summon_random(SummonRandomArgs {
        query: Some(query),
        radiant: radiant.then_some(true),
        ..SummonRandomArgs::default()
    })
}

/// R1876001: each of your Universities runs its Activate; R2521003: with `spend`, only one not
/// Activated this turn, every use of it then spent.
fn activate_universities(spend: bool) -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(your_universities),
        each: Arc::new(move |instance_id: &str| {
            run_activation(ActivationOfArgs {
                instance_id: Some(instance_id.to_string()),
                spend_use: spend.then_some(true),
                ..ActivationOfArgs::default()
            })
        }),
    })
}

fn ability(radiant: bool, current: bool) -> ActivationDecl {
    ActivationDecl {
        id: "command".to_string(),
        label: if radiant {
            "Summon a random Radiant Starship Piece, then Activate your Universities".to_string()
        } else {
            "Summon a random Starship Piece, or Activate your Universities".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: if radiant {
            vec![]
        } else {
            vec![ModeDecl {
                kind: PromptKind::Mode,
                options: vec![PIECE.to_string(), UNIVERSITIES.to_string()],
            }]
        },
        can_activate: None,
        has: None,
        run: hook(move |ctx| {
            if radiant {
                return vec![summon_piece(true), activate_universities(current)];
            }
            match ctx.modes.first().map(String::as_str) {
                Some(PIECE) => vec![summon_piece(false)],
                Some(UNIVERSITIES) => vec![activate_universities(current)],
                _ => vec![],
            }
        }),
    }
}

/// v0.5.9f's face: `current` is issue #2521's text (R2521003); `false` builds the scripts v0.5.11 ran.
fn zolarketh(radiant: bool, current: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            // R2521003: the University is summoned on its base face from either face; through v0.5.11 on
            // its Radiant face.
            let university = if current {
                json!({ "defId": UNIVERSITY })
            } else {
                json!({ "defId": UNIVERSITY, "radiant": true })
            };
            vec![
                summon(json_as(university)),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(summoned_universities),
                    each: Arc::new(|instance_id: &str| {
                        refuse_own_activate(ActivationOfArgs {
                            instance_id: Some(instance_id.to_string()),
                            ..ActivationOfArgs::default()
                        })
                    }),
                }),
            ]
        })),
        activations: vec![ability(radiant, current)],
        ..Script::default()
    }
}

/// #3041: "End of turn: Summon a random Starship Piece", ZolarKeth's once its Cry chose it (R62): its
/// controller's end of turn, and only while the memory is there.
fn gained_end_of_turn(ctx: &mut EffectContext<'_>, radiant: bool) -> Vec<Effect> {
    let chose_it = ctx
        .live_self()
        .is_some_and(|me| me.memory.get(GAINED_KEY).and_then(Value::as_bool) == Some(true));
    if chose_it { vec![summon_piece(radiant)] } else { vec![] }
}

/// #2657: "Activate: Summon a random Starship Piece", ZolarKeth's once its Cry chose it; through v0.5.11r.
fn gained_ability(radiant: bool) -> ActivationDecl {
    ActivationDecl {
        id: "piece".to_string(),
        label: if radiant {
            "Summon a random Radiant Starship Piece".to_string()
        } else {
            "Summon a random Starship Piece".to_string()
        },
        uses: ActivationUses::Count(USES),
        cost: None,
        targets: vec![],
        modes: vec![],
        can_activate: None,
        has: Some(read_hook(|args| {
            args.self_.memory.get(GAINED_KEY).and_then(Value::as_bool) == Some(true)
        })),
        run: hook(move |_ctx| vec![summon_piece(radiant)]),
    }
}

/// #2657: "Cry: Choose one: summon a University, or this gains '…'": `end_of_turn` is #3041's End of turn,
/// else v0.5.11r's Activate; `tribute` is issue #3996's Tribute, none through v0.6.5b.
fn chooses(radiant: bool, end_of_turn: bool, tribute: Option<i32>) -> Script {
    Script {
        static_flags: tribute.map(|tribute| StaticFlags {
            tribute: Some(tribute),
            ..StaticFlags::default()
        }),
        modes: vec![ModeDecl {
            kind: PromptKind::Mode,
            options: vec![CRY_UNIVERSITY.to_string(), CRY_PIECE.to_string()],
        }],
        cry: Some(hook(move |ctx| match ctx.modes.first().map(String::as_str) {
            Some(CRY_UNIVERSITY) => {
                vec![summon(json_as(json!({ "defId": UNIVERSITY, "radiant": radiant })))]
            }
            Some(CRY_PIECE) => vec![remember(RememberArgs {
                key: GAINED_KEY.to_string(),
                value: json!(true),
            })],
            _ => vec![],
        })),
        activations: if end_of_turn { vec![] } else { vec![gained_ability(radiant)] },
        end_of_turn: end_of_turn.then(|| hook(move |ctx| gained_end_of_turn(ctx, radiant))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: chooses(false, true, Some(TRIBUTE)),
        radiant: chooses(true, true, Some(TRIBUTE)),
    }
}

/// R767000: after v0.6.5b through v0.6.9n it was Tribute 1 (issue #4440); `KEPT_SCRIPTS` folds those replays
/// under it.
pub fn script_v0_6_9n() -> CardScripts {
    CardScripts {
        base: chooses(false, true, Some(TRIBUTE_V0_6_9G)),
        radiant: chooses(true, true, Some(TRIBUTE_V0_6_9G)),
    }
}

/// R767000: through v0.6.5b it took no Tribute (issue #3996); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: chooses(false, true, None),
        radiant: chooses(true, true, None),
    }
}

/// R767000: through v0.5.11r the Cry's `piece` mode gave ZolarKeth an Activate, once a turn, that summoned
/// the random Starship Piece; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11o() -> CardScripts {
    CardScripts {
        base: chooses(false, false, None),
        radiant: chooses(true, false, None),
    }
}

/// R767000: through v0.5.9f its Cry summoned a base University that couldn't use its own Activate, and
/// its own Activate summoned a random Starship Piece or ran its Universities, each at most once a turn
/// (R2521003); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_9f() -> CardScripts {
    CardScripts {
        base: zolarketh(false, true),
        radiant: zolarketh(true, true),
    }
}

/// R767000: through v0.5.11 its Cry summoned a Radiant University and "Activate your Universities"
/// spent none of their uses (R1876001); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    CardScripts {
        base: zolarketh(false, false),
        radiant: zolarketh(true, false),
    }
}

// S #26 ZolarKeth — SPEC §8.10 row 26, BUILD M12 row S 26. The tests of v0.5.9f's University runs fold under
// the kept script.
//
// A Buff is counted by the `upgraded` events naming each card. A University never Buffs itself, so one
// no other University stands beside keeps its printed `times` (2) and its one use (R386's Buffs could
// raise either); the boards below keep each count exact by that. Every scenario keeps a card in each
// hand and spares in the libraries, so R82 does not end the turn.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use jackioh_engine::play_choices::tribute_cost_of;

    use super::{
        CRY_PIECE, CRY_UNIVERSITY, ID, PIECE, TRIBUTE, TRIBUTE_V0_6_9G, UNIVERSITIES, UNIVERSITY, script_v0_5_11,
        script_v0_5_11o, script_v0_5_9f, script_v0_6_5b, script_v0_6_9n,
    };
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // Mr. Vanilla
    const FILLER: &str = "core-005";
    /// A base University's Buffs on each of its other permanents (M #97.6's `times`).
    const BASE_TIMES: usize = 2;
    /// A Radiant University's.
    const RADIANT_TIMES: usize = 4;

    /// p1 holds ZolarKeth (`radiant` its face) with 10 mana and has `field` on the field.
    fn board(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": field,
                "library": filler(6),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": filler(6) },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.state_mut().players.p2.auto_end_turn = Some(false);
        s
    }

    fn university(s: &Scenario) -> CardInstance {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .find(|unit| unit.def_id == UNIVERSITY)
            .expect("a University")
    }

    fn buffs(s: &Scenario, card: &CardInstance) -> usize {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if *instance_id == card.id))
            .count()
    }

    fn pieces(s: &Scenario) -> Vec<CardInstance> {
        (1..=5)
            .filter_map(|lane| s.unit(P1, lane))
            .filter(|unit| is_starship_piece(s.state(), unit))
            .collect()
    }

    /// The lanes of the Units `paying` boards hold for ZolarKeth's Tribute 2 (issue #4440).
    const TRIBUTE_LANES: [i32; 2] = [3, 4];

    /// `board` with a Vanilla in each of `TRIBUTE_LANES` to pay the Tribute, beside `field`.
    fn paying(radiant: bool, field: Value) -> Scenario {
        let mut field = field.as_array().cloned().unwrap_or_default();
        for lane in TRIBUTE_LANES {
            field.push(json!({ "def": VANILLA, "lane": lane }));
        }
        board(radiant, json!(field))
    }

    /// Plays ZolarKeth into zone 1 choosing `mode`, the Units in `TRIBUTE_LANES` paying its Tribute 2.
    fn cry(s: &mut Scenario, mode: &str) {
        let tributes = TRIBUTE_LANES.map(|lane| s.unit_at(P1, lane).id);
        s.play(ID, json!({ "zone": 1, "modes": [mode], "tributes": tributes }));
        for tribute in &tributes {
            s.expect_in_zone(tribute.as_str(), "graveyard");
        }
    }

    /// Issue #4440: Tribute 2 on both faces, so one Unit no longer pays it.
    #[test]
    fn r101_it_needs_tribute_2_on_both_faces() {
        for radiant in [false, true] {
            let mut s = paying(radiant, json!([]));
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE);
            let first = s.unit_at(P1, TRIBUTE_LANES[0]).id;
            s.expect_refused(|s| s.play(ID, json!({ "zone": 1, "modes": [CRY_UNIVERSITY] })));
            s.expect_refused(|s| {
                s.play(ID, json!({ "zone": 1, "modes": [CRY_UNIVERSITY], "tributes": [first.as_str()] }))
            });
            cry(&mut s, CRY_UNIVERSITY);
            s.expect_in_zone(ID, "field");
        }
    }

    /// R767000: after v0.6.5b through v0.6.9n one Unit paid it (issue #4440).
    #[test]
    fn r767000_the_script_kept_for_v0_6_9n_takes_tribute_1() {
        crate::under_kept(ID, script_v0_6_9n(), || {
            let mut s = paying(false, json!([]));
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE_V0_6_9G);
            let first = s.unit_at(P1, TRIBUTE_LANES[0]).id;
            s.play(ID, json!({ "zone": 1, "modes": [CRY_UNIVERSITY], "tributes": [first.as_str()] }));
            s.expect_in_zone(first.as_str(), "graveyard");
            s.expect_in_zone(ID, "field");
            assert!(s.unit(P1, TRIBUTE_LANES[1]).is_some(), "the second Unit stays");
        });
    }

    /// R767000: through v0.6.5b it took no Tribute (issue #3996).
    #[test]
    fn r767000_the_script_kept_for_v0_6_5b_takes_no_tribute() {
        crate::under_kept(ID, script_v0_6_5b(), || {
            let mut s = board(false, json!([]));
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), 0);
            s.play(ID, json!({ "zone": 1, "modes": [CRY_PIECE] }));
            s.end_turn();
            assert_eq!(pieces(&s).len(), 1);
        });
    }

    #[test]
    fn its_cry_summons_a_university_its_own_activate_its_own() {
        let mut s = paying(false, json!([{ "def": VANILLA, "lane": 5 }]));
        cry(&mut s, CRY_UNIVERSITY);
        let uni = university(&s);
        assert!(!uni.radiant, "a base University");
        s.activate(&uni, json!({}));
        assert_eq!(buffs(&s, &s.unit_at(P1, 5)), BASE_TIMES);
        s.expect_refused(|s| s.activate(ID, json!({})));
    }

    #[test]
    fn r62_its_cry_may_give_it_an_end_of_turn_that_summons_a_random_starship_piece_at_its_controllers_end_of_turn() {
        let mut s = paying(false, json!([]));
        cry(&mut s, CRY_PIECE);
        assert!((1..=5).filter_map(|lane| s.unit(P1, lane)).all(|unit| unit.def_id != UNIVERSITY));
        assert!(pieces(&s).is_empty(), "nothing as it is played");
        s.expect_refused(|s| s.activate(ID, json!({})));
        s.end_turn();
        let first = pieces(&s);
        assert_eq!(first.len(), 1, "one at the end of its controller's turn");
        assert!(!first[0].radiant);
        s.end_turn();
        assert_eq!(pieces(&s).len(), 1, "none at the opponent's end of turn");
        s.end_turn();
        assert_eq!(pieces(&s).len(), 2, "one more at its controller's next");
    }

    #[test]
    fn r62_without_the_cry_it_summons_no_piece_and_has_no_activate() {
        let mut s = board(false, json!([{ "def": ID, "lane": 1 }]));
        let zolar = s.unit_at(P1, 1);
        s.expect_refused(|s| s.activate(&zolar, json!({})));
        s.end_turn();
        assert!(pieces(&s).is_empty());
    }

    #[test]
    fn r62_the_university_mode_gives_no_end_of_turn() {
        let mut s = paying(false, json!([]));
        cry(&mut s, CRY_UNIVERSITY);
        s.end_turn();
        assert!(pieces(&s).is_empty());
    }

    #[test]
    fn r275_radiant_4_10_a_radiant_university_or_a_radiant_piece_at_the_end_of_turn() {
        let mut s = paying(true, json!([]));
        cry(&mut s, CRY_UNIVERSITY);
        s.expect_stats(ID, json!({ "attack": 4, "health": 10 }));
        assert!(university(&s).radiant, "a Radiant University");

        let mut s = paying(true, json!([]));
        cry(&mut s, CRY_PIECE);
        s.end_turn();
        let pieces = pieces(&s);
        assert_eq!(pieces.len(), 1);
        assert!(pieces[0].radiant);
    }

    /// R767000: through v0.5.11r its Cry's `piece` mode gave it an Activate, not an End of turn.
    #[test]
    fn r384_r767000_the_kept_script_gives_an_activate_that_summons_a_random_starship_piece_once_a_turn() {
        crate::under_kept(ID, script_v0_5_11o(), || {
            for radiant in [false, true] {
                let mut s = board(radiant, json!([]));
                s.play(ID, json!({ "zone": 1, "modes": [CRY_PIECE] }));
                s.activate(ID, json!({}));
                let pieces = pieces(&s);
                assert_eq!(pieces.len(), 1);
                assert_eq!(pieces[0].radiant, radiant);
                s.expect_refused(|s| s.activate(ID, json!({})));
            }
        });
    }

    /// R767000: nor did that script summon a piece at the end of turn.
    #[test]
    fn r62_r767000_the_kept_script_summons_no_piece_at_the_end_of_turn() {
        crate::under_kept(ID, script_v0_5_11o(), || {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1, "modes": [CRY_PIECE] }));
            s.end_turn();
            assert!(pieces(&s).is_empty());
        });
    }

    #[test]
    fn r2521003_r1876000_its_cry_summons_a_base_university_that_cannot_use_its_own_activate() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            let uni = university(&s);
            assert!(!uni.radiant, "a base University");
            s.expect_stats(&uni, json!({ "attack": 0, "health": 12 }));
            s.expect_refused_with(|s| s.activate(&uni, json!({})), "its own Activate");
            s.expect_stats(ID, json!({ "attack": 2, "health": 5 }));
        });
    }

    #[test]
    fn r1876001_activate_your_universities_runs_their_buffs_as_this_cards_effect() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 5 }]));
            s.play(ID, json!({ "zone": 1 }));
            let uni = university(&s);
            s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
            let vanilla = s.unit_at(P1, 5);
            let zolar = s.unit_at(P1, 1);
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
            assert_eq!(buffs(&s, &zolar), BASE_TIMES);
            assert_eq!(buffs(&s, &uni), 0, "never itself");
            assert_eq!(buffs(&s, &s.unit_at(P2, 1)), 0, "never an enemy's");
        });
    }

    #[test]
    fn r384_its_activate_is_once_a_turn() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s.activate(ID, json!({ "modes": [PIECE] }));
            s.expect_refused(|s| s.activate(ID, json!({ "modes": [PIECE] })));
        });
    }

    #[test]
    fn r2521003_a_university_it_ran_has_spent_its_own_use_this_turn() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            // ZolarKeth already on the field (no Cry), its one University free to activate.
            let field = json!([
                { "def": ID, "lane": 1 },
                { "def": UNIVERSITY, "lane": 4 },
                { "def": VANILLA, "lane": 5 },
            ]);
            let mut s = board(false, field);
            let zolar = s.unit_at(P1, 1);
            let free = s.unit_at(P1, 4);
            let vanilla = s.unit_at(P1, 5);
            s.activate(&zolar, json!({ "modes": [UNIVERSITIES] }));
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
            s.expect_refused_with(|s| s.activate(&free, json!({})), "already been used this turn");
        });
    }

    #[test]
    fn r2521003_a_university_that_activated_itself_is_skipped() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            // The free University (lane 2) runs before the Cry's (lane 3): no Buff of the Cry's reaches it first.
            let mut s = board(false, json!([{ "def": UNIVERSITY, "lane": 2 }, { "def": VANILLA, "lane": 5 }]));
            let free = s.unit_at(P1, 2);
            let vanilla = s.unit_at(P1, 5);
            s.activate(&free, json!({}));
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
            s.play(ID, json!({ "zone": 1 }));
            let cry = s.unit_at(P1, 3);
            assert_eq!(cry.def_id, UNIVERSITY, "the Cry's University");
            s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
            // Only the Cry's University ran: its Buffs, none from the spent one (which would reach the Cry's).
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
            assert_eq!(buffs(&s, &free), BASE_TIMES);
            assert_eq!(buffs(&s, &cry), 0, "the spent University ran nothing");
        });
    }

    #[test]
    fn r2521003_two_zolarketh_the_same_turn_buff_once_per_university() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(false, json!([{ "def": ID, "lane": 3 }, { "def": VANILLA, "lane": 5 }]));
            s.play(ID, json!({ "zone": 1 }));
            let first = s.unit_at(P1, 1);
            let second = s.unit_at(P1, 3);
            let vanilla = s.unit_at(P1, 5);
            assert_eq!(university(&s).id, s.unit_at(P1, 2).id, "the Cry's University");
            s.activate(&first, json!({ "modes": [UNIVERSITIES] }));
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
            s.activate(&second, json!({ "modes": [UNIVERSITIES] }));
            assert_eq!(buffs(&s, &vanilla), 0, "the University spent its one use on the first");
            assert_eq!(buffs(&s, &first), 0);
        });
    }

    #[test]
    fn r2521003_next_turn_it_runs_its_universities_again() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 5 }]));
            s.play(ID, json!({ "zone": 1 }));
            let vanilla = s.unit_at(P1, 5);
            s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.state().active, P1);
            s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
            assert_eq!(buffs(&s, &vanilla), BASE_TIMES);
        });
    }

    #[test]
    fn r1878000_the_other_mode_summons_a_random_starship_piece() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s.activate(ID, json!({ "modes": [PIECE] }));
            let pieces = pieces(&s);
            assert_eq!(pieces.len(), 1);
            assert!(!pieces[0].radiant);
            assert!(s.last_events().iter().all(|event| !matches!(event, GameEvent::Upgraded { .. })));
        });
    }

    #[test]
    fn r275_r2521003_radiant_4_10_a_base_university_then_a_radiant_piece_and_your_universities() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(true, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(ID, json!({ "attack": 4, "health": 10 }));
            let uni = university(&s);
            assert!(!uni.radiant, "a base University from the Radiant face too");
            s.expect_refused_with(|s| s.activate(&uni, json!({})), "its own Activate");
            s.activate(ID, json!({}));
            let pieces = pieces(&s);
            assert_eq!(pieces.len(), 1);
            assert!(pieces[0].radiant);
            // The piece arrived first, so the University Buffs it too.
            assert_eq!(buffs(&s, &pieces[0]), BASE_TIMES);
        });
    }

    /// R767000: the script kept for the replays of v0.5.11 and before: a Radiant University, and runs
    /// that spend no use.
    #[test]
    fn r767000_a_game_on_v0_5_11_summons_a_radiant_university_and_spends_no_use() {
        crate::under_kept(ID, script_v0_5_11(), || {
            let mut s = board(false, json!([{ "def": VANILLA, "lane": 5 }]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(university(&s).radiant);
            s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
            assert_eq!(buffs(&s, &s.unit_at(P1, 5)), RADIANT_TIMES);

            let field = json!([
                { "def": ID, "lane": 1 },
                { "def": UNIVERSITY, "lane": 4 },
                { "def": VANILLA, "lane": 5 },
            ]);
            let mut s = board(false, field);
            let zolar = s.unit_at(P1, 1);
            let free = s.unit_at(P1, 4);
            s.activate(&zolar, json!({ "modes": [UNIVERSITIES] }));
            // Its own use is still there to spend.
            s.activate(&free, json!({}));
            assert_eq!(buffs(&s, &s.unit_at(P1, 5)), BASE_TIMES);
        });
    }
}
