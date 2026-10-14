//! Generates `crates/cards/luau/jackioh.luau` from ts-rs definitions and the verb binding table.
//! Run with: `cargo test -p jackioh-luau --features ts --test export_api`. The feature is off by
//! default, as the engine's `ts` is, so no build of the cards (the server's, the browser's) turns
//! the engine's on; `tests/api.rs` holds the committed module to the verbs and readers meanwhile.
#![cfg(feature = "ts")]
#![allow(
    clippy::disallowed_methods,
    reason = "a generator, not the rules: it writes the committed typed module, as build.rs writes the registry"
)]

mod generator {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use jackioh_engine::effects::*;
    use jackioh_engine::script::*;
    use jackioh_engine::wire::*;
    use jackioh_engine::{
        AltWinCondition, AttackAmong, CastAfterward, CatalogQueryArgs, CostRule, GameCounters, LibraryEnd,
        ModifierExpiry, NumberRef, NumberedKey, OffFieldZone, Resume, Zone,
    };
    use jackioh_luau::api::after_check::AfterStateCheckArgs;
    use jackioh_luau::api::all_specs;
    use jackioh_luau::api::aura::{AuraApplies, AuraArgs, AuraFilter, CloakHandAuraArgs, DefenseArmorArgs};
    use jackioh_luau::api::cast::CastEachArgs;
    use jackioh_luau::api::draw_while::DrawWhileArgs;
    use jackioh_luau::api::each::{CardsSource, ForEachCardArgs};
    use jackioh_luau::api::kill_credit::WithKillCreditArgs;
    use jackioh_luau::api::radiant::RadiantChanceLuauArgs;
    use jackioh_luau::api::random_entries::DoRandomEntriesArgs;
    use jackioh_luau::api::random_entries::RandomEntryLuau;
    use jackioh_luau::reads::TurnView;
    use ts_rs::TS;

    /// The pure-read contexts the hand-written block declares, in its order: each is registered
    /// with `shared_readers!` in crates/luau/src/reads.rs.
    const PURE_READ_CONTEXTS: [&str; 5] = [
        "ConditionContext",
        "ReplacementContext",
        "HookArgs",
        "CostArgs",
        "TargetCheckArgs",
    ];

    /// The readers `shared_readers!` (crates/luau/src/reads.rs) registers on every context with a
    /// state, in its order, read off the macro's source so the typed module follows the host.
    fn shared_reader_names() -> Vec<&'static str> {
        const READS: &str = include_str!("../src/reads.rs");
        let start = READS
            .find("macro_rules! shared_readers {")
            .expect("reads.rs defines shared_readers!");
        let body = &READS[start..];
        let body = &body[..body.find("\n}\n").expect("shared_readers! closes")];
        let mut names = Vec::new();
        for piece in body.split("$registry.add_method(").skip(1) {
            let piece = piece
                .trim_start()
                .strip_prefix('"')
                .expect("a reader's name is a string literal");
            let name = &piece[..piece.find('"').expect("a reader's name is a string literal")];
            if !names.contains(&name) {
                names.push(name);
            }
        }
        assert!(
            !names.is_empty(),
            "shared_readers! registers no reader the generator can read"
        );
        names
    }

    /// `block` with every shared reader added to each pure-read context's type that does not
    /// declare it already, typed as the effect context's line for it (#872): the host registers
    /// them all on each, so a strict script may call any of them.
    fn with_shared_readers(block: &str, effect_context: &[&str]) -> String {
        let shared: Vec<(&str, &str)> = shared_reader_names()
            .into_iter()
            .map(|name| {
                let prefix = format!("\t{name}: (self: EffectContext");
                let line = effect_context.iter().find(|line| line.starts_with(&prefix)).unwrap_or_else(|| {
                    panic!("`{name}` is a shared reader (reads.rs `shared_readers!`) with no line in EffectContext's type: add one")
                });
                (name, *line)
            })
            .collect();
        let mut out = String::new();
        let mut rest = block;
        for context in PURE_READ_CONTEXTS {
            let open = format!("export type {context} = {{\n");
            let start = rest
                .find(&open)
                .unwrap_or_else(|| panic!("the hand-written block declares {context}"))
                + open.len();
            let close = start + rest[start..].find("\n}").expect("the type closes") + 1;
            let declared = &rest[start..close];
            out.push_str(&rest[..close]);
            for (name, line) in &shared {
                if !declared.contains(&format!("\t{name}: ")) {
                    out.push_str(&line.replace("EffectContext", context));
                    out.push('\n');
                }
            }
            rest = &rest[close..];
        }
        out.push_str(rest);
        out
    }

    fn repo_root() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root")
            .to_path_buf()
    }

    /// Converts a TypeScript declaration into Luau.
    ///
    /// The declaration is flattened first (doc comments go — Luau 0.740
    /// rejects `/** */` blocks between a table type's fields — and newlines
    /// become spaces), so braces balance on the one line and variants split
    /// cleanly; the Rust docs stay the source of truth.
    fn ts_to_luau(ts_decl: &str) -> String {
        let mut flat = String::new();
        for raw in ts_decl.lines() {
            let line = raw.trim();
            if line.starts_with("/**") || line.starts_with('*') || line == "*/" {
                continue;
            }
            if line.is_empty() {
                continue;
            }
            if !flat.is_empty() {
                flat.push(' ');
            }
            flat.push_str(line);
        }
        let flat = flat.trim_end().strip_suffix(';').unwrap_or(&flat).trim_end();
        let mut lines = Vec::new();
        // An intersection (`export type P = { base } & (V1 | V2)`): the base
        // table, then the parenthesised union's variants, then the group
        // close. Luau forbids mixing `&` and `|` without the parentheses.
        if let Some(amp) = flat.find(" & (") {
            let (base, group) = flat.split_at(amp);
            let group = group
                .strip_prefix(" & (")
                .unwrap_or(group)
                .trim_end()
                .strip_suffix(')')
                .unwrap_or(group);
            if let Some(open) = base.find('{') {
                let head = base[..open].trim_end().to_string();
                let inner = base[open + 1..]
                    .trim_end()
                    .strip_suffix('}')
                    .unwrap_or(&base[open + 1..]);
                let base_fields: Vec<String> = split_top_level(inner)
                    .into_iter()
                    .map(|field| field.trim().to_string())
                    .filter(|field| !field.is_empty())
                    .collect();
                if let Some(mut expanded) = split_variants("(", group, 0) {
                    expanded.remove(0);
                    // Distributable when every variant is a table: `B & (V1 | V2)` becomes the
                    // union of the flattened tables, which strict checks a literal against arm
                    // by arm — an intersection of a table with a union checks no literal at
                    // all (#464 gap 15, classic-035's `mod`). Anything else keeps the
                    // intersection rather than guessing joints.
                    let distributable = !base_fields.is_empty()
                        && expanded.last().is_some_and(|last| last == "}")
                        && expanded
                            .iter()
                            .all(|line| line == "} | {" || line == "}" || line.ends_with(','));
                    if distributable {
                        lines.push(format!("{head} {{"));
                        for field in &base_fields {
                            lines.push(format!("{field},"));
                        }
                        for line in expanded {
                            lines.push(line.clone());
                            if line == "} | {" {
                                for field in &base_fields {
                                    lines.push(format!("{field},"));
                                }
                            }
                        }
                        return lines
                            .into_iter()
                            .map(|line| convert_line(&line))
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }
                lines.push(format!("{} {{", base[..open].trim_end()));
                push_fields(&mut lines, inner);
                lines.push("} & (".to_string());
                if let Some(mut expanded) = split_variants("(", group, 0) {
                    expanded.remove(0);
                    lines.push("{".to_string());
                    lines.extend(expanded);
                    lines.push(")".to_string());
                } else {
                    lines.push(group.to_string());
                    lines.push(")".to_string());
                }
                return lines
                    .into_iter()
                    .map(|line| convert_line(&line))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
        }
        if let Some(body) = flat.strip_prefix("type ") {
            let body = format!("export type {body}");
            if let Some(expanded) = split_head_union(&body) {
                lines.extend(expanded);
            } else if let Some(open) = body.find('{') {
                lines.push(format!("{} {{", body[..open].trim_end()));
                let inner = body[open + 1..]
                    .trim_end()
                    .strip_suffix('}')
                    .unwrap_or(&body[open + 1..]);
                push_fields(&mut lines, inner);
                lines.push("}".to_string());
            } else {
                lines.push(body);
            }
        } else {
            lines.push(flat.to_string());
        }
        lines
            .into_iter()
            .map(|line| convert_line(&line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Splits a declaration head's variants at top-level `|`, then each
    /// variant's fields. Returns `None` for a single variant, leaving the
    /// line to the normal path.
    fn split_head_union(body: &str) -> Option<Vec<String>> {
        let open = body.find('{')?;
        // Depth starts inside the head's brace, so a field's own union
        // (`mana: number | nil`) never counts as a variant edge.
        split_variants(&body[..open], &body[open + 1..], 1)
    }

    /// The variants of `head { ... } | { ... }`, one field per line with
    /// `} | {` joints.
    fn split_variants(head: &str, rest: &str, initial_depth: i32) -> Option<Vec<String>> {
        let variants = split_top_level_char(rest, '|', initial_depth);
        if variants.len() < 2 {
            return None;
        }
        // A variant's close brace is never emitted mid-union: a following
        // joint's `} | {` closes it. The last table variant keeps its own
        // `}`, which ends the declaration.
        let mut out = Vec::new();
        for (i, variant) in variants.iter().enumerate() {
            let piece = variant.trim();
            if i == 0 {
                if piece.matches('{').count() == 0 && !piece.trim_end().ends_with('}') {
                    // A literal first (`"a" | {...}`): leave the whole
                    // declaration one line rather than guess joints.
                    return None;
                }
                out.push(format!("{} {{", head.trim_end()));
                // The group path hands the first variant its brace; the head
                // path consumed it already.
                let piece = piece.strip_prefix('{').unwrap_or(piece);
                let inner = piece.strip_suffix('}').unwrap_or(piece);
                push_fields(&mut out, inner);
                continue;
            }
            if let Some(stripped) = piece.strip_prefix('{') {
                out.push("} | {".to_string());
                let inner = stripped
                    .trim_end()
                    .strip_suffix('}')
                    .unwrap_or(stripped.trim_end());
                push_fields(&mut out, inner);
            } else {
                let last = out.pop().unwrap_or_default();
                let last = last.trim_end().strip_suffix(',').unwrap_or(last.trim_end());
                out.push(format!("{last} | {piece}"));
            }
        }
        let last_is_table = variants
            .last()
            .is_some_and(|variant| variant.trim().trim_end().ends_with('}'));
        if last_is_table {
            out.push("}".to_string());
        }
        Some(out)
    }

    /// A variant's comma-separated fields, one per line.
    fn push_fields(out: &mut Vec<String>, fields_part: &str) {
        for field in split_top_level(fields_part) {
            let field = field.trim();
            if !field.is_empty() {
                out.push(format!("{field},"));
            }
        }
    }

    /// Splits at `delimiter` outside every bracket kind and string literal.
    fn split_top_level_char(body: &str, delimiter: char, initial_depth: i32) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = initial_depth;
        let mut quote = None;
        let mut start = 0;
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if c == '"' || c == '\'' {
                quote = Some(c);
            } else if c == '{' || c == '[' || c == '(' || c == '<' {
                depth += 1;
            } else if c == '}' || c == ']' || c == ')' || c == '>' {
                depth -= 1;
            } else if c == delimiter && depth == 0 {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            i += 1;
        }
        parts.push(&body[start..]);
        parts
    }

    /// Splits at commas outside every bracket kind and string literal.
    fn split_top_level(body: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth = 0i32;
        let mut quote = None;
        let mut start = 0;
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if c == '"' || c == '\'' {
                quote = Some(c);
            } else if c == '{' || c == '[' || c == '(' || c == '<' {
                depth += 1;
            } else if c == '}' || c == ']' || c == ')' || c == '>' {
                depth -= 1;
            } else if c == ',' && depth == 0 {
                parts.push(&body[start..i]);
                start = i + 1;
            }
            i += 1;
        }
        parts.push(&body[start..]);
        parts
    }

    /// Luau's reserved words, which a table type cannot name bare (`end: T`
    /// is a syntax error; `["end"]: T` parses, and scripts build the value
    /// table the same way).
    const RESERVED: &[&str] = &[
        "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "if", "in", "local",
        "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
    ];

    /// `end:` / `end?:` at a field position becomes `["end"]:` / `["end"]?:`.
    fn quote_reserved(l: &str) -> String {
        for word in RESERVED {
            if l.starts_with(&format!("{word}:")) || l.starts_with(&format!("{word}?:")) {
                return format!("[\"{word}\"]{}", &l[word.len()..]);
            }
            // An inline table head (`export type X = { end: ...`).
            let inline = format!("{{ {word}:");
            if let Some(at) = l.find(&inline) {
                let head = &l[..at];
                if head.starts_with("export type ") && !head.contains('}') {
                    return format!("{head}{{ [\"{word}\"]:{}", &l[at + inline.len()..]);
                }
            }
        }
        l.to_string()
    }

    /// `"kind": T` becomes `["kind"]: T`: Luau type tables take no bare
    /// string keys (only `["lit"]:` parses), while values (`: "lit"`) and
    /// union arms (`| "lit"`) stay bare.
    fn quote_string_keys(l: &str) -> String {
        let mut out = String::new();
        let chars: Vec<(usize, char)> = l.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i].1 == '"' {
                let mut k = i + 1;
                while k < chars.len() && chars[k].1 != '"' {
                    k += 1;
                }
                let is_field = k + 1 < chars.len() && chars[k + 1].1 == ':' && {
                    let before = l[..chars[i].0].trim_end();
                    before.is_empty()
                        || before.ends_with('{')
                        || before.ends_with(',')
                        || before.ends_with('|')
                };
                if is_field {
                    out.push_str(&format!("[{}]", &l[chars[i].0..chars[k].0 + 1]));
                    i = k + 1;
                    continue;
                }
            }
            out.push(chars[i].1);
            i += 1;
        }
        out
    }

    /// One logical line from TypeScript to Luau.
    fn convert_line(line: &str) -> String {
        let mut l = quote_string_keys(&quote_reserved(line));

        // type Foo = ... -> export type Foo = ...
        if l.starts_with("type ") {
            l = format!("export {l}");
        }

        // { [key in string]: V } -> {[string]: V} (ts-rs's IndexMap)
        while let Some(start) = l.find("{ [key in string]:") {
            if let Some(end) = l[start..].find('}') {
                let v = l[start + 19..start + end].trim();
                l = format!("{}{{[string]: {v}}}{}", &l[..start], &l[start + end + 1..]);
                continue;
            }
            break;
        }

        // OneOrMany<T> -> T | {T} (a single value or a list of them)
        while let Some(start) = l.find("OneOrMany<") {
            let mut depth = 0;
            let mut end_idx = None;
            for (i, c) in l[start + 10..].char_indices() {
                if c == '<' {
                    depth += 1;
                } else if c == '>' {
                    if depth == 0 {
                        end_idx = Some(start + 10 + i);
                        break;
                    }
                    depth -= 1;
                }
            }
            if let Some(end) = end_idx {
                let inner = &l[start + 10..end];
                l = format!("{}({inner} | {{{inner}}}){}", &l[..start], &l[end + 1..]);
            } else {
                break;
            }
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

        // field?: Type | nil -> field: Type? (`?` already says nil, so the
        // union arm goes); field?: Type -> field: Type?
        if let Some(colon_idx) = l.find("?:") {
            let field_part = l[..colon_idx].trim_end();
            let mut type_part = l[colon_idx + 2..].trim().to_string();
            let comma_suffix = type_part.ends_with(',').then_some(",");
            if comma_suffix.is_some() {
                type_part.pop();
                type_part = type_part.trim_end().to_string();
            }
            for suffix in [" | nil", "| nil"] {
                if let Some(stripped) = type_part.strip_suffix(suffix) {
                    type_part = stripped.trim_end().to_string();
                    break;
                }
            }
            let indent_len = l.len() - l.trim_start().len();
            let indent = &l[..indent_len];
            l = format!(
                "{indent}{}: {type_part}?{}",
                field_part.trim_start(),
                comma_suffix.unwrap_or("")
            );
        }

        l
    }

    #[test]
    fn export_jackioh_luau_api() {
        let mut decls = BTreeMap::new();
        let ts_cfg = ts_rs::Config::default();

        macro_rules! reg {
            ( $( $t:ty ),* $(,)? ) => {
                $(
                    decls.insert(<$t as TS>::name(&ts_cfg), <$t as TS>::decl(&ts_cfg));
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
            TargetSpec,
            TargetAim,
            TargetDecl,
            TargetFilter,
            TuneDirection,
            AttackAmong,
            FieldSpellSide,
            SecretChoice,
            PredictOutcome,
            SetName,
            FaceKind,
            FilterSide,
            FilterOf,
            PromptKind,
            CostRange,
            AttackHealth,
            CraftRecipe,
            Enchantment,
            CounterKind,
            SwapWhat,
            PlayedFrom,
            RotationDirection,
            ControlHow,
            CounteredTo,
            Winner,
            GameOverReason,
            GlitchOutcome,
            FengShuiOutcome,
            RedirectWhat,
            TuningChange,
            LibraryOverflowOutcome,
            ManaSpentFor,
            CardMark,
            ModeDecl,
            Keyword,
            Tag,
            Selection,
            PreviewValue,
            StatMod,
            TuneMultiplier,
            StaticFlags,
            ActivationCost,
            ActivationUses,
            GameEventType,
            GameEvent,
            ReplacementMoment,
            ReplacementWhere,
            ReplacementInstead,
            InsteadRedirect,
            InsteadDamage,
            InsteadLasting,
            InsteadFlicker,
            InsteadTo,
            ReplacementBy,
            ReplacedEvent,
            CostAuraWhose,
            CostAura,
            CostRule,
            GraveyardPlayPermission,
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
            AuraApplies,
            AuraArgs,
            AuraFilter,
            DefenseArmorArgs,
            CloakHandAuraArgs,
            CastEachArgs,
            DrawWhileArgs,
            CardsSource,
            ForEachCardArgs,
            WithKillCreditArgs,
            RadiantChanceLuauArgs,
            DoRandomEntriesArgs,
            RandomEntryLuau,
            TurnView,
        }

        // Shared effect-argument shapes other args name
        reg! {
            PlayerSpec,
            CardScope,
            ScopeSide,
            TargetScope,
            ZoneSpec,
            LaneSpec,
            PileSpec,
            PickFilter,
            LibraryFilter,
            PlayerModifierSpec,
            RecruitFilter,
            RecruitSource,
            SecretLink,
            SplitAmong,
            StealOtherwise,
            StormSide,
            SwitchAllPositionsSide,
            TakeFromLibraryPick,
            TransformRadiant,
            NumberWhich,
            CounterDestination,
            FuseInto,
            FuseOntoPile,
            FuseOntoOtherwise,
            FuseCardsPick,
            ForcedAttackerFilter,
            ForcedSide,
            ForcedTarget,
            GiveFromHandCards,
            RewardOption,
            CellScope,
            CastNewDef,
            CastRandomCount,
            CastRandomQuery,
            BuffAllSide,
            BuffAmount,
            RollTable,
            ExileArgs,
            ExileAdjacentToArgs,
            ExileHandArgs,
            ExileMatchingArgs,
            ExileRandomFromArgs,
            DiscardHandTurn,
            DiscoverOffer,
            DelayAt,
            ReturnRandomFromGraveyardArgs,
            ReplaceHandWithRandomArgs,
            AddCostRuleArgs,
            GainBrittleArgs,
        }

        // Root and subsystem types other args name
        reg! {
            CastAfterward,
            CatalogQueryArgs,
            GameCounters,
            LibraryEnd,
            Zone,
            AltWinCondition,
            ModifierExpiry,
            Resume,
            jackioh_engine::subsystems::board_history::RollBackSides,
            jackioh_engine::subsystems::fuse::HandPrice,
            jackioh_engine::subsystems::perfect_hand::ReplaceHandWithPerfectArgs,
        }

        // Second-level shapes the args above name
        reg! {
            BuffAllBoth,
            CardZone,
            CastDef,
            CellSide,
            CostParity,
            CostRuleSpan,
            CraftHat,
            CraftHatKind,
            CraftEffect,
            CraftKeyword,
            CraftVerb,
            KeywordKind,
            NumberRef,
            NumberedKey,
            DelayPlayer,
            EmoteId,
            ExileRandomZone,
            ExileZone,
            FuseIntoPile,
            KeepFace,
            Phase,
            PileZone,
            RadiantZone,
            Rarity,
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
            DrawCombatKeywordsArgs,
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

        // ts-rs reads `ActivationUses::Unlimited`, a unit variant `#[serde(with)]` writes as the
        // string, as `null`; scripts write `uses = "unlimited"` and the loader reads that string.
        decls.insert(
            "ActivationUses".to_string(),
            "type ActivationUses = number | \"unlimited\";".to_string(),
        );

        let mut out = String::new();
        out.push_str("--!strict\n");
        out.push_str("-- Generated by crates/luau/tests/export_api.rs. DO NOT EDIT DIRECTLY.\n\n");

        out.push_str("export type EffectCall = {\n\tverb: string,\n\targs: any,\n}\n\n");

        // Luau reads types before their use, so every translated declaration
        // comes before the hand-written stubs that name them.
        for ts_decl in decls.values() {
            let luau_decl = ts_to_luau(ts_decl);
            out.push_str(&luau_decl);
            out.push_str("\n\n");
        }

        // The context type: one line per method `register_ctx` binds in
        // crates/luau/src/reads.rs, in the same order, so the stub and the
        // binding cannot drift (tests/api.rs proves every name).
        out.push_str("export type EffectContext = {\n");
        let effect_context_readers = [
            "\tcontroller: (self: EffectContext) -> PlayerId,",
            "\tradiant: (self: EffectContext) -> boolean,",
            "\tx: (self: EffectContext) -> number,",
            "\tembiggened: (self: EffectContext) -> boolean,",
            "\tmana_before_play: (self: EffectContext) -> number?,",
            "\tself_: (self: EffectContext) -> any,",
            "\tlive_self: (self: EffectContext) -> any,",
            "\tstands_since_script_began: (self: EffectContext, id: string) -> boolean,",
            "\topponent_of: (self: EffectContext, player: PlayerId) -> PlayerId,",
            "\tparam: (self: EffectContext, key: string) -> number,",
            "\tparam_value: (self: EffectContext, key: string) -> number,",
            "\tparams_view: (self: EffectContext) -> any,",
            "\trecalled: (self: EffectContext, key: string) -> any,",
            "\tchosen_options: (self: EffectContext) -> {string},",
            "\tchosen_number: (self: EffectContext) -> number?,",
            "\tchosen_cells: (self: EffectContext) -> {any},",
            "\tchosen_recipe: (self: EffectContext) -> any,",
            "\tchosen_tuning_number: (self: EffectContext) -> any,",
            "\tsummoned_so_far: (self: EffectContext) -> {string},",
            "\tafter_attack_of: (self: EffectContext) -> any,",
            "\tafter_attacked_of: (self: EffectContext) -> any,",
            "\tactivation_paid: (self: EffectContext) -> any,",
            "\thero_of: (self: EffectContext, player: PlayerId) -> any,",
            "\tzone_cards: (self: EffectContext, player: PlayerId, zone: OffFieldZone) -> {any},",
            "\tzone_count: (self: EffectContext, player: PlayerId, zone: OffFieldZone) -> number,",
            "\tcards_played_this_turn: (self: EffectContext, player: PlayerId) -> number,",
            "\tunspent_mana_of: (self: EffectContext, player: PlayerId) -> number,",
            "\tmax_mana_of: (self: EffectContext, player: PlayerId) -> number,",
            "\tplayed_ids_this_turn: (self: EffectContext, player: PlayerId) -> {string},",
            "\tplayed_earlier: (self: EffectContext, player: PlayerId, card: any) -> number,",
            "\tturn: (self: EffectContext) -> TurnView,",
            "\tgiven_lucky_on: (self: EffectContext, card: any) -> number,",
            "\tcounters: (self: EffectContext) -> GameCounters,",
            "\tdata: (self: EffectContext) -> any,",
            "\tevents: (self: EffectContext) -> {GameEvent},",
            "\tevents_from: (self: EffectContext) -> number,",
            "\tleft_field_since_resolved: (self: EffectContext, event: GameEvent) -> boolean,",
            "\twas_played_this_turn: (self: EffectContext, player: PlayerId, id: string) -> boolean,",
            "\tactive_units_of: (self: EffectContext, player: PlayerId) -> {any},",
            "\tdormant_units_of: (self: EffectContext, player: PlayerId) -> {any},",
            "\tcard_at: (self: EffectContext, player: PlayerId, lane: number) -> any,",
            "\tslots_of: (self: EffectContext, player: PlayerId, row: Row) -> {any},",
            "\tslot_of: (self: EffectContext, id: string) -> any,",
            "\tface_of: (self: EffectContext, id: string) -> any,",
            "\tstats_with_buffs: (self: EffectContext, id: string) -> any,",
            "\tunit_view: (self: EffectContext, id: string) -> any,",
            "\tdef_of: (self: EffectContext, id: string) -> any,",
            "\tprinted_cost: (self: EffectContext, id: string) -> number,",
            "\teffective_cost: (self: EffectContext, id: string) -> number,",
            "\tquery_cost: (self: EffectContext, id: string) -> number,",
            "\town_cost: (self: EffectContext, id: string) -> number?,",
            "\tcost_now: (self: EffectContext, id: string) -> number,",
            "\ttags_of: (self: EffectContext, id: string) -> {string},",
            "\tlucky_on: (self: EffectContext, id: string) -> number,",
            "\thighest_permanent_cost: (self: EffectContext, player: PlayerId) -> number,",
            "\tbase_stats_of: (self: EffectContext, id: string) -> any,",
            "\tcredit_used_this_turn: (self: EffectContext, player: PlayerId) -> number,",
            "\tarmor_on_field: (self: EffectContext) -> number,",
            "\tfind_instance: (self: EffectContext, id: string) -> any,",
            "\tkiller_of: (self: EffectContext, id: string) -> any,",
            "\tkiller_of_self: (self: EffectContext) -> any,",
            "\tluck_of: (self: EffectContext, player: PlayerId) -> number,",
            "\tplague_on: (self: EffectContext, id: string) -> number,",
            "\tplague_multiplier_of: (self: EffectContext, id: string) -> number,",
            "\tpermanents_on_field: (self: EffectContext, player: PlayerId) -> {any},",
            "\topen_zones: (self: EffectContext, player: PlayerId, row: Row) -> {number},",
            "\tfirst_free_zone: (self: EffectContext, player: PlayerId, row: Row) -> any,",
            "\tmidlane_lanes_of: (self: EffectContext, player: PlayerId) -> {number},",
            "\ttimes_played_of: (self: EffectContext, card: any) -> number,",
            "\tis_unit_token: (self: EffectContext, id: string) -> boolean,",
            "\tis_x_cost: (self: EffectContext, id: string) -> boolean,",
            "\tis_animated: (self: EffectContext, id: string) -> boolean,",
            "\tis_carried: (self: EffectContext, id: string) -> boolean,",
            "\tcarried_at: (self: EffectContext, player: PlayerId, lane: number) -> any,",
            "\tis_locked: (self: EffectContext, player: PlayerId, lane: number) -> boolean,",
            "\tis_berserk: (self: EffectContext, id: string) -> boolean,",
            "\tunit_has: (self: EffectContext, id: string, kw: any) -> boolean,",
            "\tcard_keywords: (self: EffectContext, id: string) -> {any},",
            "\tnumbered_keywords_on: (self: EffectContext, id: string) -> {any},",
            "\twin_rate_leader: (self: EffectContext, exclude: string?) -> any,",
            "\tfib: (self: EffectContext, n: number) -> number,",
            "\tinstance_of: (self: EffectContext, spec: TargetSpec) -> any,",
            "\tresolve_target: (self: EffectContext, spec: TargetSpec) -> any,",
            "\tsides_of: (self: EffectContext, side: any) -> {any},",
            "\tcards_in_scope: (self: EffectContext, scope: BoardScope) -> {any},",
            "\tcards_in_scope_aimed: (self: EffectContext, scope: BoardScope, aim: TargetAim) -> {any},",
            "\tadjacent_to: (self: EffectContext, spec: TargetSpec, scope: BoardScope) -> {any},",
            "\tunreadable_by: (self: EffectContext, id: string) -> {PlayerId},",
            "\timmutable_bars: (self: EffectContext, source: string?, id: string) -> boolean,",
            "\trng_roll: (self: EffectContext, sides: number) -> number,",
            "\trng_next_int: (self: EffectContext, min: number, max: number) -> number,",
            "\trng_chance: (self: EffectContext, num: number, den: number) -> boolean,",
            "\trng_pick: (self: EffectContext, array: {any}) -> any,",
            "\trng_shuffle: (self: EffectContext, array: {any}) -> {any},",
            "\tattack_target_of: (self: EffectContext, id: string) -> any,",
            "\trandom_attack_targets: (self: EffectContext, id: string, among: AttackAmong) -> {any},",
            "\tcard_type_of: (self: EffectContext, id: string) -> any,",
            "\tcard_type_of_face: (self: EffectContext, id: string) -> CardType,",
            "\tkeywords_of: (self: EffectContext, id: string) -> {any},",
            "\thas_keyword: (self: EffectContext, id: string, kind: any) -> boolean,",
            "\tnumbers_on: (self: EffectContext, id: string) -> {any},",
            "\tnumbered_sum: (self: EffectContext, id: string, kind: any) -> number?,",
            "\tnumbering_order: (self: EffectContext, cards: {any}) -> {any},",
            "\tapplicable_changes: (self: EffectContext, id: string, dir: TuneDirection) -> {any},",
            "\tplay_cost: (self: EffectContext, id: string) -> number,",
            "\tplayable_from_graveyard: (self: EffectContext, id: string) -> boolean,",
            "\thas_triggerable_cry: (self: EffectContext, id: string) -> boolean,",
            "\tdiscard_guarded: (self: EffectContext, player: PlayerId) -> boolean,",
            "\thand_cap_of: (self: EffectContext, player: PlayerId) -> number,",
            "\timmune_to_tribal_hate: (self: EffectContext, id: string) -> boolean,",
            "\tis_cast_on_draw: (self: EffectContext, id: string) -> boolean,",
            "\tplayed_this_game_with_tag: (self: EffectContext, player: PlayerId, tag: Tag) -> number,",
            "\tplayed_this_turn_of_type: (self: EffectContext, player: PlayerId, types: {CardType}) -> number,",
            "\tcreated_in_library: (self: EffectContext, player: PlayerId) -> {string},",
            "\tlast_face_up_played: (self: EffectContext, player: PlayerId) -> any,",
            "\tlethal_attackers_of: (self: EffectContext, player: PlayerId) -> {any},",
            "\tfill_board_zones: (self: EffectContext, player: PlayerId) -> {any},",
            "\tstacked_onto: (self: EffectContext, id: string) -> string?,",
            "\tredirect_neighbours: (self: EffectContext, id: string) -> {any},",
            "\tfusable_permanents_of: (self: EffectContext, player: PlayerId, except: string?) -> {any},",
            "\tsecret_of: (self: EffectContext, id: string) -> any,",
            "\tjudge: (self: EffectContext, guess: SecretChoice, choice: SecretChoice) -> PredictOutcome,",
            "\tdef_by_index: (self: EffectContext, set: SetName, index: string) -> any,",
            "\tfill_params: (self: EffectContext, id: string, face: FaceKind, values: any) -> string,",
            "\tgrant_param: (self: EffectContext, key: string) -> number,",
            "\treturn_price_of: (self: EffectContext, id: string) -> number,",
            "\tfield_spells_doomed: (self: EffectContext, side: FieldSpellSide) -> {any},",
            "\tsubsystems_activation_paid: (self: EffectContext) -> any,",
            "\tsubsystems_feng_shui_last_element: (self: EffectContext, player: PlayerId) -> any,",
            "\tsubsystems_feng_shui_element_preview: (self: EffectContext, label: string, elem: any) -> any,",
            "\tsubsystems_pareto_judgement_of: (self: EffectContext) -> any,",
            "\teffects_of: (self: EffectContext, card: string, face: string, hook: string) -> {string},",
        ];
        for line in effect_context_readers {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("}\n\n");

        // Hand-written (Plan Risk: ts-rs cannot derive these — each carries Rust
        // closures — so their declaration shapes are written here, matching what
        // crates/luau/src/script.rs reads off a face table), with every shared reader added to
        // each pure-read context's type.
        out.push_str(&with_shared_readers(
            "-- A trigger a card registers while it is in a zone (Script.triggers and\n\
             -- handTriggers/deckTriggers/graveyardTriggers entries).\n\
             export type TriggerDef = {\n\
             \tid: string,\n\
             \ton: { GameEventType },\n\
             \twhen: ((ctx: EffectContext, event: GameEvent) -> boolean)?,\n\
             \trun: (ctx: EffectContext, event: GameEvent) -> { EffectCall },\n\
             }\n\n\
             -- One replacement a card makes (Script.replacements entries).\n\
             export type ReplacementDef = {\n\
             \tid: string,\n\
             \ton: ReplacementMoment,\n\
             \twhere: ReplacementWhere?,\n\
             \twhen: ((ctx: ReplacementContext) -> boolean)?,\n\
             \tinstead: ReplacementInstead,\n\
             \t[\"then\"]: string?,\n\
             \tby: ReplacementBy?,\n\
             }\n\n\
             -- One activated ability a card declares (Script.activations entries).\n\
             export type ActivationDecl = {\n\
             \tid: string,\n\
             \tlabel: string,\n\
             \tuses: ActivationUses?,\n\
             \tcost: ActivationCost?,\n\
             \ttargets: { TargetDecl },\n\
             \tmodes: { ModeDecl },\n\
             \tcanActivate: ((ctx: ConditionContext) -> boolean)?,\n\
             \thas: ((ctx: HookArgs) -> boolean)?,\n\
             \trun: (ctx: EffectContext) -> { EffectCall },\n\
             }\n\n\
             -- The pure-read contexts a predicate hook is called with, as methods:\n\
             -- what `reads.rs` registers (`register_condition_ctx`,\n\
             -- `register_replacement_ctx`, `register_hook_args`).\n\
             -- Each also declares every reader `reads.rs` `shared_readers!` adds to a\n\
             -- context with a state, typed as on the effect context: the generator reads\n\
             -- the macro (`with_shared_readers`), so a strict script finds every reader\n\
             -- the host registers (#872).\n\
             export type ConditionContext = {\n\
             \tcontroller: (self: ConditionContext) -> PlayerId,\n\
             \tradiant: (self: ConditionContext) -> boolean,\n\
             \tyour_turn: (self: ConditionContext) -> boolean,\n\
             \tzone: (self: ConditionContext) -> ConditionZone,\n\
             \tself_: (self: ConditionContext) -> any,\n\
             \tparam: (self: ConditionContext, key: string) -> number,\n\
             \thero_of: (self: ConditionContext, player: PlayerId) -> any,\n\
             \tunspent_mana_of: (self: ConditionContext, player: PlayerId) -> number,\n\
             \tmax_mana_of: (self: ConditionContext, player: PlayerId) -> number,\n\
             \tcards_played_this_turn: (self: ConditionContext, player: PlayerId) -> number,\n\
             \tzone_count: (self: ConditionContext, player: PlayerId, zone: OffFieldZone) -> number,\n\
             \tturn: (self: ConditionContext) -> TurnView,\n\
             \tplayed_earlier: (self: ConditionContext, player: PlayerId, card: any) -> number,\n\
             \tparams_view: (self: ConditionContext) -> any,\n\
             \tplay_cost: (self: ConditionContext) -> number,\n\
             \ttimes_played_of: (self: ConditionContext, card: any) -> number,\n\
             \topen_zones: (self: ConditionContext, player: PlayerId, row: Row) -> {number},\n\
             \tfusable_permanents_of: (self: ConditionContext, player: PlayerId, except: string?) -> {any},\n\
             \tgiven_lucky_on: (self: ConditionContext, card: any) -> number,\n\
             \tkiller_of: (self: ConditionContext, id: string) -> any,\n\
             \tkiller_of_self: (self: ConditionContext) -> any,\n\
             \tleft_field_since_resolved: (self: ConditionContext, event: GameEvent) -> boolean,\n\
             \tcounters: (self: ConditionContext) -> GameCounters,\n\
             \tfill_params: (self: ConditionContext, id: string, face: FaceKind, values: any) -> string,\n\
             \topponent_of: (self: ConditionContext, player: PlayerId) -> PlayerId,\n\
             }\n\n\
             export type ReplacementContext = {\n\
             \tself_: (self: ReplacementContext) -> any,\n\
             \tcontroller: (self: ReplacementContext) -> PlayerId,\n\
             \tradiant: (self: ReplacementContext) -> boolean,\n\
             \tevent: (self: ReplacementContext) -> ReplacedEvent,\n\
             \tparam: (self: ReplacementContext, key: string) -> number,\n\
             \tturn: (self: ReplacementContext) -> TurnView,\n\
             \tplayed_earlier: (self: ReplacementContext, player: PlayerId, card: any) -> number,\n\
             \tparams_view: (self: ReplacementContext) -> any,\n\
             \tgiven_lucky_on: (self: ReplacementContext, card: any) -> number,\n\
             \tkiller_of: (self: ReplacementContext, id: string) -> any,\n\
             \tleft_field_since_resolved: (self: ReplacementContext, event: GameEvent) -> boolean,\n\
             \tcounters: (self: ReplacementContext) -> GameCounters,\n\
             \topponent_of: (self: ReplacementContext, player: PlayerId) -> PlayerId,\n\
             }\n\n\
             export type HookArgs = {\n\
             \tcontroller: (self: HookArgs) -> PlayerId,\n\
             \tparam: (self: HookArgs, key: string) -> number,\n\
             \tradiant: (self: HookArgs) -> boolean,\n\
             \tself_: (self: HookArgs) -> any,\n\
             \thero_of: (self: HookArgs, player: PlayerId) -> any,\n\
             \tunspent_mana_of: (self: HookArgs, player: PlayerId) -> number,\n\
             \tmax_mana_of: (self: HookArgs, player: PlayerId) -> number,\n\
             \tplague_on: (self: HookArgs, id: string) -> number,\n\
             \topponent_of: (self: HookArgs, player: PlayerId) -> PlayerId,\n\
             \tturn: (self: HookArgs) -> TurnView,\n\
             \tplayed_earlier: (self: HookArgs, player: PlayerId, card: any) -> number,\n\
             \tparams_view: (self: HookArgs) -> any,\n\
             \tgiven_lucky_on: (self: HookArgs, card: any) -> number,\n\
             \tkiller_of: (self: HookArgs, id: string) -> any,\n\
             \tleft_field_since_resolved: (self: HookArgs, event: GameEvent) -> boolean,\n\
             \tcounters: (self: HookArgs) -> GameCounters,\n\
             }\n\n\
             export type CostArgs = {\n\
             \tinstance: (self: CostArgs) -> any,\n\
             \tself_: (self: CostArgs) -> any,\n\
             \tcontroller: (self: CostArgs) -> PlayerId,\n\
             \tparam: (self: CostArgs, key: string) -> number,\n\
             \tcounters: (self: CostArgs) -> GameCounters,\n\
             \tturn: (self: CostArgs) -> TurnView,\n\
             \thero_of: (self: CostArgs, player: PlayerId) -> any,\n\
             \tunspent_mana_of: (self: CostArgs, player: PlayerId) -> number,\n\
             \tplayed_earlier: (self: CostArgs, player: PlayerId, card: any) -> number,\n\
             \tparams_view: (self: CostArgs) -> any,\n\
             \tgiven_lucky_on: (self: CostArgs, card: any) -> number,\n\
             }\n\n\
             -- A `targetChecks` predicate's argument (`register_target_check_args`, §10.6):\n\
             -- `candidate` is the card a declaration would offer (nil for a hero or a zone),\n\
             -- `player` the chooser.\n\
             export type TargetCheckArgs = {\n\
             \tself_: (self: TargetCheckArgs) -> any,\n\
             \tcandidate: (self: TargetCheckArgs) -> any,\n\
             \tplayer: (self: TargetCheckArgs) -> PlayerId,\n\
             \tradiant: (self: TargetCheckArgs) -> boolean,\n\
             \tcontroller: (self: TargetCheckArgs) -> PlayerId,\n\
             \tparam: (self: TargetCheckArgs, key: string) -> number,\n\
             \topponent_of: (self: TargetCheckArgs, player: PlayerId) -> PlayerId,\n\
             \tturn: (self: TargetCheckArgs) -> TurnView,\n\
             \tcounters: (self: TargetCheckArgs) -> GameCounters,\n\
             \tparams_view: (self: TargetCheckArgs) -> any,\n\
             \thero_of: (self: TargetCheckArgs, player: PlayerId) -> any,\n\
             \tdef_of: (self: TargetCheckArgs, id: string) -> any,\n\
             \teffective_cost: (self: TargetCheckArgs, id: string) -> number,\n\
             \tcard_type_of: (self: TargetCheckArgs, id: string) -> any,\n\
             \tunit_has: (self: TargetCheckArgs, id: string, kw: any) -> boolean,\n\
             \tnumbers_on: (self: TargetCheckArgs, id: string) -> {any},\n\
             \tunreadable_by: (self: TargetCheckArgs, id: string) -> {PlayerId},\n\
             \thas_triggerable_cry: (self: TargetCheckArgs, id: string) -> boolean,\n\
             }\n\n\
             -- Arbitrary JSON (a remembered value, a prompt's data bag): crosses\n\
             -- as `any` because no Rust type stands behind it.\n\
             export type JsonValue = any\n\n\
             -- Verb argument aliases: `pub type` names for a shared shape, whose\n\
             -- ts-rs declaration is filed under the underlying name.\n\
             export type ExileHandArgs = DiscardHandArgs\n\
             export type GainBrittleArgs = GiveBrittleArgs\n\
             export type SetRadiantArgs = RadiantTarget\n\
             export type ClearRadiantArgs = RadiantTarget\n\n",
            &effect_context_readers,
        ));

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

        // `J.config`: the engine constants card scripts read, from the same source as the
        // runtime table (`CONFIG_I32`), so the typed module and the host cannot drift (#464 gap 7).
        out.push_str("export type JConfig = {\n");
        for (name, _) in jackioh_luau::config::CONFIG_I32 {
            out.push_str(&format!("\t{name}: number,\n"));
        }
        out.push_str("}\n\n");
        out.push_str("J.config = {\n");
        for (name, value) in jackioh_luau::config::CONFIG_I32 {
            out.push_str(&format!("\t{name} = {value},\n"));
        }
        out.push_str("}\n\n");

        // Special / helper functions
        out.push_str("function J.div(a: number, b: number): number\n\treturn (a // b)\nend\n\n");
        out.push_str("function J.rem(a: number, b: number): number\n\treturn (a % b)\nend\n\n");
        out.push_str(
            "function J.sort_by<T>(list: {T}, key_fn: (item: T) -> any): {T}\n\treturn list\nend\n\n",
        );
        out.push_str("function J.list<T>(list: {T}): {T}\n\treturn list\nend\n\n");
        out.push_str("function J.query(args: any): {string}\n\treturn {}\nend\n\n");
        // `pool_catalog`: the definitions a query answers, but the calling card's own.
        out.push_str("function J.pool(ownId: string, query: CatalogQueryArgs): {any}\n\treturn {}\nend\n\n");
        out.push_str("function J.cost(defId: string): number\n\treturn 0\nend\n\n");
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
