//! Meditative #11 Double Header (SPEC §8.8 row 11; docs/meditative-set.md M6 #11; R826).
//! (4) Field Spell, Legendary.
//!   Base:    "The first card you play each turn adds a copy of it to your hand. The copy isn't Radiant
//!            and costs ({setCost})." (setCost 0)
//!   Radiant: "The first card you play each turn adds a Radiant copy of it to your hand. It costs
//!            ({setCost})."
//!
//! A trigger on its controller's `cardResolved` (§10.5 step 7, as Core #33 Unstable Clone Machine's,
//! R17) for the card at the head of their plays this turn (R213: "first" is the turn's log, not a
//! flag), played that once: a card played again later in the turn is the turn's first card no more.
//! The copy is a fresh card of the definition the event names (R71), so a fused card copies its
//! transient definition (R77) and a card that has ceased to exist since (Core #41 Sheepish) still
//! copies. A cast is a play (R70); a countered card never reached the log (R448). Double Header does
//! not answer its own play (R119), so the turn it lands copies nothing (R826).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-011";

/// Whether `instance_id` is the first card `player` played this turn, and played that once (R826).
fn first_played(state: &GameState, player: PlayerId, instance_id: &str) -> bool {
    let played = played_ids_this_turn(state, player);
    played.first().is_some_and(|first| first == instance_id)
        && played.iter().filter(|id| *id == instance_id).count() == 1
}

/// `radiant`: the Radiant face's copy is Radiant; the base face's never is.
fn first_card_copied(radiant: bool) -> TriggerDef {
    TriggerDef::new(
        if radiant {
            "m11r-first-card-copied"
        } else {
            "m11-first-card-copied"
        },
        &[GameEventType::CardResolved],
        move |ctx, event| {
            let GameEvent::CardResolved {
                player,
                instance_id,
                def_id,
                ..
            } = event
            else {
                return vec![];
            };
            let Some(this) = ctx.self_.as_ref() else {
                return vec![];
            };
            if *player != ctx.controller || *instance_id == this.id {
                return vec![];
            }
            if !first_played(&*ctx.state, *player, instance_id) {
                return vec![];
            }
            vec![add_to_hand(json_as(json!({
                "defId": def_id,
                "costOverride": param(&*ctx, "setCost"),
                "radiant": radiant,
            })))]
        },
    )
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            triggers: vec![first_card_copied(false)],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![first_card_copied(true)],
            ..Script::default()
        },
    }
}
