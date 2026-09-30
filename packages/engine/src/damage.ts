// One damage instance: the ten ordered steps of SPEC §4.4, plus heal and lose health.
// M2-T3 adds a test per step; combat's Cleave step lives in combat.ts (M2-T4).
//
// Patch v0.2.0 (docs/classic-sets.md B5) adds to the pipeline, in the order a hit meets them:
//   - E6 Spell Damage, before step 1: a Spell's hit is raised by the Spell Damage on its controller's
//     side (`spellDamageOf`), once — a Trample excess or a redirected hit is the same hit going on,
//     and is not raised again;
//   - E6 on a hero, after step 2's Armor: the divisors its controller's cards set (several multiply,
//     rounded up once), then step 3's caps, the lowest of every cap (`heroHitAmount`);
//   - E5 and E9, after the caps and before step 5: a hit that would bring its hero to 0 or less meets
//     the "would take lethal damage" replacements (`replacements.lethalHitWindow`), which may send it
//     to the other hero as a new instance from the same source;
//   - E6 Trample stated by an effect (`flags.trample`), as R346 has an effect state Pierce;
//   - E5 and E8 in every heal: "would be healed" and its conversion into Pierce damage
//     (`replacements.healingReplaced`);
//   - E7 set health (`setHeroHealth`): no pipeline, not damage, not a heal.

import type { GameEvent, PlayerId } from "@jackioh/shared";
import { armorOf, hasKeyword, opponentOf } from "@jackioh/shared";
import { ANTI_ONESHOT_CAP, DAMAGE_REDIRECT_CAP, HERO_ARMOR } from "./config";
import { cardTypeOf } from "./faces";
import { unitView } from "./layers";
import { healingReplaced, lethalHitWindow } from "./replacements";
import { isSpellSource } from "./restrictions";
import { flagsOf, scriptOf, textsOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import { activeUnitsOf, cardAt, slotsOf } from "./zones";

export type DamageTarget = { kind: "unit"; instance: CardInstance } | { kind: "hero"; player: PlayerId };

export type DamageArgs = {
  source: CardInstance | null;
  target: DamageTarget;
  amount: number;
  /**
   * `trample` (E6): the effect states that its own damage has Trample, as `ignoreArmor` states Pierce
   * (R346) and `lifesteal` Lifesteal (R85), so a Spell's printed Trample (Classic #83 Flame Lance)
   * still tramples on a repeat that has no source left.
   */
  flags?: { ignoreArmor?: boolean; combat?: boolean; lifesteal?: boolean; trample?: boolean };
};

export type DamageSink = { state: GameState; events: GameEvent[] };

function targetId(target: DamageTarget): string {
  return target.kind === "unit" ? target.instance.id : `hero-${target.player}`;
}

function controllerOf(target: DamageTarget): PlayerId {
  return target.kind === "unit" ? target.instance.controller : target.player;
}

/**
 * §4.4 step 2 for a hero: the Armor written on the hero itself plus every backrow card that grants
 * it (#84 Going Long), each contributing the `HERO_ARMOR` value its own face and price select.
 *
 * R124: hero Armor from several sources **adds up**, exactly as §6.2's Armor stacks on a unit
 * (printed + Defense +1 + auras) — two Going Longs paid 2 are Armor 4. That is deliberately the
 * opposite of `heroDamageCap` below, which takes the *smallest* cap on offer: a cap is a ceiling,
 * Armor is a reduction, so they compose in opposite directions and are not unified.
 *
 * Nothing about the grant is stored, so it stops the moment the granting card leaves the backrow.
 * Every reader of hero Armor goes through here — the pipeline, `subsystems/lethal`'s projection,
 * `subsystems/scorer` and §10.8's hero block — so no projection can disagree with the hit (R44).
 */
export function heroArmorOf(state: GameState, player: PlayerId): number {
  return slotsOf(player, "backrow")
    .map((ref) => cardAt(state, ref))
    .reduce((sum, card) => {
      if (card === null) return sum;
      const side = card.radiant ? HERO_ARMOR.radiant : HERO_ARMOR.base;
      // R124, R102: every Going Long text a card carries grants its own Armor, at the price that
      // text's card was played for — a Going Long fused onto a Going Long is Armor 4 as two apart are.
      return textsOf(card).reduce((total, text) => {
        const grants = text.flags.heroArmor === true ? 1 : typeof text.flags.heroArmor === "number" ? text.flags.heroArmor : 0;
        return total + Math.max(0, Math.trunc(grants)) * (text.embiggened ? side.embiggen : side.paid);
      }, sum);
    }, state.players[player].hero.armor);
}

/**
 * The cards acting on a player's side of the field (§3.2): the top of each unit pile and each backrow
 * card, a face-down Trap left out — its text is in nobody's use until it fires (R33). What E6's hero
 * guards read.
 */
function actingTextsOf(state: GameState, player: PlayerId): CardInstance[] {
  const backrow = slotsOf(player, "backrow").flatMap((ref) => {
    const card = cardAt(state, ref);
    if (card === null) return [];
    const type = cardTypeOf(state, card);
    const faceDown = (type === "Trap" || type === "Field Trap") && card.faceUp !== true;
    return faceDown ? [] : [card];
  });
  return [...activeUnitsOf(state, player), ...backrow];
}

/** E6: every hero guard a player's cards set on their hero now (`Script.heroGuard`). */
function heroGuardsOf(state: GameState, player: PlayerId): { cap?: number; divisor?: number }[] {
  return actingTextsOf(state, player).flatMap((card) => {
    const guard = scriptOf(card).heroGuard;
    return guard === undefined ? [] : guard({ state, self: card, radiant: card.radiant });
  });
}

/**
 * §4.4 step 3: the smallest hero cap on offer — any Anti-oneshot Armor this player controls, and every
 * per-hit cap its cards set (E6, Classic+ #11 Anime Armor's 1): the lowest cap wins.
 */
export function heroDamageCap(state: GameState, player: PlayerId): number | null {
  const oneshot = slotsOf(player, "backrow")
    .map((ref) => cardAt(state, ref))
    .flatMap((card) => {
      if (card === null || flagsOf(card).antiOneshot !== true) return [];
      return [card.radiant ? ANTI_ONESHOT_CAP.radiant : ANTI_ONESHOT_CAP.base];
    });
  const guarded = heroGuardsOf(state, player).flatMap((guard) =>
    guard.cap === undefined ? [] : [Math.max(0, Math.trunc(guard.cap))],
  );
  const caps = [...oneshot, ...guarded];
  return caps.length === 0 ? null : Math.min(...caps);
}

/**
 * E6: what a hit on this player's hero is divided by after Armor — the product of every divisor its
 * cards set (Classic #75 Argusland's 2, 4 on its Radiant face), so several multiply. 1 when none does.
 */
export function heroDamageDivisor(state: GameState, player: PlayerId): number {
  return heroGuardsOf(state, player).reduce(
    (product, guard) => (guard.divisor === undefined ? product : product * Math.max(1, Math.trunc(guard.divisor))),
    1,
  );
}

/**
 * §4.4 steps 2 and 3 on a hero, with E6 between them: Armor (unless the hit pierces, R346), then the
 * divisors, rounded up once (R463), then the lowest cap. The one reading of what a hit takes off a
 * hero, which the pipeline, R44's lethal projection and the Zephyrs scorer all call, so no projection
 * disagrees with the hit.
 */
export function heroHitAmount(state: GameState, player: PlayerId, amount: number, pierce = false): number {
  if (amount <= 0) return 0;
  const afterArmor = pierce ? amount : Math.max(0, amount - heroArmorOf(state, player));
  const divided = Math.ceil(afterArmor / heroDamageDivisor(state, player));
  const cap = heroDamageCap(state, player);
  return cap === null ? divided : Math.min(divided, cap);
}

/**
 * E6, §4.4 step 0: the Spell Damage on a player's side — every "Spell Damage +N" among the keywords
 * of the units acting on their field (the top of each pile, an animated card standing in a unit zone
 * included), summed, as Armor sums (a numbered keyword, §10.4).
 */
export function spellDamageOf(state: GameState, player: PlayerId): number {
  let total = 0;
  for (const card of activeUnitsOf(state, player)) {
    for (const keyword of unitView(state, card).keywords) {
      if (keyword.kind === "Spell Damage") total += keyword.n;
    }
  }
  return Math.max(0, total);
}

/** E6: how much a hit from this source is raised — its controller's Spell Damage, when it is a Spell. */
function spellDamageFor(state: GameState, source: CardInstance | null): number {
  if (source === null || !isSpellSource(state, source)) return 0;
  const controller = source.zone.z === "resolving" ? source.zone.player : source.controller;
  return spellDamageOf(state, controller);
}

/** The card that acts in its unit zone: on the field, and the top of its pile (§3.2). */
function actsOnField(state: GameState, unit: CardInstance): boolean {
  const zone = unit.zone;
  if (zone.z !== "field") return false;
  return cardAt(state, { player: zone.player, row: zone.row, lane: zone.lane })?.id === unit.id;
}

/**
 * R42: a unit something has already killed and the state check has not collected yet — at 0 or less
 * health, or marked destroyed (a Poisonous hit, a destroy). A unit is killed once, so whatever lands
 * on it afterwards changes nothing about who killed it: not a later hit (a Cleave, a second spell, a
 * Death of the same pass), not a Poisonous one, and not a destroy (`effects/destroy.ts`).
 */
export function alreadyKilled(state: GameState, unit: CardInstance): boolean {
  const view = unitView(state, unit);
  // §4.5 step 1's own test: an Indestructible unit dies only once its max health is gone (R69).
  if (hasKeyword(view.keywords, "Indestructible")) return view.maxHealth <= 0;
  return unit.markedDestroyed === true || view.health <= 0;
}

/**
 * R42, R89: "a death whose lethal damage instance came from this unit". A hit is lethal when it
 * takes a unit nothing has killed yet (`alreadyKilled`) to 0 or less health, and that is the moment
 * it is credited — never at death, which a layer can cause long after the last hit (an aura
 * lowering max health, #46): a hit that left such a unit standing clears any older credit, and a hit
 * on a unit something already killed changes nothing, since the first one killed it. Poisonous
 * credits its own hit in step 7 on the same terms. The state check forgets a credit whose unit is
 * standing again.
 */
function creditKiller(unit: CardInstance, source: CardInstance | null, killedBefore: boolean, after: number): void {
  if (killedBefore) return;
  if (after > 0 || source === null) {
    delete unit.lastDamagedBy;
    return;
  }
  unit.lastDamagedBy = source.id;
}

/**
 * R346: whether a hit skips §4.4 step 2. Pierce is a keyword of the source — a unit's, read through
 * the layers like its Lifesteal and Trample (§10.4), or a spell's printed on its face (#44 True
 * Strike), which the same reading finds on the card while it resolves — or the effect's own
 * `ignoreArmor`, which states that its damage pierces without the source having the keyword, as
 * R85's `lifesteal` does for Lifesteal. It skips step 2 and nothing else: Divine Shield, the hero
 * cap and Indestructible all still apply.
 */
export function pierces(state: GameState, source: CardInstance | null, flags?: DamageArgs["flags"]): boolean {
  if (flags?.ignoreArmor === true) return true;
  return source !== null && hasKeyword(unitView(state, source).keywords, "Pierce");
}

/**
 * Deal one damage instance. Returns the amount actually dealt — by a redirected hit, where the
 * instance went on to the other hero (E9). A hit of 0 before step 1 is not a damage instance at
 * all: Divine Shield stays, nothing triggers (R63), and there is nothing for Spell Damage to raise.
 */
export function dealDamage(sink: DamageSink, args: DamageArgs): number {
  const amountIn = Math.trunc(args.amount);
  if (amountIn <= 0) return 0;
  // E6: before step 1, a Spell's hit is raised by its controller's Spell Damage.
  return landHit(sink, args, amountIn + spellDamageFor(sink.state, args.source), 0);
}

/**
 * §4.4 from step 1, for a hit already raised by Spell Damage. A Trample excess (step 9) and a
 * redirected hit (E9) are new instances of the same hit, so they come back in here rather than
 * through `dealDamage`, which would raise them a second time. `redirects` counts how often this hit
 * has moved hero already (`DAMAGE_REDIRECT_CAP`).
 */
function landHit(sink: DamageSink, args: DamageArgs, amountIn: number, redirects: number): number {
  const { state, events } = sink;
  const { source, target } = args;
  if (amountIn <= 0) return 0;
  // §4: damage is a unit's on the field — it stays there between turns and leaving the field takes
  // it off (R78). A card that has left the field, or lies dormant under a Stack (R13), is no unit to
  // hit: an effect still aimed at it fizzles (§8 Conventions), rather than leaving damage on a card
  // in a graveyard or a hand that would follow it back onto the field.
  if (target.kind === "unit" && !actsOnField(state, target.instance)) return 0;

  // Step 1: Divine Shield negates the whole hit and is gone.
  if (target.kind === "unit") {
    const view = unitView(state, target.instance);
    if (hasKeyword(view.keywords, "Divine Shield") && target.instance.divineShieldSpent !== true) {
      target.instance.divineShieldSpent = true;
      events.push({ type: "divineShieldLost", instanceId: target.instance.id });
      return 0;
    }
  }

  // Step 2: Armor, unless the hit pierces it (R346). A hero's total is `heroArmorOf`: what is
  // written on the hero plus every backrow grant (#84), summed per R124. Fatigue is an ordinary
  // instance on its own hero and pays this step like any other hit (R125); only "lose health"
  // bypasses the pipeline (R18), and that never comes through here.
  // Step 3 on a hero, with E6's divisors between the two: `heroHitAmount`.
  let amount = amountIn;
  if (target.kind === "hero") {
    amount = heroHitAmount(state, target.player, amount, pierces(state, source, args.flags));
  } else if (!pierces(state, source, args.flags)) {
    amount = Math.max(0, amount - armorOf(unitView(state, target.instance).keywords));
  }

  // E5, E9: after the caps and before step 5, a hit that would bring its hero to 0 or less — this
  // hit alone, as R44 judges it — meets the "would take lethal damage" replacements, which may send
  // it to the other hero as a new instance from the same source, through that hero's Armor and caps.
  if (
    target.kind === "hero" &&
    amount > 0 &&
    redirects < DAMAGE_REDIRECT_CAP &&
    state.players[target.player].hero.health - amount <= 0
  ) {
    const to = lethalHitWindow(sink, { player: target.player, amount, sourceId: source?.id ?? null });
    if (to !== null) return landHit(sink, { ...args, target: { kind: "hero", player: to } }, amountIn, redirects + 1);
  }

  // Step 4: Indestructible units take nothing, and emit no damage event.
  if (target.kind === "unit" && hasKeyword(unitView(state, target.instance).keywords, "Indestructible")) {
    return 0;
  }

  // The zero rule: a hit reduced to 0 emits nothing and triggers nothing (R63).
  if (amount <= 0) return 0;

  // Step 5: apply, capping what a Trample source deals to a unit at its health (R63).
  let dealt = amount;
  let trampleExcess = 0;
  if (target.kind === "unit") {
    const view = unitView(state, target.instance);
    // E6: a Spell's printed Trample is read off it while it resolves, as a unit's is, or stated.
    const trample =
      args.flags?.trample === true || (source !== null && hasKeyword(unitView(state, source).keywords, "Trample"));
    if (trample && amount > view.health) {
      dealt = Math.max(0, view.health);
      trampleExcess = amount - dealt;
    }
  }

  // R63's zero rule, now for the Trample cap: a unit already at 0 or less health (max health
  // dragged down by Suppressive Aura, or damage the state check has not collected yet) has no
  // health for the hit to count against, so nothing is dealt to it. Nothing dealt is not a damage
  // instance: no `damage` event, no `lastDamagedBy`, and none of steps 6 to 8. Step 9 still runs,
  // with the whole amount, because the excess beyond that unit's health is all of it.
  if (dealt <= 0) {
    if (trampleExcess > 0) {
      landHit(sink, { source, target: { kind: "hero", player: controllerOf(target) }, amount: trampleExcess, flags: args.flags }, trampleExcess, 0);
    }
    return 0;
  }

  // R42: whether something had killed the unit before this hit, which then kills nothing.
  const killedBefore = target.kind === "unit" && alreadyKilled(state, target.instance);
  if (target.kind === "unit") {
    const before = unitView(state, target.instance).health;
    target.instance.damage += dealt;
    creditKiller(target.instance, source, killedBefore, before - dealt);
  } else {
    state.players[target.player].hero.health -= dealt;
  }

  events.push({
    type: "damage",
    sourceId: source?.id ?? null,
    targetId: targetId(target),
    amount: dealt,
    combat: args.flags?.combat === true,
  });

  // Step 6 (on-damage triggers) is dispatched by the trigger loop from the `damage` event (§10.3).

  // Step 7: Poisonous destroys a damaged unit.
  if (
    target.kind === "unit" &&
    dealt >= 1 &&
    source !== null &&
    hasKeyword(unitView(state, source).keywords, "Poisonous")
  ) {
    target.instance.markedDestroyed = true;
    // R42: the Poisonous hit is the one that destroys it, whatever health it left — unless something
    // had already killed it, in which case this hit landed on a dead unit and kills nothing.
    if (!killedBefore) target.instance.lastDamagedBy = source.id;
  }

  // Step 8: Lifesteal heals the source's controller's hero by the amount dealt. R85: an effect
  // may state that its own damage has Lifesteal, which heals without granting the source the keyword.
  // R85's heal goes to the source's controller, so an effect with no source heals nobody.
  const sourceHasLifesteal = source !== null && hasKeyword(unitView(state, source).keywords, "Lifesteal");
  if (source !== null && (sourceHasLifesteal || args.flags?.lifesteal === true)) {
    healHero(sink, source.controller, dealt);
  }

  // Step 9: Trample sends the excess to the target's controller's hero as its own instance — of the
  // same hit, so not raised again by Spell Damage.
  if (trampleExcess > 0) {
    landHit(sink, { source, target: { kind: "hero", player: controllerOf(target) }, amount: trampleExcess, flags: args.flags }, trampleExcess, 0);
  }

  return dealt;
}

/**
 * §6.3 Heal: units are capped at max health, heroes are not. E5, E8: the heal first meets the "would
 * be healed" replacements on its stated amount, a heal on an undamaged unit included (R462) — one
 * that replaced it heals nothing.
 */
export function healUnit(sink: DamageSink, instance: CardInstance, amount: number): number {
  const stated = Math.trunc(amount);
  if (stated <= 0) return 0;
  if (healingReplaced(sink, { kind: "unit", instance }, stated)) return 0;
  const healed = Math.min(instance.damage, stated);
  instance.damage -= healed;
  if (healed > 0) sink.events.push({ type: "healed", targetId: instance.id, amount: healed });
  return healed;
}

export function healHero(sink: DamageSink, player: PlayerId, amount: number): number {
  if (amount <= 0) return 0;
  const healed = Math.trunc(amount);
  if (healed <= 0) return 0;
  if (healingReplaced(sink, { kind: "hero", player }, healed)) return 0;
  sink.state.players[player].hero.health += healed;
  sink.events.push({ type: "healed", targetId: `hero-${player}`, amount: healed });
  return healed;
}

/** "Heal to full" removes all damage; "heal up to N" raises a hero to at least N (§6.3). */
export function healToFull(sink: DamageSink, instance: CardInstance): number {
  return healUnit(sink, instance, instance.damage);
}

export function healHeroUpTo(sink: DamageSink, player: PlayerId, floor: number): number {
  const hero = sink.state.players[player].hero;
  if (hero.health >= floor) return 0;
  return healHero(sink, player, floor - hero.health);
}

/** R18: lose health is not damage. No pipeline, no armor, no cap, no on-damage effects. */
export function loseHealth(sink: DamageSink, player: PlayerId, amount: number): number {
  if (amount <= 0) return 0;
  const lost = Math.trunc(amount);
  sink.state.players[player].hero.health -= lost;
  sink.events.push({ type: "healthLost", player, amount: lost });
  return lost;
}

/**
 * E7: "Set a hero's health to N" (Classic #29) — no pipeline, not damage and not a heal, like R18's
 * lose health: no Armor, no cap, no replacement, nothing that answers a hit or a heal. `healthSet`.
 */
export function setHeroHealth(sink: DamageSink, player: PlayerId, value: number, sourceId: string | null): void {
  const health = Math.trunc(value);
  sink.state.players[player].hero.health = health;
  sink.events.push({ type: "healthSet", player, health, sourceId });
}

export function enemyOf(player: PlayerId): PlayerId {
  return opponentOf(player);
}
