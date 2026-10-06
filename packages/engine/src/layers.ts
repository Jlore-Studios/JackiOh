// Stat and keyword layers (SPEC §10.4). Always computed on read, never stored.

import type { Keyword } from "@jackioh/shared";
import { PLAYER_IDS, armorOf, hasKeyword } from "@jackioh/shared";
import { activeBrittleCount } from "./brittleCount";
import { defOf } from "./catalog";
import { ANIMATED_FALLBACK_ATTACK, ANIMATED_FALLBACK_HEALTH, BACKROW_ZONES, RADIANT_FALLBACK_FACTOR } from "./config";
import { scriptOf } from "./scripts";
import type { CardInstance, GameState } from "./state";
import type { StatMod } from "./script";
import { tunedKeywords, xOf } from "./tuning";
import { activeUnitsOf, cardAt } from "./zones";

export type UnitView = {
  attack: number;
  maxHealth: number;
  health: number;
  keywords: Keyword[];
  armor: number;
  position: "ATK" | "DEF";
};

/**
 * R349: what a summon's X/X (`statsOverride`) comes to on the face the instance wears. A card that
 * prints no Radiant form of its own (`radiantFallback`, the Ghoul Token) is, made Radiant, its base
 * face with its attack and health doubled, and its X/X is that base face — so a Radiant Ghoul
 * summoned 3/3 is a 6/6. A card that prints a Radiant form keeps its X on both faces, as the Bread
 * Token's "X/X, Armor X" and an X/X Rush Token do (§7).
 */
export function wornStatsOverride(
  def: { radiantFallback?: true },
  instance: Pick<CardInstance, "radiant" | "statsOverride">,
): { attack: number; health: number } | undefined {
  const stats = instance.statsOverride;
  if (stats === undefined || !instance.radiant || def.radiantFallback !== true) return stats;
  return { attack: RADIANT_FALLBACK_FACTOR * stats.attack, health: RADIANT_FALLBACK_FACTOR * stats.health };
}

/**
 * B2.7: a face that prints its stats as multiples of X (Classic+ #69 Buff Billy's "[3X/3X]") is those
 * multiples of the X the card was played for, tuned (`tuning.xOf`), and 0/0 when it has none — a
 * Recruit, a copy, a card outside a play — as the Ghoul Token's X/X is with no X (§7). A summon's
 * `statsOverride` still wins, as it wins over any printed stats.
 */
function xStatsOf(
  face: { xStats?: { attack: number; health: number } },
  instance: CardInstance,
): { attack: number; health: number } | undefined {
  const multiples = face.xStats;
  if (multiples === undefined) return undefined;
  const x = xOf(instance);
  return { attack: multiples.attack * x, health: multiples.health * x };
}

/**
 * The card's printed face, radiant when the instance is (§5.2), as the card's lasting changes leave
 * it: B2.7's X stats, and B3.4's `tuning` — the stats changes on top of the printed stats and the
 * keyword changes on the printed keywords (`tuning.tunedKeywords`). Tuning is part of the card, kept
 * in every zone (R78 leaves it alone), so this is the face a card in a hand or a deck will enter with
 * (B3.4 rule 6) as well as layer 1 on the field.
 */
export function faceOf(state: GameState, instance: CardInstance): { attack: number; health: number; keywords: Keyword[] } {
  const def = defOf(state, instance.defId);
  const face = instance.radiant ? def.radiant : def.base;
  const stats = wornStatsOverride(def, instance) ?? xStatsOf(face, instance);
  const keywords = tunedKeywords(printedKeywordsOf(state, instance), instance);
  // R657: an Animated card with no stats from any source (no printed stats, no statsOverride, no X
  // stats) fights as a 0/1 instead of dying as a 0/0 at the state check.
  const statless = stats === undefined && face.attack === undefined && face.health === undefined;
  const animated =
    statless &&
    [...keywords, ...instance.grantedKeywords].some(
      (keyword) => keyword.kind === "Animated" || keyword.kind === "Animated on your turn",
    );
  return {
    attack: (stats?.attack ?? face.attack ?? (animated ? ANIMATED_FALLBACK_ATTACK : 0)) + (instance.tuning?.attack ?? 0),
    health: (stats?.health ?? face.health ?? (animated ? ANIMATED_FALLBACK_HEALTH : 0)) + (instance.tuning?.health ?? 0),
    keywords,
  };
}

/**
 * The keywords the running face prints, before tuning (§5.2): what a Degrade takes a printed keyword
 * off and what a numbered keyword's tuning steps from (B3.4). §7: the Bread Token's radiant "Armor X"
 * is the same X as its X/X, so the printed `n` is a placeholder the summon fills in, exactly as
 * `statsOverride` fills in the printed 0/0.
 */
export function printedKeywordsOf(state: GameState, instance: CardInstance): Keyword[] {
  const def = defOf(state, instance.defId);
  const face = instance.radiant ? def.radiant : def.base;
  const override = instance.armorOverride;
  return override === undefined
    ? [...face.keywords]
    : face.keywords.map((keyword) => (keyword.kind === "Armor" ? { kind: "Armor" as const, n: override } : keyword));
}

/**
 * B3.3 rule 5, R385: a Brittle count in force is the card's Brittle, whatever it prints: the list's
 * Brittle entries give way to one `Brittle n` of the count, a given count on a card that printed none
 * included. With no count in force (a printed Brittle that has not started, off the field) the
 * printed keyword stands.
 */
function withBrittleCount(keywords: readonly Keyword[], instance: CardInstance): Keyword[] {
  const count = activeBrittleCount(instance);
  if (count === null) return [...keywords];
  return [...keywords.filter((keyword) => keyword.kind !== "Brittle"), { kind: "Brittle", n: count }];
}

/**
 * §10.4 layers 1 to 4 of the keywords, before auras and position: the printed keywords as tuning
 * leaves them (none on a Vanilla card, §6.3) with the granted ones, as a set (§6.1), and the Brittle
 * count in force. What a card in a hand or a deck is made of (B5 E38, R243) — the granted keywords it
 * gained there ride onto the field with it — and what a Degrade may take off it (B3.4).
 */
export function cardKeywords(state: GameState, instance: CardInstance): Keyword[] {
  const printed = instance.vanilla ? [] : faceOf(state, instance).keywords;
  return asSet(withBrittleCount([...printed, ...instance.grantedKeywords], instance));
}

/** §3.2, R13: a card in a unit zone that is not the top of its pile. */
function isDormant(state: GameState, instance: CardInstance): boolean {
  const zone = instance.zone;
  if (zone.z !== "field" || zone.row !== "units") return false;
  return cardAt(state, { player: zone.player, row: zone.row, lane: zone.lane })?.id !== instance.id;
}

/** Every permanent whose aura is in play, in lane order per side (§10.4 layer 5). */
function auraSources(state: GameState): CardInstance[] {
  // Every unit read walks this list, so it is built with plain loops (#188).
  const sources: CardInstance[] = [];
  for (const player of PLAYER_IDS) {
    sources.push(...activeUnitsOf(state, player));
    const backrow = state.players[player].backrow;
    for (let lane = 1; lane <= BACKROW_ZONES; lane += 1) {
      const card = backrow[lane - 1] ?? null;
      if (card !== null) sources.push(card);
    }
  }
  return sources;
}

/**
 * Aura contributions for one unit. An aura's `applies` predicate reads instance data only: it must
 * never call back into `unitView`, or the layers would recurse.
 */
function auraMods(state: GameState, unit: CardInstance): StatMod[] {
  const mods: StatMod[] = [];
  for (const source of auraSources(state)) {
    // §6.1/§6.3: Vanilla "clears printed keywords and scripts", and an aura is part of a card's
    // Script (§10.9), so a Vanilla'd permanent projects nothing. It still *receives* aura grants:
    // an aura is the board's text, not the unit's.
    if (source.vanilla === true) continue;
    const aura = scriptOf(source).aura;
    if (aura === undefined) continue;
    for (const entry of aura({ state, self: source, radiant: source.radiant })) {
      if (entry.applies(unit)) mods.push(entry.mod);
    }
  }
  return mods;
}

/**
 * §6.1: a unit's keywords are "computed as a set per unit", and §10.4 makes them the union of the
 * printed, granted, aura and position keywords — so a keyword two sources give is had once (Tempo
 * Timmy's printed Rush under Jlockeed's Weapons, a Taunt unit's own Taunt in Defense Position), and
 * the view hands the client one entry for it (§10.8). The numbered keywords are the exception: Armor
 * sums across its sources (§10.4) and Lucky X stacks, so every entry of theirs is kept for the sum.
 */
function asSet(keywords: readonly Keyword[]): Keyword[] {
  const seen = new Set<Keyword["kind"]>();
  return keywords.filter((keyword) => {
    // B5 E6: Spell Damage is numbered too, and sums across its sources like Armor.
    if (keyword.kind === "Armor" || keyword.kind === "Lucky" || keyword.kind === "Spell Damage") return true;
    if (seen.has(keyword.kind)) return false;
    seen.add(keyword.kind);
    return true;
  });
}

/**
 * B5 E35: a keyword that holds only while a condition does (Classic #69 Plague Charger's First Strike
 * "while it has a Plague Counter"): the card's `conditionalKeywords` hook, read with its printed ones,
 * so a Vanilla takes it (`scriptOf` runs no script for one). The hook reads instance data only.
 */
function conditionalKeywordsOf(state: GameState, instance: CardInstance): Keyword[] {
  const hook = scriptOf(instance).conditionalKeywords;
  return hook === undefined ? [] : hook({ state, self: instance, radiant: instance.radiant });
}

export function unitView(state: GameState, instance: CardInstance): UnitView {
  const layered = computeLayers(state, instance);
  // Attack floors at 0; max health may fall to 0, which the state check turns into a death (§10.4).
  const clampedAttack = Math.max(0, layered.attack);
  return {
    attack: clampedAttack,
    maxHealth: layered.maxHealth,
    health: layered.maxHealth - instance.damage,
    keywords: layered.keywords,
    armor: armorOf(layered.keywords),
    position: layered.position,
  };
}

/**
 * §10.4's attack before layer 5's floor at 0 — what KY's Constant needs to set a unit's attack to a
 * number exactly (`effects/tune.setNumber`), since a card whose layers come to −2 shows 0 and a delta
 * taken off the shown number would land short.
 */
export function unclampedAttack(state: GameState, instance: CardInstance): number {
  return computeLayers(state, instance).attack;
}

/** §10.4's five layers, attack unfloored: `unitView` floors it, `unclampedAttack` reads it raw. */
function computeLayers(
  state: GameState,
  instance: CardInstance,
): { attack: number; maxHealth: number; keywords: Keyword[]; position: "ATK" | "DEF" } {
  const printed = faceOf(state, instance);
  const position = instance.position ?? "ATK";

  // Layers 1 and 3: printed stats of the running face, with Fuse already baked in (R77).
  let attack = printed.attack;
  let maxHealth = printed.health;

  // Layer 2: a card that sets its own stats from the board (#92 Felinor Fiender, R39). §10.4 words
  // it as "Felinor Fiender *adds* the sum of your Felinors' layer-4 stats", so the hook returns the
  // sum rather than a finished total, and R39's "never below printed" floors each sum at 0 — a
  // Felinor carrying a negative health buff can pull the total back toward printed, never past it.
  // R132 and R150: this is the ONE floor R39 asks for, on each stat's combined total separately.
  // The contributors reach the hook unfloored (`statsWithBuffs`), so a negative buff really does
  // pull its sum down, and flooring attack here never touches the health sum (R116's per-component
  // clamp).
  // Vanilla has cleared the card's scripts, so a Vanilla'd body keeps its printed stats (§6.1).
  if (instance.vanilla !== true) {
    const setStat = scriptOf(instance).setStat;
    if (setStat !== undefined) {
      const set = setStat({ state, self: instance, radiant: instance.radiant });
      attack += Math.max(0, set.attack ?? 0);
      maxHealth += Math.max(0, set.maxHealth ?? 0);
    }
  }

  // Layer 4: permanent buffs.
  attack += instance.buffs.attack;
  maxHealth += instance.buffs.health;

  // Layer 5: auras. A card dormant under a Stack pile is "not on the field for effects" (§3.2, R13)
  // and an aura is one, so none reaches it: it keeps its damage and its own layers 1 to 4, and the
  // board's auras apply again the moment it resumes on top. Without this an aura that shrinks max
  // health (#46) killed buried cards the top of the pile shielded.
  const auras = isDormant(state, instance) ? [] : auraMods(state, instance);
  for (const mod of auras) {
    attack += mod.attack ?? 0;
    maxHealth += mod.maxHealth ?? 0;
  }
  // §10.4: an aura that sets attack to a value (Classic #88's Radiant "0 Attack") applies after every
  // other layer, so nothing above lifts it; with several, the last in aura order holds.
  for (const mod of auras) if (mod.setAttack !== undefined) attack = mod.setAttack;

  // B3.3 rule 5: a Brittle count in force is the unit's Brittle (`withBrittleCount`).
  const keywords: Keyword[] = withBrittleCount(
    [
      ...(instance.vanilla ? [] : printed.keywords),
      // B5 E35: the keywords its text gives it only while a condition holds, beside the printed ones.
      ...conditionalKeywordsOf(state, instance),
      ...instance.grantedKeywords,
      ...auras.flatMap((mod) => mod.keywords ?? []),
    ],
    instance,
  );

  // Position grants: Defense adds Taunt and Armor +1 (§4.1).
  if (position === "DEF") keywords.push({ kind: "Taunt" }, { kind: "Armor", n: 1 });

  // R46: an Indestructible unit that would have been destroyed loses Taunt for the turn. R347: an
  // Indestructible unit never has Taunt at all — printed, granted, from an aura or from Defense
  // Position — so Indestructible, from whichever source, takes Taunt out of the set.
  // A spent Divine Shield and a used Reborn are gone until granted again (§6.1, §4.5 step 4).
  const tauntSuppressed =
    instance.tauntSuppressedTurn === state.turn || keywords.some((k) => k.kind === "Indestructible");
  const finalKeywords = asSet(
    keywords.filter(
      (k) =>
        !(tauntSuppressed && k.kind === "Taunt") &&
        !(instance.divineShieldSpent === true && k.kind === "Divine Shield") &&
        !(instance.rebornSpent === true && k.kind === "Reborn"),
    ),
  );

  return { attack, maxHealth, keywords: finalKeywords, position };
}

/**
 * §10.4 layers 1 to 4: printed stats plus permanent buffs, before auras — the reading a set-stat
 * layer that sums other units needs (#92 Felinor Fiender, R116).
 *
 * R150: NO PER-UNIT FLOOR. §10.4 floors attack at layer 5 and at nothing earlier, so this reading
 * reports what layers 1 to 4 actually come to, negative included. A `Math.max(0, …)` here would be
 * invisible to a single card and wrong for every caller that sums: a Felinor carrying a −5 buff
 * would contribute 0 instead of −2 and could never "pull the attack sum toward 0", which is exactly
 * what R132 requires of #92's total. The floor belongs where the value is finally used — on the
 * combined total in `unitView`'s layer 2 above (R116, R132), and on the displayed attack at the end
 * of `unitView` (§10.4 layer 5) — so it is applied once, by the reader that knows which number the
 * rule floors, rather than baked into a reading that has more than one reader.
 *
 * The only other readers are the tests, `query.ts`'s re-export and `viewFor`'s hand cards (R243),
 * which floor the attack they show; R89's death snapshot reads `unitView` (`stateCheck.ts`), which
 * floors its own attack, so nothing is left unclamped by this.
 */
export function statsWithBuffs(state: GameState, instance: CardInstance): { attack: number; maxHealth: number } {
  const printed = faceOf(state, instance);
  return {
    attack: printed.attack + instance.buffs.attack,
    maxHealth: printed.health + instance.buffs.health,
  };
}

export function keywordsOf(state: GameState, instance: CardInstance): Keyword[] {
  return unitView(state, instance).keywords;
}

export function unitHas(state: GameState, instance: CardInstance, kind: Keyword["kind"]): boolean {
  return hasKeyword(unitView(state, instance).keywords, kind);
}
