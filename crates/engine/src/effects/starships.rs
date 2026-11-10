//! Starships a card makes, a random Starship's launch and a shipyard's launch (SPEC §6.3 Launch;
//! R3539000–R3539004): S+ #80 The Last Voyage summons the ten Terminus Starships already holding
//! their pieces and launches one at once, S+ #155 The Far Vagabond summons one that launches for
//! its own price, and S+ #157 Phantom Shipyard exiles itself to launch as a Starship holding every
//! piece it fused in.

use serde::{Deserialize, Serialize};

use crate::config::DORMANT_STARSHIP_DEF_ID;
use crate::effects::summon::{SummonPlacement, summon_fresh, summon_fresh_standing};
use crate::effects::targets::{PlayerSpec, player_of};
use crate::script::{Effect, EffectContext};
use crate::state::find_instance_mut;
use crate::wire::{PlayerId, Row, Zone};
use crate::zones::{ZoneSlot, exile_to_pile, is_open};

/// One Starship `summon_starships` may make: its hull and the pieces it holds, oldest first.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StarshipBlueprint {
    pub def_id: String,
    pub pieces: Vec<String>,
}

/// `summonStarships`' arguments: the hulls to pick from, how many, and whose they are.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonStarshipsArgs {
    pub blueprints: Vec<StarshipBlueprint>,
    /// How many to summon; fewer when the board runs out of zones.
    pub count: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant: Option<bool>,
    /// R3539000: the price of each one's launch by hand, set on its memory; absent launches for
    /// `launch_cost` as before.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<i32>,
}

/// Summon a hull into `slot`: standing in a backrow zone as a Flying Unit does (R1081000), else
/// into the open unit zone. `None`, with nothing made, when the summon fizzles.
fn summon_hull(
    ctx: &mut EffectContext<'_>,
    def_id: &str,
    player: PlayerId,
    slot: ZoneSlot,
    radiant: Option<bool>,
    source_id: Option<String>,
) -> Option<String> {
    let at = SummonPlacement {
        lane: Some(slot.lane),
        radiant,
        source_id,
        ..SummonPlacement::default()
    };
    let made = if slot.row == Row::Backrow {
        summon_fresh_standing(ctx, def_id, player, &at)
    } else {
        summon_fresh(ctx, def_id, player, &at)
    };
    made.map(|card| card.id.clone())
}

/// R3539001: summon `count` Starships of `player`'s, each a Dormant hull holding its blueprint's
/// pieces. Each lands in `open_starship_zone` — a backrow zone, middle first, then the unit row —
/// standing in the backrow as a Flying Unit does; with no open zone, it and the rest are not
/// summoned. The picks come in a shuffled round, so no blueprint repeats until every one is out;
/// nothing is drawn when there is no blueprint or no zone (R129). A summon fires no Cry (R1).
pub fn summon_starships(args: SummonStarshipsArgs) -> Effect {
    Effect::new("summonStarships", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let radiant = args.radiant;
        let source_id = ctx.self_.as_ref().map(|me| me.id.clone());
        let len = args.blueprints.len();
        if len == 0 {
            return;
        }
        let mut order: Vec<usize> = Vec::new();
        for _ in 0..args.count.max(0) {
            let Some(slot) = crate::starship::open_starship_zone(&ctx.sink.state, player) else {
                break;
            };
            if order.is_empty() {
                order = ctx.sink.rng.shuffle(&(0..len).collect::<Vec<_>>());
            }
            let next = order.pop().unwrap_or(0);
            let blueprint = &args.blueprints[next];
            let Some(id) = summon_hull(ctx, &blueprint.def_id, player, slot, radiant, source_id.clone())
            else {
                continue;
            };
            if let Some(live) = find_instance_mut(ctx.sink.state, &id) {
                if let Some(memory) = live.starship.as_mut() {
                    memory.price = args.price;
                }
            }
            for piece in &blueprint.pieces {
                crate::starship::fuse_fresh_piece_into(
                    &mut ctx.sink,
                    &id,
                    piece,
                    radiant == Some(true),
                );
            }
        }
    })
}

/// `launchRandomStarship`'s arguments: whose hulls, and which hulls by definition.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct LaunchRandomStarshipArgs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerSpec>,
    /// The hull definitions to pick from (`hull_def_of`); any other Dormant Starship stays Dormant.
    pub hulls: Vec<String>,
}

/// R3539002: launch one of `player`'s Dormant Starships whose hull is in `hulls`, picked on the match
/// rng — a free launch (R1852006), whatever awakened it. A buried one stays buried (R1839003), and
/// nothing is drawn when there is none to launch (R129).
pub fn launch_random_starship(args: LaunchRandomStarshipArgs) -> Effect {
    Effect::new("launchRandomStarship", move |ctx| {
        let player = player_of(ctx, args.player.unwrap_or(PlayerSpec::SelfSide));
        let ids: Vec<String> = crate::starship::dormant_starships_of(&ctx.sink.state, player)
            .into_iter()
            .filter(|card| !crate::zones::is_buried(&ctx.sink.state, card))
            .filter(|card| {
                args.hulls
                    .contains(&crate::starship::hull_def_of(&ctx.sink.state, &card.def_id))
            })
            .map(|card| card.id.clone())
            .collect();
        let Some(picked) = ctx.sink.rng.pick(&ids) else {
            return;
        };
        let id = picked.clone();
        crate::slumber::awaken_card(&mut ctx.sink, &id);
    })
}

/// R3539004: exile the running shipyard and launch as a Starship holding every piece it fused in,
/// oldest first. The fresh Starship lands where the shipyard stood when that zone is still open,
/// else in `open_starship_zone`; with neither, nothing happens. The launch is free (R1852006).
pub fn launch_as_starship() -> Effect {
    Effect::new("launchAsStarship", move |ctx| {
        let Some(this) = ctx.live_self().cloned() else {
            return;
        };
        if !crate::starship::is_dormant_starship(&this) {
            return;
        }
        let Zone::Field { player, row, lane } = this.zone.clone() else {
            return;
        };
        let pieces = this
            .starship
            .as_ref()
            .map(|memory| memory.pieces.clone())
            .unwrap_or_default();
        let source_id = ctx.self_.as_ref().map(|me| me.id.clone());
        let left = ZoneSlot { player, row, lane };
        exile_to_pile(ctx, this);
        let slot = if is_open(&ctx.sink.state, left) {
            Some(left)
        } else {
            crate::starship::open_starship_zone(&ctx.sink.state, player)
        };
        let Some(slot) = slot else {
            return;
        };
        let Some(id) = summon_hull(ctx, DORMANT_STARSHIP_DEF_ID, player, slot, None, source_id)
        else {
            return;
        };
        for piece in &pieces {
            crate::starship::fuse_fresh_piece_into(
                &mut ctx.sink,
                &id,
                &piece.def_id,
                piece.radiant == Some(true),
            );
        }
        crate::slumber::awaken_card(&mut ctx.sink, &id);
    })
}
