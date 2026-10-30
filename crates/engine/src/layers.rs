//! Stat and keyword layers (SPEC §10.4). Always computed on read, never stored.

use serde::{Deserialize, Serialize};

use crate::brittle_count::active_brittle_count;
use crate::catalog::def_of;
use crate::config::{BACKROW_ZONES, FIENDER_STATS_MODE, RADIANT_FALLBACK_FACTOR, UNIT_ZONES};
use crate::script::{AuraEntry, AuraHook, HookArgs, StatMod};
use crate::state::{CardInstance, GameState, ModifierKind, Position};
use crate::tuning::{tuned_keywords, x_of};
use crate::wire::{
    AttackHealth, CardDef, CardFace, CardType, Keyword, KeywordKind, PLAYER_IDS, PlayerId, Row, Zone,
    armor_of, has_keyword,
};

/// §10.4: a unit's layered stats and keywords (`unit_view`). Not the view's `wire::UnitView`, which
/// `view_for` builds from this; `lib.rs` resolves the root name to this one.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    pub attack: i32,
    pub max_health: i32,
    pub health: i32,
    pub keywords: Vec<Keyword>,
    pub armor: i32,
    pub position: Position,
}

/// `face_of`'s answer: the printed face as the card's lasting changes leave it.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FaceStats {
    pub attack: i32,
    pub health: i32,
    pub keywords: Vec<Keyword>,
}

/// `stats_with_buffs`' answer: §10.4 layers 1 to 4, unfloored.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuffedStats {
    pub attack: i32,
    pub max_health: i32,
}

/// R349: what a summon's X/X (`statsOverride`) comes to on the face the instance wears. A card that
/// prints no Radiant form of its own (`radiantFallback`, the Ghoul Token) is, made Radiant, its base
/// face with its attack and health doubled; one that prints a Radiant form keeps its X on both (§7).
pub fn worn_stats_override(def: &CardDef, instance: &CardInstance) -> Option<AttackHealth> {
    let stats = instance.stats_override?;
    if !instance.radiant || def.radiant_fallback != Some(true) {
        return Some(stats);
    }
    Some(AttackHealth {
        attack: RADIANT_FALLBACK_FACTOR * stats.attack,
        health: RADIANT_FALLBACK_FACTOR * stats.health,
    })
}

/// B2.7: a face that prints its stats as multiples of X (Classic+ #69 Buff Billy's "[3X/3X]") is those
/// multiples of the X the card was played for, tuned (`tuning::x_of`), and 0/0 when it has none. A
/// summon's `statsOverride` still wins, as it wins over any printed stats.
fn x_stats_of(face: &CardFace, instance: &CardInstance) -> Option<AttackHealth> {
    let multiples = face.x_stats?;
    let x = x_of(instance);
    Some(AttackHealth {
        attack: multiples.attack * x,
        health: multiples.health * x,
    })
}

/// The card's printed face, radiant when the instance is (§5.2), with B2.7's X stats and B3.4's
/// `tuning` on top. Tuning is part of the card, kept in every zone (R78 leaves it alone), so this is the
/// face a card in a hand or a deck will enter with (B3.4 rule 6) as well as layer 1 on the field.
pub fn face_of(state: &GameState, instance: &CardInstance) -> FaceStats {
    let def = def_of(Some(state), &instance.def_id);
    let face = if instance.radiant { &def.radiant } else { &def.base };
    let stats = worn_stats_override(def, instance).or_else(|| x_stats_of(face, instance));
    let keywords = tuned_keywords(&printed_keywords_of(state, instance), instance);
    let tuning_attack = instance
        .tuning
        .as_ref()
        .and_then(|tuning| tuning.attack)
        .unwrap_or(0);
    let tuning_health = instance
        .tuning
        .as_ref()
        .and_then(|tuning| tuning.health)
        .unwrap_or(0);
    let attack = stats.map(|stats| stats.attack).or(face.attack).unwrap_or(0);
    let health = stats.map(|stats| stats.health).or(face.health).unwrap_or(0);
    FaceStats {
        attack: attack + tuning_attack,
        health: health + tuning_health,
        keywords,
    }
}

/// R2280000: whether the card has a stat line its definition gives it: its own printed one or a fused
/// definition's (R77, R102), a summon's `statsOverride` or X stats. Buffs, auras and `tuning` never
/// make one, and a Field Spell, Trap or Field Trap animates only with one (`animated::animated_with_stats`).
pub fn has_stat_line(state: &GameState, instance: &CardInstance) -> bool {
    let def = def_of(Some(state), &instance.def_id);
    let face = if instance.radiant { &def.radiant } else { &def.base };
    instance.stats_override.is_some()
        || face.x_stats.is_some()
        || face.attack.is_some()
        || face.health.is_some()
}

/// The keywords the running face prints, before tuning (§5.2): what a Degrade takes off and what a
/// numbered keyword's tuning steps from (B3.4). §7: the Bread Token's radiant "Armor X" is its X/X's X,
/// so the printed `n` is a placeholder the summon fills in. R1850000: a copier prints the keywords of
/// the text it copied, on its own stats (`face_of`).
pub fn printed_keywords_of(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    let (def_id, radiant) = crate::subsystems::unit_text::text_def_of(instance);
    let face = crate::faces::running_face_of(state, def_id, radiant);
    match instance.armor_override {
        None => face.keywords.clone(),
        Some(n) => face
            .keywords
            .iter()
            .map(|keyword| match keyword {
                Keyword::Armor { .. } => Keyword::Armor { n },
                other => other.clone(),
            })
            .collect(),
    }
}

/// B3.3 rule 5, R385: a Brittle count in force is the card's Brittle, whatever it prints: the list's
/// Brittle entries give way to one `Brittle n` of the count, a given count on a card that printed none
/// included. With no count in force the printed keyword stands.
fn with_brittle_count(keywords: &[Keyword], instance: &CardInstance) -> Vec<Keyword> {
    let Some(count) = active_brittle_count(instance) else {
        return keywords.to_vec();
    };
    let mut out: Vec<Keyword> = keywords
        .iter()
        .filter(|keyword| keyword.kind() != KeywordKind::Brittle)
        .cloned()
        .collect();
    out.push(Keyword::Brittle { n: count });
    out
}

/// §10.4 layers 1 to 4 of the keywords: the printed keywords as tuning leaves them (none on a Vanilla
/// card, §6.3) with the granted ones, as a set (§6.1), and the Brittle count in force: what a card in a
/// hand or a deck is made of (B5 E38, R243).
pub fn card_keywords(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    let mut all: Vec<Keyword> = if instance.vanilla || crate::alt_play::is_dormant(instance) {
        Vec::new()
    } else {
        face_of(state, instance).keywords
    };
    all.extend(instance.granted_keywords.iter().cloned());
    // R2179001: one Temporary, written as the count it has left.
    let all = crate::temporary::with_temporary_count(&all, instance);
    as_set(&with_brittle_count(&all, instance))
}

/// `zones::card_at` for a unit zone: the top of its pile.
fn unit_top(state: &GameState, player: PlayerId, lane: i32) -> Option<&CardInstance> {
    let lane = usize::try_from(lane - 1).ok()?;
    state.players[player].units.get(lane)?.as_ref()?.first()
}

/// §3.2, R13: a card in a unit zone that is not the top of its pile.
fn is_dormant(state: &GameState, instance: &CardInstance) -> bool {
    let Zone::Field {
        player,
        row: Row::Units,
        lane,
    } = instance.zone
    else {
        return false;
    };
    unit_top(state, player, lane).map(|top| top.id.as_str()) != Some(instance.id.as_str())
}

/// R1859007: the keywords its controller's "your Units have X" lends a card (`UnitsHaveKeyword`): to
/// a Unit of theirs in a unit zone or standing in the backrow (R1081000), never a Vanilla one (unlike
/// an aura's grant, §8.10), nor one Dormant or buried under a Stack pile (R13, R1839000). R1870004:
/// only while the modifier is live.
fn lent_by_player(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    if instance.vanilla
        || is_dormant(state, instance)
        || crate::slumber::is_slumbering(instance)
        || !crate::zones::in_units_row_or_standing(state, instance)
    {
        return Vec::new();
    }
    state.players[instance.controller]
        .mods
        .iter()
        .filter_map(|modifier| match &modifier.kind {
            ModifierKind::UnitsHaveKeyword { keyword } if crate::mana::modifier_is_live(state, modifier) => {
                Some(keyword.clone())
            }
            _ => None,
        })
        .collect()
}

/// Every permanent whose aura is in play, in lane order per side (§10.4 layer 5): a face-down card's
/// sleeps (R1487000).
pub(crate) fn aura_sources(state: &GameState) -> Vec<&CardInstance> {
    // Every unit read walks this list, so it is plain loops (#188), in `zones::active_units_of`'s order:
    // each unit zone's top card in lane order, the side's carried Units (R446), then the backrow by lane.
    let mut sources: Vec<&CardInstance> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        // R1839000: a Dormant card's aura is off until it awakens.
        for lane in 1..=UNIT_ZONES {
            if let Some(top) = unit_top(state, player, lane)
                && top.slumber.is_none()
            {
                sources.push(top);
            }
        }
        sources.extend(
            side.carried
                .iter()
                .flatten()
                .flatten()
                .filter(|carried| carried.slumber.is_none()),
        );
        for lane in 1..=BACKROW_ZONES {
            let card = usize::try_from(lane - 1)
                .ok()
                .and_then(|at| side.backrow.get(at))
                .and_then(|card| card.as_ref());
            if let Some(card) = card
                && !crate::preview::lasting_text_dormant(state, card)
                && card.slumber.is_none()
            {
                sources.push(card);
            }
        }
    }
    sources
}

/// Every aura source's entries, in `aura_sources` order: each source's hook asked once.
fn aura_entries(state: &GameState) -> Vec<AuraEntry<'_>> {
    let mut entries: Vec<AuraEntry<'_>> = Vec::new();
    for source in aura_sources(state) {
        // §6.1/§6.3: Vanilla clears scripts, and an aura is part of a card's Script (§10.9), so a
        // Vanilla'd permanent projects nothing. It still *receives* aura grants.
        if source.vanilla {
            continue;
        }
        let script = crate::scripts::script_of(state, source);
        let Some(aura) = script.aura.as_ref() else {
            continue;
        };
        entries.extend(aura(HookArgs {
            state,
            self_: source,
            // R1850000: the face of the text that is running, a copier's copied one.
            radiant: crate::subsystems::unit_text::text_def_of(source).1,
        }));
    }
    entries
}

/// The contributions of `entries` that reach `unit`. An aura's `applies` predicate reads instance
/// data only: it must never call back into `unit_view`, or the layers would recurse.
fn mods_of(entries: &[AuraEntry<'_>], unit: &CardInstance) -> Vec<StatMod> {
    entries
        .iter()
        .filter(|entry| (entry.applies)(unit))
        .map(|entry| entry.mod_.clone())
        .collect()
}

/// Aura contributions for one unit, every source asked for this read alone.
fn aura_mods(state: &GameState, unit: &CardInstance) -> Vec<StatMod> {
    aura_entries(state)
        .into_iter()
        .filter(|entry| (entry.applies)(unit))
        .map(|entry| entry.mod_)
        .collect()
}

/// R1180002 (Meditative+ #7 Life Mage): "Aura: Your Spells have <keyword>." — while the source acts,
/// each Spell its controller is resolving has it: a Spell in that player's resolving zone, so not a
/// Field Spell, a Trap nor a Spell set face-down (R783005). Read wherever the Spell's keywords are, as
/// damage step 8's Lifesteal (R85); the test reads no layer (`faces::card_type_of`), as an aura's must.
pub fn spells_have_aura(keyword: Keyword) -> AuraHook {
    crate::script::aura_hook(move |args| {
        let controller = args.self_.controller;
        let state = args.state;
        vec![AuraEntry {
            applies: Box::new(move |card: &CardInstance| {
                matches!(card.zone, Zone::Resolving { player } if player == controller)
                    && crate::faces::card_type_of(state, card) == CardType::Spell
            }),
            mod_: StatMod {
                keywords: Some(vec![keyword.clone()]),
                ..StatMod::default()
            },
        }]
    })
}

/// #892: layer 5's aura entries for a whole sweep. `new` asks each source's hook once and every unit
/// read through it filters the same entries; `per_read` asks on every read, as `unit_view` does, for
/// a reader that may read nothing. It borrows the state, so the state cannot change while it lives: a
/// derived value, not a cache (CLAUDE.md rule 4).
pub struct AuraTable<'a> {
    state: &'a GameState,
    entries: Option<Vec<AuraEntry<'a>>>,
}

impl<'a> AuraTable<'a> {
    pub fn new(state: &'a GameState) -> AuraTable<'a> {
        AuraTable {
            state,
            entries: Some(aura_entries(state)),
        }
    }

    pub fn per_read(state: &'a GameState) -> AuraTable<'a> {
        AuraTable { state, entries: None }
    }

    pub fn state(&self) -> &'a GameState {
        self.state
    }

    pub fn unit_view(&self, instance: &CardInstance) -> UnitView {
        layered_view(
            instance,
            compute_layers(self.state, instance, self.entries.as_deref()),
        )
    }

    pub fn unit_has(&self, instance: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&self.unit_view(instance).keywords, kind)
    }
}

/// §6.1: a unit's keywords are "computed as a set per unit", so a keyword two sources give is had once
/// and the view hands the client one entry for it (§10.8). Armor sums across its sources (§10.4) and
/// Lucky X stacks, so every entry of theirs is kept.
fn as_set(keywords: &[Keyword]) -> Vec<Keyword> {
    let mut seen: Vec<KeywordKind> = Vec::new();
    keywords
        .iter()
        .filter(|keyword| {
            let kind = keyword.kind();
            // B5 E6: Spell Damage is numbered too, and sums across its sources like Armor.
            if matches!(
                kind,
                KeywordKind::Armor | KeywordKind::Lucky | KeywordKind::SpellDamage
            ) {
                return true;
            }
            if seen.contains(&kind) {
                return false;
            }
            seen.push(kind);
            true
        })
        .cloned()
        .collect()
}

/// B5 E35: a keyword that holds only while a condition does (`conditionalKeywords`), read with the
/// printed ones, so a Vanilla takes it. The hook reads instance data only.
pub(crate) fn conditional_keywords_of(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    let script = crate::scripts::script_of(state, instance);
    match script.conditional_keywords.as_ref() {
        None => Vec::new(),
        Some(hook) => hook(HookArgs {
            state,
            self_: instance,
            radiant: instance.radiant,
        }),
    }
}

pub fn unit_view(state: &GameState, instance: &CardInstance) -> UnitView {
    layered_view(instance, compute_layers(state, instance, None))
}

/// R89, R1884004: a dead card's snapshot (a Death hook's `self_`) with the board's auras as they were on
/// its last stay: a Death that brings it back reads whether it died for want of health.
pub fn unit_view_as_it_died(state: &GameState, instance: &CardInstance) -> UnitView {
    layered_view(instance, compute_layers_on(state, instance, None, true))
}

fn layered_view(instance: &CardInstance, layered: Layered) -> UnitView {
    // Attack floors at 0; max health may fall to 0, which the state check turns into a death (§10.4).
    let clamped_attack = layered.attack.max(0);
    let armor = armor_of(&layered.keywords);
    UnitView {
        attack: clamped_attack,
        max_health: layered.max_health,
        health: layered.max_health - instance.damage,
        keywords: layered.keywords,
        armor,
        position: layered.position,
    }
}

/// §10.4's attack before layer 5's floor at 0, for KY's Constant to set a unit's attack exactly
/// (`effects/tune::set_number`): a delta taken off a −2 shown as 0 would land short.
pub fn unclamped_attack(state: &GameState, instance: &CardInstance) -> i32 {
    compute_layers(state, instance, None).attack
}

struct Layered {
    attack: i32,
    max_health: i32,
    keywords: Vec<Keyword>,
    position: Position,
}

// R39 (`FIENDER_STATS_MODE`): layer 2 adds Felinor Fiender's sum of your Felinors to its printed
// stats, each sum floored at 0, the one mode `compute_layers` plays.
const _: () = assert!(
    matches!(FIENDER_STATS_MODE.as_bytes(), b"printed-plus-sum"),
    "R39: layers.rs's layer 2 plays Felinor Fiender as its printed stats plus the sum of your Felinors"
);

/// §10.4's five layers, attack unfloored: `unit_view` floors it, `unclamped_attack` reads it raw.
/// Layer 5 reads a sweep's aura entries (`AuraTable`) when given them, else asks every source.
fn compute_layers(state: &GameState, instance: &CardInstance, table: Option<&[AuraEntry<'_>]>) -> Layered {
    compute_layers_on(state, instance, table, false)
}

/// `compute_layers`, with `as_it_died` for a card's snapshot as it died (R89): it no longer tops its
/// pile, so the dormancy test does not hold layer 5 off it.
fn compute_layers_on(
    state: &GameState,
    instance: &CardInstance,
    table: Option<&[AuraEntry<'_>]>,
    as_it_died: bool,
) -> Layered {
    let printed = face_of(state, instance);
    let position = instance.position.unwrap_or(Position::Atk);

    // Layers 1 and 3: printed stats of the running face, with Fuse already baked in (R77).
    let mut attack = printed.attack;
    let mut max_health = printed.health;

    // Layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39) adds the sum
    // of your Felinors' layer-4 stats, each sum floored at 0 by R39's "never below printed": the ONE
    // floor (R132, R150), per stat (R116), as the contributors reach the hook unfloored.
    // Vanilla has cleared the card's scripts, so a Vanilla'd body keeps its printed stats (§6.1).
    if !instance.vanilla {
        let script = crate::scripts::script_of(state, instance);
        if let Some(set_stat) = script.set_stat.as_ref() {
            let set = set_stat(HookArgs {
                state,
                self_: instance,
                radiant: instance.radiant,
            });
            attack += set.attack.unwrap_or(0).max(0);
            max_health += set.max_health.unwrap_or(0).max(0);
        }
    }

    // Layer 4: permanent buffs.
    attack += instance.buffs.attack;
    max_health += instance.buffs.health;

    // Layer 5: auras. A card dormant under a Stack pile is "not on the field for effects" (§3.2, R13),
    // so no aura reaches it until it resumes on top (one shrinking max health, #46, would kill it).
    // R1839000: no aura reaches a Dormant card either.
    let auras: Vec<StatMod> =
        if (!as_it_died && is_dormant(state, instance)) || crate::slumber::is_slumbering(instance) {
            Vec::new()
        } else {
            match table {
                Some(entries) => mods_of(entries, instance),
                None => aura_mods(state, instance),
            }
        };
    for modifier in &auras {
        attack += modifier.attack.unwrap_or(0);
        max_health += modifier.max_health.unwrap_or(0);
    }
    // §10.4: an aura that sets attack to a value (Classic #88's Radiant "0 Attack") applies after every
    // other layer, so nothing above lifts it; with several, the last in aura order holds.
    for modifier in &auras {
        if let Some(set_attack) = modifier.set_attack {
            attack = set_attack;
        }
    }

    let mut listed: Vec<Keyword> = Vec::new();
    if !instance.vanilla && !crate::alt_play::is_dormant(instance) {
        listed.extend(printed.keywords.iter().cloned());
    }
    listed.extend(conditional_keywords_of(state, instance));
    listed.extend(instance.granted_keywords.iter().cloned());
    for modifier in &auras {
        listed.extend(modifier.keywords.iter().flatten().cloned());
    }
    listed.extend(lent_by_player(state, instance));
    let mut keywords: Vec<Keyword> = with_brittle_count(&listed, instance);

    // Position grants: Defense adds Taunt and Armor +1 (§4.1).
    if position == Position::Def {
        keywords.push(Keyword::Taunt);
        keywords.push(Keyword::Armor { n: 1 });
    }

    // R46: an Indestructible unit that would have been destroyed loses Taunt for the turn. R347: an
    // Indestructible unit never has Taunt, from any source. A spent Divine Shield and a used Reborn
    // are gone until granted again (§6.1, §4.5 step 4).
    let taunt_suppressed = instance.taunt_suppressed_turn == Some(state.turn)
        || keywords
            .iter()
            .any(|keyword| keyword.kind() == KeywordKind::Indestructible);
    let shield_spent = instance.divine_shield_spent == Some(true);
    let reborn_spent = instance.reborn_spent == Some(true);
    // R783013: a used Flashback is gone for good, printed or given.
    let flashback_spent = instance.flashback_spent == Some(true);
    let kept: Vec<Keyword> = keywords
        .into_iter()
        .filter(|keyword| {
            let kind = keyword.kind();
            !(taunt_suppressed && kind == KeywordKind::Taunt)
                && !(shield_spent && kind == KeywordKind::DivineShield)
                && !(reborn_spent && kind == KeywordKind::Reborn)
                && !(flashback_spent && kind == KeywordKind::Flashback)
        })
        .collect();
    let final_keywords = as_set(&kept);

    Layered {
        attack,
        max_health,
        keywords: final_keywords,
        position,
    }
}

/// §10.4 layers 1 to 4: printed stats plus permanent buffs, before auras — the reading a set-stat
/// layer that sums other units needs (#92 Felinor Fiender, R116). R150: no per-unit floor, or a
/// Felinor with a −5 buff would add 0, not −2, and never pull the sum toward 0 as R132 requires. The
/// floor is each reader's: layer 2's total, `unit_view`'s attack, `view_for`'s hand cards (R243).
pub fn stats_with_buffs(state: &GameState, instance: &CardInstance) -> BuffedStats {
    let printed = face_of(state, instance);
    BuffedStats {
        attack: printed.attack + instance.buffs.attack,
        max_health: printed.health + instance.buffs.health,
    }
}

pub fn keywords_of(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    unit_view(state, instance).keywords
}

pub fn unit_has(state: &GameState, instance: &CardInstance, kind: KeywordKind) -> bool {
    has_keyword(&unit_view(state, instance).keywords, kind)
}
