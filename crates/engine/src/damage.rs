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
//! Stinkyscape's verbs (#1849) add three steps, each read off a static flag and skipped without it:
//! a Spell's hit on a Unit immune to Spell Damage is no instance (R1849000); before Divine Shield and
//! Armor, an own-turn redirect moves a hero's hit to the opponent (step 0a, R1849004); and after
//! Lifesteal, a reflecting Unit deals what it took again, from itself (step 8a, R1849003).
//!
//! Duos co-op (#936 S3) writes a hero's health only through `duos_life.rs`: a Duos seat's own HP,
//! then its team's pool (B13, B14), and step 4a judges a hit against both (B17). Without a Duos life
//! on the hero each of those is the arithmetic it replaced, so a 1v1 game is unchanged (B3).
//!
//! Port of `packages/engine/src/damage.ts` (part 3). TS's `DamageTarget` held the live
//! `CardInstance`; here a unit target carries the instance as the caller had it, and every read and
//! write of the target goes to the card under that id in the state as it stands now (the TS object
//! was live). A source is read as the caller handed it (TS read the object it was handed: a Death
//! hook's source is the snapshot of the card as it died, R89).

use serde::{Deserialize, Serialize};

use crate::config::{ANTI_ONESHOT_CAP, DAMAGE_REDIRECT_CAP, HALF_DAMAGE_TO_HEROES_DIVISOR, HERO_ARMOR};
use crate::replacements::LethalHitTo;
use crate::script::{EngineSink, HeroGuard, HookArgs};
use crate::state::{CardInstance, GameState, ModifierKind, find_instance, find_instance_mut};
use crate::wire::{
    GameEvent, Keyword, KeywordKind, PlayerId, RedirectWhat, Row, ZoneName, armor_of, has_keyword,
    opponent_of,
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
    /// #1750, R1750000: the hit is the attacker's own strike (its hit on the defender and its
    /// Cleave's), the only damage a Unit's Trample applies to: a defender's strike back and the
    /// damage a Unit's text deals never trample. A Spell's printed Trample and an effect's stated
    /// `trample` do not need it. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attacking: Option<bool>,
    /// #1841, R1841001: "deals half damage to heroes": a hit on a hero is divided by
    /// `HALF_DAMAGE_TO_HEROES_DIVISOR`, rounded down, before Armor (step 2); a Unit takes it whole.
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub half_to_heroes: Option<bool>,
    /// R1848001 (Stinkyscape #3 Meteor, "Tramples onto adjacent Units"): the hit tramples, and its
    /// excess goes onto the Units adjacent to its target instead of its controller's hero
    /// (`trample_onto_adjacent`), on to that hero after them where it says so.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trample_onto: Option<TrampleOnto>,
}

/// R1848001: where a hit's Trample excess goes when it is not §4.4 step 9's hero: onto the Units
/// adjacent to its target (Stinkyscape #3 Meteor's base face), or onto them and then the target's
/// controller's hero (its Radiant face, "Tramples onto adjacent Units and the enemy hero").
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum TrampleOnto {
    Adjacent,
    AdjacentAndHero,
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
        // R1487000: a face-down card's text sleeps until it is revealed or face-up.
        if crate::preview::lasting_text_off(state, card) {
            continue;
        }
        let side = HERO_ARMOR.on(card.radiant);
        // R124, R102: every Going Long text a card carries grants its own Armor, at the price that
        // text's card was played for — a Going Long fused onto a Going Long is Armor 4 as two apart are.
        for text in crate::scripts::texts_of(state, card) {
            let grants = text.flags.hero_armor.map_or(0, |grant| grant.count());
            if grants <= 0 {
                continue;
            }
            // R386: the card's declared numbers `armor` and `paidArmor` where it declares them, read as
            // they stand (#73 Anti-oneshot Armor's Radiant `armor` 1 since #2657, which `HERO_ARMOR`,
            // Going Long's table, does not hold), `HERO_ARMOR` where it does not.
            let key = if text.embiggened { "paidArmor" } else { "armor" };
            let armor = match crate::params::param_decl_of(state, &card.def_id, key) {
                Some(_) => crate::params::param_value(state, Some(card), key, Default::default()),
                None => side.on(text.embiggened),
            };
            sum += grants * armor;
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

/// The cards acting on a player's side of the field (§3.2), each once (`zones::acting_on`): the top of
/// each unit pile, the Units its carriers hold (R446), a Unit standing in the backrow among them
/// (R1081000), and each backrow card, a face-down Trap left out — its text is in nobody's use until it
/// fires (R33) or is revealed (R1487000). What E6's hero guards read.
pub(crate) fn acting_texts_of(state: &GameState, player: PlayerId) -> Vec<CardInstance> {
    crate::zones::acting_on(state, player, crate::zones::StandingAt::AmongUnits)
        .into_iter()
        // R1487000: a face-down card's text sleeps until it is revealed or face-up.
        .filter(|card| !crate::preview::lasting_text_off(state, card))
        .cloned()
        .collect()
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
/// to damage (Meditative #48, MD-C21) caps every hit at 0, and a floored hero (R2201000) every hit at
/// what it has above its floor.
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
    // R2201000: a floored hero takes no more than it has above its floor, so no hit is ever lethal.
    if let Some(floor) = crate::state::hero_floor_of(state, player) {
        caps.push((state.players[player].hero.health - floor).max(0));
    }
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

/// R2179005: the smallest per-hit cap a Unit's own guards set on it (`HeroGuard.guards_self`, C+ #11
/// Anime Armor's "Your hero and this"), or `None` when none does. Read like the hero's caps, after
/// Armor at §4.4 step 3, and before a Trample's excess is split off at step 5.
pub fn unit_damage_cap(state: &GameState, unit: &CardInstance) -> Option<i32> {
    let script = crate::scripts::script_of(state, unit);
    let guard = script.hero_guard.as_ref()?;
    guard(HookArgs {
        state,
        self_: unit,
        radiant: unit.radiant,
    })
    .iter()
    .filter(|guard| guard.guards_self == Some(true))
    .filter_map(|guard| guard.cap.map(|cap| cap.max(0)))
    .min()
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

/// E6, §4.4 step 0: a player's Spell Damage — every "Spell Damage +N" among the keywords of the units
/// acting on their field (the top of each pile, an animated card standing in a unit zone included),
/// summed, as Armor sums (a numbered keyword, §10.4). R1180000: one sum, whatever gives it — it adds
/// every Spell Damage aura acting on either side that reaches this player (Meditative+ #9's "Both
/// players have Spell Damage +X") and, R1180001, their live "Spell Damage +N this turn" modifiers
/// (Meditative+ #11). Without either, the number is the Units' alone, as before (D9). R1563002: the
/// keyword printed on a face-up backrow card it controls counts as a Unit's does (C #23 Devil's Pact).
pub fn spell_damage_of(state: &GameState, player: PlayerId) -> i32 {
    let mut total = 0;
    let units = crate::zones::active_units_of(state, player);
    for card in &units {
        for keyword in crate::layers::unit_view(state, card).keywords {
            if let Keyword::SpellDamage { n } = keyword {
                total += n;
            }
        }
    }
    for card in acting_texts_of(state, player) {
        if units.iter().any(|unit| unit.id == card.id) {
            continue;
        }
        for keyword in crate::layers::card_keywords(state, &card) {
            if let Keyword::SpellDamage { n } = keyword {
                total += n;
            }
        }
    }
    total += spell_damage_auras_for(state, player);
    for modifier in &state.players[player].mods {
        if let ModifierKind::SpellDamage { amount } = modifier.kind
            && crate::mana::modifier_is_live(state, modifier)
        {
            total += amount.max(0);
        }
    }
    total.max(0)
}

/// R1180000: the Spell Damage the auras acting on either side give this player — each text that
/// carries `spellDamageAura`, on a card acting on its controller's field (face-up, the top of its
/// pile: `acting_texts_of`), gives its controller its amount, and the opponent too when it says
/// `both`. The amount moves with the card's declared number `spellDamage` (R386), so a Nerf or a Buff
/// moves it.
fn spell_damage_auras_for(state: &GameState, player: PlayerId) -> i32 {
    let mut total = 0;
    for side in crate::wire::PLAYER_IDS {
        for card in acting_texts_of(state, side) {
            for text in crate::scripts::texts_of(state, &card) {
                let Some(aura) = text.flags.spell_damage_aura else {
                    continue;
                };
                if side == player || aura.both == Some(true) {
                    total += crate::params::declared_or(state, &card, "spellDamage", aura.amount).max(0);
                }
            }
        }
    }
    total
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
/// the layers like its Lifesteal and Trample (§10.4), or a spell's, printed on its face (#44 True
/// Strike) or given by a Buff (R1748001), which the same reading finds on the card while it resolves —
/// or the effect's own `ignore_armor`, which states that its damage pierces without the source having
/// the keyword, as R85's `lifesteal` does for Lifesteal. It skips step 2 and nothing else: Divine
/// Shield, the hero cap and Indestructible all still apply.
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
    land(sink, args, amount_in, redirects, caught, false).dealt
}

/// What one instance did: what it dealt, and, for a hit that holds its Trample excess (R1848001's
/// hits on a target's neighbours), the excess it did not send on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Landed {
    dealt: i32,
    held: i32,
}

/// `land_hit`, with `hold` set for a hit whose Trample excess goes back to its caller instead of on to
/// a hero (`Landed::held`), as R1848001 carries a Meteor's excess from one neighbour to the next.
fn land(
    sink: &mut DamageSink<'_>,
    args: &DamageArgs,
    amount_in: i32,
    redirects: u32,
    caught: &[String],
    hold: bool,
) -> Landed {
    let source = args.source.as_ref();
    let target = &args.target;
    let flags = args.flags.as_ref();
    if amount_in <= 0 {
        return Landed::default();
    }
    // §4: damage is a unit's on the field — it stays there between turns and leaving the field takes
    // it off (R78). A card that has left the field, or lies dormant under a Stack (R13), is no unit to
    // hit: an effect still aimed at it fizzles (§8 Conventions), rather than leaving damage on a card
    // in a graveyard or a hand that would follow it back onto the field.
    let unit: Option<CardInstance> = match target {
        DamageTarget::Unit { instance } => {
            let Some(live) = find_instance(sink.state, &instance.id).cloned() else {
                return Landed::default();
            };
            if !crate::zones::acts_on_field(sink.state, &live) {
                return Landed::default();
            }
            Some(live)
        }
        DamageTarget::Hero { .. } => None,
    };

    // R1849000: a Spell's hit on a Unit immune to Spell Damage is no damage instance at all — before
    // Divine Shield, which stays, and with no Trample excess to send on.
    if let Some(live) = &unit
        && crate::restrictions::spell_damage_cannot_reach(sink.state, source, live)
    {
        return Landed::default();
    }

    // R1849004, step 0a: on its controller's own turn, a hit on a hero whose side carries an own-turn
    // redirect goes to the opponent's hero instead, up to each card's cap this turn; what no cap takes
    // lands here as the rest of the same hit.
    if let DamageTarget::Hero { player } = target
        && redirects < DAMAGE_REDIRECT_CAP
        && let Some(moved) = own_turn_redirect(sink, args, *player, amount_in, redirects, caught)
    {
        let rest = amount_in - moved.taken;
        let here = if rest > 0 {
            land_hit(sink, args, rest, redirects + 1, caught)
        } else {
            0
        };
        return Landed {
            dealt: moved.dealt + here,
            held: 0,
        };
    }

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
            return Landed::default();
        }
    }

    // Step 2: Armor, unless the hit pierces it (R346). A hero's total is `hero_armor_of`: what is
    // written on the hero plus every backrow grant (#84), summed per R124. Fatigue is an ordinary
    // instance on its own hero and pays this step like any other hit (R125); only "lose health"
    // bypasses the pipeline (R18), and that never comes through here.
    // Step 3 on a hero, with E6's divisors between the two: `hero_hit_amount`.
    // R1360: `absorbed` is what the Armor took off the hit at step 2, and nothing after it — not a
    // hero's divisors or cap — so a hit that pierces has none.
    // R1841001: `struck` is the hit as it meets Armor — a hit that deals half damage to heroes is
    // halved on a hero here, rounded down, and whole on a Unit.
    let mut struck = amount_in;
    let mut amount = amount_in;
    let mut absorbed = 0;
    match (target, &unit) {
        (DamageTarget::Hero { player }, _) => {
            if flags.is_some_and(|flags| flags.half_to_heroes == Some(true)) {
                struck = amount_in / HALF_DAMAGE_TO_HEROES_DIVISOR;
            }
            let pierce = pierces(sink.state, source, flags);
            if !pierce {
                absorbed = hero_armor_of(sink.state, *player).clamp(0, struck);
            }
            amount = hero_hit_amount(sink.state, *player, struck, pierce);
        }
        (DamageTarget::Unit { .. }, Some(live)) => {
            if !pierces(sink.state, source, flags) {
                amount = (amount - armor_of(&crate::layers::unit_view(sink.state, live).keywords)).max(0);
                absorbed = (amount_in - amount).max(0);
            }
            // Step 3 on a Unit: the caps its own guards set on it (R2179005).
            if let Some(cap) = unit_damage_cap(sink.state, live) {
                amount = amount.min(cap);
            }
        }
        (DamageTarget::Unit { .. }, None) => {}
    }

    // E5, E9: after the caps and before step 5, a hit that would bring its hero to 0 or less — this
    // hit alone, as R44 judges it — meets the "would take lethal damage" replacements, which may send
    // it to the other hero as a new instance from the same source, through that hero's Armor and caps,
    // or cancel it (R1704001).
    // #936 B17: in a Duos duel the hit is judged against own HP and the team's pool together.
    if let DamageTarget::Hero { player } = target
        && amount > 0
        && redirects < DAMAGE_REDIRECT_CAP
        && crate::duos_life::effective_life(sink.state, *player) - amount <= 0
    {
        let source_id = source.map(|card| card.id.clone());
        let to = crate::replacements::lethal_hit_window(sink, target, amount, source_id, caught);
        if let Some(LethalHitTo::Canceled) = to {
            // R1704001: a canceled hit deals nothing, and emits no damage event.
            return Landed::default();
        }
        if let Some(LethalHitTo::Target(to)) = to {
            let to = *to;
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
            return land(sink, &redirected, amount_in, redirects + 1, &next_caught, hold);
        }
    }

    // The zero rule: a hit reduced to 0 by steps 2 and 3 stops there, emits no `damage` and triggers
    // nothing (R63). R1361: one the target's Armor took whole is reported by `damageAbsorbed`, a
    // report nothing answers (`triggers::dispatch_event`); a hit a cap alone stopped is not Armor's.
    // The rule stands before step 4, as §4.4 orders it, so an Indestructible unit's Armor that takes
    // a hit whole reports it like any other unit's.
    if amount <= 0 {
        if absorbed > 0 && absorbed >= struck {
            sink.events.push(GameEvent::DamageAbsorbed {
                source_id: source.map(|card| card.id.clone()),
                target_id: target_id(target),
                absorbed,
                combat: flags.is_some_and(|flags| flags.combat == Some(true)),
            });
            let at = sink.events.len() - 1;
            crate::triggers::withhold_report(sink, at);
        }
        return Landed::default();
    }

    // Step 4: Indestructible units take nothing, and emit no damage event.
    if let Some(live) = &unit
        && has_keyword(
            &crate::layers::unit_view(sink.state, live).keywords,
            KeywordKind::Indestructible,
        )
    {
        return Landed::default();
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
        if let Some(LethalHitTo::Canceled) = to {
            return Landed::default();
        }
        if let Some(LethalHitTo::Target(to)) = to {
            let to = *to;
            let mut next_caught: Vec<String> = caught.to_vec();
            if let DamageTarget::Unit { instance } = &to {
                next_caught.push(instance.id.clone());
            }
            let redirected = DamageArgs {
                target: to,
                ..args.clone()
            };
            return land(sink, &redirected, amount_in, redirects + 1, &next_caught, hold);
        }
    }

    // Step 5: apply, capping what a Trample source deals to a unit at its health (R63).
    let mut dealt = amount;
    let mut trample_excess = 0;
    if let Some(live) = &unit {
        let view = crate::layers::unit_view(sink.state, live);
        // E6: a Spell's printed Trample is read off it while it resolves, as a unit's is, or stated.
        // R1750000: a Unit's own Trample is for the hits of its attack only, never for a strike back
        // or for the damage of its text.
        // R1848001: a hit whose excess goes onto its target's neighbours tramples, stated.
        let trample = flags.is_some_and(|flags| flags.trample == Some(true) || flags.trample_onto.is_some())
            || (source_has(sink.state, source, KeywordKind::Trample)
                && (flags.is_some_and(|flags| flags.attacking == Some(true))
                    || crate::restrictions::is_spell_source(sink.state, source)));
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
        let held = send_excess(sink, args, &unit, trample_excess, caught, hold);
        return Landed { dealt: 0, held };
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
            // #936 B13, B14: a Duos hero's own HP first, then its team's pool.
            crate::duos_life::hero_loses(sink.state, *player, dealt);
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

    // R1849003, step 8a: what a reflecting Unit took is dealt again, from it.
    if let Some(live) = &unit {
        reflect_damage(sink, live, source, flags, dealt);
    }

    // Step 9: Trample sends the excess on (`send_excess`).
    let held = send_excess(sink, args, &unit, trample_excess, caught, hold);
    Landed { dealt, held }
}

/// §4.4 step 9: a Trample excess goes on as its own instance — of the same hit, so not raised again
/// by Spell Damage, and keeping the hit's catchers (R1204) — to the target's controller's hero; or,
/// R1848001, onto the target's neighbours (`trample_onto_adjacent`); or, from a hit that holds it, back
/// to its caller untouched. Returns what is held: the excess for a hit that holds it, else 0.
fn send_excess(
    sink: &mut DamageSink<'_>,
    args: &DamageArgs,
    unit: &Option<CardInstance>,
    excess: i32,
    caught: &[String],
    hold: bool,
) -> i32 {
    if excess <= 0 {
        return 0;
    }
    if hold {
        return excess;
    }
    if let (Some(onto), Some(struck)) = (args.flags.and_then(|flags| flags.trample_onto), unit) {
        trample_onto_adjacent(sink, args, struck, excess, onto, caught);
        return 0;
    }
    let to_hero = DamageArgs {
        source: args.source.clone(),
        target: DamageTarget::Hero {
            player: controller_of_now(unit, &args.target),
        },
        amount: excess,
        flags: args.flags,
    };
    land_hit(sink, &to_hero, excess, 0, caught);
    0
}

/// R1848001 (Stinkyscape #3 Meteor, "Tramples onto adjacent Units", Radiant "and the enemy hero"): a
/// hit's Trample excess carried onto the Units adjacent to its target — §3.1's neighbours in its row
/// on its side, as Cleave's are (`zones::adjacent`, R1081000), a Unit a Spell may not affect passed by
/// (R81) — and with `AdjacentAndHero` on to the target's controller's hero. The excess goes to one
/// neighbour first, drawn from the match rng when there are two (Hearthstone's Rolling Fireball, whose
/// excess rolls left or right at random), as a new instance of the same hit that counts only up to
/// that Unit's health; what passes it goes to the other neighbour the same way, and what passes both
/// to the hero. A neighbour that takes the hit whole (its Divine Shield, its Armor, Indestructible)
/// stops the rest, as a target that takes a Trample hit whole sends nothing on.
fn trample_onto_adjacent(
    sink: &mut DamageSink<'_>,
    args: &DamageArgs,
    struck: &CardInstance,
    excess: i32,
    onto: TrampleOnto,
    caught: &[String],
) {
    let mut neighbours = adjacent_units_of(sink.state, struck, args.source.as_ref());
    if neighbours.len() == 2 && sink.rng.int(2) == 1 {
        neighbours.reverse();
    }
    let mut left = excess;
    for neighbour in neighbours {
        if left <= 0 {
            return;
        }
        let hit = DamageArgs {
            source: args.source.clone(),
            target: DamageTarget::Unit { instance: neighbour },
            amount: left,
            flags: Some(DamageFlags {
                trample: Some(true),
                trample_onto: None,
                ..args.flags.unwrap_or_default()
            }),
        };
        left = land(sink, &hit, left, 0, caught, true).held;
    }
    if onto == TrampleOnto::AdjacentAndHero && left > 0 {
        let to_hero = DamageArgs {
            source: args.source.clone(),
            target: DamageTarget::Hero {
                player: struck.controller,
            },
            amount: left,
            flags: args.flags,
        };
        land_hit(sink, &to_hero, left, 0, caught);
    }
}

/// R1848001: the Units acting beside `struck` now, lane order (its lower lane first), that a hit from
/// `source` may reach.
fn adjacent_units_of(
    state: &GameState,
    struck: &CardInstance,
    source: Option<&CardInstance>,
) -> Vec<CardInstance> {
    let Some(at) = crate::zones::slot_of(state, struck) else {
        return Vec::new();
    };
    crate::zones::adjacent(at)
        .into_iter()
        .filter_map(|slot| crate::zones::card_at(state, slot).cloned())
        .filter(|neighbour| {
            neighbour.id != struck.id
                && crate::zones::is_adjacent_card(state, struck, neighbour)
                && !crate::restrictions::spell_cannot_reach(state, source, neighbour)
        })
        .collect()
}

/// R1849004: where a card with an own-turn redirect notes how much it has moved this turn,
/// `{ turn, moved }`, as an Activate ability notes its uses (`subsystems::activate`).
const OWN_TURN_REDIRECT_KEY: &str = "ownTurnRedirect";

/// R1849004: how much `card`'s own-turn redirect has taken off its controller's hero this turn, read
/// off its note (`OWN_TURN_REDIRECT_KEY`); a note from another turn is 0. What the cap's rest is
/// counted from, here and in its preview (`query::own_turn_redirect_left`, R2476000).
pub(crate) fn own_turn_redirect_moved(state: &GameState, card: &CardInstance) -> i32 {
    card.memory
        .get(OWN_TURN_REDIRECT_KEY)
        .filter(|record| {
            record.get("turn").and_then(serde_json::Value::as_i64) == Some(i64::from(state.turn))
        })
        .and_then(|record| record.get("moved").and_then(serde_json::Value::as_i64))
        .map_or(0, |moved| moved as i32)
}

/// What an own-turn redirect did to one hit: how much it took off the hero, and what that dealt to
/// the opponent's hero.
struct Moved {
    taken: i32,
    dealt: i32,
}

/// R1849004 (Stinkyscape #5 Masochism Unmasked), §4.4 step 0a: on `player`'s own turn, a hit of
/// `amount` on their hero is taken off it by each card acting on their side that redirects, in R68's
/// order, as much as the card's cap has left this turn, and dealt to the opponent's hero as a new
/// instance from the same source, times the card's multiplier. The cap counts what is taken off the
/// hero, so the Radiant face's doubling deals up to twice it. `None` when nothing moved, which is
/// every hit without such a card: nothing is read but the flag, and nothing is written.
fn own_turn_redirect(
    sink: &mut DamageSink<'_>,
    args: &DamageArgs,
    player: PlayerId,
    amount: i32,
    redirects: u32,
    caught: &[String],
) -> Option<Moved> {
    if sink.state.active != player {
        return None;
    }
    let mut rest = amount;
    let mut dealt = 0;
    for card in acting_texts_of(sink.state, player) {
        if rest <= 0 {
            break;
        }
        let Some(redirect) = crate::scripts::flags_of(sink.state, &card).own_turn_redirect else {
            continue;
        };
        // R386: the card's declared numbers `cap` and `multiplier` where it declares them.
        let cap = crate::params::declared_or(sink.state, &card, "cap", redirect.cap).max(0);
        let multiplier =
            crate::params::declared_or(sink.state, &card, "multiplier", redirect.multiplier.unwrap_or(1))
                .max(1);
        let turn = sink.state.turn;
        let used = own_turn_redirect_moved(sink.state, &card);
        let take = rest.min(cap - used);
        if take <= 0 {
            continue;
        }
        if let Some(live) = find_instance_mut(sink.state, &card.id) {
            live.memory.insert(
                OWN_TURN_REDIRECT_KEY.to_string(),
                serde_json::json!({ "turn": turn, "moved": used + take }),
            );
        }
        let opponent = opponent_of(player);
        sink.events.push(GameEvent::Redirected {
            what: RedirectWhat::Damage,
            from_id: format!("hero-{player}"),
            to_id: format!("hero-{opponent}"),
            by_instance_id: Some(card.id.clone()),
        });
        let moved = DamageArgs {
            target: DamageTarget::Hero { player: opponent },
            ..args.clone()
        };
        dealt += land_hit(
            sink,
            &moved,
            take.saturating_mul(multiplier),
            redirects + 1,
            caught,
        );
        rest -= take;
    }
    if rest == amount {
        return None;
    }
    Some(Moved {
        taken: amount - rest,
        dealt,
    })
}

/// R1849003 (Stinkyscape #104 Reflective Plating), §4.4 step 8a: what `unit` was just dealt, when its
/// text reflects it, is dealt again from it as new instances — to the enemy hero, and on the Radiant
/// face also to the attacker whose own strike it was (Cleave's included), while that attacker is on
/// the field. A strike back, a Unit's text and a Spell are never "the attacker", so two Radiant
/// Platings fighting end after one reflection each: a reflected hit is no attacker's strike.
fn reflect_damage(
    sink: &mut DamageSink<'_>,
    unit: &CardInstance,
    source: Option<&CardInstance>,
    flags: Option<&DamageFlags>,
    dealt: i32,
) {
    let text = crate::scripts::flags_of(sink.state, unit);
    let to_hero = text.reflects_damage == Some(true);
    let to_attacker =
        text.reflects_to_attacker == Some(true) && flags.is_some_and(|flags| flags.attacking == Some(true));
    if !(to_hero || to_attacker) || dealt <= 0 {
        return;
    }
    let Some(now) = find_instance(sink.state, &unit.id).cloned() else {
        return;
    };
    if to_hero {
        let hit = DamageArgs {
            source: Some(now.clone()),
            target: DamageTarget::Hero {
                player: opponent_of(now.controller),
            },
            amount: dealt,
            flags: None,
        };
        land_hit(sink, &hit, dealt, 0, &[]);
    }
    if !to_attacker {
        return;
    }
    let Some(attacker) = source.and_then(|source| find_instance(sink.state, &source.id).cloned()) else {
        return;
    };
    if !crate::zones::acts_on_field(sink.state, &attacker) {
        return;
    }
    let hit = DamageArgs {
        source: Some(now),
        target: DamageTarget::Unit { instance: attacker },
        amount: dealt,
        flags: None,
    };
    land_hit(sink, &hit, dealt, 0, &[]);
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
/// that replaced it heals nothing. R1180003: a heal no replacement took is then multiplied by its
/// controller's healing multiplier (`query::heal_multiplier_of`, Meditative+ #7's Radiant face), the
/// cap still holding, so a heal a replacement turned into damage is never doubled.
pub fn heal_unit(sink: &mut DamageSink<'_>, instance: &CardInstance, amount: i32) -> i32 {
    heal_unit_by(sink, instance, amount, true)
}

/// R1180003: "heal up to N" on a unit — what it heals is what it would restore to reach N, a floor
/// no healing multiplier pushes it past (§6.3).
pub fn heal_unit_up_to_amount(sink: &mut DamageSink<'_>, instance: &CardInstance, amount: i32) -> i32 {
    heal_unit_by(sink, instance, amount, false)
}

/// A unit's heal. R1180003: `multiplied` is whether its controller's healing multiplier reaches it.
fn heal_unit_by(sink: &mut DamageSink<'_>, instance: &CardInstance, amount: i32, multiplied: bool) -> i32 {
    let stated = amount;
    if stated <= 0 {
        return 0;
    }
    let live = crate::zones::live_or(sink.state, instance);
    if crate::replacements::healing_replaced(
        sink,
        &DamageTarget::Unit {
            instance: live.clone(),
        },
        stated,
    ) {
        return 0;
    }
    let stated = if multiplied {
        stated.saturating_mul(crate::query::heal_multiplier_of(sink.state, live.controller))
    } else {
        stated
    };
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
    heal_hero_by(sink, player, amount, true)
}

/// A hero's heal. R1180003: `multiplied` is whether the player's healing multiplier reaches it —
/// every heal of an amount, Lifesteal's included, but not "heal up to N", which sets a floor the
/// multiplier would only push past (§6.3).
fn heal_hero_by(sink: &mut DamageSink<'_>, player: PlayerId, amount: i32, multiplied: bool) -> i32 {
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
    // R1180003: the heal no replacement took, multiplied by the player's healing multiplier.
    let healed = if multiplied {
        healed.saturating_mul(crate::query::heal_multiplier_of(sink.state, player))
    } else {
        healed
    };
    // #936 B14: a spent Duos hero heals its team's pool, never past its cap; any other heals as ever.
    let healed = crate::duos_life::hero_heals(sink.state, player, healed);
    if healed <= 0 {
        return 0;
    }
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
    // R1180003: a floor, not an amount, so no healing multiplier pushes it past N.
    heal_hero_by(sink, player, floor - health, false)
}

/// R18: lose health is not damage. No pipeline, no armor, no cap, no on-damage effects.
pub fn lose_health(sink: &mut DamageSink<'_>, player: PlayerId, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    // R2201000: a floored hero loses no more than it has above its floor.
    let lost = match crate::state::hero_floor_of(sink.state, player) {
        Some(floor) => amount.min((sink.state.players[player].hero.health - floor).max(0)),
        None => amount,
    };
    if lost <= 0 {
        return 0;
    }
    // #936 B13, B14: a Duos hero's own HP first, then its team's pool.
    crate::duos_life::hero_loses(sink.state, player, lost);
    sink.events.push(GameEvent::HealthLost { player, amount: lost });
    lost
}

/// E7: "Set a hero's health to N" (Classic #29) — no pipeline, not damage and not a heal, like R18's
/// lose health: no Armor, no cap, no replacement, nothing that answers a hit or a heal. `healthSet`.
pub fn set_hero_health(sink: &mut DamageSink<'_>, player: PlayerId, value: i32, source_id: Option<String>) {
    // #936: a Duos hero's own HP, or its team's pool once spent (`duos_life::hero_set`); the event
    // reports the health the hero then reads, which outside Duos is `value`.
    crate::duos_life::hero_set(sink.state, player, value);
    let health = sink.state.players[player].hero.health;
    sink.events.push(GameEvent::HealthSet {
        player,
        health,
        source_id,
    });
}
