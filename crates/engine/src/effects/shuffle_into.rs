//! Shuffle into a library at a uniformly random position, stopping at the library cap (§6.3, R80).
//!
//! Port of `packages/engine/src/effects/shuffleInto.ts`.

use serde::{Deserialize, Serialize};

use crate::draw::shuffle_into_library;
use crate::enchantments::united_enchantments;
use crate::script::Effect;
use crate::state::{CardInstance, GameState, find_instance, new_instance};
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
            shuffle_into_library(ctx, &mut card, false, args.copy_of.as_deref());
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
