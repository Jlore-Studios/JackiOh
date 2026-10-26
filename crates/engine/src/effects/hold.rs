//! Hold a permanent beneath a card until that card leaves the field (R1850003): Stinkyscape #9
//! Ticketed's "Stack a permanent under this. Death: return that permanent to its controller", and #135
//! Sacked's "Stuff it in a 0/4 Sack", whose Sack (#135.1) summons the Unit at its Death.
//!
//! The verb takes the card off the field and keeps it in `state.held` beside the card that holds it,
//! its stay ended there (R78's reset, R174: it has left the field, so every effect aimed at it ends);
//! `card_on_card::release_held` summons it again at the state check after its holder leaves the field,
//! any way it leaves. The holder is the card running the script, or one this verb summons for it (the
//! Sack). A face-down card is never held, so nothing hidden goes beneath a card, and a unit token
//! ceases to exist as it leaves the field (R11), so there is nothing to hold.

use serde::{Deserialize, Serialize};

use crate::effects::summon::{SummonPlacement, summon_fresh};
use crate::effects::targets::{PlayerSpec, TargetSpec, instance_by_id_or_spec, player_of};
use crate::script::Effect;
use crate::state::{CardInstance, HeldCard, find_instance};
use crate::wire::GameEvent;

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
/// card, or beneath a card `holder` summons for it, until that card leaves the field. It leaves as an
/// exiled card does — no Death, R78's reset, its Stack pile's next card resuming — and `held` reports
/// it, after the holder's `summoned` when the holder is new. Nothing happens to a card off the field,
/// a face-down one, or the running card itself, nor when the running card holds and is not on the
/// field. A new holder that finds no zone leaves the card held by nothing, so the next state check
/// summons it back for whom it returns to.
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
        });
    })
}
