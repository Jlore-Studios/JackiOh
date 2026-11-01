//! S #26 ZolarKeth (SPEC §8.10 row 26): (4) Unit, Human, Acclaimed, Legendary, 2/9 → 4/18, of the
//! Stinkyscape set.
//!   Base:    "Cry: Choose one: summon a University, or this gains 'Activate: Summon a random Starship
//!            Piece.'"
//!   Radiant: "Cry: Choose one: summon a Radiant University, or this gains 'Activate: Summon a random
//!            Radiant Starship Piece.'"
//! Engine since #2657: the Cry's two modes are declared at play (R81, R1). `university` summons an
//! M #97.6 University (R64), Radiant from the Radiant face, whose own Activate is its own to use; a full
//! unit row summons nothing. `piece` has ZolarKeth remember its new ability (`memory`), so the Activate
//! is its own from then on, once a turn (R384), while it stays on the field (R78 resets the memory): a
//! random non-token Starship Piece Unit summoned (`query::starship_pieces()`, R1878000, narrowed to
//! Units, R380), Radiant from the Radiant face.
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
/// #2657: what ZolarKeth remembers once its Cry gave it the Activate.
const GAINED_KEY: &str = "gainedPieceActivate";

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

/// #2657: "Activate: Summon a random Starship Piece", ZolarKeth's once its Cry chose it.
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

/// #2657: "Cry: Choose one: summon a University, or this gains 'Activate: …'".
fn chooses(radiant: bool) -> Script {
    Script {
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
        activations: vec![gained_ability(radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: chooses(false),
        radiant: chooses(true),
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

// S #26 ZolarKeth — SPEC §8.10 row 26, BUILD M12 row S 26: "Cry (played), choose one: a University
// summoned (Radiant from the Radiant face), its own Activate its own; or ZolarKeth gains 'Activate: a
// random Starship Piece summoned', once a turn (Radiant: a Radiant piece); without that choice it has no
// Activate; radiant 4/18". The tests of v0.5.9f's University runs fold under the kept script.
//
// A Buff is counted by the `upgraded` events naming each card. A University never Buffs itself, so one
// no other University stands beside keeps its printed `times` (2) and its one use (R386's Buffs could
// raise either); the boards below keep each count exact by that. Every scenario keeps a card in each
// hand and spares in the libraries, so R82 does not end the turn.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{CRY_PIECE, CRY_UNIVERSITY, ID, PIECE, UNIVERSITIES, UNIVERSITY, script_v0_5_11, script_v0_5_9f};
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4.
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

    /// The Buffs the last step put on `card`.
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

    #[test]
    fn its_cry_summons_a_university_its_own_activate_its_own() {
        let mut s = board(false, json!([{ "def": VANILLA, "lane": 5 }]));
        s.play(ID, json!({ "zone": 1, "modes": [CRY_UNIVERSITY] }));
        let uni = university(&s);
        assert!(!uni.radiant, "a base University");
        s.activate(&uni, json!({}));
        assert_eq!(buffs(&s, &s.unit_at(P1, 5)), BASE_TIMES);
        s.expect_refused(|s| s.activate(ID, json!({})));
    }

    #[test]
    fn its_cry_may_give_it_an_activate_that_summons_a_random_starship_piece_once_a_turn() {
        let mut s = board(false, json!([]));
        s.play(ID, json!({ "zone": 1, "modes": [CRY_PIECE] }));
        assert!((1..=5).filter_map(|lane| s.unit(P1, lane)).all(|unit| unit.def_id != UNIVERSITY));
        s.activate(ID, json!({}));
        let pieces = pieces(&s);
        assert_eq!(pieces.len(), 1);
        assert!(!pieces[0].radiant);
        s.expect_refused(|s| s.activate(ID, json!({})));
    }

    #[test]
    fn without_the_cry_it_has_no_activate() {
        let mut s = board(false, json!([{ "def": ID, "lane": 1 }]));
        let zolar = s.unit_at(P1, 1);
        s.expect_refused(|s| s.activate(&zolar, json!({})));
    }

    #[test]
    fn r275_radiant_4_18_a_radiant_university_or_a_radiant_piece() {
        let mut s = board(true, json!([]));
        s.play(ID, json!({ "zone": 1, "modes": [CRY_UNIVERSITY] }));
        s.expect_stats(ID, json!({ "attack": 4, "health": 18 }));
        assert!(university(&s).radiant, "a Radiant University");

        let mut s = board(true, json!([]));
        s.play(ID, json!({ "zone": 1, "modes": [CRY_PIECE] }));
        s.activate(ID, json!({}));
        let pieces = pieces(&s);
        assert_eq!(pieces.len(), 1);
        assert!(pieces[0].radiant);
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
            s.expect_stats(ID, json!({ "attack": 2, "health": 9 }));
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
    fn r275_r2521003_radiant_4_18_a_base_university_then_a_radiant_piece_and_your_universities() {
        crate::under_kept(ID, script_v0_5_9f(), || {
            let mut s = board(true, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(ID, json!({ "attack": 4, "health": 18 }));
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
