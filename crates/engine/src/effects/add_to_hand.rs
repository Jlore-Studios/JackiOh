//! Add to hand: creates a card or moves one into a hand; a full hand burns it (§2.4, R4).
//!
//! §6.3's Add to hand row is one verb with two halves — "Put a card into a hand | Creates **or
//! moves** the card" — so there is no separate `moveToHand`. `add_to_hand({ defId })` creates a fresh
//! instance; `add_to_hand({ instance })` moves a card that already exists (#51 KY's Private Tutor's
//! revealed library card, #72 Reminisce's chosen graveyard card). Both routes end in `crate::draw`'s
//! `add_to_hand`, so §2.4's hand cap and R4's burn apply once, in one place, and R11's unit-token card
//! ceases to exist instead of reaching the graveyard.
//!
//! Port of `packages/engine/src/effects/addToHand.ts`.

use serde::{Deserialize, Serialize};

use super::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::catalog::{CatalogQueryArgs, excluding_def_id, pick_generated, query};
use crate::config::HAND_CAP;
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance_mut, new_instance};
use crate::wire::{Keyword, Zone, ZoneName};

/// The riders a card reaches a hand with. R65 starts the cost from `costOverride` IN PLACE OF the
/// printed cost, so a discount that must stack with other modifiers is a `costMod`, never a
/// `costOverride` — which is why both exist. #54 Straaza's "they cost 1" replaces the price outright
/// (`costOverride: 1`), while #7 Jewelosco Scarab's and #39 Recycling Initiative's "costs 1 less"
/// must survive alongside the next discount and the embiggen price it did not choose (`costMod: -1`).
/// R78 keeps all three of `costMod`, `costOverride` and `radiant` in every zone.
#[derive(Clone, Debug, Default, PartialEq)]
struct HandRiders {
    player: Option<PlayerSpec>,
    radiant: Option<bool>,
    cost_override: Option<i32>,
    cost_mod: Option<i32>,
    /// R637: the card is Temporary (a granted keyword) while it is in a hand.
    temporary: Option<bool>,
}

/// `costMod` ADDS (R65 sums it); `costOverride` and `radiant` replace.
///
/// The two halves land at different moments. A card generated or made Radiant is Radiant wherever it
/// ends up (R74), so the flag goes on first. A cost rider ("it costs 1 less", "they cost 1") is a
/// price for the card in the hand, so it goes on only once the card has reached one: a full hand
/// burns it instead (§2.4, R4), and a burned card is an ordinary graveyard card that R78 would
/// otherwise have carry the rider into every later zone.
fn apply_radiant_rider(card: &mut CardInstance, riders: &HandRiders) {
    if riders.radiant == Some(true) {
        card.radiant = true;
    }
}

fn apply_cost_riders(card: &mut CardInstance, riders: &HandRiders) {
    if let Some(cost_override) = riders.cost_override {
        card.cost_override = Some(cost_override);
    }
    if let Some(cost_mod) = riders.cost_mod {
        card.cost_mod += cost_mod;
    }
    // R637: a keyword for the card's stay in the hand, so it goes on with the prices, once it is there.
    if riders.temporary == Some(true)
        && !card
            .granted_keywords
            .iter()
            .any(|keyword| matches!(keyword, Keyword::Temporary))
    {
        card.granted_keywords.push(Keyword::Temporary);
    }
}

/// §2.4's pipeline, with the riders applied around it as `apply_radiant_rider` explains.
///
/// TS wrote the riders through the live card object. Here `card` is the card as found (or the fresh
/// instance), so the radiant rider goes on it before it moves, and on the card in its pile too when
/// it already is in one; the cost riders go on the card where the move put it. Whether it reaches the
/// hand is `crate::draw::add_to_hand`'s own test, read as it reads it: the owner's hand below
/// HAND_CAP (§2.4, R4) — else it is burned and takes no price.
fn put_in_hand_with(ctx: &mut EffectContext<'_>, mut card: CardInstance, riders: &HandRiders) {
    apply_radiant_rider(&mut card, riders);
    if let Some(live) = find_instance_mut(&mut *ctx.state, &card.id) {
        apply_radiant_rider(live, riders);
    }
    let reaches_hand = (ctx.state.players[card.owner].hand.len() as i32) < HAND_CAP;
    let _ = crate::draw::add_to_hand(ctx, &mut card);
    if reaches_hand && let Some(live) = find_instance_mut(&mut *ctx.state, &card.id) {
        apply_cost_riders(live, riders);
    }
}

/// The one creation path: a fresh card of `defId` with these riders, into that player's hand through
/// §2.4's pipeline. `add_to_hand` and `add_random_from_catalog` differ only in where the def comes
/// from, so they share this rather than each growing their own cap and burn handling.
fn create_in_hand(ctx: &mut EffectContext<'_>, def_id: &str, riders: &HandRiders) {
    let player = player_of(ctx, riders.player.unwrap_or(PlayerSpec::SelfSide));
    let card = new_instance(&mut *ctx.state, def_id, player, Zone::Hand { player });
    put_in_hand_with(ctx, card, riders);
}

/// `add_to_hand`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddToHandArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub def_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<TargetSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
}

impl AddToHandArgs {
    fn riders(&self) -> HandRiders {
        HandRiders {
            player: self.player,
            radiant: self.radiant,
            cost_override: self.cost_override,
            cost_mod: self.cost_mod,
            temporary: self.temporary,
        }
    }
}

/// §6.3 Add to hand. With `defId`, create a fresh card of that definition, keeping the radiant flag
/// when asked (R57). With `instance`, MOVE the card that `TargetSpec` names — `{ of: "chosen" }` for
/// a card a prompt just picked (#51's revealed library card, #72's graveyard card) — so its identity,
/// its radiant flag and its cost riders travel with it (R78). Nothing happens when the spec names no
/// card or names one that is already in a hand: the effect fizzles and the card still resolves (§6.3).
///
/// A moved card goes to its OWNER's hand, never `player`'s: §3.2 rules that "off the field a card
/// always goes to its owner's hand, library, graveyard or exile", which is what `crate::draw`'s
/// pipeline does. `player` therefore names the hand only on the creation path.
///
/// `bounce` is the neighbouring verb and deliberately not this one: it returns a card from the FIELD
/// and resets the instance (§6.3, R78), which would throw away exactly the riders this verb keeps.
pub fn add_to_hand(args: AddToHandArgs) -> Effect {
    Effect::new("addToHand", move |ctx| {
        let riders = args.riders();
        if let Some(spec) = &args.instance {
            let Some(card) = instance_of(ctx, spec) else {
                return;
            };
            if card.zone.z() == ZoneName::Hand {
                return;
            }
            put_in_hand_with(ctx, card, &riders);
            return;
        }
        let Some(def_id) = &args.def_id else {
            return;
        };
        create_in_hand(ctx, def_id, &riders);
    })
}

/// §5.1: "a random pool never offers the card that generated it". The same computation as
/// `discover_from_catalog` in `choose.rs`, and the reason both read it off `ctx.self_` rather than
/// trusting the caller: a Spell resolving its own Cry is still findable (§10.5 parks it in
/// `resolving`), so the generating def is known without the card file having to name its own id (R387).
fn pool_query(ctx: &EffectContext<'_>, args: Option<&CatalogQueryArgs>) -> CatalogQueryArgs {
    let none = CatalogQueryArgs::default();
    let args = args.unwrap_or(&none);
    let generating = ctx
        .self_
        .as_ref()
        .map(|card| card.def_id.as_str())
        .or(ctx.def_id.as_deref());
    excluding_def_id(Some(&*ctx.state), args, generating)
}

/// `add_random_from_catalog`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddRandomFromCatalogArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<CatalogQueryArgs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
}

impl AddRandomFromCatalogArgs {
    fn riders(&self) -> HandRiders {
        HandRiders {
            player: self.player,
            radiant: self.radiant,
            cost_override: self.cost_override,
            cost_mod: self.cost_mod,
            temporary: self.temporary,
        }
    }
}

/// §6.3 Add to hand, N times, from a §10.7 catalog pool: #54 Straaza's "add 2 random Units costing 3
/// or 4", #57 Conjure KY's "add 3 random KY cards", #59 Unbiased Immigration's "add a random card".
/// It is an add-to-hand, not a Discover, so it shares `create_in_hand` with `add_to_hand` above and
/// the hand cap burns the overflow (§2.4, R4).
///
/// REPEATS ARE ALLOWED, which is why this draws `count` times with `ctx.rng.pick` over the pool
/// instead of shuffling and slicing: R60 rules that "cards generated from the catalog may repeat
/// unless the card says 'different'", and #57's engine cell says "repeats allowed" outright.
/// `discover_from_catalog` deliberately does the opposite — §6.3's Discover row draws its options
/// "without replacement", so it shuffles and slices and its three options are always different.
///
/// An empty pool fizzles and the card still resolves (§6.3).
pub fn add_random_from_catalog(args: AddRandomFromCatalogArgs) -> Effect {
    Effect::new("addRandomFromCatalog", move |ctx| {
        let pool = query(&pool_query(ctx, args.query.as_ref()));
        if pool.is_empty() {
            return;
        }

        let riders = args.riders();
        let count = args.count.unwrap_or(1).max(0);
        for _ in 0..count {
            // R673: a card generated into a hand may be Glitch (`catalog::GlitchOdds` is the state).
            let Some(def) = pick_generated(&mut *ctx.sink.rng, &pool, Some(&*ctx.sink.state)) else {
                return;
            };
            let def_id = def.id.clone();
            create_in_hand(ctx, &def_id, &riders);
        }
    })
}

/// `add_random_from_graveyard`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddRandomFromGraveyardArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exile: Option<bool>,
}

/// Move random cards from your graveyard to your hand (#37 Gravedigger draws one; C #34 Ancient
/// Acquisition draws its number, from the graveyard or, on its Radiant face, the graveyard and
/// exile together — balance patch 1 made those returns random, R684). Each draw picks uniformly
/// from the cards still in the piles through the match rng, so fewer cards than asked ends it. The
/// hand cap burns the overflow (§2.4, R4).
pub fn add_random_from_graveyard(args: AddRandomFromGraveyardArgs) -> Effect {
    Effect::new("addRandomFromGraveyard", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let count = args.count.unwrap_or(1).max(0);
        for _ in 0..count {
            let side = &ctx.state.players[player];
            let mut pool: Vec<CardInstance> = side.graveyard.clone();
            if args.exile == Some(true) {
                pool.extend(side.exile.iter().cloned());
            }
            if pool.is_empty() {
                return;
            }
            let Some(mut card) = ctx.rng.pick(&pool).cloned() else {
                return;
            };
            let _ = crate::draw::add_to_hand(ctx, &mut card);
        }
    })
}
