//! Parses card script declarations and hooks into the engine's `CardScripts`.

use jackioh_engine::prelude::*;
use jackioh_engine::script::{
    ActivationCost, ActivationDecl, ActivationUses, ReplacementDef, ReplacementMoment, ReplacementWhere,
    Script, StaticFlags, TriggerDef,
};
use jackioh_engine::wire::{GameEventType, ModeDecl, TargetDecl};
use mlua::{Function, Table, Value};

use crate::hooks;
use crate::numbers;
use crate::vm;
use crate::{Face, LuauError, Site};

/// Parses a card's module into `CardScripts`.
pub fn load_card(card: &'static str, bytecode: &'static [u8]) -> Result<CardScripts, LuauError> {
    let refused = |message: String| LuauError::Load {
        card: card.to_string(),
        message,
    };

    vm::with_module(card, bytecode, |_lua, module| {
        let mut base_script = None;
        let mut radiant_script = None;
        // One adapter for one function under two hooks, across both faces too (#464 gaps 10
        // and 14): when two hook keys hold the same Luau function value — two hooks of one
        // face, or one hook of both faces, whose params differ — both share one `Arc`, so
        // `Arc::ptr_eq` holds the way it does for a Rust script that clones one hook
        // (`classicplus-070`; `classic-082` and `classicplus-047` across faces). The shared
        // function reads `param` off the running face, so one adapter serves both. The cache
        // lives here, not in `build_script`, so both faces draw from it.
        let mut hook_cache: HookCache = Vec::new();
        // Each face's table, so a face's hooks can ask whether the other face holds the same
        // function under the same key (`build_script`): none for `radiant = "base"`.
        let face_table = |face: Face| match module.raw_get::<Value>(face.key()) {
            Ok(Value::Table(table)) => Some(table),
            _ => None,
        };
        let (base_table, radiant_table) = (face_table(Face::Base), face_table(Face::Radiant));

        for pair in module.pairs::<String, Value>() {
            let (key, value) = pair.map_err(|error| refused(error.to_string()))?;
            match key.as_str() {
                "base" => {
                    let Value::Table(table) = value else {
                        return Err(refused(format!(
                            "base is of type {}, not a table of hooks",
                            value.type_name()
                        )));
                    };
                    base_script = Some(build_script(
                        card,
                        Face::Base,
                        &table,
                        bytecode,
                        &mut hook_cache,
                        radiant_table.as_ref(),
                    )?);
                }
                "radiant" => match value {
                    Value::String(s) => {
                        if s.to_str().is_ok_and(|str_val| str_val == "base") {
                            radiant_script = None; // will clone base
                        } else {
                            return Err(refused(format!(
                                "radiant string must be \"base\", got {:?}",
                                s.to_string_lossy()
                            )));
                        }
                    }
                    Value::Table(table) => {
                        radiant_script = Some(build_script(
                            card,
                            Face::Radiant,
                            &table,
                            bytecode,
                            &mut hook_cache,
                            base_table.as_ref(),
                        )?);
                    }
                    _ => {
                        return Err(refused(format!(
                            "radiant is of type {}, not a table of hooks",
                            value.type_name()
                        )));
                    }
                },
                _ => {
                    return Err(refused(format!(
                        "the module declares {key:?}, which is not base or radiant"
                    )));
                }
            }
        }

        let base = base_script.ok_or_else(|| refused("the module declares no base face".to_string()))?;
        let radiant = radiant_script.unwrap_or_else(|| base.clone());

        Ok(CardScripts { base, radiant })
    })
}

/// The direct hooks' adapters a module's faces built so far: each with the pointer of the Luau
/// function it runs and the face it runs, `None` for one that runs the face the context names
/// (`build_script`).
pub type HookCache = Vec<(*const std::ffi::c_void, Option<Face>, Hook)>;

/// Builds one `Script` for a face from its table. `hook_cache` is `load_card`'s, shared by both
/// faces, so one function value held by two hook keys gets one adapter (#464 gaps 10 and 14);
/// `other` is the other face's table, none when the Radiant face is `"base"` or either face is
/// missing.
pub fn build_script(
    card: &'static str,
    face: Face,
    table: &Table,
    bytecode: &'static [u8],
    hook_cache: &mut HookCache,
    other: Option<&Table>,
) -> Result<Script, LuauError> {
    let refused = |message: String| LuauError::Load {
        card: card.to_string(),
        message,
    };

    let mut script = Script::default();

    // Verify all keys first
    for pair in table.pairs::<String, Value>() {
        let (name, value) = pair.map_err(|error| refused(error.to_string()))?;
        let is_hook = vm::HOOKS
            .iter()
            .any(|h| *h == name || hooks::camel_to_snake(h) == name || hooks::snake_to_camel(h) == name);
        let is_decl = vm::DECLARATIONS
            .iter()
            .any(|d| *d == name || hooks::camel_to_snake(d) == name || hooks::snake_to_camel(d) == name);
        // A named sub-hook beside the hooks (`after_check`, `draw_more`, …, `vm::SUB_HOOKS`):
        // called back by name, never adapted into `Script`.
        let is_sub_hook = vm::is_sub_hook(&name);
        if !is_hook && !is_decl && !is_sub_hook {
            return Err(refused(format!(
                "{} declares {name:?}, which is not a hook",
                face.key()
            )));
        }
        if (is_hook || is_sub_hook) && !value.is_function() {
            return Err(refused(format!(
                "{} {name} is of type {}, not a function",
                face.key(),
                value.type_name()
            )));
        }
    }

    let site_load = Site {
        card,
        face,
        hook: "load",
    };

    // 1. Declarations: targets
    if let Ok(Some(val)) = table.raw_get::<Option<Value>>("targets") {
        let json = numbers::to_json(&val, site_load)?;
        script.targets = serde_json::from_value::<Vec<TargetDecl>>(json)
            .map_err(|e| refused(format!("{} targets: {e}", face.key())))?;
    }

    // 2. Declarations: modes
    if let Ok(Some(val)) = table.raw_get::<Option<Value>>("modes") {
        let json = numbers::to_json(&val, site_load)?;
        script.modes = serde_json::from_value::<Vec<ModeDecl>>(json)
            .map_err(|e| refused(format!("{} modes: {e}", face.key())))?;
    }

    // 3. Declarations: secretModes / secret_modes
    if let Ok(Some(b)) = table
        .raw_get::<Option<bool>>("secretModes")
        .or_else(|_| table.raw_get::<Option<bool>>("secret_modes"))
    {
        script.secret_modes = b;
    }

    // 4. Declarations: staticFlags / static_flags
    if let Ok(Some(val)) = table
        .raw_get::<Option<Value>>("staticFlags")
        .or_else(|_| table.raw_get::<Option<Value>>("static_flags"))
    {
        let json = numbers::to_json(&val, site_load)?;
        script.static_flags = Some(
            serde_json::from_value::<StaticFlags>(json)
                .map_err(|e| refused(format!("{} static_flags: {e}", face.key())))?,
        );
    }

    // 5. Declarations: activations
    if let Ok(Some(acts_tbl)) = hooks::get_table(table, "activations") {
        let len = acts_tbl.raw_len();
        for i in 0..len {
            let act: Table = acts_tbl.raw_get(i + 1).map_err(|e| refused(e.to_string()))?;
            let id: String = act.raw_get("id").unwrap_or_default();
            let label: String = act.raw_get("label").unwrap_or_default();

            // A count crosses the boundary as every number does (L6, #871): a fraction or a value
            // outside `i32` is refused, naming the card and face.
            let uses = match act.raw_get::<Value>("uses") {
                Ok(Value::String(s)) if s.to_str().is_ok_and(|v| v == "unlimited") => {
                    ActivationUses::Unlimited
                }
                Ok(count @ (Value::Integer(_) | Value::Number(_))) => {
                    ActivationUses::Count(numbers::to_i32(&count, site_load)?)
                }
                _ => ActivationUses::Count(1),
            };

            let cost = if let Ok(Some(cost_val)) = act.raw_get::<Option<Value>>("cost") {
                let json = numbers::to_json(&cost_val, site_load)?;
                Some(
                    serde_json::from_value::<ActivationCost>(json)
                        .map_err(|e| refused(format!("{} activations cost: {e}", face.key())))?,
                )
            } else {
                None
            };

            let targets = if let Ok(Some(t_val)) = act.raw_get::<Option<Value>>("targets") {
                let json = numbers::to_json(&t_val, site_load)?;
                serde_json::from_value::<Vec<TargetDecl>>(json)
                    .map_err(|e| refused(format!("{} activations targets: {e}", face.key())))?
            } else {
                Vec::new()
            };

            let modes = if let Ok(Some(m_val)) = act.raw_get::<Option<Value>>("modes") {
                let json = numbers::to_json(&m_val, site_load)?;
                serde_json::from_value::<Vec<ModeDecl>>(json)
                    .map_err(|e| refused(format!("{} activations modes: {e}", face.key())))?
            } else {
                Vec::new()
            };

            let site_act = Site {
                card,
                face,
                hook: "activation",
            };

            let can_activate = if hooks::get_fn(&act, "canActivate").is_ok() {
                Some(hooks::adapt_activation_can_activate(site_act, bytecode, i))
            } else {
                None
            };

            let has = if hooks::get_fn(&act, "has").is_ok() {
                Some(hooks::adapt_activation_has(site_act, bytecode, i))
            } else {
                None
            };

            let run = hooks::adapt_activation_run(site_act, bytecode, i);

            script.activations.push(ActivationDecl {
                id,
                label,
                uses,
                cost,
                targets,
                modes,
                can_activate,
                has,
                run,
            });
        }
    }

    // 6. Declarations: triggers (triggers, handTriggers, deckTriggers, graveyardTriggers)
    script.triggers = parse_triggers(card, face, table, "triggers", bytecode)?;
    script.hand_triggers = parse_triggers(card, face, table, "handTriggers", bytecode)?;
    script.deck_triggers = parse_triggers(card, face, table, "deckTriggers", bytecode)?;
    script.graveyard_triggers = parse_triggers(card, face, table, "graveyardTriggers", bytecode)?;

    // 7. Declarations: replacements
    if let Ok(Some(reps_tbl)) = hooks::get_table(table, "replacements") {
        let len = reps_tbl.raw_len();
        for i in 0..len {
            let rep: Table = reps_tbl.raw_get(i + 1).map_err(|e| refused(e.to_string()))?;
            let id: String = rep.raw_get("id").unwrap_or_default();
            let on: ReplacementMoment = {
                let on_val: Value = rep.raw_get("on").map_err(|e| refused(e.to_string()))?;
                let json = numbers::to_json(&on_val, site_load)?;
                serde_json::from_value(json)
                    .map_err(|e| refused(format!("{} replacements on: {e}", face.key())))?
            };
            let where_: Option<ReplacementWhere> = if let Ok(Some(w_val)) = rep
                .raw_get::<Option<Value>>("where")
                .or_else(|_| rep.raw_get::<Option<Value>>("where_"))
            {
                let json = numbers::to_json(&w_val, site_load)?;
                Some(
                    serde_json::from_value(json)
                        .map_err(|e| refused(format!("{} replacements where: {e}", face.key())))?,
                )
            } else {
                None
            };

            let site_rep = Site {
                card,
                face,
                hook: "replacement",
            };

            let when = if hooks::get_fn(&rep, "when").is_ok() {
                Some(hooks::adapt_replacement_when(site_rep, bytecode, i))
            } else {
                None
            };

            let instead = {
                let inst_val: Value = rep.raw_get("instead").map_err(|e| refused(e.to_string()))?;
                let json = numbers::to_json(&inst_val, site_load)?;
                serde_json::from_value(json)
                    .map_err(|e| refused(format!("{} replacements instead: {e}", face.key())))?
            };

            let then: Option<String> = rep.raw_get("then").ok();
            let by: Option<jackioh_engine::script::ReplacementBy> =
                if let Ok(Some(by_val)) = rep.raw_get::<Option<Value>>("by") {
                    let json = numbers::to_json(&by_val, site_load)?;
                    Some(
                        serde_json::from_value(json)
                            .map_err(|e| refused(format!("{} replacements by: {e}", face.key())))?,
                    )
                } else {
                    None
                };

            script.replacements.push(ReplacementDef {
                id,
                on,
                where_,
                when,
                instead,
                then,
                by,
            });
        }
    }

    // 8. Declarations: targetChecks
    if let Ok(Some(tc_tbl)) = hooks::get_table(table, "targetChecks") {
        for pair in tc_tbl.pairs::<String, Function>() {
            let (name, _) = pair.map_err(|e| refused(e.to_string()))?;
            let leaked_name: &'static str = Box::leak(name.into_boxed_str());
            let site = Site {
                card,
                face,
                hook: leaked_name,
            };
            script.target_checks.insert(
                leaked_name,
                hooks::adapt_target_check(site, bytecode, leaked_name),
            );
        }
    }

    // 9. Declarations: resume
    if let Ok(Some(res_tbl)) = hooks::get_table(table, "resume") {
        for pair in res_tbl.pairs::<String, Function>() {
            let (name, _) = pair.map_err(|e| refused(e.to_string()))?;
            let leaked_name: &'static str = Box::leak(name.into_boxed_str());
            let site = Site {
                card,
                face,
                hook: leaked_name,
            };
            script.resume.insert(
                leaked_name,
                hooks::adapt_sub_hook(site, bytecode, "resume", leaked_name),
            );
        }
    }

    // 10. Declarations: grants
    if let Ok(Some(grants_tbl)) = hooks::get_table(table, "grants") {
        for pair in grants_tbl.pairs::<String, Function>() {
            let (name, _) = pair.map_err(|e| refused(e.to_string()))?;
            let leaked_name: &'static str = Box::leak(name.into_boxed_str());
            let site = Site {
                card,
                face,
                hook: leaked_name,
            };
            script.grants.insert(
                leaked_name,
                hooks::adapt_sub_hook(site, bytecode, "grants", leaked_name),
            );
        }
    }

    // Direct hooks. One adapter for one function under two hooks (#464 gaps 10 and 14):
    // when two hook keys hold the same Luau function value, both share one `Arc`, so
    // `Arc::ptr_eq` holds the way it does for a Rust script that clones one hook. The cache
    // is `load_card`'s, shared by both faces. An adapter looks its function up under the key it
    // was built for, on the face it runs, so it is shared only where that face holds the function
    // under that key (#871). A hook whose function the other face holds under the same key gets
    // an adapter that runs the face the context names, shared with every such hook of either face;
    // any other gets one that runs its own face's function, as a Rust face runs its own closure
    // whatever the context says (a Radiant ingredient fused into a base host, #765), shared with
    // its face's other hooks holding that function.
    let mut direct_hook = |key: &'static str| -> Option<Hook> {
        let func = hooks::get_fn(table, key).ok()?;
        let ptr = func.to_pointer();
        let across_faces =
            other.is_some_and(|other| hooks::get_fn(other, key).is_ok_and(|held| held.to_pointer() == ptr));
        let runs = (!across_faces).then_some(face);
        if let Some((.., shared)) = hook_cache
            .iter()
            .find(|(cached, cached_runs, _)| *cached == ptr && *cached_runs == runs)
        {
            return Some(shared.clone());
        }
        let site = Site {
            card,
            face,
            hook: key,
        };
        let adapter = hooks::adapt_hook(site, bytecode, across_faces);
        hook_cache.push((ptr, runs, adapter.clone()));
        Some(adapter)
    };

    let has_fn = |name: &str| hooks::get_fn(table, name).is_ok();
    let site = |hook: &'static str| Site { card, face, hook };

    if has_fn("cost") {
        script.cost = Some(hooks::adapt_cost_hook(site("cost"), bytecode));
    }
    if let Some(hook) = direct_hook("cry") {
        script.cry = Some(hook);
    }
    if let Some(hook) = direct_hook("death") {
        script.death = Some(hook);
    }
    if let Some(hook) = direct_hook("entersHand") {
        script.enters_hand = Some(hook);
    }
    if let Some(hook) = direct_hook("delayed") {
        script.delayed = Some(hook);
    }
    if has_fn("setStat") {
        script.set_stat = Some(hooks::adapt_set_stat_hook(site("setStat"), bytecode));
    }
    if let Some(hook) = direct_hook("startOfTurn") {
        script.start_of_turn = Some(hook);
    }
    if let Some(hook) = direct_hook("endOfTurn") {
        script.end_of_turn = Some(hook);
    }
    if has_fn("aura") {
        script.aura = Some(hooks::adapt_aura_hook(site("aura"), bytecode));
    }
    if let Some(hook) = direct_hook("onPlayHook") {
        script.on_play_hook = Some(hook);
    }
    if has_fn("conditionMet") {
        script.condition_met = Some(hooks::adapt_condition_hook(site("conditionMet"), bytecode));
    }
    if has_fn("preview") {
        script.preview = Some(hooks::adapt_preview_hook(site("preview"), bytecode));
    }
    if has_fn("costAura") {
        script.cost_aura = Some(hooks::adapt_cost_aura_hook(site("costAura"), bytecode));
    }
    if has_fn("graveyardPlay") {
        script.graveyard_play = Some(hooks::adapt_graveyard_play_hook(site("graveyardPlay"), bytecode));
    }
    if has_fn("revealEcho") {
        script.reveal_echo = Some(hooks::adapt_reveal_echo_hook(site("revealEcho"), bytecode));
    }
    if has_fn("targetingDiscards") {
        script.targeting_discards = Some(hooks::adapt_targeting_discards_hook(
            site("targetingDiscards"),
            bytecode,
        ));
    }
    if has_fn("drawLimit") {
        script.draw_limit = Some(hooks::adapt_draw_limit_hook(site("drawLimit"), bytecode));
    }
    if has_fn("discardGuard") {
        script.discard_guard = Some(hooks::adapt_discard_guard_hook(site("discardGuard"), bytecode));
    }
    if has_fn("heroGuard") {
        script.hero_guard = Some(hooks::adapt_hero_guard_hook(site("heroGuard"), bytecode));
    }
    if has_fn("attackMods") {
        script.attack_mods = Some(hooks::adapt_attack_mod_hook(site("attackMods"), bytecode));
    }
    if has_fn("conditionalKeywords") {
        script.conditional_keywords = Some(hooks::adapt_conditional_keywords_hook(
            site("conditionalKeywords"),
            bytecode,
        ));
    }
    if let Some(hook) = direct_hook("afterAttack") {
        script.after_attack = Some(hook);
    }
    if let Some(hook) = direct_hook("afterAttacked") {
        script.after_attacked = Some(hook);
    }
    if has_fn("plagueMultiplier") {
        script.plague_multiplier = Some(hooks::adapt_plague_multiplier_hook(
            site("plagueMultiplier"),
            bytecode,
        ));
    }
    if has_fn("echoX") {
        script.echo_x = Some(hooks::adapt_echo_x_hook(site("echoX"), bytecode));
    }
    if has_fn("tributeWhen") {
        script.tribute_when = Some(hooks::adapt_tribute_when_hook(site("tributeWhen"), bytecode));
    }
    if has_fn("wouldCounter") {
        script.would_counter = Some(hooks::adapt_would_counter_hook(site("wouldCounter"), bytecode));
    }
    if let Some(hook) = direct_hook("startOfOpponentTurn") {
        script.start_of_opponent_turn = Some(hook);
    }
    if has_fn("turnHookExtra") {
        script.turn_hook_extra = Some(hooks::adapt_trigger_extra_hook(
            site("turnHookExtra"),
            bytecode,
            "turnHookExtra",
        ));
    }
    if has_fn("cryDeathExtra") {
        script.cry_death_extra = Some(hooks::adapt_trigger_extra_hook(
            site("cryDeathExtra"),
            bytecode,
            "cryDeathExtra",
        ));
    }

    Ok(script)
}

fn parse_triggers(
    card: &'static str,
    face: Face,
    table: &Table,
    key: &'static str,
    bytecode: &'static [u8],
) -> Result<Vec<TriggerDef>, LuauError> {
    let refused = |message: String| LuauError::Load {
        card: card.to_string(),
        message,
    };
    let mut triggers = Vec::new();
    if let Ok(Some(trigs_tbl)) = hooks::get_table(table, key) {
        let len = trigs_tbl.raw_len();
        let site_load = Site {
            card,
            face,
            hook: "load",
        };
        for i in 0..len {
            let trig: Table = trigs_tbl.raw_get(i + 1).map_err(|e| refused(e.to_string()))?;
            let id: String = trig.raw_get("id").unwrap_or_default();
            let on_val: Value = trig.raw_get("on").map_err(|e| refused(e.to_string()))?;
            let json = numbers::to_json(&on_val, site_load)?;
            let on: Vec<GameEventType> =
                serde_json::from_value(json).map_err(|e| refused(format!("{} {key} on: {e}", face.key())))?;

            let site_trig = Site {
                card,
                face,
                hook: key,
            };

            let when = if hooks::get_fn(&trig, "when").is_ok() {
                Some(hooks::adapt_trigger_when(site_trig, bytecode, key, i))
            } else {
                None
            };

            let run = hooks::adapt_trigger_run(site_trig, bytecode, key, i);

            triggers.push(TriggerDef { id, on, when, run });
        }
    }
    Ok(triggers)
}
