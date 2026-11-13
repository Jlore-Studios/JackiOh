//! Burn and Toxic (SPEC §6.3, Stinkyscape+ E2; R3540000–R3540004): the two counters a Unit or a hero
//! carries (`CardInstance.counters`, `PlayerState.hero_burn` and `hero_toxic`), and everything that
//! touches them, so no caller can disagree about them:
//!   - a gain (`afflict`): the verbs, the "Damage also Burns N / applies Toxic N" riders and the
//!     conversion all add to the count already there (R3540000), reported as `counterChanged` with
//!     `placed`;
//!   - the tick (`tick_at_turn_start`): at the start of its owner's turn each count deals its damage,
//!     Burn's unpierced and then dropping by `BURN_DECAY`, Toxic's piercing and lasting (R3540001);
//!   - the riders and the conversion (`ride_damage`, `converts_to`), which §4.4 reads off the source's
//!     static flags (R3540002, R3540003);
//!   - a Vanilla's clearing (`clear_afflictions`, R3540004), and the reads, re-exported by `query.rs`.
//!
//! Only a Unit acting in a unit zone, or standing in the backrow, or a hero carries them; R78's reset
//! takes a Unit's off as it leaves the field. Neither is a keyword: the layers never see them, so a
//! Nerf or a Buff cannot move them.

use crate::config::BURN_DECAY;
use crate::damage::{DamageArgs, DamageFlags, DamageTarget, deal_damage};
use crate::script::EngineSink;
use crate::state::{CardInstance, GameState, find_instance, find_instance_mut};
use crate::wire::{CounterKind, GameEvent, KeywordKind, PlayerId};
use crate::zones::StandingAt;

/// The declared number (R386) a card's "Damage also Burns N" reads its N from.
pub const BURNS_PARAM: &str = "burns";
/// The declared number (R386) a card's "Damage also applies Toxic N" reads its N from.
pub const TOXIC_PARAM: &str = "toxic";

/// The two counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Affliction {
    Burn,
    Toxic,
}

impl Affliction {
    /// The `counterChanged.counter` that reports it.
    pub fn counter(self) -> CounterKind {
        match self {
            Affliction::Burn => CounterKind::Burn,
            Affliction::Toxic => CounterKind::Toxic,
        }
    }
}

/// The Burn a Unit carries now; an untouched one carries none.
pub fn burn_on(card: &CardInstance) -> i32 {
    card.counters.burn.unwrap_or(0).max(0)
}

/// The Toxic a Unit carries now; an untouched one carries none.
pub fn toxic_on(card: &CardInstance) -> i32 {
    card.counters.toxic.unwrap_or(0).max(0)
}

/// The Burn on `player`'s hero.
pub fn hero_burn_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero_burn.unwrap_or(0).max(0)
}

/// The Toxic on `player`'s hero.
pub fn hero_toxic_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player].hero_toxic.unwrap_or(0).max(0)
}

/// The count `target` carries now: a Unit is read live under its id (0 once it is gone), never as the
/// instance its caller held.
fn counter_on(state: &GameState, target: &DamageTarget, kind: Affliction) -> i32 {
    match (target, kind) {
        (DamageTarget::Unit { instance }, Affliction::Burn) => {
            find_instance(state, &instance.id).map_or(0, burn_on)
        }
        (DamageTarget::Unit { instance }, Affliction::Toxic) => {
            find_instance(state, &instance.id).map_or(0, toxic_on)
        }
        (DamageTarget::Hero { player }, Affliction::Burn) => hero_burn_of(state, *player),
        (DamageTarget::Hero { player }, Affliction::Toxic) => hero_toxic_of(state, *player),
    }
}

/// Store `value` as `target`'s count, absent at 0 or less so a game without one hashes as before.
fn store(state: &mut GameState, target: &DamageTarget, kind: Affliction, value: i32) {
    let stored = (value > 0).then_some(value);
    match (target, kind) {
        (DamageTarget::Unit { instance }, Affliction::Burn) => {
            if let Some(card) = find_instance_mut(state, &instance.id) {
                card.counters.burn = stored;
            }
        }
        (DamageTarget::Unit { instance }, Affliction::Toxic) => {
            if let Some(card) = find_instance_mut(state, &instance.id) {
                card.counters.toxic = stored;
            }
        }
        (DamageTarget::Hero { player }, Affliction::Burn) => state.players[*player].hero_burn = stored,
        (DamageTarget::Hero { player }, Affliction::Toxic) => state.players[*player].hero_toxic = stored,
    }
}

/// R3540000: set `target`'s count to `value` and report it, with no `placed`: a tick's drop and a
/// Vanilla's clearing are no gain. A change to nothing reports nothing, and a Unit that is gone has no
/// count to set.
pub(crate) fn set_counter(sink: &mut EngineSink<'_>, target: &DamageTarget, kind: Affliction, value: i32) {
    let value = value.max(0);
    if let DamageTarget::Unit { instance } = target
        && find_instance(sink.state, &instance.id).is_none()
    {
        return;
    }
    if value == counter_on(sink.state, target, kind) {
        return;
    }
    store(sink.state, target, kind, value);
    sink.events.push(GameEvent::CounterChanged {
        instance_id: crate::damage::target_id(target),
        counter: kind.counter(),
        value,
        placed: None,
    });
}

/// R3540000: add `amount` to `target`'s `kind` count, reported once as `counterChanged` with `placed`.
/// Returns what went on — 0, with nothing changed or reported, for an amount that is not positive, a
/// finished game, or a Unit that is gone or acts neither in a unit zone nor standing in the backrow.
pub fn afflict(sink: &mut EngineSink<'_>, target: &DamageTarget, kind: Affliction, amount: i32) -> i32 {
    if amount <= 0 || sink.state.result.is_some() {
        return 0;
    }
    if let DamageTarget::Unit { instance } = target {
        let carries = find_instance(sink.state, &instance.id).is_some_and(|live| {
            crate::zones::acts_on_field(sink.state, live)
                && crate::zones::in_units_row_or_standing(sink.state, live)
        });
        if !carries {
            return 0;
        }
    }
    let value = counter_on(sink.state, target, kind).saturating_add(amount);
    store(sink.state, target, kind, value);
    sink.events.push(GameEvent::CounterChanged {
        instance_id: crate::damage::target_id(target),
        counter: kind.counter(),
        value,
        placed: Some(amount),
    });
    amount
}

/// R3540004: a Vanilla takes Burn and then Toxic off its Unit, as it takes Frozen off.
pub(crate) fn clear_afflictions(sink: &mut EngineSink<'_>, card_id: &str) {
    let Some(live) = find_instance(sink.state, card_id).cloned() else {
        return;
    };
    let target = DamageTarget::Unit { instance: live };
    set_counter(sink, &target, Affliction::Burn, 0);
    set_counter(sink, &target, Affliction::Toxic, 0);
}

/// R3540003, §4.4 step 3a: what the damage `source` deals turns into, if it states a conversion. A source
/// that states both burns.
pub(crate) fn converts_to(state: &GameState, source: &CardInstance) -> Option<Affliction> {
    let flags = crate::scripts::flags_of(state, source);
    if flags.damage_burns == Some(true) {
        Some(Affliction::Burn)
    } else if flags.damage_is_toxic == Some(true) {
        Some(Affliction::Toxic)
    } else {
        None
    }
}

/// R3540002, §4.4 step 7: the Burn and the Toxic a hit from `source` adds to its target after it landed,
/// when the target survived it: a Unit still standing and not killed, a hero with life left. N is the
/// source's declared number where it declares one (R386), the flag's otherwise; Burn goes on first.
pub(crate) fn ride_damage(sink: &mut EngineSink<'_>, source: &CardInstance, target: &DamageTarget) {
    let flags = crate::scripts::flags_of(sink.state, source);
    let burn = flags
        .burns_on_damage
        .map(|n| crate::params::declared_or(sink.state, source, BURNS_PARAM, n));
    let toxic = flags
        .toxic_on_damage
        .map(|n| crate::params::declared_or(sink.state, source, TOXIC_PARAM, n));
    if burn.is_none() && toxic.is_none() {
        return;
    }
    let survived = match target {
        DamageTarget::Unit { instance } => find_instance(sink.state, &instance.id)
            .is_some_and(|live| !crate::damage::already_killed(sink.state, live)),
        DamageTarget::Hero { player } => crate::duos_life::effective_life(sink.state, *player) > 0,
    };
    if !survived {
        return;
    }
    if let Some(n) = burn {
        afflict(sink, target, Affliction::Burn, n);
    }
    if let Some(n) = toxic {
        afflict(sink, target, Affliction::Toxic, n);
    }
}

/// R3540001: the start of `player`'s turn, in the Brittle stage: their hero first, then their Units in
/// R68's order, each takes its Burn as a sourceless §4.4 hit (so Armor and Divine Shield meet it), which
/// then drops by `BURN_DECAY` whatever the hit did, and then its Toxic as a piercing one, which stays.
/// The targets are read before the first hit, and no state check runs inside the tick (R59): the stage's
/// settle collects what it killed.
pub fn tick_at_turn_start(sink: &mut EngineSink<'_>, player: PlayerId) {
    if sink.state.result.is_some() {
        return;
    }
    let mut targets = vec![DamageTarget::Hero { player }];
    targets.extend(
        crate::zones::acting_on(sink.state, player, StandingAt::AmongUnits)
            .into_iter()
            .filter(|card| burn_on(card) > 0 || toxic_on(card) > 0)
            .cloned()
            .map(|instance| DamageTarget::Unit { instance }),
    );
    for target in targets {
        let burn = counter_on(sink.state, &target, Affliction::Burn);
        if burn > 0 {
            deal_damage(
                sink,
                DamageArgs {
                    source: None,
                    target: target.clone(),
                    amount: burn,
                    flags: Some(DamageFlags {
                        via: Some(KeywordKind::Burn),
                        ..Default::default()
                    }),
                },
            );
            set_counter(sink, &target, Affliction::Burn, (burn - BURN_DECAY).max(0));
        }
        let toxic = counter_on(sink.state, &target, Affliction::Toxic);
        if toxic > 0 {
            deal_damage(
                sink,
                DamageArgs {
                    source: None,
                    target: target.clone(),
                    amount: toxic,
                    flags: Some(DamageFlags {
                        ignore_armor: Some(true),
                        via: Some(KeywordKind::Toxic),
                        ..Default::default()
                    }),
                },
            );
        }
    }
}
