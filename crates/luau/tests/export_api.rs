//! Generates `crates/cards/luau/jackioh.luau` from ts-rs definitions and the verb binding table.
//! Run with: `cargo test -p jackioh-luau --test export_api --features ts`

#[cfg(feature = "ts")]
mod generator {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use jackioh_engine::effects::*;
    use jackioh_engine::script::*;
    use jackioh_engine::wire::*;
    use jackioh_luau::api::after_check::AfterStateCheckArgs;
    use jackioh_luau::api::aura::DefenseArmorArgs;
    use jackioh_luau::api::cast::CastEachArgs;
    use jackioh_luau::api::draw_while::DrawWhileArgs;
    use jackioh_luau::api::each::ForEachCardArgs;
    use jackioh_luau::api::kill_credit::WithKillCreditArgs;
    use jackioh_luau::api::random_entries::DoRandomEntriesArgs;
    use jackioh_luau::api::{all_specs, all_verbs};
    use ts_rs::TS;

    fn repo_root() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root")
            .to_path_buf()
    }

    /// Converts a TypeScript declaration into Luau.
    fn ts_to_luau(ts_decl: &str) -> String {
        let mut lines = Vec::new();
        for line in ts_decl.lines() {
            let mut l = line.to_string();

            // Semicolons at end of statements
            if l.ends_with(';') {
                l.pop();
            }

            // type Foo = ... -> export type Foo = ...
            if l.starts_with("type ") {
                l = format!("export {l}");
            }

            // Record<K, V> -> {[K]: V}
            while let Some(start) = l.find("Record<") {
                if let Some(end) = l[start..].find('>') {
                    let inner = &l[start + 7..start + end];
                    if let Some(comma) = inner.find(',') {
                        let k = inner[..comma].trim();
                        let v = inner[comma + 1..].trim();
                        l = format!("{}{{[{k}]: {v}}}{}", &l[..start], &l[start + end + 1..]);
                        continue;
                    }
                }
                break;
            }

            // Array<T> -> {T}
            while let Some(start) = l.find("Array<") {
                let mut depth = 0;
                let mut end_idx = None;
                for (i, c) in l[start + 6..].char_indices() {
                    if c == '<' {
                        depth += 1;
                    } else if c == '>' {
                        if depth == 0 {
                            end_idx = Some(start + 6 + i);
                            break;
                        }
                        depth -= 1;
                    }
                }
                if let Some(end) = end_idx {
                    let inner = &l[start + 6..end];
                    l = format!("{}{{{inner}}}{}", &l[..start], &l[end + 1..]);
                } else {
                    break;
                }
            }

            // null / undefined -> nil
            l = l.replace("null", "nil").replace("undefined", "nil");

            // field?: Type -> field: Type?
            // Handle optional fields in table types
            if let Some(colon_idx) = l.find("?:") {
                let field_part = l[..colon_idx].trim_end();
                let type_part = l[colon_idx + 2..].trim_start();
                let indent_len = l.len() - l.trim_start().len();
                let indent = &l[..indent_len];

                // If type_part ends with a comma, handle comma
                let (type_body, comma_suffix) = if let Some(stripped) = type_part.strip_suffix(',') {
                    (stripped.trim_end(), ",")
                } else {
                    (type_part, "")
                };

                l = format!("{indent}{}: {type_body}?{comma_suffix}", field_part.trim_start());
            }

            lines.push(l);
        }
        lines.join("\n")
    }

    #[test]
    fn export_jackioh_luau_api() {
        let mut decls = BTreeMap::new();

        macro_rules! reg {
            ( $( $t:ty ),* $(,)? ) => {
                $(
                    decls.insert(<$t as TS>::name(), <$t as TS>::decl());
                )*
            };
        }

        // Wire types
        reg! {
            CardType,
            Row,
            Position,
            PlayerId,
            ZoneName,
            OffFieldZone,
            TargetRef,
            TargetDecl,
            TargetFilter,
            ModeDecl,
            Keyword,
            Tag,
            Selection,
            PreviewValue,
            StatMod,
            TuneMultiplier,
            StaticFlags,
            ActivationCost,
            ActivationDecl,
            ActivationUses,
            GameEventType,
            TriggerDef,
            ReplacementMoment,
            ReplacementWhere,
            ReplacementInstead,
            InsteadRedirect,
            InsteadDamage,
            InsteadLasting,
            InsteadFlicker,
            InsteadTo,
            ReplacementBy,
            ReplacementDef,
            ReplacedEvent,
            CostAuraWhose,
            CostAura,
            CostRule,
            GraveyardPlayPermission,
            FaceDownPlayPermission,
            HeroGuard,
            DrawLimitPlayer,
            DrawLimit,
            AttackMod,
            SetStat,
            FlagOrCount,
            DyingUnit,
            HealedRef,
            TargetedWhat,
            ConditionZone,
        }

        // Host types
        reg! {
            AfterStateCheckArgs,
            DefenseArmorArgs,
            CastEachArgs,
            DrawWhileArgs,
            ForEachCardArgs,
            WithKillCreditArgs,
            DoRandomEntriesArgs,
        }

        // Engine verb argument types
        reg! {
            AddToHandArgs,
            TakeFromLibraryArgs,
            CaptureArgs,
            AddFromLastBoardArgs,
            AnimateArgs,
            GiveBrittleArgs,
            GainBrittleArgs,
            BuffArgs,
            BuffRandomUnitArgs,
            BuffAllUnitsArgs,
            BuffCardsArgs,
            SetCostOverrideArgs,
            SetCostModArgs,
            GrantKeywordCardsArgs,
            GrantKeywordArgs,
            GrantKeywordsRandomUnitArgs,
            GrantRandomKeywordsArgs,
            GrantAbilityArgs,
            ZoneScope,
            BoardScope,
            CastArgs,
            CastRandomArgs,
            CastNewArgs,
            CastStorm,
            Storm,
            ChooseTargetArgs,
            ChooseFromHandArgs,
            ChoosePickArgs,
            ChooseModeArgs,
            ChooseRewardArgs,
            ChooseNumberArgs,
            ChooseCostInHandArgs,
            ChooseCraftArgs,
            ChooseAnswerArgs,
            ChooseTargetWhereArgs,
            ChooseCellArgs,
            FlipCoinsArgs,
            FlipCoinKeywordArgs,
            RedirectAttackArgs,
            CancelAttackArgs,
            MayAttackAgainArgs,
            ForcedAttackRandomArgs,
            ForcedAttackOwnHeroArgs,
            ForcedAttacksArgs,
            ForcedAttacksOnArgs,
            DamageEnemyOrHealFriendArgs,
            DiscountRandomInHandArgs,
            PlacePlagueTokensArgs,
            PlacePlagueArgs,
            ClearPlagueArgs,
            ConsumePlagueArgs,
            PlacePlagueRandomArgs,
            PlacePlagueEachArgs,
            CraftCardArgs,
            TriggerCryArgs,
            DamageEffectArgs,
            DamageAllArgs,
            DamageSplitArgs,
            RollBackArgs,
            DelayArgs,
            DestroyArgs,
            DestroyAdjacentToArgs,
            DestroyAtNextTurnStartArgs,
            DestroyFieldSpellsAndHitArgs,
            SacrificeArgs,
            DrawArgs,
            DrawPricedArgs,
            DrawFromOpponentArgs,
            DrawFromLibraryArgs,
            DiscoverFromCatalogArgs,
            DiscoverFromGraveyardArgs,
            DiscoverFromLastBoardArgs,
            DiscoverFromLibraryArgs,
            DiscoverNumberArgs,
            AddRandomFromCatalogArgs,
            AddRandomFromGraveyardArgs,
            AddRandomFromLastBoardArgs,
            DrawWhileCheapArgs,
            EnchantArgs,
            EnchantNextSpellArgs,
            FlickerArgs,
            AddRolledGrapesArgs,
            FuseOntoYourCardArgs,
            FuseCardsArgs,
            FuseGeneratedArgs,
            FuseRandomIntoArgs,
            GiveFromHandArgs,
            GrantTagArgs,
            ExileHandArgs,
            DiscardHandArgs,
            ExileRandomFromHandArgs,
            DiscardRandomArgs,
            DiscardArgs,
            DiscardHandAtTurnEndArgs,
            HealArgs,
            ConvertHealingArgs,
            SetHealthArgs,
            MakeHeroImmuneArgs,
            GainHeroArmorArgs,
            AddJadeArgs,
            FillBoardRandomArgs,
            FillBoardArgs,
            ExileBottomOfLibraryArgs,
            ExileRandomFromLibraryArgs,
            AddLibraryCopiesArgs,
            LockLaneArgs,
            LockPlayedZoneArgs,
            UnlockArgs,
            LockArgs,
            WatchLaneArgs,
            LoseHealthArgs,
            GainManaArgs,
            RefreshManaArgs,
            NextTurnManaArgs,
            LoseRefreshesArgs,
            RememberArgs,
            RememberRandomArgs,
            AddRolledArgs,
            BounceArgs,
            SetHandCapArgs,
            PlagueArgs,
            AddPlayerModifierArgs,
            ForRestOfGameArgs,
            SwitchPositionOfArgs,
            SwitchAllPositionsArgs,
            SetRadiantRandomArgs,
            RadiantChanceArgs,
            RadiantTarget,
            RecruitArgs,
            RecruitAllArgs,
            RevealArgs,
            RotateArgs,
            EndTurnArgs,
            EndTurnAfterActionsArgs,
            AiPlaysOutTurnArgs,
            KeepSecretArgs,
            ResolveSecretArgs,
            GuessSecretArgs,
            ShuffleCardIntoArgs,
            ShuffleIntoArgs,
            ShuffleCopiesOfSelfArgs,
            ShuffleRandomFromCatalogArgs,
            GoBerserkArgs,
            CounterPlayArgs,
            CounterArgs,
            StealTarget,
            StealAllArgs,
            SummonArgs,
            SummonCopyArgs,
            SummonRandomFromHandArgs,
            SummonRandomArgs,
            SwapArgs,
            SwapBookArgs,
            TransformArgs,
            TransformTarget,
            TransformBeneathArgs,
            TransformRandomArgs,
            TranslateArgs,
            TuneArgs,
            SetNumberArgs,
            TriggerTurnHooksArgs,
            TakeExtraTurnArgs,
            WinGameArgs,
            AltWinArgs,
        }

        let mut out = String::new();
        out.push_str("--!strict\n");
        out.push_str("-- Generated by crates/luau/tests/export_api.rs. DO NOT EDIT DIRECTLY.\n\n");

        out.push_str("export type EffectCall = {\n\tverb: string,\n\targs: any,\n}\n\n");

        // Context type
        out.push_str("export type EffectContext = {\n");
        out.push_str("\tcontroller: (self: EffectContext) -> PlayerId,\n");
        out.push_str("\tradiant: (self: EffectContext) -> boolean,\n");
        out.push_str("\tx: (self: EffectContext) -> number,\n");
        out.push_str("\tembiggened: (self: EffectContext) -> boolean,\n");
        out.push_str("\tmana_before_play: (self: EffectContext) -> number?,\n");
        out.push_str("\tself_: (self: EffectContext) -> any,\n");
        out.push_str("\tlive_self: (self: EffectContext) -> any,\n");
        out.push_str("\tstands_since_script_began: (self: EffectContext, id: string) -> boolean,\n");
        out.push_str("\topponent_of: (self: EffectContext, player: PlayerId) -> PlayerId,\n");
        out.push_str("\tparam: (self: EffectContext, key: string) -> number,\n");
        out.push_str("\tparam_value: (self: EffectContext, key: string) -> number,\n");
        out.push_str("\tparams_view: (self: EffectContext) -> any,\n");
        out.push_str("\trecalled: (self: EffectContext, key: string) -> any,\n");
        out.push_str("\tchosen_options: (self: EffectContext) -> {string},\n");
        out.push_str("\tchosen_target: (self: EffectContext) -> any,\n");
        out.push_str("\tchosen_number: (self: EffectContext) -> number?,\n");
        out.push_str("\tchosen_cards: (self: EffectContext) -> {string},\n");
        out.push_str("\tchosen_cells: (self: EffectContext) -> {any},\n");
        out.push_str("\tchosen_player: (self: EffectContext) -> PlayerId?,\n");
        out.push_str("\tzone_cards: (self: EffectContext, player: PlayerId, zone: any) -> {any},\n");
        out.push_str("\tactive_units_of: (self: EffectContext, player: PlayerId) -> {any},\n");
        out.push_str("\thero_of: (self: EffectContext, player: PlayerId) -> any,\n");
        out.push_str("\tkiller_of: (self: EffectContext, card: any) -> any,\n");
        out.push_str("\tplague_on: (self: EffectContext, id: string) -> number,\n");
        out.push_str("\tis_friendly: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tis_enemy: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tis_unit: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tis_spell: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tis_trap: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tis_field_spell: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tunit_has: (self: EffectContext, card: any, kw: string) -> boolean,\n");
        out.push_str("\tcard_keywords: (self: EffectContext, card: any) -> {string},\n");
        out.push_str("\tunspent_mana_of: (self: EffectContext, player: PlayerId) -> number,\n");
        out.push_str("\tmax_mana_of: (self: EffectContext, player: PlayerId) -> number,\n");
        out.push_str("\tcards_played_this_turn: (self: EffectContext, player: PlayerId) -> number,\n");
        out.push_str("\tzone_count: (self: EffectContext, player: PlayerId, zone: any) -> number,\n");
        out.push_str("\tstats_with_buffs: (self: EffectContext, card: any) -> any,\n");
        out.push_str("\twin_rate_leader: (self: EffectContext, player: PlayerId) -> any,\n");
        out.push_str("\twas_played_this_turn: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tsummoned_so_far: (self: EffectContext) -> {string},\n");
        out.push_str("\ttags_of: (self: EffectContext, card: any) -> {string},\n");
        out.push_str("\tnumbered_keywords_on: (self: EffectContext, card: any, kw: string) -> {number},\n");
        out.push_str("\trng_coin: (self: EffectContext) -> boolean,\n");
        out.push_str("\trng_range: (self: EffectContext, low: number, high: number) -> number,\n");
        out.push_str("\tsubsystems_activation_paid: (self: EffectContext, id: string) -> any,\n");
        out.push_str("\tsubsystems_tributes_paid: (self: EffectContext, id: string) -> any,\n");
        out.push_str("\tsubsystems_borrowed_deck: (self: EffectContext, player: PlayerId) -> any,\n");
        out.push_str("\tsubsystems_last_turn_played: (self: EffectContext, card: any) -> number?,\n");
        out.push_str("\tsubsystems_exile_seen: (self: EffectContext, player: PlayerId) -> {string},\n");
        out.push_str("\tsubsystems_graveyard_seen: (self: EffectContext, player: PlayerId) -> {string},\n");
        out.push_str("\tsubsystems_hand_seen: (self: EffectContext, player: PlayerId) -> {string},\n");
        out.push_str("\tsubsystems_deck_seen: (self: EffectContext, player: PlayerId) -> {string},\n");
        out.push_str("\tsubsystems_last_turn_drawn: (self: EffectContext, card: any) -> number?,\n");
        out.push_str("\tsubsystems_is_degraded: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("\tsubsystems_is_upgraded: (self: EffectContext, card: any) -> boolean,\n");
        out.push_str("}\n\n");

        // Emit all translated types
        for (name, ts_decl) in &decls {
            let luau_decl = ts_to_luau(ts_decl);
            out.push_str(&luau_decl);
            out.push_str("\n\n");
        }

        // Emit J module definition
        out.push_str("local J = {}\n\n");

        let specs = all_specs();
        let mut verb_map = BTreeMap::new();
        for (verb, arg_opt) in specs {
            verb_map.insert(verb, arg_opt);
        }

        for (verb, arg_opt) in verb_map {
            if let Some(arg_type) = arg_opt {
                out.push_str(&format!(
                    "function J.{verb}(args: {arg_type}): EffectCall\n\treturn {{ verb = \"{verb}\", args = args :: any }}\nend\n\n"
                ));
            } else {
                out.push_str(&format!(
                    "function J.{verb}(args: any?): EffectCall\n\treturn {{ verb = \"{verb}\", args = args :: any }}\nend\n\n"
                ));
            }
        }

        // Special / helper functions
        out.push_str("function J.div(a: number, b: number): number\n\treturn (a // b)\nend\n\n");
        out.push_str("function J.rem(a: number, b: number): number\n\treturn (a % b)\nend\n\n");
        out.push_str("function J.sort_by<T>(list: {T}, key_fn: (item: T) -> any): {T}\n\treturn list\nend\n\n");
        out.push_str("function J.query(args: any): {string}\n\treturn {}\nend\n\n");
        out.push_str("function J.pool(name: string): {string}\n\treturn {}\nend\n\n");
        out.push_str("function J.cost(defId: string): number\n\treturn 0\nend\n\n");
        out.push_str("function J.defense_armor_aura(args: { armor: number }): EffectCall\n\treturn { verb = \"defense_armor_aura\", args = args :: any }\nend\n\n");
        out.push_str("J.TRAP_TYPES = { \"Trap\", \"Field Trap\" }\n\n");

        out.push_str("return J\n");

        let target_path = repo_root().join("crates/cards/luau/jackioh.luau");
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent).expect("creates luau dir");
        }

        std::fs::write(&target_path, &out).expect("writes jackioh.luau");
        println!("Exported {} bytes to {}", out.len(), target_path.display());
    }
}
