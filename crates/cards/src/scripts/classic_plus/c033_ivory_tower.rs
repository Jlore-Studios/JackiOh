//! C+ #33 Ivory Tower (SPEC §8.7 row 33, R418, R653, R1194000): (2) Field Spell, Rare.
//!   Base:    "The first Unit you stack onto this is fused into it.\nAura: Your Units have Flying and Stack."
//!   Radiant: "The first Unit you stack onto this becomes Radiant and is fused into it.\nAura: Your Units
//!            have Flying and Stack."
//! The engine owns the stacking (`fusesCarried`, R446, R653): a Unit you play may name this zone while
//! no Unit has stood on it this stay, and stands on it while its play resolves, its Cry included. Once
//! that play has resolved, the Unit on it is fused into this per R77, this the kept card: a Field Spell
//! still, with the Unit's text and keywords, and the Unit ceases to exist. The Radiant face makes the
//! Unit Radiant as it lands, so its Cry runs on that face, and fuses it in on its Radiant face (R469).
//! The Aura (patch v0.3.13, #1194) lends Flying and Stack to its controller's Units on the field and to
//! the Unit cards in their hand and resolving zone, where a play's placement reads them (R1194000);
//! `script_v0_3_7b` keeps the Tower without it for the replays of v0.3.7b and before (R767000).

use jackioh_engine::effects::{fuse_cards, set_radiant};
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-033";

/// The Unit standing on this Tower now, if any.
fn rider_of(state: &GameState, tower: &CardInstance) -> Option<CardInstance> {
    let at = slot_of(state, tower)?;
    carried_at(state, at).cloned()
}

/// R653: the event is the play of the Unit stacked onto this Tower, landing or resolved.
fn is_stacked_play(ctx: &EffectContext<'_>, event: &GameEvent) -> bool {
    let Some(tower) = ctx.live_self() else {
        return false;
    };
    match event {
        GameEvent::CardPlayed { instance_id, .. } | GameEvent::CardResolved { instance_id, .. } => {
            stacked_onto(tower).is_some_and(|stacked| stacked == instance_id.as_str())
        }
        _ => false,
    }
}

/// R653: once the stacked Unit's play has resolved, the Unit standing on this — that card, or what an
/// answer to the play left in its place — is fused into it.
fn fuse_in(radiant: bool) -> TriggerDef {
    TriggerDef::new("ivory-tower-fuse", &[GameEventType::CardResolved], move |ctx, event| {
        // Not a trap, so the condition is read in `run` (only traps consult `when`, R99).
        if !is_stacked_play(ctx, event) {
            return vec![];
        }
        let Some(tower) = ctx.live_self().cloned() else {
            return vec![];
        };
        let Some(rider) = rider_of(&*ctx.state, &tower) else {
            return vec![];
        };
        let mut args = json!({ "instanceIds": [rider.id], "targetInstanceId": tower.id });
        if radiant {
            args["radiantIngredients"] = json!(true);
        }
        vec![fuse_cards(json_as(args))]
    })
}

/// The Radiant face's landing trigger: the stacked Unit becomes Radiant as it is played onto this.
fn radiant_on_landing() -> TriggerDef {
    TriggerDef::new("ivory-tower-radiant", &[GameEventType::CardPlayed], |ctx, event| match event {
        GameEvent::CardPlayed { instance_id, .. } if is_stacked_play(ctx, event) => {
            vec![set_radiant(json_as(json!({ "instanceId": instance_id })))]
        }
        _ => vec![],
    })
}

/// R1194000: "Aura: Your Units have Flying and Stack." — its controller's Units on the field (R1081000),
/// and the Unit cards in their hand and resolving zone, where a play's placement reads them (§10.5
/// steps 1 and 4, R783007).
fn flying_and_stack() -> AuraHook {
    aura_hook(|args: HookArgs<'_>| {
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |card: &CardInstance| match card.zone {
                Zone::Hand { player } | Zone::Resolving { player } => {
                    player == controller && card_type_of_face(state, &card.def_id, card.radiant) == CardType::Unit
                }
                _ => card.controller == controller && in_units_row_or_standing(state, card),
            }),
            mod_: StatMod {
                keywords: Some(vec![Keyword::Flying, Keyword::Stack]),
                ..StatMod::default()
            },
        }]
    })
}

fn tower(aura: Option<AuraHook>) -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            fuses_carried: Some(true),
            ..StaticFlags::default()
        }),
        triggers: vec![fuse_in(false)],
        aura,
        ..Script::default()
    };
    let radiant = Script {
        triggers: vec![radiant_on_landing(), fuse_in(true)],
        ..base.clone()
    };
    CardScripts { base, radiant }
}

pub fn script() -> CardScripts {
    tower(Some(flying_and_stack()))
}

/// R767000: the scripts v0.3.7b and every version before it ran, with no Aura (patch v0.3.13, #1194);
/// `KEPT_SCRIPTS` folds those versions' replays under them.
pub fn script_v0_3_7b() -> CardScripts {
    tower(None)
}

// C+ #33 Ivory Tower (SPEC §8.7, R418, R653): the first Unit played onto its zone stands on it
// while play resolves, then fuses in per R77 (kept card). If Tower leaves early, Unit steps down
// (R446). Radiant makes the Unit Radiant as it lands and fuses on Radiant face (R469). Both faces'
// Aura gives your Units in hand and on the field Flying and Stack, the opponent's neither, so a Unit
// in hand is offered occupied zones and empty backrow zones; it ends with the Tower (R1194000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOWER: &str = "classicplus-033";
    const TOKENS: &str = "core-015"; // (1) 1/1, Cry: summon a Rush Token (Radiant: 3).
    const RUSH: &str = "core-t-rush";
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const TOE_CRACKER: &str = "classic-006"; // (2) 3/4, Aura: your Traps cost (0).
    const JAY: &str = "classicplus-049"; // (2) 3/6 Taunt, End of turn: a random card in your hand costs (2) less.
    const SHEEPLE: &str = "classic-082"; // (1) 1/1, Death: draw 2.
    const GUY_ATT: &str = "classicplus-005"; // (2) 6/8, Cry: destroy every backrow card you control.
    const SHEEPISH: &str = "core-041"; // (1) Trap: transforms an opponent's played Unit once its Cry resolves.
    const SHEEP: &str = "core-t-sheep";
    const MAGIC_JAMMED: &str = "core-036"; // (1) Spell: destroy target backrow card, Lock its zone.
    const BIG_SPELL: &str = "core-035"; // (1) Spell, a hand card to discount.
    const FILLER: &str = "core-005";
    const DECK: [&str; 4] = [FILLER, FILLER, FILLER, FILLER];

    #[derive(Default)]
    struct TowerOptions {
        radiant: bool,
        field: Option<Value>,
        hand: Option<Vec<&'static str>>,
        p2_backrow: Option<Value>,
    }

    /// p1 with a Tower in backrow lane 2; plays `rider` onto it. Answers `(s, tower, rider)`.
    fn tower_with(rider: &str, opts: TowerOptions) -> (Scenario, String, String) {
        let mut hand = vec![rider];
        hand.extend(opts.hand.unwrap_or_else(|| vec![FILLER]));
        let mut s = scenario(json!({
            "p1": {
                "hand": hand,
                "field": opts.field.unwrap_or_else(|| json!([])),
                "backrow": [{ "def": TOWER, "lane": 2, "radiant": opts.radiant }],
                "library": DECK,
                "mana": 8,
            },
            "p2": { "hand": [FILLER], "field": [MENACE], "backrow": opts.p2_backrow.unwrap_or_else(|| json!([])), "library": DECK },
        }));
        let tower = s.card(TOWER).id.clone();
        let rider_id = s.card(rider).id.clone();
        s.play(rider, json!({ "zone": 2, "row": "backrow" }));
        (s, tower, rider_id)
    }

    fn text_of(s: &Scenario, id: &str) -> String {
        let card = s.card(id);
        let def = def_of(Some(s.state()), &card.def_id);
        if card.radiant { def.radiant.text.clone() } else { def.base.text.clone() }
    }

    fn index_of(s: &Scenario, want: GameEventType) -> i64 {
        s.events()
            .iter()
            .position(|event| event.event_type() == want)
            .map(|at| at as i64)
            .unwrap_or(-1)
    }

    fn any_event(s: &Scenario, want: GameEventType) -> bool {
        s.events().iter().any(|event| event.event_type() == want)
    }

    fn tower_zone() -> ZoneRef {
        ZoneRef { player: P1, row: Row::Backrow, lane: 2 }
    }

    use crate::js;

    fn costs_of_hand(s: &Scenario) -> Vec<i32> {
        s.hand(P1).iter().map(|card| effective_cost(s.state(), card, Default::default())).collect()
    }

    /// R1194000: whether this card reads as having Flying and Stack where it is, through the layers.
    fn winged(s: &Scenario, card: &CardInstance) -> (bool, bool) {
        (unit_has(s.state(), card, KeywordKind::Flying), unit_has(s.state(), card, KeywordKind::Stack))
    }

    /// The first card of this def in `player`'s hand.
    fn held(s: &Scenario, player: PlayerId, def_id: &str) -> CardInstance {
        s.hand(player)
            .into_iter()
            .find(|card| card.def_id == def_id)
            .unwrap_or_else(|| panic!("no {def_id} in hand"))
    }

    /// The zones `legal_actions` lists for a play of this card, in its order.
    fn zones_listed(s: &Scenario, card: &CardInstance) -> Vec<(Row, i32)> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, zone: Some(zone), .. } if instance_id == card.id => {
                    Some((zone.row, zone.lane))
                }
                _ => None,
            })
            .collect()
    }

    /// R1194000: p1 holds a Menace, the opponent a Menace in hand and one on the field, and p1 a
    /// Vanilla on the field and the Tower (Radiant on `radiant`) in backrow lane 2.
    fn aura_board(radiant: bool) -> Scenario {
        crate::register_all();
        scenario(json!({
            "p1": {
                "hand": [MENACE, BIG_SPELL],
                "field": [{ "def": VANILLA, "lane": 1 }],
                "backrow": [{ "def": TOWER, "lane": 2, "radiant": radiant }],
                "library": DECK,
                "mana": 8,
            },
            "p2": { "hand": [MENACE], "field": [{ "def": MENACE, "lane": 1 }], "library": DECK },
        }))
    }

    /// R1194000: the Aura reaches p1's Units in hand and on the field and no Spell, and none of p2's.
    fn the_aura_reaches_your_units_alone(s: &Scenario) {
        assert_eq!(winged(s, &held(s, P1, MENACE)), (true, true));
        assert_eq!(winged(s, &s.unit(P1, 1).expect("p1's Vanilla")), (true, true));
        assert_eq!(winged(s, &held(s, P1, BIG_SPELL)), (false, false));
        assert_eq!(winged(s, &held(s, P2, MENACE)), (false, false));
        assert_eq!(winged(s, &s.unit(P2, 1).expect("p2's Menace")), (false, false));
    }

    mod c_n33_ivory_tower {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r418_r653_a_unit_you_play_may_name_the_tower_s_zone_its_cry_resolves_then_it_is_fused_into_the_tower() {
                crate::register_all();
                let (mut s, tower, rider) = tower_with(TOKENS, TowerOptions::default());
                // Its Cry ran while it stood on the Tower: a Rush Token in the unit row.
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id).as_deref(), Some(RUSH));
                // Then it was fused in: the Tower is the kept card, still a Field Spell in its zone.
                assert_eq!(s.backrow(P1, 2).map(|card| card.id), Some(tower.clone()));
                let fused = def_of(Some(s.state()), &s.card(&tower).def_id).clone();
                assert_eq!(fused.type_, CardType::FieldSpell);
                assert_eq!(fused.name, "Me and Mr Token + Ivory Tower");
                assert!(carried_at(s.state(), tower_zone()).is_none());
                s.expect_in_zone(&rider, "gone");
                assert!(index_of(&s, GameEventType::CardResolved) < index_of(&s, GameEventType::Fused));
            }

            #[test]
            fn r653_the_unit_ceases_to_exist_no_death_not_destroyed_never_in_a_graveyard() {
                crate::register_all();
                let (mut s, _tower, rider) = tower_with(SHEEPLE, TowerOptions::default());
                s.expect_in_zone(&rider, "gone");
                assert_eq!(s.pile(P1, "graveyard").len(), 0);
                assert!(!any_event(&s, GameEventType::Destroyed));
                assert_eq!(s.hand(P1).len(), 1); // the filler: Sheeople's Death drew nothing
            }

            #[test]
            fn r653_the_tower_takes_the_unit_s_text_and_keywords_its_aura_covers_your_side_from_the_backrow() {
                crate::register_all();
                let (s, tower, _rider) =
                    tower_with(TOE_CRACKER, TowerOptions { hand: Some(vec![SHEEPISH, FILLER]), ..TowerOptions::default() });
                assert!(text_of(&s, &tower).contains("Aura: Your Traps cost (0)."));
                assert_eq!(effective_cost(s.state(), s.card(SHEEPISH), Default::default()), 0);
            }

            #[test]
            fn r653_the_unit_s_keywords_do_nothing_in_the_backrow_a_fused_taunt_binds_no_attacker_and_its_end_of_turn_line_runs() {
                crate::register_all();
                let (mut s, tower, _rider) =
                    tower_with(JAY, TowerOptions { hand: Some(vec![BIG_SPELL, FILLER]), ..TowerOptions::default() });
                let keywords = def_of(Some(s.state()), &s.card(&tower).def_id).base.keywords.clone();
                assert!(keywords.iter().any(|keyword| js(keyword) == json!({ "kind": "Taunt" })));
                let before = costs_of_hand(&s);
                s.end_turn();
                let after = costs_of_hand(&s);
                assert!(after.iter().sum::<i32>() < before.iter().sum::<i32>());
                // p2's Menace may attack the hero: the Tower's Taunt is a backrow card's, and binds no attacker.
                s.attack(MENACE, "hero");
                s.expect_health(P1, HERO_HEALTH - 9);
            }

            #[test]
            fn r418_the_tower_stays_a_backrow_card_and_the_unit_s_death_lives_on_it_fires_when_the_tower_is_destroyed() {
                crate::register_all();
                let (mut s, tower, _rider) =
                    tower_with(SHEEPLE, TowerOptions { hand: Some(vec![MAGIC_JAMMED, FILLER]), ..TowerOptions::default() });
                let held = s.hand(P1).len();
                s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": tower }] }));
                s.expect_in_zone(&tower, "graveyard");
                // Magic Jammed left the hand; Sheeople's Death drew 2.
                assert_eq!(s.hand(P1).len(), held - 1 + 2);
            }

            #[test]
            fn r653_after_the_first_unit_no_more_units_can_be_stacked_onto_it() {
                crate::register_all();
                let (mut s, _tower, _rider) =
                    tower_with(TOKENS, TowerOptions { hand: Some(vec![MENACE, FILLER]), ..TowerOptions::default() });
                s.expect_refused(|s| s.play(MENACE, json!({ "zone": 2, "row": "backrow" })));
                let menace = s.card(MENACE).id.clone();
                let plays: Vec<ActionBody> = legal_actions(s.state(), P1)
                    .into_iter()
                    .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == menace))
                    .collect();
                assert!(!plays.is_empty());
                // R1194000: the Aura's Flying offers the other backrow zones, never the Tower's own.
                assert!(!plays.iter().any(|action| matches!(
                    action,
                    ActionBody::Play { zone: Some(zone), .. } if zone.row == Row::Backrow && zone.lane == 2
                )));
            }

            #[test]
            fn r653_a_tower_its_unit_s_cry_destroys_fuses_nothing_guy_att_steps_down_into_a_unit_zone_r446() {
                crate::register_all();
                let (mut s, tower, rider) = tower_with(GUY_ATT, TowerOptions::default());
                s.expect_in_zone(&tower, "graveyard");
                assert_eq!(s.unit(P1, 2).map(|unit| unit.id), Some(rider.clone()));
                assert!(!any_event(&s, GameEventType::Fused));
                assert_eq!(s.card(&rider).def_id, GUY_ATT);
            }

            #[test]
            fn r653_an_answer_that_replaces_the_unit_where_it_stands_leaves_its_replacement_to_be_fused_sheepish_s_sheep() {
                crate::register_all();
                let (s, tower, _rider) = tower_with(
                    TOKENS,
                    TowerOptions { p2_backrow: Some(json!([{ "def": SHEEPISH, "faceUp": false }])), ..TowerOptions::default() },
                );
                assert!(any_event(&s, GameEventType::TrapFired));
                assert_eq!(def_of(Some(s.state()), &s.card(&tower).def_id).name, "Sheep Token + Ivory Tower");
                assert!(carried_at(s.state(), tower_zone()).is_none());
                assert!(s.unit(P1, 2).is_none());
                assert!(
                    !s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == SHEEP))
                );
            }

            #[test]
            fn r1194000_your_units_in_hand_and_on_the_field_have_flying_and_stack_and_the_opponent_s_do_not() {
                let s = aura_board(false);
                the_aura_reaches_your_units_alone(&s);
            }

            #[test]
            fn r1194000_a_unit_in_hand_is_listed_an_occupied_zone_and_an_empty_backrow_zone_and_lands_in_each() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [MENACE, VANILLA, FILLER],
                        "field": [{ "def": VANILLA, "lane": 1 }],
                        "backrow": [{ "def": TOWER, "lane": 2 }, { "def": SHEEPISH, "lane": 3, "faceUp": false }],
                        "library": DECK,
                        "mana": 8,
                    },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let menace = held(&s, P1, MENACE);
                let listed = zones_listed(&s, &menace);
                // Stack: the occupied unit zone; Flying: the empty backrow zone; the Tower's zone as its one
                // Unit; Flying and Stack: the backrow zone a face-down Trap holds.
                for zone in [(Row::Units, 1), (Row::Backrow, 1), (Row::Backrow, 2), (Row::Backrow, 3)] {
                    assert!(listed.contains(&zone), "{zone:?} not in {listed:?}");
                }
                let beneath = s.unit(P1, 1).expect("the Vanilla").id;
                s.play(&menace.id, json!({ "zone": 1 }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(menace.id.clone()));
                let pile = pile_at(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 1 }).expect("a pile");
                assert_eq!(pile.len(), 2);
                assert_eq!(pile[1].id, beneath);
                let vanilla = held(&s, P1, VANILLA);
                s.play(&vanilla.id, json!({ "zone": 3, "row": "backrow" }));
                assert_eq!(s.backrow(P1, 3).map(|card| card.id), Some(vanilla.id.clone()));
                assert!(is_standing_unit(s.state(), &s.backrow(P1, 3).expect("the Vanilla stands")));
            }

            #[test]
            fn r1194000_flying_plays_a_unit_into_an_empty_backrow_zone_and_the_aura_ends_with_the_tower() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [VANILLA, MAGIC_JAMMED, MENACE],
                        "backrow": [{ "def": TOWER, "lane": 2 }],
                        "library": DECK,
                        "mana": 8,
                    },
                    "p2": { "hand": [FILLER], "library": DECK },
                }));
                let tower = s.card(TOWER).id.clone();
                let vanilla = held(&s, P1, VANILLA);
                s.play(&vanilla.id, json!({ "zone": 4, "row": "backrow" }));
                assert_eq!(s.backrow(P1, 4).map(|card| card.id), Some(vanilla.id.clone()));
                s.play(MAGIC_JAMMED, json!({ "targets": [{ "pick": "instance", "instanceId": tower }] }));
                s.expect_in_zone(&tower, "graveyard");
                // A Unit placed stays where it stands (R1081000); the Units in hand lose both keywords.
                assert_eq!(s.backrow(P1, 4).map(|card| card.id), Some(vanilla.id.clone()));
                let menace = held(&s, P1, MENACE);
                assert_eq!(winged(&s, &menace), (false, false));
                assert!(!zones_listed(&s, &menace).iter().any(|(row, _)| *row == Row::Backrow));
                s.expect_refused(|s| s.play(&menace.id, json!({ "zone": 5, "row": "backrow" })));
            }

            #[test]
            fn a_unit_played_into_a_unit_zone_is_untouched_and_the_tower_may_still_take_a_unit_after_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MENACE, TOKENS, FILLER], "backrow": [{ "def": TOWER, "lane": 2 }], "library": DECK, "mana": 8 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(MENACE, json!({ "zone": 1 }));
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id).as_deref(), Some(MENACE));
                assert!(!any_event(&s, GameEventType::Fused));
                s.play(TOKENS, json!({ "zone": 2, "row": "backrow" }));
                assert!(any_event(&s, GameEventType::Fused));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn the_unit_becomes_radiant_as_it_lands_so_its_cry_runs_on_the_radiant_face() {
                crate::register_all();
                let (s, _tower, _rider) = tower_with(TOKENS, TowerOptions { radiant: true, ..TowerOptions::default() });
                assert!(index_of(&s, GameEventType::RadiantSet) < index_of(&s, GameEventType::CardResolved));
                // Me and Mr Token's Radiant Cry summons 3 Rush Tokens, its base Cry 1.
                let rush = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Summoned { def_id, .. } if def_id == RUSH))
                    .count();
                assert_eq!(rush, 3);
            }

            #[test]
            fn r469_it_is_fused_in_on_its_radiant_face() {
                crate::register_all();
                let (s, tower, _rider) = tower_with(TOE_CRACKER, TowerOptions { radiant: true, ..TowerOptions::default() });
                // Cloaked Toe Cracker's Radiant face adds "After you play a Trap, gain 1 mana", on both fused faces.
                assert!(text_of(&s, &tower).contains("After you play a Trap"));
                assert!(def_of(Some(s.state()), &s.card(&tower).def_id).base.text.contains("After you play a Trap"));
            }

            #[test]
            fn r1194000_the_radiant_face_s_aura_is_the_same() {
                let mut s = aura_board(true);
                the_aura_reaches_your_units_alone(&s);
                // Flying: into an empty backrow zone, standing there, and not made Radiant (it is not on the Tower).
                let menace = held(&s, P1, MENACE);
                s.play(&menace.id, json!({ "zone": 4, "row": "backrow" }));
                assert_eq!(s.backrow(P1, 4).map(|card| card.id), Some(menace.id.clone()));
                assert!(!s.card(&menace.id).radiant);
            }

            #[test]
            fn a_unit_played_into_a_unit_zone_is_not_made_radiant() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [MENACE, FILLER], "backrow": [{ "def": TOWER, "radiant": true }], "mana": 8 },
                    "p2": { "hand": [FILLER] },
                }));
                s.play(MENACE, json!({ "zone": 1 }));
                assert!(!s.card(MENACE).radiant);
            }
        }
    }
}
