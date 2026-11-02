//! The card-script contract (SPEC §10.9, SURFACE §6.4–§6.6, §7): a card file returns `Vec<Effect>` from
//! the effects library and never touches state itself (CLAUDE.md rule 5). The types a `Script` field
//! names live here too, so it compiles on its own; their modules never redefine them.
//!
//! Hooks take `&mut EffectContext` because several draw from `ctx.rng` while they build their list
//! (Classic #10, #65, C+ #7, #8, #19.2, #19.3, #25, #37, #53, Core #23, #83); every read-only hook
//! takes a `Copy` bundle of references by value.

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

// The sink and the context

/// SURFACE §6.5: what every mutator that emits events or draws writes to, plus the per-call facts. A
/// nested sink (`reborrow`) carries the flags down and shares the frontier.
pub struct EngineSink<'a> {
    pub state: &'a mut GameState,
    pub events: &'a mut Vec<GameEvent>,
    pub rng: &'a mut Rng,
    /// The replacements' re-entrancy counter.
    pub converting: u32,
    /// Set while the scorer or the autopilot dry-runs a line.
    pub dry_running: bool,
    /// Which of the action's events the frontier has taken, shared by every sink over one event list.
    pub frontier: crate::triggers::FrontierSlot<'a>,
    /// R117: the owed items behind the one a drain is running.
    pub owed_behind: Option<indexmap::IndexSet<String>>,
    /// R1884001: the cards whose `resolveFirst` would-die replacement is resolving up this call, which
    /// a check its own follow-up runs does not ask again (`replacements::would_die_window`).
    pub dying_replaced_by: Vec<String>,
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
            dying_replaced_by: Vec::new(),
        }
    }

    /// The same sink for a nested call: the flags are copied down (exact, since every change to
    /// `converting` is undone before its call returns) and the frontier is shared, so both loops see one.
    pub fn reborrow(&mut self) -> EngineSink<'_> {
        EngineSink {
            state: &mut *self.state,
            events: &mut *self.events,
            rng: &mut *self.rng,
            converting: self.converting,
            dry_running: self.dry_running,
            frontier: crate::triggers::FrontierSlot::Shared(self.frontier.get_mut()),
            owed_behind: self.owed_behind.clone(),
            dying_replaced_by: self.dying_replaced_by.clone(),
        }
    }
}

/// What a running script sees: an `EngineSink`, passed wherever a sink is taken, plus the facts of this run.
pub struct EffectContext<'a> {
    pub sink: EngineSink<'a>,
    /// R136: where *this script's* events begin in the whole action's `events`: a card asking "what did I
    /// just do" (#60 Bear Honeypot, #24, #31, #33, #38) reads only `events[events_from..]`, so another copy
    /// or a mid-action trap cannot feed it. A step resumed after a prompt opens a new window (`summoned`).
    pub events_from: usize,
    /// R174: the field's departures when this run began (`stays::exit_mark`), carried across a pause, so a
    /// later effect tells a card that stood there then from a new stay of it. Absent reads as "now".
    pub exits_from: Option<u32>,
    /// R174, §10.6: the departures when this run's own prompt chose `targets`, so a card that left and
    /// stood there again when offered (a Reborn body, R83) is picked on that stay. Absent: `exits_from`.
    pub chosen_from: Option<u32>,
    /// R174, R212: the cards a queued trigger's event names and the departures when it happened
    /// (`stays::event_stay_of`), so they are judged from then: a trigger reading the played unit's id
    /// misses the Reborn body an earlier trigger on it made (R59). Absent unless a queued trigger's run.
    pub event_stay: Option<EventStay>,
    /// R136: the units this run summoned in the actions before a prompt split it, which the resumed list's
    /// `events_from` window misses (`work::RunMarks`). Absent for a run that has not paused.
    pub summoned: Option<Vec<String>>,
    /// R98: the running card sat in the resolving zone as the run began (§10.5 step 4: a Spell, or a
    /// permanent that found no zone), so a continuation re-entered once it has left resumes with no self
    /// (`prompts::run_resume`). Carried across a pause (`work::RunMarks`); absent for any other card.
    pub self_resolving: Option<bool>,
    /// Who is resolving this: the controller of `self_`, or the player who cast the card.
    pub controller: PlayerId,
    /// The running instance as the context was built, when it still exists (a Death hook's is the card as
    /// it died, R89); `live_self` reads it as it stands now.
    pub self_: Option<CardInstance>,
    /// R127: the running definition, set where a continuation is re-entered (`prompts::run_resume`): a step
    /// that asks again must name its script once `self_` is `None`. Absent where `self_` names it.
    pub def_id: Option<String>,
    /// Whether the radiant text is the one running (§5.2).
    pub radiant: bool,
    pub targets: Vec<Selection>,
    pub modes: Vec<String>,
    pub x: i32,
    pub embiggened: bool,
    /// Captured data from a Resume, for chained steps (§10.6).
    pub data: IndexMap<String, Value>,
    /// Classic #22 Mid Runner's "If you had 4 or more mana when you played this": the mana as the play or
    /// cast began (§10.5 step 1), on its own Cry and continuations (`resolve::MANA_BEFORE_PLAY_KEY`).
    pub mana_before_play: Option<i32>,
}

impl<'a> EffectContext<'a> {
    /// A context with every fact of the run at rest, for tests; the engine's is `resolve::make_context`.
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

    /// The running card as it stands now, or as the context was built when it is no longer anywhere.
    /// R89: a context whose data carries the card's snapshot (`prompts::SELF_KEY`: a Death hook, a dead
    /// attacker's After Attack, their continuations) runs as that snapshot, never as the card R78 reset on
    /// the board under the same id, so a Death hook reads the memory it died with (#22's meal, R41).
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

// Effects and hooks (SURFACE §6.6)

/// What `Effect::expand` hands every rebuild so the part is the same one (`None` on the first build):
/// JSON, because it rides a paused step (`work::PausedStep.memo`).
pub type Memo = Option<Value>;

pub type EffectApply = Arc<dyn Fn(&mut EffectContext<'_>) + Send + Sync>;

pub type EffectExpand = Arc<dyn Fn(&mut EffectContext<'_>, &Memo) -> EffectPart + Send + Sync>;

/// One state change from the effects library. Effects are built by engine code, so a card file
/// composing them stays pure.
#[derive(Clone)]
pub struct Effect {
    pub kind: &'static str,
    pub apply: EffectApply,
    /// A part of a composed list, built when the list reaches it, so a later fused ingredient reads the
    /// board the earlier ones left (R77, R102: #68 after Reno's heal, #22 after #100's exile). A prompt in
    /// it pauses there (`work::PausedStep.part`); only that part is rebuilt, from `memo` (#95's roll, R87).
    pub expand: Option<EffectExpand>,
}

impl Effect {
    pub fn new(kind: &'static str, apply: impl Fn(&mut EffectContext<'_>) + Send + Sync + 'static) -> Effect {
        Effect {
            kind,
            apply: Arc::new(apply),
            expand: None,
        }
    }

    /// An effect that is a lazily built part.
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

pub type Hook = Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync>;

/// Builds a `Hook` from a closure: `hook(|ctx| vec![…])` (SURFACE §6.6, §7.2).
pub fn hook(f: impl Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync + 'static) -> Hook {
    Arc::new(f)
}

/// `TriggerDef.when`: R99's condition over the context and the event. `&mut`: Classic+ #74's
/// predicate writes its own card while it declines.
pub type TriggerWhen = Arc<dyn Fn(&mut EffectContext<'_>, &GameEvent) -> bool + Send + Sync>;

pub type TriggerRun = Arc<dyn Fn(&mut EffectContext<'_>, &GameEvent) -> Vec<Effect> + Send + Sync>;

/// A trigger a card registers while it is in a given zone (§10.3).
#[derive(Clone)]
pub struct TriggerDef {
    pub id: String,
    pub on: Vec<GameEventType>,
    /// R99: the trigger's condition, kept out of `run` so a trap can decline an event unspent (R61: an
    /// empty effect list is "fired and did nothing"); absent, the `on` match arms it. The traps' dispatch
    /// reads a trap's; every other trigger's is read as its queued entry resolves (R2271000).
    pub when: Option<TriggerWhen>,
    pub run: TriggerRun,
    /// R2396000: "Whenever this destroys a Unit": answers a `destroyed` its card is R42's killer of
    /// though the card has left the field since, as the snapshot it killed with
    /// (`triggers::offer_to_gone_killer`).
    pub even_if_gone: bool,
}

impl TriggerDef {
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
            even_if_gone: false,
        }
    }

    pub fn with_when(
        mut self,
        when: impl Fn(&mut EffectContext<'_>, &GameEvent) -> bool + Send + Sync + 'static,
    ) -> TriggerDef {
        self.when = Some(Arc::new(when));
        self
    }

    /// R2396000: the same trigger, answering a kill even once its card is gone.
    pub fn even_if_gone(mut self) -> TriggerDef {
        self.even_if_gone = true;
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

/// The argument of every pure-read hook on the card itself (`aura`, `setStat`, `heroGuard`, …).
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// One entry an aura returns: the units it touches (`applies` may borrow the hook's arguments) and its mod.
pub struct AuraEntry<'a> {
    pub applies: Box<dyn Fn(&CardInstance) -> bool + 'a>,
    pub mod_: StatMod,
}

/// An aura contributes stat and keyword layers while its card is in play (§10.4 layer 5).
pub type AuraHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<AuraEntry<'a>> + Send + Sync>;

pub fn aura_hook(f: impl for<'a> Fn(HookArgs<'a>) -> Vec<AuraEntry<'a>> + Send + Sync + 'static) -> AuraHook {
    Arc::new(f)
}

/// MD-D4, R1120: what one combat-only attack modifier adds to an attacker's strike in the combat it
/// started against a Unit — attack on top of the layered value, and Poisonous on those hits (§4.4
/// step 7). Never a layer: the shown attack and §4.2 step 1's "attack above 0" do not see it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct AttackMod {
    pub attack: i32,
    pub poisonous: bool,
}

/// `Script.attack_mods`' argument: `HookArgs` with the combat's attacker and defender. A PURE READ.
#[derive(Clone, Copy)]
pub struct AttackModArgs<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    pub radiant: bool,
    pub attacker: &'a CardInstance,
    pub defender: &'a CardInstance,
}

/// MD-D4, R1120: what this card adds to an attacker's strike, read only inside a combat against a
/// Unit (declared or forced).
pub type AttackModHook = Arc<dyn for<'a> Fn(AttackModArgs<'a>) -> AttackMod + Send + Sync>;

pub fn attack_mod_hook(
    f: impl for<'a> Fn(AttackModArgs<'a>) -> AttackMod + Send + Sync + 'static,
) -> AttackModHook {
    Arc::new(f)
}

/// `boolean | number`: a static flag a fused card may carry several times (R102).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(untagged)]
pub enum FlagOrCount {
    Flag(bool),
    Count(i32),
}

impl FlagOrCount {
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TuneMultiplier {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upgrade: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub degrade: Option<i32>,
}

/// A card's static flags. Data, so a card may write them as an object literal
/// (`json_as(json!({ "castOnDraw": true }))`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct StaticFlags {
    /// Plays itself on draw, then draws again (§6.2, R58).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_on_draw: Option<bool>,
    /// Starts in the opening hand instead of a draw (§6.2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quickdraw: Option<bool>,
    /// R1861000: sells from its owner's hand at an open Marketplace, as a tray Resource sells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hand_sale: Option<bool>,
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
    /// the granting instance's own face, as #84's Armor is (`HERO_ARMOR`).
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
    /// (`resolve::note_return_price`) for its return to read (`query::return_price_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_keeps_price: Option<bool>,
    // Static flags by workstream: field (B3.1, E20, E21, E22)
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
    // Static flags by workstream: play pipeline (E1, E2, E5 targeting, E11, E12, E15)
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
    // Static flags by workstream: damage and combat (E5, E6, E8, E35)
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
    /// MD-B1, R940: while this card acts on the field, harmful effects whose card filter names a tribal tag
    /// can neither pick it nor reach it through scopes; combat bonuses against defender tags do not apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immune_to_tribal_hate: Option<bool>,
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
    /// Meditative #91 Windfast (ME-ATTACKSUMMON, R1202, R831006): when this would attack, a random
    /// Unit summoned from its controller's hand makes that attack instead.
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
    // Meditative
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
    // Meditative+ (issue #1180)
    /// R1180000 (Meditative+ #9 Magic Amplifier): "Aura: Both players have Spell Damage +X". While this
    /// card acts on the field, its controller — and with `both`, the opponent too — has Spell Damage +X,
    /// summed with every other source (`damage::spell_damage_of`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spell_damage_aura: Option<SpellDamageAura>,
    /// R1180003 (Meditative+ #7 Life Mage's Radiant face): "healing your hero and Units is twice as
    /// effective". While this card acts on the field, every heal of its controller's hero and Units is
    /// `HEAL_MULTIPLIER` times as large (`query::heal_multiplier_of`); several multiply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doubles_healing: Option<bool>,
    /// R1180005 (Meditative+ #17 Dogpile): in its owner's hand or deck, this card answers their Spells
    /// that targeted an enemy (`targeted.rs`): the engine names the one card that answers each such
    /// Spell on its `cardResolved` (`answer`), which the card's hand and deck triggers read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers_targeted_spells: Option<bool>,
    // Meditative+ (issue #1181)
    /// R1181002 (Meditative+ #22 Mountaintop Garden): while this card acts on the field, every embiggen
    /// card of both players (R1178001) plays its Embiggened version at its normal price, and its dearer
    /// price, the normal price plus this premium, plays it Radiant and Embiggened; a Radiant card has its
    /// normal price only (`mana::garden_premium`). The premium is the card's declared number `premium`
    /// where it declares it, else this; two Gardens apply the least.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embiggen_garden: Option<i32>,
    // Stinkyscape's mechanic (issue #1839)
    /// R1839000: "Dormant" (SPEC §6.1): this Unit enters the field Dormant — from a hand, a deck, a
    /// graveyard or the resolving zone, any way it arrives — and is reached by nothing but an awakening
    /// until it awakens (`slumber.rs`). What it prints is a `SlumberText`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slumber: Option<SlumberText>,
    // Stinkyscape's Starship (issue #1852)
    /// R1852000, R1852003: this is a Starship's hull — the Dormant Starship (`DORMANT_STARSHIP_DEF_ID`),
    /// and every fusion it keeps. Entering the field from off it as a Unit, the card starts a Starship's
    /// memory (`CardInstance.starship`); while it is Dormant its one ability is the engine's launch
    /// (`starship::launch_decl`), a Stack card played onto it is exiled before it resolves (R1852007),
    /// and its awakening launches it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starship_hull: Option<bool>,
    // Stinkyscape M15 (issue #1849)
    /// R1849000 (Stinkyscape #74 Paragon-X): "Immune to Spell Damage". While this Unit is on the field,
    /// a hit from a Spell deals it nothing and is no damage instance (§4.4 step 0); Spells may still
    /// target it and do everything else to it (`restrictions::spell_damage_cannot_reach`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub immune_to_spell_damage: Option<bool>,
    /// R1849002 (Stinkyscape #105 Disintegrex): "Anytime something would be Destroyed, it is Exiled
    /// instead". While this card acts on the field, every permanent the state check collects is exiled
    /// instead (§4.5 step 1, `replacements::destroy_exiles`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destroyed_exiled: Option<bool>,
    /// R1849002 (Disintegrex's Radiant face): the same, for the permanents of this card's opponent only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enemy_destroyed_exiled: Option<bool>,
    /// R1849003 (Stinkyscape #104 Reflective Plating): "Any damage this takes is dealt to the enemy hero"
    /// — as a new instance from this Unit, after Lifesteal (§4.4 step 8a, `damage::reflect_damage`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflects_damage: Option<bool>,
    /// R1849003 (Reflective Plating's Radiant face): "… and the attacker": what an attacker's own strike
    /// deals this Unit is dealt back to that attacker too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflects_to_attacker: Option<bool>,
    /// R1849004 (Stinkyscape #5 Masochism Unmasked): while this card acts on its controller's field, the
    /// damage their hero would take on their own turn goes to the opponent's hero instead, up to a cap a
    /// turn (§4.4 step 0a, `damage::own_turn_redirect`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_turn_redirect: Option<OwnTurnRedirect>,
    /// R1849005 (Stinkyscape #157 The Gauntlet): while this card acts on the field, the Units in the
    /// unit zones of lanes `GAUNTLET_LANES` attack only one another, and no other Unit attacks them
    /// (`restrictions::attack_restriction_in`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gauntlet: Option<bool>,
    /// R1849006 (Stinkyscape #23 Warp Disruptor): "Aura: Units cannot be in Defense Position". While
    /// this card acts on the field, no Unit on either side may switch to Defense Position, and one in it
    /// switches to Attack Position at the state check (`restrictions::defense_barred`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units_never_defense: Option<bool>,
    /// R1849009 (Stinkyscape #174 Aerelret, #22 Gatinsir): "30% chance to Reveal": the percent chance a
    /// face-down trap fires when its triggers admit an event, rolled once per firing; a miss leaves it
    /// set (`traps::chance_admits`). The card's declared number `chance` where it declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reveal_chance: Option<i32>,
    // Stinkyscape (issue #1844)
    /// R1844000 (Stinkyscape #46 Circadian Halfcycler): while this card acts on the field, the
    /// autopilot plays the first this-many R456 actions of each of its controller's turns, then hands
    /// the turn back (`subsystems::autopilot::aura_reach`); several take the largest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pilots_actions: Option<i32>,
    /// R1844000 (Stinkyscape #47 Circadian Fullcycler): while this card acts on the field, the
    /// autopilot plays each of its controller's turns whole, as Cognitive Offloading does.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pilots_turn: Option<bool>,
    /// R1844001 (Stinkyscape #147.2 Wardrum Blob): while this card acts on the field, its controller
    /// is shown the Pareto hint on each of their turns (`subsystems::pareto::pareto_hint_ids`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hints_plays: Option<bool>,
    // Stinkyscape S16 (issue #1869)
    /// R1869000 (Stinkyscape #16 Rod of Ages): "The first Unit you play onto this is Equipped into it".
    /// While this Unit's definition is no fusion, any Unit card its controller plays may be played onto
    /// it as an Equip play (R1086) though it has no Equip (`play_choices::equip_onto_at`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lends_equip: Option<bool>,
}

/// R1849004 (Stinkyscape #5 Masochism Unmasked): "Redirect all damage you take on your turn to your
/// opponent (up to 20)". `cap` is how much may be taken off its controller's hero each turn, per card
/// (its declared number `cap` where it declares it); `multiplier` is how many times what moves is dealt
/// to the opponent ("and double it", the Radiant face's 2). Absent is 1.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct OwnTurnRedirect {
    pub cap: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplier: Option<i32>,
}

/// R1839000–R1839002: what a card's "Dormant" prints (`StaticFlags.slumber`). `turns` is "Dormant for
/// N turns", the countdown it enters with (R1839001); absent, it is Dormant until an effect awakens it.
/// `wakesBy` names the one Activate ability the card keeps while it is Dormant (the Dormant Starship's
/// "pay (3) to awaken", whose effect is `effects::awaken` on itself, R1839002); its other abilities,
/// like the rest of its text, are off until it awakens. Named apart from the "dormant" of a card
/// beneath a Stack pile (§3.2, R13).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SlumberText {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turns: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wakes_by: Option<String>,
}

/// R1180000 (Meditative+ #9 Magic Amplifier): the Spell Damage an aura gives — `amount` to the
/// source's controller, and to the opponent as well when `both`. The amount is the printed number,
/// moved by as much as a Nerf, a Buff or KY's Constant moved the card's declared `spellDamage`
/// (`params::declared_or`, R386).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SpellDamageAura {
    pub amount: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub both: Option<bool>,
}

string_union! {
    /// R195, R280: where `view_for` is asking about a card.
    pub enum ConditionZone {
        Hand = "hand",
        Field = "field",
    }
}

/// R195, R280, §10.9: the argument of the two read-only hooks `view_for` asks, the "yellow glow"
/// predicate (`conditionMet`) and a formula's number now (`preview`): PURE READS that must agree with
/// what the card's own resolution would do now.
#[derive(Clone, Copy)]
pub struct ConditionContext<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    /// The card's controller, never the viewer as such: R195 asks only of the viewer's own cards, R280
    /// of any card the viewer may read, the other seat's public ones included.
    pub controller: PlayerId,
    pub radiant: bool,
    /// "hand": as if played now. "field": as the card on the field reads it now.
    pub zone: ConditionZone,
    /// `state.active == controller`, so a card file never reads `state.active` itself.
    pub your_turn: bool,
}

pub type ConditionHook = Arc<dyn for<'a> Fn(ConditionContext<'a>) -> bool + Send + Sync>;

/// R280, §10.9: the labeled numbers a card's formula comes to now (#31's Fib(cost+1), #70's sum), each
/// `label` an exact substring of the running face's text (the client prints the value after it, §10.10).
/// A PURE READ on the function the resolution computes with, so the two agree, of only what its
/// controller may read (§9.1): `view_for` shows it to every viewer of the card (`preview.rs`).
pub type PreviewHook = Arc<dyn for<'a> Fn(ConditionContext<'a>) -> Vec<PreviewValue> + Send + Sync>;

/// R2321000, §10.8: one count a card on the field keeps toward what it will do to a Unit on the field,
/// `value` of `goal` (TG #31 Level Up!'s attacks toward making the Unit Radiant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitTally {
    pub instance_id: String,
    pub value: i32,
    pub goal: i32,
}

/// R2321000: the counts a card keeps on the Units of the field, surfaced on each as `UnitView.tallies`
/// (`preview::tallies_on`). Asked as `preview` is; a PURE READ of the card's own memory and the board,
/// so a count the card would forget (a Unit that left and came back, R174) is not shown.
pub type TallyHook = Arc<dyn for<'a> Fn(ConditionContext<'a>) -> Vec<UnitTally> + Send + Sync>;

/// Builds a `ConditionHook`, a `PreviewHook` or a `TallyHook`.
#[allow(clippy::type_complexity)]
pub fn condition_hook<R: 'static>(
    f: impl for<'a> Fn(ConditionContext<'a>) -> R + Send + Sync + 'static,
) -> Arc<dyn for<'a> Fn(ConditionContext<'a>) -> R + Send + Sync> {
    Arc::new(f)
}

/// `Script.cost`'s argument.
#[derive(Clone, Copy)]
pub struct CostArgs<'a> {
    pub state: &'a GameState,
    pub instance: &'a CardInstance,
}

pub type CostHook = Arc<dyn for<'a> Fn(CostArgs<'a>) -> i32 + Send + Sync>;

pub fn cost_hook(f: impl for<'a> Fn(CostArgs<'a>) -> i32 + Send + Sync + 'static) -> CostHook {
    Arc::new(f)
}

/// `Script.setStat`'s answer.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SetStat {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_health: Option<i32>,
}

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
/// is `minCost` or more (Classic #68 Radiant), and never a cast (R70). The rule's fields are flattened
/// beside these.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct CostAura {
    #[serde(flatten)]
    pub rule: CostRule,
    pub whose: CostAuraWhose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ban: Option<bool>,
}

/// The argument of `Script.costAura`, as `Script.aura`'s is: a pure read of the board.
pub type CostAuraArgs<'a> = HookArgs<'a>;

pub type CostAuraHook = Arc<dyn for<'a> Fn(CostAuraArgs<'a>) -> Vec<CostAura> + Send + Sync>;

/// E11, R454: what one card on the field lets its controller play from their own graveyard.
///
/// - `units`: Units only (Classic #74); absent, every card type (Classic #28, In Too Deep's reward L).
/// - `minPrice`: the least price a play under it may have, as it would be paid — after every discount
///   (Classic #28 Radiant: (1), which stops a (0) loop).
/// - `plague`: plays under it pay with the Plague Counters on the granting card (Classic #74): each token
///   pays PLAGUE_TOKEN_MANA, at least MIN_PLAGUE_PAYMENT of them, at most the tokens there and the
///   price, and the rest in mana. Such a play must spend tokens; a permission without `plague` is paid
///   in mana alone.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct GraveyardPlayPermission {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub units: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_price: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plague: Option<bool>,
}

pub type GraveyardPlayHook =
    Arc<dyn for<'a> Fn(CostAuraArgs<'a>) -> Vec<GraveyardPlayPermission> + Send + Sync>;

/// R1045: the Echo this card adds to each Spell its controller reveals while it acts on their field
/// (Radiant Paranoia), read as the Spell reveals (`alt_play::reveal_echo_bonus`). 0 adds none.
pub type RevealEchoHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

pub type TradesPerTurnHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

pub type TargetingDiscardsHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

pub type RecordsPlayAsHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Option<PlayRecord> + Send + Sync>;

/// One guard `Script.heroGuard` returns: a per-hit cap and a divisor applied after Armor.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct HeroGuard {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cap: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divisor: Option<i32>,
    /// R2179005: the cap holds on each hit on the guarding Unit itself too, at §4.4 step 3 after its
    /// Armor ("Your hero and this can't take more than (1) damage at a time", C+ #11 Anime Armor).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guards_self: Option<bool>,
}

pub type HeroGuardHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<HeroGuard> + Send + Sync>;

pub type ConditionalKeywordsHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<Keyword> + Send + Sync>;

pub type PlagueMultiplierHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

pub type EchoXHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

/// ME-TRIG, R820: how many extra times this card makes its controller's hooks of one kind run.
pub type TriggerExtraHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> i32 + Send + Sync>;

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

pub type WouldCounterHook = Arc<dyn for<'a> Fn(WouldCounterArgs<'a>) -> bool + Send + Sync>;

pub fn would_counter_hook(
    f: impl for<'a> Fn(WouldCounterArgs<'a>) -> bool + Send + Sync + 'static,
) -> WouldCounterHook {
    Arc::new(f)
}

pub type ModeAllowedHook = Arc<dyn Fn(&GameState, PlayerId, &str) -> bool + Send + Sync>;

pub fn mode_allowed_hook(
    f: impl Fn(&GameState, PlayerId, &str) -> bool + Send + Sync + 'static,
) -> ModeAllowedHook {
    Arc::new(f)
}

pub type PlayAllowedHook = Arc<dyn Fn(&GameState, PlayerId) -> bool + Send + Sync>;

pub fn play_allowed_hook(
    f: impl Fn(&GameState, PlayerId) -> bool + Send + Sync + 'static,
) -> PlayAllowedHook {
    Arc::new(f)
}

pub type HasHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> bool + Send + Sync>;

/// A card face's hooks and declarations. Every hook given no `rng` (a `HookArgs`, a `&GameState`) is a
/// PURE READ, so one reading a declared number follows its Nerfs and Buffs (B3.4); a list-valued one
/// lets a fused card carry each ingredient's (R102).
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
    /// R781004: run on the card a Fuse keeps once this text is fused into it (Meditative #65's Equip), as
    /// R43's roll is: what this text drew for its own card ceased with it, so the kept card draws its own.
    pub fused_into: Option<Hook>,
    /// Named continuations a prompt answer re-enters (§10.6, R81).
    pub resume: IndexMap<&'static str, Hook>,
    /// A delayed effect this card scheduled, resolved at its R62 point.
    pub delayed: Option<Hook>,
    /// §10.4 layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39).
    pub set_stat: Option<SetStatHook>,
    pub start_of_turn: Option<Hook>,
    pub end_of_turn: Option<Hook>,
    /// ME-GRANT (MD-D13): the Death abilities this card grants, by key (`meditative-058#bookDeath` grants
    /// face `bookDeath`), so state stays plain data; keys name their definition, so a fusion's never collide.
    pub grants: IndexMap<&'static str, Hook>,
    pub aura: Option<AuraHook>,
    pub triggers: Vec<TriggerDef>,
    pub on_play_hook: Option<Hook>,
    pub hand_triggers: Vec<TriggerDef>,
    pub static_flags: Option<StaticFlags>,
    /// The play-time choices this card declares (R81).
    pub targets: Vec<TargetDecl>,
    pub modes: Vec<ModeDecl>,
    /// R831008: which of the card's own declared modes may be chosen now (Meditative #98 Showdown: only an
    /// empty lane), asked by `legalActions`, the play's refusal and a cast's own pick alike.
    pub mode_allowed: Option<ModeAllowedHook>,
    /// R1851000: whether the card may be played now, when its play needs something on the board but
    /// declares no target (R703; Meditative #53 Prestige's Radiant face). It reads what its player may
    /// see (§9.1), off the face the play resolves (R214, R399), asked by `legalActions`, the refusal,
    /// `playReason` (`play_choices::play_allowed`) and a set Spell's Reveal (R783006); never by a cast,
    /// an Echo repeat or a random cast (R70, R1851001). A fusion holds each ingredient's (R1851002).
    pub play_allowed: Option<PlayAllowedHook>,
    /// R865: the card's declared modes are kept secret — filed as a secret record, not carried on the
    /// play's events, and blanked from an in-flight play the other seat reads.
    pub secret_modes: bool,
    /// R195: the condition `view_for` surfaces as `conditionActive` (§10.8).
    pub condition_met: Option<ConditionHook>,
    /// R280: the numbers the card's formula comes to now, which `view_for` surfaces as `preview` (§10.8).
    pub preview: Option<PreviewHook>,
    /// R2321000: the counts the card keeps on Units of the field, surfaced on them as `tallies` (§10.8).
    pub tallies: Option<TallyHook>,
    /// B3.2, R384: the card's Activate abilities, used by the `activate` action while it acts on the
    /// field (`subsystems::activate`).
    pub activations: Vec<ActivationDecl>,
    /// §10.6: the named predicates a declaration's `TargetFilter.check` points at, for a filter no data
    /// field can say (Classic #32's lane rule, #48's lines of code). A pure read, like `conditionMet`.
    pub target_checks: IndexMap<&'static str, TargetCheck>,
    // Play pipeline B (E11, E15): hooks, not flags, so a Degrade or Upgrade of the card's declared
    // numbers moves what it grants and a grant can hang on its state (Classic #90's reward L).
    /// E15, R455: the price rules this card lays on cards while it acts (`cost_rules.rs`).
    pub cost_aura: Option<CostAuraHook>,
    /// E11, R454: the permissions this card gives its controller to play cards from their graveyard.
    pub graveyard_play: Option<GraveyardPlayHook>,
    pub reveal_echo: Option<RevealEchoHook>,
    /// B5 E5, Classic #89 Paul Allen's Ghost: "to target this with anything but an attack, a player must
    /// also discard N cards", 0 or absent none. Random at pay time (R682), so no action carries them: a play
    /// (§10.5 step 2), activation or prompt answer pays them (`targeting.rs`, `targeting_point.rs`).
    pub targeting_discards: Option<TargetingDiscardsHook>,
    /// B5 E4, R451: what this card's play records as the last Spell played and its player's last face-up
    /// play: absent, the card itself; Classic #57 Echo returns the Spell it copied, or `None` for nothing.
    pub records_play_as: Option<RecordsPlayAsHook>,
    // Script hooks by workstream: activate and turn (E27, E28)
    /// B5 E3, R457: the draw limits this card sets while it acts on the field (Classic #4, #49); the
    /// lowest limit on a player holds.
    pub draw_limit: Option<DrawLimitHook>,
    /// R800: the players (relative to its controller) whose hand this card, acting on the field, guards
    /// from an effect's discard while it is not their turn (Meditative #1, `draw::discard_guarded`).
    pub discard_guard: Option<DiscardGuardHook>,
    // Script hooks by workstream: damage and combat (E5, E6, E8, E9, E35)
    /// B5 E5, R460: the events this card changes before they happen (`replacements.rs`), declared as data
    /// and decided synchronously, never an effect list. The "targeted" entry (`TargetedReplacement`)
    /// covers every targeting: attack, play, cast, activation and prompt pick.
    pub replacements: Vec<ReplacementDef>,
    /// B5 E6: what this card does to hits on its controller's hero while it acts on the field: a per-hit
    /// cap (the lowest wins, Classic+ #11 Anime Armor) and a divisor after Armor (several multiply, rounded
    /// up once, Classic #75 Argusland).
    pub hero_guard: Option<HeroGuardHook>,
    /// MD-D4, R1120: the combat-only attack modifiers this card lays while it acts on the field.
    pub attack_mods: Option<AttackModHook>,
    /// B5 E35: keywords the card has only while a condition holds (Classic #69 Plague Charger), read in
    /// the layers with its printed keywords (§10.4), so a Vanilla takes them. A PURE READ of instance
    /// data that must never call back into `unit_view`, or the layers would recurse.
    pub conditional_keywords: Option<ConditionalKeywordsHook>,
    /// "When this attacks" (Classic #13, #45, Classic+ #73.1, Stinkyscape #75): run for the attacker before
    /// any damage of each attack, a declared one's once its trap window has closed (before the defender's
    /// `allyAttacked`), a forced one's before its combat (R2285000). Parkable (R113); `ctx.data`:
    /// `combat::when_attacks_of`. An attacker or target gone after it does not fight (R220).
    pub when_attacks: Option<Hook>,
    /// "After this attacks" (Classic+ #73.1, Stinkyscape #38, #91, #92 and #94): run for the attacker
    /// once the state check closing each of its combats (declared or forced) has run, only while it is
    /// still on the field on the stay it attacked from (R2285001), unless `after_attack_even_if_gone`;
    /// never for an attack called off before it fought (R44). `ctx.data`: `combat::after_attack_of`.
    pub after_attack: Option<Hook>,
    /// R2396000: "Whenever this attacks" (Core #32 Prem Panther): `after_attack` runs also for an
    /// attacker that died or left the field in that combat, as the snapshot it fought with
    /// (`prompts::SELF_KEY`), `survived` false.
    pub after_attack_even_if_gone: bool,
    /// "After this is attacked" (Meditative #49.3 AI Girlfriend): run for the defender once the state
    /// check closing each combat it was the target of (declared or forced) has run, also when it died
    /// there, then on the snapshot it fought with (R78, R89). Not for a Cleave splash nor an attack on the
    /// hero (MD-C26, R1026). `ctx.data`: `combat::after_attacked_of`.
    pub after_attacked: Option<Hook>,
    /// R1849001 (Stinkyscape #91 The Propagation's Wasp): "Evolve: X", run for this Unit only when an
    /// effect evolves it (`effects::evolve`), resumably like `fusedInto` (`prompts::run_evolve`).
    pub evolve: Option<Hook>,
    /// R1849008 (Stinkyscape #14 Arm of the Jempire): "When one of your Units is attacked". Run for each
    /// card acting on the defender's side once a declared attack on one of its Units has come through
    /// §4.2 step 4's trap window, before its combat, in R68's order; never for an attack on the hero or a
    /// forced one (R121). `ctx.data`: `combat::ally_attacked_of`.
    pub ally_attacked: Option<Hook>,
    /// R2199000 (Training Grounds #31 Level Up!): "After one of your Units attacks". Run for each card
    /// acting on the attacker's side, the attacker included, in R68's order, once the state check closing
    /// each of its combats (declared or forced) has run, after `afterAttack` and `afterAttacked`; not for
    /// an attack called off before it fought (R44). `ctx.data`: `combat::ally_attack_of`.
    pub ally_attack: Option<Hook>,
    // Script hooks by workstream: prompts and generation (E13, E19, E26)
    /// B5 E19, R471: "Plague Counters placed on this are doubled" (Classic #27 Pestilent Slime): what each
    /// placement is multiplied by as it lands, floored at 1; a fusion's multiply (`subsystems::fuse`).
    pub plague_multiplier: Option<PlagueMultiplierHook>,
    /// R802: a computed Echo X (Meditative #5), read once as the card is played, when its Echo repeats are
    /// queued (§10.5 step 4), so X stays fixed while it resolves. The larger of it and
    /// `staticFlags.echo` is the printed Echo, which tuning and Twinspell add to (`echo::printed_echo`).
    pub echo_x: Option<EchoXHook>,
    /// B5 E26, R464: triggers this card answers while in its owner's library (Classic+ #37 Wardrum). It is
    /// hidden (§9.1), so its queue entries take no number (R177) and, within a side, follow the hand's and
    /// precede the graveyard's in creation order, never library order (`triggers::trigger_holders_of`).
    pub deck_triggers: Vec<TriggerDef>,
    /// B5 E26: triggers this card answers while it lies in a graveyard (Classic #47). R153's other
    /// graveyard answer, the end-of-turn return, stays the `endOfTurn` hook's.
    pub graveyard_triggers: Vec<TriggerDef>,
    /// B5 E33, R404: the card's quest tree (Classic #90 In Too Deep), as data: `subsystems::quests` opens
    /// it as the card enters, counts on `memory.quest` and reports `questCompleted` to the card's trigger.
    pub quests: Option<QuestBook>,
    /// R1854000: a staged quest's stages, in order (Stinkyscape #7, #112 Job Application), for a card with
    /// no `quests`: it runs on `staged_book(stages)`, one public stage open at a time, the first on entry.
    pub stages: Vec<QuestStage>,
    // Script hooks by workstream: Classic #46–#90 (card-specific, cards-classic-b)
    /// Classic #88 Siphon Squad, R403: "When …, Tribute this" — a condition on the card's own text that
    /// every state check reads (§4.5), the one right after the card arrives included; while it holds, the
    /// card acting on the field (face-down too) is sacrificed (`state_check.rs`).
    pub tribute_when: Option<TributeWhenHook>,
    /// Classic #87 Plague Chalice, R667: whether the card's own trigger would now counter a play `player`
    /// makes paying `costPaid`. A PURE READ the trigger asks too, so they agree; `counter_warning.rs`
    /// reads it to warn the viewer off a hand card (`counteredOnPlay`, §10.8).
    pub would_counter: Option<WouldCounterHook>,
    /// Classic #62 Living Bomb, R400: "At the start of your opponent's turn": the other side's `startOfTurn`,
    /// queued right after the active player's at R62's trigger point, so R68's order holds.
    pub start_of_opponent_turn: Option<Hook>,
    // Meditative (R1420)
    /// ME-TRIG, R820, R821 (Meditative #9 Joint Filing): N extra runs of its controller's start- and
    /// end-of-turn hooks, asked as they are queued (`multipliers::extra_runs`); the highest N holds.
    pub turn_hook_extra: Option<TriggerExtraHook>,
    /// ME-TRIG, R822, R823 (Meditative #10 Double Counting): N extra runs of its controller's Cry and Death
    /// effects, asked as a permanent's Cry runs and as a state check's dead have left; the highest N holds.
    pub cry_death_extra: Option<TriggerExtraHook>,
    /// R1487000, R767000: the card's lasting text works while it is set face-down, as R403's first
    /// reading had it. Only a kept script sets it (C #88 Siphon Squad's v0.3.7b), so its replays fold
    /// as they played; every other card's sleeps there (`preview::lasting_text_dormant`).
    pub live_face_down: bool,
    // Card-on-card verbs (#1850)
    /// R1850002: "Exile all …" as a lasting text: while the card acts face-up on the field, every state
    /// check exiles each card on the field each entry reaches (`card_on_card::exile_by_auras`).
    pub exile_auras: Vec<ExileAura>,
    /// R1850005: "When this has N or more keywords, make it Radiant" — the N, read at every state check
    /// against the card's distinct keyword kinds through the layers (`card_on_card::radiant_by_keywords`).
    pub radiant_at_keywords: Option<i32>,
    // Stinkyscape's trade cards (#1864)
    /// R1864004: the Trades a turn this card gives its controller while it acts on their field, read by
    /// `trade::trades_per_turn_of`; several cards do not add, the most holds.
    pub trades_per_turn: Option<TradesPerTurnHook>,
}

/// R1850002: one exile aura: the cards on the field it reaches (`scope`, read as an enemy's harm reads
/// it, so Stealth and Immune keep theirs), the cost they must pass when set (`cost`, as R66's sweeps read
/// it) and the keyword kinds that spare a card through the layers (`not_keywords`: Scornboi's Starship
/// Piece and Starship, R1852000, R1852003). The card itself and a face-down card are never reached.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ExileAura {
    pub scope: crate::effects::BoardScope,
    pub cost: Option<crate::effects::CostFilter>,
    pub not_keywords: Vec<crate::wire::KeywordKind>,
}

impl Script {
    /// The effect-list hook a string names (a work step, a trigger point, `Resume.hook`), by its field's
    /// camelCase name; `None` for any other name or an absent hook.
    pub fn hook_named(&self, name: &str) -> Option<&Hook> {
        match name {
            "cry" => self.cry.as_ref(),
            "death" => self.death.as_ref(),
            "startOfGame" => self.start_of_game.as_ref(),
            "entersHand" => self.enters_hand.as_ref(),
            "fusedInto" => self.fused_into.as_ref(),
            "delayed" => self.delayed.as_ref(),
            "startOfTurn" => self.start_of_turn.as_ref(),
            "endOfTurn" => self.end_of_turn.as_ref(),
            "onPlayHook" => self.on_play_hook.as_ref(),
            "whenAttacks" => self.when_attacks.as_ref(),
            "afterAttack" => self.after_attack.as_ref(),
            "afterAttacked" => self.after_attacked.as_ref(),
            "evolve" => self.evolve.as_ref(),
            "allyAttacked" => self.ally_attacked.as_ref(),
            "allyAttack" => self.ally_attack.as_ref(),
            "startOfOpponentTurn" => self.start_of_opponent_turn.as_ref(),
            _ => None,
        }
    }

    /// The static flags, all unset when the script declares none.
    pub fn flags(&self) -> StaticFlags {
        self.static_flags.clone().unwrap_or_default()
    }
}

/// `ActivationDecl.uses`: "Activate" (1), "Activate N" (N) or "Activate ♾️" ("unlimited").
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1841000: "has an extra turn cooldown": the ability can't be used on its user's next turn
    /// after a use. Absent: no cooldown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_turn_cooldown: Option<bool>,
    /// The declared number (`params`) the mana price reads, so a Buffed price reaches the cost
    /// (B3.4). Skipped in serialization: data, never JSON. Absent: the price is `mana` as written.
    #[serde(skip)]
    pub mana_param: Option<&'static str>,
}

/// B3.2, R384: one Activate ability. "unlimited" `uses` is bounded by `ACTIVATE_UNLIMITED_CAP`, and the
/// tuning key "Activate" moves a number (B3.4). `cost` may be mana, a random discard (Classic #15), a
/// Tribute (Classic #21) or the card itself (Classic #84); `targets` and `modes` travel as a play's do
/// (R81); `canActivate` is a pure read (Classic #7). `cost.tribute` counts units, one each (a Sheep Token's
/// "worth 2" counts only toward a play's Tribute X, §6.3); `tributeSelf` bypasses Indestructible as every
/// Sacrifice does.
#[derive(Clone)]
pub struct ActivationDecl {
    pub id: String,
    pub label: String,
    pub uses: ActivationUses,
    pub cost: Option<ActivationCost>,
    pub targets: Vec<TargetDecl>,
    pub modes: Vec<ModeDecl>,
    pub can_activate: Option<ConditionHook>,
    /// Whether the card has this ability now, when that depends on the instance: one it lacks is neither
    /// listed, shown nor accepted (Heroic Power has only the power it rolled, B3.2 rule 10). Absent: always.
    pub has: Option<HasHook>,
    pub run: Hook,
}

/// R384: the `Resume.hook` an ability's effect list runs under (`activation:<id>`), so a tail a prompt
/// parks comes back to it (`work::script_step_for`). Distinct from every `Script` key.
pub const ACTIVATION_HOOK_PREFIX: &str = "activation:";

pub fn activation_hook(id: &str) -> String {
    format!("{ACTIVATION_HOOK_PREFIX}{id}")
}

/// R384, R102: a face's abilities in its order, ids made unique (a fusion's second `power` is `power#2`),
/// so the `activate` action, the view and a resumed tail all name the same one.
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DrawLimit {
    pub player: DrawLimitPlayer,
    pub count: i32,
}

pub type DrawLimitHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<DrawLimit> + Send + Sync>;

pub type DiscardGuardHook = Arc<dyn for<'a> Fn(HookArgs<'a>) -> Vec<DrawLimitPlayer> + Send + Sync>;

/// `TargetCheck`'s argument. `candidate` is the card a declaration would offer (`None` for a hero or a
/// zone), `self_` the card declaring it, `player` the chooser.
#[derive(Clone, Copy)]
pub struct TargetCheckArgs<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    pub player: PlayerId,
    pub radiant: bool,
    pub candidate: Option<&'a CardInstance>,
    pub selection: &'a Selection,
}

/// §10.6: a card-specific target predicate (`TargetFilter.check`). A pure read.
pub type TargetCheck = Arc<dyn for<'a> Fn(TargetCheckArgs<'a>) -> bool + Send + Sync>;

pub fn target_check(f: impl for<'a> Fn(TargetCheckArgs<'a>) -> bool + Send + Sync + 'static) -> TargetCheck {
    Arc::new(f)
}

// B5 E5, E9 (damage and combat): what `Script.replacements` holds, above all the "targeted" one every
// targeting answers through (`replacements::answer_targeting`).

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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DyingUnit {
    pub instance_id: String,
    pub controller: PlayerId,
    /// R1875006 (Stinkyscape #138 Pocket Peptides, "would die in combat"): a hit killed it (R42) and
    /// the last hit it took in this action was combat damage (§4.3, Cleave's included). Only ever
    /// `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<bool>,
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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

/// What a replacement's `when` reads: a PURE READ.
#[derive(Clone, Copy)]
pub struct ReplacementContext<'a> {
    pub state: &'a GameState,
    pub self_: &'a CardInstance,
    /// The card's controller: "your" in its text (§8 Conventions).
    pub controller: PlayerId,
    pub radiant: bool,
    /// Always the moment the declaration is `on`.
    pub event: &'a ReplacedEvent,
}

/// `ReplacementDef.when`.
pub type ReplacementWhen = Arc<dyn for<'a> Fn(ReplacementContext<'a>) -> bool + Send + Sync>;

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
/// data, so a card may write it as a literal (`json_as(json!({ "to": "exile" }))`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// R1704001 (on `lethalHit`): only ever `Some(true)`, the hit is canceled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
    /// R1875006 (on `wouldDie`): only ever `Some(true)`. Nothing is flickered: the card's `then`
    /// resolves at once, before any card moves, and the state check starts over.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolve_first: Option<bool>,
}

/// One replacement a card makes. The moment fixes what `instead` may say:
///  - `lethalHit`: `redirect: "enemyHero"` (E9) moves the hit, as a new instance, through the enemy hero's
///    Armor and caps; it and `cancel` (Classic #52, R1704001) answer only for its own hero, and
///    `redirect: "self"` (R1204) for its controller's hero and other Units, once each per hit.
///  - `healed`: `damage: "pierce"` turns the heal into Pierce damage from this card (E8), and
///    `lasting: "thisTurn"` every later heal on its enemies this turn; answers only a heal on its enemies.
///  - `wouldDie`: `flicker: "yours"` (E22) sends its controller's dying units back to their zones at once,
///    reset and summoning sick, no Death, no Reborn; or `resolveFirst` (#138, R1875006, R460).
///  - `toGraveyard`: `to`, exile or the bottom of the owner's library.
///  - `targeted`: `interpose: true` summons this from its controller's hand to the leftmost open unit zone
///    (none: nothing), summoning sick, and the attack or pick moves to it; answers only its controller's
///    unit targeted by the other player, and `by` narrows it (Classic #33 Joro, R651).
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
    /// R651 (on `targeted` only): when present, it answers only a targeting by this kind of card.
    pub by: Option<ReplacementBy>,
}

/// B5 E5, E9: THE "a friendly unit is targeted" declaration (Classic #33 Joro): `on` is `Targeted`.
pub type TargetedReplacement = ReplacementDef;

// B5 E33, R404: quests, the data `Script.quests` holds.

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
///
/// R1854002, the staged quests' goals (Stinkyscape #7 The Greatest Heist, #112 Job Application):
///
/// - `unitsMadeRadiant`: Units of yours with `tag` (any Unit when absent) made Radiant, wherever they
///   are: each `radiantSet` reported on one.
/// - `unitsDied`: Units with `tag` (any when absent) that died under your control (R172).
/// - `cardsStolen`: enemy cards you stole, into your hand or onto your side of the field.
/// - `playCosting`: a card of yours played whose cost paid (R56, a cast's 0 included, R70) is `cost`,
///   or the number the quest card remembers under `remembered` while it holds one (goal 1).
/// - `differentCostsInTurn`: cards of yours played in one turn at `count` different costs paid.
/// - `damageInstance`: one damage instance of at least `amount` that your hero took or a card or hero
///   of yours dealt (goal 1).
/// - `byEffect`: nothing counts it; only an effect completes it (R1854001: the card's own text, or
///   "complete the current stage on all your Quests"), goal 1.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum QuestGoal {
    Draws {
        count: i32,
    },
    EnemyPermanentsDestroyed {
        count: i32,
    },
    UnspentManaAtTurnEnd {
        mana: i32,
    },
    DamageToEnemies {
        amount: i32,
    },
    CardsExiled {
        count: i32,
    },
    DeckEmptiedByDraw,
    PermanentsControlled {
        count: i32,
    },
    UnitTotals {
        total: i32,
    },
    UnitsInGraveyard {
        count: i32,
    },
    UnitsMadeRadiant {
        count: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        tag: Option<Tag>,
    },
    UnitsDied {
        count: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        tag: Option<Tag>,
    },
    CardsStolen {
        count: i32,
    },
    PlayCosting {
        cost: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        remembered: Option<String>,
    },
    DifferentCostsInTurn {
        count: i32,
    },
    DamageInstance {
        amount: i32,
    },
    ByEffect,
}

/// One quest: its text as the view shows it, what completes it, and the rewards it offers, in order.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct QuestDef {
    pub id: String,
    pub text: String,
    pub goal: QuestGoal,
    pub rewards: Vec<String>,
}

/// One reward: its text, and the quest it opens next (null at the end of a line).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct QuestRewardDef {
    pub id: String,
    pub text: String,
    pub next: Option<String>,
}

/// A card's quest tree (`Script.quests`): the quest that opens as it enters, every quest, every reward.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct QuestBook {
    pub first: String,
    pub quests: Vec<QuestDef>,
    pub rewards: Vec<QuestRewardDef>,
}

/// R1854000: one stage of a staged quest (`Script.stages`): a single goal opens as is, one of several is
/// drawn from the match rng as it opens (Job Application's Interview). The reward is the card's own
/// effects, from its trigger on its own `questCompleted` (`subsystems::completed_stage_of`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct QuestStage {
    pub name: String,
    pub goals: Vec<QuestStageGoal>,
    pub reward: QuestStageReward,
}

/// R1854000: one goal a stage may set: its quest id (unique in the card's stages), its text as the
/// view shows it and what completes it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct QuestStageGoal {
    pub id: String,
    pub text: String,
    pub goal: QuestGoal,
}

/// R1854000: a stage's reward as the view shows it: an id and its text.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct QuestStageReward {
    pub id: String,
    pub text: String,
}

/// R1854000: the quest book a staged quest's line runs on: every stage's goals as quests, each offering
/// its stage's reward, none naming a next quest, since the card's reward effects move the line on
/// (`effects::next_stage`, `effects::hear_back`, `effects::open_stage`).
pub fn staged_book(stages: &[QuestStage]) -> QuestBook {
    QuestBook {
        first: stages
            .first()
            .and_then(|stage| stage.goals.first())
            .map(|goal| goal.id.clone())
            .unwrap_or_default(),
        quests: stages
            .iter()
            .flat_map(|stage| {
                stage.goals.iter().map(|goal| QuestDef {
                    id: goal.id.clone(),
                    text: goal.text.clone(),
                    goal: goal.goal.clone(),
                    rewards: vec![stage.reward.id.clone()],
                })
            })
            .collect(),
        rewards: stages
            .iter()
            .map(|stage| QuestRewardDef {
                id: stage.reward.id.clone(),
                text: stage.reward.text.clone(),
                next: None,
            })
            .collect(),
    }
}

/// A card's two scripts (SURFACE §7.1): the base face's and the radiant face's.
#[derive(Clone, Default)]
pub struct CardScripts {
    pub base: Script,
    pub radiant: Script,
}

/// A script with no hook, no declaration and no flag.
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
