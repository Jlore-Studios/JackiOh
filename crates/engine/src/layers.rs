//! Stat and keyword layers (SPEC §10.4). Always computed on read, never stored.
//!
//! Port of `packages/engine/src/layers.ts`: the same order of aura sources (each side's acting units
//! in lane order, its carried Units, then its backrow by lane) and of stat mods as TS.

use serde::{Deserialize, Serialize};

use crate::brittle_count::active_brittle_count;
use crate::catalog::def_of;
use crate::config::{ANIMATED_FALLBACK_ATTACK, ANIMATED_FALLBACK_HEALTH, BACKROW_ZONES, RADIANT_FALLBACK_FACTOR, UNIT_ZONES};
use crate::script::{HookArgs, Script, StatMod, empty_script};
use crate::state::{CardInstance, GameState, Position};
use crate::tuning::{tuned_keywords, x_of};
use crate::wire::{AttackHealth, CardDef, CardFace, Keyword, KeywordKind, PLAYER_IDS, PlayerId, Row, Zone, armor_of, has_keyword};

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

/// TS `scripts.scriptOf(instance)`: the face that is running, and no script at all on a Vanilla card
/// (§6.3, R115). A private copy over `scripts::script_of` (SURFACE §6.6).
fn running_script(state: &GameState, instance: &CardInstance) -> Script {
    if instance.vanilla {
        return empty_script();
    }
    let entry = crate::scripts::script_of(state, &instance.def_id);
    if instance.radiant { entry.radiant } else { entry.base }
}

/// R349: what a summon's X/X (`statsOverride`) comes to on the face the instance wears. A card that
/// prints no Radiant form of its own (`radiantFallback`, the Ghoul Token) is, made Radiant, its base
/// face with its attack and health doubled, and its X/X is that base face — so a Radiant Ghoul
/// summoned 3/3 is a 6/6. A card that prints a Radiant form keeps its X on both faces, as the Bread
/// Token's "X/X, Armor X" and an X/X Rush Token do (§7).
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
/// multiples of the X the card was played for, tuned (`tuning::x_of`), and 0/0 when it has none — a
/// Recruit, a copy, a card outside a play — as the Ghoul Token's X/X is with no X (§7). A summon's
/// `statsOverride` still wins, as it wins over any printed stats.
fn x_stats_of(face: &CardFace, instance: &CardInstance) -> Option<AttackHealth> {
    let multiples = face.x_stats?;
    let x = x_of(instance);
    Some(AttackHealth {
        attack: multiples.attack * x,
        health: multiples.health * x,
    })
}

/// The card's printed face, radiant when the instance is (§5.2), as the card's lasting changes leave
/// it: B2.7's X stats, and B3.4's `tuning` — the stats changes on top of the printed stats and the
/// keyword changes on the printed keywords (`tuning::tuned_keywords`). Tuning is part of the card, kept
/// in every zone (R78 leaves it alone), so this is the face a card in a hand or a deck will enter with
/// (B3.4 rule 6) as well as layer 1 on the field.
pub fn face_of(state: &GameState, instance: &CardInstance) -> FaceStats {
    let def = def_of(Some(state), &instance.def_id);
    let face = if instance.radiant { &def.radiant } else { &def.base };
    let stats = worn_stats_override(def, instance).or_else(|| x_stats_of(face, instance));
    let keywords = tuned_keywords(&printed_keywords_of(state, instance), instance);
    // R657: an Animated card with no stats from any source (no printed stats, no statsOverride, no X
    // stats) fights as a 0/1 instead of dying as a 0/0 at the state check.
    let statless = stats.is_none() && face.attack.is_none() && face.health.is_none();
    let animated = statless
        && keywords
            .iter()
            .chain(instance.granted_keywords.iter())
            .any(|keyword| matches!(keyword, Keyword::Animated | Keyword::AnimatedOnYourTurn));
    let tuning_attack = instance.tuning.as_ref().and_then(|tuning| tuning.attack).unwrap_or(0);
    let tuning_health = instance.tuning.as_ref().and_then(|tuning| tuning.health).unwrap_or(0);
    let attack = stats
        .map(|stats| stats.attack)
        .or(face.attack)
        .unwrap_or(if animated { ANIMATED_FALLBACK_ATTACK } else { 0 });
    let health = stats
        .map(|stats| stats.health)
        .or(face.health)
        .unwrap_or(if animated { ANIMATED_FALLBACK_HEALTH } else { 0 });
    FaceStats {
        attack: attack + tuning_attack,
        health: health + tuning_health,
        keywords,
    }
}

/// The keywords the running face prints, before tuning (§5.2): what a Degrade takes a printed keyword
/// off and what a numbered keyword's tuning steps from (B3.4). §7: the Bread Token's radiant "Armor X"
/// is the same X as its X/X, so the printed `n` is a placeholder the summon fills in, exactly as
/// `statsOverride` fills in the printed 0/0.
pub fn printed_keywords_of(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    let def = def_of(Some(state), &instance.def_id);
    let face = if instance.radiant { &def.radiant } else { &def.base };
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
/// included. With no count in force (a printed Brittle that has not started, off the field) the
/// printed keyword stands.
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

/// §10.4 layers 1 to 4 of the keywords, before auras and position: the printed keywords as tuning
/// leaves them (none on a Vanilla card, §6.3) with the granted ones, as a set (§6.1), and the Brittle
/// count in force. What a card in a hand or a deck is made of (B5 E38, R243) — the granted keywords it
/// gained there ride onto the field with it — and what a Degrade may take off it (B3.4).
pub fn card_keywords(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    let mut all: Vec<Keyword> = if instance.vanilla {
        Vec::new()
    } else {
        face_of(state, instance).keywords
    };
    all.extend(instance.granted_keywords.iter().cloned());
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

/// Every permanent whose aura is in play, in lane order per side (§10.4 layer 5).
fn aura_sources(state: &GameState) -> Vec<&CardInstance> {
    // Every unit read walks this list, so it is built with plain loops (#188), straight off the rows
    // as `zones::active_units_of` reads them: each unit zone's top card in lane order, then the Units
    // the side's carriers hold (R446), then the backrow by lane.
    let mut sources: Vec<&CardInstance> = Vec::new();
    for player in PLAYER_IDS {
        let side = &state.players[player];
        for lane in 1..=UNIT_ZONES {
            if let Some(top) = unit_top(state, player, lane) {
                sources.push(top);
            }
        }
        sources.extend(side.carried.iter().flatten().flatten());
        for lane in 1..=BACKROW_ZONES {
            let card = usize::try_from(lane - 1)
                .ok()
                .and_then(|at| side.backrow.get(at))
                .and_then(|card| card.as_ref());
            if let Some(card) = card {
                sources.push(card);
            }
        }
    }
    sources
}

/// Aura contributions for one unit. An aura's `applies` predicate reads instance data only: it must
/// never call back into `unit_view`, or the layers would recurse.
fn aura_mods(state: &GameState, unit: &CardInstance) -> Vec<StatMod> {
    let mut mods: Vec<StatMod> = Vec::new();
    for source in aura_sources(state) {
        // §6.1/§6.3: Vanilla "clears printed keywords and scripts", and an aura is part of a card's
        // Script (§10.9), so a Vanilla'd permanent projects nothing. It still *receives* aura grants:
        // an aura is the board's text, not the unit's.
        if source.vanilla {
            continue;
        }
        let script = running_script(state, source);
        let Some(aura) = script.aura.as_ref() else {
            continue;
        };
        let args = HookArgs {
            state,
            self_: source,
            radiant: source.radiant,
        };
        for entry in aura(args) {
            if (entry.applies)(unit) {
                mods.push(entry.mod_);
            }
        }
    }
    mods
}

/// §6.1: a unit's keywords are "computed as a set per unit", and §10.4 makes them the union of the
/// printed, granted, aura and position keywords — so a keyword two sources give is had once (Tempo
/// Timmy's printed Rush under Jlockeed's Weapons, a Taunt unit's own Taunt in Defense Position), and
/// the view hands the client one entry for it (§10.8). The numbered keywords are the exception: Armor
/// sums across its sources (§10.4) and Lucky X stacks, so every entry of theirs is kept for the sum.
fn as_set(keywords: &[Keyword]) -> Vec<Keyword> {
    let mut seen: Vec<KeywordKind> = Vec::new();
    keywords
        .iter()
        .filter(|keyword| {
            let kind = keyword.kind();
            // B5 E6: Spell Damage is numbered too, and sums across its sources like Armor.
            if matches!(kind, KeywordKind::Armor | KeywordKind::Lucky | KeywordKind::SpellDamage) {
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

/// B5 E35: a keyword that holds only while a condition does (Classic #69 Plague Charger's First Strike
/// "while it has a Plague Counter"): the card's `conditionalKeywords` hook, read with its printed ones,
/// so a Vanilla takes it (`scripts::script_of` runs no script for one). The hook reads instance data only.
fn conditional_keywords_of(state: &GameState, instance: &CardInstance) -> Vec<Keyword> {
    let script = running_script(state, instance);
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
    let layered = compute_layers(state, instance);
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

/// §10.4's attack before layer 5's floor at 0 — what KY's Constant needs to set a unit's attack to a
/// number exactly (`effects/tune::set_number`), since a card whose layers come to −2 shows 0 and a delta
/// taken off the shown number would land short.
pub fn unclamped_attack(state: &GameState, instance: &CardInstance) -> i32 {
    compute_layers(state, instance).attack
}

/// `compute_layers`' answer.
struct Layered {
    attack: i32,
    max_health: i32,
    keywords: Vec<Keyword>,
    position: Position,
}

/// §10.4's five layers, attack unfloored: `unit_view` floors it, `unclamped_attack` reads it raw.
fn compute_layers(state: &GameState, instance: &CardInstance) -> Layered {
    let printed = face_of(state, instance);
    let position = instance.position.unwrap_or(Position::Atk);

    // Layers 1 and 3: printed stats of the running face, with Fuse already baked in (R77).
    let mut attack = printed.attack;
    let mut max_health = printed.health;

    // Layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39). §10.4 words
    // it as "Felinor Fiender *adds* the sum of your Felinors' layer-4 stats", so the hook returns the
    // sum rather than a finished total, and R39's "never below printed" floors each sum at 0 — a
    // Felinor carrying a negative health buff can pull the total back toward printed, never past it.
    // R132 and R150: this is the ONE floor R39 asks for, on each stat's combined total separately.
    // The contributors reach the hook unfloored (`stats_with_buffs`), so a negative buff really does
    // pull its sum down, and flooring attack here never touches the health sum (R116's per-component
    // clamp).
    // Vanilla has cleared the card's scripts, so a Vanilla'd body keeps its printed stats (§6.1).
    if !instance.vanilla {
        let script = running_script(state, instance);
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

    // Layer 5: auras. A card dormant under a Stack pile is "not on the field for effects" (§3.2, R13)
    // and an aura is one, so none reaches it: it keeps its damage and its own layers 1 to 4, and the
    // board's auras apply again the moment it resumes on top. Without this an aura that shrinks max
    // health (#46) killed buried cards the top of the pile shielded.
    let auras: Vec<StatMod> = if is_dormant(state, instance) {
        Vec::new()
    } else {
        aura_mods(state, instance)
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

    // B3.3 rule 5: a Brittle count in force is the unit's Brittle (`with_brittle_count`).
    let mut listed: Vec<Keyword> = Vec::new();
    if !instance.vanilla {
        listed.extend(printed.keywords.iter().cloned());
    }
    // B5 E35: the keywords its text gives it only while a condition holds, beside the printed ones.
    listed.extend(conditional_keywords_of(state, instance));
    listed.extend(instance.granted_keywords.iter().cloned());
    for modifier in &auras {
        listed.extend(modifier.keywords.iter().flatten().cloned());
    }
    let mut keywords: Vec<Keyword> = with_brittle_count(&listed, instance);

    // Position grants: Defense adds Taunt and Armor +1 (§4.1).
    if position == Position::Def {
        keywords.push(Keyword::Taunt);
        keywords.push(Keyword::Armor { n: 1 });
    }

    // R46: an Indestructible unit that would have been destroyed loses Taunt for the turn. R347: an
    // Indestructible unit never has Taunt at all — printed, granted, from an aura or from Defense
    // Position — so Indestructible, from whichever source, takes Taunt out of the set.
    // A spent Divine Shield and a used Reborn are gone until granted again (§6.1, §4.5 step 4).
    let taunt_suppressed = instance.taunt_suppressed_turn == Some(state.turn)
        || keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Indestructible);
    let shield_spent = instance.divine_shield_spent == Some(true);
    let reborn_spent = instance.reborn_spent == Some(true);
    let kept: Vec<Keyword> = keywords
        .into_iter()
        .filter(|keyword| {
            let kind = keyword.kind();
            !(taunt_suppressed && kind == KeywordKind::Taunt)
                && !(shield_spent && kind == KeywordKind::DivineShield)
                && !(reborn_spent && kind == KeywordKind::Reborn)
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
/// layer that sums other units needs (#92 Felinor Fiender, R116).
///
/// R150: NO PER-UNIT FLOOR. §10.4 floors attack at layer 5 and at nothing earlier, so this reading
/// reports what layers 1 to 4 actually come to, negative included. A `max(0, …)` here would be
/// invisible to a single card and wrong for every caller that sums: a Felinor carrying a −5 buff
/// would contribute 0 instead of −2 and could never "pull the attack sum toward 0", which is exactly
/// what R132 requires of #92's total. The floor belongs where the value is finally used — on the
/// combined total in `unit_view`'s layer 2 above (R116, R132), and on the displayed attack at the end
/// of `unit_view` (§10.4 layer 5) — so it is applied once, by the reader that knows which number the
/// rule floors, rather than baked into a reading that has more than one reader.
///
/// The only other readers are the tests, `query.rs`'s re-export and `view_for`'s hand cards (R243),
/// which floor the attack they show; R89's death snapshot reads `unit_view` (`state_check.rs`), which
/// floors its own attack, so nothing is left unclamped by this.
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
