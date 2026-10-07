//! #51 KY's Private Tutor (SPEC §8.3, §6.3 Discover, §10.5 steps 5-6, §10.6, §10.8, §5.1; R4, R11,
//! R50, R60, R65, R81, R90). Spell, cost 1, tag KY, Epic.
//!   Base:    "Choose a type (Spell, Unit, Field Spell, Trap), then a cost bracket (0-1, 2, 3, 4+),
//!            offering only options with a match in your library; reveal 3 random matching library
//!            cards; choose one to hand. No match at all → add a KY's Empty Notebook"
//!   Radiant: "Echo (resolves a second time)" — §8 Conventions: the cell restates nothing of the
//!            base clause, so the whole four-step sequence is kept and simply happens twice.
//!   Engine:  "Three chained pending choices; Field Trap counts as Trap; brackets read costs per R65
//!            (X cards as 0, embiggen cards at their base price)".
//!
//! A FOUR-STEP MACHINE, THREE OF THEM PROMPTS. §9.3 forbids callbacks in state, so a chain is a
//! named step plus captured data (§10.6): `prompts.rs`'s `resume_self(ctx, step, data)` records
//! `{ defId, hook: "resume", step, radiant, instanceId, data }` on the `PendingChoice`, and
//! `answer_prompt` re-enters `script.resume[step]` with the answer in `ctx.targets`. So the steps are
//! entries of the `resume` table below and nothing else:
//!
//!   cry            → "start"   read the library, offer the types that have a match       (prompt 1)
//!   resume.bracket → the type  offer the cost brackets that have a match for that type   (prompt 2)
//!   resume.reveal  → +bracket  reveal 3 random matching library cards                    (prompt 3)
//!   resume.take    → the card  move that library card to hand
//!
//! Each step carries forward what the earlier ones learned in `data` (the chosen type), because
//! §10.6's continuation is data, not a closure, and because a step may resume with `ctx.self_`
//! `None` — this is a Spell, and by resolution time the card is in `resolving` on its way to the
//! graveyard. `ctx.data` is `IndexMap<String, Value>`, so it is narrowed with `as_str`, never cast.
//!
//! "OFFERING ONLY OPTIONS WITH A MATCH IN YOUR LIBRARY" is why the two mode prompts are computed
//! from the library rather than fixed: a type with no match is never offered, and neither is a
//! bracket with no match for the type already chosen. Reading the library is reading state, which a
//! hook may do (CLAUDE.md rule 5 bans mutation, not reads); the cards are not revealed by it —
//! §10.8 rules that "a card revealed out of a library … is revealed only as an option of the prompt
//! that reveals it", so only prompt 3 ever exposes a card, only to the chooser, and the rest of the
//! library stays hidden from both players. The type and bracket prompts leak nothing but the
//! existence of a match, which is what the §8 row asks them to say.
//!
//! "FIELD TRAP COUNTS AS TRAP" (Engine cell) is the same reading as `query.rs`'s `TRAP_TYPES`, R61
//! (#85) and R35 (#83): a `type` filter matches the field exactly, so "Trap" has to name both
//! "Trap" and "Field Trap" or #18 Bread and Butter and #71 Intern Stimmy would silently vanish.
//!
//! BRACKETS READ COSTS PER R65 (Engine cell): `effective_cost(state, card)` is R65's one calculation
//! for an instance, the same one #30 Archivist and #94 Genn's Greed read their library cards with
//! (R24, R66). A library card was never played, so an X-cost card has no X and reads 0 (#74 Adaptive
//! UI and #96 My Pawn sit in the "0-1" bracket) and an embiggen card its base price (#59 Unbiased
//! Immigration, "2 embiggen 4", reads 2); and R65 names library filters as its own ground, so the
//! card's `cost_mod` (kept in every zone, R78 — #95's "costs 2 less"), its `cost_override`, Ceaseless
//! Void's computed cost and a rolled Heroic Power's X all count, as do the player's live discounts.
//! The definition's printed cost (`query_cost`) would see none of them.
//!
//! "NO MATCH AT ALL → ADD A KY'S EMPTY NOTEBOOK": no type has a match exactly when the library is
//! empty, since every card in it has one of the five types and all five map onto the four options.
//! The token is created fresh in hand by `add_to_hand` (R4's cap applies; a spell token is an ordinary
//! hand card, R11) and it is never reachable from a random pool (§5.1), only from here.
//!
//! TWO MISSING VERBS (reported, not worked around, and not faked with a different verb):
//!
//!   1. ```text
//!      discoverFromLibrary({ step: string, count?: number, player?: "self" | "enemy",
//!                            filter?: { type?: CardType | CardType[];
//!                                       costRange?: { min?: number; max?: number } },
//!                            prompt?: string, data?: Record<string, unknown> }): Effect
//!      ```
//!      §6.3's Discover row already describes this card as the primitive: "'Reveal N matching cards,
//!      then choose one' (KY's Private Tutor) is this same primitive with the library as the pool:
//!      the revealed cards are that prompt's options, so only the chooser ever sees them (§10.8)".
//!      `discoverFromCatalog` queries the CATALOG, which would offer cards that are not in the
//!      library at all, and `discoverFromGraveyard` is the same shape over the wrong pile — so the
//!      third one is needed: options are the actual library INSTANCES (`{ pick: "instance",
//!      instanceId }`, as `discoverFromGraveyard` builds them), drawn without replacement from the
//!      matching subset with `ctx.rng.shuffle` so the three are always different (R60, §6.3), and
//!      fewer than `count` matches offer what exists (§6.3: no options at all fizzles).
//!
//!   2. addToHand({ instance: { of: "chosen", index?: number } }) — an added overload of the
//!      existing verb, for §6.3's "Add to hand: CREATES OR MOVES the card". Today `addToHand` only
//!      creates a fresh instance from a `defId`, which would leave the revealed card in the library
//!      and put a copy in hand. The chosen library instance must MOVE zones (library → hand) with
//!      its identity, its radiant flag and its `costOverride` intact (R78: those persist in every
//!      zone), through the same `draw.ts` pipeline, so the hand cap burns it when the hand is full
//!      (R4). `bounce` is not that verb — it returns a card from the FIELD to its owner's hand and
//!      resets the instance (§6.3, R78) — so it is deliberately not used here.
//!
//! THE RADIANT ECHO. §6.2's "Echo X" is "recast this card X more times … The repeats outstanding
//! live in `state.echoQueue` and resolve one at a time in the resolution loop, so a prompt inside
//! one repeat pauses the rest until it is answered (§10.5 step 6)". That is `StaticFlags.echo`
//! (R30): the count of EXTRA resolutions, 1 here, which `play_steps.rs`'s `echo_step` reads when it
//! queues an `EchoRepeat`, summed with the `echoNextSpell` player modifier (#79 Twinspell). Each
//! repeat re-enters `cry` and opens its OWN fresh prompts, which is what makes the radiant face run
//! the whole four-step sequence twice.
//!
//! Why not grant this card the existing `echoNextSpell` modifier instead: that modifier has expiry
//! `{ until: "used" }` and applies to the NEXT spell played, so it would have to be granted
//! mid-resolution of this one (§10.5 step 5, after step 2 already consumed it); no effect verb
//! grants a `PlayerModifier` at all; and a card that re-entered its own chain by hand would double
//! up with Twinspell — 2 engine repeats × 2 card repeats = 4 resolutions where §6.2 wants 3.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-051";

/// #51.1, the token the empty-library path adds (§8.3, §7).
const NOTEBOOK: &str = "core-051-1";

/// §6.3 Discover: 1 of 3, so three library cards are revealed.
const REVEAL_COUNT: i32 = 3;

/// One of the four type options (TS `TypeOption`, the literal union of `TYPE_OPTIONS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TypeOption {
    Spell,
    Unit,
    FieldSpell,
    Trap,
}

impl TypeOption {
    /// The option as the prompt prints it and the answer names it.
    fn as_str(self) -> &'static str {
        match self {
            TypeOption::Spell => "Spell",
            TypeOption::Unit => "Unit",
            TypeOption::FieldSpell => "Field Spell",
            TypeOption::Trap => "Trap",
        }
    }
}

/// The four type options, in the §8 row's printed order.
const TYPE_OPTIONS: [TypeOption; 4] = [
    TypeOption::Spell,
    TypeOption::Unit,
    TypeOption::FieldSpell,
    TypeOption::Trap,
];

/// One of the four cost brackets (TS `BracketOption`, the literal union of `BRACKET_OPTIONS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BracketOption {
    ZeroToOne,
    Two,
    Three,
    FourPlus,
}

impl BracketOption {
    /// The bracket as the prompt prints it and the answer names it.
    fn as_str(self) -> &'static str {
        match self {
            BracketOption::ZeroToOne => "0-1",
            BracketOption::Two => "2",
            BracketOption::Three => "3",
            BracketOption::FourPlus => "4+",
        }
    }
}

/// The four cost brackets, in the §8 row's printed order.
const BRACKET_OPTIONS: [BracketOption; 4] = [
    BracketOption::ZeroToOne,
    BracketOption::Two,
    BracketOption::Three,
    BracketOption::FourPlus,
];

/// The key the chosen type travels under, from the bracket step to the reveal step (§10.6).
const TYPE_KEY: &str = "type";

/// R65's out-of-play cost range a bracket means (TS `{ min?: number; max?: number }`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BracketRange {
    min: Option<i32>,
    max: Option<i32>,
}

impl BracketRange {
    /// The `costRange` literal the reveal's filter takes: only the bounds the bracket has.
    fn to_json(self) -> Value {
        let mut range = json!({});
        if let Some(min) = self.min {
            range["min"] = json!(min);
        }
        if let Some(max) = self.max {
            range["max"] = json!(max);
        }
        range
    }
}

/// Which card types one option covers. Engine cell: "Field Trap counts as Trap"; every other option
/// is its own type, so "Spell" never reaches a Field Spell and "Field Spell" never a Spell.
fn types_for(option: TypeOption) -> Vec<CardType> {
    match option {
        TypeOption::Trap => vec![CardType::Trap, CardType::FieldTrap],
        TypeOption::Spell => vec![CardType::Spell],
        TypeOption::Unit => vec![CardType::Unit],
        TypeOption::FieldSpell => vec![CardType::FieldSpell],
    }
}

/// R65's out-of-play cost range a bracket means; "4+" has no upper bound.
fn range_for(bracket: BracketOption) -> BracketRange {
    match bracket {
        BracketOption::ZeroToOne => BracketRange {
            min: Some(0),
            max: Some(1),
        },
        BracketOption::Two => BracketRange {
            min: Some(2),
            max: Some(2),
        },
        BracketOption::Three => BracketRange {
            min: Some(3),
            max: Some(3),
        },
        BracketOption::FourPlus => BracketRange {
            min: Some(4),
            max: None,
        },
    }
}

fn matches_type(ctx: &EffectContext<'_>, card: &CardInstance, option: TypeOption) -> bool {
    let type_: CardType = def_of(Some(&*ctx.state), &card.def_id).type_;
    types_for(option).contains(&type_)
}

/// R65: the library card's own cost (`effective_cost`), as #30 and #94 read theirs (R24, R66).
fn matches_bracket(ctx: &EffectContext<'_>, card: &CardInstance, bracket: BracketOption) -> bool {
    let cost = effective_cost(&*ctx.state, card, Default::default());
    let range = range_for(bracket);
    if let Some(min) = range.min
        && cost < min
    {
        return false;
    }
    range.max.is_none_or(|max| cost <= max)
}

/// "Your library" (§8 Conventions: "your" means the controller), top card first. Reading state,
/// never touching it: `zone_cards` is the engine's read-only pile reader (engine/src/query.rs) and
/// hands back a copy (BUILD M3-T1).
///
/// R218: a unit-token card in the library (#33's copy of a played Rush Token card, R34) leaves it only
/// by being drawn or played (R11), and "choose one to hand" is neither, so the Tutor passes over it —
/// as the reveal does (`discover_from_library`) — and never offers a type or a bracket only it matches.
fn library_cards(ctx: &EffectContext<'_>) -> Vec<CardInstance> {
    let state: &GameState = &*ctx.state;
    zone_cards(state, ctx.controller, OffFieldZone::Library)
        .into_iter()
        .filter(|card| !is_unit_token(state, card))
        .collect()
}

/// A `chosen_options` answer narrowed back to the option it must be, or `None` (never a cast).
fn type_option_of(picked: Option<&str>) -> Option<TypeOption> {
    TYPE_OPTIONS.iter().copied().find(|option| Some(option.as_str()) == picked)
}

fn bracket_option_of(picked: Option<&str>) -> Option<BracketOption> {
    BRACKET_OPTIONS.iter().copied().find(|option| Some(option.as_str()) == picked)
}

/// §10.6's captured data is JSON, so what comes back out is narrowed, not asserted.
fn captured_type(ctx: &EffectContext<'_>) -> Option<TypeOption> {
    let stored = ctx.data.get(TYPE_KEY).and_then(Value::as_str);
    type_option_of(stored)
}

/// Step 1 (the spell's own resolution, §10.5 step 5): offer the types that have a match in the
/// library. No type does — an empty library — so the Notebook is added instead.
fn start_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let ctx: &EffectContext<'_> = ctx;
    let library = library_cards(ctx);
    let options: Vec<&'static str> = TYPE_OPTIONS
        .iter()
        .copied()
        .filter(|option| library.iter().any(|card| matches_type(ctx, card, *option)))
        .map(TypeOption::as_str)
        .collect();
    if options.is_empty() {
        return vec![add_to_hand(json_as(json!({ "defId": NOTEBOOK })))];
    }

    vec![choose_mode(json_as(json!({
        "options": options,
        "step": "bracket",
        "prompt": "Choose a card type",
    })))]
}

/// Step 2: the type is answered, so offer the cost brackets that have a match FOR THAT TYPE, and
/// carry the type forward — the reveal step needs it and `ctx.targets` will by then hold the
/// bracket answer instead.
fn bracket_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let ctx: &EffectContext<'_> = ctx;
    let Some(type_) = type_option_of(chosen_options(ctx).first().map(String::as_str)) else {
        return vec![];
    };

    let matching: Vec<CardInstance> = library_cards(ctx)
        .into_iter()
        .filter(|card| matches_type(ctx, card, type_))
        .collect();
    let options: Vec<&'static str> = BRACKET_OPTIONS
        .iter()
        .copied()
        .filter(|bracket| matching.iter().any(|card| matches_bracket(ctx, card, *bracket)))
        .map(BracketOption::as_str)
        .collect();
    if options.is_empty() {
        return vec![];
    }

    vec![choose_mode(json_as(json!({
        "options": options,
        "step": "reveal",
        "data": { TYPE_KEY: type_.as_str() },
        "prompt": format!("Choose a cost bracket for {}", type_.as_str()),
    })))]
}

/// Step 3: both halves of the filter are known, so reveal three random matching library cards. The
/// options are the library's own instances, which is what keeps §10.8's "revealed only as an option
/// of the prompt that reveals it" true.
fn reveal_step(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    let ctx: &EffectContext<'_> = ctx;
    let type_ = captured_type(ctx);
    let bracket = bracket_option_of(chosen_options(ctx).first().map(String::as_str));
    let (Some(type_), Some(bracket)) = (type_, bracket) else {
        return vec![];
    };

    vec![discover_from_library(json_as(json!({
        "step": "take",
        "count": REVEAL_COUNT,
        "player": "self",
        "filter": { "type": types_for(type_), "costRange": range_for(bracket).to_json() },
        // R373: the prompt is a player's to read, and players read the rules' library as the Deck.
        "prompt": "Reveal 3 cards from your deck; choose one to add to your hand",
    })))]
}

/// Step 4: "choose one to hand" — the revealed instance moves library → hand (§6.3, R4).
fn take_step(_ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![add_to_hand(json_as(json!({ "instance": { "of": "chosen" } })))]
}

/// The three named continuations a prompt answer re-enters (`RESUME_HOOK`, §10.6).
fn steps() -> IndexMap<&'static str, Hook> {
    IndexMap::from([
        ("bracket", hook(bracket_step)),
        ("reveal", hook(reveal_step)),
        ("take", hook(take_step)),
    ])
}

/// "Echo (resolves a second time)": one EXTRA resolution, driven by §10.5 step 6.
fn radiant_flags() -> StaticFlags {
    StaticFlags {
        echo: Some(1),
        ..StaticFlags::default()
    }
}

pub fn script() -> CardScripts {
    let steps = steps();
    let base = Script {
        cry: Some(hook(start_step)),
        resume: steps.clone(),
        ..Script::default()
    };
    // The radiant face is the same four steps — each repeat re-enters `cry` and opens its own fresh
    // prompts (§6.2 Echo X, §10.5 step 6) — plus the count of repeats it owes.
    let radiant = Script {
        cry: Some(hook(start_step)),
        resume: steps,
        static_flags: Some(radiant_flags()),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// #51 KY's Private Tutor — SPEC §8.3 row 51, §6.3 (Discover), §10.5 steps 5-6, §10.6, §10.8,
// §5.1; R4, R60, R65, R113.
//
// BUILD M4-T4 row 51: "Only types and brackets with a match offered; 3 random matches revealed; no
// match → Notebook; Field Trap counts as Trap; radiant runs twice".
//
// The card is a four-step machine whose middle three steps are prompts (§10.6), so almost every
// case below is a chain: play, then `answer` once per prompt. R113 is the rule that makes the chain
// a test subject in its own right — "a work item that cannot be resumed is a lost sequence … and
// must never be dropped in silence" — so the cases assert both halves of it: answering one prompt
// opens the next, and the card really finishes (no prompt left open, no work owed, the Spell in the
// graveyard). One case takes the paused game through a JSON round trip and resumes the revived
// state through `reduce`, because §9.3's "mid-action choices are state, not callbacks" is only true
// if the pause survives serialization.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TUTOR: &str = "core-051";
    const NOTEBOOK: &str = "core-051-1";

    /// §2.5/R82 TURN ANCHOR. #10 Rapid Replenish is a 0-cost Spell and therefore always an affordable
    /// play, so one in hand keeps p1's turn from auto-ending once the Tutor has left the hand — which
    /// would clear `turn_log` and deal fatigue under the assertions. It is deliberately NOT one of the
    /// library fixtures below, so no assertion about a library card is ambiguous about which copy it
    /// means.
    const ANCHOR: &str = "core-010";

    // The library fixture, chosen so each of the four type options has a match and each type offers a
    // different set of brackets. Costs are the printed ones; R65's out-of-play reading is its own case.
    const SPELL_0A: &str = "core-039"; // Recycling Initiative — Spell, 0
    const SPELL_0B: &str = "core-048"; // 5pek Controller      — Spell, 0
    const SPELL_1A: &str = "core-005"; // Stockpile            — Spell, 1
    const SPELL_1B: &str = "core-035"; // Lunar Eclipse        — Spell, 1
    const SPELL_2: &str = "core-069"; //  Call to Arms         — Spell, 2 (Hit Job costs 3 since patch v0.2.0)
    const UNIT_1: &str = "core-008"; //   Mr. Vanilla          — Unit, 1
    const UNIT_4: &str = "core-025"; //   4-mana 7/7           — Unit, 4
    const FIELD_3: &str = "core-006"; //  Mana Well            — Field Spell, 3
    const FIELD_TRAP_1: &str = "core-018"; // Bread and Butter — Field Trap, 1
    const TRAP_1: &str = "core-041"; //   Sheepish             — Trap, 1

    const LIBRARY: [&str; 10] = [
        SPELL_0A,
        SPELL_0B,
        SPELL_1A,
        SPELL_1B,
        SPELL_2,
        UNIT_1,
        UNIT_4,
        FIELD_3,
        FIELD_TRAP_1,
        TRAP_1,
    ];

    const SEED: &str = "ky-tutor";

    fn must<T>(value: Option<T>, what: &str) -> T {
        value.unwrap_or_else(|| panic!("the scenario has no {what}"))
    }

    fn open(s: &Scenario) -> PendingChoice {
        must(s.state().pending.clone(), "an open prompt")
    }

    /// A `mode` prompt's options as the plain option strings the card declared (§10.6).
    fn mode_options(pending: &PendingChoice) -> Vec<String> {
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Mode { option } => Some(option.clone()),
                _ => None,
            })
            .collect()
    }

    /// A Discover-from-library prompt's options are LIBRARY INSTANCES, so read their def ids.
    fn revealed_def_ids(s: &Scenario, pending: &PendingChoice) -> Vec<String> {
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(s.card(instance_id).def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn revealed_instance_ids(pending: &PendingChoice) -> Vec<String> {
        pending
            .options
            .iter()
            .filter_map(|option| match &option.selection {
                Selection::Instance { instance_id } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn hand_def_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.def_id.clone()).collect()
    }

    fn hand_ids(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.id.clone()).collect()
    }

    /// TS `tutor(opts)`'s options: `{ radiant?: boolean; library?: readonly string[] }`.
    #[derive(Default)]
    struct TutorOpts {
        radiant: bool,
        library: Option<Vec<&'static str>>,
    }

    fn tutor(opts: TutorOpts) -> Scenario {
        let card = if opts.radiant {
            json!({ "def": TUTOR, "radiant": true })
        } else {
            json!(TUTOR)
        };
        let library = opts.library.unwrap_or_else(|| LIBRARY.to_vec());
        scenario(json!({
            "seed": SEED,
            "p1": { "hand": [card, ANCHOR], "library": library },
            "p2": { "hand": [ANCHOR], "library": LIBRARY },
        }))
    }

    /// `tutor({ library })`.
    fn tutor_with(library: &[&'static str]) -> Scenario {
        tutor(TutorOpts {
            library: Some(library.to_vec()),
            ..TutorOpts::default()
        })
    }

    /// `tutor({ radiant: true, library })`.
    fn radiant_tutor_with(library: Option<&[&'static str]>) -> Scenario {
        tutor(TutorOpts {
            radiant: true,
            library: library.map(<[&'static str]>::to_vec),
        })
    }

    /// The chain finished cleanly: no prompt open, nothing owed, the Spell in the graveyard (R113).
    fn expect_chain_finished(s: &mut Scenario) {
        assert!(s.state().pending.is_none());
        assert!(s.state().work.is_empty());
        s.expect_in_zone(TUTOR, "graveyard");
    }

    /// TS `Object.keys(script).sort()`: the fields a face declares, by their TS names.
    fn keys_of(script: &Script) -> Vec<&'static str> {
        let present: [(&'static str, bool); 36] = [
            ("cost", script.cost.is_some()),
            ("cry", script.cry.is_some()),
            ("death", script.death.is_some()),
            ("startOfGame", script.start_of_game.is_some()),
            ("resume", !script.resume.is_empty()),
            ("delayed", script.delayed.is_some()),
            ("setStat", script.set_stat.is_some()),
            ("startOfTurn", script.start_of_turn.is_some()),
            ("endOfTurn", script.end_of_turn.is_some()),
            ("aura", script.aura.is_some()),
            ("triggers", !script.triggers.is_empty()),
            ("onPlayHook", script.on_play_hook.is_some()),
            ("handTriggers", !script.hand_triggers.is_empty()),
            ("staticFlags", script.static_flags.is_some()),
            ("targets", !script.targets.is_empty()),
            ("modes", !script.modes.is_empty()),
            ("conditionMet", script.condition_met.is_some()),
            ("preview", script.preview.is_some()),
            ("activations", !script.activations.is_empty()),
            ("targetChecks", !script.target_checks.is_empty()),
            ("costAura", script.cost_aura.is_some()),
            ("graveyardPlay", script.graveyard_play.is_some()),
            ("targetingDiscards", script.targeting_discards.is_some()),
            ("recordsPlayAs", script.records_play_as.is_some()),
            ("drawLimit", script.draw_limit.is_some()),
            ("replacements", !script.replacements.is_empty()),
            ("heroGuard", script.hero_guard.is_some()),
            ("conditionalKeywords", script.conditional_keywords.is_some()),
            ("afterAttack", script.after_attack.is_some()),
            ("plagueMultiplier", script.plague_multiplier.is_some()),
            ("deckTriggers", !script.deck_triggers.is_empty()),
            ("graveyardTriggers", !script.graveyard_triggers.is_empty()),
            ("quests", script.quests.is_some()),
            ("tributeWhen", script.tribute_when.is_some()),
            ("wouldCounter", script.would_counter.is_some()),
            ("startOfOpponentTurn", script.start_of_opponent_turn.is_some()),
        ];
        let mut keys: Vec<&'static str> = present.iter().filter(|(_, is)| *is).map(|(key, _)| *key).collect();
        keys.sort();
        keys
    }

    /// TS `Object.keys(script.resume ?? {}).sort()`.
    fn step_names(script: &Script) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = script.resume.keys().copied().collect();
        names.sort();
        names
    }

    fn sorted(mut ids: Vec<String>) -> Vec<String> {
        ids.sort();
        ids
    }

    // ---------------------------------------------------------------------------
    // The card's shape (§8.3, §10.9)
    // ---------------------------------------------------------------------------

    mod n51_ky_s_private_tutor_the_card {
        use super::*;

        #[test]
        fn s8_3_is_a_1_cost_epic_spell_tagged_ky() {
            crate::register_all();
            let def = crate::card_def(super::super::ID);
            assert_eq!(def.id, TUTOR);
            assert_eq!(def.type_, CardType::Spell);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert_eq!(def.rarity, Rarity::Epic);
            assert!(def.tags.contains(&Tag::Ky));
        }

        #[test]
        fn s10_9_both_faces_are_a_cry_plus_the_same_three_named_continuations_s10_6() {
            crate::register_all();
            let CardScripts { base, radiant } = super::super::script();
            assert_eq!(keys_of(&base), vec!["cry", "resume"]);
            assert_eq!(step_names(&base), vec!["bracket", "reveal", "take"]);
            assert_eq!(step_names(&radiant), vec!["bracket", "reveal", "take"]);
            // R81: nothing about this card travels in the `play` action — every choice is a prompt.
            assert!(base.targets.is_empty());
            assert!(base.modes.is_empty());
        }

        #[test]
        fn s8_3_the_radiant_cell_is_echo_so_the_radiant_face_is_the_same_steps_plus_one_repeat() {
            crate::register_all();
            let CardScripts { base, radiant } = super::super::script();
            assert_eq!(
                serde_json::to_value(&radiant.static_flags).unwrap(),
                json!({ "echo": 1 })
            );
            assert!(base.static_flags.is_none());
        }
    }

    // ---------------------------------------------------------------------------
    // The chain (§10.6, R113)
    // ---------------------------------------------------------------------------

    mod n51_ky_s_private_tutor_base {
        use super::*;

        #[test]
        fn r113_three_chained_prompts_type_then_bracket_then_the_reveal_and_the_card_finishes() {
            crate::register_all();
            let mut s = tutor(TutorOpts::default());
            s.play(TUTOR, json!({}));

            // Prompt 1 — the type, to the caster.
            let types = open(&s);
            assert_eq!(types.kind, PromptKind::Mode);
            assert_eq!(types.player_id, P1);
            assert_eq!(mode_options(&types), vec!["Spell", "Unit", "Field Spell", "Trap"]);

            // Answering it opens the NEXT one rather than ending the sequence (R113).
            s.answer(json!("Spell"));
            let brackets = open(&s);
            assert_eq!(brackets.kind, PromptKind::Mode);
            assert_ne!(brackets.id, types.id);

            s.answer(json!("0-1"));
            let reveal = open(&s);
            assert_eq!(reveal.kind, PromptKind::Discover);
            assert_ne!(reveal.id, brackets.id);
            let picked = must(revealed_instance_ids(&reveal).first().cloned(), "a revealed card");

            s.answer(json!(picked));

            // The fourth step is not a prompt: it moves the pick and the sequence ends.
            expect_chain_finished(&mut s);
            assert!(hand_ids(&s).contains(&picked));
            s.expect_events(json!(["cardPlayed", "promptOpened", "promptAnswered", "addedToHand", "cardResolved"]));
        }

        #[test]
        fn r113_the_paused_game_survives_a_json_round_trip_and_resumes_from_the_revived_state() {
            crate::register_all();
            let mut s = tutor(TutorOpts::default());
            s.play(TUTOR, json!({}));
            s.answer(json!("Spell"));
            s.answer(json!("0-1"));

            let paused = s.state().clone();
            assert_eq!(paused.pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Discover));

            // §9.3: a mid-action choice is state, not a callback. Nothing in the pause is a closure, so a
            // round trip through JSON is the same game — the prompt, its options and its `resume` included.
            let revived: GameState =
                serde_json::from_value(serde_json::to_value(&paused).expect("the state serialises"))
                    .expect("the state deserialises");
            assert_eq!(revived, paused);
            assert_eq!(
                revived.pending.as_ref().map(|pending| &pending.resume),
                paused.pending.as_ref().map(|pending| &pending.resume)
            );

            // And it really resumes: the revived state answers through the ordinary reducer and the card
            // finishes there, with no prompt left open and nothing owed on `state.work` (R113).
            let revived_prompt = must(revived.pending.clone(), "a revived prompt");
            let choice_id = revived_prompt.id.clone();
            let instance_id = must(revealed_instance_ids(&revived_prompt).first().cloned(), "an option");
            let action: Action = json_as(json!({
                "type": "answer",
                "playerId": "p1",
                "choiceId": choice_id,
                "selection": [{ "pick": "instance", "instanceId": instance_id }],
                "nonce": "round-trip-1",
            }));
            let result = reduce(&revived, &action);

            assert!(result.error.is_none());
            assert!(result.state.pending.is_none());
            assert!(result.state.work.is_empty());
            assert!(result.state.players.p1.hand.iter().any(|card| card.id == instance_id));
        }

        #[test]
        fn s8_3_only_types_with_a_match_in_the_library_are_offered() {
            crate::register_all();
            // A library of nothing but Units: three of the four options have no match and are not offered.
            let mut s = tutor_with(&[UNIT_1, UNIT_4]);
            s.play(TUTOR, json!({}));

            assert_eq!(mode_options(&open(&s)), vec!["Unit"]);
        }

        #[test]
        fn s8_3_only_brackets_with_a_match_for_the_chosen_type_are_offered() {
            crate::register_all();
            let mut s = tutor(TutorOpts::default());
            s.play(TUTOR, json!({}));

            // Spells in the fixture cost 0, 0, 1, 1 and 2 — so "0-1" and "2", never "3" or "4+".
            s.answer(json!("Spell"));
            assert_eq!(mode_options(&open(&s)), vec!["0-1", "2"]);
        }

        #[test]
        fn s8_3_the_brackets_are_recomputed_per_type_not_fixed() {
            crate::register_all();
            let mut units = tutor(TutorOpts::default());
            units.play(TUTOR, json!({})).answer(json!("Unit"));
            // Mr. Vanilla at 1 and the 4-mana 7/7 at 4: the two ends and nothing between them.
            assert_eq!(mode_options(&open(&units)), vec!["0-1", "4+"]);

            let mut fields = tutor(TutorOpts::default());
            fields.play(TUTOR, json!({})).answer(json!("Field Spell"));
            // Mana Well is the only Field Spell in the fixture, at 3.
            assert_eq!(mode_options(&open(&fields)), vec!["3"]);
        }

        #[test]
        fn s8_3_reveals_3_random_matching_library_cards_all_of_them_matching_the_two_answers() {
            crate::register_all();
            let mut s = tutor(TutorOpts::default());
            s.play(TUTOR, json!({})).answer(json!("Spell")).answer(json!("0-1"));

            let reveal = open(&s);
            // Four Spells in the fixture cost 0 or 1, so the Discover offers three of them.
            assert_eq!(reveal.options.len(), 3);
            let distinct: IndexSet<String> = revealed_instance_ids(&reveal).into_iter().collect();
            assert_eq!(distinct.len(), 3);
            for def_id in revealed_def_ids(&s, &reveal) {
                assert!([SPELL_0A, SPELL_0B, SPELL_1A, SPELL_1B].contains(&def_id.as_str()));
            }
        }

        #[test]
        fn s6_3_fewer_than_three_matches_reveal_what_exists() {
            crate::register_all();
            let mut s = tutor_with(&[SPELL_2, UNIT_1]);
            s.play(TUTOR, json!({})).answer(json!("Spell"));
            // Call to Arms is the only Spell, so "2" is the only bracket and it is the only reveal.
            assert_eq!(mode_options(&open(&s)), vec!["2"]);

            s.answer(json!("2"));
            let reveal = open(&s);
            assert_eq!(revealed_def_ids(&s, &reveal), vec![SPELL_2]);
        }

        #[test]
        fn r60_the_three_revealed_cards_are_the_seed_s_so_a_replay_reveals_the_same_three() {
            crate::register_all();
            let mut first = tutor(TutorOpts::default());
            first.play(TUTOR, json!({})).answer(json!("Spell")).answer(json!("0-1"));
            let mut second = tutor(TutorOpts::default());
            second.play(TUTOR, json!({})).answer(json!("Spell")).answer(json!("0-1"));

            assert_eq!(
                revealed_def_ids(&second, &open(&second)),
                revealed_def_ids(&first, &open(&first))
            );
        }

        #[test]
        fn s6_3_the_chosen_card_moves_library_hand_it_is_not_copied() {
            crate::register_all();
            let mut s = tutor(TutorOpts::default());
            let library_before = s.pile(P1, "library").len();
            s.play(TUTOR, json!({})).answer(json!("Spell")).answer(json!("0-1"));
            let picked = must(revealed_instance_ids(&open(&s)).first().cloned(), "a revealed card");

            s.answer(json!(picked));

            s.expect_in_zone(&picked, "hand");
            assert_eq!(s.pile(P1, "library").len(), library_before - 1);
            assert!(!s.pile(P1, "library").iter().any(|card| card.id == picked));
            // The same instance, not a fresh one made from its def.
            assert_eq!(s.hand(P1).iter().filter(|card| card.id == picked).count(), 1);
        }

        #[test]
        fn the_engine_cell_s_field_trap_counts_as_trap_the_trap_option_reaches_a_field_trap() {
            crate::register_all();
            let mut s = tutor_with(&[FIELD_TRAP_1]);
            s.play(TUTOR, json!({}));

            // A `type` filter matches the field exactly, so "Trap" has to name both or #18 Bread and Butter
            // and #71 Intern Stimmy would silently vanish from this card (the same reading as R35 and R61).
            assert_eq!(mode_options(&open(&s)), vec!["Trap"]);

            s.answer(json!("Trap"));
            assert_eq!(mode_options(&open(&s)), vec!["0-1"]);

            s.answer(json!("0-1"));
            assert_eq!(revealed_def_ids(&s, &open(&s)), vec![FIELD_TRAP_1]);
        }

        #[test]
        fn the_engine_cell_s_trap_also_still_reaches_an_ordinary_trap_and_both_together() {
            crate::register_all();
            let mut s = tutor_with(&[TRAP_1, FIELD_TRAP_1]);
            s.play(TUTOR, json!({})).answer(json!("Trap")).answer(json!("0-1"));

            assert_eq!(
                sorted(revealed_def_ids(&s, &open(&s))),
                sorted(vec![FIELD_TRAP_1.to_string(), TRAP_1.to_string()])
            );
        }

        #[test]
        fn r65_the_brackets_read_costs_out_of_play_an_x_cost_card_is_0_and_an_embiggen_card_its_base() {
            crate::register_all();
            // #74 Adaptive UI is a Spell printed "X"; R65 reads it as 0 in a library, so it is in "0-1".
            let mut x_cost = tutor_with(&["core-074"]);
            x_cost.play(TUTOR, json!({})).answer(json!("Spell"));
            assert_eq!(mode_options(&open(&x_cost)), vec!["0-1"]);

            // #59 Unbiased Immigration is a Field Spell printed "2 embiggen 4"; out of play it reads 2.
            let mut embiggen = tutor_with(&["core-059"]);
            embiggen.play(TUTOR, json!({})).answer(json!("Field Spell"));
            assert_eq!(mode_options(&open(&embiggen)), vec!["2"]);
        }

        #[test]
        fn s8_3_no_match_at_all_an_empty_library_adds_a_ky_s_empty_notebook_and_opens_no_prompt() {
            crate::register_all();
            let mut s = tutor_with(&[]);

            s.play(TUTOR, json!({}));

            // No type has a match exactly when the library is empty, since every card in it has one of the
            // five types and all five map onto the four options.
            assert!(s.state().pending.is_none());
            assert!(hand_def_ids(&s).contains(&NOTEBOOK.to_string()));
            expect_chain_finished(&mut s);
            s.expect_events(json!(["cardPlayed", "addedToHand", "cardResolved"]));
        }

        #[test]
        fn s5_1_the_notebook_is_created_here_and_nowhere_else_it_is_not_taken_out_of_the_library() {
            crate::register_all();
            let mut s = tutor_with(&[]);
            s.play(TUTOR, json!({}));

            // §7/R11: a fresh token instance, owned by the caster.
            let notebook = must(
                s.hand(P1).into_iter().find(|card| card.def_id == NOTEBOOK),
                "a Notebook in hand",
            );
            assert_eq!(notebook.owner, P1);
            assert!(s.pile(P1, "library").is_empty());
        }

        #[test]
        fn s10_8_the_reveal_is_shown_to_the_caster_only() {
            crate::register_all();
            let mut s = tutor(TutorOpts::default());
            s.play(TUTOR, json!({})).answer(json!("Spell")).answer(json!("0-1"));

            let mine = must(s.view(P1).pending, "p1's own prompt view");
            assert_eq!(serde_json::to_value(&mine).unwrap()["forYou"], json!(true));

            // The opponent learns that p1 is choosing and nothing else: no options, so no card that was
            // revealed out of p1's library reaches p2's view.
            let theirs = must(s.view(P2).pending, "p2's view of the prompt");
            assert_eq!(serde_json::to_value(&theirs).unwrap()["forYou"], json!(false));
            let text = serde_json::to_string(&theirs).expect("a prompt view serialises");
            for def_id in revealed_def_ids(&s, &open(&s)) {
                assert!(!text.contains(&def_id));
            }
        }
    }

    // ---------------------------------------------------------------------------
    // The radiant face: Echo (§6.2, §10.5 step 6)
    // ---------------------------------------------------------------------------

    mod n51_ky_s_private_tutor_radiant {
        use super::*;

        #[test]
        fn s8_3_echo_the_whole_four_step_sequence_runs_a_second_time() {
            crate::register_all();
            let mut s = radiant_tutor_with(None);
            let library_before = s.pile(P1, "library").len();

            s.play(TUTOR, json!({}));

            // First resolution: the same three prompts as the base face.
            assert_eq!(mode_options(&open(&s)), vec!["Spell", "Unit", "Field Spell", "Trap"]);
            s.answer(json!("Spell")).answer(json!("0-1"));
            let first_pick = must(revealed_instance_ids(&open(&s)).first().cloned(), "a first reveal");
            s.answer(json!(first_pick));

            // §10.5 step 6: the repeat opens its OWN fresh prompts rather than finishing the card.
            let second_types = open(&s);
            assert_eq!(second_types.kind, PromptKind::Mode);
            s.answer(json!("Unit")).answer(json!("0-1"));
            let second_reveal = open(&s);
            assert_eq!(second_reveal.kind, PromptKind::Discover);
            let second_pick = must(revealed_instance_ids(&second_reveal).first().cloned(), "a second reveal");

            s.answer(json!(second_pick));

            expect_chain_finished(&mut s);
            let hand = hand_ids(&s);
            assert!(hand.contains(&first_pick));
            assert!(hand.contains(&second_pick));
            assert_eq!(s.pile(P1, "library").len(), library_before - 2);
        }

        #[test]
        fn s8_3_the_repeat_reads_the_library_as_it_is_now_the_first_pick_is_not_offered_again() {
            crate::register_all();
            // Two Spells at 0-1, so the first resolution takes one and the second can only be the other.
            let mut s = radiant_tutor_with(Some(&[SPELL_0A, SPELL_1A, UNIT_1]));
            s.play(TUTOR, json!({})).answer(json!("Spell")).answer(json!("0-1"));
            let first_reveal = open(&s);
            assert_eq!(
                sorted(revealed_def_ids(&s, &first_reveal)),
                sorted(vec![SPELL_0A.to_string(), SPELL_1A.to_string()])
            );
            let first_pick = must(revealed_instance_ids(&first_reveal).first().cloned(), "a first reveal");
            s.answer(json!(first_pick));

            s.answer(json!("Spell")).answer(json!("0-1"));
            let second_reveal = open(&s);

            assert!(!revealed_instance_ids(&second_reveal).contains(&first_pick));
            assert_eq!(revealed_instance_ids(&second_reveal).len(), 1);
        }

        #[test]
        fn s8_3_the_radiant_face_still_falls_back_to_the_notebook_twice_on_an_empty_library() {
            crate::register_all();
            let mut s = radiant_tutor_with(Some(&[]));

            s.play(TUTOR, json!({}));

            // Neither resolution finds a match, and neither opens a prompt.
            assert!(s.state().pending.is_none());
            assert_eq!(hand_def_ids(&s).iter().filter(|def_id| *def_id == NOTEBOOK).count(), 2);
            expect_chain_finished(&mut s);
        }

        #[test]
        fn s8_conventions_the_radiant_cell_restates_none_of_the_base_clause_so_every_step_is_kept() {
            crate::register_all();
            let mut s = radiant_tutor_with(Some(&[FIELD_TRAP_1, UNIT_1]));
            s.play(TUTOR, json!({}));

            // The type filter, the bracket filter and "Field Trap counts as Trap" all still apply.
            assert_eq!(mode_options(&open(&s)), vec!["Unit", "Trap"]);
            s.answer(json!("Trap")).answer(json!("0-1"));
            assert_eq!(revealed_def_ids(&s, &open(&s)), vec![FIELD_TRAP_1]);
        }
    }

    mod n51_ky_s_private_tutor_r218_a_unit_token_card_in_the_library {
        use super::*;

        #[test]
        fn r218_a_unit_token_card_in_the_library_never_reaches_a_hand_through_the_tutor_s3_2_r11() {
            crate::register_all();
            // #33 copies of a played Rush Token card (R34) are how one gets into a library. R11: it
            // "ceases to exist if it leaves that zone other than by being drawn or played", and the Tutor's
            // "choose one to hand" is neither, so R218's reasoning for Recruit holds here too. Either the
            // Tutor passes over it (so "Unit" is not offered at all) or the card ceases to exist on the
            // way; it never lands in the hand.
            let mut g = scenario(json!({
                "p1": { "hand": ["core-051", "core-010"], "library": ["core-t-rush", "core-005"] },
                "p2": { "hand": ["core-008"], "library": LIBRARY },
            }));
            let token = must(
                g.pile(P1, "library").into_iter().find(|card| card.def_id == "core-t-rush"),
                "the Rush Token card in p1's library",
            );

            g.play("core-051", json!({}));
            let types: Option<Vec<String>> = g.state().pending.as_ref().map(mode_options);
            if types.is_some_and(|types| types.contains(&"Unit".to_string())) {
                g.answer(json!("Unit"));
                g.answer(json!("0-1"));
                g.answer(json!([{ "pick": "instance", "instanceId": token.id }]));
            }

            assert!(!g.pile(P1, "hand").iter().any(|card| card.id == token.id));
        }
    }
}
