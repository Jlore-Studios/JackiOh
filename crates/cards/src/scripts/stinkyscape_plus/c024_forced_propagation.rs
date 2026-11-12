//! SPEC §8.12 S+ #24 Forced Propagation, a card of the Stinkyscape+ set (R1420, R3535000). (1)
//! Spell, Epic, Embiggened (3).
//!   Base:    "Choose a non-fused card in your hand or field. Shuffle {copy} copies of it into your deck.
//!            It keeps no cost changes or buffs. Embiggened (3): Shuffle {copy2} copies of it instead."
//!   Radiant: the same, "then buff them".
//!
//! A declared target: one of your own cards in your hand, a Unit or a backrow card (R81, R3557001). A
//! fused card (a definition made by a Fuse, R179) is no card to copy and the Spell does nothing with it.
//! The copies are fresh cards of the chosen card's definition (`shuffle_into`, R57's cost and buff
//! riders left out), `copy` of them, `copy2` when Embiggened (`ctx.embiggened`), each on the face the
//! chosen card wears. The Radiant face marks the new cards by the cast and then Buffs each of them once,
//! the Buffs staying silent in the deck (R440, R3557001).

use std::sync::Arc;

use jackioh_engine::effects::{shuffle_into, upgrade};
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-024";

/// The key the copies of one cast carry in their memory (R3557001), so the Radiant sweep Buffs them
/// alone and no copy an earlier cast made.
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

/// This cast's mark: the casting card's id and the turn, so two casts never share one.
fn cast_mark(ctx: &EffectContext<'_>) -> String {
    let id = ctx.self_.as_ref().map_or("", |card| card.id.as_str());
    format!("{id}@{}", ctx.state.turn)
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

// S+ #24 Forced Propagation — SPEC §8.12 row 24 (R3557001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::{ID, MARK_KEY};
    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const SEED: &str = "r3557001";
    const FILLER: &str = "core-005";
    const BODY: &str = "core-019"; // Midrange Menace
    const SPELL: &str = "core-088"; // Twisting Nether: a Spell, never Buffed.

    fn pick(id: &str) -> Value {
        json!({ "targets": [{ "pick": "instance", "instanceId": id }] })
    }

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

    #[test]
    fn r3557001_ten_fresh_copies_of_a_field_card_shuffle_into_the_deck() {
        crate::register_all();
        let mut s = board(false);
        let body = s.unit_at(P1, 1).id;
        s.card_mut(&body).cost_mod = 2;
        s.play(ID, pick(&body));
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
        s.play(ID, pick(&spell));
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
        s.play(ID, pick(&body));
        let made = copies(&s, BODY);
        assert_eq!(made.len(), 10);
        let buffed = s.events().iter().filter(|event| event.event_type().as_str() == "upgraded").count();
        assert_eq!(buffed, 10, "one Buff for each copy");
        assert_eq!(s.state().players[P1].library.iter().filter(|card| card.def_id == FILLER).count(), 3);
        let original = s.card(&body);
        assert_eq!(original.buffs, AttackHealth { attack: 0, health: 0 }, "the chosen card is not Buffed");
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
        s.play(ID, pick(&fused.id));
        assert_eq!(s.state().players[P1].library.len(), before, "nothing was copied");
        s.expect_in_zone(&fused.id, "hand");
    }
}
