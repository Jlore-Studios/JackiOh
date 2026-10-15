//! What a script reaches: the readers on `ctx` userdata, and the `J` module helpers (#442 L7, L8).
//!
//! A reader is a method of `ctx` (`ctx:hero_of(p)`), since `ctx` is the only way to the state, and
//! only for as long as the hook call lasts. Order matters (L7): every reader returns arrays, never
//! maps/hash tables.

use jackioh_engine::prelude::*;
use jackioh_engine::script::{AttackModArgs, ReplacementContext, TargetCheckArgs};
use mlua::{
    AnyUserData, Error, FromLuaMulti, IntoLuaMulti, Lua, LuaSerdeExt, Table, UserDataMethods,
    UserDataRegistry, Value,
};

use crate::numbers::arg;
use crate::sandbox::uncapped;

/// Helper to serialize a value to Luau. A Rust `None` hands Luau `nil`, not mlua's truthy
/// `null`: `serialize_none_to_null` is off, so an absent answer is falsy and `type(x) == "table"`
/// stays false for it (#464 gap 12).
fn to_lua<T: serde::Serialize + ?Sized>(lua: &Lua, val: &T) -> mlua::Result<Value> {
    lua.to_value_with(
        val,
        mlua::serde::SerializeOptions::new()
            .serialize_none_to_null(false)
            .serialize_unit_to_null(false),
    )
}

/// The turn as a script reads it (#464 gap 4): the turn number and the active player.
#[derive(serde::Serialize, Clone, Copy, Debug)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TurnView {
    pub number: i32,
    pub active: PlayerId,
}

/// Helper to deserialize a Luau argument into serde type T: the one walked boundary, so a
/// script's `{}` where a list stands reads as the empty list (#464 gap 1).
fn from_lua<T: serde::de::DeserializeOwned>(val: Value) -> mlua::Result<T> {
    arg(val)
}

/// A hook call's context behind a userdata whose readers are registered once per VM
/// (`register_types`). mlua builds a fresh metatable, one closure per reader, for every scoped
/// userdata of a non-`'static` type, which cost a hook call more than the hook itself did (a
/// data-only aura took about 110 µs a call, #462's notes), so the context crosses erased to
/// `'static` behind this pointer instead, for the one call `scoped` lends it to.
pub(crate) struct Live<T: 'static>(*mut T);

impl<T: 'static> Live<T> {
    fn get(&mut self) -> &mut T {
        // SAFETY: `scoped` builds a `Live` from a `&mut` borrow it holds across one `Lua::scope`,
        // and Luau reaches it only through that scope's userdata, which mlua invalidates as the
        // scope ends. Every reader borrows the userdata exclusively (`Readers` registers each as a
        // function that takes it with `borrow_mut_scoped`), so no other reference to `*self.0`
        // exists while one runs, and the erased lifetime never leaves a reader: each answers with
        // values Luau owns.
        unsafe { &mut *self.0 }
    }
}

/// The readers of one kind of context, written as against mlua's own registry for it (`&T` to
/// `add_method`, `&mut T` to `add_method_mut`) and registered over the erased `Live<T>`.
///
/// Each is registered as a plain function of its `self` userdata, borrowed mutably for the call,
/// rather than with mlua's `add_method_mut`, whose one closure per reader, shared by every
/// context of the VM, refuses to be re-entered ("mutable callback called recursively"): a reader
/// that runs another hook (`effective_cost`, `effects_of`) reaches the same reader on that hook's
/// own context, and must reach the depth cap (`LUAU_HOOK_DEPTH`), not that error.
pub struct Readers<'r, T: 'static>(&'r mut UserDataRegistry<Live<T>>);

impl<T: 'static> Readers<'_, T> {
    pub fn add_method<M, A, R>(&mut self, name: &str, method: M)
    where
        M: Fn(&Lua, &T, A) -> mlua::Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        self.add_method_mut(name, move |lua, ctx: &mut T, args| method(lua, ctx, args));
    }

    pub fn add_method_mut<M, A, R>(&mut self, name: &str, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> mlua::Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        self.0
            .add_function(name, move |lua, (this, args): (AnyUserData, A)| {
                // The answer is built with the memory cap lifted (`uncapped`); a hook the reader
                // runs puts it back for its own call.
                uncapped(lua, || {
                    this.borrow_mut_scoped::<Live<T>, _>(|live| {
                        method(lua, live.get(), args)?.into_lua_multi(lua)
                    })?
                })
            });
    }
}

/// A context a hook hands Luau, and the `'static` form its readers are registered for.
pub(crate) trait HookCtx {
    type Static: 'static;
}

impl HookCtx for &mut EffectContext<'_> {
    type Static = &'static mut EffectContext<'static>;
}

macro_rules! hook_ctx {
    ($($ctx:ident),* $(,)?) => {
        $(impl HookCtx for $ctx<'_> {
            type Static = $ctx<'static>;
        })*
    };
}

hook_ctx!(
    HookArgs,
    ConditionContext,
    CostArgs,
    WouldCounterArgs,
    TargetCheckArgs,
    AttackModArgs,
    ReplacementContext,
);

/// Runs `f` with `ctx` as the userdata Luau reads it through, for this call alone.
pub(crate) fn scoped<C: HookCtx, R>(
    lua: &Lua,
    mut ctx: C,
    f: impl FnOnce(AnyUserData) -> mlua::Result<R>,
) -> mlua::Result<R> {
    let mut live = Live(std::ptr::from_mut(&mut ctx).cast::<C::Static>());
    lua.scope(|scope| f(scope.create_any_userdata_ref_mut(&mut live)?))
}

/// Registers every context's readers on a VM's `lua`, once, as it is built.
pub(crate) fn register_types(lua: &Lua) -> mlua::Result<()> {
    lua.register_userdata_type::<Live<&'static mut EffectContext<'static>>>(|r| {
        register_ctx(&mut Readers(r))
    })?;
    lua.register_userdata_type::<Live<HookArgs<'static>>>(|r| register_hook_args(&mut Readers(r)))?;
    lua.register_userdata_type::<Live<ConditionContext<'static>>>(|r| {
        register_condition_ctx(&mut Readers(r))
    })?;
    lua.register_userdata_type::<Live<CostArgs<'static>>>(|r| register_cost_args(&mut Readers(r)))?;
    lua.register_userdata_type::<Live<WouldCounterArgs<'static>>>(|r| {
        register_would_counter_args(&mut Readers(r))
    })?;
    lua.register_userdata_type::<Live<TargetCheckArgs<'static>>>(|r| {
        register_target_check_args(&mut Readers(r))
    })?;
    lua.register_userdata_type::<Live<AttackModArgs<'static>>>(|r| {
        register_attack_mod_args(&mut Readers(r))
    })?;
    lua.register_userdata_type::<Live<ReplacementContext<'static>>>(|r| {
        register_replacement_ctx(&mut Readers(r))
    })?;
    Ok(())
}

/// Registers all read methods on `&mut EffectContext<'_>`.
pub fn register_ctx(registry: &mut Readers<'_, &'static mut EffectContext<'static>>) {
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
    registry.add_method("played_earlier", |_, ctx, (p, card): (Value, Value)| {
        let player = arg::<PlayerId>(p)?;
        // Nil is "nothing played yet", an id string names the card, and an instance table carries
        // it, so a condition hook passes its `self_` and a Cry its `live_self()` (#464 gap 3).
        if card.is_nil() {
            Ok(played_earlier(ctx.state, player, CardOrId::Nothing))
        } else if let Value::String(id) = &card {
            Ok(played_earlier(ctx.state, player, CardOrId::Id(&id.to_str()?)))
        } else {
            let card = from_lua::<CardInstance>(card)?;
            Ok(played_earlier(ctx.state, player, CardOrId::Card(&card)))
        }
    });
    registry.add_method("turn", |lua, ctx, ()| {
        to_lua(
            lua,
            &TurnView {
                number: ctx.state.turn,
                active: ctx.state.active,
            },
        )
    });
    registry.add_method("given_lucky_on", |_, ctx, card: Value| {
        // An id string or an instance table: only the granted Lucky, never the printed kind
        // (R1438, #464 gap 5).
        if let Value::String(id) = &card {
            let id = id.to_str()?;
            Ok(find_instance(ctx.state, &id).map_or(0, given_lucky_on))
        } else {
            let card = from_lua::<CardInstance>(card)?;
            Ok(given_lucky_on(&card))
        }
    });
    registry.add_method("counters", |lua, ctx, ()| to_lua(lua, &ctx.state.counters));
    registry.add_method("data", |lua, ctx, ()| to_lua(lua, &ctx.data));
    registry.add_method("events", |lua, ctx, ()| {
        // R136: this script's own window onto the action's events, `events[events_from..]`
        // (#464 gap 6).
        to_lua(lua, &ctx.sink.events[ctx.events_from..])
    });
    registry.add_method("events_from", |_, ctx, ()| Ok(ctx.events_from as i32));
    registry.add_method("left_field_since_resolved", |_, ctx, event: Value| {
        // R174, R427: whether the card the trigger event names has left the field since the play
        // resolved. The event is the one the trigger's `when` or `run` was called with (#464 gap 9).
        let event = from_lua::<GameEvent>(event)?;
        Ok(left_field_since_resolved(ctx.state, &event))
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
        // In a Death hook the live instance is the reset graveyard card, which carries no
        // `last_damaged_by` (R42/R361): an id naming the running card reads its own snapshot
        // instead (#464 gap 9).
        let card = match ctx.self_.as_ref().filter(|me| me.id == id) {
            Some(_) => ctx.live_self(),
            None => find_instance(ctx.state, &id),
        };
        to_lua(lua, &killer_of(ctx.state, card))
    });
    registry.add_method("killer_of_self", |lua, ctx, ()| {
        to_lua(lua, &killer_of(ctx.state, ctx.live_self()))
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
    registry.add_method("times_played_of", |_, ctx, card: Value| {
        // An id string or an instance table, so a preview passes its `self_` (#464 gap 3).
        if let Value::String(id) = &card {
            let id = id.to_str()?;
            Ok(find_instance(ctx.state, &id).map_or(0, times_played_of))
        } else {
            let card = from_lua::<CardInstance>(card)?;
            Ok(times_played_of(&card))
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
    // R781001: whether `source` (an instance id, or nil for no source) may not target `id`.
    registry.add_method(
        "immutable_bars",
        |_, ctx, (source, id): (Option<String>, String)| {
            let source = source
                .as_deref()
                .and_then(|source| find_instance(ctx.state, source));
            Ok(find_instance(ctx.state, &id).is_some_and(|card| immutable_bars(ctx.state, source, card)))
        },
    );
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

/// The readers every context userdata with a `state` carries (#464 gap 3): one definition, bound
/// onto each context, so a condition, preview or cost hook never lacks a reader the effect context
/// has. The projections read off `ctx`, whatever its context type: the state, the running card
/// (`me`), its controller and whether the radiant face runs. Readers that need the sink (the
/// events window), the data bag, the rng or a prompt answer stay on the effect context alone.
/// What the shared readers read off any context: the state, the running card and whether the
/// radiant face runs. Implemented for every context userdata with a `state`, so the macro below
/// binds once and attaches everywhere (#464 gap 3).
trait SharedCtx {
    fn shared_state(&self) -> &GameState;
    fn shared_me(&self) -> Option<&CardInstance>;
    fn shared_radiant(&self) -> bool;
}

impl SharedCtx for ConditionContext<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn shared_radiant(&self) -> bool {
        self.radiant
    }
}

impl SharedCtx for CostArgs<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.instance)
    }
    fn shared_radiant(&self) -> bool {
        false
    }
}

impl SharedCtx for HookArgs<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn shared_radiant(&self) -> bool {
        self.radiant
    }
}

impl SharedCtx for jackioh_engine::script::AttackModArgs<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn shared_radiant(&self) -> bool {
        self.radiant
    }
}

impl SharedCtx for jackioh_engine::script::ReplacementContext<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn shared_radiant(&self) -> bool {
        self.radiant
    }
}

impl SharedCtx for WouldCounterArgs<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn shared_radiant(&self) -> bool {
        false
    }
}

impl SharedCtx for jackioh_engine::script::TargetCheckArgs<'_> {
    fn shared_state(&self) -> &GameState {
        self.state
    }
    fn shared_me(&self) -> Option<&CardInstance> {
        Some(self.self_)
    }
    fn shared_radiant(&self) -> bool {
        self.radiant
    }
}

macro_rules! shared_readers {
    ($registry:ident) => {
        $registry.add_method("turn", |lua, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(
                lua,
                &TurnView {
                    number: state.turn,
                    active: state.active,
                },
            )
        });
        $registry.add_method("param", |_, ctx, key: String| Ok(param(ctx, &key)));
        $registry.add_method("opponent_of", |lua, _, p: Value| {
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &player.opponent())
        });
        $registry.add_method("params_view", |lua, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let me: Option<&CardInstance> = ctx.shared_me();
            let _ = ctx;
            if let Some(card) = me {
                to_lua(lua, &params_view(state, card))
            } else {
                Ok(Value::Nil)
            }
        });
        $registry.add_method("counters", |lua, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(lua, &state.counters)
        });
        $registry.add_method("hero_of", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &hero_of(state, player))
        });
        $registry.add_method("zone_cards", |lua, ctx, (p, z): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let zone = from_lua::<OffFieldZone>(z)?;
            to_lua(lua, &zone_cards(state, player, zone))
        });
        $registry.add_method("zone_count", |_, ctx, (p, z): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let zone = from_lua::<OffFieldZone>(z)?;
            Ok(zone_count(state, player, zone))
        });
        $registry.add_method("cards_played_this_turn", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(cards_played_this_turn(state, player))
        });
        $registry.add_method("unspent_mana_of", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(unspent_mana_of(state, player))
        });
        $registry.add_method("max_mana_of", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(max_mana_of(state, player))
        });
        $registry.add_method("played_ids_this_turn", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &played_ids_this_turn(state, player))
        });
        $registry.add_method("played_earlier", |_, ctx, (p, card): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            if card.is_nil() {
                Ok(played_earlier(state, player, CardOrId::Nothing))
            } else if let Value::String(id) = &card {
                Ok(played_earlier(state, player, CardOrId::Id(&id.to_str()?)))
            } else {
                let card = from_lua::<CardInstance>(card)?;
                Ok(played_earlier(state, player, CardOrId::Card(&card)))
            }
        });
        $registry.add_method("was_played_this_turn", |_, ctx, (p, id): (Value, String)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(was_played_this_turn(state, player, &id))
        });
        $registry.add_method("active_units_of", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &active_units_of(state, player))
        });
        $registry.add_method("dormant_units_of", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &dormant_units_of(state, player))
        });
        $registry.add_method("card_at", |lua, ctx, (p, lane): (Value, i32)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let slot = ZoneRef {
                player,
                row: Row::Units,
                lane,
            };
            to_lua(lua, &card_at(state, slot))
        });
        $registry.add_method("slots_of", |lua, _, (p, r): (Value, Value)| {
            let player = arg::<PlayerId>(p)?;
            let row = from_lua::<Row>(r).unwrap_or(Row::Units);
            to_lua(lua, &slots_of(player, row))
        });
        $registry.add_method("slot_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let card = find_instance(state, &id);
            to_lua(lua, &card.and_then(|c| slot_of(state, c)))
        });
        $registry.add_method("face_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let card = find_instance(state, &id);
            to_lua(lua, &card.map(|c| face_of(state, c)))
        });
        $registry.add_method("stats_with_buffs", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let card = find_instance(state, &id);
            to_lua(lua, &card.map(|c| stats_with_buffs(state, c)))
        });
        $registry.add_method("unit_view", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let card = find_instance(state, &id);
            to_lua(lua, &card.map(|c| unit_view(state, c)))
        });
        $registry.add_method("def_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(lua, &jackioh_engine::catalog::find_def(Some(state), &id))
        });
        $registry.add_method("printed_cost", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(printed_cost(state, card))
            } else if let Some(def) = jackioh_engine::catalog::find_def(Some(state), &id) {
                Ok(query_cost(def))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("effective_cost", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(effective_cost(
                    state,
                    card,
                    jackioh_engine::mana::CostOptions::default(),
                ))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("query_cost", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(def) = jackioh_engine::catalog::find_def(Some(state), &id) {
                Ok(query_cost(def))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("own_cost", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(own_cost(state, card))
            } else {
                Ok(None)
            }
        });
        $registry.add_method("cost_now", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(cost_now(state, card))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("play_cost", |_, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let me: Option<&CardInstance> = ctx.shared_me();
            let _ = ctx;
            Ok(me.map_or(0, |card| play_cost(state, card)))
        });
        $registry.add_method("tags_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &tags_of(state, card))
            } else {
                to_lua(lua, &Vec::<Tag>::new())
            }
        });
        $registry.add_method("lucky_on", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(lucky_on(state, card))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("given_lucky_on", |_, ctx, card: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Value::String(id) = &card {
                let id = id.to_str()?;
                Ok(find_instance(state, &id).map_or(0, given_lucky_on))
            } else {
                let card = from_lua::<CardInstance>(card)?;
                Ok(given_lucky_on(&card))
            }
        });
        $registry.add_method("highest_permanent_cost", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(highest_permanent_cost(state, player))
        });
        $registry.add_method("base_stats_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let card = find_instance(state, &id);
            to_lua(lua, &card.map(|c| base_stats_of(state, c)))
        });
        $registry.add_method("credit_used_this_turn", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(credit_used_this_turn(state, player))
        });
        $registry.add_method("armor_on_field", |_, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            Ok(armor_on_field(state))
        });
        $registry.add_method("find_instance", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(lua, &find_instance(state, &id))
        });
        $registry.add_method("killer_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let me: Option<&CardInstance> = ctx.shared_me();
            let _ = ctx;
            let card = match me.filter(|card| card.id == id) {
                Some(card) => Some(card),
                None => find_instance(state, &id),
            };
            to_lua(lua, &killer_of(state, card))
        });
        $registry.add_method("killer_of_self", |lua, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let me: Option<&CardInstance> = ctx.shared_me();
            let _ = ctx;
            to_lua(lua, &killer_of(state, me))
        });
        $registry.add_method("left_field_since_resolved", |_, ctx, event: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let event = from_lua::<GameEvent>(event)?;
            Ok(left_field_since_resolved(state, &event))
        });
        $registry.add_method("luck_of", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(luck_of(state, player))
        });
        $registry.add_method("plague_on", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(plague_on(card))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("plague_multiplier_of", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(plague_multiplier_of(state, card))
            } else {
                Ok(1)
            }
        });
        $registry.add_method("permanents_on_field", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &permanents_on_field(state, player))
        });
        $registry.add_method("open_zones", |lua, ctx, (p, r): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let row = from_lua::<Row>(r)?;
            to_lua(lua, &open_zones(state, player, row))
        });
        $registry.add_method("first_free_zone", |lua, ctx, (p, r): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let row = from_lua::<Row>(r)?;
            to_lua(lua, &first_free_zone(state, player, row))
        });
        $registry.add_method("midlane_lanes_of", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &midlane_lanes_of(state, player))
        });
        $registry.add_method("times_played_of", |_, ctx, card: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Value::String(id) = &card {
                let id = id.to_str()?;
                Ok(find_instance(state, &id).map_or(0, times_played_of))
            } else {
                let card = from_lua::<CardInstance>(card)?;
                Ok(times_played_of(&card))
            }
        });
        $registry.add_method("is_unit_token", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(is_unit_token(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("is_x_cost", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(is_x_cost(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("is_animated", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(is_animated(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("is_carried", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(is_carried(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("carried_at", |lua, ctx, (p, lane): (Value, i32)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let slot = ZoneRef {
                player,
                row: Row::Backrow,
                lane,
            };
            to_lua(lua, &carried_at(state, slot))
        });
        $registry.add_method("is_locked", |_, ctx, (p, lane): (Value, i32)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let slot = ZoneRef {
                player,
                row: Row::Units,
                lane,
            };
            Ok(is_locked(state, slot))
        });
        $registry.add_method("is_berserk", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(is_berserk(card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("unit_has", |_, ctx, (id, k): (String, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let kind = from_lua::<KeywordKind>(k)?;
            if let Some(card) = find_instance(state, &id) {
                Ok(unit_has(state, card, kind))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("card_keywords", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &card_keywords(state, card))
            } else {
                to_lua(lua, &Vec::<Keyword>::new())
            }
        });
        $registry.add_method("numbered_keywords_on", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &numbered_keywords_on(state, card))
            } else {
                to_lua(lua, &Vec::<jackioh_engine::NumberedKeyword>::new())
            }
        });
        $registry.add_method("card_type_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &card_type_of(state, card))
            } else {
                Ok(Value::Nil)
            }
        });
        $registry.add_method("card_type_of_face", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let radiant: bool = ctx.shared_radiant();
            let _ = ctx;
            to_lua(lua, &card_type_of_face(state, &id, radiant))
        });
        $registry.add_method("keywords_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &keywords_of(state, card))
            } else {
                to_lua(lua, &Vec::<Keyword>::new())
            }
        });
        $registry.add_method("has_keyword", |_, ctx, (id, k): (String, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let kind = from_lua::<KeywordKind>(k)?;
            if let Some(card) = find_instance(state, &id) {
                Ok(has_keyword(&keywords_of(state, card), kind))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("numbers_on", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &numbers_on(state, card))
            } else {
                to_lua(lua, &Vec::<NumberOnCard>::new())
            }
        });
        $registry.add_method("numbered_sum", |_, ctx, (id, k): (String, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let kind = from_lua::<KeywordKind>(k)?;
            if let Some(card) = find_instance(state, &id) {
                Ok(numbered_sum(&keywords_of(state, card), kind))
            } else {
                Ok(None)
            }
        });
        $registry.add_method("numbering_order", |lua, ctx, cards: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let cards = from_lua::<Vec<CardInstance>>(cards)?;
            to_lua(lua, &numbering_order(state, &cards))
        });
        $registry.add_method("applicable_changes", |lua, ctx, (id, dir): (String, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let direction = from_lua::<TuneDirection>(dir)?;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &applicable_changes(state, card, direction))
            } else {
                to_lua(lua, &Vec::<TuneRow>::new())
            }
        });
        $registry.add_method("unreadable_by", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &unreadable_by(state, card))
            } else {
                to_lua(lua, &Vec::<PlayerId>::new())
            }
        });
        $registry.add_method("stacked_onto", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(stacked_onto(card).map(str::to_string))
            } else {
                Ok(None)
            }
        });
        $registry.add_method("redirect_neighbours", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                to_lua(lua, &redirect_neighbours(state, card))
            } else {
                to_lua(lua, &Vec::<CardInstance>::new())
            }
        });
        $registry.add_method(
            "fusable_permanents_of",
            |lua, ctx, (p, except): (Value, Option<String>)| {
                let state: &GameState = ctx.shared_state();
                let _ = ctx;
                let player = arg::<PlayerId>(p)?;
                to_lua(lua, &fusable_permanents_of(state, player, except.as_deref()))
            },
        );
        $registry.add_method("fill_board_zones", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &fill_board_zones(state, player))
        });
        $registry.add_method("created_in_library", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &created_in_library(state, player))
        });
        $registry.add_method("last_face_up_played", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &last_face_up_played(state, player))
        });
        $registry.add_method("lethal_attackers_of", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(lua, &lethal_attackers_of(state, player))
        });
        $registry.add_method("played_this_game_with_tag", |_, ctx, (p, t): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let tag = from_lua::<Tag>(t)?;
            Ok(played_this_game_with_tag(state, player, tag))
        });
        $registry.add_method("played_this_turn_of_type", |_, ctx, (p, t): (Value, Value)| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            let types = from_lua::<Vec<CardType>>(t)?;
            Ok(played_this_turn_of_type(state, player, &types))
        });
        $registry.add_method("hand_cap_of", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(hand_cap_of(state, player))
        });
        $registry.add_method("discard_guarded", |_, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            Ok(jackioh_engine::draw::discard_guarded(state, player))
        });
        $registry.add_method("playable_from_graveyard", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(playable_from_graveyard(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("has_triggerable_cry", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(has_triggerable_cry(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("immune_to_tribal_hate", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(immune_to_tribal_hate(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method(
            "immutable_bars",
            |_, ctx, (source, id): (Option<String>, String)| {
                let state: &GameState = ctx.shared_state();
                let _ = ctx;
                let source = source.as_deref().and_then(|source| find_instance(state, source));
                Ok(find_instance(state, &id).is_some_and(|card| immutable_bars(state, source, card)))
            },
        );
        $registry.add_method("is_cast_on_draw", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(is_cast_on_draw(state, card))
            } else {
                Ok(false)
            }
        });
        $registry.add_method("return_price_of", |_, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            if let Some(card) = find_instance(state, &id) {
                Ok(return_price_of(card))
            } else {
                Ok(0)
            }
        });
        $registry.add_method("secret_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(lua, &secret_of(state, &id))
        });
        $registry.add_method("judge", |lua, _, (guess, choice): (Value, Value)| {
            let guess = from_lua::<SecretChoice>(guess)?;
            let choice = from_lua::<SecretChoice>(choice)?;
            to_lua(lua, &judge(guess, choice))
        });
        $registry.add_method("def_by_index", |lua, _, (set, index): (Value, String)| {
            let set = from_lua::<SetName>(set)?;
            to_lua(lua, &def_by_index(set, &index))
        });
        $registry.add_method(
            "fill_params",
            |_, ctx, (id, face, values): (String, Value, Value)| {
                let state: &GameState = ctx.shared_state();
                let _ = ctx;
                let face = from_lua::<FaceKind>(face)?;
                let values = if values.is_nil() {
                    None
                } else {
                    Some(from_lua::<IndexMap<String, i32>>(values)?)
                };
                if let Some(def) = jackioh_engine::catalog::find_def(Some(state), &id) {
                    Ok(fill_params(def, face, values.as_ref()))
                } else {
                    Ok(String::new())
                }
            },
        );
        $registry.add_method("win_rate_leader", |lua, _, exclude: Option<String>| {
            let exclude_str = exclude.as_deref().unwrap_or("");
            to_lua(lua, &win_rate_leader(exclude_str))
        });
        $registry.add_method("fib", |_, _, n: i32| Ok(fib(n)));
        $registry.add_method("attack_target_of", |lua, ctx, id: String| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(lua, &attack_target_of(state, &id))
        });
        $registry.add_method(
            "random_attack_targets",
            |lua, ctx, (id, among): (String, Value)| {
                let state: &GameState = ctx.shared_state();
                let _ = ctx;
                let among = from_lua::<AttackAmong>(among)?;
                if let Some(card) = find_instance(state, &id) {
                    to_lua(lua, &random_attack_targets(state, card, among))
                } else {
                    to_lua(lua, &Vec::<AttackTarget>::new())
                }
            },
        );
        $registry.add_method("subsystems_feng_shui_last_element", |lua, ctx, p: Value| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            let player = arg::<PlayerId>(p)?;
            to_lua(
                lua,
                &jackioh_engine::subsystems::feng_shui::last_element(state, player),
            )
        });
        $registry.add_method(
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
        $registry.add_method("subsystems_pareto_judgement_of", |lua, ctx, ()| {
            let state: &GameState = ctx.shared_state();
            let _ = ctx;
            to_lua(lua, &jackioh_engine::subsystems::pareto::judgement_of(state))
        });
    };
}

/// Registers methods on `HookArgs<'_>`.
pub fn register_hook_args(registry: &mut Readers<'_, HookArgs<'static>>) {
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.self_.controller));
    registry.add_method("radiant", |_, args, ()| Ok(args.radiant));
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    shared_readers!(registry);
}

/// Registers methods on `ConditionContext<'_>`.
pub fn register_condition_ctx(registry: &mut Readers<'_, ConditionContext<'static>>) {
    registry.add_method("controller", |lua, ctx, ()| to_lua(lua, &ctx.controller));
    registry.add_method("radiant", |_, ctx, ()| Ok(ctx.radiant));
    registry.add_method("your_turn", |_, ctx, ()| Ok(ctx.your_turn));
    registry.add_method("zone", |lua, ctx, ()| to_lua(lua, &ctx.zone));
    registry.add_method("self_", |lua, ctx, ()| to_lua(lua, &ctx.self_));
    shared_readers!(registry);
}

/// Registers methods on `CostArgs<'_>`.
pub fn register_cost_args(registry: &mut Readers<'_, CostArgs<'static>>) {
    registry.add_method("instance", |lua, args, ()| to_lua(lua, &args.instance));
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.instance));
    registry.add_method("controller", |lua, args, ()| {
        to_lua(lua, &args.instance.controller)
    });
    shared_readers!(registry);
}

/// Registers methods on `WouldCounterArgs<'_>`.
pub fn register_would_counter_args(registry: &mut Readers<'_, WouldCounterArgs<'static>>) {
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.controller));
    registry.add_method("player", |lua, args, ()| to_lua(lua, &args.player));
    registry.add_method("cost_paid", |_, args, ()| Ok(args.cost_paid));
    shared_readers!(registry);
}

/// Registers methods on `TargetCheckArgs<'_>`.
pub fn register_target_check_args(registry: &mut Readers<'_, TargetCheckArgs<'static>>) {
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("candidate", |lua, args, ()| to_lua(lua, &args.candidate));
    registry.add_method("player", |lua, args, ()| to_lua(lua, &args.player));
    registry.add_method("radiant", |_, args, ()| Ok(args.radiant));
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.self_.controller));
    shared_readers!(registry);
}

/// Registers methods on `AttackModArgs<'_>`.
pub fn register_attack_mod_args(registry: &mut Readers<'_, AttackModArgs<'static>>) {
    registry.add_method("self_", |lua, args, ()| to_lua(lua, &args.self_));
    registry.add_method("attacker", |lua, args, ()| to_lua(lua, &args.attacker));
    registry.add_method("defender", |lua, args, ()| to_lua(lua, &args.defender));
    registry.add_method("radiant", |_, args, ()| Ok(args.radiant));
    registry.add_method("controller", |lua, args, ()| to_lua(lua, &args.self_.controller));
    shared_readers!(registry);
}

/// Registers methods on `ReplacementContext<'_>`.
pub fn register_replacement_ctx(registry: &mut Readers<'_, ReplacementContext<'static>>) {
    registry.add_method("self_", |lua, ctx, ()| to_lua(lua, &ctx.self_));
    registry.add_method("controller", |lua, ctx, ()| to_lua(lua, &ctx.controller));
    registry.add_method("radiant", |_, ctx, ()| Ok(ctx.radiant));
    registry.add_method("event", |lua, ctx, ()| to_lua(lua, &ctx.event));
    shared_readers!(registry);
}
