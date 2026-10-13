//! What a script reaches: the readers on `ctx` userdata, and the `J` module helpers (#442 L7, L8).
//!
//! A reader is a method of `ctx` (`ctx:hero_of(p)`), since `ctx` is the only way to the state, and
//! only for as long as the hook call lasts. Order matters (L7): every reader returns arrays, never
//! maps/hash tables.

use jackioh_engine::prelude::*;
use mlua::{Error, Lua, LuaSerdeExt, Table, UserDataMethods, UserDataRegistry, Value};
use serde_json::Value as Json;

use crate::numbers::arg;

/// Helper to serialize a value to Luau.
fn to_lua<T: serde::Serialize>(lua: &Lua, val: &T) -> mlua::Result<Value> {
    lua.to_value(val)
}

/// Helper to deserialize a Luau argument into serde type T.
fn from_lua<T: serde::de::DeserializeOwned>(val: Value) -> mlua::Result<T> {
    let json: Json = serde_json::from_value(arg::<Json>(val)?).map_err(|e| Error::runtime(e.to_string()))?;
    serde_json::from_value(json).map_err(|e| Error::runtime(e.to_string()))
}

/// Registers all read methods on `&mut EffectContext<'_>`.
pub fn register_ctx(registry: &mut UserDataRegistry<&mut EffectContext<'_>>) {
    // ---- Context properties ----
    registry.add_method("controller", |lua, ctx, ()| to_lua(lua, &ctx.controller));
    registry.add_method("radiant", |_, ctx, ()| Ok(ctx.radiant));
    registry.add_method("x", |_, ctx, ()| Ok(ctx.x));
    registry.add_method("embiggened", |_, ctx, ()| Ok(ctx.embiggened));
    registry.add_method("mana_before_play", |_, ctx, ()| Ok(ctx.mana_before_play));
    registry.add_method("self_", |lua, ctx, ()| to_lua(lua, &ctx.self_));
    registry.add_method("live_self", |lua, ctx, ()| to_lua(lua, &ctx.live_self()));
    registry.add_method("stands_since_script_began", |_, ctx, id: String| {
        Ok(stands_since_script_began(ctx, &id))
    });
    registry.add_method("opponent_of", |lua, _, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &player.opponent())
    });

    // ---- Parameters & memory ----
    registry.add_method("param", |_, ctx, key: String| Ok(param(&**ctx, &key)));
    registry.add_method("param_value", |_, ctx, key: String| Ok(param(&**ctx, &key)));
    registry.add_method("params_view", |lua, ctx, ()| {
        if let Some(card) = ctx.live_self() {
            to_lua(lua, &params_view(ctx.state, card))
        } else {
            Ok(Value::Nil)
        }
    });
    registry.add_method("recalled", |lua, ctx, key: String| {
        to_lua(lua, &recalled(ctx, &key))
    });

    // ---- Choice readers ----
    registry.add_method("chosen_options", |lua, ctx, ()| to_lua(lua, &chosen_options(ctx)));
    registry.add_method("chosen_number", |_, ctx, ()| Ok(chosen_number(ctx)));
    registry.add_method("chosen_cells", |lua, ctx, ()| to_lua(lua, &chosen_cells(ctx)));
    registry.add_method("chosen_recipe", |lua, ctx, ()| to_lua(lua, &chosen_recipe(ctx)));
    registry.add_method("chosen_tuning_number", |lua, ctx, ()| {
        to_lua(lua, &chosen_tuning_number(ctx))
    });
    registry.add_method("summoned_so_far", |lua, ctx, ()| {
        to_lua(lua, &summoned_so_far(ctx))
    });

    // ---- Combat & Activation facts ----
    registry.add_method("after_attack_of", |lua, ctx, ()| {
        to_lua(lua, &after_attack_of(ctx))
    });
    registry.add_method("after_attacked_of", |lua, ctx, ()| {
        to_lua(lua, &after_attacked_of(ctx))
    });
    registry.add_method("activation_paid", |lua, ctx, ()| {
        to_lua(lua, &jackioh_engine::subsystems::activate::activation_paid(ctx))
    });

    // ---- State query readers ----
    registry.add_method("hero_of", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &hero_of(ctx.state, player))
    });
    registry.add_method("zone_cards", |lua, ctx, (p, z): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let zone = from_lua::<OffFieldZone>(z)?;
        to_lua(lua, &zone_cards(ctx.state, player, zone))
    });
    registry.add_method("zone_count", |_, ctx, (p, z): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let zone = from_lua::<OffFieldZone>(z)?;
        Ok(zone_count(ctx.state, player, zone))
    });
    registry.add_method("cards_played_this_turn", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(cards_played_this_turn(ctx.state, player))
    });
    registry.add_method("unspent_mana_of", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(unspent_mana_of(ctx.state, player))
    });
    registry.add_method("max_mana_of", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(max_mana_of(ctx.state, player))
    });
    registry.add_method("played_ids_this_turn", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &played_ids_this_turn(ctx.state, player))
    });
    registry.add_method("played_earlier", |_, ctx, (p, id): (Value, String)| {
        let player = arg::<PlayerId>(p)?;
        Ok(played_earlier(ctx.state, player, &id[..]))
    });
    registry.add_method("was_played_this_turn", |_, ctx, (p, id): (Value, String)| {
        let player = arg::<PlayerId>(p)?;
        Ok(was_played_this_turn(ctx.state, player, &id))
    });
    registry.add_method("active_units_of", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &active_units_of(ctx.state, player))
    });
    registry.add_method("dormant_units_of", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &dormant_units_of(ctx.state, player))
    });
    registry.add_method("card_at", |lua, ctx, (p, lane): (Value, i32)| {
        let player = arg::<PlayerId>(p)?;
        let slot = ZoneRef {
            player,
            row: Row::Units,
            lane,
        };
        to_lua(lua, &card_at(ctx.state, slot))
    });
    registry.add_method("slots_of", |lua, _, (p, r): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let row = from_lua::<Row>(r).unwrap_or(Row::Units);
        to_lua(lua, &slots_of(player, row))
    });
    registry.add_method("slot_of", |lua, ctx, id: String| {
        let card = find_instance(ctx.state, &id);
        to_lua(lua, &card.and_then(|c| slot_of(ctx.state, c)))
    });
    registry.add_method("face_of", |lua, ctx, id: String| {
        let card = find_instance(ctx.state, &id);
        to_lua(lua, &card.map(|c| face_of(ctx.state, c)))
    });
    registry.add_method("stats_with_buffs", |lua, ctx, id: String| {
        let card = find_instance(ctx.state, &id);
        to_lua(lua, &card.map(|c| stats_with_buffs(ctx.state, c)))
    });
    registry.add_method("unit_view", |lua, ctx, id: String| {
        let card = find_instance(ctx.state, &id);
        to_lua(lua, &card.map(|c| unit_view(ctx.state, c)))
    });
    registry.add_method("def_of", |lua, ctx, id: String| {
        to_lua(lua, &jackioh_engine::catalog::find_def(Some(ctx.state), &id))
    });
    registry.add_method("printed_cost", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(printed_cost(ctx.state, card))
        } else if let Some(def) = jackioh_engine::catalog::find_def(Some(ctx.state), &id) {
            Ok(query_cost(def))
        } else {
            Ok(0)
        }
    });
    registry.add_method("effective_cost", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(effective_cost(
                ctx.state,
                card,
                jackioh_engine::mana::CostOptions::default(),
            ))
        } else {
            Ok(0)
        }
    });
    registry.add_method("query_cost", |_, ctx, id: String| {
        if let Some(def) = jackioh_engine::catalog::find_def(Some(ctx.state), &id) {
            Ok(query_cost(def))
        } else {
            Ok(0)
        }
    });
    registry.add_method("own_cost", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(own_cost(ctx.state, card))
        } else {
            Ok(None)
        }
    });
    registry.add_method("cost_now", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(cost_now(ctx.state, card))
        } else {
            Ok(0)
        }
    });
    registry.add_method("tags_of", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &tags_of(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<Tag>::new())
        }
    });
    registry.add_method("lucky_on", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(lucky_on(ctx.state, card))
        } else {
            Ok(0)
        }
    });
    registry.add_method("highest_permanent_cost", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(highest_permanent_cost(ctx.state, player))
    });
    registry.add_method("base_stats_of", |lua, ctx, id: String| {
        let card = find_instance(ctx.state, &id);
        to_lua(lua, &card.map(|c| base_stats_of(ctx.state, c)))
    });
    registry.add_method("credit_used_this_turn", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(credit_used_this_turn(ctx.state, player))
    });
    registry.add_method("armor_on_field", |_, ctx, ()| Ok(armor_on_field(ctx.state)));
    registry.add_method("find_instance", |lua, ctx, id: String| {
        to_lua(lua, &find_instance(ctx.state, &id))
    });
    registry.add_method("killer_of", |lua, ctx, id: String| {
        let card = find_instance(ctx.state, &id);
        to_lua(lua, &killer_of(ctx.state, card))
    });
    registry.add_method("luck_of", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(luck_of(ctx.state, player))
    });
    registry.add_method("plague_on", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(plague_on(card))
        } else {
            Ok(0)
        }
    });
    registry.add_method("plague_multiplier_of", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(plague_multiplier_of(ctx.state, card))
        } else {
            Ok(1)
        }
    });
    registry.add_method("permanents_on_field", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &permanents_on_field(ctx.state, player))
    });
    registry.add_method("open_zones", |lua, ctx, (p, r): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let row = from_lua::<Row>(r)?;
        to_lua(lua, &open_zones(ctx.state, player, row))
    });
    registry.add_method("first_free_zone", |lua, ctx, (p, r): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let row = from_lua::<Row>(r)?;
        to_lua(lua, &first_free_zone(ctx.state, player, row))
    });
    registry.add_method("midlane_lanes_of", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &midlane_lanes_of(ctx.state, player))
    });
    registry.add_method("times_played_of", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(times_played_of(card))
        } else {
            Ok(0)
        }
    });
    registry.add_method("is_unit_token", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(is_unit_token(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("is_x_cost", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(is_x_cost(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("is_animated", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(is_animated(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("is_carried", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(is_carried(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("carried_at", |lua, ctx, (p, lane): (Value, i32)| {
        let player = arg::<PlayerId>(p)?;
        let slot = ZoneRef {
            player,
            row: Row::Backrow,
            lane,
        };
        to_lua(lua, &carried_at(ctx.state, slot))
    });
    registry.add_method("is_locked", |_, ctx, (p, lane): (Value, i32)| {
        let player = arg::<PlayerId>(p)?;
        let slot = ZoneRef {
            player,
            row: Row::Units,
            lane,
        };
        Ok(is_locked(ctx.state, slot))
    });
    registry.add_method("is_berserk", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(is_berserk(card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("unit_has", |_, ctx, (id, k): (String, Value)| {
        let kind = from_lua::<KeywordKind>(k)?;
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(unit_has(ctx.state, card, kind))
        } else {
            Ok(false)
        }
    });
    registry.add_method("card_keywords", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &card_keywords(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<Keyword>::new())
        }
    });
    registry.add_method("numbered_keywords_on", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &numbered_keywords_on(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<jackioh_engine::NumberedKeyword>::new())
        }
    });
    registry.add_method("win_rate_leader", |lua, _, exclude: Option<String>| {
        let exclude_str = exclude.as_deref().unwrap_or("");
        to_lua(lua, &win_rate_leader(exclude_str))
    });
    registry.add_method("fib", |_, _, n: i32| Ok(fib(n)));

    // ---- Targeting & Scopes ----
    registry.add_method("instance_of", |lua, ctx, spec: Value| {
        let spec = from_lua::<TargetSpec>(spec)?;
        to_lua(lua, &instance_of(ctx, &spec))
    });
    registry.add_method("resolve_target", |lua, ctx, spec: Value| {
        let spec = from_lua::<TargetSpec>(spec)?;
        to_lua(lua, &resolve_target(ctx, &spec))
    });
    registry.add_method("sides_of", |lua, ctx, side: Value| {
        let side = if side.is_nil() {
            None
        } else {
            Some(from_lua::<ScopeSide>(side)?)
        };
        to_lua(lua, &sides_of(ctx, side))
    });
    registry.add_method("cards_in_scope", |lua, ctx, scope: Value| {
        let scope = from_lua::<BoardScope>(scope)?;
        to_lua(lua, &cards_in_scope(ctx, &scope))
    });
    registry.add_method(
        "cards_in_scope_aimed",
        |lua, ctx, (scope, aim): (Value, Value)| {
            let scope = from_lua::<BoardScope>(scope)?;
            let aim = from_lua::<TargetAim>(aim)?;
            to_lua(lua, &cards_in_scope_aimed(ctx, &scope, aim))
        },
    );
    registry.add_method("adjacent_to", |lua, ctx, (spec, scope): (Value, Value)| {
        let spec = from_lua::<TargetSpec>(spec)?;
        let scope = from_lua::<BoardScope>(scope)?;
        to_lua(lua, &adjacent_to(ctx, &spec, &scope))
    });
    registry.add_method("unreadable_by", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &unreadable_by(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<PlayerId>::new())
        }
    });

    // ---- RNG draws ----
    registry.add_method_mut("rng_roll", |_, ctx, sides: i32| Ok(ctx.rng.int(sides) + 1));
    registry.add_method_mut("rng_next_int", |_, ctx, (min, max): (i32, i32)| {
        let span = (max - min + 1).max(1);
        Ok(min + ctx.rng.int(span))
    });
    registry.add_method_mut("rng_chance", |_, ctx, (num, den): (i32, i32)| {
        let p = if den > 0 { (num as f64) / (den as f64) } else { 0.0 };
        Ok(ctx.rng.chance(p))
    });
    registry.add_method_mut("rng_pick", |_, ctx, array: Table| {
        let len = array.raw_len();
        if len == 0 {
            return Ok(Value::Nil);
        }
        let at = ctx.rng.int(len as i32);
        array.raw_get(at + 1)
    });
    registry.add_method_mut("rng_shuffle", |lua, ctx, array: Table| {
        let len = array.raw_len();
        let mut items = Vec::new();
        for i in 1..=len {
            items.push(array.raw_get::<Value>(i)?);
        }
        let shuffled = ctx.rng.shuffle(&items);
        let out = lua.create_table()?;
        for (i, item) in shuffled.into_iter().enumerate() {
            out.raw_set(i + 1, item)?;
        }
        Ok(Value::Table(out))
    });

    // ---- Combat targets & types ----
    registry.add_method("attack_target_of", |lua, ctx, id: String| {
        to_lua(lua, &attack_target_of(ctx.state, &id))
    });
    registry.add_method(
        "random_attack_targets",
        |lua, ctx, (id, among): (String, Value)| {
            let among = from_lua::<AttackAmong>(among)?;
            if let Some(card) = find_instance(ctx.state, &id) {
                to_lua(lua, &random_attack_targets(ctx.state, card, among))
            } else {
                to_lua(lua, &Vec::<AttackTarget>::new())
            }
        },
    );
    registry.add_method("card_type_of", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &card_type_of(ctx.state, card))
        } else {
            Ok(Value::Nil)
        }
    });
    registry.add_method("card_type_of_face", |lua, ctx, id: String| {
        to_lua(lua, &card_type_of_face(ctx.state, &id, ctx.radiant))
    });
    registry.add_method("keywords_of", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &keywords_of(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<Keyword>::new())
        }
    });
    registry.add_method("has_keyword", |_, ctx, (id, k): (String, Value)| {
        let kind = from_lua::<KeywordKind>(k)?;
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(has_keyword(&keywords_of(ctx.state, card), kind))
        } else {
            Ok(false)
        }
    });
    registry.add_method("numbers_on", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &numbers_on(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<NumberOnCard>::new())
        }
    });
    registry.add_method("numbered_sum", |_, ctx, (id, k): (String, Value)| {
        let kind = from_lua::<KeywordKind>(k)?;
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(numbered_sum(&keywords_of(ctx.state, card), kind))
        } else {
            Ok(None)
        }
    });
    registry.add_method("numbering_order", |lua, ctx, cards: Value| {
        let cards = from_lua::<Vec<CardInstance>>(cards)?;
        to_lua(lua, &numbering_order(ctx.state, &cards))
    });

    // ---- Tune rows ----
    registry.add_method("applicable_changes", |lua, ctx, (id, dir): (String, Value)| {
        let direction = from_lua::<TuneDirection>(dir)?;
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &applicable_changes(ctx.state, card, direction))
        } else {
            to_lua(lua, &Vec::<TuneRow>::new())
        }
    });

    // ---- Costs, guards and playability ----
    registry.add_method("play_cost", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(play_cost(ctx.state, card))
        } else {
            Ok(0)
        }
    });
    registry.add_method("playable_from_graveyard", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(playable_from_graveyard(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("has_triggerable_cry", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(has_triggerable_cry(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("discard_guarded", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(jackioh_engine::draw::discard_guarded(ctx.state, player))
    });
    registry.add_method("hand_cap_of", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(hand_cap_of(ctx.state, player))
    });
    registry.add_method("immune_to_tribal_hate", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(immune_to_tribal_hate(ctx.state, card))
        } else {
            Ok(false)
        }
    });
    registry.add_method("is_cast_on_draw", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(is_cast_on_draw(ctx.state, card))
        } else {
            Ok(false)
        }
    });

    // ---- History & publicity ----
    registry.add_method("played_this_game_with_tag", |_, ctx, (p, t): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let tag = from_lua::<Tag>(t)?;
        Ok(played_this_game_with_tag(ctx.state, player, tag))
    });
    registry.add_method("played_this_turn_of_type", |_, ctx, (p, t): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let types = from_lua::<Vec<CardType>>(t)?;
        Ok(played_this_turn_of_type(ctx.state, player, &types))
    });
    registry.add_method("created_in_library", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &created_in_library(ctx.state, player))
    });
    registry.add_method("last_face_up_played", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &last_face_up_played(ctx.state, player))
    });
    registry.add_method("lethal_attackers_of", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &lethal_attackers_of(ctx.state, player))
    });

    // ---- Zones, stacking and neighbours ----
    registry.add_method("fill_board_zones", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &fill_board_zones(ctx.state, player))
    });
    registry.add_method("stacked_onto", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(stacked_onto(card).map(str::to_string))
        } else {
            Ok(None)
        }
    });
    registry.add_method("redirect_neighbours", |lua, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            to_lua(lua, &redirect_neighbours(ctx.state, card))
        } else {
            to_lua(lua, &Vec::<CardInstance>::new())
        }
    });
    registry.add_method(
        "fusable_permanents_of",
        |lua, ctx, (p, except): (Value, Option<String>)| {
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &fusable_permanents_of(ctx.state, player, except.as_deref()))
        },
    );

    // ---- Secrets ----
    registry.add_method("secret_of", |lua, ctx, id: String| {
        to_lua(lua, &secret_of(ctx.state, &id))
    });
    registry.add_method("judge", |lua, _, (guess, choice): (Value, Value)| {
        let guess = from_lua::<SecretChoice>(guess)?;
        let choice = from_lua::<SecretChoice>(choice)?;
        to_lua(lua, &judge(guess, choice))
    });

    // ---- Catalog data ----
    registry.add_method("def_by_index", |lua, _, (set, index): (Value, String)| {
        let set = from_lua::<SetName>(set)?;
        to_lua(lua, &def_by_index(set, &index))
    });
    registry.add_method(
        "fill_params",
        |_, ctx, (id, face, values): (String, Value, Value)| {
            let face = from_lua::<FaceKind>(face)?;
            let values = if values.is_nil() {
                None
            } else {
                Some(from_lua::<IndexMap<String, i32>>(values)?)
            };
            if let Some(def) = jackioh_engine::catalog::find_def(Some(ctx.state), &id) {
                Ok(fill_params(def, face, values.as_ref()))
            } else {
                Ok(String::new())
            }
        },
    );
    registry.add_method("grant_param", |_, ctx, key: String| Ok(grant_param(ctx, &key)));
    registry.add_method("return_price_of", |_, ctx, id: String| {
        if let Some(card) = find_instance(ctx.state, &id) {
            Ok(return_price_of(card))
        } else {
            Ok(0)
        }
    });
    // `roll_weighted` is intentionally unbound: its table is `&[RollOdds]`, whose
    // `def_id` is `&'static str`, so a Luau table cannot build one without leaking.
    // `docs/v0.4.0/api-gaps.md` records what meditative-054 waits for.
    registry.add_method("field_spells_doomed", |lua, ctx, side: Value| {
        let side = from_lua::<FieldSpellSide>(side)?;
        let reader = jackioh_engine::effects::datacenter::SweepReader {
            state: ctx.state,
            self_: ctx.self_.as_ref(),
            def_id: ctx.def_id.as_deref(),
            radiant: ctx.radiant,
            controller: ctx.controller,
        };
        to_lua(lua, &field_spells_doomed(reader, side))
    });

    // ---- Subsystems ----
    registry.add_method("subsystems_activation_paid", |lua, ctx, ()| {
        to_lua(lua, &jackioh_engine::subsystems::activate::activation_paid(ctx))
    });
    registry.add_method("subsystems_feng_shui_last_element", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(
            lua,
            &jackioh_engine::subsystems::feng_shui::last_element(ctx.state, player),
        )
    });
    registry.add_method(
        "subsystems_feng_shui_element_preview",
        |lua, _, (label, elem): (String, Value)| {
            let element = if elem.is_nil() {
                None
            } else {
                Some(from_lua::<jackioh_engine::CardElement>(elem)?)
            };
            to_lua(
                lua,
                &jackioh_engine::subsystems::feng_shui::element_preview(&label, element),
            )
        },
    );
    registry.add_method("subsystems_pareto_judgement_of", |lua, ctx, ()| {
        to_lua(lua, &jackioh_engine::subsystems::pareto::judgement_of(ctx.state))
    });

    // ---- Another card's hook (test helper) ----
    registry.add_method_mut(
        "effects_of",
        |lua, ctx, (card, face, hook): (Value, Value, Value)| {
            let (card, face, hook) = (arg::<String>(card)?, arg::<String>(face)?, arg::<String>(hook)?);
            let scripts = jackioh_engine::scripts::scripts_ref(ctx.state, &card);
            let script = match face.as_str() {
                "base" => &scripts.base,
                "radiant" => &scripts.radiant,
                other => return Err(Error::runtime(format!("{other:?} is not base or radiant"))),
            };
            let Some(run) = script.hook_named(&hook).cloned() else {
                return Err(Error::runtime(format!("{card} {face} has no {hook}")));
            };
            let kinds: Vec<&str> = run(ctx).iter().map(|effect| effect.kind).collect();
            to_lua(lua, &kinds)
        },
    );
}

/// Registers methods on `HookArgs<'_>`.
pub fn register_hook_args(registry: &mut UserDataRegistry<HookArgs<'_>>) {
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.self_.controller));
    registry.add_method("param", |_, args, key: String| Ok(param(args, &key)));
    registry.add_method("radiant", |_, args, ()| Ok(args.radiant));
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("hero_of", |lua, args, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &hero_of(args.state, player))
    });
    registry.add_method("unspent_mana_of", |_, args, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(unspent_mana_of(args.state, player))
    });
    registry.add_method("max_mana_of", |_, args, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(max_mana_of(args.state, player))
    });
    registry.add_method("plague_on", |_, args, id: String| {
        if let Some(card) = find_instance(args.state, &id) {
            Ok(plague_on(card))
        } else {
            Ok(0)
        }
    });
    registry.add_method("opponent_of", |lua, _, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &player.opponent())
    });
}

/// Registers methods on `ConditionContext<'_>`.
pub fn register_condition_ctx(registry: &mut UserDataRegistry<ConditionContext<'_>>) {
    registry.add_method("controller", |lua, ctx, ()| to_lua(lua, &ctx.controller));
    registry.add_method("radiant", |_, ctx, ()| Ok(ctx.radiant));
    registry.add_method("your_turn", |_, ctx, ()| Ok(ctx.your_turn));
    registry.add_method("zone", |lua, ctx, ()| to_lua(lua, &ctx.zone));
    registry.add_method("self_", |lua, ctx, ()| to_lua(lua, &ctx.self_));
    registry.add_method("param", |_, ctx, key: String| Ok(param(ctx, &key)));
    registry.add_method("hero_of", |lua, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &hero_of(ctx.state, player))
    });
    registry.add_method("unspent_mana_of", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(unspent_mana_of(ctx.state, player))
    });
    registry.add_method("max_mana_of", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(max_mana_of(ctx.state, player))
    });
    registry.add_method("cards_played_this_turn", |_, ctx, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(cards_played_this_turn(ctx.state, player))
    });
    registry.add_method("zone_count", |_, ctx, (p, z): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        let zone = from_lua::<OffFieldZone>(z)?;
        Ok(zone_count(ctx.state, player, zone))
    });
}

/// Registers methods on `CostArgs<'_>`.
pub fn register_cost_args(registry: &mut UserDataRegistry<CostArgs<'_>>) {
    registry.add_method("instance", |lua, args, ()| to_lua(lua, &args.instance));
    registry.add_method("hero_of", |lua, args, p: Value| {
        let player = arg::<PlayerId>(p)?;
        to_lua(lua, &hero_of(args.state, player))
    });
    registry.add_method("unspent_mana_of", |_, args, p: Value| {
        let player = arg::<PlayerId>(p)?;
        Ok(unspent_mana_of(args.state, player))
    });
}

/// Registers methods on `WouldCounterArgs<'_>`.
pub fn register_would_counter_args(registry: &mut UserDataRegistry<WouldCounterArgs<'_>>) {
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.controller));
    registry.add_method("player", |lua, args, ()| to_lua(lua, &args.player));
    registry.add_method("cost_paid", |_, args, ()| Ok(args.cost_paid));
}

/// Registers methods on `TargetCheckArgs<'_>`.
pub fn register_target_check_args(
    registry: &mut UserDataRegistry<jackioh_engine::script::TargetCheckArgs<'_>>,
) {
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("candidate", |lua, args, ()| to_lua(lua, &args.candidate));
    registry.add_method("player", |lua, args, ()| to_lua(lua, &args.player));
    registry.add_method("radiant", |_, args, ()| Ok(args.radiant));
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.self_.controller));
}

/// Registers methods on `AttackModArgs<'_>`.
pub fn register_attack_mod_args(registry: &mut UserDataRegistry<jackioh_engine::script::AttackModArgs<'_>>) {
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("attacker", |lua, args, ()| to_lua(lua, &args.attacker));
    registry.add_method("defender", |lua, args, ()| to_lua(lua, &args.defender));
    registry.add_method("radiant", |_, args, ()| Ok(args.radiant));
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.self_.controller));
    registry.add_method("param", |_, args, key: String| Ok(param(args, &key)));
}

/// Registers methods on `ReplacementContext<'_>`.
pub fn register_replacement_ctx(
    registry: &mut UserDataRegistry<jackioh_engine::script::ReplacementContext<'_>>,
) {
    registry.add_method("self_", |lua, ctx, ()| to_lua(lua, &ctx.self_));
    registry.add_method("controller", |lua, ctx, ()| to_lua(lua, &ctx.controller));
    registry.add_method("radiant", |_, ctx, ()| Ok(ctx.radiant));
    registry.add_method("event", |lua, ctx, ()| to_lua(lua, &ctx.event));
    registry.add_method("param", |_, ctx, key: String| Ok(param(ctx, &key)));
}
