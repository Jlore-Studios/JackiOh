//! One damage instance: the ten ordered steps of SPEC §4.4, plus heal and lose health.
//! M2-T3 adds a test per step; combat's Cleave step lives in combat.rs (M2-T4).
//!
//! Patch v0.2.0 (docs/classic-sets.md B5) adds to the pipeline, in the order a hit meets them:
//!   - E6 Spell Damage, before step 1: a Spell's hit is raised by the Spell Damage on its controller's
//!     side (`spell_damage_of`), once — a Trample excess or a redirected hit is the same hit going on,
//!     and is not raised again;
//!   - E6 on a hero, after step 2's Armor: the divisors its controller's cards set (several multiply,
//!     rounded up once), then step 3's caps, the lowest of every cap (`hero_hit_amount`);
//!   - E5 and E9, after the caps and before step 5: a hit that would bring its hero to 0 or less meets
//!     the "would take lethal damage" replacements (`replacements::lethal_hit_window`), which may send it
//!     to the other hero as a new instance from the same source;
//!   - E6 Trample stated by an effect (`flags.trample`), as R346 has an effect state Pierce;
//!   - E5 and E8 in every heal: "would be healed" and its conversion into Pierce damage
//!     (`replacements::healing_replaced`);
//!   - E7 set health (`set_hero_health`): no pipeline, not damage, not a heal.
//!
//! Patch v0.3.X (docs/meditative-set.md M8, MN05) reports what Armor does: the `damage` event carries
//! the Armor's part of the hit (`absorbed`, R1360), and a hit the Armor takes whole, which R63's zero
//! rule stops, is reported by `damageAbsorbed` (R1361), a report that nothing answers.
//!
//! Port of `packages/engine/src/damage.ts` (part 3). TS's `DamageTarget` held the live
//! `CardInstance`; here a unit target carries the instance as the caller had it, and every read and
//! write of the target goes to the card under that id in the state as it stands now (the TS object
//! was live). A source is read as the caller handed it (TS read the object it was handed: a Death
//! hook's source is the snapshot of the card as it died, R89).

use serde::{Deserialize, Serialize};

use crate::config::{ANTI_ONESHOT_CAP, DAMAGE_REDIRECT_CAP, HERO_ARMOR};
use crate::script::{EngineSink, HeroGuard, HookArgs};
use crate::state::{CardInstance, GameState, ModifierKind, find_instance, find_instance_mut};
use crate::wire::{
    CardType, GameEvent, Keyword, KeywordKind, PlayerId, Row, ZoneName, armor_of, has_keyword, opponent_of,
};

/// TS `{ kind: "unit"; instance } | { kind: "hero"; player }`. The unit is carried by value, as TS's
/// object was: a target is built once per hit and matched at every card and test that aims one, and a
/// `Box` there would buy nothing but noise at those sites.
#[allow(clippy::large_enum_variant)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DamageTarget {
    Unit { instance: CardInstance },
    Hero { player: PlayerId },
}

/// `DamageArgs.flags`.
///
/// `trample` (E6): the effect states that its own damage has Trample, as `ignore_armor` states Pierce
/// (R346) and `lifesteal` Lifesteal (R85), so a Spell's printed Trample (Classic #83 Flame Lance)
/// still tramples on a repeat that has no source left.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "camelCase")]
pub struct DamageFlags {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignore_armor: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub combat: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifesteal: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trample: Option<bool>,
    /// MD-D4, R1120: the effect states that its own damage has Poisonous, as `lifesteal` states
    /// Lifesteal — so a combat strike an attack modifier gilded destroys its Unit target (step 7)
    /// without the source having the keyword. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poisonous: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageArgs {
    pub source: Option<CardInstance>,
    pub target: DamageTarget,
    pub amount: i32,
    pub flags: Option<DamageFlags>,
}

/// TS `DamageSink = { state, events }`: every caller hands the engine's sink (SURFACE §6.5).
pub type DamageSink<'a> = EngineSink<'a>;

fn target_id(target: &DamageTarget) -> String {
    match target {
        DamageTarget::Unit { instance } => instance.id.clone(),
        DamageTarget::Hero { player } => format!("hero-{player}"),
    }
}

fn controller_of(target: &DamageTarget) -> PlayerId {
    match target {
        DamageTarget::Unit { instance } => instance.controller,
        DamageTarget::Hero { player } => *player,
    }
}

// ---------------------------------------------------------------------------------------------
// Private copies of `scripts.rs`'s readers (fullsend rule 5): the running face's script, its static
// flags, and every text a card carries (`scripts.ts`'s `scriptOf`, `flagsOf`, `textsOf`).
// ---------------------------------------------------------------------------------------------

/// §4.4 step 2 for a hero: the Armor written on the hero itself plus every backrow card that grants
/// it (#84 Going Long), each contributing the `HERO_ARMOR` value its own face and price select.
///
/// R124: hero Armor from several sources **adds up**, exactly as §6.2's Armor stacks on a unit
/// (printed + Defense +1 + auras) — two Going Longs paid 2 are Armor 4. That is deliberately the
/// opposite of `hero_damage_cap` below, which takes the *smallest* cap on offer: a cap is a ceiling,
/// Armor is a reduction, so they compose in opposite directions and are not unified.
///
/// Nothing about the grant is stored, so it stops the moment the granting card leaves the backrow.
/// R757: #98's Armor Up adds its player modifier's Armor while the modifier stands.
/// Every reader of hero Armor goes through here — the pipeline, `subsystems::lethal`'s projection,
/// `subsystems::scorer` and §10.8's hero block — so no projection can disagree with the hit (R44).
pub fn hero_armor_of(state: &GameState, player: PlayerId) -> i32 {
    let mut sum = state.players[player].hero.armor + modifier_armor_of(state, player);
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        let Some(card) = crate::zones::card_at(state, slot) else {
            continue;
        };
        let side = HERO_ARMOR.on(card.radiant);
        // R124, R102: every Going Long text a card carries grants its own Armor, at the price that
        // text's card was played for — a Going Long fused onto a Going Long is Armor 4 as two apart are.
        for text in crate::scripts::texts_of(state, card) {
            let grants = text.flags.hero_armor.map_or(0, |grant| grant.count());
            if grants <= 0 {
                continue;
            }
            // R386: the card's declared numbers `armor` and `paidArmor` where it declares them.
            let key = if text.embiggened { "paidArmor" } else { "armor" };
            sum += grants * crate::params::declared_or(state, card, key, side.on(text.embiggened));
        }
    }
    sum
}

/// R757: the Armor a player's `heroArmor` modifiers add, each until its expiry takes it off.
fn modifier_armor_of(state: &GameState, player: PlayerId) -> i32 {
    state.players[player]
        .mods
        .iter()
        .fold(0, |sum, modifier| match modifier.kind {
            ModifierKind::HeroArmor { amount } => sum + amount.max(0),
            _ => sum,
        })
}

/// The cards acting on a player's side of the field (§3.2): the top of each unit pile and each backrow
/// card, a face-down Trap left out — its text is in nobody's use until it fires (R33). What E6's hero
/// guards read.
pub(crate) fn acting_texts_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    let mut out: Vec<CardInstance> = crate::zones::active_units_of(state, player)
        .into_iter()
        .cloned()
        .collect();
    for slot in crate::zones::slots_of(player, Row::Backrow) {
        let Some(card) = crate::zones::card_at(state, slot) else {
            continue;
        };
        let card_type = crate::faces::card_type_of(state, card);
        let face_down =
            (card_type == CardType::Trap || card_type == CardType::FieldTrap) && card.face_up != Some(true);
        if !face_down {
            out.push(card.clone());
        }
    }
    out
}

/// E6: every hero guard a player's cards set on their hero now (`Script.heroGuard`).
fn hero_guards_of(state: &GameState, player: PlayerId) -> Vec<HeroGuard> {
    acting_texts_of(state, player)
        .iter()
        .flat_map(
            |card| match crate::scripts::script_of(state, card).hero_guard.clone() {
                Some(guard) => guard(HookArgs {
                    state,
                    self_: card,
                    radiant: card.radiant,
                }),
                None => Vec::new(),
            },
        )
        .collect()
}

/// §4.4 step 3: the smallest hero cap on offer — any Anti-oneshot Armor acting on this player's side,
/// in its backrow zone or animated into a unit zone (R383: it keeps all of its text), and every
/// per-hit cap its cards set (E6, Classic+ #11 Anime Armor's 1): the lowest cap wins. A hero immune
/// to damage (Meditative #48, MD-C21) caps every hit at 0.
pub fn hero_damage_cap(state: &GameState, player: PlayerId) -> Option<i32> {
    let mut caps: Vec<i32> = acting_texts_of(state, player)
        .iter()
        .filter(|card| crate::scripts::flags_of(state, card).anti_oneshot == Some(true))
        // R386: the cap is the card's declared number `cap` where it declares one.
        .map(|card| crate::params::declared_or(state, card, "cap", ANTI_ONESHOT_CAP.on(card.radiant)))
        .collect();
    caps.extend(
        hero_guards_of(state, player)
            .iter()
            .filter_map(|guard| guard.cap.map(|cap| cap.max(0))),
    );
    // MD-C21: immunity is a cap of 0, so it wins over every other cap.
    if state.players[player]
        .mods
        .iter()
        .any(|modifier| matches!(modifier.kind, ModifierKind::HeroImmune))
    {
        caps.push(0);
    }
    caps.into_iter().min()
}

/// E6: what a hit on this player's hero is divided by after Armor — the product of every divisor its
/// cards set (Classic #75 Argusland's 2, 4 on its Radiant face), so several multiply. 1 when none does.
pub fn hero_damage_divisor(state: &GameState, player: PlayerId) -> i32 {
    hero_guards_of(state, player)
        .iter()
        .fold(1, |product, guard| match guard.divisor {
            Some(divisor) => product * divisor.max(1),
            None => product,
        })
}

/// §4.4 steps 2 and 3 on a hero, with E6 between them: Armor (unless the hit pierces, R346), then the
/// divisors, rounded up once (R463), then the lowest cap. The one reading of what a hit takes off a
/// hero, which the pipeline, R44's lethal projection and the Zephyrs scorer all call, so no projection
/// disagrees with the hit. (TS `pierce = false`: pass `false` for an ordinary hit.)
pub fn hero_hit_amount(state: &GameState, player: PlayerId, amount: i32, pierce: bool) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let after_armor = if pierce {
        amount
    } else {
        (amount - hero_armor_of(state, player)).max(0)
    };
    let divisor = hero_damage_divisor(state, player);
    // `Math.ceil(afterArmor / divisor)` on a non-negative amount and a divisor of at least 1.
    let divided = (after_armor + divisor - 1) / divisor;
    match hero_damage_cap(state, player) {
        None => divided,
        Some(cap) => divided.min(cap),
    }
}

/// E6, §4.4 step 0: the Spell Damage on a player's side — every "Spell Damage +N" among the keywords
/// of the units acting on their field (the top of each pile, an animated card standing in a unit zone
/// included), summed, as Armor sums (a numbered keyword, §10.4).
pub fn spell_damage_of(state: &GameState, player: PlayerId) -> i32 {
    let mut total = 0;
    for card in crate::zones::active_units_of(state, player) {
        for keyword in crate::layers::unit_view(state, card).keywords {
            if let Keyword::SpellDamage { n } = keyword {
                total += n;
            }
        }
    }
    total.max(0)
}

/// E6: how much a hit from this source is raised — its controller's Spell Damage, when it is a Spell.
fn spell_damage_for(state: &GameState, source: Option<&CardInstance>) -> i32 {
    let Some(source) = source else {
        return 0;
    };
    if !crate::restrictions::is_spell_source(state, Some(source)) {
        return 0;
    }
    let controller = if source.zone.z() == ZoneName::Resolving {
        source.zone.player()
    } else {
        source.controller
    };
    spell_damage_of(state, controller)
}

/// R42: a unit something has already killed and the state check has not collected yet — at 0 or less
/// health, or marked destroyed (a Poisonous hit, a destroy). A unit is killed once, so whatever lands
/// on it afterwards changes nothing about who killed it: not a later hit (a Cleave, a second spell, a
/// Death of the same pass), not a Poisonous one, and not a destroy (`effects::destroy`).
pub fn already_killed(state: &GameState, unit: &CardInstance) -> bool {
    let view = crate::layers::unit_view(state, unit);
    // §4.5 step 1's own test: an Indestructible unit dies only once its max health is gone (R69).
    if has_keyword(&view.keywords, KeywordKind::Indestructible) {
        return view.max_health <= 0;
    }
    unit.marked_destroyed == Some(true) || view.health <= 0
}

/// R42, R89: "a death whose lethal damage instance came from this unit". A hit is lethal when it
/// takes a unit nothing has killed yet (`already_killed`) to 0 or less health, and that is the moment
/// it is credited — never at death, which a layer can cause long after the last hit (an aura
/// lowering max health, #46): a hit that left such a unit standing clears any older credit, and a hit
/// on a unit something already killed changes nothing, since the first one killed it. Poisonous
/// credits its own hit in step 7 on the same terms. The state check forgets a credit whose unit is
/// standing again.
fn credit_killer(unit: &mut CardInstance, source: Option<&CardInstance>, killed_before: bool, after: i32) {
    if killed_before {
        return;
    }
    let Some(source) = source.filter(|_| after <= 0) else {
        unit.last_damaged_by = None;
        return;
    };
    // R412: a kill credit in force on the source names another unit (`kill_credit.rs`).
    let killer = crate::kill_credit::credited_killer_id(source, unit);
    unit.last_damaged_by = Some(killer);
}

/// R346: whether a hit skips §4.4 step 2. Pierce is a keyword of the source — a unit's, read through
/// the layers like its Lifesteal and Trample (§10.4), or a spell's printed on its face (#44 True
/// Strike), which the same reading finds on the card while it resolves — or the effect's own
/// `ignore_armor`, which states that its damage pierces without the source having the keyword, as
/// R85's `lifesteal` does for Lifesteal. It skips step 2 and nothing else: Divine Shield, the hero
/// cap and Indestructible all still apply.
pub fn pierces(state: &GameState, source: Option<&CardInstance>, flags: Option<&DamageFlags>) -> bool {
    if flags.is_some_and(|flags| flags.ignore_armor == Some(true)) {
        return true;
    }
    source.is_some_and(|source| {
        has_keyword(
            &crate::layers::unit_view(state, source).keywords,
            KeywordKind::Pierce,
        )
    })
}

/// Whether the source reads `kind` through the layers (§10.4).
fn source_has(state: &GameState, source: Option<&CardInstance>, kind: KeywordKind) -> bool {
    source.is_some_and(|source| has_keyword(&crate::layers::unit_view(state, source).keywords, kind))
}

/// Deal one damage instance. Returns the amount actually dealt — by a redirected hit, where the
/// instance went on to the other hero (E9). A hit of 0 before step 1 is not a damage instance at
/// all: Divine Shield stays, nothing triggers (R63), and there is nothing for Spell Damage to raise.
pub fn deal_damage(sink: &mut DamageSink<'_>, args: DamageArgs) -> i32 {
    let amount_in = args.amount;
    if amount_in <= 0 {
        return 0;
    }
    // E6: before step 1, a Spell's hit is raised by its controller's Spell Damage.
    let raised = amount_in + spell_damage_for(sink.state, args.source.as_ref());
    land_hit(sink, &args, raised, 0, &[])
}

/// §4.4 from step 1, for a hit already raised by Spell Damage. A Trample excess (step 9) and a
/// redirected hit (E9) are new instances of the same hit, so they come back in here rather than
/// through `deal_damage`, which would raise them a second time. `redirects` counts how often this hit
/// has moved already (`DAMAGE_REDIRECT_CAP`); `caught` names the guards that have caught it, so each
/// catches a given hit once (R1204, R460).
fn land_hit(
    sink: &mut DamageSink<'_>,
    args: &DamageArgs,
    amount_in: i32,
    redirects: u32,
    caught: &[String],
) -> i32 {
    let source = args.source.as_ref();
    let target = &args.target;
    let flags = args.flags.as_ref();
    if amount_in <= 0 {
        return 0;
    }
    // §4: damage is a unit's on the field — it stays there between turns and leaving the field takes
    // it off (R78). A card that has left the field, or lies dormant under a Stack (R13), is no unit to
    // hit: an effect still aimed at it fizzles (§8 Conventions), rather than leaving damage on a card
    // in a graveyard or a hand that would follow it back onto the field.
    let unit: Option<CardInstance> = match target {
        DamageTarget::Unit { instance } => {
            let Some(live) = find_instance(sink.state, &instance.id).cloned() else {
                return 0;
            };
            if !crate::zones::acts_on_field(sink.state, &live) {
                return 0;
            }
            Some(live)
        }
        DamageTarget::Hero { .. } => None,
    };

    // Step 1: Divine Shield negates the whole hit and is gone.
    if let Some(live) = &unit {
        let view = crate::layers::unit_view(sink.state, live);
        if has_keyword(&view.keywords, KeywordKind::DivineShield) && live.divine_shield_spent != Some(true) {
            if let Some(card) = find_instance_mut(sink.state, &live.id) {
                card.divine_shield_spent = Some(true);
            }
            sink.events.push(GameEvent::DivineShieldLost {
                instance_id: live.id.clone(),
            });
            return 0;
        }
    }

    // Step 2: Armor, unless the hit pierces it (R346). A hero's total is `hero_armor_of`: what is
    // written on the hero plus every backrow grant (#84), summed per R124. Fatigue is an ordinary
    // instance on its own hero and pays this step like any other hit (R125); only "lose health"
    // bypasses the pipeline (R18), and that never comes through here.
    // Step 3 on a hero, with E6's divisors between the two: `hero_hit_amount`.
    // R1360: `absorbed` is what the Armor took off the hit at step 2, and nothing after it — not a
    // hero's divisors or cap — so a hit that pierces has none.
    let mut amount = amount_in;
    let mut absorbed = 0;
    match (target, &unit) {
        (DamageTarget::Hero { player }, _) => {
            let pierce = pierces(sink.state, source, flags);
            if !pierce {
                absorbed = hero_armor_of(sink.state, *player).clamp(0, amount_in);
            }
            amount = hero_hit_amount(sink.state, *player, amount, pierce);
        }
        (DamageTarget::Unit { .. }, Some(live)) => {
            if !pierces(sink.state, source, flags) {
                amount = (amount - armor_of(&crate::layers::unit_view(sink.state, live).keywords)).max(0);
                absorbed = (amount_in - amount).max(0);
            }
        }
        (DamageTarget::Unit { .. }, None) => {}
    }

    // E5, E9: after the caps and before step 5, a hit that would bring its hero to 0 or less — this
    // hit alone, as R44 judges it — meets the "would take lethal damage" replacements, which may send
    // it to the other hero as a new instance from the same source, through that hero's Armor and caps.
    if let DamageTarget::Hero { player } = target
        && amount > 0
        && redirects < DAMAGE_REDIRECT_CAP
        && sink.state.players[*player].hero.health - amount <= 0
    {
        let source_id = source.map(|card| card.id.clone());
        let to = crate::replacements::lethal_hit_window(sink, target, amount, source_id, caught);
        if let Some(to) = to {
            // R1204: the hit the guard catches is never caught again by the same guard — and a
            // hero-to-hero redirect changes controllers, so the old side's catchers can never answer
            // the new walk anyway. Only a Unit destination names its catcher.
            let mut next_caught: Vec<String> = caught.to_vec();
            if let DamageTarget::Unit { instance } = &to {
                next_caught.push(instance.id.clone());
            }
            let redirected = DamageArgs {
                target: to,
                ..args.clone()
            };
            return land_hit(sink, &redirected, amount_in, redirects + 1, &next_caught);
        }
    }

    // The zero rule: a hit reduced to 0 by steps 2 and 3 stops there, emits no `damage` and triggers
    // nothing (R63). R1361: one the target's Armor took whole is reported by `damageAbsorbed`, a
    // report nothing answers (`triggers::dispatch_event`); a hit a cap alone stopped is not Armor's.
    // The rule stands before step 4, as §4.4 orders it, so an Indestructible unit's Armor that takes
    // a hit whole reports it like any other unit's.
    if amount <= 0 {
        if absorbed > 0 && absorbed >= amount_in {
            sink.events.push(GameEvent::DamageAbsorbed {
                source_id: source.map(|card| card.id.clone()),
                target_id: target_id(target),
                absorbed,
                combat: flags.is_some_and(|flags| flags.combat == Some(true)),
            });
            let at = sink.events.len() - 1;
            crate::triggers::withhold_report(sink, at);
        }
        return 0;
    }

    // Step 4: Indestructible units take nothing, and emit no damage event.
    if let Some(live) = &unit
        && has_keyword(
            &crate::layers::unit_view(sink.state, live).keywords,
            KeywordKind::Indestructible,
        )
    {
        return 0;
    }

    // ME-LETHALGUARD (R1204): §4.4 step 4a opens for Units too. A hit that alone would leave a Unit
    // nothing has killed yet (R42) at 0 or less — judged after steps 0 to 4, so a Divine Shield
    // that took the hit and an Indestructible that takes nothing never open it — meets the
    // replacements, which may send it to a guard as a new instance from the same source, through
    // the guard's own pipeline.
    if let Some(live) = &unit
        && amount > 0
        && redirects < DAMAGE_REDIRECT_CAP
        && !already_killed(sink.state, live)
        && crate::layers::unit_view(sink.state, live).health - amount <= 0
    {
        let source_id = source.map(|card| card.id.clone());
        let to = crate::replacements::lethal_hit_window(sink, target, amount, source_id, caught);
        if let Some(to) = to {
            let mut next_caught: Vec<String> = caught.to_vec();
            if let DamageTarget::Unit { instance } = &to {
                next_caught.push(instance.id.clone());
            }
            let redirected = DamageArgs {
                target: to,
                ..args.clone()
            };
            return land_hit(sink, &redirected, amount_in, redirects + 1, &next_caught);
        }
    }

    // Step 5: apply, capping what a Trample source deals to a unit at its health (R63).
    let mut dealt = amount;
    let mut trample_excess = 0;
    if let Some(live) = &unit {
        let view = crate::layers::unit_view(sink.state, live);
        // E6: a Spell's printed Trample is read off it while it resolves, as a unit's is, or stated.
        let trample = flags.is_some_and(|flags| flags.trample == Some(true))
            || source_has(sink.state, source, KeywordKind::Trample);
        if trample && amount > view.health {
            dealt = view.health.max(0);
            trample_excess = amount - dealt;
        }
    }

    // R63's zero rule, now for the Trample cap: a unit already at 0 or less health (max health
    // dragged down by Suppressive Aura, or damage the state check has not collected yet) has no
    // health for the hit to count against, so nothing is dealt to it. Nothing dealt is not a damage
    // instance: no `damage` event, no `lastDamagedBy`, and none of steps 6 to 8. Step 9 still runs,
    // with the whole amount, because the excess beyond that unit's health is all of it.
    if dealt <= 0 {
        if trample_excess > 0 {
            let excess = DamageArgs {
                source: args.source.clone(),
                target: DamageTarget::Hero {
                    player: controller_of_now(&unit, target),
                },
                amount: trample_excess,
                flags: args.flags,
            };
            land_hit(sink, &excess, trample_excess, 0, caught);
        }
        return 0;
    }

    // R42: whether something had killed the unit before this hit, which then kills nothing.
    let killed_before = unit.as_ref().is_some_and(|live| already_killed(sink.state, live));
    match (target, &unit) {
        (DamageTarget::Unit { .. }, Some(live)) => {
            let before = crate::layers::unit_view(sink.state, live).health;
            if let Some(card) = find_instance_mut(sink.state, &live.id) {
                card.damage += dealt;
                credit_killer(card, source, killed_before, before - dealt);
            }
        }
        (DamageTarget::Hero { player }, _) => {
            sink.state.players[*player].hero.health -= dealt;
        }
        (DamageTarget::Unit { .. }, None) => {}
    }

    sink.events.push(GameEvent::Damage {
        source_id: source.map(|card| card.id.clone()),
        target_id: target_id(target),
        amount: dealt,
        combat: flags.is_some_and(|flags| flags.combat == Some(true)),
        absorbed,
    });

    // Step 6 (on-damage triggers) is dispatched by the trigger loop from the `damage` event (§10.3).

    // Step 7: Poisonous destroys a damaged unit.
    if let (Some(live), Some(source_card)) = (&unit, source)
        && dealt >= 1
        && (source_has(sink.state, Some(source_card), KeywordKind::Poisonous)
            || flags.is_some_and(|flags| flags.poisonous == Some(true)))
        && let Some(card) = find_instance_mut(sink.state, &live.id)
    {
        card.marked_destroyed = Some(true);
        // R42: the Poisonous hit is the one that destroys it, whatever health it left — unless something
        // had already killed it, in which case this hit landed on a dead unit and kills nothing.
        if !killed_before {
            let killer = crate::kill_credit::credited_killer_id(source_card, card);
            card.last_damaged_by = Some(killer);
        }
    }

    // MD-D31, R1124: exile on damage rides beside Poisonous — when the source's card exiles on
    // damage, the damaged Unit is marked, and the next state check exiles it ahead of deaths (§4.5
    // step 1). A hit the pipeline stopped (Divine Shield, Indestructible, the zero rule) never
    // reaches here, so none of them is exiled.
    if let (Some(live), Some(source_card)) = (&unit, source)
        && dealt >= 1
        && crate::scripts::flags_of(sink.state, source_card).exiles_on_damage == Some(true)
        && let Some(card) = find_instance_mut(sink.state, &live.id)
    {
        card.marked_exiled = Some(true);
    }

    // Step 8: Lifesteal heals the source's controller's hero by the amount dealt. R85: an effect
    // may state that its own damage has Lifesteal, which heals without granting the source the keyword.
    // R85's heal goes to the source's controller, so an effect with no source heals nobody.
    let source_has_lifesteal = source_has(sink.state, source, KeywordKind::Lifesteal);
    if let Some(source_card) = source
        && (source_has_lifesteal || flags.is_some_and(|flags| flags.lifesteal == Some(true)))
    {
        heal_hero(sink, source_card.controller, dealt);
    }

    // Step 9: Trample sends the excess to the target's controller's hero as its own instance — of the
    // same hit, so not raised again by Spell Damage. R1204: the excess keeps the hit's catchers.
    if trample_excess > 0 {
        let excess = DamageArgs {
            source: args.source.clone(),
            target: DamageTarget::Hero {
                player: controller_of_now(&unit, target),
            },
            amount: trample_excess,
            flags: args.flags,
        };
        land_hit(sink, &excess, trample_excess, 0, caught);
    }

    dealt
}

/// `controllerOf(target)` on TS's live object: the unit's controller as it stands now.
fn controller_of_now(unit: &Option<CardInstance>, target: &DamageTarget) -> PlayerId {
    match unit {
        Some(live) => live.controller,
        None => controller_of(target),
    }
}

/// §6.3 Heal: units are capped at max health, heroes are not. E5, E8: the heal first meets the "would
/// be healed" replacements on its stated amount, a heal on an undamaged unit included (R462) — one
/// that replaced it heals nothing.
pub fn heal_unit(sink: &mut DamageSink<'_>, instance: &CardInstance, amount: i32) -> i32 {
    let stated = amount;
    if stated <= 0 {
        return 0;
    }
    let live = find_instance(sink.state, &instance.id)
        .cloned()
        .unwrap_or_else(|| instance.clone());
    if crate::replacements::healing_replaced(
        sink,
        &DamageTarget::Unit {
            instance: live.clone(),
        },
        stated,
    ) {
        return 0;
    }
    let damage_now = find_instance(sink.state, &live.id).map_or(live.damage, |card| card.damage);
    let healed = damage_now.min(stated);
    if let Some(card) = find_instance_mut(sink.state, &live.id) {
        card.damage -= healed;
    }
    if healed > 0 {
        sink.events.push(GameEvent::Healed {
            target_id: live.id.clone(),
            amount: healed,
        });
    }
    healed
}

pub fn heal_hero(sink: &mut DamageSink<'_>, player: PlayerId, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let healed = amount;
    if healed <= 0 {
        return 0;
    }
    if crate::replacements::healing_replaced(sink, &DamageTarget::Hero { player }, healed) {
        return 0;
    }
    sink.state.players[player].hero.health += healed;
    sink.events.push(GameEvent::Healed {
        target_id: format!("hero-{player}"),
        amount: healed,
    });
    healed
}

/// "Heal to full" removes all damage; "heal up to N" raises a hero to at least N (§6.3).
pub fn heal_to_full(sink: &mut DamageSink<'_>, instance: &CardInstance) -> i32 {
    let damage = find_instance(sink.state, &instance.id).map_or(instance.damage, |card| card.damage);
    heal_unit(sink, instance, damage)
}

pub fn heal_hero_up_to(sink: &mut DamageSink<'_>, player: PlayerId, floor: i32) -> i32 {
    let health = sink.state.players[player].hero.health;
    if health >= floor {
        return 0;
    }
    heal_hero(sink, player, floor - health)
}

/// R18: lose health is not damage. No pipeline, no armor, no cap, no on-damage effects.
pub fn lose_health(sink: &mut DamageSink<'_>, player: PlayerId, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let lost = amount;
    sink.state.players[player].hero.health -= lost;
    sink.events.push(GameEvent::HealthLost { player, amount: lost });
    lost
}

/// E7: "Set a hero's health to N" (Classic #29) — no pipeline, not damage and not a heal, like R18's
/// lose health: no Armor, no cap, no replacement, nothing that answers a hit or a heal. `healthSet`.
pub fn set_hero_health(sink: &mut DamageSink<'_>, player: PlayerId, value: i32, source_id: Option<String>) {
    let health = value;
    sink.state.players[player].hero.health = health;
    sink.events.push(GameEvent::HealthSet {
        player,
        health,
        source_id,
    });
}

pub fn enemy_of(player: PlayerId) -> PlayerId {
    opponent_of(player)
}
