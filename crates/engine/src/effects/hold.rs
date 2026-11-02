//! Hold a permanent beneath a card until that card leaves the field (R1850003): Stinkyscape #9
//! Ticketed's "Stack a permanent under this. Death: return that permanent to its controller", and #135
//! Sacked's "Stuff it in a 0/4 Sack", whose Sack (#135.1) summons the Unit at its Death.
//!
//! A held card waits in `state.held` beside its holder, its stay ended (R78's reset, R174: every effect
//! aimed at it ends); `card_on_card::release_held` summons it again at the state check after its holder
//! leaves the field, any way it leaves. A face-down card is never held: nothing hidden goes beneath one.
//!
//! R1865000: Stinkyscape #112 Job Application's "send a card from your hand in as your resume" sets a
//! hand card aside the same way, and its Interview's reward gives it back.

use serde::{Deserialize, Serialize};

use crate::effects::summon::{SummonPlacement, summon_fresh};
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_by_id_or_spec, player_of, self_on_its_stay};
use crate::effects::tune::{TuneDirection, tune_once};
use crate::script::Effect;
use crate::state::{CardInstance, HeldCard, find_instance, find_instance_mut};
use crate::wire::{GameEvent, ZoneName};

/// Whom a held card is summoned for once its holder leaves: its controller as it was taken, or the
/// holder's controller.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum HoldRelease {
    #[default]
    ItsController,
    HoldersController,
}

/// A card this verb summons to hold the taken one (#135.1's Sack), for `player` (the running card's
/// controller by default), at R64's placement.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct NewHolder {
    pub def_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
}

/// `holdBeneath`'s argument: the permanent to take (the first chosen one by default), whom it returns
/// to, and the card that holds it when that is not the running card.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HoldArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<HoldRelease>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<NewHolder>,
}

/// R1850003: take the face-up permanent `target` names off the field and hold it beneath the running
/// card, or a card `holder` summons for it. It leaves as an exiled card does (no Death, R78's reset, its
/// pile's next card resuming); `held` reports it, after a new holder's `summoned`. A new holder that
/// finds no zone leaves the card held by nothing, so the next state check summons it back.
pub fn hold_beneath(args: HoldArgs) -> Effect {
    Effect::new("holdBeneath", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, None, args.target.as_ref()) else {
            return;
        };
        let Some(card) = find_instance(ctx.state, &card.id).cloned() else {
            return;
        };
        if !crate::zones::acts_on_field(ctx.state, &card) || crate::preview::is_face_down(ctx.state, &card) {
            return;
        }
        let me = ctx.self_.as_ref().map(|me| me.id.clone());
        if me.as_deref() == Some(card.id.as_str()) {
            return;
        }
        let holds_itself = args.holder.is_none();
        if holds_itself
            && !me
                .as_deref()
                .and_then(|id| find_instance(ctx.state, id))
                .is_some_and(|me| crate::zones::acts_on_field(ctx.state, me))
        {
            return;
        }
        let holder_player = match &args.holder {
            Some(holder) => player_of(ctx, holder.player.unwrap_or(PlayerSpec::SelfSide)),
            None => ctx.controller,
        };
        let to = match args.release.unwrap_or_default() {
            HoldRelease::ItsController => card.controller,
            HoldRelease::HoldersController => holder_player,
        };
        let token = crate::zones::is_unit_token(ctx.state, &card);
        let mut taken: CardInstance = card.clone();
        crate::zones::cease_to_exist(ctx.state, &mut taken);
        crate::zones::reset_instance(&mut taken);
        let holder_id = match &args.holder {
            None => me.unwrap_or_default(),
            Some(holder) => {
                let at = SummonPlacement {
                    source_id: me,
                    ..SummonPlacement::default()
                };
                summon_fresh(ctx, &holder.def_id, holder_player, &at)
                    .map(|placed| placed.id)
                    .unwrap_or_default()
            }
        };
        ctx.events.push(GameEvent::Held {
            player: card.controller,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            holder_id: holder_id.clone(),
        });
        // R11: a unit token ceased to exist as it left the field; nothing is left to hold.
        if token {
            return;
        }
        let holder_mark = crate::stays::exit_mark(ctx.state);
        ctx.state.held.get_or_insert_with(Vec::new).push(HeldCard {
            holder_id,
            holder_mark,
            to,
            card: taken,
            exile_on_release: None,
        });
    })
}

/// `setAsideFromHand`'s argument: the hand card to take (the first chosen one by default).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetAsideArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetSpec>,
}

/// R1865000: take the hand card `target` names out of its owner's hand, as it was (no reset), and hold
/// it beneath the running card, acting on the field on its run's stay, until `return_held_to_hand` or
/// the holder leaving the field, when the next state check exiles it. In no pile, so both seats read it.
pub fn set_aside_from_hand(args: SetAsideArgs) -> Effect {
    Effect::new("setAsideFromHand", move |ctx| {
        let Some(card) = instance_by_id_or_spec(ctx, None, args.target.as_ref()) else {
            return;
        };
        let Some(card) = find_instance(ctx.state, &card.id).cloned() else {
            return;
        };
        if card.zone.z() != ZoneName::Hand {
            return;
        }
        let Some(me) = self_on_its_stay(ctx).filter(|me| crate::zones::acts_on_field(ctx.state, me)) else {
            return;
        };
        let token = crate::zones::is_unit_token(ctx.state, &card);
        let mut taken: CardInstance = card.clone();
        crate::zones::cease_to_exist(ctx.state, &mut taken);
        ctx.events.push(GameEvent::Held {
            player: card.owner,
            instance_id: card.id.clone(),
            def_id: card.def_id.clone(),
            holder_id: me.id.clone(),
        });
        // R11: a unit token ceased to exist as it left the hand; nothing is left to hold.
        if token {
            return;
        }
        let holder_mark = crate::stays::exit_mark(ctx.state);
        ctx.state.held.get_or_insert_with(Vec::new).push(HeldCard {
            holder_id: me.id,
            holder_mark,
            to: card.owner,
            card: taken,
            exile_on_release: Some(true),
        });
    })
}

/// `returnHeldToHand`'s argument: the price the card comes back at (`costOverride`, none keeping its
/// own) and how many times it is Buffed in the hand.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ReturnHeldArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrades: Option<i32>,
}

/// R1865000: every card the running card holds on its stay, in the order taken, back to the hand of the
/// player it returns to. With `cost_override` its price is reset (R766) and set; in the hand it is
/// Buffed `upgrades` times (B3.4). A full hand burns it unbuffed (§2.4), the printed card as a burned
/// hand card is (R215). Nothing happens while the running card is off the field on its run's stay.
pub fn return_held_to_hand(args: ReturnHeldArgs) -> Effect {
    Effect::new("returnHeldToHand", move |ctx| {
        let Some(me) = self_on_its_stay(ctx).filter(|me| crate::zones::acts_on_field(ctx.state, me)) else {
            return;
        };
        let Some(held) = ctx.state.held.take() else {
            return;
        };
        let (mine, kept): (Vec<HeldCard>, Vec<HeldCard>) = held.into_iter().partition(|entry| {
            entry.holder_id == me.id
                && !crate::stays::left_field_after(ctx.state, entry.holder_mark, &entry.holder_id)
        });
        ctx.state.held = if kept.is_empty() { None } else { Some(kept) };
        for entry in mine {
            let mut card = entry.card;
            if let Some(cost) = args.cost_override {
                crate::zones::reset_price(&mut card);
                card.cost_override = Some(cost.max(0));
            }
            if crate::draw::add_to_hand(&mut ctx.sink, &mut card) != crate::draw::AddToHandOutcome::Hand {
                if let Some(burned) = find_instance_mut(ctx.state, &card.id) {
                    crate::zones::reset_instance(burned);
                }
                continue;
            }
            for _ in 0..args.upgrades.unwrap_or(0).max(0) {
                tune_once(ctx, &card, TuneDirection::Upgrade, true);
            }
        }
    })
}
