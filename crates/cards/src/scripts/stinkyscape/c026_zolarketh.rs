//! S #26 ZolarKeth (SPEC §8.10 row 26): (4) Unit, Human, Acclaimed, Legendary, 2/9 → 4/18, of the held
//! Stinkyscape set (R1420).
//!   Base:    "Cry: Summon a Radiant University. It can't use its own Activate. / Activate: Choose one:
//!            summon a random Starship Piece, or Activate your Universities."
//!   Radiant: "Cry: Summon a Radiant University. It can't use its own Activate. / Activate: Summon a
//!            random Radiant Starship Piece, then Activate your Universities."
//! Engine: Cry (R1): a Radiant M #97.6 University summoned (R64), whose own Activate is then refused
//! (`refuse_own_activate`, R1876000): no zone is Locked, and a full unit row summons nothing. Activate
//! (R384, once a turn): the base face declares two modes (R81): a random non-token Starship Piece Unit
//! summoned (`query::starship_pieces()`, R1878000, narrowed to Units, R380), or each University of its
//! controller's acting on the field, in board order, runs its Activate's effect (Buff each of its other
//! permanents N times) as an ability of this card, spending none of its uses and the refused one
//! included (`run_activation`, R1876001). The Radiant face does both in order, the piece Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-026";

/// M #97.6 University.
const UNIVERSITY: &str = "meditative-097-6";
/// §8.10: "Activate" is once a turn.
const USES: i32 = 1;
/// The base face's two modes, in the order the card prints them (R81).
const PIECE: &str = "piece";
const UNIVERSITIES: &str = "universities";

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

fn activate_universities() -> Effect {
    for_each_card(ForEachCardArgs {
        cards: Arc::new(your_universities),
        each: Arc::new(|instance_id: &str| {
            run_activation(ActivationOfArgs {
                instance_id: Some(instance_id.to_string()),
                ..ActivationOfArgs::default()
            })
        }),
    })
}

fn ability(radiant: bool) -> ActivationDecl {
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
                return vec![summon_piece(true), activate_universities()];
            }
            match ctx.modes.first().map(String::as_str) {
                Some(PIECE) => vec![summon_piece(false)],
                Some(UNIVERSITIES) => vec![activate_universities()],
                _ => vec![],
            }
        }),
    }
}

fn zolarketh(radiant: bool) -> Script {
    Script {
        cry: Some(hook(|_ctx| {
            vec![
                summon(json_as(json!({ "defId": UNIVERSITY, "radiant": true }))),
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
        activations: vec![ability(radiant)],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zolarketh(false),
        radiant: zolarketh(true),
    }
}

// S #26 ZolarKeth — SPEC §8.10 row 26, BUILD M12 row S 26: "Cry (played): a Radiant University summoned,
// its own Activate refused; Activate mode 1: a random Starship Piece summoned; mode 2: each University of
// yours buffs your other permanents as its Activate does, using none of its uses; radiant 4/18, both
// modes in order, the piece Radiant".
//
// A Buff is counted by the `upgraded` events naming each card. Every scenario keeps a card in each hand
// and spares in the libraries, so R82 does not end the turn.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::{ID, PIECE, UNIVERSITIES, UNIVERSITY};
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

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
    fn r1876000_its_cry_summons_a_radiant_university_that_cannot_use_its_own_activate() {
        let mut s = board(false, json!([]));
        s.play(ID, json!({ "zone": 1 }));
        let uni = university(&s);
        assert!(uni.radiant);
        s.expect_refused_with(|s| s.activate(&uni, json!({})), "its own Activate");
        s.expect_stats(ID, json!({ "attack": 2, "health": 9 }));
    }

    #[test]
    fn r1876001_activate_your_universities_runs_their_buffs_spending_none_of_their_uses() {
        let mut s = board(false, json!([{ "def": VANILLA, "lane": 5 }]));
        s.play(ID, json!({ "zone": 1 }));
        let uni = university(&s);
        s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
        let vanilla = s.unit_at(P1, 5);
        let zolar = s.unit_at(P1, 1);
        // A Radiant University Buffs 4 times each of its other permanents (M #97.6's `times`).
        assert_eq!(buffs(&s, &vanilla), 4);
        assert_eq!(buffs(&s, &zolar), 4);
        assert_eq!(buffs(&s, &uni), 0, "never itself");
        assert_eq!(buffs(&s, &s.unit_at(P2, 1)), 0, "never an enemy's");
    }

    #[test]
    fn r384_its_activate_is_once_a_turn() {
        let mut s = board(false, json!([]));
        s.play(ID, json!({ "zone": 1 }));
        s.activate(ID, json!({ "modes": [PIECE] }));
        s.expect_refused(|s| s.activate(ID, json!({ "modes": [PIECE] })));
    }

    #[test]
    fn r1876001_a_university_free_to_activate_keeps_its_own_use() {
        let mut s = board(false, json!([{ "def": UNIVERSITY, "lane": 4 }, { "def": VANILLA, "lane": 5 }]));
        s.play(ID, json!({ "zone": 1 }));
        s.activate(ID, json!({ "modes": [UNIVERSITIES] }));
        let free = s.unit_at(P1, 4);
        let vanilla = s.unit_at(P1, 5);
        // Both Universities ran: the Radiant one's 4, then the free one's 2 or more (the Radiant one's
        // Buffs may have raised its `times`, R386).
        assert!(buffs(&s, &vanilla) >= 6, "{}", buffs(&s, &vanilla));
        // Its own use is still there to spend.
        s.activate(&free, json!({}));
        assert!(buffs(&s, &vanilla) >= 2);
    }

    #[test]
    fn r1878000_the_other_mode_summons_a_random_starship_piece() {
        let mut s = board(false, json!([]));
        s.play(ID, json!({ "zone": 1 }));
        s.activate(ID, json!({ "modes": [PIECE] }));
        let pieces = pieces(&s);
        assert_eq!(pieces.len(), 1);
        assert!(!pieces[0].radiant);
        assert!(s.last_events().iter().all(|event| !matches!(event, GameEvent::Upgraded { .. })));
    }

    #[test]
    fn r275_radiant_4_18_summons_a_radiant_piece_then_activates_your_universities() {
        let mut s = board(true, json!([]));
        s.play(ID, json!({ "zone": 1 }));
        s.expect_stats(ID, json!({ "attack": 4, "health": 18 }));
        s.activate(ID, json!({}));
        let pieces = pieces(&s);
        assert_eq!(pieces.len(), 1);
        assert!(pieces[0].radiant);
        // The piece arrived first, so the University Buffs it too.
        assert_eq!(buffs(&s, &pieces[0]), 4);
    }
}
