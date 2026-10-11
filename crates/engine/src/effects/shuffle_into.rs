//! Shuffle into a library at a uniformly random position, stopping at the library cap (§6.3, R80).
//!
//! Port of `packages/engine/src/effects/shuffleInto.ts`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::draw::{ShuffleInOutcome, shuffle_into_library};
use crate::enchantments::united_enchantments;
use crate::own_library::hide_from_owner;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut, new_instance};
use crate::tuning::copy_tuning;
use crate::wire::Zone;

use super::targets::{PlayerSpec, player_of};

/// R57 as patch v0.2.0 extends it (B3.4 rule 4, R443): a copy shuffled into a library carries its
/// source's `tuning` and enchantments beside the radiant flag — never its Brittle count, which a copy
/// never inherits — and its `chinese` flag (ME-CN, R1300). `source` is the card copied, when it still
/// exists and is of the copy's definition.
fn carry_from(copy: &mut CardInstance, source: Option<&CardInstance>) {
    let Some(source) = source else {
        return;
    };
    if source.def_id != copy.def_id {
        return;
    }
    copy.chinese = source.chinese;
    // MD-B15, R923: a copy keeps the granted tags of the card copied.
    copy.granted_tags = source.granted_tags.clone();
    if let Some(tuning) = copy_tuning(source.tuning.as_ref()) {
        copy.tuning = Some(tuning);
    }
    if let Some(enchantments) = united_enchantments(std::slice::from_ref(source)) {
        copy.enchantments = Some(enchantments);
    }
}

fn source_of<'a>(state: &'a GameState, id: Option<&str>) -> Option<&'a CardInstance> {
    find_instance(state, id?)
}

/// `shuffleInto`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShuffleIntoArgs {
    pub def_id: String,
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_of: Option<String>,
    /// R1246: written on each new card's `memory` (§10.1), so a card shuffled in remembers what made it
    /// (Meditative #96's Journey Complete, `memory.journey`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<IndexMap<String, Value>>,
    /// ME-DECK-BOTTOM (Meditative #49): put the fresh card at this end of the library instead of
    /// a random position. Absent is the shuffle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<crate::ownership::LibraryEnd>,
    /// R1106 (MD-D10, Meditative #54 Money Machine): the card goes in unseen, so its owner's list
    /// counts it as unknown until it leaves (R312's `hide_from_owner`), not with what it is (R311).
    /// Absent is the open shuffle-in every other card makes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unseen: Option<bool>,
}

/// Shuffle fresh copies of a definition into a library (CN-Viral Injection's CN-Virus, Unstable Clone
/// Machine's copies). `copy_of` is the instance the copies are copies of, when they copy a card rather
/// than make one the text names: a copy a full library refuses is judged by that card (R316), which
/// may be a Trap its controller has just set face-down (#33).
pub fn shuffle_into(args: ShuffleIntoArgs) -> Effect {
    Effect::new("shuffleInto", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        for _ in 0..args.count.max(0) {
            let mut card = new_instance(&mut *ctx.state, &args.def_id, player, Zone::Library { player });
            if args.radiant == Some(true) {
                card.radiant = true;
            }
            let source = source_of(ctx.state, args.copy_of.as_deref()).cloned();
            carry_from(&mut card, source.as_ref());
            if let Some(memory) = &args.memory {
                card.memory.extend(memory.clone());
            }
            let outcome = match args.position {
                None => shuffle_into_library(ctx, &mut card, false, args.copy_of.as_deref()),
                Some(end) => {
                    crate::draw::put_into_library(ctx, &mut card, false, args.copy_of.as_deref(), end)
                }
            };
            // R1106: an unseen card's record is cleared once it lies in the library.
            if args.unseen == Some(true)
                && outcome == ShuffleInOutcome::Library
                && let Some(live) = find_instance_mut(ctx.state, &card.id)
            {
                hide_from_owner(live);
            }
        }
    })
}

/// `shuffleCopiesOfSelf`'s argument.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShuffleCopiesOfSelfArgs {
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// Shuffle copies of the card that is resolving (CN-Virus's own copies).
pub fn shuffle_copies_of_self(args: ShuffleCopiesOfSelfArgs) -> Effect {
    Effect::new("shuffleCopiesOfSelf", move |ctx| {
        // TS read the live `ctx.self`: the card as it stands now.
        let Some(this) = ctx.live_self().cloned() else {
            return;
        };
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        for _ in 0..args.count.max(0) {
            let mut card = new_instance(&mut *ctx.state, &this.def_id, player, Zone::Library { player });
            card.radiant = this.radiant;
            carry_from(&mut card, Some(&this));
            shuffle_into_library(ctx, &mut card, false, Some(this.id.as_str()));
        }
    })
}
