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

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::targets::{PlayerSpec, TargetSpec, instance_of, player_of};
use crate::catalog::{CatalogQueryArgs, excluding_def_id, pick_generated, query};
use crate::script::{Effect, EffectContext};
use crate::state::{CardInstance, find_instance, find_instance_mut, new_instance};
use crate::wire::{AttackHealth, Keyword, Tuning, Zone, ZoneName};

/// The riders a card reaches a hand with. R65 starts the cost from `costOverride` IN PLACE OF the
/// printed cost, so a discount that must stack with other modifiers is a `costMod`, never a
/// `costOverride` — which is why both exist. #54 Straaza's "they cost 1" replaces the price outright
/// (`costOverride: 1`), while #7 Jewelosco Scarab's and #39 Recycling Initiative's "costs 1 less"
/// must survive alongside the next discount and the embiggen price it did not choose (`costMod: -1`).
/// R78 keeps all three of `costMod`, `costOverride` and `radiant` in every zone, except that R766
/// takes the two prices off a card that reaches a graveyard or an exile pile.
#[derive(Clone, Debug, Default, PartialEq)]
struct HandRiders {
    player: Option<PlayerSpec>,
    radiant: Option<bool>,
    cost_override: Option<i32>,
    /// R1181003: the Embiggened version's own price (Meditative+ #23's "its Embiggened version costs (0)").
    embiggen_cost_override: Option<i32>,
    cost_mod: Option<i32>,
    /// R1704003: a raise by `cost_mod` stops at this cost, and leaves an X-cost card as it is.
    cost_mod_up_to: Option<i32>,
    /// R637: the card is Temporary (a granted keyword) while it is in a hand.
    temporary: Option<bool>,
    /// ME-CN, R1300: the card is a copy of a Chinese card, so it is Chinese too.
    chinese: Option<bool>,
    /// R57: a definition-remade copy may carry the source's layer-1 face without using `copyOf`
    /// (Classic #14 Shadowstep, which R1300 says is not Chinese).
    stats_override: Option<AttackHealth>,
    /// R175: the Bread Token's Armor X travels with its X/X face.
    armor_override: Option<i32>,
    /// R57: tuning is one of the instance riders a fresh copy keeps.
    tuning: Option<Tuning>,
    /// R1438: the card is given Lucky X (a granted keyword) as it reaches the hand.
    lucky: Option<i32>,
    /// ME-SECRET, R862: what the new card remembers from the start (Mind Games' link rides the
    /// Fortify Mind it hands the opponent).
    memory: Option<IndexMap<String, Value>>,
    /// R1848003: an X-cost card reaches the hand with its X set at this.
    fixed_x: Option<i32>,
    /// R1878003: the card is given these keywords as it reaches the hand.
    keywords: Option<Vec<Keyword>>,
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
    // ME-SECRET, R862: the link rides the instance from the start, so the card remembers its secret
    // wherever it lands — even burned into the graveyard, where a full hand puts it (§2.4, R4).
    if let Some(memory) = &riders.memory {
        for (key, value) in memory {
            card.memory.insert(key.clone(), value.clone());
        }
    }
    // ME-CN, R1300: what the card is shown in, like what face it wears, so a burned card keeps it too.
    if riders.chinese == Some(true) {
        card.chinese = Some(true);
    }
    if let Some(stats) = riders.stats_override {
        card.stats_override = Some(stats);
    }
    if let Some(armor) = riders.armor_override {
        card.armor_override = Some(armor);
    }
    if let Some(tuning) = &riders.tuning {
        card.tuning = Some(tuning.clone());
    }
}

fn apply_cost_riders(card: &mut CardInstance, riders: &HandRiders) {
    if let Some(cost_override) = riders.cost_override {
        card.cost_override = Some(cost_override);
    }
    if let Some(cost) = riders.embiggen_cost_override {
        card.embiggen_cost_override = Some(cost);
    }
    if let Some(cost_mod) = riders.cost_mod {
        card.cost_mod += cost_mod;
    }
    // R637: a keyword for the card's stay in the hand, so it goes on with the prices, once it is there.
    if riders.temporary == Some(true)
        && !card
            .granted_keywords
            .iter()
            .any(|keyword| matches!(keyword, Keyword::Temporary { .. }))
    {
        card.granted_keywords.push(Keyword::TEMPORARY);
    }
    // R1438: given Lucky is the card's own from the hand on, so it goes on with the prices, once it is
    // there (a burned card gets none), and adds to any Lucky the card has: Lucky sums (§6.1). The view
    // shows it to the hand's owner alone (B5 E38), and no event says it.
    if let Some(n) = riders.lucky
        && n > 0
    {
        card.granted_keywords.push(Keyword::Lucky { n });
    }
    // R1878003: "It gains Starship Piece" — given in the hand as Lucky is, it is the card's from there
    // on: R78's reset takes a granted keyword off only as the card leaves the field, so it rides the
    // card onto the field, where the layers read it (§10.4).
    for keyword in riders.keywords.iter().flatten() {
        card.granted_keywords.push(keyword.clone());
    }
}

/// §2.4's pipeline, with the riders applied around it as `apply_radiant_rider` explains.
///
/// TS wrote the riders through the live card object. Here `card` is the card as found (or the fresh
/// instance), so the radiant rider goes on it before it moves, and on the card in its pile too when
/// it already is in one; the cost riders go on the card where the move put it. Whether it reached the
/// hand is `crate::draw::add_to_hand`'s outcome: a card a full hand burns (§2.4, R4, R1143,
/// `query::room_in`) takes no price.
fn put_in_hand_with(ctx: &mut EffectContext<'_>, mut card: CardInstance, riders: &HandRiders) {
    apply_radiant_rider(&mut card, riders);
    if let Some(live) = find_instance_mut(&mut *ctx.state, &card.id) {
        apply_radiant_rider(live, riders);
    }
    let reaches_hand = crate::draw::add_to_hand(ctx, &mut card) == crate::draw::AddToHandOutcome::Hand;
    if reaches_hand && let Some(live) = find_instance_mut(&mut *ctx.state, &card.id) {
        apply_cost_riders(live, riders);
    }
    // R1848003: "their prices are set at 1, 2, 3, 4" — a price for the card in the hand, as the other
    // cost riders are, and only an X-cost card has an X to set.
    if reaches_hand
        && let Some(fixed) = riders.fixed_x
        && find_instance(&*ctx.state, &card.id).is_some_and(|live| crate::mana::is_x_cost(&*ctx.state, live))
        && let Some(live) = find_instance_mut(&mut *ctx.state, &card.id)
    {
        live.fixed_x = Some(fixed.max(0));
    }
    // R1704003: "it costs (1) more (up to (4))" — the raise is taken back as far as it went past the
    // cap, and whole from an X-cost card, whose X is no cost a number may move (R65).
    if reaches_hand
        && let (Some(cap), Some(raise)) = (riders.cost_mod_up_to, riders.cost_mod)
        && let Some(live) = find_instance(&*ctx.state, &card.id)
    {
        let back = match crate::numbers::own_cost(&*ctx.state, live) {
            None => raise,
            Some(after) => (after - cap).clamp(0, raise.max(0)),
        };
        if let Some(live) = find_instance_mut(&mut *ctx.state, &card.id) {
            live.cost_mod -= back;
        }
    }
}

/// The one creation path: a fresh card of `defId` with these riders, into that player's hand through
/// §2.4's pipeline. `add_to_hand` and `add_random_from_catalog` differ only in where the def comes
/// from, so they share this rather than each growing their own cap and burn handling.
fn create_in_hand(ctx: &mut EffectContext<'_>, def_id: &str, riders: &HandRiders) {
    let player = player_of(ctx, riders.player.unwrap_or(PlayerSpec::SelfSide));
    let card = new_instance(&mut *ctx.state, def_id, player, Zone::Hand { player });
    crate::generators::record_generator(ctx, &card);
    let id = card.id.clone();
    put_in_hand_with(ctx, card, riders);
    crate::generators::credit_creator(ctx, &id);
}

/// R57, R1242: what a copy made into a hand carries of the card copied, as `library_copies`'s do.
fn copy_riders(copy: &mut CardInstance, source: &CardInstance) {
    copy.radiant = source.radiant;
    // ME-CN, R1300: a copy of a Chinese card is Chinese.
    copy.chinese = source.chinese;
    // MD-B15, R923: a copy keeps the granted tags of the card copied.
    copy.granted_tags = source.granted_tags.clone();
    copy.stats_override = source.stats_override;
    copy.armor_override = source.armor_override;
    copy.tuning = source.tuning.clone();
}

/// R2198001 (Training Grounds #37 Seance, "an exact copy … keeping its cost changes"): what `exact`
/// adds to `copy_riders`: the rest of R57's copy of a Unit on the field (its buffs, granted keywords
/// and Vanilla state) and its price (`costMod`, `costOverride`, the Embiggened version's price and a
/// set X), which a card keeps from the hand onto the field and back (R78, R766). Damage, exertion,
/// counters, memory, the Brittle and times-played counts start afresh, as R57's do (R385, R429). A
/// price the effect states (`costOverride`) sets the copy's price outright, so the copied one is left
/// off (the Radiant Seance's "It costs (0)").
fn exact_riders(copy: &mut CardInstance, source: &CardInstance, priced: bool) {
    copy.buffs = source.buffs;
    copy.granted_keywords = source.granted_keywords.clone();
    copy.vanilla = source.vanilla;
    if !priced {
        copy.cost_mod = source.cost_mod;
        copy.cost_override = source.cost_override;
        copy.embiggen_cost_override = source.embiggen_cost_override;
        copy.fixed_x = source.fixed_x;
    }
}

/// `add_to_hand`'s arguments (TS's inline object).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1181003: the Embiggened version's own price, once the card is in the hand; the normal version
    /// keeps its price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embiggen_cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    /// ME-CN, R1300: the card made is a copy of a Chinese card (T-AI-5, Counterspell, Recycling
    /// Initiative), so it is Chinese too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chinese: Option<bool>,
    /// R57: the layer-1 face a definition-remade copy keeps without taking every `copyOf` rider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats_override: Option<AttackHealth>,
    /// R175: the Bread Token's Armor X beside its copied X/X face.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_override: Option<i32>,
    /// R57: tuning a definition-remade copy keeps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tuning: Option<Tuning>,
    /// R1438: give the card Lucky X once it is in the hand.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lucky: Option<i32>,
    /// R1242: create a copy of this instance instead (Meditative #95's fused card), a fresh card of
    /// its definition carrying R57's riders: the radiant flag, the `chinese` flag, granted tags,
    /// `statsOverride` and `tuning`. The card copied stays where it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_of: Option<String>,
    /// R2198001: with `copyOf`, an exact copy: R57's whole copy and the copied card's price as well
    /// (`exact_riders`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exact: Option<bool>,
    /// ME-SECRET, R862: what the new card remembers from the start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<IndexMap<String, Value>>,
    /// R1848003 (Stinkyscape #102 Book of Nono-Words, "their prices are set at 1, 2, 3, 4"): an X-cost
    /// card reaches the hand with its X set at this (`CardInstance.fixed_x`), so it is played for that X
    /// and costs it. Nothing on a card that has no X, or one a full hand burns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_x: Option<i32>,
}

impl AddToHandArgs {
    fn riders(&self) -> HandRiders {
        HandRiders {
            player: self.player,
            radiant: self.radiant,
            cost_override: self.cost_override,
            embiggen_cost_override: self.embiggen_cost_override,
            cost_mod: self.cost_mod,
            cost_mod_up_to: None,
            temporary: self.temporary,
            chinese: self.chinese,
            stats_override: self.stats_override,
            armor_override: self.armor_override,
            tuning: self.tuning.clone(),
            lucky: self.lucky,
            memory: self.memory.clone(),
            fixed_x: self.fixed_x,
            keywords: None,
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
/// With `copyOf`, create a copy of that instance (R1242) with R57's riders, into `player`'s hand, the
/// card copied staying where it is; it names no card, nothing happens.
///
/// `bounce` is the neighbouring verb and deliberately not this one: it returns a card from the FIELD
/// and resets the instance (§6.3, R78), which would throw away exactly the riders this verb keeps.
pub fn add_to_hand(args: AddToHandArgs) -> Effect {
    Effect::new("addToHand", move |ctx| {
        let riders = args.riders();
        if let Some(source_id) = &args.copy_of {
            let Some(source) = find_instance(ctx.state, source_id).cloned() else {
                return;
            };
            let player = player_of(ctx, riders.player.unwrap_or(PlayerSpec::SelfSide));
            let mut copy = new_instance(&mut *ctx.state, &source.def_id, player, Zone::Hand { player });
            copy_riders(&mut copy, &source);
            if args.exact == Some(true) {
                exact_riders(&mut copy, &source, args.cost_override.is_some());
            }
            crate::generators::record_generator(ctx, &copy);
            let id = copy.id.clone();
            put_in_hand_with(ctx, copy, &riders);
            crate::generators::credit_creator(ctx, &id);
            return;
        }
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1181003: the Embiggened version's own price, once the card is in the hand; the normal version
    /// keeps its price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embiggen_cost_override: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod: Option<i32>,
    /// R1704003: a `costMod` raise stops at this cost (M #37's "(up to (4))"), and an X-cost card
    /// takes none of it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_mod_up_to: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    /// ME-CN, R921: the cards added are shown in Chinese (Meditative #33).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chinese: Option<bool>,
    /// R1438: give each card added Lucky X once it is in the hand (Meditative #101 Gachaholic).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lucky: Option<i32>,
    /// R1878003: give each card added these keywords once it is in the hand (Stinkyscape #130's "It
    /// gains Starship Piece").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,
}

impl AddRandomFromCatalogArgs {
    fn riders(&self) -> HandRiders {
        HandRiders {
            player: self.player,
            radiant: self.radiant,
            cost_override: self.cost_override,
            embiggen_cost_override: self.embiggen_cost_override,
            cost_mod: self.cost_mod,
            cost_mod_up_to: self.cost_mod_up_to,
            temporary: self.temporary,
            chinese: self.chinese,
            stats_override: None,
            armor_override: None,
            tuning: None,
            lucky: self.lucky,
            memory: None,
            fixed_x: None,
            keywords: self.keywords.clone(),
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
