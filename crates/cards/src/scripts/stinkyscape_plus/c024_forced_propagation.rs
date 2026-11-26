//! SPEC §8.12 S+ #24 Forced Propagation, a card of the Stinkyscape+ set (R1420, R3535000). (1)
//! Spell, Epic, Embiggened (3).
//!   Base:    "Choose a non-fused card in your hand or field. Shuffle {copy} copies of it into your deck.
//!            It keeps no cost changes or buffs. Embiggened (+2): Shuffle {copy2} copies of it instead."
//!   Radiant: the same, "then buff them".
//!
//! A declared target: one of your own cards in your hand, a Unit or a backrow card (R81, R3557001). A
//! fused card (a definition made by a Fuse, R179) is no card to copy and the Spell does nothing with it.
//! The copies are fresh cards of the chosen card's definition (`shuffle_into`, R57's cost and buff
//! riders left out), `copy` of them, `copy2` when Embiggened (`ctx.embiggened`), each on the face the
//! chosen card wears. The Radiant face marks the new cards by the resolution and then Buffs each of
//! them once, the Buffs staying silent in the deck (R440, R3557001).

use std::sync::Arc;

use jackioh_engine::effects::{shuffle_into, upgrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-024";

/// The key the copies of one resolution carry in their memory (R3557001), so the Radiant sweep
/// Buffs them alone and no copy another resolution made.
const MARK_KEY: &str = "propagation";

/// "Buff them": one Buff each.
const BUFFS: i32 = 1;

/// The card the target names, when it is a card to copy (never a fused one).
fn chosen_card(ctx: &EffectContext<'_>) -> Option<CardInstance> {
    let chosen = match ctx.targets.first() {
        Some(Selection::Instance { instance_id }) => find_instance(&*ctx.state, instance_id)?.clone(),
        _ => return None,
    };
    if jackioh_engine::subsystems::fused_ingredients(&*ctx.state, &chosen.def_id).is_some() {
        return None;
    }
    Some(chosen)
}

/// This resolution's mark: the casting card's id, the turn, and where this run's events begin
/// (R136), so two resolutions never share one (R3557001). An Echo repeat is the same card on the
/// same turn, but a later run with a later window.
fn cast_mark(ctx: &EffectContext<'_>) -> String {
    let id = ctx.self_.as_ref().map_or("", |card| card.id.as_str());
    format!("{id}@{turn}@{from}", turn = ctx.state.turn, from = ctx.events_from)
}

fn propagation(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "ally", "of": ["unit", "backrow", "hand"] }),
        )],
        cry: Some(hook(move |ctx| {
            let Some(chosen) = chosen_card(ctx) else {
                return vec![];
            };
            let copies = if ctx.embiggened { param(ctx, "copy2") } else { param(ctx, "copy") };
            let mark = cast_mark(ctx);
            let mut args = json!({ "defId": chosen.def_id, "count": copies, "radiant": chosen.radiant });
            let mut effects = Vec::new();
            if radiant {
                args["memory"] = json!({ MARK_KEY: mark.clone() });
            }
            effects.push(shuffle_into(json_as(args)));
            if radiant {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |now: &mut EffectContext<'_>| {
                        let player = now.controller;
                        zone_cards(now.state, player, OffFieldZone::Library)
                            .into_iter()
                            .filter(|card| card.memory.get(MARK_KEY).and_then(Value::as_str) == Some(mark.as_str()))
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        upgrade(json_as(json!({ "instanceId": instance_id, "times": BUFFS })))
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: propagation(false),
        radiant: propagation(true),
    }
}

/// R767000: the mark through v0.6.13c: the casting card's id and the turn, so two casts on
/// different turns never shared one, but an Echo repeat on the same turn did.
fn cast_mark_v0_6_13c(ctx: &EffectContext<'_>) -> String {
    let id = ctx.self_.as_ref().map_or("", |card| card.id.as_str());
    format!("{id}@{turn}", turn = ctx.state.turn)
}

fn propagation_v0_6_13c(radiant: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(
            1,
            1,
            json!({ "side": "ally", "of": ["unit", "backrow", "hand"] }),
        )],
        cry: Some(hook(move |ctx| {
            let Some(chosen) = chosen_card(ctx) else {
                return vec![];
            };
            let copies = if ctx.embiggened { param(ctx, "copy2") } else { param(ctx, "copy") };
            let mark = cast_mark_v0_6_13c(ctx);
            let mut args = json!({ "defId": chosen.def_id, "count": copies, "radiant": chosen.radiant });
            let mut effects = Vec::new();
            if radiant {
                args["memory"] = json!({ MARK_KEY: mark.clone() });
            }
            effects.push(shuffle_into(json_as(args)));
            if radiant {
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(move |now: &mut EffectContext<'_>| {
                        let player = now.controller;
                        zone_cards(now.state, player, OffFieldZone::Library)
                            .into_iter()
                            .filter(|card| card.memory.get(MARK_KEY).and_then(Value::as_str) == Some(mark.as_str()))
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(|instance_id: &str| {
                        upgrade(json_as(json!({ "instanceId": instance_id, "times": BUFFS })))
                    }),
                }));
            }
            effects
        })),
        ..Script::default()
    }
}

/// R767000: the scripts through v0.6.13c; `KEPT_SCRIPTS` folds those replays under them.
pub fn script_v0_6_13c() -> CardScripts {
    CardScripts {
        base: propagation_v0_6_13c(false),
        radiant: propagation_v0_6_13c(true),
    }
}

// S+ #24 Forced Propagation — SPEC §8.12 row 24 (R3557001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, MARK_KEY, script_v0_6_13c};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const SEED: &str = "r3557001";
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // Midrange Menace
    const SPELL: &str = "core-088"; // Twisting Nether: a Spell, never Buffed.
    const PROJECTOR: &str = "stinkyscapeplus-062"; // Overdrive Projector: its Spell Echo aura

    fn pick_embiggened(id: &str) -> Value {
        json!({ "embiggen": true, "targets": [{ "pick": "instance", "instanceId": id }] })
    }

    /// A fused card (R179) of p1's in hand, made as the engine's Fuse makes one.
    fn fused_in_hand(s: &mut Scenario, ingredients: &[&str]) -> CardInstance {
        use jackioh_engine::subsystems::{self, FuseArgs};
        let mut events: Vec<GameEvent> = vec![];
        let mut rng = Rng::new(&s.state().seed, s.state().rng_cursor);
        let made = {
            let state = s.state_mut();
            let parts: Vec<CardInstance> = ingredients
                .iter()
                .map(|def_id| new_instance(&mut *state, def_id, P1, Zone::Gone { player: P1 }))
                .collect();
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            subsystems::fuse(&mut sink, FuseArgs { ingredients: parts, to_hand: Some(P1), ..FuseArgs::default() })
        };
        s.state_mut().rng_cursor = rng.cursor();
        made.expect("the fused card")
    }

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": SEED,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, SPELL, FILLER],
                "field": [BODY],
                "library": filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn copies(s: &Scenario, def_id: &str) -> Vec<CardInstance> {
        s.state().players[P1].library.iter().filter(|card| card.def_id == def_id).cloned().collect()
    }

    /// p1 casts Radiant Forced Propagation with an awake Overdrive Projector behind it, so the
    /// Spell resolves twice (Echo +1, R3556001). The scenario-placed Projector carries no slumber,
    /// so it is awake from the first turn.
    fn echo_board() -> Scenario {
        scenario(json!({
            "seed": "r3557001-echo",
            "p1": {
                "hand": [{ "def": ID, "radiant": true }, FILLER],
                "field": [BODY],
                "backrow": [{ "def": PROJECTOR, "faceUp": true }],
                "library": filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// R3557001: an Echo repeat of the same card in the same turn Buffs only the copies it made.
    #[test]
    fn r3557001_an_echo_repeat_buffs_only_the_copies_it_made() {
        let mut s = echo_board();
        let body = s.unit_at(P1, 1).id;
        s.play(ID, pick::targets(pick::id(&body)));
        while s.state().pending.is_some() {
            s.answer(pick::id(&body));
        }
        let made = copies(&s, BODY);
        assert_eq!(made.len(), 20, "Echo +1: two resolutions of ten copies");
        let mut buffed: Vec<String> = s
            .events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        buffed.sort();
        assert_eq!(buffed.len(), 20, "each copy is Buffed once, by the resolution that made it");
        let mut ids: Vec<String> = made.iter().map(|card| card.id.clone()).collect();
        ids.sort();
        assert_eq!(buffed, ids, "each copy exactly once");
    }

    /// R767000: the script kept for v0.6.13c shares one mark across Echo repeats, so the repeat
    /// re-Buffs the first ten. Pins the old era's behavior for replays.
    #[test]
    fn r767000_the_script_kept_for_v0_6_13c_shares_one_mark_across_echo_repeats() {
        crate::under_kept(ID, script_v0_6_13c(), || {
            let mut s = echo_board();
            let body = s.unit_at(P1, 1).id;
            s.play(ID, pick::targets(pick::id(&body)));
            while s.state().pending.is_some() {
                s.answer(pick::id(&body));
            }
            assert_eq!(copies(&s, BODY).len(), 20, "Echo +1: two resolutions of ten copies");
            let buffed = s.events().iter().filter(|event| event.event_type().as_str() == "upgraded").count();
            assert_eq!(buffed, 30, "the old mark is shared, so the repeat re-Buffs the first ten");
        });
    }

    #[test]
    fn r3557001_ten_fresh_copies_of_a_field_card_shuffle_into_the_deck() {
        crate::register_all();
        let mut s = board(false);
        let body = s.unit_at(P1, 1).id;
        s.card_mut(&body).cost_mod = 2;
        s.play(ID, pick::targets(pick::id(&body)));
        let made = copies(&s, BODY);
        assert_eq!(made.len(), 10);
        assert!(made.iter().all(|card| card.cost_mod == 0 && card.buffs == AttackHealth { attack: 0, health: 0 }));
        assert!(made.iter().all(|card| card.memory.get(MARK_KEY).is_none()));
        s.expect_in_zone(&body, "field");
    }

    #[test]
    fn r3557001_a_card_of_your_hand_is_copied_too() {
        crate::register_all();
        let mut s = board(false);
        let spell = s.card(SPELL).id.clone();
        s.play(ID, pick::targets(pick::id(&spell)));
        assert_eq!(copies(&s, SPELL).len(), 10);
        s.expect_in_zone(&spell, "hand");
    }

    #[test]
    fn r3557001_embiggened_it_shuffles_forty() {
        crate::register_all();
        let mut s = board(false);
        let body = s.unit_at(P1, 1).id;
        s.play(ID, pick_embiggened(&body));
        assert_eq!(copies(&s, BODY).len(), 40);
    }

    #[test]
    fn r3557001_the_radiant_face_buffs_each_copy_once_and_no_other_card() {
        crate::register_all();
        let mut s = board(true);
        let body = s.unit_at(P1, 1).id;
        s.play(ID, pick::targets(pick::id(&body)));
        let made = copies(&s, BODY);
        assert_eq!(made.len(), 10);
        let buffed = s.events().iter().filter(|event| event.event_type().as_str() == "upgraded").count();
        assert_eq!(buffed, 10, "one Buff for each copy");
        assert_eq!(s.state().players[P1].library.iter().filter(|card| card.def_id == FILLER).count(), 3);
        let original = s.card(&body);
        assert_eq!(original.buffs, AttackHealth { attack: 0, health: 0 }, "the chosen card is not Buffed");
    }

    #[test]
    fn r3557001_a_second_radiant_cast_buffs_its_own_copies_and_none_the_first_made() {
        let mut s = scenario(json!({
            "seed": SEED,
            "p1": {
                "hand": [{ "def": ID, "radiant": true }, { "def": ID, "radiant": true }, FILLER],
                "field": [BODY],
                "library": filler(3),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let body = s.unit_at(P1, 1).id;
        s.play(ID, pick::targets(pick::id(&body)));
        s.play(ID, pick::targets(pick::id(&body)));
        assert_eq!(copies(&s, BODY).len(), 20);
        let buffed = s.last_events().iter().filter(|event| event.event_type().as_str() == "upgraded").count();
        assert_eq!(buffed, 10, "the second cast Buffs its ten copies alone");
    }

    #[test]
    fn r3557001_a_fused_card_is_copied_into_nothing() {
        let mut s = scenario(json!({
            "seed": SEED,
            "p1": { "hand": [ID, FILLER], "library": filler(3), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        let fused = fused_in_hand(&mut s, &[BODY, SPELL]);
        let before = s.state().players[P1].library.len();
        s.play(ID, pick::targets(pick::id(&fused.id)));
        assert_eq!(s.state().players[P1].library.len(), before, "nothing was copied");
        s.expect_in_zone(&fused.id, "hand");
    }
}
