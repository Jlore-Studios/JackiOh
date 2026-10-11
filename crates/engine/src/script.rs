//! The card-script contract (SPEC §10.9). A card file returns `Vec<Effect>` from the effects library
//! and never touches state itself (CLAUDE.md rule 5); the engine applies the effects.
//!
//! Port of `packages/engine/src/script.ts` (part 1's type freeze, SURFACE §6.4–§6.6, §7). TS's
//! closure objects stay closures (§6.6): a hook is an `Arc<dyn Fn …>` built with `hook(|ctx| …)` (or
//! the typed constructors below), an effect is `Effect { kind, apply, expand }`. Also here:
//!
//! - `EngineSink` (TS `resolve.ts`'s `{ state, events, rng }`, SURFACE §6.5) and `EffectContext`,
//!   which *is* a sink plus the run's facts: it derefs to its `EngineSink`, so `ctx.state` reads as in
//!   TS and a TS call `f(ctx)` to a sink-taking function is `f(ctx)` here too.
//! - The types a `Script` field names that TS declares in another module, because the script must
//!   compile on its own: `CostAura`, `CostAuraWhose`, `CostAuraArgs` (TS `costRules.ts`),
//!   `GraveyardPlayPermission` (`graveyardPlay.ts`), `QuestBook` and its parts (`subsystems/quests.ts`),
//!   and the replacement declarations `ReplacementDef` … `ReplacedEvent` (`replacements.ts`, which
//!   `script.ts` re-exported). Their TS modules' ports use these and do not redefine them.
//!
//! Hooks take `&mut EffectContext`: several TS card hooks draw from `ctx.rng` while they build their
//! list (Classic #10, #65, C+ #7, #8, #19.2, #19.3, #25, #37, #53, Core #23, #83), which a shared
//! reference cannot do. Every read-only hook (`ConditionHook`, `AuraHook`, …) takes plain-data
//! arguments by value, each a `Copy` bundle of references.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rng::Rng;
use crate::state::{CardInstance, CostRule, EventStay, GameState, PlayRecord};
use crate::wire::{
    CardType, GameEvent, GameEventType, Keyword, ModeDecl, PlayerId, PreviewValue, Selection, Tag,
    TargetDecl, string_union,
};

// ---------------------------------------------------------------------------------------------
// The sink and the context
// ---------------------------------------------------------------------------------------------

/// TS `resolve.ts`'s `EngineSink = { state, events, rng }` (SURFACE §6.5): what every mutator that
/// emits events or draws writes to, plus the per-call facts that were module `let`s or transient sink
/// fields in TS. A nested sink (`reborrow`) carries the flags down and shares the frontier; a TS
/// mutator that took a sink takes `&mut EngineSink`.
pub struct EngineSink<'a> {
    pub state: &'a mut GameState,
    pub events: &'a mut Vec<GameEvent>,
    pub rng: &'a mut Rng,
    /// `replacements.ts:383`'s re-entrancy counter, formerly a module `let`.
    pub converting: u32,
    /// `subsystems/scorer.ts:225`'s flag, formerly a module `let`.
    pub dry_running: bool,
    /// Which of the action's events the frontier has taken (TS `SettleSink.dispatched` and
    /// `triggers.ts`'s `collected`/`dispatchedElsewhere`), shared by every sink over one event list.
    pub frontier: crate::triggers::FrontierSlot<'a>,
    /// R117: the owed items behind the one a drain is running (TS `work.ts`'s `DrainSink.owedBehind`).
    pub owed_behind: Option<indexmap::IndexSet<String>>,
}

impl<'a> EngineSink<'a> {
    /// A sink with both per-call flags at rest.
    pub fn new(state: &'a mut GameState, events: &'a mut Vec<GameEvent>, rng: &'a mut Rng) -> EngineSink<'a> {
        EngineSink {
            state,
            events,
            rng,
            converting: 0,
            dry_running: false,
            frontier: crate::triggers::FrontierSlot::default(),
            owed_behind: None,
        }
    }

    /// The same sink, borrowed again for a nested call: the flags travel down with it (every change
    /// to `converting` is undone before the call that made it returns, so a copy is exact) and the
    /// frontier is shared, so a loop on a context and the action's own loop see one frontier.
    pub fn reborrow(&mut self) -> EngineSink<'_> {
        EngineSink {
            state: &mut *self.state,
            events: &mut *self.events,
            rng: &mut *self.rng,
            converting: self.converting,
            dry_running: self.dry_running,
            frontier: crate::triggers::FrontierSlot::Shared(self.frontier.get_mut()),
            owed_behind: self.owed_behind.clone(),
        }
    }
}

/// What a running script sees (TS `EffectContext`). It is an `EngineSink` (TS: the context has the
/// sink's three fields, and is passed wherever a sink is taken) plus the facts of this run.
pub struct EffectContext<'a> {
    /// `state`, `rng`, `events` and the per-call flags; reached through `Deref` as `ctx.state` etc.
    pub sink: EngineSink<'a>,
    /// R136: where *this script's* events begin in `events`. The array is the whole action's sink, so
    /// a card that asks "what did I just do" — #60 Bear Honeypot's "they attack it", and the same
    /// shape in #24, #31, #33, #38 — must read `events[events_from..]` and never the earlier
    /// entries, or a second copy of a card, or a trap firing mid-action, feeds its condition. Set once
    /// where the context is built (`resolve::make_context`), so a step re-entered after a prompt opens
    /// a new window on the action it resumes in; what the list did before the pause is `summoned`.
    pub events_from: usize,
    /// R174: the field's departures when this script's run began (`stays::exit_mark`), so an effect later
    /// in the list can tell a card an earlier one took off the field from the card that stood there
    /// when the run began — across a prompt too, since a paused list resumes with the mark it began
    /// with (`work::PausedStep.exits_from`). Set by `make_context`; absent reads as "now".
    pub exits_from: Option<u32>,
    /// R174, §10.6: the field's departures when `targets` were chosen, where that is later than the
    /// run began — the answer to this run's own prompt, picked as the prompt offered the board. A card
    /// the list took off the field before it asked, and that stood there again when the prompt offered
    /// it (a Reborn body, R83), is picked on that new stay, and the answered step's effect lands on it.
    /// Absent reads as `exits_from`: a play's declared targets were chosen as its run began.
    pub chosen_from: Option<u32>,
    /// R174, R212: the cards the event a queued trigger answers names, and the field's departures when
    /// that event happened (`stays::event_stay_of`). The loop hands the trigger its event some time
    /// later, so a card the event names is judged from then: a trigger that reads the played unit's id
    /// off its `cardPlayed` does not land on the Reborn body an earlier trigger on the same event made
    /// (R59). Every other card the run aims at is judged from `exits_from`, when the run began. Carried
    /// across a pause with the run's other marks. Absent for any run that is not a queued trigger's.
    pub event_stay: Option<EventStay>,
    /// R136: the units this script's run summoned in the actions before a prompt split it. The window
    /// `events_from` opens is the action's own event list, and a list the answer continues resumes in
    /// a later action, so what its head summoned is carried here (`work::PausedStep.summoned`,
    /// `work::RunMarks`). Absent for a run that has not paused.
    pub summoned: Option<Vec<String>>,
    /// R98: the card running the script sat in the resolving zone as the run began (§10.5 step 4) — a
    /// Spell resolving, or a permanent that found no zone. The run is that card's while it stays there,
    /// so a continuation re-entered once the card has left it — the Spell's own list put it back in its
    /// owner's hand before it asked — resumes with no self (`prompts::run_resume`). Carried across a
    /// pause with the run's other marks (`work::RunMarks`, `work::PausedStep`). Absent for any other card.
    pub self_resolving: Option<bool>,
    /// Who is resolving this: the controller of `self_`, or the player who cast the card.
    pub controller: PlayerId,
    /// The instance whose script is running, when it still exists (TS `self`).
    ///
    /// TS held the live object, so a write through it reached the state and a read saw every earlier
    /// effect's change. Here it is the instance as the context was built (a Death hook's is the card as
    /// it died, R89); `live_self` reads the card as it stands now.
    pub self_: Option<CardInstance>,
    /// R127: the definition whose script is running, set where a continuation is re-entered
    /// (`prompts::run_resume`), because `self_` is `None` once the card has ceased to exist and a step
    /// that asks again must still name its script. Absent elsewhere, where `self_` names it.
    pub def_id: Option<String>,
    /// Whether the radiant text is the one running (§5.2).
    pub radiant: bool,
    pub targets: Vec<Selection>,
    pub modes: Vec<String>,
    pub x: i32,
    pub embiggened: bool,
    /// Captured data from a Resume, for chained steps (§10.6).
    pub data: IndexMap<String, Value>,
    /// play pipeline B (Classic #22 Mid Runner: "If you had 4 or more mana when you played this"): the
    /// current mana of the player who played or cast the card as the play began — at §10.5 step 1, before
    /// step 2 paid, or as a cast began — on the played card's own Cry and every continuation of it
    /// (`resolve::MANA_BEFORE_PLAY_KEY`, which rides the card's data across a pause). Absent anywhere else.
    pub mana_before_play: Option<i32>,
}

impl<'a> EffectContext<'a> {
    /// A context over `sink` with every fact of the run at rest (no self, no targets, an empty data
    /// bag, the window opening where the event list stands). `resolve::make_context` is the engine's
    /// constructor; this one is for tests and for building one by hand.
    pub fn new(sink: EngineSink<'a>, controller: PlayerId) -> EffectContext<'a> {
        let events_from = sink.events.len();
        EffectContext {
            sink,
            events_from,
            exits_from: None,
            chosen_from: None,
            event_stay: None,
            summoned: None,
            self_resolving: None,
            controller,
            self_: None,
            def_id: None,
            radiant: false,
            targets: Vec::new(),
            modes: Vec::new(),
            x: 0,
            embiggened: false,
            data: IndexMap::new(),
            mana_before_play: None,
        }
    }

    /// The running card as it stands now (TS's live `ctx.self`): looked up by id, or the card the
    /// context was built with when it is no longer anywhere (a Death hook's card that ceased to exist).
    ///
    /// R89: a context whose data carries the card's snapshot (`prompts::SELF_KEY`: a Death hook, a dead
    /// attacker's After Attack, and every continuation of either) runs as that snapshot, never as the
    /// card R78 has reset on the board under the same id: TS's `ctx.self` was the snapshot object there
    /// (`stateCheck.runDeathPass`, `combat.runAfterAttack`, `prompts.runResume`), so a Death hook reads
    /// the memory its card died with (#22 Carnivorous Cube's meal, R41).
    pub fn live_self(&self) -> Option<&CardInstance> {
        let snapshot = self.self_.as_ref()?;
        if self.data.contains_key(crate::prompts::SELF_KEY) {
            return Some(snapshot);
        }
        crate::state::find_instance(self.sink.state, &snapshot.id).or(Some(snapshot))
    }
}

impl<'a> Deref for EffectContext<'a> {
    type Target = EngineSink<'a>;

    fn deref(&self) -> &EngineSink<'a> {
        &self.sink
    }
}

impl<'a> DerefMut for EffectContext<'a> {
    fn deref_mut(&mut self) -> &mut EngineSink<'a> {
        &mut self.sink
    }
}

// ---------------------------------------------------------------------------------------------
// Effects and hooks (SURFACE §6.6)
// ---------------------------------------------------------------------------------------------

/// What `Effect::expand` hands every rebuild so the part is the same one (TS `memo: unknown`, with
/// `undefined` on the first build): JSON, because it rides a paused step (`work::PausedStep.memo`).
pub type Memo = Option<Value>;

/// TS `apply: (ctx) => void`.
pub type EffectApply = Arc<dyn Fn(&mut EffectContext<'_>) + Send + Sync>;

/// TS `expand?: (ctx, memo) => EffectPart`.
pub type EffectExpand = Arc<dyn Fn(&mut EffectContext<'_>, &Memo) -> EffectPart + Send + Sync>;

/// One state change from the effects library. Effects are built by engine code, so a card file
/// composing them stays pure.
#[derive(Clone)]
pub struct Effect {
    pub kind: &'static str,
    pub apply: EffectApply,
    /// A part of a composed list, built when the list reaches it: a fused hook runs each ingredient's
    /// list in turn (R77, R102), and a later ingredient's list reads the board the earlier ones left
    /// (#68's threshold after Reno's heal, #22's meal after #100's exile), which a list built all at
    /// once cannot. `prompts::apply_resumable` runs the part it builds as a nested list, so a prompt
    /// inside it pauses the part and everything after it, and the pause records where it stood
    /// (`work::PausedStep.part`) — so the part is built again on resume, and only the part the pause
    /// stood in. `memo` is what the first build must hand every rebuild so the part is the same one:
    /// #95's roll, which must not be rolled again (R87). A caller that only calls `apply` gets the
    /// part built and applied in one go, as `resolve::lazy_part` writes it.
    pub expand: Option<EffectExpand>,
}

impl Effect {
    /// TS `{ kind, apply(ctx) { … } }`.
    pub fn new(kind: &'static str, apply: impl Fn(&mut EffectContext<'_>) + Send + Sync + 'static) -> Effect {
        Effect {
            kind,
            apply: Arc::new(apply),
            expand: None,
        }
    }

    /// TS `{ kind, expand, apply }`: an effect that is a lazily built part.
    pub fn with_expand(
        kind: &'static str,
        apply: impl Fn(&mut EffectContext<'_>) + Send + Sync + 'static,
        expand: impl Fn(&mut EffectContext<'_>, &Memo) -> EffectPart + Send + Sync + 'static,
    ) -> Effect {
        Effect {
            kind,
            apply: Arc::new(apply),
            expand: Some(Arc::new(expand)),
        }
    }
}

impl std::fmt::Debug for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Effect")
            .field("kind", &self.kind)
            .field("expand", &self.expand.is_some())
            .finish()
    }
}

/// What a part of a composed list builds (`Effect.expand`): its effects, and what a rebuild reads.
#[derive(Clone, Debug, Default)]
pub struct EffectPart {
    pub effects: Vec<Effect>,
    pub memo: Memo,
}

/// TS `Hook = (ctx) => Effect[]`.
pub type Hook = Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync>;

/// Builds a `Hook` from a closure: `hook(|ctx| vec![…])` (SURFACE §6.6, §7.2).
pub fn hook(f: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static) -> Hook {
    Arc::new(f)
}

/// `TriggerDef.when`: R99's condition over the context and the event. `&mut`: Classic+ #74's
/// predicate writes its own card while it declines (part 8.3).
pub type TriggerWhen = Arc<dyn Fn(&mut EffectContext<'_>, &GameEvent) -> bool + Send + Sync>;

/// `TriggerDef.run`: reads the event and the state; returns the effects to queue, or none.
pub type TriggerRun = Arc<dyn Fn(&mut EffectContext<'_>, &GameEvent) -> Vec<Effect> + Send + Sync>;

/// A trigger a card registers while it is in a given zone (§10.3). TS's `ctx & { event }` is the
/// context and the event as two arguments.
#[derive(Clone)]
pub struct TriggerDef {
    pub id: String,
    /// Which events wake it.
    pub on: Vec<GameEventType>,
    /// R99: the trigger's condition, kept out of `run` so a trap can decline an event without being
    /// spent — R61 makes an empty effect list mean "fired and did nothing". Absent, the `on` match
    /// alone arms it, so a trap with no predicate answers every event it names on either side.
    pub when: Option<TriggerWhen>,
    /// Reads the event and the state; returns the effects to queue, or none.
    pub run: TriggerRun,
}

impl TriggerDef {
    /// `{ id, on, run }`.
    pub fn new(
        id: impl Into<String>,
        on: &[GameEventType],
        run: impl Fn(&mut EffectContext<'_>, &GameEvent) -> Vec<Effect> + Send + Sync + 'static,
    ) -> TriggerDef {
        TriggerDef {
            id: id.into(),
            on: on.to_vec(),
            when: None,
            run: Arc::new(run),
        }
    }

    /// The same trigger with a `when`.
    pub fn with_when(
        mut self,
        when: impl Fn(&mut EffectContext<'_>, &GameEvent) -> bool + Send + Sync + 'static,
    ) -> TriggerDef {
        self.when = Some(Arc::new(when));
        self
    }
}

impl std::fmt::Debug for TriggerDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TriggerDef")
            .field("id", &self.id)
            .field("on", &self.on)
            .finish()
    }
}

/// `{ state; self; radiant }`: the argument of every pure-read hook TS gave that object (`aura`,
/// `setStat`, `costAura`, `graveyardPlay`, `targetingDiscards`, `recordsPlayAs`, `drawLimit`,
/// `heroGuard`, `conditionalKeywords`, `plagueMultiplier`, `tributeWhen`, `ActivationDecl.has`), and of
/// the two the Meditative set adds, `discardGuard` (R800) and `echoX` (R802).
#[derive(Clone, Copy)]
pub struct HookArgs<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    pub radiant: bool,
}

/// Builds any `HookArgs` hook whose answer borrows nothing (`read_hook(|a| a.self_.damage > 0)`).
#[allow(clippy::type_complexity)]
pub fn read_hook<R: 'static>(
    f: impl for<'a> Fn(HookArgs<'a>) -> R + Send + Sync + 'static,
) -> Arc<dyn for<'a> Fn(HookArgs<'a>) -> R + Send + Sync> {
    Arc::new(f)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatMod {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_health: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<Keyword>>,
    /// §10.4 layer 5: an aura that SETS attack to a value (Classic #88 Siphon Squad's Radiant "Enemy Units
    /// have 0 Attack"), applied after every other layer; with several, the last in aura order holds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_attack: Option<i32>,
}

/// One entry an aura returns: which units it touches, and what it gives them. `applies` may borrow
/// the hook's arguments (`move |unit| unit.controller == a.self_.controller`).
pub struct AuraEntry<'a> {
    /// Which units the aura touches.
    pub applies: Box<dyn Fn(&CardInstance) -> bool + 'a>,
    pub mod_: StatMod,
}

/// An aura contributes stat and keyword layers while its card is in play (§10.4 layer 5).
pub type AuraHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<AuraEntry<'a>> + Send + Sync>;

/// Builds an `AuraHook`.
pub fn aura_hook(f: impl for<'a> Fn(HookArgs<'a>) -> Vec<AuraEntry<'a>> + Send + Sync + 'static) -> AuraHook {
    Arc::new(f)
}

/// MD-D4, R1120: what one combat-only attack modifier adds to an attacker's strike in the combat it
/// started against a Unit — attack on top of the layered value, and Poisonous on those hits (§4.4
/// step 7). Never a layer: the shown attack and §4.2 step 1's "attack above 0" do not see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct AttackMod {
    pub attack: i32,
    pub poisonous: bool,
}

/// `Script.attack_mods`' argument: `{ state; self; radiant }` with the combat's attacker and
/// defender. A PURE READ, like `aura`'s: no writes, no rng.
#[derive(Clone, Copy)]
pub struct AttackModArgs<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    pub radiant: bool,
    pub attacker: &'a CardInstance,
    pub defender: &'a CardInstance,
}

/// MD-D4, R1120: a combat-only attack modifier — what this card adds to an attacker's strike while
/// that attacker attacks a Unit. Read only inside a combat against a Unit (declared or forced).
pub type AttackModHook = Arc<dyn for<'a> Fn(AttackModArgs<'a>) -> AttackMod + Send + Sync>;

/// Builds an `AttackModHook`.
pub fn attack_mod_hook(
    f: impl for<'a> Fn(AttackModArgs<'a>) -> AttackMod + Send + Sync + 'static,
) -> AttackModHook {
    Arc::new(f)
}

/// `boolean | number`: a static flag a fused card may carry several times (R102).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum FlagOrCount {
    Flag(bool),
    Count(i32),
}

impl FlagOrCount {
    /// How many times the flag is granted: `true` is once, `false` none, a number itself.
    pub fn count(self) -> i32 {
        match self {
            FlagOrCount::Flag(true) => 1,
            FlagOrCount::Flag(false) => 0,
            FlagOrCount::Count(n) => n,
        }
    }
}

/// R1160 (Meditative #84 Volatility, ME-TUNEMULT): how many times as effective Buffs
/// (`upgrade`) and Nerfs (`degrade`) are on this card. Each direction names its own multiplier;
/// a direction no ingredient names is plain.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TuneMultiplier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrade: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degrade: Option<i32>,
}

/// A card's static flags. Data, so a card may write them as the TS object literal
/// (`json_as(json!({ "castOnDraw": true }))`). `deftDuelist` (legacy, read nowhere) is not ported
/// (SURFACE §7.2).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StaticFlags {
    /// Plays itself on draw, then draws again (§6.2, R58).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_on_draw: Option<bool>,
    /// Starts in the opening hand instead of a draw (§6.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quickdraw: Option<bool>,
    /// Replaces an empty-library draw with a Rush Token card (#75).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub infinite_reserves: Option<bool>,
    /// Cannot switch to Defense Position (#65.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub never_defense: Option<bool>,
    /// R30: this card's own Echo, so its play resolves this many extra times.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<i32>,
    /// R30, R209: the Echo this permanent's rider gives the next Spell, read off its face now (#79).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo_grant: Option<i32>,
    /// #38: while on the field, its controller's cards gain "Combo X: deal X damage to the enemy hero"
    /// (radiant 2X, dealt as one hit, R281). A number is how many times the card grants it: a card
    /// fused from two Quickstrikers carries both texts (R102), and `true` is once. The multiple of X
    /// each grant deals is not the flag's: it is `QUICKSTRIKER_COMBO_MULTIPLE` in `config.rs`, picked by
    /// the granting instance's own face, as #84's Armor is (`HERO_ARMOR`), so a base and a Radiant
    /// Quickstriker side by side deal X and then 2X.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quickstriker: Option<FlagOrCount>,
    /// #64 Gifted Program: while on the field, the first card costing this much or less its controller
    /// plays each turn becomes Radiant as it is played (§10.5 step 3, R56, R213).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gifted_program: Option<i32>,
    /// Tribute cost in units, Sheep Tokens counting 2 (§6.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute: Option<i32>,
    /// §3.2, §7: what this unit counts toward a Tribute while it is on the field — the Sheep Token's
    /// "worth 2 Tributes" (3 on its radiant face). Absent is 1. It is the face's text, so a Vanilla
    /// unit is worth 1, and a fused card takes the larger of its ingredients' (R102).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute_worth: Option<i32>,
    /// R101: only a card that says so may pay its Tribute with the opponent's units (§8 #55).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute_enemies: Option<bool>,
    /// R360: a play whose Tribute took any of the opponent's units summons this card for the opponent
    /// (§8 #55's base face: "If opposing Units are used, summon for your opponent").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_tribute_hands_over: Option<bool>,
    /// Anti-oneshot Armor: caps each hit on this player's hero at ANTI_ONESHOT_CAP (§4.4 step 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anti_oneshot: Option<bool>,
    /// #84 Going Long: while this card is in a backrow it gives that hero Armor from `HERO_ARMOR`,
    /// picked by the instance's own `radiant` and `embiggened`, and §4.4 step 2 subtracts it. R124:
    /// several sources add up, so this is a layer and not a value — a card carrying its own numbers
    /// would put rules constants in a card file, which BUILD §2 keeps in `config.rs`. A card fused from
    /// two carries both grants (R102), so a fused face may hold a count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero_armor: Option<FlagOrCount>,
    /// R429 (Core patches, v0.2.0): §10.5 step 4 counts each play of this card on its instance
    /// (`CardInstance.timesPlayed`, `times_played.rs`) — #31 KY's Math Equation's "times played".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counts_plays: Option<bool>,
    /// R429, R766 (issues #557, #572): this Spell's own end-of-turn return (§5.1, R155) gives back the
    /// climb its earlier returns gave it, and no other change to its price. R766 still takes the price
    /// off it in the graveyard it lands in, so §10.5 step 7 notes the climb it carried into its play
    /// (`resolve::note_return_price`) for its return to read (`query::return_price_of`) — #31 KY's Math
    /// Equation's climb, (2), (3), (4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_keeps_price: Option<bool>,
    // ---- v0.2.0 static flags, by workstream: field (B3.1, E20, E21, E22) ----
    /// B5 E21, R446: "A Unit may be played on top of this". While this backrow card acts in its zone, a
    /// Unit its controller plays may name that zone and stand on it: a Unit for every rule that can
    /// neither attack nor be attacked, with this card still acting beneath it (`zones::carrier_zones_for`,
    /// `zones::is_carried`). No card prints it since patch v0.2.3; `fusesCarried` builds on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrier: Option<bool>,
    /// R653: "The first Unit you stack onto this is fused into it" (Classic+ #33 Ivory Tower). A carrier,
    /// as `carrier` is, that takes one Unit a stay on the field (`zones::stacked_onto` names it) and none
    /// while it is Immutable, whose text could take no Unit in (R23). The fusion is the card's own, once
    /// that Unit's play has resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuses_carried: Option<bool>,
    // ---- v0.2.0 static flags, by workstream: play pipeline (E1, E2, E5 targeting, E11, E12, E15) ----
    /// Classic+ #68 Organic Produce: while on the field, every card its controller plays carrying one of
    /// these tags becomes Radiant as it is played (§10.5 step 3) — R213's Gifted Program rule by tag, on
    /// every such play rather than the first cheap one (`play_choices::play_made_radiant`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radiant_plays_tagged: Option<Vec<Tag>>,
    /// B5 E14, Classic #57 Echo, R399, R545–R547: "This has the text of the last Spell either player
    /// played". The card's text is the copied Spell's face (`subsystems::copied_text`): its declared
    /// choices, its resolution and prompt continuations, its Echo X and Cast on draw, its `preview` and
    /// `conditionMet`. The card keeps its own name, cost, type and tags, and its own other flags (Echo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copies_last_spell: Option<bool>,
    // ---- v0.2.0 static flags, by workstream: damage and combat (E5, E6, E8, E35) ----
    /// B5 E35: no attack may be made on this unit, declared or forced (Classic+ #51 J15 Fighter).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cant_be_attacked: Option<bool>,
    /// B5 E35: only a unit standing in this unit's lane may attack it, declared or forced (Classic+
    /// #19.1 Top Loser); a Taunt on it binds only the attackers that may reach it (§4.2 step 3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attacked_only_from_lane: Option<bool>,
    /// B5 E35: this unit neither attacks nor is attacked, declared or forced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cant_attack_or_be_attacked: Option<bool>,
    /// B5 E35: "This can't go Berserk" (Classic+ #19.5's Radiant face).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub never_berserk: Option<bool>,
    /// B5 E8: while this card acts on the field — face-up, when it is a Trap or Field Trap — a heal of X
    /// on one of its controller's enemies deals X Pierce damage to it instead, from this card (Classic+
    /// #22 Blood Moon's Radiant Field Trap, "From now on").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heal_to_damage: Option<bool>,
    /// Meditative #86 Mayor Medinamogger (ME-RANDOMTARGETS, R1200): while this card acts on the
    /// field, both players' declared targets, `target` prompts and attack targets are drawn at
    /// random from the legal ones.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub random_targets: Option<bool>,
    /// Meditative #91 Windfast (ME-ATTACKSUMMON, R1202): when this would attack, a Unit summoned
    /// from its controller's hand makes that attack instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_from_hand: Option<bool>,
    /// Meditative #91 Windfast's base face (ME-ATTACKSUMMON, R1202): the summoned substitute is
    /// bounced to its controller's hand after its combat if it is still on the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounce_attacker: Option<bool>,
    /// Meditative #91.1 Windfurious Prime (ME-ATTACKSUMMON, R1203): when this attacks, summoned
    /// joiners attack its target first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_joiners: Option<bool>,
    /// MD-D28, R1125: while this card acts on the field, each `play` the opponent makes is judged
    /// against their other playable cards (`subsystems::pareto`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judges_plays: Option<bool>,
    /// MD-D29, R1127: while this card acts on the field, the opponent's emotes reach the engine as
    /// `Emote` actions (`GameEvent::Emoted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hears_emotes: Option<bool>,
    /// MD-D31, R1124: damage this card deals to a Unit marks it, and the next state check exiles it
    /// ahead of deaths (§4.4 step 7, §4.5 step 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exiles_on_damage: Option<bool>,
    /// MD-F12, R1281: its controller may play any Unit onto its zone as if it had Stack.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_base: Option<bool>,
    /// MD-F12, R1281: how many times the Unit is Upgraded as it lands, read through `buffs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack_base_buffs: Option<i32>,
    /// MD-F13, R1282: may tribute any permanent costing this or less on either side, read through `cheap`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute_cheap: Option<i32>,
    /// MD-F14, R1283: multiplier for attacks across its lane in combat, read through `multiplier`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_multiplier: Option<i32>,
    /// R1160 (Meditative #84 Volatility, ME-TUNEMULT): how many times as effective Buffs and
    /// Nerfs are on this card, read off its running face in every zone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tune_multiplier: Option<TuneMultiplier>,
    /// R1223: while this card acts on its controller's field, they may spend mana they don't have,
    /// owing up to this much at once (Meditative #89 Jlarna's credit line).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_line: Option<i32>,
    /// R1223: how many instalments a debt on this credit line is split into (Jlarna: 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_instalments: Option<i32>,
    /// R1225: this face carries the end-of-turn Tribute for an unused credit line (Jlarna's base
    /// face). The view and the AI read it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_lapses: Option<bool>,
    /// MD-B2, R941: while this card acts on the field — face-up, top of its pile — no player generates
    /// mana naturally: each start-of-turn refresh sets max mana as usual but current mana to the
    /// next-turn rider only (Meditative #26).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_natural_mana: Option<bool>,
    // ---- Meditative ----
    /// R981–R984, Meditative #40 Feng Shui: while this card acts on the field, it judges every face-up
    /// play at §10.5 step 3 by its element against its player's last (`subsystems::feng_shui`). On its
    /// Radiant face it rewards only its controller's plays and punishes only the opponent's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feng_shui: Option<bool>,
    /// R987, Meditative #40 Feng Shui: "You have Luck X". While this card acts on the field, every roll
    /// with a best that its controller's cards make rolls X more times (`query::luck_of`); X is the
    /// card's declared number `luck` where it declares it, else this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luck: Option<i32>,
}

string_union! {
    /// R195, R280: where `view_for` is asking about a card.
    pub enum ConditionZone {
        Hand = "hand",
        Field = "field",
    }
}

/// R195, R280, §10.9: the argument of the two read-only hooks `view_for` asks, the Hearthstone "yellow
/// glow" predicate (`conditionMet`) and the number a formula comes to now (`preview`). A hook is a
/// PURE READ — it never writes, never draws from `rng` (the context carries none), never returns
/// effects — and must agree with what the card's own resolution would do if it resolved now.
#[derive(Clone, Copy)]
pub struct ConditionContext<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    /// The card's controller. R195 only ever asks about the viewer's own cards, so there this is the
    /// viewer; R280 asks about any card the viewer may read, the other seat's public ones included, so
    /// there it is the card's controller and never the viewer as such.
    pub controller: PlayerId,
    /// Whether the Radiant face is the one running (§5.2).
    pub radiant: bool,
    /// "hand": as if played now. "field": as the card on the field reads it now.
    pub zone: ConditionZone,
    /// `state.active === controller`, so a card file never reads `state.active` itself.
    pub your_turn: bool,
}

pub type ConditionHook = Arc<dyn for<'a> Fn(ConditionContext<'a>) -> bool + Send + Sync>;

/// R280, §10.9: the labelled numbers a card's formula comes to now — #31's Fib(cost+1), #70's sum
/// over missing health and exile. Each `label` is the formula as the running face prints it, an
/// exact substring of that face's catalog text (the client prints the value in braces right after
/// the label's first occurrence, §10.10), and each `value` what it would come to if the card resolved
/// now. The hook is asked with the same context `conditionMet` is (the running face, the card's
/// controller, the zone, `yourTurn`) and is a PURE READ, built on the same function the card's own
/// resolution computes the number with, so the two cannot disagree. It reads only what the card's
/// controller may read (§9.1) — a hero's health, a pile's size, the plays this turn, its own cost and
/// counters — never a library's contents or order or a hidden hand, because `view_for` shows the
/// result to every viewer who may read the card, the other seat included (`preview.rs`). An empty
/// list is no preview at all.
pub type PreviewHook = Arc<dyn for<'a> Fn(ConditionContext<'a>) -> Vec<PreviewValue> + Send + Sync>;

/// Builds a `ConditionHook` (`R = bool`) or a `PreviewHook` (`R = Vec<PreviewValue>`).
#[allow(clippy::type_complexity)]
pub fn condition_hook<R: 'static>(
    f: impl for<'a> Fn(ConditionContext<'a>) -> R + Send + Sync + 'static,
) -> Arc<dyn for<'a> Fn(ConditionContext<'a>) -> R + Send + Sync> {
    Arc::new(f)
}

/// `Script.cost`'s argument: `{ state, instance }`.
#[derive(Clone, Copy)]
pub struct CostArgs<'a> {
    pub state: &'a GameState,
    pub instance: &'a CardInstance,
}

/// Ceaseless Void's computed cost (R55).
pub type CostHook = Arc<dyn for<'a> Fn(CostArgs<'a>) -> i32 + Send + Sync>;

/// Builds a `CostHook`.
pub fn cost_hook(f: impl for<'a> Fn(CostArgs<'a>) -> i32 + Send + Sync + 'static) -> CostHook {
    Arc::new(f)
}

/// `Script.setStat`'s answer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetStat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_health: Option<i32>,
}

/// §10.4 layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39).
pub type SetStatHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> SetStat + Send + Sync>;

string_union! {
    /// Whose cards a cost aura reaches, seen from the aura card's controller (TS `costRules.ts`).
    pub enum CostAuraWhose {
        Yours = "yours",
        Opponents = "opponents",
        All = "all",
    }
}

/// E15: what a card on the field lays on prices while it acts there (`Script.costAura`). A rule with
/// `ban: true` is "can't play" — it prices nothing and refuses a play of a card it reaches whose price
/// is `minCost` or more (Classic #68 Radiant), and never a cast (R70). (TS `costRules.ts`:
/// `CostRule & { whose; ban? }`, the rule's fields flattened beside these.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct CostAura {
    #[serde(flatten)]
    pub rule: CostRule,
    pub whose: CostAuraWhose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ban: Option<bool>,
}

/// The argument of `Script.costAura`, as `Script.aura`'s is: a pure read of the board (TS `costRules.ts`).
pub type CostAuraArgs<'a> = HookArgs<'a>;

/// E15, R455: the price rules this card lays on cards while it acts (`cost_rules.rs`).
pub type CostAuraHook = Arc<dyn for<'a> Fn(CostAuraArgs<'a>) -> Vec<CostAura> + Send + Sync>;

/// E11, R454: what one card on the field lets its controller play from their own graveyard (TS
/// `graveyardPlay.ts`).
///
/// - `units`: Units only (Classic #74); absent, every card type (Classic #28, In Too Deep's reward L).
/// - `minPrice`: the least price a play under it may have, as it would be paid — after every discount
///   (Classic #28 Radiant: (1), which stops a (0) loop).
/// - `plague`: plays under it pay with the Plague Counters on the granting card (Classic #74): each token
///   pays PLAGUE_TOKEN_MANA, at least MIN_PLAGUE_PAYMENT of them, at most the tokens there and the
///   price, and the rest in mana. Such a play must spend tokens; a permission without `plague` is paid
///   in mana alone.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraveyardPlayPermission {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_price: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plague: Option<bool>,
}

/// E11, R454: the permissions this card gives its controller to play cards from their graveyard.
pub type GraveyardPlayHook =
    Arc<dyn for<'a> Fn(CostAuraArgs<'a>) -> Vec<GraveyardPlayPermission> + Send + Sync>;

/// ME-ALTPLAY, R1040, R1044: the permission this card gives its controller to play cards face-down
/// as Traps — Units under Knowledge Breaker's Aura, Spells under Paranoia's. `echo` is the Echo a
/// Radiant Paranoia adds to a set Spell (R1045), once per play.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct FaceDownPlayPermission {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spells: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub echo: Option<i32>,
}

/// ME-ALTPLAY, R1040, R1044: the face-down play permissions this card grants while it acts.
pub type FaceDownPlayHook =
    Arc<dyn for<'a> Fn(CostAuraArgs<'a>) -> Vec<FaceDownPlayPermission> + Send + Sync>;

/// B5 E5: "to target this with anything but an attack, a player must also discard N cards".
pub type TargetingDiscardsHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

/// B5 E4, R451: what this card's play records as the card played.
pub type RecordsPlayAsHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Option<PlayRecord> + Send + Sync>;

/// One guard `Script.heroGuard` returns: a per-hit cap and a divisor applied after Armor.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct HeroGuard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divisor: Option<i32>,
}

/// B5 E6: what this card does to hits on its controller's hero while it acts on the field.
pub type HeroGuardHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<HeroGuard> + Send + Sync>;

/// B5 E35: keywords the card has only while a condition holds.
pub type ConditionalKeywordsHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<Keyword> + Send + Sync>;

/// B5 E19, R471: what each Plague Counter placement onto this card is multiplied by.
pub type PlagueMultiplierHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

/// R802: a computed Echo X, the repeats the card's Echo makes as it is played.
pub type EchoXHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

/// ME-TRIG, R820: how many extra times this card makes its controller's hooks of one kind run.
pub type TriggerExtraHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

/// Classic #88 Siphon Squad, R403: "When …, Tribute this".
pub type TributeWhenHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> bool + Send + Sync>;

/// `Script.wouldCounter`'s argument.
#[derive(Clone, Copy)]
pub struct WouldCounterArgs<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    pub controller: PlayerId,
    pub player: PlayerId,
    pub cost_paid: i32,
}

/// Classic #87 Plague Chalice, R667: whether the card's own trigger would counter a play.
pub type WouldCounterHook = Arc<dyn for<'a> Fn(WouldCounterArgs<'a>) -> bool + Send + Sync>;

/// Builds a `WouldCounterHook`.
pub fn would_counter_hook(
    f: impl for<'a> Fn(WouldCounterArgs<'a>) -> bool + Send + Sync + 'static,
) -> WouldCounterHook {
    Arc::new(f)
}

/// `ActivationDecl.has`: whether the card has this ability now.
pub type HasHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> bool + Send + Sync>;

#[derive(Clone, Default)]
pub struct Script {
    /// Ceaseless Void's computed cost (R55); everything else uses the printed cost.
    pub cost: Option<CostHook>,
    pub cry: Option<Hook>,
    pub death: Option<Hook>,
    /// After the mulligan, before turn 1: only Heroic Power uses it (§6.2, R43).
    pub start_of_game: Option<Hook>,
    /// MD-B18, R925: "When this enters your hand" (Meditative #37) — run by `draw::run_arrival_hooks`
    /// on every hand arrival, after R151's start-of-game clause.
    pub enters_hand: Option<Hook>,
    /// Named continuations a prompt answer re-enters (§10.6, R81). Empty: none.
    pub resume: IndexMap<&'static str, Hook>,
    /// A delayed effect this card scheduled, resolved at its R62 point.
    pub delayed: Option<Hook>,
    /// §10.4 layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39).
    pub set_stat: Option<SetStatHook>,
    pub start_of_turn: Option<Hook>,
    pub end_of_turn: Option<Hook>,
    /// ME-GRANT (MD-D13): the Death abilities this card grants, by key — `meditative-058#bookDeath`
    /// grants face `bookDeath`. A grant names a hook here, so state stays plain data (a closure never
    /// enters it, as `fuse.rs` explains). A fused card merges its ingredients' maps (R102): the keys
    /// already name their definition, so they never collide.
    pub grants: IndexMap<&'static str, Hook>,
    pub aura: Option<AuraHook>,
    pub triggers: Vec<TriggerDef>,
    pub on_play_hook: Option<Hook>,
    pub hand_triggers: Vec<TriggerDef>,
    pub static_flags: Option<StaticFlags>,
    /// The play-time choices this card declares (R81).
    pub targets: Vec<TargetDecl>,
    pub modes: Vec<ModeDecl>,
    /// R865: the card's declared modes are kept secret — filed as a secret record, not carried on the
    /// play's events, and blanked from an in-flight play the other seat reads.
    pub secret_modes: bool,
    /// R195: the condition `view_for` surfaces as `conditionActive` (§10.8).
    pub condition_met: Option<ConditionHook>,
    /// R280: the numbers the card's formula comes to now, which `view_for` surfaces as `preview` (§10.8).
    pub preview: Option<PreviewHook>,
    // ---- Patch v0.2.0 ----
    /// B3.2, R384: the card's Activate abilities ("Activate:", "Activate N:", "Activate ♾️:"), used by
    /// the `activate` action while the card acts on the field (`subsystems::activate`).
    pub activations: Vec<ActivationDecl>,
    /// §10.6: the named predicates a declaration's `TargetFilter.check` points at, for a filter no data
    /// field can say (Classic #32's lane rule, #48's lines of code). A pure read, like `conditionMet`.
    pub target_checks: IndexMap<&'static str, TargetCheck>,
    // play pipeline B (E11, E15). Both are pure reads, like `aura`, asked of a card acting on the field
    // (the top of its pile, or its backrow card), and both return lists so a fused card carries each
    // ingredient's (R102). A hook rather than a static flag, so a Degrade or Upgrade of the card's
    // declared numbers moves what it grants (B3.4), and so a grant can hang on the card's state (Classic
    // #90 In Too Deep's reward L).
    /// E15, R455: the price rules this card lays on cards while it acts (`cost_rules.rs`).
    pub cost_aura: Option<CostAuraHook>,
    /// E11, R454: the permissions this card gives its controller to play cards from their graveyard.
    pub graveyard_play: Option<GraveyardPlayHook>,
    /// ME-ALTPLAY, R1040, R1044: the permissions this card gives its controller to play cards
    /// face-down as Traps.
    pub face_down_play: Option<FaceDownPlayHook>,
    /// B5 E5, Classic #89 Paul Allen's Ghost: "to target this with anything but an attack, a player must
    /// also discard N cards" — N now, read while the card is on the field (a pure read, so a Degrade or
    /// Upgrade of the declared number reaches it through `param`). 0 or absent is no cost. The discards
    /// are random at pay time (R682), so no action carries them: a play (§10.5 step 2) or an activation
    /// declaring it pays them as it pays its price, and a prompt answer naming it pays them before it
    /// goes on (`targeting.rs`, `targeting_point.rs`).
    pub targeting_discards: Option<TargetingDiscardsHook>,
    /// B5 E4, R451: what this card's play records as the card played — the last Spell played (Classic
    /// #57) and its player's last face-up play. Absent records the card itself; Classic #57 Echo
    /// returns the Spell it copied, and `None` records nothing (an Echo with nothing to copy).
    pub records_play_as: Option<RecordsPlayAsHook>,
    // ---- v0.2.0 script hooks, by workstream: activate and turn (E27, E28) ----
    /// B5 E3, R457: the draw limits this card sets while it acts on the field ("Your opponent can't draw
    /// more than 1 card each turn": Classic #4, #49). A pure read like an aura, so a card computes its
    /// number (a declared, tunable one included) from its own instance. The lowest limit on a player holds.
    pub draw_limit: Option<DrawLimitHook>,
    /// R800: the players this card guards from an effect's discard while it acts on the field ("You
    /// can't be forced to discard cards during your opponent's turn": Meditative #1), relative to its
    /// controller, as a draw limit's are. A pure read like `draw_limit`; a guarded player's hand loses
    /// no card to an effect's discard while it is not their turn (`draw::discard_guarded`).
    pub discard_guard: Option<DiscardGuardHook>,
    // ---- v0.2.0 script hooks, by workstream: damage and combat (E5, E6, E8, E9, E35) ----
    /// B5 E5, R460: the events this card changes before they happen — a lethal hit on its hero, a heal
    /// on an enemy, its units' deaths, a card's way to a graveyard, a friendly unit targeted by the
    /// opponent (`replacements.rs`). Declared as data and decided synchronously, never an effect list.
    /// The "targeted" entry (`TargetedReplacement`) covers every targeting: attack, play, cast,
    /// activation and prompt pick.
    pub replacements: Vec<ReplacementDef>,
    /// B5 E6: what this card does to hits on its controller's hero while it acts on the field — a
    /// per-hit cap (the lowest of every cap wins, Classic+ #11 Anime Armor) and a divisor applied after
    /// Armor (several multiply, rounded up once, Classic #75 Argusland). A PURE READ, like `aura`; a
    /// list, so a fused card carries each ingredient's.
    pub hero_guard: Option<HeroGuardHook>,
    /// MD-D4, R1120: the combat-only attack modifiers this card lays while it acts on the field. A
    /// PURE READ, like `aura`; a fused card carries each ingredient's (`subsystems::fuse`).
    pub attack_mods: Option<AttackModHook>,
    /// B5 E35: keywords the card has only while a condition holds (Classic #69 Plague Charger's First
    /// Strike "while it has a Plague Counter"), read in the layers with its printed keywords (§10.4), so a
    /// Vanilla takes them. A PURE READ of instance data: like an aura's `applies`, it must never call
    /// back into `unit_view`, or the layers would recurse.
    pub conditional_keywords: Option<ConditionalKeywordsHook>,
    /// "After this attacks" (Classic #13 Boots on the Ground, Classic+ #73.1 Classic Golem, Core #32
    /// Prem Panther): run for the attacker once the state check that closes each of its combats has run,
    /// a declared attack's or a forced one's, also when it died there — then on the snapshot it fought
    /// with, as a Death hook reads its card (R78, R89). Not for an attack called off before it fought
    /// (R44). `ctx.data` holds the combat's facts, read with `combat::after_attack_of`: `{ targetId,
    /// destroyedIds, survived, forced }`. A whole effect list, parkable like any (R113).
    pub after_attack: Option<Hook>,
    /// "After this is attacked" (Meditative #49.3 AI Girlfriend): run for the defender once the
    /// state check that closes each combat it was the target of has run, a declared attack's or a
    /// forced one's, also when it died there — then on the snapshot it fought with, as a Death hook
    /// reads its card (R78, R89). Not for a Cleave splash (no attack on it) nor for an attack on
    /// the hero (MD-C26, R1026). `ctx.data` holds the combat's facts, read with
    /// `combat::after_attacked_of`: `{ attackerId, forced, attackerSurvived }`. A whole effect
    /// list, parkable like any (R113).
    pub after_attacked: Option<Hook>,
    // ---- v0.2.0 script hooks, by workstream: prompts and generation (E13, E19, E26) ----
    /// B5 E19, R471: "Plague Counters placed on this are doubled" (Classic #27 Pestilent Slime; tripled on
    /// its Radiant face). What each placement onto this card is multiplied by, asked of the card as it
    /// receives the placement — a pure read (a card reads its declared number here, B3.4), floored at 1.
    /// A fused card's multipliers multiply (`subsystems::fuse`).
    pub plague_multiplier: Option<PlagueMultiplierHook>,
    /// R802: a computed Echo X ("Echo X … X is 2 times your max mana": Meditative #5), read once as the
    /// card is played, when its Echo repeats are queued (§10.5 step 4), so X stays fixed while it
    /// resolves. A pure read; the larger of it and `staticFlags.echo` is the card's printed Echo, which
    /// its "Echo" tuning step and a Twinspell grant then add to (`echo::printed_echo`).
    pub echo_x: Option<EchoXHook>,
    /// B5 E26, R464: triggers this card answers while it lies in its owner's library ("While this is in
    /// your deck: …", Classic+ #37 Wardrum). A library card is hidden (§9.1), so its queue entries take
    /// no number (R177), and within a side they come after the hand's and before the graveyard's, in the
    /// order the instances were created — never by library position (`triggers::trigger_holders_of`).
    pub deck_triggers: Vec<TriggerDef>,
    /// B5 E26: triggers this card answers while it lies in a graveyard ("While this is in your
    /// graveyard: when one of your Traps reveals, return this", Classic #47). R153's other graveyard
    /// answer, the end-of-turn return, stays the `endOfTurn` hook's.
    pub graveyard_triggers: Vec<TriggerDef>,
    /// B5 E33, R404: the card's quest tree (Classic #90 In Too Deep) — its quests, what completes each and
    /// the rewards each offers, as data. `subsystems::quests` keeps the count on the instance
    /// (`memory.quest`), opens the first quest as the card enters the field and reports each completion
    /// (`questCompleted`), which the card's own trigger answers with its rewards.
    pub quests: Option<QuestBook>,
    // ---- v0.2.0 script hooks, by workstream: Classic #46–#90 (card-specific, cards-classic-b) ----
    /// Classic #88 Siphon Squad, R403: "When …, Tribute this" — a condition on the card's own text that
    /// every state check reads (§4.5), the one right after the card arrives included; while it holds, the
    /// card acting on the field (face-down too) is sacrificed (`state_check.rs`). A PURE READ, like `aura`.
    pub tribute_when: Option<TributeWhenHook>,
    /// Classic #87 Plague Chalice, R667: the static half of a counter the card's own trigger makes —
    /// whether that trigger, the card standing as it does now, would counter a play `player` makes
    /// paying `costPaid`. The trigger asks the same predicate, so the two cannot disagree, and
    /// `counter_warning.rs` reads it to warn the viewer off a hand card (`counteredOnPlay`, §10.8). A
    /// PURE READ, like `tributeWhen`.
    pub would_counter: Option<WouldCounterHook>,
    /// Classic #62 Living Bomb, R400: "At the start of your opponent's turn" — the `startOfTurn` hook of
    /// the other side, queued right after the active player's at R62's start-of-turn trigger point, so
    /// R68's order (the active player's cards, then the opponent's) holds.
    pub start_of_opponent_turn: Option<Hook>,
    // ---- Meditative (R1420) ----
    /// ME-TRIG, R820, R821 (Meditative #9 Joint Filing): "Your Start of turn and End of turn effects
    /// trigger N additional times" — N now, asked of the card acting on its controller's field as their
    /// start-of-turn or end-of-turn hooks are queued (`multipliers::extra_runs`). A PURE READ, like
    /// `aura`, so a Nerf or Buff of the declared number moves it; several cards do not add, the highest
    /// holds.
    pub turn_hook_extra: Option<TriggerExtraHook>,
    /// ME-TRIG, R822, R823 (Meditative #10 Double Counting): "Your Cry and Death effects trigger N
    /// additional times" — N now, asked as a permanent's Cry runs for its controller and as the cards a
    /// state check collects have left the field. A PURE READ; the highest holds.
    pub cry_death_extra: Option<TriggerExtraHook>,
}

impl Script {
    /// `script[name]` for the effect-list hooks TS named by string (`work.scriptStepFor`,
    /// `triggers.TRIGGER_HOOKS`, `Resume.hook`): "cry", "death", "startOfGame", "entersHand",
    /// "delayed", "startOfTurn", "endOfTurn", "onPlayHook", "afterAttack", "startOfOpponentTurn".
    /// `None` for any other name or an absent hook.
    pub fn hook_named(&self, name: &str) -> Option<&Hook> {
        match name {
            "cry" => self.cry.as_ref(),
            "death" => self.death.as_ref(),
            "startOfGame" => self.start_of_game.as_ref(),
            "entersHand" => self.enters_hand.as_ref(),
            "delayed" => self.delayed.as_ref(),
            "startOfTurn" => self.start_of_turn.as_ref(),
            "endOfTurn" => self.end_of_turn.as_ref(),
            "onPlayHook" => self.on_play_hook.as_ref(),
            "afterAttack" => self.after_attack.as_ref(),
            "afterAttacked" => self.after_attacked.as_ref(),
            "startOfOpponentTurn" => self.start_of_opponent_turn.as_ref(),
            _ => None,
        }
    }

    /// `script.staticFlags ?? {}`.
    pub fn flags(&self) -> StaticFlags {
        self.static_flags.clone().unwrap_or_default()
    }
}

/// `ActivationDecl.uses`: "Activate" (1), "Activate N" (N) or "Activate ♾️" ("unlimited").
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ActivationUses {
    Count(i32),
    #[serde(with = "unlimited")]
    Unlimited,
}

mod unlimited {
    //! `"unlimited"` as a unit variant inside an untagged enum.
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("unlimited")
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(), D::Error> {
        let text = String::deserialize(deserializer)?;
        if text == "unlimited" {
            Ok(())
        } else {
            Err(D::Error::custom("expected \"unlimited\""))
        }
    }
}

/// `ActivationDecl.cost`: what the ability pays as it is activated.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActivationCost {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discard_random: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute_self: Option<bool>,
    /// A Tribute cost that may not take the card itself, even when it is a Unit (Classic #21
    /// Turtinator, which cannot Tribute itself; R683).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tribute_excludes_self: Option<bool>,
    /// MD-D9: either player may activate an ability with this cost (Meditative #54's shape, proved
    /// with the engine fixture while #54 waits for #525's Jade). The non-controller activates in
    /// their own main phase, paying from their own mana. Absent: the controller's alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub either_player: Option<bool>,
    /// The declared number (`params`) the mana price reads, so a Buffed price reaches the cost
    /// (B3.4). Skipped in serialization: data, never JSON. Absent: the price is `mana` as written.
    #[serde(skip)]
    pub mana_param: Option<&'static str>,
}

/// B3.2, R384: one Activate ability. `uses` is "Activate" (1), "Activate N" (N) or "Activate ♾️"
/// ("unlimited", bounded by `ACTIVATE_UNLIMITED_CAP`); Degrade and Upgrade move a number by the tuning
/// key "Activate" (B3.4). `cost` is what the ability pays as it is activated — mana (Heroic Power's
/// "spend (X)", which the Heroic Power patch moves here), a random discard (Classic #15), a Tribute of
/// the controller's units (Classic #21, the card itself allowed), or the card itself (Classic #84).
/// `targets` and `modes` travel in the action as a play's do (R81). `canActivate` is a pure read for a
/// condition the text sets (Classic #7: "that Spell" must exist). `run` is the effect.
///
/// R384: `cost.tribute` counts units, one each — "Tribute a Unit" is one unit, so a Sheep Token's
/// "worth 2" does not stretch it (that worth counts only toward a play's Tribute X, §6.3). The card
/// itself may be one of them when it is a Unit. `tributeSelf` is "Tribute this", which bypasses
/// Indestructible as every Sacrifice does (§6.3).
#[derive(Clone)]
pub struct ActivationDecl {
    pub id: String,
    pub label: String,
    pub uses: ActivationUses,
    pub cost: Option<ActivationCost>,
    pub targets: Vec<TargetDecl>,
    pub modes: Vec<ModeDecl>,
    pub can_activate: Option<ConditionHook>,
    /// Whether the card has this ability now, when that depends on the instance — an ability it lacks is
    /// neither listed, shown nor accepted. The Heroic Power patch declares one ability per power and
    /// has only the one it rolled (B3.2 rule 10). Absent: always.
    pub has: Option<HasHook>,
    pub run: Hook,
}

/// R384: the `Resume.hook` an ability's own effect list runs under, `activation:<id>`, so a tail a
/// prompt parks comes back to the same ability (`work::script_step_for`), as a trigger's comes back by
/// its id. Distinct from every `Script` key.
pub const ACTIVATION_HOOK_PREFIX: &str = "activation:";

pub fn activation_hook(id: &str) -> String {
    format!("{ACTIVATION_HOOK_PREFIX}{id}")
}

/// R384, R102: a face's abilities with ids made unique — a card fused from two Activate cards has both
/// abilities, and the second of two that share an id is `<id>#2` — so the `activate` action, the view
/// and a resumed tail all name the same one. Order is the face's own.
pub fn activation_decls(script: &Script) -> Vec<ActivationDecl> {
    let mut seen: IndexMap<String, i32> = IndexMap::new();
    script
        .activations
        .iter()
        .map(|decl| {
            let count = seen.get(&decl.id).copied().unwrap_or(0) + 1;
            seen.insert(decl.id.clone(), count);
            if count == 1 {
                decl.clone()
            } else {
                ActivationDecl {
                    id: format!("{}#{count}", decl.id),
                    ..decl.clone()
                }
            }
        })
        .collect()
}

string_union! {
    /// `DrawLimit.player`, relative to the card's controller.
    pub enum DrawLimitPlayer {
        SelfSide = "self",
        Enemy = "enemy",
        Both = "both",
    }
}

/// B5 E3, R457: one draw limit a card sets. `player` is relative to the card's controller: "enemy" is
/// the opponent (Classic #4), "both" every player (Classic #49); `count` is how many draws that player
/// may make each turn, whoever's turn it is.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct DrawLimit {
    pub player: DrawLimitPlayer,
    pub count: i32,
}

pub type DrawLimitHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<DrawLimit> + Send + Sync>;

/// R800: the players a card guards from an effect's discard, each relative to its controller.
pub type DiscardGuardHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<DrawLimitPlayer> + Send + Sync>;

/// §10.6: a card-specific target predicate's argument (`TargetFilter.check`). `candidate` is the card a
/// declaration would offer (`None` for a hero or a zone), `self_` the card declaring it, `player` the
/// chooser.
#[derive(Clone, Copy)]
pub struct TargetCheckArgs<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    pub player: PlayerId,
    pub radiant: bool,
    pub candidate: Option<&'a CardInstance>,
    pub selection: &'a Selection,
}

/// §10.6: a card-specific target predicate (`TargetFilter.check`). A pure read: no writes, no rng.
pub type TargetCheck = Arc<dyn for<'a> Fn(TargetCheckArgs<'a>) -> bool + Send + Sync>;

/// Builds a `TargetCheck`.
pub fn target_check(f: impl for<'a> Fn(TargetCheckArgs<'a>) -> bool + Send + Sync + 'static) -> TargetCheck {
    Arc::new(f)
}

// ---------------------------------------------------------------------------------------------
// B5 E5, E9 (damage and combat): what `Script.replacements` holds (TS `replacements.ts`, which
// `script.ts` re-exported beside `Script`) — above all the "targeted" declaration, the one
// declaration of "a friendly unit is targeted" that an attack, a play, a cast, an activation and a
// prompt pick all answer through (`replacements::answer_targeting`).
// ---------------------------------------------------------------------------------------------

string_union! {
    /// The five points an event can be replaced at.
    pub enum ReplacementMoment {
        LethalHit = "lethalHit",
        Healed = "healed",
        WouldDie = "wouldDie",
        ToGraveyard = "toGraveyard",
        Targeted = "targeted",
    }
}

/// A heal's target, by id, so a replaced event is plain JSON (§10.1).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum HealedRef {
    Hero {
        player: PlayerId,
    },
    Unit {
        instance_id: String,
        controller: PlayerId,
    },
}

/// One unit a `wouldDie` event names.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct DyingUnit {
    pub instance_id: String,
    pub controller: PlayerId,
}

string_union! {
    /// `ReplacedEvent::Targeted.what`.
    pub enum TargetedWhat {
        Attack = "attack",
        Target = "target",
    }
}

/// The event a replacement is offered, as plain JSON: the follow-up step reads it back (R113).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "moment", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ReplacedEvent {
    /// A hit of `amount` (after Armor, divisor and caps) would bring `player`'s hero — or, since
    /// ME-LETHALGUARD (R1204), the Unit `instance_id` names — to 0 or less.
    LethalHit {
        player: PlayerId,
        amount: i32,
        source_id: Option<String>,
        /// The Unit the hit would kill, when it is a Unit's hit. Absent for a hero's, as before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        instance_id: Option<String>,
    },
    /// A heal of `amount` (its stated amount, R462) would land on `target`.
    Healed { target: HealedRef, amount: i32 },
    /// §4.5 step 1 collected these units, which would now die.
    WouldDie { units: Vec<DyingUnit> },
    /// This card would go to its owner's graveyard, from `from`.
    ToGraveyard {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
        from: String,
    },
    /// `by` chose this unit of `controller`'s — as an attack's target, or as a play's or prompt's pick.
    /// `source` is the targeting card's type (a Spell, a Unit, a Trap …); an attack has none (R651).
    Targeted {
        instance_id: String,
        controller: PlayerId,
        by: PlayerId,
        what: TargetedWhat,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<CardType>,
    },
}

impl ReplacedEvent {
    /// `event.moment`.
    pub fn moment(&self) -> ReplacementMoment {
        match self {
            ReplacedEvent::LethalHit { .. } => ReplacementMoment::LethalHit,
            ReplacedEvent::Healed { .. } => ReplacementMoment::Healed,
            ReplacedEvent::WouldDie { .. } => ReplacementMoment::WouldDie,
            ReplacedEvent::ToGraveyard { .. } => ReplacementMoment::ToGraveyard,
            ReplacedEvent::Targeted { .. } => ReplacementMoment::Targeted,
        }
    }
}

string_union! {
    /// Where the card must stand to replace: acting on the field (a unit on top of its pile, a backrow
    /// card — a Trap face-down, a Field Trap either way); in its controller's hand (Classic #33 Joro); or
    /// wherever it is, for an event about the card itself (Classic #60 Pile On's "if this would go").
    pub enum ReplacementWhere {
        Field = "field",
        Hand = "hand",
        SelfCard = "self",
    }
}

/// What a replacement's `when` reads: a PURE READ, like `conditionMet` — no writes, no rng.
#[derive(Clone, Copy)]
pub struct ReplacementContext<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    /// The card's controller: "your" in its text (§8 Conventions).
    pub controller: PlayerId,
    pub radiant: bool,
    /// Always the moment the declaration is `on` (TS narrowed the type; here the variant says it).
    pub event: &'a ReplacedEvent,
}

/// `ReplacementDef.when`.
pub type ReplacementWhen = Arc<dyn for<'a> Fn(ReplacementContext<'a>) -> bool + Send + Sync>;

/// Builds a `ReplacementWhen`.
pub fn replacement_when(
    f: impl for<'a> Fn(ReplacementContext<'a>) -> bool + Send + Sync + 'static,
) -> ReplacementWhen {
    Arc::new(f)
}

string_union! {
    /// `instead.redirect` (on `lethalHit`).
    pub enum InsteadRedirect {
        EnemyHero = "enemyHero",
        /// ME-LETHALGUARD (R1204): the hit goes to the replacing card itself, as a new instance
        /// from the same source through its own pipeline (Meditative #92 Unan).
        SelfCard = "self",
    }
}

string_union! {
    /// `instead.damage` (on `healed`).
    pub enum InsteadDamage {
        Pierce = "pierce",
    }
}

string_union! {
    /// `instead.lasting` (on `healed`).
    pub enum InsteadLasting {
        ThisTurn = "thisTurn",
    }
}

string_union! {
    /// `instead.flicker` (on `wouldDie`).
    pub enum InsteadFlicker {
        Yours = "yours",
    }
}

string_union! {
    /// `instead.to` (on `toGraveyard`).
    pub enum InsteadTo {
        Exile = "exile",
        BottomOfLibrary = "bottomOfLibrary",
    }
}

string_union! {
    /// R651: `by` on a `targeted` replacement.
    pub enum ReplacementBy {
        Spell = "spell",
    }
}

/// What a replacement does instead. The moment fixes which keys it may set (see `ReplacementDef`);
/// data, so a card may write it as the TS literal (`json_as(json!({ "to": "exile" }))`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementInstead {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redirect: Option<InsteadRedirect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage: Option<InsteadDamage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lasting: Option<InsteadLasting>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flicker: Option<InsteadFlicker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<InsteadTo>,
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpose: Option<bool>,
}

/// One replacement a card makes. The moment fixes what `instead` may say:
///  - `lethalHit`: `redirect: "enemyHero"` — the hit moves to the enemy hero as a new instance from
///    the same source, through that hero's Armor and caps (E9). Answers only for its own hero.
///    `redirect: "self"` — the hit moves to the replacing card itself as a new instance from the
///    same source, through its own pipeline (ME-LETHALGUARD, R1204). Answers for its controller's
///    hero and other Units, once each per hit.
///  - `healed`: `damage: "pierce"` — the heal becomes that much Pierce damage from this card (E8);
///    `lasting: "thisTurn"` also converts every later heal on its enemies this turn (a player
///    modifier). Answers only a heal on its controller's enemies.
///  - `wouldDie`: `flicker: "yours"` — its controller's units among the dying flicker instead (E22):
///    back in their zones at once, reset, summoning sick, no Death, no Reborn. One firing for them all.
///  - `toGraveyard`: `to` — exile, or the bottom of the owner's library.
///  - `targeted`: `interpose: true` — summon this from its controller's hand (leftmost open unit
///    zone; with none it does nothing), no Cry, summoning sick, and the attack or pick moves to it.
///    Answers only its controller's unit targeted by the other player (`TargetedReplacement`); `by:
///    "spell"` answers only a targeting whose `source` is a Spell (Classic #33 Joro, R651).
#[derive(Clone)]
pub struct ReplacementDef {
    pub id: String,
    pub on: ReplacementMoment,
    /// Default "field".
    pub where_: Option<ReplacementWhere>,
    /// Whether it replaces this event; absent, the moment alone arms it. Declining leaves a Trap set (R99).
    pub when: Option<ReplacementWhen>,
    pub instead: ReplacementInstead,
    /// A step of the card's `resume` table resolved after the replaced event, owed on `state.work`.
    pub then: Option<String>,
    /// R651 (on `targeted` only): when present, the replacement answers only a targeting by this kind of card.
    pub by: Option<ReplacementBy>,
}

/// B5 E5, E9: THE declaration of "a friendly unit is targeted" (Classic #33 Joro): a `ReplacementDef`
/// whose `on` is `Targeted` (TS narrowed the type; here `on` says it).
pub type TargetedReplacement = ReplacementDef;

// ---------------------------------------------------------------------------------------------
// B5 E33, R404: quests (TS `subsystems/quests.ts`), the data `Script.quests` holds.
// ---------------------------------------------------------------------------------------------

/// What completes a quest (`subsystems::quests`):
///
/// - `draws`: draws of yours — every draw that took a card, one burned at the hand cap or cast on draw
///   included; a draw a limit stopped never happened, and a fatigue draw takes no card (R541).
/// - `enemyPermanentsDestroyed`: permanents your opponent controlled as they were destroyed, by anything.
/// - `unspentManaAtTurnEnd`: a turn of yours ended with at least `mana` unspent (goal 1).
/// - `damageToEnemies`: damage your cards dealt to your opponent's hero and the units they controlled.
/// - `cardsExiled`: cards entering either exile pile (a unit token ceases to exist instead, R11).
/// - `deckEmptiedByDraw`: a draw of yours took the last card of your deck (goal 1); a deck already empty
///   when the quest opens completes it at once.
/// - board: `permanentsControlled` (the tops of your unit piles and your backrow cards, this one
///   included), `unitTotals` (your Units' total attack and total health both at least `total`),
///   `unitsInGraveyard` (Units in your graveyard).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestGoal {
    Draws { count: i32 },
    EnemyPermanentsDestroyed { count: i32 },
    UnspentManaAtTurnEnd { mana: i32 },
    DamageToEnemies { amount: i32 },
    CardsExiled { count: i32 },
    DeckEmptiedByDraw,
    PermanentsControlled { count: i32 },
    UnitTotals { total: i32 },
    UnitsInGraveyard { count: i32 },
}

/// One quest: its text as the view shows it, what completes it, and the rewards it offers, in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct QuestDef {
    pub id: String,
    pub text: String,
    pub goal: QuestGoal,
    pub rewards: Vec<String>,
}

/// One reward: its text, and the quest it opens next (null at the end of a line).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct QuestRewardDef {
    pub id: String,
    pub text: String,
    pub next: Option<String>,
}

/// A card's quest tree (`Script.quests`): the quest that opens as it enters, every quest, every reward.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct QuestBook {
    pub first: String,
    pub quests: Vec<QuestDef>,
    pub rewards: Vec<QuestRewardDef>,
}

// ---------------------------------------------------------------------------------------------

/// A card's two scripts (SURFACE §7.1): the base face's and the radiant face's.
#[derive(Clone, Default)]
pub struct CardScripts {
    pub base: Script,
    pub radiant: Script,
}

/// TS `EMPTY_SCRIPT = {}`: a script with no hook, no declaration and no flag.
pub fn empty_script() -> Script {
    Script::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::PlayerId;

    #[test]
    fn activation_ids_are_made_unique_in_face_order() {
        let decl = |id: &str| ActivationDecl {
            id: id.into(),
            label: "Activate".into(),
            uses: ActivationUses::Count(1),
            cost: None,
            targets: vec![],
            modes: vec![],
            can_activate: None,
            has: None,
            run: hook(|_ctx| vec![]),
        };
        let script = Script {
            activations: vec![decl("power"), decl("other"), decl("power")],
            ..Script::default()
        };
        let ids: Vec<String> = activation_decls(&script).into_iter().map(|d| d.id).collect();
        assert_eq!(ids, ["power", "other", "power#2"]);
        assert_eq!(activation_hook("power#2"), "activation:power#2");
        assert_eq!(
            serde_json::to_string(&ActivationUses::Unlimited).unwrap(),
            "\"unlimited\""
        );
        assert_eq!(
            serde_json::from_str::<ActivationUses>("3").unwrap(),
            ActivationUses::Count(3)
        );
    }

    #[test]
    fn a_context_is_a_sink_and_hooks_may_draw() {
        let mut state = crate::state::GameState {
            seed: "s".into(),
            ..serde_json::from_value(serde_json::json!({
                "seed": "s", "rngCursor": 0, "turn": 0, "active": "p1", "phase": "setup",
                "players": { "p1": serde_json::to_value(crate::state::create_player_state()).unwrap(),
                             "p2": serde_json::to_value(crate::state::create_player_state()).unwrap() },
                "pending": null, "triggerQueue": [], "declaredAttack": null, "work": [], "workCursor": 0,
                "echoQueue": [], "dispatch": [], "delayed": [],
                "counters": { "drawn": 0, "played": 0, "destroyed": 0, "exiled": 0 },
                "transientDefs": {}, "reserved": [], "mulliganed": [], "result": null,
                "nextId": 1, "nextSeq": 1, "applied": []
            }))
            .unwrap()
        };
        let mut events = Vec::new();
        let mut rng = Rng::new("golden", 0);
        let sink = EngineSink::new(&mut state, &mut events, &mut rng);
        let mut ctx = EffectContext::new(sink, PlayerId::P1);
        let drawing = hook(|ctx| {
            let _ = ctx.rng.coin();
            vec![Effect::new("noop", |ctx| ctx.state.turn += 1)]
        });
        let effects = drawing(&mut ctx);
        for effect in &effects {
            (effect.apply)(&mut ctx);
        }
        fn turn_of(sink: &EngineSink<'_>) -> i32 {
            sink.state.turn
        }
        assert_eq!(turn_of(&ctx), 1);
        assert_eq!(ctx.rng.cursor(), 1);
    }
}
